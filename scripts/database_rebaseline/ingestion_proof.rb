# frozen_string_literal: true

require "json"
require_relative "ingestion_existing"
require_relative "ingestion_approved_deltas"
require_relative "ingestion_compilation"
require_relative "ingestion_wrapper"
require_relative "ingestion_policy"
require_relative "ingestion_dependencies"
require_relative "ingestion_validation"

module RevaerDatabaseRebaseline
  # Uses FinalProof's private transport and evidence owner, never runtime SQL.
  module IngestionProof
    include IngestionExisting
    include IngestionApprovedDeltas
    include IngestionCompilation
    include IngestionWrapper
    include IngestionPolicy
    include IngestionDependencies
    include IngestionValidation
    INGESTION_TABLES = %w[
      canonical_torrent canonical_torrent_source canonical_torrent_source_attr
      canonical_torrent_source_context_score canonical_torrent_best_source_context
      search_request_source_observation search_request_source_observation_attr
      canonical_torrent_signal canonical_external_id canonical_size_sample
      canonical_size_rollup search_request_canonical search_page search_page_item
      search_filter_decision source_metadata_conflict source_metadata_conflict_audit_log
      indexer_health_event
    ].freeze
    INGESTION_HELPERS = %w[
      search_result_ingest search_result_ingest_v1 normalize_title_v1 normalize_magnet_uri_v1
      derive_magnet_hash_v1 compute_title_size_hash_v1 policy_text_match_v1
      policy_uuid_match_v1 policy_int_match_v1 policy_release_group_match_v1
      log_source_metadata_conflict_v1 policy_action_to_decision_type
    ].freeze
    INGESTION_UUID = /\A[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/
    INGESTION_PENDING = [
      "complete ingestion branch and helper execution matrix",
      "mutating helper-first compilation and complete warm-cache call paths",
      "in-call setting observations and reachable native/trigger/dynamic-call closure"
    ].freeze
    INGESTION_ARGUMENTS = {
      search_request_public_id_input: "'56900000-0000-4000-8000-000000000002'::uuid",
      indexer_instance_public_id_input: "'56900000-0000-4000-8000-000000000001'::uuid",
      source_guid_input: "'ingestion-proof-source'::varchar",
      details_url_input: "NULL::varchar", download_url_input: "NULL::varchar",
      magnet_uri_input: "NULL::varchar", title_raw_input: "'Ingestion proof title'::varchar",
      size_bytes_input: "1024::bigint", infohash_v1_input: "repeat('a', 40)::char(40)",
      infohash_v2_input: "NULL::char(64)", magnet_hash_input: "NULL::char(64)",
      seeders_input: "5", leechers_input: "2", published_at_input: "NULL::timestamptz",
      uploader_input: "NULL::varchar", observed_at_input: "'2026-09-10T00:00:00Z'::timestamptz",
      attr_keys_input: "NULL::public.observation_attr_key[]", attr_types_input: "NULL::public.attr_value_type[]",
      attr_value_text_input: "NULL::varchar[]", attr_value_int_input: "NULL::integer[]",
      attr_value_bigint_input: "NULL::bigint[]", attr_value_numeric_input: "NULL::numeric[]",
      attr_value_bool_input: "NULL::boolean[]", attr_value_uuid_input: "NULL::uuid[]"
    }.freeze

    private

    def verify_ingestion_parity!
      @contract.validate_output_path!
      @ingestion_evidence = File.join(@contract.output_path, "ingestion-proof")
      raise Failure, "ingestion evidence directory must not be a symlink" if File.symlink?(@ingestion_evidence)

      FileUtils.mkdir_p(@ingestion_evidence, mode: 0o700)
      @ingestion_results = []
      @ingestion_inventory = {}
      @ingestion_stop = "proof interrupted before acceptance"
      ingestion_inventory!
      verify_ingestion_compilation!
      verify_ingestion_wrapper!
      verify_ingestion_policy!
      verify_ingestion_validation!
      verify_ingestion_dependencies!
      verify_ingestion_session_controls!
      verify_existing_ingestion_parity!
      ingestion_cases.each do |name, changes, expected, helpers_first|
        helpers_first = helpers_first == true
        query = ingestion_session(changes, repeat: name == "warm-committed", helpers_first:)
        File.binwrite(File.join(@ingestion_evidence, "#{name}.sql"), query)
        reference = ingestion_isolated(name, query, source: "reference_proof", role: "postgres", variant: "reference", helpers_first:)
        final = ingestion_isolated(name, query, source: @database, role: @runtime, variant: "final", helpers_first:)
        equivalent = reference == final
        approved_delta = ingestion_approved_delta(name, reference, final)
        admissible = equivalent || !approved_delta.nil?
        accepted = admissible && final.fetch("states") == expected
        @ingestion_results << { name:, expected:, helpers_first:, reference:, final:, equivalent:, approved_delta:, accepted: }
        check("ingestion #{name} parity or exact approved correction", admissible)
        check("ingestion #{name} required outcome", accepted)
        next if accepted

        @ingestion_stop = if equivalent
                            "shared reference/final failure at #{name}; not a demonstrated D3-only regression"
                          else
                            "reference/final behavior differs at #{name}; further semantic review required"
                          end
        raise Failure, "#{@ingestion_stop}; retained ingestion-proof evidence"
      end
      @ingestion_stop = "mandatory counterexamples passed, but the complete D3 scope remains unproven"
      check("ingestion complete conditional D3 proof", false)
      raise Failure, @ingestion_stop
    rescue Failure => error
      @ingestion_stop = error.message
      raise
    ensure
      ingestion_report! if @ingestion_results
    end

    def ingestion_inventory!
      query = <<~SQL
        SELECT json_build_object('version', current_setting('server_version_num'),
          'routines', (SELECT json_agg(row_to_json(r) ORDER BY r.name) FROM (
            SELECT p.proname AS name, p.oid::regprocedure::text AS signature,
              p.prosrc AS source, p.proconfig AS settings, p.prosecdef AS definer,
              pg_get_userbyid(p.proowner) AS owner
            FROM pg_proc p WHERE p.pronamespace = 'public'::regnamespace
              AND p.proname IN (#{INGESTION_HELPERS.map { |name| literal(name) }.join(',')})
          ) r),
          'triggers', (SELECT COALESCE(json_agg(pg_get_triggerdef(t.oid) ORDER BY t.oid), '[]')
            FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid
            WHERE c.relnamespace = 'public'::regnamespace AND NOT t.tgisinternal
              AND c.relname IN (#{INGESTION_TABLES.map { |name| literal(name) }.join(',')})));
      SQL
      ["reference_proof", @database].each do |database|
        value = sql(query, role: "postgres", database:)
        File.binwrite(File.join(@ingestion_evidence, "#{database}-inventory.json"), value + "\n")
        @ingestion_inventory[database] = JSON.parse(value)
        raise Failure, "ingestion proof requires PostgreSQL 16.14" unless @ingestion_inventory.fetch(database).fetch("version") == "160014"
      end
      reference = @ingestion_inventory.fetch("reference_proof").fetch("routines")
      final = @ingestion_inventory.fetch(@database).fetch("routines")
      expected_names = INGESTION_HELPERS.sort
      unless reference.map { |row| row.fetch("name") }.sort == expected_names && final.map { |row| row.fetch("name") }.sort == expected_names
        raise Failure, "ingestion reviewed routine inventory changed"
      end
      reference.zip(final).each do |old, fresh|
        same_body = old.fetch("source") == fresh.fetch("source")
        if old.fetch("name") == "search_result_ingest_v1"
          approved = FinalSql.new(@contract).approved_ingestion_body(old.fetch("source"))
          same_body = "\n#variable_conflict use_column\n#{approved.delete_prefix("\n")}" == fresh.fetch("source")
          unless Array(old.fetch("settings")).include?("plpgsql.variable_conflict=use_column") &&
                 Array(fresh.fetch("settings")).none? { |entry| entry.start_with?("plpgsql.variable_conflict=") }
            raise Failure, "ingestion reference must retain its GUC and final only its local directive"
          end
        end
        raise Failure, "ingestion function body or signature changed beyond D3/D4/D5" unless same_body && old.fetch("signature") == fresh.fetch("signature")
      end
      check("ingestion frozen body retained independently of approved D3/D4/D5 deltas", true)
    end

    def ingestion_cases
      cold = [
        ["request-missing", { search_request_public_id_input: "NULL::uuid" }, ["P0001"]],
        ["instance-missing", { indexer_instance_public_id_input: "NULL::uuid" }, ["P0001"]],
        ["new-v1", {}, ["00000"]],
        ["new-v2", { infohash_v2_input: "repeat('b', 64)::char(64)" }, ["00000"]],
        ["new-magnet", { infohash_v1_input: "NULL::char(40)", magnet_uri_input: "'magnet:?dn=Proof&xt=opaque'::varchar" }, ["00000"]],
        ["new-title-size", { infohash_v1_input: "NULL::char(40)" }, ["00000"]]
      ]
      cold + cold.map { |name, changes, expected| ["helpers-first-#{name}", changes, expected, true] } +
        [["warm-committed", {}, ["00000", "00000"]]]
    end

    def ingestion_call(changes)
      unknown = changes.keys - INGESTION_ARGUMENTS.keys
      raise Failure, "unknown ingestion fixture argument" unless unknown.empty?

      arguments = INGESTION_ARGUMENTS.merge(changes).map { |name, value| "#{name} => #{value}" }.join(",\n")
      "SELECT row_to_json(r) FROM public.search_result_ingest_v1(#{arguments}) r;"
    end

    def ingestion_session(changes, repeat: false, helpers_first: false)
      second = if repeat
                 <<~SQL
                   COMMIT;
                   BEGIN;
                   SELECT 'clock:' || to_json(transaction_timestamp())::text;
                   SAVEPOINT ingestion_second;
                   \\set ON_ERROR_STOP off
                   #{ingestion_call(changes)}
                   \\echo state: :SQLSTATE
                   \\set ON_ERROR_STOP on
                   \\if :ERROR
                   ROLLBACK TO SAVEPOINT ingestion_second;
                   \\endif
                 SQL
               else
                 "\\if :ERROR\nROLLBACK TO SAVEPOINT ingestion_call;\n\\endif"
               end
      <<~SQL
        \\set VERBOSITY verbose
        SET statement_timeout = '120s';
        DO $$ BEGIN NULL; END $$;
        BEGIN;
        SELECT 'clock:' || to_json(transaction_timestamp())::text;
        SELECT 'role:' || json_build_object('session', session_user, 'current', current_user,
          'superuser', r.rolsuper, 'create_role', r.rolcreaterole, 'bypass_rls', r.rolbypassrls)::text
          FROM pg_roles r WHERE r.rolname = current_user;
        SELECT 'before:' || COALESCE(current_setting('plpgsql.variable_conflict', true), '<unloaded>');
        #{helpers_first ? File.binread(File.expand_path('../tests/database-ingestion-helper-first.sql', __dir__)) : ''}
        SAVEPOINT ingestion_call;
        \\set ON_ERROR_STOP off
        #{ingestion_call(changes)}
        \\echo state: :SQLSTATE
        \\set ON_ERROR_STOP on
        #{second}
        SELECT 'after:' || COALESCE(current_setting('plpgsql.variable_conflict', true), '<unloaded>');
        COMMIT;
      SQL
    end

    def verify_ingestion_session_controls!
      parts = ingestion_session({}, repeat: true).split(ingestion_call({}), -1)
      raise Failure, "ingestion warm control must replace exactly two calls" unless parts.length == 3

      observations = ["2", "3", "1/0"].map do |second_value|
        query = <<~SQL
          CREATE TEMP TABLE ingestion_harness_counter (value integer NOT NULL);
          #{parts.fetch(0)}
          INSERT INTO ingestion_harness_counter VALUES (1);
          #{parts.fetch(1)}
          INSERT INTO ingestion_harness_counter VALUES (#{second_value});
          #{parts.fetch(2)}
          SELECT 'writes:' || COALESCE(string_agg(value::text, ',' ORDER BY value), '')
            FROM ingestion_harness_counter;
        SQL
        File.binwrite(File.join(@ingestion_evidence, "session-control-#{second_value.tr('/', '-')}.sql"), query)
        outcome = result(query, role: "postgres", database: "reference_proof")
        prefix = File.join(@ingestion_evidence, "session-control-#{second_value.tr('/', '-')}")
        File.binwrite("#{prefix}.stdout", outcome.stdout)
        File.binwrite("#{prefix}.stderr", outcome.stderr)
        ingestion_control_records(outcome, failure_expected: second_value == "1/0")
      end
      check("ingestion warm control commits successful second writes", observations.fetch(0) == ["writes:1,2"])
      check("ingestion warm control exposes different second writes", observations.fetch(1) == ["writes:1,3"] && observations.fetch(0) != observations.fetch(1))
      check("ingestion warm control rolls back only failed second writes", observations.fetch(2) == ["writes:1"])
    end

    def ingestion_control_records(outcome, failure_expected:)
      raise Failure, "ingestion session control failed" unless outcome.success

      lines = outcome.stdout.lines.map(&:strip)
      expected_states = ["state: 00000", failure_expected ? "state: 22012" : "state: 00000"]
      unless lines.grep(/\Astate:/) == expected_states
        raise Failure, "ingestion session control SQLSTATE mismatch"
      end
      diagnostics_match = if failure_expected
                            outcome.stderr.match?(/\AERROR:\s+22012: division by zero\nLOCATION:\s+int4div, int\.c:[0-9]+\n\z/)
                          else
                            outcome.stderr.empty?
                          end
      raise Failure, "ingestion session control unexpected diagnostic" unless diagnostics_match

      lines.select { |line| line.start_with?("writes:") }
    end

    def ingestion_isolated(name, query, source:, role:, variant:, helpers_first: false)
      database = "ingestion_#{variant}_proof"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.expand_path("../tests/database-ingestion-proof-seed.sql", __dir__))
        sql(seed, role: "postgres", database:)
        before = ingestion_snapshot(database)
        value = ingestion_execute(name, query, role:, database:, variant:, helpers_first:)
        after = ingestion_snapshot(database)
        snapshots = { "before" => before, "after" => after }
        File.write(File.join(@ingestion_evidence, "#{name}-#{variant}-tables.json"), JSON.pretty_generate(snapshots) + "\n")
        ingestion_comparable(value.merge(snapshots))
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def ingestion_snapshot(database)
      pairs = INGESTION_TABLES.map do |table|
        "#{literal(table)}, (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)->(SELECT a.attname FROM pg_attribute a WHERE a.attrelid = 'public.#{table}'::regclass AND a.attnum = 1)), '[]') FROM public.#{identifier(table)} t)"
      end
      JSON.parse(sql("SELECT json_build_object(#{pairs.join(',')});", role: "postgres", database:))
    end

    def ingestion_execute(name, query, role:, database:, variant:, helpers_first: false)
      outcome = result(query, role:, database:)
      File.binwrite(File.join(@ingestion_evidence, "#{name}-#{variant}.stdout"), outcome.stdout)
      File.binwrite(File.join(@ingestion_evidence, "#{name}-#{variant}.stderr"), outcome.stderr)
      raise Failure, "ingestion proof transport failed" unless outcome.success && !outcome.stderr.match?(/\bWARNING\b/)

      ingestion_parse(outcome.stdout, outcome.stderr, role:, helpers_first:)
    end

    def ingestion_parse(stdout, stderr, role:, helpers_first: false)
      lines = stdout.lines.map(&:strip)
      unless lines.all? { |line| line.match?(/\A(?:(?:clock:|role:|before:|helpers:|after:|\{).*|state: [0-9A-Z]{5})\z/) }
        raise Failure, "ingestion stdout contains an unrecognized record"
      end
      states = lines.filter_map { |line| line[/\Astate: ([0-9A-Z]{5})\z/, 1] }
      results = lines.select { |line| line.start_with?("{") }.map { |line| JSON.parse(line) }
      roles = lines.select { |line| line.start_with?("role:") }.map { |line| JSON.parse(line.delete_prefix("role:")) }
      unless roles.length == 1 && roles.first.values_at("session", "current") == [role, role]
        raise Failure, "ingestion must connect directly without role substitution"
      end
      if role == @runtime && roles.first.values_at("superuser", "create_role", "bypass_rls") != [false, false, false]
        raise Failure, "ingestion final runtime gained a forbidden capability"
      end
      diagnostics = ingestion_diagnostics(stderr, role:)
      errors = diagnostics.map { |entry| entry.fetch("error") }
      details = diagnostics.filter_map { |entry| entry.fetch("detail") }
      hints = diagnostics.filter_map { |entry| entry.fetch("hint") }
      unless !states.empty? && results.length == states.count("00000") && errors.map { |value| value[0, 5] } == states.reject { |state| state == "00000" }
        raise Failure, "ingestion result/error framing is incomplete"
      end
      before = lines.grep(/\Abefore:/).map { |line| line.delete_prefix("before:") }
      after = lines.grep(/\Aafter:/).map { |line| line.delete_prefix("after:") }
      raise Failure, "ingestion caller variable-conflict scope changed" unless before == ["error"] && after == before

      clocks = lines.grep(/\Aclock:/).map { |line| JSON.parse(line.delete_prefix("clock:")) }
      raise Failure, "ingestion transaction clock evidence missing" unless clocks.length == states.length

      helpers = lines.grep(/\Ahelpers:/).map { |line| JSON.parse(line.delete_prefix("helpers:")) }
      expected_helpers = helpers_first ? [ingestion_helper_expectations] : []
      raise Failure, "ingestion helper-first known answers changed or missing" unless helpers == expected_helpers
      if helpers_first
        helper_position = lines.index { |line| line.start_with?("helpers:") }
        before_position = lines.index { |line| line.start_with?("before:") }
        result_position = lines.index { |line| line.start_with?("state:", "{") }
        unless before_position < helper_position && helper_position < result_position
          raise Failure, "ingestion helpers were not observed before ingestion"
        end
      end

      { "states" => states, "results" => results, "errors" => errors, "details" => details, "hints" => hints, "diagnostics" => diagnostics,
        "clocks" => clocks, "helpers" => helpers, "caller_settings" => { "before" => before, "after" => after } }
    end

    def ingestion_diagnostics(stderr, role:)
      stderr.split(/(?=^ERROR:  )/).map do |record|
        frame = record.match(/\AERROR:  (?<error>[0-9A-Z]{5}: [^\n]+)\n(?:DETAIL:  (?<detail>[^\n]+)\n)?(?:HINT:  (?<hint>[^\n]+)\n)?CONTEXT:  (?:(?<statement>SQL statement "[^"]*")\n)?(?<routine>PL\/pgSQL function search_result_ingest_v1\([^\n]+\)) line (?<line>[1-9][0-9]*) at (?<operation>RAISE|SQL statement)\nLOCATION:  (?<location>[A-Za-z0-9_]+, [A-Za-z0-9_]+\.c:[1-9][0-9]*)\n\z/)
        raise Failure, "ingestion diagnostic contains an unrecognized record" unless frame

        entry = frame.named_captures
        routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |value| value.fetch("name") == "search_result_ingest_v1" }
        unless routine && entry.fetch("routine") == "PL/pgSQL function #{routine.fetch('signature')}"
          raise Failure, "ingestion diagnostic signature differs from the frozen routine"
        end
        source = routine.fetch("source")
        if entry.fetch("statement")
          statement = entry.fetch("statement").delete_prefix('SQL statement "').delete_suffix('"')
          unless !statement.strip.empty? && source.include?(statement)
            raise Failure, "ingestion diagnostic SQL is outside the frozen routine"
          end
        end
        # The independently checked D3 directive adds exactly one source line.
        line = Integer(entry.fetch("line")) - (role == @runtime ? 1 : 0)
        unless line.positive? && line <= source.lines.length
          raise Failure, "ingestion diagnostic line is outside the frozen routine"
        end

        entry.merge("line" => line)
      end
    end

    def ingestion_helper_expectations
      {
        "normalize_title_v1" => [nil, "proof"],
        "normalize_magnet_uri_v1" => [nil, nil, "https://example.invalid/proof", "magnet:?", "magnet:?dn=Proof&xt=opaque"],
        "derive_magnet_hash_v1" => [nil, Digest::SHA256.hexdigest(["b" * 64].pack("H*")),
                                    Digest::SHA256.hexdigest(["a" * 40].pack("H*")),
                                    Digest::SHA256.hexdigest("magnet:?dn=Proof&xt=opaque")],
        "compute_title_size_hash_v1" => [nil, nil, Digest::SHA256.hexdigest("proof|1024")],
        "policy_text_match_v1" => [false, true, true, true, true, true, false, true, false, false, false],
        "policy_uuid_match_v1" => [false, true, false, false],
        "policy_int_match_v1" => [false, true, false, false],
        "policy_release_group_match_v1" => [true, false, false],
        "policy_action_to_decision_type" => %w[drop_canonical drop_source downrank flag flag flag]
      }
    end

    def ingestion_comparable(value)
      identities = {}
      value.fetch("after").values_at("canonical_torrent", "canonical_torrent_source").flatten.each do |row|
        key = row.key?("canonical_torrent_public_id") ? "canonical_torrent" : "canonical_torrent_source"
        uuid = row.fetch("#{key}_public_id")
        raise Failure, "ingestion generated identity is invalid or duplicated" unless uuid.match?(INGESTION_UUID) && !identities.key?(uuid)

        identities[uuid] = "<#{key}:#{row.fetch("#{key}_id")}>"
      end
      value.fetch("results").each do |row|
        %w[canonical_torrent_public_id canonical_torrent_source_public_id].each do |key|
          raise Failure, "ingestion result identity has no committed row" unless identities.key?(row.fetch(key))
        end
      end
      normalized = ingestion_normalize(value, identities, value.fetch("clocks"))
      normalized.delete("clocks")
      normalized
    end

    def ingestion_normalize(value, identities, clocks)
      case value
      when Hash then value.transform_values { |item| ingestion_normalize(item, identities, clocks) }
      when Array then value.map { |item| ingestion_normalize(item, identities, clocks) }
      when String
        return identities.fetch(value) if identities.key?(value)

        position = clocks.index(value)
        position ? "<transaction-time:#{position}>" : value
      else value
      end
    end

    def ingestion_report!
      report = { complete: false, passed: false, stop_reason: @ingestion_stop,
                 postgres_image: @contract.postgres_image,
                 candidate_sha256: @contract.expected_candidate_sha256,
                 final_sha256: @contract.final_sha256,
                 required_scope_unproven: INGESTION_PENDING, cases: @ingestion_results }
      File.write(File.join(@ingestion_evidence, "report.json"), JSON.pretty_generate(report) + "\n")
    end
  end
end
