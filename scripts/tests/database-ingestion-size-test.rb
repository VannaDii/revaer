# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionSizeTest
    private

    def size_test_evidence(test_case, mode, variant)
      seed = "2026-09-13T00:00:00+00:00"
      spec = test_case.fetch(:size_case)
      base, domains = size_expected_inputs(spec, seed)
      role = variant == "reference" ? "postgres" : @runtime
      session = { rollback: mode == "warm-rollback", size_case: spec }
      frames = Array.new(session.fetch(:rollback) ? 2 : 1) do |index|
        frame = { "clock" => "2026-09-13T00:00:0#{index + 1}+00:00", "backend" => "51541", "state" => "00000",
          "before" => "error", "after" => "error", "within" => "true",
          "outside" => (variant == "reference" && !(session[:rollback] && index.zero?)).to_s,
          "role" => { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" },
          "result" => { "canonical_torrent_public_id" => "56900000-0000-4000-8000-#{format('%012d', index * 2 + 1)}",
            "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-#{format('%012d', index * 2 + 2)}",
            "canonical_changed" => true, "observation_created" => true, "durable_source_created" => true } }
        expected = size_expected_tables(spec, frame, index)
        frame.merge("tables_before" => attributes_empty, "tables_after" => expected,
          "tables_finish" => session[:rollback] && index.zero? ? attributes_empty : expected)
      end
      read = { "stdout" => JSON.generate(domains) + "\n", "stderr" => "", "data" => domains }
      evidence = { "seed_clock" => seed, "fixtures" => [], "before" => attributes_empty, "frames" => frames,
        "after" => frames.last.fetch("tables_finish"), "inputs_before" => base, "inputs_after" => base,
        "size_inputs_before" => read, "size_inputs_after" => read }
      [session, Marshal.load(Marshal.dump(evidence))]
    end

    def size_tests!
      cases = wrapper_size_cases
      assert(cases.length == 19 && cases.map { |item| item.fetch(:name) }.uniq.length == 19, "nineteen distinct sampling decisions")
      assert(IngestionSize::SIZE_CUTOFF == 10 * (1024**4), "exact frozen ten TiB cutoff")
      assert(cases.count { |item| item.fetch(:size_case).fetch(:sampled) } == 9, "nine independently selected sample-admission controls")
      cases.each do |test_case|
        spec = test_case.fetch(:size_case)
        query = size_seed_sql(spec)
        assert(!query.match?(/CREATE.*FUNCTION|SET ROLE|SET plpgsql|DROP TABLE|DISCARD/), "fixture cannot replace code or repair compilation")
        assert(query.include?("effective_media_domain_id = #{size_domain_id(spec.fetch(:effective)) || 'NULL'}"), "effective domain is explicit")
        IngestionWrapper::WRAPPER_MODES.each do |mode|
          %w[reference final].each do |variant|
            session, evidence = size_test_evidence(test_case, mode, variant)
            assert(size_evidence?(spec, session, evidence), "valid #{test_case.fetch(:name)} #{mode} #{variant}")
            frames = evidence.fetch("frames")
            assert(frames.first.fetch("tables_after").fetch("canonical_size_sample").length == (spec.fetch(:sampled) ? 1 : 0), "explicit sample presence")
            expected_size = spec.fetch(:sampled) || spec.fetch(:fallback) ? spec.fetch(:bytes) : nil
            assert(frames.first.fetch("tables_after").fetch("canonical_torrent").first.fetch("size_bytes") == expected_size, "fallback and rejected sampling differ")
            next unless mode == "warm-rollback"

            assert(frames.first.fetch("tables_finish") == attributes_empty, "whole first call rolls back")
            assert(frames.last.fetch("tables_after").fetch("canonical_torrent").first.fetch("canonical_torrent_id") == 2, "identity consumed across rollback")
          end
        end
      end
      size_mutation_tests!(cases)
    end

    def size_mutation_tests!(cases)
      %w[size-one-sample size-above-cutoff size-title-fallback].each do |name|
        test_case = cases.find { |item| item.fetch(:name) == name }
        spec = test_case.fetch(:size_case)
        session, evidence = size_test_evidence(test_case, "warm-rollback", "final")
        raw = Marshal.dump(evidence)
        evidence.fetch("frames").last.fetch("tables_after").each do |table, rows|
          changed = Marshal.load(raw)
          changed.fetch("frames").last.fetch("tables_after").fetch(table) << { "unexpected" => true }
          assert(!size_evidence?(spec, session, changed), "unexpected #{table} mutation rejected")
          rows.each_with_index do |row, index|
            row.each_key do |column|
              changed = Marshal.load(raw)
              changed.fetch("frames").last.fetch("tables_after").fetch(table).fetch(index)[column] = "altered"
              assert(!size_evidence?(spec, session, changed), "changed #{table}.#{column} rejected")
            end
          end
        end
        %w[tables_before tables_finish].each do |key|
          changed = Marshal.load(raw)
          changed.fetch("frames").first[key] = changed.fetch("frames").first.fetch("tables_after")
          assert(!size_evidence?(spec, session, changed), "partial rollback or nonempty start rejected")
        end
        changed = Marshal.load(raw)
        changed.fetch("frames").last["clock"] = "2026-02-30T00:00:00+00:00"
        assert(!size_evidence?(spec, session, changed), "impossible transaction clock rejected")
        changed = Marshal.load(raw)
        changed.fetch("frames").last.fetch("tables_after").fetch("canonical_torrent").first["canonical_torrent_id"] = 2.0
        assert(!size_evidence?(spec, session, changed), "floating-point identity cannot masquerade as integer")
        changed = Marshal.load(raw)
        changed.fetch("frames").last.fetch("tables_after").fetch("canonical_torrent_source_context_score").first["score_total_context"] = 0
        assert(!size_evidence?(spec, session, changed), "numeric score must retain its decimal JSON representation")
        changed = Marshal.load(raw)
        changed.fetch("inputs_after").fetch("search_request").first["effective_media_domain_id"] = 1
        assert(!size_evidence?(spec, session, changed), "read input mutation rejected")
        changed = Marshal.load(raw)
        read = changed.fetch("size_inputs_after")
        read.fetch("data").fetch("media_domain").first["media_domain_key"] = "ebooks"
        read["stdout"] = JSON.generate(read.fetch("data"))
        assert(!size_evidence?(spec, session, changed), "coherent domain substitution rejected")
        changed = Marshal.load(raw)
        read = changed.fetch("size_inputs_after")
        decoded = JSON.parse(read.fetch("stdout"))
        decoded.fetch("media_domain").first["media_domain_id"] = 1.0
        read["stdout"] = JSON.generate(decoded)
        assert(!size_evidence?(spec, session, changed), "raw and declared input numeric types must agree")
        changed = Marshal.load(raw)
        changed.fetch("size_inputs_after")["stderr"] = "unexpected"
        assert(!size_evidence?(spec, session, changed), "read diagnostics rejected")
        changed = Marshal.load(raw)
        read = changed.fetch("size_inputs_after")
        read["stdout"] = read.fetch("stdout").sub('{', '{"media_domain":[],')
        rejected("duplicate metadata JSON evidence") { size_evidence?(spec, session, changed) }
      end
    end
  end
end
