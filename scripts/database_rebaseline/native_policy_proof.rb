# frozen_string_literal: true

require_relative "native_policy_phases"
require_relative "native_policy_cast_scope"

module RevaerDatabaseRebaseline
  module NativePolicyProof
    POLICY_NATIVE_SERVER_OPTIONS = %w[-c logging_collector=on -c log_destination=jsonlog
      -c log_directory=/tmp/native-policy -c log_filename=observation -c log_rotation_age=0
      -c log_rotation_size=0 -c log_disconnections=on].freeze
    POLICY_NATIVE_PRODUCERS = %w[native_policy_proof.rb native_policy_phases.rb native_policy_cast_phases.rb
      native_policy_cast_scope.rb native_sessions.rb native_processes.rb native_trace.rb native_tooling.rb
      native_trust_rank_proof.rb native-policy.gdb native-policy-cast.gdb].freeze
    POLICY_NATIVE_CAST_SOURCE = "crates/revaer-data/migrations/0093_policy_action_decision_type_cast.sql"
    POLICY_NATIVE_OBSERVER = {
      "session_preload_libraries" => "auto_explain", "auto_explain.log_min_duration" => "0",
      "auto_explain.log_nested_statements" => "on", "auto_explain.log_analyze" => "on",
      "auto_explain.log_verbose" => "on", "auto_explain.log_timing" => "off", "auto_explain.log_format" => "json"
    }.freeze

    private

    def verify_ingestion_native_policy!
      @contract.validate_output_path!
      parent = File.join(@contract.output_path, "ingestion-native-policy")
      raise Failure, "native policy parent must not be a symlink" if File.symlink?(parent)

      FileUtils.mkdir_p(parent, mode: 0o700)
      @native_policy_evidence = Dir.mktmpdir("run-", parent)
      source_before = native_policy_source_hashes
      source_commit = @runner.run!(["git", "rev-parse", "HEAD"], chdir: @contract.root).strip
      POLICY_NATIVE_PRODUCERS.each do |name|
        native_write(name, File.binread(File.join(@contract.root, "scripts/database_rebaseline", name)), directory: @native_policy_evidence)
      end
      first = @checks.length
      contexts = []
      completed = false
      begin
        native_policy_prepare!(parent)
        native_before = dependency_native_identity
        observer_before = native_policy_observer_hash
        raise Failure, "native policy requires pinned arm64 PostgreSQL" unless native_before.fetch("architecture") == "arm64" && sql("SHOW server_version_num", role: "postgres") == "160014"

        native_policy_record(@native_policy_evidence, "routine-inventory.json", @ingestion_inventory)
        headers = @runner.run!(["docker", "exec", @container, "tar", "-cf", "-", "-C", "/usr/local/include/postgresql/server",
          *NativeTrustRankProof::NATIVE_HEADERS, "executor/execdesc.h"])
        native_write("target-abi-headers.tar", headers, directory: @native_policy_evidence)
        NativePolicyPhases::CASES.product(%w[cold helpers-first], %w[reference final]).each do |name, mode, variant|
          contexts << native_policy_pair(name, mode, variant)
        end
        check("native policy all twenty source-bound plain/observed contexts", contexts.length == 20)
        check("native policy source bytes unchanged", source_before == native_policy_source_hashes)
        check("native policy target bytes unchanged", native_before == dependency_native_identity && observer_before == native_policy_observer_hash)
        native_policy_record(@native_policy_evidence, "identity.json", { source_commit:, source_before:, source_after: native_policy_source_hashes,
          native_before:, native_after: dependency_native_identity, observer_before:, observer_after: native_policy_observer_hash,
          target_abi_headers_sha256: Digest::SHA256.hexdigest(headers) })
        completed = true
      ensure
        @native_policy_context = nil
        @native_policy_target = nil
        server = @runner.capture(["docker", "exec", @container, "cat", "/tmp/native-policy/observation.json"])
        native_write("server-final.jsonl", server.stdout, directory: @native_policy_evidence)
        native_write("server-final.stderr", server.stderr, directory: @native_policy_evidence)
        check("native policy final server log retained", server.success && server.stderr.empty? && !server.stdout.empty?)
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |row| row.fetch(:passed) }, d3_complete: false,
          source_commit:, source_sha256: source_before, contexts:, checks:,
          cleanup_required: "cleanup.json must independently confirm removal of owned resources and clients",
          scope: "Five policy cases, cold/helper-first reference/final pairs, including eight inlined-cast contexts" }
        path = native_policy_record(@native_policy_evidence, "proof-result.json", report)
        @native_policy_result = { path:, sha256: Digest::SHA256.file(path).hexdigest, passed: report.fetch(:passed) }
      end
      raise Failure, "native policy proof failed; evidence retained" unless @checks.drop(first).all? { |row| row.fetch(:passed) }
    end

    def native_policy_source_hashes
      paths = POLICY_NATIVE_PRODUCERS.map { |name| "scripts/database_rebaseline/#{name}" } + IngestionAttributes::ATTRIBUTE_SOURCE_FILES + [POLICY_NATIVE_CAST_SOURCE]
      policy_source_hashes.merge(paths.uniq.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] })
    end

    def native_policy_record(directory, name, data)
      native_write(name, JSON.pretty_generate(data) + "\n", directory:)
    end

    def native_policy_prepare!(parent)
      native_prepare_tooling!(parent)
      native_write("tooling-receipt.json", File.binread(@native_tooling.receipt_path), directory: @native_policy_evidence)
      probe = File.binread(File.join(@contract.root, "scripts/database_rebaseline/native-policy.gdb"))
      @native_policy_sessions = NativeSessions.new(runner: @runner, processes: NativeProcesses.new(directory: @native_policy_evidence),
        directory: @native_policy_evidence, container: @container, tooling: @native_tooling, probe:)
    end

    def native_policy_pair(name, mode, variant)
      test_case = policy_cases.find { |row| row.fetch(:name) == name }
      raise Failure, "canonical native policy case missing" unless test_case

      source, role = variant == "reference" ? ["reference_proof", "postgres"] : [@database, @runtime]
      context_name = [name, mode, variant].join("-")
      arms = %w[plain observed].to_h do |arm|
        previous = [@policy_evidence, @correction_evidence, @native_policy_context]
        directory = File.join(@native_policy_evidence, "#{context_name}-#{arm}")
        Dir.mkdir(directory, 0o700)
        @policy_evidence = directory
        @correction_evidence = directory
        @native_policy_context = { name: "#{context_name}-#{arm}", directory:, observed: arm == "observed",
          inlined: variant == "reference" && (mode == "helpers-first" || !name.start_with?("release-regex-error-")),
          regex_error: test_case.key?(:error), variant: }
        evidence = policy_isolated(test_case, variant, source, role, mode:)
        comparable = policy_comparable(evidence, count: 3)
        native_policy_record(directory, "application.json", evidence)
        native_policy_record(directory, "comparable.json", comparable)
        if arm == "observed"
          qualification = native_policy_qualify!(evidence, name, mode, variant)
          native_policy_record(directory, "readback.json", qualification)
        end
        [arm, comparable]
      ensure
        @policy_evidence, @correction_evidence, @native_policy_context = previous
      end
      equal = arms.fetch("plain") == arms.fetch("observed")
      check("native policy #{context_name} complete plain/observed application pair", equal)
      raise Failure, "native policy observation changed application behavior" unless equal

      { name:, mode:, variant:, plain_equals_observed: true }
    end

    def policy_execute(session_case, database, role, prefix, helpers: false, test_case: nil)
      previous = @native_policy_target
      @native_policy_target = @native_policy_context&.merge(database:, role:) if test_case
      observed_cast = @native_policy_target && @native_policy_target.values_at(:observed, :inlined).all?
      native_policy_observer_settings(database, true) if observed_cast
      evidence = super
      native_policy_server_records(database, evidence.fetch("frames").first.fetch("backend")) if observed_cast
      evidence
    ensure
      @native_policy_target = previous
      native_policy_observer_settings(database, false) if observed_cast
    end

    def native_policy_result(query, role:, database:)
      target = @native_policy_target
      raise Failure, "native policy transport target changed" unless target.values_at(:database, :role) == [database, role]

      probe = target.fetch(:regex_error) ? File.binread(File.join(@contract.root, "scripts/database_rebaseline/native-policy.gdb")) : "# Qualified helper and callback capture only.\n"
      probe += File.binread(File.join(@contract.root, "scripts/database_rebaseline/native-policy-cast.gdb")) if target.fetch(:inlined)
      capture = @native_policy_sessions.execute(command: command(role, database), query:, name: target.fetch(:name), observed: target.fetch(:observed), probe:)
      if capture.debugger
        trace = NativeTrace.new(query: ->(statement) { sql(statement, role: "postgres", database:) }, directory: target.fetch(:directory),
          observations: :policy, trigger_expectation: target.fetch(:regex_error) ? :absent : :present).collect!(capture.debugger)
        @native_policy_context[:trace] = trace
        @native_policy_context[:cast_catalog] = native_policy_cast_catalog(database)
      end
      capture.application
    end

    def native_policy_qualify!(evidence, name, mode, variant)
      context = @native_policy_context
      trace = context.fetch(:trace)
      native_policy_bind_sources!(trace.fetch(:catalog), variant, context.fetch(:directory))
      if context.fetch(:inlined)
        routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |row| row.fetch("name") == "search_result_ingest_v1" }
        cast = File.binread(File.join(@contract.root, POLICY_NATIVE_CAST_SOURCE)).scan(/AS \$\$(.*?)\$\$;/m)
        raise Failure, "frozen policy cast source absent or duplicated" unless cast.length == 1

        NativePolicyCastScope.new(routine:, helper_query: policy_helpers_sql, cast_source: cast.first.first, database: "ingestion_policy_reference")
          .validate!(events: trace.fetch(:events), records: context.fetch(:server_records), evidence:,
            catalog: context.fetch(:cast_catalog), name:, mode:)
      else
        NativePolicyPhases.new.validate!(trace.fetch(:events), evidence.fetch("frames"), name:, mode:, variant:)
      end
    end

    def native_policy_bind_sources!(catalog, variant, directory)
      routines = @ingestion_inventory.fetch("#{variant}_proof").fetch("routines")
      catalog.each do |row|
        if row.fetch("name") == "snapshot"
          text = File.binread(File.join(directory, "observer-#{variant}.sql"))
          pairs = IngestionProof::INGESTION_TABLES.map do |table|
            "'#{table}', (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)->(SELECT a.attname FROM pg_attribute a WHERE a.attrelid = 'public.#{table}'::regclass AND a.attnum = 1)), '[]') FROM public.\"#{table}\" t)"
          end
          body = " SELECT json_build_object(#{pairs.join(',')}); "
          raise Failure, "native policy snapshot source changed" unless text.scan("AS $$#{body}$$;").length == 1

          expected = { "schema" => "ingestion_observation", "signature" => "ingestion_observation.snapshot()", "language" => "sql",
            "source_sha256" => Digest::SHA256.hexdigest(body), "config" => ["search_path=pg_catalog"] }
        else
          matches = routines.select { |item| item.fetch("signature") == row.fetch("signature") }
          raise Failure, "native policy helper source missing or duplicate" unless matches.length == 1

          routine = matches.first
          expected = { "schema" => "public", "signature" => routine.fetch("signature"), "language" => routine.fetch("name") == "policy_action_to_decision_type" ? "sql" : "plpgsql",
            "source_sha256" => Digest::SHA256.hexdigest(routine.fetch("source")), "config" => routine.fetch("settings") }
        end
        raise Failure, "native policy catalog/source binding changed" unless row.slice(*expected.keys) == expected
      end
    end

    def native_policy_cast_catalog(database)
      query = <<~SQL
        SELECT json_build_object('function', pg_get_functiondef(c.castfunc), 'source', p.prosrc, 'config', p.proconfig,
          'security_definer', p.prosecdef, 'volatility', p.provolatile, 'language', l.lanname,
          'castfunc', c.castfunc::bigint, 'castmethod', c.castmethod, 'castcontext', c.castcontext)
        FROM pg_cast c JOIN pg_proc p ON p.oid=c.castfunc JOIN pg_language l ON l.oid=p.prolang
        WHERE c.castsource='public.policy_action'::regtype AND c.casttarget='public.decision_type'::regtype;
      SQL
      catalog = metadata_json_parse(sql(query, role: "postgres", database:))
      native_policy_record(@native_policy_context.fetch(:directory), "cast-catalog.json", catalog)
      catalog
    end

    def native_policy_observer_settings(database, enabled)
      query = POLICY_NATIVE_OBSERVER.map do |key, value|
        enabled ? "ALTER DATABASE #{identifier(database)} SET #{key} TO #{literal(value)};" : "ALTER DATABASE #{identifier(database)} RESET #{key};"
      end.join("\n")
      sql(query, role: "postgres", database: "postgres")
    end

    def native_policy_observer_hash
      bytes = @runner.run!(["docker", "exec", @container, "sha256sum", "/usr/local/lib/postgresql/auto_explain.so"])
      raise Failure, "native policy observer identity malformed" unless bytes.match?(/\A[a-f0-9]{64}  \/usr\/local\/lib\/postgresql\/auto_explain\.so\n\z/)

      bytes.split.fetch(0)
    end

    def native_policy_server_records(database, backend)
      raise Failure, "native policy backend identity invalid" unless backend.match?(/\A[1-9][0-9]*\z/)

      directory = @native_policy_context.fetch(:directory)
      raw = @runner.run!(["docker", "exec", @container, "cat", "/tmp/native-policy/observation.json"])
      native_write("server-through-case.jsonl", raw, directory:)
      records = raw.lines.map { |line| metadata_json_parse(line) }.select { |row| row.fetch("pid") == Integer(backend) && row["dbname"] == database }
      raise Failure, "native policy completed server records absent" unless records.any? { |row| row.fetch("message").start_with?("disconnection:") }

      native_policy_record(directory, "server-records.json", records)
      @native_policy_context[:server_records] = records
    end
  end
end
