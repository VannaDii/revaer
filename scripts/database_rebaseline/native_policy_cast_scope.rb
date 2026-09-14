# frozen_string_literal: true

require "json"
require_relative "native_policy_cast_phases"
require_relative "ingestion_metadata"

module RevaerDatabaseRebaseline
  class NativePolicyCastScope
    BRANCHES = [["drop_canonical", "drop_canonical"], ["drop_source", "drop_source"],
                ["downrank", "downrank"], ["flag", "flag"], ["require", "flag"], ["prefer", "flag"]].freeze

    def initialize(routine:, helper_query:, cast_source:, database:)
      source = routine.fetch("source")
      statements = source.scan(/INSERT INTO search_filter_decision \(\n.*?FROM tmp_policy_matches;/m)
      raise Failure, "frozen cast statement is not unique" unless statements.length == 1

      @decision = statements.fetch(0).delete_suffix(";")
      line = source.lines.index { |text| text.strip == "INSERT INTO search_filter_decision (" }
      raise Failure, "frozen cast callsite line absent" unless line

      @context = "SQL statement \"#{@decision}\"\nPL/pgSQL function #{routine.fetch('signature')} line #{line + 1} at SQL statement"
      @helpers = helper_query.strip
      @cast_source = cast_source
      @database = database
    end

    def validate!(events:, records:, evidence:, catalog:, name:, mode:)
      raise Failure, "unknown cached cast context" unless NativePolicyCastPhases::CASES.include?(name) && %w[cold helpers-first].include?(mode)
      raise Failure, "error cold case is not a missing cast context" if name.start_with?("release-regex-error-") && mode == "cold"
      raise Failure, "unqualified event kind in raw native input" unless events.all? { |row| %w[call regex executor_end].include?(row.fetch("kind")) }
      raise Failure, "reference cast definition or binding changed" unless catalog.values_at("language", "volatility", "security_definer", "config", "castmethod", "castcontext", "source") ==
        ["sql", "i", false, nil, "f", "a", @cast_source] && catalog.fetch("castfunc").is_a?(Integer) && catalog.fetch("castfunc").positive?

      frames = evidence.fetch("frames")
      backend = frames.fetch(0).fetch("backend")
      raise Failure, "invalid or mixed application backend" unless backend.match?(/\A[1-9][0-9]*\z/) && frames.all? { |row| row.fetch("backend") == backend }
      backend = Integer(backend)
      raise Failure, "mixed server record provenance" unless records.all? { |row| row.fetch("pid") == backend && row.fetch("dbname") == @database && row.fetch("user") == "postgres" }
      lines = records.map { |row| row.fetch("line_num") }
      raise Failure, "duplicate or unordered server records" unless lines.all? { |line| line.is_a?(Integer) && line.positive? } && lines.each_cons(2).all? { |a, b| a < b }
      native_lines = events.map { |row| row.fetch("native_line") }
      raise Failure, "duplicate or unordered native records" unless native_lines.all? { |line| line.is_a?(Integer) && line.positive? } && native_lines.each_cons(2).all? { |a, b| a < b }

      executions = events.select { |row| row.fetch("kind") == "executor_end" }
      queries = mode == "helpers-first" ? [@helpers] : []
      queries += [@decision, @decision] unless name.start_with?("release-regex-error-")
      raise Failure, "native cast execution sequence changed" unless executions.map { |row| row.fetch("query") } == queries
      raise Failure, "unexpected separately dispatched reference cast" if events.any? { |row| row["kind"] == "call" && row["name"] == "policy_action_to_decision_type" }

      snapshots = events.each_index.select { |index| events.fetch(index).values_at("kind", "schema", "name") == ["call", "ingestion_observation", "snapshot"] }
      raise Failure, "native snapshot inventory changed" unless snapshots.length == 9

      positions = mode == "helpers-first" ? [snapshots.fetch(0) - 1] : []
      positions += [snapshots.fetch(1) - 1, snapshots.fetch(4) - 1] unless name.start_with?("release-regex-error-")
      raise Failure, "cast execution did not finish after its policy work" unless executions.map { |row| events.index(row) } == positions

      plans = records.filter_map do |row|
        message = row.fetch("message")
        next unless message.match?(/\Aduration: .* plan:\n/)

        plan = JSON.parse(message.split("plan:\n", 2).fetch(1), object_class: IngestionMetadata::UniqueObject, allow_duplicate_key: false)
        query = plan.fetch("Query Text")
        next unless query.include?("policy_helpers:") || query.include?("INSERT INTO search_filter_decision")

        { "record" => row, "explain" => plan }
      end
      raise Failure, "completed plans do not match native execution sequence" unless plans.map { |row| row.fetch("explain").fetch("Query Text") } == queries
      qualified = executions.zip(plans).to_h do |event, plan|
        raise Failure, "wrong native backend or null query descriptor" unless event.fetch("backend") == backend && event.fetch("descriptor").match?(/\A0x[1-9a-f][0-9a-f]*\z/)

        control = event.fetch("query") == @helpers
        raise Failure, "wrong in-call compiler setting" unless event.fetch("compiler_setting") == (control ? 0 : 2)
        raise Failure, "wrong server plan severity" unless plan.fetch("record").fetch("error_severity") == "LOG"

        tree = plan.fetch("explain").fetch("Plan")
        if control
          raise Failure, "helper-first control gained an enclosing routine" unless plan.fetch("record")["context"].nil?
          control_plan!(tree)
        else
          raise Failure, "cast plan not bound to exact frozen ingestion callsite" unless plan.fetch("record").fetch("context") == @context
          decision_plan!(tree, name == "populated-all-fields" ? 11 : 1)
        end
        [event.fetch("native_line"), { "kind" => "inlined_cast", "schema" => "public",
          "name" => "policy_action_to_decision_type", "language" => "sql_inlined",
          "compiler_setting" => event.fetch("compiler_setting"), "native_line" => event.fetch("native_line"),
          "native_execution" => event, "executed_plan" => plan }]
      end
      combined = events.map { |row| row.fetch("kind") == "executor_end" ? qualified.fetch(row.fetch("native_line")) : row }
      phases = NativePolicyCastPhases.new.validate!(combined, frames, name:, mode:, variant: "reference")
      { "qualified_casts" => qualified.values, "phases" => phases, "complete_d3" => false }
    end

    private

    def expression(variable)
      "CASE #{variable} " + BRANCHES.map { |from, to| "WHEN '#{from}'::policy_action THEN '#{to}'::decision_type" }.join(" ") + " ELSE NULL::decision_type END"
    end

    def control_plan!(tree)
      raise Failure, "helper control was not executed once" unless tree.values_at("Node Type", "Actual Rows", "Actual Loops") == ["Result", 1, 1]

      children = tree.fetch("Plans")
      raise Failure, "helper aggregate inventory changed" unless children.length == 1

      aggregate = children.fetch(0)
      expected = "json_agg(#{expression('(x.v)::policy_action')} ORDER BY x.n)"
      raise Failure, "helper cast expression or actual aggregate changed" unless aggregate.values_at("Node Type", "Parent Relationship", "Actual Rows", "Actual Loops", "Output") == ["Aggregate", "InitPlan", 1, 1, [expected]]
      scans = aggregate.fetch("Plans")
      raise Failure, "helper input scan inventory changed" unless scans.length == 1

      expected_input = "unnest('{drop_canonical,drop_source,downrank,flag,require,prefer}'::text[])"
      raise Failure, "helper known-answer inputs were not executed" unless scans.fetch(0).values_at("Node Type", "Function Name", "Schema", "Actual Rows", "Actual Loops", "Function Call") == ["Function Scan", "unnest", "pg_catalog", 6, 1, expected_input]
    end

    def decision_plan!(tree, rows)
      raise Failure, "decision INSERT was not executed once" unless tree.values_at("Node Type", "Operation", "Relation Name", "Schema", "Actual Rows", "Actual Loops") == ["ModifyTable", "Insert", "search_filter_decision", "public", 0, 1]

      scans = tree.fetch("Plans")
      raise Failure, "decision input scan inventory changed" unless scans.length == 1

      scan = scans.fetch(0)
      raise Failure, "policy match rows or scan changed" unless scan.values_at("Node Type", "Relation Name", "Schema", "Actual Rows", "Actual Loops") == ["Seq Scan", "tmp_policy_matches", "pg_temp", rows, 1]
      outputs = scan.fetch("Output")
      raise Failure, "decision output no longer executes exact frozen cast" unless outputs.fetch(7) == expression("tmp_policy_matches.action") && outputs.length == 10 &&
        outputs.none? { |output| output.include?("policy_action_to_decision_type(") }
    end
  end
end
