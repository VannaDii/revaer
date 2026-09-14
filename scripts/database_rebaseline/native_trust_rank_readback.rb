# frozen_string_literal: true

require "json"
require_relative "native_trust_rank_source"

# Loaded by the proof owner after FinalProof is defined; do not require it here.

module RevaerDatabaseRebaseline
  # Pure readback adapter. None of FinalProof's bootstrap or execution methods run.
  class NativeTrustRankApplicationOracle < FinalProof
    Root = Data.define(:root)

    def initialize(root:, inventory:)
      @contract = Root.new(root:)
      @ingestion_inventory = inventory
    end

    def verify!(arm, case_name:, mode:, variant:)
      evidence = arm.fetch("application")
      spec = attributes_cases.find { |item| item.fetch(:name) == case_name }
      raise Failure, "K1 canonical case absent" unless spec

      role = evidence.fetch("context").fetch("session")
      owner = @ingestion_inventory.fetch("final_proof").fetch("routines").find { |row| row.fetch("name") == "search_result_ingest_v1" }.fetch("owner")
      raise Failure, "K1 canonical owner changed" unless owner.match?(/\Aproof_owner_[a-f0-9]{16}\z/)
      expected_role = variant == "reference" ? "postgres" : owner.sub("proof_owner_", "proof_runtime_")
      raise Failure, "K1 canonical role changed" unless role == expected_role

      { "stdout" => "stdout", "stderr" => "stderr" }.each do |raw, key|
        raise Failure, "K1 raw application #{raw} changed" unless arm.fetch(raw).eql?(evidence.fetch(key))
        raise Failure, "K1 raw fixture #{raw} changed" unless arm.fetch("fixture_#{raw}").eql?(evidence.fetch("fixture_transport").fetch(key))
      end
      [evidence, evidence.fetch("fixture_transport")].each { |record| metadata_transport_json!(record.fetch("stdout")) }
      session = attributes_session(spec, mode)
      raise Failure, "K1 canonical application SQL changed" unless arm.fetch("sql") == attributes_query(session)
      raise Failure, "K1 canonical fixture SQL changed" unless arm.fetch("fixture_sql") == attributes_query(calls: [attributes_arguments(spec)])

      attributes_validate!(evidence, spec, mode, variant, role)
      { "application" => compilation_comparable("fixture" => evidence.fetch("fixture"), "frames" => evidence.fetch("frames")),
        "inputs" => validation_comparable(evidence, { site: nil }).fetch("inputs") }
    end
  end

  class NativeTrustRankReadback
    CASES = %w[null-instance-trust-key missing-public-trust-tier].freeze
    MODES = %w[cold helpers-first].freeze
    VARIANTS = %w[reference final].freeze
    CATALOG_FIELDS = %w[oid schema name signature language source_sha256 config].freeze

    def initialize(root:, source_commit:, producer_sha256:)
      check(source_commit.is_a?(String) && source_commit.match?(/\A[a-f0-9]{40}\z/), "trusted source commit invalid")
      check(producer_sha256.is_a?(Hash) && producer_sha256.key?("native-trust-rank.gdb"), "trusted producer map invalid")
      check(producer_sha256.all? do |name, digest|
        name.is_a?(String) && name.match?(/\A[a-zA-Z0-9][a-zA-Z0-9._-]*\z/) &&
          digest.is_a?(String) && digest.match?(/\A[a-f0-9]{64}\z/)
      end, "trusted producer identity invalid")
      # These expectations come from the running proof's registry, never its bundle.
      @source_commit = source_commit.dup.freeze
      @producer_sha256 = producer_sha256.to_h { |name, digest| [name.dup.freeze, digest.dup.freeze] }.freeze
      @root = root
      @source = NativeTrustRankSource.new(NativeTrustRankSource::PINS.to_h { |path, _digest| [path, read(File.join(root, path))] })
    end

    def read_run!(directory)
      names = CASES.product(MODES, VARIANTS, %w[plain traced]).map { |parts| parts.join("-") }
      actual = Dir.children(directory).select { |name| File.directory?(File.join(directory, name)) }
      check(actual.sort == names.sort, "run context directory set changed")
      inventories = %w[reference_proof final_proof].to_h { |name| [name, json(read(File.join(directory, "#{name}-inventory.json")))] }
      contexts = CASES.product(MODES, VARIANTS).map do |case_name, mode, variant|
        prefix = [case_name, mode, variant].join("-")
        trace = File.join(directory, "#{prefix}-traced")
        debugger = File.join(directory, "#{prefix}-traced-observed-debugger")
        { "case" => case_name, "mode" => mode, "variant" => variant,
          "plain" => read_arm(File.join(directory, "#{prefix}-plain"), prefix), "observed" => read_arm(trace, prefix),
          "events" => json(read(File.join(trace, "trust-rank-events.json"))),
          "native" => json(read(File.join(trace, "native-events.json"))),
          "catalog" => json(read(File.join(trace, "call-catalog.json"))),
          "triggers" => json(read("#{debugger}.stdout.triggers.json")),
          "trigger_catalog" => json(read("#{debugger}.stdout.trigger-catalog.json")),
          "snapshot_sql" => read(File.join(trace, "observer-#{variant}.sql")),
          "debugger_stdout" => read("#{debugger}.stdout"), "debugger_stderr" => read("#{debugger}.stderr"),
          "debugger_script" => read("#{debugger}.gdb") }
      end
      { "inventory" => inventories, "contexts" => contexts,
        "producers" => @producer_sha256.to_h { |name, _digest| [name, read(File.join(directory, name))] },
        "provenance" => json(read(File.join(directory, "report.json"))).slice("source_commit", "source_before", "source_after") }
    rescue SystemCallError, JSON::ParserError, KeyError, IndexError, TypeError, ArgumentError, NoMethodError => error
      raise Failure, "K1 evidence read failed: #{error.message}"
    end

    def validate!(bundle)
      validate_producers!(bundle)
      inventory = bundle.fetch("inventory")
      validate_inventory!(inventory)
      contexts = bundle.fetch("contexts")
      check(contexts.map { |context| context.values_at("case", "mode", "variant") }.eql?(CASES.product(MODES, VARIANTS)), "context matrix changed")
      oracle = NativeTrustRankApplicationOracle.new(root: @root, inventory:)
      results = contexts.map do |context|
        dimensions = { case_name: context.fetch("case"), mode: context.fetch("mode"), variant: context.fetch("variant") }
        plain = oracle.verify!(context.fetch("plain"), **dimensions)
        observed = oracle.verify!(context.fetch("observed"), **dimensions)
        check(plain.eql?(observed), "complete plain/observed application pair changed")
        %w[plain observed].each do |arm|
          check(context.fetch(arm).fetch("comparable").eql?(arm == "plain" ? plain : observed), "serialized comparable changed")
        end
        validate_context!(context, inventory)
      end
      { "kind" => "native-trust-rank-readback", "source_commit" => @source_commit, "producer_sha256" => @producer_sha256, "contexts" => results,
        "calls" => results.sum { |item| item.fetch("intervals").length }, "events" => results.sum { |item| item.fetch("events") },
        "warning_free" => results.all? { |item| item.fetch("debugger_stderr").empty? },
        "canonical_proof" => false, "d3_complete" => false }
    rescue KeyError, IndexError, TypeError, ArgumentError, NoMethodError => error
      raise Failure, "K1 malformed evidence: #{error.message}"
    end

    private

    def read(path)
      check(File.lstat(path).file?, "evidence must be a regular non-symlink file: #{path}")
      File.binread(path)
    rescue SystemCallError => error
      raise Failure, "K1 cannot read evidence: #{error.message}"
    end

    def json(bytes)
      JSON.parse(bytes, object_class: IngestionMetadata::UniqueObject, allow_duplicate_key: false)
      # The duplicate-detecting setter is only a parse guard; canonical oracle
      # normalization legitimately replaces keys in ordinary in-memory hashes.
      JSON.parse(bytes, allow_duplicate_key: false)
    rescue JSON::ParserError => error
      raise Failure, "K1 invalid or duplicate JSON: #{error.message}"
    end

    def read_arm(directory, prefix)
      { "application" => json(read(File.join(directory, "application.json"))),
        "comparable" => json(read(File.join(directory, "comparable.json"))) }.merge(
          %w[stdout stderr sql fixture_stdout fixture_stderr fixture_sql].to_h do |name|
            suffix = name.start_with?("fixture_") ? "-fixture.#{name.delete_prefix('fixture_')}" : ".#{name}"
            [name, read(File.join(directory, "#{prefix}#{suffix}"))]
          end
        )
    end

    def check(condition, message)
      raise Failure, "K1 #{message}" unless condition
    end

    def validate_inventory!(inventory)
      check(inventory.keys.sort == %w[final_proof reference_proof], "inventory variants changed")
      VARIANTS.each do |variant|
        rows = inventory.fetch("#{variant}_proof").fetch("routines")
        matches = rows.select { |row| row.fetch("name") == "search_result_ingest_v1" }
        check(matches.length == 1, "root catalog identity missing or duplicate")
        row = matches.first
        check(row.fetch("signature") == @source.signature && row.fetch("source") == @source.bodies.fetch(variant), "root source/catalog binding changed")
        expected = variant == "reference" ? ["plpgsql.variable_conflict=use_column"] : ["search_path=pg_catalog, public"]
        check(row.fetch("settings") == expected && row.fetch("definer") == (variant == "final"), "root compiled scope changed")
      end
    end

    def validate_producers!(bundle)
      provenance = bundle.fetch("provenance")
      check(provenance.fetch("source_commit") == @source_commit, "producer source checkpoint changed")
      check(bundle.fetch("producers").keys.sort == @producer_sha256.keys.sort, "retained producer set changed")
      check(provenance.fetch("source_before").is_a?(Hash) && provenance.fetch("source_before").eql?(provenance.fetch("source_after")), "producer source registry changed during run")
      @producer_sha256.each do |name, digest|
        check(Digest::SHA256.hexdigest(bundle.fetch("producers").fetch(name)) == digest, "retained producer bytes changed: #{name}")
        %w[source_before source_after].each do |phase|
          entries = provenance.fetch(phase).select { |path, _hash| File.basename(path) == name }
          check(entries.length == 1 && entries.values == [digest], "retained producer seal changed: #{name}")
        end
      end
      bundle.fetch("contexts").each do |context|
        script = context.fetch("debugger_script")
        probe = bundle.fetch("producers").fetch("native-trust-rank.gdb")
        ready = script.index('printf "READY:%d\\n"')
        check(!probe.empty? && script.scan(probe).length == 1 && ready && script.index(probe) < ready, "retained debugger probe insertion changed")
      end
    end

    def validate_context!(context, inventory)
      stdout = context.fetch("debugger_stdout")
      stderr = context.fetch("debugger_stderr")
      check(![stdout, stderr].any? { |bytes| bytes.match?(/K1_ERROR|Python Exception|Traceback|Error in sourced|Cannot access memory|No symbol|exited with code|received signal/) }, "probe error retained")
      check(stderr.empty?, "debugger stderr must be empty")
      lines = stdout.lines(chomp: true)
      check(lines.none? { |line| line.start_with?("PARENT_RI:", "REGEX:") }, "unexpected native callback marker")
      check(lines.count("READY:0") == 1 && lines.last&.match?(/\A\[Inferior 1 \(process [1-9][0-9]*\) exited normally\]\z/), "debugger lifecycle changed")
      events = context.fetch("events")
      raw = lines.each_with_index.filter_map do |line, index|
        next unless line.start_with?("K1:")

        value = json(line.delete_prefix("K1:"))
        check(!value.key?("native_line"), "raw K1 native_line injected")
        value.merge("native_line" => index + 1)
      end
      check(raw.eql?(events), "raw debugger K1/JSON mismatch")
      check(!events.empty?, "events missing")
      catalog = validate_catalog!(context, inventory)
      native = validate_native!(context, lines, catalog)
      frames = context.fetch("observed").fetch("application").fetch("frames")
      backend = Integer(frames.first.fetch("backend"), 10)
      check(lines.last == "[Inferior 1 (process #{backend}) exited normally]", "debugger/application backend changed")
      expected_names = context.fetch("mode") == "helpers-first" ? @source.helper_plan : []
      expected = expected_names.map { |name| [name, 0] }
      setting = context.fetch("variant") == "reference" ? 2 : 0
      frames.each do |frame|
        expected.concat([["snapshot", 0], ["search_result_ingest_v1", setting], ["derive_magnet_hash_v1", setting], ["normalize_title_v1", setting]])
        expected << ["RI_FKey_check_ins", setting] if frame.fetch("state") == "00000"
        expected.concat([["snapshot", 0], ["snapshot", 0]])
      end
      check(native.map { |event| [event.fetch("name", event["function"]), event.fetch("compiler_setting")] }.eql?(expected), "source-derived native call order or helper-first scope changed")
      root = catalog.values.find { |row| row.fetch("name") == "search_result_ingest_v1" }
      roots = native.select { |event| event["name"] == "search_result_ingest_v1" }
      snapshots = native.select { |event| event["name"] == "snapshot" }
      check(roots.length == 3 && snapshots.length == 9, "root/snapshot intervals changed")
      consumed = []
      intervals = frames.each_with_index.map do |frame, index|
        start, finish, committed = snapshots.slice(index * 3, 3).map { |event| event.fetch("native_line") }
        call = roots.fetch(index).fetch("native_line")
        check(start < call && call < finish && finish < committed, "application frame boundaries changed")
        inside = events.select { |event| event.fetch("native_line").between?(call + 1, finish - 1) }
        first_helper = native.find { |event| event.fetch("native_line") > call && event["name"] == "derive_magnet_hash_v1" }
        check(inside.all? { |event| event.fetch("native_line") < first_helper.fetch("native_line") }, "K1 branch moved past source helper boundary")
        expected_events = @source.expected_events(case_name: context.fetch("case"), variant: context.fetch("variant"), backend:, oid: root.fetch("oid"))
        check(inside.map { |event| event.reject { |key, _value| key == "native_line" } }.eql?(expected_events), "source-derived K1 sequence/locals/identities changed in call #{index + 1}")
        consumed.concat(inside)
        { "call" => index + 1, "backend" => backend, "state" => frame.fetch("state"),
          "snapshot_before_line" => start, "root_line" => call, "snapshot_after_line" => finish, "snapshot_finish_line" => committed,
          "first_k1_line" => inside.first.fetch("native_line"), "last_k1_line" => inside.last.fetch("native_line"),
          "branch" => context.fetch("case") == CASES.first ? "null-key-lookup-skipped" : "missing-row-null-rank-fallback", "rank" => 0, "bucket" => 0 }
      end
      check(consumed.eql?(events), "K1 events outside canonical root intervals")
      context.slice("case", "mode", "variant").merge("events" => events.length, "intervals" => intervals, "debugger_stderr" => stderr)
    end

    def validate_catalog!(context, inventory)
      rows = context.fetch("catalog")
      check(rows.all? { |row| row.keys.sort == CATALOG_FIELDS.sort }, "call catalog shape changed")
      check(rows.map { |row| row.fetch("oid") }.uniq.length == rows.length && rows.map { |row| row.fetch("name") }.uniq.length == rows.length, "call catalog duplicate identity")
      expected_names = %w[snapshot search_result_ingest_v1 derive_magnet_hash_v1 normalize_title_v1]
      expected_names += @source.helper_plan if context.fetch("mode") == "helpers-first"
      check(rows.map { |row| row.fetch("name") }.sort == expected_names.uniq.sort, "call catalog scope changed")
      source_rows = inventory.fetch("#{context.fetch('variant')}_proof").fetch("routines")
      rows.each do |row|
        check(row.fetch("oid").is_a?(Integer) && row.fetch("oid").positive?, "invalid routine oid")
        if row.fetch("name") == "snapshot"
          validate_snapshot!(row, context.fetch("snapshot_sql"))
        else
          source = source_rows.select { |item| item.fetch("name") == row.fetch("name") }
          check(source.length == 1, "helper source identity missing or duplicate")
          source = source.first
          language = row.fetch("name") == "policy_action_to_decision_type" ? "sql" : "plpgsql"
          check(row.slice("schema", "signature", "language", "source_sha256", "config").eql?(
            { "schema" => "public", "signature" => source.fetch("signature"), "language" => language,
              "source_sha256" => Digest::SHA256.hexdigest(source.fetch("source")), "config" => source.fetch("settings") }), "source/call catalog binding changed")
        end
      end
      rows.to_h { |row| [row.fetch("oid"), row] }
    end

    def validate_snapshot!(row, sql)
      pairs = IngestionProof::INGESTION_TABLES.map do |table|
        "'#{table}', (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)->(SELECT a.attname FROM pg_attribute a WHERE a.attrelid = 'public.#{table}'::regclass AND a.attnum = 1)), '[]') FROM public.\"#{table}\" t)"
      end
      source = " SELECT json_build_object(#{pairs.join(',')}); "
      check(sql.scan("AS $$#{source}$$;").length == 1, "snapshot source changed")
      expected = { "schema" => "ingestion_observation", "name" => "snapshot", "signature" => "ingestion_observation.snapshot()", "language" => "sql",
        "source_sha256" => Digest::SHA256.hexdigest(source), "config" => ["search_path=pg_catalog"] }
      check(row.reject { |key, _value| key == "oid" }.eql?(expected), "snapshot source/catalog binding changed")
    end

    def validate_native!(context, lines, catalog)
      trigger_catalog = context.fetch("trigger_catalog")
      check(trigger_catalog.length == 1, "trigger catalog scope changed")
      triggers = context.fetch("triggers").each
      reconstructed = []
      pending_ri = nil
      ready = lines.index("READY:0")
      lines.each_with_index do |line, index|
        if line.start_with?("CALL:")
          match = line.match(/\ACALL:(sql|plpgsql):([1-9][0-9]*):([012])\z/)
          check(match && index > ready, "raw native call malformed")
          row = catalog.fetch(Integer(match[2], 10))
          check(row.fetch("language") == match[1], "raw native language changed")
          reconstructed << row.merge("kind" => "call", "compiler_setting" => Integer(match[3], 10), "native_line" => index + 1)
        elsif line.start_with?("RI:")
          check(pending_ri.nil?, "duplicate unbound RI event")
          pending_ri = line.delete_prefix("RI:")
        elsif line.start_with?("TRIGGER:")
          trigger = triggers.next
          check(trigger.reject { |key, _value| key == "compiler_setting" }.eql?(trigger_catalog.first), "native trigger/catalog binding changed")
          check(pending_ri == "#{trigger.fetch('function')}:#{trigger.fetch('compiler_setting')}", "raw RI/trigger binding changed")
          expected = "TRIGGER:#{trigger.fetch('function')}:#{trigger.fetch('function_oid')}:#{trigger.fetch('trigger_oid')}:#{trigger.fetch('compiler_setting')}"
          check(line == expected && trigger.fetch("function") == "RI_FKey_check_ins" && trigger.fetch("table") == "canonical_torrent_signal", "raw native trigger binding changed")
          reconstructed << trigger.merge("kind" => "trigger", "native_line" => index + 1)
          pending_ri = nil
        end
      end
      check(pending_ri.nil? && context.fetch("triggers").length == reconstructed.count { |event| event.fetch("kind") == "trigger" }, "unbound trigger evidence")
      check(reconstructed.eql?(context.fetch("native")), "raw debugger/native JSON mismatch")
      reconstructed
    rescue StopIteration
      raise Failure, "K1 missing trigger binding"
    end
  end
end
