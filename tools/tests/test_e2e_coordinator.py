"""Exercise the coordinator's ownership and failure boundaries without a server.

HTTP, SQLx and process adapters have independent real-tool tests. Here injected
failures prove orchestration: a port is never adopted, services stop before the
unique database is dropped, and partial runs cannot leave a passing summary.
"""

import json
from collections.abc import Iterator, Mapping
from contextlib import contextmanager
from dataclasses import dataclass, field, replace
from pathlib import Path
from urllib.parse import urlsplit

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options, TaskResult
from revaer_tooling.e2e.api import ApiClient, ApiSession
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.http import HttpRequest, HttpResult
from revaer_tooling.external.python import E2ePytestArgs
from revaer_tooling.external.rust import TrunkServeArgs
from revaer_tooling.external.serving import ApplicationArgs, ServingExecutable, ServingKind
from revaer_tooling.process import Completed
from revaer_tooling.settings import load_settings
from revaer_tooling.tasks import e2e
from revaer_tooling.tasks.build import SyncAssets


@dataclass
class Scenario:
    failure: str = ""
    events: list[str] = field(default_factory=list)
    phase_count: int = 0
    catalogs: list[Path | None] = field(default_factory=list)
    auth_headers: list[dict[str, str]] = field(default_factory=list)

    def visit(self, event: str) -> None:
        self.events.append(event)
        if self.failure == event:
            raise ToolingError("injected " + event)


@dataclass
class Service:
    scenario: Scenario
    name: str
    pid: int

    def poll(self) -> int | None:
        if self.scenario.failure == "service-exit" and self.scenario.phase_count > 0:
            return 1
        return None

    def stop(self) -> None:
        self.scenario.visit("stop-" + self.name)

    def wait(self) -> Completed:
        raise AssertionError("The service owner must stop, not wait for a long-lived service")


@pytest.fixture
def coordinator(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Context, Scenario]:
    for name in (".git", "tools/src/revaer_tooling", "tests/test-results", "docs/api"):
        (tmp_path / name).mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    (tmp_path / "Cargo.toml").touch()
    (tmp_path / "tests/e2e.toml").write_text('[api]\nrunner="binary"\nmedia_runner="lib-test"\n')
    (tmp_path / "docs/api/openapi.json").write_text(
        json.dumps(
            {
                "paths": {
                    "/v1/media/root-catalog/readiness": {
                        "get": {
                            "responses": {
                                "200": {
                                    "content": {"application/json": {"schema": {"type": "object"}}}
                                }
                            }
                        }
                    }
                }
            }
        )
    )
    for name in ("api-coverage-stale.json", "ui-coverage-stale.json", "selection-stale.json"):
        (tmp_path / "tests/test-results" / name).write_text('["stale"]')
    monkeypatch.chdir(tmp_path)
    # Explicit settings prevent a developer's .env or E2E selection from changing
    # which resource transitions this fault-injection suite exercises.
    context = make_context(Options())
    context = replace(context, settings=load_settings({"E2E_HTTP_WAIT_ATTEMPTS": "1"}))
    scenario = Scenario()

    @contextmanager
    def connection(owner: Context) -> Iterator[str]:
        assert not owner.settings.database.reset
        scenario.visit("admin-connect")
        yield "postgres://fixture.invalid/postgres"

    def database_create(url: str) -> Completed:
        assert urlsplit(url).path.startswith("/revaer_e2e_")
        scenario.visit("create-database")
        return Completed(0, "")

    def database_migrate(url: str, migrations: Path) -> Completed:
        assert urlsplit(url).path != "/postgres"
        scenario.visit("migrate")
        return Completed(0, "")

    def database_drop(url: str) -> Completed:
        assert urlsplit(url).path.startswith("/revaer_e2e_")
        scenario.visit("drop-database")
        return Completed(0, "")

    def require_free(port: int) -> None:
        scenario.visit("free-" + str(port))

    def require_owner(port: int, pid: int) -> None:
        assert (port, pid) in ((7070, 100), (8080, 200))
        scenario.visit("owner-" + str(port))

    def executable(kind: ServingKind) -> ServingExecutable:
        return ServingExecutable(tmp_path / "application", kind)

    def sync_assets(owner: Context) -> TaskResult:
        return TaskResult()

    def start_api(args: ApplicationArgs) -> Service:
        scenario.visit("start-api")
        scenario.catalogs.append(args.media_root_catalog)
        return Service(scenario, "api", 100)

    def start_ui(args: TrunkServeArgs) -> Service:
        scenario.visit("start-ui")
        return Service(scenario, "ui", 200)

    def health(args: HttpRequest) -> HttpResult:
        if args.url.endswith("/v1/media/root-catalog/readiness"):
            scenario.visit("catalog-readiness")
            state = "missing" if scenario.failure == "catalog-unready" else "ready"
            return HttpResult(
                200,
                {"content-type": "application/json"},
                json.dumps({"source_state": state, "attestation_state": state}),
            )
        scenario.visit("health-" + str(urlsplit(args.url).port))
        return HttpResult(200, {}, "")

    def auth(client: ApiClient, mode: str, filesystem: str) -> ApiSession:
        scenario.visit("auth-" + mode)
        scenario.auth_headers.append(client.headers)
        return ApiSession(mode, "fixture-key" if mode == "api_key" else None)

    def run_phase(args: E2ePytestArgs, environment: Mapping[str, str]) -> Completed:
        scenario.phase_count += 1
        scenario.visit(args.phase)
        if scenario.failure == "interrupt" and scenario.phase_count == 2:
            raise KeyboardInterrupt
        return Completed(0, "")

    monkeypatch.setattr(e2e, "database_connection", connection)
    monkeypatch.setattr(e2e, "configure_auth", auth)
    monkeypatch.setattr(SyncAssets, "run", sync_assets)
    for tool, method, implementation in (
        (context.tools.sqlx, "create_database", database_create),
        (context.tools.sqlx, "migrate", database_migrate),
        (context.tools.sqlx, "drop_database", database_drop),
        (context.tools.listeners, "require_free", require_free),
        (context.tools.listeners, "require_owner", require_owner),
        (context.tools.cargo, "serving_executable", executable),
        (context.tools.application, "start", start_api),
        (context.tools.trunk, "start", start_ui),
        (context.tools.http, "request", health),
        (context.tools.python, "e2e", run_phase),
    ):
        monkeypatch.setattr(tool, method, implementation)
    return context, scenario


def test_complete_run_authenticates_in_order_and_cleans_owned_resources(
    coordinator: tuple[Context, Scenario],
) -> None:
    context, scenario = coordinator
    e2e.UiE2e.run(context)
    assert [event for event in scenario.events if event.startswith("auth-")] == [
        "auth-none",
        "auth-api_key",
    ]
    assert scenario.events[-3:] == ["stop-ui", "stop-api", "drop-database"]
    results = context.root / "tests/test-results"
    assert not list(results.glob("*stale.json"))
    assert json.loads((results / "python-e2e-summary.json").read_text())["status"] == "passed"


@pytest.mark.parametrize(
    "failure", ("", "api-none-missing-catalog", "catalog-unready", "interrupt")
)
def test_media_restarts_after_reset_and_never_manufactures_catalog_readiness(
    coordinator: tuple[Context, Scenario],
    monkeypatch: pytest.MonkeyPatch,
    failure: str,
) -> None:
    context, scenario = coordinator
    scenario.failure = failure
    context.fs.write(
        context.root / "config/database-rebaseline.env", "TRANSITION_PHASE=feature-development\n"
    )

    @contextmanager
    def database(owner: Context) -> Iterator[str]:
        scenario.visit("create-database")
        try:
            yield "postgres://fixture.invalid/owned"
        finally:
            scenario.visit("drop-database")

    monkeypatch.setattr(e2e, "single_init_database", database)
    if failure:
        with pytest.raises((ToolingError, KeyboardInterrupt)):
            e2e.UiE2e.run(context)
    else:
        e2e.UiE2e.run(context)
        missing = context.root / "tests/.runtime/missing-media-roots.json"
        assert scenario.catalogs == [
            missing,
            missing,
            missing,
            None,
            missing,
            missing,
            missing,
            None,
            None,
        ]
        assert scenario.events.count("catalog-readiness") == 3
        assert [event for event in scenario.events if event.startswith("auth-")] == [
            "auth-none",
            "auth-none",
            "auth-api_key",
            "auth-api_key",
        ]
        assert scenario.events.count("start-api") == scenario.events.count("stop-api") == 9
        assert scenario.auth_headers == [{}, {}, {}, {"x-revaer-api-key": "fixture-key"}]
    assert scenario.events[-1] == "drop-database"
    for index, event in enumerate(scenario.events):
        if event == "auth-none" or event == "auth-api_key":
            assert scenario.events[index + 1] == "stop-api"
            assert scenario.events[index + 2] == "free-7070"
    summary = json.loads((context.root / "tests/test-results/python-e2e-summary.json").read_text())
    assert summary["status"] == ("failed" if failure else "passed")
    assert set(summary["phases"]) == set(context.settings.e2e.phases(media=True))


@pytest.mark.parametrize(
    "failure",
    [
        "free-7070",
        "migrate",
        "start-api",
        "health-7070",
        "start-ui",
        "owner-8080",
        "api-api-key",
        "service-exit",
        "interrupt",
        "stop-ui",
        "drop-database",
    ],
)
def test_failures_keep_cleanup_order_and_failed_summary(
    coordinator: tuple[Context, Scenario],
    failure: str,
) -> None:
    context, scenario = coordinator
    scenario.failure = failure
    with pytest.raises((ToolingError, KeyboardInterrupt)):
        e2e.UiE2e.run(context)
    events = scenario.events
    if "create-database" in events:
        assert events[-1] == "drop-database"
    if "start-api" in events and failure != "start-api":
        assert events.index("stop-api") < events.index("drop-database")
    if "start-ui" in events and failure != "start-ui":
        assert events.index("stop-ui") < events.index("stop-api")
    if failure in ("free-7070", "migrate", "start-api", "health-7070", "start-ui", "owner-8080"):
        assert not any(event.startswith("auth-") for event in events)
    summary = json.loads((context.root / "tests/test-results/python-e2e-summary.json").read_text())
    assert summary["status"] == "failed"
    if failure == "api-api-key":
        assert summary["phases"] == {
            "api-none": "passed",
            "api-api-key": "failed",
            "ui-chromium": "not run",
        }
    # A failed invocation releases its lock, so a subsequent repair can run.
    scenario.failure = ""
    e2e.UiE2e.run(context)
    repaired = json.loads((context.root / "tests/test-results/python-e2e-summary.json").read_text())
    assert repaired["status"] == "passed"


@pytest.mark.parametrize("failure", ["", "suite", "coverage", "interrupt", "archive"])
def test_runbook_archives_the_current_outcome_under_the_suite_lock(
    coordinator: tuple[Context, Scenario], monkeypatch: pytest.MonkeyPatch, failure: str
) -> None:
    context, _ = coordinator
    destination = context.root / "artifacts/runbook"
    context.fs.write(destination / "summary.txt", "runbook=ok\nold marker\n")
    context.fs.write(destination / "logs/old.log", "previous run")

    def suite(owner: Context, paths: e2e.RunPaths) -> None:
        with (
            pytest.raises(ToolingError, match="Another operation"),
            owner.fs.lock(paths.runtime / "e2e.lock"),
        ):
            pytest.fail("Runbook released its lock while the suite was running")
        owner.fs.write(paths.logs / "api.log", "current log")
        owner.fs.write(paths.report / "index.html", "current report")
        owner.fs.write(paths.results / "result.json", "current result")
        owner.fs.write(paths.results / "e2e-state.json", "legacy plaintext session")
        if failure == "archive":
            (paths.logs / "outside-link").symlink_to(context.root / "Cargo.toml")
        if failure == "suite":
            raise ToolingError("injected suite failure")
        if failure == "interrupt":
            raise KeyboardInterrupt

    def coverage(owner: Context) -> TaskResult:
        assert owner.settings.e2e.coverage_directory == context.root / "tests/test-results"
        if failure == "coverage":
            raise ToolingError("injected route-coverage failure")
        return TaskResult()

    monkeypatch.setattr(e2e, "run_suite", suite)
    monkeypatch.setattr(e2e.UiE2eCoverage, "run", coverage)
    if failure:
        with pytest.raises((ToolingError, KeyboardInterrupt)):
            e2e.Runbook.run(context)
    else:
        e2e.Runbook.run(context)
    if failure == "archive":
        assert not (destination / "summary.txt").exists()
    else:
        status = "failed" if failure else "ok"
        assert (destination / "summary.txt").read_text().startswith(f"runbook={status}\n")
        assert (destination / "logs/api.log").read_text() == "current log"
        assert (destination / "playwright-report/index.html").read_text() == "current report"
        assert (destination / "test-results/result.json").read_text() == "current result"
        assert not (destination / "logs/old.log").exists()
        assert not (destination / "test-results/e2e-state.json").exists()
        assert (context.root / "tests/test-results/e2e-state.json").is_file()
