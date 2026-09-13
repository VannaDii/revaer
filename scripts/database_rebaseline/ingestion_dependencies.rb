# frozen_string_literal: true

require "json"
require "digest"
require_relative "../postgres_pristine/container"
require_relative "../postgres_pristine/query"

module RevaerDatabaseRebaseline
  # Case 11: catalog edges are not a PL/pgSQL parser or callback execution trace.
  module IngestionDependencies
    DEPENDENCY_READS = %w[
      canonical_disambiguation_rule canonical_torrent_source_base_score
      indexer_instance search_request search_request_indexer_run trust_tier
      policy_snapshot_rule policy_rule policy_set policy_rule_value_set_item
      indexer_instance_media_domain media_domain search_profile_tag_prefer indexer_instance_tag
    ].freeze
    DEPENDENCY_DML = {
      "canonical_torrent" => %w[insert update], "canonical_torrent_source" => %w[insert update],
      "canonical_torrent_source_attr" => %w[insert update], "canonical_torrent_source_context_score" => %w[insert update],
      "canonical_torrent_best_source_context" => %w[insert update], "search_request_source_observation" => %w[insert update],
      "search_request_source_observation_attr" => %w[insert update], "canonical_torrent_signal" => %w[insert update],
      "canonical_external_id" => %w[insert update], "canonical_size_sample" => %w[insert delete],
      "canonical_size_rollup" => %w[insert update], "search_request_canonical" => %w[insert],
      "search_page" => %w[insert update], "search_page_item" => %w[insert], "search_filter_decision" => %w[insert],
      "source_metadata_conflict" => %w[insert], "source_metadata_conflict_audit_log" => %w[insert],
      "indexer_health_event" => %w[insert]
    }.freeze
    DEPENDENCY_NATIVE_NAMES = %w[
      lower btrim char_length replace regexp_replace regexp_matches split_part strpos position substring
      string_to_array array_to_string unnest encode decode now array_length array_agg
      count min max sum percentile_cont gen_random_uuid
    ].freeze
    DEPENDENCY_NATIVE_OPERATORS = %w[= <> < <= > >= + - * / % || ~ ~* !~ !~* ~~ ~~* !~~ !~~*].freeze
    DEPENDENCY_EXTENSION_ROOTS = [
      "public.digest(bytea,text)", "public.digest(text,text)",
      "public.unaccent(text)", "public.unaccent(regdictionary,text)"
    ].freeze
    DEPENDENCY_REFERENCES = {
      "pg_proc" => %w[prolang prorettype provariadic prosupport],
      "pg_type" => %w[typelem typarray typbasetype typcollation typsubscript typinput typoutput typreceive typsend typmodin typmodout typanalyze],
      "pg_trigger" => %w[tgfoid tgconstraint], "pg_cast" => %w[castsource casttarget castfunc],
      "pg_operator" => %w[oprleft oprright oprresult oprcom oprnegate oprcode oprrest oprjoin],
      "pg_opclass" => %w[opcmethod opcfamily opcintype opckeytype], "pg_opfamily" => %w[opfmethod],
      "pg_amop" => %w[amoplefttype amoprighttype amopopr amopmethod amopsortfamily],
      "pg_amproc" => %w[amproclefttype amprocrighttype amproc],
      "pg_ts_dict" => %w[dicttemplate], "pg_ts_template" => %w[tmplinit tmpllexize],
      "pg_language" => %w[lanplcallfoid laninline lanvalidator]
    }.freeze
    DEPENDENCY_ARRAY_REFERENCES = {
      "pg_proc" => { "proargtypes" => "pg_type", "proallargtypes" => "pg_type", "protrftypes" => "pg_type" },
      "pg_constraint" => %w[conpfeqop conppeqop conffeqop conexclop].to_h { |field| [field, "pg_operator"] }
    }.freeze
    DEPENDENCY_LIMITS = [
      "Explicit relation/routine roots come from the source-pinned case-11 audit, not pg_depend discovery of PL/pgSQL strings.",
      "Native function/operator roots include installed overload families; presence does not prove PL/pgSQL overload selection or execution.",
      "Only ingestion relation roots expand attached defaults/checks/indexes/rules/policies/triggers; other reached relations are dependency identities, not new DML roots.",
      "Catalog definitions exclude physical statistics and ownership/ACL fields; existing FinalProof privilege and D1 gates remain mandatory.",
      "Native C/internal bodies, RI SPI plans, type I/O and access-method callbacks are anchored to the image and retained native file identities, not PL/pgSQL variable substitution.",
      "Observed row changes establish eligible FK dispatch and known skip inputs, not a direct trace of internal callback entry or every callback branch.",
      "Parent-side FK slots are installed metadata. Unchanged referenced keys do not establish parent update callback execution; size-sample deletion has no incoming FK in this inventory.",
      "Fixture search_request_indexer_run trigger dispatch is separate from ingestion, which only reads that relation.",
      "Triggers attached to read-only inputs are catalog inventory, not ingestion DML reachability.",
      "PL/pgSQL helpers first compiled inside frozen ingestion may inherit use_column; final ambient error differs. Existing in-call observations cover only their recorded successful paths.",
      "No complete late-binding, dynamic native-provider, error-path, architecture, amd64, package, or full-D3 certification is issued."
    ].freeze
    DEPENDENCY_EXTRA_CATALOGS = %w[pg_constraint pg_attrdef pg_rewrite pg_am].freeze

    private

    def verify_ingestion_dependencies!
      @contract.validate_output_path!
      @dependency_evidence = File.join(@contract.output_path, "ingestion-dependencies")
      dependency_directory!(@dependency_evidence)
      first = @checks.length
      report = { completed: false, passed: false, d3_complete: false, limits: DEPENDENCY_LIMITS,
                 candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
                 postgres_image: @contract.postgres_image, read_only_roots: DEPENDENCY_READS, authored_dml: DEPENDENCY_DML,
                 variants: {}, observations: {} }
      begin
        %w[reference final].each do |variant|
          database = variant == "reference" ? "reference_proof" : @database
          report[:variants][variant] = dependency_snapshot(database, variant)
        end
        reference, final = report.fetch(:variants).values_at("reference", "final")
        dependency_validate_pair!(reference, final)
        check("ingestion dependencies exact paired catalog and native identity", true)
        yield if block_given?
        report[:observations] = dependency_observations(final)
        report[:native_answers] = dependency_native_answers!
        report[:completed] = true
      ensure
        report[:checks] = @checks.drop(first)
        report[:passed] = report[:completed] && report[:checks].all? { |entry| entry.fetch(:passed) }
        dependency_write!("report.json", JSON.pretty_generate(report) + "\n")
      end
    end

    def dependency_directory!(path)
      raise Failure, "dependency evidence directory must not be a symlink" if File.symlink?(path)

      FileUtils.mkdir_p(path, mode: 0o700)
      File.chmod(0o700, path)
    end

    def dependency_write!(name, bytes)
      raise Failure, "invalid dependency evidence name" unless name.match?(/\A[a-zA-Z0-9_.-]+\z/)

      path = File.join(@dependency_evidence, name)
      raise Failure, "dependency evidence file must not be a symlink" if File.symlink?(path)

      File.open(path, File::WRONLY | File::CREAT | File::TRUNC | File::NOFOLLOW, 0o600) { |file| file.write(bytes) }
      File.chmod(0o600, path)
    end

    def dependency_query(query, database, label)
      dependency_write!("#{label}.sql", query)
      outcome = result("\\set VERBOSITY verbose\n#{query}", role: "postgres", database:)
      dependency_write!("#{label}.stdout", outcome.stdout)
      dependency_write!("#{label}.stderr", outcome.stderr)
      raise Failure, "dependency catalog transport or diagnostic failure: #{label}" unless outcome.success && outcome.stderr.empty?

      outcome.stdout.lines.map { |line| JSON.parse(line) }
    rescue JSON::ParserError
      raise Failure, "invalid dependency catalog JSON: #{label}"
    end

    def dependency_roots
      (DEPENDENCY_READS + DEPENDENCY_DML.keys).sort
    end

    def dependency_edges_sql
      edges = ["SELECT classid, objid, objsubid, refclassid, refobjid, refobjsubid, 'pg_depend:' || deptype::text AS via FROM pg_depend WHERE objid <> 0 AND refobjid <> 0"]
      attach = {
        "pg_attrdef" => "adrelid", "pg_constraint" => "conrelid", "pg_trigger" => "tgrelid",
        "pg_rewrite" => "ev_class", "pg_policy" => "polrelid"
      }
      attach.each do |catalog, relation|
        edges << "SELECT 'pg_class'::regclass, t.#{relation}, 0, '#{catalog}'::regclass, t.oid, 0, 'relation:#{catalog}' FROM #{catalog} t JOIN relation_roots r ON r.oid=t.#{relation}"
      end
      edges << "SELECT 'pg_class'::regclass, i.indrelid, 0, 'pg_class'::regclass, i.indexrelid, 0, 'relation:index' FROM pg_index i JOIN relation_roots r ON r.oid=i.indrelid"
      edges << "SELECT 'pg_class'::regclass, d.refobjid, d.refobjsubid, 'pg_class'::regclass, d.objid, 0, 'relation:owned-sequence' FROM pg_depend d JOIN relation_roots r ON r.oid=d.refobjid JOIN pg_class s ON s.oid=d.objid AND s.relkind='S' WHERE d.classid='pg_class'::regclass AND d.refclassid='pg_class'::regclass AND d.deptype IN ('i','a')"
      edges << "SELECT 'pg_class'::regclass, seqrelid, 0, 'pg_type'::regclass, seqtypid, 0, 'sequence:type' FROM pg_sequence"
      edges << "SELECT 'pg_class'::regclass, a.attrelid, a.attnum, 'pg_type'::regclass, a.atttypid, 0, 'column:type' FROM pg_attribute a JOIN relation_roots r ON r.oid=a.attrelid WHERE a.attnum>0 AND NOT a.attisdropped"
      edges << "SELECT 'pg_class'::regclass, a.attrelid, a.attnum, 'pg_collation'::regclass, a.attcollation, 0, 'column:collation' FROM pg_attribute a JOIN relation_roots r ON r.oid=a.attrelid WHERE a.attnum>0 AND NOT a.attisdropped AND a.attcollation<>0"
      DEPENDENCY_REFERENCES.each do |catalog, columns|
        columns.each do |column|
          target = RevaerPostgresPristine::REFERENCES.fetch(column)
          edges << "SELECT '#{catalog}'::regclass, t.oid, 0, '#{target}'::regclass, t.#{column}::oid, 0, '#{catalog}:#{column}' FROM #{catalog} t WHERE t.#{column}::oid<>0"
        end
      end
      %w[proargtypes proallargtypes protrftypes].each do |column|
        edges << "SELECT 'pg_proc'::regclass, t.oid, 0, 'pg_type'::regclass, v, 0, 'pg_proc:#{column}' FROM pg_proc t CROSS JOIN LATERAL unnest(t.#{column}::oid[]) v WHERE v<>0"
      end
      %w[conpfeqop conppeqop conffeqop conexclop].each do |column|
        edges << "SELECT 'pg_constraint'::regclass, t.oid, 0, 'pg_operator'::regclass, v, 0, 'pg_constraint:#{column}' FROM pg_constraint t CROSS JOIN LATERAL unnest(t.#{column}) v WHERE v<>0"
      end
      { "indclass" => "pg_opclass", "indcollation" => "pg_collation" }.each do |column, target|
        edges << "SELECT 'pg_class'::regclass, t.indexrelid, 0, '#{target}'::regclass, v, 0, 'pg_index:#{column}' FROM pg_index t CROSS JOIN LATERAL unnest(t.#{column}::oid[]) v WHERE v<>0"
      end
      %w[pg_amop pg_amproc].each do |catalog|
        column = catalog == "pg_amop" ? "amopfamily" : "amprocfamily"
        edges << "SELECT 'pg_opfamily'::regclass, t.#{column}, 0, '#{catalog}'::regclass, t.oid, 0, 'family:#{catalog}' FROM #{catalog} t"
      end
      edges << "SELECT 'pg_am'::regclass, oid, 0, 'pg_proc'::regclass, amhandler::oid, 0, 'access-method:handler' FROM pg_am WHERE amhandler::oid<>0"
      edges << "SELECT 'pg_class'::regclass, oid, 0, 'pg_am'::regclass, relam, 0, 'relation:access-method' FROM pg_class WHERE relam<>0"
      %w[aggtransfn aggfinalfn aggcombinefn aggserialfn aggdeserialfn aggmtransfn aggminvtransfn aggmfinalfn].each do |column|
        edges << "SELECT 'pg_proc'::regclass, aggfnoid::oid, 0, 'pg_proc'::regclass, #{column}::oid, 0, 'aggregate:#{column}' FROM pg_aggregate WHERE #{column}::oid<>0"
      end
      %w[aggtranstype aggmtranstype].each do |column|
        edges << "SELECT 'pg_proc'::regclass, aggfnoid::oid, 0, 'pg_type'::regclass, #{column}, 0, 'aggregate:#{column}' FROM pg_aggregate WHERE #{column}<>0"
      end
      edges << "SELECT 'pg_proc'::regclass, aggfnoid::oid, 0, 'pg_operator'::regclass, aggsortop, 0, 'aggregate:sort-operator' FROM pg_aggregate WHERE aggsortop<>0"
      edges.join("\nUNION\n")
    end

    def dependency_graph_query
      <<~SQL
        BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY;
        SET LOCAL search_path TO pg_catalog;
        SET LOCAL statement_timeout TO '120s';
        WITH RECURSIVE relation_roots AS MATERIALIZED (
          SELECT oid FROM pg_class WHERE relnamespace='public'::regnamespace AND relname IN (#{dependency_roots.map { |name| literal(name) }.join(',')})
        ), roots(classid,objid) AS (
          SELECT 'pg_class'::regclass,oid FROM relation_roots
          UNION SELECT 'pg_proc'::regclass,oid FROM pg_proc WHERE
            (pronamespace='public'::regnamespace AND proname IN (#{(IngestionProof::INGESTION_HELPERS + %w[digest unaccent]).map { |name| literal(name) }.join(',')})) OR
            (pronamespace='pg_catalog'::regnamespace AND proname IN (#{DEPENDENCY_NATIVE_NAMES.map { |name| literal(name) }.join(',')}))
          UNION SELECT 'pg_cast'::regclass,oid FROM pg_cast WHERE castsource='public.policy_action'::regtype AND casttarget='public.decision_type'::regtype
          UNION SELECT 'pg_ts_dict'::regclass,oid FROM pg_ts_dict WHERE dictnamespace=(SELECT pronamespace FROM pg_proc WHERE oid='public.unaccent(text)'::regprocedure) AND dictname='unaccent'
          UNION SELECT 'pg_operator'::regclass,oid FROM pg_operator WHERE oprnamespace='pg_catalog'::regnamespace AND oprname IN (#{DEPENDENCY_NATIVE_OPERATORS.map { |name| literal(name) }.join(',')})
        ), edges(classid,objid,objsubid,refclassid,refobjid,refobjsubid,via) AS MATERIALIZED (
          #{dependency_edges_sql}
        ), walk(classid,objid) AS (
          SELECT classid,objid FROM roots
          UNION SELECT e.refclassid,e.refobjid FROM walk w JOIN edges e ON e.classid=w.classid AND e.objid=w.objid
        )
        SELECT json_build_object(
          'version',current_setting('server_version_num'),'server',version(),
          'roots',(SELECT json_agg(json_build_array(classid::regclass::text,objid) ORDER BY classid,objid) FROM roots),
          'nodes',(SELECT json_agg(json_build_object('catalog',classid::regclass::text,'oid',objid,'identity',(pg_identify_object(classid,objid,0)).identity) ORDER BY classid,objid) FROM walk),
          'edges',(SELECT json_agg(json_build_object('from',json_build_array(e.classid::regclass::text,e.objid),'to',json_build_array(e.refclassid::regclass::text,e.refobjid),'columns',json_build_array(e.objsubid,e.refobjsubid),'via',e.via) ORDER BY e.classid,e.objid,e.objsubid,e.refclassid,e.refobjid,e.refobjsubid,e.via)
            FROM edges e JOIN walk w ON w.classid=e.classid AND w.objid=e.objid)
        );
        ROLLBACK;
      SQL
    end

    def dependency_extra_fields(catalog, query)
      ref = ->(target, value) { query.reference(target, value) }
      case catalog
      when "pg_constraint"
        fields = %w[conname contype condeferrable condeferred convalidated conislocal coninhcount connoinherit confupdtype confdeltype confmatchtype conkey confkey confdelsetcols]
        fields.map { |name| "t.#{name}" } +
          { "conrelid" => "pg_class", "confrelid" => "pg_class", "conindid" => "pg_class", "contypid" => "pg_type", "conparentid" => "pg_constraint" }.map { |name, target| "#{ref.call(target, "t.#{name}")} AS #{name}" } +
          %w[conpfeqop conppeqop conffeqop conexclop].map { |name| "ARRAY(SELECT #{ref.call('pg_operator', 'v')} FROM unnest(t.#{name}) WITH ORDINALITY u(v,n) ORDER BY n) AS #{name}" } +
          ["pg_get_constraintdef(t.oid,false) AS definition"]
      when "pg_attrdef"
        ["#{ref.call('pg_class', 't.adrelid')} AS relation", "t.adnum", "pg_get_expr(t.adbin,t.adrelid,false) AS definition"]
      when "pg_rewrite"
        ["t.rulename", "t.ev_type", "t.ev_enabled", "t.is_instead", "#{ref.call('pg_class', 't.ev_class')} AS relation", "pg_get_ruledef(t.oid,false) AS definition"]
      when "pg_am"
        ["t.amname", "t.amtype", "#{ref.call('pg_proc', 't.amhandler::oid')} AS handler"]
      else
        raise Failure, "unsupported additional dependency catalog: #{catalog}"
      end
    end

    def dependency_metadata_query(graph, columns)
      query = RevaerPostgresPristine::Query.new
      statements = graph.fetch("nodes").group_by { |node| node.fetch("catalog") }.sort.map do |catalog, nodes|
        fields = if DEPENDENCY_EXTRA_CATALOGS.include?(catalog)
                   dependency_extra_fields(catalog, query)
                 else
                   expected = RevaerPostgresPristine::COLUMNS.fetch(catalog) { raise Failure, "unsupported dependency catalog: #{catalog}" }
                   inventory = columns.fetch(catalog) { raise Failure, "dependency catalog columns drifted or missing: #{catalog}" }
                   raise Failure, "dependency catalog columns drifted: #{catalog}" unless inventory.map(&:first) == expected

                   inventory.filter_map do |name, type|
                     next if name == "oid" || RevaerPostgresPristine::EXCLUDED.key?(name) || name.end_with?("owner", "acl")

                     "#{query.expression(name, type)} AS #{identifier(name)}"
                   end + query.extras(catalog)
                 end
        if catalog == "pg_proc"
          fields << "CASE WHEN t.prokind<>'a' THEN pg_get_functiondef(t.oid) END AS definition"
          fields << "(SELECT to_jsonb(a)-'aggfnoid'-ARRAY['aggtransfn','aggfinalfn','aggcombinefn','aggserialfn','aggdeserialfn','aggmtransfn','aggminvtransfn','aggmfinalfn','aggtranstype','aggmtranstype','aggsortop'] FROM pg_aggregate a WHERE a.aggfnoid=t.oid) AS aggregate"
        end
        fields << "(SELECT to_jsonb(s)-'seqrelid'-'seqtypid' FROM pg_sequence s WHERE s.seqrelid=t.oid) AS sequence" if catalog == "pg_class"
        "SELECT json_build_object('catalog',#{literal(catalog)},'oid',t.oid,'value',row_to_json(v)) FROM #{identifier(catalog)} t CROSS JOIN LATERAL (SELECT #{fields.join(',')}) v WHERE t.oid IN (#{nodes.map { |node| Integer(node.fetch('oid')) }.join(',')}) ORDER BY t.oid;"
      end
      "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL search_path TO pg_catalog; SET LOCAL statement_timeout TO '120s';\n#{statements.join("\n")}\nROLLBACK;"
    end

    def dependency_snapshot(database, variant)
      records = dependency_query(dependency_graph_query, database, "#{variant}-graph")
      raise Failure, "dependency graph record missing or duplicated" unless records.length == 1

      graph = records.fetch(0)
      dependency_validate_graph!(graph)
      inventory = dependency_query(RevaerPostgresPristine::Query.new.inventory_sql, database, "#{variant}-columns")
      columns = inventory.to_h { |row| [row.fetch("catalog"), row.fetch("columns")] }
      metadata = dependency_query(dependency_metadata_query(graph, columns), database, "#{variant}-metadata")
      raise Failure, "dependency metadata inventory incomplete" unless metadata.map { |row| row.values_at("catalog", "oid") }.sort == graph.fetch("nodes").map { |node| node.values_at("catalog", "oid") }.sort

      values = metadata.to_h { |row| [row.values_at("catalog", "oid"), row.fetch("value")] }
      graph.fetch("nodes").each do |node|
        node["value"] = values.fetch(node.values_at("catalog", "oid"))
        node["metadata_sha256"] = Digest::SHA256.hexdigest(JSON.generate(node.fetch("value")))
        node["identity_sha256"] = Digest::SHA256.hexdigest(JSON.generate(node.values_at("catalog", "identity")))
        %w[definition prosrc].each do |field|
          value = node.fetch("value")[field]
          node["#{field}_sha256"] = Digest::SHA256.hexdigest(value) if value.is_a?(String)
        end
      end
      snapshot = { "graph" => graph, "native" => dependency_native_identity }
      dependency_write!("#{variant}.json", JSON.pretty_generate(snapshot) + "\n")
      snapshot
    end

    def dependency_validate_graph!(graph)
      raise Failure, "dependency PostgreSQL identity changed" unless graph.fetch("version") == "160014"

      nodes = graph.fetch("nodes")
      keys = nodes.map { |node| node.values_at("catalog", "oid") }
      indexed = nodes.to_h { |node| [node.values_at("catalog", "oid"), node] }
      raise Failure, "dependency graph is empty, excessive or duplicated" unless nodes.length.between?(1, 20_000) && keys.uniq == keys
      raise Failure, "dependency root missing" unless (graph.fetch("roots") - keys).empty?
      raise Failure, "dependency roots duplicated" unless graph.fetch("roots").uniq == graph.fetch("roots")
      raise Failure, "dependency graph contains unresolved identity" unless nodes.all? { |node| node.fetch("identity").is_a?(String) && !node.fetch("identity").empty? }

      edges = graph.fetch("edges")
      raise Failure, "dependency edge missing or duplicated" if edges.empty? || edges.uniq != edges
      raise Failure, "dependency edge escapes inventory or loses column provenance" unless edges.all? { |edge| indexed.key?(edge.fetch("from")) && indexed.key?(edge.fetch("to")) && edge.fetch("columns").length == 2 && edge.fetch("columns").all? { |column| column.is_a?(Integer) && column >= 0 } }

      adjacency = edges.group_by { |edge| edge.fetch("from") }
      queue = graph.fetch("roots").dup
      reached = queue.to_h { |key| [key, true] }
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
      raise Failure, "dependency graph contains unreachable evidence" unless reached.keys.sort == keys.sort
      relations = graph.fetch("roots").filter_map do |key|
        indexed.fetch(key).fetch("identity") if key.first == "pg_class"
      end
      raise Failure, "ingestion relation roots incomplete" unless relations.sort == dependency_roots.map { |name| "public.#{name}" }
    end

    def dependency_native_identity
      format = '{"Id":{{json .Id}},"RepoDigests":{{json .RepoDigests}},"Os":{{json .Os}},"Architecture":{{json .Architecture}}}'
      image = JSON.parse(@runner.run!(["docker", "image", "inspect", @contract.postgres_image, "--format", format]))
      container_image = @runner.run!(["docker", "inspect", @container, "--format", '{{.Image}}']).strip
      raise Failure, "dependency container image drift" unless image.fetch("Id") == container_image && image.fetch("RepoDigests").include?(@contract.postgres_image.delete_prefix("docker.io/library/"))

      files = %w[/usr/local/bin/postgres /usr/local/lib/postgresql/pgcrypto.so /usr/local/lib/postgresql/unaccent.so /usr/local/lib/postgresql/plpgsql.so /usr/local/share/postgresql/tsearch_data/unaccent.rules]
      lines = @runner.run!(["docker", "exec", @container, "sha256sum", *files]).lines(chomp: true)
      raise Failure, "native file identity incomplete or duplicated" unless lines.length == files.length

      hashes = lines.to_h do |line|
        match = line.match(/\A([0-9a-f]{64})  (\/[^\n]+)\z/)
        raise Failure, "invalid native file fingerprint" unless match

        [match[2], match[1]]
      end
      raise Failure, "native file identity incomplete" unless hashes.keys == files

      { "image" => @contract.postgres_image, "image_id" => container_image, "os" => image.fetch("Os"), "architecture" => image.fetch("Architecture"), "files" => hashes }
    end

    def dependency_canonical(snapshot, variant)
      graph = snapshot.fetch("graph")
      dependency_validate_graph!(graph)
      identities = {}
      nodes = graph.fetch("nodes").map do |node|
        value = Marshal.load(Marshal.dump(node.fetch("value")))
        identity = node.fetch("identity")
        if node.fetch("catalog") == "pg_trigger" && value.fetch("tgisinternal")
          raise Failure, "unexpected internal trigger identity" unless value.fetch("tgname").match?(/\ARI_ConstraintTrigger_[ac]_[1-9][0-9]*\z/) && value.fetch("tgconstraint")

          value["tgname"] = "<FK-slot:#{value.fetch('tgtype')}:#{value.fetch('tgconstrrelid')}>"
          identity = "#{value.fetch('tgconstraint')} ON #{value.fetch('tgrelid')} #{value.fetch('tgname')}"
        end
        dependency_routine!(value, variant) if node.fetch("catalog") == "pg_proc"
        key = [node.fetch("catalog"), identity]
        identities[node.values_at("catalog", "oid")] = key
        { "identity" => key, "value" => value }
      end
      raise Failure, "duplicate logical dependency identity" unless nodes.map { |node| node.fetch("identity") }.uniq.length == nodes.length

      { "nodes" => nodes.sort_by { |node| node.fetch("identity") },
        "roots" => graph.fetch("roots").map { |key| identities.fetch(key) }.sort,
        "edges" => graph.fetch("edges").map { |edge| edge.merge("from" => identities.fetch(edge.fetch("from")), "to" => identities.fetch(edge.fetch("to"))) }.sort_by { |edge| JSON.generate(edge) },
        "version" => graph.fetch("version"), "server" => graph.fetch("server"), "native" => snapshot.fetch("native") }
    end

    def dependency_routine!(value, variant)
      return unless %w[public revaer_config revaer_runtime].include?(value.fetch("pronamespace")) && %w[sql plpgsql].include?(value.fetch("prolang"))

      @dependency_routines ||= FinalSql.new(@contract).routines(@candidate)
      candidates = @dependency_routines.select { |routine| routine.schema == value.fetch("pronamespace") && routine.name == value.fetch("proname") }
      raise Failure, "unreviewed authored dependency routine" unless candidates.length == 1

      routine = candidates.first
      settings = Array(value.fetch("proconfig"))
      if variant == "final"
        raise Failure, "authored dependency search path changed" unless settings == ["search_path=#{routine.path}"]
        raise Failure, "authored dependency security mode changed" unless value.fetch("prosecdef") == !routine.trigger
      else
        raise Failure, "frozen authored dependency security mode changed" unless value.fetch("prosecdef") == false

        if routine.name == "search_result_ingest_v1"
          raise Failure, "frozen ingestion compiler setting changed" unless settings == ["plpgsql.variable_conflict=use_column"]

          value["prosrc"] = "\n#variable_conflict use_column\n#{FinalSql.new(@contract).approved_ingestion_body(value.fetch('prosrc')).delete_prefix("\n")}"
          settings = []
        end
        raise Failure, "unexpected frozen authored dependency settings" unless settings.empty?
      end
      value["proconfig"] = ["<validated-approved-search-path>"]
      value["prosecdef"] = "<approved-authored-security>"
      # All body, argument and behavior fields remain compared. The raw deparser
      # rendering is retained separately; its approved SET/SECURITY text differs.
      value.delete("definition")
      value.delete("prosqlbody") if value["prosqlbody"].nil?
    end

    def dependency_validate_pair!(reference, final)
      old = dependency_canonical(reference, "reference")
      fresh = dependency_canonical(final, "final")
      unless old == fresh
        changed = (old.fetch("nodes") - fresh.fetch("nodes")).map { |node| node.fetch("identity").join(":") }
        raise Failure, "dependency definition/binding drift: #{changed.first(8).join(', ')}"
      end
      dependency_validate_declared_roots!(fresh)
      dependency_validate_bindings!(fresh)
      dependency_validate_dispatch!(fresh)
    end

    def dependency_signature(identity)
      match = identity.match(/\A([^()]+)\(([^()]*)\)\z/)
      raise Failure, "invalid dependency routine/operator identity" unless match

      name = match[1].gsub(/"([a-z_][a-z0-9_]*)"/, '\1')
      [name, match[2].split(",").map { |type| type.strip.delete_prefix("pg_catalog.") }]
    end

    def dependency_pristine_roots
      @dependency_pristine_roots ||= begin
        path = File.join(@contract.root, "config/postgres-pristine-16.14.tsv")
        rows = File.foreach(path).filter_map do |line|
          catalog, *fields = line.chomp.split("\t")
          next unless %w[pg_proc pg_operator].include?(catalog)

          values = fields.to_h do |field|
            key, encoded = field.split("=", 2)
            [key, JSON.parse(encoded)]
          end
          raise Failure, "duplicate pristine dependency field" unless values.length == fields.length

          [catalog, values]
        end
        functions = rows.select { |catalog, row| catalog == "pg_proc" && row.fetch("pronamespace") == "pg_catalog" && DEPENDENCY_NATIVE_NAMES.include?(row.fetch("proname")) }
        operators = rows.select { |catalog, row| catalog == "pg_operator" && row.fetch("oprnamespace") == "pg_catalog" && DEPENDENCY_NATIVE_OPERATORS.include?(row.fetch("oprname")) }
        unless functions.map { |_, row| row.fetch("proname") }.uniq.sort == DEPENDENCY_NATIVE_NAMES.sort && operators.map { |_, row| row.fetch("oprname") }.uniq.sort == DEPENDENCY_NATIVE_OPERATORS.sort
          raise Failure, "declared native dependency families missing from pinned catalog"
        end
        functions + operators
      end
    rescue JSON::ParserError, Errno::ENOENT => error
      raise Failure, "cannot read pinned dependency root inventory: #{error.message}"
    end

    def dependency_catalog_signature(catalog, value)
      if catalog == "pg_proc"
        name = "#{value.fetch('pronamespace')}.#{value.fetch('proname')}"
        args = value.fetch("proargtypes")
      else
        name = "#{value.fetch('oprnamespace')}.#{value.fetch('oprname')}"
        args = value.values_at("oprleft", "oprright").map { |type| type || "NONE" }
      end
      [name, args.map { |type| type.delete_prefix("pg_catalog.") }]
    end

    def dependency_validate_declared_roots!(snapshot)
      @dependency_routines ||= FinalSql.new(@contract).routines(@candidate)
      authored = @dependency_routines.select { |routine| routine.schema == "public" && IngestionProof::INGESTION_HELPERS.include?(routine.name) }
      raise Failure, "declared authored dependency roots incomplete" unless authored.map(&:name).sort == IngestionProof::INGESTION_HELPERS.sort

      expected = authored.map { |routine| ["pg_proc", *dependency_signature(routine.identity)] }
      expected.concat(DEPENDENCY_EXTENSION_ROOTS.map { |identity| ["pg_proc", *dependency_signature(identity)] })
      expected.concat(dependency_pristine_roots.map { |catalog, value| [catalog, *dependency_catalog_signature(catalog, value)] })
      expected.concat(dependency_roots.map { |table| ["pg_class", "public.#{table}"] })
      expected.concat([["pg_cast", "(public.policy_action AS public.decision_type)"], ["pg_ts_dict", "public.unaccent"]])
      nodes = snapshot.fetch("nodes").to_h { |node| [node.fetch("identity"), node.fetch("value")] }
      actual = snapshot.fetch("roots").map do |key|
        catalog, identity = key
        next key unless %w[pg_proc pg_operator].include?(catalog)

        signature = dependency_signature(identity)
        raise Failure, "dependency root identity disagrees with signature fields" unless signature == dependency_catalog_signature(catalog, nodes.fetch(key))

        [catalog, *signature]
      end
      raise Failure, "declared dependency root identities/overloads missing or substituted" unless expected.sort == actual.sort
    end

    def dependency_reference_targets(node)
      catalog = node.fetch("identity").first
      value = node.fetch("value")
      references = DEPENDENCY_REFERENCES.fetch(catalog, []).map do |field|
        [RevaerPostgresPristine::REFERENCES.fetch(field), value.fetch(field), "#{catalog}:#{field}"]
      end
      DEPENDENCY_ARRAY_REFERENCES.fetch(catalog, {}).each do |field, target|
        Array(value.fetch(field)).each { |identity| references << [target, identity, "#{catalog}:#{field}"] }
      end
      references << ["pg_proc", value.fetch("handler"), "access-method:handler"] if catalog == "pg_am"
      references << ["pg_am", value.fetch("relam"), "relation:access-method"] if catalog == "pg_class"
      references.reject { |_, identity, _| identity.nil? }
    rescue KeyError => error
      raise Failure, "dependency reference field missing: #{error.message}"
    end

    def dependency_validate_bindings!(snapshot)
      nodes = snapshot.fetch("nodes").to_h { |node| [node.fetch("identity"), node] }
      edges = snapshot.fetch("edges").to_h { |edge| [[edge.fetch("from"), edge.fetch("to"), edge.fetch("via"), edge.fetch("columns")], true] }
      nodes.each_value do |node|
        dependency_reference_targets(node).each do |catalog, identity, via|
          target = [catalog, identity]
          raise Failure, "dependency reference target omitted: #{via} #{identity}" unless nodes.key?(target)
          raise Failure, "dependency reference edge omitted or substituted: #{via}" unless edges.key?([node.fetch("identity"), target, via, [0, 0]])
        end
      end
    end

    def dependency_validate_dispatch!(snapshot)
      nodes = snapshot.fetch("nodes")
      relation_nodes = nodes.select { |node| node.fetch("identity").first == "pg_class" && dependency_roots.map { |name| "public.#{name}" }.include?(node.fetch("identity").last) }
      unless relation_nodes.all? { |node| node.fetch("value").values_at("relkind", "relrowsecurity", "relforcerowsecurity", "relhasrules") == ["r", false, false, false] }
        raise Failure, "unexpected ingestion relation kind/RLS/rule dispatch"
      end
      cast = nodes.select { |node| node.fetch("identity").first == "pg_cast" && node.fetch("value").values_at("castsource", "casttarget") == %w[public.policy_action public.decision_type] }
      unless cast.length == 1 && cast.first.fetch("value").values_at("castfunc", "castmethod", "castcontext") == ["public.policy_action_to_decision_type(public.policy_action)", "f", "a"]
        raise Failure, "actual policy cast binding changed"
      end
      dictionary = nodes.find { |node| node.fetch("identity") == %w[pg_ts_dict public.unaccent] }
      template = nodes.find { |node| node.fetch("identity") == %w[pg_ts_template public.unaccent] }
      unless dictionary && template && dictionary.fetch("value").values_at("dicttemplate", "dictinitoption") == ["public.unaccent", "rules = 'unaccent'"] &&
             template.fetch("value").values_at("tmplinit", "tmpllexize") == ["public.unaccent_init(pg_catalog.internal)", "public.unaccent_lexize(pg_catalog.internal,pg_catalog.internal,pg_catalog.internal,pg_catalog.internal)"]
        raise Failure, "unaccent dictionary/template callback binding changed"
      end
      fks = nodes.select { |node| node.fetch("identity").first == "pg_constraint" && node.fetch("value").fetch("contype") == "f" }
      outbound = fks.select { |node| DEPENDENCY_DML.key?(node.fetch("value").fetch("conrelid").delete_prefix("public.")) }
      raise Failure, "ingestion write FK inventory changed" unless outbound.length == 31
      fks.each do |node|
        fk = node.fetch("value")
        unless %w[conpfeqop conppeqop conffeqop].all? { |field| fk.fetch(field).length == fk.fetch("conkey").length && fk.fetch(field).none?(&:nil?) }
          raise Failure, "FK equality operator binding incomplete"
        end
      end
      incoming_samples = fks.any? { |node| node.fetch("value").fetch("confrelid") == "public.canonical_size_sample" }
      raise Failure, "size-sample deletion gained parent FK reachability" if incoming_samples
      triggers = nodes.select { |node| node.fetch("identity").first == "pg_trigger" }
      write_triggers = triggers.select { |node| DEPENDENCY_DML.key?(node.fetch("value").fetch("tgrelid").delete_prefix("public.")) }
      raise Failure, "ingestion write trigger inventory changed" unless write_triggers.length == 118 && write_triggers.all? { |node| node.fetch("value").fetch("tgisinternal") }
      triggers.each do |node|
        trigger = node.fetch("value")
        next unless trigger.fetch("tgisinternal")

        fk = fks.find { |entry| entry.fetch("identity").last == trigger.fetch("tgconstraint") }
        raise Failure, "internal trigger is not a resolved FK" unless fk

        constraint = fk.fetch("value")
        child = trigger.fetch("tgrelid") == constraint.fetch("conrelid")
        function = if child
                     { 5 => '"RI_FKey_check_ins"()', 17 => '"RI_FKey_check_upd"()' }.fetch(trigger.fetch("tgtype"))
                   else
                     event, action = { 9 => ["del", "confdeltype"], 17 => ["upd", "confupdtype"] }.fetch(trigger.fetch("tgtype"))
                     name = { "a" => "noaction", "r" => "restrict", "c" => "cascade", "n" => "setnull", "d" => "setdefault" }.fetch(constraint.fetch(action))
                     "\"RI_FKey_#{name}_#{event}\"()"
                   end
        raise Failure, "unexpected FK trigger function dispatch" unless trigger.fetch("tgfoid") == "pg_catalog.#{function}" && trigger.fetch("tgenabled") == "O"
      end
    end

    # Registries belong to the producing matrices. Never reconstruct them from disk.
    def dependency_read_observation(path, registry)
      hash = registry[path] if registry.is_a?(Hash)
      unless File.expand_path(path) == path && hash.is_a?(String) && hash.match?(/\A[0-9a-f]{64}\z/)
        raise Failure, "dependency evidence lacks current-process validated bytes: #{path}"
      end

      bytes = File.open(path, File::RDONLY | File::NOFOLLOW, &:read)
      raise Failure, "dependency validated evidence bytes changed: #{path}" unless Digest::SHA256.hexdigest(bytes) == hash

      [JSON.parse(bytes), { path:, sha256: hash }]
    rescue SystemCallError, IOError, JSON::ParserError => error
      raise Failure, "cannot read dependency validated evidence: #{path}: #{error.message}"
    end

    def dependency_observation_cases(directory)
      %w[reference final].flat_map do |variant|
        role = variant == "reference" ? "postgres" : @runtime
        if directory == "ingestion-wrapper"
          wrapper_cases.flat_map do |test_case|
            IngestionWrapper::WRAPPER_MODES.map do |mode|
              ["#{test_case.fetch(:name)}-#{mode}-#{variant}", role, mode == "warm-rollback" ? 2 : 1]
            end
          end
        else
          compilation_cases.map { |test_case| ["#{test_case.fetch(:name)}-#{variant}-observed", role, test_case.fetch(:calls).length] }
        end
      end
    end

    def dependency_observation_frames(record, role, count)
      calls = record.fetch("frames")
      unless calls.is_a?(Array) && count.positive? && calls.length == count && calls.uniq.length == count
        raise Failure, "dependency callback observation frames incomplete or duplicated"
      end
      unless calls.all? { |frame| frame.fetch("state") == "00000" && frame.fetch("role").fetch("session") == role }
        raise Failure, "dependency callback observations have stale role or failed calls"
      end

      calls
    end

    def dependency_observations(snapshot)
      catalogs = dependency_canonical(snapshot, "final").fetch("nodes")
      constraints = catalogs.select { |node| node.fetch("identity").first == "pg_constraint" }.map { |node| node.fetch("value") }
      columns = catalogs.select { |node| node.fetch("identity").first == "pg_class" }.to_h do |node|
        [node.fetch("identity").last, Array(node.fetch("value")["columns"]).to_h { |column| [column.fetch(0), column.fetch(1)] }]
      end
      files = []
      frames = []
      settings = []
      registries = { "ingestion-compilation" => @compilation_validated_evidence, "ingestion-wrapper" => @wrapper_validated_evidence }
      directories = { "ingestion-compilation" => @compilation_evidence, "ingestion-wrapper" => @wrapper_evidence }
      registries.each do |directory, registry|
        current = directories.fetch(directory)
        raise Failure, "dependency evidence lacks current-process directory" unless current.is_a?(String)

        report_path = File.join(current, "report.json")
        report, evidence = dependency_read_observation(report_path, registry)
        expected = report.fetch("checks").map { |entry| entry.transform_keys(&:to_sym) }
        first = @checks.index(expected.first)
        raise Failure, "dependency observations require successful current-process #{directory}" unless report.fetch("completed") && report.fetch("passed") && !expected.empty? && expected.all? { |entry| entry.fetch(:passed) } && first && @checks.slice(first, expected.length) == expected
        raise Failure, "dependency observed source identity changed" unless report.fetch("candidate_sha256") == @contract.expected_candidate_sha256 && report.fetch("final_sha256") == @contract.final_sha256

        files << evidence
        dependency_observation_cases(directory).each do |name, role, count|
          path = File.join(current, "#{name}.json")
          record, evidence = dependency_read_observation(path, registry)
          calls = dependency_observation_frames(record, role, count)

          files << evidence
          frames.concat(calls.map { |frame| [name, frame] })
          if directory == "ingestion-compilation"
            settings.concat(record.fetch("events").map { |event| { case: name, event:, origin: "disposable setting observer; not native FK callback entry" } })
          end
        end
      end
      events = frames.flat_map { |name, frame| dependency_fk_events(name, frame, constraints, columns) }
      raise Failure, "missing changed observation FK evidence" unless events.any? { |entry| entry.fetch(:relation) == "public.search_request_source_observation" && entry.fetch(:outcome) == "changed-key update eligible for check_upd" }
      check("ingestion dependencies existing observed FK inputs and changed-key outcomes", true)
      { files:, setting_events: settings, fk_inputs: events, callback_entry_trace: false, installed_callbacks_are_execution: false }
    end

    def dependency_fk_events(name, frame, constraints, catalog_columns)
      constraints.select { |fk| fk.fetch("contype") == "f" && DEPENDENCY_DML.key?(fk.fetch("conrelid").delete_prefix("public.")) }.flat_map do |fk|
        relation = fk.fetch("conrelid")
        table = relation.delete_prefix("public.")
        before = frame.fetch("tables_before").fetch(table)
        after = frame.fetch("tables_after").fetch(table)
        primary = constraints.find { |entry| entry.fetch("conrelid") == relation && entry.fetch("contype") == "p" }
        raise Failure, "observed relation primary key missing" unless primary

        columns = catalog_columns.fetch(relation)
        key_names = primary.fetch("conkey").map { |index| columns.fetch(index) }
        fk_names = fk.fetch("conkey").map { |index| columns.fetch(index) }
        after.filter_map do |row|
          old = before.find { |entry| entry.values_at(*key_names) == row.values_at(*key_names) }
          next if old == row

          outcome = if row.values_at(*fk_names).any?(&:nil?)
                      "NULL-key input; lookup skip, not callback-entry proof"
                    elsif old.nil?
                      "non-NULL insert eligible for check_ins"
                    elsif old.values_at(*fk_names) == row.values_at(*fk_names)
                      "unchanged-key update; RI skip eligibility, not callback-entry proof"
                    else
                      "changed-key update eligible for check_upd"
                    end
          { case: name, clock: frame.fetch("clock"), relation:, constraint: fk.fetch("conname"), key_columns: fk_names,
            before: old&.values_at(*fk_names), after: row.values_at(*fk_names), outcome: }
        end
      end
    end

    def dependency_native_answers!
      values = {}
      { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (database, role)|
        query = <<~SQL
          DO $$ BEGIN NULL; END $$;
          BEGIN READ ONLY;
          SELECT json_build_object('backend',pg_backend_pid(),'role',session_user,'current',current_user,'before',current_setting('plpgsql.variable_conflict'));
          SELECT json_build_object('title',public.normalize_title_v1(U&'H\\00F4tel.1080p'),'unaccent',public.unaccent(U&'caf\\00e9'),
            'digest_text',encode(public.digest('abc'::text,'sha256'),'hex'),'digest_bytes',encode(public.digest(decode('616263','hex'),'sha256'),'hex'),
            'cast',('flag'::public.policy_action)::public.decision_type,
            'uuid_binding',(pg_identify_object('pg_proc'::regclass,'gen_random_uuid()'::regprocedure,0)).identity);
          SELECT json_build_object('after',current_setting('plpgsql.variable_conflict'));
          COMMIT;
          BEGIN READ ONLY;
          SELECT json_build_object('backend',pg_backend_pid(),'role',session_user,'current',current_user,'before',current_setting('plpgsql.variable_conflict'));
          SELECT json_build_object('title',public.normalize_title_v1(U&'H\\00F4tel.1080p'),'unaccent',public.unaccent(U&'caf\\00e9'),
            'digest_text',encode(public.digest('abc'::text,'sha256'),'hex'),'digest_bytes',encode(public.digest(decode('616263','hex'),'sha256'),'hex'),
            'cast',('flag'::public.policy_action)::public.decision_type,
            'uuid_binding',(pg_identify_object('pg_proc'::regclass,'gen_random_uuid()'::regprocedure,0)).identity);
          SELECT json_build_object('after',current_setting('plpgsql.variable_conflict'));
          COMMIT;
        SQL
        dependency_write!("#{variant}-native-answers.sql", query)
        outcome = result(query, database:, role:)
        dependency_write!("#{variant}-native-answers.stdout", outcome.stdout)
        dependency_write!("#{variant}-native-answers.stderr", outcome.stderr)
        raise Failure, "native answer transport/diagnostic failure" unless outcome.success && outcome.stderr.empty?

        records = outcome.stdout.lines.map { |line| JSON.parse(line) }
        dependency_validate_native_answers!(records, role)

        values[variant] = records
        check("ingestion dependencies #{variant} cold/warm native entries and actual cast answers", true)
      end
      values
    end

    def dependency_validate_native_answers!(records, role)
      digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
      expected = { "title" => "hotel", "unaccent" => "cafe", "digest_text" => digest, "digest_bytes" => digest, "cast" => "flag", "uuid_binding" => "pg_catalog.gen_random_uuid()" }
      valid = records.is_a?(Array) && records.length == 6 && records.all? { |record| record.is_a?(Hash) }
      valid &&= records.values_at(1, 4) == [expected, expected] && records.values_at(2, 5) == [{ "after" => "error" }] * 2
      valid &&= records.values_at(0, 3).all? { |record| record.values_at("role", "current", "before") == [role, role, "error"] }
      raise Failure, "cold/warm native known answer or caller provenance changed" unless valid

      backends = records.values_at(0, 3).map { |record| record["backend"] }
      raise Failure, "native answer backend identity must be a positive Integer" unless backends.all? { |backend| backend.is_a?(Integer) && backend.positive? }
      raise Failure, "native answers require the same backend" unless backends.uniq.length == 1
    end
  end
end
