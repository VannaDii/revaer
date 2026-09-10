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
  end
end
