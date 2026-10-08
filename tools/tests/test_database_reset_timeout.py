"""Native reset scope, cancellation and contention; optional real initializer."""

import hashlib
import os
import secrets
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.database.contract import BUILD_INPUTS, Contract, PostgresPin, assignments
from revaer_tooling.database.reset_timeout import RESET, ResetTimeoutProof
from revaer_tooling.errors import ToolingError
from test_database_contract import contract_root
from test_database_postgres import receipt

__all__ = ["contract_root"]

FIXTURE = f"""
CREATE SCHEMA revaer_system;
CREATE SCHEMA revaer_config;
CREATE TABLE public.app_profile (id integer);
CREATE FUNCTION {RESET} RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog SET lock_timeout = '5s' AS $reset$
BEGIN TRUNCATE public.app_profile; END; $reset$;
GRANT USAGE ON SCHEMA revaer_config, revaer_system TO proof_runtime;
GRANT EXECUTE ON FUNCTION {RESET} TO proof_runtime;
"""


def test_native_reset_scopes_and_observer_cleanup(contract_root: Path) -> None:
    context = make_context(Options())
    contract = replace(
        Contract.load(contract_root, context.fs),
        postgres=PostgresPin.load(assignments(context.root, context.fs, BUILD_INPUTS)),
    )
    databases = context.tools.proof_databases
    checks: dict[str, bool] = {}

    def check(label: str, passed: bool) -> None:
        assert label not in checks
        checks[label] = passed

    with databases.open(contract, user="postgres", database="final_proof") as connection:
        connection.sql(
            "CREATE ROLE proof_owner LOGIN; CREATE ROLE proof_runtime LOGIN; "
            "ALTER DATABASE final_proof OWNER TO proof_owner;"
        )
        initializer = os.environ.get("REVAER_TEST_INITIALIZER")
        source = Path(initializer).read_bytes() if initializer else FIXTURE.encode()
        connection.sql(source.decode(), role="proof_owner", transaction=True)
        if initializer:
            digest = hashlib.sha256(source).hexdigest()
            connection.sql(
                "SELECT * FROM revaer_system.seal_database_baseline_v1(1::smallint, "
                f"decode('{digest}', 'hex'), 'proof_runtime');",
                role="proof_owner",
            )
        with ThreadPoolExecutor(max_workers=1) as executor:
            proof = ResetTimeoutProof(
                connection,
                "proof_owner",
                "proof_runtime",
                check,
                executor,
                time.monotonic,
                time.sleep,
                lambda: secrets.token_hex(16),
            )
            proof.verify()
            assert len(checks) == 7
            assert all(checks.values()), checks
            assert (
                connection.sql(
                    "SELECT count(*) FROM pg_proc WHERE proname IN "
                    "('proof_reset_observer', 'proof_nested_reset')"
                )
                == "0"
            )
            transcript = contract.output / "final-reset-timeout-transcript.txt"
            assert transcript.stat().st_mode & 0o777 == 0o600
            assert "reset-timeout-caught:57014:17s" in transcript.read_text()
            assert "reset-timeout-caught:55P03:17s" in transcript.read_text()

            # Removing the actual routine's bound must be detected, without
            # spending another contention timeout on the deliberately bad case.
            connection.sql(f"ALTER FUNCTION {RESET} RESET lock_timeout", role="proof_owner")
            checks.clear()
            proof.observer("")
            connection.sql(
                "CREATE TRIGGER proof_reset_timeout BEFORE TRUNCATE ON public.app_profile "
                "FOR EACH STATEMENT EXECUTE FUNCTION revaer_system.proof_reset_observer()",
                role="proof_owner",
            )
            proof.success()
            assert len(checks) == 2
            assert not any(checks.values())
            connection.sql(
                "DROP TRIGGER proof_reset_timeout ON public.app_profile; "
                "DROP FUNCTION revaer_system.proof_reset_observer();",
                role="proof_owner",
            )

            # A warning from the real observer is fatal and must still remove
            # both observer objects and retain the diagnostic transcript.
            class WarningProof(ResetTimeoutProof):
                def observer(self, body: str) -> None:
                    super().observer("RAISE WARNING 'unexpected observer warning'; " + body)

            warning = WarningProof(
                connection,
                "proof_owner",
                "proof_runtime",
                check,
                executor,
                time.monotonic,
                time.sleep,
                lambda: secrets.token_hex(16),
            )
            with pytest.raises(ToolingError, match="PostgreSQL warning"):
                warning.verify()
            assert (
                connection.sql(
                    "SELECT count(*) FROM pg_proc WHERE proname = 'proof_reset_observer'"
                )
                == "0"
            )
            assert "unexpected observer warning" in transcript.read_text()
    assert receipt(contract)["storage_removed"] is True
