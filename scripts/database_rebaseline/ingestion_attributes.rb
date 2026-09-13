# frozen_string_literal: true

require "tmpdir"

module RevaerDatabaseRebaseline
  # Closure-audit family 7, not a D3 certificate. Preserve both frozen defects:
  # literal-backslash suffix recognition and NULL-distinct duplicate signals.
  module IngestionAttributes
    ATTRIBUTE_INPUTS = [
      ["tracker_name", :text, "Proof Tracker"], ["tracker_category", :int, 2000],
      ["tracker_subcategory", :int, 2040], ["size_bytes_reported", :bigint, 4_294_967_296],
      ["files_count", :int, 3], ["season", :int, 2], ["episode", :int, 7], ["year", :int, 2026],
      ["release_group", :text, "GrOuP"], ["freeleech", :bool, true], ["internal_flag", :bool, false],
      ["scene_flag", :bool, true], ["minimum_ratio", :numeric, "1.2345"], ["minimum_seed_time_hours", :int, 48],
      ["language_primary", :text, " EN "], ["subtitles_primary", :text, " Fr-CA "]
    ].map(&:freeze).freeze
    # These answers are specified separately from the input arrays, not calculated
    # by invoking a database helper or normalizing the observed output.
    ATTRIBUTE_ANSWERS = [
      ["tracker_name", :text, "Proof Tracker"], ["tracker_category", :int, 2000],
      ["tracker_subcategory", :int, 2040], ["size_bytes_reported", :bigint, 4_294_967_296],
      ["files_count", :int, 3], ["season", :int, 2], ["episode", :int, 7], ["year", :int, 2026],
      ["release_group", :text, "group"], ["freeleech", :bool, true], ["internal_flag", :bool, false],
      ["scene_flag", :bool, true], ["minimum_ratio", :numeric, 1.2345], ["minimum_seed_time_hours", :int, 48],
      ["language_primary", :text, "en"], ["subtitles_primary", :text, "fr-ca"]
    ].map(&:freeze).freeze
    ATTRIBUTE_DURABLE = %w[tracker_name tracker_category tracker_subcategory size_bytes_reported files_count season episode year].freeze
    ATTRIBUTE_SIGNALS = [["language", "en", nil], ["subtitles", "fr-ca", nil], ["year", nil, 2026],
                         ["season", nil, 2], ["episode", nil, 7]].map(&:freeze).freeze
    ATTRIBUTE_MAGNET = "750837cbeaadaf72ab0a852ee3e0517760ca56eb6b8fc3c08b2d9e73348b6bea"
    ATTRIBUTE_SOURCE_FILES = (IngestionValidation::VALIDATION_SOURCE_FILES + %w[
      scripts/database_rebaseline/ingestion_attributes.rb scripts/tests/database-ingestion-attributes-test.rb
      scripts/database_rebaseline/ingestion_wrapper.rb scripts/database_rebaseline/ingestion_existing.rb
      scripts/database_rebaseline/ingestion_approved_deltas.rb scripts/database_rebaseline/final_proof.rb
      scripts/database_rebaseline/final_sql.rb config/database-rebaseline.env .github/build-inputs.env
      crates/revaer-data/init.sql
      crates/revaer-data/migrations/0012_indexer_core.sql crates/revaer-data/migrations/0014_indexer_instances.sql
      crates/revaer-data/migrations/0016_search_profiles_torznab.sql
      crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql
    ]).uniq.freeze

    private

    def attributes_case(name, **changes)
      { name:, title: "Proof.1080p-GROUP\\", normalized: "proof", release: "group", rank: 19, confidence: 0.5,
        inputs: ATTRIBUTE_INPUTS, answers: ATTRIBUTE_ANSWERS, signals: ATTRIBUTE_SIGNALS }.merge(changes)
    end

    def attributes_without_release
      ATTRIBUTE_ANSWERS.reject { |key, _type, _value| key == "release_group" }
    end

    def attributes_cases
      cases = [[19, 0.5], [20, 0.6], [30, 0.7], [40, 0.8]].map do |rank, confidence|
        attributes_case("typed-rank-#{rank}", rank:, confidence:)
      end
      cases << attributes_case("ordinary-suffix-defect", title: "Proof.1080p-GROUP", normalized: "proof group", release: nil, answers: attributes_without_release)
      cases << attributes_case("low-confidence-suffix", title: "Proof-GROUP\\", release: nil, answers: attributes_without_release)
      %w[REPACK PROPER WEB].each do |token|
        inputs = ATTRIBUTE_INPUTS.map { |key, type, value| [key, type, key == "release_group" ? token : value] }
        cases << attributes_case("discouraged-#{token.downcase}", title: "Proof.1080p-#{token}\\", inputs:, release: nil, answers: attributes_without_release)
      end
      mismatch = ATTRIBUTE_INPUTS.map { |key, type, value| [key, type, key == "release_group" ? "OTHER" : value] }
      cases << attributes_case("mismatched-release", inputs: mismatch, answers: attributes_without_release)
      absent = ATTRIBUTE_INPUTS.reject { |key, _type, _value| key == "release_group" }
      cases << attributes_case("token-without-attribute", inputs: absent, answers: attributes_without_release)
      zero_keys = %w[tracker_category tracker_subcategory files_count size_bytes_reported season episode year minimum_ratio minimum_seed_time_hours]
      zero_inputs = ATTRIBUTE_INPUTS.map { |key, type, value| [key, type, zero_keys.include?(key) ? 0 : (type == :bool ? false : value)] }
      zero_answers = ATTRIBUTE_ANSWERS.map { |key, type, value| [key, type, zero_keys.include?(key) ? 0 : (type == :bool ? false : value)] }
      cases << attributes_case("zero-and-false", inputs: zero_inputs, answers: zero_answers,
                               signals: [["language", "en", nil], ["subtitles", "fr-ca", nil], ["year", nil, 0], ["season", nil, 0], ["episode", nil, 0]])
      [["null-instance-trust-key", nil, 40, "0052:655 false; retain initialization at 471"],
       ["missing-public-trust-tier", "public", nil, "0052:655 true; 660 true; assign zero at 661"]].each do |name, key, public_rank, branch|
        cases << attributes_case(name, title: "Trust rank proof", normalized: "trust rank proof", release: nil,
          rank: 0, confidence: 0.5, trust_fixture: { key:, public_rank: },
          inputs: [["language_primary", :text, "en"]], answers: [["language_primary", :text, "en"]], signals: [["language", "en", nil]],
          trust_rank_expectation: { "source_branch" => branch, "rank" => 0, "bucket" => 0, "confidence" => 0.5,
                                    "native_branch_evidence" => "pending" })
      end
      cases
    end

    def attributes_trust_fixture(test_case)
      test_case.fetch(:trust_fixture) { { key: "public", public_rank: test_case.fetch(:rank) } }
    end

    def attributes_fixture_sql(test_case)
      trust = attributes_trust_fixture(test_case)
      statements = [validation_fixture_sql(fixture: {})]
      if test_case.key?(:trust_fixture)
        statements << "UPDATE public.indexer_instance SET trust_tier_key = #{policy_sql_value(trust.fetch(:key))} WHERE indexer_instance_id = 569001;"
      end
      rank = trust.fetch(:public_rank)
      statements << if rank.nil?
                      "DELETE FROM public.trust_tier WHERE trust_tier_key = 'public';"
                    else
                      "UPDATE public.trust_tier SET rank = #{Integer(rank)} WHERE trust_tier_key = 'public';"
                    end
      statements.join("\n")
    end

    def attributes_arguments(test_case)
      rows = test_case.fetch(:inputs)
      values = { title_raw_input: "#{literal(test_case.fetch(:title))}::varchar", size_bytes_input: "NULL::bigint",
                 attr_keys_input: "ARRAY[#{rows.map { |key, _type, _value| literal(key) }.join(',')}]::public.observation_attr_key[]",
                 attr_types_input: "ARRAY[#{rows.map { |_key, type, _value| literal(type.to_s) }.join(',')}]::public.attr_value_type[]" }
      IngestionValidation::VALIDATION_CHANNELS.each do |channel, sql_type|
        entries = rows.map { |_key, type, value| type == channel ? policy_sql_value(value) : "NULL" }
        values["attr_value_#{channel}_input".to_sym] = "ARRAY[#{entries.join(',')}]::#{sql_type}[]"
      end
      values
    end

    def attributes_source_hashes
      ATTRIBUTE_SOURCE_FILES.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] }
    end

    def attributes_write(name, bytes)
      raise Failure, "invalid attribute evidence name" unless name.match?(/\A[a-zA-Z0-9_.-]+\z/)

      path = File.join(@attributes_evidence, name)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(bytes) }
      path
    rescue SystemCallError => error
      raise Failure, "cannot retain attribute evidence: #{error.message}"
    end

    def verify_ingestion_attributes!
      @contract.validate_output_path!
      directory = File.join(@contract.output_path, "ingestion-attributes")
      raise Failure, "attribute evidence directory must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @attributes_evidence = Dir.mktmpdir("run-", directory)
      @attributes_validated_evidence = {}
      previous = @correction_evidence
      @correction_evidence = @attributes_evidence
      first = @checks.length
      hashes = attributes_source_hashes
      cases = []
      completed = false
      begin
        attributes_write("routine-inventory.json", JSON.pretty_generate(@ingestion_inventory) + "\n")
        attributes_cases.each do |test_case|
          %w[cold helpers-first].each do |mode|
            pair = { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.to_h do |variant, (source, role)|
              [variant, attributes_isolated(test_case, mode, variant, source, role)]
            end
            equal = attributes_comparable(pair.fetch("reference")) == attributes_comparable(pair.fetch("final"))
            check("attributes #{test_case.fetch(:name)} #{mode} paired application outside exact D4", equal)
            cases << { name: test_case.fetch(:name), mode:, accepted: equal, equivalent: false,
                       approved_delta: "ADR 588 D4: frozen third-call 42P07 versus independently checked final success" }.merge(test_case.slice(:trust_rank_expectation))
          end
        end
        check("attributes source bytes unchanged during matrix", hashes == attributes_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
                   scope: "family 7; real fixture first signals, distinct cold/helper-first backend, rollback then two commits",
                   limits: "No frozen successful committed reuse, signal upsert branch, native callback closure or full D3 claim. UUID has no valid observation key. D5 ID cases remain in the required correction matrix.",
                   preserved_defects: ["ordinary suffix regex requires a literal backslash", "NULL-distinct signal key inserts repeated rows without increasing confidence"],
                   pending_evidence: "K1 source-branch entry and local rank/bucket require parent native observation; matching language confidence is not branch execution evidence.",
                   candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
                   postgres_image: @contract.postgres_image, container: @container, source_sha256: hashes, checks:, cases: }
        bytes = JSON.pretty_generate(report) + "\n"
        path = attributes_write("report.json", bytes)
        @attributes_validated_evidence[path] = Digest::SHA256.hexdigest(bytes) if report.fetch(:passed)
        @correction_evidence = previous
      end
      raise Failure, "attribute matrix failed; retained exact evidence" unless @checks.drop(first).all? { |entry| entry.fetch(:passed) }
    end

    def attributes_isolated(test_case, mode, variant, source, role)
      name = "#{test_case.fetch(:name)}-#{mode}-#{variant}"
      database = "ingestion_attributes_#{variant}"
      prefix = File.join(@attributes_evidence, name)
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql"))
        seed += "\n#{attributes_fixture_sql(test_case)}"
        seed_clock = policy_setup!(seed, database, "#{prefix}-seed")
        correction_observer!(database, variant)
        inputs = policy_read_snapshot(database)
        arguments = attributes_arguments(test_case)
        fixture = attributes_execute({ calls: [arguments] }, database, role, "#{name}-fixture")
        before = ingestion_snapshot(database)
        inputs_fixture = policy_read_snapshot(database)
        tested = attributes_execute(attributes_session(test_case, mode), database, role, name)
        evidence = tested.merge("fixture_transport" => fixture, "fixture" => fixture.fetch("frames").first,
                                "before" => before, "after" => ingestion_snapshot(database), "seed_clock" => seed_clock,
                                "inputs_before" => inputs, "inputs_fixture" => inputs_fixture, "inputs_after" => policy_read_snapshot(database))
        evidence.merge!(test_case.slice(:trust_rank_expectation).transform_keys(&:to_s))
        bytes = JSON.pretty_generate(evidence) + "\n"
        path = attributes_write("#{name}.json", bytes)
        attributes_validate!(evidence, test_case, mode, variant, role)
        check("attributes #{name} exact outcomes diagnostics roles clocks and complete state", true)
        @attributes_validated_evidence[path] = Digest::SHA256.hexdigest(bytes)
        dependency_read_observation(path, @attributes_validated_evidence).first
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def attributes_session(test_case, mode)
      raise Failure, "unknown attribute session mode" unless %w[cold helpers-first].include?(mode)

      { calls: Array.new(3) { attributes_arguments(test_case) }, rollback: true, helpers: mode == "helpers-first" }
    end

    def attributes_query(session)
      query = correction_session(session)
      anchor = "DO $$ BEGIN NULL; END $$;"
      raise Failure, "attribute context insertion point changed" unless query.scan(anchor).length == 1

      query.sub(anchor, "#{anchor}\nSELECT 'attribute_context:' || json_build_object('backend', pg_backend_pid()::text, 'session', session_user, 'current', current_user, 'setting', current_setting('plpgsql.variable_conflict'))::text;")
    end

    def attributes_execute(session, database, role, name)
      query = attributes_query(session)
      attributes_write("#{name}.sql", query)
      outcome = result(query, role:, database:)
      attributes_write("#{name}.stdout", outcome.stdout)
      attributes_write("#{name}.stderr", outcome.stderr)
      raise Failure, "attribute transport failed; retained original diagnostic" unless outcome.success

      attributes_parse(outcome.stdout, outcome.stderr, session, role).merge("stdout" => outcome.stdout, "stderr" => outcome.stderr)
    end

    def attributes_parse(stdout, stderr, session, role)
      first, remaining = stdout.split("\n", 2)
      raise Failure, "attribute context missing or reordered" unless first&.start_with?("attribute_context:") && remaining

      frames = correction_parse(remaining, stderr, session)
      diagnostics = ingestion_diagnostics(stderr, role:)
      frames.reject { |frame| frame.fetch("state") == "00000" }.zip(diagnostics).each { |frame, diagnostic| frame["diagnostic"] = diagnostic }
      { "context" => JSON.parse(first.delete_prefix("attribute_context:")), "frames" => frames }
    rescue JSON::ParserError
      raise Failure, "invalid attribute JSON evidence"
    end

    def attributes_validate!(evidence, test_case, mode, variant, role)
      raise Failure, "attribute trust-rank expectation or pending native evidence changed" unless evidence["trust_rank_expectation"] == test_case[:trust_rank_expectation]

      fixture = evidence.fetch("fixture")
      transport = evidence.fetch("fixture_transport")
      [[transport, { calls: [attributes_arguments(test_case)] }], [evidence, attributes_session(test_case, mode)]].each do |record, session|
        parsed = attributes_parse(record.fetch("stdout"), record.fetch("stderr"), session, role)
        raise Failure, "attribute serialized transport changed" unless record.slice("context", "frames") == parsed
        expected = { "backend" => record.fetch("frames").first.fetch("backend"), "session" => role, "current" => role, "setting" => "error" }
        raise Failure, "attribute entry context changed" unless record.fetch("context") == expected
      end
      frames = evidence.fetch("frames")
      all = [fixture] + frames
      raise Failure, "attribute fixture transport changed" unless transport.fetch("frames") == [fixture]
      raise Failure, "attribute cold backend or role/GUC/clock changed" unless validation_context?(frames, role) && validation_context?([fixture], role) &&
        fixture.fetch("backend") != frames.first.fetch("backend") && all.map { |frame| frame.fetch("clock") }.uniq.length == 4
      raise Failure, "attribute required success or exact D4 changed" unless fixture.fetch("state") == "00000" && !fixture.key?("diagnostic") &&
        validation_states?(frames, { site: nil }, variant, role)
      raise Failure, "attribute scratch lifetime changed" unless fixture.values_at("within", "outside") == ["true", (variant == "reference").to_s] &&
        validation_lifetime?(frames, { site: nil }, variant)
      raise Failure, "attribute read inputs changed" unless attributes_inputs?(evidence, test_case)
      raise Failure, "attribute fixture/rollback continuity changed" unless fixture.fetch("tables_before") == attributes_empty &&
        correction_snapshots?({}, attributes_empty, [fixture], evidence.fetch("before")) &&
        correction_snapshots?({ rollback: true }, evidence.fetch("before"), frames, evidence.fetch("after"))
      identities = attributes_identities(fixture.fetch("tables_after"))
      expected = attributes_initial_tables(test_case, fixture.fetch("clock"), identities)
      raise Failure, "attribute independently specified first results changed" unless attributes_first_tables?(fixture.fetch("tables_after"), expected) && attributes_result?(fixture, identities, first: true)

      frames.each_with_index do |frame, index|
        next unless frame.fetch("state") == "00000"

        changed = attributes_repeated_tables(test_case, frame, index)
        raise Failure, "attribute independently specified repeat results changed" unless frame.fetch("tables_after") == changed && attributes_result?(frame, identities, first: false)
      end
      true
    end

    def attributes_inputs?(evidence, test_case)
      trust = attributes_trust_fixture(test_case)
      return false unless evidence.fetch("inputs_fixture") == evidence.fetch("inputs_before") && validation_inputs?(evidence, { fixture: { trust_key: trust.fetch(:key) } })
      return false if evidence.fetch("fixture").fetch("clock") == evidence.fetch("seed_clock")

      ranks = evidence.fetch("inputs_before").fetch("trust_tier").map { |row| row.values_at("trust_tier_key", "rank") }.sort
      expected = [["semi_private", 20], ["private", 30], ["invite_only", 40]]
      expected << ["public", trust.fetch(:public_rank)] unless trust.fetch(:public_rank).nil?
      empty = IngestionPolicy::POLICY_READ_TABLES - %w[indexer_definition indexer_instance policy_snapshot search_request search_request_indexer_run trust_tier media_domain]
      ranks == expected.sort && empty.all? { |table| evidence.fetch("inputs_before").fetch(table).empty? }
    end

    def attributes_empty
      IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
    end

    def attributes_identities(tables)
      identities = IngestionExisting::EXISTING_IDENTITIES.to_h do |table|
        rows = tables.fetch(table)
        raise Failure, "attribute stored identity selection changed" unless rows.length == 1 && rows.first.fetch("#{table}_id") == 1

        uuid = rows.first.fetch("#{table}_public_id")
        raise Failure, "attribute stored UUID invalid" unless uuid.is_a?(String) && uuid.match?(IngestionProof::INGESTION_UUID)

        ["#{table}_public_id", uuid]
      end
      raise Failure, "attribute stored UUIDs collide" unless identities.values.uniq.length == 2

      identities
    end

    def attributes_result?(frame, identities, first:)
      frame.fetch("result") == identities.merge("observation_created" => first, "durable_source_created" => first, "canonical_changed" => first)
    end

    def attributes_first_tables?(actual, expected)
      table = "search_request_source_observation_attr"
      rows = actual.fetch(table)
      # INSERT SELECT has no ORDER BY after tmp_attrs normalization. Retain all
      # generated IDs, validate their exact unique range, and compare full values.
      ids = rows.map { |row| row.fetch("observation_attr_id") }
      return false unless ids.all? { |id| id.is_a?(Integer) } && ids.sort == (1..expected.fetch(table).length).to_a

      values = ->(items) { items.map { |row| row.reject { |key, _value| key == "observation_attr_id" } }.sort_by { |row| JSON.generate(row.sort) } }
      actual.reject { |key, _value| key == table } == expected.reject { |key, _value| key == table } &&
        values.call(rows) == values.call(expected.fetch(table))
    end

    def attributes_value_columns(type, value, uuid: true)
      channels = uuid ? IngestionValidation::VALIDATION_CHANNELS.keys : IngestionValidation::VALIDATION_CHANNELS.keys - [:uuid]
      channels.to_h { |channel| ["value_#{channel}", channel == type ? value : nil] }
    end

    def attributes_signal_rows(test_case, offset)
      rows = test_case.fetch(:signals).map { |key, text, int| [key, text, int, test_case.fetch(:confidence)] }
      rows.unshift(["release_group", test_case.fetch(:release), nil, 0.9]) if test_case.fetch(:release)
      rows.each_with_index.map do |(key, text, int, confidence), index|
        { "canonical_torrent_signal_id" => offset + index + 1, "canonical_torrent_id" => 1, "signal_key" => key,
          "value_text" => text, "value_int" => int, "confidence" => confidence, "parser_version" => 1 }
      end
    end

    def attributes_initial_tables(test_case, clock, identities)
      tables = attributes_empty
      tables["canonical_torrent"] = [{ "canonical_torrent_id" => 1, "canonical_torrent_public_id" => identities.fetch("canonical_torrent_public_id"),
        "identity_confidence" => 1.0, "identity_strategy" => "infohash_v1", "infohash_v1" => "a" * 40,
        "infohash_v2" => nil, "magnet_hash" => ATTRIBUTE_MAGNET, "title_size_hash" => nil, "imdb_id" => nil, "tmdb_id" => nil,
        "tvdb_id" => nil, "ids_confidence" => nil, "title_display" => test_case.fetch(:title), "title_normalized" => test_case.fetch(:normalized),
        "size_bytes" => nil, "created_at" => clock, "updated_at" => clock }]
      tables["canonical_torrent_source"] = [{ "canonical_torrent_source_id" => 1, "indexer_instance_id" => 569001,
        "canonical_torrent_source_public_id" => identities.fetch("canonical_torrent_source_public_id"), "source_guid" => "ingestion-proof-source",
        "infohash_v1" => "a" * 40, "infohash_v2" => nil, "magnet_hash" => ATTRIBUTE_MAGNET, "title_normalized" => test_case.fetch(:normalized),
        "size_bytes" => nil, "last_seen_at" => "2026-09-10T00:00:00+00:00", "last_seen_seeders" => 5, "last_seen_leechers" => 2,
        "last_seen_published_at" => nil, "last_seen_download_url" => nil, "last_seen_magnet_uri" => nil, "last_seen_details_url" => nil,
        "last_seen_uploader" => nil, "created_at" => clock, "updated_at" => clock }]
      tables["canonical_torrent_source_context_score"] = [{ "canonical_torrent_source_context_score_id" => 1,
        "context_key_type" => "search_request", "context_key_id" => 569001, "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1,
        "score_total_context" => 0, "score_policy_adjust" => 0, "score_tag_adjust" => 0, "is_dropped" => false, "computed_at" => clock }]
      tables["search_request_source_observation"] = [{ "observation_id" => 1, "search_request_id" => 569001, "indexer_instance_id" => 569001,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "observed_at" => "2026-09-10T00:00:00+00:00", "seeders" => 5, "leechers" => 2,
        "published_at" => nil, "uploader" => nil, "source_guid" => "ingestion-proof-source", "details_url" => nil, "download_url" => nil,
        "magnet_uri" => nil, "title_raw" => test_case.fetch(:title), "size_bytes" => nil, "infohash_v1" => "a" * 40, "infohash_v2" => nil,
        "magnet_hash" => ATTRIBUTE_MAGNET, "guid_conflict" => false, "was_downranked" => false, "was_flagged" => false }]
      tables["search_request_canonical"] = [{ "search_request_canonical_id" => 1, "search_request_id" => 569001, "canonical_torrent_id" => 1, "first_seen_at" => clock }]
      tables["search_page"] = [{ "search_page_id" => 1, "search_request_id" => 569001, "page_number" => 1, "sealed_at" => nil }]
      tables["search_page_item"] = [{ "search_page_item_id" => 1, "search_page_id" => 1, "search_request_canonical_id" => 1, "position" => 1 }]
      tables["search_request_source_observation_attr"] = test_case.fetch(:answers).each_with_index.map do |(key, type, value), index|
        attributes_value_columns(type, value).merge("observation_attr_id" => index + 1, "observation_id" => 1, "attr_key" => key, "created_at" => clock)
      end
      durable = test_case.fetch(:answers).select { |key, _type, _value| ATTRIBUTE_DURABLE.include?(key) }
      tables["canonical_torrent_source_attr"] = durable.each_with_index.map do |(key, type, value), index|
        attributes_value_columns(type, value, uuid: false).merge("canonical_torrent_source_attr_id" => index + 1, "canonical_torrent_source_id" => 1, "attr_key" => key)
      end
      tables["canonical_torrent_signal"] = attributes_signal_rows(test_case, 0)
      tables
    end

    def attributes_repeated_tables(test_case, frame, index)
      clocks = { "canonical_torrent" => "updated_at", "canonical_torrent_source" => "updated_at", "canonical_torrent_source_context_score" => "computed_at" }
      expected = frame.fetch("tables_before").to_h do |table, rows|
        [table, rows.map { |row| clocks.key?(table) ? row.merge(clocks.fetch(table) => frame.fetch("clock")) : row.dup }]
      end
      # PostgreSQL identity sequences advance even for rolled-back inserts. The
      # frozen nullable uniqueness key does NOT exercise its ON CONFLICT UPDATE.
      signal_count = test_case.fetch(:signals).length + (test_case.fetch(:release) ? 1 : 0)
      expected["canonical_torrent_signal"] += attributes_signal_rows(test_case, signal_count * (index + 1))
      expected
    end

    def attributes_comparable(evidence)
      application = compilation_comparable("fixture" => evidence.fetch("fixture"), "frames" => evidence.fetch("frames").first(2))
      inputs = validation_comparable(evidence, { site: nil }).fetch("inputs")
      { "application" => application, "inputs" => inputs }.merge(evidence.slice("trust_rank_expectation"))
    end
  end
end
