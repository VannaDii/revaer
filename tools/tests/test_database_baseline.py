"""Reject misleading native diagnostics instead of accepting any failed query."""

from pathlib import Path

import pytest
from fixtures.database import CONTAINER, ProofDocker
from revaer_tooling.database.baseline import BaselineProof
from revaer_tooling.database.postgres import Connection
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.process import Completed


@pytest.mark.parametrize(
    "code,diagnostic,accepted",
    (
        (3, "ERROR:  P0001: denied\nDETAIL:  baseline_shape_invalid\n", True),
        (0, "ERROR:  P0001: denied\nDETAIL:  baseline_shape_invalid\n", False),
        (3, "ERROR:  42501: denied\nDETAIL:  baseline_shape_invalid\n", False),
        (3, "ERROR:  42501: expected P0001\nDETAIL:  baseline_shape_invalid\n", False),
        (3, "ERROR:  P0001: denied\nDETAIL:  baseline_shape_invalid_suffix\n", False),
        (3, "ERROR:  P0001: denied\n", False),
        (3, "ERROR:  P0001: denied\nDETAIL:  baseline_shape_invalid\nWARNING: unexpected\n", False),
        (3, "CONTEXT: ERROR: P0001: quoted text\nDETAIL:  baseline_shape_invalid\n", False),
    ),
)
def test_denial_requires_actual_sqlstate_exact_detail_and_no_warning(
    tmp_path: Path, code: int, diagnostic: str, accepted: bool
) -> None:
    docker = ProofDocker(tmp_path)
    docker.query_result = Completed(code, "", diagnostic)
    connection = Connection(
        docker, FileSystem(), CONTAINER, "postgres", "proof", "", None, tmp_path
    )
    checks: list[bool] = []
    proof = BaselineProof(
        connection, "owner", "runtime", "a" * 64, lambda _, passed: checks.append(passed)
    )
    proof.denied("malformed", "SELECT 1", state="P0001", detail="baseline_shape_invalid")
    assert checks == [accepted]
    assert docker.queries[0].role == "runtime"
    assert docker.queries[0].sql.startswith("\\set VERBOSITY verbose\n")


@pytest.mark.parametrize("digest", ("", "a" * 63, "A" * 64, "g" * 64, "a" * 64 + "'"))
def test_invalid_initializer_identity_fails_before_native_execution(
    tmp_path: Path, digest: str
) -> None:
    docker = ProofDocker(tmp_path)
    connection = Connection(
        docker, FileSystem(), CONTAINER, "postgres", "proof", "", None, tmp_path
    )
    with pytest.raises(ToolingError, match="SHA-256"):
        BaselineProof(connection, "owner", "runtime", digest, lambda *_: None)
    assert docker.queries == []
