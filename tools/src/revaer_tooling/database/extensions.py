"""Prove stock extension identity and privileges inside the owned final database.

The caller owns container lifetime and aggregates labeled checks. This stage
never creates a server or selects a connection from the environment.
"""

import json
from collections.abc import Callable

from ..external.postgres import identifier
from ..json_data import JsonObject, array_value, decode_unique, object_value
from .postgres import Connection

# The complete catalog snapshot also catches membership, dictionary and ACL drift.
SNAPSHOT = """SET search_path TO pg_catalog;
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
        SELECT json_build_object('template',
          m.tmplnamespace::regnamespace::text || '.' || m.tmplname,
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
      has_function_privilege('{runtime}', p.oid, 'EXECUTE') AS runtime_execute,
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
"""


class ExtensionProof:
    def __init__(
        self,
        connection: Connection,
        owner: str,
        runtime: str,
        check: Callable[[str, bool], None],
    ) -> None:
        self.connection = connection
        self.owner, self.runtime = identifier(owner), identifier(runtime)
        self.check = check

    def snapshot(self, *, database: str | None = None, mutation: str | None = None) -> JsonObject:
        query = SNAPSHOT.format(runtime=self.runtime)
        # psql closes this connection on both success and error. A failed query
        # rolls back the mutation even when the final ROLLBACK was not reached.
        if mutation is not None:
            query = f"BEGIN; {mutation};\n{query}\nROLLBACK;"
        return object_value(
            decode_unique(
                self.connection.sql(
                    query,
                    database=database,
                    role="postgres" if mutation is not None else self.owner,
                )
            )
        )

    def verify(self, reference_database: str = "extension_proof") -> None:
        self.connection.sql(
            "CREATE EXTENSION pgcrypto WITH SCHEMA public; "
            "CREATE EXTENSION unaccent WITH SCHEMA public;",
            database=reference_database,
            role=self.owner,
        )
        stock = self.snapshot(database=reference_database)
        observed = self.snapshot()
        for name, snapshot in (("stock", stock), ("observed", observed)):
            self.connection.fs.write(
                self.connection.output / f"final-{name}-extensions.json",
                json.dumps(snapshot, indent=2) + "\n",
                0o600,
            )
        self.check(
            "exact pinned stock extension definitions ownership membership and ACLs",
            observed == stock,
        )
        routines = [object_value(row) for row in array_value(stock.get("routines"))]
        self.check(
            "stock extension routine and callback inventory",
            len(routines) == 40
            and sum(row.get("internal_callback") is True for row in routines) == 2,
        )
        self.check(
            "stock extensions remain superuser-owned SECURITY INVOKER primitives",
            bool(routines)
            and all(
                row.get("owner") == "postgres"
                and row.get("security_definer") is False
                and row.get("runtime_execute") is True
                for row in routines
            ),
        )
        for name, mutation in self.mutations().items():
            self.check(f"D1 rejects {name}", self.snapshot(mutation=mutation) != stock)
            self.check(f"D1 {name} rolls back exactly", self.snapshot() == stock)
        self.verify_runtime_primitives()

    def mutations(self) -> dict[str, str]:
        """The original eleven independent catalog/permission rejection cases."""
        return {
            "additional extension": ("CREATE EXTENSION hstore"),
            "changed extension version": (
                "UPDATE pg_extension SET extversion = 'unapproved' WHERE extname = 'pgcrypto'"
            ),
            "removed extension membership": (
                "ALTER EXTENSION pgcrypto DROP FUNCTION public.digest(text, text)"
            ),
            "additional extension membership": (
                "CREATE FUNCTION public.extra_extension_member() RETURNS integer "
                "LANGUAGE SQL AS 'SELECT 1'; "
                "ALTER EXTENSION pgcrypto ADD FUNCTION public.extra_extension_member()"
            ),
            "changed extension definition": ("ALTER FUNCTION public.digest(text, text) COST 200"),
            "changed dictionary definition": (
                "UPDATE pg_ts_dict SET dictinitoption = 'rules = ''proof-missing-rules''' "
                "WHERE oid = 'public.unaccent'::regdictionary"
            ),
            "changed dictionary template": (
                "UPDATE pg_ts_template SET tmplinit = 0 WHERE oid = "
                "(SELECT dicttemplate FROM pg_ts_dict WHERE oid = 'public.unaccent'::regdictionary)"
            ),
            "changed extension owner": (
                f"ALTER FUNCTION public.digest(text, text) OWNER TO {self.owner}"
            ),
            "changed extension ACL": (
                f"GRANT EXECUTE ON FUNCTION public.digest(text, text) TO {self.runtime}"
            ),
            "removed inherited extension execution": (
                "REVOKE EXECUTE ON FUNCTION public.digest(text, text) FROM PUBLIC"
            ),
            "SECURITY DEFINER extension member": (
                "ALTER FUNCTION public.digest(text, text) SECURITY DEFINER"
            ),
        }

    def verify_runtime_primitives(self) -> None:
        self.check(
            "runtime stock digest primitive",
            self.connection.sql(
                "SELECT encode(public.digest('abc', 'sha256'), 'hex')", role=self.runtime
            )
            == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        )
        self.check(
            "runtime stock unaccent primitive",
            self.connection.sql("SELECT public.unaccent(U&'caf\\00e9')", role=self.runtime)
            == "cafe",
        )
