# frozen_string_literal: true

require "digest"
require_relative "support"
require_relative "final_sql"

module RevaerDatabaseRebaseline
  # A ledger for these frozen bytes, not a general PL/pgSQL parser.
  class NativeTrustRankSource
    MIGRATION = "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql"
    PINS = {
      MIGRATION => "621c1421a6cab3731e4e6939c2687942a3bf343ce578f87cbc62cd2deb571930",
      "scripts/database_rebaseline/final_sql.rb" => "3310e98ad17292b72d35c605bb6cd234fdd582140ae146ba3ca6afd80cd4cd91",
      "scripts/tests/database-ingestion-helper-first.sql" => "75389464e9eacb6cb4731adc2d555e774b265e074f60b3305c41e1b458dc73fc"
    }.freeze
    LOCALS = %w[instance_trust_tier_key instance_trust_rank trust_bucket].freeze
    attr_reader :signature, :bodies

    def initialize(files)
      PINS.each do |path, digest|
        raise Failure, "K1 frozen source changed: #{path}" unless Digest::SHA256.hexdigest(files.fetch(path)) == digest
      end
      definition = files.fetch(MIGRATION).split("CREATE OR REPLACE FUNCTION search_result_ingest_v1(", 2).fetch(1)
      parameters, rest = definition.split(")\nRETURNS TABLE(", 2)
      outputs, rest = rest.split(")\nLANGUAGE plpgsql\nAS $$", 2)
      reference = rest.split("$$;", 2).first
      types = parameters.lines.map(&:strip).reject(&:empty?).map do |line|
        type = line.delete_suffix(",").split(" ", 2).last.downcase.gsub(/\([^)]*\)/, "")
        type.sub(/\Avarchar/, "character varying").sub(/\Achar\b/, "character").sub(/\Atimestamptz\b/, "timestamp with time zone")
      end
      @signature = "search_result_ingest_v1(#{types.join(',')})"
      output_count = outputs.lines.map(&:strip).reject(&:empty?).length
      # Multiple OUT parameters add their composite row datum before FOUND.
      @argument_datums = types.length + output_count + (output_count > 1 ? 1 : 0) + 1
      final = "\n#variable_conflict use_column\n#{FinalSql.new(nil).approved_ingestion_body(reference).delete_prefix("\n")}"
      @bodies = { "reference" => reference, "final" => final }.freeze
      @helper_sql = files.fetch("scripts/tests/database-ingestion-helper-first.sql")
      @models = @bodies.transform_values { |body| model(body) }
    end

    def model(body)
      lines = body.lines
      begin_line = unique_line(lines, "BEGIN")
      declaration = unique_line(lines, "DECLARE")
      # Grammar assigns stmtids after the child statements. In this pinned body,
      # every executable terminator corresponds to one such grammar reduction.
      masked = body.gsub(/--[^\n]*|'(?:[^']|'')*'|"(?:[^"]|"")*"/m) { |token| token.gsub(/[^\n]/, " ") }
      count = 0
      ids = masked.lines.each_with_index.map do |line, index|
        count += line.count(";") if index + 1 >= begin_line
        count
      end
      variables = LOCALS.to_h do |name|
        index = lines.index { |line| line.strip.start_with?("#{name} ") }
        raise Failure, "K1 local declaration missing" unless index

        [name, { "dno" => @argument_datums + index - declaration, "declaration_line" => index + 1 }]
      end
      outer = unique_line(lines, "IF instance_trust_tier_key IS NOT NULL THEN")
      null = unique_line(lines, "IF instance_trust_rank IS NULL THEN")
      bucket = unique_line(lines, "IF instance_trust_rank >= 40 THEN")
      fallback = unique_line(lines, "instance_trust_rank := 0;")
      zero = unique_line(lines, "trust_bucket := 0;")
      d4 = lines.index { |line| line.strip.start_with?("CREATE TEMP TABLE tmp_policy_rules ") }
      raise Failure, "K1 D4 ordering changed" unless d4 && d4 + 1 > zero

      location = ->(line) { [line, ids.fetch(line - 1)] }
      if_location = lambda do |line|
        indent = lines.fetch(line - 1)[/\A */]
        finish = ((line)...lines.length).find { |index| lines.fetch(index).chomp == "#{indent}END IF;" }
        raise Failure, "K1 IF source boundary missing" unless finish

        [line, ids.fetch(finish)]
      end
      { lines:, variables:, defaults: LOCALS.drop(1).to_h { |name| [name, lines.fetch(variables.fetch(name).fetch("declaration_line") - 1).split(":=", 2).last.strip.delete_suffix(";")] },
        block: [begin_line, count], outer: if_location.call(outer), null: if_location.call(null),
        fallback: location.call(fallback), bucket: if_location.call(bucket), zero: location.call(zero), d4: d4 + 1 }
    end

    def expected_events(case_name:, variant:, backend:, oid:)
      raise Failure, "K1 case changed" unless %w[null-instance-trust-key missing-public-trust-tier].include?(case_name)

      model = @models.fetch(variant)
      key_null = case_name == "null-instance-trust-key"
      events = []
      append = lambda do |kind, query, target, location, before, after, answer|
        event = { "kind" => kind, "query" => query, "phase" => "entry", "before" => context(model, variant, backend, oid, location, before) }
        event["target"] = target if target
        events << event
        returned = event.merge("phase" => "return", "after" => context(model, variant, backend, oid, location, after))
        returned["answer"] = answer if kind == "boolean"
        events << returned
      end
      append.call("assignment", model.fetch(:defaults).fetch("instance_trust_rank"), "instance_trust_rank", model.fetch(:block), [true, nil, nil], [true, 0, nil], nil)
      append.call("assignment", model.fetch(:defaults).fetch("trust_bucket"), "trust_bucket", model.fetch(:block), [true, 0, nil], [true, 0, 0], nil)
      query = source_query(model, :outer)
      append.call("boolean", query, nil, model.fetch(:outer), [key_null, 0, 0], [key_null, 0, 0], key_null ? 0 : 1)
      unless key_null
        append.call("boolean", source_query(model, :null), nil, model.fetch(:null), [false, nil, 0], [false, nil, 0], 1)
        append.call("assignment", source_query(model, :fallback), "instance_trust_rank", model.fetch(:fallback), [false, nil, 0], [false, 0, 0], nil)
      end
      model.fetch(:lines).select { |line| line.strip.match?(/\A(?:IF|ELSIF) instance_trust_rank >= \d+ THEN\z/) }.each do |line|
        query = line.strip.sub(/\A(?:IF|ELSIF) /, "").delete_suffix(" THEN")
        threshold = Integer(query.split(">= ", 2).last)
        append.call("boolean", query, nil, model.fetch(:bucket), [key_null, 0, 0], [key_null, 0, 0], 0 >= threshold ? 1 : 0)
      end
      append.call("assignment", source_query(model, :zero), "trust_bucket", model.fetch(:zero), [key_null, 0, 0], [key_null, 0, 0], nil)
      events
    end

    def helper_plan
      direct = @helper_sql.scan(/public\.([a-z_0-9]+)\(/).flatten
      # Constant SQL helpers are folded before PL/pgSQL evaluation. The last
      # derive call normalizes its URI; two non-NULL release tokens invoke text.
      folded, runtime = direct.partition { |name| name == "policy_action_to_decision_type" }
      seen = Hash.new(0)
      folded + runtime.flat_map do |name|
        seen[name] += 1
        nested = if name == "derive_magnet_hash_v1" && seen[name] == 4
                   ["normalize_magnet_uri_v1"]
                 elsif name == "policy_release_group_match_v1" && seen[name] <= 2
                   ["policy_text_match_v1"]
                 else
                   []
                 end
        [name] + nested
      end
    end

    private

    def unique_line(lines, text)
      found = lines.each_index.select { |index| lines.fetch(index).strip == text }
      raise Failure, "K1 source anchor changed: #{text}" unless found.length == 1

      found.first + 1
    end

    def source_query(model, key)
      model.fetch(:lines).fetch(model.fetch(key).first - 1).strip.sub(/\AIF /, "").delete_suffix(" THEN").delete_suffix(";")
    end

    def context(model, variant, backend, oid, location, values)
      key_null, rank, bucket = values
      locals = model.fetch(:variables).transform_values(&:dup)
      locals.fetch("instance_trust_tier_key")["isnull"] = key_null
      { "instance_trust_rank" => rank, "trust_bucket" => bucket }.each do |name, value|
        locals.fetch(name).merge!("isnull" => value.nil?, "value" => value)
      end
      { "backend" => backend, "function_oid" => oid, "signature" => @signature,
        "compiler_setting" => variant == "reference" ? 2 : 0, "resolve_option" => 2,
        "line" => location.first, "statement_id" => location.last, "locals" => locals }
    end
  end
end
