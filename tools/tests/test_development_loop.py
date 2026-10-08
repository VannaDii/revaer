"""Failure injection around the development coordinator's owned native boundary."""

from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass, field, replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options, TaskResult
from revaer_tooling.development.loop import DevelopmentLoop
from revaer_tooling.development.processes import ProcessIdentity
from revaer_tooling.development.session import Session, directory
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.base import ToolVersion
from revaer_tooling.external.rust import DevelopmentArgs
from revaer_tooling.process import Completed, RunningProcess
from revaer_tooling.settings import load_settings
from revaer_tooling.tasks import development
from revaer_tooling.tasks.build import SyncAssets
from revaer_tooling.tasks.development import Development, Zombies


@dataclass
class Child:
    pid: int
    code: int | None = None
    stopped: bool = False
    stop_failure: bool = False

    def poll(self) -> int | None:
        return self.code

    def wait(self) -> Completed:
        if self.code is None:
            raise AssertionError("A running service must not block the watcher")
        if self.code:
            raise CommandError("fixture compilation failed", self.code)
        return Completed(0, "")

    def stop(self) -> None:
        self.stopped = True
        if self.stop_failure:
            raise ToolingError("fixture cleanup failure")


@dataclass
class Scenario:
    children: list[Child] = field(default_factory=list)
    arguments: list[DevelopmentArgs] = field(default_factory=list)
    messages: list[str] = field(default_factory=list)
    events: list[str] = field(default_factory=list)
    seconds: float = 1.0
    failure: str = ""
    listeners: bool = True

    def start(self, args: DevelopmentArgs) -> RunningProcess:
        if self.failure == "start-ui" and self.children:
            raise ToolingError("fixture Trunk startup failure")
        process = Child(100 + len(self.children))
        self.children.append(process)
        self.arguments.append(args)
        return process


@pytest.fixture
def coordinator(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Context, Scenario]:
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    (tmp_path / "tools/versions.toml").write_text('[cargo.trunk]\nversion="0.21.14"\n')
    monkeypatch.chdir(tmp_path)
    scenario = Scenario()
    context = replace(
        make_context(Options()), settings=load_settings({}), emit=scenario.messages.append
    )
    monkeypatch.setattr(context.tools.processes, "clock", lambda: scenario.seconds)
    monkeypatch.setattr(context.tools.processes, "stop_workers", lambda identities: None)
    monkeypatch.setattr(
        context.tools.processes, "identity", lambda pid: ProcessIdentity(pid, 100.0)
    )
    monkeypatch.setattr(
        context.tools.processes, "require_listener", lambda owners, identity: bool(owners)
    )
    monkeypatch.setattr(
        context.tools.listeners,
        "owners",
        lambda port: frozenset((123,)) if scenario.listeners else frozenset(),
    )
    monkeypatch.setattr(context.tools.cargo, "development", scenario.start)
    monkeypatch.setattr(context.tools.trunk, "development", scenario.start)

    def changes() -> Iterator[frozenset[Path]]:
        yield frozenset()
        yield frozenset((tmp_path / "input.rs",))
        raise KeyboardInterrupt

    monkeypatch.setattr(context.tools.watcher, "changes", changes)
    return context, scenario


def run(context: Context) -> None:
    DevelopmentLoop(
        context, "postgres://fixture.invalid/revaer", directory(context.root, context.fs)
    ).run()


def test_restart_retires_previous_api_and_preserves_separate_logs(
    coordinator: tuple[Context, Scenario],
) -> None:
    context, scenario = coordinator
    with pytest.raises(KeyboardInterrupt):
        run(context)
    assert len(scenario.children) == 3
    assert all(child.stopped for child in scenario.children)
    assert [args.log_path.name for args in scenario.arguments] == [
        "api-1.log",
        "trunk-serve.log",
        "api-2.log",
    ]
    assert [args.rust_log for args in scenario.arguments] == ["debug", "info", "debug"]
    assert all(
        args.database_url == "postgres://fixture.invalid/revaer" for args in scenario.arguments
    )
    assert not (context.root / "target/rv-dev/session.json").exists()
    assert any("ready on port 7070" in message for message in scenario.messages)


@pytest.mark.parametrize("exit_code", [0, 101])
def test_failed_or_exited_api_waits_for_an_edit_then_recovers(
    coordinator: tuple[Context, Scenario],
    monkeypatch: pytest.MonkeyPatch,
    exit_code: int,
) -> None:
    context, scenario = coordinator

    def changes() -> Iterator[frozenset[Path]]:
        scenario.children[0].code = exit_code
        yield frozenset()
        assert scenario.children[0].stopped
        scenario.seconds = 10000  # An ended API does not spin or expire while awaiting an edit.
        yield frozenset()
        yield frozenset((context.root / "fixed.rs",))
        assert len(scenario.children) == 3
        assert not scenario.children[2].stopped
        raise KeyboardInterrupt

    monkeypatch.setattr(context.tools.watcher, "changes", changes)
    with pytest.raises(KeyboardInterrupt):
        run(context)
    assert all(item.stopped for item in scenario.children)
    assert any("waiting for an edit" in message for message in scenario.messages)


@pytest.mark.parametrize(
    "failure", ["start-ui", "ui-exit", "timeout", "watch-error", "watch-end", "identity"]
)
def test_partial_startup_and_watcher_failures_stop_every_started_child(
    coordinator: tuple[Context, Scenario],
    monkeypatch: pytest.MonkeyPatch,
    failure: str,
) -> None:
    context, scenario = coordinator
    scenario.failure = failure

    def changes() -> Iterator[frozenset[Path]]:
        if failure == "ui-exit":
            scenario.children[1].code = 0
        if failure == "timeout":
            scenario.seconds = 10000
            scenario.listeners = False
        if failure == "watch-error":
            raise ToolingError("fixture watcher failure")
        if failure != "watch-end":
            yield frozenset()

    monkeypatch.setattr(context.tools.watcher, "changes", changes)
    if failure == "identity":

        def identify(pid: int) -> ProcessIdentity:
            if pid == 100:
                raise ToolingError("fixture identity failure")
            return ProcessIdentity(pid, 100.0)

        monkeypatch.setattr(context.tools.processes, "identity", identify)
    with pytest.raises(ToolingError):
        run(context)
    assert scenario.children
    assert all(child.stopped for child in scenario.children)
    assert not (context.root / "target/rv-dev/session.json").exists()


def test_cleanup_failure_retains_receipt_and_still_stops_other_child(
    coordinator: tuple[Context, Scenario],
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    context, scenario = coordinator

    def changes() -> Iterator[frozenset[Path]]:
        scenario.children[1].stop_failure = True
        yield frozenset()
        raise KeyboardInterrupt

    monkeypatch.setattr(context.tools.watcher, "changes", changes)
    with pytest.raises(ToolingError, match="cleanup failed"):
        run(context)
    assert all(child.stopped for child in scenario.children)
    assert (context.root / "target/rv-dev/session.json").is_file()


def test_development_task_orders_preflight_assets_database_and_cleanup(
    coordinator: tuple[Context, Scenario],
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    context, scenario = coordinator
    monkeypatch.setattr(
        context.tools.trunk, "verify", lambda version: ToolVersion(Path("trunk"), version)
    )
    monkeypatch.setattr(
        context.tools.rustup, "target_add", lambda target: scenario.events.append(target)
    )
    monkeypatch.setattr(
        context.tools.listeners, "require_free", lambda port: scenario.events.append(f"port-{port}")
    )
    monkeypatch.setattr(SyncAssets, "run", lambda context: TaskResult())

    @contextmanager
    def database(owner: Context) -> Iterator[str]:
        scenario.events.append("database-lock")
        try:
            yield "postgres://fixture.invalid/revaer"
        finally:
            assert all(child.stopped for child in scenario.children)
            scenario.events.append("database-unlock")

    monkeypatch.setattr(development, "prepared_database", database)
    with pytest.raises(KeyboardInterrupt):
        Development.run(context)
    assert scenario.events == [
        "port-7070",
        "port-8080",
        "wasm32-unknown-unknown",
        "database-lock",
        "database-unlock",
    ]


def test_stale_receipt_prevents_new_development_before_external_operations(
    coordinator: tuple[Context, Scenario],
) -> None:
    context, scenario = coordinator
    state = directory(context.root, context.fs)
    (state / "session.json").write_text("stale")
    with pytest.raises(ToolingError, match="rv zombies"):
        Development.run(context)
    assert not scenario.children


def test_zombies_only_uses_the_selected_receipt(
    coordinator: tuple[Context, Scenario],
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    context, scenario = coordinator
    assert "No recorded" in Zombies.run(context).message
    state = directory(context.root, context.fs)
    session = Session(context.root, ProcessIdentity(10, 100.0), (ProcessIdentity(20, 200.0),))
    session.write(context.fs, state / "session.json")

    def stop(supervisor: ProcessIdentity, workers: tuple[ProcessIdentity, ...]) -> None:
        assert supervisor == session.supervisor
        assert workers == session.workers
        scenario.events.append("stop-owned")

    monkeypatch.setattr(context.tools.processes, "stop", stop)
    assert "stopped" in Zombies.run(context).message
    assert scenario.events == ["stop-owned"]
    assert not (state / "session.json").exists()
