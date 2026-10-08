"""Real watchfiles notifications and native Git ignore decisions."""

import os
import subprocess
import threading
import time
from contextlib import closing
from pathlib import Path

import pytest
from revaer_tooling.development.settings import load_development_settings
from revaer_tooling.development.watching import FileWatcher
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.git import Git
from revaer_tooling.process import ProcessRunner


def git(root: Path) -> Git:
    subprocess.run(
        ["git", "init", "--initial-branch=main", str(root)], check=True, capture_output=True
    )
    return Git("git", ProcessRunner(lambda message: None), root, dict(os.environ))


def test_native_git_ignore_handles_nested_rules_tracked_files_and_literal_names(
    tmp_path: Path,
) -> None:
    repository = git(tmp_path)
    (tmp_path / ".gitignore").write_text("*.tmp\n!keep.tmp\n")
    for name in ("tracked.tmp", "untracked.tmp", "keep.tmp", "space\nname.tmp", "-option.rs"):
        (tmp_path / name).touch()
    subprocess.run(["git", "add", "-f", "tracked.tmp"], cwd=tmp_path, check=True)
    names = ("tracked.tmp", "untracked.tmp", "keep.tmp", "space\nname.tmp", "-option.rs")
    assert repository.ignored(names) == {"untracked.tmp", "space\nname.tmp"}
    assert repository.ignored(()) == frozenset()
    for name in ("../outside", "/absolute", "has\0nul"):
        with pytest.raises(ToolingError, match="paths within"):
            repository.ignored((name,))


def test_native_watcher_filters_outputs_and_detects_create_modify_delete(tmp_path: Path) -> None:
    repository = git(tmp_path)
    (tmp_path / ".gitignore").write_text("ignored/\n")
    (tmp_path / "ignored").mkdir()
    (tmp_path / "target").mkdir()
    (tmp_path / "artifacts").mkdir()
    source = tmp_path / "input.rs"
    watcher = FileWatcher(tmp_path, repository)
    watcher.verify()
    batches: list[frozenset[Path]] = []
    errors: list[BaseException] = []
    ready = threading.Event()
    done = threading.Event()

    def consume() -> None:
        try:
            with closing(watcher.changes()) as iterator:
                for batch in iterator:
                    ready.set()
                    batches.append(batch)
                    if done.is_set():
                        break
        except BaseException as error:
            errors.append(error)

    worker = threading.Thread(target=consume)
    worker.start()
    try:
        assert ready.wait(5)
        for target in (
            tmp_path / "ignored/build.rs",
            tmp_path / "target/build.rs",
            tmp_path / "artifacts/report.rs",
        ):
            target.write_text("generated")
        for content in ("first", "second", None):
            previous = sum(source in batch for batch in batches)
            if content is None:
                source.unlink()
            else:
                source.write_text(content)
            deadline = time.monotonic() + 5
            while (
                sum(source in batch for batch in batches) <= previous
                and time.monotonic() < deadline
            ):
                time.sleep(0.02)
            assert sum(source in batch for batch in batches) > previous
    finally:
        done.set()
        worker.join(timeout=5)
    assert not worker.is_alive()
    assert not errors
    # FSEvents can deliver an already-queued config event after subscription.
    # .gitignore is a legitimate input; generated paths must never be emitted.
    assert set().union(*batches) <= {source, tmp_path / ".gitignore"}


@pytest.mark.parametrize(
    "name,value",
    [("DEV_STARTUP_TIMEOUT", "0"), ("DEV_STARTUP_TIMEOUT", "no"), ("DEV_SKIP_PORT_CHECK", "yes")],
)
def test_development_settings_fail_before_startup(name: str, value: str) -> None:
    with pytest.raises(ToolingError, match=name):
        load_development_settings({name: value})


def test_development_defaults_and_existing_overrides() -> None:
    settings = load_development_settings({})
    assert settings.startup_timeout == 180
    assert not settings.skip_port_check
    assert (settings.api_log, settings.ui_log) == ("debug", "info")
    settings = load_development_settings(
        {"DEV_STARTUP_TIMEOUT": "60", "DEV_SKIP_PORT_CHECK": "1", "RUST_LOG": "trace"}
    )
    assert settings.startup_timeout == 60
    assert settings.skip_port_check
    assert settings.api_log == settings.ui_log == "trace"
