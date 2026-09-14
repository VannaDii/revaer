# frozen_string_literal: true

require_relative "ingestion_wrapper"
require_relative "ingestion_sampling"

module RevaerDatabaseRebaseline
  # Independent committed wrapper outcomes; frozen D4 errors are not parity.
  module IngestionCommittedScoring
    COMMITTED_SCORING_MODES = %w[cold helpers-first].freeze
    COMMITTED_SCORING_SEQUENCES = {
      "canonical_torrent" => "canonical_torrent_id",
      "canonical_torrent_source" => "canonical_torrent_source_id",
      "search_request_source_observation" => "observation_id",
      "canonical_torrent_source_base_score" => "canonical_torrent_source_base_score_id",
      "canonical_torrent_source_context_score" => "canonical_torrent_source_context_score_id",
      "canonical_torrent_best_source_context" => "canonical_torrent_best_source_context_id",
      "search_request_canonical" => "search_request_canonical_id",
      "search_page" => "search_page_id", "search_page_item" => "search_page_item_id"
    }.freeze
    COMMITTED_SCORING_BRANCHES = {
      "stored-score" => ["0052_indexer_search_result_ingest_proc.sql", 1851, 1857],
      "score-upsert" => ["0052_indexer_search_result_ingest_proc.sql", 1883, 1911],
      "ranked-source" => ["0052_indexer_search_result_ingest_proc.sql", 1914, 1922],
      "seeder-promotion" => ["0052_indexer_search_result_ingest_proc.sql", 1935, 1965],
      "title-selection" => ["0052_indexer_search_result_ingest_proc.sql", 2063, 2086],
      "public-wrapper-tail" => ["0120_search_result_ingest_seed_best_source_context.sql", 100, 129]
    }.freeze

    private

    def committed_scoring_cases
      names = %w[stored-score-no-seeder-promotion stored-score-seeder-promotion]
      names.map do |name|
        fixture = wrapper_cases.find { |item| item.fetch(:name) == name }
        raise Failure, "committed scoring fixture missing" unless fixture

        incoming = name == names.first ? 99 : 100
        fixture.merge(name: "committed-#{name}", wrapper: true, best: "low",
          promotion: { incoming:, best_seeders: 40 },
          fixtures: fixture.fetch(:fixtures).map { |args| args.merge(size_bytes_input: "NULL::bigint") },
          arguments: fixture.fetch(:arguments).merge(size_bytes_input: "NULL::bigint"))
      end
    end

    def committed_scoring_session(spec, mode)
      raise Failure, "unknown committed scoring mode" unless COMMITTED_SCORING_MODES.include?(mode)

      warm = spec.fetch(:fixtures).last
      refresh = wrapper_arguments("high", minute: 4, seeders: 40, title: "Higher refreshed").merge(size_bytes_input: "NULL::bigint")
      { calls: [warm, spec.fetch(:arguments), refresh, spec.fetch(:arguments)],
        wrapper: true, helpers: mode == "helpers-first", finish_setting: true }
    end

    def committed_scoring_sources
      COMMITTED_SCORING_BRANCHES.to_h do |name, (file, first, last)|
        path = "crates/revaer-data/migrations/#{file}"
        body = File.binread(File.join(@contract.root, path)).lines.slice(first - 1, last - first + 1).join
        [name, { path:, first_line: first, last_line: last, sha256: Digest::SHA256.hexdigest(body), source: body }]
      end
    end

    def committed_scoring_answers(spec)
      { "sources" => { "low" => 1, "high" => 2 }, "observations" => { "low" => 1, "high" => 2 },
        "base_scores" => { "low" => 10.0, "high" => 100.0 },
        "context_scores" => [[0.0, 100.0], [10.0, 100.0], [10.0, 100.0], [10.0, 100.0]],
        "title_display" => ["Higher ranked title", "Higher ranked title", "Higher refreshed", "Higher refreshed"],
        "best_source_ids" => [2, 1, 2, 1],
        "incoming_seeders" => spec.fetch(:promotion).fetch(:incoming), "ranked_source_seeders" => 40,
        "v1_promotes_low" => spec.fetch(:promotion).fetch(:incoming) == 100,
        "wrapper_selects_low_despite_lower_score" => true }
    end

    def committed_scoring_tables(spec, before, clock, index)
      return wrapper_promotion_after(spec, before, clock) if [1, 3].include?(index)

      tables = before.transform_values { |rows| rows.map(&:dup) }
      minute, title = index.zero? ? [2, "Higher ranked title"] : [4, "Higher refreshed"]
      tables.fetch("canonical_torrent").first.merge!("title_display" => title, "updated_at" => clock)
      tables.fetch("canonical_torrent_source").last.merge!("last_seen_at" => sampling_observed(minute), "updated_at" => clock)
      tables.fetch("search_request_source_observation").last.merge!("observed_at" => sampling_observed(minute), "title_raw" => title)
      tables.fetch("canonical_torrent_source_context_score").last.merge!("score_total_context" => 100.0, "computed_at" => clock)
      tables.fetch("canonical_torrent_best_source_context").first.merge!("canonical_torrent_source_id" => 2, "computed_at" => clock)
      tables
    end

    def committed_scoring_sequences(calls, scores: false, promotions: 0)
      return COMMITTED_SCORING_SEQUENCES.to_h { |table, _column| [table, nil] } if calls.zero?

      { "canonical_torrent" => 1, "canonical_torrent_source" => [calls, 2].min,
        "search_request_source_observation" => [calls, 2].min,
        "canonical_torrent_source_base_score" => scores ? 2 : nil,
        "canonical_torrent_source_context_score" => calls,
        "canonical_torrent_best_source_context" => calls + promotions,
        "search_request_canonical" => calls, "search_page" => 1, "search_page_item" => 1 }
    end

    def committed_scoring_read(database, name)
      inputs = IngestionPolicy::POLICY_READ_TABLES.map do |table|
        "#{literal(table)}, (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)::text), '[]') FROM public.#{identifier(table)} t)"
      end
      sequences = COMMITTED_SCORING_SEQUENCES.map do |table, column|
        "#{literal(table)}, (SELECT last_value FROM pg_catalog.pg_sequences WHERE schemaname = 'public' AND format('%I.%I', schemaname, sequencename)::regclass = pg_get_serial_sequence(#{literal("public.#{table}")}, #{literal(column)})::regclass)"
      end
      query = "SELECT json_build_object('tables', ingestion_observation.snapshot(), 'inputs', json_build_object(#{inputs.join(',')}), 'sequences', json_build_object(#{sequences.join(',')}));"
      raw = metadata_transport(query, database, "postgres", "#{name}-read")
      raw.merge("data" => metadata_read_parse(raw))
    end

    def committed_scoring_read?(record, tables, inputs, sequences)
      data = metadata_read_parse(record)
      JSON.generate(data) == JSON.generate(record.fetch("data")) && data.keys.sort == %w[inputs sequences tables] &&
        size_tables_equal?(data.fetch("tables"), tables) && size_tables_equal?(data.fetch("inputs"), inputs) &&
        data.fetch("sequences").eql?(sequences)
    end

    def committed_scoring_parse(record, session, variant)
      frames = sampling_parse(record, session, variant)
      unless size_tables_equal?({ "frames" => frames }, { "frames" => record.fetch("frames") })
        raise Failure, "committed scoring raw and declared frames differ"
      end
      frames
    end

    def committed_scoring_transition!(frame, before, after, identity, variant)
      unless (identity ? frame.fetch("result") == identity : !frame.key?("result")) &&
             frame.values_at("within", "outside") == ["true", (variant == "reference").to_s] &&
             size_tables_equal?(frame.fetch("tables_before"), before) && size_tables_equal?(frame.fetch("tables_after"), after) &&
             size_tables_equal?(frame.fetch("tables_finish"), after)
        raise Failure, "committed scoring full transition result identity or scratch lifetime changed"
      end
    end

    def committed_scoring_validate!(evidence, spec, mode, variant, role)
      raise Failure, "committed scoring variant changed" unless %w[reference final].include?(variant)
      raise Failure, "committed scoring branch answers changed" unless evidence.fetch("answers") == committed_scoring_answers(spec)

      fixtures = evidence.fetch("fixtures")
      raise Failure, "committed scoring fixture count changed" unless fixtures.length == 3

      fixture_frames = fixtures.each_with_index.map do |record, index|
        session = { calls: [spec.fetch(:fixtures).fetch(index)], wrapper: true, finish_setting: true }
        committed_scoring_parse(record, session, variant).first
      end
      tested = committed_scoring_parse(evidence.fetch("tested"), committed_scoring_session(spec, mode), variant)
      all = fixture_frames + tested
      clocks = [evidence.fetch("seed_clock"), *all.map { |frame| frame.fetch("clock") }]
      backends = fixture_frames.map { |frame| frame.fetch("backend") }
      unless fixture_frames.all? { |frame| validation_context?([frame], role) } && validation_context?(tested, role) &&
             backends.uniq.length == 3 && !backends.include?(tested.first.fetch("backend")) &&
             all.all? { |frame| frame.fetch("finished_setting") == "error" } &&
             clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == clocks.length
        raise Failure, "committed scoring backend role settings or clock provenance changed"
      end
      canonical = fixture_frames.first.fetch("result").fetch("canonical_torrent_public_id")
      low, high = fixture_frames.first(2).map { |frame| frame.fetch("result").fetch("canonical_torrent_source_public_id") }
      unless [canonical, low, high].uniq.length == 3 && [canonical, low, high].all? { |id| id.is_a?(String) && IngestionProof::INGESTION_UUID.match?(id) }
        raise Failure, "committed scoring generated identities changed"
      end
      result = ->(source, fresh, changed) do
        { "canonical_torrent_public_id" => canonical, "canonical_torrent_source_public_id" => source,
          "observation_created" => fresh, "durable_source_created" => fresh, "canonical_changed" => changed }
      end
      snapshots = wrapper_promotion_fixtures(spec.fetch(:promotion), fixture_frames.map { |frame| frame.fetch("clock") }, fixture_frames.map { |frame| frame.fetch("result") })
      inputs = metadata_read_tables(evidence.fetch("seed_clock"))
      scored_inputs = inputs.merge("canonical_torrent_source_base_score" => wrapper_promotion_inputs(evidence.fetch("seed_clock")).fetch("canonical_torrent_source_base_score"))
      unless committed_scoring_read?(evidence.fetch("initial"), attributes_empty, inputs, committed_scoring_sequences(0))
        raise Failure, "committed scoring independent initial inputs state or sequences changed"
      end
      before = attributes_empty
      fixture_frames.each_with_index do |frame, index|
        after = snapshots.fetch(index)
        committed_scoring_transition!(frame, before, after, result.call(index.zero? ? low : high, index < 2, index.zero?), variant)
        unless committed_scoring_read?(fixtures.fetch(index).fetch("read"), after, index < 2 ? inputs : scored_inputs,
                                      committed_scoring_sequences(index + 1, scores: index == 2))
          raise Failure, "committed scoring independent fixture inputs state or sequences changed"
        end
        before = after
      end
      tested.each_with_index do |frame, index|
        success = variant == "final" || index.zero?
        after = success ? committed_scoring_tables(spec, before, frame.fetch("clock"), index) : before
        identity = success ? result.call([1, 3].include?(index) ? low : high, false, false) : nil
        committed_scoring_transition!(frame, before, after, identity, variant)
        before = after
      end
      promotions = variant == "final" && spec.fetch(:promotion).fetch(:incoming) == 100 ? 2 : 0
      sequences = committed_scoring_sequences(variant == "final" ? 7 : 4, scores: true, promotions:)
      unless committed_scoring_read?(evidence.fetch("after"), before, scored_inputs, sequences)
        raise Failure, "committed scoring independent final inputs state or sequences changed"
      end
      true
    end

    def committed_scoring_isolated(spec, mode, variant, source, role)
      name = "#{spec.fetch(:name)}-#{mode}-#{variant}"
      database = "ingestion_committed_scoring_#{variant}"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")) + "\nUPDATE public.trust_tier SET created_at = transaction_timestamp();\nUPDATE public.media_domain SET created_at = transaction_timestamp();"
        clock = policy_setup!(seed, database, File.join(@committed_scoring_evidence, "#{name}-seed"))
        correction_observer!(database, variant)
        evidence = { "seed_clock" => clock, "answers" => committed_scoring_answers(spec),
          "initial" => committed_scoring_read(database, "#{name}-initial"), "fixtures" => [] }
        spec.fetch(:fixtures).each_with_index do |arguments, index|
          wrapper_scores!(database, File.join(@committed_scoring_evidence, name)) if index == 2
          session = { calls: [arguments], wrapper: true, finish_setting: true }
          raw = metadata_transport(correction_session(session), database, role, "#{name}-fixture-#{index}")
          evidence.fetch("fixtures") << raw.merge("frames" => sampling_parse(raw, session, variant),
            "read" => committed_scoring_read(database, "#{name}-fixture-#{index}"))
        end
        session = committed_scoring_session(spec, mode)
        raw = metadata_transport(correction_session(session), database, role, "#{name}-tested")
        evidence["tested"] = raw.merge("frames" => sampling_parse(raw, session, variant))
        evidence["after"] = committed_scoring_read(database, "#{name}-after")
        metadata_write("#{name}.json", JSON.pretty_generate(evidence) + "\n")
        check("committed scoring #{name} full state inputs identities scores titles sequences and exact D4", committed_scoring_validate!(evidence, spec, mode, variant, role))
        { name:, validated: true, equivalent: false, approved_delta: "D4",
          states: evidence.fetch("tested").fetch("frames").map { |frame| frame.fetch("state") } }
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def verify_ingestion_committed_scoring!
      directory = File.join(@contract.output_path, "ingestion-committed-scoring")
      raise Failure, "committed scoring evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @committed_scoring_evidence = Dir.mktmpdir("run-", directory)
      previous_metadata, previous_correction = @metadata_evidence, @correction_evidence
      @metadata_evidence = @correction_evidence = @committed_scoring_evidence
      hashes = committed_scoring_source_hashes
      first = @checks.length
      cases = []
      completed = false
      begin
        committed_scoring_cases.each do |spec|
          COMMITTED_SCORING_MODES.each do |mode|
            { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
              cases << committed_scoring_isolated(spec, mode, variant, source, role)
            end
          end
        end
        check("committed scoring source bytes unchanged", hashes == committed_scoring_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
          candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
          postgres_image: @contract.postgres_image, source_sha256: hashes, source_branches: committed_scoring_sources, checks:, cases:,
          limitations: ["Frozen first-call success and exact subsequent D4 failures are not successful equivalence.",
            "Sequence consumption discriminates promotion versus wrapper-only writes; it is not a native callsite trace.",
            "No hash-fill, paging-boundary, concurrency, native closure or complete D3 claim; integration and full gates remain pending."] }
        metadata_write("report.json", JSON.pretty_generate(report) + "\n")
        @metadata_evidence, @correction_evidence = previous_metadata, previous_correction
      end
    end

    def committed_scoring_source_hashes
      paths = %w[scripts/tests/database-ingestion-committed-scoring-test.rb scripts/tests/database-ingestion-sampling-test.rb
        scripts/tests/database-ingestion-attributes-test.rb]
      wrapper_source_hashes.merge(paths.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] })
    end
  end
end
