# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Exact ADR 588 counterexamples, not normalization of the frozen reference.
  module IngestionApprovedDeltas
    APPROVED_REJECTIONS = {
      "D4" => {
        "error" => '42P07: relation "tmp_policy_rules" already exists',
        "line" => 1026, "location" => "CreateTableAsRelExists, createas.c:406",
        "statement_sha256" => "b55bc5a5393cb31760dd3d1f0d60e1c1bad9403b4572a013d50d9f766813856e"
      },
      "D5" => {
        "error" => "42P10: there is no unique or exclusion constraint matching the ON CONFLICT specification",
        "line" => 2107, "location" => "infer_arbiter_indexes, plancat.c:920",
        "statement_sha256" => "6fa4994788151a9ac4c4984689bac6626146e6cca2d82e9920a182f09df1b7a6"
      }
    }.freeze
    REFRESH_CLOCKS = {
      "canonical_torrent" => "updated_at", "canonical_torrent_source" => "updated_at",
      "canonical_torrent_source_context_score" => "computed_at", "canonical_size_rollup" => "updated_at"
    }.freeze

    private

    def ingestion_approved_delta(name, reference, final)
      case name
      when "warm-committed"
        "ADR 588 D4" if ingestion_approved_warm?(reference, final)
      when "existing-external-id"
        "ADR 588 D5" if ingestion_approved_external?(reference, final)
      end
    end

    def ingestion_approved_rejection?(value, choice)
      expected = APPROVED_REJECTIONS.fetch(choice)
      return false unless value.values_at("errors", "details", "hints") == [[expected.fetch("error")], [], []]
      return false unless value.fetch("diagnostics").length == 1

      diagnostic = value.fetch("diagnostics").first
      statement = diagnostic.fetch("statement")
      routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |entry| entry.fetch("name") == "search_result_ingest_v1" }
      statement.is_a?(String) && Digest::SHA256.hexdigest(statement) == expected.fetch("statement_sha256") &&
        diagnostic == expected.reject { |key, _| key == "statement_sha256" }.merge(
          "detail" => nil, "hint" => nil, "statement" => statement,
          "routine" => "PL/pgSQL function #{routine.fetch('signature')}", "operation" => "SQL statement"
        )
    end

    def ingestion_approved_success(value, results, after)
      value.merge("states" => Array.new(results.length, "00000"), "results" => results,
                  "errors" => [], "details" => [], "hints" => [], "diagnostics" => [], "after" => after)
    end

    def ingestion_approved_result(created:)
      { "canonical_torrent_public_id" => "<canonical_torrent:1>",
        "canonical_torrent_source_public_id" => "<canonical_torrent_source:1>",
        "observation_created" => created, "durable_source_created" => created, "canonical_changed" => created }
    end

    def ingestion_approved_refresh(tables, clock)
      raise Failure, "approved ingestion table inventory changed" unless tables.keys.sort == IngestionProof::INGESTION_TABLES.sort

      tables.to_h do |table, rows|
        column = REFRESH_CLOCKS[table]
        next [table, rows] unless column

        unless rows.length == 1 && rows.first.key?(column)
          raise Failure, "approved ingestion refresh requires exactly one existing row"
        end
        [table, [rows.first.merge(column => clock)]]
      end
    end

    def ingestion_approved_warm?(reference, final)
      return false unless reference.fetch("states") == %w[00000 42P07] && ingestion_approved_rejection?(reference, "D4")
      return false unless reference.fetch("results") == [ingestion_approved_result(created: true)] &&
                          reference.fetch("before").values.all?(&:empty?)

      after = ingestion_approved_refresh(reference.fetch("after"), "<transaction-time:1>")
      results = [ingestion_approved_result(created: true), ingestion_approved_result(created: false)]
      final == ingestion_approved_success(reference, results, after)
    end

    def ingestion_approved_external?(reference, final)
      return false unless reference.keys == IngestionExisting::EXISTING_STAGES && final.keys == reference.keys
      return false unless reference.fetch("fixture") == final.fetch("fixture")

      old = reference.fetch("tested")
      return false unless old.fetch("states") == ["42P10"] && old.fetch("results").empty? &&
                          ingestion_approved_rejection?(old, "D5") && old.fetch("before") == old.fetch("after")
      return false unless reference.fetch("fixture").fetch("after") == old.fetch("before")

      expected = ingestion_approved_success(old, [ingestion_approved_result(created: false)],
                                           ingestion_approved_external_tables(old.fetch("before")))
      final.fetch("tested") == expected && ingestion_existing_outcome?("existing-external-id", final)
    end

    def ingestion_approved_external_tables(before)
      clock = "<tested-transaction-time>"
      observed = "2026-09-10T00:01:00+00:00"
      tables = ingestion_approved_refresh(before, clock)
      tables["canonical_torrent"] = [tables.fetch("canonical_torrent").first.merge("imdb_id" => "tt1234567")]
      tables["canonical_torrent_source"] = [tables.fetch("canonical_torrent_source").first.merge("last_seen_at" => observed, "last_seen_seeders" => 17)]
      tables["canonical_size_rollup"] = [tables.fetch("canonical_size_rollup").first.merge("sample_count" => 2)]
      observations = before.fetch("search_request_source_observation")
      raise Failure, "approved external-id fixture requires one observation" unless observations.length == 1

      tables["search_request_source_observation"] = [observations.first.merge("observed_at" => observed, "seeders" => 17)]
      tables["canonical_torrent_source_attr"] = before.fetch("canonical_torrent_source_attr") + [{
        "canonical_torrent_source_attr_id" => 3, "canonical_torrent_source_id" => 1,
        "attr_key" => "imdb_id", "value_text" => "tt1234567", "value_int" => nil,
        "value_bigint" => nil, "value_numeric" => nil, "value_bool" => nil
      }]
      tables["search_request_source_observation_attr"] = before.fetch("search_request_source_observation_attr") + [{
        "observation_attr_id" => 4, "observation_id" => 1,
        "attr_key" => "imdb_id", "value_text" => "tt1234567", "value_int" => nil,
        "value_bigint" => nil, "value_numeric" => nil, "value_bool" => nil,
        "value_uuid" => nil, "created_at" => clock
      }]
      raise Failure, "approved external-id fixture must not contain an identifier" unless before.fetch("canonical_external_id").empty?

      tables["canonical_external_id"] = [{
        "canonical_external_id_id" => 1, "canonical_torrent_id" => 1, "id_type" => "imdb",
        "id_value_text" => "tt1234567", "id_value_int" => nil, "source_canonical_torrent_source_id" => 1,
        "trust_tier_rank" => 10, "first_seen_at" => observed, "last_seen_at" => observed
      }]
      tables["canonical_size_sample"] = before.fetch("canonical_size_sample") + [{
        "canonical_size_sample_id" => 2, "canonical_torrent_id" => 1,
        "observed_at" => observed, "size_bytes" => 1024
      }]
      tables
    end
  end
end
