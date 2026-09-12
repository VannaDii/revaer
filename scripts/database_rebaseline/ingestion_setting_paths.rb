# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # NOTICE transport survives rollback; paired plain runs check observer effects.
  module IngestionSettingPaths
    SETTING_PATH_NAMES = %w[success-rollback late-validation-error policy-regex-error].freeze
    SETTING_SOURCE_FILES = (IngestionValidation::VALIDATION_SOURCE_FILES + %w[
      scripts/database_rebaseline/ingestion_setting_paths.rb scripts/tests/database-ingestion-setting-paths-test.rb
      scripts/tests/database-ingestion-policy-seed.sql
    ]).freeze

    private

    def setting_path_case(name, mode)
      raise Failure, "unknown setting path or mode" unless SETTING_PATH_NAMES.include?(name) && %w[cold helpers-first].include?(mode)

      arguments = case name
                  when "success-rollback" then {}
                  when "late-validation-error" then validation_attributes("year", :int, { int: "-1" })
                  when "policy-regex-error" then { search_request_public_id_input: "#{literal(IngestionPolicy::POLICY_REQUEST)}::uuid" }
                  end
      { name:, calls: Array.new(name == "success-rollback" ? 3 : 2) { arguments },
        rollback: true, helpers: mode == "helpers-first", finish_setting: true }
    end

    def setting_path_hashes
      SETTING_SOURCE_FILES.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] }
    end

    def verify_ingestion_setting_paths!
      @contract.validate_output_path!
      directory = File.join(@contract.output_path, "ingestion-setting-paths")
      dependency_directory!(directory)
      @setting_path_evidence = File.join(directory, "run-#{Process.pid}-#{SecureRandom.hex(4)}")
      dependency_directory!(@setting_path_evidence)
      @setting_path_validated_evidence = {}
      previous = @correction_evidence
      @correction_evidence = @setting_path_evidence
      first = @checks.length
      hashes = setting_path_hashes
      cases = []
      completed = false
      begin
        validation_sites!
        SETTING_PATH_NAMES.each do |name|
          %w[cold helpers-first].each do |mode|
            test_case = setting_path_case(name, mode)
            variants = { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.to_h do |variant, (source, role)|
              plain = setting_path_isolated(test_case, variant, source, role, observed: false)
              observed = setting_path_isolated(test_case, variant, source, role, observed: true)
              same = setting_path_comparable(plain) == setting_path_comparable(observed)
              check("setting path #{name} #{mode} #{variant} observer preserves application evidence", same)
              [variant, observed]
            end
            count = name == "success-rollback" ? 2 : nil
            same = setting_path_comparable(variants.fetch("reference"), count:) == setting_path_comparable(variants.fetch("final"), count:)
            check("setting path #{name} #{mode} paired evidence outside approved D4 correction", same)
            cases << { name:, mode:, accepted: same, equivalent: same && count.nil?,
                       approved_delta: count ? "ADR 588 D4: frozen third-call 42P07 versus final success" : nil }
          end
        end
        check("setting path declared source bytes unchanged", hashes == setting_path_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
                   source_hashes: hashes, candidate_sha256: @contract.expected_candidate_sha256,
                   final_sha256: @contract.final_sha256, postgres_image: @contract.postgres_image,
                   limits: ["Three observed paths are not complete helper/trigger or native closure.",
                            "NOTICE records are disposable test instrumentation, not production telemetry.",
                            "Sequence rollback and concurrent interleavings are not proved."], checks:, cases: }
        setting_path_write("report.json", JSON.pretty_generate(report) + "\n")
        @correction_evidence = previous
      end
      raise Failure, "setting path proof failed; retained original evidence" unless @checks.drop(first).all? { |entry| entry.fetch(:passed) }
    end

    def setting_path_write(name, bytes)
      raise Failure, "invalid setting evidence name" unless name.match?(/\A[a-zA-Z0-9_.-]+\z/)

      path = File.join(@setting_path_evidence, name)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL | File::NOFOLLOW, 0o600) { |file| file.write(bytes) }
      path
    end

    def setting_path_isolated(test_case, variant, source, role, observed:)
      name = "#{test_case.fetch(:name)}-#{test_case[:helpers] ? 'helpers-first' : 'cold'}-#{variant}-#{observed ? 'observed' : 'plain'}"
      database = "ingestion_setting_#{variant}"
      prefix = File.join(@setting_path_evidence, name)
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql"))
        seed += "\n" + validation_fixture_sql(validation_case("setting-inputs", nil))
        if test_case.fetch(:name) == "policy-regex-error"
          policy_case = policy_cases.find { |entry| entry.fetch(:name) == "invalid-regex" }
          seed += "\n" + File.binread(policy_seed_path) + "\n" + policy_rules_sql(policy_case)
        end
        seed_clock = policy_setup!(seed, database, "#{prefix}-seed")
        correction_observer!(database, variant)
        before = ingestion_snapshot(database)
        inputs = policy_read_snapshot(database)
        setting_path_observer!(database, name) if observed
        query = "\\set SHOW_CONTEXT always\n" + correction_session(test_case)
        setting_path_write("#{name}.sql", query)
        outcome = result(query, role:, database:)
        setting_path_write("#{name}.stdout", outcome.stdout)
        setting_path_write("#{name}.stderr", outcome.stderr)
        raise Failure, "setting path transport failed; retained exact output" unless outcome.success

        evidence = { "before" => before, "after" => ingestion_snapshot(database), "seed_clock" => seed_clock,
                     "inputs_before" => inputs, "inputs_after" => policy_read_snapshot(database),
                     "stdout" => outcome.stdout, "stderr" => outcome.stderr }
        frames, events = setting_path_parse(evidence, test_case, role)
        evidence.merge!("frames" => frames, "events" => events)
        bytes = JSON.pretty_generate(evidence) + "\n"
        path = setting_path_write("#{name}.json", bytes)
        setting_path_validate!(evidence, test_case, variant, role, observed:)
        check("setting path #{name} exact error, settings, context and persisted effects", true)
        @setting_path_validated_evidence[path] = Digest::SHA256.hexdigest(bytes)
        dependency_read_observation(path, @setting_path_validated_evidence).first
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def setting_path_observer!(database, name)
      query = <<~SQL
        CREATE FUNCTION ingestion_observation.trace_setting() RETURNS trigger
        LANGUAGE plpgsql SET search_path TO pg_catalog AS $observer$
        BEGIN
          RAISE NOTICE 'ingestion-setting:%', json_build_object(
            'relation', TG_TABLE_NAME, 'operation', TG_OP, 'backend', pg_backend_pid()::text,
            'session', session_user, 'current', current_user,
            'setting', current_setting('plpgsql.variable_conflict'), 'clock', transaction_timestamp());
          RETURN NEW;
        END;
        $observer$;
        REVOKE ALL ON FUNCTION ingestion_observation.trace_setting() FROM PUBLIC;
        #{IngestionCompilation::COMPILATION_OBSERVERS.map { |table| "CREATE TRIGGER ingestion_setting_trace BEFORE INSERT ON public.#{identifier(table)} FOR EACH ROW EXECUTE FUNCTION ingestion_observation.trace_setting();" }.join("\n")}
      SQL
      setting_path_write("#{name}-observer.sql", query)
      sql(query, role: "postgres", database:)
    end

    def setting_path_stack(role)
      routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |entry| entry.fetch("name") == "search_result_ingest_v1" }
      source = routine.fetch("source")
      statements = source.scan(/INSERT INTO canonical_torrent \(\n.*?RETURNING canonical_torrent_id INTO canonical_id;/m)
      line = source.lines.index { |value| value.strip == "INSERT INTO canonical_torrent (" }
      raise Failure, "setting observer frozen insertion site changed" unless statements.length == 1 && line == 422

      statement = statements.first.delete_suffix(" INTO canonical_id;")
      "SQL statement \"#{statement}\"\nPL/pgSQL function #{routine.fetch('signature')} line #{line + 1 + (role == @runtime ? 1 : 0)} at SQL statement\n"
    end

    def setting_path_notices(stderr, role:)
      events = []
      errors = []
      sequence = []
      stderr.split(/(?=^(?:NOTICE|ERROR):  )/).each do |record|
        if record.start_with?("ERROR:  ")
          errors << record
          sequence << ["error", record[/\AERROR:  ([A-Z0-9]{5}):/, 1]]
          next
        end
        match = record.match(/\ANOTICE:  00000: ingestion-setting:(?<event>\{[^\n]+\})\nCONTEXT:  PL\/pgSQL function ingestion_observation.trace_setting\(\) line 3 at RAISE\n(?<stack>.+)LOCATION:  exec_stmt_raise, pl_exec.c:3897\n\z/m)
        raise Failure, "unexpected setting notice or diagnostic" unless match
        raise Failure, "unexpected setting observer call stack" unless match[:stack] == setting_path_stack(role)

        event = JSON.parse(match[:event])
        events << event
        sequence << ["notice", event.fetch("clock")]
      end
      [events, errors.join, sequence]
    rescue JSON::ParserError
      raise Failure, "invalid setting notice JSON"
    end

    def setting_path_parse(evidence, test_case, role)
      events, errors, sequence = setting_path_notices(evidence.fetch("stderr"), role:)
      stdout = evidence.fetch("stdout")
      session = test_case
      if test_case[:helpers] && test_case.fetch(:name) == "policy-regex-error"
        first, stdout = stdout.split("\n", 2)
        raise Failure, "setting path helper answers missing or changed" unless first&.start_with?("helpers:") &&
          JSON.parse(first.delete_prefix("helpers:")) == ingestion_helper_expectations && stdout

        session = test_case.merge(helpers: false)
      end
      frames = if test_case.fetch(:name) == "policy-regex-error"
                 policy_error_parse(stdout, errors, session, role)
               else
                 parsed = correction_parse(stdout, errors, session)
                 parsed.reject { |frame| frame.fetch("state") == "00000" }.zip(ingestion_diagnostics(errors, role:)).each do |frame, diagnostic|
                   frame["diagnostic"] = diagnostic
                 end
                 parsed
               end
      expected = frames.each_with_index.flat_map do |frame, index|
        records = events.any? && index < 2 ? [["notice", frame.fetch("clock")]] : []
        records << ["error", frame.fetch("state")] unless frame.fetch("state") == "00000"
        records
      end
      raise Failure, "setting notice/error per-call ordering changed" unless sequence == expected

      [frames, events]
    rescue JSON::ParserError
      raise Failure, "invalid setting path helper JSON"
    end

    def setting_path_validate!(evidence, test_case, variant, role, observed:)
      frames, events = setting_path_parse(evidence, test_case, role)
      raise Failure, "setting path serialized evidence differs from original output" unless evidence.values_at("frames", "events") == [frames, events]
      raise Failure, "setting path caller role/GUC/backend or clocks changed" unless validation_context?(frames, role) &&
        frames.all? { |frame| frame.fetch("finished_setting") == "error" }
      raise Failure, "setting path read inputs changed" unless setting_path_inputs?(evidence, test_case)
      raise Failure, "setting path exact outcome or rollback changed" unless setting_path_outcomes?(evidence, test_case, variant, role)
      raise Failure, "setting path in-call evidence changed" unless setting_path_events?(frames, events, variant, role, observed:)

      true
    end

    def setting_path_inputs?(evidence, test_case)
      inputs = evidence.fetch("inputs_before")
      return false unless inputs == evidence.fetch("inputs_after") && inputs.keys.sort == IngestionPolicy::POLICY_READ_TABLES.sort &&
        validation_clock?(evidence.fetch("seed_clock")) && evidence.fetch("frames").none? { |frame| frame.fetch("clock") == evidence.fetch("seed_clock") }

      regex = test_case.fetch(:name) == "policy-regex-error"
      counts = IngestionPolicy::POLICY_READ_TABLES.to_h { |table| [table, 0] }.merge(
        "indexer_definition" => 1, "indexer_instance" => 1, "policy_snapshot" => 1,
        "search_request" => 1, "search_request_indexer_run" => 1, "trust_tier" => 4, "media_domain" => 7)
      counts.merge!("policy_snapshot" => 2, "search_request" => 2, "search_request_indexer_run" => 2,
        "indexer_instance_media_domain" => 1, "search_profile_tag_prefer" => 1, "indexer_instance_tag" => 1,
        "policy_snapshot_rule" => 1, "policy_rule" => 7, "policy_set" => 4, "policy_rule_value_set" => 6,
        "policy_rule_value_set_item" => 14, "search_profile" => 1, "tag" => 1) if regex
      return false unless inputs.all? { |table, rows| rows.is_a?(Array) && rows.length == counts.fetch(table) }
      return false unless inputs.fetch("trust_tier").map { |row| row.values_at("trust_tier_key", "rank") }.sort ==
        [["public", 10], ["semi_private", 20], ["private", 30], ["invite_only", 40]].sort
      return false unless inputs.fetch("media_domain").map { |row| row.fetch("media_domain_key") }.sort ==
        %w[adult_movies adult_scenes audiobooks ebooks movies software tv]

      # Check the original base fixture independently; retain the full regex
      # extension in both raw snapshots and the separate checks below.
      base = inputs.to_h do |table, rows|
        selected = %w[search_request policy_snapshot search_request_indexer_run].include?(table)
        [table, selected ? rows.select { |row| row.fetch(table == "search_request_indexer_run" ? "search_request_id" : "#{table}_id") == 569001 } : rows]
      end
      return false unless validation_inputs?(evidence.merge("inputs_before" => base, "inputs_after" => base), { fixture: {} })
      return false unless setting_path_input_clocks?(inputs, evidence.fetch("seed_clock"))

      !regex || setting_path_regex_inputs?(inputs)
    end

    def setting_path_input_clocks?(inputs, seed_clock)
      fixed = %w[policy_set policy_rule search_profile tag]
      updated = %w[indexer_definition indexer_instance policy_set policy_rule search_profile tag]
      (fixed + %w[indexer_definition indexer_instance search_request policy_snapshot trust_tier media_domain]).all? do |table|
        inputs.fetch(table).all? do |row|
          dated = fixed.include?(table) || (table == "policy_snapshot" && row.fetch("policy_snapshot_id") == 596001)
          expected = dated ? "2026-09-11T00:00:00+00:00" : seed_clock
          row.fetch("created_at", nil) == expected && (!updated.include?(table) || row.fetch("updated_at", nil) == expected)
        end
      end
    end

    def setting_path_regex_inputs?(inputs)
      request = inputs.fetch("search_request").find { |row| row.fetch("search_request_id") == 596001 }
      rule = inputs.fetch("policy_rule").find { |row| row.fetch("policy_rule_public_id") == "59600000-0000-4000-8000-000000001001" }
      snapshot = inputs.fetch("policy_snapshot").find { |row| row.fetch("policy_snapshot_id") == 596001 }
      return false unless request && rule && snapshot
      return false unless request.values_at("search_request_public_id", "policy_snapshot_id", "search_profile_id", "status", "query_text", "page_size", "canceled_at", "finished_at", "failure_class") ==
        [IngestionPolicy::POLICY_REQUEST, 596001, 596001, "running", "D3 populated policy", 10, nil, nil, nil] && snapshot.fetch("snapshot_hash") == "d" * 64
      return false unless rule.values_at("policy_rule_id", "policy_set_id", "rule_type", "match_field", "match_operator", "match_value_text", "match_value_int", "match_value_uuid", "value_set_id", "action", "severity", "is_case_insensitive", "is_disabled", "expires_at") ==
        [1, 596004, "block_title_regex", "title", "regex", "[", nil, nil, nil, "flag", "soft", true, false, nil]
      return false unless inputs.fetch("policy_snapshot_rule").map { |row| row.values_at("policy_snapshot_id", "policy_rule_public_id", "rule_order") } ==
        [[596001, "59600000-0000-4000-8000-000000001001", 10]]
      return false unless inputs.fetch("policy_set").map { |row| row.values_at("policy_set_id", "scope", "is_enabled", "deleted_at") }.sort ==
        (1..4).zip(%w[request profile user global]).map { |id, scope| [596000 + id, scope, true, nil] }
      return false unless inputs.fetch("search_request_indexer_run").map { |row| row.values_at("search_request_id", "indexer_instance_id", "status") }.sort ==
        [[569001, 569001, "queued"], [596001, 569001, "queued"]]

      policy_value_sets?(inputs)
    end

    def setting_path_outcomes?(evidence, test_case, variant, role)
      frames = evidence.fetch("frames")
      if test_case.fetch(:name) == "success-rollback"
        control = validation_case("null-arrays", nil)
        return validation_states?(frames, control, variant, role) && validation_images?(evidence, control) &&
          validation_lifetime?(frames, control, variant) && validation_control?(frames)
      end
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      return false unless evidence.values_at("before", "after") == [empty, empty] && frames.all? do |frame|
        frame.values_at("tables_before", "tables_after", "tables_finish") == [empty, empty, empty] &&
          frame.values_at("within", "outside") == %w[false false] && !frame.key?("result")
      end

      if test_case.fetch(:name) == "late-validation-error"
        expected = validation_guard_diagnostic(:year)
        frames.all? { |frame| frame.fetch("state") == "P0001" && frame.fetch("diagnostic") == expected }
      else
        expected = { "state" => "2201B", "message" => "invalid regular expression: brackets [] not balanced",
                     "line" => 1278, "helper_line" => 14, "location" => "RE_compile_and_cache, regexp.c:222" }
        frames.all? { |frame| frame.fetch("state") == "2201B" && frame.fetch("diagnostic") == expected }
      end
    end

    def setting_path_events?(frames, events, variant, role, observed:)
      return events.empty? unless observed

      # The third successful-control call reuses the committed canonical row.
      expected = frames.take(2).map do |frame|
        { "relation" => "canonical_torrent", "operation" => "INSERT", "backend" => frame.fetch("backend"),
          "session" => role, "current" => variant == "reference" ? "postgres" : @owner,
          "setting" => variant == "reference" ? "use_column" : "error", "clock" => frame.fetch("clock") }
      end
      events == expected
    end

    def setting_path_comparable(evidence, count: nil)
      frames = evidence.fetch("frames")
      selected = count ? frames.take(count) : frames
      { frames: compilation_comparable("fixture" => nil, "frames" => selected),
        inputs: policy_comparable_inputs(evidence.merge("seed_clocks" => { "base" => evidence.fetch("seed_clock") })) }
    end
  end
end
