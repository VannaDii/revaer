# frozen_string_literal: true

require_relative "database-ingestion-attributes-test"

module RevaerDatabaseRebaseline
  class IngestionSamplingTest < IngestionAttributesTest
    def run_tests!
      @assertions = 0
      @runtime = "sampling_unit_runtime"
      routines = { "search_result_ingest_v1" => "0052_indexer_search_result_ingest_proc.sql",
        "search_result_ingest" => "0120_search_result_ingest_seed_best_source_context.sql" }.map do |name, path|
        source = File.binread(File.join(@contract.root, "crates/revaer-data/migrations", path))
        body = source.split("CREATE OR REPLACE FUNCTION #{name}(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
        { "name" => name, "signature" => "#{name}(uuid)", "source" => body }
      end
      @ingestion_inventory = { "reference_proof" => { "routines" => routines } }
      assert(sampling_cases.map { |spec| spec.fetch(:name) } == %w[three-samples duplicate-then-new out-of-order], "three distinct committed sampling paths")
      sampling_cases.each do |spec|
        SAMPLING_MODES.each do |mode|
          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = sampling_test_evidence(spec, mode, variant, role)
            assert(sampling_validate!(evidence, spec, mode, variant, role), "synthetic #{spec.fetch(:name)} #{mode} #{variant}")
            query = correction_session(sampling_session(spec, mode))
            assert(query.scan("FROM public.search_result_ingest(").length == 3 && query.scan("COMMIT;").length == 3, "three real wrapper calls across commits")
            assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no compiler or temporary namespace repair")
            assert(evidence.fetch("tested").fetch("stderr").scan("ERROR:  42P07:").length == (variant == "reference" ? 2 : 0), "D4 errors retained only on frozen reference")
          end
        end
      end
      rejected("unknown sampling mode") { sampling_session(sampling_cases.first, "repair") }
      sampling_mutations!
      puts "database-ingestion-sampling-test: #{@assertions} assertions passed"
    end

    private

    def attributes_source_hashes
      sampling_source_hashes
    end

    def verify_ingestion_attributes!
      verify_ingestion_sampling!
    ensure
      @attributes_evidence = @sampling_evidence
    end

    def sampling_test_transport(frames, mode, variant)
      lines = []
      lines << "helpers:#{JSON.generate(ingestion_helper_expectations)}" if mode == "helpers-first"
      frames.each do |frame|
        correction_records(finish_setting: true).each do |key|
          lines << JSON.generate(frame.fetch("result")) if key == "state" && frame.key?("result")
          value = frame.fetch(key)
          lines << "#{key}:#{%w[role clock tables_before tables_after tables_finish].include?(key) ? JSON.generate(value) : value}"
        end
      end
      { "stdout" => lines.join("\n") + "\n", "stderr" => variant == "reference" ? sampling_diagnostic * 2 : "", "frames" => frames }
    end

    def sampling_test_read(tables, inputs, sequence)
      data = { "tables" => copy(tables), "inputs" => copy(inputs), "sample_sequence" => sequence }
      { "stdout" => JSON.generate(data) + "\n", "stderr" => "", "data" => data }
    end

    def sampling_test_evidence(spec, mode, variant, role)
      identities = { "canonical_torrent_public_id" => "56900000-0000-4000-8000-000000000090",
        "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-000000000091" }
      frames = (1..3).map { |number| { "clock" => "2026-09-12T00:00:0#{number}+00:00", "result" => identities } }
      before = attributes_empty
      frames.each_index do |index|
        success = variant == "final" || index.zero?
        after = success ? sampling_tables(spec, frames, index) : before
        frame = attributes_test_frame(role, index + 1, before, after, first: index.zero?).merge("backend" => "100", "finished_setting" => "error")
        unless success
          frame["state"] = "42P07"
          frame.delete("result")
        end
        frames[index] = frame
        before = after
      end
      seed = "2026-09-12T00:00:00+00:00"
      inputs = metadata_read_tables(seed)
      { "seed_clock" => seed, "tested" => sampling_test_transport(frames, mode, variant),
        "initial" => sampling_test_read(attributes_empty, inputs, nil),
        "after" => sampling_test_read(before, inputs, variant == "reference" ? 1 : 3) }
    end

    def sampling_mutations!
      spec = sampling_cases.first
      original = sampling_test_evidence(spec, "helpers-first", "final", @runtime)
      validate = ->(changed) { sampling_validate!(changed, spec, "helpers-first", "final", @runtime) }
      changed = copy(original)
      changed.fetch("tested")["stdout"] = changed.fetch("tested").fetch("stdout").sub('role:{', 'role:{"session":"duplicate",')
      rejected("duplicate metadata JSON") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("tested").fetch("frames").last["backend"] = "altered"
      rejected("raw and declared") { validate.call(changed) }
      %w[backend before after finished_setting clock within outside].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").last[key] = key == "clock" ? "2026-02-30T00:00:00+00:00" : "altered"
        changed["tested"] = sampling_test_transport(changed.fetch("tested").fetch("frames"), "helpers-first", "final")
        rejected("sampling") { validate.call(changed) }
      end
      %w[session current superuser create_role bypass_rls].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").last.fetch("role")[key] = "altered"
        changed["tested"] = sampling_test_transport(changed.fetch("tested").fetch("frames"), "helpers-first", "final")
        rejected("backend role") { validate.call(changed) }
      end
      original.fetch("after").fetch("data").fetch("tables").each do |table, rows|
        changed = copy(original)
        read = changed.fetch("after")
        read.fetch("data").fetch("tables").fetch(table) << { "unexpected" => true }
        read["stdout"] = JSON.generate(read.fetch("data"))
        rejected("independent inputs") { validate.call(changed) }
        rows.each_with_index do |row, index|
          row.each_key do |column|
            changed = copy(original)
            read = changed.fetch("after")
            read.fetch("data").fetch("tables").fetch(table).fetch(index)[column] = "altered"
            read["stdout"] = JSON.generate(read.fetch("data"))
            rejected("independent inputs") { validate.call(changed) }
          end
        end
      end
      %w[tables_before tables_after tables_finish result].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").last.fetch(key)["unexpected"] = true
        changed["tested"] = sampling_test_transport(changed.fetch("tested").fetch("frames"), "helpers-first", "final")
        rejected("full committed transition") { validate.call(changed) }
      end
      [2, 3.0, nil].each do |sequence|
        changed = copy(original)
        changed.fetch("after").fetch("data")["sample_sequence"] = sequence
        changed.fetch("after")["stdout"] = JSON.generate(changed.fetch("after").fetch("data"))
        rejected("independent inputs") { validate.call(changed) }
      end
      changed = copy(original)
      changed.fetch("after")["stdout"] = changed.fetch("after").fetch("stdout").sub('"sample_sequence":3', '"sample_sequence":3.0')
      rejected("independent inputs") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("after").fetch("data").fetch("inputs").fetch("trust_tier").first["default_weight"] = 0
      changed.fetch("after")["stdout"] = JSON.generate(changed.fetch("after").fetch("data"))
      rejected("independent inputs") { validate.call(changed) }
      reference = sampling_test_evidence(spec, "helpers-first", "reference", "postgres")
      %w[42P07 tmp_policy_rules createas.c:406].each do |token|
        changed = copy(reference)
        changed.fetch("tested")["stderr"] = changed.fetch("tested").fetch("stderr").gsub(token, "altered")
        rejected("exact D4 diagnostic") { sampling_validate!(changed, spec, "helpers-first", "reference", "postgres") }
      end
      changed = copy(original)
      changed.fetch("tested")["stderr"] = sampling_diagnostic
      rejected("required outcomes") { validate.call(changed) }
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    proof = RevaerDatabaseRebaseline::IngestionSamplingTest.new
    if ARGV.empty?
      proof.run_tests!
    elsif ARGV.length == 2 && ARGV.first == "--live"
      proof.run_live!(ARGV.fetch(1))
    else
      raise RevaerDatabaseRebaseline::Failure, "expected no arguments or --live exact-candidate-path"
    end
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-sampling-test: #{error.message}"
    exit 1
  end
end
