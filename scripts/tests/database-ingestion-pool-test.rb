# frozen_string_literal: true

require_relative "../database_rebaseline/final_proof"

module RevaerDatabaseRebaseline
  class IngestionPoolTest < FinalProof
    def run_tests!
      @assertions = 0
      %w[reference final].each do |variant|
        value = fixture(variant)
        validate(value, variant)
        assert(@checks.all? { |item| item.fetch(:passed) }, "valid synthetic #{variant} fixture")
        %w[pid database session_role current_role superuser create_role bypass_rls server_version conflict_setting].each do |key|
          rejected(value, variant) { |changed| changed.fetch("frames").last.fetch("after")[key] = "changed" }
        end
        rejected(value, variant) { |changed| changed.fetch("frames").first.fetch("before")["conflict_setting"] = "error" }
        rejected(value, variant) { |changed| changed.fetch("frames").pop }
        rejected(value, variant) { |changed| changed.fetch("frames").first["name"] = "other" }
        rejected(value, variant) { |changed| changed.fetch("frames").first["unexpected"] = true }
        rejected(value, variant) { |changed| changed.fetch("before").delete("indexer_health_event") }
        rejected(value, variant) { |changed| changed.fetch("after")["unexpected"] = [] }
        rejected(value, variant) { |changed| changed.fetch("frames").first.fetch("outcome").fetch("row")["canonical"] = "invalid" }
        %w[observation_created durable_source_created canonical_changed].each do |key|
          rejected(value, variant) { |changed| changed.fetch("frames").first.fetch("outcome").fetch("row")[key] = false }
        end
        %w[operation state message detail].each do |key|
          rejected(value, variant) { |changed| changed.fetch("frames")[2].fetch("outcome")[key] = "changed" }
        end
        rejected(value, variant) { |changed| changed.fetch("frames")[2].fetch("outcome").delete("detail") }
        rejected(value, variant) { |changed| changed.fetch("before").fetch("canonical_torrent") << { "unexpected" => true } }
        rejected(value, variant) { |changed| changed.fetch("after").fetch("canonical_torrent_source").first["last_seen_at"] = "2026-09-10T01:00:00Z" }
        rejected(value, variant) { |changed| changed.fetch("after").fetch("canonical_torrent").clear }
      end
      duplicate_rejected = false
      begin
        metadata_json_parse('[{"state":"00000","state":"P0001"}]')
      rescue Failure
        duplicate_rejected = true
      end
      assert(duplicate_rejected, "duplicate JSON rejected")
      control_rejected = false
      begin
        rejected(fixture("final"), "final") { |_unchanged| }
      rescue Failure
        control_rejected = true
      end
      assert(control_rejected, "mutation harness cannot accept an unchanged control")
      puts "database-ingestion-pool-test: #{@assertions} assertions passed"
    end

    private

    def assert(value, label)
      @assertions += 1
      raise Failure, "pool test failed: #{label}" unless value
    end

    def validate(value, variant)
      @checks = []
      @failures = []
      role = variant == "reference" ? "postgres" : "proof_runtime_unit"
      pool_validate!(value, variant, "ingestion_pool_unit", role)
    end

    def rejected(original, variant)
      changed = JSON.parse(JSON.generate(original))
      yield changed
      rejected = begin
        validate(changed, variant)
        @checks.any? { |item| !item.fetch(:passed) }
      rescue Failure
        true
      end
      assert(rejected, "mutation rejected")
    end

    def fixture(variant)
      role = variant == "reference" ? "postgres" : "proof_runtime_unit"
      session = { "pid" => 123, "database" => "ingestion_pool_unit", "session_role" => role,
                  "current_role" => role, "superuser" => variant == "reference", "create_role" => variant == "reference",
                  "bypass_rls" => variant == "reference", "server_version" => "160014", "conflict_setting" => "error" }
      canonical = "56900000-0000-4000-8000-000000000010"
      source = "56900000-0000-4000-8000-000000000011"
      frames = IngestionPool::POOL_STEPS.each_with_index.map do |name, index|
        state = index == 2 ? "P0001" : (variant == "reference" && index.positive? ? "42P07" : "00000")
        outcome = if state == "00000"
                    { "kind" => "success", "row" => { "canonical" => canonical, "source" => source,
                      "observation_created" => index.zero?, "durable_source_created" => index.zero?, "canonical_changed" => index.zero? } }
                  else
                    { "kind" => "database_error", "operation" => "search result ingest", "state" => state,
                      "message" => state == "P0001" ? "Failed to ingest search result" : 'relation "tmp_policy_rules" already exists',
                      "detail" => state == "P0001" ? "search_request_not_found" : nil }
                  end
        { "name" => name, "before" => session.merge("conflict_setting" => index.zero? ? nil : "error"),
          "after" => session.dup, "outcome" => outcome }
      end
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      { "frames" => frames, "before" => empty,
        "after" => empty.merge("canonical_torrent" => [{ "canonical_torrent_public_id" => canonical }],
                     "canonical_torrent_source" => [{ "canonical_torrent_source_public_id" => source, "source_guid" => "pool-proof-source",
                       "last_seen_at" => "2026-09-10T00:#{variant == 'reference' ? '00' : '03'}:00+00:00" }],
                     "search_request_source_observation" => [{}]) }
    end
  end
end

RevaerDatabaseRebaseline::IngestionPoolTest.new.run_tests!
