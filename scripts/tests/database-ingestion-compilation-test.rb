# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionCompilationTest
    private

    def compilation_tests!
      cases = compilation_cases
      assert(cases.map { |entry| entry.fetch(:name) } == %w[cold-canonical-setting cold-logger-setting logger-first-setting cold-decision-setting], "retain the bounded compilation case inventory")
      logger = cases.fetch(2)
      rejected("real persisted source") { compilation_call(logger.fetch(:calls).first, nil) }
      rejected("unknown compilation") { compilation_call({ operation: :unknown }, 1) }
      query = correction_session(logger) { |operation| compilation_call(operation, 1) }
      assert(query.scan("public.log_source_metadata_conflict_v1(").length == 2 && query.scan("public.search_result_ingest_v1(").length == 1, "compile the actual mutating helper before ingestion")
      assert(query.scan("BEGIN;").length == 3 && query.scan("COMMIT;").length == 3, "logger-first operations require separate same-backend commits")
      assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "compilation proof must not repair or change its tested backend")
      assert(correction_session(calls: [{}]).include?(ingestion_call({})), "default D4/D5 session must retain the real ingestion call")
      compilation_event_tests!(cases)
      compilation_normalization_tests!
      compilation_logger_tests!
      compilation_decision_tests!
    end

    def compilation_event_tests!(cases)
      frame = correction_test_frames.first
      @owner = "compilation_test_owner"
      cases.each do |test_case|
        frames = test_case.fetch(:calls).each_with_index.map do |operation, index|
          frame.merge("clock" => "2026-09-10T00:0#{index}:00+00:00", "within" => (operation.fetch(:operation) == :ingest).to_s)
        end
        %w[reference final].each do |variant|
          role = variant == "reference" ? "postgres" : @runtime
          events = []
          test_case.fetch(:calls).zip(frames).each do |operation, item|
            helper = operation.fetch(:operation) == :logger
            relations = helper ? ["source_metadata_conflict"] : ["canonical_torrent"]
            relations += %w[source_metadata_conflict source_metadata_conflict] if !helper && test_case[:fixture]
            relations += ["search_filter_decision"] if test_case[:policy]
            relations.each do |relation|
              events << { "event_id" => events.length + 1, "relation_name" => relation, "operation" => "INSERT",
                          "backend" => 101, "session_role" => role, "current_role_name" => variant == "reference" ? "postgres" : @owner,
                          "variable_conflict" => variant == "reference" && !helper ? "use_column" : "error", "transaction_clock" => item.fetch("clock") }
            end
          end
          assert(compilation_settings?(test_case, frames, events, variant, role, observed: true), "exact #{variant} in-call setting sequence")
          assert(compilation_settings?(test_case, frames, [], variant, role, observed: false), "plain run has no observer events")
          assert(!compilation_settings?(test_case, frames, events, variant, role, observed: false), "plain run must reject observer events")
          assert(!compilation_settings?(test_case, frames, events.drop(1), variant, role, observed: true), "missing in-call event must fail")
          assert(!compilation_settings?(test_case, frames, events + events.take(1), variant, role, observed: true), "extra in-call event must fail")
          events.first.each_key do |key|
            changed = Marshal.load(Marshal.dump(events))
            changed.first[key] = "changed"
            assert(!compilation_settings?(test_case, frames, changed, variant, role, observed: true), "changed event #{key} must fail")
          end
          evidence = { "frames" => frames.map { |item| item.merge("outside" => (item.fetch("within") == "true" && variant == "reference").to_s) }, "fixture" => nil }
          assert(compilation_temp_lifetime?(test_case, evidence, variant), "exact #{variant} approved temporary-table lifetime")
          %w[within outside].each do |key|
            changed = Marshal.load(Marshal.dump(evidence))
            changed.fetch("frames").first[key] = "unexpected"
            assert(!compilation_temp_lifetime?(test_case, changed, variant), "changed #{key} must not normalize away")
          end
        end
      end
    end

    def compilation_normalization_tests!
      frame = correction_test_frames.first
      evidence = { "fixture" => nil, "frames" => [frame] }
      original = Marshal.dump(evidence)
      normalized = compilation_comparable(evidence).first
      assert(original == Marshal.dump(evidence), "compilation comparison must not rewrite retained raw evidence")
      assert(normalized.fetch("result").fetch("canonical_torrent_public_id") == "<canonical_torrent:1>", "normalize only verified generated public identities")
      assert(normalized.fetch("tables_after").fetch("canonical_torrent").first.fetch("created_at") == "<transaction:0>", "normalize named generated transaction clocks")
      changed = Marshal.load(original)
      changed.fetch("frames").first.fetch("tables_after").fetch("canonical_torrent").first["title"] = frame.fetch("result").fetch("canonical_torrent_public_id")
      assert(compilation_comparable(changed).first.fetch("tables_after").fetch("canonical_torrent").first.fetch("title") == frame.fetch("result").fetch("canonical_torrent_public_id"), "UUID-shaped authored content must stay visible")
      changed.fetch("frames").first["result"] = { "logger" => frame.fetch("result").fetch("canonical_torrent_public_id") }
      assert(compilation_comparable(changed).first.fetch("result").fetch("logger") == frame.fetch("result").fetch("canonical_torrent_public_id"), "non-identity results must never normalize")
      changed = Marshal.load(original)
      changed.fetch("frames").first.fetch("tables_after").fetch("canonical_torrent").first["created_at"] = "2026-09-10T00:03:00+00:00"
      assert(compilation_comparable(changed).first.fetch("tables_after").fetch("canonical_torrent").first.fetch("created_at") == "2026-09-10T00:03:00+00:00", "unobserved timestamps must remain visible")
    end

    def compilation_logger_frame
      frame = correction_test_frames.first
      before = frame.fetch("tables_after")
      after = Marshal.load(Marshal.dump(before))
      after["source_metadata_conflict"] = [{
        "source_metadata_conflict_id" => 1, "canonical_torrent_source_id" => 2, "conflict_type" => "tracker_name",
        "existing_value" => "", "incoming_value" => "", "observed_at" => frame.fetch("clock"),
        "resolved_at" => nil, "resolved_by_user_id" => nil, "resolution" => nil, "resolution_note" => nil
      }]
      after["source_metadata_conflict_audit_log"] = [{
        "source_metadata_conflict_audit_log_id" => 1, "conflict_id" => 1, "action" => "created", "actor_user_id" => 0,
        "occurred_at" => frame.fetch("clock"), "note" => nil
      }]
      after["indexer_health_event"] = [{
        "indexer_health_event_id" => 1, "indexer_instance_id" => 569001, "occurred_at" => frame.fetch("clock"),
        "event_type" => "identity_conflict", "latency_ms" => nil, "http_status" => nil, "error_class" => nil, "detail" => "tracker_name"
      }]
      frame.merge("result" => { "logger" => "" }, "tables_before" => before, "tables_after" => after, "tables_finish" => after)
    end

    def compilation_logger_tests!
      operation = compilation_cases.fetch(2).fetch(:calls).first
      frame = compilation_logger_frame
      assert(compilation_logger_outcome?(operation, frame), "real source logger NULL fallback known answers")
      frame.fetch("tables_after").each do |table, rows|
        changed = Marshal.load(Marshal.dump(frame))
        changed.fetch("tables_after").fetch(table) << { "unexpected" => true }
        assert(!compilation_logger_outcome?(operation, changed), "logger must reject unrelated or extra #{table} rows")
        next unless %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].include?(table)

        rows.first.each_key do |key|
          changed = Marshal.load(Marshal.dump(frame))
          changed.fetch("tables_after").fetch(table).first[key] = "changed"
          assert(!compilation_logger_outcome?(operation, changed), "logger must reject changed #{table}.#{key}")
        end
      end
      assert(!compilation_logger_outcome?(operation, frame.merge("result" => { "logger" => nil })), "logger void result shape is mandatory")
      truncated = Marshal.load(Marshal.dump(frame))
      truncated.fetch("tables_after").fetch("source_metadata_conflict").first.merge!("conflict_type" => "tracker_category", "existing_value" => "A" * 256, "incoming_value" => "B" * 256, "observed_at" => "2026-09-10T00:02:00+00:00")
      truncated.fetch("tables_after").fetch("indexer_health_event").first.merge!("detail" => "tracker_category", "occurred_at" => "2026-09-10T00:02:00+00:00")
      operation = compilation_cases.fetch(2).fetch(:calls).fetch(1)
      assert(compilation_logger_outcome?(operation, truncated), "logger truncates both values but retains explicit observation time")
      truncated.fetch("tables_after").fetch("source_metadata_conflict").first["incoming_value"] = "B" * 257
      assert(!compilation_logger_outcome?(operation, truncated), "logger overlong value must fail")
    end

    def compilation_decision_tests!
      tables = correction_test_frames.first.fetch("tables_after")
      tables["search_request_source_observation"] = [{ "was_flagged" => true }]
      tables["search_filter_decision"] = [{
        "search_filter_decision_id" => 1, "search_request_id" => 588003,
        "policy_rule_public_id" => "58800000-0000-4000-8000-000000000005", "policy_snapshot_id" => 588002,
        "observation_id" => 1, "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1,
        "decision" => "flag", "decision_detail" => nil, "decided_at" => "2026-09-10T00:00:00+00:00"
      }]
      assert(compilation_decision?(tables), "policy cast must persist the exact flag decision and relationships")
      tables.fetch("search_filter_decision").first.each_key do |key|
        changed = Marshal.load(Marshal.dump(tables))
        changed.fetch("search_filter_decision").first[key] = "changed"
        assert(!compilation_decision?(changed), "changed policy decision #{key} must fail")
      end
      tables.fetch("search_request_source_observation").first["was_flagged"] = false
      assert(!compilation_decision?(tables), "persisted decision without flagged observation must fail")
    end
  end
end
