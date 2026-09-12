# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionHashFillTest
    private

    def hash_fill_tests!
      cases = wrapper_hash_fill_cases
      assert(cases.map { |entry| entry.fetch(:name) } == %w[fill-v1-uncontested fill-v1-competing fill-v2-uncontested fill-v2-competing fill-magnet-uncontested fill-magnet-competing], "six independent missing-hash cases")
      expected = [
        ["a" * 40, nil, "750837cbeaadaf72ab0a852ee3e0517760ca56eb6b8fc3c08b2d9e73348b6bea"],
        [nil, "b" * 64, "4ca14526b2751b640d549ce7caf8ac39438592211a0ec370064d57666a682ad6"],
        [nil, nil, "c" * 64]
      ]
      assert(hash_fill_specs.map { |spec| spec.fetch(:hashes) } == expected, "independent decoded-byte SHA256 known answers")
      cases.each do |test_case|
        arguments = test_case.fetch(:arguments)
        fixtures = test_case.fetch(:fixtures)
        assert(fixtures.first.values_at(:infohash_v1_input, :infohash_v2_input, :magnet_hash_input) == ["NULL::char(40)", "NULL::char(64)", "NULL::char(64)"], "target begins with all identity hashes absent")
        assert(arguments.fetch(:source_guid_input) == "'fill-target'::varchar" && arguments.fetch(:seeders_input) == "17", "actual fill retains target GUID and updates observation")
        peer_guid = test_case.fetch(:hash_fill).fetch(:competing) ? "NULL::varchar" : "'fill-peer'::varchar"
        assert(fixtures.last.fetch(:source_guid_input) == peer_guid, "only GUID-less peer competes with fill")
        [false, true].each do |rollback|
          evidence = hash_fill_test_evidence(test_case, rollback)
          assert(hash_fill_test_valid?(test_case, rollback, evidence), "exact synthetic #{test_case.fetch(:name)} rollback=#{rollback}")
          hash_fill_test_mutations!(test_case, rollback, evidence)
        end
      end
      evidence = hash_fill_test_evidence(cases.first, false)
      evidence["fixtures"] = []
      rejected("requires two real fixtures") { hash_fill_test_valid?(cases.first, false, evidence) }
    end

    def hash_fill_test_evidence(test_case, rollback)
      identities = [1, 3].map do |index|
        { "canonical_torrent_public_id" => "56900000-0000-4000-8000-#{format('%012d', index)}",
          "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-#{format('%012d', index + 1)}",
          "observation_created" => true, "durable_source_created" => true, "canonical_changed" => true }
      end
      clocks = %w[2026-09-12T00:00:01+00:00 2026-09-12T00:00:02+00:00]
      first, baseline = hash_fill_fixture_tables(test_case.fetch(:hash_fill), clocks, identities)
      previous = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      fixtures = [first, baseline].each_with_index.map do |tables, index|
        frame = { "result" => identities.fetch(index), "clock" => clocks.fetch(index),
          "tables_before" => previous, "tables_after" => tables, "tables_finish" => tables }
        previous = tables
        frame
      end
      outcome = { "canonical_torrent_public_id" => identities.last.fetch("canonical_torrent_public_id"),
        "canonical_torrent_source_public_id" => identities.first.fetch("canonical_torrent_source_public_id"),
        "observation_created" => false, "durable_source_created" => false, "canonical_changed" => false }
      frames = Array.new(rollback ? 2 : 1) do |index|
        clock = "2026-09-12T00:00:0#{index + 3}+00:00"
        after = hash_fill_after(test_case.fetch(:hash_fill), baseline, clock, index)
        { "result" => outcome, "clock" => clock, "tables_before" => baseline,
          "tables_after" => after, "tables_finish" => rollback && index.zero? ? baseline : after }
      end
      seed_clock = "2026-09-12T00:00:00+00:00"
      inputs = hash_fill_read_tables(seed_clock)
      Marshal.load(Marshal.dump({ "fixtures" => fixtures, "before" => baseline, "frames" => frames, "seed_clock" => seed_clock, "inputs_before" => inputs, "inputs_after" => inputs }))
    end

    def hash_fill_test_valid?(test_case, rollback, evidence)
      checks = []
      proof = FinalProof.new
      proof.define_singleton_method(:check) { |_name, passed| checks << passed }
      proof.send(:hash_fill_verify!, test_case.fetch(:name), test_case, { rollback: }, evidence)
      checks.length == (rollback ? 5 : 4) && checks.all?
    end

    def hash_fill_test_mutations!(test_case, rollback, evidence)
      IngestionProof::INGESTION_TABLES.each do |table|
        changed = Marshal.load(Marshal.dump(evidence))
        changed.fetch("frames").last.fetch("tables_after").fetch(table) << { "unexpected" => true }
        assert(!hash_fill_test_valid?(test_case, rollback, changed), "reject extra #{table} write")
        # Keep every baseline/continuity image coherently wrong. The independent
        # fixture oracle, not a copy-to-copy comparison, must reject this.
        changed = Marshal.load(Marshal.dump(evidence))
        changed.fetch("fixtures").each do |frame|
          %w[tables_before tables_after tables_finish].each { |key| frame.fetch(key).fetch(table) << { "unexpected" => true } }
        end
        changed.fetch("before").fetch(table) << { "unexpected" => true }
        changed.fetch("frames").each do |frame|
          %w[tables_before tables_after tables_finish].each { |key| frame.fetch(key).fetch(table) << { "unexpected" => true } }
        end
        assert(!hash_fill_test_valid?(test_case, rollback, changed), "reject coherent unrelated #{table} baseline drift")
      end
      mutations = {
        "selected source" => ->(entry) { entry.fetch("frames").last.fetch("result")["canonical_torrent_source_public_id"] = entry.fetch("fixtures").last.fetch("result").fetch("canonical_torrent_source_public_id") },
        "selected canonical" => ->(entry) { entry.fetch("frames").last.fetch("result")["canonical_torrent_public_id"] = entry.fetch("fixtures").first.fetch("result").fetch("canonical_torrent_public_id") },
        "fixture result flag" => ->(entry) { entry.fetch("fixtures").first.fetch("result")["canonical_changed"] = false },
        "fixture extra result field" => ->(entry) { entry.fetch("fixtures").first.fetch("result")["unexpected"] = true },
        "fixture UUID type" => ->(entry) { entry.fetch("fixtures").first.fetch("result")["canonical_torrent_public_id"] = 42 },
        "fixture UUID alias" => ->(entry) { entry.fetch("fixtures").last["result"] = entry.fetch("fixtures").first.fetch("result").dup },
        "observation hash" => ->(entry) { entry.fetch("frames").last.fetch("tables_after").fetch("search_request_source_observation").first["magnet_hash"] = nil },
        "durable hash" => ->(entry) { entry.fetch("frames").last.fetch("tables_after").fetch("canonical_torrent_source").first[test_case.fetch(:hash_fill).fetch(:field)] = "wrong" },
        "sequence gap" => ->(entry) { entry.fetch("frames").last.fetch("tables_after").fetch("canonical_size_sample").last["canonical_size_sample_id"] = 99 },
        "unchanged peer" => ->(entry) { entry.fetch("frames").last.fetch("tables_after").fetch("canonical_torrent_source").last["last_seen_seeders"] = 17 },
        "transaction finish" => ->(entry) { entry.fetch("frames").first["tables_finish"] = {} }
      }
      mutations.each do |name, mutate|
        changed = Marshal.load(Marshal.dump(evidence))
        mutate.call(changed)
        assert(!hash_fill_test_valid?(test_case, rollback, changed), "reject #{name}")
      end
      evidence.fetch("inputs_before").each do |table, rows|
        changed = Marshal.load(Marshal.dump(evidence))
        %w[inputs_before inputs_after].each { |key| changed.fetch(key)[table] = rows.empty? ? [{ "unexpected" => true }] : [] }
        assert(!hash_fill_test_valid?(test_case, rollback, changed), "reject coherent #{table} cardinality drift")
        next if rows.empty?

        rows.first.each_key do |column|
          changed = Marshal.load(Marshal.dump(evidence))
          %w[inputs_before inputs_after].each { |key| changed.fetch(key).fetch(table).first[column] = "wrong" }
          assert(!hash_fill_test_valid?(test_case, rollback, changed), "reject coherent #{table}.#{column} drift")
        end
      end
      return unless test_case.fetch(:hash_fill).fetch(:competing)

      %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].each do |table|
        changed = Marshal.load(Marshal.dump(evidence))
        changed.fetch("frames").last.fetch("tables_after")[table] = []
        assert(!hash_fill_test_valid?(test_case, rollback, changed), "reject missing #{table} consequence")
      end
    end
  end
end
