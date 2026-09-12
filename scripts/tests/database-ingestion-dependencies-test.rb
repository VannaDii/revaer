# frozen_string_literal: true

require "tmpdir"
require_relative "../database_rebaseline/final_proof"
require_relative "../database_rebaseline/ingestion_dependencies"

module RevaerDatabaseRebaseline
  class IngestionDependenciesTest < FinalProof
    include IngestionDependencies

    def run_tests!
      @assertions = 0
      fixture = dependency_test_snapshot
      dependency_validate_pair!(fixture, fixture)
      assert(true, "synthetic paired graph is accepted")
      dependency_graph_tests!(fixture)
      dependency_drift_tests!(fixture)
      dependency_routine_tests!
      dependency_observation_tests!
      dependency_output_tests!
      dependency_native_tests!
      dependency_cleanup_tests!
      puts "database-ingestion-dependencies-test: #{@assertions} assertions passed"
    end

    # Focused driver only. Parent owns wiring this module into the canonical gate.
    def run_live!
      @contract.freeze!
      final = FinalSql.new(@contract).verify!
      @candidate = File.binread(@contract.candidate_path)
      @routines = FinalSql.new(@contract).routines(@candidate)
      @dependency_evidence = File.join(@contract.output_path, "ingestion-dependencies")
      @contract.validate_output_path!
      dependency_directory!(@dependency_evidence)
      @volume = "#{@container}-native-data"
      @owned_container = false
      @owned_volume = false
      report = { completed: false, passed: false, d3_complete: false, limits: DEPENDENCY_LIMITS,
                 started_at: Time.now.utc.strftime("%Y-%m-%dT%H:%M:%SZ"),
                 source_commit: @runner.run!(["git", "rev-parse", "HEAD"], chdir: @contract.root).strip,
                 source_status: @runner.run!(["git", "status", "--porcelain=v1"], chdir: @contract.root),
                 source_files: %w[scripts/database_rebaseline/ingestion_dependencies.rb scripts/tests/database-ingestion-dependencies-test.rb].to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] },
                 postgres_image: @contract.postgres_image, candidate_sha256: @contract.expected_candidate_sha256,
                 final_sha256: @contract.final_sha256, container: @container, volume: @volume,
                 cleanup: false }
      dependency_write!("live-report.json", JSON.pretty_generate(report) + "\n")
      dependency_write!("report.json", JSON.pretty_generate(completed: false, passed: false, d3_complete: false) + "\n")
      begin
        dependency_start!
        wait_ready!
        provision!
        check("dependency pinned server identity", sql("SELECT current_setting('server_version_num')", role: "postgres") == "160014")
        apply!("reference_proof", @candidate, role: "postgres")
        apply!(@database, final)
        seal!
        @ingestion_evidence = File.join(@contract.output_path, "ingestion-proof")
        dependency_directory!(@ingestion_evidence)
        @ingestion_inventory = {}
        ingestion_inventory!
        verify_ingestion_dependencies! do
          verify_ingestion_corrections!
          verify_ingestion_compilation!
          verify_ingestion_wrapper!
          raise Failure, "existing ingestion matrices failed" unless @failures.empty?
        end
        raise Failure, "dependency checks failed" unless @failures.empty?

        report[:completed] = true
      rescue StandardError => error
        report[:error] = "#{error.class}: #{error.message}"
        raise
      ensure
        begin
          dependency_cleanup!
          report[:cleanup] = true
        rescue Failure => error
          report[:cleanup_error] = error.message
          raise
        ensure
          report[:checks] = @checks
          report[:cleanup_records] = @dependency_cleanup_records
          report[:finished_at] = Time.now.utc.strftime("%Y-%m-%dT%H:%M:%SZ")
          report[:passed] = report[:completed] && report[:cleanup] && @failures.empty?
          dependency_write!("live-report.json", JSON.pretty_generate(report) + "\n")
        end
      end
      puts "database-ingestion-dependencies-live: #{@checks.length} checks passed; d3_complete=false; exact container and volume removed"
    end

    private

    def assert(value, message)
      raise Failure, message unless value

      @assertions += 1
    end

    def rejected(message)
      yield
    rescue Failure => error
      assert(error.message.include?(message), "unexpected failure: #{error.message}")
    else
      raise Failure, "expected rejection: #{message}"
    end

    def copy(value)
      Marshal.load(Marshal.dump(value))
    end

    def dependency_start!
      present = @runner.run!(["docker", "container", "ls", "-a", "--format", "{{.Names}}", "--filter", "name=^/#{@container}$"])
      volumes = @runner.run!(["docker", "volume", "ls", "--format", "{{.Name}}", "--filter", "name=^#{@volume}$"])
      raise Failure, "refuse pre-existing dependency resources" unless present.strip.empty? && volumes.strip.empty?

      # Mark the exact fresh names before creation so partial failures are cleaned.
      @owned_volume = true
      @runner.run!(["docker", "volume", "create", @volume])
      @owned_container = true
      @runner.run!([
        "docker", "run", "-d", "--pull=never", "--network", "none", "--name", @container,
        "--shm-size", "1g", "--mount", "type=volume,source=#{@volume},target=/var/lib/postgresql/data",
        "-e", "POSTGRES_HOST_AUTH_METHOD=trust", "-e", "POSTGRES_INITDB_ARGS=--locale=C --encoding=UTF8 --data-checksums",
        "-e", "TZ=UTC", @contract.postgres_image
      ])
    end

    def dependency_cleanup!
      failures = []
      @dependency_cleanup_records = []
      [[@owned_container, ["docker", "rm", "-fv", @container]],
       [@owned_volume, ["docker", "volume", "rm", @volume]]].each do |owned, command|
        next unless owned

        outcome = @runner.capture(command)
        @dependency_cleanup_records << { command:, stdout: outcome.stdout, stderr: outcome.stderr, success: outcome.success }
        failures << command.last unless outcome.success
      end
      raise Failure, "exact dependency resource cleanup failed: #{failures.join(', ')}" unless failures.empty?
    end

    # This is deliberately synthetic, not a newly frozen production catalog.
    def dependency_test_snapshot
      nodes = []
      add = lambda do |catalog, identity, value|
        node = { "catalog" => catalog, "oid" => nodes.length + 1, "identity" => identity, "value" => value }
        nodes << node
        node
      end
      dependency_roots.each do |table|
        add.call("pg_class", "public.#{table}", {
          "relkind" => "r", "relrowsecurity" => false, "relforcerowsecurity" => false, "relhasrules" => false,
          "columns" => [[1, "id", "pg_catalog.int8"], [2, "parent_id", "pg_catalog.int8"]],
          "constraints" => [["check_positive", true, "CHECK ((id > 0))"]], "index_definition" => nil
        })
      end
      add.call("pg_attrdef", "public.canonical_torrent.id", { "definition" => "gen_random_uuid()" })
      add.call("pg_class", "public.synthetic_index", { "index_definition" => [true, "btree (id)", "(id IS NOT NULL)"] })
      add.call("pg_cast", "(public.policy_action AS public.decision_type)", {
        "castsource" => "public.policy_action", "casttarget" => "public.decision_type",
        "castfunc" => "public.policy_action_to_decision_type(public.policy_action)", "castcontext" => "a", "castmethod" => "f"
      })
      add.call("pg_ts_dict", "public.unaccent", { "dicttemplate" => "public.unaccent", "dictinitoption" => "rules = 'unaccent'" })
      add.call("pg_ts_template", "public.unaccent", { "tmplinit" => "public.unaccent_init(pg_catalog.internal)", "tmpllexize" => "public.unaccent_lexize(pg_catalog.internal,pg_catalog.internal,pg_catalog.internal,pg_catalog.internal)" })
      add.call("pg_proc", "public.digest(pg_catalog.text,pg_catalog.text)", {
        "pronamespace" => "public", "proname" => "digest", "prolang" => "c", "prosrc" => "pg_digest", "probin" => "$libdir/pgcrypto", "proconfig" => nil
      })
      31.times do |index|
        child = "public.canonical_torrent"
        parent = index < 28 ? "public.search_page" : "public.search_request"
        identity = "synthetic_fk_#{index} on #{child}"
        add.call("pg_constraint", identity, { "conname" => "synthetic_fk_#{index}", "contype" => "f", "conrelid" => child, "confrelid" => parent,
          "conkey" => [2], "confkey" => [1], "confupdtype" => "a", "confdeltype" => "a",
          "conpfeqop" => ["pg_catalog.=(bigint,bigint)"], "conppeqop" => ["pg_catalog.=(bigint,bigint)"], "conffeqop" => ["pg_catalog.=(bigint,bigint)"] })
        [[child, parent, 5, "check_ins"], [child, parent, 17, "check_upd"], [parent, child, 9, "noaction_del"], [parent, child, 17, "noaction_upd"]].each do |table, other, type, name|
          id = nodes.length + 1
          add.call("pg_trigger", "RI_ConstraintTrigger_c_#{id} on #{table}", {
            "tgname" => "RI_ConstraintTrigger_c_#{id}", "tgisinternal" => true, "tgconstraint" => identity,
            "tgrelid" => table, "tgconstrrelid" => other, "tgtype" => type, "tgfoid" => "pg_catalog.\"RI_FKey_#{name}\"()", "tgenabled" => "O"
          })
        end
      end
      roots = nodes.first(dependency_roots.length).map { |node| node.values_at("catalog", "oid") }
      edges = nodes.drop(1).map { |node| { "from" => roots.first, "to" => node.values_at("catalog", "oid"), "columns" => [0, 0], "via" => "synthetic:binding" } }
      { "graph" => { "version" => "160014", "server" => "synthetic PostgreSQL 16.14", "roots" => roots, "nodes" => nodes, "edges" => edges },
        "native" => { "image" => "synthetic-pinned-image", "files" => { "postgres" => "a" * 64 } } }
    end

    def dependency_graph_tests!(fixture)
      graph = fixture.fetch("graph")
      changed = copy(graph)
      changed["version"] = "160013"
      rejected("PostgreSQL identity changed") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("nodes") << copy(changed.fetch("nodes").first)
      rejected("duplicated") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("nodes").first["identity"] = ""
      rejected("unresolved identity") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("roots") << ["pg_proc", 999_999]
      rejected("root missing") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("roots").pop
      rejected("relation roots incomplete") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("roots") << copy(changed.fetch("roots").first)
      rejected("roots duplicated") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("edges").pop
      rejected("unreachable evidence") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("edges").first["to"] = ["pg_proc", 999_999]
      rejected("escapes inventory") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("edges").first["columns"] = []
      rejected("loses column provenance") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("edges") << copy(changed.fetch("edges").first)
      rejected("edge missing or duplicated") { dependency_validate_graph!(changed) }
      query = dependency_graph_query
      %w[pg_depend pg_attrdef pg_constraint pg_trigger pg_rewrite pg_policy pg_index pg_ts_dict pg_ts_template pg_aggregate pg_am pg_amop pg_amproc pg_sequence].each do |catalog|
        assert(query.include?(catalog), "structural query must include #{catalog}")
      end
      %w[conpfeqop conppeqop conffeqop typinput typoutput typreceive typsend tmplinit tmpllexize aggtransfn aggcombinefn].each do |column|
        assert(query.include?(column), "follow callback/equality binding #{column}")
      end
      assert(query.include?("READ ONLY") && !query.include?("SET plpgsql"), "catalog walk cannot repair or recompile routines")
      rejected("catalog columns drifted") { dependency_metadata_query(graph, { "pg_class" => [] }) }
    end

    def dependency_drift_tests!(fixture)
      mutations = [
        ["pg_class", "relkind", "v"], ["pg_class", "relrowsecurity", true], ["pg_class", "relforcerowsecurity", true],
        ["pg_class", "relhasrules", true], ["pg_class", "columns", [[1, "wrong_column"]]],
        ["pg_class", "constraints", [["check_positive", true, "CHECK ((id >= 0))"]]],
        ["pg_attrdef", "definition", "NULL"], ["pg_cast", "castfunc", "public.wrong_cast(public.policy_action)"],
        ["pg_cast", "castmethod", "b"], ["pg_cast", "castcontext", "i"],
        ["pg_ts_dict", "dicttemplate", "pg_catalog.simple"], ["pg_ts_dict", "dictinitoption", "rules = 'wrong'"],
        ["pg_ts_template", "tmplinit", "public.wrong_init(internal)"], ["pg_ts_template", "tmpllexize", "public.wrong_lexize(internal)"],
        ["pg_proc", "prosrc", "wrong_digest"], ["pg_proc", "probin", "$libdir/wrong"], ["pg_proc", "proconfig", ["search_path=public"]],
        ["pg_constraint", "conpfeqop", ["pg_catalog.<(bigint,bigint)"]], ["pg_constraint", "conppeqop", []], ["pg_constraint", "conffeqop", [nil]],
        ["pg_constraint", "confrelid", "public.policy_rule"], ["pg_constraint", "confdeltype", "c"],
        ["pg_trigger", "tgfoid", "pg_catalog.\"RI_FKey_check_upd\"()"], ["pg_trigger", "tgenabled", "D"]
      ]
      mutations.each do |catalog, field, value|
        changed = copy(fixture)
        changed.fetch("graph").fetch("nodes").find { |node| node.fetch("catalog") == catalog }.fetch("value")[field] = value
        rejected("definition/binding drift") { dependency_validate_pair!(fixture, changed) }
      end
      changed = copy(fixture)
      changed.fetch("graph").fetch("nodes").find { |node| node.fetch("identity") == "public.synthetic_index" }.fetch("value")["index_definition"] = [false, "btree (id)", nil]
      rejected("definition/binding drift") { dependency_validate_pair!(fixture, changed) }
      changed = copy(fixture)
      changed.fetch("graph").fetch("edges").last["via"] = "aggregate:wrong-transition-binding"
      rejected("definition/binding drift") { dependency_validate_pair!(fixture, changed) }
      changed = copy(fixture)
      changed.fetch("native").fetch("files")["postgres"] = "b" * 64
      rejected("definition/binding drift") { dependency_validate_pair!(fixture, changed) }
      %w[relkind relrowsecurity relhasrules].each do |field|
        changed = copy(fixture)
        changed.fetch("graph").fetch("nodes").first.fetch("value")[field] = field == "relkind" ? "v" : true
        rejected("relation kind/RLS/rule dispatch") { dependency_validate_pair!(changed, changed) }
      end
      changed = copy(fixture)
      changed.fetch("graph").fetch("nodes").find { |node| node.fetch("catalog") == "pg_trigger" }.fetch("value")["tgfoid"] = "pg_catalog.\"RI_FKey_check_upd\"()"
      rejected("trigger function dispatch") { dependency_validate_pair!(changed, changed) }
      raw = Marshal.dump(fixture)
      dependency_canonical(fixture, "final")
      assert(Marshal.dump(fixture) == raw, "canonicalization preserves retained raw catalog evidence")
    end

    def dependency_routine_tests!
      @dependency_routines = [FinalSql::Routine.new(identity: "helper(text)", schema: "public", name: "helper", trigger: false, path: "pg_catalog, public")]
      source = { "pronamespace" => "public", "proname" => "helper", "prolang" => "plpgsql", "prosrc" => "BEGIN RETURN lower($1); END;",
                 "proconfig" => ["search_path=pg_catalog, public"], "prosecdef" => true, "definition" => "retained deparser text", "prosqlbody" => nil }
      old = source.merge("proconfig" => nil, "prosecdef" => false)
      final = copy(source)
      dependency_routine!(old, "reference")
      dependency_routine!(final, "final")
      assert(old == final, "only approved authored security/path delta normalizes")
      changed = source.merge("proconfig" => ["search_path=public"])
      rejected("search path changed") { dependency_routine!(changed, "final") }
      changed = source.merge("prosecdef" => false)
      rejected("security mode changed") { dependency_routine!(changed, "final") }
      changed = source.merge("proconfig" => ["plpgsql.variable_conflict=use_column"], "prosecdef" => false)
      rejected("unexpected frozen authored") { dependency_routine!(changed, "reference") }
      changed = source.merge("proconfig" => ["search_path=public"], "prosecdef" => false)
      rejected("unexpected frozen authored") { dependency_routine!(changed, "reference") }
      changed = source.merge("proconfig" => nil)
      rejected("frozen authored dependency security mode changed") { dependency_routine!(changed, "reference") }
      changed = source.merge("proname" => "unreviewed_helper")
      rejected("unreviewed authored") { dependency_routine!(changed, "final") }
      @dependency_routines << FinalSql::Routine.new(identity: "search_result_ingest_v1(uuid)", schema: "public", name: "search_result_ingest_v1", trigger: false, path: "pg_catalog, public")
      body = "\n" + FinalSql::INGESTION_DELTAS.flat_map { |original, (_, count)| Array.new(count, original) }.join("\n")
      frozen = source.merge("proname" => "search_result_ingest_v1", "proconfig" => ["plpgsql.variable_conflict=use_column"], "prosecdef" => false, "prosrc" => body)
      corrected = source.merge("proname" => "search_result_ingest_v1", "prosrc" => "\n#variable_conflict use_column\n#{FinalSql.new(@contract).approved_ingestion_body(body).delete_prefix("\n")}")
      changed = frozen.merge("proconfig" => ["plpgsql.variable_conflict=use_variable"])
      rejected("frozen ingestion compiler setting changed") { dependency_routine!(changed, "reference") }
      dependency_routine!(frozen, "reference")
      dependency_routine!(corrected, "final")
      assert(frozen == corrected, "only exact existing D3/D4/D5 body transforms normalize")
      @dependency_routines = nil
    end

    def dependency_observation_tests!
      relation = "public.canonical_torrent"
      constraints = [{ "contype" => "p", "conrelid" => relation, "conkey" => [1] },
                     { "contype" => "f", "conrelid" => relation, "conkey" => [3], "conname" => "parent_fk" }]
      columns = { relation => { 1 => "id", 3 => "parent_id", 2 => "data" } }
      frame = { "clock" => "recorded-clock", "tables_before" => { "canonical_torrent" => [] },
                "tables_after" => { "canonical_torrent" => [{ "data" => "new", "parent_id" => 2, "id" => 1 }] } }
      event = dependency_fk_events("real-case", frame, constraints, columns).first
      assert(event.fetch(:outcome) == "non-NULL insert eligible for check_ins", "insert uses catalog key positions, not JSON key order")
      frame.fetch("tables_after").fetch("canonical_torrent").first["parent_id"] = nil
      assert(dependency_fk_events("real-case", frame, constraints, columns).first.fetch(:outcome).start_with?("NULL-key"), "nullable FK is not evidence of lookup execution")
      frame.fetch("tables_before")["canonical_torrent"] = [{ "id" => 1, "parent_id" => 1, "data" => "old" }]
      frame.fetch("tables_after").fetch("canonical_torrent").first["parent_id"] = 1
      assert(dependency_fk_events("real-case", frame, constraints, columns).first.fetch(:outcome).start_with?("unchanged-key"), "unchanged key remains explicitly qualified")
      frame.fetch("tables_after").fetch("canonical_torrent").first["parent_id"] = 2
      assert(dependency_fk_events("real-case", frame, constraints, columns).first.fetch(:outcome).start_with?("changed-key"), "observed FK key change is discriminated")
      frame.fetch("tables_after")["canonical_torrent"] = copy(frame.fetch("tables_before").fetch("canonical_torrent"))
      assert(dependency_fk_events("real-case", frame, constraints, columns).empty?, "unchanged rows do not manufacture callback evidence")
      frame.fetch("tables_after")["canonical_torrent"] = []
      assert(dependency_fk_events("real-case", frame, constraints, columns).empty?, "row deletion does not manufacture child callback evidence")
      rejected("primary key missing") { dependency_fk_events("real-case", frame, constraints.drop(1), columns) }
      assert(DEPENDENCY_LIMITS.any? { |text| text.include?("Unchanged referenced keys") }, "retain parent update qualification")
      assert(DEPENDENCY_LIMITS.any? { |text| text.include?("not pg_depend discovery") }, "retain late-binding qualification")
      assert(DEPENDENCY_LIMITS.any? { |text| text.include?("ambient error differs") }, "retain helper compiler-setting qualification")
    end

    def dependency_output_tests!
      Dir.mktmpdir("revaer-native-dependencies-test.") do |directory|
        @dependency_evidence = directory
        dependency_write!("test.json", "{}\n")
        assert(File.stat(File.join(directory, "test.json")).mode & 0o777 == 0o600, "new evidence is private")
        rejected("invalid dependency evidence name") { dependency_write!("../escape", "no") }
        File.symlink(File.join(directory, "test.json"), File.join(directory, "link.json"))
        rejected("must not be a symlink") { dependency_write!("link.json", "no") }
        File.symlink(directory, File.join(directory, "link-dir"))
        rejected("must not be a symlink") { dependency_directory!(File.join(directory, "link-dir")) }
        fake = Object.new
        fake.define_singleton_method(:capture) { |_command, **_options| CommandRunner::Result.new(stdout: "not-json\n", stderr: "", success: true) }
        previous = @runner
        @runner = fake
        rejected("invalid dependency catalog JSON") { dependency_query("SELECT 1", "synthetic", "invalid") }
        fake.define_singleton_method(:capture) { |_command, **_options| CommandRunner::Result.new(stdout: "{}\n", stderr: "NOTICE: unexpected\n", success: true) }
        rejected("transport or diagnostic failure") { dependency_query("SELECT 1", "synthetic", "diagnostic") }
        fake.define_singleton_method(:capture) { |_command, **_options| CommandRunner::Result.new(stdout: "{}\n{}\n", stderr: "", success: true) }
        rejected("graph record missing or duplicated") { dependency_snapshot("synthetic", "duplicate") }
        @runner = previous
      end
    end

    def dependency_native_tests!
      previous = @runner
      image = { "Id" => "sha256:#{'c' * 64}", "RepoDigests" => [@contract.postgres_image.delete_prefix("docker.io/library/")], "Os" => "linux", "Architecture" => "arm64" }
      fake = Object.new
      mode = :valid
      fake.define_singleton_method(:run!) do |command, **_options|
        case command.fetch(1)
        when "image" then JSON.generate(image)
        when "inspect" then mode == :wrong_image ? "sha256:#{'d' * 64}" : image.fetch("Id")
        when "exec"
          lines = command.drop(4).map { |path| "#{'a' * 64}  #{path}\n" }
          lines.pop if mode == :missing_file
          lines << lines.first if mode == :duplicate_file
          lines[0] = "not-a-hash\n" if mode == :invalid_hash
          lines.join
        else raise Failure, "unexpected native identity command"
        end
      end
      @runner = fake
      value = dependency_native_identity
      assert(value.fetch("files").length == 5 && value.fetch("architecture") == "arm64", "native file identities are anchored to the exact selected image")
      mode = :wrong_image
      rejected("container image drift") { dependency_native_identity }
      mode = :missing_file
      rejected("native file identity incomplete") { dependency_native_identity }
      mode = :duplicate_file
      rejected("native file identity incomplete") { dependency_native_identity }
      mode = :invalid_hash
      rejected("invalid native file fingerprint") { dependency_native_identity }
      @runner = previous
    end

    def dependency_cleanup_tests!
      previous = @runner
      commands = []
      fake = Object.new
      fake.define_singleton_method(:capture) do |command, **_options|
        commands << command
        CommandRunner::Result.new(stdout: "", stderr: "failed", success: false)
      end
      @runner = fake
      @owned_container = true
      @owned_volume = true
      @volume = "#{@container}-native-data"
      rejected("exact dependency resource cleanup failed") { dependency_cleanup! }
      assert(commands == [["docker", "rm", "-fv", @container], ["docker", "volume", "rm", @volume]], "container failure still attempts exact owned volume cleanup")
      @owned_container = false
      @owned_volume = false
      commands.clear
      dependency_cleanup!
      assert(commands.empty?, "never clean unowned resources")
      @runner = previous
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    mode = ARGV.empty? ? "--unit" : ARGV.fetch(0)
    raise RevaerDatabaseRebaseline::Failure, "usage: database-ingestion-dependencies-test.rb [--unit|--live]" unless ARGV.length <= 1 && %w[--unit --live].include?(mode)

    contract = RevaerDatabaseRebaseline::Contract.new(root: File.expand_path("../..", __dir__))
    proof = RevaerDatabaseRebaseline::IngestionDependenciesTest.new(contract)
    mode == "--live" ? proof.run_live! : proof.run_tests!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-dependencies: #{error.message}"
    exit 1
  end
end
