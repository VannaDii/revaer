"""Execute extension catalog mutations on the pinned native PostgreSQL image."""

import json
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.database.contract import BUILD_INPUTS, Contract, PostgresPin, assignments
from revaer_tooling.database.extensions import ExtensionProof
from revaer_tooling.errors import ToolingError
from test_database_contract import contract_root
from test_database_postgres import receipt

__all__ = ["contract_root"]


def test_native_extension_identity_mutations_and_transaction_rollback(contract_root: Path) -> None:
    context = make_context(Options())
    contract = replace(
        Contract.load(contract_root, context.fs),
        postgres=PostgresPin.load(assignments(context.root, context.fs, BUILD_INPUTS)),
    )
    runtime = context.tools.proof_databases
    runtime.docker.verify_image(contract.postgres)
    checks: dict[str, bool] = {}

    def check(label: str, passed: bool) -> None:
        assert label not in checks
        checks[label] = passed

    with runtime.open(contract, user="postgres", database="final_proof") as connection:
        connection.sql("CREATE ROLE proof_owner LOGIN; CREATE ROLE proof_runtime LOGIN;")
        for database in ("final_proof", "extension_proof"):
            if database != "final_proof":
                runtime.docker.create_database(connection.container, "postgres", database)
            connection.sql(f"ALTER DATABASE {database} OWNER TO proof_owner;")
        connection.sql(
            "CREATE EXTENSION pgcrypto WITH SCHEMA public; "
            "CREATE EXTENSION unaccent WITH SCHEMA public;",
            role="proof_owner",
        )
        proof = ExtensionProof(connection, "proof_owner", "proof_runtime", check)
        proof.verify()
        assert len(checks) == 27
        assert all(checks.values()), checks
        stock = proof.snapshot()
        assert json.loads((contract.output / "final-stock-extensions.json").read_text()) == stock
        assert json.loads((contract.output / "final-observed-extensions.json").read_text()) == stock
        assert (contract.output / "final-stock-extensions.json").stat().st_mode & 0o777 == 0o600

        # An error before the explicit ROLLBACK must still undo all earlier SQL.
        with pytest.raises(ToolingError, match="query failed"):
            proof.snapshot(mutation="ALTER FUNCTION public.digest(text, text) COST 200; SELECT 1/0")
        assert proof.snapshot() == stock
        connection.sql("ALTER FUNCTION public.digest(text, text) COST 200")
        assert proof.snapshot() != stock
        # Exercise the verifier itself on a drifted target, not only its reader.
        runtime.docker.create_database(connection.container, "postgres", "second_reference")
        connection.sql("ALTER DATABASE second_reference OWNER TO proof_owner")
        checks.clear()
        proof.verify("second_reference")
        assert (
            checks["exact pinned stock extension definitions ownership membership and ACLs"]
            is False
        )
        assert checks["D1 changed extension definition rolls back exactly"] is False
        with pytest.raises(ToolingError, match="lowercase SQL identifiers"):
            ExtensionProof(connection, "proof_owner", "injected'; SELECT 1", check)
    assert receipt(contract)["storage_removed"] is True
