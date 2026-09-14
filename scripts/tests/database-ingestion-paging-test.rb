# frozen_string_literal: true

require_relative "database-ingestion-sampling-test"
require_relative "../database_rebaseline/ingestion_paging"

module RevaerDatabaseRebaseline
  class IngestionPagingTest < IngestionSamplingTest
    include IngestionPaging

    def run_tests!
      sampling_test_setup!
      assert(paging_cases.map { |spec| spec.fetch(:reuse) } == [11, 1], "both new and original page reuse")
      paging_cases.each do |spec|
        PAGING_MODES.each do |mode|
          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = paging_test_evidence(spec, mode, variant, role)
            assert(paging_validate!(evidence, spec, mode, variant, role), "synthetic #{spec.fetch(:name)} #{mode} #{variant}")
            query = correction_session(paging_session(paging_observations(spec).drop(9), mode))
            assert(query.scan("FROM public.search_result_ingest(").length == 3 && query.scan("COMMIT;").length == 3, "three actual wrapper calls across commits")
            assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no caller or temporary namespace repair")
            assert(mode != "helpers-first" || query.index("helpers:") < query.index("BEGIN;"), "helpers precede tested ingestion")
            paging_test_answers!(evidence, spec, variant)
          end
        end
      end
      rejected("unknown paging mode") { paging_session([[10, 0]], "repair") }
      paging_test_mutations!
      puts "database-ingestion-paging-test: #{@assertions} assertions passed"
    end

    private

    def attributes_source_hashes
      paging_source_hashes
    end

    def verify_ingestion_attributes!
      verify_ingestion_paging!
    ensure
      @attributes_evidence = @paging_evidence
    end

    def paging_test_read(tables, inputs, counters)
      sampling_test_read(tables, inputs, counters.fetch("canonical_size_sample")).merge(
        "sequences" => { "stdout" => JSON.generate(counters) + "\n", "stderr" => "" })
    end

    def paging_test_evidence(spec, mode, variant, role)
      observations = paging_observations(spec)
      before = attributes_empty
      successes = variant == "final" ? 12 : 10
      prepared = nil
      frames = observations.each_with_index.map do |observation, index|
        number = index + 1
        frame = { "backend" => index < 9 ? (100 + index).to_s : "200",
          "clock" => format("2026-09-12T00:00:%02d+00:00", number), "finished_setting" => "error",
          "before" => "error", "after" => "error", "state" => "00000", "within" => "true", "outside" => (variant == "reference").to_s,
          "role" => { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" } }
        if index < successes
          frame["result"] = { "canonical_torrent_public_id" => format("56900000-0000-4000-8000-%012d", number * 2),
            "canonical_torrent_source_public_id" => format("56900000-0000-4000-8000-%012d", number * 2 + 1) }
          frame["result"] = paging_result(before, frame, observation.first)
          after = paging_tables(before, frame, observation, number)
        else
          frame.delete("result")
          frame["state"] = "42P07"
          after = before
        end
        frame.merge!("tables_before" => before, "tables_after" => after, "tables_finish" => after)
        before = after
        prepared = after if index == 8
        frame
      end
      seed = "2026-09-12T00:00:00+00:00"
      inputs = metadata_read_tables(seed)
      { "seed_clock" => seed, "fixtures" => frames.first(9).map { |frame| sampling_test_transport([frame], "cold", variant) },
        "tested" => sampling_test_transport(frames.drop(9), mode, variant),
        "initial" => paging_test_read(attributes_empty, inputs, paging_counters([], 0)),
        "prepared" => paging_test_read(prepared, inputs, paging_counters(observations.first(9), 9)),
        "after" => paging_test_read(before, inputs, paging_counters(observations, successes)) }
    end

    def paging_test_answers!(evidence, spec, variant)
      frames = evidence.fetch("tested").fetch("frames")
      first = frames.first.fetch("tables_finish")
      assert(first.fetch("search_page").map { |row| row.values_at("page_number", "sealed_at") } == [[1, nil]], "tenth item fills, not seals, page one")
      assert(first.fetch("search_page_item").map { |row| row.fetch("position") } == (1..10).to_a, "first commit stores all ten positions")
      counters = metadata_read_parse(evidence.fetch("after").fetch("sequences"))
      if variant == "final"
        second, last = frames.drop(1).map { |frame| frame.fetch("tables_finish") }
        assert(second.fetch("search_page").map { |row| row.values_at("search_page_id", "page_number", "sealed_at") } == [[1, 1, frames.fetch(1).fetch("clock")], [2, 2, nil]], "post-commit full boundary seals first page and creates second")
        assert(second.fetch("search_page_item").last == { "search_page_item_id" => 11, "search_page_id" => 2, "search_request_canonical_id" => 11, "position" => 1 }, "eleventh item has independently specified placement")
        %w[search_request_canonical search_page search_page_item].each do |table|
          assert(last.fetch(table) == second.fetch(table), "reuse does not create or alter #{table}")
        end
        selected = (spec.fetch(:reuse) == 1 ? evidence.fetch("fixtures").first.fetch("frames").first : frames.fetch(1)).fetch("result")
        assert(frames.last.fetch("result") == selected.merge("canonical_changed" => false, "observation_created" => false, "durable_source_created" => false), "reuse targets independently selected earlier identity")
        assert(counters.values_at("canonical_torrent", "canonical_torrent_source", "search_page", "search_page_item", "search_request_canonical", "canonical_size_sample", "canonical_size_rollup", "canonical_torrent_source_context_score", "canonical_torrent_best_source_context") == [11, 11, 2, 11, 12, 12, 12, 12, 12], "upserts consume identities but duplicate paging does not")
      else
        assert(frames.map { |frame| frame.fetch("state") } == %w[00000 42P07 42P07], "frozen post-commit failures retained")
        assert(frames.drop(1).all? { |frame| frame.fetch("tables_finish") == first }, "D4 rolls back all table writes")
        allocated = spec.fetch(:reuse) == 11 ? 12 : 11
        assert(counters.values_at("canonical_torrent", "canonical_torrent_source", "search_page", "search_page_item", "search_request_canonical", "canonical_size_sample") == [allocated, allocated, 1, 10, 10, 10], "frozen pre-D4 allocations survive failed new-item attempts")
      end
      assert(evidence.fetch("after").fetch("data").fetch("tables").length == 18 && evidence.fetch("after").fetch("data").fetch("inputs").length == 19 && counters.length == 18, "complete write read and sequence inventories")
    end

    def paging_test_refresh!(evidence, mode, variant)
      evidence["fixtures"] = evidence.fetch("fixtures").map { |record| sampling_test_transport(record.fetch("frames"), "cold", variant) }
      evidence["tested"] = sampling_test_transport(evidence.fetch("tested").fetch("frames"), mode, variant)
    end

    def paging_test_mutations!
      spec = paging_cases.first
      original = paging_test_evidence(spec, "helpers-first", "final", @runtime)
      validate = ->(changed) { paging_validate!(changed, spec, "helpers-first", "final", @runtime) }
      changed = copy(original)
      changed.fetch("fixtures").pop
      rejected("fixture count") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("tested").fetch("frames").last["backend"] = "999"
      rejected("raw and declared") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("tested")["stdout"] = changed.fetch("tested").fetch("stdout").sub("role:{", 'role:{"session":"duplicate",')
      rejected("duplicate metadata JSON") { validate.call(changed) }
      %w[backend before after finished_setting clock within outside].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").last[key] = key == "clock" ? "2026-02-30T00:00:00+00:00" : "altered"
        paging_test_refresh!(changed, "helpers-first", "final")
        rejected("paging") { validate.call(changed) }
      end
      %w[session current superuser create_role bypass_rls].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").last.fetch("role")[key] = "altered"
        paging_test_refresh!(changed, "helpers-first", "final")
        rejected("backend role") { validate.call(changed) }
      end
      changed = copy(original)
      changed.fetch("fixtures").first.fetch("frames").first["backend"] = "200"
      paging_test_refresh!(changed, "helpers-first", "final")
      rejected("backend role") { validate.call(changed) }
      %w[initial prepared after].each do |stage|
        %w[tables inputs].each do |kind|
          original.fetch(stage).fetch("data").fetch(kind).each_key do |table|
            changed = copy(original)
            record = changed.fetch(stage)
            record.fetch("data").fetch(kind).fetch(table) << { "unexpected" => true }
            record["stdout"] = JSON.generate(record.fetch("data"))
            rejected("independent inputs") { validate.call(changed) }
          end
        end
        IngestionProof::INGESTION_TABLES.each do |table|
          changed = copy(original)
          record = changed.fetch(stage).fetch("sequences")
          values = metadata_read_parse(record).to_h
          values[table] = values.fetch(table).nil? ? 1 : values.fetch(table).to_f
          record["stdout"] = JSON.generate(values)
          rejected("exact sequence") { validate.call(changed) }
        end
      end
      paging_test_coherent_mutations!(original, spec)
      reference = paging_test_evidence(spec, "helpers-first", "reference", "postgres")
      %w[42P07 tmp_policy_rules createas.c:406].each do |token|
        changed = copy(reference)
        changed.fetch("tested")["stderr"] = changed.fetch("tested").fetch("stderr").gsub(token, "altered")
        rejected("exact D4 diagnostic") { paging_validate!(changed, spec, "helpers-first", "reference", "postgres") }
      end
    end

    def paging_test_coherent_mutations!(original, spec)
      mutations = [
        ->(tables) { tables.fetch("search_page").first["sealed_at"] = nil },
        ->(tables) { tables.fetch("search_page").last["page_number"] = 3 },
        ->(tables) { tables.fetch("search_page_item").last["position"] = 2 },
        ->(tables) { tables.fetch("search_page_item") << tables.fetch("search_page_item").last.merge("search_page_item_id" => 12) },
        ->(tables) { tables.fetch("search_request_canonical").last["search_request_canonical_id"] = 12 },
        ->(tables) { tables.fetch("canonical_torrent_best_source_context").last["canonical_torrent_source_id"] = 1 }
      ]
      mutations.each do |mutate|
        changed = copy(original)
        frame = changed.fetch("tested").fetch("frames").last
        tables = copy(frame.fetch("tables_finish"))
        mutate.call(tables)
        frame.merge!("tables_after" => tables, "tables_finish" => tables)
        changed["after"] = paging_test_read(tables, metadata_read_tables(changed.fetch("seed_clock")), paging_counters(paging_observations(spec), 12))
        paging_test_refresh!(changed, "helpers-first", "final")
        rejected("full committed transition") { paging_validate!(changed, spec, "helpers-first", "final", @runtime) }
      end
      changed = copy(original)
      frame = changed.fetch("tested").fetch("frames").last
      before = copy(changed.fetch("tested").fetch("frames").fetch(1).fetch("tables_finish"))
      frame["result"] = paging_result(before, frame, 1)
      tables = paging_tables(before, frame, [1, 2], 12)
      frame.merge!("tables_after" => tables, "tables_finish" => tables)
      changed["after"] = paging_test_read(tables, metadata_read_tables(changed.fetch("seed_clock")), paging_counters(paging_observations(spec), 12))
      paging_test_refresh!(changed, "helpers-first", "final")
      rejected("full committed transition") { paging_validate!(changed, spec, "helpers-first", "final", @runtime) }
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    proof = RevaerDatabaseRebaseline::IngestionPagingTest.new
    if ARGV.empty?
      proof.run_tests!
    elsif ARGV.length == 2 && ARGV.first == "--live"
      proof.run_live!(ARGV.fetch(1))
    else
      raise RevaerDatabaseRebaseline::Failure, "expected no arguments or --live exact-candidate-path"
    end
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-paging-test: #{error.message}"
    exit 1
  end
end
