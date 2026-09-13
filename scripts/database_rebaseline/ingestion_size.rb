# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Frozen sampling decisions with full independently modeled one-result state.
  module IngestionSize
    SIZE_CUTOFF = 10_995_116_277_760
    SIZE_DOMAINS = { "movies" => 1, "tv" => 2, "audiobooks" => 3, "ebooks" => 4, "software" => 5,
                     "adult_movies" => 6, "adult_scenes" => 7 }.freeze
    SIZE_TITLE = "Size proof"

    private

    def wrapper_size_cases
      specs = [
        ["one-sample", 1024, nil, nil, [], true],
        ["below-cutoff", SIZE_CUTOFF - 1, nil, nil, [], true],
        ["at-cutoff", SIZE_CUTOFF, nil, nil, [], true],
        ["above-cutoff", SIZE_CUTOFF + 1, nil, nil, [], false],
        ["single-movies", SIZE_CUTOFF + 1, nil, nil, ["movies"], false],
        ["multiple-exceptions", SIZE_CUTOFF + 1, nil, nil, %w[ebooks software], false],
        ["multiple-mixed", SIZE_CUTOFF + 1, nil, nil, %w[movies ebooks], false],
        ["effective-movies", SIZE_CUTOFF + 1, "movies", "movies", ["ebooks"], false],
        ["requested-only", SIZE_CUTOFF + 1, "ebooks", nil, [], false],
        ["zero", 0, "ebooks", "ebooks", [], false],
        ["null", nil, "ebooks", "ebooks", [], false],
        ["title-fallback", 1024, "ebooks", "ebooks", [], false, true],
        ["large-title-fallback", SIZE_CUTOFF + 1, "ebooks", "ebooks", [], false, true]
      ]
      %w[ebooks audiobooks software].each do |domain|
        specs << ["single-#{domain}", SIZE_CUTOFF + 1, nil, nil, [domain], true]
        specs << ["effective-#{domain}", SIZE_CUTOFF + 1, domain, domain, %w[movies tv], true]
      end
      specs.map do |name, bytes, requested, effective, domains, sampled, fallback|
        arguments = wrapper_arguments("size-source", title: SIZE_TITLE).merge(size_bytes_input: "#{bytes.nil? ? 'NULL' : bytes}::bigint")
        arguments[:infohash_v1_input] = "NULL::char(40)" if fallback
        { name: "size-#{name}", fixtures: [], arguments:, wrapper: true,
          size_case: { bytes:, requested:, effective:, domains:, sampled:, fallback: fallback == true } }
      end
    end

    def size_domain_id(domain)
      domain.nil? ? nil : SIZE_DOMAINS.fetch(domain)
    end

    def size_seed_sql(spec)
      links = spec.fetch(:domains).map { |domain| "(569001, #{size_domain_id(domain)})" }
      <<~SQL
        UPDATE public.media_domain SET created_at = transaction_timestamp();
        UPDATE public.search_request
        SET requested_media_domain_id = #{size_domain_id(spec.fetch(:requested)) || 'NULL'},
            effective_media_domain_id = #{size_domain_id(spec.fetch(:effective)) || 'NULL'}
        WHERE search_request_id = 569001;
        #{links.empty? ? '' : "INSERT INTO public.indexer_instance_media_domain (indexer_instance_id, media_domain_id) VALUES #{links.join(',')};"}
      SQL
    end

    def size_input_snapshot(database, prefix)
      query = <<~SQL
        SELECT json_build_object(
          'media_domain', (SELECT json_agg(row_to_json(t) ORDER BY media_domain_id) FROM public.media_domain t),
          'indexer_instance_media_domain', (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY indexer_instance_media_domain_id), '[]') FROM public.indexer_instance_media_domain t));
      SQL
      outcome = result(query, role: "postgres", database:)
      { "sql" => query, "stdout" => outcome.stdout, "stderr" => outcome.stderr }.each do |extension, bytes|
        File.open("#{prefix}.#{extension}", File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(bytes) }
      end
      raise Failure, "size read transport failed" unless outcome.success && outcome.stderr.empty?

      { "stdout" => outcome.stdout, "stderr" => outcome.stderr, "data" => metadata_json_parse(outcome.stdout) }
    end

    def size_tables_equal?(actual, expected)
      return false unless actual.keys.sort == expected.keys.sort

      expected.all? do |table, rows|
        encode = ->(items) { items.map { |row| JSON.generate(row.sort) }.sort }
        encode.call(actual.fetch(table)) == encode.call(rows)
      end
    end

    def size_expected_tables(spec, frame, ordinal)
      identities = frame.fetch("result").slice("canonical_torrent_public_id", "canonical_torrent_source_public_id")
      shape = { title: SIZE_TITLE, normalized: "size proof", answers: [], signals: [], release: nil }
      tables = attributes_initial_tables(shape, frame.fetch("clock"), identities)
      canonical = tables.fetch("canonical_torrent").first
      canonical["size_bytes"] = spec.fetch(:sampled) || spec.fetch(:fallback) ? spec.fetch(:bytes) : nil
      if spec.fetch(:fallback)
        canonical.merge!("identity_strategy" => "title_size_fallback", "identity_confidence" => 0.6,
          "title_size_hash" => Digest::SHA256.hexdigest("size proof|#{spec.fetch(:bytes)}"))
        %w[canonical_torrent canonical_torrent_source search_request_source_observation].each do |table|
          tables.fetch(table).first.merge!("infohash_v1" => nil, "magnet_hash" => nil)
        end
      end
      %w[canonical_torrent_source search_request_source_observation].each do |table|
        tables.fetch(table).first.merge!("source_guid" => "size-source", "size_bytes" => spec.fetch(:bytes))
      end
      tables["canonical_torrent_best_source_context"] = [{ "canonical_torrent_best_source_context_id" => 1,
        "context_key_type" => "search_request", "context_key_id" => 569001, "canonical_torrent_id" => 1,
        "canonical_torrent_source_id" => 1, "computed_at" => frame.fetch("clock") }]
      tables.fetch("canonical_torrent_source_context_score").first.merge!(
        "score_total_context" => 0.0, "score_policy_adjust" => 0.0, "score_tag_adjust" => 0.0)
      if spec.fetch(:sampled)
        tables["canonical_size_sample"] = [{ "canonical_size_sample_id" => 1, "canonical_torrent_id" => 1,
          "observed_at" => "2026-09-10T00:00:00+00:00", "size_bytes" => spec.fetch(:bytes) }]
        tables["canonical_size_rollup"] = [{ "canonical_size_rollup_id" => 1, "canonical_torrent_id" => 1,
          "sample_count" => 1, "size_median" => spec.fetch(:bytes), "size_min" => spec.fetch(:bytes),
          "size_max" => spec.fetch(:bytes), "updated_at" => frame.fetch("clock") }]
      end
      # A rolled-back first call consumes the generated identities, not fixed seed IDs.
      tables.transform_values do |rows|
        rows.map { |row| row.to_h { |key, value| [key, key.end_with?("_id") && value == 1 ? ordinal + 1 : value] } }
      end
    end

    def size_expected_inputs(spec, clock)
      base = hash_fill_read_tables(clock)
      base.fetch("search_request").first.merge!("requested_media_domain_id" => size_domain_id(spec.fetch(:requested)),
        "effective_media_domain_id" => size_domain_id(spec.fetch(:effective)))
      domains = { "media_domain" => metadata_read_tables(clock).fetch("media_domain"),
        "indexer_instance_media_domain" => spec.fetch(:domains).each_with_index.map do |domain, index|
          { "indexer_instance_media_domain_id" => index + 1, "indexer_instance_id" => 569001, "media_domain_id" => size_domain_id(domain) }
        end }
      [base, domains]
    end

    def size_evidence?(spec, session, evidence)
      return false unless evidence.fetch("fixtures").empty? && evidence.fetch("before") == attributes_empty

      frames = evidence.fetch("frames")
      return false unless frames.length == (session[:rollback] ? 2 : 1)

      clocks = [evidence.fetch("seed_clock"), *frames.map { |frame| frame.fetch("clock") }]
      return false unless clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == clocks.length

      base, domains = size_expected_inputs(spec, evidence.fetch("seed_clock"))
      return false unless %w[inputs_before inputs_after].all? { |key| size_tables_equal?(evidence.fetch(key), base) }
      return false unless %w[size_inputs_before size_inputs_after].all? do |key|
        read = evidence.fetch(key)
        read.fetch("stderr").empty? && metadata_json_parse(read.fetch("stdout")) == read.fetch("data") && size_tables_equal?(read.fetch("data"), domains)
      end

      uuids = frames.flat_map { |frame| frame.fetch("result").values_at("canonical_torrent_public_id", "canonical_torrent_source_public_id") }
      return false unless uuids.uniq.length == frames.length * 2 && uuids.all? { |value| value.is_a?(String) && IngestionProof::INGESTION_UUID.match?(value) }

      frames.each_with_index.all? do |frame, index|
        expected = size_expected_tables(spec, frame, index)
        finish = session[:rollback] && index.zero? ? attributes_empty : expected
        frame.fetch("tables_before") == attributes_empty && size_tables_equal?(frame.fetch("tables_after"), expected) &&
          size_tables_equal?(frame.fetch("tables_finish"), finish) && (index < frames.length - 1 || size_tables_equal?(evidence.fetch("after"), finish))
      end
    end
  end
end
