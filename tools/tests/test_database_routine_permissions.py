"""Native grant mutations must fail the matching routine-permission assertion."""

import json
from dataclasses import replace
from pathlib import Path

from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.database.baseline import BaselineProof
from revaer_tooling.database.contract import BUILD_INPUTS, Contract, PostgresPin, assignments
from revaer_tooling.database.final_sql import Routine
from test_database_contract import contract_root
from test_database_postgres import receipt

__all__ = ["contract_root"]

RESET = "revaer_config.factory_reset_without_media_defaults_v1()"
NORMAL = "public.proof_read()"
EXPECTED = (
    Routine(RESET, "revaer_config", "factory_reset_without_media_defaults_v1", False, "pg_catalog"),
    Routine(NORMAL, "public", "proof_read", False, "pg_catalog"),
    Routine("public.proof_trigger()", "public", "proof_trigger", True, "pg_catalog"),
)


def test_native_runtime_grant_inventory_rejects_independent_mutations(contract_root: Path) -> None:
    context = make_context(Options())
    contract = replace(
        Contract.load(contract_root, context.fs),
        postgres=PostgresPin.load(assignments(context.root, context.fs, BUILD_INPUTS)),
    )
    runtime = context.tools.proof_databases
    checks: dict[str, bool] = {}

    def check(label: str, passed: bool) -> None:
        checks[label] = passed

    with runtime.open(contract, user="postgres", database="proof") as db:
        db.sql(
            "CREATE ROLE proof_owner LOGIN; CREATE ROLE proof_runtime LOGIN; "
            "ALTER DATABASE proof OWNER TO proof_owner; "
            "REVOKE ALL ON DATABASE proof FROM PUBLIC; "
            "GRANT CONNECT ON DATABASE proof TO proof_runtime;"
        )
        db.sql(
            f"""
            CREATE SCHEMA revaer_config;
            CREATE SCHEMA revaer_system;
            CREATE EXTENSION pgcrypto;
            CREATE TABLE public.proof_table (id integer);
            CREATE SEQUENCE public.proof_sequence;
            CREATE FUNCTION {NORMAL} RETURNS integer LANGUAGE sql SECURITY DEFINER
                SET search_path = pg_catalog AS 'SELECT 1';
            CREATE FUNCTION {RESET} RETURNS void LANGUAGE sql SECURITY DEFINER
                SET lock_timeout = '5s' SET search_path = pg_catalog AS 'SELECT NULL';
            CREATE FUNCTION revaer_system.read_database_baseline_v1() RETURNS integer
                LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, revaer_system
                AS 'SELECT 1';
            CREATE FUNCTION public.proof_trigger() RETURNS trigger LANGUAGE plpgsql
                SECURITY DEFINER SET search_path = pg_catalog AS 'BEGIN RETURN NULL; END;';
            REVOKE ALL ON FUNCTION {NORMAL}, {RESET},
                revaer_system.read_database_baseline_v1(), public.proof_trigger() FROM PUBLIC;
            GRANT USAGE ON SCHEMA public, revaer_config, revaer_system TO proof_runtime;
            GRANT EXECUTE ON FUNCTION {NORMAL}, {RESET},
                revaer_system.read_database_baseline_v1() TO proof_runtime;
        """,
            role="proof_owner",
        )
        proof = BaselineProof(db, "proof_owner", "proof_runtime", "a" * 64, check)
        proof.routine_permissions(EXPECTED)
        assert len(checks) == 6, checks
        assert all(checks.values()), checks
        evidence = contract.output / "final-runtime-routines.json"
        stock = json.loads(evidence.read_text())
        assert any(row["extension"] for row in stock)
        assert evidence.stat().st_mode & 0o777 == 0o600
        mutations = (
            (
                f"ALTER FUNCTION {NORMAL} SECURITY INVOKER",
                f"ALTER FUNCTION {NORMAL} SECURITY DEFINER",
                "authored runtime routines hardened",
            ),
            (
                f"ALTER FUNCTION {NORMAL} SET search_path = pg_catalog, pg_temp",
                f"ALTER FUNCTION {NORMAL} SET search_path = pg_catalog",
                "authored runtime routines hardened",
            ),
            (
                f"ALTER FUNCTION {RESET} RESET lock_timeout",
                f"ALTER FUNCTION {RESET} RESET ALL; "
                f"ALTER FUNCTION {RESET} SET lock_timeout = '5s'; "
                f"ALTER FUNCTION {RESET} SET search_path = pg_catalog",
                "exact per-routine search paths",
            ),
            (
                f"GRANT EXECUTE ON FUNCTION {NORMAL} TO PUBLIC",
                f"REVOKE EXECUTE ON FUNCTION {NORMAL} FROM PUBLIC",
                "no authored PUBLIC routine execution",
            ),
            (
                "GRANT SELECT ON public.proof_table TO proof_runtime",
                "REVOKE SELECT ON public.proof_table FROM proof_runtime",
                "no runtime relation privileges",
            ),
            (
                "GRANT USAGE ON public.proof_sequence TO proof_runtime",
                "REVOKE USAGE ON public.proof_sequence FROM proof_runtime",
                "no runtime relation privileges",
            ),
            (
                "GRANT TEMPORARY ON DATABASE proof TO proof_runtime",
                "REVOKE TEMPORARY ON DATABASE proof FROM proof_runtime",
                "exact runtime database privilege matrix",
            ),
            (
                f"ALTER FUNCTION {NORMAL} OWNER TO postgres",
                f"ALTER FUNCTION {NORMAL} OWNER TO proof_owner",
                "authored runtime routines hardened",
            ),
            (
                f"REVOKE EXECUTE ON FUNCTION {NORMAL} FROM proof_runtime",
                f"GRANT EXECUTE ON FUNCTION {NORMAL} TO proof_runtime",
                "exact authored runtime grant count",
            ),
            (
                "GRANT EXECUTE ON FUNCTION public.proof_trigger() TO proof_runtime",
                "REVOKE EXECUTE ON FUNCTION public.proof_trigger() FROM proof_runtime",
                "authored runtime routines hardened",
            ),
            (
                f"ALTER FUNCTION {NORMAL} RENAME TO unexpected_read",
                "ALTER FUNCTION public.unexpected_read() RENAME TO proof_read",
                "exact per-routine search paths",
            ),
        )
        for mutation, restore, failed in mutations:
            db.sql(mutation)
            checks.clear()
            try:
                proof.routine_permissions(EXPECTED)
                assert checks[failed] is False, (mutation, checks)
            finally:
                db.sql(restore)
            checks.clear()
            proof.routine_permissions(EXPECTED)
            assert all(checks.values()), (restore, checks)
            assert json.loads(evidence.read_text()) == stock
    assert receipt(contract)["storage_removed"] is True
