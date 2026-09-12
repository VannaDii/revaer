# frozen_string_literal: true

require "tmpdir"
require_relative "../database_rebaseline/final_proof"
require_relative "../database_rebaseline/ingestion_proof"
require_relative "database-ingestion-existing-test"
require_relative "database-ingestion-corrections-test"
require_relative "database-ingestion-approved-deltas-test"
require_relative "database-ingestion-compilation-test"
require_relative "database-ingestion-wrapper-test"

module RevaerDatabaseRebaseline
  class IngestionProofTest < FinalProof
    include IngestionProof
    include IngestionExistingTest
    include IngestionCorrectionsTest
    include IngestionApprovedDeltasTest
    include IngestionCompilationTest
    include IngestionWrapperTest

    def run_tests!
      @assertions = 0
      @runtime = "ingestion_test_runtime"
      @ingestion_inventory = {
        "reference_proof" => { "routines" => [{
          "name" => "search_result_ingest_v1", "signature" => "search_result_ingest_v1(uuid)",
          "source" => "fixture line\n" * 140
        }] }
      }
      argument_tests!
      framing_tests!
      diagnostic_tests!
      helper_first_tests!
      session_control_tests!
      normalization_tests!
      existing_data_tests!
      correction_tests!
      approved_delta_tests!
      compilation_tests!
      wrapper_tests!
      cleanup_tests!
      puts "database-ingestion-proof-test: #{@assertions} assertions passed"
    end

    private

    def assert(value, message)
      raise Failure, message unless value

      @assertions += 1
    end

    def rejected(message)
      begin
        yield
      rescue Failure => error
        assert(error.message.include?(message), "unexpected failure: #{error.message}")
        return
      end
      raise Failure, "expected rejection: #{message}"
    end

    def argument_tests!
      assert(ingestion_call({}).scan("_input =>").length == 24, "ingestion argument inventory incomplete")
      rejected("unknown ingestion fixture") { ingestion_call(unapproved_input: "NULL") }
      warm = ingestion_session({}, repeat: true)
      assert(warm.scan("FROM public.search_result_ingest_v1(").length == 2, "warm proof must call real ingestion twice")
      assert(warm.include?("COMMIT;\nBEGIN;"), "warm proof must commit between calls")
      assert(warm.include?("\\if :ERROR\nROLLBACK TO SAVEPOINT ingestion_second;\n\\endif"), "successful second writes must not be rolled back")
      assert(!warm.include?("DROP TABLE") && !warm.include?("DISCARD") && !warm.include?("\\connect"), "warm proof must not repair the backend")
      assert(!warm.include?("SET ROLE") && !warm.include?("SET plpgsql.variable_conflict"), "proof must not substitute roles or alter compilation settings")
      assert(ingestion_session({}).include?("\\if :ERROR"), "failed calls require explicit rollback")
      assert(ingestion_cases.last.last == ["00000", "00000"], "shared warm error must not become accepted success")
    end

    def sample_stdout
      role = { session: @runtime, current: @runtime, superuser: false, create_role: false, bypass_rls: false }
      ["role:#{JSON.generate(role)}", 'clock:"2026-09-10T00:00:00+00:00"',
       "before:error", "state: P0001", "after:error"].join("\n") + "\n"
    end

    def sample_stderr
      "ERROR:  P0001: Failed to ingest search result\nDETAIL:  search_request_missing\n" \
        "CONTEXT:  PL/pgSQL function search_result_ingest_v1(uuid) line 131 at RAISE\n" \
        "LOCATION:  exec_stmt_raise, pl_exec.c:3897\n"
    end

    def framing_tests!
      actual = ingestion_parse(sample_stdout, sample_stderr, role: @runtime)
      assert(actual.fetch("states") == ["P0001"], "SQLSTATE not retained")
      assert(actual.fetch("details") == ["search_request_missing"], "error detail not retained")
      rejected("unrecognized record") { ingestion_parse(sample_stdout + "unexpected\n", sample_stderr, role: @runtime) }
      rejected("role substitution") { ingestion_parse(sample_stdout.sub('"current":"ingestion_test_runtime"', '"current":"postgres"'), sample_stderr, role: @runtime) }
      %w[superuser create_role bypass_rls].each do |flag|
        rejected("forbidden capability") { ingestion_parse(sample_stdout.sub("\"#{flag}\":false", "\"#{flag}\":true"), sample_stderr, role: @runtime) }
      end
      rejected("scope changed") { ingestion_parse(sample_stdout.sub("after:error", "after:use_column"), sample_stderr, role: @runtime) }
      rejected("clock evidence") { ingestion_parse(sample_stdout.lines.reject { |line| line.start_with?("clock:") }.join, sample_stderr, role: @runtime) }
      rejected("framing is incomplete") { ingestion_parse(sample_stdout.sub("P0001", "00000"), sample_stderr, role: @runtime) }
      rejected("framing is incomplete") { ingestion_parse(sample_stdout, "", role: @runtime) }
      changed = ingestion_parse(sample_stdout, sample_stderr.sub("search_request_missing", "different_detail"), role: @runtime)
      assert(actual != changed, "error details must affect parity")
      hint = ingestion_parse(sample_stdout, sample_stderr.sub("CONTEXT:", "HINT:  retained hint\nCONTEXT:"), role: @runtime)
      assert(actual != hint, "error hints must affect parity")
    end

    def diagnostic_tests!
      expected = ingestion_diagnostics(sample_stderr, role: @runtime)
      assert(expected.first.fetch("line") == 130, "only the exact D3 line offset may normalize")
      assert(expected == ingestion_diagnostics(sample_stderr.sub("line 131", "line 130"), role: "postgres"), "D3 reference/final stack locations must align")
      assert(expected != ingestion_diagnostics(sample_stderr.sub("line 131", "line 132"), role: @runtime), "other stack locations must affect parity")
      assert(expected != ingestion_diagnostics(sample_stderr.sub("pl_exec.c:3897", "pl_exec.c:3898"), role: @runtime), "native diagnostic location must affect parity")
      ["state: BROKEN\n", "state: 00000 trailing\n", "state: \n"].each do |record|
        rejected("unrecognized record") { ingestion_parse(sample_stdout + record, sample_stderr, role: @runtime) }
      end
      ["NOTICE: extra\n", "WARNING: extra\n", "arbitrary stderr\n", "DETAIL: extra\n"].each do |record|
        rejected("unrecognized record") { ingestion_parse(sample_stdout, sample_stderr + record, role: @runtime) }
        rejected("unrecognized record") { ingestion_parse(sample_stdout, record + sample_stderr, role: @runtime) }
      end
      rejected("unrecognized record") { ingestion_diagnostics(sample_stderr.sub("CONTEXT:", "CONTEXT: extra\n"), role: @runtime) }
      rejected("unrecognized record") { ingestion_diagnostics(sample_stderr.delete_suffix("\n"), role: @runtime) }
      rejected("outside the frozen routine") { ingestion_diagnostics(sample_stderr.sub("line 131", "line 1"), role: @runtime) }
      rejected("outside the frozen routine") { ingestion_diagnostics(sample_stderr.sub("line 131", "line 999999"), role: @runtime) }
      rejected("signature differs") { ingestion_diagnostics(sample_stderr.sub("ingest_v1(uuid)", "ingest_v1(text)"), role: @runtime) }
      statement = "SELECT proof\n  FROM frozen_fixture"
      @ingestion_inventory.fetch("reference_proof").fetch("routines").first["source"] += statement
      multiline = sample_stderr.sub("CONTEXT:  ", "CONTEXT:  SQL statement \"#{statement}\"\n")
      assert(ingestion_diagnostics(multiline, role: @runtime).first.fetch("statement").include?(statement), "multiline diagnostic SQL must be retained")
      rejected("outside the frozen routine") { ingestion_diagnostics(multiline.sub("frozen_fixture", "changed_fixture"), role: @runtime) }
      rejected("outside the frozen routine") { ingestion_diagnostics(multiline.sub("FROM frozen_fixture", "NOTICE: concealed"), role: @runtime) }
      ["", "\n"].each do |empty|
        rejected("outside the frozen routine") { ingestion_diagnostics(multiline.sub(statement, empty), role: @runtime) }
      end
    end

    def sample_value
      canonical = "56900000-0000-4000-8000-000000000003"
      source = "56900000-0000-4000-8000-000000000004"
      clock = "2026-09-10T00:00:00+00:00"
      {
        "clocks" => [clock],
        "results" => [{ "canonical_torrent_public_id" => canonical, "canonical_torrent_source_public_id" => source, "canonical_changed" => true }],
        "after" => {
          "canonical_torrent" => [{ "canonical_torrent_id" => 1, "canonical_torrent_public_id" => canonical, "created_at" => clock, "title" => "proof" }],
          "canonical_torrent_source" => [{ "canonical_torrent_source_id" => 2, "canonical_torrent_source_public_id" => source, "canonical_torrent_id" => 1 }]
        }
      }
    end

    def helper_first_tests!
      cases = ingestion_cases
      assert(cases.length == 13, "retain six cold cases, six helper-first cases and the warm counterexample")
      assert(cases.count { |entry| entry[3] } == 6, "all cold cases must repeat after helper compilation")
      expected = ingestion_helper_expectations
      assert(expected.keys.sort == INGESTION_HELPERS.reject { |name| %w[search_result_ingest search_result_ingest_v1 log_source_metadata_conflict_v1].include?(name) }.sort, "pure helper inventory changed")
      warm = ingestion_session({}, helpers_first: true)
      assert(warm.index("SELECT 'helpers:'") < warm.index("SAVEPOINT ingestion_call;"), "helpers must compile before ingestion")
      assert(!ingestion_session({}).include?("SELECT 'helpers:'"), "cold cases must retain uncompiled helpers")
      assert(!warm.include?("SET plpgsql.variable_conflict") && !warm.include?("SET ROLE") && !warm.include?("DISCARD"), "helper-first proof must not change session authority")
      expected.each_key do |name|
        assert(warm.include?("public.#{name}("), "missing direct helper call: #{name}")
      end
      record = "helpers:#{JSON.generate(expected)}\n"
      stdout = sample_stdout.sub("state:", "#{record}state:")
      actual = ingestion_parse(stdout, sample_stderr, role: @runtime, helpers_first: true)
      assert(actual.fetch("helpers") == [expected], "exact helper outputs must be retained")
      rejected("known answers") { ingestion_parse(sample_stdout, sample_stderr, role: @runtime, helpers_first: true) }
      rejected("known answers") { ingestion_parse(stdout, sample_stderr, role: @runtime) }
      rejected("known answers") { ingestion_parse(stdout + record, sample_stderr, role: @runtime, helpers_first: true) }
      expected.each_key do |name|
        changed = JSON.parse(JSON.generate(expected))
        changed.fetch(name)[0] = "unexpected"
        changed_record = "helpers:#{JSON.generate(changed)}\n"
        rejected("known answers") { ingestion_parse(stdout.sub(record, changed_record), sample_stderr, role: @runtime, helpers_first: true) }
      end
      rejected("not observed before") { ingestion_parse(sample_stdout + record, sample_stderr, role: @runtime, helpers_first: true) }
      rejected("not observed before") { ingestion_parse(record + sample_stdout, sample_stderr, role: @runtime, helpers_first: true) }
      result = JSON.generate(sample_value.fetch("results").first) + "\n"
      late = sample_stdout.sub("state: P0001", "#{result}#{record}state: 00000")
      rejected("not observed before") { ingestion_parse(late, "", role: @runtime, helpers_first: true) }
    end

    def session_control_tests!
      success = CommandRunner::Result.new(stdout: "state: 00000\nstate: 00000\nwrites:1,2\n", stderr: "", success: true)
      assert(ingestion_control_records(success, failure_expected: false) == ["writes:1,2"], "successful control writes retained")
      failed_call = CommandRunner::Result.new(stdout: "state: 00000\nstate: 22012\nwrites:1\n", stderr: "ERROR:  22012: division by zero\nLOCATION:  int4div, int.c:870\n", success: true)
      assert(ingestion_control_records(failed_call, failure_expected: true) == ["writes:1"], "division-by-zero control accepted")
      rejected("SQLSTATE mismatch") { ingestion_control_records(failed_call, failure_expected: false) }
      rejected("SQLSTATE mismatch") { ingestion_control_records(success, failure_expected: true) }
      changed = failed_call.with(stdout: failed_call.stdout.sub("22012", "42501"))
      rejected("SQLSTATE mismatch") { ingestion_control_records(changed, failure_expected: true) }
      changed = failed_call.with(stderr: failed_call.stderr.sub("22012: division by zero", "42501: permission denied"))
      rejected("unexpected diagnostic") { ingestion_control_records(changed, failure_expected: true) }
      [success, failed_call].each do |outcome|
        changed = outcome.with(stderr: outcome.stderr + "WARNING: unexpected warning\n")
        rejected("unexpected diagnostic") { ingestion_control_records(changed, failure_expected: outcome == failed_call) }
      end
      changed = failed_call.with(stderr: failed_call.stderr + "ERROR:  22012: division by zero\nLOCATION:  int4div, int.c:870\n")
      rejected("unexpected diagnostic") { ingestion_control_records(changed, failure_expected: true) }
      changed = success.with(success: false)
      rejected("control failed") { ingestion_control_records(changed, failure_expected: false) }
    end

    def normalization_tests!
      value = sample_value
      actual = ingestion_comparable(value)
      assert(actual.fetch("after").fetch("canonical_torrent").first.fetch("created_at") == "<transaction-time:0>", "transaction timestamps not accounted for")
      assert(value == sample_value, "normalization must not mutate raw evidence")
      replacement = JSON.parse(JSON.generate(value).gsub("56900000", "56900001").gsub("2026-09-10T00:00:00", "2026-09-10T00:00:01"))
      assert(actual == ingestion_comparable(replacement), "generated identities and transaction clocks must compare")
      changed = sample_value
      changed.fetch("results").first["canonical_changed"] = false
      assert(actual != ingestion_comparable(changed), "result flag changes must remain visible")
      changed = sample_value
      changed.fetch("after").fetch("canonical_torrent").first["title"] = "changed"
      assert(actual != ingestion_comparable(changed), "table mutation changes must remain visible")
      changed = sample_value
      changed.fetch("after").fetch("canonical_torrent_source").first["canonical_torrent_id"] = 3
      assert(actual != ingestion_comparable(changed), "relationship changes must remain visible")
      changed = sample_value
      changed.fetch("after").fetch("canonical_torrent").first["created_at"] = "2026-01-01T00:00:00+00:00"
      assert(actual != ingestion_comparable(changed), "non-transaction timestamps must remain visible")
      changed = sample_value
      changed.fetch("after").fetch("canonical_torrent").first["canonical_torrent_public_id"] = "invalid"
      rejected("identity is invalid") { ingestion_comparable(changed) }
      changed = sample_value
      changed.fetch("after").fetch("canonical_torrent_source").first["canonical_torrent_source_public_id"] = changed.fetch("results").first.fetch("canonical_torrent_public_id")
      rejected("identity is invalid or duplicated") { ingestion_comparable(changed) }
      changed = sample_value
      changed.fetch("results").first["canonical_torrent_public_id"] = "56900000-0000-4000-8000-000000000005"
      rejected("no committed row") { ingestion_comparable(changed) }
    end

    def cleanup_tests!
      commands = []
      transport = Object.new
      transport.define_singleton_method(:capture) do |_command, stdin_data:|
        commands << stdin_data
        if stdin_data.start_with?("INSERT INTO") || stdin_data.start_with?("-- Disposable")
          CommandRunner::Result.new(stdout: "", stderr: "injected fixture failure", success: false)
        else
          CommandRunner::Result.new(stdout: "", stderr: "", success: true)
        end
      end
      @runner = transport
      Dir.mktmpdir("revaer-ingestion-proof-test.") do |directory|
        @contract = Struct.new(:output_path).new(directory)
        rejected("PostgreSQL proof query failed") do
          ingestion_isolated("cleanup-test", "SELECT 1", source: "reference_proof", role: "postgres", variant: "reference")
        end
        assert(commands.last == 'DROP DATABASE "ingestion_reference_proof" WITH (FORCE)', "failed fixture must drop its clone")
        rejected("PostgreSQL proof query failed") do
          ingestion_existing_isolated("cleanup-test", {}, {}, source: "reference_proof", role: "postgres", variant: "reference")
        end
        assert(commands.last == 'DROP DATABASE "ingestion_reference_proof" WITH (FORCE)', "failed existing-data fixture must drop its clone")
      end
    end
  end
end

RevaerDatabaseRebaseline::IngestionProofTest.new.run_tests!
