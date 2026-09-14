# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionIdentity
    IDENTITY_ROW_KEYS = {
      "canonical_torrent" => "canonical_torrent_id", "canonical_torrent_source" => "canonical_torrent_source_id",
      "search_request_source_observation" => "observation_id"
    }.freeze
    IDENTITY_OBSERVED_AT = "2026-09-10T00:01:00+00:00"
    private

    def wrapper_identity_specs
      v1 = "a" * 40
      v2 = "b" * 64
      magnet = "c" * 64
      v2_hash = Digest::SHA256.hexdigest([v2].pack("H*"))
      v1_hash = Digest::SHA256.hexdigest([v1].pack("H*"))
      [
        ["v2-only", { infohash_v2_input: "repeat('b',64)::char(64)" }, "infohash_v2", 1.0, [nil, v2, v2_hash, nil]],
        ["v2-precedence", { infohash_v1_input: "repeat('a',40)::char(40)", infohash_v2_input: "repeat('b',64)::char(64)" }, "infohash_v2", 1.0, [v1, v2, v2_hash, nil]],
        ["empty-v1", { infohash_v1_input: "''::char(40)", infohash_v2_input: "repeat('b',64)::char(64)" }, "infohash_v2", 1.0, [nil, v2, v2_hash, nil]],
        ["blank-v1", { infohash_v1_input: "'   '::char(40)", infohash_v2_input: "repeat('b',64)::char(64)" }, "infohash_v2", 1.0, [nil, v2, v2_hash, nil]],
        ["empty-v2", { infohash_v1_input: "repeat('a',40)::char(40)", infohash_v2_input: "''::char(64)" }, "infohash_v1", 1.0, [v1, nil, v1_hash, nil]],
        ["blank-v2", { infohash_v1_input: "repeat('a',40)::char(40)", infohash_v2_input: "'   '::char(64)" }, "infohash_v1", 1.0, [v1, nil, v1_hash, nil]],
        ["empty-magnet", { infohash_v1_input: "repeat('a',40)::char(40)", magnet_hash_input: "''::char(64)" }, "infohash_v1", 1.0, [v1, nil, v1_hash, nil]],
        ["blank-magnet", { infohash_v1_input: "repeat('a',40)::char(40)", magnet_hash_input: "'   '::char(64)" }, "infohash_v1", 1.0, [v1, nil, v1_hash, nil]],
        ["magnet-btih", { magnet_uri_input: "'magnet:?xt=urn:btih:#{v1.upcase}'::varchar" }, "infohash_v1", 1.0, [v1, nil, v1_hash, nil]],
        ["magnet-btmh", { magnet_uri_input: "'magnet:?xt=urn:btmh:1220#{v2.upcase}'::varchar" }, "infohash_v2", 1.0, [nil, v2, v2_hash, nil]],
        ["explicit-magnet", { magnet_hash_input: "repeat('c',64)::char(64)", magnet_uri_input: "'magnet:?dn=other'::varchar" }, "magnet_hash", 0.85, [nil, nil, magnet, nil]],
        ["non-magnet-uri", { magnet_uri_input: "'  https://example.invalid/Identity.torrent?DN=Proof  '::varchar" }, "magnet_hash", 0.85, [nil, nil, Digest::SHA256.hexdigest("https://example.invalid/Identity.torrent?DN=Proof"), nil]],
        ["magnet-no-query", { magnet_uri_input: "'  MAGNET:  '::varchar" }, "magnet_hash", 0.85, [nil, nil, Digest::SHA256.hexdigest("magnet:?"), nil]],
        ["magnet-empty-query", { magnet_uri_input: "'magnet:?'::varchar" }, "magnet_hash", 0.85, [nil, nil, Digest::SHA256.hexdigest("magnet:?"), nil]],
        ["magnet-empty-keys", { magnet_uri_input: "'magnet:?=ignored&&'::varchar" }, "magnet_hash", 0.85, [nil, nil, Digest::SHA256.hexdigest("magnet:?"), nil]],
        ["magnet-bare-key", { magnet_uri_input: "'magnet:?DN=Proof&XT'::varchar" }, "magnet_hash", 0.85, [nil, nil, Digest::SHA256.hexdigest("magnet:?dn=Proof&xt"), nil]],
        ["title-size", {}, "title_size_fallback", 0.6, [nil, nil, nil, Digest::SHA256.hexdigest("identity proof|1024")]]
      ]
    end

    def wrapper_identity_cases
      base = wrapper_arguments("unused", title: "Identity proof").merge(
        source_guid_input: "NULL::varchar", infohash_v1_input: "NULL::char(40)",
        infohash_v2_input: "NULL::char(64)", magnet_hash_input: "NULL::char(64)"
      )
      wrapper_identity_specs.flat_map do |name, changes, strategy, confidence, hashes|
        arguments = base.merge(changes)
        %w[new reuse promote-guid].map do |operation|
          tested = arguments.merge(observed_at_input: "'2026-09-10T00:01:00Z'::timestamptz", seeders_input: "17")
          guid = operation == "promote-guid" ? "promoted-identity" : nil
          tested[:source_guid_input] = "'  promoted-identity  '::varchar" if guid
          colliding = name == "v2-precedence" && operation != "new"
          decoy = wrapper_arguments("unused", title: "Unrelated identity").merge(source_guid_input: "NULL::varchar",
            infohash_v1_input: colliding ? "repeat('a',40)::char(40)" : "repeat('d',40)::char(40)")
          fixture = colliding ? arguments.merge(infohash_v1_input: "NULL::char(40)") : arguments
          stored = colliding ? [nil, *hashes.drop(1)] : hashes
          uri = { "non-magnet-uri" => "https://example.invalid/Identity.torrent?DN=Proof", "magnet-no-query" => "MAGNET:" }[name]
          { name: "identity-#{name}-#{operation}", fixtures: operation == "new" ? [decoy] : [decoy, fixture], arguments: tested,
            wrapper: true, identity: { strategy:, confidence:, hashes: stored, source_hashes: hashes.first(3), observation_hashes: hashes.first(3), guid:, operation:, uri: } }
        end
      end
    end

    def wrapper_identity?(test_case, frame, canonical, source)
      expected = test_case.fetch(:identity)
      tables = frame.fetch("tables_after")
      return false unless IDENTITY_ROW_KEYS.keys.all? { |table| tables.fetch(table).length == 2 }
      return false unless canonical.values_at("identity_strategy", "identity_confidence") == expected.values_at(:strategy, :confidence)
      return false unless canonical.values_at("infohash_v1", "infohash_v2", "magnet_hash", "title_size_hash") == expected.fetch(:hashes)
      return false unless source.values_at("infohash_v1", "infohash_v2", "magnet_hash") == expected.fetch(:source_hashes)
      return false unless source.fetch("source_guid") == expected.fetch(:guid)

      observation = tables.fetch("search_request_source_observation").find { |row| row.fetch("canonical_torrent_id") == canonical.fetch("canonical_torrent_id") }
      return false unless observation

      links = [canonical.fetch("canonical_torrent_id"), source.fetch("canonical_torrent_source_id"), expected.fetch(:guid)]
      return false unless observation.values_at("canonical_torrent_id", "canonical_torrent_source_id", "source_guid") == links
      return false unless wrapper_identity_observation?(expected, source, observation)

      wrapper_identity_preserved?(expected, frame, [canonical, source, observation])
    end

    def wrapper_identity_observation?(expected, source, observation)
      if expected[:uri]
        return false unless source.fetch("last_seen_magnet_uri") == expected.fetch(:uri) && observation.fetch("magnet_uri") == expected.fetch(:uri)
      end

      observation.values_at("infohash_v1", "infohash_v2", "magnet_hash") == expected.fetch(:observation_hashes) &&
        observation.values_at("title_raw", "size_bytes", "observed_at", "seeders", "leechers") == ["Identity proof", 1024, IDENTITY_OBSERVED_AT, 17, 2] &&
        source.values_at("last_seen_at", "last_seen_seeders", "last_seen_leechers") == [IDENTITY_OBSERVED_AT, 17, 2]
    end

    def wrapper_identity_preserved?(expected, frame, selected)
      before = frame.fetch("tables_before")
      after = frame.fetch("tables_after")
      return false unless wrapper_identity_prior_selection?(expected, before, selected)

      IDENTITY_ROW_KEYS.zip(selected).all? do |(table, key), current|
        id = current.fetch(key)
        next false unless id.is_a?(Integer) && id.positive?

        untouched = before.fetch(table).reject { |row| row.fetch(key) == id }
        next false unless untouched.length == 1 && untouched == after.fetch(table).reject { |row| row.fetch(key) == id }

        previous = before.fetch(table).find { |row| row.fetch(key) == id }
        next previous.nil? if expected.fetch(:operation) == "new"

        keys = table == "search_request_source_observation" ? [key] : [key, "#{table}_public_id"]
        previous && previous.values_at(*keys) == current.values_at(*keys) &&
          (table == "canonical_torrent" || previous.fetch("source_guid").nil?)
      end
    end

    def wrapper_identity_prior_selection?(expected, before, selected)
      return true if expected.fetch(:operation) == "new"

      targets = before.fetch("search_request_source_observation").select do |row|
        row.values_at("title_raw", "size_bytes", "source_guid") == ["Identity proof", 1024, nil] &&
          row.values_at("infohash_v1", "infohash_v2", "magnet_hash") == expected.fetch(:hashes).first(3)
      end
      return false unless targets.one?

      keys = IDENTITY_ROW_KEYS.values
      selected.zip(keys).map { |row, key| row.fetch(key) } == targets.first.values_at(*keys)
    end
  end
end
