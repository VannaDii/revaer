# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionIdentityTest
    private

    def identity_tests!
      cases = wrapper_identity_cases
      assert(cases.length == 18 && cases.map { |entry| entry.fetch(:name) }.uniq.length == 18, "six identity strategies exercise new, reuse and GUID promotion")
      cases.each do |test_case|
        assert(test_case.fetch(:fixtures).first.fetch(:title_raw_input) == "'Unrelated identity'::varchar", "a distinct lower-ID source must already exist")
        if test_case.fetch(:name).include?("v2-precedence") && test_case.fetch(:identity).fetch(:operation) != "new"
          fixtures = test_case.fetch(:fixtures)
          assert(fixtures.first.fetch(:infohash_v1_input) == "repeat('a',40)::char(40)" && fixtures.last.fetch(:infohash_v1_input) == "NULL::char(40)", "v1 and v2 lookup must point to different stored canonicals")
        end
        frame = identity_test_frame(test_case)
        assert(wrapper_outcome?(test_case, frame), "exact #{test_case.fetch(:name)} identity and preserved row keys")
        identity_mutations!(test_case, frame)
      end
    end

    def identity_test_frame(test_case)
      frame = wrapper_test_frame
      expected = test_case.fetch(:identity)
      tables = frame.fetch("tables_after")
      canonical = tables.fetch("canonical_torrent").first
      canonical["canonical_torrent_id"] = 2
      canonical.merge!("identity_strategy" => expected.fetch(:strategy), "identity_confidence" => expected.fetch(:confidence))
      %w[infohash_v1 infohash_v2 magnet_hash title_size_hash].zip(expected.fetch(:hashes)).each { |key, value| canonical[key] = value }
      source = tables.fetch("canonical_torrent_source").first
      %w[infohash_v1 infohash_v2 magnet_hash].zip(expected.fetch(:source_hashes)).each { |key, value| source[key] = value }
      source["source_guid"] = expected.fetch(:guid)
      source.merge!("last_seen_at" => IngestionIdentity::IDENTITY_OBSERVED_AT, "last_seen_seeders" => 17, "last_seen_leechers" => 2)
      observation = { "observation_id" => 2, "canonical_torrent_id" => 2, "canonical_torrent_source_id" => 2,
        "source_guid" => expected.fetch(:guid), "title_raw" => "Identity proof", "size_bytes" => 1024,
        "observed_at" => IngestionIdentity::IDENTITY_OBSERVED_AT, "seeders" => 17, "leechers" => 2 }
      %w[infohash_v1 infohash_v2 magnet_hash].zip(expected.fetch(:observation_hashes)).each { |key, value| observation[key] = value }
      tables["search_request_source_observation"] = [observation]
      tables["canonical_torrent_best_source_context"] = [{ "canonical_torrent_id" => 2, "canonical_torrent_source_id" => 2 }]
      fresh = expected.fetch(:operation) == "new"
      %w[observation_created durable_source_created canonical_changed].each { |key| frame.fetch("result")[key] = fresh }
      frame["tables_before"] = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      unless fresh
        %w[canonical_torrent canonical_torrent_source search_request_source_observation].each do |table|
          previous = tables.fetch(table).first.dup
          previous["source_guid"] = nil unless table == "canonical_torrent"
          %w[infohash_v1 infohash_v2 magnet_hash].zip(expected.fetch(:hashes).first(3)).each { |key, value| previous[key] = value }
          frame.fetch("tables_before")[table] = [previous]
        end
      end
      IngestionIdentity::IDENTITY_ROW_KEYS.each do |table, key|
        decoy = tables.fetch(table).first.merge(key => 1, "fixture" => "untouched decoy")
        decoy["#{table}_public_id"] = table == "canonical_torrent" ? "59600000-0000-4000-8000-000000000001" : "59600000-0000-4000-8000-000000000002" unless table == "search_request_source_observation"
        decoy["canonical_torrent_id"] = 1 if table == "search_request_source_observation"
        decoy["canonical_torrent_source_id"] = 1 if table == "search_request_source_observation"
        decoy["source_guid"] = nil unless table == "canonical_torrent"
        decoy["title_raw"] = "Unrelated identity" if table == "search_request_source_observation"
        tables.fetch(table).unshift(decoy)
        frame.fetch("tables_before").fetch(table).unshift(decoy.dup)
      end
      frame
    end

    def identity_mutations!(test_case, frame)
      columns = {
        "canonical_torrent" => %w[identity_strategy identity_confidence infohash_v1 infohash_v2 magnet_hash title_size_hash],
        "canonical_torrent_source" => %w[source_guid infohash_v1 infohash_v2 magnet_hash last_seen_at last_seen_seeders last_seen_leechers],
        "search_request_source_observation" => %w[observation_id canonical_torrent_id canonical_torrent_source_id source_guid infohash_v1 infohash_v2 magnet_hash title_raw size_bytes observed_at seeders leechers]
      }
      columns.each do |table, keys|
        keys.each do |key|
          changed = Marshal.load(Marshal.dump(frame))
          changed.fetch("tables_after").fetch(table).last[key] = "unexpected"
          assert(!wrapper_outcome?(test_case, changed), "#{test_case.fetch(:name)} rejects #{table}.#{key} drift")
        end
        changed = Marshal.load(Marshal.dump(frame))
        changed.fetch("tables_after").fetch(table) << changed.fetch("tables_after").fetch(table).first.dup
        assert(!wrapper_outcome?(test_case, changed), "#{test_case.fetch(:name)} rejects duplicate #{table} rows")
        next if test_case.fetch(:identity).fetch(:operation) == "new"

        changed = Marshal.load(Marshal.dump(frame))
        key = table == "search_request_source_observation" ? "observation_id" : "#{table}_public_id"
        changed.fetch("tables_before").fetch(table).last[key] = "unexpected"
        assert(!wrapper_outcome?(test_case, changed), "#{test_case.fetch(:name)} rejects replacing an existing #{table} identity")
      end
      changed = Marshal.load(Marshal.dump(frame))
      changed.fetch("tables_after").fetch("canonical_torrent_best_source_context").first["canonical_torrent_source_id"] = 1
      assert(!wrapper_outcome?(test_case, changed), "identity dispatch must retain the exact wrapper best-source assertion")
      changed = Marshal.load(Marshal.dump(frame))
      changed.fetch("tables_after").fetch("search_request_source_observation").last["observation_id"] = 0
      assert(!wrapper_outcome?(test_case, changed), "new observation identities must be positive integers")
      identity_wrong_selection!(test_case, frame) unless test_case.fetch(:identity).fetch(:operation) == "new"
    end

    def identity_wrong_selection!(test_case, frame)
      changed = Marshal.load(Marshal.dump(frame))
      before = changed.fetch("tables_before")
      after = changed.fetch("tables_after")
      %w[canonical_torrent_source search_request_source_observation].each do |table|
        key = IngestionIdentity::IDENTITY_ROW_KEYS.fetch(table)
        wrong = after.fetch(table).last.merge(key => before.fetch(table).first.fetch(key))
        if table == "canonical_torrent_source"
          public_key = "canonical_torrent_source_public_id"
          wrong[public_key] = before.fetch(table).first.fetch(public_key)
          changed.fetch("result")[public_key] = wrong.fetch(public_key)
        else
          wrong["canonical_torrent_source_id"] = before.fetch("canonical_torrent_source").first.fetch("canonical_torrent_source_id")
        end
        after[table] = [wrong, before.fetch(table).last.dup]
      end
      after.fetch("canonical_torrent_best_source_context").first["canonical_torrent_source_id"] = before.fetch("canonical_torrent_source").first.fetch("canonical_torrent_source_id")
      assert(!wrapper_outcome?(test_case, changed), "coherent decoy selection must not replace the intended prior source and observation")
    end
  end
end
