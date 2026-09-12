# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Closure-audit case 6 only. Expected guard sites are frozen-source coordinates,
  # not lines learned from an observed error. D3 completeness remains parent-owned.
  module IngestionValidation
    VALIDATION_SITES = {
      request_null: [130, "search_request_missing"], instance_null: [137, "indexer_instance_missing"],
      request_absent: [149, "search_request_not_found"], request_status: [156, "search_request_not_running"],
      instance_absent: [168, "indexer_instance_not_found"], instance_disabled: [175, "indexer_instance_disabled"],
      instance_not_ready: [182, "indexer_instance_not_ready"], instance_not_in_search: [194, "indexer_instance_not_in_search"],
      title_blank: [225, "missing_title"], hash_v1: [241, "invalid_hash"], hash_v2: [248, "invalid_hash"],
      hash_magnet: [255, "invalid_hash"], identity_absent: [286, "insufficient_identity"],
      title_normalized: [331, "missing_title"], identity_fallback: [353, "insufficient_identity"],
      companion: [761, "attr_length_mismatch"], length: [775, "attr_length_mismatch"],
      duplicate: [811, "duplicate_attr_key"], channel: [837, "attr_value_invalid"], type: [870, "attr_type_mismatch"],
      tracker_category: [904, "attr_value_invalid"], tracker_subcategory: [911, "attr_value_invalid"],
      files_count: [918, "attr_value_invalid"], size_bytes_reported: [925, "attr_value_invalid"],
      season: [932, "attr_value_invalid"], episode: [939, "attr_value_invalid"], year: [946, "attr_value_invalid"],
      tmdb_id: [953, "attr_value_invalid"], tvdb_id: [960, "attr_value_invalid"], imdb_id: [969, "attr_value_invalid"],
      language_primary: [982, "attr_value_invalid"], subtitles_primary: [995, "attr_value_invalid"]
    }.freeze
    VALIDATION_CHANNELS = { text: "varchar", int: "integer", bigint: "bigint", numeric: "numeric", bool: "boolean", uuid: "uuid" }.freeze
    VALIDATION_VALUES = { text: "'tracker'", int: "1", bigint: "1024", numeric: "1.5", bool: "true", uuid: "'56900000-0000-4000-8000-000000000099'" }.freeze
    VALIDATION_TYPES = {
      text: %w[tracker_name release_group language_primary subtitles_primary imdb_id],
      bigint: %w[size_bytes_reported],
      int: %w[tracker_category tracker_subcategory files_count season episode year tmdb_id tvdb_id minimum_seed_time_hours],
      numeric: %w[minimum_ratio], bool: %w[freeleech internal_flag scene_flag]
    }.freeze
    VALIDATION_SEEDED = %w[indexer_definition indexer_instance policy_snapshot search_request].freeze
    VALIDATION_INPUT_CLOCKS = IngestionWrapper::WRAPPER_INPUT_CLOCKS.merge(
      "search_request" => %w[created_at canceled_at finished_at],
      "trust_tier" => %w[created_at], "media_domain" => %w[created_at]
    ).freeze
    VALIDATION_SOURCE_FILES = %w[
      scripts/database_rebaseline/ingestion_validation.rb scripts/tests/database-ingestion-validation-test.rb
      scripts/database_rebaseline/ingestion_proof.rb scripts/database_rebaseline/ingestion_corrections.rb
      scripts/database_rebaseline/ingestion_policy.rb scripts/database_rebaseline/ingestion_compilation.rb
      scripts/database_rebaseline/ingestion_dependencies.rb scripts/tests/database-ingestion-proof-seed.sql
      scripts/tests/database-ingestion-helper-first.sql
    ].freeze

    private

    def validation_case(name, site, arguments = {}, **fixture)
      { name:, site:, arguments:, fixture: }
    end

    def validation_attributes(key = "tracker_name", type = :text, values = { text: "'tracker'" }, count: 1)
      arrays = { attr_keys_input: "ARRAY[#{Array.new(count, literal(key)).join(',')}]::public.observation_attr_key[]",
                 attr_types_input: "ARRAY[#{Array.new(count, type ? literal(type.to_s) : 'NULL').join(',')}]::public.attr_value_type[]" }
      VALIDATION_CHANNELS.each do |channel, sql_type|
        arrays["attr_value_#{channel}_input".to_sym] = "ARRAY[#{Array.new(count, values.fetch(channel, 'NULL')).join(',')}]::#{sql_type}[]"
      end
      arrays
    end

    def validation_cases
      cases = [
        validation_case("request-null", :request_null, { search_request_public_id_input: "NULL::uuid" }),
        validation_case("instance-null", :instance_null, { indexer_instance_public_id_input: "NULL::uuid" }),
        validation_case("request-absent", :request_absent, { search_request_public_id_input: "'56900000-0000-4000-8000-000000000099'::uuid" }),
        validation_case("instance-absent", :instance_absent, { indexer_instance_public_id_input: "'56900000-0000-4000-8000-000000000099'::uuid" }),
        validation_case("instance-disabled", :instance_disabled, {}, enabled: false),
        validation_case("instance-deleted", :instance_disabled, {}, deleted: true),
        validation_case("instance-not-in-search", :instance_not_in_search, {}, run: false),
        validation_case("title-null", :title_blank, { title_raw_input: "NULL::varchar" }),
        validation_case("title-empty", :title_blank, { title_raw_input: "''::varchar" }),
        validation_case("title-whitespace", :title_blank, { title_raw_input: "'   '::varchar" }),
        validation_case("title-token-only", :title_normalized, { title_raw_input: "'1080p.H264.mkv'::varchar" }),
        validation_case("title-punctuation", :title_normalized, { title_raw_input: "'...'::varchar" }),
        validation_case("identity-all-absent", :identity_absent, { source_guid_input: "NULL::varchar", infohash_v1_input: "NULL::char(40)", size_bytes_input: "NULL::bigint" }),
        validation_case("identity-guid-without-size", :identity_fallback, { infohash_v1_input: "NULL::char(40)", size_bytes_input: "NULL::bigint" })
      ]
      %w[canceled finished failed].each { |status| cases << validation_case("request-#{status}", :request_status, {}, status:) }
      %w[needs_secret test_failed unmapped_definition duplicate_suspected].each do |migration|
        cases << validation_case("instance-#{migration}", :instance_not_ready, {}, migration:)
      end
      { hash_v1: :infohash_v1_input, hash_v2: :infohash_v2_input, hash_magnet: :magnet_hash_input }.each do |site, argument|
        cases << validation_case(site.to_s.tr('_', '-'), site, { argument => "'invalid'" })
      end
      attributes = validation_attributes
      attributes.each_key do |key|
        next if key == :attr_keys_input

        sql_type = attributes.fetch(key).split('::').last
        cases << validation_case("missing-#{key}", :companion, attributes.merge(key => "NULL::#{sql_type}"))
        cases << validation_case("empty-#{key}", :length, attributes.merge(key => "ARRAY[]::#{sql_type}"))
        cases << validation_case("long-#{key}", :length, attributes.merge(key => validation_attributes(count: 2).fetch(key)))
      end
      cases << validation_case("empty-keys-nonempty-companions", :length, attributes.merge(attr_keys_input: "ARRAY[]::public.observation_attr_key[]"))
      cases << validation_case("duplicate-key", :duplicate, validation_attributes(count: 2))
      cases << validation_case("null-type-zero-values", :channel, validation_attributes("tracker_name", nil, {}))
      VALIDATION_CHANNELS.each_key.with_index do |channel, index|
        key = VALIDATION_TYPES.fetch(channel, ["tracker_name"]).first
        other = VALIDATION_CHANNELS.keys.fetch((index + 1) % VALIDATION_CHANNELS.length)
        cases << validation_case("zero-#{channel}", :channel, validation_attributes(key, channel, {}))
        cases << validation_case("wrong-channel-#{channel}", :channel, validation_attributes(key, channel, { other => VALIDATION_VALUES.fetch(other) }))
        cases << validation_case("multiple-#{channel}", :channel, validation_attributes(key, channel, VALIDATION_VALUES.slice(channel, other)))
      end
      VALIDATION_TYPES.each do |type, keys|
        wrong = type == :text ? :int : :text
        keys.each { |key| cases << validation_case("type-#{key}", :type, validation_attributes(key, wrong, { wrong => VALIDATION_VALUES.fetch(wrong) })) }
      end
      %w[tracker_category tracker_subcategory files_count size_bytes_reported season episode year].each do |key|
        type = key == "size_bytes_reported" ? :bigint : :int
        cases << validation_case("negative-#{key}", key.to_sym, validation_attributes(key, type, { type => "-1" }))
      end
      %w[tmdb_id tvdb_id].each do |key|
        [-1, 0].each { |value| cases << validation_case("#{key}-#{value}", key.to_sym, validation_attributes(key, :int, { int: value.to_s })) }
      end
      %w[bad tt123456 tt1234567890 tt123456x].each do |value|
        cases << validation_case("imdb-#{value}", :imdb_id, validation_attributes("imdb_id", :text, { text: literal(value) }))
      end
      %w[language_primary subtitles_primary].each do |key|
        ["", "   "].each_with_index do |value, index|
          cases << validation_case("blank-#{key}-#{index}", key.to_sym, validation_attributes(key, :text, { text: literal(value) }))
        end
      end
      cases + [
        validation_case("null-arrays", nil),
        validation_case("aligned-empty-arrays", nil, validation_attributes(count: 0)),
        validation_case("null-keys-ignore-companions", nil, attributes.merge(attr_keys_input: "NULL::public.observation_attr_key[]", attr_value_int_input: "NULL::integer[]")),
        validation_case("null-migration-state", nil, {}, migration: nil)
      ]
    end

    def validation_fixture_sql(test_case)
      fixture = test_case.fetch(:fixture)
      # Retain the existing lookup values, with explicit disposable seed-clock
      # provenance instead of discarding arbitrary init-time timestamps.
      statements = ["UPDATE public.trust_tier SET created_at = transaction_timestamp();",
                    "UPDATE public.media_domain SET created_at = transaction_timestamp();"]
      if fixture.key?(:status)
        status = fixture.fetch(:status)
        canceled = status == "canceled" ? "transaction_timestamp()" : "NULL"
        failure = status == "failed" ? "'coordinator_error'" : "NULL"
        statements << "UPDATE public.search_request SET status = #{literal(status)}, finished_at = transaction_timestamp(), canceled_at = #{canceled}, failure_class = #{failure} WHERE search_request_id = 569001;"
      end
      statements << "UPDATE public.indexer_instance SET is_enabled = false WHERE indexer_instance_id = 569001;" if fixture[:enabled] == false
      statements << "UPDATE public.indexer_instance SET deleted_at = '2026-09-10T00:00:00Z' WHERE indexer_instance_id = 569001;" if fixture[:deleted]
      if fixture.key?(:migration)
        value = fixture.fetch(:migration)
        statements << "UPDATE public.indexer_instance SET migration_state = #{value ? literal(value) : 'NULL'} WHERE indexer_instance_id = 569001;"
      end
      statements << "DELETE FROM public.search_request_indexer_run WHERE search_request_id = 569001 AND indexer_instance_id = 569001;" if fixture[:run] == false
      statements.join("\n")
    end

    def validation_session_case(test_case, mode)
      raise Failure, "unknown validation mode" unless %w[cold helpers-first].include?(mode)

      control = test_case.fetch(:site).nil?
      { calls: Array.new(control ? 3 : 2) { test_case.fetch(:arguments) }, helpers: mode == "helpers-first", rollback: control }
    end

    def validation_source_hashes
      VALIDATION_SOURCE_FILES.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] }
    end

    def verify_ingestion_validation!
      @contract.validate_output_path!
      directory = File.join(@contract.output_path, "ingestion-validation")
      raise Failure, "validation evidence directory must not be a symlink" if File.symlink?(directory)

      @validation_evidence = File.join(directory, "run-#{Process.pid}-#{SecureRandom.hex(4)}")
      FileUtils.mkdir_p(@validation_evidence, mode: 0o700)
      @validation_validated_evidence = {}
      previous = @correction_evidence
      @correction_evidence = @validation_evidence
      first = @checks.length
      hashes = validation_source_hashes
      cases = []
      complete = false
      begin
        validation_sites!
        validation_cases.each do |test_case|
          %w[cold helpers-first].each do |mode|
            variants = { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.to_h do |variant, (source, role)|
              [variant, validation_isolated(test_case, mode, variant, source, role)]
            end
            equal = validation_comparable(variants.fetch("reference"), test_case) == validation_comparable(variants.fetch("final"), test_case)
            check("validation #{test_case.fetch(:name)} #{mode} paired evidence outside the approved correction", equal)
            cases << validation_case_result(test_case, mode, equal)
          end
        end
        check("validation source bytes unchanged during matrix", hashes == validation_source_hashes)
        complete = true
      ensure
        checks = @checks.drop(first)
        report = { completed: complete, passed: complete && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
                   scope: "closure-audit case 6; errors repeat after savepoint rollback and commit on the same backend",
                   controls: "successful first call rolled back, second committed, third retains exact frozen D4 counterexample",
                   limits: "No successful frozen committed reuse, sequence rollback, complete helper/callback closure, or full D3 claim.",
                   postgres_image: @contract.postgres_image, candidate_sha256: @contract.expected_candidate_sha256,
                   final_sha256: @contract.final_sha256, source_sha256: hashes, roles: { reference: "postgres", final: @runtime },
                   container: @container, checks:, cases: }
        bytes = JSON.pretty_generate(report) + "\n"
        path = validation_write("report.json", bytes)
        @validation_validated_evidence[path] = Digest::SHA256.hexdigest(bytes) if report.fetch(:passed)
        @correction_evidence = previous
      end
      raise Failure, "validation matrix failed; retained exact evidence" unless @checks.drop(first).all? { |entry| entry.fetch(:passed) }
    end

    def validation_case_result(test_case, mode, accepted)
      corrected = test_case.fetch(:site).nil?
      { name: test_case.fetch(:name), mode:, equivalent: accepted && !corrected, accepted:,
        approved_delta: corrected ? "ADR 588 D4: third call retains frozen 42P07, final succeeds" : nil }
    end

    def validation_write(name, bytes)
      raise Failure, "invalid validation evidence name" unless name.match?(/\A[a-zA-Z0-9_.-]+\z/)

      path = File.join(@validation_evidence, name)
      raise Failure, "validation evidence must not be a symlink" if File.symlink?(path)

      File.open(path, File::WRONLY | File::CREAT | File::TRUNC, 0o600) { |file| file.write(bytes) }
      path
    end

    def validation_isolated(test_case, mode, variant, source, role)
      database = "ingestion_validation_#{variant}"
      name = "#{test_case.fetch(:name)}-#{mode}-#{variant}"
      prefix = File.join(@validation_evidence, name)
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql"))
        seed_clock = policy_setup!(seed + "\n" + validation_fixture_sql(test_case), database, "#{prefix}-seed")
        correction_observer!(database, variant)
        before = ingestion_snapshot(database)
        inputs = policy_read_snapshot(database)
        session = validation_session_case(test_case, mode)
        query = correction_session(session)
        validation_write("#{name}.sql", query)
        outcome = result(query, role:, database:)
        validation_write("#{name}.stdout", outcome.stdout)
        validation_write("#{name}.stderr", outcome.stderr)
        raise Failure, "validation transport failed; retained diagnostic" unless outcome.success

        evidence = { "name" => name, "seed_clock" => seed_clock, "before" => before, "after" => ingestion_snapshot(database),
                     "inputs_before" => inputs, "inputs_after" => policy_read_snapshot(database),
                     "stdout" => outcome.stdout, "stderr" => outcome.stderr,
                     "frames" => validation_parse(outcome.stdout, outcome.stderr, test_case, mode, role) }
        bytes = JSON.pretty_generate(evidence) + "\n"
        path = validation_write("#{name}.json", bytes)
        validation_validate!(evidence, test_case, mode, variant, role)
        check("validation #{name} exact site, framing, inputs, role/GUC/backend and 18-table state", true)
        @validation_validated_evidence[path] = Digest::SHA256.hexdigest(bytes)
        # Only this producer registers original serialized, successfully validated bytes.
        dependency_read_observation(path, @validation_validated_evidence).first
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def validation_sites!
      routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |row| row.fetch("name") == "search_result_ingest_v1" }
      raise Failure, "validation frozen routine missing" unless routine

      VALIDATION_SITES.each_value do |line, detail|
        actual = routine.fetch("source").lines[(line - 1), 4]&.map(&:strip)
        expected = ["RAISE EXCEPTION USING", "ERRCODE = errcode,", "MESSAGE = base_message,", "DETAIL = '#{detail}';"]
        raise Failure, "validation frozen guard coordinate changed: #{line} #{detail}" unless actual == expected
      end
    end

    def validation_parse(stdout, stderr, test_case, mode, role)
      frames = correction_parse(stdout, stderr, validation_session_case(test_case, mode))
      diagnostics = ingestion_diagnostics(stderr, role:)
      frames.reject { |frame| frame.fetch("state") == "00000" }.zip(diagnostics).each do |frame, diagnostic|
        frame["diagnostic"] = diagnostic
      end
      frames
    end

    def validation_guard_diagnostic(site)
      line, detail = VALIDATION_SITES.fetch(site)
      routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |row| row.fetch("name") == "search_result_ingest_v1" }
      { "error" => "P0001: Failed to ingest search result", "detail" => detail, "hint" => nil, "statement" => nil,
        "routine" => "PL/pgSQL function #{routine.fetch('signature')}", "line" => line, "operation" => "RAISE",
        "location" => "exec_stmt_raise, pl_exec.c:3897" }
    end

    def validation_validate!(evidence, test_case, mode, variant, role)
      frames = evidence.fetch("frames")
      parsed = validation_parse(evidence.fetch("stdout"), evidence.fetch("stderr"), test_case, mode, role)
      raise Failure, "validation serialized frames differ from original transport" unless frames == parsed
      raise Failure, "validation diagnostic or guard site changed" unless validation_states?(frames, test_case, variant, role)
      raise Failure, "validation role/GUC/backend or clocks changed" unless validation_context?(frames, role)
      raise Failure, "validation input fixture or read-input state changed" unless validation_inputs?(evidence, test_case)
      raise Failure, "validation incomplete 18-table rollback evidence" unless validation_images?(evidence, test_case)
      raise Failure, "validation temporary-table lifetime changed" unless validation_lifetime?(frames, test_case, variant)
      raise Failure, "validation accepted control outcome changed" unless test_case[:site] || validation_control?(frames)

      true
    end

    def validation_states?(frames, test_case, variant, role)
      if test_case[:site]
        expected = validation_guard_diagnostic(test_case.fetch(:site))
        return frames.length == 2 && frames.all? { |frame| frame.fetch("state") == "P0001" && !frame.key?("result") && frame.fetch("diagnostic") == expected }
      end
      states = variant == "reference" ? %w[00000 00000 42P07] : %w[00000 00000 00000]
      return false unless frames.map { |frame| frame.fetch("state") } == states

      frames.each_with_index.all? do |frame, index|
        if variant == "reference" && index == 2
          !frame.key?("result") && frame.fetch("diagnostic") == ingestion_diagnostics(policy_d4_expected, role:).first
        else
          frame.key?("result") && !frame.key?("diagnostic")
        end
      end
    end

    def validation_context?(frames, role)
      capabilities = { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" }
      backend = frames.first.fetch("backend")
      clocks = frames.map { |frame| frame.fetch("clock") }
      backend.is_a?(String) && backend.match?(/\A[1-9][0-9]*\z/) && frames.all? do |frame|
        frame.fetch("backend") == backend && frame.fetch("role") == capabilities && frame.values_at("before", "after") == %w[error error]
      end && clocks.uniq.length == frames.length && clocks.all? { |clock| validation_clock?(clock) }
    end

    def validation_clock?(clock)
      clock.is_a?(String) && clock.match?(/\A\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{1,6})?\+00:00\z/)
    end

    def validation_inputs?(evidence, test_case)
      inputs = evidence.fetch("inputs_before")
      return false unless inputs == evidence.fetch("inputs_after") && inputs.keys.sort == IngestionPolicy::POLICY_READ_TABLES.sort
      return false unless validation_clock?(evidence.fetch("seed_clock"))
      return false if evidence.fetch("frames").any? { |frame| frame.fetch("clock") == evidence.fetch("seed_clock") }
      return false unless VALIDATION_SEEDED.all? do |table|
        rows = inputs.fetch(table)
        rows.length == 1 && rows.first.fetch("#{table}_id") == 569001 && rows.first.fetch("created_at") == evidence.fetch("seed_clock") &&
          (!rows.first.key?("updated_at") || rows.first.fetch("updated_at") == evidence.fetch("seed_clock"))
      end

      fixture = test_case.fetch(:fixture)
      request = inputs.fetch("search_request").first
      instance = inputs.fetch("indexer_instance").first
      runs = inputs.fetch("search_request_indexer_run")
      expected_runs = fixture[:run] == false ? [] : [[569001, 569001, "queued"]]
      terminal = fixture.key?(:status) ? evidence.fetch("seed_clock") : nil
      return false unless request.values_at("canceled_at", "finished_at", "failure_class") ==
                          [fixture[:status] == "canceled" ? terminal : nil, terminal, fixture[:status] == "failed" ? "coordinator_error" : nil]
      return false unless %w[trust_tier media_domain].all? do |table|
        inputs.fetch(table).any? && inputs.fetch(table).all? { |row| row.fetch("created_at") == evidence.fetch("seed_clock") }
      end

      request.values_at("search_request_public_id", "policy_snapshot_id", "status", "page_size", "query_text") ==
        ["56900000-0000-4000-8000-000000000002", 569001, fixture.fetch(:status, "running"), 10, "Ingestion proof"] &&
        instance.values_at("indexer_instance_public_id", "indexer_definition_id", "is_enabled", "deleted_at", "migration_state", "trust_tier_key") ==
        ["56900000-0000-4000-8000-000000000001", 569001, fixture.fetch(:enabled, true), fixture[:deleted] ? "2026-09-10T00:00:00+00:00" : nil, fixture.fetch(:migration, "ready"), "public"] &&
        runs.map { |row| row.values_at("search_request_id", "indexer_instance_id", "status") } == expected_runs &&
        inputs.fetch("policy_snapshot").first.fetch("snapshot_hash") == "b" * 64 &&
        inputs.fetch("indexer_definition").first.values_at("upstream_slug", "definition_hash") == ["ingestion-proof", "a" * 64]
    end

    def validation_images?(evidence, test_case)
      before = evidence.fetch("before")
      frames = evidence.fetch("frames")
      return false unless before == IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      return false unless correction_snapshots?({ rollback: test_case[:site].nil? }, before, frames, evidence.fetch("after"))
      return true unless test_case[:site]

      frames.all? { |frame| %w[tables_before tables_after tables_finish].all? { |key| frame.fetch(key) == before } } && evidence.fetch("after") == before
    end

    def validation_lifetime?(frames, test_case, variant)
      within = test_case[:site] ? %w[false false] : %w[true true true]
      outside = test_case[:site] ? %w[false false] : (variant == "reference" ? %w[false true true] : %w[false false false])
      frames.map { |frame| frame.fetch("within") } == within && frames.map { |frame| frame.fetch("outside") } == outside
    end

    def validation_control?(frames)
      populated = %w[canonical_torrent canonical_torrent_source canonical_torrent_source_context_score search_request_source_observation canonical_size_sample canonical_size_rollup search_request_canonical search_page search_page_item]
      frames.select { |frame| frame.fetch("state") == "00000" }.all? do |frame|
        tables = frame.fetch("tables_after")
        next false unless tables.all? { |table, rows| rows.length == (populated.include?(table) ? 1 : 0) }
        next false unless correction_results?({ name: "validation-control" }, [frame], tables) && validation_control_repeat?(frame)

        canonical = tables.fetch("canonical_torrent").first
        source = tables.fetch("canonical_torrent_source").first
        observation = tables.fetch("search_request_source_observation").first
        canonical.values_at("infohash_v1", "title_display", "title_normalized", "size_bytes", "identity_strategy") ==
          ["a" * 40, "Ingestion proof title", "ingestion proof title", 1024, "infohash_v1"] &&
          source.values_at("indexer_instance_id", "source_guid", "last_seen_seeders", "last_seen_leechers") == [569001, "ingestion-proof-source", 5, 2] &&
          observation.values_at("search_request_id", "indexer_instance_id", "canonical_torrent_id", "canonical_torrent_source_id", "title_raw", "size_bytes", "seeders", "leechers") ==
            [569001, 569001, canonical.fetch("canonical_torrent_id"), source.fetch("canonical_torrent_source_id"), "Ingestion proof title", 1024, 5, 2]
      end
    end

    def validation_control_repeat?(frame)
      before = frame.fetch("tables_before")
      return true if before.fetch("canonical_torrent").empty?

      clock_columns = { "canonical_torrent" => "updated_at", "canonical_torrent_source" => "updated_at",
                        "canonical_torrent_source_context_score" => "computed_at", "canonical_size_rollup" => "updated_at" }
      expected = before.to_h do |table, rows|
        column = clock_columns[table]
        [table, rows.map { |row| column ? row.merge(column => frame.fetch("clock")) : row }]
      end
      frame.fetch("tables_after") == expected
    end

    def validation_comparable(evidence, test_case)
      frames = evidence.fetch("frames").first(2)
      # Shared normalization is applied only after independent frame validation.
      comparable = if test_case[:site]
                     frames.each_with_index.map do |frame, index|
                       frame.merge("backend" => "<validated-backend>", "role" => "<validated-direct-role>", "clock" => "<transaction:#{index}>")
                     end
                   else
                     compilation_comparable("fixture" => nil, "frames" => frames)
                   end
      inputs = evidence.fetch("inputs_before").to_h do |table, rows|
        [table, rows.map do |row|
          row.to_h do |column, value|
            seeded_clock = VALIDATION_INPUT_CLOCKS.fetch(table, []).include?(column) && value == evidence.fetch("seed_clock")
            [column, seeded_clock ? "<validated-seed-clock>" : value]
          end
        end]
      end
      { "frames" => comparable, "inputs" => inputs, "site" => test_case.fetch(:site) }
    end
  end
end
