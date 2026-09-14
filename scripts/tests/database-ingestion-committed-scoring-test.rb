# frozen_string_literal: true

require_relative "database-ingestion-sampling-test"
require_relative "../database_rebaseline/ingestion_committed_scoring"

module RevaerDatabaseRebaseline
  class IngestionCommittedScoringTest < IngestionSamplingTest
    include IngestionCommittedScoring

    def run_tests!
      sampling_test_setup!
      assert(committed_scoring_cases.length == 2, "exactly two discriminating seeder cases")
      assert(IngestionProof::INGESTION_TABLES.length == 18 && IngestionPolicy::POLICY_READ_TABLES.length == 19, "complete write and read inventories")
      committed_scoring_source_tests!
      committed_scoring_cases.each do |spec|
        COMMITTED_SCORING_MODES.each do |mode|
          query = correction_session(committed_scoring_session(spec, mode))
          assert(query.scan("FROM public.search_result_ingest(").length == 4 && query.scan(/^COMMIT;$/).length == 4, "four actual public wrapper commits")
          assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect|^ROLLBACK;$/), "no reconnect, rollback warmup or settings repair")
          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = committed_scoring_test_evidence(spec, mode, variant, role)
            assert(committed_scoring_validate!(evidence, spec, mode, variant, role), "independent #{spec.fetch(:name)} #{mode} #{variant}")
            expected = variant == "reference" ? %w[00000 42P07 42P07 42P07] : Array.new(4, "00000")
            assert(evidence.fetch("tested").fetch("frames").map { |frame| frame.fetch("state") } == expected, "frozen error-to-success difference retained")
          end
        end
        committed_scoring_mutations!(spec)
      end
      rejected("unknown committed scoring mode") { committed_scoring_session(committed_scoring_cases.first, "reconnect") }
      puts "database-ingestion-committed-scoring-test: #{@assertions} assertions passed"
    end

    private

    def attributes_source_hashes
      committed_scoring_source_hashes
    end

    def verify_ingestion_attributes!
      verify_ingestion_committed_scoring!
    ensure
      @attributes_evidence = @committed_scoring_evidence
    end

    def committed_scoring_source_tests!
      needles = { "stored-score" => "SELECT score_total_base", "score-upsert" => "score_total_context = EXCLUDED.score_total_context",
        "ranked-source" => "ORDER BY score_total_context DESC, canonical_torrent_source_id ASC",
        "seeder-promotion" => "AND seeders_input >= 100 THEN", "title-selection" => "COALESCE(bs.score_total_base, 0) DESC",
        "public-wrapper-tail" => "canonical_torrent_source_id = EXCLUDED.canonical_torrent_source_id" }
      committed_scoring_sources.each do |name, mapping|
        assert(mapping.fetch(:source).include?(needles.fetch(name)), "exact frozen branch coordinates #{name}")
      end
      spec = committed_scoring_cases.first
      args = committed_scoring_session(spec, "cold").fetch(:calls)
      assert(args.values_at(1, 3) == [spec.fetch(:arguments)] * 2, "last call reuses the same low-source observation")
      assert(args.map { |call| call.fetch(:source_guid_input) } == %w[high low high low].map { |guid| "'#{guid}'::varchar" }, "distinct mapped source order")
      assert(committed_scoring_cases.map { |item| item.fetch(:promotion) } == [{ incoming: 99, best_seeders: 40 }, { incoming: 100, best_seeders: 40 }], "literal promotion boundary and ranked source")
    end

    def committed_scoring_test_read(tables, inputs, sequences)
      data = { "tables" => copy(tables), "inputs" => copy(inputs), "sequences" => copy(sequences) }
      { "stdout" => JSON.generate(data) + "\n", "stderr" => "", "data" => data }
    end

    def committed_scoring_test_frame(role, number, before, after)
      { "backend" => "200", "clock" => "2026-09-12T00:00:0#{number}+00:00", "before" => "error", "after" => "error",
        "role" => { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" },
        "state" => "00000", "within" => "true", "outside" => (role == "postgres").to_s, "finished_setting" => "error",
        "tables_before" => before, "tables_after" => after, "tables_finish" => after }
    end

    def committed_scoring_test_evidence(spec, mode, variant, role)
      canonical = "56900000-0000-4000-8000-000000000090"
      low = "56900000-0000-4000-8000-000000000091"
      high = "56900000-0000-4000-8000-000000000092"
      identities = [low, high, high].map { |source| { "canonical_torrent_public_id" => canonical, "canonical_torrent_source_public_id" => source } }
      clocks = (1..7).map { |number| "2026-09-12T00:00:0#{number}+00:00" }
      snapshots = wrapper_promotion_fixtures(spec.fetch(:promotion), clocks.first(3), identities)
      seed = "2026-09-12T00:00:00+00:00"
      inputs = metadata_read_tables(seed)
      scored = inputs.merge("canonical_torrent_source_base_score" => wrapper_promotion_inputs(seed).fetch("canonical_torrent_source_base_score"))
      evidence = { "seed_clock" => seed, "answers" => committed_scoring_answers(spec),
        "initial" => committed_scoring_test_read(attributes_empty, inputs, committed_scoring_sequences(0)), "fixtures" => [] }
      before = attributes_empty
      snapshots.each_with_index do |tables, index|
        frame = committed_scoring_test_frame(role, index + 1, before, tables).merge(
          "backend" => (100 + index).to_s,
          "result" => identities.fetch(index).merge("canonical_changed" => index.zero?, "durable_source_created" => index < 2, "observation_created" => index < 2))
        evidence.fetch("fixtures") << sampling_test_transport([frame], "cold", variant).merge(
          "read" => committed_scoring_test_read(tables, index < 2 ? inputs : scored, committed_scoring_sequences(index + 1, scores: index == 2)))
        before = tables
      end
      frames = (0...4).map do |index|
        success = variant == "final" || index.zero?
        after = success ? committed_scoring_tables(spec, before, clocks.fetch(index + 3), index) : before
        frame = committed_scoring_test_frame(role, index + 4, before, after)
        if success
          frame["result"] = { "canonical_torrent_public_id" => canonical, "canonical_torrent_source_public_id" => [1, 3].include?(index) ? low : high,
            "canonical_changed" => false, "durable_source_created" => false, "observation_created" => false }
        else
          frame.delete("result")
          frame["state"] = "42P07"
        end
        before = after
        frame
      end
      evidence["tested"] = sampling_test_transport(frames, mode, variant)
      promotions = variant == "final" && spec.fetch(:promotion).fetch(:incoming) == 100 ? 2 : 0
      evidence["after"] = committed_scoring_test_read(before, scored, committed_scoring_sequences(variant == "final" ? 7 : 4, scores: true, promotions:))
      evidence
    end

    def committed_scoring_refresh_test!(evidence, mode, variant)
      evidence["tested"] = sampling_test_transport(evidence.fetch("tested").fetch("frames"), mode, variant)
      evidence.fetch("after")["stdout"] = JSON.generate(evidence.fetch("after").fetch("data")) + "\n"
    end

    def committed_scoring_mutations!(spec)
      original = committed_scoring_test_evidence(spec, "helpers-first", "final", @runtime)
      validate = ->(changed) { committed_scoring_validate!(changed, spec, "helpers-first", "final", @runtime) }
      frames = original.fetch("tested").fetch("frames")
      answers = committed_scoring_answers(spec)
      assert(frames.map { |frame| frame.fetch("tables_after").fetch("canonical_torrent").first.fetch("title_display") } == answers.fetch("title_display"), "positive post-commit title change and score-ranked title retention")
      assert(frames.map { |frame| frame.fetch("tables_after").fetch("canonical_torrent_best_source_context").first.fetch("canonical_torrent_source_id") } == [2, 1, 2, 1], "wrapper changes best context despite lower score")
      assert(frames.fetch(1).fetch("tables_after").fetch("canonical_torrent_source_context_score").map { |row| row.fetch("score_total_context") } == [10.0, 100.0], "incoming and ranked scores stay distinct")
      expected_best_sequence = spec.fetch(:promotion).fetch(:incoming) == 100 ? 9 : 7
      assert(original.fetch("after").fetch("data").fetch("sequences").fetch("canonical_torrent_best_source_context") == expected_best_sequence, "two extra promotion identity allocations at 100 only")

      %w[canonical_torrent canonical_torrent_source search_request_source_observation canonical_torrent_source_context_score canonical_torrent_best_source_context].each do |table|
        frames.last.fetch("tables_after").fetch(table).each_with_index do |row, index|
          row.each_key do |column|
            changed = copy(original)
            last = changed.fetch("tested").fetch("frames").last
            tables = copy(last.fetch("tables_after"))
            tables.fetch(table).fetch(index)[column] = "altered"
            last.merge!("tables_after" => tables, "tables_finish" => tables)
            changed.fetch("after").fetch("data")["tables"] = tables
            committed_scoring_refresh_test!(changed, "helpers-first", "final")
            rejected("full transition") { validate.call(changed) }
          end
        end
      end
      %w[tables inputs].each do |section|
        original.fetch("after").fetch("data").fetch(section).each do |table, rows|
          mutations = [->(items) { items << { "unexpected" => true } }]
          rows.each_with_index do |row, index|
            row.each_key { |column| mutations << ->(items) { items.fetch(index)[column] = "altered" } }
          end
          mutations.each do |mutate|
            changed = copy(original)
            mutate.call(changed.fetch("after").fetch("data").fetch(section).fetch(table))
            committed_scoring_refresh_test!(changed, "helpers-first", "final")
            rejected("independent final") { validate.call(changed) }
          end
        end
      end
      COMMITTED_SCORING_SEQUENCES.each_key do |table|
        [nil, 0, 7.0].each do |value|
          changed = copy(original)
          changed.fetch("after").fetch("data").fetch("sequences")[table] = value
          committed_scoring_refresh_test!(changed, "helpers-first", "final")
          rejected("independent final") { validate.call(changed) }
        end
      end
      %w[backend before after finished_setting clock within outside].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").last[key] = key == "backend" ? "201" : "altered"
        committed_scoring_refresh_test!(changed, "helpers-first", "final")
        rejected("committed scoring") { validate.call(changed) }
      end
      %w[session current superuser create_role bypass_rls].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").last.fetch("role")[key] = "altered"
        committed_scoring_refresh_test!(changed, "helpers-first", "final")
        rejected("backend role") { validate.call(changed) }
      end
      changed = copy(original)
      changed.fetch("tested").fetch("frames").last.fetch("result")["canonical_torrent_source_public_id"] = original.fetch("fixtures").fetch(1).fetch("frames").first.fetch("result").fetch("canonical_torrent_source_public_id")
      committed_scoring_refresh_test!(changed, "helpers-first", "final")
      rejected("full transition") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("tested").fetch("frames").last["backend"] = "201"
      rejected("raw and declared") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("tested")["stdout"] = changed.fetch("tested").fetch("stdout").sub('role:{', 'role:{"session":"duplicate",')
      rejected("duplicate metadata JSON") { validate.call(changed) }
      reference = committed_scoring_test_evidence(spec, "helpers-first", "reference", "postgres")
      %w[42P07 tmp_policy_rules createas.c:406].each do |token|
        changed = copy(reference)
        changed.fetch("tested")["stderr"] = changed.fetch("tested").fetch("stderr").gsub(token, "altered")
        rejected("exact D4 diagnostic") { committed_scoring_validate!(changed, spec, "helpers-first", "reference", "postgres") }
      end
      changed = copy(original)
      changed.fetch("tested")["stderr"] = sampling_diagnostic
      rejected("required outcomes") { validate.call(changed) }
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    proof = RevaerDatabaseRebaseline::IngestionCommittedScoringTest.new
    if ARGV.empty?
      proof.run_tests!
    elsif ARGV.length == 2 && ARGV.first == "--live"
      proof.run_live!(ARGV.fetch(1))
    else
      raise RevaerDatabaseRebaseline::Failure, "expected no arguments or --live exact-candidate-path"
    end
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-committed-scoring-test: #{error.message}"
    exit 1
  end
end
