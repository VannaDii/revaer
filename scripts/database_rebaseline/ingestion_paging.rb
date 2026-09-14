# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Qualify paging after a real commit without repairing the frozen D4 backend.
  module IngestionPaging
    PAGING_MODES = %w[cold helpers-first].freeze

    private

    def paging_cases
      [{ name: "reuse-new-page-item", reuse: 11 }, { name: "reuse-original-page-item", reuse: 1 }]
    end

    def paging_observations(spec)
      (1..10).map { |id| [id, 0] } + [[11, 1], [spec.fetch(:reuse), 2]]
    end

    def paging_session(observations, mode = "cold")
      raise Failure, "unknown paging mode" unless PAGING_MODES.include?(mode)

      { calls: observations.map { |id, minute| wrapper_arguments("page-#{id}", hash: id.to_s(16), minute:, title: IngestionSize::SIZE_TITLE) },
        wrapper: true, helpers: mode == "helpers-first", finish_setting: true }
    end

    def paging_result(before, frame, id)
      canonical = before.fetch("canonical_torrent").find { |row| row.fetch("canonical_torrent_id") == id }
      identities = if canonical
                     source = before.fetch("canonical_torrent_source").find { |row| row.fetch("source_guid") == "page-#{id}" }
                     { "canonical_torrent_public_id" => canonical.fetch("canonical_torrent_public_id"),
                       "canonical_torrent_source_public_id" => source.fetch("canonical_torrent_source_public_id") }
                   else
                     frame.fetch("result").slice("canonical_torrent_public_id", "canonical_torrent_source_public_id")
                   end
      identities.merge("canonical_changed" => canonical.nil?, "observation_created" => canonical.nil?, "durable_source_created" => canonical.nil?)
    end

    def paging_tables(before, frame, observation, ordinal)
      id, minute = observation
      tables = before.transform_values { |rows| rows.map(&:dup) }
      clock = frame.fetch("clock")
      observed = sampling_observed(minute)
      if before.fetch("canonical_torrent").none? { |row| row.fetch("canonical_torrent_id") == id }
        added = size_expected_tables({ bytes: 1024, sampled: true, fallback: false }, frame, id - 1)
        hash = id.to_s(16) * 40
        %w[canonical_torrent canonical_torrent_source search_request_source_observation].each do |table|
          added.fetch(table).first.merge!("infohash_v1" => hash, "magnet_hash" => Digest::SHA256.hexdigest([hash].pack("H*")))
        end
        added.fetch("canonical_torrent_source").first.merge!("source_guid" => "page-#{id}", "last_seen_at" => observed)
        added.fetch("search_request_source_observation").first.merge!("source_guid" => "page-#{id}", "observed_at" => observed)
        added.fetch("canonical_size_sample").first["observed_at"] = observed
        page = id <= 10 ? 1 : 2
        added["search_page"] = if [1, 11].include?(id)
                                 [{ "search_page_id" => page, "search_request_id" => 569001, "page_number" => page, "sealed_at" => nil }]
                               else
                                 []
                               end
        added.fetch("search_page_item").first.merge!("search_page_id" => page, "position" => id <= 10 ? id : 1)
        tables.fetch("search_page").first["sealed_at"] = clock if id == 11
        added.each { |table, rows| tables.fetch(table).concat(rows) }
      else
        # 0052 updates these clocks and consumes upsert identities even though
        # search_request_canonical's DO NOTHING prevents another page/item.
        { "canonical_torrent" => "updated_at", "canonical_torrent_source" => "updated_at",
          "canonical_torrent_source_context_score" => "computed_at", "canonical_torrent_best_source_context" => "computed_at" }.each do |table, column|
          key = table == "canonical_torrent_source" ? "canonical_torrent_source_id" : "canonical_torrent_id"
          tables.fetch(table).find { |row| row.fetch(key) == id }[column] = clock
        end
        tables.fetch("canonical_torrent_source").find { |row| row.fetch("canonical_torrent_source_id") == id }["last_seen_at"] = observed
        tables.fetch("search_request_source_observation").find { |row| row.fetch("observation_id") == id }["observed_at"] = observed
        tables.fetch("canonical_size_sample") << { "canonical_size_sample_id" => ordinal, "canonical_torrent_id" => id,
          "observed_at" => observed, "size_bytes" => 1024 }
        tables.fetch("canonical_size_rollup").find { |row| row.fetch("canonical_torrent_id") == id }.merge!("sample_count" => 2, "updated_at" => clock)
      end
      tables
    end

    def paging_counters(observations, successes)
      counters = IngestionProof::INGESTION_TABLES.to_h { |table| [table, 0] }
      existing = []
      observations.each_with_index do |(id, _minute), index|
        fresh = !existing.include?(id)
        # Both inserts precede tmp_policy_rules in 0052. D4 rolls back rows,
        # never nextval, including a second attempt at the still-missing item.
        %w[canonical_torrent canonical_torrent_source].each { |table| counters[table] += 1 } if fresh
        next if index >= successes

        %w[canonical_torrent_source_context_score canonical_torrent_best_source_context canonical_size_sample canonical_size_rollup search_request_canonical].each do |table|
          counters[table] += 1
        end
        next unless fresh

        %w[search_request_source_observation search_page_item].each { |table| counters[table] += 1 }
        counters["search_page"] += 1 if [1, 11].include?(id)
        existing << id
      end
      counters.transform_values { |value| value.zero? ? nil : value }
    end

    def paging_read(database, name)
      record = sampling_read(database, name)
      pairs = IngestionProof::INGESTION_TABLES.map do |table|
        column = IngestionMetadata::METADATA_SEQUENCES.fetch(table, table == "search_request_source_observation" ? "observation_id" : "#{table}_id")
        "#{literal(table)}, (SELECT last_value FROM pg_catalog.pg_sequences WHERE schemaname = 'public' AND format('%I.%I', schemaname, sequencename)::regclass = pg_get_serial_sequence(#{literal("public.#{table}")}, #{literal(column)})::regclass)"
      end
      record.merge("sequences" => metadata_transport("SELECT json_build_object(#{pairs.join(',')})::text;", database, "postgres", "#{name}-sequences"))
    end

    def paging_read?(record, tables, inputs, counters)
      sequences = metadata_read_parse(record.fetch("sequences"))
      sampling_read?(record, tables, inputs, counters.fetch("canonical_size_sample")) &&
        sequences.keys.sort == counters.keys.sort && sequences.all? { |table, value| value.eql?(counters.fetch(table)) }
    end

    def paging_parse(record, session, variant)
      frames = sampling_parse(record, session, variant)
      unless size_tables_equal?({ "frames" => frames }, { "frames" => record.fetch("frames") })
        raise Failure, "paging raw and declared frames differ"
      end
      frames
    end

    def paging_validate!(evidence, spec, mode, variant, role)
      raise Failure, "paging fixture count changed" unless evidence.fetch("fixtures").length == 9

      observations = paging_observations(spec)
      fixtures = evidence.fetch("fixtures").each_with_index.flat_map do |record, index|
        paging_parse(record, paging_session(observations.slice(index, 1)), variant)
      end
      tested = paging_parse(evidence.fetch("tested"), paging_session(observations.drop(9), mode), variant)
      frames = fixtures + tested
      clocks = [evidence.fetch("seed_clock"), *frames.map { |frame| frame.fetch("clock") }]
      fixture_backends = fixtures.map { |frame| frame.fetch("backend") }
      unless fixtures.all? { |frame| validation_context?([frame], role) } && validation_context?(tested, role) &&
             fixture_backends.uniq.length == 9 && !fixture_backends.include?(tested.first.fetch("backend")) &&
             frames.all? { |frame| frame.fetch("finished_setting") == "error" } &&
             clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == clocks.length
        raise Failure, "paging backend role settings or clock provenance changed"
      end
      successes = variant == "final" ? 12 : 10
      identities = frames.first([successes, 11].min).flat_map do |frame|
        frame.fetch("result").values_at("canonical_torrent_public_id", "canonical_torrent_source_public_id")
      end
      unless identities.uniq.length == identities.length && identities.all? { |id| id.is_a?(String) && IngestionProof::INGESTION_UUID.match?(id) }
        raise Failure, "paging generated identities changed"
      end
      before = attributes_empty
      prepared = nil
      frames.each_with_index do |frame, index|
        success = index < successes
        after = success ? paging_tables(before, frame, observations.fetch(index), index + 1) : before
        result = success ? frame.fetch("result") == paging_result(before, frame, observations.fetch(index).first) : !frame.key?("result")
        unless result && frame.values_at("within", "outside") == ["true", (variant == "reference").to_s] &&
               size_tables_equal?(frame.fetch("tables_before"), before) && size_tables_equal?(frame.fetch("tables_after"), after) &&
               size_tables_equal?(frame.fetch("tables_finish"), after)
          raise Failure, "paging full committed transition identity or scratch lifetime changed"
        end
        before = after
        prepared = after if index == 8
      end
      inputs = metadata_read_tables(evidence.fetch("seed_clock"))
      unless paging_read?(evidence.fetch("initial"), attributes_empty, inputs, paging_counters([], 0)) &&
             paging_read?(evidence.fetch("prepared"), prepared, inputs, paging_counters(observations.first(9), 9)) &&
             paging_read?(evidence.fetch("after"), before, inputs, paging_counters(observations, successes))
        raise Failure, "paging independent inputs state or exact sequence use changed"
      end
      true
    end

    def paging_isolated(spec, mode, variant, source, role)
      name = "#{spec.fetch(:name)}-#{mode}-#{variant}"
      database = "ingestion_paging_#{variant}"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")) + "\nUPDATE public.trust_tier SET created_at = transaction_timestamp();\nUPDATE public.media_domain SET created_at = transaction_timestamp();"
        clock = policy_setup!(seed, database, File.join(@paging_evidence, "#{name}-seed"))
        correction_observer!(database, variant)
        evidence = { "seed_clock" => clock, "initial" => paging_read(database, "#{name}-initial"), "fixtures" => [] }
        observations = paging_observations(spec)
        observations.first(9).each_with_index do |observation, index|
          session = paging_session([observation])
          raw = metadata_transport(correction_session(session), database, role, "#{name}-fixture-#{index + 1}")
          evidence.fetch("fixtures") << raw.merge("frames" => sampling_parse(raw, session, variant))
        end
        evidence["prepared"] = paging_read(database, "#{name}-prepared")
        session = paging_session(observations.drop(9), mode)
        raw = metadata_transport(correction_session(session), database, role, "#{name}-tested")
        evidence["tested"] = raw.merge("frames" => sampling_parse(raw, session, variant))
        evidence["after"] = paging_read(database, "#{name}-after")
        metadata_write("#{name}.json", JSON.pretty_generate(evidence) + "\n")
        check("paging #{name} full state identities inputs sequences and D4", paging_validate!(evidence, spec, mode, variant, role))
        { name:, validated: true, application_succeeded: variant == "final", equivalent: false,
          approved_delta: "D4", states: evidence.fetch("tested").fetch("frames").map { |frame| frame.fetch("state") } }
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def verify_ingestion_paging!
      directory = File.join(@contract.output_path, "ingestion-paging")
      raise Failure, "paging evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @paging_evidence = Dir.mktmpdir("run-", directory)
      previous_metadata, previous_correction = @metadata_evidence, @correction_evidence
      @metadata_evidence = @correction_evidence = @paging_evidence
      hashes = paging_source_hashes
      first = @checks.length
      cases = []
      completed = false
      begin
        paging_cases.each do |spec|
          PAGING_MODES.each do |mode|
            { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
              cases << paging_isolated(spec, mode, variant, source, role)
            end
          end
        end
        raise Failure, "paging source bytes changed during matrix" unless hashes == paging_source_hashes

        check("paging source bytes unchanged", true)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
          reference_failures_are_application_success: false, candidate_sha256: @contract.expected_candidate_sha256,
          final_sha256: @contract.final_sha256, postgres_image: @contract.postgres_image, source_sha256: hashes, checks:, cases:,
          limitations: ["Frozen post-commit D4 failures are not successful paging or semantic equivalence.",
                       "Bounded serial full-page/reuse cases do not prove concurrent paging, native callbacks or complete D3."] }
        metadata_write("report.json", JSON.pretty_generate(report) + "\n")
        @metadata_evidence, @correction_evidence = previous_metadata, previous_correction
      end
    end

    def paging_source_hashes
      path = "scripts/tests/database-ingestion-paging-test.rb"
      sampling_source_hashes.merge(path => Digest::SHA256.file(File.join(@contract.root, path)).hexdigest)
    end
  end
end
