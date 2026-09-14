# frozen_string_literal: true

require_relative "database-ingestion-sampling-test"
require_relative "../database_rebaseline/ingestion_committed_hash_fill"

module RevaerDatabaseRebaseline
  class IngestionCommittedHashFillTest < IngestionSamplingTest
    include IngestionCommittedHashFill

    def run_tests!
      sampling_test_setup!
      cases = wrapper_hash_fill_cases
      assert(cases.map { |spec| spec.fetch(:name) } == %w[fill-v1-uncontested fill-v1-competing fill-v2-uncontested fill-v2-competing fill-magnet-uncontested fill-magnet-competing], "six explicit hash fill cases")
      cases.each do |spec|
        IngestionSampling::SAMPLING_MODES.each do |mode|
          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = committed_hash_test_evidence(spec, mode, variant, role)
            assert(committed_hash_validate!(evidence, spec, mode, variant, role), "exact committed #{spec.fetch(:name)} #{mode} #{variant}")
            query = correction_session(committed_hash_session(spec, mode))
            assert(query.scan("FROM public.search_result_ingest(").length == 3 && query.scan(/^COMMIT;$/).length == 3, "warm fill and reuse cross real commits")
            assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no backend or compiler repair")
            states = variant == "reference" ? %w[00000 42P07 42P07] : %w[00000 00000 00000]
            assert(evidence.fetch("tested").fetch("frames").map { |frame| frame.fetch("state") } == states, "frozen repeated calls retain D4 failures")
            conflicts = evidence.fetch("after").fetch("data").fetch("tables").fetch("source_metadata_conflict")
            count = spec.fetch(:hash_fill).fetch(:competing) ? 2 : 0
            assert(conflicts.length == (variant == "final" ? count : 0), "only successful commits retain conflict rows")
            sequence = evidence.fetch("after").fetch("data").fetch("sequences").fetch("source_metadata_conflict_source_metadata_conflict_id_seq")
            assert(sequence == (count.zero? ? nil : count), "both frozen failed calls consume conflict identities before D4")
          end
        end
        committed_hash_mutations!(spec)
      end
      rejected("unknown committed hash-fill mode") { committed_hash_session(cases.first, "reconnect") }
      puts "database-ingestion-committed-hash-fill-test: #{@assertions} assertions passed"
    end

    private

    def attributes_source_hashes
      committed_hash_source_hashes
    end

    def verify_ingestion_attributes!
      verify_ingestion_committed_hash_fill!
    ensure
      @attributes_evidence = @committed_hash_evidence
    end

    def committed_hash_test_read(tables, inputs, sequences)
      data = { "tables" => copy(tables), "inputs" => copy(inputs), "sequences" => sequences }
      { "stdout" => JSON.generate(data) + "\n", "stderr" => "", "data" => data }
    end

    def committed_hash_test_evidence(spec, mode, variant, role)
      identities = [90, 92].map do |number|
        { "canonical_torrent_public_id" => format("56900000-0000-4000-8000-%012d", number),
          "canonical_torrent_source_public_id" => format("56900000-0000-4000-8000-%012d", number + 1) }
      end
      frames = (1..5).map do |number|
        { "clock" => format("2026-09-12T00:00:%02d+00:00", number), "result" => identities.fetch(number == 2 ? 1 : 0) }
      end
      fixtures = committed_hash_fixtures(spec, frames.first(2))
      transitions = committed_hash_tables(spec, fixtures.last, frames.drop(2))
      before = attributes_empty
      frames.each_index do |index|
        success = variant == "final" || index < 3
        expected = index < 2 ? fixtures.fetch(index) : transitions.fetch(index - 2)
        after = success ? expected : before
        frame = { "backend" => (100 + [index, 2].min).to_s, "finished_setting" => "error", "clock" => frames.fetch(index).fetch("clock"),
          "before" => "error", "after" => "error", "state" => "00000", "within" => "true", "outside" => (variant == "reference").to_s,
          "role" => { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" },
          "tables_before" => before, "tables_after" => after, "tables_finish" => after }
        if success
          identity = index < 2 ? identities.fetch(index) : identities.first.merge(index > 2 ? identities.last.slice("canonical_torrent_public_id") : {})
          frame["result"] = identity.merge("canonical_changed" => index < 2, "observation_created" => index < 2, "durable_source_created" => index < 2)
        else
          frame["state"] = "42P07"
          frame.delete("result")
        end
        frames[index] = frame
        before = after
      end
      seed = "2026-09-12T00:00:00+00:00"
      inputs = metadata_read_tables(seed)
      conflicts = spec.fetch(:hash_fill).fetch(:competing) ? 2 : nil
      sequences = variant == "final" ? committed_hash_sequence_values(3, 5, conflicts) : committed_hash_sequence_values(1, 3, conflicts)
      { "seed_clock" => seed, "fixtures" => frames.first(2).map { |frame| sampling_test_transport([frame], "cold", variant) },
        "tested" => sampling_test_transport(frames.drop(2), mode, variant),
        "initial" => committed_hash_test_read(attributes_empty, inputs, committed_hash_sequence_values(nil, nil, nil)),
        "prepared" => committed_hash_test_read(fixtures.last, inputs, committed_hash_sequence_values(1, 2, nil)),
        "after" => committed_hash_test_read(before, inputs, sequences) }
    end

    def committed_hash_mutations!(spec)
      original = committed_hash_test_evidence(spec, "helpers-first", "final", @runtime)
      validate = ->(evidence) { committed_hash_validate!(evidence, spec, "helpers-first", "final", @runtime) }
      original.fetch("after").fetch("data").fetch("tables").each_key do |table|
        changed = copy(original)
        frame = changed.fetch("tested").fetch("frames").last
        tables = copy(frame.fetch("tables_after"))
        tables.fetch(table) << { "unexpected" => true }
        frame.merge!("tables_after" => tables, "tables_finish" => tables)
        changed["tested"] = sampling_test_transport(changed.fetch("tested").fetch("frames"), "helpers-first", "final")
        changed["after"] = committed_hash_test_read(tables, changed.fetch("after").fetch("data").fetch("inputs"), changed.fetch("after").fetch("data").fetch("sequences"))
        rejected("independent fixture fill reuse or rollback") { validate.call(changed) }
      end
      %w[canonical_torrent_source search_request_source_observation canonical_torrent_source_context_score source_metadata_conflict].each do |table|
        original.fetch("after").fetch("data").fetch("tables").fetch(table).each_with_index do |row, index|
          row.each_key do |column|
            changed = copy(original)
            read = changed.fetch("after")
            read.fetch("data").fetch("tables").fetch(table).fetch(index)[column] = "altered"
            read["stdout"] = JSON.generate(read.fetch("data"))
            rejected("independent read inputs or identity sequences") { validate.call(changed) }
          end
        end
      end
      original.fetch("after").fetch("data").fetch("sequences").each do |name, value|
        [value.nil? ? 0 : value.to_f, 99, "missing"].each do |replacement|
          changed = copy(original)
          sequences = changed.fetch("after").fetch("data").fetch("sequences")
          replacement == "missing" ? sequences.delete(name) : sequences[name] = replacement
          changed.fetch("after")["stdout"] = JSON.generate(changed.fetch("after").fetch("data"))
          rejected("independent read inputs or identity sequences") { validate.call(changed) }
        end
      end
      %w[backend clock before after within outside finished_setting].each do |key|
        changed = copy(original)
        changed.fetch("tested").fetch("frames").last[key] = "altered"
        changed["tested"] = sampling_test_transport(changed.fetch("tested").fetch("frames"), "helpers-first", "final")
        rejected("committed hash backend") { validate.call(changed) }
      end
      changed = copy(original)
      changed.fetch("fixtures").first.fetch("frames").first["backend"] = "102"
      changed.fetch("fixtures")[0] = sampling_test_transport(changed.fetch("fixtures").first.fetch("frames"), "cold", "final")
      rejected("committed hash backend") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("tested")["stdout"] = changed.fetch("tested").fetch("stdout").sub('role:{', 'role:{"session":"duplicate",')
      rejected("duplicate metadata JSON") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("after").fetch("data").fetch("inputs").fetch("trust_tier").first["rank"] = 99
      changed.fetch("after")["stdout"] = JSON.generate(changed.fetch("after").fetch("data"))
      rejected("independent read inputs or identity sequences") { validate.call(changed) }
      reference = committed_hash_test_evidence(spec, "helpers-first", "reference", "postgres")
      %w[42P07 tmp_policy_rules createas.c:406].each do |token|
        changed = copy(reference)
        changed.fetch("tested")["stderr"] = changed.fetch("tested").fetch("stderr").gsub(token, "altered")
        rejected("exact D4 diagnostic") { committed_hash_validate!(changed, spec, "helpers-first", "reference", "postgres") }
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    proof = RevaerDatabaseRebaseline::IngestionCommittedHashFillTest.new
    if ARGV.empty?
      proof.run_tests!
    elsif ARGV.length == 2 && ARGV.first == "--live"
      proof.run_live!(ARGV.fetch(1))
    else
      raise RevaerDatabaseRebaseline::Failure, "expected no arguments or --live exact-candidate-path"
    end
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-committed-hash-fill-test: #{error.message}"
    exit 1
  end
end
