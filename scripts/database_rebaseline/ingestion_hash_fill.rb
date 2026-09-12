# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Missing source hashes and competing GUID-less sources, not full D3 closure.
  module IngestionHashFill
    HASH_FILL_COLUMNS = %w[infohash_v1 infohash_v2 magnet_hash].freeze
    HASH_FILL_OBSERVED = "2026-09-10T00:01:00+00:00"

    private

    def hash_fill_specs
      v1 = "a" * 40
      v2 = "b" * 64
      magnet = "c" * 64
      [
        { kind: "v1", field: "infohash_v1", value: v1, sql: "repeat('a',40)::char(40)", hashes: [v1, nil, Digest::SHA256.hexdigest([v1].pack("H*"))] },
        { kind: "v2", field: "infohash_v2", value: v2, sql: "repeat('b',64)::char(64)", hashes: [nil, v2, Digest::SHA256.hexdigest([v2].pack("H*"))] },
        { kind: "magnet", field: "magnet_hash", value: magnet, sql: "repeat('c',64)::char(64)", hashes: [nil, nil, magnet] }
      ]
    end

    def wrapper_hash_fill_cases
      base = wrapper_arguments("fill-target", title: "Fill proof").merge(
        infohash_v1_input: "NULL::char(40)", infohash_v2_input: "NULL::char(64)", magnet_hash_input: "NULL::char(64)"
      )
      hash_fill_specs.flat_map do |spec|
        [false, true].map do |competing|
          field = "#{spec.fetch(:field)}_input".to_sym
          peer = base.merge(field => spec.fetch(:sql), source_guid_input: competing ? "NULL::varchar" : "'fill-peer'::varchar")
          arguments = base.merge(field => spec.fetch(:sql), observed_at_input: "'2026-09-10T00:01:00Z'::timestamptz", seeders_input: "17")
          { name: "fill-#{spec.fetch(:kind)}-#{competing ? 'competing' : 'uncontested'}",
            fixtures: [base, peer], arguments:, wrapper: true, hash_fill: spec.merge(competing:) }
        end
      end
    end

    def hash_fill_verify!(name, test_case, session, evidence)
      fixtures = evidence.fetch("fixtures")
      raise Failure, "hash-fill proof requires two real fixtures" unless fixtures.length == 2

      identities = fixtures.map { |frame| frame.fetch("result") }
      uuids = identities.flat_map { |result| result.values_at("canonical_torrent_public_id", "canonical_torrent_source_public_id") }
      valid = uuids.uniq.length == 4 && uuids.all? { |value| value.is_a?(String) && IngestionProof::INGESTION_UUID.match?(value) }
      check("#{name} independently validated fixture UUIDs", valid)
      clocks = fixtures.map { |frame| frame.fetch("clock") }
      spec = test_case.fetch(:hash_fill)
      first, baseline = hash_fill_fixture_tables(spec, clocks, identities)
      inputs = hash_fill_read_tables(evidence.fetch("seed_clock"))
      check("#{name} exact independently specified read inputs", evidence.fetch("inputs_before") == inputs && evidence.fetch("inputs_after") == inputs)
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      previous = empty
      fixture_valid = fixtures.zip([first, baseline]).all? do |frame, expected|
        match = frame.fetch("tables_before") == previous && frame.fetch("tables_after") == expected && frame.fetch("tables_finish") == expected &&
          frame.fetch("result").keys.sort == %w[canonical_changed canonical_torrent_public_id canonical_torrent_source_public_id durable_source_created observation_created] &&
          frame.fetch("result").values_at("observation_created", "durable_source_created", "canonical_changed") == [true, true, true]
        previous = expected
        match
      end
      check("#{name} exact independently specified fixture tables", fixture_valid && evidence.fetch("before") == baseline)
      expected_result = {
        "canonical_torrent_public_id" => identities.fetch(1).fetch("canonical_torrent_public_id"),
        "canonical_torrent_source_public_id" => identities.fetch(0).fetch("canonical_torrent_source_public_id"),
        "observation_created" => false, "durable_source_created" => false, "canonical_changed" => false
      }
      evidence.fetch("frames").each_with_index do |frame, index|
        expected = hash_fill_after(spec, baseline, frame.fetch("clock"), index)
        finish = session[:rollback] && index.zero? ? baseline : expected
        check("#{name} call #{index + 1} exact hash fill conflicts and complete state",
          frame.fetch("result") == expected_result && frame.fetch("tables_before") == baseline &&
          frame.fetch("tables_after") == expected && frame.fetch("tables_finish") == finish)
      end
    end

    def hash_fill_read_tables(clock)
      # Exact proof-seed values and frozen 0013/0014/0019/0023 column defaults.
      {
        "indexer_definition" => [{ "indexer_definition_id" => 569001, "upstream_source" => "cardigann",
          "upstream_slug" => "ingestion-proof", "display_name" => "Ingestion proof", "protocol" => "torrent", "engine" => "cardigann",
          "schema_version" => 1, "definition_hash" => "a" * 64, "is_deprecated" => false, "created_at" => clock, "updated_at" => clock }],
        "indexer_instance" => [{ "indexer_instance_id" => 569001, "indexer_instance_public_id" => "56900000-0000-4000-8000-000000000001",
          "indexer_definition_id" => 569001, "display_name" => "Ingestion proof", "is_enabled" => true, "migration_state" => "ready",
          "migration_detail" => nil, "enable_rss" => true, "enable_automatic_search" => true, "enable_interactive_search" => true,
          "priority" => 50, "trust_tier_key" => "public", "routing_policy_id" => nil, "connect_timeout_ms" => 5000,
          "read_timeout_ms" => 15000, "max_parallel_requests" => 2, "created_by_user_id" => 0, "updated_by_user_id" => 0,
          "created_at" => clock, "updated_at" => clock, "deleted_at" => nil }],
        "policy_snapshot" => [{ "policy_snapshot_id" => 569001, "created_at" => clock, "snapshot_hash" => "b" * 64,
          "ref_count" => 0, "excluded_disabled_count" => 0, "excluded_expired_count" => 0 }],
        "search_request" => [{ "search_request_id" => 569001, "search_request_public_id" => "56900000-0000-4000-8000-000000000002",
          "user_id" => nil, "search_profile_id" => nil, "policy_set_id" => nil, "policy_snapshot_id" => 569001,
          "requested_media_domain_id" => nil, "effective_media_domain_id" => nil, "query_text" => "Ingestion proof",
          "query_type" => "free_text", "torznab_mode" => nil, "page_size" => 10, "season_number" => nil, "episode_number" => nil,
          "created_at" => clock, "canceled_at" => nil, "finished_at" => nil, "status" => "running", "failure_class" => nil, "error_detail" => nil }],
        "search_request_indexer_run" => [{ "search_request_indexer_run_id" => 1, "search_request_id" => 569001,
          "indexer_instance_id" => 569001, "started_at" => nil, "finished_at" => nil, "next_attempt_at" => nil,
          "attempt_count" => 0, "rate_limited_attempt_count" => 0, "last_error_class" => nil, "last_rate_limit_scope" => nil,
          "last_correlation_id" => nil, "status" => "queued", "error_class" => nil, "error_detail" => nil,
          "items_seen_count" => 0, "items_emitted_count" => 0, "canonical_added_count" => 0 }],
        "canonical_torrent_source_base_score" => []
      }
    end

    def hash_fill_fixture_tables(spec, clocks, identities)
      parts = [0, 1].map { |index| hash_fill_fixture_part(spec, index, clocks.fetch(index), identities.fetch(index)) }
      [parts.first, parts.first.to_h { |table, rows| [table, rows + parts.last.fetch(table)] }]
    end

    def hash_fill_fixture_part(spec, index, clock, identity)
      # Reuse the existing complete-column baseline with no attribute/signal rows.
      shape = { title: "Fill proof", normalized: "fill proof", answers: [], signals: [], release: nil }
      tables = attributes_initial_tables(shape, clock, identity)
      number = index + 1
      hashes = index.zero? ? [nil, nil, nil] : spec.fetch(:hashes)
      guid = index.zero? ? "fill-target" : (spec.fetch(:competing) ? nil : "fill-peer")
      canonical = tables.fetch("canonical_torrent").first
      canonical.merge!("canonical_torrent_id" => number, "size_bytes" => 1024,
        "identity_strategy" => index.zero? ? "title_size_fallback" : spec.fetch(:field),
        "identity_confidence" => index.zero? ? 0.6 : (spec.fetch(:kind) == "magnet" ? 0.85 : 1.0),
        "title_size_hash" => index.zero? ? Digest::SHA256.hexdigest("fill proof|1024") : nil)
      source = tables.fetch("canonical_torrent_source").first
      source.merge!("canonical_torrent_source_id" => number, "source_guid" => guid, "size_bytes" => 1024)
      observation = tables.fetch("search_request_source_observation").first
      observation.merge!("observation_id" => number, "canonical_torrent_id" => number,
        "canonical_torrent_source_id" => number, "source_guid" => guid, "size_bytes" => 1024)
      [canonical, source, observation].each { |row| HASH_FILL_COLUMNS.zip(hashes).each { |key, value| row[key] = value } }
      score = tables.fetch("canonical_torrent_source_context_score").first
      score.merge!("canonical_torrent_source_context_score_id" => number, "canonical_torrent_id" => number, "canonical_torrent_source_id" => number)
      tables["canonical_torrent_best_source_context"] = [{ "canonical_torrent_best_source_context_id" => number,
        "context_key_type" => "search_request", "context_key_id" => 569001, "canonical_torrent_id" => number,
        "canonical_torrent_source_id" => number, "computed_at" => clock }]
      tables.fetch("search_request_canonical").first.merge!("search_request_canonical_id" => number, "canonical_torrent_id" => number)
      tables.fetch("search_page_item").first.merge!("search_page_item_id" => number, "search_request_canonical_id" => number, "position" => number)
      unless index.zero?
        tables["search_page"] = []
        tables["canonical_size_sample"] = [hash_fill_sample(1, "2026-09-10T00:00:00+00:00")]
        tables["canonical_size_rollup"] = [{ "canonical_size_rollup_id" => 1, "canonical_torrent_id" => 2,
          "sample_count" => 1, "size_median" => 1024, "size_min" => 1024, "size_max" => 1024, "updated_at" => clock }]
      end
      tables
    end

    def hash_fill_after(spec, before, clock, ordinal)
      tables = before.to_h { |table, rows| [table, rows.map(&:dup)] }
      tables.fetch("canonical_torrent").last["updated_at"] = clock
      hashes = spec.fetch(:hashes).dup
      hashes[HASH_FILL_COLUMNS.index(spec.fetch(:field))] = nil if spec.fetch(:competing)
      source = tables.fetch("canonical_torrent_source").first
      HASH_FILL_COLUMNS.zip(hashes).each { |key, value| source[key] = value }
      source.merge!("last_seen_at" => HASH_FILL_OBSERVED, "last_seen_seeders" => 17, "updated_at" => clock)
      observation = tables.fetch("search_request_source_observation").first
      HASH_FILL_COLUMNS.zip(spec.fetch(:hashes)).each { |key, value| observation[key] = value }
      observation.merge!("canonical_torrent_id" => 2, "observed_at" => HASH_FILL_OBSERVED, "seeders" => 17)
      score = tables.fetch("canonical_torrent_source_context_score").first.merge(
        "canonical_torrent_source_context_score_id" => ordinal + 3, "canonical_torrent_id" => 2, "computed_at" => clock)
      tables.fetch("canonical_torrent_source_context_score") << score
      tables.fetch("canonical_torrent_best_source_context").last.merge!("canonical_torrent_source_id" => 1, "computed_at" => clock)
      tables.fetch("canonical_size_sample") << hash_fill_sample(ordinal + 2, HASH_FILL_OBSERVED)
      tables.fetch("canonical_size_rollup").first.merge!("sample_count" => 2, "updated_at" => clock)
      hash_fill_conflict_tables!(tables, spec.fetch(:value), clock, ordinal + 1) if spec.fetch(:competing)
      tables
    end

    def hash_fill_sample(id, observed)
      { "canonical_size_sample_id" => id, "canonical_torrent_id" => 2, "observed_at" => observed, "size_bytes" => 1024 }
    end

    def hash_fill_conflict_tables!(tables, value, clock, id)
      tables["source_metadata_conflict"] = [{ "source_metadata_conflict_id" => id, "canonical_torrent_source_id" => 1,
        "conflict_type" => "hash", "existing_value" => value, "incoming_value" => value, "observed_at" => HASH_FILL_OBSERVED,
        "resolved_at" => nil, "resolved_by_user_id" => nil, "resolution" => nil, "resolution_note" => nil }]
      tables["source_metadata_conflict_audit_log"] = [{ "source_metadata_conflict_audit_log_id" => id, "conflict_id" => id,
        "action" => "created", "actor_user_id" => 0, "occurred_at" => clock, "note" => nil }]
      tables["indexer_health_event"] = [{ "indexer_health_event_id" => id, "indexer_instance_id" => 569001,
        "occurred_at" => HASH_FILL_OBSERVED, "event_type" => "identity_conflict", "latency_ms" => nil,
        "http_status" => nil, "error_class" => nil, "detail" => "hash" }]
    end
  end
end
