# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Proof-only fixtures and validators. No production body or caller setting changes.
  module IngestionPolicy
    POLICY_REQUEST = "59600000-0000-4000-8000-000000000012"
    POLICY_FIELDS = {
      "infohash_v1" => ["block_infohash_v1", "a" * 40],
      "infohash_v2" => ["block_infohash_v2", "b" * 64],
      "magnet_hash" => ["block_magnet", "c" * 64],
      "title" => ["block_title_regex", "policy"],
      "release_group" => ["block_release_group", "group"],
      "uploader" => ["block_uploader", "uploader"],
      "tracker" => ["block_tracker", "proof-tracker"],
      "indexer_instance_public_id" => ["block_indexer_instance", "56900000-0000-4000-8000-000000000001"],
      "media_domain_key" => ["require_media_domain", "movies"],
      "trust_tier_key" => ["prefer_trust_tier", "public"],
      "trust_tier_rank" => ["require_trust_tier_min", 10]
    }.freeze
    POLICY_REQUIRE = {
      "indexer_instance_public_id" => ["allow_indexer_instance", "drop_source"],
      "title" => ["allow_title_regex", "drop_canonical"],
      "release_group" => ["allow_release_group", "drop_canonical"],
      "media_domain_key" => ["require_media_domain", "drop_source"],
      "trust_tier_rank" => ["require_trust_tier_min", "drop_source"]
    }.freeze
    POLICY_READ_TABLES = %w[
      canonical_disambiguation_rule canonical_torrent_source_base_score indexer_instance trust_tier
      indexer_instance_media_domain media_domain search_profile_tag_prefer indexer_instance_tag
      policy_snapshot_rule policy_rule policy_set policy_rule_value_set policy_rule_value_set_item
      search_request search_request_indexer_run search_profile policy_snapshot tag indexer_definition
    ].freeze

    private

    def verify_ingestion_policy!
      directory = File.join(@contract.output_path, "ingestion-policy")
      raise Failure, "ingestion policy evidence must not be a symlink" if File.symlink?(directory)

      @policy_evidence = File.join(directory, "run-#{Process.pid}-#{SecureRandom.hex(4)}")
      raise Failure, "ingestion policy evidence must not be a symlink" if File.symlink?(@policy_evidence)

      FileUtils.mkdir_p(@policy_evidence, mode: 0o700)
      previous_correction_path = @correction_evidence
      @correction_evidence = @policy_evidence
      first_check = @checks.length
      source_hashes = policy_source_hashes
      cases = []
      completed = false
      begin
        policy_cast_inventory!
        policy_cases.each do |test_case|
          %w[cold helpers-first].each do |mode|
            variants = {}
            { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
              evidence = policy_isolated(test_case, variant, source, role, mode:)
              variants[variant] = evidence
            end
            comparable = variants.transform_values { |evidence| policy_comparable(evidence, count: 2) }
            equal = comparable.fetch("reference") == comparable.fetch("final")
            check("policy #{test_case.fetch(:name)} #{mode} cold and warm-after-rollback parity", equal)
            cases << { name: test_case.fetch(:name), mode:, equivalent: equal }
          end
        end
        check("policy proof source files unchanged during execution", source_hashes == policy_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first_check)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
                   warm_proof: "first real ingestion rolled back, same target committed on the same backend",
                   committed_reuse: "separate third call: frozen D4 failure versus final success; never frozen successful committed reuse",
                   fixture_boundary: "controlled persisted snapshots include defensive NULLs and synthetic literal-backslash suffixes; not policy-creation API conformance or ordinary suffix recognition",
                   score_boundary: "base scores outside [-10000,10000] are constraint-unreachable; non-drop totals cross those bounds through valid policy/tag adjustments",
                   candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
                   postgres_image: @contract.postgres_image, seed_sha256: Digest::SHA256.file(policy_seed_path).hexdigest,
                   roles: { reference: "postgres", final: @runtime, owner: @owner }, container: @container, source_sha256: source_hashes,
                   checks:, cases: }
        File.binwrite(File.join(@policy_evidence, "report.json"), JSON.pretty_generate(report) + "\n")
        @correction_evidence = previous_correction_path
      end
    end

    def policy_seed_path
      File.join(@contract.root, "scripts/tests/database-ingestion-policy-seed.sql")
    end

    def policy_source_hashes
      %w[scripts/database_rebaseline/ingestion_policy.rb scripts/tests/database-ingestion-policy-test.rb
         scripts/tests/database-ingestion-policy-seed.sql scripts/database_rebaseline/ingestion_proof.rb
         scripts/database_rebaseline/ingestion_corrections.rb scripts/database_rebaseline/ingestion_compilation.rb].to_h do |path|
        [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest]
      end
    end

    def policy_cast_inventory!
      query = <<~SQL
        SELECT json_build_object('source', c.castsource::regtype::text, 'target', c.casttarget::regtype::text,
          'method', c.castmethod, 'context', c.castcontext, 'function', c.castfunc::regprocedure::text)
        FROM pg_cast c WHERE c.castsource = 'public.policy_action'::regtype AND c.casttarget = 'public.decision_type'::regtype;
      SQL
      inventory = ["reference_proof", @database].to_h { |database| [database, JSON.parse(sql(query, role: "postgres", database:))] }
      File.binwrite(File.join(@policy_evidence, "cast-inventory.json"), JSON.pretty_generate(inventory) + "\n")
      File.binwrite(File.join(@policy_evidence, "routine-inventory.json"), JSON.pretty_generate(@ingestion_inventory) + "\n")
      expected = { "source" => "policy_action", "target" => "decision_type", "method" => "f", "context" => "a",
                   "function" => "policy_action_to_decision_type(policy_action)" }
      check("policy actual action::decision_type dispatch in both databases", inventory.values.all? { |value| value == expected })
    end

    def policy_rule(field, operator: "in_set", matched: true, action: "flag", severity: "soft", scope: 4, type: nil, **extra)
      rule_type, value = POLICY_FIELDS.fetch(field)
      kind = field == "indexer_instance_public_id" ? "uuid" : (field == "trust_tier_rank" ? "int" : "text")
      set = 596201 + %w[text uuid int].index(kind) + (matched ? 0 : 3)
      { field:, type: type || rule_type, operator:, action:, severity:, scope:, insensitive: true,
        text: kind == "text" && operator != "in_set" ? (matched ? value : "not-a-match") : nil,
        int: kind == "int" && operator != "in_set" ? (matched ? value : 99) : nil,
        uuid: kind == "uuid" && operator != "in_set" ? (matched ? value : "59600000-0000-4000-8000-000000000099") : nil,
        set: operator == "in_set" ? set : nil }.merge(extra)
    end

    def policy_case(name, rules, decisions: nil, adjust: 0, drop: false, **extra)
      rules = rules.each_with_index.map { |rule, index| rule.merge(id: index + 1, order: (index + 1) * 10) }
      { name:, rules:, decisions: decisions || rules.map { |rule| [rule.fetch(:id), rule.fetch(:action)] }, adjust:, drop:,
        flag: rules.any? { |rule| rule.fetch(:action) == "flag" },
        downrank: rules.any? { |rule| rule.fetch(:action) == "downrank" }, tag: 0, base: 37 }.merge(extra)
    end

    def policy_require_rules(matched:, scope: 4)
      POLICY_REQUIRE.map do |field, (type, _decision)|
        policy_rule(field, matched:, scope:, type:, action: "require", operator: field == "trust_tier_rank" ? "eq" : "in_set")
      end
    end

    def policy_cases
      cases = [policy_case("populated-all-fields", POLICY_FIELDS.keys.map { |field| policy_rule(field) })]
      cases << policy_case("populated-nonmatches", POLICY_FIELDS.keys.map { |field| policy_rule(field, matched: false) }, decisions: [], flag: false)
      cases << policy_case("null-set-ids", POLICY_FIELDS.keys.map { |field| policy_rule(field, set: nil) }, decisions: [], flag: false)
      %w[eq contains starts_with ends_with].each do |operator|
        fields = operator == "eq" ? POLICY_FIELDS.keys : POLICY_FIELDS.keys - %w[indexer_instance_public_id trust_tier_rank]
        cases << policy_case("operator-#{operator}", fields.map { |field| policy_rule(field, operator:) })
      end
      [[false, "^policy$", true], [false, "^POLICY$", false], [true, "^POLICY$", true]].each_with_index do |(insensitive, text, matched), index|
        cases << policy_case("regex-#{index}", [policy_rule("title", operator: "regex", insensitive:, text:)], decisions: matched ? [[1, "flag"]] : [], flag: matched)
      end
      cases << policy_case("invalid-regex", [policy_rule("title", operator: "regex", text: "[")], decisions: [], flag: false, error: "2201B")
      %w[drop_canonical drop_source downrank flag].each do |action|
        %w[hard soft].each do |severity|
          adjust = action == "downrank" ? (severity == "hard" ? -50 : -10) : 0
          cases << policy_case("#{action}-#{severity}", [policy_rule("title", action:, severity:)], adjust:, drop: action.start_with?("drop_"))
        end
      end
      prefer = [["indexer_instance_public_id", "prefer_indexer_instance"], ["trust_tier_key", "prefer_trust_tier"],
                ["title", "allow_title_regex"], ["release_group", "allow_release_group"], ["uploader", "block_uploader"]]
               .map { |field, type| policy_rule(field, type:, action: "prefer") }
      cases << policy_case("all-prefer-branches", prefer, decisions: [], adjust: 41)
      [15, 10, 8, 8, 0].each_with_index do |adjust, index|
        cases << policy_case("prefer-branch-#{index}", [prefer.fetch(index)], decisions: [], adjust:)
      end
      [-50, 50].each { |tag| cases << policy_case("prefer-tag-#{tag}", prefer, decisions: [], adjust: 41, tag:) }
      cases << policy_case("require-all-pass", policy_require_rules(matched: true), decisions: [])
      failures = POLICY_REQUIRE.values.map.with_index { |(_type, decision), index| [index + 1, decision] }
      cases << policy_case("require-all-fail", policy_require_rules(matched: false), decisions: failures, drop: true)
      POLICY_REQUIRE.keys.each_with_index do |field, index|
        rules = policy_require_rules(matched: true)
        rules[index] = policy_require_rules(matched: false).fetch(index)
        cases << policy_case("require-fail-#{field}", rules, decisions: [failures.fetch(index)], drop: true)
      end
      cases << policy_case("require-trust-null", [policy_rule("trust_tier_rank", action: "require", operator: "eq", int: nil)], decisions: [])
      cases << policy_case("require-trust-below", [policy_rule("trust_tier_rank", action: "require", operator: "eq", int: 9)], decisions: [])
      (1..4).each do |scope|
        [true, false].each do |matched|
          chosen = policy_require_rules(matched:, scope:)
          lower = ((scope + 1)..4).flat_map { |rank| policy_require_rules(matched: !matched, scope: rank) }
          # Deliberately order lower-precedence rules first. Scope wins over rule order.
          rules = lower + chosen
          decisions = matched ? [] : failures.map { |id, action| [id + lower.length, action] }
          cases << policy_case("scope-#{scope}-#{matched}", rules, decisions:, drop: !matched)
        end
      end
      rules = policy_require_rules(matched: false) + policy_require_rules(matched: true)
      cases << policy_case("same-scope-any-match-trust-maximum", rules, decisions: [[5, "drop_source"]], drop: true)
      ordered = policy_case("same-scope-ordered-failures", policy_require_rules(matched: false) * 2,
                            decisions: failures.map { |id, action| [id + 5, action] }, drop: true)
      ordered.fetch(:rules).each_with_index { |rule, index| rule[:order] = 1000 - index * 10 }
      cases << ordered
      cases << policy_case("release-token-match", [policy_rule("release_group")], token: "GROUP")
      cases << policy_case("release-token-false-persisted-true", [policy_rule("release_group")], token: "OTHER")
      cases << policy_case("release-token-and-persisted-false", [policy_rule("release_group", matched: false)], token: "OTHER", decisions: [], flag: false)
      cases + policy_total_score_cases
    end

    def policy_total_score_cases
      # 0024 constrains the base itself. These tags need no tag clamp and none
      # of these rules drops a source; only the final total can exceed its bounds.
      [
        ["total-score-below-minimum", -10000, "downrank", -10, 9, -10000],
        ["total-score-exact-minimum", -10000, "downrank", -10, 10, -10000],
        ["total-score-above-maximum", 10000, "prefer", 15, -14, 10000],
        ["total-score-exact-maximum", 10000, "prefer", 15, -15, 10000]
      ].map do |name, base, action, adjust, tag, total|
        rule = action == "downrank" ? policy_rule("title", action:) :
          policy_rule("indexer_instance_public_id", type: "prefer_indexer_instance", action:)
        policy_case(name, [rule], decisions: action == "prefer" ? [] : [[1, "downrank"]], base:, adjust:, tag:, total:)
      end
    end

    def policy_arguments(token: nil)
      # The frozen regex requires a literal backslash. Do not silently fix it or
      # pretend an ordinary -GROUP suffix enters its token branch.
      title = token ? "Policy.1080p-#{token}\\" : "Policy.1080p"
      ingestion_existing_attrs.merge(title_raw_input: "#{literal(title)}::varchar", uploader_input: "'Uploader'::varchar",
                                     infohash_v2_input: "repeat('b',64)::char(64)", magnet_hash_input: "repeat('c',64)::char(64)",
                                     size_bytes_input: "NULL::bigint")
    end

    def policy_rule_uuid(id)
      format("59600000-0000-4000-8000-%012d", 1000 + id)
    end

    def policy_rules_sql(test_case)
      rules = test_case.fetch(:rules).map do |rule|
        values = [596000 + rule.fetch(:scope), policy_rule_uuid(rule.fetch(:id)), rule.fetch(:type), rule.fetch(:field),
                  rule.fetch(:operator), rule.fetch(:text), rule.fetch(:int), rule.fetch(:uuid), rule.fetch(:set),
                  rule.fetch(:action), rule.fetch(:severity), rule.fetch(:insensitive)]
        "(#{values.map { |value| policy_sql_value(value) }.join(',')},0,0,'2026-09-11T00:00:00Z','2026-09-11T00:00:00Z')"
      end
      members = test_case.fetch(:rules).map { |rule| "(596001,#{literal(policy_rule_uuid(rule.fetch(:id)))},#{rule.fetch(:order)})" }
      <<~SQL
        INSERT INTO public.policy_rule (policy_set_id, policy_rule_public_id, rule_type, match_field, match_operator,
          match_value_text, match_value_int, match_value_uuid, value_set_id, action, severity, is_case_insensitive,
          created_by_user_id, updated_by_user_id, created_at, updated_at) VALUES #{rules.join(',')};
        INSERT INTO public.policy_snapshot_rule (policy_snapshot_id, policy_rule_public_id, rule_order) VALUES #{members.join(',')};
        UPDATE public.search_profile_tag_prefer SET weight_override = #{test_case.fetch(:tag)} WHERE search_profile_id = 596001;
      SQL
    end

    def policy_sql_value(value)
      case value
      when NilClass then "NULL"
      when Integer, TrueClass, FalseClass then value.to_s
      when String then literal(value)
      else raise Failure, "unsupported policy fixture SQL value"
      end
    end

    def policy_read_snapshot(database)
      pairs = POLICY_READ_TABLES.map do |table|
        "#{literal(table)}, (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)::text), '[]') FROM public.#{identifier(table)} t)"
      end
      JSON.parse(sql("SELECT json_build_object(#{pairs.join(',')});", role: "postgres", database:))
    end

    def policy_comparable_inputs(evidence)
      images = evidence.fetch("inputs_before")
      clocks = evidence.fetch("seed_clocks")
      raise Failure, "policy read-input inventory changed" unless images.keys.sort == POLICY_READ_TABLES.sort

      images.to_h do |table, rows|
        normalized = rows.map do |row|
          row.to_h do |column, value|
            phase = clocks.key(value) if %w[created_at updated_at].include?(column)
            [column, phase ? "<validated-#{phase}-seed-transaction>" : value]
          end
        end
        [table, normalized.sort_by { |row| JSON.generate(row.sort) }]
      end
    end

    def policy_isolated(test_case, variant, source, role, mode:)
      database = "ingestion_policy_#{variant}"
      name = "#{test_case.fetch(:name)}-#{mode}-#{variant}"
      prefix = File.join(@policy_evidence, name)
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed_clocks = { "base" => policy_setup!(File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")), database, "#{prefix}-base-seed") }
        seed_clocks["policy"] = policy_setup!(File.binread(policy_seed_path), database, "#{prefix}-policy-seed")
        correction_observer!(database, variant)
        fixture_case = { calls: [policy_arguments(token: "GROUP")] }
        fixture = policy_execute(fixture_case, database, role, "#{prefix}-fixture").fetch("frames").fetch(0)
        check("#{name} actual fixture ingestion and signal", policy_fixture?(fixture))
        raise Failure, "policy fixture ingestion failed; retained exact evidence" unless policy_fixture?(fixture)

        setup = policy_rules_sql(test_case) + <<~SQL
          INSERT INTO public.canonical_torrent_source_base_score (
            canonical_torrent_id, canonical_torrent_source_id, score_total_base, score_seed, score_leech,
            score_age, score_trust, score_health, score_reputation, computed_at
          ) SELECT canonical_torrent_id, canonical_torrent_source_id, #{test_case.fetch(:base)}, 0, 0, 0, 0, 0, 0, '2026-09-11T00:00:00Z'
            FROM public.search_request_source_observation WHERE search_request_id = 569001;
        SQL
        seed_clocks["rules"] = policy_setup!(setup, database, "#{prefix}-setup")
        before = ingestion_snapshot(database)
        inputs = policy_read_snapshot(database)
        args = policy_arguments(token: test_case[:token]).merge(search_request_public_id_input: "#{literal(POLICY_REQUEST)}::uuid")
        session_case = { calls: [args, args, args], rollback: true }
        evidence = policy_execute(session_case, database, role, prefix, helpers: mode == "helpers-first", test_case:)
        evidence.merge!("fixture" => fixture, "before" => before, "after" => ingestion_snapshot(database),
                        "inputs_before" => inputs, "inputs_after" => policy_read_snapshot(database),
                        "inputs_sha256" => Digest::SHA256.hexdigest(JSON.generate(inputs)), "seed_clocks" => seed_clocks)
        File.binwrite("#{prefix}.json", JSON.pretty_generate(evidence) + "\n")
        policy_verify!(name, test_case, evidence, variant, role, mode:)
        evidence
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def policy_setup!(query, database, prefix)
      query = "\\set VERBOSITY verbose\nBEGIN;\nSELECT 'seed_clock:' || to_json(transaction_timestamp())::text;\n#{query}\nCOMMIT;"
      File.binwrite("#{prefix}.sql", query)
      outcome = result(query, role: "postgres", database:)
      File.binwrite("#{prefix}.stdout", outcome.stdout)
      File.binwrite("#{prefix}.stderr", outcome.stderr)
      raise Failure, "policy fixture setup failed; retained exact diagnostic" unless outcome.success && outcome.stderr.empty?

      match = outcome.stdout.match(/\Aseed_clock:("[^\n]+")\n\z/)
      raise Failure, "policy seed clock framing changed" unless match

      clock = JSON.parse(match[1])
      raise Failure, "invalid policy seed clock" unless clock.is_a?(String) && clock.match?(/\A\d{4}-\d\d-\d\dT.*\+00:00\z/)

      clock
    end

    def policy_fixture?(frame)
      tables = frame.fetch("tables_after")
      frame.fetch("state") == "00000" && frame.fetch("tables_before").values.all?(&:empty?) &&
        frame.fetch("tables_finish") == tables && correction_results?({}, [frame], tables) &&
        tables.fetch("canonical_torrent_signal").length == 1 &&
        tables.fetch("canonical_torrent_signal").first.values_at("signal_key", "value_text", "confidence") == ["release_group", "group", 0.9]
    end

    def policy_helper_answers
      { "text" => [true, false, false], "uuid" => [true, false, false], "int" => [true, false, false],
        "regex" => [true, false, true], "release" => [true, true, true, false],
        "cast" => %w[drop_canonical drop_source downrank flag flag flag] }
    end

    def policy_helpers_sql
      <<~SQL
        SELECT 'policy_helpers:' || json_build_object('backend', pg_backend_pid()::text,
          'session', session_user, 'current', current_user, 'setting', current_setting('plpgsql.variable_conflict'),
          'answers', json_build_object(
            'text', json_build_array(public.policy_text_match_v1('PoLiCy','in_set',NULL,596201,false),
              public.policy_text_match_v1('policy','in_set',NULL,596204,true), public.policy_text_match_v1('policy','in_set',NULL,NULL,true)),
            'uuid', json_build_array(public.policy_uuid_match_v1('56900000-0000-4000-8000-000000000001','in_set',NULL,596202),
              public.policy_uuid_match_v1('56900000-0000-4000-8000-000000000001','in_set',NULL,596205),
              public.policy_uuid_match_v1('56900000-0000-4000-8000-000000000001','in_set',NULL,NULL)),
            'int', json_build_array(public.policy_int_match_v1(10,'in_set',NULL,596203), public.policy_int_match_v1(10,'in_set',NULL,596206),
              public.policy_int_match_v1(10,'in_set',NULL,NULL)),
            'regex', json_build_array(public.policy_text_match_v1('policy','regex','^policy$',NULL,false),
              public.policy_text_match_v1('policy','regex','^POLICY$',NULL,false), public.policy_text_match_v1('policy','regex','^POLICY$',NULL,true)),
            'release', json_build_array(public.policy_release_group_match_v1(1,'group','in_set',NULL,596201,true),
              public.policy_release_group_match_v1(1,'other','in_set',NULL,596201,true), public.policy_release_group_match_v1(1,NULL,'in_set',NULL,596201,true),
              public.policy_release_group_match_v1(1,'other','in_set',NULL,596204,true)),
            'cast', (SELECT json_agg(public.policy_action_to_decision_type(v::public.policy_action) ORDER BY n)
              FROM unnest(ARRAY['drop_canonical','drop_source','downrank','flag','require','prefer']) WITH ORDINALITY x(v,n))
          ))::text;
      SQL
    end

    def policy_execute(session_case, database, role, prefix, helpers: false, test_case: nil)
      query = correction_session(session_case)
      if helpers
        anchor = "DO $$ BEGIN NULL; END $$;"
        raise Failure, "policy helper insertion point changed" unless query.scan(anchor).length == 1

        query = query.sub(anchor, "#{anchor}\n#{policy_helpers_sql}")
      end
      File.binwrite("#{prefix}.sql", query)
      outcome = result(query, role:, database:)
      File.binwrite("#{prefix}.stdout", outcome.stdout)
      File.binwrite("#{prefix}.stderr", outcome.stderr)
      raise Failure, "policy proof transport failed" unless outcome.success

      stdout = outcome.stdout
      helper_record = nil
      if helpers
        first, stdout = stdout.split("\n", 2)
        raise Failure, "policy helper-first record missing or reordered" unless first&.start_with?("policy_helpers:") && stdout

        helper_record = JSON.parse(first.delete_prefix("policy_helpers:"))
      end
      frames = if test_case && test_case[:error]
                 policy_error_parse(stdout, outcome.stderr, session_case, role)
               else
                 parsed = correction_parse(stdout, outcome.stderr, session_case)
                 errors = parsed.reject { |frame| frame.fetch("state") == "00000" }
                 errors.zip(outcome.stderr.split(/(?=^ERROR:  )/)).each { |frame, raw| frame["diagnostic"] = policy_d4_diagnostic(raw) }
                 parsed
               end
      { "frames" => frames, "helpers" => helper_record }
    rescue JSON::ParserError
      raise Failure, "invalid policy JSON evidence"
    end

    def policy_d4_expected
      routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |row| row.fetch("name") == "search_result_ingest_v1" }
      source = routine.fetch("source")
      statements = source.scan(/CREATE TEMP TABLE tmp_policy_rules AS\n.*?;/m)
      raise Failure, "frozen policy scratch statement changed" unless statements.length == 1

      line = source.lines.index { |value| value.strip == "CREATE TEMP TABLE tmp_policy_rules AS" }
      raise Failure, "frozen policy scratch line missing" unless line

      "ERROR:  42P07: relation \"tmp_policy_rules\" already exists\n" \
        "CONTEXT:  SQL statement \"#{statements.first.delete_suffix(';')}\"\n" \
        "PL/pgSQL function #{routine.fetch('signature')} line #{line + 1} at SQL statement\n" \
        "LOCATION:  CreateTableAsRelExists, createas.c:406\n"
    end

    def policy_d4_diagnostic(raw)
      raise Failure, "policy D4 diagnostic differs from exact frozen statement and native stack" unless raw == policy_d4_expected

      { "state" => "42P07", "message" => 'relation "tmp_policy_rules" already exists', "detail" => nil, "wrapper" => nil, "raw" => raw }
    end

    def policy_error_parse(stdout, stderr, session_case, role)
      # Nested regex diagnostics are not accepted by the shared D4/D5 parser.
      # Parse their full pinned stack here instead of dropping context or warnings.
      records = stderr.split(/(?=^ERROR:  )/)
      lines = stdout.lines(chomp: true)
      frames = session_case.fetch(:calls).map do |_call|
        correction_records(session_case).to_h do |key|
          line = lines.shift
          raise Failure, "policy error record missing or reordered: #{key}" unless line&.start_with?("#{key}:")

          value = line.delete_prefix("#{key}:")
          [key, %w[role clock tables_before tables_after tables_finish].include?(key) ? JSON.parse(value) : value.strip]
        end
      end
      raise Failure, "unexpected policy error framing" unless lines.empty? && records.length == frames.length

      frames.zip(records).each do |frame, record|
        raise Failure, "unexpected policy regex SQLSTATE" unless frame.fetch("state") == "2201B"

        frame["diagnostic"] = policy_regex_diagnostic(record, role)
      end
      frames
    rescue JSON::ParserError
      raise Failure, "invalid policy error JSON"
    end

    def policy_regex_diagnostic(record, role)
      match = record.match(/\AERROR:  2201B: invalid regular expression: brackets \[\] not balanced\nCONTEXT:  PL\/pgSQL function policy_text_match_v1\(text,policy_match_operator,text,bigint,boolean\) line (?<helper_line>[1-9][0-9]*) at RETURN\nPL\/pgSQL function search_result_ingest_v1\((?<signature>[^\n]+)\) line (?<line>[1-9][0-9]*) at assignment\nLOCATION:  (?<location>RE_compile_and_cache, regexp.c:222)\n\z/)
      raise Failure, "unrecognized policy regex diagnostic" unless match

      routines = @ingestion_inventory.fetch("reference_proof").fetch("routines")
      ingest = routines.find { |routine| routine.fetch("name") == "search_result_ingest_v1" }
      helper = routines.find { |routine| routine.fetch("name") == "policy_text_match_v1" }
      line = Integer(match[:line]) - (role == @runtime ? 1 : 0)
      helper_line = Integer(match[:helper_line])
      valid = ingest.fetch("signature") == "search_result_ingest_v1(#{match[:signature]})" &&
              helper.fetch("source").lines.fetch(helper_line - 1, "").include?("RETURN candidate_input ~* match_value_text_input;") &&
              ingest.fetch("source").lines.fetch(line - 1, "").include?("rule_matched := policy_text_match_v1(")
      raise Failure, "policy regex stack outside exact routines" unless valid

      { "state" => "2201B", "message" => "invalid regular expression: brackets [] not balanced",
        "line" => line, "helper_line" => helper_line, "location" => match[:location] }
    end

    def policy_verify!(name, test_case, evidence, variant, role, mode:)
      frames = evidence.fetch("frames")
      check("#{name} exact outcome sequence and D4 diagnostic", policy_states?(test_case, frames, variant))
      check("#{name} direct roles settings cold and warm backend", policy_context?(evidence, role, mode:))
      check("#{name} all 18 images rollback and committed continuity", policy_images?(evidence))
      check("#{name} unchanged pinned read inputs", policy_inputs?(evidence, test_case))
      check("#{name} exact temp lifetime without backend repair", policy_lifetime?(test_case, frames, variant))
      check("#{name} exact policy decisions scores drops and identities", policy_outcomes?(test_case, evidence))
    end

    def policy_states?(test_case, frames, variant)
      expected = test_case[:error] ? Array.new(3, test_case.fetch(:error)) : ["00000", "00000", variant == "reference" ? "42P07" : "00000"]
      return false unless frames.map { |frame| frame.fetch("state") } == expected

      frames.all? do |frame|
        state = frame.fetch("state")
        if state == "00000"
          frame.key?("result") && !frame.key?("diagnostic")
        elsif state == "42P07"
          diagnostic = frame.fetch("diagnostic")
          !frame.key?("result") && diagnostic == policy_d4_diagnostic(policy_d4_expected)
        else
          !frame.key?("result") && frame.fetch("diagnostic").values_at("state", "message") == ["2201B", "invalid regular expression: brackets [] not balanced"]
        end
      end
    end

    def policy_context?(evidence, role, mode:)
      fixture = evidence.fetch("fixture")
      frames = evidence.fetch("frames")
      all = [fixture] + frames
      capabilities = { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" }
      backend = frames.first.fetch("backend")
      clocks = all.map { |frame| frame.fetch("clock") }
      context = all.all? { |frame| frame.fetch("role") == capabilities && frame.values_at("before", "after") == %w[error error] } &&
                backend.match?(/\A[1-9][0-9]*\z/) && frames.all? { |frame| frame.fetch("backend") == backend } &&
                fixture.fetch("backend") != backend && fixture.fetch("backend").match?(/\A[1-9][0-9]*\z/) &&
                clocks.uniq.length == all.length && clocks.all? { |clock| clock.is_a?(String) && clock.match?(/\A\d{4}-\d\d-\d\dT.*\+00:00\z/) }
      helpers = mode == "helpers-first" ? { "backend" => backend, "session" => role, "current" => role, "setting" => "error", "answers" => policy_helper_answers } : nil
      context && evidence.fetch("helpers") == helpers
    end

    def policy_images?(evidence)
      frames = evidence.fetch("frames")
      fixture = evidence.fetch("fixture")
      before = evidence.fetch("before")
      return false unless policy_fixture?(fixture) && fixture.fetch("tables_finish") == before
      return false unless correction_snapshots?({ rollback: true }, before, frames, evidence.fetch("after")) ||
                          (frames.all? { |frame| frame.fetch("state") != "00000" } && correction_snapshots?({}, before, frames, evidence.fetch("after")))

      frames.fetch(0).fetch("tables_finish") == before && frames.fetch(1).fetch("tables_before") == before
    end

    def policy_inputs?(evidence, test_case)
      before, after = evidence.values_at("inputs_before", "inputs_after")
      return false unless before == after && before.keys.sort == POLICY_READ_TABLES.sort && policy_input_clocks?(evidence) &&
                          evidence.fetch("inputs_sha256") == Digest::SHA256.hexdigest(JSON.generate(before))

      rules = before.fetch("policy_rule").select { |row| test_case.fetch(:rules).any? { |rule| row.fetch("policy_rule_public_id") == policy_rule_uuid(rule.fetch(:id)) } }
      return false unless rules.length == test_case.fetch(:rules).length

      test_case.fetch(:rules).all? do |rule|
        row = rules.find { |item| item.fetch("policy_rule_public_id") == policy_rule_uuid(rule.fetch(:id)) }
        row.values_at("match_field", "rule_type", "match_operator", "action", "severity", "match_value_text", "match_value_int", "match_value_uuid", "value_set_id", "is_case_insensitive", "policy_set_id") ==
          rule.values_at(:field, :type, :operator, :action, :severity, :text, :int, :uuid, :set, :insensitive) + [596000 + rule.fetch(:scope)]
      end && policy_selected_inputs?(before, test_case)
    end

    def policy_input_clocks?(evidence)
      clocks = evidence.fetch("seed_clocks")
      return false unless clocks.keys == %w[base policy rules] && clocks.values.uniq.length == 3 &&
                          clocks.values.all? { |clock| clock.is_a?(String) && clock.match?(/\A\d{4}-\d\d-\d\dT.*\+00:00\z/) }

      inputs = evidence.fetch("inputs_before")
      %w[indexer_instance indexer_definition search_request policy_snapshot trust_tier media_domain].all? do |table|
        inputs.fetch(table).all? do |row|
          expected = if %w[trust_tier media_domain].include?(table) || (table == "search_request" && row.fetch("search_request_id") == 596001)
                       clocks.fetch("policy")
                     elsif table == "policy_snapshot" && row.fetch("policy_snapshot_id") == 596001
                       "2026-09-11T00:00:00+00:00"
                     else
                       clocks.fetch("base")
                     end
          row.fetch("created_at") == expected && (!row.key?("updated_at") || row.fetch("updated_at") == expected)
        end
      end
    end

    def policy_selected_inputs?(inputs, test_case)
      members = inputs.fetch("policy_snapshot_rule").select { |row| row.fetch("policy_snapshot_id") == 596001 }
      expected = test_case.fetch(:rules).map { |rule| [policy_rule_uuid(rule.fetch(:id)), rule.fetch(:order)] }.sort
      sets = inputs.fetch("policy_set").select { |row| (596001..596004).cover?(row.fetch("policy_set_id")) }
      scope_pairs = (1..4).zip(%w[request profile user global]).map { |id, scope| [596000 + id, scope] }
      base = inputs.fetch("canonical_torrent_source_base_score")
      tags = inputs.fetch("search_profile_tag_prefer").select { |row| row.fetch("search_profile_id") == 596001 }
      members.map { |row| row.values_at("policy_rule_public_id", "rule_order") }.sort == expected &&
        sets.map { |row| row.values_at("policy_set_id", "scope") }.sort == scope_pairs &&
        base == [policy_base_score(test_case)] &&
        tags.length == 1 && tags.first.values_at("tag_id", "weight_override") == [596001, test_case.fetch(:tag)] && policy_value_sets?(inputs)
    end

    def policy_base_score(test_case)
      { "canonical_torrent_source_base_score_id" => 1, "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1,
        "score_total_base" => test_case.fetch(:base), "score_seed" => 0, "score_leech" => 0, "score_age" => 0,
        "score_trust" => 0, "score_health" => 0, "score_reputation" => 0, "computed_at" => "2026-09-11T00:00:00+00:00" }
    end

    def policy_value_sets?(inputs)
      expected = POLICY_FIELDS.reject { |field, _value| %w[indexer_instance_public_id trust_tier_rank].include?(field) }.values.map { |_type, value| [596201, value, nil, nil, nil] }
      expected += [[596204, "not-a-match", nil, nil, nil], [596202, nil, nil, "56900000-0000-4000-8000-000000000001", nil],
                   [596205, nil, nil, "59600000-0000-4000-8000-000000000099", nil], [596203, nil, 10, nil, nil], [596206, nil, 99, nil, nil]]
      items = inputs.fetch("policy_rule_value_set_item").select { |row| (596201..596206).cover?(row.fetch("value_set_id")) }
      items.map { |row| row.values_at("value_set_id", "value_text", "value_int", "value_uuid", "value_bigint") }.sort_by { |row| JSON.generate(row) } == expected.sort_by { |row| JSON.generate(row) }
    end

    def policy_lifetime?(test_case, frames, variant)
      within = test_case[:error] ? %w[false false false] : %w[true true true]
      outside = test_case[:error] || variant == "final" ? %w[false false false] : %w[false true true]
      frames.map { |frame| frame.fetch("within") } == within && frames.map { |frame| frame.fetch("outside") } == outside
    end

    def policy_outcomes?(test_case, evidence)
      frames = evidence.fetch("frames")
      frames.each_with_index.all? do |frame, index|
        if frame.fetch("state") != "00000"
          frame.fetch("tables_before") == frame.fetch("tables_after") && frame.fetch("tables_after") == frame.fetch("tables_finish")
        else
          policy_success?(test_case, frame, index)
        end
      end
    end

    def policy_success?(test_case, frame, index)
      before, after = frame.values_at("tables_before", "tables_after")
      canonical = before.fetch("canonical_torrent").first
      source = before.fetch("canonical_torrent_source").first
      result = frame.fetch("result")
      expected_result = { "canonical_torrent_public_id" => canonical.fetch("canonical_torrent_public_id"),
                          "canonical_torrent_source_public_id" => source.fetch("canonical_torrent_source_public_id"),
                          "canonical_changed" => false, "durable_source_created" => false, "observation_created" => index < 2 }
      return false unless result == expected_result

      observation = after.fetch("search_request_source_observation").select { |row| row.fetch("search_request_id") == 596001 }
      return false unless observation.length == 1

      observation = observation.first
      valid = observation.values_at("canonical_torrent_id", "canonical_torrent_source_id", "was_flagged", "was_downranked") ==
              [canonical.fetch("canonical_torrent_id"), source.fetch("canonical_torrent_source_id"), test_case.fetch(:flag), test_case.fetch(:downrank)]
      valid && policy_decisions?(test_case, frame, observation) && policy_score?(test_case, after, canonical, source) &&
        policy_write_images?(test_case, frame, index)
    end

    def policy_decisions?(test_case, frame, observation)
      before, after = frame.values_at("tables_before", "tables_after")
      old = before.fetch("search_filter_decision")
      actual = after.fetch("search_filter_decision")
      return false unless actual.take(old.length) == old

      fresh = actual.drop(old.length)
      expected = test_case.fetch(:decisions).map do |id, decision|
        { "search_request_id" => 596001, "policy_rule_public_id" => policy_rule_uuid(id), "policy_snapshot_id" => 596001,
          "observation_id" => observation.fetch("observation_id"), "canonical_torrent_id" => observation.fetch("canonical_torrent_id"),
          "canonical_torrent_source_id" => observation.fetch("canonical_torrent_source_id"), "decision" => decision,
          "decision_detail" => nil, "decided_at" => frame.fetch("clock") }
      end
      # INSERT SELECT has no ORDER BY; retain IDs, validate uniqueness, compare the complete decision multiset.
      fresh.all? { |row| row.key?("search_filter_decision_id") && row.fetch("search_filter_decision_id").is_a?(Integer) && row.fetch("search_filter_decision_id").positive? } &&
        actual.map { |row| row.fetch("search_filter_decision_id") }.uniq.length == actual.length &&
        fresh.map { |row| row.reject { |column, _value| column == "search_filter_decision_id" } }.sort_by { |row| JSON.generate(row.sort) } == expected.sort_by { |row| JSON.generate(row.sort) }
    end

    def policy_score?(test_case, tables, canonical, source)
      rows = tables.fetch("canonical_torrent_source_context_score").select { |row| row.fetch("context_key_id") == 596001 }
      tag = test_case.fetch(:tag).clamp(-15, 15)
      total = test_case.fetch(:total) { test_case.fetch(:drop) ? -10000 : test_case.fetch(:base) + test_case.fetch(:adjust) + tag }
      # The direct v1 call has no wrapper tail to seed a best-source row. With
      # five seeders, even an admitted boundary score leaves this request unselected.
      return false if test_case.key?(:total) && !tables.fetch("canonical_torrent_best_source_context").empty?

      rows.length == 1 && rows.first.values_at("context_key_type", "canonical_torrent_id", "canonical_torrent_source_id", "score_total_context", "score_policy_adjust", "score_tag_adjust", "is_dropped") ==
        ["search_request", canonical.fetch("canonical_torrent_id"), source.fetch("canonical_torrent_source_id"), total, test_case.fetch(:adjust), test_case.fetch(:drop) ? 0 : tag, test_case.fetch(:drop)]
    end

    def policy_write_images?(test_case, frame, index)
      before, after = frame.values_at("tables_before", "tables_after")
      expected = before.transform_values { |rows| rows.map(&:dup) }
      clock = frame.fetch("clock")
      %w[canonical_torrent canonical_torrent_source].each do |table|
        return false unless expected.fetch(table).length == 1

        expected.fetch(table).first["updated_at"] = clock
      end
      new_observation = index < 2
      identity = index.zero? ? 2 : 3
      tag = test_case.fetch(:drop) ? 0 : test_case.fetch(:tag).clamp(-15, 15)
      score = {
        "canonical_torrent_source_context_score_id" => identity, "context_key_type" => "search_request", "context_key_id" => 596001,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1,
        "score_total_context" => test_case.fetch(:total) { test_case.fetch(:drop) ? -10000 : test_case.fetch(:base) + test_case.fetch(:adjust) + tag },
        "score_policy_adjust" => test_case.fetch(:adjust), "score_tag_adjust" => tag,
        "is_dropped" => test_case.fetch(:drop), "computed_at" => clock
      }
      expected["canonical_torrent_source_context_score"] = expected.fetch("canonical_torrent_source_context_score").reject { |row| row.fetch("context_key_id") == 596001 } + [score]
      if new_observation
        policy_expected_observation!(expected, test_case, identity, clock)
        policy_expected_page!(expected, identity, clock) unless test_case.fetch(:drop)
      end
      if test_case[:token]
        expected.fetch("canonical_torrent_signal") << {
          "canonical_torrent_signal_id" => index + 2, "canonical_torrent_id" => 1, "signal_key" => "release_group",
          "value_text" => test_case.fetch(:token).downcase, "value_int" => nil, "confidence" => 0.9, "parser_version" => 1
        }
      end
      # The separately validated complete multiset has no SQL ordering contract.
      expected["search_filter_decision"] = after.fetch("search_filter_decision")
      expected == after
    end

    def policy_expected_observation!(tables, test_case, identity, clock)
      observation = tables.fetch("search_request_source_observation").find { |row| row.fetch("search_request_id") == 569001 }
      title = test_case[:token] ? "Policy.1080p-#{test_case.fetch(:token)}\\" : "Policy.1080p"
      tables.fetch("search_request_source_observation") << observation.merge(
        "observation_id" => identity, "search_request_id" => 596001, "title_raw" => title,
        "was_flagged" => test_case.fetch(:flag), "was_downranked" => test_case.fetch(:downrank)
      )
      attrs = tables.fetch("search_request_source_observation_attr").select { |row| row.fetch("observation_id") == 1 }
      tables.fetch("search_request_source_observation_attr").concat(attrs.each_with_index.map do |row, offset|
        row.merge("observation_attr_id" => identity * 2 - 1 + offset, "observation_id" => identity, "created_at" => clock)
      end)
    end

    def policy_expected_page!(tables, identity, clock)
      tables.fetch("search_request_canonical") << { "search_request_canonical_id" => identity, "search_request_id" => 596001, "canonical_torrent_id" => 1, "first_seen_at" => clock }
      tables.fetch("search_page") << { "search_page_id" => identity, "search_request_id" => 596001, "page_number" => 1, "sealed_at" => nil }
      tables.fetch("search_page_item") << { "search_page_item_id" => identity, "search_page_id" => identity, "search_request_canonical_id" => identity, "position" => 1 }
    end

    def policy_comparable(evidence, count:)
      frames = evidence.fetch("frames").take(count)
      # Shared comparison normalizes only validated generated IDs and named transaction columns.
      comparable = compilation_comparable("fixture" => evidence.fetch("fixture"), "frames" => frames.map { |frame| frame.key?("result") ? frame : frame.merge("result" => {}) })
      frames.each_with_index { |frame, index| comparable.fetch(index + 1).delete("result") unless frame.key?("result") }
      { "application" => comparable, "inputs" => policy_comparable_inputs(evidence) }
    end
  end
end
