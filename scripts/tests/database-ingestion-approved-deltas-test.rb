# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionApprovedDeltasTest
    private

    def approved_diagnostic(choice)
      sql = File.binread(File.expand_path("../../crates/revaer-data/init.sql", __dir__))
      offset = 0
      routines = SqlStatements.new(sql).boundaries.filter_map do |boundary|
        statement = sql.byteslice(offset, boundary.byte_count - offset)
        offset = boundary.byte_count
        statement if statement.match?(/^CREATE FUNCTION public\.search_result_ingest_v1\(/)
      end
      raise Failure, "approved diagnostic fixture routine changed" unless routines.length == 1

      pattern = choice == "D4" ? /^        (CREATE TEMP TABLE tmp_policy_rules ON COMMIT DROP AS\n.*?);$/m : /^        (INSERT INTO canonical_external_id \(\n.*?'imdb'.*?);$/m
      match = routines.first.match(pattern)
      raise Failure, "approved diagnostic fixture SQL missing" unless match

      statement = match[1].sub("tmp_policy_rules ON COMMIT DROP AS", "tmp_policy_rules AS")
      statement = statement.sub("\n        WHERE id_value_text IS NOT NULL", "")
      expected = IngestionApprovedDeltas::APPROVED_REJECTIONS.fetch(choice)
      expected.reject { |key, _| key == "statement_sha256" }.merge(
        "detail" => nil, "hint" => nil, "statement" => "SQL statement \"#{statement}\"",
        "routine" => "PL/pgSQL function search_result_ingest_v1(uuid)", "operation" => "SQL statement"
      )
    end

    def approved_pair(choice)
      fixture = existing_outcome_sample
      fixture.each_value do |stage|
        stage.values_at("before", "after").each do |tables|
          tables.fetch("canonical_torrent").each { |row| row["updated_at"] = "2026-09-10T00:00:00+00:00" }
          tables.fetch("canonical_torrent_source").each do |row|
            row.merge!("canonical_torrent_source_id" => 1, "updated_at" => "2026-09-10T00:00:00+00:00")
          end
        end
      end
      stages = existing_compare(fixture)
      stages.each_value do |stage|
        stage.merge!("errors" => [], "hints" => [], "diagnostics" => [], "helpers" => [],
                     "caller_settings" => { "before" => ["error"], "after" => ["error"] })
      end
      before = stages.fetch("fixture").fetch("after")
      before["canonical_torrent_source_context_score"] = [{ "computed_at" => "<fixture-transaction-time>" }]
      before["canonical_size_rollup"] = [{ "updated_at" => "<fixture-transaction-time>", "sample_count" => 1 }]
      diagnostic = approved_diagnostic(choice)
      reference = if choice == "D4"
                    stages.fetch("fixture").merge("states" => %w[00000 42P07], "errors" => [diagnostic.fetch("error")], "diagnostics" => [diagnostic])
                  else
                    stages.fetch("tested").merge("states" => ["42P10"], "results" => [], "errors" => [diagnostic.fetch("error")],
                                                  "diagnostics" => [diagnostic], "before" => before, "after" => before)
                  end
      final = Marshal.load(Marshal.dump(reference))
      final["after"] = Marshal.load(Marshal.dump(final.fetch("after")))
      clock = choice == "D4" ? "<transaction-time:1>" : "<tested-transaction-time>"
      final.fetch("after").fetch("canonical_torrent").first["updated_at"] = clock
      final.fetch("after").fetch("canonical_torrent_source").first["updated_at"] = clock
      final.fetch("after").fetch("canonical_torrent_source_context_score").first["computed_at"] = clock
      final.fetch("after").fetch("canonical_size_rollup").first["updated_at"] = clock
      final.merge!("errors" => [], "diagnostics" => [], "states" => choice == "D4" ? %w[00000 00000] : ["00000"])
      reused = stages.fetch("fixture").fetch("results").first.merge("observation_created" => false, "durable_source_created" => false, "canonical_changed" => false)
      final["results"] = choice == "D4" ? [stages.fetch("fixture").fetch("results").first, reused] : [reused]
      return [reference, final] if choice == "D4"

      # Explicit known answers are independent of the production delta builder.
      observed = "2026-09-10T00:01:00+00:00"
      after = final.fetch("after")
      after.fetch("canonical_torrent").first["imdb_id"] = "tt1234567"
      after.fetch("canonical_torrent_source").first.merge!("last_seen_at" => observed, "last_seen_seeders" => 17)
      after.fetch("canonical_size_rollup").first["sample_count"] = 2
      after.fetch("search_request_source_observation").first.merge!("observed_at" => observed, "seeders" => 17)
      after.fetch("canonical_torrent_source_attr") << {
        "canonical_torrent_source_attr_id" => 3, "canonical_torrent_source_id" => 1, "attr_key" => "imdb_id",
        "value_text" => "tt1234567", "value_int" => nil, "value_bigint" => nil, "value_numeric" => nil, "value_bool" => nil
      }
      after.fetch("search_request_source_observation_attr") << {
        "observation_attr_id" => 4, "observation_id" => 1, "attr_key" => "imdb_id", "value_text" => "tt1234567",
        "value_int" => nil, "value_bigint" => nil, "value_numeric" => nil, "value_bool" => nil, "value_uuid" => nil,
        "created_at" => clock
      }
      after.fetch("canonical_external_id") << {
        "canonical_external_id_id" => 1, "canonical_torrent_id" => 1, "id_type" => "imdb", "id_value_text" => "tt1234567",
        "id_value_int" => nil, "source_canonical_torrent_source_id" => 1, "trust_tier_rank" => 10,
        "first_seen_at" => observed, "last_seen_at" => observed
      }
      after.fetch("canonical_size_sample") << {
        "canonical_size_sample_id" => 2, "canonical_torrent_id" => 1, "observed_at" => observed, "size_bytes" => 1024
      }
      [stages.merge("tested" => reference), stages.merge("tested" => final)]
    end

    def approved_delta_tests!
      { "warm-committed" => "D4", "existing-external-id" => "D5" }.each do |name, choice|
        reference, final = approved_pair(choice)
        original = Marshal.dump([reference, final])
        assert(ingestion_approved_delta(name, reference, final) == "ADR 588 #{choice}", "exact #{choice} correction must be recognized")
        assert(original == Marshal.dump([reference, final]), "#{choice} adjudication must not rewrite evidence")
        assert(ingestion_approved_delta("different-case", reference, final).nil?, "#{choice} correction must not extend to another case")
        assert(ingestion_approved_delta(name, reference, reference).nil?, "shared #{choice} failure must remain failed")
        assert(ingestion_approved_delta(name, final, final).nil?, "missing #{choice} frozen counterexample must be rejected")
        approved_delta_mutations!(name, reference, final, choice)
      end
    end

    def approved_delta_mutations!(name, reference, final, choice)
      IngestionProof::INGESTION_TABLES.each do |table|
        changed = Marshal.load(Marshal.dump(final))
        tested = choice == "D4" ? changed : changed.fetch("tested")
        tested.fetch("after").fetch(table) << { "unapproved" => true }
        assert(ingestion_approved_delta(name, reference, changed).nil?, "#{choice} must reject additional #{table} writes")
      end
      %w[errors details hints states results diagnostics caller_settings helpers].each do |key|
        changed = Marshal.load(Marshal.dump(final))
        tested = choice == "D4" ? changed : changed.fetch("tested")
        tested[key] = ["unapproved"]
        assert(ingestion_approved_delta(name, reference, changed).nil?, "#{choice} must reject changed #{key}")
      end
      %w[error detail hint statement routine line operation location].each do |key|
        changed = Marshal.load(Marshal.dump(reference))
        tested = choice == "D4" ? changed : changed.fetch("tested")
        tested.fetch("diagnostics").first[key] = "unapproved"
        assert(ingestion_approved_delta(name, changed, final).nil?, "#{choice} must reject changed diagnostic #{key}")
      end
      changed = Marshal.load(Marshal.dump(final))
      tested = choice == "D4" ? changed : changed.fetch("tested")
      tested.fetch("results").last["canonical_torrent_public_id"] = "<canonical_torrent:99>"
      assert(ingestion_approved_delta(name, reference, changed).nil?, "#{choice} must reject changed result identity")
      return unless choice == "D5"

      %w[fixture tested].each do |stage|
        changed = Marshal.load(Marshal.dump(final))
        changed.fetch(stage).fetch("before").fetch("canonical_external_id") << { "unapproved" => true }
        assert(ingestion_approved_delta(name, reference, changed).nil?, "D5 must reject #{stage} input drift")
      end
    end
  end
end
