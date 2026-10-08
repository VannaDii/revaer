"""Native transaction controls for the same sessions used by ingestion proofs."""

from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.database.contract import BUILD_INPUTS, Contract, PostgresPin, assignments
from revaer_tooling.database.ingestion.sessions import (
    ARGUMENTS,
    SessionControls,
    cases,
    control_records,
    ingestion_call,
    session,
)
from revaer_tooling.errors import ToolingError
from revaer_tooling.process import Completed
from test_database_contract import contract_root
from test_database_postgres import receipt

__all__ = ["contract_root"]


def test_fixture_argument_boundary_and_case_matrix() -> None:
    with pytest.raises(ToolingError, match="Unknown"):
        ingestion_call({"unreviewed_argument": "NULL"})
    with pytest.raises(ToolingError, match="SQL fixture"):
        session({}, helper_sql=" ")
    assert len(ARGUMENTS) == 24
    matrix = cases()
    assert len(matrix) == 13
    assert len({case.name for case in matrix}) == 13
    assert sum(case.helpers_first for case in matrix) == 6
    assert sum(case.repeat for case in matrix) == 1
    for case in matrix:
        sql = session(
            case.changes,
            repeat=case.repeat,
            helper_sql="SELECT 'helpers:fixture';" if case.helpers_first else None,
        )
        assert sql.count(ingestion_call(case.changes)) == len(case.expected)
        if case.helpers_first:
            assert sql.index("SELECT 'helpers:fixture'") < sql.index("SAVEPOINT ingestion_call")


@pytest.mark.parametrize(
    "outcome,failure_expected",
    (
        (Completed(1, "state: 00000\nstate: 00000\nwrites:1,2\n"), False),
        (Completed(0, "state: 00000\nwrites:1\n"), False),
        (Completed(0, "state: 00000\nstate: 00000\n", "WARNING: unknown\n"), False),
        (Completed(0, "state: 00000\nstate: 22012\n", ""), True),
        (Completed(0, "state: 00000\nstate: 22012\n", "ERROR:  42501: denied\n"), True),
        (
            Completed(
                0,
                "state: 00000\nstate: 22012\n",
                "ERROR:  22012: division by zero\nLOCATION:  int4div, int.c:870\n"
                "NOTICE: unexpected\n",
            ),
            True,
        ),
    ),
)
def test_control_rejects_transport_state_and_diagnostic_mismatch(
    outcome: Completed, failure_expected: bool
) -> None:
    with pytest.raises(ToolingError):
        control_records(outcome, failure_expected=failure_expected)


def test_native_warm_session_commits_and_rolls_back_only_the_failed_second_call(
    contract_root: Path,
) -> None:
    context = make_context(Options())
    contract = replace(
        Contract.load(contract_root, context.fs),
        postgres=PostgresPin.load(assignments(context.root, context.fs, BUILD_INPUTS)),
    )
    runtime = context.tools.proof_databases
    checks: dict[str, bool] = {}
    with runtime.open(contract, user="postgres", database="reference_proof") as connection:
        SessionControls(connection, lambda label, passed: checks.update({label: passed})).verify()
        assert len(checks) == 3, checks
        assert all(checks.values()), checks
        for name, writes in (("2", "1,2"), ("3", "1,3"), ("1-0", "1")):
            prefix = contract.output / ("session-control-" + name)
            assert f"writes:{writes}\n" in prefix.with_suffix(".stdout").read_text()
            assert prefix.with_suffix(".sql").stat().st_mode & 0o777 == 0o600
            assert prefix.with_suffix(".stderr").stat().st_mode & 0o777 == 0o600
    assert receipt(contract)["storage_removed"] is True
