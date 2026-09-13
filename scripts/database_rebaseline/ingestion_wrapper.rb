# frozen_string_literal: true

require "tmpdir"
require_relative "ingestion_identity"
require_relative "ingestion_hash_fill"
require_relative "ingestion_size"

module RevaerDatabaseRebaseline
  # Frozen warm evidence uses a real rollback, never a repaired temporary namespace.
  module IngestionWrapper
    include IngestionIdentity
    include IngestionHashFill
    include IngestionSize
    WRAPPER_MODES = %w[cold helpers-first warm-rollback].freeze
    WRAPPER_INPUTS = %w[indexer_definition indexer_instance policy_snapshot search_request search_request_indexer_run canonical_torrent_source_base_score].freeze
    WRAPPER_INPUT_CLOCKS = {
      "indexer_definition" => %w[created_at updated_at], "indexer_instance" => %w[created_at updated_at],
      "policy_snapshot" => %w[created_at], "search_request" => %w[created_at]
    }.freeze

    private

    def verify_ingestion_wrapper!
      @wrapper_validated_evidence = {}
      directory = File.join(@contract.output_path, "ingestion-wrapper")
      raise Failure, "wrapper evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @wrapper_evidence = Dir.mktmpdir("run-", directory)
      first = @checks.length
      source_hashes = wrapper_source_hashes
      cases = []
      completed = false
      begin
        wrapper_cases.each do |test_case|
          WRAPPER_MODES.each do |mode|
            variants = {}
            { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
              variants[variant] = wrapper_isolated(test_case, mode, variant, source, role)
            end
            equivalent = variants.fetch("reference") == variants.fetch("final")
            check("wrapper #{test_case.fetch(:name)} #{mode} exact application parity", equivalent)
            cases << { name: test_case.fetch(:name), mode:, equivalent:, variants: }
          end
        end
        check("wrapper source bytes unchanged during matrix", source_hashes == wrapper_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
                   warm_scope: "actual first ingestion rolled back, same-backend retry committed; not successful frozen committed reuse",
                   candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
                   postgres_image: @contract.postgres_image, source_sha256: source_hashes, checks:, cases: }
        path = File.join(@wrapper_evidence, "report.json")
        bytes = JSON.pretty_generate(report) + "\n"
        File.binwrite(path, bytes)
        @wrapper_validated_evidence[path] = Digest::SHA256.hexdigest(bytes) if report.fetch(:passed)
      end
    end

    def wrapper_source_hashes
      paths = Dir.glob("scripts/database_rebaseline/*.rb", base: @contract.root) + %w[
        scripts/database-rebaseline.rb scripts/stack_asset_exception.rb scripts/tests/database-ingestion-proof-test.rb
        scripts/tests/database-ingestion-wrapper-test.rb scripts/tests/database-ingestion-identity-test.rb
        scripts/tests/database-ingestion-hash-fill-test.rb scripts/tests/database-ingestion-proof-seed.sql
        scripts/tests/database-ingestion-size-test.rb
        scripts/tests/database-ingestion-helper-first.sql config/database-rebaseline.env
        .github/build-inputs.env crates/revaer-data/init.sql
      ]
      paths.sort.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] }
    end

    def wrapper_arguments(guid, hash: "a", minute: 0, size: 1024, seeders: 5, title: "Wrapper proof")
      { source_guid_input: "#{literal(guid)}::varchar", infohash_v1_input: "repeat(#{literal(hash)},40)::char(40)",
        observed_at_input: "'2026-09-10T00:#{format('%02d', minute)}:00Z'::timestamptz",
        size_bytes_input: "#{size}::bigint", seeders_input: seeders.to_s, title_raw_input: "#{literal(title)}::varchar" }
    end

    def wrapper_cases
      high = wrapper_arguments("high", seeders: 40, title: "Higher ranked title")
      low = wrapper_arguments("low", minute: 1, title: "Lower ranked title")
      scoring = { fixtures: [low, high, high.merge(observed_at_input: "'2026-09-10T00:02:00Z'::timestamptz")], scoring: true }
      pages = { fixtures: (1..10).map { |index| wrapper_arguments("page-#{index}", hash: index.to_s(16)) }, paging: true }
      [
        { name: "first-visible-wrapper", fixtures: [], arguments: wrapper_arguments("first"), wrapper: true },
        scoring.merge(name: "stored-score-no-seeder-promotion", arguments: wrapper_arguments("low", minute: 3, seeders: 99, title: "Low refreshed"), wrapper: false, best: "high"),
        scoring.merge(name: "stored-score-seeder-promotion", arguments: wrapper_arguments("low", minute: 3, seeders: 100, title: "Low refreshed"), wrapper: false, best: "low"),
        scoring.merge(name: "wrapper-visible-source-tail", arguments: wrapper_arguments("low", minute: 3, seeders: 99, title: "Low refreshed"), wrapper: true, best: "low"),
        pages.merge(name: "next-page-wrapper", arguments: wrapper_arguments("page-11", hash: "b", minute: 1), wrapper: true, new_page: true),
        pages.merge(name: "reuse-full-page-wrapper", arguments: wrapper_arguments("page-1", hash: "1", minute: 1), wrapper: true, new_page: false),
        { name: "third-size-sample", fixtures: [wrapper_arguments("sample", size: 100), wrapper_arguments("sample", minute: 1, size: 900)],
          arguments: wrapper_arguments("sample", minute: 2, size: 500), wrapper: true, samples: 3 },
        { name: "trim-size-samples", fixtures: (0...25).map { |index| wrapper_arguments("sample", minute: index, size: (index + 1) * 100) },
          arguments: wrapper_arguments("sample", minute: 25, size: 2600), wrapper: true, samples: 25 }
      ] + [{ name: "second-size-sample", fixtures: [wrapper_arguments("sample", size: 100)],
             arguments: wrapper_arguments("sample", minute: 1, size: 900), wrapper: true, samples: 2 }] +
        wrapper_identity_cases + wrapper_hash_fill_cases + wrapper_size_cases
    end

    def wrapper_isolated(test_case, mode, variant, source, role)
      database = "ingestion_wrapper_#{variant}"
      name = "#{test_case.fetch(:name)}-#{mode}-#{variant}"
      prefix = File.join(@wrapper_evidence, name)
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql"))
        seed += "\n#{size_seed_sql(test_case.fetch(:size_case))}" if test_case[:size_case]
        seed_query = "BEGIN; SELECT to_json(transaction_timestamp());\n#{seed}\nCOMMIT;"
        File.binwrite("#{prefix}-seed.sql", seed_query)
        seed_clock = JSON.parse(sql(seed_query, role: "postgres", database:))
        correction_observer!(database, variant)
        fixtures = []
        test_case.fetch(:fixtures).each_with_index do |arguments, index|
          wrapper_scores!(database, prefix) if test_case[:scoring] && index == 2
          fixture_case = { name: "wrapper-fixture", calls: [arguments], wrapper: true }
          fixtures.concat(wrapper_execute(fixture_case, database, role, "#{prefix}-fixture-#{index}"))
        end
        before = ingestion_snapshot(database)
        inputs_before = wrapper_inputs(database)
        size_before = size_input_snapshot(database, "#{prefix}-size-before") if test_case[:size_case]
        calls = [test_case.fetch(:arguments)] * (mode == "warm-rollback" ? 2 : 1)
        session = { name:, calls:, wrapper: test_case.fetch(:wrapper), helpers: mode == "helpers-first", rollback: mode == "warm-rollback", size_case: test_case[:size_case] }
        frames = wrapper_execute(session, database, role, prefix)
        evidence = { "seed_clock" => seed_clock, "fixtures" => fixtures, "before" => before, "inputs_before" => inputs_before,
                     "frames" => frames, "after" => ingestion_snapshot(database), "inputs_after" => wrapper_inputs(database) }
        evidence.merge!("size_inputs_before" => size_before, "size_inputs_after" => size_input_snapshot(database, "#{prefix}-size-after")) if test_case[:size_case]
        bytes = JSON.pretty_generate(evidence) + "\n"
        File.binwrite("#{prefix}.json", bytes)
        first = @checks.length
        wrapper_verify!(name, test_case, session, evidence, variant, role)
        if @checks.drop(first).all? { |entry| entry.fetch(:passed) }
          @wrapper_validated_evidence["#{prefix}.json"] = Digest::SHA256.hexdigest(bytes)
        end
        { "application" => compilation_comparable("fixture" => nil, "frames" => fixtures + frames), "inputs" => wrapper_comparable_inputs(evidence) }
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def wrapper_execute(test_case, database, role, prefix)
      query = correction_session(test_case)
      File.binwrite("#{prefix}.sql", query)
      outcome = result(query, role:, database:)
      File.binwrite("#{prefix}.stdout", outcome.stdout)
      File.binwrite("#{prefix}.stderr", outcome.stderr)
      raise Failure, "wrapper transport failed" unless outcome.success

      metadata_transport_json!(outcome.stdout) if test_case[:size_case]
      correction_parse(outcome.stdout, outcome.stderr, test_case)
    end

    def wrapper_scores!(database, prefix)
      query = <<~SQL
        INSERT INTO public.canonical_torrent_source_base_score (
          canonical_torrent_id,canonical_torrent_source_id,score_total_base,
          score_seed,score_leech,score_age,score_trust,score_health,score_reputation,computed_at
        ) SELECT c.canonical_torrent_id,s.canonical_torrent_source_id,
          CASE s.source_guid WHEN 'high' THEN 100 ELSE 10 END,0,0,0,0,0,0,'2026-09-10T00:00:00Z'
        FROM public.canonical_torrent c JOIN public.canonical_torrent_source s ON s.infohash_v1=c.infohash_v1
        WHERE s.source_guid IN ('high','low');
      SQL
      File.binwrite("#{prefix}-base-scores.sql", query)
      sql(query, role: "postgres", database:)
    end

    def wrapper_inputs(database)
      WRAPPER_INPUTS.to_h do |table|
        value = sql("SELECT COALESCE(json_agg(row_to_json(r) ORDER BY to_jsonb(r)), '[]') FROM public.#{identifier(table)} r", role: "postgres", database:)
        [table, JSON.parse(value)]
      end
    end

    def wrapper_verify!(name, test_case, session, evidence, variant, role)
      frames = evidence.fetch("frames")
      fixtures = evidence.fetch("fixtures")
      all = fixtures + frames
      check("#{name} real successful calls", all.all? { |frame| frame.fetch("state") == "00000" })
      capabilities = { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" }
      check("#{name} direct roles and unchanged settings", all.all? { |frame| frame.fetch("role") == capabilities && frame.values_at("before", "after") == %w[error error] })
      backend = frames.first.fetch("backend")
      check("#{name} fixture-separated same tested backend", backend.match?(/\A[1-9][0-9]*\z/) && frames.all? { |frame| frame.fetch("backend") == backend } && fixtures.none? { |frame| frame.fetch("backend") == backend })
      clocks = all.map { |frame| frame.fetch("clock") }
      check("#{name} distinct real transaction clocks", clocks.uniq.length == all.length && clocks.all? { |clock| clock.match?(/\A\d{4}-\d\d-\d\dT.*\+00:00\z/) })
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      fixture_continuity = fixtures.empty? ? evidence.fetch("before") == empty : correction_snapshots?({}, empty, fixtures, evidence.fetch("before"))
      check("#{name} exact persisted and rollback continuity", fixture_continuity && correction_snapshots?(session, evidence.fetch("before"), frames, evidence.fetch("after")))
      lifetime = fixtures.all? { |frame| frame.values_at("within", "outside") == ["true", (variant == "reference").to_s] }
      lifetime &&= frames.each_with_index.all? do |frame, index|
        outside = variant == "reference" && !(session[:rollback] && index.zero?)
        frame.values_at("within", "outside") == ["true", outside.to_s]
      end
      check("#{name} exact D4 lifetime including rollback", lifetime)
      check("#{name} immutable read inputs", evidence.fetch("inputs_before") == evidence.fetch("inputs_after"))
      seed_clock = evidence.fetch("seed_clock")
      check("#{name} input clock provenance", seed_clock.match?(/\A\d{4}-\d\d-\d\dT.*\+00:00\z/) && WRAPPER_INPUT_CLOCKS.all? do |table, columns|
        evidence.fetch("inputs_before").fetch(table).all? { |row| columns.all? { |column| row.fetch(column) == seed_clock } }
      end)
      check("#{name} exact wrapper/scoring/page/sample result", frames.all? { |frame| wrapper_outcome?(test_case, frame) })
      hash_fill_verify!(name, test_case, session, evidence) if test_case[:hash_fill]
      check("#{name} independent sampling decisions full state domain inputs and rollback", size_evidence?(test_case.fetch(:size_case), session, evidence)) if test_case[:size_case]
    end

    def wrapper_comparable_inputs(evidence)
      evidence.fetch("inputs_before").to_h do |table, rows|
        [table, rows.map do |row|
          row.to_h { |column, value| [column, WRAPPER_INPUT_CLOCKS.fetch(table, []).include?(column) && value == evidence.fetch("seed_clock") ? "<validated-seed-transaction>" : value] }
        end]
      end
    end

    def wrapper_outcome?(test_case, frame)
      result = frame.fetch("result")
      return false unless result.keys.sort == %w[canonical_changed canonical_torrent_public_id canonical_torrent_source_public_id durable_source_created observation_created]

      tables = frame.fetch("tables_after")
      canonical = tables.fetch("canonical_torrent").find { |row| row.fetch("canonical_torrent_public_id") == result.fetch("canonical_torrent_public_id") }
      source = tables.fetch("canonical_torrent_source").find { |row| row.fetch("canonical_torrent_source_public_id") == result.fetch("canonical_torrent_source_public_id") }
      return false unless canonical && source

      fresh = test_case.fetch(:fixtures).empty? || test_case[:new_page] == true || test_case.fetch(:identity, {})[:operation] == "new"
      return false unless result.values_at("observation_created", "durable_source_created", "canonical_changed") == [fresh, fresh, fresh]
      return wrapper_scoring?(test_case, tables, canonical, source) if test_case[:scoring]
      return wrapper_paging?(test_case, frame) if test_case[:paging]
      return wrapper_samples?(test_case, tables, canonical) if test_case[:samples]
      return false if test_case[:identity] && !wrapper_identity?(test_case, frame, canonical, source)

      tables.fetch("canonical_torrent_best_source_context").one? { |row| row.fetch("canonical_torrent_id") == canonical.fetch("canonical_torrent_id") && row.fetch("canonical_torrent_source_id") == source.fetch("canonical_torrent_source_id") }
    end

    def wrapper_scoring?(test_case, tables, canonical, source)
      sources = tables.fetch("canonical_torrent_source").to_h { |row| [row.fetch("source_guid"), row] }
      return false unless sources.keys.sort == %w[high low] && source == sources.fetch("low")

      scores = tables.fetch("canonical_torrent_source_context_score").to_h { |row| [row.fetch("canonical_torrent_source_id"), row.fetch("score_total_context")] }
      expected = { sources.fetch("high").fetch("canonical_torrent_source_id") => 100, source.fetch("canonical_torrent_source_id") => 10 }
      best = tables.fetch("canonical_torrent_best_source_context")
      scores == expected && canonical.fetch("title_display") == "Higher ranked title" && best.length == 1 &&
        best.first.fetch("canonical_torrent_source_id") == sources.fetch(test_case.fetch(:best)).fetch("canonical_torrent_source_id")
    end

    def wrapper_paging?(test_case, frame)
      tables = frame.fetch("tables_after")
      pages = tables.fetch("search_page").sort_by { |row| row.fetch("page_number") }
      items = tables.fetch("search_page_item")
      return pages == frame.fetch("tables_before").fetch("search_page") && items == frame.fetch("tables_before").fetch("search_page_item") unless test_case[:new_page]
      return false unless pages.map { |row| row.fetch("page_number") } == [1, 2] && pages.map { |row| row.fetch("sealed_at") } == [frame.fetch("clock"), nil]

      pages.map { |page| items.select { |item| item.fetch("search_page_id") == page.fetch("search_page_id") }.map { |item| item.fetch("position") }.sort } == [(1..10).to_a, [1]] &&
        tables.fetch("search_request_canonical").length == 11 && tables.fetch("canonical_torrent_best_source_context").length == 11
    end

    def wrapper_samples?(test_case, tables, canonical)
      values = case test_case.fetch(:samples)
               when 2 then [100, 900]
               when 3 then [100, 500, 900]
               when 25 then (2..26).map { |value| value * 100 }
               else return false
               end
      samples = tables.fetch("canonical_size_sample")
      rollups = tables.fetch("canonical_size_rollup")
      median = values.length == 2 ? 500 : values.fetch(values.length / 2)
      displayed = values.length == 2 ? 100 : median
      samples.map { |row| row.fetch("size_bytes") }.sort == values && canonical.fetch("size_bytes") == displayed &&
        rollups.length == 1 && rollups.first.values_at("sample_count", "size_median", "size_min", "size_max") == [values.length, median, values.min, values.max]
    end
  end
end
