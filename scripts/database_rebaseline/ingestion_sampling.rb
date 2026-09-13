# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Committed application-wrapper calls retain the frozen D4 failures separately.
  module IngestionSampling
    SAMPLING_MODES = %w[cold helpers-first].freeze

    private

    def sampling_cases
      [
        { name: "three-samples", values: [[100, 0], [900, 1], [500, 2]],
          sample_ids: [[1], [1, 2], [1, 2, 3]], medians: [100, 500, 500], canonical_sizes: [100, 100, 500] },
        { name: "duplicate-then-new", values: [[100, 0], [100, 0], [900, 1]],
          sample_ids: [[1], [1], [1, 3]], medians: [100, 100, 500], canonical_sizes: [100, 100, 100] },
        { name: "out-of-order", values: [[100, 2], [900, 0], [500, 1]],
          sample_ids: [[1], [1, 2], [1, 2, 3]], medians: [100, 500, 500], canonical_sizes: [100, 900, 500] }
      ]
    end

    def sampling_session(spec, mode)
      raise Failure, "unknown sampling mode" unless SAMPLING_MODES.include?(mode)

      { calls: spec.fetch(:values).map { |bytes, minute| wrapper_arguments("size-source", size: bytes, minute:, title: IngestionSize::SIZE_TITLE) },
        wrapper: true, helpers: mode == "helpers-first", finish_setting: true }
    end

    def sampling_observed(minute)
      "2026-09-10T00:#{format('%02d', minute)}:00+00:00"
    end

    def sampling_tables(spec, frames, index)
      first = frames.first
      first_size, = spec.fetch(:values).first
      tables = size_expected_tables({ bytes: first_size, sampled: true, fallback: false }, first, 0)
      clock = frames.fetch(index).fetch("clock")
      { "canonical_torrent" => "updated_at", "canonical_torrent_source" => "updated_at",
        "canonical_torrent_source_context_score" => "computed_at", "canonical_torrent_best_source_context" => "computed_at" }.each do |table, column|
        tables.fetch(table).first[column] = clock
      end
      tables.fetch("canonical_torrent").first["size_bytes"] = spec.fetch(:canonical_sizes).fetch(index)
      tables.fetch("canonical_torrent_source").first["last_seen_at"] = sampling_observed(spec.fetch(:values).first(index + 1).map(&:last).max)
      bytes, minute = spec.fetch(:values).fetch(index)
      tables.fetch("search_request_source_observation").first.merge!("observed_at" => sampling_observed(minute), "size_bytes" => bytes)
      samples = spec.fetch(:sample_ids).fetch(index).map do |id|
        sample_bytes, sample_minute = spec.fetch(:values).fetch(id - 1)
        { "canonical_size_sample_id" => id, "canonical_torrent_id" => 1,
          "observed_at" => sampling_observed(sample_minute), "size_bytes" => sample_bytes }
      end
      tables["canonical_size_sample"] = samples
      tables.fetch("canonical_size_rollup").first.merge!("sample_count" => samples.length,
        "size_median" => spec.fetch(:medians).fetch(index), "size_min" => samples.map { |row| row.fetch("size_bytes") }.min,
        "size_max" => samples.map { |row| row.fetch("size_bytes") }.max, "updated_at" => clock)
      tables
    end

    def sampling_diagnostic
      expected = IngestionApprovedDeltas::APPROVED_REJECTIONS.fetch("D4")
      routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |row| row.fetch("name") == "search_result_ingest_v1" }
      statement = routine.fetch("source").scan(/CREATE TEMP TABLE tmp_policy_rules AS\n.*?;/m)
      unless statement.length == 1 && Digest::SHA256.hexdigest("SQL statement \"#{statement.first.delete_suffix(';')}\"") == expected.fetch("statement_sha256")
        raise Failure, "sampling frozen D4 statement changed"
      end
      diagnostic = policy_d4_expected
      raise Failure, "sampling frozen D4 line changed" unless diagnostic.include?(" line #{expected.fetch('line')} at SQL statement\n")

      diagnostic.sub("\nLOCATION:", "\n#{disambiguation_wrapper_stack}LOCATION:")
    end

    def sampling_parse(record, session, variant)
      metadata_transport_json!(record.fetch("stdout"))
      frames = correction_frames(record.fetch("stdout"), session)
      states = variant == "reference" ? %w[00000 42P07 42P07] : %w[00000 00000 00000]
      expected = variant == "reference" ? sampling_diagnostic * 2 : ""
      unless frames.map { |frame| frame.fetch("state") } == states && record.fetch("stderr") == expected
        raise Failure, "sampling required outcomes or exact D4 diagnostic changed"
      end
      frames
    end

    def sampling_read(database, name)
      inputs = IngestionPolicy::POLICY_READ_TABLES.map do |table|
        "#{literal(table)}, (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)::text), '[]') FROM public.#{identifier(table)} t)"
      end
      query = <<~SQL
        SELECT json_build_object('tables', ingestion_observation.snapshot(), 'inputs', json_build_object(#{inputs.join(',')}),
          'sample_sequence', (SELECT last_value FROM pg_catalog.pg_sequences WHERE schemaname = 'public'
            AND sequencename = 'canonical_size_sample_canonical_size_sample_id_seq'));
      SQL
      record = metadata_transport(query, database, "postgres", "#{name}-read")
      record.merge("data" => metadata_read_parse(record))
    end

    def sampling_read?(record, tables, inputs, sequence)
      data = metadata_read_parse(record)
      JSON.generate(data) == JSON.generate(record.fetch("data")) && data.keys.sort == %w[inputs sample_sequence tables] &&
        size_tables_equal?(data.fetch("tables"), tables) && size_tables_equal?(data.fetch("inputs"), inputs) &&
        data.fetch("sample_sequence").eql?(sequence)
    end

    def sampling_validate!(evidence, spec, mode, variant, role)
      frames = sampling_parse(evidence.fetch("tested"), sampling_session(spec, mode), variant)
      unless size_tables_equal?({ "frames" => frames }, { "frames" => evidence.fetch("tested").fetch("frames") })
        raise Failure, "sampling raw and declared frames differ"
      end
      clocks = [evidence.fetch("seed_clock"), *frames.map { |frame| frame.fetch("clock") }]
      unless validation_context?(frames, role) && frames.all? { |frame| frame.fetch("finished_setting") == "error" } &&
             clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == clocks.length
        raise Failure, "sampling backend role settings or clock provenance changed"
      end
      first_result = frames.first.fetch("result")
      identities = first_result.slice("canonical_torrent_public_id", "canonical_torrent_source_public_id")
      unless identities.length == 2 && identities.values.uniq.length == 2 && identities.values.all? { |id| id.is_a?(String) && IngestionProof::INGESTION_UUID.match?(id) }
        raise Failure, "sampling generated identities changed"
      end
      before = attributes_empty
      frames.each_with_index do |frame, index|
        success = variant == "final" || index.zero?
        after = success ? sampling_tables(spec, frames, index) : before
        expected_result = identities.merge("canonical_changed" => index.zero?, "observation_created" => index.zero?, "durable_source_created" => index.zero?)
        unless (success ? frame.fetch("result") == expected_result : !frame.key?("result")) &&
               frame.values_at("within", "outside") == ["true", (variant == "reference").to_s] &&
               size_tables_equal?(frame.fetch("tables_before"), before) && size_tables_equal?(frame.fetch("tables_after"), after) &&
               size_tables_equal?(frame.fetch("tables_finish"), after)
          raise Failure, "sampling full committed transition or scratch lifetime changed"
        end
        before = after
      end
      inputs = metadata_read_tables(evidence.fetch("seed_clock"))
      unless sampling_read?(evidence.fetch("initial"), attributes_empty, inputs, nil) &&
             sampling_read?(evidence.fetch("after"), before, inputs, variant == "final" ? 3 : 1)
        raise Failure, "sampling independent inputs final state or sequence changed"
      end
      true
    end

    def sampling_isolated(spec, mode, variant, source, role)
      name = "#{spec.fetch(:name)}-#{mode}-#{variant}"
      database = "ingestion_sampling_#{variant}"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")) + "\nUPDATE public.trust_tier SET created_at = transaction_timestamp();\nUPDATE public.media_domain SET created_at = transaction_timestamp();"
        clock = policy_setup!(seed, database, File.join(@sampling_evidence, "#{name}-seed"))
        correction_observer!(database, variant)
        evidence = { "seed_clock" => clock, "initial" => sampling_read(database, "#{name}-initial") }
        session = sampling_session(spec, mode)
        raw = metadata_transport(correction_session(session), database, role, name)
        evidence["tested"] = raw.merge("frames" => sampling_parse(raw, session, variant))
        evidence["after"] = sampling_read(database, "#{name}-after")
        metadata_write("#{name}.json", JSON.pretty_generate(evidence) + "\n")
        check("sampling #{name} exact committed state D4 diagnostics inputs and sequence", sampling_validate!(evidence, spec, mode, variant, role))
        { name:, validated: true, states: evidence.fetch("tested").fetch("frames").map { |frame| frame.fetch("state") } }
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def verify_ingestion_sampling!
      directory = File.join(@contract.output_path, "ingestion-sampling")
      raise Failure, "sampling evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @sampling_evidence = Dir.mktmpdir("run-", directory)
      previous_metadata, previous_correction = @metadata_evidence, @correction_evidence
      @metadata_evidence = @correction_evidence = @sampling_evidence
      hashes = sampling_source_hashes
      first = @checks.length
      cases = []
      completed = false
      begin
        sampling_cases.each do |spec|
          SAMPLING_MODES.each do |mode|
            { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
              cases << sampling_isolated(spec, mode, variant, source, role)
            end
          end
        end
        check("sampling source bytes unchanged", hashes == sampling_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
          candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
          postgres_image: @contract.postgres_image, source_sha256: hashes, checks:, cases:,
          limitations: ["Reference committed reuse retains exact D4 failure, never successful parity.",
                       "Three-call sampling cases do not prove all warm wrapper branches or native callback counts.",
                       "Only the size-sample identity sequence is directly observed; other sequence allocation is not certified."] }
        metadata_write("report.json", JSON.pretty_generate(report) + "\n")
        @metadata_evidence, @correction_evidence = previous_metadata, previous_correction
      end
    end

    def sampling_source_hashes
      path = "scripts/tests/database-ingestion-sampling-test.rb"
      wrapper_source_hashes.merge(path => Digest::SHA256.file(File.join(@contract.root, path)).hexdigest)
    end
  end
end
