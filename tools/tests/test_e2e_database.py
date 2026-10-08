"""Media E2E keeps explicit endpoint ownership and stops services before drop."""

import secrets
import shutil
from dataclasses import replace
from pathlib import Path
from urllib.parse import urlsplit

import pytest
from revaer_tooling.context import Context, TaskResult
from revaer_tooling.database.lifecycle_settings import LifecycleSettings
from revaer_tooling.e2e.database import uses_single_init, verify_endpoint
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks import e2e
from revaer_tooling.tasks.database_lifecycle import DatabaseTestDrop, DatabaseTestInit
from test_e2e_coordinator import Scenario
from test_e2e_coordinator import coordinator as coordinator


@pytest.mark.parametrize("host", ("localhost", "127.0.0.1", "host.docker.internal"))
def test_local_endpoint_requires_exact_loopback_binding(host: str) -> None:
    password = secrets.token_hex(24)
    verify_endpoint(f"postgresql://user:{password}@{host}:5450/postgres", "127.0.0.1:5450\n")


@pytest.mark.parametrize(
    "url,binding",
    (
        ("postgres://remote.invalid:5450/postgres", "127.0.0.1:5450"),
        ("postgres://localhost:5450/postgres?host=remote.invalid", "127.0.0.1:5450"),
        ("postgres://localhost:5450/postgres#fragment", "127.0.0.1:5450"),
        ("https://localhost:5450/postgres", "127.0.0.1:5450"),
        ("postgres://localhost:5450/postgres", "0.0.0.0:5450"),
        ("postgres://localhost:5450/postgres", "127.0.0.1:5451"),
        ("postgres://localhost:5450/postgres", "127.0.0.1:5450\n[::]:5450"),
    ),
)
def test_unverified_endpoint_is_rejected(url: str, binding: str) -> None:

    with pytest.raises(ToolingError):
        verify_endpoint(url, binding)


@pytest.mark.parametrize("failure", ("", "start-api", "api-api-key"))
def test_media_coordinator_uses_sealed_runtime_and_preserves_cleanup_order(
    coordinator: tuple[Context, Scenario],
    monkeypatch: pytest.MonkeyPatch,
    failure: str,
) -> None:

    context, scenario = coordinator
    source = Path(__file__).resolve().parents[2]
    (context.root / "config").mkdir()
    (context.root / "config/database-rebaseline.env").write_text(
        "TRANSITION_PHASE=feature-development\n"
    )
    (context.root / ".github").mkdir()
    shutil.copyfile(source / ".github/build-inputs.env", context.root / ".github/build-inputs.env")
    context = replace(
        context,
        settings=replace(
            context.settings,
            lifecycle=LifecycleSettings("selected", "fixture", secrets.token_hex(16), ""),
            e2e=replace(
                context.settings.e2e, admin_url="postgres://fixture@localhost:5450/postgres"
            ),
        ),
    )
    scenario.failure = failure
    container = "a" * 64
    selections: list[Context] = []

    def select(name: str, image: str) -> str:
        assert name == "selected"
        assert "@sha256:" in image
        scenario.visit("select-container")
        return container

    def binding(identity: str) -> str:
        assert identity == container
        return "127.0.0.1:5450"

    def ready(urls: tuple[str, ...]) -> str:
        scenario.visit("ready")
        return urls[0]

    def initialize(selected: Context) -> TaskResult:
        assert selected.settings.lifecycle.container == container
        assert len(selected.settings.lifecycle.runtime_password) == 64
        assert selected.options.database_name.startswith("revaer_test_")
        selections.append(selected)
        scenario.visit("initialize-sealed")
        return TaskResult()

    def drop(selected: Context) -> TaskResult:
        assert selected is selections[0]
        scenario.visit("drop-sealed")
        return TaskResult()

    monkeypatch.setattr(context.tools.lifecycle_docker, "select", select)
    monkeypatch.setattr(context.tools.lifecycle_docker, "binding", binding)
    monkeypatch.setattr(context.tools.pg_isready, "wait", ready)
    monkeypatch.setattr(DatabaseTestInit, "run", initialize)
    monkeypatch.setattr(DatabaseTestDrop, "run", drop)
    # Exercise URL construction directly, then the full service/phase coordinator.
    with e2e.temporary_database(context) as url:
        parsed = urlsplit(url)
        assert parsed.username == selections[0].options.database_name + "_runtime"
        assert parsed.password == selections[0].settings.lifecycle.runtime_password
        assert parsed.path == "/" + selections[0].options.database_name
    selections.clear()
    scenario.events.clear()
    if failure:
        with pytest.raises(ToolingError, match="injected"):
            e2e.UiE2e.run(context)
    else:
        e2e.UiE2e.run(context)
    assert "migrate" not in scenario.events
    assert "create-database" not in scenario.events
    assert scenario.events[-1] == "drop-sealed"
    if "start-ui" in scenario.events:
        assert scenario.events[-3:] == ["stop-ui", "stop-api", "drop-sealed"]
    elif failure != "start-api":
        assert scenario.events[-2:] == ["stop-api", "drop-sealed"]


@pytest.mark.parametrize("phase", ("", "unknown"))
def test_invalid_phase_cannot_fall_back_to_migrations(
    coordinator: tuple[Context, Scenario],
    phase: str,
) -> None:
    context, _ = coordinator
    directory = context.root / "config"
    directory.mkdir()
    (directory / "database-rebaseline.env").write_text(
        f"TRANSITION_PHASE={phase}\n" if phase else "# missing phase\n"
    )
    with pytest.raises(ToolingError, match="transition phase"):
        uses_single_init(context)
