"""Real Unix processes prove checkout isolation, orphan recovery and PID safety."""

import json
import os
import secrets
import subprocess
import sys
import time
from dataclasses import replace
from pathlib import Path

import psutil
import pytest
from revaer_tooling.development.processes import ProcessIdentity, ProcessManager
from revaer_tooling.development.session import Session, directory
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.process import Invocation, ProcessRunner, RunningProcess


def manager(root: Path) -> ProcessManager:
    return ProcessManager(
        root, os.getpid(), os.getuid(), time.monotonic, lambda: secrets.token_hex(32)
    )


def child(root: Path, code: str = "import time; time.sleep(60)") -> RunningProcess:
    return ProcessRunner(lambda message: None).start(
        Invocation(
            (sys.executable, "-c", code),
            root,
            {**os.environ, "RV_DEV_SESSION": secrets.token_hex(32)},
            capture=True,
        )
    )


def identified(processes: ProcessManager, running: RunningProcess) -> ProcessIdentity:
    identity = processes.identity(running.pid)
    assert identity is not None
    return identity


def test_stop_preflights_every_checkout_before_signalling(tmp_path: Path) -> None:
    foreign = tmp_path.parent / (tmp_path.name + "-foreign")
    foreign.mkdir()
    parent, worker = child(tmp_path), child(foreign)
    processes = manager(tmp_path)
    try:
        supervisor = identified(processes, parent)
        other = identified(manager(foreign), worker)
        with pytest.raises(ToolingError, match="another user or checkout"):
            processes.stop(supervisor, (other,))
        assert parent.poll() is None and worker.poll() is None
    finally:
        parent.stop()
        worker.stop()


def test_stale_pid_does_not_stop_its_replacement(tmp_path: Path) -> None:
    running = child(tmp_path)
    processes = manager(tmp_path)
    try:
        identity = identified(processes, running)
        stale = replace(identity, created=identity.created - 100)
        assert not processes.members(stale)
        processes.stop(stale, (stale,))
        assert running.poll() is None
    finally:
        running.stop()


def test_owned_group_and_supervisor_stop_without_touching_foreign_process(tmp_path: Path) -> None:
    parent, worker, unrelated = child(tmp_path), child(tmp_path), child(tmp_path)
    processes = manager(tmp_path)
    try:
        supervisor, identity = identified(processes, parent), identified(processes, worker)
        assert {item.pid for item in processes.members(identity)} == {worker.pid}
        processes.stop(supervisor, (identity,))
        assert parent.poll() is not None and worker.poll() is not None
        assert unrelated.poll() is None
    finally:
        for running in (parent, worker, unrelated):
            running.stop()


def test_orphaned_descendant_retains_the_recorded_session(tmp_path: Path) -> None:
    code = (
        "import pathlib, subprocess, sys, time; "
        "p = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'], "
        "stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); "
        "pathlib.Path('child.pid').write_text(str(p.pid)); "
        "\nwhile not pathlib.Path('leave').exists(): time.sleep(0.02)\n"
    )
    running, supervisor = child(tmp_path, code), child(tmp_path)
    processes = manager(tmp_path)
    descendant: psutil.Process | None = None
    try:
        identity, parent = identified(processes, running), identified(processes, supervisor)
        deadline = time.monotonic() + 5
        while not (tmp_path / "child.pid").is_file() and time.monotonic() < deadline:
            time.sleep(0.02)
        descendant = psutil.Process(int((tmp_path / "child.pid").read_text()))
        (tmp_path / "leave").touch()
        assert running.wait().code == 0
        assert descendant.pid in {item.pid for item in processes.members(identity)}
        processes.stop(parent, (identity,))
        assert not descendant.is_running() or descendant.status() == psutil.STATUS_ZOMBIE
    finally:
        if descendant is not None and descendant.is_running():
            descendant.kill()
            descendant.wait(timeout=5)
        running.stop()
        supervisor.stop()


def test_sigterm_resistant_worker_is_killed(tmp_path: Path) -> None:
    running = child(
        tmp_path,
        "import pathlib, signal, time; signal.signal(signal.SIGTERM, signal.SIG_IGN); "
        "pathlib.Path('ready').touch(); time.sleep(60)",
    )
    processes = manager(tmp_path)
    try:
        identity = identified(processes, running)
        deadline = time.monotonic() + 5
        while not (tmp_path / "ready").exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        assert (tmp_path / "ready").exists()
        processes._terminate(processes.members(identity), grace=1)
        assert running.poll() is not None
    finally:
        running.stop()


def test_listener_must_belong_to_recorded_session(tmp_path: Path) -> None:
    running = child(tmp_path)
    processes = manager(tmp_path)
    try:
        identity = identified(processes, running)
        assert not processes.require_listener(frozenset(), identity)
        assert processes.require_listener(frozenset((running.pid,)), identity)
        with pytest.raises(ToolingError, match="unowned"):
            processes.require_listener(frozenset((os.getpid(),)), identity)
    finally:
        running.stop()


@pytest.mark.parametrize("token", ["", "a" * 64])
def test_unknown_session_token_prevents_all_cleanup(tmp_path: Path, token: str) -> None:
    parent, worker = child(tmp_path), child(tmp_path)
    processes = manager(tmp_path)
    try:
        supervisor = identified(processes, parent)
        identity = replace(identified(processes, worker), token=token)
        with pytest.raises(ToolingError, match="token"):
            processes.stop(supervisor, (identity,))
        assert parent.poll() is None and worker.poll() is None
    finally:
        parent.stop()
        worker.stop()


def test_cleanup_refuses_its_own_pid_before_inspection(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.chdir(tmp_path)
    processes = manager(tmp_path)
    own = processes.identity(os.getpid())
    assert own is not None
    with pytest.raises(ToolingError, match="command itself"):
        processes.stop(own, ())
    with pytest.raises(ToolingError, match=r"not an owned session|own session"):
        processes.members(own)


def test_cleanup_refuses_a_worker_that_shares_its_parents_session(tmp_path: Path) -> None:
    running = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(60)"],
        cwd=tmp_path,
        env={**os.environ, "RV_DEV_SESSION": secrets.token_hex(32)},
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=False,
    )
    try:
        processes = manager(tmp_path)
        identity = processes.identity(running.pid)
        assert identity is not None
        with pytest.raises(ToolingError, match="not an owned session"):
            processes.members(identity)
        assert running.poll() is None
    finally:
        running.terminate()
        running.wait(timeout=5)


def test_inspection_failure_is_not_treated_as_absence(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    running = child(tmp_path)
    processes = manager(tmp_path)

    def inaccessible(process: psutil.Process) -> str:
        raise psutil.AccessDenied(process.pid)

    try:
        monkeypatch.setattr(psutil.Process, "cwd", inaccessible)
        with pytest.raises(ToolingError, match="Cannot inspect"):
            processes.identity(running.pid)
        assert running.poll() is None
    finally:
        running.stop()


@pytest.mark.parametrize(
    "pid,created", [(0, 1.0), (-3, 1.0), (7, 0.0), (7, float("nan")), (7, float("inf"))]
)
def test_process_identity_rejects_invalid_values(pid: int, created: float) -> None:
    with pytest.raises(ToolingError, match="identity"):
        ProcessIdentity(pid, created).validate()


def test_receipt_round_trip_is_private_and_checkout_scoped(tmp_path: Path) -> None:
    fs = FileSystem()
    state = directory(tmp_path, fs)
    path = state / "session.json"
    session = Session(tmp_path, ProcessIdentity(10, 100.0), (ProcessIdentity(11, 101.0),))
    session.write(fs, path)
    assert Session.read(fs, path, tmp_path) == session
    assert path.stat().st_mode & 0o777 == 0o600
    with pytest.raises(ToolingError, match="checkout"):
        Session.read(fs, path, tmp_path.parent)


@pytest.mark.parametrize("mutation", ["schema", "extra", "pid", "created", "workers", "size"])
def test_receipt_rejects_malformed_values(tmp_path: Path, mutation: str) -> None:
    fs = FileSystem()
    path = directory(tmp_path, fs) / "session.json"
    Session(tmp_path, ProcessIdentity(10, 100.0), ()).write(fs, path)
    value = json.loads(path.read_text())
    if mutation == "schema":
        value["schema"] = True
    elif mutation == "extra":
        value["extra"] = "unexpected"
    elif mutation in ("pid", "created"):
        value["supervisor"][mutation] = True
    elif mutation == "workers":
        value["workers"] = [value["supervisor"]] * 3
    else:
        value["extra"] = "x" * 4096
    path.write_text(json.dumps(value))
    with pytest.raises(ToolingError):
        Session.read(fs, path, tmp_path)


@pytest.mark.parametrize("link", ["symlink", "hardlink"])
def test_receipt_does_not_follow_links(tmp_path: Path, link: str) -> None:
    fs = FileSystem()
    path = directory(tmp_path, fs) / "session.json"
    source = tmp_path / "original"
    Session(tmp_path, ProcessIdentity(10, 100.0), ()).write(fs, source)
    if link == "symlink":
        path.symlink_to(source)
    else:
        path.hardlink_to(source)
    with pytest.raises(ToolingError, match="unlinked"):
        Session.read(fs, path, tmp_path)


def test_state_does_not_follow_parent_links(tmp_path: Path) -> None:
    (tmp_path / "target").symlink_to(tmp_path.parent, target_is_directory=True)
    with pytest.raises(ToolingError, match="unlinked"):
        directory(tmp_path, FileSystem())
