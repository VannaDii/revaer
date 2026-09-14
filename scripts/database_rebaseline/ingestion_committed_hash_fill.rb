# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionCommittedHashFill
    COMMITTED_HASH_SEQUENCES = %w[
      canonical_size_sample_canonical_size_sample_id_seq
      canonical_torrent_source_cont_canonical_torrent_source_cont_seq
      source_metadata_conflict_source_metadata_conflict_id_seq
      source_metadata_conflict_audi_source_metadata_conflict_audi_seq
      indexer_health_event_indexer_health_event_id_seq
    ].freeze

    private

    def committed_hash_session(test_case, mode)
      raise Failure, "unknown committed hash-fill mode" unless IngestionSampling::SAMPLING_MODES.include?(mode)

      repeated = test_case.fetch(:arguments).merge(observed_at_input: "'2026-09-10T00:02:00Z'::timestamptz", seeders_input: "19")
      { calls: [test_case.fetch(:fixtures).first, test_case.fetch(:arguments), repeated], wrapper: true, helpers: mode == "helpers-first", finish_setting: true }
    end

    def committed_hash_fixtures(test_case, frames)
      tables = hash_fill_fixture_tables(test_case.fetch(:hash_fill), frames.map { |frame| frame.fetch("clock") }, frames.map { |frame| frame.fetch("result") })
      tables.each do |snapshot|
        snapshot.fetch("canonical_torrent_source_context_score").each do |row|
          %w[score_total_context score_policy_adjust score_tag_adjust].each { |key| row[key] = 0.0 }
        end
      end
      tables
    end

    def committed_hash_tables(test_case, baseline, frames)
      warm = baseline.transform_values { |rows| rows.map(&:dup) }
      %w[canonical_torrent canonical_torrent_source].each { |table| warm.fetch(table).first["updated_at"] = frames.first.fetch("clock") }
      %w[canonical_torrent_source_context_score canonical_torrent_best_source_context].each { |table| warm.fetch(table).first["computed_at"] = frames.first.fetch("clock") }
      spec = test_case.fetch(:hash_fill)
      filled = hash_fill_after(spec, warm, frames.fetch(1).fetch("clock"), 0)
      filled.fetch("canonical_torrent_source_context_score").last["canonical_torrent_source_context_score_id"] = 4
      repeated = filled.transform_values { |rows| rows.map(&:dup) }
      clock = frames.fetch(2).fetch("clock")
      observed = "2026-09-10T00:02:00+00:00"
      repeated.fetch("canonical_torrent").last["updated_at"] = clock
      repeated.fetch("canonical_torrent_source").first.merge!("updated_at" => clock, "last_seen_at" => observed, "last_seen_seeders" => 19)
      repeated.fetch("search_request_source_observation").first.merge!("observed_at" => observed, "seeders" => 19)
      %w[canonical_torrent_source_context_score canonical_torrent_best_source_context].each { |table| repeated.fetch(table).last["computed_at"] = clock }
      repeated.fetch("canonical_size_sample") << hash_fill_sample(3, observed)
      repeated.fetch("canonical_size_rollup").first.merge!("sample_count" => 3, "updated_at" => clock)
      if spec.fetch(:competing)
        new_conflict = attributes_empty
        hash_fill_conflict_tables!(new_conflict, spec.fetch(:value), clock, 2)
        new_conflict.fetch("source_metadata_conflict").first["observed_at"] = observed
        new_conflict.fetch("indexer_health_event").first["occurred_at"] = observed
        %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].each { |table| repeated.fetch(table).concat(new_conflict.fetch(table)) }
      end
      [warm, filled, repeated]
    end

    def committed_hash_sequence_values(sample, score, conflicts)
      COMMITTED_HASH_SEQUENCES.zip([sample, score, conflicts, conflicts, conflicts]).to_h
    end

    def committed_hash_read(database, name)
      inputs = IngestionPolicy::POLICY_READ_TABLES.map do |table|
        "#{literal(table)}, (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)::text), '[]') FROM public.#{identifier(table)} t)"
      end
      query = <<~SQL
        SELECT json_build_object('tables', ingestion_observation.snapshot(), 'inputs', json_build_object(#{inputs.join(',')}),
          'sequences', (SELECT json_object_agg(sequencename, last_value) FROM pg_catalog.pg_sequences
            WHERE schemaname = 'public' AND sequencename IN (#{COMMITTED_HASH_SEQUENCES.map { |name| literal(name) }.join(',')})));
      SQL
      raw = metadata_transport(query, database, "postgres", name)
      raw.merge("data" => metadata_read_parse(raw))
    end

    def committed_hash_read?(record, tables, inputs, sequences)
      data = metadata_read_parse(record)
      JSON.generate(data) == JSON.generate(record.fetch("data")) && data.keys.sort == %w[inputs sequences tables] &&
        size_tables_equal?(data.fetch("tables"), tables) && size_tables_equal?(data.fetch("inputs"), inputs) && data.fetch("sequences").eql?(sequences)
    end

    def committed_hash_validate!(evidence, test_case, mode, variant, role)
      fixtures = evidence.fetch("fixtures").map.with_index do |record, index|
        session = { calls: [test_case.fetch(:fixtures).fetch(index)], wrapper: true, finish_setting: true }
        parsed = sampling_parse(record, session, variant)
        raise Failure, "committed hash fixture framing changed" unless parsed == record.fetch("frames")

        parsed.first
      end
      frames = sampling_parse(evidence.fetch("tested"), committed_hash_session(test_case, mode), variant)
      raise Failure, "committed hash raw and declared frames differ" unless frames == evidence.fetch("tested").fetch("frames")

      all = fixtures + frames
      clocks = [evidence.fetch("seed_clock"), *all.map { |frame| frame.fetch("clock") }]
      unless fixtures.length == 2 && fixtures.all? { |frame| validation_context?([frame], role) } && validation_context?(frames, role) &&
             fixtures.map { |frame| frame.fetch("backend") }.uniq.length == 2 && fixtures.none? { |frame| frame.fetch("backend") == frames.first.fetch("backend") } &&
             all.all? { |frame| frame.fetch("finished_setting") == "error" && frame.values_at("within", "outside") == ["true", (variant == "reference").to_s] } &&
             clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == clocks.length
        raise Failure, "committed hash backend role settings clocks or scratch lifetime changed"
      end
      identities = fixtures.map { |frame| frame.fetch("result").slice("canonical_torrent_public_id", "canonical_torrent_source_public_id") }
      uuids = identities.flat_map(&:values)
      unless identities.all? { |identity| identity.length == 2 } && uuids.uniq.length == 4 && uuids.all? { |id| id.is_a?(String) && IngestionProof::INGESTION_UUID.match?(id) }
        raise Failure, "committed hash fixture identities changed"
      end
      expected_fixtures = committed_hash_fixtures(test_case, fixtures)
      expected_tested = committed_hash_tables(test_case, expected_fixtures.last, frames)
      before = attributes_empty
      all.each_with_index do |frame, index|
        success = index < 3 || variant == "final"
        expected = index < 2 ? expected_fixtures.fetch(index) : expected_tested.fetch(index - 2)
        after = success ? expected : before
        identity = index < 2 ? identities.fetch(index) : identities.first.merge(index > 2 ? identities.last.slice("canonical_torrent_public_id") : {})
        result = identity.merge("canonical_changed" => index < 2, "observation_created" => index < 2, "durable_source_created" => index < 2)
        unless (success ? frame.fetch("result") == result : !frame.key?("result")) &&
               size_tables_equal?(frame.fetch("tables_before"), before) && size_tables_equal?(frame.fetch("tables_after"), after) && size_tables_equal?(frame.fetch("tables_finish"), after)
          raise Failure, "committed hash independent fixture fill reuse or rollback changed"
        end
        before = after
      end
      inputs = metadata_read_tables(evidence.fetch("seed_clock"))
      # 0052 logs hash conflicts before tmp_policy_rules; rollback keeps nextval.
      conflicts = test_case.fetch(:hash_fill).fetch(:competing) ? 2 : nil
      sequences = variant == "final" ? committed_hash_sequence_values(3, 5, conflicts) : committed_hash_sequence_values(1, 3, conflicts)
      unless committed_hash_read?(evidence.fetch("initial"), attributes_empty, inputs, committed_hash_sequence_values(nil, nil, nil)) &&
             committed_hash_read?(evidence.fetch("prepared"), expected_fixtures.last, inputs, committed_hash_sequence_values(1, 2, nil)) &&
             committed_hash_read?(evidence.fetch("after"), before, inputs, sequences)
        raise Failure, "committed hash independent read inputs or identity sequences changed"
      end
      true
    end

    def committed_hash_isolated(test_case, mode, variant, source, role)
      name = "#{test_case.fetch(:name)}-#{mode}-#{variant}"
      database = "ingestion_committed_hash_#{variant}"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")) + "\nUPDATE public.trust_tier SET created_at = transaction_timestamp();\nUPDATE public.media_domain SET created_at = transaction_timestamp();"
        clock = policy_setup!(seed, database, File.join(@committed_hash_evidence, "#{name}-seed"))
        correction_observer!(database, variant)
        evidence = { "seed_clock" => clock, "initial" => committed_hash_read(database, "#{name}-initial"), "fixtures" => [] }
        test_case.fetch(:fixtures).each_with_index do |arguments, index|
          session = { calls: [arguments], wrapper: true, finish_setting: true }
          raw = metadata_transport(correction_session(session), database, role, "#{name}-fixture-#{index}")
          evidence.fetch("fixtures") << raw.merge("frames" => sampling_parse(raw, session, variant))
        end
        evidence["prepared"] = committed_hash_read(database, "#{name}-prepared")
        session = committed_hash_session(test_case, mode)
        raw = metadata_transport(correction_session(session), database, role, "#{name}-tested")
        evidence["tested"] = raw.merge("frames" => sampling_parse(raw, session, variant))
        evidence["after"] = committed_hash_read(database, "#{name}-after")
        metadata_write("#{name}.json", JSON.pretty_generate(evidence) + "\n")
        check("committed hash #{name} exact fill reuse conflicts full state inputs and sequences", committed_hash_validate!(evidence, test_case, mode, variant, role))
        { name:, validated: true, states: evidence.fetch("tested").fetch("frames").map { |frame| frame.fetch("state") } }
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def verify_ingestion_committed_hash_fill!
      directory = File.join(@contract.output_path, "ingestion-committed-hash")
      raise Failure, "committed hash evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @committed_hash_evidence = Dir.mktmpdir("run-", directory)
      previous_metadata, previous_correction = @metadata_evidence, @correction_evidence
      @metadata_evidence = @correction_evidence = @committed_hash_evidence
      hashes = committed_hash_source_hashes
      first = @checks.length
      cases = []
      completed = false
      begin
        wrapper_hash_fill_cases.each do |test_case|
          IngestionSampling::SAMPLING_MODES.each do |mode|
            { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
              cases << committed_hash_isolated(test_case, mode, variant, source, role)
            end
          end
        end
        check("committed hash source bytes unchanged", hashes == committed_hash_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
          candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
          postgres_image: @contract.postgres_image, source_sha256: hashes, checks:, cases:,
          limitations: ["Frozen post-commit D4 failures remain failures, not semantic equivalence.",
                       "Five identity sequences are observed; unobserved native callsites and other sequence allocation remain unqualified."] }
        metadata_write("report.json", JSON.pretty_generate(report) + "\n")
        @metadata_evidence, @correction_evidence = previous_metadata, previous_correction
      end
    end

    def committed_hash_source_hashes
      path = "scripts/tests/database-ingestion-committed-hash-fill-test.rb"
      sampling_source_hashes.merge(path => Digest::SHA256.file(File.join(@contract.root, path)).hexdigest)
    end
  end
end
