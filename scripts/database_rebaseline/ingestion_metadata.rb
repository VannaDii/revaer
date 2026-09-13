# frozen_string_literal: true

require "date"

module RevaerDatabaseRebaseline
  # Changed observation values and first-wins durable metadata, not full D3.
  module IngestionMetadata
    METADATA_INPUTS = [
      ["tracker_name", :text, "Changed Tracker"], ["tracker_category", :int, 4000],
      ["tracker_subcategory", :int, 4040], ["size_bytes_reported", :bigint, 8_589_934_592],
      ["files_count", :int, 6], ["season", :int, 3], ["episode", :int, 8], ["year", :int, 2027],
      ["release_group", :text, "TeAm"], ["freeleech", :bool, false], ["internal_flag", :bool, true],
      ["scene_flag", :bool, false], ["minimum_ratio", :numeric, "2.3456"], ["minimum_seed_time_hours", :int, 96],
      ["language_primary", :text, " DE "], ["subtitles_primary", :text, " Pt-BR "]
    ].map(&:freeze).freeze
    METADATA_ANSWERS = [
      ["tracker_name", :text, "Changed Tracker"], ["tracker_category", :int, 4000],
      ["tracker_subcategory", :int, 4040], ["size_bytes_reported", :bigint, 8_589_934_592],
      ["files_count", :int, 6], ["season", :int, 3], ["episode", :int, 8], ["year", :int, 2027],
      ["release_group", :text, "team"], ["freeleech", :bool, false], ["internal_flag", :bool, true],
      ["scene_flag", :bool, false], ["minimum_ratio", :numeric, 2.3456], ["minimum_seed_time_hours", :int, 96],
      ["language_primary", :text, "de"], ["subtitles_primary", :text, "pt-br"]
    ].map(&:freeze).freeze
    METADATA_SEQUENCES = {
      "search_request_source_observation_attr" => "observation_attr_id",
      "canonical_torrent_source_attr" => "canonical_torrent_source_attr_id",
      "canonical_torrent_signal" => "canonical_torrent_signal_id",
      "source_metadata_conflict" => "source_metadata_conflict_id",
      "source_metadata_conflict_audit_log" => "source_metadata_conflict_audit_log_id",
      "indexer_health_event" => "indexer_health_event_id"
    }.freeze
    METADATA_JSON_RECORDS = %w[attribute_context helpers role clock tables_before tables_after tables_finish].freeze
    METADATA_SOURCE_FILES = (IngestionAttributes::ATTRIBUTE_SOURCE_FILES + %w[
      scripts/database_rebaseline/ingestion_metadata.rb scripts/tests/database-ingestion-metadata-test.rb
      scripts/tests/database-ingestion-proof-test.rb scripts/database_rebaseline/contract.rb scripts/database_rebaseline/support.rb
      scripts/database_rebaseline/ingestion_hash_fill.rb crates/revaer-data/migrations/0012_indexer_core.sql
      crates/revaer-data/migrations/0022_indexer_canonicalization.sql crates/revaer-data/migrations/0025_indexer_conflicts_decisions.sql
      crates/revaer-data/migrations/0030_indexer_seed_data.sql crates/revaer-data/migrations/0045_indexer_conflict_resolution_procs.sql
      crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql
    ]).uniq.freeze
    METADATA_LIMITS = [
      "0052:347-419 inserts conflicts, created audit rows and identity_conflict health events; no conflict UPDATE or upsert exists in this helper.",
      "Conflict resolution/reopen UPDATEs belong to 0045:238-243,348-353, not the ingestion call path.",
      "0052:2182-2430 guards durable inserts with existing-value IS NULL; 0022:280-304 requires the matching value non-NULL. Serial valid ingestion cannot take their ON CONFLICT UPDATE; concurrency is not covered.",
      "Frozen third committed reuse still fails with exact D4 42P07. Literal-backslash suffix and NULL-distinct signal duplication remain visible. External-ID D5 stays in its required correction matrix.",
      "No full D3, native closure, exhaustive metadata or feature-completion claim. Six affected sequences are observed; other sequences are not certified."
    ].freeze

    private

    def metadata_cases
      base = attributes_case("base", rank: 10).merge(conflicts: [], observed: "2026-09-10T00:00:00+00:00", seeders: 5, leechers: 2,
        published: nil, uploader: nil, details: nil, download: nil)
      changed = attributes_case("changed", rank: 10, title: "Proof.1080p-TEAM\\", release: "team",
        inputs: METADATA_INPUTS, answers: METADATA_ANSWERS,
        signals: [["language", "de", nil], ["subtitles", "pt-br", nil], ["year", nil, 2027], ["season", nil, 3], ["episode", nil, 8]])
      changed.merge!(conflicts: [["tracker_name", "Proof Tracker", "Changed Tracker"], ["tracker_category", "2000", "4000"], ["tracker_category", "2040", "4040"]],
        observed: "2026-09-11T00:00:00+00:00", seeders: 9, leechers: 4, published: "2026-09-09T00:00:00+00:00",
        uploader: "Changed uploader", details: "https://proof.invalid/details/changed", download: "https://proof.invalid/download/changed")
      sparse = base.merge(inputs: IngestionAttributes::ATTRIBUTE_INPUTS.first(8), answers: IngestionAttributes::ATTRIBUTE_ANSWERS.first(8),
        signals: [["year", nil, 2026], ["season", nil, 2], ["episode", nil, 7]])
      older = changed.merge(inputs: METADATA_INPUTS.map { |key, type, value| [key, type, key == "tracker_name" ? "Z" * 512 : value] },
        answers: METADATA_ANSWERS.map { |key, type, value| [key, type, key == "tracker_name" ? "Z" * 512 : value] },
        conflicts: [["tracker_name", "Proof Tracker", "Z" * 256], ["tracker_category", "2000", "4000"], ["tracker_category", "2040", "4040"]],
        observed: "2026-09-09T01:00:00+00:00", seeders: 1, leechers: 0, published: "2026-09-08T00:00:00+00:00",
        uploader: "Older uploader", details: "https://proof.invalid/details/older", download: "https://proof.invalid/download/older")
      [{ name: "replace-typed", fixtures: [base], incoming: changed },
       { name: "extend-typed", fixtures: [sparse], incoming: changed },
       { name: "existing-conflicts-stale-long", fixtures: [base, changed], incoming: older }]
    end

    def metadata_arguments(shape)
      attributes_arguments(shape).merge(observed_at_input: "#{literal(shape.fetch(:observed))}::timestamptz",
        seeders_input: "#{shape.fetch(:seeders)}::integer", leechers_input: "#{shape.fetch(:leechers)}::integer",
        published_at_input: "#{policy_sql_value(shape.fetch(:published))}::timestamptz", uploader_input: "#{policy_sql_value(shape.fetch(:uploader))}::varchar",
        details_url_input: "#{policy_sql_value(shape.fetch(:details))}::varchar", download_url_input: "#{policy_sql_value(shape.fetch(:download))}::varchar")
    end

    def metadata_session(spec, mode)
      raise Failure, "unknown metadata mode" unless %w[cold helpers-first].include?(mode)

      { calls: Array.new(3) { metadata_arguments(spec.fetch(:incoming)) }, rollback: true, helpers: mode == "helpers-first" }
    end

    def metadata_source_hashes
      METADATA_SOURCE_FILES.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] }
    end

    def metadata_write(name, bytes)
      raise Failure, "invalid metadata evidence name" unless name.match?(/\A[a-zA-Z0-9_.-]+\z/)

      path = File.join(@metadata_evidence, name)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(bytes) }
      path
    rescue SystemCallError => error
      raise Failure, "cannot retain metadata evidence: #{error.message}"
    end

    def metadata_transport(query, database, role, name)
      metadata_write("#{name}.sql", query)
      outcome = result(query, role:, database:)
      metadata_write("#{name}.stdout", outcome.stdout)
      metadata_write("#{name}.stderr", outcome.stderr)
      raise Failure, "metadata transport failed; original evidence retained" unless outcome.success

      { "stdout" => outcome.stdout, "stderr" => outcome.stderr }
    end

    def metadata_execute(session, database, role, name)
      raw = metadata_transport(attributes_query(session), database, role, name)
      metadata_transport_json!(raw.fetch("stdout"))
      raw.merge(attributes_parse(raw.fetch("stdout"), raw.fetch("stderr"), session, role))
    end

    def metadata_strict_json!
      # Older JSON runtimes can silently ignore unknown parser options.
      JSON.parse('{"outer":{"field":null,"field":true}}', allow_duplicate_key: false)
      raise Failure, "metadata JSON parser lacks duplicate-field rejection"
    rescue JSON::ParserError
      nil
    end

    def metadata_json_parse(value)
      metadata_strict_json!
      JSON.parse(value, allow_duplicate_key: false)
    rescue JSON::ParserError
      raise Failure, "invalid or duplicate metadata JSON evidence"
    end

    def metadata_transport_json!(stdout)
      stdout.each_line(chomp: true) do |line|
        if line.start_with?("{")
          metadata_json_parse(line)
        else
          prefix, value = line.split(":", 2)
          metadata_json_parse(value) if METADATA_JSON_RECORDS.include?(prefix)
        end
      end
    end

    def metadata_read(database, name)
      inputs = IngestionPolicy::POLICY_READ_TABLES.map do |table|
        "#{literal(table)}, (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)::text), '[]') FROM public.#{identifier(table)} t)"
      end
      sequences = METADATA_SEQUENCES.map do |table, column|
        "#{literal(table)}, (SELECT last_value FROM pg_catalog.pg_sequences WHERE schemaname = 'public' AND format('%I.%I', schemaname, sequencename)::regclass = pg_get_serial_sequence(#{literal("public.#{table}")}, #{literal(column)})::regclass)"
      end
      query = "\\set VERBOSITY verbose\nSELECT json_build_object('inputs', json_build_object(#{inputs.join(',')}), 'sequences', json_build_object(#{sequences.join(',')}))::text;"
      raw = metadata_transport(query, database, "postgres", "#{name}-read")
      raw.merge("data" => metadata_read_parse(raw))
    end

    def metadata_read_parse(record)
      raise Failure, "metadata read diagnostic changed" unless record.fetch("stderr").empty?

      metadata_json_parse(record.fetch("stdout"))
    end

    def metadata_read_tables(clock)
      tables = IngestionPolicy::POLICY_READ_TABLES.to_h { |table| [table, []] }.merge(hash_fill_read_tables(clock))
      tables["trust_tier"] = [["public", "Public", 0, 10], ["semi_private", "Semi-Private", 5, 20],
                              ["private", "Private", 10, 30], ["invite_only", "Invite Only", 15, 40]].each_with_index.map do |(key, name, weight, rank), index|
        { "trust_tier_id" => index + 1, "trust_tier_key" => key, "display_name" => name, "default_weight" => weight, "rank" => rank, "created_at" => clock }
      end
      tables["media_domain"] = [["movies", "Movies"], ["tv", "TV"], ["audiobooks", "Audiobooks"], ["ebooks", "Ebooks"],
                                ["software", "Software"], ["adult_movies", "Adult Movies"], ["adult_scenes", "Adult Scenes"]].each_with_index.map do |(key, name), index|
        { "media_domain_id" => index + 1, "media_domain_key" => key, "display_name" => name, "created_at" => clock }
      end
      tables
    end

    def metadata_tables_equal?(actual, expected)
      actual.keys.sort == expected.keys.sort && expected.all? do |table, rows|
        normalize = ->(items) { items.sort_by { |row| JSON.generate(row.sort) } }
        normalize.call(actual.fetch(table)) == normalize.call(rows)
      end
    end

    def metadata_conflicts!(tables, shape, clock, offset)
      shape.fetch(:conflicts).each_with_index do |(type, existing, incoming), index|
        id = offset + index + 1
        tables.fetch("source_metadata_conflict") << { "source_metadata_conflict_id" => id, "canonical_torrent_source_id" => 1,
          "conflict_type" => type, "existing_value" => existing, "incoming_value" => incoming, "observed_at" => shape.fetch(:observed),
          "resolved_at" => nil, "resolved_by_user_id" => nil, "resolution" => nil, "resolution_note" => nil }
        tables.fetch("source_metadata_conflict_audit_log") << { "source_metadata_conflict_audit_log_id" => id, "conflict_id" => id,
          "action" => "created", "actor_user_id" => 0, "occurred_at" => clock, "note" => nil }
        tables.fetch("indexer_health_event") << { "indexer_health_event_id" => id, "indexer_instance_id" => 569001,
          "occurred_at" => shape.fetch(:observed), "event_type" => "identity_conflict", "latency_ms" => nil,
          "http_status" => nil, "error_class" => nil, "detail" => type }
      end
    end

    # Only generated observation IDs are bound from evidence. Value expectations
    # and prior rows always originate in the independent model, never a snapshot.
    def metadata_bind_attrs!(expected, actual, offset, attempts, previous)
      old = previous.to_h { |row| [row.fetch("attr_key"), row.fetch("observation_attr_id")] }
      rows = actual.fetch("search_request_source_observation_attr")
      bindings = rows.to_h { |row| [row.fetch("attr_key"), row.fetch("observation_attr_id")] }
      ids = bindings.values
      raise Failure, "metadata attribute identity range changed" unless rows.length == bindings.length && ids.uniq.length == ids.length && ids.all? { |id| id.is_a?(Integer) }

      expected.fetch("search_request_source_observation_attr").each do |row|
        key = row.fetch("attr_key")
        id = bindings.fetch(key)
        valid = old.key?(key) ? id == old.fetch(key) : id.between?(offset + 1, offset + attempts)
        raise Failure, "metadata attribute identity range changed" unless valid

        row["observation_attr_id"] = id
      end
    end

    def metadata_changed_tables(previous, shape, clock, counters)
      tables = Marshal.load(Marshal.dump(previous))
      tables.fetch("canonical_torrent").first.merge!("title_display" => shape.fetch(:title), "updated_at" => clock)
      source = tables.fetch("canonical_torrent_source").first
      source["updated_at"] = clock
      if shape.fetch(:observed) > source.fetch("last_seen_at")
        { observed: "at", seeders: "seeders", leechers: "leechers", published: "published_at", uploader: "uploader", details: "details_url", download: "download_url" }.each do |key, column|
          source["last_seen_#{column}"] = shape.fetch(key)
        end
      end
      tables.fetch("canonical_torrent_source_context_score").first["computed_at"] = clock
      observation = tables.fetch("search_request_source_observation").first
      { observed: "observed_at", seeders: "seeders", leechers: "leechers", published: "published_at", uploader: "uploader", details: "details_url", download: "download_url", title: "title_raw" }.each do |key, column|
        observation[column] = shape.fetch(key)
      end
      attrs = tables.fetch("search_request_source_observation_attr")
      shape.fetch(:answers).each do |key, type, value|
        row = attrs.find { |item| item.fetch("attr_key") == key }
        unless row
          row = { "observation_attr_id" => 0, "observation_id" => 1, "attr_key" => key, "created_at" => clock }
          attrs << row
        end
        row.merge!(attributes_value_columns(type, value))
      end
      tables.fetch("canonical_torrent_signal").concat(attributes_signal_rows(shape, counters.fetch("canonical_torrent_signal")))
      metadata_conflicts!(tables, shape, clock, counters.fetch("source_metadata_conflict"))
      tables
    end

    def metadata_advance!(counters, shape, first: false)
      counters["search_request_source_observation_attr"] += shape.fetch(:answers).length
      counters["canonical_torrent_source_attr"] += 8 if first
      counters["canonical_torrent_signal"] += shape.fetch(:signals).length + 1
      %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].each { |table| counters[table] += shape.fetch(:conflicts).length }
    end

    def metadata_clock?(clock)
      return false unless clock.is_a?(String)

      match = clock.match(/\A(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)(?:\.\d{1,6})?\+00:00\z/)
      return false unless match

      year, month, day, hour, minute, second = match.captures.map { |part| Integer(part, 10) }
      year.positive? && Date.valid_date?(year, month, day, Date::GREGORIAN) && hour < 24 && minute < 60 && second < 60
    end

    def metadata_validate!(evidence, spec, mode, variant, role)
      fixtures = evidence.fetch("fixtures")
      raise Failure, "metadata fixture count changed" unless fixtures.length == spec.fetch(:fixtures).length

      records = fixtures + [evidence]
      sessions = spec.fetch(:fixtures).map { |shape| { calls: [metadata_arguments(shape)] } } + [metadata_session(spec, mode)]
      records.zip(sessions).each do |record, session|
        metadata_transport_json!(record.fetch("stdout"))
        parsed = attributes_parse(record.fetch("stdout"), record.fetch("stderr"), session, role)
        raise Failure, "metadata serialized transport changed" unless parsed == record.slice("context", "frames")
        expected = { "backend" => record.fetch("frames").first.fetch("backend"), "session" => role, "current" => role, "setting" => "error" }
        raise Failure, "metadata entry context changed" unless record.fetch("context") == expected && validation_context?(record.fetch("frames"), role)
      end
      all = records.flat_map { |record| record.fetch("frames") }
      clocks = [evidence.fetch("seed_clock")] + all.map { |frame| frame.fetch("clock") }
      raise Failure, "metadata cold backend or clocks changed" unless records.map { |record| record.fetch("context").fetch("backend") }.uniq.length == records.length &&
        clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == clocks.length
      frames = evidence.fetch("frames")
      raise Failure, "metadata required success or exact D4 changed" unless validation_states?(frames, { site: nil }, variant, role) &&
        fixtures.all? { |record| record.fetch("frames").length == 1 && record.fetch("frames").first.fetch("state") == "00000" }
      raise Failure, "metadata scratch lifetime changed" unless validation_lifetime?(frames, { site: nil }, variant) &&
        fixtures.all? { |record| record.fetch("frames").first.values_at("within", "outside") == ["true", (variant == "reference").to_s] }

      counters = METADATA_SEQUENCES.to_h { |table, _column| [table, 0] }
      sequence_stages = [counters.dup]
      expected = attributes_empty
      identities = attributes_identities(fixtures.first.fetch("frames").first.fetch("tables_after"))
      all.each_with_index do |frame, index|
        fixture = index < fixtures.length
        shape = fixture ? spec.fetch(:fixtures).fetch(index) : spec.fetch(:incoming)
        raise Failure, "metadata independent before state changed" unless metadata_tables_equal?(frame.fetch("tables_before"), expected)
        changed = if frame.fetch("state") != "00000"
                    expected
                  elsif index.zero?
                    attributes_initial_tables(shape, frame.fetch("clock"), identities)
                  else
                    metadata_changed_tables(expected, shape, frame.fetch("clock"), counters)
                  end
        if frame.fetch("state") == "00000"
          metadata_bind_attrs!(changed, frame.fetch("tables_after"), counters.fetch("search_request_source_observation_attr"), shape.fetch(:answers).length,
            expected.fetch("search_request_source_observation_attr"))
          metadata_advance!(counters, shape, first: index.zero?)
          raise Failure, "metadata result flags or identity changed" unless attributes_result?(frame, identities, first: index.zero?)
        end
        raise Failure, "metadata independent changed state mismatch" unless metadata_tables_equal?(frame.fetch("tables_after"), changed)
        expected = changed unless index == fixtures.length
        raise Failure, "metadata whole rollback or finish changed" unless metadata_tables_equal?(frame.fetch("tables_finish"), expected)
        sequence_stages << counters.dup if fixture || index == all.length - 1
        if fixture && index == fixtures.length - 1
          raise Failure, "metadata stored fixture changed" unless metadata_tables_equal?(evidence.fetch("before"), expected)
        end
      end
      raise Failure, "metadata independent final state changed" unless metadata_tables_equal?(evidence.fetch("after"), expected)
      reads = evidence.fetch("reads")
      raise Failure, "metadata read stage count changed" unless reads.length == sequence_stages.length

      reads.zip(sequence_stages).each do |record, sequence|
        data = metadata_read_parse(record)
        expected_sequence = sequence.transform_values { |value| value.zero? ? nil : value }
        raise Failure, "metadata serialized read changed" unless data == record.fetch("data")
        raise Failure, "metadata read defaults or sequence changed" unless data.keys.sort == %w[inputs sequences] &&
          metadata_tables_equal?(data.fetch("inputs"), metadata_read_tables(evidence.fetch("seed_clock"))) && data.fetch("sequences") == expected_sequence
      end
      true
    rescue KeyError, TypeError, NoMethodError => error
      raise Failure, "invalid metadata evidence shape: #{error.message}"
    end

    def metadata_isolated(spec, mode, variant, source, role)
      name = "#{spec.fetch(:name)}-#{mode}-#{variant}"
      database = "ingestion_metadata_#{variant}"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")) + "\n#{validation_fixture_sql(fixture: {})}"
        seed_clock = policy_setup!(seed, database, File.join(@metadata_evidence, "#{name}-seed"))
        correction_observer!(database, variant)
        reads = [metadata_read(database, "#{name}-seed")]
        fixtures = spec.fetch(:fixtures).each_with_index.map do |shape, index|
          record = metadata_execute({ calls: [metadata_arguments(shape)] }, database, role, "#{name}-fixture-#{index}")
          reads << metadata_read(database, "#{name}-fixture-#{index}")
          record
        end
        before = ingestion_snapshot(database)
        tested = metadata_execute(metadata_session(spec, mode), database, role, name)
        reads << metadata_read(database, name)
        evidence = tested.merge("fixtures" => fixtures, "before" => before, "after" => ingestion_snapshot(database), "reads" => reads, "seed_clock" => seed_clock)
        bytes = JSON.pretty_generate(evidence) + "\n"
        path = metadata_write("#{name}.json", bytes)
        metadata_validate!(evidence, spec, mode, variant, role)
        check("metadata #{name} independent fixture changed values whole rollback reads and sequences", true)
        @metadata_validated_evidence[path] = Digest::SHA256.hexdigest(bytes)
        dependency_read_observation(path, @metadata_validated_evidence).first
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def verify_ingestion_metadata!
      @contract.validate_output_path!
      directory = File.join(@contract.output_path, "ingestion-metadata")
      raise Failure, "metadata evidence directory must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @metadata_evidence = Dir.mktmpdir("run-", directory)
      @metadata_validated_evidence = {}
      previous = @correction_evidence
      @correction_evidence = @metadata_evidence
      first = @checks.length
      hashes = metadata_source_hashes
      cases = []
      completed = false
      begin
        metadata_write("routine-inventory.json", JSON.pretty_generate(@ingestion_inventory) + "\n")
        metadata_cases.each do |spec|
          %w[cold helpers-first].each do |mode|
            pair = { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.to_h do |variant, (source, role)|
              [variant, metadata_isolated(spec, mode, variant, source, role)]
            end
            equal = metadata_comparable(pair.fetch("reference")) == metadata_comparable(pair.fetch("final"))
            check("metadata #{spec.fetch(:name)} #{mode} paired outcomes outside exact D4", equal)
            cases << { name: spec.fetch(:name), mode:, accepted: equal, equivalent: false,
                       approved_delta: "ADR 588 D4: exact frozen third-call 42P07 versus independently checked final success" }
          end
        end
        check("metadata source bytes unchanged during matrix", hashes == metadata_source_hashes)
        completed = true
      ensure
        begin
          checks = @checks.drop(first)
          report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
                     scope: "changed typed attributes, durable first-wins, repeated conflicts, stale observation, long conflict truncation; cold/helpers-first, whole rollback/retry",
                     limits: METADATA_LIMITS, source_sha256: hashes, candidate_sha256: @contract.expected_candidate_sha256,
                     final_sha256: @contract.final_sha256, postgres_image: @contract.postgres_image, container: @container, checks:, cases: }
          bytes = JSON.pretty_generate(report) + "\n"
          path = metadata_write("report.json", bytes)
          @metadata_validated_evidence[path] = Digest::SHA256.hexdigest(bytes) if report.fetch(:passed)
        ensure
          @correction_evidence = previous
        end
      end
      raise Failure, "metadata matrix failed; original evidence retained" unless @checks.drop(first).all? { |entry| entry.fetch(:passed) }
    end

    def metadata_comparable(evidence)
      frames = evidence.fetch("fixtures").flat_map { |record| record.fetch("frames") } + evidence.fetch("frames").first(2)
      compilation_comparable("fixture" => nil, "frames" => frames).map do |frame|
        %w[tables_before tables_after tables_finish].each do |key|
          # Unordered INSERT SELECT assigns these IDs. Their ranges, reuse and
          # exact sequence counters are checked before any cross-variant mapping.
          frame.fetch(key)["search_request_source_observation_attr"] = frame.fetch(key).fetch("search_request_source_observation_attr").map do |row|
            row.reject { |column, _value| column == "observation_attr_id" }
          end.sort_by { |row| row.fetch("attr_key") }
        end
        frame
      end
    end
  end
end
