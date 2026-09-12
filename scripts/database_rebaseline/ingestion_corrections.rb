# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # D4/D5 regression evidence, independent of the incomplete D3 certificate.
  module IngestionCorrections
    CORRECTION_IDS = { "imdb" => "tt1234567", "tmdb" => 123_456, "tvdb" => 654_321 }.freeze
    CORRECTION_RECORDS = %w[backend role clock before tables_before state within after tables_after outside tables_finish].freeze

    private

    def verify_ingestion_corrections!
      @correction_evidence = File.join(@contract.output_path, "ingestion-corrections")
      raise Failure, "ingestion correction evidence must not be a symlink" if File.symlink?(@correction_evidence)

      FileUtils.mkdir_p(@correction_evidence, mode: 0o700)
      cases = []
      first_check = @checks.length
      complete = false
      begin
        correction_cases.each do |test_case|
          { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
            cases << correction_isolated(test_case, variant, source, role)
          end
        end
        control = correction_cases.find { |test_case| test_case.fetch(:name) == "rollback-retry" }
        cases << correction_isolated(control.merge(name: "rollback-negative-control", commit_control: true), "final", @database, @runtime)
        complete = true
      ensure
        checks = @checks.drop(first_check)
        report = { completed: complete, passed: complete && checks.all? { |entry| entry.fetch(:passed) },
                   d3_complete: false, reference_failures_are_application_success: false,
                   final_sha256: @contract.final_sha256, candidate_sha256: @contract.expected_candidate_sha256,
                   image: @contract.postgres_image, checks:, cases: }
        File.binwrite(File.join(@correction_evidence, "report.json"), JSON.pretty_generate(report) + "\n")
      end
    end

    def correction_id_arguments(id, invalid: false)
      value = CORRECTION_IDS.fetch(id)
      text = id == "imdb"
      value = text ? "bad" : 0 if invalid
      {
        attr_keys_input: "ARRAY['#{id}_id']::public.observation_attr_key[]",
        attr_types_input: "ARRAY['#{text ? 'text' : 'int'}']::public.attr_value_type[]",
        attr_value_text_input: "ARRAY[#{text ? literal(value) : 'NULL'}]::varchar[]",
        attr_value_int_input: "ARRAY[#{text ? 'NULL' : value}]::integer[]",
        attr_value_bigint_input: "ARRAY[NULL]::bigint[]", attr_value_numeric_input: "ARRAY[NULL]::numeric[]",
        attr_value_bool_input: "ARRAY[NULL]::boolean[]", attr_value_uuid_input: "ARRAY[NULL]::uuid[]"
      }
    end

    def correction_cases
      later = { observed_at_input: "'2026-09-10T00:01:00Z'::timestamptz", seeders_input: "17" }
      cases = [
        { name: "committed-reuse", calls: [{}, later] },
        { name: "wrapper-reuse", calls: [{}, later], wrapper: true },
        { name: "helpers-first-reuse", calls: [{}, later], helpers: true },
        { name: "rollback-retry", calls: [{}, later], rollback: true },
        { name: "invalid-retry", calls: [{ search_request_public_id_input: "NULL::uuid" }, later] },
        { name: "same-transaction", calls: [{}, later], same_transaction: true },
        { name: "policy-change", calls: [{}, later.merge(search_request_public_id_input: "'58800000-0000-4000-8000-000000000003'::uuid")] }
      ]
      CORRECTION_IDS.each_key do |id|
        args = correction_id_arguments(id)
        cases << { name: "#{id}-upsert", calls: [args, args.merge(later)], id: }
        cases << { name: "#{id}-invalid", calls: [correction_id_arguments(id, invalid: true)], invalid: true, id: }
      end
      cases
    end

    def correction_states(test_case, variant)
      return ["P0001"] if test_case[:invalid]
      return %w[P0001 00000] if test_case.fetch(:name) == "invalid-retry"
      return %w[42P10 42P10] if test_case[:id] && variant == "reference"
      return %w[00000 42P07] if test_case[:same_transaction] || (variant == "reference" && !test_case[:rollback])

      %w[00000 00000]
    end

    def correction_isolated(test_case, variant, source, role)
      database = "ingestion_correction_#{variant}"
      name = "#{test_case.fetch(:name)}-#{variant}"
      prefix = File.join(@correction_evidence, name)
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        %w[database-ingestion-proof-seed.sql database-ingestion-corrections-seed.sql].each do |fixture|
          sql(File.binread(File.join(@contract.root, "scripts/tests", fixture)), role: "postgres", database:)
        end
        correction_observer!(database, variant)
        before = ingestion_snapshot(database)
        query = correction_session(test_case)
        File.binwrite("#{prefix}.sql", query)
        outcome = result(query, role:, database:)
        File.binwrite("#{prefix}.stdout", outcome.stdout)
        File.binwrite("#{prefix}.stderr", outcome.stderr)
        raise Failure, "ingestion correction transport failed" unless outcome.success

        frames = correction_parse(outcome.stdout, outcome.stderr, test_case)
        after = ingestion_snapshot(database)
        evidence = { name:, role:, before:, frames:, after:, application_succeeded: frames.all? { |frame| frame.fetch("state") == "00000" } }
        File.binwrite("#{prefix}.json", JSON.pretty_generate(evidence) + "\n")
        correction_verify!(name, test_case, variant, role, before, frames, after)
        evidence
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def correction_observer!(database, variant)
      owner = variant == "reference" ? "postgres" : @owner
      pairs = IngestionProof::INGESTION_TABLES.map do |table|
        "#{literal(table)}, (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)->(SELECT a.attname FROM pg_attribute a WHERE a.attrelid = 'public.#{table}'::regclass AND a.attnum = 1)), '[]') FROM public.#{identifier(table)} t)"
      end
      query = <<~SQL
        CREATE SCHEMA ingestion_observation AUTHORIZATION #{identifier(@owner)};
        REVOKE ALL ON SCHEMA ingestion_observation FROM PUBLIC;
        CREATE FUNCTION ingestion_observation.snapshot() RETURNS json
        LANGUAGE sql STABLE SECURITY DEFINER SET search_path TO pg_catalog
        AS $$ SELECT json_build_object(#{pairs.join(',')}); $$;
        ALTER FUNCTION ingestion_observation.snapshot() OWNER TO #{identifier(owner)};
        REVOKE ALL ON FUNCTION ingestion_observation.snapshot() FROM PUBLIC;
        GRANT USAGE ON SCHEMA ingestion_observation TO #{identifier(@runtime)};
        GRANT EXECUTE ON FUNCTION ingestion_observation.snapshot() TO #{identifier(@runtime)};
      SQL
      File.binwrite(File.join(@correction_evidence, "observer-#{variant}.sql"), query)
      sql(query, role: "postgres", database:)
    end

    def correction_session(test_case)
      pieces = ["\\set VERBOSITY verbose", "SET statement_timeout = '120s';", "DO $$ BEGIN NULL; END $$;"]
      if test_case[:helpers]
        pieces << File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-helper-first.sql"))
      end
      test_case.fetch(:calls).each_with_index do |arguments, index|
        call = block_given? ? yield(arguments) : ingestion_call(arguments)
        call = call.sub("public.search_result_ingest_v1(", "public.search_result_ingest(") if test_case[:wrapper]
        finish = test_case[:rollback] && index.zero? && !test_case[:commit_control] ? "ROLLBACK" : "COMMIT"
        finish = "" if test_case[:same_transaction] && index.zero?
        pieces << <<~SQL
          #{test_case[:same_transaction] && index.positive? ? '' : 'BEGIN;'}
          SELECT 'backend:' || pg_backend_pid();
          SELECT 'role:' || json_build_object('session', session_user, 'current', current_user,
            'superuser', r.rolsuper, 'create_role', r.rolcreaterole, 'bypass_rls', r.rolbypassrls)::text
            FROM pg_roles r WHERE r.rolname = current_user;
          SELECT 'clock:' || to_json(transaction_timestamp())::text;
          SELECT 'before:' || current_setting('plpgsql.variable_conflict');
          SELECT 'tables_before:' || ingestion_observation.snapshot()::text;
          SAVEPOINT operation;
          \\set ON_ERROR_STOP off
          #{call}
          \\echo state: :SQLSTATE
          \\if :ERROR
          ROLLBACK TO SAVEPOINT operation;
          \\endif
          \\set ON_ERROR_STOP on
          SELECT 'within:' || (to_regclass('pg_temp.tmp_policy_rules') IS NOT NULL)::text;
          SELECT 'after:' || current_setting('plpgsql.variable_conflict');
          SELECT 'tables_after:' || ingestion_observation.snapshot()::text;
          #{finish.empty? ? '' : "#{finish};"}
          SELECT 'outside:' || #{finish.empty? ? "'not-committed'" : "(to_regclass('pg_temp.tmp_policy_rules') IS NOT NULL)::text"};
          SELECT 'tables_finish:' || ingestion_observation.snapshot()::text;
          #{test_case[:finish_setting] ? "SELECT 'finished_setting:' || current_setting('plpgsql.variable_conflict');" : ''}
        SQL
      end
      pieces.join("\n")
    end

    def correction_parse(stdout, stderr, test_case)
      lines = stdout.lines(chomp: true)
      if test_case[:helpers]
        line = lines.shift
        unless line&.start_with?("helpers:") && JSON.parse(line.delete_prefix("helpers:")) == ingestion_helper_expectations
          raise Failure, "ingestion correction helper-first answers missing or changed"
        end
      end
      frames = test_case.fetch(:calls).map do |_call|
        frame = {}
        correction_records(test_case).each do |key|
          if key == "state" && lines.first&.start_with?("{")
            frame["result"] = JSON.parse(lines.shift)
          end
          line = lines.shift
          raise Failure, "ingestion correction record missing or reordered: #{key}" unless line&.start_with?("#{key}:")

          value = line.delete_prefix("#{key}:")
          frame[key] = if %w[role clock tables_before tables_after tables_finish].include?(key)
                         JSON.parse(value)
                       else
                         value.strip
                       end
        end
        unless frame.fetch("state").match?(/\A[A-Z0-9]{5}\z/) && frame.key?("result") == (frame.fetch("state") == "00000")
          raise Failure, "ingestion correction result framing changed"
        end
        frame
      end
      raise Failure, "ingestion correction unexpected stdout" unless lines.empty?

      expected = frames.map { |frame| frame.fetch("state") }.reject { |state| state == "00000" }
      diagnostics = correction_diagnostics(stderr, wrapper: test_case[:wrapper] == true)
      raise Failure, "ingestion correction unexpected diagnostic" unless diagnostics.map { |entry| entry.fetch("state") } == expected

      frames.reject { |frame| frame.fetch("state") == "00000" }.zip(diagnostics).each do |frame, diagnostic|
        frame["diagnostic"] = diagnostic
      end
      frames
    rescue JSON::ParserError
      raise Failure, "ingestion correction invalid JSON evidence"
    end

    def correction_records(test_case)
      test_case[:finish_setting] ? CORRECTION_RECORDS + ["finished_setting"] : CORRECTION_RECORDS
    end

    def correction_diagnostics(stderr, wrapper:)
      stderr.split(/(?=^ERROR:  )/).map do |record|
        match = record.match(/\AERROR:  (?<state>[A-Z0-9]{5}): (?<message>[^\n]+)\n(?:DETAIL:  (?<detail>[^\n]+)\n)?CONTEXT:  (?:SQL statement "[^"]*"\n)?PL\/pgSQL function search_result_ingest_v1\([^\n]+\) line [1-9][0-9]* at (?:RAISE|SQL statement)\n(?<wrapper>SQL statement "[^"]*"\nPL\/pgSQL function search_result_ingest\([^\n]+\) line [1-9][0-9]* at SQL statement\n)?LOCATION:  [A-Za-z0-9_]+, [A-Za-z0-9_]+\.c:[1-9][0-9]*\n\z/)
        unless match && !match[:wrapper].nil? == wrapper && !record.match?(/\b(?:WARNING|NOTICE|FATAL|PANIC)\b/)
          raise Failure, "ingestion correction unexpected diagnostic"
        end

        match.named_captures
      end
    end

    def correction_verify!(name, test_case, variant, role, before, frames, after)
      expected = correction_states(test_case, variant)
      check("#{name} exact SQLSTATE sequence (reference errors remain failures)", frames.map { |frame| frame.fetch("state") } == expected)
      details = frames.filter_map { |frame| frame.fetch("diagnostic", {}).fetch("detail", nil) }
      required_details = test_case[:invalid] ? ["attr_value_invalid"] : (test_case.fetch(:name) == "invalid-retry" ? ["search_request_missing"] : [])
      check("#{name} exact rejection details", details == required_details)
      check("#{name} unchanged connected backend", frames.map { |frame| frame.fetch("backend") }.uniq.length == 1 && frames.first.fetch("backend").match?(/\A[1-9][0-9]*\z/))
      capabilities = { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" }
      check("#{name} unchanged authority and caller setting", frames.all? { |frame| frame.fetch("role") == capabilities && frame.values_at("before", "after") == %w[error error] })
      clocks = frames.map { |frame| frame.fetch("clock") }
      check("#{name} transaction provenance", clocks.all? { |clock| clock.is_a?(String) && clock.match?(/\A\d{4}-\d\d-\d\dT.*\+00:00\z/) } && clocks.uniq.length == (test_case[:same_transaction] ? 1 : frames.length))
      check("#{name} empty initial application state", before.values.all?(&:empty?))
      check("#{name} complete table and rollback evidence", correction_snapshots?(test_case, before, frames, after))
      if test_case[:commit_control]
        check("#{name} actual commit rejected by rollback predicate", !correction_snapshots?(test_case.merge(commit_control: false), before, frames, after))
      end
      if variant == "final"
        outside = test_case[:same_transaction] ? %w[not-committed false] : Array.new(frames.length, "false")
        within = test_case[:same_transaction] ? %w[true true] : expected.map { |state| state == "00000" ? "true" : "false" }
        check("#{name} exact policy table lifetime", frames.map { |frame| frame.fetch("outside") } == outside && frames.map { |frame| frame.fetch("within") } == within)
        check("#{name} exact application result and identity links", correction_results?(test_case, frames, after))
      end
      if test_case.fetch(:name) == "policy-change"
        decisions = after.fetch("search_filter_decision")
        check("#{name} new policy snapshot applied", decisions.length == (variant == "final" ? 1 : 0) && decisions.all? { |row| row.fetch("policy_rule_public_id") == "58800000-0000-4000-8000-000000000005" })
      end
    end

    def correction_snapshots?(test_case, before, frames, after)
      images = frames.flat_map { |frame| frame.values_at("tables_before", "tables_after", "tables_finish") } + [before, after]
      return false unless images.all? { |tables| tables.keys.sort == IngestionProof::INGESTION_TABLES.sort }
      return false unless frames.first.fetch("tables_before") == before && frames.last.fetch("tables_finish") == after

      frames.each_with_index.all? do |frame, index|
        prior, changed, finished = frame.values_at("tables_before", "tables_after", "tables_finish")
        continuity = index.zero? || prior == frames.fetch(index - 1).fetch("tables_finish")
        failed_preserved = frame.fetch("state") == "00000" || prior == changed
        rolled_back = test_case[:rollback] && index.zero? && !test_case[:commit_control]
        continuity && failed_preserved && (rolled_back ? finished == before && changed != before : finished == changed)
      end
    end

    def correction_results?(test_case, frames, after)
      successes = frames.select { |frame| frame.fetch("state") == "00000" }
      return after.values.all?(&:empty?) if successes.empty?

      return false unless successes.all? do |frame|
        tables = frame.fetch("tables_after")
        %w[canonical_torrent canonical_torrent_source].all? do |table|
          rows = tables.fetch(table)
          rows.length == 1 && rows.first.fetch("#{table}_public_id").match?(IngestionProof::INGESTION_UUID) &&
            rows.first.fetch("#{table}_public_id") == frame.fetch("result").fetch("#{table}_public_id")
        end && frame.fetch("result").keys.sort == %w[canonical_changed canonical_torrent_public_id canonical_torrent_source_public_id durable_source_created observation_created].sort && correction_flags?(test_case, frame)
      end
      return true unless test_case[:id]
      return false unless successes.length == 2

      id = test_case.fetch(:id)
      first, second = successes.map { |frame| frame.fetch("tables_after").fetch("canonical_external_id") }
      return false unless first.length == 1 && second.length == 1

      old, fresh = first.first, second.first
      old.merge("last_seen_at" => "2026-09-10T00:01:00+00:00") == fresh &&
        old.fetch("first_seen_at") == "2026-09-10T00:00:00+00:00" &&
        fresh.values_at("id_type", id == "imdb" ? "id_value_text" : "id_value_int") == [id, CORRECTION_IDS.fetch(id)] &&
        fresh.fetch("source_canonical_torrent_source_id") == after.fetch("canonical_torrent_source").first.fetch("canonical_torrent_source_id") &&
        fresh.fetch("canonical_torrent_id") == after.fetch("canonical_torrent").first.fetch("canonical_torrent_id") &&
        after.fetch("canonical_torrent").first.fetch("#{id}_id") == CORRECTION_IDS.fetch(id)
    end

    def correction_flags?(test_case, frame)
      before = frame.fetch("tables_before")
      first = before.fetch("canonical_torrent").empty?
      observation = first || test_case.fetch(:name) == "policy-change"
      frame.fetch("result").values_at("observation_created", "durable_source_created", "canonical_changed") == [observation, first, first]
    end
  end
end
