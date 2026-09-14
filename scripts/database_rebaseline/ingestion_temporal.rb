# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Keep incoming observations distinct from strictly newer durable metadata.
  module IngestionTemporal
    TEMPORAL_MODES = %w[cold helpers-first warm-committed].freeze
    TEMPORAL_FIELDS = %w[seeders leechers published_at download_url magnet_uri details_url uploader].freeze

    private

    def temporal_cases
      %w[guidless promote-guid].flat_map do |identity|
        { "older" => 0, "equal" => 1, "newer" => 2 }.map do |age, minute|
          { name: "#{identity}-#{age}", minute:, guid: identity == "promote-guid" ? "temporal-promoted" : nil }
        end
      end
    end

    def temporal_observation(minute, guid: nil, changed: false)
      label = changed ? "incoming" : "original"
      { "minute" => minute, "source_guid" => guid, "seeders" => changed ? 17 : 5, "leechers" => changed ? 9 : 2,
        "published_at" => "2026-09-09T00:0#{changed ? 2 : 1}:00+00:00", "uploader" => label,
        "download_url" => "https://example.invalid/#{label}.torrent", "details_url" => "https://example.invalid/#{label}",
        "magnet_uri" => "magnet:?dn=#{label}" }
    end

    def temporal_arguments(observation)
      values = wrapper_arguments("unused", title: IngestionSize::SIZE_TITLE, minute: observation.fetch("minute"))
      values[:source_guid_input] = "#{policy_sql_value(observation.fetch('source_guid'))}::varchar"
      TEMPORAL_FIELDS.each do |key|
        type = %w[seeders leechers].include?(key) ? "integer" : (key == "published_at" ? "timestamptz" : "varchar")
        values["#{key}_input".to_sym] = "#{policy_sql_value(observation.fetch(key))}::#{type}"
      end
      values
    end

    def temporal_observations(spec, mode)
      raise Failure, "unknown temporal mode" unless TEMPORAL_MODES.include?(mode)

      original = temporal_observation(1)
      [original, *(mode == "warm-committed" ? [original] : []), temporal_observation(spec.fetch(:minute), guid: spec.fetch(:guid), changed: true)]
    end

    def temporal_session(observations, helpers: false)
      { calls: observations.map { |observation| temporal_arguments(observation) }, wrapper: true, helpers:, finish_setting: true }
    end

    def temporal_tables(observations, frames, index)
      tables = size_expected_tables({ bytes: 1024, sampled: true, fallback: false }, frames.first, 0)
      clock = frames.fetch(index).fetch("clock")
      history = observations.first(index + 1)
      current = history.last
      latest = history.first
      history.each { |item| latest = item if item.fetch("minute") > latest.fetch("minute") }
      guid = history.filter_map { |item| item.fetch("source_guid") }.last
      tables.fetch("canonical_torrent").first["updated_at"] = clock
      source = tables.fetch("canonical_torrent_source").first
      source.merge!("source_guid" => guid, "last_seen_at" => sampling_observed(latest.fetch("minute")), "updated_at" => clock)
      TEMPORAL_FIELDS.each { |key| source["last_seen_#{key}"] = latest.fetch(key) }
      observation = tables.fetch("search_request_source_observation").first
      observation.merge!("source_guid" => guid, "observed_at" => sampling_observed(current.fetch("minute")))
      TEMPORAL_FIELDS.each { |key| observation[key] = current.fetch(key) }
      %w[canonical_torrent_source_context_score canonical_torrent_best_source_context].each do |table|
        tables.fetch(table).first["computed_at"] = clock
      end
      samples = []
      history.each_with_index do |item, ordinal|
        observed = sampling_observed(item.fetch("minute"))
        next if samples.any? { |sample| sample.fetch("observed_at") == observed }

        samples << { "canonical_size_sample_id" => ordinal + 1, "canonical_torrent_id" => 1, "observed_at" => observed, "size_bytes" => 1024 }
      end
      tables["canonical_size_sample"] = samples
      tables.fetch("canonical_size_rollup").first.merge!("sample_count" => samples.length, "updated_at" => clock)
      tables
    end

    def temporal_validate!(evidence, spec, mode, variant, role)
      observations = temporal_observations(spec, mode)
      fixture = sampling_parse(evidence.fetch("fixture"), temporal_session(observations.first(1)), variant)
      tested = sampling_parse(evidence.fetch("tested"), temporal_session(observations.drop(1), helpers: mode == "helpers-first"), variant)
      unless fixture == evidence.fetch("fixture").fetch("frames") && tested == evidence.fetch("tested").fetch("frames")
        raise Failure, "temporal raw and declared frames differ"
      end
      frames = fixture + tested
      clocks = [evidence.fetch("seed_clock"), *frames.map { |frame| frame.fetch("clock") }]
      unless validation_context?(fixture, role) && validation_context?(tested, role) &&
             fixture.first.fetch("backend") != tested.first.fetch("backend") &&
             frames.all? { |frame| frame.fetch("finished_setting") == "error" } &&
             clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == clocks.length
        raise Failure, "temporal backend role settings or clock provenance changed"
      end
      identities = fixture.first.fetch("result").slice("canonical_torrent_public_id", "canonical_torrent_source_public_id")
      unless identities.length == 2 && identities.values.uniq.length == 2 && identities.values.all? { |id| id.is_a?(String) && IngestionProof::INGESTION_UUID.match?(id) }
        raise Failure, "temporal generated identities changed"
      end
      before = attributes_empty
      frames.each_with_index do |frame, index|
        success = variant == "final" || mode != "warm-committed" || index < 2
        after = success ? temporal_tables(observations, frames, index) : before
        result = identities.merge("canonical_changed" => index.zero?, "observation_created" => index.zero?, "durable_source_created" => index.zero?)
        unless (success ? frame.fetch("result") == result : !frame.key?("result")) &&
               frame.values_at("within", "outside") == ["true", (variant == "reference").to_s] &&
               size_tables_equal?(frame.fetch("tables_before"), before) && size_tables_equal?(frame.fetch("tables_after"), after) &&
               size_tables_equal?(frame.fetch("tables_finish"), after)
          raise Failure, "temporal full committed transition or scratch lifetime changed"
        end
        before = after
      end
      inputs = metadata_read_tables(evidence.fetch("seed_clock"))
      sequence = variant == "reference" && mode == "warm-committed" ? 2 : frames.length
      unless sampling_read?(evidence.fetch("initial"), attributes_empty, inputs, nil) &&
             sampling_read?(evidence.fetch("prepared"), temporal_tables(observations, fixture, 0), inputs, 1) &&
             sampling_read?(evidence.fetch("after"), before, inputs, sequence)
        raise Failure, "temporal independent inputs state or sequence changed"
      end
      true
    end

    def temporal_isolated(spec, mode, variant, source, role)
      name = "#{spec.fetch(:name)}-#{mode}-#{variant}"
      database = "ingestion_temporal_#{variant}"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")) + "\nUPDATE public.trust_tier SET created_at = transaction_timestamp();\nUPDATE public.media_domain SET created_at = transaction_timestamp();"
        clock = policy_setup!(seed, database, File.join(@temporal_evidence, "#{name}-seed"))
        correction_observer!(database, variant)
        evidence = { "seed_clock" => clock, "initial" => sampling_read(database, "#{name}-initial") }
        observations = temporal_observations(spec, mode)
        sessions = { "fixture" => temporal_session(observations.first(1)), "tested" => temporal_session(observations.drop(1), helpers: mode == "helpers-first") }
        sessions.each do |key, session|
          raw = metadata_transport(correction_session(session), database, role, "#{name}-#{key}")
          evidence[key] = raw.merge("frames" => sampling_parse(raw, session, variant))
          evidence[key == "fixture" ? "prepared" : "after"] = sampling_read(database, "#{name}-#{key}-read")
        end
        metadata_write("#{name}.json", JSON.pretty_generate(evidence) + "\n")
        check("temporal #{name} independent full state identity freshness inputs and sequence", temporal_validate!(evidence, spec, mode, variant, role))
        { name:, validated: true, states: evidence.fetch("tested").fetch("frames").map { |frame| frame.fetch("state") } }
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def verify_ingestion_temporal!
      directory = File.join(@contract.output_path, "ingestion-temporal")
      raise Failure, "temporal evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @temporal_evidence = Dir.mktmpdir("run-", directory)
      previous_metadata, previous_correction = @metadata_evidence, @correction_evidence
      @metadata_evidence = @correction_evidence = @temporal_evidence
      hashes = temporal_source_hashes
      first = @checks.length
      cases = []
      completed = false
      begin
        temporal_cases.each do |spec|
          TEMPORAL_MODES.each do |mode|
            { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
              cases << temporal_isolated(spec, mode, variant, source, role)
            end
          end
        end
        check("temporal source bytes unchanged", hashes == temporal_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
          candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
          postgres_image: @contract.postgres_image, source_sha256: hashes, checks:, cases:,
          limitations: ["Frozen committed reuse retains exact D4 failure, never successful parity.",
                       "Temporal identity cases do not establish all hash-fill, scoring, paging or native callsites."] }
        metadata_write("report.json", JSON.pretty_generate(report) + "\n")
        @metadata_evidence, @correction_evidence = previous_metadata, previous_correction
      end
    end

    def temporal_source_hashes
      path = "scripts/tests/database-ingestion-temporal-test.rb"
      sampling_source_hashes.merge(path => Digest::SHA256.file(File.join(@contract.root, path)).hexdigest)
    end
  end
end
