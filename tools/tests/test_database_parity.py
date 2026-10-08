"""Exercise native schema/seed drift and narrow normalization boundaries."""

import json
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.database.contract import BUILD_INPUTS, Contract, PostgresPin, assignments
from revaer_tooling.database.final_sql import approved_legacy_deltas
from revaer_tooling.database.parity import (
    DatabaseParity,
    normalize_routine_security,
    normalize_seed_identities,
)
from revaer_tooling.errors import ToolingError
from test_database_contract import contract_root
from test_database_final_sql import CANDIDATE
from test_database_postgres import receipt

__all__ = ["contract_root"]


def test_security_normalization_preserves_body_and_unrelated_settings() -> None:
    source = b"""CREATE FUNCTION public.f() RETURNS text
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog'
    SET lock_timeout TO '5s'
    AS $$ SELECT ' SECURITY DEFINER';
    SET search_path TO 'body_text'; $$;
"""
    expected = source.replace(b"LANGUAGE sql SECURITY DEFINER", b"LANGUAGE sql").replace(
        b"    SET search_path TO 'pg_catalog'\n", b""
    )
    assert normalize_routine_security(source) == expected


@pytest.mark.parametrize(
    "identities",
    (
        (),
        ("a" * 32,),
        ("aa", "bb"),
        ("00000000-0000-4000-8000-000000000001",) * 2,
        ("00000000-0000-1000-8000-000000000001", "00000000-0000-4000-8000-000000000002"),
        ("00000000-0000-4000-7000-000000000001", "00000000-0000-4000-8000-000000000002"),
    ),
)
def test_only_two_distinct_uuid4_seed_ids_may_be_normalized(identities: tuple[str, ...]) -> None:
    rows = [
        {"display_name": str(index), "rate_limit_policy_public_id": value}
        for index, value in enumerate(identities)
    ]
    with pytest.raises(ToolingError, match="identities"):
        normalize_seed_identities("public.rate_limit_policy:" + json.dumps(rows))


def test_unrelated_seed_bytes_are_not_reencoded() -> None:
    source = 'public.other:[{"id":"00000000-0000-4000-8000-000000000001", "name":"é"}]\n'
    assert normalize_seed_identities(source) == source


def test_native_parity_detects_schema_seed_and_generated_identity_drift(
    contract_root: Path,
) -> None:
    context = make_context(Options())
    contract = replace(
        Contract.load(contract_root, context.fs),
        postgres=PostgresPin.load(assignments(context.root, context.fs, BUILD_INPUTS)),
    )
    runtime = context.tools.proof_databases
    checks: dict[str, bool] = {}
    with runtime.open(contract, user="postgres", database="observed") as db:
        db.sql("CREATE ROLE proof_owner LOGIN; ALTER DATABASE observed OWNER TO proof_owner")
        db.docker.create_database(db.container, "postgres", "reference_proof")
        db.sql("ALTER DATABASE reference_proof OWNER TO proof_owner")
        for database in ("observed", "reference_proof"):
            db.sql(
                "CREATE SCHEMA revaer_config; CREATE SCHEMA revaer_runtime;",
                role="proof_owner",
                database=database,
            )
            # The real ingestion body contains nested dollar quotes. Preserve
            # that envelope so native pg_dump emits its reviewed $_$ delimiter.
            candidate = CANDIDATE.replace(
                b"BEGIN\n    CREATE TEMP", b"BEGIN\n    PERFORM '$$';\n    CREATE TEMP"
            )
            source = approved_legacy_deltas(candidate) if database == "observed" else candidate
            db.sql(
                source.decode(),
                role="proof_owner" if database == "observed" else "postgres",
                database=database,
            )
            db.sql(
                """
                CREATE TABLE public.rate_limit_policy (
                    rate_limit_policy_public_id uuid DEFAULT gen_random_uuid(),
                    display_name text, changed_at timestamptz DEFAULT now());
                INSERT INTO public.rate_limit_policy(display_name) VALUES ('alpha'), ('beta');
                CREATE TABLE public.empty_table (id integer);
            """,
                role="proof_owner",
                database=database,
            )
        # These differences are explicitly covered by other stages. They must
        # not conceal unrelated routine body, table or seed changes here.
        db.sql(
            "CREATE SCHEMA revaer_system; "
            "ALTER FUNCTION public.observe() SECURITY DEFINER; "
            "ALTER FUNCTION public.observe() SET search_path = pg_catalog",
            role="proof_owner",
        )
        parity = DatabaseParity(
            db, "proof_owner", lambda name, passed: checks.update({name: passed})
        )
        parity.verify()
        assert len(checks) == 2, checks
        assert all(checks.values()), checks
        schema_check = "two-way legacy schema and extension parity"
        seed_check = "two-way seed parity excluding generated timestamp columns"
        for name in ("schema.sql", "seeds.jsonl"):
            reference = contract.output / f"final-reference-{name}"
            observed = contract.output / f"final-observed-{name}"
            assert reference.read_bytes() == observed.read_bytes()
            assert observed.stat().st_mode & 0o777 == 0o600
        db.sql("ALTER TABLE public.empty_table ADD COLUMN changed boolean", role="proof_owner")
        parity.verify()
        assert checks[schema_check] is False
        db.sql("ALTER TABLE public.empty_table DROP COLUMN changed", role="proof_owner")
        db.sql(
            "UPDATE public.rate_limit_policy SET display_name = 'changed' "
            "WHERE display_name = 'alpha'",
            role="proof_owner",
        )
        parity.verify()
        assert checks[schema_check] is True
        assert checks[seed_check] is False
        db.sql(
            "UPDATE public.rate_limit_policy SET rate_limit_policy_public_id = "
            "'00000000-0000-1000-8000-000000000001'",
            role="proof_owner",
        )
        with pytest.raises(ToolingError, match="identities"):
            parity.verify()
        assert not (contract.output / "final-observed-seeds.jsonl").exists()
        assert not (contract.output / "final-reference-seeds.jsonl").exists()
    assert receipt(contract)["storage_removed"] is True
