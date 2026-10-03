"""Construct the existing catalog projections with explicit OID/ACL normalization."""

from textwrap import dedent

from ...errors import ToolingError
from .spec import Specification

SCALAR_TYPES = frozenset(
    ("name", "text", "bool", "int2", "int4", "float4", "char", "bytea", "_text", "_char", "pg_lsn")
)


def literal(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


class Query:
    def __init__(self, spec: Specification) -> None:
        self.spec = spec

    def inventory(self) -> str:
        catalogs = ", ".join(map(literal, self.spec.columns))
        return dedent(f"""
            SELECT json_build_object('kind', 'columns', 'catalog', c.relname, 'columns',
                json_agg(json_build_array(a.attname, y.typname) ORDER BY a.attnum))
            FROM pg_attribute a JOIN pg_class c ON c.oid = a.attrelid
                JOIN pg_type y ON y.oid = a.atttypid
            WHERE c.relnamespace = 'pg_catalog'::regnamespace AND a.attnum > 0
                AND NOT a.attisdropped AND c.relname IN ({catalogs}) GROUP BY c.relname;
        """)

    @staticmethod
    def identity() -> str:
        # Zero membership is a fixture-provisioning invariant. It does not
        # redefine the separate runtime role admission policy in the initializer.
        return dedent("""
            SELECT json_build_object('kind', 'identity', 'database', '<database>',
              'owner', '<database_owner>',
              'server_version_num', current_setting('server_version_num'),
              'encoding', current_setting('server_encoding'), 'collate', d.datcollate,
              'ctype', d.datctype, 'integer_datetimes', current_setting('integer_datetimes'),
              'standard_conforming_strings', current_setting('standard_conforming_strings'),
              'data_checksums', current_setting('data_checksums'),
              'owner_valid', d.datdba = r.oid AND session_user = current_user AND r.rolcanlogin
                AND NOT r.rolsuper AND NOT r.rolcreatedb AND NOT r.rolcreaterole
                AND NOT r.rolreplication AND NOT r.rolbypassrls
                AND NOT EXISTS (SELECT 1 FROM pg_auth_members WHERE member = r.oid))
            FROM pg_database d JOIN pg_roles r ON r.rolname = session_user
            WHERE d.datname = current_database();
        """)

    def projection(self, catalog: str, columns: tuple[tuple[str, str], ...]) -> list[str]:
        if tuple(column for column, _ in columns) != self.spec.columns.get(catalog):
            raise ToolingError("Catalog column inventory drift")
        return [
            f"{self.expression(column, kind)} AS {column}"
            for column, kind in columns
            if column != "oid" and column not in self.spec.excluded
        ]

    def expression(self, column: str, kind: str) -> str:
        if column == "relname":
            return self.relation_name("t.oid")
        if kind == "pg_node_tree":
            try:
                return self.spec.expressions[column]
            except KeyError as error:
                raise ToolingError("Unaccounted catalog expression column") from error
        if kind == "_aclitem":
            return self.acl("t." + column)
        if kind == "int2vector":
            return f"t.{column}::smallint[]"
        if kind in ("oid", "regproc", "_oid", "oidvector"):
            try:
                catalog = self.spec.references[column]
            except KeyError as error:
                raise ToolingError("Unaccounted catalog reference column") from error
            if kind in ("oid", "regproc"):
                return self.reference(catalog, f"t.{column}::oid")
            return (
                f"CASE WHEN t.{column} IS NOT NULL THEN ARRAY(SELECT "
                f"{self.reference(catalog, 'v')} "
                f"FROM unnest(t.{column}::oid[]) WITH ORDINALITY a(v, n) ORDER BY n) END"
            )
        if kind not in SCALAR_TYPES:
            raise ToolingError("Unaccounted catalog column type")
        return f"t.{column}"

    def reference(self, catalog: str, oid: str) -> str:
        if catalog == "pg_authid":
            value = (
                "(SELECT CASE WHEN rolname = session_user THEN '<database_owner>' "
                f"ELSE rolname::text END FROM pg_roles WHERE oid = {oid})"
            )
        elif catalog == "pg_database":
            value = (
                "(SELECT CASE WHEN datname = current_database() THEN '<database>' "
                f"ELSE datname::text END FROM pg_database WHERE oid = {oid})"
            )
        elif catalog == "pg_class":
            value = self.relation_name(oid)
        else:
            value = (
                f"(SELECT identity FROM pg_identify_object('pg_catalog.{catalog}'::regclass, "
                f"{oid}, 0))"
            )
        zero = "'PUBLIC'" if catalog == "pg_authid" else "NULL"
        return (
            f"CASE WHEN {oid} = 0 THEN {zero} ELSE COALESCE({value}, '<unresolved_reference>') END"
        )

    @staticmethod
    def relation_name(oid: str) -> str:
        return dedent(f"""
            (SELECT CASE WHEN n.nspname = 'pg_toast' THEN
              COALESCE((SELECT 'TOAST OF ' || format('%I.%I', pn.nspname, p.relname)
                FROM pg_class p JOIN pg_namespace pn ON pn.oid = p.relnamespace
                WHERE p.reltoastrelid = c.oid),
                (SELECT 'TOAST INDEX OF ' || format('%I.%I', pn.nspname, p.relname)
                 FROM pg_index i JOIN pg_class p ON p.reltoastrelid = i.indrelid
                 JOIN pg_namespace pn ON pn.oid = p.relnamespace WHERE i.indexrelid = c.oid))
              ELSE format('%I.%I', n.nspname, c.relname) END
             FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.oid = {oid})
        """).strip()

    def acl(self, value: str) -> str:
        return dedent(f"""
            CASE WHEN {value} IS NOT NULL THEN
              (SELECT COALESCE(json_agg(entry ORDER BY entry::text COLLATE "C"), '[]'::json)
               FROM (SELECT json_build_array({self.reference("pg_authid", "grantor")},
                 {self.reference("pg_authid", "grantee")}, privilege_type, is_grantable) AS entry
                 FROM aclexplode({value})) privileges) ELSE NULL END
        """).strip()

    def condition(self, catalog: str) -> str:
        namespace = next(
            (column for column in self.spec.columns[catalog] if column.endswith("namespace")), None
        )
        if catalog == "pg_namespace":
            namespace = "oid"
        if namespace:
            return (
                f"t.{namespace} NOT IN (SELECT oid FROM pg_namespace "
                "WHERE nspname ~ '^pg_(toast_)?temp_[0-9]+$')"
            )
        relation = {
            "pg_trigger": "tgrelid",
            "pg_policy": "polrelid",
            "pg_publication_rel": "prrelid",
        }.get(catalog)
        return (
            f"t.{relation} IN (SELECT oid FROM pg_class WHERE relpersistence <> 't')"
            if relation
            else "true"
        )

    def catalog(self, catalog: str, columns: tuple[tuple[str, str], ...]) -> str:
        projection = ",\n".join(self.projection(catalog, columns) + self.extras(catalog))
        return (
            f"SELECT json_build_object('kind', 'rows', 'catalog', '{catalog}', "
            "'rows', COALESCE(json_agg(row_to_json(s)), '[]'::json)) "
            f"FROM (SELECT {projection} FROM pg_catalog.{catalog} t "
            f"WHERE {self.condition(catalog)}) s;"
        )

    def extras(self, catalog: str) -> list[str]:
        if catalog == "pg_class":
            return self.relation_extras()
        if catalog == "pg_type":
            return [
                "(SELECT json_agg(enumlabel ORDER BY enumsortorder) FROM pg_enum "
                "WHERE enumtypid = t.oid) AS enum_labels",
                "(SELECT json_agg(json_build_array(conname, convalidated, "
                'pg_get_constraintdef(oid, false)) ORDER BY conname COLLATE "C") '
                "FROM pg_constraint WHERE contypid = t.oid) AS constraints",
            ]
        if catalog == "pg_ts_config":
            return [
                "(SELECT json_agg(json_build_array(maptokentype, mapseqno, "
                f"{self.reference('pg_ts_dict', 'mapdict')}) ORDER BY maptokentype, mapseqno) "
                "FROM pg_ts_config_map WHERE mapcfg = t.oid) AS mappings"
            ]
        return []

    def relation_extras(self) -> list[str]:
        return [
            "CASE WHEN t.relkind IN ('v', 'm') THEN pg_get_viewdef(t.oid, false) "
            "END AS view_definition",
            dedent(f"""
                (SELECT json_build_array(i.indnatts, i.indnkeyatts, i.indisunique,
                  i.indnullsnotdistinct, i.indisprimary, i.indisexclusion, i.indimmediate,
                  i.indisclustered, i.indisvalid, i.indcheckxmin, i.indisready, i.indislive,
                  i.indisreplident, {self.reference("pg_class", "i.indrelid")},
                  ARRAY(SELECT pg_get_indexdef(i.indexrelid, k, false)
                    FROM generate_series(1, i.indnatts) k),
                  ARRAY(SELECT {self.reference("pg_opclass", "v")}
                    FROM unnest(i.indclass::oid[]) WITH ORDINALITY a(v, n) ORDER BY n),
                  ARRAY(SELECT {self.reference("pg_collation", "v")}
                    FROM unnest(i.indcollation::oid[]) WITH ORDINALITY a(v, n) ORDER BY n),
                  i.indoption::smallint[], pg_get_expr(i.indpred, i.indrelid, false))
                 FROM pg_index i WHERE i.indexrelid = t.oid) AS index_definition
            """).strip(),
            "CASE WHEN t.relkind = 'p' THEN pg_get_partkeydef(t.oid) END AS partition_key",
            dedent(f"""
                (SELECT json_agg(json_build_array(a.attnum, a.attname,
                  {self.reference("pg_type", "a.atttypid")}, a.atttypmod, a.attnotnull,
                  a.attidentity, a.attgenerated, a.attisdropped, a.attinhcount, a.attislocal,
                  a.attstorage, a.attcompression,
                  {self.reference("pg_collation", "a.attcollation")},
                  {self.acl("a.attacl")}, pg_get_expr(d.adbin, d.adrelid, false)) ORDER BY a.attnum)
                 FROM pg_attribute a LEFT JOIN pg_attrdef d
                   ON d.adrelid = a.attrelid AND d.adnum = a.attnum
                 WHERE a.attrelid = t.oid AND a.attnum > 0) AS columns
            """).strip(),
            "(SELECT json_agg(json_build_array(conname, convalidated, "
            'pg_get_constraintdef(oid, false)) ORDER BY conname COLLATE "C") '
            "FROM pg_constraint WHERE conrelid = t.oid) AS constraints",
            "(SELECT json_agg(json_build_array(rulename, ev_enabled, pg_get_ruledef(oid, false)) "
            'ORDER BY rulename COLLATE "C") FROM pg_rewrite WHERE ev_class = t.oid) AS rules',
        ]
