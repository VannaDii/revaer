# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionWrapperTest
    private

    def wrapper_tests!
      cases = wrapper_cases
      assert(cases.length == 73 && cases.map { |item| item.fetch(:name) }.uniq.length == 73, "retain distinct wrapper/scoring/page/size/identity/hash-fill/promotion cases")
      assert(IngestionProof::INGESTION_HELPERS.include?("search_result_ingest"), "inventory must include the actual Rust wrapper")
      assert(IngestionWrapper::WRAPPER_MODES == %w[cold helpers-first warm-rollback], "retain independent cold and warm compilation modes")
      guard_path = "scripts/stack_asset_exception.rb"
      assert(wrapper_source_hashes.fetch(guard_path) == Digest::SHA256.file(File.join(@contract.root, guard_path)).hexdigest,
        "retain the loaded changed-line guard in proof source provenance")
      cases.each do |test_case|
        calls = [test_case.fetch(:arguments)] * 2
        query = correction_session(name: "wrapper-test", calls:, wrapper: test_case.fetch(:wrapper), rollback: true)
        routine = test_case.fetch(:wrapper) ? "search_result_ingest" : "search_result_ingest_v1"
        assert(query.scan("public.#{routine}(").length == 2, "both real calls must use the selected entry point")
        assert(query.scan(/^ROLLBACK;$/).length == 1 && query.scan(/^COMMIT;$/).length == 1, "warm rollback and later commit must remain explicit")
        assert(!query.match?(/DROP TABLE|DISCARD|SET ROLE|SET plpgsql|\\connect/), "never repair the frozen backend or change authority")
        assert(query.scan("tables_finish:").length == 2, "retain both transaction outcomes")
      end
      assert(cases.fetch(4).fetch(:fixtures).length == 10, "fill the real minimum-size page")
      assert(cases.find { |item| item.fetch(:name) == "trim-size-samples" }.fetch(:fixtures).length == 25, "reach actual size-sample retention boundary")
      wrapper_result_tests!(cases)
      wrapper_input_tests!
      wrapper_scoring_tests!(cases)
      wrapper_binding_control!(cases)
      wrapper_promotion_tests!(cases)
      wrapper_paging_tests!(cases)
      wrapper_sample_tests!(cases)
    end

    def wrapper_test_frame
      frame = correction_test_frames.first
      frame.fetch("tables_after")["canonical_torrent_best_source_context"] = [{ "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 2 }]
      frame
    end

    def wrapper_result_tests!(cases)
      frame = wrapper_test_frame
      assert(wrapper_outcome?(cases.first, frame), "first wrapper call must persist its admitted best source")
      %w[observation_created durable_source_created canonical_changed].each do |key|
        changed = Marshal.load(Marshal.dump(frame))
        changed.fetch("result")[key] = false
        assert(!wrapper_outcome?(cases.first, changed), "incorrect #{key} must fail")
      end
      %w[canonical_torrent_public_id canonical_torrent_source_public_id].each do |key|
        changed = Marshal.load(Marshal.dump(frame))
        changed.fetch("result")[key] = "unknown"
        assert(!wrapper_outcome?(cases.first, changed), "unmatched returned #{key} must fail")
      end
      changed = Marshal.load(Marshal.dump(frame))
      changed.fetch("result")["unexpected"] = true
      assert(!wrapper_outcome?(cases.first, changed), "extra result field must fail")
      changed = Marshal.load(Marshal.dump(frame))
      changed.fetch("tables_after").fetch("canonical_torrent_best_source_context").first["canonical_torrent_source_id"] = 9
      assert(!wrapper_outcome?(cases.first, changed), "wrapper must select the actual admitted source")
      baseline = compilation_comparable("fixture" => nil, "frames" => [frame])
      IngestionProof::INGESTION_TABLES.each do |table|
        changed = Marshal.load(Marshal.dump(frame))
        changed.fetch("tables_after").fetch(table) << { "unexpected_write" => true }
        if %w[canonical_torrent canonical_torrent_source].include?(table)
          changed.fetch("tables_after").fetch(table).last.merge!("#{table}_id" => 9, "#{table}_public_id" => "56900000-0000-4000-8000-000000000099")
        end
        assert(baseline != compilation_comparable("fixture" => nil, "frames" => [changed]), "complete #{table} image must affect comparison")
      end
    end

    def wrapper_input_tests!
      clock = "2026-09-10T00:00:00+00:00"
      inputs = IngestionWrapper::WRAPPER_INPUTS.to_h { |table| [table, [{ "fixture_text" => clock }]] }
      IngestionWrapper::WRAPPER_INPUT_CLOCKS.each { |table, columns| columns.each { |column| inputs.fetch(table).first[column] = clock } }
      evidence = { "inputs_before" => inputs, "seed_clock" => clock }
      raw = Marshal.dump(evidence)
      compared = wrapper_comparable_inputs(evidence)
      assert(Marshal.dump(evidence) == raw, "input normalization must preserve raw evidence")
      compared.each do |table, rows|
        assert(rows.first.fetch("fixture_text") == clock, "never normalize timestamp-shaped input text")
        IngestionWrapper::WRAPPER_INPUT_CLOCKS.fetch(table, []).each do |column|
          assert(rows.first.fetch(column) == "<validated-seed-transaction>", "normalize only observed #{table}.#{column}")
          changed = Marshal.load(raw)
          changed.fetch("inputs_before").fetch(table).first[column] = "2026-09-10T00:01:00+00:00"
          assert(wrapper_comparable_inputs(changed) != compared, "unobserved #{table}.#{column} must remain visible")
        end
      end
    end

    def wrapper_scoring_tables
      frame = wrapper_test_frame
      tables = frame.fetch("tables_after")
      tables.fetch("canonical_torrent").first["title_display"] = "Higher ranked title"
      low = tables.fetch("canonical_torrent_source").first.merge("source_guid" => "low")
      high = low.merge("canonical_torrent_source_id" => 3, "source_guid" => "high")
      tables["canonical_torrent_source"] = [low, high]
      tables["canonical_torrent_source_context_score"] = [
        { "canonical_torrent_source_id" => 2, "score_total_context" => 10 },
        { "canonical_torrent_source_id" => 3, "score_total_context" => 100 }
      ]
      [tables, low]
    end

    def wrapper_scoring_tests!(cases)
      cases.select { |item| item[:scoring] }.each do |test_case|
        tables, low = wrapper_scoring_tables
        canonical = tables.fetch("canonical_torrent").first
        tables.fetch("canonical_torrent_best_source_context").first["canonical_torrent_source_id"] = test_case.fetch(:best) == "high" ? 3 : 2
        assert(wrapper_scoring?(test_case, tables, canonical, low), "exact #{test_case.fetch(:name)} selected source")
        tables.fetch("canonical_torrent_source_context_score").last["score_total_context"] = 10
        assert(!wrapper_scoring?(test_case, tables, canonical, low), "stored score must discriminate against the local low score")
        tables.fetch("canonical_torrent_source_context_score").last["score_total_context"] = 100
        canonical["title_display"] = "Low refreshed"
        assert(!wrapper_scoring?(test_case, tables, canonical, low), "display title must use the higher-ranked stored observation")
      end
    end

    def wrapper_binding_control!(cases)
      test_case = cases.find { |item| item.fetch(:name) == "stored-score-seeder-promotion" }
      assert(test_case.fetch(:fixtures).first.fetch(:source_guid_input) == "'low'::varchar", "higher stored score must lose an incorrect ID-only tie-break")
      tables, low = wrapper_scoring_tables
      canonical = tables.fetch("canonical_torrent").first
      low["last_seen_seeders"] = 100
      tables.fetch("canonical_torrent_source").last["last_seen_seeders"] = 40
      assert(wrapper_scoring?(test_case, tables, canonical, low), "stored-column ordering promotes the new hundred-seeder source")
      local_score = 10
      wrong_choice = tables.fetch("canonical_torrent_source_context_score").min_by { |row| [-local_score, row.fetch("canonical_torrent_source_id")] }
      wrong_source = tables.fetch("canonical_torrent_source").find { |row| row.fetch("canonical_torrent_source_id") == wrong_choice.fetch("canonical_torrent_source_id") }
      promotes = local_score >= local_score + 2 || (20...100).cover?(wrong_source.fetch("last_seen_seeders"))
      assert(!promotes && wrong_source == low, "incorrect local-variable ordering selects the current low-score source and cannot promote it")
      tables.fetch("canonical_torrent_best_source_context").first["canonical_torrent_source_id"] = 3
      assert(!wrapper_scoring?(test_case, tables, canonical, low), "retained previous best from local-variable ordering must fail actual proof")
    end

    def wrapper_promotion_tests!(cases)
      expected = {
        "promotion-incoming-null" => [nil, 40, "high"], "promotion-best-null" => [100, nil, "high"],
        "promotion-best-19" => [100, 19, "high"], "promotion-best-20" => [100, 20, "low"],
        "promotion-best-99" => [100, 99, "low"], "promotion-best-100" => [100, 100, "high"]
      }
      boundaries = cases.select { |test_case| test_case[:promotion] }
      assert(boundaries.map { |test_case| test_case.fetch(:name) } == expected.keys, "six distinct promotion predicate arms and edge controls")
      schema = File.read(File.join(@contract.root, "crates/revaer-data/migrations/0022_indexer_canonicalization.sql"))
      assert(schema.include?("last_seen_seeders INTEGER,") && schema.include?("last_seen_seeders IS NULL OR last_seen_seeders >= 0"), "both NULL seeder arms are constraint-reachable without DDL changes")
      boundaries.each do |test_case|
        incoming, best_seeders, best = expected.fetch(test_case.fetch(:name))
        assert(test_case.fetch(:promotion).values_at(:incoming, :best_seeders) + [test_case.fetch(:best)] == [incoming, best_seeders, best], "literal independent promotion expectation")
        assert(test_case.fetch(:arguments).fetch(:seeders_input) == (incoming.nil? ? "NULL::integer" : incoming.to_s), "real typed incoming NULL or hundred-seeder argument")
        assert(test_case.fetch(:fixtures).drop(1).all? { |args| args.fetch(:seeders_input) == (best_seeders.nil? ? "NULL::integer" : best_seeders.to_s) }, "distinct best source retains exact boundary seeders through fixture refresh")
        assert(!test_case.fetch(:wrapper) && test_case.fetch(:fixtures).first.fetch(:source_guid_input) == "'low'::varchar", "direct v1 predicate is not masked by wrapper admission or ID-only ordering")
        IngestionWrapper::WRAPPER_MODES.each do |mode|
          session = { rollback: mode == "warm-rollback" }
          evidence = wrapper_promotion_test_evidence(test_case, session)
          assert(wrapper_promotion_evidence?(test_case, session, evidence), "complete promotion fixture/call/rollback/read state #{mode}")
          scores = evidence.fetch("inputs_before").fetch("canonical_torrent_source_base_score") +
            evidence.fetch("after").fetch("canonical_torrent_source_context_score")
          assert(scores.all? { |row| row.select { |key, _value| key.start_with?("score_") }.values.all? { |value| value.is_a?(Float) } }, "literal NUMERIC scores retain the actual PostgreSQL JSON number shape")
          baseline = evidence.fetch("before")
          %w[canonical_torrent canonical_torrent_source search_request_source_observation].each do |table|
            assert(baseline.fetch(table).all? { |row| row.fetch("magnet_hash") == "750837cbeaadaf72ab0a852ee3e0517760ca56eb6b8fc3c08b2d9e73348b6bea" },
              "valid v1 still derives a magnet hash without a magnet URI")
          end
          assert(baseline.fetch("canonical_torrent_source").map { |row| row.values_at("source_guid", "last_seen_seeders") } == [["low", 5], ["high", best_seeders]], "independent distinct durable seeder inputs")
          assert(baseline.fetch("canonical_torrent_best_source_context").first.fetch("canonical_torrent_source_id") == 2 &&
            baseline.fetch("canonical_torrent_source_context_score").map { |row| row.fetch("score_total_context") } == [0, 100], "selected high source and stored score remain distinct before incoming refresh")
          evidence.fetch("frames").each do |frame|
            after = frame.fetch("tables_after")
            assert(after.fetch("canonical_torrent_source_context_score").map { |row| row.fetch("score_total_context") } == [10, 100], "promotion is discriminated by stored scores, not incoming rank")
            assert(after.fetch("canonical_torrent_best_source_context").first.fetch("canonical_torrent_source_id") == (best == "high" ? 2 : 1), "literal selected-source outcome")
          end
          wrapper_promotion_mutations!(test_case, session, evidence)
        end
      end
    end

    def wrapper_promotion_test_evidence(test_case, session)
      canonical = "56900000-0000-4000-8000-000000000090"
      low = "56900000-0000-4000-8000-000000000091"
      high = "56900000-0000-4000-8000-000000000092"
      identities = [low, high, high].each_with_index.map do |source, index|
        { "canonical_torrent_public_id" => canonical, "canonical_torrent_source_public_id" => source,
          "observation_created" => index < 2, "durable_source_created" => index < 2, "canonical_changed" => index.zero? }
      end
      clocks = %w[2026-09-10T01:00:00+00:00 2026-09-10T01:01:00+00:00 2026-09-10T01:02:00+00:00]
      snapshots = wrapper_promotion_fixtures(test_case.fetch(:promotion), clocks, identities)
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      fixtures = snapshots.each_with_index.map do |tables, index|
        { "clock" => clocks.fetch(index), "result" => identities.fetch(index), "tables_before" => index.zero? ? empty : snapshots.fetch(index - 1),
          "tables_after" => tables, "tables_finish" => tables }
      end
      baseline = snapshots.last
      frames = Array.new(session[:rollback] ? 2 : 1) do |index|
        clock = "2026-09-10T02:0#{index}:00+00:00"
        after = wrapper_promotion_after(test_case, baseline, clock)
        { "clock" => clock, "result" => identities.first.merge("observation_created" => false, "durable_source_created" => false, "canonical_changed" => false),
          "tables_before" => baseline, "tables_after" => after, "tables_finish" => session[:rollback] && index.zero? ? baseline : after }
      end
      seed_clock = "2026-09-10T00:00:00+00:00"
      inputs = wrapper_promotion_inputs(seed_clock)
      # Separate all images so a single-location mutation cannot silently alter
      # both sides of a continuity assertion through shared Ruby references.
      JSON.parse(JSON.generate("seed_clock" => seed_clock, "fixtures" => fixtures, "frames" => frames, "before" => baseline,
        "after" => frames.last.fetch("tables_after"), "inputs_before" => inputs, "inputs_after" => inputs))
    end

    def wrapper_promotion_mutations!(test_case, session, evidence)
      raw = JSON.generate(evidence)
      %w[fixtures frames].each do |phase|
        evidence.fetch(phase).each_with_index do |frame, index|
          frame.fetch("result").each_key do |column|
            changed = JSON.parse(raw)
            changed.fetch(phase).fetch(index).fetch("result")[column] = "changed"
            assert(!wrapper_promotion_evidence?(test_case, session, changed), "reject #{phase} #{index} result #{column}")
          end
          %w[tables_before tables_after tables_finish].each do |key|
            frame.fetch(key).each do |table, rows|
              changed = JSON.parse(raw)
              changed.fetch(phase).fetch(index).fetch(key).fetch(table) << { "unexpected" => true }
              assert(!wrapper_promotion_evidence?(test_case, session, changed), "reject #{phase} #{index} #{key} extra #{table}")
              rows.each_with_index do |row, row_index|
                row.each_key do |column|
                  changed = JSON.parse(raw)
                  changed.fetch(phase).fetch(index).fetch(key).fetch(table).fetch(row_index)[column] = "changed"
                  assert(!wrapper_promotion_evidence?(test_case, session, changed), "reject #{phase} #{index} #{key} #{table}.#{column}")
                end
              end
            end
          end
        end
      end
      evidence.fetch("inputs_before").each do |table, rows|
        rows.each_with_index do |row, index|
          row.each_key do |column|
            changed = JSON.parse(raw)
            %w[inputs_before inputs_after].each { |key| changed.fetch(key).fetch(table).fetch(index)[column] = "changed" }
            assert(!wrapper_promotion_evidence?(test_case, session, changed), "reject coherently mutated read input #{table}.#{column}")
          end
        end
      end
      changed = JSON.parse(raw)
      changed.fetch("before").fetch("canonical_torrent_best_source_context").first["canonical_torrent_source_id"] = 1
      assert(!wrapper_promotion_evidence?(test_case, session, changed), "selected source must be distinct before promotion")
      changed = JSON.parse(raw)
      changed.fetch("after").fetch("canonical_torrent_best_source_context").first["canonical_torrent_source_id"] = test_case.fetch(:best) == "high" ? 1 : 2
      assert(!wrapper_promotion_evidence?(test_case, session, changed), "reject opposite persisted selection")
      assert(JSON.generate(evidence) == raw, "promotion verification retains raw evidence")
    end

    def wrapper_paging_tests!(cases)
      frame = wrapper_test_frame
      tables = frame.fetch("tables_after")
      tables["search_page"] = [{ "search_page_id" => 5, "page_number" => 1, "sealed_at" => frame.fetch("clock") }, { "search_page_id" => 9, "page_number" => 2, "sealed_at" => nil }]
      tables["search_page_item"] = (1..10).map { |position| { "search_page_id" => 5, "position" => position } } + [{ "search_page_id" => 9, "position" => 1 }]
      tables["search_request_canonical"] = Array.new(11) { {} }
      tables["canonical_torrent_best_source_context"] = Array.new(11) { {} }
      test_case = cases.find { |item| item[:new_page] }
      assert(wrapper_paging?(test_case, frame), "full page seals and next distinct canonical starts at position one")
      tables.fetch("search_page_item").last["position"] = 2
      assert(!wrapper_paging?(test_case, frame), "new-page offset error must fail")
      tables.fetch("search_page_item").last["position"] = 1
      tables.fetch("search_page").first["sealed_at"] = nil
      assert(!wrapper_paging?(test_case, frame), "missing page seal must fail")
    end

    def wrapper_sample_tests!(cases)
      cases.select { |item| item[:samples] }.each do |test_case|
        values, median, displayed = case test_case.fetch(:name)
                                    when "second-size-sample" then [[100, 900], 500, 100]
                                    when "third-size-sample" then [[100, 500, 900], 500, 500]
                                    when "trim-size-samples" then [(2..26).map { |value| value * 100 }, 1400, 1400]
                                    else raise Failure, "unknown sample test case"
                                    end
        canonical = { "size_bytes" => displayed }
        tables = { "canonical_size_sample" => values.map { |value| { "size_bytes" => value } }, "canonical_size_rollup" => [{ "sample_count" => values.length, "size_median" => median, "size_min" => values.min, "size_max" => values.max }] }
        assert(wrapper_samples?(test_case, tables, canonical), "exact retained sample set and median")
        tables.fetch("canonical_size_sample") << { "size_bytes" => 100 }
        assert(!wrapper_samples?(test_case, tables, canonical), "extra old sample must fail")
        tables.fetch("canonical_size_sample").pop
        canonical["size_bytes"] += 1
        assert(!wrapper_samples?(test_case, tables, canonical), "wrong canonical display size must fail")
        canonical["size_bytes"] = displayed
        %w[sample_count size_median size_min size_max].each do |column|
          altered = Marshal.load(Marshal.dump(tables))
          altered.fetch("canonical_size_rollup").first[column] += 1
          assert(!wrapper_samples?(test_case, altered, canonical), "wrong rollup #{column} must fail")
        end
        if test_case.fetch(:samples) == 2
          assert(test_case.fetch(:fixtures).first.fetch(:size_bytes_input) == "100::bigint" &&
            test_case.fetch(:arguments).fetch(:size_bytes_input) == "900::bigint", "two real samples discriminate first value from median")
          assert(!wrapper_samples?(test_case, tables, { "size_bytes" => 500 }), "two-sample median must not replace the first display size")
        end
      end
      assert(!wrapper_samples?({ samples: 4 }, {}, {}), "unknown sample model must fail closed")
    end
  end
end
