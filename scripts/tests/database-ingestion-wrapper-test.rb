# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionWrapperTest
    private

    def wrapper_tests!
      cases = wrapper_cases
      assert(cases.length == 8 && cases.map { |item| item.fetch(:name) }.uniq.length == 8, "retain distinct wrapper/scoring/page/size cases")
      assert(IngestionProof::INGESTION_HELPERS.include?("search_result_ingest"), "inventory must include the actual Rust wrapper")
      assert(IngestionWrapper::WRAPPER_MODES == %w[cold helpers-first warm-rollback], "retain independent cold and warm compilation modes")
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
      assert(cases.last.fetch(:fixtures).length == 25, "reach actual size-sample retention boundary")
      wrapper_result_tests!(cases)
      wrapper_input_tests!
      wrapper_scoring_tests!(cases)
      wrapper_binding_control!(cases)
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
        values = test_case.fetch(:samples) == 3 ? [100, 500, 900] : (2..26).map { |value| value * 100 }
        canonical = { "size_bytes" => values.fetch(values.length / 2) }
        tables = { "canonical_size_sample" => values.map { |value| { "size_bytes" => value } }, "canonical_size_rollup" => [{ "sample_count" => values.length, "size_median" => canonical.fetch("size_bytes"), "size_min" => values.min, "size_max" => values.max }] }
        assert(wrapper_samples?(test_case, tables, canonical), "exact retained sample set and median")
        tables.fetch("canonical_size_sample") << { "size_bytes" => 100 }
        assert(!wrapper_samples?(test_case, tables, canonical), "extra old sample must fail")
        tables.fetch("canonical_size_sample").pop
        canonical["size_bytes"] += 1
        assert(!wrapper_samples?(test_case, tables, canonical), "wrong canonical median must fail")
      end
    end
  end
end
