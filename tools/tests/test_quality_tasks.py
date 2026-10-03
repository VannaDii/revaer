"""Validation composition preserves database ownership and stops on failure.

Individual gates have real-tool integration tests. These tests isolate the
composition boundary to prove that failure cannot reach a later release build
and that every gate receives the selected, locked database connection.
"""

from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options, TaskResult
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.database import with_database
from revaer_tooling.tasks import quality
from revaer_tooling.tasks.build import BuildRelease
from revaer_tooling.tasks.database import DatabaseStart


@pytest.mark.parametrize("failure", [None, "start", "gate", "build"])
@pytest.mark.parametrize("explicit", [False, True])
def test_ci_holds_connection_until_gates_finish_and_never_builds_after_failure(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, failure: str | None, explicit: bool
) -> None:
    context = make_context(Options())
    database = replace(
        context.settings.database,
        test_url="postgres://tests@disposable/postgres" if explicit else None,
    )
    context = replace(context, settings=replace(context.settings, database=database))
    calls: list[str] = []
    lock = tmp_path / "database.lock"
    normalized = "postgres://local@127.0.0.1:5441/revaer?sslmode=disable"

    def start(active: Context) -> TaskResult:
        calls.append("start")
        if failure == "start":
            raise ToolingError("start failure")
        return TaskResult()

    @contextmanager
    def connection(active: Context) -> Iterator[str]:
        with active.fs.lock(lock):
            yield normalized

    def gate(active: Context) -> TaskResult:
        calls.append("gate")
        with pytest.raises(ToolingError, match="Another operation"), active.fs.lock(lock):
            raise AssertionError("The database lock must cover every gate")
        assert active.settings.database.url == normalized
        assert active.settings.database.test_url == (
            "postgres://tests@disposable/postgres"
            if explicit
            else "postgres://local@127.0.0.1:5441/postgres?sslmode=disable"
        )
        if failure == "gate":
            raise ToolingError("gate failure")
        return TaskResult("completed gate")

    def build(active: Context) -> TaskResult:
        calls.append("build")
        with active.fs.lock(lock):
            # Database protection can end once all validation consumers finish.
            if failure == "build":
                raise ToolingError("build failure")
        return TaskResult()

    monkeypatch.setattr(DatabaseStart, "run", start)
    monkeypatch.setattr(quality, "database_connection", connection)
    monkeypatch.setattr(quality, "VALIDATION_STEPS", {"first": gate, "second": gate})
    monkeypatch.setattr(BuildRelease, "run", build)
    if failure:
        with pytest.raises(ToolingError, match=f"{failure} failure"):
            quality.Ci.run(context)
    else:
        assert quality.Ci.run(context).message == "Validation and release build passed"
    assert calls == {
        "start": ["start"],
        "gate": ["start", "gate"],
    }.get(failure or "", ["start", "gate", "gate", "build"])
    with context.fs.lock(lock):
        assert context.settings.database.url == database.url


def test_database_selection_preserves_options_and_encodes_the_name() -> None:
    assert with_database("postgresql://user:p%40ss@[::1]/old?sslmode=require", "new/name") == (
        "postgresql://user:p%40ss@[::1]/new%2Fname?sslmode=require"
    )


@pytest.mark.parametrize(
    ("url", "name"),
    [
        ("sqlite:/example", "name"),
        ("postgres://server/old#fragment", "name"),
        ("postgres://server/old?dbname=other", "name"),
        ("postgres://server/old", ""),
        ("postgres://server/old", "with\x00nul"),
    ],
)
def test_database_selection_rejects_ambiguous_or_invalid_identity(url: str, name: str) -> None:
    with pytest.raises(ToolingError):
        with_database(url, name)
