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
      assert(cases.length == 69 && cases.map { |entry| entry.fetch(:name) }.uniq.length == 69, "exact populated policy matrix")
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
      policy_null_tests!(cases)
      policy_error_tests!
      policy_regex_failure_tests!(cases)
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

    def policy_null_tests!(cases)
      nulls = cases.select { |test_case| test_case[:null_candidate] }
      assert(nulls.map { |test_case| test_case.fetch(:null_candidate) } == %w[infohash_v1 infohash_v2 magnet_hash uploader], "four real in-ingestion NULL candidate families")
      rules = File.read(File.join(@contract.root, "crates/revaer-data/migrations/0019_policy_sets.sql"))
      canonical = File.read(File.join(@contract.root, "crates/revaer-data/migrations/0022_indexer_canonicalization.sql"))
      assert(rules.include?("match_value_text VARCHAR(512),"), "NULL regex/equality text operands are constraint-reachable")
      %w[match_operator is_case_insensitive].zip(["policy_match_operator", "BOOLEAN"]).each do |column, type|
        assert(rules.include?("#{column} #{type} NOT NULL"), "NULL #{column} is constraint-unreachable, never bypassed")
      end
      assert(canonical.include?("(identity_strategy = 'title_size_fallback' AND title_size_hash IS NOT NULL)") &&
        canonical.include?("title_size_hash IS NULL OR size_bytes IS NOT NULL"), "hashless NULL magnet needs a real title/size identity")
      assert(canonical.include?("CONSTRAINT canonical_torrent_signal_single_value_chk CHECK"), "all-NULL signal values are not a legal fallback fixture")
      rejected("unknown policy NULL candidate") { policy_arguments(null_candidate: "title") }
      rejected("conflicting policy magnet") { policy_arguments(null_candidate: "magnet_hash", derived_magnet: true) }
      nulls.each do |test_case|
        field = test_case.fetch(:null_candidate)
        rule = test_case.fetch(:rules).first
        assert(rule.values_at(:field, :operator, :text) == [field, "regex", "["] && test_case.values_at(:decisions, :flag, :drop, :adjust) == [[], false, false, 0], "NULL candidate must bypass an otherwise failing regex")
        %w[GROUP OTHER].each do |token|
          arguments = policy_arguments(token:, null_candidate: field)
          assert(arguments.fetch("#{field}_input".to_sym).start_with?("NULL::"), "real fixture and tested arguments retain typed #{field} NULL")
          if field == "magnet_hash"
            assert(arguments.values_at(:infohash_v1_input, :infohash_v2_input, :magnet_hash_input, :magnet_uri_input, :size_bytes_input) ==
              ["NULL::char(40)", "NULL::char(64)", "NULL::char(64)", "NULL::varchar", "0::bigint"], "no supplied infohash or URI can silently derive a non-NULL magnet candidate")
          end
        end
        policy_null_success_tests!(test_case)
      end
      operands = cases.select { |test_case| test_case.fetch(:name).start_with?("null-regex-operand-", "null-eq-operand-") }
      assert(operands.map { |test_case| test_case.fetch(:name) } == %w[null-regex-operand-title null-regex-operand-release-token null-regex-operand-release-signal
        null-eq-operand-title null-eq-operand-release-token null-eq-operand-release-signal], "NULL right operands cover title, release token and persisted signal")
      operands.each do |test_case|
        rule = test_case.fetch(:rules).first
        assert(rule.fetch(:text).nil? && rule.fetch(:set).nil? && test_case.values_at(:decisions, :flag, :drop, :adjust) == [[], false, false, 0], "NULL right operand produces no decision, flag, drop or score adjustment")
        assert(policy_rules_sql(test_case).include?("'#{rule.fetch(:operator)}',NULL,NULL,NULL,NULL,'flag'"), "persist the actual NULL operand without constraints or helper substitution")
        policy_null_success_tests!(test_case)
      end
      derived = cases.select { |test_case| test_case[:derived_magnet] }
      assert(derived.map { |test_case| test_case.fetch(:name) } == ["nonnull-candidate-magnet_hash-regex-error"], "retain the separate real v1-derived non-NULL control")
      args = policy_arguments(derived_magnet: true)
      assert(args.values_at(:infohash_v1_input, :infohash_v2_input, :magnet_hash_input, :magnet_uri_input) ==
        ["repeat('a',40)::char(40)", "NULL::char(64)", "NULL::char(64)", "NULL::varchar"], "NULL magnet argument with valid v1 still derives a non-NULL candidate")
      assert(Digest::SHA256.hexdigest(["a" * 40].pack("H*")) == "750837cbeaadaf72ab0a852ee3e0517760ca56eb6b8fc3c08b2d9e73348b6bea", "independent decoded-v1 digest control")
    end

    def policy_null_success_tests!(test_case)
      frames = 3.times.map { |index| policy_null_success_frame(test_case, index) }
      frames.each_with_index do |frame, index|
        assert(policy_success?(test_case, frame, index), "independent NULL success and complete writes #{test_case.fetch(:name)} #{index}")
        frame.fetch("tables_after").each do |table, rows|
          next if table == "search_filter_decision"

          rows.each_with_index do |row, row_index|
            row.each_key do |column|
              changed = policy_copy(frame)
              changed.fetch("tables_after").fetch(table)[row_index][column] = "changed"
              assert(!policy_write_images?(test_case, changed, index), "reject NULL success mutation #{table}.#{column}")
            end
          end
        end
        %w[was_flagged was_downranked].each do |column|
          changed = policy_copy(frame)
          changed.fetch("tables_after").fetch("search_request_source_observation").last[column] = true
          assert(!policy_success?(test_case, changed, index), "NULL nonmatch cannot flag or downrank")
        end
      end
      return unless test_case[:null_candidate]

      evidence = { "fixture" => { "tables_after" => frames.first.fetch("tables_before") }, "before" => frames.first.fetch("tables_before"),
                   "after" => frames.last.fetch("tables_after"), "frames" => frames }
      assert(policy_candidate_images?(test_case, evidence), "persisted canonical/source/observation NULL inputs are independently pinned")
      policy_null_candidate_mutations!(test_case, evidence)
    end

    def policy_null_success_frame(test_case, index)
      frame = policy_total_boundary_frame(test_case, index, 37)
      hashes, strategy, confidence, size, uploader = case test_case[:derived_magnet] ? "derived-v1-magnet" : test_case[:null_candidate]
        when "infohash_v1" then [[nil, "b" * 64, "c" * 64], "infohash_v2", 1.0, nil, "Uploader"]
        when "infohash_v2" then [["a" * 40, nil, "c" * 64], "infohash_v1", 1.0, nil, "Uploader"]
        when "magnet_hash" then [[nil, nil, nil], "title_size_fallback", 0.6, 0, "Uploader"]
        when "uploader" then [["a" * 40, "b" * 64, "c" * 64], "infohash_v2", 1.0, nil, nil]
        when "derived-v1-magnet" then [["a" * 40, nil, "750837cbeaadaf72ab0a852ee3e0517760ca56eb6b8fc3c08b2d9e73348b6bea"], "infohash_v1", 1.0, nil, "Uploader"]
        else [["a" * 40, "b" * 64, "c" * 64], "infohash_v2", 1.0, nil, "Uploader"]
      end
      signal = { "canonical_torrent_signal_id" => 1, "canonical_torrent_id" => 1, "signal_key" => "release_group",
                 "value_text" => "group", "value_int" => nil, "confidence" => 0.9, "parser_version" => 1 }
      %w[tables_before tables_after].each do |key|
        tables = frame.fetch(key)
        %w[canonical_torrent canonical_torrent_source search_request_source_observation].each do |table|
          tables.fetch(table).each do |row|
            row.merge!(%w[infohash_v1 infohash_v2 magnet_hash].zip(hashes).to_h.merge("size_bytes" => size))
          end
        end
        tables.fetch("canonical_torrent").first.merge!("identity_strategy" => strategy, "identity_confidence" => confidence,
          "title_size_hash" => size == 0 ? Digest::SHA256.hexdigest("policy|0") : nil)
        tables.fetch("canonical_torrent_source").first["last_seen_uploader"] = uploader
        tables.fetch("search_request_source_observation").each do |row|
          row["uploader"] = uploader
          row["title_raw"] = "Policy.1080p-GROUP\\" if test_case[:token] && row.fetch("search_request_id") == 596001
        end
        tables["canonical_torrent_signal"] = [signal.dup]
        tables.fetch("canonical_torrent_signal") << signal.merge("canonical_torrent_signal_id" => 3) if test_case[:token] && index == 2
      end
      frame.fetch("tables_after").fetch("canonical_torrent_signal") << signal.merge("canonical_torrent_signal_id" => index + 2) if test_case[:token]
      frame.merge("state" => "00000", "tables_finish" => policy_copy(index.zero? ? frame.fetch("tables_before") : frame.fetch("tables_after")))
    end

    def policy_null_candidate_mutations!(test_case, evidence)
      { "canonical_torrent" => %w[identity_strategy identity_confidence title_size_hash],
        "canonical_torrent_source" => ["last_seen_uploader"], "search_request_source_observation" => ["uploader"] }.each do |table, extra|
        (%w[infohash_v1 infohash_v2 magnet_hash size_bytes] + extra).each do |column|
          [false, true].each do |missing|
            changed = policy_copy(evidence)
            images = [changed.fetch("fixture").fetch("tables_after"), changed.fetch("before"), changed.fetch("after")]
            images += changed.fetch("frames").flat_map { |frame| frame.values_at("tables_before", "tables_after", "tables_finish") }
            images.each { |tables| tables.fetch(table).each { |row| missing ? row.delete(column) : row[column] = "changed" } }
            assert(!policy_candidate_images?(test_case, changed), "reject coherent NULL input substitution/omission #{table}.#{column}")
          end
        end
      end
      changed = policy_copy(evidence)
      changed.fetch("fixture").fetch("tables_after").fetch("canonical_torrent_source").first["last_seen_uploader"] = "changed"
      assert(!policy_candidate_images?(test_case, changed), "fixture uploader is part of the NULL input oracle")
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

    def policy_regex_failure_tests!(cases)
      previous_inventory = @ingestion_inventory
      sql = File.read(File.join(@contract.root, "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql"))
      signatures = { "search_result_ingest_v1" => "search_result_ingest_v1(uuid)",
        "policy_text_match_v1" => "policy_text_match_v1(text,policy_match_operator,text,bigint,boolean)",
        "policy_release_group_match_v1" => "policy_release_group_match_v1(bigint,text,policy_match_operator,text,bigint,boolean)" }
      @ingestion_inventory = { "reference_proof" => { "routines" => signatures.map do |name, signature|
        source = sql[/CREATE OR REPLACE FUNCTION #{name}\(.*?\nAS \$\$(.*?)\$\$;/m, 1]
        assert(!source.nil?, "exact frozen #{name} source is available for the diagnostic oracle")
        { "name" => name, "signature" => signature, "source" => source }
      end } }
      failures = cases.select { |test_case| test_case[:regex_failure] }
      assert(failures.map { |test_case| test_case.fetch(:name) } == %w[nonnull-candidate-infohash_v1-regex-error nonnull-candidate-infohash_v2-regex-error
        nonnull-candidate-magnet_hash-regex-error nonnull-candidate-uploader-regex-error release-regex-error-token release-regex-error-persisted-signal], "nonnull and both release-group failure controls remain distinct")
      failures.each do |test_case|
        diagnostic = policy_test_regex_failure(test_case)
        expected = policy_regex_diagnostic(diagnostic, "postgres", test_case:)
        assert(expected.fetch("raw") == diagnostic, "independently pinned exact B6 native error stack")
        final = diagnostic.sub(/(search_result_ingest_v1\(uuid\) line )(\d+)/) { "#{Regexp.last_match(1)}#{Integer(Regexp.last_match(2)) + 1}" }
        assert(policy_regex_diagnostic(final, @runtime, test_case:) == expected, "only exact D3 caller displacement is normalized")
        [diagnostic + "NOTICE: extra\n", diagnostic + "WARNING: extra\n", diagnostic + "DETAIL: hidden\n", diagnostic + diagnostic,
         diagnostic.sub("2201B", "42501"), diagnostic.sub("line 14 at RETURN", "line 15 at RETURN"),
         diagnostic.sub("regexp.c:222", "regexp.c:223"), diagnostic.sub(" at assignment", " at RETURN")].each do |changed|
          rejected("exact B6 callsite") { policy_regex_diagnostic(changed, "postgres", test_case:) }
        end
        other = failures.find { |item| item.fetch(:regex_failure) != test_case.fetch(:regex_failure) }
        rejected("exact B6 callsite") { policy_regex_diagnostic(policy_test_regex_failure(other), "postgres", test_case:) }
        if test_case.fetch(:regex_failure).fetch(:path) == "persisted-signal"
          [diagnostic.sub(/SQL expression ".*?"\n/m, ""), diagnostic.sub("value_text,", "release_group_token_input,")].each do |changed|
            rejected("exact B6 callsite") { policy_regex_diagnostic(changed, "postgres", test_case:) }
          end
        end
        policy_regex_failure_frames!(test_case, diagnostic)
      end
      rejected("missing or ambiguous") { policy_source_line("RETURN TRUE;\nRETURN TRUE;\n", "RETURN TRUE;") }
    ensure
      @ingestion_inventory = previous_inventory
    end

    def policy_test_regex_failure(test_case)
      spec = test_case.fetch(:regex_failure)
      line = { "infohash_v1" => 1254, "infohash_v2" => 1262, "magnet_hash" => 1270, "uploader" => 1295, "release_group" => 1286 }.fetch(spec.fetch(:field))
      stack = "ERROR:  2201B: invalid regular expression: brackets [] not balanced\n" \
              "CONTEXT:  PL/pgSQL function policy_text_match_v1(text,policy_match_operator,text,bigint,boolean) line 14 at RETURN\n"
      if spec.fetch(:path) == "token"
        stack += "PL/pgSQL function policy_release_group_match_v1(bigint,text,policy_match_operator,text,bigint,boolean) line 4 at IF\n"
      elsif spec.fetch(:path) == "persisted-signal"
        stack += <<~TRACE
          SQL expression "EXISTS (
                  SELECT 1
                  FROM canonical_torrent_signal
                  WHERE canonical_torrent_id = canonical_torrent_id_input
                    AND signal_key = 'release_group'
                    AND policy_text_match_v1(
                        value_text,
                        match_operator_input,
                        match_value_text_input,
                        value_set_id_input,
                        is_case_insensitive_input
                    )
              )"
          PL/pgSQL function policy_release_group_match_v1(bigint,text,policy_match_operator,text,bigint,boolean) line 15 at RETURN
        TRACE
      end
      stack + "PL/pgSQL function search_result_ingest_v1(uuid) line #{line} at assignment\nLOCATION:  RE_compile_and_cache, regexp.c:222\n"
    end

    def policy_regex_failure_frames!(test_case, diagnostic)
      session = { calls: [{}, {}, {}], rollback: true }
      before = test_case[:derived_magnet] ? policy_null_success_frame(test_case, 0).fetch("tables_before") : policy_test_tables
      values = { "backend" => "101", "role" => {}, "clock" => "2026-09-11T01:00:00+00:00", "before" => "error", "after" => "error",
        "state" => "2201B", "within" => "false", "outside" => "false", "tables_before" => before, "tables_after" => before, "tables_finish" => before }
      stdout = correction_records(session).map do |key|
        value = values.fetch(key)
        value = JSON.generate(value) if %w[role clock tables_before tables_after tables_finish].include?(key)
        "#{key}:#{value}\n"
      end.join * 3
      frames = policy_error_parse(stdout, diagnostic * 3, session, "postgres", test_case:)
      if test_case[:derived_magnet]
        evidence = { "fixture" => { "tables_after" => before }, "before" => before, "after" => before, "frames" => frames }
        assert(policy_candidate_images?(test_case, evidence), "regex failure retains the actual derived magnet on canonical/source/observation")
        policy_null_candidate_mutations!(test_case, evidence)
      end
      %w[reference final].each do |variant|
        assert(policy_states?(test_case, frames, variant), "three exact B6 errors, never misclassified as D4/D5 #{variant}")
        frames.each_index do |index|
          changed = policy_copy(frames)
          changed[index]["state"] = "42P07"
          assert(!policy_states?(test_case, changed, variant), "D4 cannot excuse a missing B6 regex failure")
          frames[index].fetch("diagnostic").each_key do |key|
            changed = policy_copy(frames)
            changed[index].fetch("diagnostic")[key] = "changed"
            assert(!policy_states?(test_case, changed, variant), "reject altered B6 structured diagnostic #{key}")
          end
        end
      end
      assert(policy_lifetime?(test_case, frames, "reference") && policy_lifetime?(test_case, frames, "final"), "each failed regex removes its uncommitted temporary table")
      assert(policy_outcomes?(test_case, "frames" => frames) && correction_snapshots?({}, before, frames, before), "exact error savepoint and whole-transaction rollback images")
      frames.each_index do |index|
        IngestionProof::INGESTION_TABLES.each do |table|
          changed = policy_copy(frames)
          changed[index].fetch("tables_after").fetch(table) << { "unexpected" => true }
          assert(!policy_outcomes?(test_case, "frames" => changed), "reject failed-regex write #{index} #{table}")
        end
      end
      rejected("unexpected policy error framing") { policy_error_parse(stdout, diagnostic * 2, session, "postgres", test_case:) }
      rejected("exact B6 callsite") { policy_error_parse(stdout, diagnostic * 3 + "WARNING: extra\n", session, "postgres", test_case:) }
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
      if test_case[:null_candidate] || test_case[:derived_magnet]
        assert(policy_candidate_images?(test_case, evidence), "replay exact NULL/derived candidate fixture and persisted inputs")
        policy_null_candidate_mutations!(test_case, evidence)
      end
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
