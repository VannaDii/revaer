# frozen_string_literal: true

require_relative "database-ingestion-validation-test"
require_relative "../database_rebaseline/ingestion_setting_paths"

module RevaerDatabaseRebaseline
  class IngestionSettingPathsTest < IngestionValidationTest
    include IngestionSettingPaths

    def run_tests!
      @assertions = 0
      @runtime = "validation_unit_runtime"
      @owner = "setting_unit_owner"
      source = File.binread(File.join(@contract.root, "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql"))
      body = source.split("CREATE OR REPLACE FUNCTION search_result_ingest_v1(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
      @ingestion_inventory = { "reference_proof" => { "routines" => [{ "name" => "search_result_ingest_v1", "signature" => "search_result_ingest_v1(uuid)", "source" => body }] } }
      setting_session_tests!
      setting_notice_tests!
      setting_validation_tests!
      setting_review_regressions!
      setting_bytes_tests!
      puts "database-ingestion-setting-paths-test: #{@assertions} assertions passed"
    end

    private

    def setting_session_tests!
      rejected("unknown setting") { setting_path_case("unknown", "cold") }
      rejected("unknown setting") { setting_path_case("success-rollback", "unknown") }
      assert(correction_records({}) == IngestionCorrections::CORRECTION_RECORDS, "ordinary protocol unchanged")
      assert(!correction_session(calls: [{}]).include?("finished_setting:"), "ordinary session unchanged")
      SETTING_PATH_NAMES.each do |name|
        %w[cold helpers-first].each do |mode|
          test_case = setting_path_case(name, mode)
          query = correction_session(test_case)
          count = name == "success-rollback" ? 3 : 2
          assert(query.scan("SELECT 'finished_setting:'").length == count, "every transaction exit is observed")
          assert(query.scan("ROLLBACK;").length == 1 && query.scan("COMMIT;").length == count - 1, "full rollback then same-backend commits")
          assert(query.scan("ROLLBACK TO SAVEPOINT operation;").length == count, "errors roll back the complete call")
          assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no backend repair or compiler GUC changes")
          assert(query.include?("helpers:") == (mode == "helpers-first"), "exact helper compilation mode")
          assert(correction_records(test_case).last == "finished_setting", "strict extra record order")
        end
      end
    end

    def setting_notice(event, role: "postgres")
      "NOTICE:  00000: ingestion-setting:#{JSON.generate(event)}\n" \
        "CONTEXT:  PL/pgSQL function ingestion_observation.trace_setting() line 3 at RAISE\n" \
        "#{setting_path_stack(role)}" \
        "LOCATION:  exec_stmt_raise, pl_exec.c:3897\n"
    end

    def setting_notice_tests!
      event = { "relation" => "canonical_torrent", "clock" => "2026-09-11T01:00:01+00:00" }
      notice = setting_notice(event)
      assert(setting_path_stack("postgres").end_with?("line 423 at SQL statement\n"), "frozen insertion coordinate independently fixed")
      assert(setting_path_stack(@runtime).end_with?("line 424 at SQL statement\n"), "only approved directive offset changes")
      assert(setting_path_notices("", role: "postgres") == [[], "", []], "empty plain diagnostics remain empty")
      error = "ERROR:  P0001: original error is retained for the exact application parser\n"
      expected = [[event, event], error, [["notice", event.fetch("clock")], ["error", "P0001"], ["notice", event.fetch("clock")]]]
      assert(setting_path_notices(notice + error + notice, role: "postgres") == expected, "combined ordering and original error bytes remain visible")
      ["NOTICE: unrelated\n", "WARNING: unexpected\n", notice + "extra\n", notice.sub("00000", "01000"),
       notice.sub("line 3", "line 4"), notice.sub("at RAISE", "at RETURN"), notice.sub("ingestion-setting:", "other:")].each do |mutated|
        rejected("notice") { setting_path_notices(mutated, role: "postgres") }
      end
      [notice.sub("INSERT INTO canonical_torrent", "INSERT INTO other"), notice.sub("line 423", "line 424"),
       notice.sub("canonical_public_id,", "unrelated_value,"), notice.sub("search_result_ingest_v1(uuid)", "search_result_ingest_v1(text)")].each do |mutated|
        rejected("call stack") { setting_path_notices(mutated, role: "postgres") }
      end
      rejected("JSON") { setting_path_notices(notice.sub(JSON.generate(event), "{broken}"), role: "postgres") }
    end

    def setting_error_evidence(mode, variant, observed:)
      role = variant == "reference" ? "postgres" : @runtime
      test_case = validation_case("negative-year", :year, validation_attributes("year", :int, { int: "-1" }))
      evidence = validation_error_fixture(test_case, mode, role)
      inputs = evidence.fetch("inputs_before")
      clock = evidence.fetch("seed_clock")
      inputs["trust_tier"] = [["public", 10], ["semi_private", 20], ["private", 30], ["invite_only", 40]].map do |key, rank|
        { "trust_tier_key" => key, "rank" => rank, "created_at" => clock }
      end
      inputs["media_domain"] = %w[adult_movies adult_scenes audiobooks ebooks movies software tv].map { |key| { "media_domain_key" => key, "created_at" => clock } }
      evidence["inputs_after"] = copy(inputs)
      evidence.fetch("frames").each { |frame| frame["finished_setting"] = "error" }
      lines = mode == "helpers-first" ? ["helpers:#{JSON.generate(ingestion_helper_expectations)}"] : []
      evidence.fetch("frames").each do |frame|
        correction_records(finish_setting: true).each do |key|
          value = frame.fetch(key)
          value = JSON.generate(value) if %w[role clock tables_before tables_after tables_finish].include?(key)
          lines << "#{key}:#{value}"
        end
      end
      evidence["stdout"] = lines.join("\n") + "\n"
      events = observed ? evidence.fetch("frames").map do |frame|
        { "relation" => "canonical_torrent", "operation" => "INSERT", "backend" => frame.fetch("backend"),
          "session" => role, "current" => variant == "reference" ? "postgres" : @owner,
          "setting" => variant == "reference" ? "use_column" : "error", "clock" => frame.fetch("clock") }
      end : []
      records = evidence.fetch("stderr").split(/(?=^ERROR:  )/)
      evidence["stderr"] = records.each_with_index.map { |record, index| (observed ? setting_notice(events.fetch(index), role:) : "") + record }.join
      evidence["events"] = events
      evidence
    end

    def setting_validation_tests!
      %w[cold helpers-first].each do |mode|
        test_case = setting_path_case("late-validation-error", mode)
        %w[reference final].each do |variant|
          role = variant == "reference" ? "postgres" : @runtime
          [false, true].each do |observed|
            evidence = setting_error_evidence(mode, variant, observed:)
            assert(setting_path_validate!(evidence, test_case, variant, role, observed:), "complete #{variant} #{mode} evidence accepted")
            changed = copy(evidence)
            changed.fetch("frames").first["finished_setting"] = "use_column"
            changed["stdout"] = changed.fetch("stdout").sub("finished_setting:error", "finished_setting:use_column")
            rejected("caller") { setting_path_validate!(changed, test_case, variant, role, observed:) }
            changed = copy(evidence)
            changed["stdout"] = changed.fetch("stdout").sub("finished_setting:error\n", "")
            rejected("record") { setting_path_parse(changed, test_case, role) }
            changed = copy(evidence)
            changed.fetch("inputs_after").fetch("search_request").first["page_size"] = 11
            rejected("read inputs") { setting_path_validate!(changed, test_case, variant, role, observed:) }
            IngestionProof::INGESTION_TABLES.each do |table|
              changed = copy(evidence)
              changed.fetch("after").fetch(table) << { "unexpected" => true }
              assert(!setting_path_outcomes?(changed, test_case, variant, role), "reject unintended #{table} writes")
            end
            next unless observed

            events = evidence.fetch("events")
            [[], events.reverse, events + events.take(1)].each do |mutation|
              assert(!setting_path_events?(evidence.fetch("frames"), mutation, variant, role, observed:), "reject missing, reordered or duplicated events")
            end
            events.first.each_key do |key|
              changed = copy(events)
              changed.first[key] = "changed"
              assert(!setting_path_events?(evidence.fetch("frames"), changed, variant, role, observed:), "reject changed event #{key}")
            end
          end
          plain = setting_error_evidence(mode, variant, observed: false)
          observed = setting_error_evidence(mode, variant, observed: true)
          assert(setting_path_comparable(plain) == setting_path_comparable(observed), "observer-only metadata is not application behavior")
        end
      end
    end

    def setting_bytes_tests!
      Dir.mktmpdir("revaer-setting-bytes.") do |directory|
        @setting_path_evidence = directory
        evidence = setting_error_evidence("cold", "final", observed: true)
        bytes = JSON.pretty_generate(evidence) + "\n"
        path = setting_path_write("fixture.json", bytes)
        assert((File.stat(path).mode & 0o777) == 0o600, "private evidence permissions")
        registry = { path => Digest::SHA256.hexdigest(bytes) }
        assert(dependency_read_observation(path, registry).first == evidence, "explicit synthetic registry checks original bytes")
        File.binwrite(path, bytes + " ")
        rejected("bytes changed") { dependency_read_observation(path, registry) }
        rejected("invalid setting") { setting_path_write("../escape", bytes) }
        begin
          setting_path_write("fixture.json", bytes)
          raise Failure, "existing evidence was overwritten"
        rescue Errno::EEXIST
          assert(true, "existing evidence cannot be overwritten")
        end
      end
    end

    def setting_review_regressions!
      test_case = setting_path_case("late-validation-error", "cold")
      evidence = setting_error_evidence("cold", "final", observed: true)
      records = evidence.fetch("stderr").split(/(?=^(?:NOTICE|ERROR):  )/)
      [[1, 0, 3, 2], [1, 3, 0, 2], [0, 2, 1, 3]].each do |order|
        changed = evidence.merge("stderr" => order.map { |index| records.fetch(index) }.join)
        rejected("ordering") { setting_path_validate!(changed, test_case, "final", @runtime, observed: true) }
      end
      empty = IngestionPolicy::POLICY_READ_TABLES.to_h { |table| [table, []] }
      erased = evidence.merge("inputs_before" => empty, "inputs_after" => empty)
      assert(!setting_path_inputs?(erased, test_case), "matching empty read inputs cannot certify a seeded case")
      evidence.fetch("inputs_before").each do |table, rows|
        next if rows.empty?

        changed = copy(evidence)
        %w[inputs_before inputs_after].each { |key| changed.fetch(key)[table] = [] }
        assert(!setting_path_inputs?(changed, test_case), "missing seeded #{table} cannot pass equality")
      end
      changed = copy(evidence)
      %w[inputs_before inputs_after].each { |key| changed.fetch(key).fetch("search_request").first["created_at"] = "changed" }
      assert(!setting_path_inputs?(changed, test_case), "coherent seed-clock drift cannot normalize away")
      regex_inputs = setting_regex_test_inputs
      assert(setting_path_regex_inputs?(regex_inputs), "independent selected title-rule fixture")
      %w[search_request policy_rule policy_snapshot policy_snapshot_rule policy_set search_request_indexer_run policy_rule_value_set_item].each do |table|
        changed = copy(regex_inputs)
        changed[table] = []
        assert(!setting_path_regex_inputs?(changed), "missing regex prerequisite #{table} rejected")
      end
      %w[match_field match_operator match_value_text is_case_insensitive is_disabled expires_at].each do |key|
        changed = copy(regex_inputs)
        changed.fetch("policy_rule").first[key] = "changed"
        assert(!setting_path_regex_inputs?(changed), "selected rule #{key} must match the intended branch")
      end
      diagnostic = { "state" => "2201B", "message" => "invalid regular expression: brackets [] not balanced",
                     "line" => 1278, "helper_line" => 14, "location" => "RE_compile_and_cache, regexp.c:222" }
      regex_evidence = copy(evidence)
      regex_evidence.fetch("frames").each { |frame| frame.merge!("state" => "2201B", "diagnostic" => copy(diagnostic)) }
      regex_case = setting_path_case("policy-regex-error", "cold")
      assert(setting_path_outcomes?(regex_evidence, regex_case, "final", @runtime), "exact title regex diagnostic accepted")
      regex_evidence.fetch("frames").each { |frame| frame.fetch("diagnostic")["line"] = 1254 }
      assert(!setting_path_outcomes?(regex_evidence, regex_case, "final", @runtime), "valid infohash callsite cannot stand in for the selected title rule")
    end

    def setting_regex_test_inputs
      rule_keys = %w[policy_rule_public_id policy_rule_id policy_set_id rule_type match_field match_operator match_value_text match_value_int match_value_uuid value_set_id action severity is_case_insensitive is_disabled expires_at]
      rule = rule_keys.zip(["59600000-0000-4000-8000-000000001001", 1, 596004, "block_title_regex", "title", "regex", "[", nil, nil, nil, "flag", "soft", true, false, nil]).to_h
      items = ["a" * 40, "b" * 64, "c" * 64, "policy", "group", "uploader", "proof-tracker", "movies", "public"].map { |value| { "value_set_id" => 596201, "value_text" => value } }
      items += [{ "value_set_id" => 596204, "value_text" => "not-a-match" },
                { "value_set_id" => 596202, "value_uuid" => "56900000-0000-4000-8000-000000000001" },
                { "value_set_id" => 596205, "value_uuid" => "59600000-0000-4000-8000-000000000099" },
                { "value_set_id" => 596203, "value_int" => 10 }, { "value_set_id" => 596206, "value_int" => 99 }]
      { "search_request" => [{ "search_request_id" => 596001, "search_request_public_id" => "59600000-0000-4000-8000-000000000012",
          "policy_snapshot_id" => 596001, "search_profile_id" => 596001, "status" => "running", "query_text" => "D3 populated policy", "page_size" => 10 }],
        "policy_rule" => [rule], "policy_snapshot" => [{ "policy_snapshot_id" => 596001, "snapshot_hash" => "d" * 64 }],
        "policy_snapshot_rule" => [{ "policy_snapshot_id" => 596001, "policy_rule_public_id" => "59600000-0000-4000-8000-000000001001", "rule_order" => 10 }],
        "policy_set" => (1..4).zip(%w[request profile user global]).map { |id, scope| { "policy_set_id" => 596000 + id, "scope" => scope, "is_enabled" => true } },
        "search_request_indexer_run" => [569001, 596001].map { |id| { "search_request_id" => id, "indexer_instance_id" => 569001, "status" => "queued" } },
        "policy_rule_value_set_item" => items }
    end
  end
end

RevaerDatabaseRebaseline::IngestionSettingPathsTest.new.run_tests! if $PROGRAM_NAME == __FILE__
