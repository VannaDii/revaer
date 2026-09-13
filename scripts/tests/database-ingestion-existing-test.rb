# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionExistingTest
    private

    def existing_sample
      fixture = sample_value
      fixture["before"] = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      fixture["after"] = fixture.fetch("before").merge(fixture.fetch("after"))
      fixture["backend"] = 101
      fixture["role"] = { "session" => @runtime, "current" => @runtime, "superuser" => false, "create_role" => false, "bypass_rls" => false }
      tested = Marshal.load(Marshal.dump(fixture))
      tested["before"] = Marshal.load(Marshal.dump(fixture.fetch("after")))
      tested["backend"] = 102
      tested["clocks"] = ["2026-09-10T00:01:00+00:00"]
      tested.fetch("after").fetch("canonical_torrent").first["updated_at"] = tested.fetch("clocks").first
      { "fixture" => fixture, "tested" => tested }
    end

    def existing_compare(value)
      ingestion_existing_comparable(value, role: @runtime)
    end

    def existing_data_tests!
      value = existing_sample
      normalized = existing_compare(value)
      assert(value == existing_sample, "existing-data normalization must preserve raw evidence")
      fixture = normalized.fetch("fixture")
      tested = normalized.fetch("tested")
      assert(fixture.fetch("after").fetch("canonical_torrent").first.fetch("created_at") == "<fixture-transaction-time>", "seed clock must retain fixture provenance")
      assert(tested.fetch("before").fetch("canonical_torrent").first.fetch("created_at") == "<fixture-transaction-time>", "before image must retain seed-clock provenance")
      assert(tested.fetch("after").fetch("canonical_torrent").first.fetch("updated_at") == "<tested-transaction-time>", "new timestamps must retain tested provenance")
      replacement = JSON.parse(JSON.generate(value).gsub("56900000", "56900001").gsub("2026-09-10T00:", "2026-09-11T00:"))
      assert(normalized == existing_compare(replacement), "only generated identities and named stage clocks should normalize")
      changed = existing_sample
      changed.fetch("tested")["backend"] = 101
      rejected("distinct fixture and tested") { existing_compare(changed) }
      changed = existing_sample
      changed.fetch("tested")["clocks"] = changed.fetch("fixture").fetch("clocks")
      rejected("provenance missing or ambiguous") { existing_compare(changed) }
      [[], ["invalid"], [nil], ["2026-09-10T00:01:00+00:00", "2026-09-10T00:02:00+00:00"]].each do |clocks|
        changed = existing_sample
        changed.fetch("fixture")["clocks"] = clocks
        rejected("provenance missing or ambiguous") { existing_compare(changed) }
      end
      changed = existing_sample
      changed.fetch("tested").fetch("before").fetch("canonical_torrent").first["title"] = "changed"
      rejected("fixture continuity") { existing_compare(changed) }
      IngestionExisting::EXISTING_STAGES.each do |stage|
        changed = existing_sample
        changed.fetch(stage).fetch("role")["superuser"] = true
        rejected("exact role contract") { existing_compare(changed) }
        changed = existing_sample
        changed.fetch(stage).fetch("results").first["canonical_torrent_public_id"] = "56900000-0000-4000-8000-000000000008"
        rejected("no committed stage row") { existing_compare(changed) }
        changed = existing_sample
        changed.fetch(stage).fetch("results").first["canonical_changed"] = false
        assert(normalized != existing_compare(changed), "#{stage} result flags must affect parity")
      end
      changed = existing_sample
      changed.fetch("fixture").fetch("before").delete("search_page")
      rejected("table inventory incomplete") { existing_compare(changed) }
      changed = existing_sample
      changed.fetch("tested").fetch("after").fetch("canonical_torrent").first["canonical_torrent_public_id"] = "56900000-0000-4000-8000-000000000007"
      rejected("identity is invalid, duplicated or changed") { existing_compare(changed) }
      changed = existing_sample
      changed.fetch("tested").fetch("after").fetch("canonical_torrent").push(changed.fetch("tested").fetch("after").fetch("canonical_torrent").first.dup)
      rejected("identity is invalid, duplicated or changed") { existing_compare(changed) }
      before_only = { "canonical_torrent_id" => 3, "canonical_torrent_public_id" => "56900000-0000-4000-8000-000000000009", "title" => "removed fixture row" }
      changed = existing_sample
      changed.fetch("fixture").fetch("before").fetch("canonical_torrent") << before_only
      before_image = existing_compare(changed).fetch("fixture").fetch("before").fetch("canonical_torrent")
      assert(before_image.first.fetch("canonical_torrent_public_id") == "<canonical_torrent:3>", "before-only identity must retain its row association")
      assert(normalized != existing_compare(changed), "before-only rows must affect parity")
      ["title", "observed_at", "value_text", "unexpected_at"].each do |column|
        changed = existing_sample
        clock = changed.fetch("fixture").fetch("clocks").first
        changed.fetch("tested").fetch("after").fetch("canonical_torrent").first[column] = clock
        assert(existing_compare(changed).fetch("tested").fetch("after").fetch("canonical_torrent").first.fetch(column) == clock, "non-generated #{column} must not normalize as a clock")
      end
      changed = existing_sample
      uuid = changed.fetch("fixture").fetch("results").first.fetch("canonical_torrent_public_id")
      changed.fetch("tested").fetch("after").fetch("canonical_torrent").first["title"] = uuid
      assert(existing_compare(changed).fetch("tested").fetch("after").fetch("canonical_torrent").first.fetch("title") == uuid, "arbitrary identity-shaped text must not normalize")
      changed = existing_sample
      changed.fetch("fixture")["clocks"] = ["2026-09-10T01:00:00+00:00"]
      assert(normalized != existing_compare(changed), "lost seed timestamp provenance must affect parity")
      changed = existing_sample
      changed.fetch("tested").fetch("after").fetch("canonical_torrent_source").first["canonical_torrent_id"] = 3
      assert(normalized != existing_compare(changed), "numeric relationship identities must remain exact")
      existing_parse_tests!
      existing_outcome_tests!
      existing_v2_tests!
    end

    def existing_parse_tests!
      stdout = "backend:101\n#{sample_stdout}"
      parsed = ingestion_existing_parse(stdout, sample_stderr, role: @runtime)
      assert(parsed.fetch("backend") == 101, "backend identity must be retained")
      assert(parsed.fetch("role").fetch("current") == @runtime, "exact observed role must be retained")
      [sample_stdout, "backend:0\n#{sample_stdout}", "backend:101 trailing\n#{sample_stdout}"].each do |text|
        rejected("backend evidence missing") { ingestion_existing_parse(text, sample_stderr, role: @runtime) }
      end
      rejected("unrecognized record") { ingestion_existing_parse("backend:101\n#{stdout}", sample_stderr, role: @runtime) }
      rejected("unrecognized record") { ingestion_existing_parse(stdout, sample_stderr + "NOTICE: unexpected\n", role: @runtime) }
      changed = stdout.sub('"bypass_rls":false', '"bypass_rls":false,"other":true')
      rejected("exact role contract") { ingestion_existing_parse(changed, sample_stderr, role: @runtime) }
      assert(ingestion_existing_cases.length == 8, "existing-data matrix must retain the external-id counterexample and seven bounded paths")
      assert(ingestion_cases.last == ["warm-committed", {}, ["00000", "00000"]], "existing-data cases must never replace the warm counterexample")
    end

    def existing_outcome_sample
      value = existing_sample
      value.each do |stage, data|
        data["states"] = ["00000"]
        data["details"] = []
        data.fetch("results").first.merge!("observation_created" => stage == "fixture", "durable_source_created" => stage == "fixture", "canonical_changed" => stage == "fixture")
        data.fetch("after").fetch("canonical_torrent_source").first.merge!("infohash_v1" => "a" * 40, "last_seen_seeders" => stage == "fixture" ? 5 : 17)
        data.fetch("after")["search_request_source_observation"] = [{ "observation_id" => 1 }]
        data.fetch("after").fetch("canonical_torrent").first["title_display"] = stage == "fixture" ? "Ingestion proof title" : "Updated proof title"
      end
      value.fetch("tested")["before"] = Marshal.load(Marshal.dump(value.fetch("fixture").fetch("after")))
      value
    end

    def existing_outcome_tests!
      value = existing_outcome_sample
      assert(ingestion_existing_outcome?("existing-refresh", value), "bounded refresh known answer must pass")
      changed = Marshal.load(Marshal.dump(value))
      changed.fetch("fixture").fetch("results").first["canonical_changed"] = false
      assert(!ingestion_existing_outcome?("existing-refresh", changed), "shared fixture flag error must not pass")
      changed = Marshal.load(Marshal.dump(value))
      changed.fetch("tested").fetch("results").first["durable_source_created"] = true
      assert(!ingestion_existing_outcome?("existing-refresh", changed), "shared source recreation must not pass")
      changed = Marshal.load(Marshal.dump(value))
      changed.fetch("tested").fetch("after").fetch("canonical_torrent_source").first["last_seen_seeders"] = 5
      assert(!ingestion_existing_outcome?("existing-refresh", changed), "shared missing refresh must not pass")
      changed = Marshal.load(Marshal.dump(value))
      changed.fetch("tested")["states"] = ["42P10"]
      changed.fetch("tested")["results"] = []
      assert(!ingestion_existing_outcome?("existing-external-id", changed), "shared external-id upsert error must remain a required failure")
      changed = Marshal.load(Marshal.dump(value))
      changed.fetch("tested").merge!("states" => ["P0001"], "details" => ["attr_length_mismatch"], "results" => [])
      changed.fetch("tested")["after"] = Marshal.load(Marshal.dump(changed.fetch("tested").fetch("before")))
      assert(ingestion_existing_outcome?("existing-attrs-rollback", changed), "exact attribute rejection must preserve prior state")
      changed.fetch("tested").fetch("after").fetch("canonical_torrent").first["title_display"] = "leaked partial write"
      assert(!ingestion_existing_outcome?("existing-attrs-rollback", changed), "shared rollback write leak must not pass")
      IngestionProof::INGESTION_TABLES.reject { |table| IngestionExisting::EXISTING_IDENTITIES.include?(table) }.each do |table|
        changed = existing_sample
        changed.fetch("tested").fetch("after").fetch(table) << { "unexpected_mutation" => true }
        assert(existing_compare(existing_sample) != existing_compare(changed), "#{table} mutations must affect parity")
      end
    end

    def existing_v2_test_evidence(mode, variant)
      role = variant == "reference" ? "postgres" : @runtime
      uuid_prefix = variant == "reference" ? "59610000" : "59620000"
      identities = { "canonical_torrent_public_id" => "#{uuid_prefix}-0000-4000-8000-000000000001",
        "canonical_torrent_source_public_id" => "#{uuid_prefix}-0000-4000-8000-000000000002" }
      clock = "2026-09-12T00:00:01+00:00"
      shape = { title: "Ingestion proof title", normalized: "ingestion proof title", answers: [], signals: [], release: nil }
      tables = attributes_initial_tables(shape, clock, identities)
      %w[canonical_torrent canonical_torrent_source search_request_source_observation].each do |table|
        tables.fetch(table).first.merge!("infohash_v1" => nil, "infohash_v2" => "b" * 64,
          "magnet_hash" => Digest::SHA256.hexdigest(["b" * 64].pack("H*")), "size_bytes" => 1024)
      end
      tables.fetch("canonical_torrent").first["identity_strategy"] = "infohash_v2"
      tables["canonical_torrent_best_source_context"] = [{ "canonical_torrent_best_source_context_id" => 1,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "computed_at" => clock }]
      fixture = { "backend" => "101", "role" => { "session" => role, "current" => role, "superuser" => role == "postgres",
        "create_role" => role == "postgres", "bypass_rls" => role == "postgres" }, "clock" => clock,
        "state" => "00000", "before" => "error", "after" => "error", "finished_setting" => "error",
        "within" => "true", "outside" => (variant == "reference").to_s,
        "result" => identities.merge("observation_created" => true, "durable_source_created" => true, "canonical_changed" => true),
        "tables_before" => attributes_empty, "tables_after" => tables, "tables_finish" => tables }
      frames = (0...(mode == "warm-rollback" ? 2 : 1)).map do |ordinal|
        existing_v2_test_frame(fixture, ordinal, mode, variant, uuid_prefix)
      end
      seed_clock = "2026-09-12T00:00:00+00:00"
      { "seed_clock" => seed_clock, "fixtures" => [fixture], "frames" => frames,
        "before" => tables, "after" => frames.last.fetch("tables_finish"),
        "inputs_before" => hash_fill_read_tables(seed_clock), "inputs_after" => hash_fill_read_tables(seed_clock) }
    end

    def existing_v2_test_frame(fixture, ordinal, mode, variant, uuid_prefix)
      frame = Marshal.load(Marshal.dump(fixture))
      frame["backend"] = "102"
      frame["clock"] = "2026-09-12T00:00:0#{ordinal + 2}+00:00"
      frame["tables_before"] = Marshal.load(Marshal.dump(fixture.fetch("tables_after")))
      tables = frame.fetch("tables_after")
      uuid = "#{uuid_prefix}-0000-4000-8000-00000000000#{ordinal + 3}"
      hashes = { "infohash_v2" => "c" * 64, "magnet_hash" => Digest::SHA256.hexdigest(["c" * 64].pack("H*")) }
      tables.fetch("canonical_torrent") << tables.fetch("canonical_torrent").first.merge(hashes).merge(
        "canonical_torrent_id" => ordinal + 2, "canonical_torrent_public_id" => uuid, "created_at" => frame.fetch("clock"), "updated_at" => frame.fetch("clock"))
      tables.fetch("canonical_torrent_source").first.merge!("last_seen_at" => "2026-09-10T00:01:00+00:00", "last_seen_seeders" => 17, "updated_at" => frame.fetch("clock"))
      tables.fetch("search_request_source_observation").first.merge!(hashes.merge(
        "canonical_torrent_id" => ordinal + 2, "observed_at" => "2026-09-10T00:01:00+00:00", "seeders" => 17))
      tables.fetch("canonical_torrent_best_source_context") << { "canonical_torrent_best_source_context_id" => ordinal + 2,
        "canonical_torrent_id" => ordinal + 2, "canonical_torrent_source_id" => 1, "computed_at" => frame.fetch("clock") }
      [["b" * 64, "c" * 64], %w[b c].map { |value| Digest::SHA256.hexdigest([value * 64].pack("H*")) }].each_with_index do |(existing, incoming), index|
        id = ordinal * 2 + index + 1
        tables.fetch("source_metadata_conflict") << { "source_metadata_conflict_id" => id, "canonical_torrent_source_id" => 1,
          "conflict_type" => "hash", "existing_value" => existing, "incoming_value" => incoming,
          "observed_at" => "2026-09-10T00:01:00+00:00", "resolved_at" => nil, "resolved_by_user_id" => nil, "resolution" => nil, "resolution_note" => nil }
        tables.fetch("source_metadata_conflict_audit_log") << { "source_metadata_conflict_audit_log_id" => id, "conflict_id" => id,
          "action" => "created", "actor_user_id" => 0, "occurred_at" => frame.fetch("clock"), "note" => nil }
        tables.fetch("indexer_health_event") << { "indexer_health_event_id" => id, "indexer_instance_id" => 569001,
          "occurred_at" => "2026-09-10T00:01:00+00:00", "event_type" => "identity_conflict", "latency_ms" => nil,
          "http_status" => nil, "error_class" => nil, "detail" => "hash" }
      end
      frame.fetch("result").merge!("canonical_torrent_public_id" => uuid, "observation_created" => false, "durable_source_created" => false)
      rollback = mode == "warm-rollback" && ordinal.zero?
      frame["outside"] = (variant == "reference" && !rollback).to_s
      frame["tables_finish"] = rollback ? frame.fetch("tables_before") : tables
      frame
    end

    def existing_v2_tests!
      fixture, tested = existing_v2_arguments
      assert(fixture.fetch(:infohash_v2_input) == "repeat('b',64)::char(64)" && tested.fetch(:infohash_v2_input) == "repeat('c',64)::char(64)", "selected GUID fixture must change a non-NULL v2 hash")
      [fixture, tested].each do |arguments|
        call = ingestion_call(arguments)
        assert(call.include?("source_guid_input => 'ingestion-proof-source'::varchar"), "both real ingestions must select the same GUID")
        assert(call.include?("infohash_v1_input => NULL::char(40)") && call.include?("magnet_hash_input => NULL::char(64)"), "v2 and derived magnet conflicts must not be hidden by v1 or explicit magnet input")
      end
      IngestionWrapper::WRAPPER_MODES.each do |mode|
        session = existing_v2_session(mode)
        query = correction_session(session)
        assert(query.scan("FROM public.search_result_ingest(").length == session.fetch(:calls).length, "#{mode} executes actual wrapper ingestion")
        assert(query.scan(/^ROLLBACK;$/).length == (mode == "warm-rollback" ? 1 : 0) && query.scan(/^COMMIT;$/).length == 1, "#{mode} retains actual rollback and committed retry")
        assert(query.include?("finished_setting:") && !query.match?(/DROP TABLE|DISCARD|SET ROLE|SET plpgsql|\\connect/), "#{mode} retains caller settings without backend repair")
        assert(query.include?("helpers:") == (mode == "helpers-first"), "#{mode} preserves actual helper-first ordering")
        variants = %w[reference final].to_h do |variant|
          evidence = existing_v2_test_evidence(mode, variant)
          role = variant == "reference" ? "postgres" : @runtime
          assert(existing_v2_evidence?(evidence, mode, variant, role), "#{mode} #{variant} exact selected-GUID v2 conflict evidence")
          existing_v2_mutations!(evidence, mode, variant, role)
          [variant, compilation_comparable("fixture" => evidence.fetch("fixtures").first, "frames" => evidence.fetch("frames"))]
        end
        assert(variants.fetch("reference") == variants.fetch("final"), "#{mode} raw retained identity, log and rollback evidence compares across exact variants")
      end
    end

    def existing_v2_mutations!(evidence, mode, variant, role)
      raw = JSON.generate(evidence)
      frame = evidence.fetch("frames").last
      %w[canonical_torrent canonical_torrent_source search_request_source_observation source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].each do |table|
        frame.fetch("tables_after").fetch(table).each_with_index do |row, index|
          row.each_key do |column|
            changed = JSON.parse(raw)
            changed.fetch("frames").last.fetch("tables_after").fetch(table).fetch(index)[column] = "unexpected"
            existing_v2_test_commit!(changed)
            assert(!existing_v2_evidence?(changed, mode, variant, role), "#{mode} rejects exact #{table}.#{column} drift")
          end
        end
        %w[missing duplicate].each do |mutation|
          changed = JSON.parse(raw)
          rows = changed.fetch("frames").last.fetch("tables_after").fetch(table)
          mutation == "missing" ? rows.pop : rows.push(rows.last.dup)
          existing_v2_test_commit!(changed)
          assert(!existing_v2_evidence?(changed, mode, variant, role), "#{mode} rejects #{mutation} #{table} rows")
        end
      end
      %w[canonical_torrent_public_id canonical_torrent_source_public_id observation_created durable_source_created canonical_changed].each do |key|
        changed = JSON.parse(raw)
        changed.fetch("frames").last.fetch("result")[key] = "unexpected"
        assert(!existing_v2_evidence?(changed, mode, variant, role), "#{mode} rejects #{key} drift")
      end
      %w[state before after finished_setting within outside backend clock].each do |key|
        changed = JSON.parse(raw)
        changed.fetch("frames").last[key] = "unexpected"
        assert(!existing_v2_evidence?(changed, mode, variant, role), "#{mode} rejects #{key} provenance drift")
      end
      changed = JSON.parse(raw)
      changed.fetch("frames").last.fetch("role")["superuser"] = role != "postgres"
      assert(!existing_v2_evidence?(changed, mode, variant, role), "#{mode} rejects role drift")
      changed = JSON.parse(raw)
      changed.fetch("frames").last["diagnostic"] = { "state" => "P0001", "detail" => "unexpected" }
      assert(!existing_v2_evidence?(changed, mode, variant, role), "#{mode} rejects a shared application failure")
      changed = JSON.parse(raw)
      changed.fetch("inputs_after").fetch("indexer_instance").first["trust_tier_key"] = "private"
      assert(!existing_v2_evidence?(changed, mode, variant, role), "#{mode} rejects changed read inputs")
      changed = JSON.parse(raw)
      changed.fetch("frames").last.fetch("tables_after").fetch("canonical_torrent_best_source_context").last["canonical_torrent_source_id"] = 2
      existing_v2_test_commit!(changed)
      assert(!existing_v2_evidence?(changed, mode, variant, role), "#{mode} rejects the wrong wrapper best-source identity")
      baseline = compilation_comparable("fixture" => evidence.fetch("fixtures").first, "frames" => evidence.fetch("frames"))
      IngestionProof::INGESTION_TABLES.each do |table|
        changed = JSON.parse(raw)
        rows = changed.fetch("frames").last.fetch("tables_after").fetch(table)
        rows.empty? ? rows.push("unexpected_write" => true) : rows.first["unexpected_write"] = true
        compared = compilation_comparable("fixture" => changed.fetch("fixtures").first, "frames" => changed.fetch("frames"))
        assert(compared != baseline, "#{mode} retains every #{table} write in cross-variant comparison")
        next unless mode == "warm-rollback"

        changed = JSON.parse(raw)
        changed.fetch("frames").first.fetch("tables_finish").fetch(table) << { "rollback_leak" => true }
        assert(!existing_v2_evidence?(changed, mode, variant, role), "#{table} rollback leaks cannot pass warm evidence")
      end
      return unless mode == "warm-rollback"

      assert(frame.fetch("tables_after").fetch("canonical_torrent").last.fetch("canonical_torrent_id") == 3, "rolled-back canonical allocation must remain consumed")
      assert(frame.fetch("tables_after").fetch("source_metadata_conflict").map { |row| row.fetch("source_metadata_conflict_id") } == [3, 4], "both rolled-back logger sequence allocations must remain consumed")
      changed = JSON.parse(raw)
      changed.fetch("frames").last["backend"] = "103"
      assert(!existing_v2_evidence?(changed, mode, variant, role), "a cold reconnect cannot pass as warm retry")
      changed = JSON.parse(raw)
      changed.fetch("frames").first["tables_finish"] = changed.fetch("frames").first.fetch("tables_after")
      assert(!existing_v2_evidence?(changed, mode, variant, role), "a committed first call cannot pass rollback-warm evidence")
      changed = JSON.parse(raw)
      tables = changed.fetch("frames").last.fetch("tables_after")
      %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].each do |table|
        tables.fetch(table).each_with_index do |row, index|
          row["#{table}_id"] = index + 1
          row["conflict_id"] = index + 1 if table == "source_metadata_conflict_audit_log"
        end
      end
      existing_v2_test_commit!(changed)
      assert(!existing_v2_evidence?(changed, mode, variant, role), "coherently reset logger/audit/health sequences cannot hide rolled-back allocations")
    end

    def existing_v2_test_commit!(evidence)
      frame = evidence.fetch("frames").last
      frame["tables_finish"] = JSON.parse(JSON.generate(frame.fetch("tables_after")))
      evidence["after"] = JSON.parse(JSON.generate(frame.fetch("tables_finish")))
    end
  end
end

if $PROGRAM_NAME == __FILE__
  require_relative "../database_rebaseline/final_proof"
  require_relative "database-ingestion-identity-test"

  module RevaerDatabaseRebaseline
    class IngestionIdentityCasesTest < FinalProof
      include IngestionExistingTest
      include IngestionIdentityTest

      def run_tests!
        @assertions = 0
        identity_tests!
        existing_v2_tests!
        puts "database-ingestion-identity-existing-test: #{@assertions} assertions passed"
      end

      private

      def assert(value, message)
        raise Failure, message unless value

        @assertions += 1
      end
    end
  end

  RevaerDatabaseRebaseline::IngestionIdentityCasesTest.new.run_tests!
end
