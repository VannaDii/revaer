# frozen_string_literal: true

require_relative "database-ingestion-attributes-test"

module RevaerDatabaseRebaseline
  class IngestionGuidTest < IngestionAttributesTest
    def run_tests!
      @assertions = 0
      @runtime = "guid_unit_runtime"
      guid_test_inventory!
      GUID_KINDS.product(GUID_MODES).each do |kind, mode|
        pair = %w[reference final].to_h do |variant|
          role = variant == "reference" ? "postgres" : @runtime
          evidence = guid_test_evidence(kind, mode, role)
          plain = guid_validate!(evidence, kind, mode, variant, role)
          observed = guid_test_observed(evidence, kind, mode, variant, role)
          assert(plain == guid_validate!(observed, kind, mode, variant, role, observed: true), "GUID trace preserves synthetic application evidence")
          guid_trace_mutations!(observed, kind, mode, variant, role)
          [variant, plain]
        end
        assert(JSON.generate(pair.fetch("reference")) == JSON.generate(pair.fetch("final")), "GUID bounded synthetic path parity")
        guid_mutations!(kind, mode)
      end
      guid_instrument_tests!
      puts "database-ingestion-guid-test: #{@assertions} assertions passed"
    end

    private

    def guid_test_inventory!
      routines = { "search_result_ingest_v1" => "0052_indexer_search_result_ingest_proc.sql",
        "log_source_metadata_conflict_v1" => "0052_indexer_search_result_ingest_proc.sql",
        "search_result_ingest" => "0120_search_result_ingest_seed_best_source_context.sql" }.map do |name, path|
        text = File.binread(File.join(@contract.root, "crates/revaer-data/migrations", path))
        source = text.split("CREATE OR REPLACE FUNCTION #{name}(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
        { "name" => name, "signature" => "#{name}(uuid)", "source" => source }
      end
      final = copy(routines)
      final.first["source"] = "\n#variable_conflict use_column" + final.first.fetch("source")
      @ingestion_inventory = { "reference_proof" => { "routines" => routines }, @database => { "routines" => final } }
    end

    def attributes_source_hashes
      guid_source_hashes
    end

    def verify_ingestion_attributes!
      verify_ingestion_guid!
    ensure
      @attributes_evidence = @guid_evidence
    end

    def guid_test_transport(frames, helpers: false)
      lines = helpers ? ["helpers:#{JSON.generate(ingestion_helper_expectations)}"] : []
      frames.each do |frame|
        correction_records(guid_session({})).each do |key|
          lines << JSON.generate(frame.fetch("result")) if key == "state"
          value = frame.fetch(key)
          lines << "#{key}:#{%w[role clock tables_before tables_after tables_finish].include?(key) ? JSON.generate(value) : value}"
        end
      end
      { "stdout" => lines.join("\n") + "\n", "stderr" => "", "frames" => frames }
    end

    def guid_test_frame(role, number, before, after, source: 1)
      row = after.fetch("canonical_torrent_source").find { |item| item.fetch("canonical_torrent_source_id") == source }
      canonical = after.fetch("canonical_torrent").find { |item| item.fetch("canonical_torrent_id") == source }
      { "backend" => (100 + number).to_s, "clock" => "2026-09-12T00:00:0#{number}+00:00",
        "before" => "error", "after" => "error", "finished_setting" => "error",
        "role" => { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" },
        "state" => "00000", "within" => "true", "outside" => (role == "postgres").to_s,
        "tables_before" => copy(before), "tables_after" => copy(after), "tables_finish" => copy(after),
        "result" => { "canonical_torrent_public_id" => canonical.fetch("canonical_torrent_public_id"),
          "canonical_torrent_source_public_id" => row.fetch("canonical_torrent_source_public_id"),
          "canonical_changed" => false, "observation_created" => false, "durable_source_created" => false } }
    end

    # Minimal synthetic records test the oracle, never stand in for live SQL evidence.
    def guid_test_evidence(kind, mode, role)
      before = attributes_empty
      fixtures = (1..(kind == "competing-guid" ? 2 : 1)).map do |id|
        previous = copy(before)
        before.fetch("canonical_torrent") << { "canonical_torrent_id" => id, "canonical_torrent_public_id" => "56900000-0000-4000-8000-00000000009#{id * 2 - 2}" }
        before.fetch("canonical_torrent_source") << { "canonical_torrent_source_id" => id, "canonical_torrent_id" => id,
          "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-00000000009#{id * 2 - 1}", "source_guid" => nil, "infohash_v1" => (id == 1 ? "a" : "b") * 40 }
        guid_test_transport([guid_test_frame(role, id, previous, before, source: id)])
      end
      intermediate = copy(before)
      selected = kind == "competing-guid" ? 2 : 1
      intermediate.fetch("canonical_torrent_source").fetch(selected - 1)["source_guid"] = selected == 2 ? "wanted" : "other"
      b = guid_test_frame(role, 4, before, intermediate, source: selected)
      after = copy(intermediate)
      existing = before.fetch("canonical_torrent_source").fetch(selected - 1).fetch("canonical_torrent_source_public_id")
      after.merge!(guid_expected_records(1, existing, "2026-09-12T00:00:05+00:00", id: mode == "logger-first" ? 2 : 1))
      after["search_request_source_observation"] = [{ "source_guid" => "wanted", "guid_conflict" => true, "canonical_torrent_source_id" => 1, "canonical_torrent_id" => 1 }]
      a = guid_test_frame(role, 5, before, after)
      a.fetch("result")["observation_created"] = kind != "competing-guid"
      warm = []
      if mode == "logger-first"
        clock = "2026-09-12T00:00:03+00:00"
        warmed = before.merge(guid_expected_records(1, "", clock, type: "tracker_name", incoming: "", observed: clock))
        frame = guid_test_frame(role, 3, before, warmed).merge("backend" => a.fetch("backend"), "within" => "false", "outside" => "false",
          "tables_finish" => copy(before), "result" => { "logger" => "" })
        warm << frame
      end
      seed = "2026-09-12T00:00:00+00:00"
      inputs = metadata_read_tables(seed)
      { "seed_clock" => seed, "fixtures" => fixtures, "before" => before, "intermediate" => intermediate, "after" => after,
        "paused" => guid_test_transport(warm + [a], helpers: mode == "helpers-first"), "contender" => guid_test_transport([b], helpers: mode == "helpers-first"),
        "barrier" => { "pid" => 105, "usename" => role, "application_name" => "guid-race-paused", "classid" => "588", "objid" => "1029", "objsubid" => 2, "granted" => false },
        "inputs_before" => inputs, "inputs_after" => copy(inputs) }
    end

    def guid_mutations!(kind, mode)
      original = guid_test_evidence(kind, mode, @runtime)
      validate = ->(value) { guid_validate!(value, kind, mode, "final", @runtime) }
      changed = copy(original)
      changed.fetch("paused").fetch("frames").first["after"] = "use_column"
      rejected("raw and declared") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("paused")["stderr"] = "unexpected diagnostic"
      rejected("unexpected diagnostic") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("paused")["stdout"] = changed.fetch("paused").fetch("stdout").sub('role:{', 'role:{"session":"duplicate",')
      rejected("duplicate metadata JSON") { validate.call(changed) }
      %w[before after finished_setting within outside backend clock].each do |key|
        changed = copy(original)
        frame = changed.fetch("paused").fetch("frames").last
        frame[key] = key == "clock" ? "2026-02-30T00:00:00+00:00" : "101"
        changed["paused"] = guid_test_transport(changed.fetch("paused").fetch("frames"), helpers: mode == "helpers-first")
        rejected("GUID") { validate.call(changed) }
      end
      original.fetch("barrier").each_key do |key|
        changed = copy(original)
        changed.fetch("barrier")[key] = "altered"
        rejected("exact barrier") { validate.call(changed) }
      end
      %w[session current superuser create_role bypass_rls].each do |key|
        changed = copy(original)
        frame = changed.fetch("paused").fetch("frames").last
        frame.fetch("role")[key] = "altered"
        changed["paused"] = guid_test_transport(changed.fetch("paused").fetch("frames"), helpers: mode == "helpers-first")
        rejected("direct authority") { validate.call(changed) }
      end
      IngestionProof::INGESTION_TABLES.each do |table|
        changed = copy(original)
        changed.fetch("after").fetch(table) << { "unexpected" => true }
        rejected("committed transition") { validate.call(changed) }
      end
      guid_expected_records(1, "unused", "unused").each do |table, rows|
        rows.first.each_key do |column|
          changed = copy(original)
          changed.fetch("after").fetch(table).first[column] = "altered"
          frame = changed.fetch("paused").fetch("frames").last
          frame["tables_after"] = frame["tables_finish"] = copy(changed.fetch("after"))
          changed["paused"] = guid_test_transport(changed.fetch("paused").fetch("frames"), helpers: mode == "helpers-first")
          rejected("independent conflict") { validate.call(changed) }
        end
      end
      changed = copy(original)
      changed.fetch("inputs_after").fetch("trust_tier").first["default_weight"] = 666.0
      rejected("read inputs") { validate.call(changed) }
      IngestionPolicy::POLICY_READ_TABLES.each do |table|
        changed = copy(original)
        changed.fetch("inputs_before").fetch(table) << { "unexpected" => true }
        changed["inputs_after"] = copy(changed.fetch("inputs_before"))
        rejected("read inputs") { validate.call(changed) }
      end
      changed = copy(original)
      changed.fetch("barrier")["classid"] = 588.0
      rejected("exact barrier") { validate.call(changed) }
      changed = copy(original)
      frame = changed.fetch("paused").fetch("frames").last
      frame.fetch("result")["observation_created"] = kind == "competing-guid"
      changed["paused"] = guid_test_transport(changed.fetch("paused").fetch("frames"), helpers: mode == "helpers-first")
      rejected("observation binding") { validate.call(changed) }
      guid_warm_mutations!(original, mode, validate) if mode == "logger-first"
    end

    def guid_warm_mutations!(original, mode, validate)
      %w[backend within outside before after finished_setting].each do |key|
        changed = copy(original)
        changed.fetch("paused").fetch("frames").first[key] = "altered"
        changed["paused"] = guid_test_transport(changed.fetch("paused").fetch("frames"), helpers: mode == "helpers-first")
        rejected("mutating logger-first") { validate.call(changed) }
      end
      changed = copy(original)
      warm = changed.fetch("paused").fetch("frames").first
      warm["tables_finish"] = copy(warm.fetch("tables_after"))
      changed["paused"] = guid_test_transport(changed.fetch("paused").fetch("frames"))
      rejected("mutating logger-first") { validate.call(changed) }
    end

    def guid_test_notices(records)
      records.map do |record|
        "NOTICE:  00000: ingestion-guid-setting:#{JSON.generate(record.fetch('event'))}\n" \
          "CONTEXT:  PL/pgSQL function ingestion_observation.trace_guid() line 3 at RAISE\n#{record.fetch('stack')}" \
          "LOCATION:  exec_stmt_raise, pl_exec.c:3897\n"
      end.join
    end

    def guid_test_observed(evidence, kind, mode, variant, role)
      value = copy(evidence)
      frames = value.fetch("paused").fetch("frames")
      records = guid_trace_expected(frames, { kind:, variant:, role:, logger: true }, warm_logger: mode == "logger-first")
      value.fetch("paused")["stderr"] = guid_test_notices(records)
      value
    end

    def guid_trace_mutations!(original, kind, mode, variant, role)
      validate = ->(value) { guid_validate!(value, kind, mode, variant, role, observed: true) }
      records = JSON.parse(JSON.generate(guid_trace_records(original.fetch("paused").fetch("stderr"))))
      [records.drop(1), records.reverse, records + [records.first]].each do |altered|
        changed = copy(original)
        changed.fetch("paused")["stderr"] = guid_test_notices(altered)
        rejected("in-call settings") { validate.call(changed) }
      end
      records.each_index do |index|
        records.fetch(index).fetch("event").each_key do |key|
          altered = copy(records)
          altered.fetch(index).fetch("event")[key] = "altered"
          changed = copy(original)
          changed.fetch("paused")["stderr"] = guid_test_notices(altered)
          rejected("in-call settings") { validate.call(changed) }
        end
        changed = copy(original)
        altered = copy(records)
        altered.fetch(index)["stack"] += "unexpected frame\n"
        changed.fetch("paused")["stderr"] = guid_test_notices(altered)
        rejected("statement stack") { validate.call(changed) }
      end
      changed = copy(original)
      changed.fetch("paused")["stderr"] = ""
      rejected("in-call settings") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("paused")["stderr"] = original.fetch("paused").fetch("stderr").sub('"schema":', '"schema":"duplicate","schema":')
      rejected("duplicate metadata JSON") { validate.call(changed) }
    end

    def guid_instrument_tests!
      original = File.binread(File.join(@contract.root, "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql"))
      GUID_KINDS.each do |kind|
        changed = guid_instrument(original, kind)
        hook = "        IF pg_catalog.current_setting('application_name') = 'guid-race-paused' THEN\n            PERFORM pg_catalog.pg_advisory_xact_lock(588, 1029);\n        END IF;\n"
        assert(changed.scan(hook).length == 1 && changed.sub(hook, "") == original, "only exact disposable scheduling hook changes")
        rejected("one exact observation site") { guid_instrument(original + original, kind) }
        rejected("one exact observation site") { guid_instrument("", kind) }
      end
      query = correction_session(guid_session({}, helpers: true))
      assert(query.scan("FROM public.search_result_ingest(").length == 1 && query.scan("COMMIT;").length == 1, "one actual wrapper and committed lifetime")
      assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no compiler or namespace repair")
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    proof = RevaerDatabaseRebaseline::IngestionGuidTest.new
    if ARGV.empty?
      proof.run_tests!
    elsif ARGV.length == 2 && ARGV.first == "--live"
      proof.run_live!(ARGV.fetch(1))
    else
      raise RevaerDatabaseRebaseline::Failure, "expected no arguments or --live exact-candidate-path"
    end
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-guid-test: #{error.message}"
    exit 1
  end
end
