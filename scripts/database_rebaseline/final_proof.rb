# frozen_string_literal: true

require "json"
require_relative "../database-rebaseline"
require_relative "extension_proof"
require_relative "reset_timeout_proof"

module RevaerDatabaseRebaseline
  # Disposable transition proof only, not an operator or application initializer.
  class FinalProof
    include ExtensionProof
    include ResetTimeoutProof
    def initialize(contract = Contract.new, runner: CommandRunner.new)
      @contract = contract
      @runner = runner
      @failures = []
      @checks = []
      @completed = false
      suffix = SecureRandom.hex(8)
      @container = "revaer-final-proof-#{Process.pid}-#{suffix}"
      @owner = "proof_owner_#{suffix}"
      @runtime = "proof_runtime_#{suffix}"
      @outsider = "proof_other_#{suffix}"
      @database = "final_proof"
    end

    def run!
      @contract.freeze!
      final = FinalSql.new(@contract).verify!
      @candidate = File.binread(@contract.candidate_path)
      @routines = FinalSql.new(@contract).routines(@candidate)
      started = false
      begin
        @runner.run!([
          "docker", "run", "-d", "--name", @container, "--shm-size", "1g",
          "-e", "POSTGRES_HOST_AUTH_METHOD=trust",
          "-e", "POSTGRES_INITDB_ARGS=--locale=C --encoding=UTF8 --data-checksums",
          "-e", "TZ=UTC", @contract.postgres_image
        ])
        started = true
        wait_ready!
        provision!
        check("pinned server identity", sql("SELECT current_setting('server_version_num')", role: @owner) == "160014")
        versions = @runner.run!(["docker", "exec", @container, "sh", "-eu", "-c", "psql --version; pg_dump --version"])
        check("pinned client identity", versions.lines.all? { |line| line.strip.end_with?(" #{@contract.postgres_version}") })
        apply!("reference_proof", @candidate, role: "postgres")
        apply!(@database, final)
        verify_parity!
        verify_unsealed!
        verify_seal_inputs!
        seal!
        verify_baseline!
        verify_read_failures!
        verify_privileges!
        verify_extension_boundary!
        verify_call_paths!
        verify_reset_timeout_scope!
        verify_atomicity!(final)
        verify_timeout_preservation!(final)
        sql("ALTER ROLE #{identifier(@owner)} NOLOGIN", role: "postgres", database: "postgres")
        check("runtime read after bootstrap disabled", sql("SELECT count(*) FROM revaer_system.read_database_baseline_v1()", role: @runtime) == "1")
        verify_runtime_extension_primitives!
        verify_call_paths!
        @completed = true
      ensure
        begin
          @runner.run!(["docker", "rm", "-f", @container]) if started
        rescue Failure
          @failures << "disposable container cleanup failed"
          raise
        ensure
          write_evidence!
        end
      end
      raise Failure, "final proof failed: #{@failures.join('; ')}" unless @failures.empty?

      puts "database-final-proof: #{@checks.length} checks passed"
    end

    private

    def identifier(value)
      '"' + value.gsub('"', '""') + '"'
    end

    def literal(value)
      "'#{value.gsub("'", "''")}'"
    end

    def command(role, database)
      ["docker", "exec", "-i", @container, "psql", "--no-psqlrc", "-X", "-qAt",
       "--set", "ON_ERROR_STOP=1", "--set", "VERBOSITY=sqlstate", "-h", "127.0.0.1", "-U", role, "-d", database]
    end

    def result(query, role: @owner, database: @database)
      @runner.capture(command(role, database), stdin_data: query)
    end

    def sql(query, role: @owner, database: @database)
      outcome = result(query, role:, database:)
      if !outcome.success || outcome.stderr.match?(/\bWARNING\b/)
        File.write(File.join(@contract.output_path, "final-query-error.txt"), outcome.stderr)
        raise Failure, "PostgreSQL proof query failed or emitted a warning; retained diagnostic"
      end
      outcome.stdout.strip
    end

    def denied(label, query, role: @runtime, state: "42501", detail: nil)
      outcome = result("\\set VERBOSITY verbose\n#{query}", role:)
      check(label, !outcome.success && outcome.stderr.include?(state) && (detail.nil? || outcome.stderr.include?("DETAIL:  #{detail}")))
    end

    def check(label, passed)
      @checks << { check: label, passed: }
      @failures << label unless passed
      puts "database-final-proof: #{label}=#{passed ? 'passed' : 'FAILED'}"
    end

    def wait_ready!
      120.times do
        outcome = result("SELECT 1", role: "postgres", database: "postgres")
        return if outcome.success && outcome.stdout.strip == "1"

        sleep(0.25)
      end
      raise Failure, "final proof PostgreSQL did not become ready"
    end

    def provision!
      [@owner, @runtime, @outsider].each do |role|
        sql("CREATE ROLE #{identifier(role)} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS", role: "postgres", database: "postgres")
      end
      [@database, "reference_proof", "extension_proof", "rollback_proof", "timeout_proof"].each do |database|
        sql("CREATE DATABASE #{identifier(database)} OWNER #{identifier(@owner)} TEMPLATE template0 LC_COLLATE 'C' LC_CTYPE 'C' ENCODING 'UTF8'", role: "postgres", database: "postgres")
      end
    end

    def apply!(database, bytes, role: @owner)
      outcome = result("\\set VERBOSITY default\nBEGIN;\n#{bytes}\nCOMMIT;", database:, role:)
      return if outcome.success && !outcome.stderr.match?(/\bWARNING\b/)

      File.write(File.join(@contract.output_path, "#{database}-apply-error.txt"), outcome.stderr)
      raise Failure, "constrained owner apply failed (#{database}); retained PostgreSQL diagnostic"
    end

    def schema(database)
      dump = @runner.run!([
        "docker", "exec", @container, "pg_dump", "-U", "postgres", "-d", database,
        "--schema-only", "--no-owner", "--no-privileges", "--no-tablespaces",
        "--exclude-schema=revaer_system"
      ])
      dump.lines.reject { |line| line.match?(CandidateBuilder::RESTRICT_PATTERN) }.join
    end

    def normalized_routine_security(source)
      source = source.sub("--\n-- Name: public; Type: SCHEMA; Schema: -; Owner: -\n--\n\n-- *not* creating schema, since initdb creates it\n\n\n", "")
      source.gsub(/(^CREATE FUNCTION .*?^    AS )/m) do |header|
        header.gsub(/ SECURITY DEFINER\b/, "").gsub(/^    SET search_path TO [^\n]*\n/, "")
      end
    end

    def verify_parity!
      legacy = FinalSql.new(@contract).approved_legacy_deltas(normalized_routine_security(schema("reference_proof")))
      final = normalized_routine_security(schema(@database))
      File.binwrite(File.join(@contract.output_path, "final-reference-schema.sql"), legacy)
      File.binwrite(File.join(@contract.output_path, "final-observed-schema.sql"), final)
      check("two-way legacy schema and extension parity", final == legacy)
      seed_query = <<~SQL
        SELECT format('%I.%I', n.nspname, c.relname),
               COALESCE(string_agg(quote_literal(a.attname), ',' ORDER BY a.attnum)
                   FILTER (WHERE a.atttypid IN ('timestamptz'::regtype, 'timestamp'::regtype)), '')
        FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
        LEFT JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped
        WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime') AND c.relkind = 'r'
        GROUP BY n.nspname, c.relname ORDER BY n.nspname, c.relname;
      SQL
      tables = sql(seed_query).lines(chomp: true)
      queries = tables.map do |line|
        table, clocks = line.split("|", 2)
        "SELECT #{literal(table)} || ':' || COALESCE(jsonb_agg(v ORDER BY v::text)::text, '[]') FROM (SELECT to_jsonb(t) - ARRAY[#{clocks}]::text[] AS v FROM #{table} t) rows;"
      end.join("\n")
      reference = normalize_seed_identities(sql(queries, role: "postgres", database: "reference_proof"))
      actual = normalize_seed_identities(sql(queries))
      File.binwrite(File.join(@contract.output_path, "final-reference-seeds.jsonl"), reference)
      File.binwrite(File.join(@contract.output_path, "final-observed-seeds.jsonl"), actual)
      check("two-way seed parity excluding generated timestamp columns", actual == reference)
    end

    def normalize_seed_identities(source)
      source.lines.map do |line|
        next line unless line.start_with?("public.rate_limit_policy:")

        table, encoded = line.split(":", 2)
        rows = JSON.parse(encoded)
        ids = rows.map { |row| row.fetch("rate_limit_policy_public_id") }
        unless ids.length == 2 && ids.uniq.length == 2 && ids.all? { |id| id.match?(/\A[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/) }
          raise Failure, "canonical seed public identities are invalid"
        end
        rows.each { |row| row["rate_limit_policy_public_id"] = "<#{row.fetch('display_name')}>" }
        "#{table}:#{JSON.generate(rows)}\n"
      end.join
    end

    def verify_unsealed!
      denied("empty baseline is invalid", "SELECT * FROM revaer_system.read_database_baseline_v1()", role: @owner, state: "P0001", detail: "baseline_shape_invalid")
      denied("runtime cannot seal before grants", seal_query, state: "42501")
    end

    def seal_query(version: "1", digest: "decode('#{@contract.final_sha256}', 'hex')", runtime: literal(@runtime))
      "SELECT * FROM revaer_system.seal_database_baseline_v1(#{version}::smallint, #{digest}, #{runtime});"
    end

    def verify_seal_inputs!
      %w[NULL -1 0 2 32767].each do |version|
        denied("reject contract version #{version}", seal_query(version:), role: @owner, state: "P0001")
      end
      ["NULL", "decode('', 'hex')", "decode(repeat('aa', 31), 'hex')", "decode(repeat('aa', 33), 'hex')"].each_with_index do |digest, index|
        denied("reject invalid digest #{index}", seal_query(digest:), role: @owner, state: "P0001")
      end
      ["NULL", "''", "repeat('x', 64)", "'absent'", literal(@owner)].each_with_index do |runtime, index|
        denied("reject invalid runtime #{index}", seal_query(runtime:), role: @owner, state: "P0001")
      end
      check("failed seals leave no row", sql("SELECT count(*) FROM revaer_system.database_baseline") == "0")
    end

    def seal!
      sql("BEGIN; #{seal_query} COMMIT;")
      denied("reject reseal", seal_query, role: @owner, state: "P0001")
    end

    def verify_baseline!
      shape = sql(<<~SQL)
        SELECT string_agg(a.attname || ':' || format_type(a.atttypid, a.atttypmod) || ':' || a.attnotnull::text, ',' ORDER BY a.attnum)
        FROM pg_attribute a WHERE a.attrelid = 'revaer_system.database_baseline'::regclass AND a.attnum > 0 AND NOT a.attisdropped;
      SQL
      expected = "baseline_id:smallint:true,contract_version:smallint:true,init_sha256:bytea:true,postgres_version_num:integer:true,schema_owner_role:name:true,runtime_role:name:true,sealed_at:timestamp with time zone:true"
      check("exact baseline column shape", shape == expected)
      check("exact baseline constraint count", sql("SELECT count(*) FROM pg_constraint WHERE conrelid = 'revaer_system.database_baseline'::regclass") == "8")
      check("exact sealed digest and principals", sql("SELECT contract_version = 1 AND encode(init_sha256, 'hex') = '#{@contract.final_sha256}' AND postgres_version_num = 160014 AND schema_owner_role = #{literal(@owner)} AND runtime_role = #{literal(@runtime)} FROM revaer_system.read_database_baseline_v1()", role: @runtime) == "t")
      check("no migration metadata", sql("SELECT to_regclass('public._sqlx_migrations') IS NULL") == "t")
    end

    def verify_read_failures!
      original = sql("SELECT row_to_json(b)::text FROM revaer_system.database_baseline b")
      [
        "DELETE FROM revaer_system.database_baseline;",
        "DROP TABLE revaer_system.database_baseline;",
        "ALTER TABLE revaer_system.database_baseline DROP COLUMN contract_version;",
        "ALTER TABLE revaer_system.database_baseline DROP CONSTRAINT database_baseline_runtime_nonempty; UPDATE revaer_system.database_baseline SET runtime_role = '';",
        "ALTER TABLE revaer_system.database_baseline DROP CONSTRAINT database_baseline_contract_v1; UPDATE revaer_system.database_baseline SET contract_version = 2;",
        "ALTER TABLE revaer_system.database_baseline DROP CONSTRAINT database_baseline_sha256_length; UPDATE revaer_system.database_baseline SET init_sha256 = decode('aa', 'hex');",
        "ALTER TABLE revaer_system.database_baseline DROP CONSTRAINT database_baseline_pkey, DROP CONSTRAINT database_baseline_singleton; INSERT INTO revaer_system.database_baseline SELECT 2, contract_version, init_sha256, postgres_version_num, schema_owner_role, runtime_role, sealed_at FROM revaer_system.database_baseline;"
      ].each_with_index do |mutation, index|
        denied("malformed baseline read #{index}", "BEGIN; #{mutation} SELECT * FROM revaer_system.read_database_baseline_v1();", role: @owner, state: "P0001", detail: "baseline_shape_invalid")
      end
      check("read failure fixtures roll back exact baseline", sql("SELECT row_to_json(b)::text FROM revaer_system.database_baseline b") == original)
    end

    def verify_privileges!
      denied("runtime seal denied", seal_query)
      denied("runtime DDL denied", "CREATE TABLE public.forbidden (id integer)")
      denied("runtime temp denied", "CREATE TEMP TABLE forbidden (id integer)")
      denied("runtime baseline DML denied", "DELETE FROM revaer_system.database_baseline")
      denied("runtime application table DML denied", "DELETE FROM public.app_profile")
      denied("runtime table read denied", "SELECT * FROM public.app_profile")
      denied("runtime sequence denied", "SELECT nextval('public.app_user_user_id_seq')")
      before = sql("SELECT nspacl::text FROM pg_namespace WHERE nspname = 'public'")
      outcome = result("GRANT CREATE ON SCHEMA public TO PUBLIC", role: @runtime)
      check("runtime schema grant cannot change ACL", !outcome.stderr.empty? && sql("SELECT nspacl::text FROM pg_namespace WHERE nspname = 'public'") == before)
      denied("runtime extension DDL denied", "CREATE EXTENSION hstore")
      denied("runtime role substitution denied", "SET ROLE #{identifier(@owner)}")
      check("outsider database connect denied", sql("SELECT has_database_privilege(#{literal(@outsider)}, current_database(), 'CONNECT')") == "f" && !result("SELECT 1", role: @outsider).success)
      sql("GRANT #{identifier(@runtime)} TO #{identifier(@outsider)}", role: "postgres", database: "postgres")
      denied("SET ROLE surrogate read denied", "SET ROLE #{identifier(@runtime)}; SELECT * FROM revaer_system.read_database_baseline_v1()", role: @outsider, detail: "baseline_read_denied")
      sql("REVOKE #{identifier(@runtime)} FROM #{identifier(@outsider)}", role: "postgres", database: "postgres")
      rows = JSON.parse(sql(<<~SQL))
        SET search_path TO pg_catalog;
        SELECT COALESCE(json_agg(row_to_json(r)), '[]') FROM (
          SELECT p.oid::regprocedure::text AS identity, p.prosecdef,
                 pg_get_userbyid(p.proowner) = #{literal(@owner)} AS owned,
                 p.proconfig,
                 p.prorettype IN ('trigger'::regtype, 'event_trigger'::regtype) AS trigger,
                 EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e') AS extension
          FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
          WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime', 'revaer_system')
            AND has_function_privilege(#{literal(@runtime)}, p.oid, 'EXECUTE')
          ORDER BY p.oid::regprocedure::text
        ) r;
      SQL
      File.write(File.join(@contract.output_path, "final-runtime-routines.json"), JSON.pretty_generate(rows) + "\n")
      authored = rows.reject { |row| row.fetch("extension") }
      check("authored runtime routines hardened", authored.all? { |row| row.fetch("owned") && row.fetch("prosecdef") && !row.fetch("trigger") && row.fetch("proconfig").is_a?(Array) && !row.fetch("proconfig").join.include?("pg_temp") })
      check("exact authored runtime grant count", authored.length == @routines.count { |routine| !routine.trigger } + 1)
      paths = @routines.reject(&:trigger).to_h { |routine| [routine.identity.gsub(', ', ','), ["search_path=#{routine.path}"]] }
      paths["revaer_system.read_database_baseline_v1()"] = ["search_path=pg_catalog, revaer_system"]
      paths["revaer_config.factory_reset_without_media_defaults_v1()"].unshift("lock_timeout=5s")
      check("exact per-routine search paths", authored.all? { |row| row.fetch("proconfig") == paths[row.fetch("identity")] })
      check("no authored PUBLIC routine execution", sql(<<~SQL) == "0")
        SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace,
            LATERAL aclexplode(COALESCE(p.proacl, acldefault('f', p.proowner))) acl
        WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime', 'revaer_system')
            AND p.proowner = (SELECT oid FROM pg_roles WHERE rolname = #{literal(@owner)}) AND acl.grantee = 0;
      SQL
      check("no runtime relation privileges", sql(<<~SQL) == "0")
        SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime', 'revaer_system') AND (
            (c.relkind = 'r' AND has_table_privilege(#{literal(@runtime)}, c.oid, 'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')) OR
            (c.relkind = 'S' AND has_sequence_privilege(#{literal(@runtime)}, c.oid, 'USAGE,SELECT,UPDATE')));
      SQL
      check("exact runtime database privilege matrix", sql("SELECT has_database_privilege(#{literal(@runtime)}, current_database(), 'CONNECT') AND NOT has_database_privilege(#{literal(@runtime)}, current_database(), 'CREATE,TEMPORARY')") == "t")
    end

    def verify_call_paths!
      check("runtime configuration read", sql("SELECT count(*) FROM revaer_config.fetch_app_profile_row('00000000-0000-0000-0000-000000000001')", role: @runtime) == "1")
      sql("SELECT revaer_config.update_app_instance_name('00000000-0000-0000-0000-000000000001', 'baseline-proof')", role: @runtime)
      check("runtime configuration write and trigger", sql("SELECT instance_name FROM revaer_config.fetch_app_profile_row('00000000-0000-0000-0000-000000000001')", role: @runtime) == "baseline-proof")
      sql("SELECT revaer_config.upsert_secret('proof-secret', decode('aa', 'hex'), 'proof')", role: @runtime)
      check("runtime stored secret path", sql("SELECT count(*) FROM revaer_config.fetch_secret_by_name('proof-secret')", role: @runtime) == "1")
      check("runtime torrent listing", sql("SELECT count(*) FROM revaer_runtime.list_torrents()", role: @runtime) == "0")
      check("runtime media queue read", sql("SELECT count(*) FROM public.media_job_list_v1(NULL, NULL)", role: @runtime) == "0")
      check("runtime media recovery read", sql("SELECT count(*) FROM public.media_job_worker_recover_stale_v1(60)", role: @runtime) == "0")
    end

    def verify_atomicity!(final)
      outcome = result("BEGIN;\n#{final}\n#{seal_query}\nSELECT 1 / 0;\nCOMMIT;", database: "rollback_proof")
      check("injected post-seal transaction failure", !outcome.success && outcome.stderr.include?("22012"))
      check("failed transaction leaves no user schemas", sql("SELECT count(*) FROM pg_namespace WHERE nspname IN ('revaer_config', 'revaer_runtime', 'revaer_system')", database: "rollback_proof") == "0")
      check("failed transaction leaves public empty", sql("SELECT count(*) FROM pg_class WHERE relnamespace = 'public'::regnamespace", database: "rollback_proof") == "0")
    end

    def verify_timeout_preservation!(final)
      observed = sql("BEGIN; SET LOCAL statement_timeout = '120s'; SET LOCAL lock_timeout = '120s'; SET LOCAL idle_in_transaction_session_timeout = '30s';\n#{final}\nSELECT current_setting('statement_timeout') || ',' || current_setting('lock_timeout') || ',' || current_setting('idle_in_transaction_session_timeout'); ROLLBACK;", database: "timeout_proof").lines.last.to_s.strip
      check("initializer timeout preservation", observed == "2min,2min,30s")
      File.write(File.join(@contract.output_path, "final-timeout-observed.txt"), "statement_timeout,lock_timeout,idle_in_transaction_session_timeout\n#{observed}\n")
    end

    def write_evidence!
      @contract.validate_output_path!
      FileUtils.mkdir_p(@contract.output_path, mode: 0o700)
      evidence = { completed: @completed, passed: @completed && @failures.empty?, final_init_sha256: @contract.final_sha256, frozen_candidate_sha256: @contract.expected_candidate_sha256,
                   image: @contract.postgres_image, checks: @checks, failures: @failures }
      File.write(File.join(@contract.output_path, "final-proof.json"), JSON.pretty_generate(evidence) + "\n")
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    RevaerDatabaseRebaseline::FinalProof.new.run!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-final-proof: #{error.message}"
    exit 1
  end
end
