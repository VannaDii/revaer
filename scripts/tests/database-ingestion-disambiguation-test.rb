# frozen_string_literal: true

require_relative "database-ingestion-attributes-test"

module RevaerDatabaseRebaseline
  class IngestionDisambiguationTest < IngestionAttributesTest
    def run_tests!
      @assertions = 0
      @runtime = "disambiguation_unit_runtime"
      routines = { "search_result_ingest_v1" => "0052_indexer_search_result_ingest_proc.sql",
        "search_result_ingest" => "0120_search_result_ingest_seed_best_source_context.sql" }.map do |name, path|
        source = File.binread(File.join(@contract.root, "crates/revaer-data/migrations", path))
        body = source.split("CREATE OR REPLACE FUNCTION #{name}(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
        { "name" => name, "signature" => "#{name}(uuid)", "source" => body }
      end
      @ingestion_inventory = { "reference_proof" => { "routines" => routines } }
      assert(disambiguation_cases.length == 6, "three identity strategies and two orientations")
      disambiguation_cases.each do |spec|
        DISAMBIGUATION_MODES.each do |mode|
          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = disambiguation_test_evidence(spec, mode, variant, role)
            assert(disambiguation_validate!(evidence, spec, mode, variant, role), "synthetic #{spec.fetch(:name)} #{mode} #{variant}")
            query = correction_session(disambiguation_session(spec, mode))
            assert(query.scan("FROM public.search_result_ingest(").length == 2 && query.scan("COMMIT;").length == 2, "real wrapper calls with committed warm retry")
            assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no compiler or temporary namespace repair")
            assert(evidence.fetch("tested").fetch("stderr").scan("ERROR:  23505:").length == 2, "errors remain errors")
          end
        end
      end
      disambiguation_mutations!
      puts "database-ingestion-disambiguation-test: #{@assertions} assertions passed"
    end

    private

    def attributes_source_hashes
      wrapper_source_hashes.merge("scripts/tests/database-ingestion-disambiguation-test.rb" => Digest::SHA256.file(__FILE__).hexdigest)
    end

    def verify_ingestion_attributes!
      verify_ingestion_disambiguation!
    ensure
      @attributes_evidence = @disambiguation_evidence
    end

    def disambiguation_test_transport(frames, mode, spec, role, fixture: false)
      lines = []
      lines << "helpers:#{JSON.generate(ingestion_helper_expectations)}" if mode == "helpers-first" && !fixture
      frames.each do |frame|
        correction_records(finish_setting: true).each do |key|
          lines << JSON.generate(frame.fetch("result")) if key == "state" && frame.key?("result")
          value = frame.fetch(key)
          lines << "#{key}:#{%w[role clock tables_before tables_after tables_finish].include?(key) ? JSON.generate(value) : value}"
        end
      end
      { "stdout" => lines.join("\n") + "\n", "stderr" => fixture ? "" : disambiguation_diagnostic(spec, role) * frames.length, "frames" => frames }
    end

    def disambiguation_test_read(tables, inputs, sequence)
      data = { "tables" => tables, "inputs" => inputs, "canonical_sequence" => sequence }
      { "stdout" => JSON.generate(data) + "\n", "stderr" => "", "data" => data }
    end

    def disambiguation_test_evidence(spec, mode, variant, role)
      first = { "clock" => "2026-09-13T00:00:01+00:00", "result" => {
        "canonical_torrent_public_id" => "56900000-0000-4000-8000-000000000090",
        "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-000000000091" } }
      tables = disambiguation_fixture_tables(spec, first)
      fixture = attributes_test_frame(role, 1, attributes_empty, tables, first: true).merge("finished_setting" => "error")
      frames = [3, 4].map do |number|
        frame = attributes_test_frame(role, number, tables, tables).merge("finished_setting" => "error", "state" => "23505", "within" => "false", "outside" => "false")
        frame.delete("result")
        frame
      end
      seed = "2026-09-13T00:00:00+00:00"
      rule_clock = "2026-09-13T00:00:02+00:00"
      inputs = metadata_read_tables(seed)
      with_rule = copy(inputs)
      with_rule.fetch("canonical_disambiguation_rule") << disambiguation_rule(spec, first.fetch("result").fetch("canonical_torrent_public_id"), rule_clock)
      # The shared test frame helper uses the same clock day for all frames.
      [fixture, *frames].each { |frame| frame["clock"] = frame.fetch("clock").sub("2026-09-12", "2026-09-13") }
      { "seed_clock" => seed, "rule_clock" => rule_clock,
        "fixture" => disambiguation_test_transport([fixture], mode, spec, role, fixture: true),
        "tested" => disambiguation_test_transport(frames, mode, spec, role),
        "initial" => disambiguation_test_read(attributes_empty, inputs, nil),
        "before_rule" => disambiguation_test_read(tables, inputs, 1),
        "before" => disambiguation_test_read(tables, with_rule, 1),
        "after" => disambiguation_test_read(tables, with_rule, 3) }
    end

    def disambiguation_mutations!
      spec = disambiguation_cases.first
      original = disambiguation_test_evidence(spec, "helpers-first", "final", @runtime)
      validate = ->(changed) { disambiguation_validate!(changed, spec, "helpers-first", "final", @runtime) }
      %w[23505 canonical_torrent_infohash_v1_uq public nbtinsert.c:666].each do |token|
        changed = copy(original)
        changed.fetch("tested")["stderr"] = changed.fetch("tested").fetch("stderr").gsub(token, "altered")
        rejected("native diagnostic") { validate.call(changed) }
      end
      changed = copy(original)
      changed.fetch("tested")["stdout"] = changed.fetch("tested").fetch("stdout").sub('role:{', 'role:{"session":"duplicate",')
      rejected("duplicate metadata JSON") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("tested").fetch("frames").first["backend"] = "altered"
      rejected("raw and declared") { validate.call(changed) }
      %w[backend before after finished_setting clock within outside].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").first[key] = key == "clock" ? "2026-02-30T00:00:00+00:00" : "altered"
        changed["tested"] = disambiguation_test_transport(changed.fetch("tested").fetch("frames"), "helpers-first", spec, @runtime)
        rejected("disambiguation") { validate.call(changed) }
      end
      original.fetch("before").fetch("data").fetch("tables").each do |table, rows|
        changed = copy(original)
        changed.fetch("after").fetch("data").fetch("tables").fetch(table) << { "unexpected" => true }
        changed.fetch("after")["stdout"] = JSON.generate(changed.fetch("after").fetch("data"))
        rejected("disambiguation") { validate.call(changed) }
        rows.each_with_index do |row, index|
          row.each_key do |column|
            changed = copy(original)
            read = changed.fetch("after")
            read.fetch("data").fetch("tables").fetch(table).fetch(index)[column] = "altered"
            read["stdout"] = JSON.generate(read.fetch("data"))
            rejected("disambiguation") { validate.call(changed) }
          end
        end
      end
      %w[identity_left_value_uuid identity_right_value_text identity_left_type rule_type created_at].each do |column|
        changed = copy(original)
        %w[before after].each do |key|
          read = changed.fetch(key)
          read.fetch("data").fetch("inputs").fetch("canonical_disambiguation_rule").first[column] = "altered"
          read["stdout"] = JSON.generate(read.fetch("data"))
        end
        rejected("rule or durable state") { validate.call(changed) }
      end
      [2, 3.0, nil].each do |sequence|
        changed = copy(original)
        changed.fetch("after").fetch("data")["canonical_sequence"] = sequence
        changed.fetch("after")["stdout"] = JSON.generate(changed.fetch("after").fetch("data"))
        rejected("rule or durable state") { validate.call(changed) }
      end
      changed = copy(original)
      changed.fetch("after")["stdout"] = changed.fetch("after").fetch("stdout").sub('"canonical_sequence":3', '"canonical_sequence":3.0')
      rejected("rule or durable state") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("after").fetch("data").fetch("inputs").fetch("trust_tier").first["default_weight"] = 0
      changed.fetch("after")["stdout"] = JSON.generate(changed.fetch("after").fetch("data"))
      rejected("rule or durable state") { validate.call(changed) }
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    proof = RevaerDatabaseRebaseline::IngestionDisambiguationTest.new
    if ARGV.empty?
      proof.run_tests!
    elsif ARGV.length == 2 && ARGV.first == "--live"
      proof.run_live!(ARGV.fetch(1))
    else
      raise RevaerDatabaseRebaseline::Failure, "expected no arguments or --live exact-candidate-path"
    end
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-disambiguation-test: #{error.message}"
    exit 1
  end
end
