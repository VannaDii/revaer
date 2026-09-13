# frozen_string_literal: true

require "tmpdir"
require_relative "database-ingestion-proof-live"
require_relative "../database_rebaseline/ingestion_policy"

module RevaerDatabaseRebaseline
  module IngestionPolicyTest
    private

    def policy_tests!
      previous_inventory = @ingestion_inventory
      cases = policy_cases
      assert(cases.length == 53 && cases.map { |entry| entry.fetch(:name) }.uniq.length == 53, "exact populated policy matrix")
      assert(cases.first.fetch(:rules).map { |rule| rule.fetch(:field) } == IngestionPolicy::POLICY_FIELDS.keys, "all actual match fields")
      assert(IngestionPolicy::POLICY_REQUIRE.length == 5, "all five require families")
      assert(cases.count { |entry| entry.fetch(:name).start_with?("scope-") } == 8, "every require precedence level passes and fails")
      query = correction_session(calls: [{}, {}, {}], rollback: true)
      assert(query.scan("BEGIN;").length == 3 && query.scan("ROLLBACK;").length == 1 && query.scan("COMMIT;").length == 2, "warm-after-rollback and committed reuse are distinct")
      assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no backend repairs or new compiler privilege")
      assert(policy_helpers_sql.scan("public.policy_release_group_match_v1(1,").length == 4, "real persisted-signal helper paths")
      assert(policy_arguments(token: "GROUP").fetch(:title_raw_input).include?("GROUP\\'"), "literal frozen suffix regex input is explicit")
      rejected("unsupported policy fixture") { policy_sql_value([]) }
      %w[true false].each do |value|
        assert(policy_sql_value(value == "true") == value, "typed boolean fixture input")
      end
      assert(policy_sql_value(nil) == "NULL" && policy_sql_value("a'b") == "'a''b'", "typed NULL and quoted fixture input")
      policy_model_tests!(cases)
      policy_total_boundary_tests!(cases)
      policy_error_tests!
      policy_input_normalization_tests!
      policy_cleanup_test!
    ensure
      @ingestion_inventory = previous_inventory
    end

    def policy_model_tests!(cases)
      cases.each do |test_case|
        statements = policy_rules_sql(test_case)
        assert(test_case.fetch(:rules).all? { |rule| statements.include?(policy_rule_uuid(rule.fetch(:id))) }, "every #{test_case.fetch(:name)} rule enters snapshot")
      end
      test_case = cases.first
      before = policy_test_tables
      clock = "2026-09-11T01:00:00+00:00"
      frame = policy_test_frame(before, clock)
      observation = frame.fetch("tables_after").fetch("search_request_source_observation").last
      frame.fetch("tables_after")["search_filter_decision"] = test_case.fetch(:decisions).map.with_index do |(id, decision), index|
        { "search_filter_decision_id" => index + 1, "search_request_id" => 596001,
          "policy_rule_public_id" => policy_rule_uuid(id), "policy_snapshot_id" => 596001,
          "observation_id" => observation.fetch("observation_id"), "canonical_torrent_id" => 1,
          "canonical_torrent_source_id" => 1, "decision" => decision, "decision_detail" => nil, "decided_at" => clock }
      end
      assert(policy_decisions?(test_case, frame, observation), "complete logged action cast rows")
      frame.fetch("tables_after").fetch("search_filter_decision").first.each_key do |key|
        changed = policy_copy(frame)
        changed.fetch("tables_after").fetch("search_filter_decision").first[key] = "changed"
        assert(!policy_decisions?(test_case, changed, observation), "reject changed decision #{key}")
      end
      [-1, 0, 1].each do |delta|
        changed = policy_copy(frame)
        rows = changed.fetch("tables_after").fetch("search_filter_decision")
        delta.negative? ? rows.pop : rows << rows.first.dup
        assert(!policy_decisions?(test_case, changed, observation), "reject missing duplicate extra decisions #{delta}")
      end
      original = policy_copy(frame)
      frame.fetch("tables_after").fetch("search_filter_decision").reverse!
      assert(policy_decisions?(test_case, frame, observation), "unordered INSERT SELECT comparison retains a complete multiset")
      assert(policy_decisions?(test_case, original, observation), "decision validator does not mutate evidence")
      @ingestion_inventory = { "reference_proof" => { "routines" => [{ "name" => "search_result_ingest_v1",
        "signature" => "search_result_ingest_v1(uuid)", "source" => "\nCREATE TEMP TABLE tmp_policy_rules AS\nSELECT 1;\n" }] } }
      %w[reference final].each do |variant|
        frames = Array.new(3) { policy_copy(frame) }
        frames.last.delete("result") if variant == "reference"
        if variant == "reference"
          frames.last.merge!("state" => "42P07", "diagnostic" => policy_d4_diagnostic(policy_d4_expected))
        end
        assert(policy_states?(test_case, frames, variant), "exact #{variant} successful rollback then committed reuse outcomes")
        frames.last["state"] = "42501"
        assert(!policy_states?(test_case, frames, variant), "permission error is never accepted as D4")
        lifetimes = Array.new(3) { |index| { "within" => "true", "outside" => (variant == "reference" && index.positive?).to_s } }
        assert(policy_lifetime?(test_case, lifetimes, variant), "#{variant} natural rollback and commit temp lifetime")
        %w[within outside].each do |key|
          lifetimes.each_index do |index|
            changed = policy_copy(lifetimes)
            changed[index][key] = "changed"
            assert(!policy_lifetime?(test_case, changed, variant), "reject lifetime mutation #{variant} #{index} #{key}")
          end
        end
      end
      ["WARNING: extra\n", "NOTICE: extra\n", "DETAIL: extra\n"].each do |extra|
        rejected("exact frozen statement") { policy_d4_diagnostic(policy_d4_expected + extra) }
      end
      ["SELECT 1", "search_result_ingest_v1", "createas.c:406", "42P07"].each do |fragment|
        rejected("exact frozen statement") { policy_d4_diagnostic(policy_d4_expected.sub(fragment, "changed")) }
      end
      policy_score_tests!(cases, before)
    end

    def policy_test_tables
      tables = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      tables["canonical_torrent"] = [{ "canonical_torrent_id" => 1, "canonical_torrent_public_id" => "59600000-0000-4000-8000-000000000090" }]
      tables["canonical_torrent_source"] = [{ "canonical_torrent_source_id" => 1, "canonical_torrent_source_public_id" => "59600000-0000-4000-8000-000000000091" }]
      tables
    end

    def policy_test_frame(before, clock)
      after = policy_copy(before)
      after["search_request_source_observation"] = [{ "observation_id" => 2, "search_request_id" => 596001,
                                                    "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1,
                                                    "was_flagged" => true, "was_downranked" => false }]
      { "state" => "00000", "tables_before" => before, "tables_after" => after, "clock" => clock, "result" => {} }
    end

    def policy_score_tests!(cases, tables)
      cases.reject { |test_case| test_case[:error] }.each do |test_case|
        tag = test_case.fetch(:tag).clamp(-15, 15)
        total = test_case.fetch(:total) { test_case.fetch(:drop) ? -10000 : 37 + test_case.fetch(:adjust) + tag }
        rows = [{ "context_key_id" => 596001, "context_key_type" => "search_request", "canonical_torrent_id" => 1,
                  "canonical_torrent_source_id" => 1, "score_total_context" => total,
                  "score_policy_adjust" => test_case.fetch(:adjust), "score_tag_adjust" => test_case.fetch(:drop) ? 0 : tag,
                  "is_dropped" => test_case.fetch(:drop) }]
        images = tables.merge("canonical_torrent_source_context_score" => rows)
        canonical = tables.fetch("canonical_torrent").first
        source = tables.fetch("canonical_torrent_source").first
        assert(policy_score?(test_case, images, canonical, source), "known score #{test_case.fetch(:name)}")
        rows.first.each_key do |column|
          changed = policy_copy(images)
          changed.fetch("canonical_torrent_source_context_score").first[column] = "changed"
          assert(!policy_score?(test_case, changed, canonical, source), "reject changed score #{column}")
        end
      end
    end

    def policy_total_boundary_tests!(cases)
      expected = {
        "total-score-below-minimum" => [-10000, -10, 9, -10001, -10000],
        "total-score-exact-minimum" => [-10000, -10, 10, -10000, -10000],
        "total-score-above-maximum" => [10000, 15, -14, 10001, 10000],
        "total-score-exact-maximum" => [10000, 15, -15, 10000, 10000]
      }
      boundaries = cases.select { |test_case| test_case.key?(:total) }
      assert(boundaries.map { |test_case| test_case.fetch(:name) } == expected.keys, "four literal total-score clamp and equality controls")
      schema = File.read(File.join(@contract.root, "crates/revaer-data/migrations/0024_indexer_scoring.sql"))
      assert(schema.include?("CHECK (score_total_base BETWEEN -10000 AND 10000)"), "out-of-range base fixtures are constraint-unreachable, not total-clamp proof")
      boundaries.each do |test_case|
        base, adjust, tag, raw, total = expected.fetch(test_case.fetch(:name))
        assert(test_case.values_at(:base, :adjust, :tag, :total, :drop) == [base, adjust, tag, total, false], "independent bounded non-drop inputs")
        assert((-10000..10000).cover?(base) && (-15..15).cover?(tag) && base + adjust + tag == raw, "valid base and unclamped tag reach literal total")
        assert(test_case.fetch(:rules).none? { |rule| rule.fetch(:action).start_with?("drop_") }, "drop sentinel never substitutes for a total clamp")
        row = policy_base_score(test_case)
        assert(row == { "canonical_torrent_source_base_score_id" => 1, "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1,
          "score_total_base" => base, "score_seed" => 0, "score_leech" => 0, "score_age" => 0, "score_trust" => 0,
          "score_health" => 0, "score_reputation" => 0, "computed_at" => "2026-09-11T00:00:00+00:00" }, "complete persisted base-score input")
        3.times do |index|
          frame = policy_total_boundary_frame(test_case, index, total)
          assert(policy_success?(test_case, frame, index), "literal boundary complete write state including rollback sequence gaps #{index}")
          image = frame.fetch("tables_after")
          [-10001, -9999, 9999, 10001, 0].each do |wrong|
            changed = policy_copy(frame)
            changed.fetch("tables_after").fetch("canonical_torrent_source_context_score").last["score_total_context"] = wrong
            assert(!policy_success?(test_case, changed, index), "reject unclamped or off-boundary total #{wrong}")
          end
          IngestionProof::INGESTION_TABLES.each do |table|
            changed = policy_copy(frame)
            changed.fetch("tables_after").fetch(table) << { "unexpected" => true }
            assert(!policy_write_images?(test_case, changed, index), "reject unexpected complete boundary image #{table}") unless table == "search_filter_decision"
            image.fetch(table).each_with_index do |actual, row_index|
              actual.each_key do |column|
                next if table == "search_filter_decision"

                changed = policy_copy(frame)
                changed.fetch("tables_after").fetch(table)[row_index][column] = "changed"
                assert(!policy_write_images?(test_case, changed, index), "reject boundary #{table}.#{column}")
              end
            end
          end
          changed = policy_copy(frame)
          changed.fetch("tables_after").fetch("canonical_torrent_best_source_context") << {
            "context_key_id" => 596001, "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1 }
          assert(!policy_success?(test_case, changed, index), "admitted v1 boundary cannot invent wrapper best-source selection")
          changed.fetch("tables_after")["canonical_torrent_best_source_context"] = [{ "unexpected" => true }]
          assert(!policy_success?(test_case, changed, index), "malformed unselected-source evidence fails closed")
          changed = policy_copy(frame)
          changed.fetch("tables_after").fetch("canonical_torrent_source_context_score").last["is_dropped"] = true
          assert(!policy_success?(test_case, changed, index), "negative clamp is not the drop sentinel")
        end
      end
    end

    def policy_total_boundary_frame(test_case, index, total)
      before = policy_test_tables
      before["search_request_source_observation"] = [{ "observation_id" => 1, "search_request_id" => 569001,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "title_raw" => "Policy.1080p-GROUP\\",
        "was_flagged" => false, "was_downranked" => false }]
      before = policy_total_boundary_frame(test_case, 1, total).fetch("tables_after") if index == 2
      after = policy_copy(before)
      clock = "2026-09-11T01:0#{index}:00+00:00"
      %w[canonical_torrent canonical_torrent_source].each { |table| after.fetch(table).first["updated_at"] = clock }
      identity = index.zero? ? 2 : 3
      after["canonical_torrent_source_context_score"] = [{ "canonical_torrent_source_context_score_id" => identity,
        "context_key_type" => "search_request", "context_key_id" => 596001, "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1,
        "score_total_context" => total, "score_policy_adjust" => test_case.fetch(:adjust), "score_tag_adjust" => test_case.fetch(:tag),
        "is_dropped" => false, "computed_at" => clock }]
      if index < 2
        after.fetch("search_request_source_observation") << { "observation_id" => identity, "search_request_id" => 596001,
          "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "title_raw" => "Policy.1080p",
          "was_flagged" => false, "was_downranked" => test_case.fetch(:downrank) }
        after["search_request_canonical"] = [{ "search_request_canonical_id" => identity, "search_request_id" => 596001, "canonical_torrent_id" => 1, "first_seen_at" => clock }]
        after["search_page"] = [{ "search_page_id" => identity, "search_request_id" => 596001, "page_number" => 1, "sealed_at" => nil }]
        after["search_page_item"] = [{ "search_page_item_id" => identity, "search_page_id" => identity, "search_request_canonical_id" => identity, "position" => 1 }]
      end
      test_case.fetch(:decisions).each do |id, decision|
        after.fetch("search_filter_decision") << { "search_filter_decision_id" => index + 1, "search_request_id" => 596001,
          "policy_rule_public_id" => policy_rule_uuid(id), "policy_snapshot_id" => 596001, "observation_id" => identity,
          "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "decision" => decision, "decision_detail" => nil, "decided_at" => clock }
      end
      { "tables_before" => before, "tables_after" => after, "clock" => clock, "result" => {
        "canonical_torrent_public_id" => before.fetch("canonical_torrent").first.fetch("canonical_torrent_public_id"),
        "canonical_torrent_source_public_id" => before.fetch("canonical_torrent_source").first.fetch("canonical_torrent_source_public_id"),
        "canonical_changed" => false, "durable_source_created" => false, "observation_created" => index < 2 } }
    end

    def policy_error_tests!
      @ingestion_inventory = { "reference_proof" => { "routines" => [
        { "name" => "search_result_ingest_v1", "signature" => "search_result_ingest_v1(uuid)", "source" => "\nrule_matched := policy_text_match_v1(\n" },
        { "name" => "policy_text_match_v1", "source" => "\nRETURN candidate_input ~* match_value_text_input;\n" }
      ] } }
      diagnostic = "ERROR:  2201B: invalid regular expression: brackets [] not balanced\n" \
                   "CONTEXT:  PL/pgSQL function policy_text_match_v1(text,policy_match_operator,text,bigint,boolean) line 2 at RETURN\n" \
                   "PL/pgSQL function search_result_ingest_v1(uuid) line 2 at assignment\n" \
                   "LOCATION:  RE_compile_and_cache, regexp.c:222\n"
      expected = policy_regex_diagnostic(diagnostic, "postgres")
      assert(expected.fetch("state") == "2201B", "nested invalid regex diagnostic retained")
      assert(expected == policy_regex_diagnostic(diagnostic.sub("(uuid) line 2", "(uuid) line 3"), @runtime), "only exact D3 line displacement normalized")
      ["WARNING: extra\n", "NOTICE: extra\n", "DETAIL: hidden\n", diagnostic].each do |suffix|
        rejected("unrecognized policy regex") { policy_regex_diagnostic(diagnostic + suffix, "postgres") }
      end
      rejected("unrecognized policy regex") { policy_regex_diagnostic(diagnostic.sub("2201B", "42501"), "postgres") }
      rejected("outside exact routines") { policy_regex_diagnostic(diagnostic.sub("line 2 at assignment", "line 3 at assignment"), "postgres") }
      rejected("outside exact routines") { policy_regex_diagnostic(diagnostic.sub("line 2 at RETURN", "line 3 at RETURN"), "postgres") }
      rejected("outside exact routines") { policy_regex_diagnostic(diagnostic.sub("(uuid)", "(text)"), "postgres") }
      rejected("read-input inventory") { policy_comparable_inputs("inputs_before" => {}, "seed_clocks" => {}) }
    end

    def policy_cleanup_test!
      commands = []
      runner = Object.new
      runner.define_singleton_method(:capture) do |_command, stdin_data:|
        commands << stdin_data
        failed = stdin_data.include?("-- Disposable")
        CommandRunner::Result.new(stdout: "", stderr: failed ? "injected fixture error" : "", success: !failed)
      end
      previous_runner = @runner
      previous_contract = @contract
      @runner = runner
      Dir.mktmpdir("revaer-policy-test.") do |directory|
        @policy_evidence = directory
        @contract = Struct.new(:output_path, :root).new(directory, File.expand_path("../..", __dir__))
        rejected("policy fixture setup failed") do
          policy_isolated(policy_cases.first, "reference", "reference_proof", "postgres", mode: "cold")
        end
        assert(commands.last == 'DROP DATABASE "ingestion_policy_reference" WITH (FORCE)', "fixture failure drops owned clone with forced session cleanup")
      end
    ensure
      @runner = previous_runner
      @contract = previous_contract
    end

    def policy_input_normalization_tests!
      base = "2026-09-11T00:01:00+00:00"
      policy = "2026-09-11T00:02:00+00:00"
      rows = [{ "created_at" => base, "updated_at" => policy, "display_name" => base, "expires_at" => policy },
              { "created_at" => "unobserved", "updated_at" => nil }]
      inputs = IngestionPolicy::POLICY_READ_TABLES.to_h { |table| [table, []] }.merge("policy_rule" => rows)
      evidence = { "inputs_before" => inputs, "seed_clocks" => { "base" => base, "policy" => policy, "rules" => "2026-09-11T00:03:00+00:00" } }
      original = Marshal.dump(evidence)
      normalized = policy_comparable_inputs(evidence).fetch("policy_rule")
      assert(normalized.include?(rows.last), "unknown and NULL audit timestamps remain visible")
      expected = { "created_at" => "<validated-base-seed-transaction>", "updated_at" => "<validated-policy-seed-transaction>",
                   "display_name" => base, "expires_at" => policy }
      assert(normalized.include?(expected), "normalize only named metadata matching a recorded seed transaction")
      assert(Marshal.dump(evidence) == original, "retain raw seed clocks and full read rows")
    end

    def policy_copy(value)
      Marshal.load(Marshal.dump(value))
    end

    def policy_live_mutations!(test_case, evidence, variant, role, mode:)
      assert(policy_states?(test_case, evidence.fetch("frames"), variant), "replay exact states")
      assert(policy_context?(evidence, role, mode:), "replay direct context")
      assert(policy_images?(evidence), "replay complete images")
      assert(policy_inputs?(evidence, test_case), "replay pinned inputs")
      assert(policy_lifetime?(test_case, evidence.fetch("frames"), variant), "replay temp lifetime")
      assert(policy_outcomes?(test_case, evidence), "replay exact application outcomes")
      policy_context_mutations!(evidence, role, mode:)
      policy_input_mutations!(test_case, evidence)
      policy_image_mutations!(test_case, evidence)
      original = Marshal.dump(evidence)
      comparable = policy_comparable(evidence, count: 2)
      assert(Marshal.dump(evidence) == original, "comparison preserves raw evidence")
      changed = policy_copy(evidence)
      changed.fetch("frames").first.fetch("tables_after").fetch("canonical_torrent").first["title_display"] = "unexpected"
      assert(policy_comparable(changed, count: 2) != comparable, "arbitrary text is not normalized away")
      changed = policy_copy(evidence)
      changed.fetch("frames").first.fetch("tables_after").fetch("canonical_torrent").first["updated_at"] = "2026-01-01T00:00:00+00:00"
      assert(policy_comparable(changed, count: 2) != comparable, "unobserved timestamp is not normalized away")
      frames = policy_copy(evidence.fetch("frames"))
      frames.last["state"] = "00001"
      assert(!policy_states?(test_case, frames, variant), "unapproved error cannot pass")
    end

    def policy_context_mutations!(evidence, role, mode:)
      evidence.fetch("frames").each_index do |index|
        %w[before after backend clock].each do |key|
          changed = policy_copy(evidence)
          changed.fetch("frames")[index][key] = key == "clock" ? evidence.fetch("fixture").fetch("clock") : "changed"
          assert(!policy_context?(changed, role, mode:), "reject #{index} context #{key}")
        end
        evidence.fetch("frames")[index].fetch("role").each_key do |key|
          changed = policy_copy(evidence)
          changed.fetch("frames")[index].fetch("role")[key] = "changed"
          assert(!policy_context?(changed, role, mode:), "reject #{index} authority #{key}")
        end
      end
      changed = policy_copy(evidence)
      changed.fetch("fixture")["backend"] = evidence.fetch("frames").first.fetch("backend")
      assert(!policy_context?(changed, role, mode:), "fixture must not warm the tested connection")
      if mode == "helpers-first"
        evidence.fetch("helpers").fetch("answers").each_key do |key|
          changed = policy_copy(evidence)
          changed.fetch("helpers").fetch("answers")[key] = []
          assert(!policy_context?(changed, role, mode:), "reject missing populated helper #{key}")
        end
      end
      changed = policy_copy(evidence)
      changed["helpers"] = mode == "helpers-first" ? nil : {}
      assert(!policy_context?(changed, role, mode:), "helper evidence cannot move between cold and helper-first modes")
    end

    def policy_input_mutations!(test_case, evidence)
      evidence.fetch("inputs_before").each_key do |table|
        changed = policy_copy(evidence)
        changed.fetch("inputs_after").delete(table)
        assert(!policy_inputs?(changed, test_case), "reject missing read input #{table}")
      end
      %w[policy_rule policy_snapshot_rule policy_set policy_rule_value_set_item canonical_torrent_source_base_score search_profile_tag_prefer].each do |table|
        changed = policy_copy(evidence)
        rows = changed.fetch("inputs_before").fetch(table)
        row = table == "policy_rule" ? rows.find { |item| item.fetch("policy_rule_public_id") == policy_rule_uuid(1) } : rows.last
        column = { "policy_rule" => "action", "policy_snapshot_rule" => "rule_order", "policy_set" => "scope",
                   "policy_rule_value_set_item" => "value_text", "canonical_torrent_source_base_score" => "score_total_base",
                   "search_profile_tag_prefer" => "weight_override" }.fetch(table)
        row[column] = "changed"
        changed["inputs_after"] = policy_copy(changed.fetch("inputs_before"))
        changed["inputs_sha256"] = Digest::SHA256.hexdigest(JSON.generate(changed.fetch("inputs_before")))
        assert(!policy_inputs?(changed, test_case), "reject consistently changed fixture #{table}.#{column}")
      end
    end

    def policy_image_mutations!(test_case, evidence)
      evidence.fetch("frames").each_with_index do |frame, index|
        %w[tables_before tables_after tables_finish].each do |key|
          IngestionProof::INGESTION_TABLES.each do |table|
            changed = policy_copy(evidence)
            changed.fetch("frames")[index].fetch(key).delete(table)
            assert(!policy_images?(changed), "reject missing #{index} #{key} #{table}")
          end
        end
        frame.fetch("tables_after").each do |table, rows|
          changed = policy_copy(evidence)
          changed.fetch("frames")[index].fetch("tables_after").fetch(table) << (rows.empty? ? { "unexpected" => true } : policy_copy(rows.first))
          assert(!policy_outcomes?(test_case, changed), "reject extra application row #{index} #{table}")
          next unless frame.fetch("state") == "00000" && table != "search_filter_decision" && !rows.empty?

          rows.last.each_key do |column|
            changed = policy_copy(evidence)
            changed.fetch("frames")[index].fetch("tables_after").fetch(table).last[column] = "changed"
            assert(!policy_write_images?(test_case, changed.fetch("frames")[index], index), "reject full-image column #{index} #{table}.#{column}")
          end
        end
      end
    end
  end

  class IngestionPolicyHarness < FinalProof
    include IngestionPolicy
    include IngestionPolicyTest

    def run_tests!
      @assertions = 0
      policy_tests!
      puts "database-ingestion-policy-test: #{@assertions} assertions passed"
    end

    def replay!(directory)
      @assertions = 0
      report = JSON.parse(File.binread(File.join(directory, "report.json")))
      raise Failure, "policy replay requires a complete passing matrix, never an incomplete run" unless report.fetch("completed") && report.fetch("passed") && !report.fetch("d3_complete")

      @runtime = report.fetch("roles").fetch("final")
      @owner = report.fetch("roles").fetch("owner")
      @ingestion_inventory = JSON.parse(File.binread(File.join(directory, "routine-inventory.json")))
      raise Failure, "unexpected retained policy role" unless @runtime.match?(/\Aproof_runtime_[a-f0-9]{16}\z/)

      policy_cases.each do |test_case|
        %w[cold helpers-first].each do |mode|
          variants = %w[reference final].to_h do |variant|
            path = File.join(directory, "#{test_case.fetch(:name)}-#{mode}-#{variant}.json")
            evidence = JSON.parse(File.binread(path))
            role = report.fetch("roles").fetch(variant)
            policy_live_mutations!(test_case, evidence, variant, role, mode:)
            [variant, evidence]
          end
          assert(policy_comparable(variants.fetch("reference"), count: 2) == policy_comparable(variants.fetch("final"), count: 2), "replayed frozen/final parity")
        end
      end
      puts "database-ingestion-policy-test: #{@assertions} live-evidence mutation assertions passed"
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
  end

  # Reuse FinalProof provisioning, byte verification, direct roles and guaranteed
  # unique-container teardown. This callback selects policy-only local evidence.
  class IngestionPolicyLive < IngestionProofLive
    include IngestionPolicy

    private

    def verify_ingestion_corrections!
      @ingestion_evidence = File.join(@contract.output_path, "ingestion-policy-inventory")
      FileUtils.mkdir_p(@ingestion_evidence, mode: 0o700)
      @ingestion_inventory = {}
      ingestion_inventory!
      verify_ingestion_policy!
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    if ARGV.empty?
      RevaerDatabaseRebaseline::IngestionPolicyHarness.new.run_tests!
    elsif ARGV.length == 2 && ARGV.first == "--live"
      RevaerDatabaseRebaseline::IngestionPolicyLive.new.run_ingestion!(ARGV.fetch(1), corrections_only: true)
    elsif ARGV.length == 2 && ARGV.first == "--replay"
      RevaerDatabaseRebaseline::IngestionPolicyHarness.new.replay!(ARGV.fetch(1))
    else
      raise RevaerDatabaseRebaseline::Failure, "usage: database-ingestion-policy-test.rb [--live PINNED_CANDIDATE | --replay EVIDENCE_DIRECTORY]"
    end
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-policy-test: #{error.message}"
    exit 1
  end
end
