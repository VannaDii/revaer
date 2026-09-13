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
      dependency_omission_tests!(fixture)
      dependency_routine_tests!
      dependency_observation_tests!
      dependency_evidence_tests!(fixture)
      dependency_output_tests!
      dependency_native_tests!
      dependency_native_answer_tests!
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
        present = nodes.find { |node| node.values_at("catalog", "identity") == [catalog, identity] }
        next present if present

        defaults = DEPENDENCY_REFERENCES.fetch(catalog, []).to_h { |field| [field, nil] }
        defaults.merge!(DEPENDENCY_ARRAY_REFERENCES.fetch(catalog, {}).transform_values { nil })
        defaults["relam"] = nil if catalog == "pg_class"
        defaults.merge!("pronamespace" => "pg_catalog", "proname" => "synthetic_callback", "proargtypes" => [], "prosrc" => "synthetic") if catalog == "pg_proc"
        value = defaults.merge(value)
        node = { "catalog" => catalog, "oid" => nodes.length + 1, "identity" => identity, "value" => value }
        nodes << node
        node
      end
      dependency_roots.each do |table|
        add.call("pg_class", "public.#{table}", {
          "relkind" => "r", "relam" => "heap", "relrowsecurity" => false, "relforcerowsecurity" => false, "relhasrules" => false,
          "columns" => [[1, "id", "pg_catalog.int8"], [2, "parent_id", "pg_catalog.int8"]],
          "constraints" => [["check_positive", true, "CHECK ((id > 0))"]], "index_definition" => nil
        })
      end
      add.call("pg_cast", "(public.policy_action AS public.decision_type)", {
        "castsource" => "public.policy_action", "casttarget" => "public.decision_type",
        "castfunc" => "public.policy_action_to_decision_type(public.policy_action)", "castcontext" => "a", "castmethod" => "f"
      })
      add.call("pg_ts_dict", "public.unaccent", { "dicttemplate" => "public.unaccent", "dictinitoption" => "rules = 'unaccent'" })
      @dependency_routines = IngestionProof::INGESTION_HELPERS.map do |name|
        type = name == "policy_action_to_decision_type" ? "public.policy_action" : "text"
        FinalSql::Routine.new(identity: "public.#{name}(#{type})", schema: "public", name:, trigger: false, path: "pg_catalog, public")
      end
      (@dependency_routines.map(&:identity) + DEPENDENCY_EXTENSION_ROOTS).each do |identity|
        name, args = dependency_signature(identity)
        add.call("pg_proc", identity, { "pronamespace" => "public", "proname" => name.delete_prefix("public."), "proargtypes" => args, "prolang" => "c" })
      end
      dependency_pristine_roots.each do |catalog, value|
        name, args = dependency_catalog_signature(catalog, value)
        add.call(catalog, "#{name}(#{args.join(',')})", copy(value))
      end
      roots = nodes.map { |node| node.values_at("catalog", "oid") }
      add.call("pg_attrdef", "public.canonical_torrent.id", { "definition" => "gen_random_uuid()" })
      add.call("pg_class", "public.synthetic_index", { "relam" => "btree", "index_definition" => [true, "btree (id)", "(id IS NOT NULL)"] })
      %w[heap btree].zip(%w[heap_tableam_handler bthandler]).each do |name, handler|
        add.call("pg_am", name, { "amname" => name, "handler" => "pg_catalog.#{handler}(pg_catalog.internal)" })
      end
      add.call("pg_ts_template", "public.unaccent", { "tmplinit" => "public.unaccent_init(pg_catalog.internal)", "tmpllexize" => "public.unaccent_lexize(pg_catalog.internal,pg_catalog.internal,pg_catalog.internal,pg_catalog.internal)" })
      31.times do |index|
        child = index.zero? ? "public.search_request_source_observation" : "public.canonical_torrent"
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
      %w[canonical_torrent search_request_source_observation].each do |table|
        add.call("pg_constraint", "#{table}_pkey on public.#{table}", { "contype" => "p", "conrelid" => "public.#{table}", "conkey" => [1] })
      end
      edges = nodes.drop(1).map { |node| { "from" => roots.first, "to" => node.values_at("catalog", "oid"), "columns" => [0, 0], "via" => "synthetic:binding" } }
      index = 0
      while index < nodes.length
        node = nodes.fetch(index)
        canonical = { "identity" => node.values_at("catalog", "identity"), "value" => node.fetch("value") }
        dependency_reference_targets(canonical).each do |catalog, identity, via|
          target = add.call(catalog, identity, {})
          edge = { "from" => node.values_at("catalog", "oid"), "to" => target.values_at("catalog", "oid"), "columns" => [0, 0], "via" => via }
          edges << edge
          edges << edge.merge("via" => "pg_depend:n") if node.fetch("catalog") == "pg_am"
        end
        index += 1
      end
      { "graph" => { "version" => "160014", "server" => "synthetic PostgreSQL 16.14", "roots" => roots, "nodes" => nodes, "edges" => edges.uniq },
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
      changed.fetch("roots").delete_at(1)
      rejected("relation roots incomplete") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      changed.fetch("roots") << copy(changed.fetch("roots").first)
      rejected("roots duplicated") { dependency_validate_graph!(changed) }
      changed = copy(graph)
      isolated = changed.fetch("nodes").find { |node| node.fetch("catalog") == "pg_attrdef" }.values_at("catalog", "oid")
      changed.fetch("edges").reject! { |edge| edge.fetch("to") == isolated }
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
      graph = changed.fetch("graph")
      trigger = graph.fetch("nodes").find { |node| node.fetch("catalog") == "pg_trigger" }
      trigger.fetch("value")["tgfoid"] = "pg_catalog.\"RI_FKey_check_upd\"()"
      target = graph.fetch("nodes").find { |node| node.fetch("identity") == trigger.fetch("value").fetch("tgfoid") }
      graph.fetch("edges").find { |edge| edge.fetch("from") == trigger.values_at("catalog", "oid") && edge.fetch("via") == "pg_trigger:tgfoid" }["to"] = target.values_at("catalog", "oid")
      rejected("trigger function dispatch") { dependency_validate_pair!(changed, changed) }
      raw = Marshal.dump(fixture)
      dependency_canonical(fixture, "final")
      assert(Marshal.dump(fixture) == raw, "canonicalization preserves retained raw catalog evidence")
    end

    def dependency_routine_tests!
      previous = @dependency_routines
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
      @dependency_routines = previous
    end

    def dependency_prune!(graph, removed)
      graph.fetch("roots").reject! { |key| removed.include?(key) }
      graph.fetch("edges").reject! { |edge| removed.include?(edge.fetch("from")) || removed.include?(edge.fetch("to")) }
      reached = graph.fetch("roots").to_h { |key| [key, true] }
      queue = reached.keys
      adjacency = graph.fetch("edges").group_by { |edge| edge.fetch("from") }
      index = 0
      while index < queue.length
        adjacency.fetch(queue.fetch(index), []).each do |edge|
          key = edge.fetch("to")
          next if reached.key?(key)

          reached[key] = true
          queue << key
        end
        index += 1
      end
      graph.fetch("nodes").select! { |node| reached.key?(node.values_at("catalog", "oid")) }
      graph.fetch("edges").select! { |edge| reached.key?(edge.fetch("from")) && reached.key?(edge.fetch("to")) }
    end

    def dependency_omission_tests!(fixture)
      roots = [
        ["public.policy_text_match_v1(text)"], DEPENDENCY_EXTENSION_ROOTS.first(2),
        [DEPENDENCY_EXTENSION_ROOTS.first], ["pg_catalog.lower(text)"], ["pg_catalog.=(bigint,bigint)"]
      ]
      callbacks = [
        %w[pg_catalog.heap_tableam_handler(pg_catalog.internal) pg_catalog.bthandler(pg_catalog.internal)],
        ["public.unaccent_init(pg_catalog.internal)"],
        ["public.unaccent_lexize(pg_catalog.internal,pg_catalog.internal,pg_catalog.internal,pg_catalog.internal)"],
        ['pg_catalog."RI_FKey_check_ins"()']
      ]
      (roots + callbacks).each do |identities|
        changed = copy(fixture)
        graph = changed.fetch("graph")
        removed = graph.fetch("nodes").select { |node| identities.include?(node.fetch("identity")) }.map { |node| node.values_at("catalog", "oid") }
        assert(removed.length == identities.length, "negative control identifies exact root/callback nodes")
        dependency_prune!(graph, removed)
        dependency_validate_graph!(graph)
        assert(dependency_canonical(changed, "reference") == dependency_canonical(changed, "final"), "coherent omission cannot rely on paired inequality")
        expected = roots.include?(identities) ? "root identities/overloads missing or substituted" : "reference target omitted"
        rejected(expected) { dependency_validate_pair!(changed, changed) }
      end
      changed = copy(fixture)
      graph = changed.fetch("graph")
      original_roots = copy(graph.fetch("roots"))
      graph.fetch("edges").reject! { |edge| edge.fetch("via") == "access-method:handler" }
      dependency_validate_graph!(graph)
      assert(graph.fetch("roots") == original_roots, "missing callback edges leave root inventory intact")
      rejected("reference edge omitted or substituted") { dependency_validate_pair!(changed, changed) }
      changed = copy(fixture)
      graph = changed.fetch("graph")
      graph.fetch("nodes").find { |node| node.fetch("catalog") == "pg_am" }.fetch("value").delete("handler")
      rejected("reference field missing") { dependency_validate_pair!(changed, changed) }
    end

    def dependency_evidence_fixture(directory)
      contract = Contract.new(root: @contract.root)
      contract.define_singleton_method(:output_path) { directory }
      proof = self.class.new(contract)
      proof.instance_variable_set(:@dependency_routines, @dependency_routines)
      originals = {}
      checks = []
      %w[compilation wrapper].each do |family|
        path = File.join(directory, "ingestion-#{family}")
        FileUtils.mkdir_p(path)
        path = Dir.mktmpdir("run-", path) if family == "wrapper"
        proof.instance_variable_set("@#{family}_evidence", path)
        entries = proof.send(:dependency_observation_cases, "ingestion-#{family}").to_h do |name, role, count|
          frames = count.times.map do |index|
            before = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
            before["search_request_source_observation"] = [{ "id" => 1, "parent_id" => 1 }]
            after = copy(before)
            after["search_request_source_observation"].first["parent_id"] = 2
            { "clock" => "synthetic-clock-#{index}", "state" => "00000", "role" => { "session" => role }, "tables_before" => before, "tables_after" => after }
          end
          [File.join(path, "#{name}.json"), { "synthetic_case" => name, "frames" => frames, "events" => [] }]
        end
        check = { check: "synthetic #{family} validation", passed: true }
        checks << check
        entries[File.join(path, "report.json")] = { completed: true, passed: true, checks: [check],
          candidate_sha256: contract.expected_candidate_sha256, final_sha256: contract.final_sha256 }
        # Only synthetic unit fixtures register hashes here, from original serialized bytes.
        registry = entries.to_h do |file, record|
          bytes = JSON.pretty_generate(record) + "\n"
          originals[file] = bytes
          File.binwrite(file, bytes)
          [file, Digest::SHA256.hexdigest(bytes)]
        end
        proof.instance_variable_set("@#{family}_validated_evidence", registry)
      end
      proof.instance_variable_set(:@checks, checks)
      [proof, originals]
    end

    def dependency_evidence_tests!(fixture)
      Dir.mktmpdir("revaer-native-registered-evidence.") do |directory|
        proof, originals = dependency_evidence_fixture(directory)
        evidence = proof.send(:dependency_observations, fixture)
        assert(evidence.fetch(:files).length == originals.length, "all report/frame bytes are tied to the explicit unit registries")
        assert(evidence.fetch(:callback_entry_trace) == false, "validated FK input bytes do not claim callback execution")
        %w[compilation wrapper].each do |family|
          variable = "@#{family}_validated_evidence"
          registry = proof.instance_variable_get(variable)
          [nil, {}, "not a registry"].each do |missing|
            proof.instance_variable_set(variable, missing)
            rejected("lacks current-process validated bytes") { proof.send(:dependency_observations, fixture) }
          end
          proof.instance_variable_set(variable, registry)
          files = originals.keys.select { |path| path.include?("/ingestion-#{family}/") }
          report = files.find { |path| path.end_with?("/report.json") }
          frame_file = files.find { |path| JSON.parse(originals.fetch(path)).fetch("frames", []).length > 1 }
          [report, frame_file].each do |path|
            hash = registry.delete(path)
            rejected("lacks current-process validated bytes") { proof.send(:dependency_observations, fixture) }
            registry[path] = hash
            File.binwrite(path, "{")
            rejected("validated evidence bytes changed") { proof.send(:dependency_observations, fixture) }
            File.binwrite(path, originals.fetch(path))
            File.unlink(path)
            rejected("cannot read dependency validated evidence") { proof.send(:dependency_observations, fixture) }
            File.binwrite(path, originals.fetch(path))
          end
          record = JSON.parse(originals.fetch(frame_file))
          calls = record.fetch("frames")
          [[], calls.drop(1), calls + [calls.first], [calls.first] * calls.length].each do |changed|
            File.binwrite(frame_file, JSON.generate(record.merge("frames" => changed)))
            rejected("validated evidence bytes changed") { proof.send(:dependency_observations, fixture) }
          end
          substitute = files.find { |path| path != report && path != frame_file }
          File.binwrite(frame_file, originals.fetch(substitute))
          rejected("validated evidence bytes changed") { proof.send(:dependency_observations, fixture) }
          File.binwrite(frame_file, originals.fetch(frame_file))
        end
        original = proof.instance_variable_get(:@compilation_validated_evidence)
        proof.instance_variable_set(:@compilation_validated_evidence, proof.instance_variable_get(:@wrapper_validated_evidence))
        rejected("lacks current-process validated bytes") { proof.send(:dependency_observations, fixture) }
        proof.instance_variable_set(:@compilation_validated_evidence, original)
        current = proof.instance_variable_get(:@wrapper_evidence)
        proof.instance_variable_set(:@wrapper_evidence, nil)
        rejected("lacks current-process directory") { proof.send(:dependency_observations, fixture) }
        proof.instance_variable_set(:@wrapper_evidence, File.dirname(current))
        rejected("lacks current-process validated bytes") { proof.send(:dependency_observations, fixture) }
        proof.instance_variable_set(:@wrapper_evidence, current)
        assert(proof.send(:dependency_observations, fixture).fetch(:files) == evidence.fetch(:files), "rejected mutations do not change the producers' registries")
        dependency_evidence_reader_tests!(directory)
      end
    end

    def dependency_evidence_reader_tests!(directory)
      path = File.join(directory, "synthetic-invalid.json")
      File.binwrite(path, "{")
      registry = { path => Digest::SHA256.hexdigest("{") }
      rejected("cannot read dependency validated evidence") { dependency_read_observation(path, registry) }
      rejected("lacks current-process validated bytes") { dependency_read_observation(path, { path => "invalid" }) }
      rejected("lacks current-process validated bytes") { dependency_read_observation("./relative.json", { "./relative.json" => "a" * 64 }) }
      File.symlink(path, File.join(directory, "linked.json"))
      linked = File.join(directory, "linked.json")
      rejected("cannot read dependency validated evidence") { dependency_read_observation(linked, { linked => registry.fetch(path) }) }
      call = { "state" => "00000", "role" => { "session" => "postgres" } }
      [[], [call], [call, call]].each do |calls|
        rejected("frames incomplete or duplicated") { dependency_observation_frames({ "frames" => calls }, "postgres", 2) }
      end
      rejected("stale role or failed calls") { dependency_observation_frames({ "frames" => [call] }, "runtime", 1) }
      rejected("stale role or failed calls") { dependency_observation_frames({ "frames" => [call.merge("state" => "XX000")] }, "postgres", 1) }
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

    def dependency_native_answer_tests!
      digest = Digest::SHA256.hexdigest("abc")
      answer = { "title" => "hotel", "unaccent" => "cafe", "digest_text" => digest, "digest_bytes" => digest,
                 "cast" => "flag", "uuid_binding" => "pg_catalog.gen_random_uuid()" }
      records = [{ "backend" => 123, "role" => "postgres", "current" => "postgres", "before" => "error" }, answer, { "after" => "error" }] * 2
      dependency_validate_native_answers!(records, "postgres")
      assert(true, "positive integer same-backend native answers pass")
      [nil, 0, -1, "123", 123.0].each do |backend|
        [[0], [3], [0, 3]].each do |indexes|
          changed = copy(records)
          indexes.each { |index| changed[index] = changed.fetch(index).merge("backend" => backend) }
          rejected("backend identity must be a positive Integer") { dependency_validate_native_answers!(changed, "postgres") }
        end
      end
      changed = copy(records)
      changed[3] = changed.fetch(3).merge("backend" => 124)
      rejected("require the same backend") { dependency_validate_native_answers!(changed, "postgres") }
      changed = copy(records)
      changed.fetch(0).delete("backend")
      rejected("backend identity must be a positive Integer") { dependency_validate_native_answers!(changed, "postgres") }
      [records.drop(1), records + [records.first], nil].each do |changed|
        rejected("known answer or caller provenance changed") { dependency_validate_native_answers!(changed, "postgres") }
      end
      previous = @runner
      fake = Object.new
      fake.define_singleton_method(:capture) do |command, **_options|
        role = command.fetch(command.index("-U") + 1)
        value = records.map { |record| record.key?("backend") ? record.merge("role" => role, "current" => role) : record }
        CommandRunner::Result.new(stdout: value.map { |record| JSON.generate(record) + "\n" }.join, stderr: "", success: true)
      end
      Dir.mktmpdir("revaer-native-answer-unit.") do |directory|
        @runner = fake
        @dependency_evidence = directory
        assert(dependency_native_answers!.keys == %w[reference final], "native transport uses the strict answer validator for both variants")
      end
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
