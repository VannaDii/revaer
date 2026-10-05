"""Sealing, rejection and transaction checks for the disposable final baseline.

The owning proof creates the databases and principals and verifies initializer
bytes before invoking these stages. No method adopts an operator connection or
changes the reviewed initializer. Every expected failure checks its SQLSTATE.
"""

import json
import re
from collections.abc import Callable

from ..errors import ToolingError
from ..external.postgres import QueryArgs, identifier
from ..json_data import array_value, decode_unique, object_value
from ..process import Completed
from .final_sql import Routine
from .postgres import Connection

BASELINE = "revaer_system.database_baseline"
READ = "revaer_system.read_database_baseline_v1()"


class BaselineProof:
    def __init__(
        self,
        connection: Connection,
        owner: str,
        runtime: str,
        digest: str,
        check: Callable[[str, bool], None],
    ) -> None:
        if not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ToolingError("Baseline proof requires the exact initializer SHA-256")
        self.connection = connection
        self.owner, self.runtime = identifier(owner), identifier(runtime)
        self.digest, self.check = digest, check

    def sql(self, source: str, *, role: str | None = None, database: str | None = None) -> str:
        return self.connection.sql(source, role=role or self.owner, database=database)

    def result(
        self, source: str, *, role: str | None = None, database: str | None = None
    ) -> Completed:
        # Verbose output preserves both SQLSTATE and DETAIL for negative proofs.
        return self.connection.docker.query(
            self.connection.container,
            QueryArgs(
                role or self.owner,
                database or self.connection.database,
                "\\set VERBOSITY verbose\n" + source,
            ),
        )

    def denied(
        self,
        label: str,
        source: str,
        *,
        role: str | None = None,
        state: str = "42501",
        detail: str | None = None,
    ) -> None:
        outcome = self.result(source, role=role or self.runtime)
        # Match the actual error field: a code quoted in SQL or diagnostic text
        # must not accidentally satisfy an expected denial.
        self.check(
            label,
            outcome.code != 0
            and re.search(r"^ERROR:[ \t]+" + re.escape(state) + r":", outcome.stderr, re.M)
            is not None
            and (
                detail is None
                or re.search(r"^DETAIL:[ \t]+" + re.escape(detail) + r"$", outcome.stderr, re.M)
                is not None
            )
            and re.search(r"\bWARNING\b", outcome.stderr) is None,
        )

    def seal_query(
        self, *, version: str = "1", digest: str | None = None, runtime: str | None = None
    ) -> str:
        # Overrides are SQL expressions belonging only to the fixed invalid-input
        # matrix below. Public CLI input is never substituted into these slots.
        digest = digest if digest is not None else f"decode('{self.digest}', 'hex')"
        runtime = runtime if runtime is not None else f"'{self.runtime}'"
        return (
            "SELECT * FROM revaer_system.seal_database_baseline_v1("
            f"{version}::smallint, {digest}, {runtime});"
        )

    def unsealed(self) -> None:
        self.denied(
            "empty baseline is invalid",
            f"SELECT * FROM {READ}",
            role=self.owner,
            state="P0001",
            detail="baseline_shape_invalid",
        )
        self.denied("runtime cannot seal before grants", self.seal_query())

    def seal_inputs(self) -> None:
        for version in ("NULL", "-1", "0", "2", "32767"):
            self.denied(
                f"reject contract version {version}",
                self.seal_query(version=version),
                role=self.owner,
                state="P0001",
            )
        for index, digest in enumerate(
            (
                "NULL",
                "decode('', 'hex')",
                "decode(repeat('aa', 31), 'hex')",
                "decode(repeat('aa', 33), 'hex')",
            )
        ):
            self.denied(
                f"reject invalid digest {index}",
                self.seal_query(digest=digest),
                role=self.owner,
                state="P0001",
            )
        for index, runtime in enumerate(
            ("NULL", "''", "repeat('x', 64)", "'absent'", f"'{self.owner}'")
        ):
            self.denied(
                f"reject invalid runtime {index}",
                self.seal_query(runtime=runtime),
                role=self.owner,
                state="P0001",
            )
        self.check("failed seals leave no row", self.sql(f"SELECT count(*) FROM {BASELINE}") == "0")

    def seal(self) -> None:
        self.sql(f"BEGIN; {self.seal_query()} COMMIT;")
        self.denied("reject reseal", self.seal_query(), role=self.owner, state="P0001")

    def baseline(self) -> None:
        shape = self.sql(
            "SELECT string_agg(a.attname || ':' || format_type(a.atttypid, a.atttypmod) "
            "|| ':' || a.attnotnull::text, ',' ORDER BY a.attnum) FROM pg_attribute a "
            f"WHERE a.attrelid = '{BASELINE}'::regclass AND a.attnum > 0 AND NOT a.attisdropped"
        )
        self.check(
            "exact baseline column shape",
            shape
            == "baseline_id:smallint:true,contract_version:smallint:true,init_sha256:bytea:true,"
            "postgres_version_num:integer:true,schema_owner_role:name:true,runtime_role:name:true,"
            "sealed_at:timestamp with time zone:true",
        )
        self.check(
            "exact baseline constraint count",
            self.sql(f"SELECT count(*) FROM pg_constraint WHERE conrelid = '{BASELINE}'::regclass")
            == "8",
        )
        self.check(
            "exact sealed digest and principals",
            self.sql(
                f"SELECT contract_version = 1 AND encode(init_sha256, 'hex') = '{self.digest}' "
                f"AND postgres_version_num = 160014 AND schema_owner_role = '{self.owner}' "
                f"AND runtime_role = '{self.runtime}' FROM {READ}",
                role=self.runtime,
            )
            == "t",
        )
        self.check(
            "no migration metadata",
            self.sql("SELECT to_regclass('public._sqlx_migrations') IS NULL") == "t",
        )

    def read_failures(self) -> None:
        query = f"SELECT row_to_json(b)::text FROM {BASELINE} b"
        original = self.sql(query)
        mutations = (
            f"DELETE FROM {BASELINE};",
            f"DROP TABLE {BASELINE};",
            f"ALTER TABLE {BASELINE} DROP COLUMN contract_version;",
            f"ALTER TABLE {BASELINE} DROP CONSTRAINT database_baseline_runtime_nonempty; "
            f"UPDATE {BASELINE} SET runtime_role = '';",
            f"ALTER TABLE {BASELINE} DROP CONSTRAINT database_baseline_contract_v1; "
            f"UPDATE {BASELINE} SET contract_version = 2;",
            f"ALTER TABLE {BASELINE} DROP CONSTRAINT database_baseline_sha256_length; "
            f"UPDATE {BASELINE} SET init_sha256 = decode('aa', 'hex');",
            f"ALTER TABLE {BASELINE} DROP CONSTRAINT database_baseline_pkey, "
            f"DROP CONSTRAINT database_baseline_singleton; INSERT INTO {BASELINE} "
            "SELECT 2, contract_version, init_sha256, postgres_version_num, "
            f"schema_owner_role, runtime_role, sealed_at FROM {BASELINE};",
        )
        for index, mutation in enumerate(mutations):
            self.denied(
                f"malformed baseline read {index}",
                f"BEGIN; {mutation} SELECT * FROM {READ};",
                role=self.owner,
                state="P0001",
                detail="baseline_shape_invalid",
            )
        self.check("read failure fixtures roll back exact baseline", self.sql(query) == original)

    def permissions(self, outsider: str) -> None:
        outsider = identifier(outsider)
        for label, query in (
            ("runtime seal denied", self.seal_query()),
            ("runtime DDL denied", "CREATE TABLE public.forbidden (id integer)"),
            ("runtime temp denied", "CREATE TEMP TABLE forbidden (id integer)"),
            ("runtime baseline DML denied", f"DELETE FROM {BASELINE}"),
            ("runtime application table DML denied", "DELETE FROM public.app_profile"),
            ("runtime table read denied", "SELECT * FROM public.app_profile"),
            ("runtime sequence denied", "SELECT nextval('public.app_user_user_id_seq')"),
            ("runtime extension DDL denied", "CREATE EXTENSION hstore"),
            ("runtime role substitution denied", f"SET ROLE {self.owner}"),
        ):
            self.denied(label, query)
        acl_query = "SELECT nspacl::text FROM pg_namespace WHERE nspname = 'public'"
        before = self.sql(acl_query)
        result = self.result("GRANT CREATE ON SCHEMA public TO PUBLIC", role=self.runtime)
        # PostgreSQL may return a warning rather than a nonzero exit for this
        # unauthorized GRANT. Prove both the diagnostic and unchanged ACL.
        self.check(
            "runtime schema grant cannot change ACL",
            bool(result.stderr) and self.sql(acl_query) == before,
        )
        self.check(
            "outsider database connect denied",
            self.sql(f"SELECT has_database_privilege('{outsider}', current_database(), 'CONNECT')")
            == "f"
            and self.result("SELECT 1", role=outsider).code != 0,
        )
        self.sql(f"GRANT {self.runtime} TO {outsider}", role="postgres", database="postgres")
        try:
            self.denied(
                "SET ROLE surrogate read denied",
                f"SET ROLE {self.runtime}; SELECT * FROM {READ}",
                role=outsider,
                detail="baseline_read_denied",
            )
        finally:
            self.sql(f"REVOKE {self.runtime} FROM {outsider}", role="postgres", database="postgres")

    def routine_permissions(self, routines: tuple[Routine, ...]) -> None:
        """Compare the native grant inventory with independently classified routines."""
        paths: dict[str, list[str]] = {}
        for routine in routines:
            if routine.trigger:
                continue
            identity = routine.identity.replace(", ", ",")
            if identity in paths:
                raise ToolingError("Duplicate expected runtime routine identity")
            paths[identity] = [f"search_path={routine.search_path}"]
        reset = "revaer_config.factory_reset_without_media_defaults_v1()"
        if reset not in paths or READ in paths:
            raise ToolingError("Expected routine inventory has an invalid lifecycle boundary")
        paths[reset].insert(0, "lock_timeout=5s")
        paths[READ] = ["search_path=pg_catalog, revaer_system"]
        rows = [
            object_value(row)
            for row in array_value(
                decode_unique(
                    self.sql(f"""
            SET search_path TO pg_catalog;
            SELECT COALESCE(json_agg(row_to_json(r)), '[]') FROM (
              SELECT p.oid::regprocedure::text AS identity, p.prosecdef,
                     pg_get_userbyid(p.proowner) = '{self.owner}' AS owned,
                     p.proconfig,
                     p.prorettype IN ('trigger'::regtype, 'event_trigger'::regtype) AS trigger,
                     EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid = 'pg_proc'::regclass
                       AND d.objid = p.oid AND d.deptype = 'e') AS extension
              FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
              WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime', 'revaer_system')
                AND has_function_privilege('{self.runtime}', p.oid, 'EXECUTE')
              ORDER BY p.oid::regprocedure::text
            ) r;
        """)
                )
            )
        ]
        self.connection.fs.write(
            self.connection.output / "final-runtime-routines.json",
            json.dumps(rows, indent=2) + "\n",
            0o600,
        )
        # Missing extension classification must not hide an authored routine.
        if any(not isinstance(row.get("extension"), bool) for row in rows):
            raise ToolingError("Runtime routine inventory lacks extension classification")
        authored = [row for row in rows if row["extension"] is False]
        self.check(
            "authored runtime routines hardened",
            bool(authored)
            and all(
                row.get("owned") is True
                and row.get("prosecdef") is True
                and row.get("trigger") is False
                and isinstance(row.get("proconfig"), list)
                and all(
                    isinstance(value, str) and "pg_temp" not in value
                    for value in array_value(row["proconfig"])
                )
                for row in authored
            ),
        )
        self.check("exact authored runtime grant count", len(authored) == len(paths))
        self.check(
            "exact per-routine search paths",
            len(authored) == len(paths)
            and all(isinstance(row.get("identity"), str) for row in authored)
            and {str(row["identity"]) for row in authored} == set(paths)
            and all(row.get("proconfig") == paths.get(str(row["identity"])) for row in authored),
        )
        self.check(
            "no authored PUBLIC routine execution",
            self.sql(f"""
                SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace,
                    LATERAL aclexplode(COALESCE(p.proacl, acldefault('f', p.proowner))) acl
                WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime', 'revaer_system')
                    AND p.proowner = (SELECT oid FROM pg_roles WHERE rolname = '{self.owner}')
                    AND acl.grantee = 0;
            """)
            == "0",
        )
        self.check(
            "no runtime relation privileges",
            self.sql(f"""
                SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime', 'revaer_system')
                AND ((c.relkind = 'r' AND has_table_privilege('{self.runtime}', c.oid,
                    'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER'))
                  OR (c.relkind = 'S' AND has_sequence_privilege('{self.runtime}', c.oid,
                    'USAGE,SELECT,UPDATE')));
            """)
            == "0",
        )
        self.check(
            "exact runtime database privilege matrix",
            self.sql(
                f"SELECT has_database_privilege('{self.runtime}', current_database(), 'CONNECT') "
                f"AND NOT has_database_privilege('{self.runtime}', "
                "current_database(), 'CREATE,TEMPORARY')"
            )
            == "t",
        )

    def call_paths(self) -> None:
        profile = "'00000000-0000-0000-0000-000000000001'"
        self.check(
            "runtime configuration read",
            self.sql(
                f"SELECT count(*) FROM revaer_config.fetch_app_profile_row({profile})",
                role=self.runtime,
            )
            == "1",
        )
        self.sql(
            f"SELECT revaer_config.update_app_instance_name({profile}, 'baseline-proof')",
            role=self.runtime,
        )
        self.check(
            "runtime configuration write and trigger",
            self.sql(
                f"SELECT instance_name FROM revaer_config.fetch_app_profile_row({profile})",
                role=self.runtime,
            )
            == "baseline-proof",
        )
        self.sql(
            "SELECT revaer_config.upsert_secret('proof-secret', decode('aa', 'hex'), 'proof')",
            role=self.runtime,
        )
        for label, query, expected in (
            (
                "runtime stored secret path",
                "SELECT count(*) FROM revaer_config.fetch_secret_by_name('proof-secret')",
                "1",
            ),
            ("runtime torrent listing", "SELECT count(*) FROM revaer_runtime.list_torrents()", "0"),
            (
                "runtime media queue read",
                "SELECT count(*) FROM public.media_job_list_v1(NULL, NULL)",
                "0",
            ),
            (
                "runtime bounded startup recovery on empty owned fixture",
                "SELECT count(*) FROM public.media_job_worker_resume_interrupted_v1("
                "workspace_root_input => '/revaer-baseline-proof')",
                "0",
            ),
        ):
            self.check(label, self.sql(query, role=self.runtime) == expected)

    def atomicity(self, final: bytes) -> None:
        result = self.result(
            f"BEGIN;\n{final.decode()}\n{self.seal_query()}\nSELECT 1 / 0; COMMIT;",
            database="rollback_proof",
        )
        self.check(
            "injected post-seal transaction failure",
            result.code != 0 and re.search(r"ERROR:\s+22012:", result.stderr) is not None,
        )
        self.check(
            "failed transaction leaves no user schemas",
            self.sql(
                "SELECT count(*) FROM pg_namespace WHERE nspname IN "
                "('revaer_config', 'revaer_runtime', 'revaer_system')",
                database="rollback_proof",
            )
            == "0",
        )
        self.check(
            "failed transaction leaves public empty",
            self.sql(
                "SELECT count(*) FROM pg_class WHERE relnamespace = 'public'::regnamespace",
                database="rollback_proof",
            )
            == "0",
        )

    def timeout_preservation(self, final: bytes) -> None:
        rows = self.sql(
            "BEGIN; SET LOCAL statement_timeout = '120s'; SET LOCAL lock_timeout = '120s'; "
            "SET LOCAL idle_in_transaction_session_timeout = '30s';\n" + final.decode() + "\n"
            "SELECT current_setting('statement_timeout') || ',' || current_setting('lock_timeout') "
            "|| ',' || current_setting('idle_in_transaction_session_timeout'); ROLLBACK;",
            database="timeout_proof",
        ).splitlines()
        observed = rows[-1].strip() if rows else ""
        self.check("initializer timeout preservation", observed == "2min,2min,30s")
        self.connection.fs.write(
            self.connection.output / "final-timeout-observed.txt",
            "statement_timeout,lock_timeout,idle_in_transaction_session_timeout\n"
            + observed
            + "\n",
            0o600,
        )

    def disable_bootstrap(self) -> None:
        self.sql(f"ALTER ROLE {self.owner} NOLOGIN", role="postgres", database="postgres")
        self.check(
            "runtime read after bootstrap disabled",
            self.sql(f"SELECT count(*) FROM {READ}", role=self.runtime) == "1",
        )
