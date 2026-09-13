# frozen_string_literal: true

require_relative "../database_rebaseline/final_proof"

module RevaerDatabaseRebaseline
  class IngestionCancellationTest < FinalProof
    def run_tests!
      @assertions = 0
      %w[reference final].each do |variant|
        value = fixture(variant)
        validate(value, variant)
        assert(@checks.length == 1 && @checks.first.fetch(:passed), "valid synthetic #{variant}")
        value.fetch("blocked").each_key do |key|
          rejected(value, variant) { |changed| changed.fetch("blocked")[key] = "changed" }
        end
        value.fetch("locker").each_key do |key|
          rejected(value, variant) { |changed| changed.fetch("locker")[key] = "changed" }
        end
        %w[before after].each do |boundary|
          IngestionPool::POOL_SESSION_KEYS.each do |key|
            rejected(value, variant) { |changed| changed.fetch("frames").last.fetch(boundary)[key] = "changed" }
          end
        end
        value.fetch("checkpoint").first.fetch("outcome").each_key do |key|
          rejected(value, variant) do |changed|
            changed.fetch("checkpoint").first.fetch("outcome")[key] = "changed"
            changed.fetch("frames").first.fetch("outcome")[key] = "changed"
          end
        end
        IngestionProof::INGESTION_TABLES.each do |table|
          rejected(value, variant) { |changed| changed.fetch("cancelled").fetch(table) << { "unexpected" => true } }
          rejected(value, variant) { |changed| changed.fetch("cancelled").delete(table) }
        end
        %w[before cancelled after inputs_before inputs_cancelled inputs_after].each do |boundary|
          rejected(value, variant) { |changed| changed.fetch(boundary)["unexpected"] = [] }
        end
        IngestionPolicy::POLICY_READ_TABLES.each do |table|
          rejected(value, variant) { |changed| changed.fetch("inputs_cancelled").fetch(table) << { "unexpected" => true } }
        end
        IngestionPool::POOL_ROW_KEYS.each do |key|
          rejected(value, variant) { |changed| changed.fetch("frames").last.fetch("outcome").fetch("row")[key] = false }
        end
        rejected(value, variant) { |changed| changed.fetch("frames").pop }
        rejected(value, variant) { |changed| changed["checkpoint"] = [] }
        rejected(value, variant) { |changed| changed["cancel_result"] = "f" }
        rejected(value, variant) { |changed| changed.fetch("frames").last["unexpected"] = true }
        rejected(value, variant) { |changed| changed.fetch("after").fetch("canonical_torrent").clear }
        rejected(value, variant) { |changed| changed.fetch("after").fetch("canonical_torrent_source").first["last_seen_at"] = "changed" }
      end
      duplicate_rejected = begin
        metadata_json_parse('{"state":"00000","state":"57014"}')
        false
      rescue Failure
        true
      end
      assert(duplicate_rejected, "duplicate JSON rejected")
      unchanged_rejected = begin
        rejected(fixture("final"), "final") { |_value| }
        false
      rescue Failure
        true
      end
      assert(unchanged_rejected, "unchanged negative control fails the mutation harness")
      puts "database-ingestion-cancellation-test: #{@assertions} assertions passed"
    end

    private

    def assert(value, label)
      @assertions += 1
      raise Failure, "cancellation test failed: #{label}" unless value
    end

    def validate(value, variant)
      @checks = []
      @failures = []
      role = variant == "reference" ? "postgres" : "proof_runtime_unit"
      cancellation_validate!(value, variant, "ingestion_pool_cancel_unit", role)
    end

    def rejected(original, variant)
      changed = JSON.parse(JSON.generate(original))
      yield changed
      rejected = begin
        validate(changed, variant)
        false
      rescue Failure
        true
      end
      assert(rejected, "mutation rejected")
    end

    def fixture(variant)
      database = "ingestion_pool_cancel_unit"
      role = variant == "reference" ? "postgres" : "proof_runtime_unit"
      backend = { "pid" => 123, "database" => database, "session_role" => role, "current_role" => role,
                  "superuser" => variant == "reference", "create_role" => variant == "reference", "bypass_rls" => variant == "reference",
                  "server_version" => "160014", "conflict_setting" => "error" }
      failed = { "name" => "cancelled", "before" => backend.merge("conflict_setting" => nil), "after" => backend,
                 "outcome" => { "kind" => "database_error", "operation" => "search result ingest", "state" => "57014",
                                "message" => "canceling statement due to user request", "detail" => nil } }
      row = { "canonical" => "56900000-0000-4000-8000-000000000010", "source" => "56900000-0000-4000-8000-000000000011",
              "observation_created" => true, "durable_source_created" => true, "canonical_changed" => true }
      recovered = { "name" => "recovery", "before" => backend, "after" => backend, "outcome" => { "kind" => "success", "row" => row } }
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      inputs = IngestionPolicy::POLICY_READ_TABLES.to_h { |table| [table, []] }
      { "locker" => { "pid" => 321, "database" => database, "role" => "postgres" },
        "blocked" => { "pid" => 123, "database" => database, "role" => role, "application" => IngestionCancellation::CANCELLATION_APPLICATION,
                       "state" => "active", "wait_type" => "Lock", "wait_event" => "relation", "blockers" => [321],
                       "source_insert_wait" => true, "canonical_write_lock" => true, "query" => "SELECT * FROM search_result_ingest(" },
        "checkpoint" => [failed], "cancel_result" => "t", "frames" => [failed, recovered],
        "before" => empty, "cancelled" => empty, "inputs_before" => inputs, "inputs_cancelled" => inputs, "inputs_after" => inputs,
        "after" => empty.merge("canonical_torrent" => [{ "canonical_torrent_public_id" => row.fetch("canonical") }],
          "canonical_torrent_source" => [{ "canonical_torrent_source_public_id" => row.fetch("source"), "source_guid" => "pool-proof-source", "last_seen_at" => "2026-09-10T00:01:00+00:00" }],
          "search_request_source_observation" => [{}]) }
    end
  end
end

RevaerDatabaseRebaseline::IngestionCancellationTest.new.run_tests!
