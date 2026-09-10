# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Live, disposable D1 evidence within the final-init transition proof.
  module ExtensionProof
    private

    def extension_snapshot_query
      <<~SQL
        SET search_path TO pg_catalog;
        SELECT json_build_object(
          'extensions', (SELECT json_agg(row_to_json(r) ORDER BY r.name) FROM (
            SELECT e.extname AS name, e.extversion AS version,
              n.nspname AS schema, pg_get_userbyid(e.extowner) AS owner,
              e.extrelocatable AS relocatable, e.extconfig, e.extcondition
            FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace
          ) r),
          'members', (SELECT json_agg(row_to_json(r) ORDER BY r.extension, r.class, r.identity) FROM (
            SELECT e.extname AS extension, d.classid::regclass::text AS class,
              (pg_identify_object(d.classid, d.objid, d.objsubid)).identity AS identity,
              CASE WHEN d.classid = 'pg_ts_dict'::regclass THEN (
                SELECT json_build_object('template', m.tmplnamespace::regnamespace::text || '.' || m.tmplname,
                  'options', t.dictinitoption, 'owner', pg_get_userbyid(t.dictowner))
                FROM pg_ts_dict t JOIN pg_ts_template m ON m.oid = t.dicttemplate
                WHERE t.oid = d.objid) ELSE NULL END AS dictionary,
              CASE WHEN d.classid = 'pg_ts_template'::regclass THEN (
                SELECT json_build_object('init', t.tmplinit::regprocedure::text,
                  'lexize', t.tmpllexize::regprocedure::text)
                FROM pg_ts_template t WHERE t.oid = d.objid) ELSE NULL END AS template
            FROM pg_depend d JOIN pg_extension e ON e.oid = d.refobjid
            WHERE d.refclassid = 'pg_extension'::regclass AND d.deptype = 'e'
          ) r),
          'routines', (SELECT json_agg(row_to_json(r) ORDER BY r.extension, r.identity) FROM (
            SELECT e.extname AS extension, p.oid::regprocedure::text AS identity,
              pg_get_functiondef(p.oid) AS definition, pg_get_userbyid(p.proowner) AS owner,
              p.prosecdef AS security_definer,
              'internal'::regtype = ANY(p.proargtypes::oid[]) AS internal_callback,
              has_function_privilege(#{literal(@runtime)}, p.oid, 'EXECUTE') AS runtime_execute,
              (SELECT json_agg(row_to_json(a) ORDER BY a.grantor, a.grantee, a.privilege_type) FROM (
                SELECT pg_get_userbyid(acl.grantor) AS grantor,
                  CASE WHEN acl.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(acl.grantee) END AS grantee,
                  acl.privilege_type, acl.is_grantable
                FROM aclexplode(COALESCE(p.proacl, acldefault('f', p.proowner))) acl
              ) a) AS acl
            FROM pg_proc p JOIN pg_depend d ON d.classid = 'pg_proc'::regclass AND d.objid = p.oid
              AND d.refclassid = 'pg_extension'::regclass AND d.deptype = 'e'
            JOIN pg_extension e ON e.oid = d.refobjid
            WHERE e.extname IN ('pgcrypto', 'unaccent')
          ) r)
        );
      SQL
    end

    def extension_snapshot(database: @database, mutation: nil)
      query = extension_snapshot_query
      query = "BEGIN; #{mutation};\n#{query}\nROLLBACK;" if mutation
      JSON.parse(sql(query, database:, role: mutation ? 'postgres' : @owner))
    end

    def verify_extension_boundary!
      sql("CREATE EXTENSION pgcrypto WITH SCHEMA public; CREATE EXTENSION unaccent WITH SCHEMA public;", database: "extension_proof")
      @stock_extensions = extension_snapshot(database: "extension_proof")
      observed = extension_snapshot
      %w[stock observed].zip([@stock_extensions, observed]).each do |name, snapshot|
        File.write(File.join(@contract.output_path, "final-#{name}-extensions.json"), JSON.pretty_generate(snapshot) + "\n")
      end
      check("exact pinned stock extension definitions ownership membership and ACLs", observed == @stock_extensions)
      routines = @stock_extensions.fetch("routines")
      check("stock extension routine and callback inventory", routines.length == 40 && routines.count { |row| row.fetch("internal_callback") } == 2)
      check("stock extensions remain superuser-owned SECURITY INVOKER primitives", routines.all? do |row|
        row.fetch("owner") == "postgres" && !row.fetch("security_definer") && row.fetch("runtime_execute")
      end)
      mutations = {
        "additional extension" => "CREATE EXTENSION hstore",
        "changed extension version" => "UPDATE pg_extension SET extversion = 'unapproved' WHERE extname = 'pgcrypto'",
        "removed extension membership" => "ALTER EXTENSION pgcrypto DROP FUNCTION public.digest(text, text)",
        "additional extension membership" => "CREATE FUNCTION public.extra_extension_member() RETURNS integer LANGUAGE SQL AS 'SELECT 1'; ALTER EXTENSION pgcrypto ADD FUNCTION public.extra_extension_member()",
        "changed extension definition" => "ALTER FUNCTION public.digest(text, text) COST 200",
        "changed dictionary definition" => "UPDATE pg_ts_dict SET dictinitoption = 'rules = ''proof-missing-rules''' WHERE oid = 'public.unaccent'::regdictionary",
        "changed dictionary template" => "UPDATE pg_ts_template SET tmplinit = 0 WHERE oid = (SELECT dicttemplate FROM pg_ts_dict WHERE oid = 'public.unaccent'::regdictionary)",
        "changed extension owner" => "ALTER FUNCTION public.digest(text, text) OWNER TO #{identifier(@owner)}",
        "changed extension ACL" => "GRANT EXECUTE ON FUNCTION public.digest(text, text) TO #{identifier(@runtime)}",
        "removed inherited extension execution" => "REVOKE EXECUTE ON FUNCTION public.digest(text, text) FROM PUBLIC",
        "SECURITY DEFINER extension member" => "ALTER FUNCTION public.digest(text, text) SECURITY DEFINER"
      }
      mutations.each do |name, mutation|
        check("D1 rejects #{name}", extension_snapshot(mutation:) != @stock_extensions)
        check("D1 #{name} rolls back exactly", extension_snapshot == @stock_extensions)
      end
      verify_runtime_extension_primitives!
    end

    def verify_runtime_extension_primitives!
      check("runtime stock digest primitive", sql("SELECT encode(public.digest('abc', 'sha256'), 'hex')", role: @runtime) == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
      check("runtime stock unaccent primitive", sql("SELECT public.unaccent(U&'caf\\00e9')", role: @runtime) == "cafe")
    end
  end
end
