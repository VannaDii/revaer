# frozen_string_literal: true

require "json"
require_relative "../database_rebaseline/native_cancellation_comparison"

# Self-contained synthetic cancellation pairs; no live provenance claim.
module CancellationPairUnitFixtures
  CASES = %w[reference-cold reference-warm_committed final-cold final-warm_committed].freeze
  BOUNDARIES = %w[before prepared cancelled after].freeze
  CLOCK_COLUMNS = {
    "canonical_torrent" => %w[created_at updated_at],
    "canonical_torrent_source" => %w[created_at updated_at],
    "canonical_torrent_source_context_score" => %w[computed_at],
    "canonical_torrent_best_source_context" => %w[computed_at],
    "canonical_size_rollup" => %w[updated_at],
    "search_request_canonical" => %w[first_seen_at]
  }.freeze

  # Session/error fixtures are ported from database-ingestion-cancellation-test;
  # row and input fixtures from database-ingestion-sample-race-test and
  # database-ingestion-attributes-test. All clocks here are synthetic.
  def self.fixture(variant, cache_state)
    database = "ingestion_pool_cancel_unit"
    role = variant == "reference" ? "postgres" : "proof_runtime_unit"
    backend = { "pid" => 123, "database" => database, "session_role" => role, "current_role" => role,
                "superuser" => variant == "reference", "create_role" => variant == "reference", "bypass_rls" => variant == "reference",
                "server_version" => "160014", "conflict_setting" => "error" }
    failed = { "name" => "cancelled", "before" => backend.merge("conflict_setting" => nil), "after" => backend,
               "outcome" => { "kind" => "database_error", "operation" => "search result ingest", "state" => "57014",
                              "message" => "canceling statement due to user request", "detail" => nil } }
    row = { "canonical" => "56900000-0000-4000-8000-000000000010", "source" => "56900000-0000-4000-8000-000000000011",
            "observation_created" => true, "durable_source_created" => true, "canonical_changed" => true }
    recovered = { "name" => "recovery", "before" => backend, "after" => backend, "outcome" => { "kind" => "success", "row" => row } }
    value = { "cache_state" => cache_state,
      "preparation" => { "frames" => [], "session" => failed.fetch("before"),
        "start_signal" => { "database" => database, "pid" => 123, "cache_state" => cache_state } },
      "locker" => { "pid" => 321, "database" => database, "role" => "postgres" },
      "blocked" => { "pid" => 123, "database" => database, "role" => role, "application" => "revaer-ingestion-cancellation-proof",
                     "state" => "active", "wait_type" => "Lock", "wait_event" => "relation", "blockers" => [321],
                     "source_insert_wait" => true, "canonical_write_lock" => true, "query" => "SELECT * FROM search_result_ingest(" },
      "checkpoint" => [failed], "cancel_result" => "t", "frames" => [failed, recovered] }
    return value if cache_state == "cold"

    warm_row = row.merge("canonical" => "56900000-0000-4000-8000-000000000020", "source" => "56900000-0000-4000-8000-000000000021")
    warmup = { "name" => "warmup", "before" => backend.merge("conflict_setting" => nil), "after" => backend,
               "outcome" => { "kind" => "success", "row" => warm_row } }
    failed["before"] = backend
    value["preparation"]["frames"] = [warmup]
    value["preparation"]["session"] = backend
    value["checkpoint"] = [warmup, failed]
    value["frames"] = [warmup, failed, recovered]
    if variant == "reference"
      recovered["outcome"] = { "kind" => "database_error", "operation" => "search result ingest", "state" => "42P07",
        "message" => 'relation "tmp_policy_rules" already exists', "detail" => nil }
    end
    value
  end

  def self.inputs(clock, rank, trust_key: "public")
    inputs = RevaerDatabaseRebaseline::CancellationPairComparison::INPUTS.to_h { |table| [table, []] }
    inputs["indexer_definition"] = [{ "indexer_definition_id" => 569001, "created_at" => clock, "updated_at" => clock, "upstream_slug" => "ingestion-proof", "definition_hash" => "a" * 64 }]
    inputs["indexer_instance"] = [{ "indexer_instance_id" => 569001, "created_at" => clock, "updated_at" => clock,
      "indexer_instance_public_id" => "56900000-0000-4000-8000-000000000001", "indexer_definition_id" => 569001,
      "is_enabled" => true, "deleted_at" => nil, "migration_state" => "ready", "trust_tier_key" => trust_key }]
    inputs["policy_snapshot"] = [{ "policy_snapshot_id" => 569001, "created_at" => clock, "snapshot_hash" => "b" * 64 }]
    inputs["search_request"] = [{ "search_request_id" => 569001, "created_at" => clock,
      "search_request_public_id" => "56900000-0000-4000-8000-000000000002", "policy_snapshot_id" => 569001,
      "status" => "running", "page_size" => 10, "query_text" => "Ingestion proof", "finished_at" => nil, "canceled_at" => nil, "failure_class" => nil }]
    ranks = [["semi_private", 20], ["private", 30], ["invite_only", 40]]
    ranks.unshift(["public", rank]) unless rank.nil?
    inputs["trust_tier"] = ranks.map do |key, value|
      { "trust_tier_key" => key, "created_at" => clock, "rank" => value }
    end
    inputs["media_domain"] = [{ "media_domain_key" => "movies", "created_at" => clock }]
    inputs["search_request_indexer_run"] = [{ "search_request_id" => 569001, "indexer_instance_id" => 569001, "status" => "queued" }]
    inputs
  end

  def self.tables(seed, clock)
    tables = RevaerDatabaseRebaseline::CancellationPairComparison::TABLES.to_h { |table| [table, []] }
    magnet = "750837cbeaadaf72ab0a852ee3e0517760ca56eb6b8fc3c08b2d9e73348b6bea"
    canonical_uuid = "56900000-0000-4000-8000-000000000090"
    source_uuid = "56900000-0000-4000-8000-000000000091"
    observed = "2026-09-10T00:25:00+00:00"
    tables["canonical_torrent"] = [{ "canonical_torrent_id" => 1, "canonical_torrent_public_id" => canonical_uuid,
      "identity_confidence" => 1.0, "identity_strategy" => "infohash_v1", "infohash_v1" => "a" * 40,
      "infohash_v2" => nil, "magnet_hash" => magnet, "title_size_hash" => nil, "imdb_id" => nil, "tmdb_id" => nil,
      "tvdb_id" => nil, "ids_confidence" => nil, "title_display" => "Size proof", "title_normalized" => "size proof",
      "size_bytes" => 1024, "created_at" => seed, "updated_at" => clock }]
    tables["canonical_torrent_source"] = [{ "canonical_torrent_source_id" => 1, "indexer_instance_id" => 569001,
      "canonical_torrent_source_public_id" => source_uuid, "source_guid" => "size-source", "infohash_v1" => "a" * 40,
      "infohash_v2" => nil, "magnet_hash" => magnet, "title_normalized" => "size proof", "size_bytes" => 1024,
      "last_seen_at" => observed, "last_seen_seeders" => 5, "last_seen_leechers" => 2, "last_seen_published_at" => nil,
      "last_seen_download_url" => nil, "last_seen_magnet_uri" => nil, "last_seen_details_url" => nil, "last_seen_uploader" => nil,
      "created_at" => seed, "updated_at" => clock }]
    tables["canonical_size_rollup"] = [{ "canonical_size_rollup_id" => 1, "canonical_torrent_id" => 1,
      "sample_count" => 26, "size_median" => 1024, "size_min" => 1024, "size_max" => 1024, "updated_at" => "2026-09-09T00:00:00+00:00" }]
    tables["canonical_torrent_source_context_score"] = [{ "canonical_torrent_source_context_score_id" => 1,
      "context_key_type" => "search_request", "context_key_id" => 569001, "canonical_torrent_id" => 1,
      "canonical_torrent_source_id" => 1, "score_total_context" => 0.0, "score_policy_adjust" => 0.0,
      "score_tag_adjust" => 0.0, "is_dropped" => false, "computed_at" => clock }]
    tables["canonical_torrent_best_source_context"] = [{ "canonical_torrent_best_source_context_id" => 1,
      "context_key_type" => "search_request", "context_key_id" => 569001, "canonical_torrent_id" => 1,
      "canonical_torrent_source_id" => 1, "computed_at" => clock }]
    tables["search_request_source_observation"] = [{ "observation_id" => 1, "search_request_id" => 569001, "indexer_instance_id" => 569001,
      "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "observed_at" => observed, "seeders" => 5, "leechers" => 2,
      "published_at" => nil, "uploader" => nil, "source_guid" => "size-source", "details_url" => nil, "download_url" => nil,
      "magnet_uri" => nil, "title_raw" => "Size proof", "size_bytes" => 1024, "infohash_v1" => "a" * 40, "infohash_v2" => nil,
      "magnet_hash" => magnet, "guid_conflict" => false, "was_downranked" => false, "was_flagged" => false }]
    tables["search_request_canonical"] = [{ "search_request_canonical_id" => 1, "search_request_id" => 569001,
      "canonical_torrent_id" => 1, "first_seen_at" => clock }]
    tables["search_page"] = [{ "search_page_id" => 1, "search_request_id" => 569001, "page_number" => 1, "sealed_at" => nil }]
    tables["search_page_item"] = [{ "search_page_item_id" => 1, "search_page_id" => 1, "search_request_canonical_id" => 1, "position" => 1 }]
    tables
  end

  def self.load
    CASES.to_h do |name|
      variant, cache = name.split("-", 2)
      sample = fixture(variant, cache)
      seed = "2040-01-01T00:00:00+00:00"
      read_inputs = inputs(seed, 10)
      BOUNDARIES.each { |boundary| sample["inputs_#{boundary}"] = clone(read_inputs) }
      images = {}
      sample.fetch("frames").each do |frame|
        next unless frame.fetch("outcome").fetch("kind") == "success"

        stage = frame.fetch("name")
        id = stage == "warmup" ? 1 : 2
        rows = tables(seed, seed)
        rows.each_value do |items|
          items.each do |row|
            row.keys.each { |key| row[key] = id if key.end_with?("_id") && row[key] == 1 }
            row["source_guid"] = stage == "warmup" ? "pool-proof-warmup" : "pool-proof-source" if row.key?("source_guid")
            row["infohash_v1"] = (stage == "warmup" ? "b" : "a") * 40 if row.key?("infohash_v1")
          end
        end
        outcome = frame.fetch("outcome").fetch("row")
        rows.fetch("canonical_torrent").first["canonical_torrent_public_id"] = outcome.fetch("canonical")
        rows.fetch("canonical_torrent_source").first["canonical_torrent_source_public_id"] = outcome.fetch("source")
        images[stage] = rows
      end
      empty = RevaerDatabaseRebaseline::CancellationPairComparison::TABLES.to_h { |table| [table, []] }
      prepared = images.fetch("warmup", empty)
      sample["before"] = clone(empty)
      sample["prepared"] = clone(prepared)
      sample["cancelled"] = clone(prepared)
      sample["after"] = empty.to_h { |table, _| [table, images.values.flat_map { |image| image.fetch(table) }] }
      [name, clone(sample)]
    end
  end

  def self.clone(value)
    JSON.parse(JSON.generate(value))
  end

  def self.pair_side(sample, variant, cache, side)
    evidence = clone(sample)
    clock = { "seed" => "2040-01-0#{side + 1}T00:00:00.000001+00:00",
              "cancelled" => "2040-01-0#{side + 1}T00:02:00.000003+00:00" }
    clock["warmup"] = "2040-01-0#{side + 1}T00:01:00.000002+00:00" if cache == "warm_committed"
    clock["recovery"] = "2040-01-0#{side + 1}T00:03:00.000004+00:00" unless variant == "reference" && cache == "warm_committed"
    ids = %w[warmup recovery].each_with_index.to_h do |stage, index|
      [stage, { "canonical" => format("%08x-1111-4111-8111-111111111111", 100 + side * 10 + index),
                "source" => format("%08x-2222-4222-8222-222222222222", 200 + side * 10 + index) }]
    end
    source_stages = evidence.fetch("after").fetch("canonical_torrent_source").to_h do |row|
      [row.fetch("canonical_torrent_source_id"), row.fetch("source_guid") == "pool-proof-warmup" ? "warmup" : "recovery"]
    end
    canonical_stages = evidence.fetch("after").fetch("canonical_torrent").to_h do |row|
      [row.fetch("canonical_torrent_id"), row.fetch("infohash_v1") == "b" * 40 ? "warmup" : "recovery"]
    end
    BOUNDARIES.each do |boundary|
      CLOCK_COLUMNS.each do |table, columns|
        evidence.fetch(boundary).fetch(table).each do |row|
          stage = table == "canonical_torrent_source" ? source_stages.fetch(row.fetch("canonical_torrent_source_id")) : canonical_stages.fetch(row.fetch("canonical_torrent_id"))
          columns.each { |column| row[column] = clock.fetch(stage) }
          row["canonical_torrent_public_id"] = ids.fetch(stage).fetch("canonical") if table == "canonical_torrent"
          row["canonical_torrent_source_public_id"] = ids.fetch(stage).fetch("source") if table == "canonical_torrent_source"
        end
      end
      %w[indexer_definition indexer_instance policy_snapshot search_request].each do |table|
        evidence.fetch("inputs_#{boundary}").fetch(table).each do |row|
          row["created_at"] = clock.fetch("seed")
          row["updated_at"] = clock.fetch("seed") if %w[indexer_definition indexer_instance].include?(table)
        end
      end
    end
    preparation = evidence.fetch("preparation")
    groups = [evidence.fetch("frames"), evidence.fetch("checkpoint"), preparation.fetch("frames")]
    groups.each do |frames|
      frames.each do |frame|
        next unless frame.fetch("outcome").fetch("kind") == "success"

        %w[canonical source].each { |key| frame.fetch("outcome").fetch("row")[key] = ids.fetch(frame.fetch("name")).fetch(key) }
      end
    end
    sessions = groups.flatten.flat_map { |frame| [frame.fetch("before"), frame.fetch("after")] }
    sessions += [preparation.fetch("session"), preparation.fetch("start_signal"), evidence.fetch("blocked")]
    sessions.each { |session| session["pid"] = 1000 + side; session["database"] = "unit-cancellation-#{side}" }
    evidence.fetch("locker")["pid"] = 2000 + side
    evidence.fetch("locker")["database"] = "unit-cancellation-#{side}"
    evidence.fetch("blocked")["blockers"] = [2000 + side]
    [evidence, clock]
  end
end

class NativeCancellationComparisonTest
  def initialize
    @assertions = 0
  end

  def assert(condition, label)
    raise "FAIL: #{label}" unless condition
    @assertions += 1
  end

  def reject(label)
    begin
      yield
    rescue RevaerDatabaseRebaseline::Failure
      @assertions += 1
      return
    end
    raise "FAIL: accepted #{label}"
  end

  def compare(left, right, lc, rc)
    @comparator.compare!(left, right, plain_clocks: lc, observed_clocks: rc)
  end

  def mutation(label, left, right, lc, rc)
    changed = CancellationPairUnitFixtures.clone(right)
    yield changed
    reject(label) { compare(left, changed, lc, rc) }
  end

  def shared_mutation(label, right, rc)
    changed = CancellationPairUnitFixtures.clone(right)
    yield changed
    reject("shared #{label}") { compare(changed, changed, rc, rc) }
  end

  def leaves(value, path = [], result = [])
    case value
    when Hash then value.each { |key, item| leaves(item, path + [key], result) }
    when Array then value.each_with_index { |item, index| leaves(item, path + [index], result) }
    else result << [path, value]
    end
    result
  end

  def run
    CancellationPairUnitFixtures.load.each do |name, sample|
      variant, cache = name.split("-", 2)
      @comparator = RevaerDatabaseRebaseline::CancellationPairComparison.new(variant: variant, cache_state: cache)
      left, lc = CancellationPairUnitFixtures.pair_side(sample, variant, cache, 0)
      right, rc = CancellationPairUnitFixtures.pair_side(sample, variant, cache, 1)
      original_left = CancellationPairUnitFixtures.clone(left)
      original_right = CancellationPairUnitFixtures.clone(right)
      result = compare(left, right, lc, rc)
      assert(result.fetch(:plain).eql?(result.fetch(:observed)), "#{name} allowed drift")
      assert(left.eql?(original_left) && right.eql?(original_right), "inputs unmodified")
      assert(result.fetch(:plain).keys.sort == left.keys.sort, "all top-level keys retained")
      %w[before prepared cancelled after].each do |boundary|
        assert(result.fetch(:plain).fetch(boundary).keys.length == 18, "18 write tables")
        assert(result.fetch(:plain).fetch("inputs_#{boundary}").keys.length == 19, "19 input tables")
        mutation("missing image", left, right, lc, rc) { |v| v.delete(boundary) }
        mutation("missing read image", left, right, lc, rc) { |v| v.delete("inputs_#{boundary}") }
        %w[indexer_definition indexer_instance policy_snapshot search_request].each do |table|
          shared_mutation("wrong seed clock", right, rc) do |v|
            v.fetch("inputs_#{boundary}").fetch(table).first["created_at"] = rc.fetch("cancelled")
          end
        end
      end
      mutation("changed setting", left, right, lc, rc) { |v| v.fetch("frames").last.fetch("after")["conflict_setting"] = "use_column" }
      mutation("changed error", left, right, lc, rc) { |v| v.fetch("frames").find { |f| f.fetch("name") == "cancelled" }.fetch("outcome")["detail"] = "extra" }
      mutation("changed row", left, right, lc, rc) { |v| v.fetch("after").fetch("canonical_torrent").first["size_bytes"] += 1 }
      mutation("removed key", left, right, lc, rc) { |v| v.fetch("after").fetch("canonical_torrent").first.delete("title_display") }
      mutation("new key", left, right, lc, rc) { |v| v["unanticipated"] = true }
      mutation("new table", left, right, lc, rc) { |v| v.fetch("after")["unanticipated"] = [{ "value" => "kept" }] }
      shared_mutation("source guid mismatch", right, rc) { |v| v.fetch("after").fetch("canonical_torrent_source").last["source_guid"] = "not-declared" }
      shared_mutation("source hash mismatch", right, rc) { |v| v.fetch("after").fetch("canonical_torrent_source").last["infohash_v1"] = "c" * 40 }
      shared_mutation("canonical hash mismatch", right, rc) { |v| v.fetch("after").fetch("canonical_torrent").last["infohash_v1"] = "c" * 40 }
      shared_mutation("observation binding", right, rc) { |v| v.fetch("after").fetch("search_request_source_observation").last["canonical_torrent_id"] = 999 }
      shared_mutation("wrong application clock", right, rc) { |v| v.fetch("after").fetch("canonical_torrent").last["updated_at"] = rc.fetch("cancelled") }
      shared_mutation("rollback mutation", right, rc) { |v| v.fetch("cancelled").fetch("canonical_torrent_source_attr") << { "value" => 1 } }
      shared_mutation("missing table", right, rc) { |v| v.fetch("after").delete("indexer_health_event") }
      shared_mutation("unowned pid", right, rc) { |v| v.fetch("blocked")["pid"] += 1 }
      shared_mutation("unowned database", right, rc) { |v| v.fetch("preparation").fetch("start_signal")["database"] = "other" }
      shared_mutation("unowned locker", right, rc) { |v| v.fetch("blocked")["blockers"] = [123456] }
      if cache == "warm_committed"
        shared_mutation("warm row changed", right, rc) { |v| v.fetch("after").fetch("canonical_size_rollup").first["sample_count"] += 1 }
      end
      reject("missing cancelled clock") { compare(left, right, lc, rc.reject { |key, _| key == "cancelled" }) }
      reject("non-UTC clock") { compare(left, right, lc, rc.merge("cancelled" => "2040-01-01T01:00:00+01:00")) }
      reject("incorrect expected clock") { compare(left, right, lc, rc.merge("seed" => lc.fetch("seed"))) }

      # Same unknown payload is retained; per-run UUID/time text in that payload
      # must NOT become equal merely because it matches a declared normalizable value.
      extended_left = CancellationPairUnitFixtures.clone(left)
      extended_right = CancellationPairUnitFixtures.clone(right)
      [extended_left, extended_right].each { |v| v["extra"] = { "data" => ["literal", 7, nil] } }
      extra_result = compare(extended_left, extended_right, lc, rc)
      assert(extra_result.fetch(:plain).fetch("extra") == extended_left.fetch("extra"), "unknown payload retained")
      ["canonical", "clock"].each do |kind|
        extended_left["extra"] = kind == "clock" ? lc.fetch("seed") : left.fetch("after").fetch("canonical_torrent").first.fetch("canonical_torrent_public_id")
        extended_right["extra"] = kind == "clock" ? rc.fetch("seed") : right.fetch("after").fetch("canonical_torrent").first.fetch("canonical_torrent_public_id")
        reject("arbitrary #{kind} string") { compare(extended_left, extended_right, lc, rc) }
      end

      # Mutate every scalar leaf in the full synthetic shape, including settings,
      # attributes, SQLSTATE, numeric relationships, input data and literal clocks.
      leaf_count = 0
      leaves(right).each do |path, value|
        mutation("leaf #{path.join('.')}", left, right, lc, rc) do |v|
          parent = path.take(path.length - 1).reduce(v) { |node, key| node.fetch(key) }
          parent[path.last] = case value
                              when String then value + "-mutated"
                              when Numeric then value + 1
                              when true then false
                              when false then true
                              when nil then "unexpected"
                              else raise "unsupported leaf"
                              end
        end
        leaf_count += 1
      end
      puts "PASS #{name}: allowed drift, focused mutations, #{leaf_count} scalar-leaf rejection checks"
    end
    puts "PASS #{@assertions} assertions; synthetic fixtures and clocks only; no live provenance claim"
  end
end

NativeCancellationComparisonTest.new.run if $PROGRAM_NAME == __FILE__
