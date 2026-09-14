# frozen_string_literal: true

require_relative "database-ingestion-policy-test"
require_relative "../database_rebaseline/native_policy_phases"
require_relative "../database_rebaseline/native_policy_cast_scope"

module RevaerDatabaseRebaseline
  # In-memory unit inputs use committed policy builders. No capture, server,
  # transaction, native identity or persisted-state qualification is claimed.
  class NativePolicyFixtures
    include IngestionPolicy
    include IngestionPolicyTest

    HELPERS = %w[policy_text_match_v1 policy_uuid_match_v1 policy_int_match_v1
                 policy_release_group_match_v1 policy_action_to_decision_type].freeze
    CASES = {
      "populated-all-fields" => { regex: false, helpers: HELPERS },
      "release-token-match" => { regex: false, helpers: %w[policy_text_match_v1 policy_release_group_match_v1 policy_action_to_decision_type] },
      "release-token-false-persisted-true" => { regex: false, helpers: %w[policy_text_match_v1 policy_release_group_match_v1 policy_action_to_decision_type] },
      "release-regex-error-token" => { regex: true, helpers: %w[policy_release_group_match_v1 policy_text_match_v1] },
      "release-regex-error-persisted-signal" => { regex: true, helpers: %w[policy_release_group_match_v1 policy_text_match_v1] }
    }.freeze
    INGEST = "search_result_ingest_v1"
    DATABASE = "synthetic_native_policy"
    BACKEND = 4242
    DECISION = <<~SQL.chomp.freeze
      INSERT INTO search_filter_decision (
          search_request_id, policy_rule_public_id, policy_snapshot_id, observation_id,
          canonical_torrent_id, canonical_torrent_source_id, decision, decided_at
      )
      SELECT request_id, policy_rule_public_id, request_snapshot_id, observation_id_value,
             canonical_id, source_id, action::decision_type, now()
      FROM tmp_policy_matches
    SQL
    ROUTINE = { "signature" => "search_result_ingest_v1(uuid)", "source" => "\nBEGIN\n  #{DECISION};\nEND;\n" }.freeze
    CONTEXT = "SQL statement \"#{DECISION}\"\nPL/pgSQL function search_result_ingest_v1(uuid) line 3 at SQL statement"
    CAST_BRANCHES = "WHEN 'drop_canonical'::policy_action THEN 'drop_canonical'::decision_type " \
      "WHEN 'drop_source'::policy_action THEN 'drop_source'::decision_type " \
      "WHEN 'downrank'::policy_action THEN 'downrank'::decision_type " \
      "WHEN 'flag'::policy_action THEN 'flag'::decision_type " \
      "WHEN 'require'::policy_action THEN 'flag'::decision_type " \
      "WHEN 'prefer'::policy_action THEN 'flag'::decision_type ELSE NULL::decision_type END"
    CAST_SOURCE = " SELECT CASE $1 #{CAST_BRANCHES}; "

    def call(name, setting = 0, schema = "public")
      { "kind" => "call", "schema" => schema, "name" => name, "compiler_setting" => setting }
    end

    def snapshot
      call("snapshot", 0, "ingestion_observation")
    end

    def phase(name, variant, mode, inlined: false)
      spec = CASES.fetch(name)
      setting = { "reference" => 2, "final" => 0 }.fetch(variant)
      states = if spec.fetch(:regex)
                 %w[2201B 2201B 2201B]
               else
                 { "reference" => %w[00000 00000 42P07], "final" => %w[00000 00000 00000] }.fetch(variant)
               end
      events = mode == "helpers-first" ? HELPERS.map { |helper| call(helper) } : []
      states.each do |state|
        events.push(snapshot, call(INGEST, setting))
        # Upstream regex work is separate from the nested policy-error operator.
        if spec.fetch(:regex)
          events << { "kind" => "regex", "name" => "textregexeq", "compiler_setting" => setting }
          events << { "kind" => "regex", "name" => "texticregexeq", "compiler_setting" => setting }
        end
        events.concat(spec.fetch(:helpers).map { |helper| call(helper, setting) }) unless state == "42P07"
        events << { "kind" => "regex", "name" => "texticregexeq", "compiler_setting" => setting } if spec.fetch(:regex)
        events.push(snapshot, snapshot)
      end
      if inlined
        events.each do |event|
          event.merge!("kind" => "inlined_cast", "language" => "sql_inlined") if event["name"] == "policy_action_to_decision_type"
        end
      end
      frames = states.each_with_index.map do |state, index|
        policy_test_frame(policy_test_tables, "2026-09-11T01:0#{index}:00+00:00").merge("state" => state, "backend" => BACKEND.to_s)
      end
      { events:, frames:, request: { name:, variant:, mode: } }
    end

    def indices(fixture, name)
      fixture.fetch(:events).each_index.select { |index| fixture.fetch(:events).fetch(index)["name"] == name }
    end

    def move(events, from, to)
      event = events.delete_at(from)
      events.insert(to, event)
    end

    def error_operator_indices(fixture)
      events = fixture.fetch(:events)
      events.each_index.select do |index|
        index.positive? && events.fetch(index).fetch("kind") == "regex" &&
          events.fetch(index - 1).values_at("kind", "schema", "name") == ["call", "public", "policy_text_match_v1"]
      end
    end

    def scope_validator(**overrides)
      NativePolicyCastScope.new(**{ routine: ROUTINE, helper_query: policy_helpers_sql, cast_source: CAST_SOURCE, database: DATABASE }.merge(overrides))
    end

    def scope(name, mode, database: DATABASE)
      fixture = phase(name, "reference", mode)
      records = []
      events = fixture.fetch(:events).each_with_index.map do |event, index|
        if event["name"] == "policy_action_to_decision_type"
          control = event.fetch("compiler_setting").zero?
          query = control ? policy_helpers_sql.strip : DECISION
          plan = control ? control_plan : decision_plan(name == "populated-all-fields" ? 11 : 1)
          record = { "pid" => BACKEND, "dbname" => database, "user" => "postgres", "line_num" => records.length + 1,
                     "error_severity" => "LOG", "message" => "duration: 0.001 ms plan:\n#{JSON.generate('Query Text' => query, 'Plan' => plan)}" }
          record["context"] = CONTEXT unless control
          records << record
          event = { "kind" => "executor_end", "query" => query, "backend" => BACKEND,
                    "descriptor" => "0xabc", "compiler_setting" => event.fetch("compiler_setting") }
        end
        event.merge("native_line" => index + 1)
      end
      { events:, records:, evidence: { "frames" => fixture.fetch(:frames) }, name:, mode:,
        catalog: { "language" => "sql", "volatility" => "i", "security_definer" => false, "config" => nil,
                   "castmethod" => "f", "castcontext" => "a", "source" => CAST_SOURCE, "castfunc" => 1234 } }
    end

    private

    def control_plan
      scan = { "Node Type" => "Function Scan", "Function Name" => "unnest", "Schema" => "pg_catalog",
               "Actual Rows" => 6, "Actual Loops" => 1,
               "Function Call" => "unnest('{drop_canonical,drop_source,downrank,flag,require,prefer}'::text[])" }
      aggregate = { "Node Type" => "Aggregate", "Parent Relationship" => "InitPlan", "Actual Rows" => 1, "Actual Loops" => 1,
                    "Output" => ["json_agg(CASE (x.v)::policy_action #{CAST_BRANCHES} ORDER BY x.n)"], "Plans" => [scan] }
      { "Node Type" => "Result", "Actual Rows" => 1, "Actual Loops" => 1, "Plans" => [aggregate] }
    end

    def decision_plan(rows)
      outputs = ["nextval('search_filter_decision_id_seq'::regclass)", "596001", "tmp_policy_matches.policy_rule_public_id", "596001",
                 "2", "1", "1", "CASE tmp_policy_matches.action #{CAST_BRANCHES}", "NULL::text", "now()"]
      scan = { "Node Type" => "Seq Scan", "Relation Name" => "tmp_policy_matches", "Schema" => "pg_temp",
               "Actual Rows" => rows, "Actual Loops" => 1, "Output" => outputs }
      { "Node Type" => "ModifyTable", "Operation" => "Insert", "Relation Name" => "search_filter_decision", "Schema" => "public",
        "Actual Rows" => 0, "Actual Loops" => 1, "Plans" => [scan] }
    end
  end

  class NativePolicyTest
    def initialize
      @fixtures = NativePolicyFixtures.new
      @scope = @fixtures.scope_validator
      @scope_base = @fixtures.scope("populated-all-fields", "helpers-first")
      @assertions = 0
      @mutations = 0
    end

    def run!
      [NativePolicyPhases.new, NativePolicyCastPhases.new].each do |validator|
        @phase_validator = validator
        @inlined = validator.is_a?(NativePolicyCastPhases)
        phase_tests!
        control_regex_tests!
      end
      context_tests!
      scope_identity_tests!
      scope_sequence_tests!
      scope_plan_tests!
      scope_constructor_tests!
      puts "database-native-policy-test: #{@assertions} assertions passed (#{@mutations} rejected mutations); 20 synthetic contexts (12 dispatch + 8 inlined cast); complete_d3=false"
    end

    private

    def copy(value)
      Marshal.load(Marshal.dump(value))
    end

    def assert(value, label)
      raise Failure, "test failed: #{label}" unless value

      @assertions += 1
    end

    def rejected(label, error_class: Failure)
      yield
    rescue error_class
      @assertions += 1
      @mutations += 1
    else
      raise Failure, "mutation accepted: #{label}"
    end

    def phase(name, variant, mode)
      @fixtures.phase(name, variant, mode, inlined: @inlined)
    end

    def phase_check(label, original, reject = true)
      fixture = copy(original)
      yield fixture if block_given?
      raise Failure, "ineffective mutation: #{label}" if reject && fixture == original

      validate = lambda { @phase_validator.validate!(fixture.fetch(:events), fixture.fetch(:frames), **fixture.fetch(:request)) }
      if reject
        rejected("#{@phase_validator.class}: #{label}", &validate)
      else
        phases = validate.call
        assert(phases.map { |item| item.fetch(:state) } == fixture.fetch(:frames).map { |frame| frame.fetch("state") }, label)
        assert(phases.map { |item| item.fetch(:ordinal) } == [0, 1, 2], "three ordered intervals: #{label}")
      end
    end

    def phase_tests!
      NativePolicyFixtures::CASES.each_key do |name|
        %w[reference final].product(%w[cold helpers-first]).each do |variant, mode|
          phase_check("#{name}/#{variant}/#{mode}", phase(name, variant, mode), false)
        end
      end
      full = phase("populated-all-fields", "final", "cold")
      ingest = NativePolicyFixtures::INGEST
      helpers = NativePolicyFixtures::HELPERS
      indices = @fixtures.method(:indices)
      call = @fixtures.method(:call)
      move = @fixtures.method(:move)
      operators = @fixtures.method(:error_operator_indices)
      # Ported structural mutations exercise every phase without expanding the
      # full case cross-product. Mutation construction is outside rejection rescue.
      3.times do |ordinal|
        phase_check("phase#{ordinal}: delete ingestion", full) { |f| f[:events].delete_at(indices.call(f, ingest).fetch(ordinal)) }
        phase_check("phase#{ordinal}: duplicate ingestion", full) { |f| f[:events].insert(indices.call(f, ingest).fetch(ordinal), call.call(ingest)) }
        phase_check("phase#{ordinal}: entry setting", full) { |f| f[:events][indices.call(f, ingest).fetch(ordinal)]["compiler_setting"] = 2 }
        3.times do |boundary|
          position = ordinal * 3 + boundary
          phase_check("snapshot#{position}: delete", full) { |f| f[:events].delete_at(indices.call(f, "snapshot").fetch(position)) }
          phase_check("snapshot#{position}: setting", full) { |f| f[:events][indices.call(f, "snapshot").fetch(position)]["compiler_setting"] = 2 }
        end
        phase_check("phase#{ordinal}: before snapshot past entry", full) do |f|
          move.call(f[:events], indices.call(f, "snapshot").fetch(ordinal * 3), indices.call(f, ingest).fetch(ordinal))
        end
        helpers.each do |helper|
          phase_check("phase#{ordinal}: missing #{helper}", full) { |f| f[:events].delete_at(indices.call(f, helper).fetch(ordinal)) }
          phase_check("phase#{ordinal}: #{helper} setting", full) { |f| f[:events][indices.call(f, helper).fetch(ordinal)]["compiler_setting"] = 2 }
        end
      end
      phase_check("duplicate snapshot", full) { |f| f[:events] << @fixtures.snapshot }
      phase_check("finish past next entry", full) { |f| move.call(f[:events], indices.call(f, "snapshot").fetch(2), indices.call(f, ingest).fetch(1)) }
      phase_check("next before snapshot ahead of prior entry", full) { |f| move.call(f[:events], indices.call(f, "snapshot").fetch(3), indices.call(f, ingest).first) }
      helpers.each do |helper|
        phase_check("cold: prepend #{helper}", full) { |f| f[:events].unshift(call.call(helper)) }
        warm = phase("populated-all-fields", "reference", "helpers-first")
        phase_check("helpers-first: missing #{helper}", warm) { |f| f[:events].delete_at(indices.call(f, helper).first) }
        phase_check("helpers-first: #{helper} setting", warm) { |f| f[:events][indices.call(f, helper).first]["compiler_setting"] = 2 }
      end
      phase_check("unexpected nonpolicy control", phase("release-token-match", "final", "helpers-first")) { |f| f[:events].unshift(call.call("normalize_title_v1")) }
      %w[reference final].each do |variant|
        regex = phase("release-regex-error-token", variant, "cold")
        wrong_setting = variant == "reference" ? 0 : 2
        3.times do |ordinal|
          phase_check("#{variant} regex#{ordinal}: setting", regex) { |f| f[:events][operators.call(f).fetch(ordinal)]["compiler_setting"] = wrong_setting }
          phase_check("#{variant} regex#{ordinal}: operator", regex) { |f| f[:events][operators.call(f).fetch(ordinal)]["name"] = "textregexeq" }
          phase_check("#{variant} regex#{ordinal}: missing", regex) { |f| f[:events].delete_at(operators.call(f).fetch(ordinal)) }
          phase_check("#{variant} regex#{ordinal}: duplicate", regex) do |f|
            index = operators.call(f).fetch(ordinal)
            f[:events].insert(index, f[:events].fetch(index).dup)
          end
          phase_check("#{variant} regex#{ordinal}: before helper", regex) do |f|
            index = operators.call(f).fetch(ordinal)
            move.call(f[:events], index, index - 1)
          end
          phase_check("#{variant} regex#{ordinal}: success", regex) { |f| f[:frames][ordinal]["state"] = "00000" }
          phase_check("#{variant} upstream regex#{ordinal}: setting", regex) { |f| f[:events][indices.call(f, ingest).fetch(ordinal) + 1]["compiler_setting"] = wrong_setting }
          phase_check("#{variant} upstream regex#{ordinal}: operator", regex) { |f| f[:events][indices.call(f, ingest).fetch(ordinal) + 1]["name"] = "unknown_regex" }
        end
        success = phase("release-token-match", variant, "cold")
        phase_check("#{variant}: inverted third outcome", success) { |f| f[:frames][2]["state"] = variant == "reference" ? "00000" : "42P07" }
        phase_check("#{variant}: premature second D4", success) { |f| f[:frames][1]["state"] = "42P07" }
      end
      d4 = phase("release-token-match", "reference", "cold")
      helpers.each do |helper|
        phase_check("D4: downstream #{helper}", d4) { |f| f[:events].insert(indices.call(f, ingest).fetch(2) + 1, call.call(helper, 2)) }
      end
      phase_check("missing frame", full) { |f| f[:frames].pop }
      phase_check("extra frame", full) { |f| f[:frames] << { "state" => "00000" } }
      phase_check("unknown helper", full) { |f| f[:events].insert(indices.call(f, ingest).first + 1, call.call("unknown_helper_v1")) }
      phase_check("wrong schema", full) { |f| f[:events][indices.call(f, helpers.first).first]["schema"] = "other" }
      phase_check("unexpected success regex", full) do |f|
        f[:events].insert(indices.call(f, ingest).first + 1, { "kind" => "regex", "name" => "texticregexeq", "compiler_setting" => 0 })
      end
      { "before first snapshot" => ->(_f) { 0 },
        "after first after-snapshot" => ->(f) { indices.call(f, "snapshot").fetch(1) + 1 },
        "after first finish" => ->(f) { indices.call(f, "snapshot").fetch(2) + 1 },
        "after second before-snapshot" => ->(f) { indices.call(f, "snapshot").fetch(3) + 1 },
        "after third finish" => ->(f) { f[:events].length } }.each do |position, at|
        [call.call("policy_text_match_v1", 2), call.call("normalize_title_v1", 2), call.call("unknown_helper_v1"),
         { "kind" => "regex", "name" => "texticregexeq", "compiler_setting" => 2 }].each do |event|
          phase_check("outside #{position}: #{event.fetch('name')}", full) { |f| f[:events].insert(at.call(f), event.dup) }
        end
      end
      { name: "unknown-case", variant: "unknown-variant", mode: "unknown-mode" }.each do |key, value|
        phase_check("unknown request #{key}", full) { |f| f[:request][key] = value }
      end
      phase_check("unknown native kind", full) { |f| f[:events].first["kind"] = "unknown" }
      return unless @inlined

      { "schema" => "other", "name" => "other_cast", "language" => "sql" }.each do |key, value|
        phase_check("inlined identity #{key}", full) { |f| f[:events][indices.call(f, "policy_action_to_decision_type").first][key] = value }
      end
    end

    def control_regex_tests!
      %w[reference final].each do |variant|
        %w[textregexeq texticregexeq].each do |operator|
          fixture = phase("release-regex-error-token", variant, "helpers-first")
          fixture.fetch(:events).insert(1, { "kind" => "regex", "name" => operator, "compiler_setting" => 0 })
          phase_check("#{variant} helper-first #{operator}", fixture, false)
          phase_check("control regex setting", fixture) { |f| f[:events][1]["compiler_setting"] = 2 }
          phase_check("control regex operator", fixture) { |f| f[:events][1]["name"] = "unknown_regex" }
          phase_check("control regex predecessor", fixture) { |f| @fixtures.move(f[:events], 1, 0) }
        end
      end
    end

    def context_tests!
      counts = Hash.new(0)
      cast_settings = []
      NativePolicyFixtures::CASES.each do |name, spec|
        %w[reference final].product(%w[cold helpers-first]).each do |variant, mode|
          fixture = @fixtures.phase(name, variant, mode)
          if variant == "reference" && (!spec.fetch(:regex) || mode == "helpers-first")
            data = @fixtures.scope(name, mode)
            before = copy(data)
            result = @scope.validate!(**data)
            casts = result.fetch("qualified_casts")
            expected = (mode == "helpers-first" ? [0] : []) + (spec.fetch(:regex) ? [] : [2, 2])
            assert(casts.map { |cast| cast.fetch("compiler_setting") } == expected, "exact inlined settings #{name}/#{mode}")
            assert(!result.fetch("complete_d3") && data == before, "no D3 claim or input mutation")
            casts.each do |cast|
              execution = cast.fetch("native_execution")
              record = cast.fetch("executed_plan").fetch("record")
              assert(data.fetch(:events).include?(execution) && data.fetch(:records).include?(record), "retained exact execution and plan")
              assert(cast.values_at("kind", "schema", "name", "language", "native_line") ==
                ["inlined_cast", "public", "policy_action_to_decision_type", "sql_inlined", execution.fetch("native_line")], "qualified cast identity")
            end
            phases = result.fetch("phases")
            cast_settings.concat(expected)
            counts[:cast] += 1
          else
            phases = NativePolicyPhases.new.validate!(fixture.fetch(:events), fixture.fetch(:frames), **fixture.fetch(:request))
            counts[:dispatch] += 1
          end
          assert(phases.map { |item| item.fetch(:state) } == fixture.fetch(:frames).map { |frame| frame.fetch("state") }, "context outcomes #{name}/#{variant}/#{mode}")
          assert(phases.map { |item| item.fetch(:ordinal) } == [0, 1, 2], "context phase order")
          phases.each do |item|
            assert(item.fetch(:entry) < item.fetch(:snapshot_after) && item.fetch(:helpers).first.fetch("name") == NativePolicyFixtures::INGEST, "exact root interval")
          end
        end
      end
      assert(counts == { dispatch: 12, cast: 8 }, "twenty bounded contexts")
      assert(cast_settings.count(0) == 5 && cast_settings.count(2) == 12, "five control and twelve in-call cast executions")
    end

    def mutate_scope(label, original: @scope_base, error_class: Failure)
      data = copy(original)
      yield data
      raise Failure, "ineffective mutation: #{label}" if data == original

      rejected(label, error_class:) { @scope.validate!(**data) }
    end

    def executions(data)
      data.fetch(:events).select { |event| event.fetch("kind") == "executor_end" }
    end

    def renumber_events(data)
      data.fetch(:events).each_with_index { |event, index| event["native_line"] = index + 1 }
    end

    def renumber_records(data)
      data.fetch(:records).each_with_index { |record, index| record["line_num"] = index + 1 }
    end

    def edit_plan(data, index)
      record = data.fetch(:records).fetch(index)
      plan = JSON.parse(record.fetch("message").split("plan:\n", 2).fetch(1))
      yield plan
      record["message"] = "duration: 0.001 ms plan:\n#{JSON.generate(plan)}"
    end

    def scope_identity_tests!
      { "language" => "plpgsql", "volatility" => "v", "security_definer" => true,
        "config" => [], "castmethod" => "b", "castcontext" => "i", "source" => "changed" }.each do |key, value|
        mutate_scope("catalog #{key}") { |data| data.fetch(:catalog)[key] = value }
      end
      [0, -1, "1234", nil].each do |value|
        mutate_scope("cast OID #{value.inspect}") { |data| data.fetch(:catalog)["castfunc"] = value }
      end
      ["0", "01", "+4242", "4242x", ""].each do |backend|
        mutate_scope("invalid application backend #{backend.inspect}") { |data| data.fetch(:evidence).fetch("frames").each { |frame| frame["backend"] = backend } }
      end
      mutate_scope("mixed application backend") { |data| data.fetch(:evidence).fetch("frames").last["backend"] = "4243" }
      { "pid" => 4243, "dbname" => "ingestion_policy_reference", "user" => "other_user" }.each do |key, value|
        mutate_scope("server provenance #{key}") { |data| data.fetch(:records).first[key] = value }
      end
      mutate_scope("string server PID") { |data| data.fetch(:records).first["pid"] = "4242" }
      %i[records events].each do |key|
        field = key == :records ? "line_num" : "native_line"
        [0, -1, "1"].each { |value| mutate_scope("#{key} invalid line #{value.inspect}") { |data| data.fetch(key).first[field] = value } }
        mutate_scope("#{key} duplicate line") { |data| data.fetch(key)[1][field] = data.fetch(key).first.fetch(field) }
        mutate_scope("#{key} unordered lines") { |data| data.fetch(key)[0, 2] = data.fetch(key).first(2).reverse }
      end
      3.times do |index|
        mutate_scope("native backend #{index}") { |data| executions(data).fetch(index)["backend"] = 4243 }
        mutate_scope("native backend type #{index}") { |data| executions(data).fetch(index)["backend"] = "4242" }
        mutate_scope("executor compiler setting #{index}") { |data| executions(data).fetch(index)["compiler_setting"] = index.zero? ? 2 : 0 }
        mutate_scope("server severity #{index}") { |data| data.fetch(:records).fetch(index)["error_severity"] = "NOTICE" }
      end
      ["0x0", "0x01", "0xA", "0xgg", "", "abc"].each do |descriptor|
        mutate_scope("native query descriptor #{descriptor.inspect}") { |data| executions(data).first["descriptor"] = descriptor }
      end
      mutate_scope("unknown raw event") { |data| data.fetch(:events).first["kind"] = "unknown" }
      mutate_scope("prequalified raw cast") { |data| executions(data).first["kind"] = "inlined_cast" }
      mutate_scope("separately dispatched reference cast") do |data|
        data.fetch(:events).unshift(@fixtures.call("policy_action_to_decision_type"))
        renumber_events(data)
      end
    end

    def scope_sequence_tests!
      mutate_scope("unknown cast name") { |data| data[:name] = "unknown" }
      mutate_scope("unknown cast mode") { |data| data[:mode] = "unknown" }
      error = @fixtures.scope("release-regex-error-token", "helpers-first")
      mutate_scope("cold error is dispatch-only", original: error) { |data| data[:mode] = "cold" }
      3.times do |index|
        mutate_scope("missing native execution #{index}") { |data| data.fetch(:events).delete(executions(data).fetch(index)); renumber_events(data) }
        mutate_scope("missing completed plan #{index}") { |data| data.fetch(:records).delete_at(index); renumber_records(data) }
        mutate_scope("changed native query #{index}") { |data| executions(data).fetch(index)["query"] += " " }
        mutate_scope("changed completed query #{index}") { |data| edit_plan(data, index) { |plan| plan["Query Text"] += " " } }
        mutate_scope("execution moved past snapshot #{index}") do |data|
          position = data.fetch(:events).index(executions(data).fetch(index))
          @fixtures.move(data.fetch(:events), position, position + 1)
          renumber_events(data)
        end
      end
      mutate_scope("duplicate native execution") do |data|
        data.fetch(:events) << executions(data).last.dup
        renumber_events(data)
      end
      mutate_scope("duplicate completed plan") { |data| data.fetch(:records) << data.fetch(:records).last.dup; renumber_records(data) }
      mutate_scope("reordered completed plans") { |data| data.fetch(:records).reverse!; renumber_records(data) }
      mutate_scope("jointly changed query") do |data|
        executions(data).fetch(1)["query"] += " "
        edit_plan(data, 1) { |plan| plan["Query Text"] += " " }
      end
      mutate_scope("missing native snapshot") do |data|
        data.fetch(:events).delete_at(@fixtures.indices(data, "snapshot").first)
        renumber_events(data)
      end
      mutate_scope("extra native snapshot") { |data| data.fetch(:events) << @fixtures.snapshot; renumber_events(data) }
      mutate_scope("downstream helper after completed cast") do |data|
        index = data.fetch(:events).index(executions(data).fetch(1))
        data.fetch(:events).insert(index + 1, @fixtures.call("policy_text_match_v1", 2))
        renumber_events(data)
      end
      ["state", "compiler_setting"].each do |field|
        mutate_scope("cast scope preserves phase #{field}") do |data|
          if field == "state"
            data.fetch(:evidence).fetch("frames").last[field] = "00000"
          else
            index = @fixtures.indices(data, "policy_text_match_v1").fetch(1)
            data.fetch(:events).fetch(index)[field] = 0
          end
        end
      end
      mutate_scope("regex scope preserves nested operator", original: error) do |data|
        index = @fixtures.error_operator_indices(data).first
        data.fetch(:events).fetch(index)["name"] = "textregexeq"
      end
      ["non-plan record", "unrelated plan"].each do |label|
        data = copy(@scope_base)
        record = data.fetch(:records).first.dup
        record["message"] = label == "non-plan record" ? "synthetic unrelated log" : "duration: 0.001 ms plan:\n#{JSON.generate('Query Text' => 'SELECT 1', 'Plan' => {})}"
        data.fetch(:records).unshift(record)
        renumber_records(data)
        assert(@scope.validate!(**data).fetch("qualified_casts").length == 3, "retain but do not count #{label}")
        mutate_scope("#{label} provenance remains exact", original: data) { |changed| changed.fetch(:records).first["dbname"] = "other_database" }
      end
      mutate_scope("missing plan prefix") { |data| data.fetch(:records).first["message"] = data.fetch(:records).first.fetch("message").sub("duration:", "other:") }
      { "duplicate top-level key" => ->(message) { message.sub('"Query Text":', '"Plan":{},"Query Text":') },
        "duplicate nested key" => ->(message) { message.sub('"Node Type":"Result"', '"Node Type":"Result","Node Type":"Result"') },
        "escaped duplicate key" => ->(message) { message.sub('"Node Type":"Result"', '"Node Type":"Result","Node\\u0020Type":"Result"') },
        "malformed plan JSON" => ->(_message) { "duration: 0.001 ms plan:\n{" } }.each do |label, change|
        mutate_scope(label, error_class: JSON::ParserError) { |data| data.fetch(:records).first["message"] = change.call(data.fetch(:records).first.fetch("message")) }
      end
    end

    def scope_plan_tests!
      mutate_scope("helper control acquired context") { |data| data.fetch(:records).first["context"] = NativePolicyFixtures::CONTEXT }
      [nil, "", NativePolicyFixtures::CONTEXT.sub("line 3", "line 4"),
       NativePolicyFixtures::CONTEXT.sub("(uuid)", "(text)"), NativePolicyFixtures::CONTEXT.sub("SELECT request_id", "SELECT  request_id")].each do |context|
        mutate_scope("exact frozen source callsite #{context.inspect}") { |data| data.fetch(:records).fetch(1)["context"] = context }
      end
      { "Node Type" => "ProjectSet", "Actual Rows" => 0, "Actual Loops" => 0 }.each do |key, value|
        mutate_scope("control root #{key}") { |data| edit_plan(data, 0) { |plan| plan.fetch("Plan")[key] = value } }
      end
      { "Node Type" => "Result", "Parent Relationship" => "SubPlan", "Actual Rows" => 0, "Actual Loops" => 2,
        "Output" => ["json_agg(NULL::decision_type)"] }.each do |key, value|
        mutate_scope("control aggregate #{key}") { |data| edit_plan(data, 0) { |plan| plan.fetch("Plan").fetch("Plans").first[key] = value } }
      end
      { "Node Type" => "Seq Scan", "Function Name" => "other", "Schema" => "public", "Actual Rows" => 5, "Actual Loops" => 0,
        "Function Call" => "unnest('{drop_canonical,drop_source,downrank,flag,prefer,require}'::text[])" }.each do |key, value|
        mutate_scope("control inputs #{key}") { |data| edit_plan(data, 0) { |plan| plan.fetch("Plan").fetch("Plans").first.fetch("Plans").first[key] = value } }
      end
      { "Node Type" => "Result", "Operation" => "Update", "Relation Name" => "other", "Schema" => "private",
        "Actual Rows" => 1, "Actual Loops" => 0 }.each do |key, value|
        mutate_scope("decision root #{key}") { |data| edit_plan(data, 1) { |plan| plan.fetch("Plan")[key] = value } }
      end
      { "Node Type" => "Index Scan", "Relation Name" => "other", "Schema" => "public", "Actual Rows" => 1, "Actual Loops" => 2 }.each do |key, value|
        mutate_scope("decision match scan #{key}") { |data| edit_plan(data, 1) { |plan| plan.fetch("Plan").fetch("Plans").first[key] = value } }
      end
      single = @fixtures.scope("release-token-match", "helpers-first")
      mutate_scope("single-row case cannot use populated row count", original: single) do |data|
        edit_plan(data, 1) { |plan| plan.fetch("Plan").fetch("Plans").first["Actual Rows"] = 11 }
      end
      [[0, 0], [0, 1], [1, 0]].each do |record, depth|
        [0, 2].each do |count|
          mutate_scope("child inventory #{record}/#{depth}/#{count}") do |data|
            edit_plan(data, record) do |plan|
              tree = plan.fetch("Plan")
              depth.times { tree = tree.fetch("Plans").first }
              child = tree.fetch("Plans").first
              tree["Plans"] = Array.new(count) { copy(child) }
            end
          end
        end
      end
      %w[drop_canonical drop_source downrank flag require prefer].each do |action|
        [0, 1].each do |index|
          mutate_scope("exact cast branch #{action}/#{index}") do |data|
            edit_plan(data, index) do |plan|
              outputs = plan.fetch("Plan").fetch("Plans").first.fetch("Output")
              column = index.zero? ? 0 : 7
              outputs[column] = outputs.fetch(column).sub("WHEN '#{action}'::policy_action THEN", "WHEN '#{action}'::policy_action IS NOT NULL THEN")
            end
          end
        end
      end
      { "NULL default" => ->(output) { output.sub("ELSE NULL::decision_type", "ELSE 'flag'::decision_type") },
        "cast result type" => ->(output) { output.sub("'flag'::decision_type", "'flag'::text") },
        "require mapping" => ->(output) { output.sub("WHEN 'require'::policy_action THEN 'flag'", "WHEN 'require'::policy_action THEN 'drop_canonical'") },
        "prefer mapping" => ->(output) { output.sub("WHEN 'prefer'::policy_action THEN 'flag'", "WHEN 'prefer'::policy_action THEN 'downrank'") },
        "cast input" => ->(output) { output.sub("tmp_policy_matches.action", "other.action") } }.each do |label, change|
        mutate_scope("decision #{label}") do |data|
          edit_plan(data, 1) do |plan|
            outputs = plan.fetch("Plan").fetch("Plans").first.fetch("Output")
            outputs[7] = change.call(outputs.fetch(7))
          end
        end
      end
      mutate_scope("control aggregate input") do |data|
        edit_plan(data, 0) { |plan| plan.fetch("Plan").fetch("Plans").first.fetch("Output")[0].sub!("(x.v)::policy_action", "(x.n)::policy_action") }
      end
      mutate_scope("control aggregate ordering") do |data|
        edit_plan(data, 0) { |plan| plan.fetch("Plan").fetch("Plans").first.fetch("Output")[0].sub!("ORDER BY x.n", "ORDER BY x.v") }
      end
      [9, 11].each do |length|
        mutate_scope("decision output count #{length}") do |data|
          edit_plan(data, 1) do |plan|
            outputs = plan.fetch("Plan").fetch("Plans").first.fetch("Output")
            length == 9 ? outputs.pop : outputs << "NULL::text"
          end
        end
      end
      mutate_scope("cast in wrong output column") do |data|
        edit_plan(data, 1) do |plan|
          outputs = plan.fetch("Plan").fetch("Plans").first.fetch("Output")
          outputs[6], outputs[7] = outputs.fetch(7), outputs.fetch(6)
        end
      end
      mutate_scope("extra separately dispatched expression") do |data|
        edit_plan(data, 1) { |plan| plan.fetch("Plan").fetch("Plans").first.fetch("Output")[0] = "policy_action_to_decision_type(tmp_policy_matches.action)" }
      end
    end

    def scope_constructor_tests!
      ["different_native_policy", "ingestion_policy_reference"].each do |database|
        validator = @fixtures.scope_validator(database:)
        data = @fixtures.scope("populated-all-fields", "helpers-first", database:)
        assert(validator.validate!(**data).fetch("qualified_casts").length == 3, "injected exact database #{database}")
        rejected("old database cannot satisfy injected identity") { validator.validate!(**@scope_base) }
      end
      { helper_query: "SELECT 'policy_helpers:changed'", cast_source: "changed" }.each do |key, value|
        validator = @fixtures.scope_validator(**{ key => value })
        rejected("independently injected #{key}") { validator.validate!(**@scope_base) }
      end
      source = copy(NativePolicyFixtures::ROUTINE)
      source["source"] = "\n#{source.fetch('source')}"
      validator = @fixtures.scope_validator(routine: source)
      rejected("old callsite cannot satisfy shifted source") { validator.validate!(**@scope_base) }
      source = copy(NativePolicyFixtures::ROUTINE)
      source["signature"] = "search_result_ingest_v1(text)"
      validator = @fixtures.scope_validator(routine: source)
      rejected("old callsite cannot satisfy different signature") { validator.validate!(**@scope_base) }
      ["\nBEGIN\nEND;\n", "#{NativePolicyFixtures::ROUTINE.fetch('source')}#{NativePolicyFixtures::ROUTINE.fetch('source')}",
       "BEGIN #{NativePolicyFixtures::DECISION};\nEND;\n"].each do |body|
        rejected("unique statement and source line required") { @fixtures.scope_validator(routine: NativePolicyFixtures::ROUTINE.merge("source" => body)) }
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    raise RevaerDatabaseRebaseline::Failure, "no evidence arguments accepted; synthetic unit tests only" unless ARGV.empty?

    RevaerDatabaseRebaseline::NativePolicyTest.new.run!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-native-policy-test: #{error.message}"
    exit 1
  end
end
