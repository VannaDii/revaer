"""Verify cleanup under interruption, uncertain creation and native SQL failures."""

import json
import secrets
import sys
import time
from dataclasses import replace
from pathlib import Path

import pytest
from fixtures.database import CONTAINER, ProofDocker
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.database.contract import BUILD_INPUTS, Contract, PostgresPin, assignments
from revaer_tooling.database.postgres import ProofDatabase
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.postgres import DumpArgs, PostgresDocker, ProofContainerArgs, QueryArgs
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.process import Completed
from test_database_contract import contract_root
from test_external import RecordingRunner

__all__ = ["contract_root"]


def database(root: Path) -> tuple[ProofDatabase, ProofDocker, Contract]:
    docker = ProofDocker(root)
    tokens = iter(("c" * 32, "d" * 32))
    runtime = ProofDatabase(docker, FileSystem(), lambda duration: None, lambda: next(tokens))
    return runtime, docker, Contract.load(root, runtime.fs)


def receipt(contract: Contract) -> dict[str, object]:
    paths = list(contract.output.glob("postgres-*.json"))
    assert len(paths) == 1
    result: dict[str, object] = json.loads(paths[0].read_text())
    assert paths[0].stat().st_mode & 0o777 == 0o600
    assert "d" * 32 not in paths[0].read_text()
    assert not list(contract.output.glob("postgres-credentials-*"))
    return result


@pytest.mark.parametrize("failure", ("create", "volumes", "start", "ready", "port", "query"))
def test_partial_startup_and_uncertain_creation_remove_owned_resources(
    contract_root: Path,
    failure: str,
) -> None:
    runtime, docker, contract = database(contract_root)
    docker.failures.add(failure)
    with (
        pytest.raises(ToolingError, match=failure),
        runtime.open(contract, user="fixture", database="fixture", publish=True) as connection,
    ):
        connection.sql("SELECT 1;")
    assert "remove" in docker.calls
    assert not docker.exists
    evidence = receipt(contract)
    assert evidence["completed"] is False
    if failure != "volumes":
        assert evidence["storage_removed"] is True


@pytest.mark.parametrize("cleanup", ("remove", "require_volumes_removed"))
def test_cleanup_failure_retains_original_error_and_private_receipt(
    contract_root: Path,
    cleanup: str,
) -> None:
    runtime, docker, contract = database(contract_root)
    docker.failures.add(cleanup)
    with (
        pytest.raises(ToolingError, match="earlier ToolingError: primary proof failure"),
        runtime.open(contract, user="fixture", database="fixture"),
    ):
        raise ToolingError("primary proof failure")
    evidence = receipt(contract)
    assert evidence["storage_removed"] is False
    assert cleanup in str(evidence["cleanup_error"])


def test_interruption_and_readiness_timeout_cannot_leave_a_success_receipt(
    contract_root: Path,
) -> None:
    runtime, docker, contract = database(contract_root)
    with (
        pytest.raises(KeyboardInterrupt),
        runtime.open(contract, user="fixture", database="fixture"),
    ):
        raise KeyboardInterrupt
    assert not docker.exists
    assert receipt(contract)["completed"] is False
    runtime, docker, contract = database(contract_root)
    tokens = iter(("e" * 32, "f" * 32))
    runtime.token = lambda: next(tokens)
    docker.is_ready = False
    with (
        pytest.raises(ToolingError, match="did not become ready"),
        runtime.open(contract, user="fixture", database="fixture"),
    ):
        pytest.fail("Unready server was yielded")
    assert docker.calls.count("ready") == 120
    assert not docker.exists


@pytest.mark.parametrize(
    "outcome", (Completed(3, "", "ERROR: 22012\n"), Completed(0, "", "WARNING: example\n"))
)
def test_sql_failure_or_warning_is_retained_and_fails_the_proof(
    contract_root: Path,
    outcome: Completed,
) -> None:
    runtime, docker, contract = database(contract_root)
    docker.query_result = outcome
    with (
        pytest.raises(ToolingError, match="retained query-error"),
        runtime.open(contract, user="fixture", database="fixture") as connection,
    ):
        connection.sql("SELECT 1 / 0;")
    assert (contract.output / "query-error.txt").read_text() == outcome.stderr
    assert receipt(contract)["completed"] is False


def test_unpublished_database_rejects_host_connection_url(contract_root: Path) -> None:
    runtime, _, contract = database(contract_root)
    with runtime.open(contract, user="fixture", database="fixture") as connection:
        with pytest.raises(ToolingError, match="no published"):
            connection.url()
        assert connection.sql("SELECT 1", role="other", database="other") == "1"
    assert receipt(contract)["completed"] is True


def test_existing_ownership_receipt_is_never_reused(contract_root: Path) -> None:
    runtime, _, contract = database(contract_root)
    with runtime.open(contract, user="fixture", database="fixture"):
        pass
    original = receipt(contract)
    runtime, docker, contract = database(contract_root)
    with (
        pytest.raises(ToolingError, match="refusing to reuse"),
        runtime.open(contract, user="fixture", database="fixture"),
    ):
        pytest.fail("Existing ownership was reused")
    assert not docker.calls
    assert receipt(contract) == original


@pytest.mark.parametrize(
    "port",
    ("0.0.0.0:1234\n", "127.0.0.1:0\n", "127.0.0.1:65536\n", "127.0.0.1:1234\n127.0.0.1:1235\n"),
)
def test_native_adapter_rejects_nonlocal_or_ambiguous_ports(tmp_path: Path, port: str) -> None:
    docker = PostgresDocker(sys.executable, RecordingRunner(port), tmp_path, {})
    with pytest.raises(ToolingError, match="loopback"):
        docker.port(CONTAINER)


@pytest.mark.parametrize(
    "mounts",
    (
        "[]",
        '[{"Type":"bind"}]',
        '[{"Type":"volume","RW":true,"Destination":"/elsewhere"}]',
        '[{"Type":"volume","RW":true,"Destination":"/var/lib/postgresql/data","Name":"named"}]',
    ),
)
def test_native_adapter_rejects_unowned_storage(tmp_path: Path, mounts: str) -> None:
    docker = PostgresDocker(sys.executable, RecordingRunner(mounts), tmp_path, {})
    with pytest.raises(ToolingError):
        docker.volumes(CONTAINER)


@pytest.mark.parametrize(
    "payload",
    (
        [],
        [{"Id": "unexpected"}],
        [{"Id": "sha256:" + "a" * 64, "RepoDigests": []}],
        [{"Id": "sha256:" + "a" * 64, "RepoDigests": [None]}],
        [{"Id": "sha256:" + "a" * 64, "RepoDigests": ["image"], "Architecture": False}],
    ),
)
def test_native_adapter_rejects_missing_image_identity(
    tmp_path: Path, payload: list[dict[str, object]]
) -> None:
    docker = PostgresDocker(sys.executable, RecordingRunner(json.dumps(payload)), tmp_path, {})
    with pytest.raises(ToolingError):
        docker.image_identity(PostgresPin("docker.io/library/postgres@sha256:" + "a" * 64, "16.14"))


def test_adapter_rejects_unsafe_names_and_credentials_before_docker(contract_root: Path) -> None:
    runner = RecordingRunner()
    docker = PostgresDocker(sys.executable, runner, contract_root, {})
    contract = Contract.load(contract_root, FileSystem())
    credentials = contract_root / "credentials"
    credentials.write_text("POSTGRES_PASSWORD=fixture\n")
    credentials.chmod(0o644)
    args = ProofContainerArgs(
        "rv-pg-proof-" + "a" * 32, contract.postgres, "fixture", "fixture", credentials
    )
    for invalid in (replace(args, name="foreign"), args):
        with pytest.raises(ToolingError):
            docker.create(invalid)
    credentials.chmod(0o600)
    with pytest.raises(ToolingError, match="SQL identifiers"):
        docker.create(replace(args, user="--help"))
    with pytest.raises(ToolingError, match="complete container ID"):
        docker.query("named-container", QueryArgs("fixture", "fixture", "SELECT 1"))
    assert not runner.calls


def test_native_postgresql_transaction_warning_and_storage_lifecycle(contract_root: Path) -> None:
    # Uses the same reviewed image as candidate generation, with networking
    # disabled and storage owned only by this fixture's exact container ID.
    context = make_context(Options())
    contract = Contract.load(contract_root, context.fs)
    contract = replace(
        contract,
        postgres=PostgresPin.load(
            assignments(context.root, context.fs, BUILD_INPUTS),
        ),
    )
    runtime = ProofDatabase(
        context.tools.proof_databases.docker, context.fs, time.sleep, lambda: secrets.token_hex(16)
    )
    runtime.docker.verify_image(contract.postgres)
    with runtime.open(contract, user="fixture", database="fixture") as connection:
        assert connection.sql("SHOW server_version_num") == "160014"
        connection.sql("CREATE TABLE item (id integer)")
        with pytest.raises(ToolingError, match="query failed"):
            connection.sql("INSERT INTO item VALUES (1); SELECT 1 / 0;", transaction=True)
        assert connection.sql("SELECT count(*) FROM item") == "0"
        with pytest.raises(ToolingError, match="query failed"):
            connection.sql("DO $$ BEGIN RAISE WARNING 'fixture warning'; END $$;")
        runtime.docker.create_database(connection.container, "fixture", "second")
        dump = runtime.docker.dump(connection.container, DumpArgs("fixture", "fixture"))
        assert "CREATE TABLE public.item" in dump
    assert receipt(contract)["storage_removed"] is True
