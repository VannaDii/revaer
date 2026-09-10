# frozen_string_literal: true

require_relative "columns"

module RevaerPostgresPristine
  class Query
    SCALAR_TYPES = %w[name text bool int2 int4 float4 char bytea _text _char pg_lsn].freeze

    def inventory_sql
      <<~SQL
        SELECT json_build_object('kind', 'columns', 'catalog', c.relname, 'columns',
          json_agg(json_build_array(a.attname, y.typname) ORDER BY a.attnum))
        FROM pg_attribute a JOIN pg_class c ON c.oid = a.attrelid JOIN pg_type y ON y.oid = a.atttypid
        WHERE c.relnamespace = 'pg_catalog'::regnamespace AND a.attnum > 0 AND NOT a.attisdropped
        AND c.relname IN (#{COLUMNS.keys.map { |name| "'#{name}'" }.join(', ')}) GROUP BY c.relname;
      SQL
    end

    # Zero memberships is a fixture provisioning invariant, not ADR 551 runtime admission.
    # The approved runtime membership check concerns the runtime role's membership in the owner.
    def identity_sql
      <<~SQL
        SELECT json_build_object('kind', 'identity', 'database', '<database>', 'owner', '<database_owner>',
          'server_version_num', current_setting('server_version_num'), 'encoding', current_setting('server_encoding'),
          'collate', d.datcollate, 'ctype', d.datctype, 'integer_datetimes', current_setting('integer_datetimes'),
          'standard_conforming_strings', current_setting('standard_conforming_strings'),
          'data_checksums', current_setting('data_checksums'),
          'owner_valid', d.datdba = r.oid AND session_user = current_user AND r.rolcanlogin AND NOT r.rolsuper
            AND NOT r.rolcreatedb AND NOT r.rolcreaterole AND NOT r.rolreplication AND NOT r.rolbypassrls
            AND NOT EXISTS (SELECT 1 FROM pg_auth_members WHERE member = r.oid))
        FROM pg_database d JOIN pg_roles r ON r.rolname = session_user WHERE d.datname = current_database();
      SQL
    end

    def projection(catalog, columns)
      raise Failure, "catalog column inventory drift" unless columns.map(&:first) == COLUMNS.fetch(catalog)

      columns.filter_map do |name, type|
        next if name == "oid" || EXCLUDED.key?(name)

        "#{expression(name, type)} AS #{name}"
      end
    end

    def expression(name, type)
      return relation_name("t.oid") if name == "relname"
      return EXPRESSIONS.fetch(name) if type == "pg_node_tree"
      return acl("t.#{name}") if type == "_aclitem"
      return "t.#{name}::smallint[]" if type == "int2vector"
      if %w[oid regproc _oid oidvector].include?(type)
        catalog = REFERENCES.fetch(name)
        return reference(catalog, "t.#{name}::oid") if %w[oid regproc].include?(type)

        return "CASE WHEN t.#{name} IS NOT NULL THEN ARRAY(SELECT #{reference(catalog, 'v')} " \
               "FROM unnest(t.#{name}::oid[]) WITH ORDINALITY a(v, n) ORDER BY n) END"
      end
      raise Failure, "unaccounted catalog column type" unless SCALAR_TYPES.include?(type)

      "t.#{name}"
    end

    def reference(catalog, oid)
      value = case catalog
              when "pg_authid"
                "(SELECT CASE WHEN rolname = session_user THEN '<database_owner>' ELSE rolname::text END " \
                  "FROM pg_roles WHERE oid = #{oid})"
              when "pg_database"
                "(SELECT CASE WHEN datname = current_database() THEN '<database>' ELSE datname::text END " \
                  "FROM pg_database WHERE oid = #{oid})"
              when "pg_class" then relation_name(oid)
              else "(SELECT identity FROM pg_identify_object('pg_catalog.#{catalog}'::regclass, #{oid}, 0))"
              end
      zero = catalog == "pg_authid" ? "'PUBLIC'" : "NULL"
      "CASE WHEN #{oid} = 0 THEN #{zero} ELSE COALESCE(#{value}, '<unresolved_reference>') END"
    end

    def relation_name(oid)
      <<~SQL.strip
        (SELECT CASE WHEN n.nspname = 'pg_toast' THEN
          COALESCE((SELECT 'TOAST OF ' || format('%I.%I', pn.nspname, p.relname)
            FROM pg_class p JOIN pg_namespace pn ON pn.oid = p.relnamespace WHERE p.reltoastrelid = c.oid),
            (SELECT 'TOAST INDEX OF ' || format('%I.%I', pn.nspname, p.relname)
             FROM pg_index i JOIN pg_class p ON p.reltoastrelid = i.indrelid
             JOIN pg_namespace pn ON pn.oid = p.relnamespace WHERE i.indexrelid = c.oid))
          ELSE format('%I.%I', n.nspname, c.relname) END
         FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.oid = #{oid})
      SQL
    end

    def acl(value)
      <<~SQL.strip
        CASE WHEN #{value} IS NOT NULL THEN (SELECT COALESCE(json_agg(entry ORDER BY entry::text COLLATE "C"), '[]'::json)
          FROM (SELECT json_build_array(#{reference('pg_authid', 'grantor')},
            #{reference('pg_authid', 'grantee')}, privilege_type, is_grantable) AS entry
            FROM aclexplode(#{value})) privileges) ELSE NULL END
      SQL
    end

    def condition(catalog)
      namespace = COLUMNS.fetch(catalog).find { |name| name.end_with?("namespace") }
      namespace = "oid" if catalog == "pg_namespace"
      return "t.#{namespace} NOT IN (SELECT oid FROM pg_namespace WHERE nspname ~ '^pg_(toast_)?temp_[0-9]+$')" if namespace

      relation = { "pg_trigger" => "tgrelid", "pg_policy" => "polrelid", "pg_publication_rel" => "prrelid" }[catalog]
      return "t.#{relation} IN (SELECT oid FROM pg_class WHERE relpersistence <> 't')" if relation

      "true"
    end

    def catalog_sql(catalog, columns)
      fields = projection(catalog, columns)
      extra = extras(catalog)
      fields.concat(extra)
      <<~SQL
        SELECT json_build_object('kind', 'rows', 'catalog', '#{catalog}',
          'rows', COALESCE(json_agg(row_to_json(s)), '[]'::json))
        FROM (SELECT #{fields.join(",\n")} FROM pg_catalog.#{catalog} t WHERE #{condition(catalog)}) s;
      SQL
    end

    def extras(catalog)
      case catalog
      when "pg_class"
        ["CASE WHEN t.relkind IN ('v', 'm') THEN pg_get_viewdef(t.oid, false) END AS view_definition",
         "(SELECT json_build_array(i.indnatts, i.indnkeyatts, i.indisunique, i.indnullsnotdistinct, i.indisprimary, " \
           "i.indisexclusion, i.indimmediate, i.indisclustered, i.indisvalid, i.indcheckxmin, i.indisready, " \
           "i.indislive, i.indisreplident, #{reference('pg_class', 'i.indrelid')}, " \
           "ARRAY(SELECT pg_get_indexdef(i.indexrelid, k, false) FROM generate_series(1, i.indnatts) k), " \
           "ARRAY(SELECT #{reference('pg_opclass', 'v')} FROM unnest(i.indclass::oid[]) WITH ORDINALITY a(v, n) ORDER BY n), " \
           "ARRAY(SELECT #{reference('pg_collation', 'v')} FROM unnest(i.indcollation::oid[]) WITH ORDINALITY a(v, n) ORDER BY n), " \
           "i.indoption::smallint[], pg_get_expr(i.indpred, i.indrelid, false)) " \
           "FROM pg_index i WHERE i.indexrelid = t.oid) AS index_definition",
         "CASE WHEN t.relkind = 'p' THEN pg_get_partkeydef(t.oid) END AS partition_key",
         "(SELECT json_agg(json_build_array(a.attnum, a.attname, #{reference('pg_type', 'a.atttypid')}, " \
           "a.atttypmod, a.attnotnull, a.attidentity, a.attgenerated, a.attisdropped, " \
           "a.attinhcount, a.attislocal, a.attstorage, a.attcompression, #{reference('pg_collation', 'a.attcollation')}, " \
           "#{acl('a.attacl')}, pg_get_expr(d.adbin, d.adrelid, false)) ORDER BY a.attnum) " \
           "FROM pg_attribute a LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum " \
           "WHERE a.attrelid = t.oid AND a.attnum > 0) AS columns",
         "(SELECT json_agg(json_build_array(conname, convalidated, pg_get_constraintdef(oid, false)) " \
           "ORDER BY conname COLLATE \"C\") FROM pg_constraint WHERE conrelid = t.oid) AS constraints",
         "(SELECT json_agg(json_build_array(rulename, ev_enabled, pg_get_ruledef(oid, false)) " \
           "ORDER BY rulename COLLATE \"C\") FROM pg_rewrite WHERE ev_class = t.oid) AS rules"]
      when "pg_type"
        ["(SELECT json_agg(enumlabel ORDER BY enumsortorder) FROM pg_enum WHERE enumtypid = t.oid) AS enum_labels",
         "(SELECT json_agg(json_build_array(conname, convalidated, pg_get_constraintdef(oid, false)) " \
           "ORDER BY conname COLLATE \"C\") FROM pg_constraint WHERE contypid = t.oid) AS constraints"]
      when "pg_ts_config"
        ["(SELECT json_agg(json_build_array(maptokentype, mapseqno, " \
           "#{reference('pg_ts_dict', 'mapdict')}) ORDER BY maptokentype, mapseqno) " \
           "FROM pg_ts_config_map WHERE mapcfg = t.oid) AS mappings"]
      else []
      end
    end
  end
end
