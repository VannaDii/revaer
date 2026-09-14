# frozen_string_literal: true

require_relative "database-ingestion-sampling-test"

module RevaerDatabaseRebaseline
  class IngestionTemporalTest < IngestionSamplingTest
    def run_tests!
      sampling_test_setup!
      assert(temporal_cases.map { |spec| spec.fetch(:name) } == %w[guidless-older guidless-equal guidless-newer promote-guid-older promote-guid-equal promote-guid-newer], "six discriminating identity and temporal cases")
      temporal_cases.each do |spec|
        TEMPORAL_MODES.each do |mode|
          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = temporal_test_evidence(spec, mode, variant, role)
            assert(temporal_validate!(evidence, spec, mode, variant, role), "exact temporal #{spec.fetch(:name)} #{mode} #{variant}")
            session = temporal_session(temporal_observations(spec, mode).drop(1), helpers: mode == "helpers-first")
            query = correction_session(session)
            count = mode == "warm-committed" ? 2 : 1
            assert(query.scan("FROM public.search_result_ingest(").length == count && query.scan(/^COMMIT;$/).length == count, "real tested wrapper calls across commits")
            assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no compiler or temporary namespace repair")
            states = variant == "reference" && mode == "warm-committed" ? %w[00000 42P07] : Array.new(count, "00000")
            assert(evidence.fetch("tested").fetch("frames").map { |frame| frame.fetch("state") } == states, "frozen D4 remains a failure")
          end
        end
        temporal_mutations!(spec)
      end
      rejected("unknown temporal mode") { temporal_observations(temporal_cases.first, "reconnect") }
      puts "database-ingestion-temporal-test: #{@assertions} assertions passed"
    end

    private

    def attributes_source_hashes
      temporal_source_hashes
    end

    def verify_ingestion_attributes!
      verify_ingestion_temporal!
    ensure
      @attributes_evidence = @temporal_evidence
    end

    def temporal_test_evidence(spec, mode, variant, role)
      observations = temporal_observations(spec, mode)
      identities = { "canonical_torrent_public_id" => "56900000-0000-4000-8000-000000000090",
        "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-000000000091" }
      frames = observations.each_index.map { |index| { "clock" => format("2026-09-12T00:00:%02d+00:00", index + 1), "result" => identities } }
      before = attributes_empty
      frames.each_index do |index|
        success = variant == "final" || mode != "warm-committed" || index < 2
        after = success ? temporal_tables(observations, frames, index) : before
        frame = attributes_test_frame(role, index + 1, before, after, first: index.zero?).merge("finished_setting" => "error")
        unless success
          frame["state"] = "42P07"
          frame.delete("result")
        end
        frames[index] = frame
        before = after
      end
      seed = "2026-09-12T00:00:00+00:00"
      inputs = metadata_read_tables(seed)
      sequence = variant == "reference" && mode == "warm-committed" ? 2 : frames.length
      { "seed_clock" => seed, "fixture" => sampling_test_transport(frames.first(1), "cold", variant),
        "tested" => sampling_test_transport(frames.drop(1), mode, variant),
        "initial" => sampling_test_read(attributes_empty, inputs, nil),
        "prepared" => sampling_test_read(frames.first.fetch("tables_after"), inputs, 1),
        "after" => sampling_test_read(before, inputs, sequence) }
    end

    def temporal_mutations!(spec)
      original = temporal_test_evidence(spec, "warm-committed", "final", @runtime)
      last = original.fetch("tested").fetch("frames").last
      source = last.fetch("tables_after").fetch("canonical_torrent_source").first
      observation = last.fetch("tables_after").fetch("search_request_source_observation").first
      assert(source.fetch("source_guid") == spec.fetch(:guid) && observation.fetch("source_guid") == spec.fetch(:guid), "GUID promotion preserves the selected source and observation")
      expected_seeders = spec.fetch(:minute) > 1 ? 17 : 5
      assert(source.fetch("last_seen_seeders") == expected_seeders && observation.fetch("seeders") == 17, "strictly newer durable metadata differs from incoming observation")
      %w[canonical_torrent_source search_request_source_observation].each do |table|
        last.fetch("tables_after").fetch(table).first.each_key do |column|
          changed = copy(original)
          frame = changed.fetch("tested").fetch("frames").last
          tables = copy(frame.fetch("tables_after"))
          tables.fetch(table).first[column] = "altered"
          frame.merge!("tables_after" => tables, "tables_finish" => tables)
          changed["tested"] = sampling_test_transport(changed.fetch("tested").fetch("frames"), "warm-committed", "final")
          changed["after"] = sampling_test_read(tables, changed.fetch("after").fetch("data").fetch("inputs"), 3)
          rejected("full committed transition") { temporal_validate!(changed, spec, "warm-committed", "final", @runtime) }
        end
      end
      %w[backend clock before after within outside finished_setting].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").last[key] = "altered"
        changed["tested"] = sampling_test_transport(changed.fetch("tested").fetch("frames"), "warm-committed", "final")
        rejected("temporal") { temporal_validate!(changed, spec, "warm-committed", "final", @runtime) }
      end
      changed = copy(original)
      changed.fetch("tested")["stdout"] = changed.fetch("tested").fetch("stdout").sub('role:{', 'role:{"session":"duplicate",')
      rejected("duplicate metadata JSON") { temporal_validate!(changed, spec, "warm-committed", "final", @runtime) }
      changed = copy(original)
      changed.fetch("after").fetch("data")["sample_sequence"] = 2
      changed.fetch("after")["stdout"] = JSON.generate(changed.fetch("after").fetch("data"))
      rejected("independent inputs") { temporal_validate!(changed, spec, "warm-committed", "final", @runtime) }
      reference = temporal_test_evidence(spec, "warm-committed", "reference", "postgres")
      reference.fetch("tested")["stderr"] = reference.fetch("tested").fetch("stderr").gsub("42P07", "XXXXX")
      rejected("exact D4 diagnostic") { temporal_validate!(reference, spec, "warm-committed", "reference", "postgres") }
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    proof = RevaerDatabaseRebaseline::IngestionTemporalTest.new
    if ARGV.empty?
      proof.run_tests!
    elsif ARGV.length == 2 && ARGV.first == "--live"
      proof.run_live!(ARGV.fetch(1))
    else
      raise RevaerDatabaseRebaseline::Failure, "expected no arguments or --live exact-candidate-path"
    end
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-temporal-test: #{error.message}"
    exit 1
  end
end
