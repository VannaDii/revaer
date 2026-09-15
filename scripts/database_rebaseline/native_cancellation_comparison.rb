# frozen_string_literal: true

require "time"
require_relative "support"
require_relative "ingestion_pool"

module RevaerDatabaseRebaseline
  # The caller must run canonical cancellation validators first and independently
  # witness the supplied transaction clocks. Pair equality is not D3 completion.
  class CancellationPairComparison
    IMAGES = %w[before prepared cancelled after].freeze
    TABLES = %w[
      canonical_torrent canonical_torrent_source canonical_torrent_source_attr
      canonical_torrent_source_context_score canonical_torrent_best_source_context
      search_request_source_observation search_request_source_observation_attr
      canonical_torrent_signal canonical_external_id canonical_size_sample
      canonical_size_rollup search_request_canonical search_page search_page_item
      search_filter_decision source_metadata_conflict source_metadata_conflict_audit_log
      indexer_health_event
    ].freeze
    INPUTS = %w[
      canonical_disambiguation_rule canonical_torrent_source_base_score indexer_instance
      trust_tier indexer_instance_media_domain media_domain search_profile_tag_prefer
      indexer_instance_tag policy_snapshot_rule policy_rule policy_set policy_rule_value_set
      policy_rule_value_set_item search_request search_request_indexer_run search_profile
      policy_snapshot tag indexer_definition
    ].freeze
    # Only columns exercised by this exact retained cancellation shape are eligible.
    WRITE_CLOCKS = {
      "canonical_torrent" => %w[created_at updated_at],
      "canonical_torrent_source" => %w[created_at updated_at],
      "canonical_torrent_source_context_score" => %w[computed_at],
      "canonical_torrent_best_source_context" => %w[computed_at],
      "canonical_size_rollup" => %w[updated_at],
      "search_request_canonical" => %w[first_seen_at]
    }.freeze
    SEED_CLOCKS = {
      "indexer_definition" => %w[created_at updated_at],
      "indexer_instance" => %w[created_at updated_at],
      "policy_snapshot" => %w[created_at], "search_request" => %w[created_at]
    }.freeze
    UUID = /\A[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\z/
    UTC = /\A\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|\+00:00)\z/

    def initialize(variant:, cache_state:)
      demand(%w[reference final].include?(variant), "unknown variant")
      demand(%w[cold warm_committed].include?(cache_state), "unknown cache state")
      @variant = variant
      @cache_state = cache_state
    end

    def compare!(plain, observed, plain_clocks:, observed_clocks:)
      left = normalize(plain, plain_clocks)
      right = normalize(observed, observed_clocks)
      demand(left.eql?(right), "paired evidence mismatch at #{difference(left, right, '$')}")
      { plain: left, observed: right }
    rescue KeyError, NoMethodError, TypeError, ArgumentError => error
      raise Failure, "malformed cancellation comparison: #{error.class}: #{error.message}"
    end

    private

    def demand(condition, message)
      raise Failure, message unless condition
    end

    def warm?
      @cache_state == "warm_committed"
    end

    def frozen_recovery?
      warm? && @variant == "reference"
    end

    def stages
      (warm? ? ["warmup"] : []) + (frozen_recovery? ? [] : ["recovery"])
    end

    def copy(value)
      case value
      when Hash
        demand(value.keys.all? { |key| key.is_a?(String) }, "non-string evidence key")
        value.to_h { |key, item| [key, copy(item)] }
      when Array then value.map { |item| copy(item) }
      when String then value.dup
      when Integer, TrueClass, FalseClass, NilClass then value
      when Float
        demand(value.finite?, "nonfinite evidence value")
        value
      else raise Failure, "non-JSON evidence value"
      end
    end

    def normalize(raw, clocks)
      demand(clocks.is_a?(Hash) && clocks.keys.sort == (["seed", "cancelled"] + stages).sort,
             "clock keys do not match case")
      clocks.each_value do |clock|
        demand(clock.is_a?(String) && UTC.match?(clock) && Time.iso8601(clock).utc_offset.zero?,
               "clock must be exact UTC ISO timestamp")
      end
      evidence = copy(raw)
      validate_images(evidence)
      identities = bind_identities(evidence.fetch("after"), stages)
      normalize_sessions(evidence, identities)
      IMAGES.each do |boundary|
        image = evidence.fetch(boundary)
        expected = boundary == "before" ? [] : (boundary == "after" ? stages : (warm? ? ["warmup"] : []))
        local = bind_identities(image, expected)
        local.each { |stage, identity| demand(identity.eql?(identities.fetch(stage)), "#{boundary} identity changed") }
        normalize_image(image, local, clocks)
        SEED_CLOCKS.each do |table, columns|
          evidence.fetch("inputs_#{boundary}").fetch(table).each do |row|
            columns.each { |column| replace(row, column, clocks.fetch("seed"), "clock:seed") }
          end
        end
      end
      evidence
    end

    def validate_images(evidence)
      demand(evidence.fetch("cache_state") == @cache_state, "cache state changed")
      IMAGES.each do |boundary|
        [[boundary, TABLES], ["inputs_#{boundary}", INPUTS]].each do |key, tables|
          image = evidence.fetch(key)
          demand(image.is_a?(Hash) && (tables - image.keys).empty?, "#{key} missing table image")
          demand(image.values.all? { |rows| rows.is_a?(Array) && rows.all? { |row| row.is_a?(Hash) } },
                 "#{key} invalid rows")
        end
      end
      demand(evidence.fetch("before").values.all?(&:empty?), "before contains application writes")
      demand(evidence.fetch("prepared").eql?(evidence.fetch("cancelled")), "literal cancellation rollback changed")
      IMAGES.drop(1).each do |boundary|
        demand(evidence.fetch("inputs_before").eql?(evidence.fetch("inputs_#{boundary}")), "read inputs changed")
      end
      if warm?
        evidence.fetch("prepared").each do |table, rows|
          after = evidence.fetch("after").fetch(table)
          demand(rows.all? { |row| after.any? { |item| item.eql?(row) } }, "immutable warm row changed in #{table}")
        end
      else
        demand(evidence.fetch("prepared").eql?(evidence.fetch("before")), "cold prepared image changed")
      end
      demand(!frozen_recovery? || evidence.fetch("after").eql?(evidence.fetch("prepared")), "frozen recovery wrote data")
    end

    def positive_id(value)
      demand(value.is_a?(Integer) && value.positive?, "invalid numeric identity")
      value
    end

    def one(rows, label, &predicate)
      selected = rows.select(&predicate)
      demand(selected.length == 1, "missing or ambiguous #{label}")
      selected.first
    end

    def bind_identities(image, expected)
      sources = image.fetch("canonical_torrent_source")
      canonicals = image.fetch("canonical_torrent")
      observations = image.fetch("search_request_source_observation")
      demand([sources, canonicals, observations].all? { |rows| rows.length == expected.length }, "identity row count changed")
      identities = expected.to_h do |stage|
        guid, hash = stage == "warmup" ? ["pool-proof-warmup", "b" * 40] : ["pool-proof-source", "a" * 40]
        source = one(sources, "#{stage} source") { |row| row.fetch("source_guid") == guid }
        demand(source.fetch("infohash_v1") == hash, "source hash mismatch")
        sid = positive_id(source.fetch("canonical_torrent_source_id"))
        observation = one(observations, "#{stage} observation") { |row| row.fetch("canonical_torrent_source_id").eql?(sid) }
        demand(observation.values_at("source_guid", "infohash_v1") == [guid, hash], "observation source/hash mismatch")
        cid = positive_id(observation.fetch("canonical_torrent_id"))
        canonical = one(canonicals, "#{stage} canonical") { |row| row.fetch("canonical_torrent_id").eql?(cid) }
        demand(canonical.fetch("infohash_v1") == hash, "canonical hash mismatch")
        public_ids = [canonical.fetch("canonical_torrent_public_id"), source.fetch("canonical_torrent_source_public_id")]
        demand(public_ids.all? { |id| id.is_a?(String) && UUID.match?(id) }, "invalid public identity")
        [stage, { "canonical_id" => cid, "source_id" => sid, "canonical" => public_ids.first, "source" => public_ids.last }]
      end
      %w[canonical_id source_id].each do |key|
        ids = identities.values.map { |identity| identity.fetch(key) }
        demand(ids.uniq.length == ids.length, "reused #{key}")
      end
      ids = identities.values.flat_map { |identity| identity.values_at("canonical", "source") }
      demand(ids.uniq.length == ids.length, "reused public identity")
      identities
    end

    def row_stage(row, identities)
      bindings = { "canonical_torrent_id" => "canonical_id", "canonical_torrent_source_id" => "source_id" }
      present = bindings.select { |column, _| row.key?(column) }
      demand(!present.empty?, "clock row has no identity binding")
      candidates = identities.select do |_, identity|
        present.all? { |column, key| row.fetch(column).eql?(identity.fetch(key)) }
      end
      demand(candidates.length == 1, "application row identity relationship changed")
      candidates.keys.first
    end

    def normalize_image(image, identities, clocks)
      WRITE_CLOCKS.each do |table, columns|
        image.fetch(table).each do |row|
          stage = row_stage(row, identities)
          identity = identities.fetch(stage)
          if table == "canonical_torrent"
            replace(row, "canonical_torrent_public_id", identity.fetch("canonical"), "uuid:#{stage}:canonical")
          elsif table == "canonical_torrent_source"
            replace(row, "canonical_torrent_source_public_id", identity.fetch("source"), "uuid:#{stage}:source")
          end
          columns.each { |column| replace(row, column, clocks.fetch(stage), "clock:#{stage}") }
        end
      end
    end

    def replace(object, key, expected, label)
      demand(object.fetch(key).eql?(expected), "unexpected value at normalized #{key} (#{label})")
      object[key] = "@cancellation-pair:#{label}"
    end

    def normalize_sessions(evidence, identities)
      preparation = evidence.fetch("preparation")
      session = preparation.fetch("session")
      pid = positive_id(session.fetch("pid"))
      database = session.fetch("database")
      demand(database.is_a?(String) && !database.empty?, "invalid owned database")
      locker = evidence.fetch("locker")
      locker_pid = positive_id(locker.fetch("pid"))
      demand(locker_pid != pid, "locker is application backend")
      frames = evidence.fetch("frames")
      checkpoint = evidence.fetch("checkpoint")
      initial = preparation.fetch("frames")
      names = (warm? ? ["warmup"] : []) + %w[cancelled recovery]
      demand(frames.map { |frame| frame.fetch("name") } == names, "frame sequence changed")
      demand(checkpoint.eql?(frames.take(names.length - 1)) && initial.eql?(frames.take(warm? ? 1 : 0)), "checkpoint continuity changed")
      demand(frames.find { |frame| frame.fetch("name") == "cancelled" }.fetch("before").eql?(session), "prepared session changed")
      demand(!warm? || initial.first.fetch("after").eql?(session), "warm session changed")
      demand(evidence.fetch("cancel_result") == "t", "cancel result changed")
      all_frames = [frames, checkpoint, initial]
      all_frames.each do |group|
        group.each do |frame|
          %w[before after].each do |boundary|
            value = frame.fetch(boundary)
            demand((IngestionPool::POOL_SESSION_KEYS - value.keys).empty?, "incomplete session")
            replace(value, "pid", pid, "backend")
            replace(value, "database", database, "database")
          end
          outcome = frame.fetch("outcome")
          stage = frame.fetch("name")
          if identities.key?(stage)
            demand(outcome.fetch("kind") == "success", "expected successful #{stage}")
            row = outcome.fetch("row")
            demand((IngestionPool::POOL_ROW_KEYS - row.keys).empty?, "incomplete outcome row")
            %w[canonical source].each { |key| replace(row, key, identities.fetch(stage).fetch(key), "uuid:#{stage}:#{key}") }
          else
            state, message = stage == "cancelled" ? ["57014", "canceling statement due to user request"] : ["42P07", 'relation "tmp_policy_rules" already exists']
            demand(outcome.values_at("kind", "operation", "state", "message", "detail") ==
              ["database_error", "search result ingest", state, message, nil] && outcome.key?("detail"), "exact error changed")
          end
        end
      end
      replace(session, "pid", pid, "backend")
      replace(session, "database", database, "database")
      signal = preparation.fetch("start_signal")
      demand(signal.fetch("cache_state") == @cache_state, "start cache state changed")
      replace(signal, "pid", pid, "backend")
      replace(signal, "database", database, "database")
      replace(locker, "pid", locker_pid, "locker")
      replace(locker, "database", database, "database")
      blocked = evidence.fetch("blocked")
      replace(blocked, "pid", pid, "backend")
      replace(blocked, "database", database, "database")
      demand(blocked.fetch("blockers").eql?([locker_pid]), "unexpected locker blockers")
      blocked["blockers"] = ["@cancellation-pair:locker"]
    end

    def difference(left, right, path)
      return path unless left.class == right.class
      if left.is_a?(Hash)
        return "#{path} keys" unless left.keys.sort == right.keys.sort
        left.each { |key, value| return difference(value, right.fetch(key), "#{path}.#{key}") unless value.eql?(right.fetch(key)) }
      elsif left.is_a?(Array)
        return "#{path} length" unless left.length == right.length
        left.each_with_index { |value, index| return difference(value, right[index], "#{path}[#{index}]") unless value.eql?(right[index]) }
      end
      path
    end
  end
end
