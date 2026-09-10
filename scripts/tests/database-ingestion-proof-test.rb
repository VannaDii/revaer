# frozen_string_literal: true

require "tmpdir"
require_relative "../database_rebaseline/final_proof"
require_relative "../database_rebaseline/ingestion_proof"

module RevaerDatabaseRebaseline
  class IngestionProofTest < FinalProof
    include IngestionProof

    def run_tests!
      @assertions = 0
      @runtime = "ingestion_test_runtime"
      argument_tests!
      framing_tests!
      normalization_tests!
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
      "ERROR:  P0001: Failed to ingest search result\nDETAIL:  search_request_missing\n"
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
      hint = ingestion_parse(sample_stdout, sample_stderr + "HINT:  retained hint\n", role: @runtime)
      assert(actual != hint, "error hints must affect parity")
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
      end
    end
  end
end

RevaerDatabaseRebaseline::IngestionProofTest.new.run_tests!
