"""Official uv child limits and fail-closed kcov diagnostics."""

import json
import os
import resource
import sys
from pathlib import Path

import pytest
from revaer_tooling.bootstrap.settings import load_kcov_settings
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.bash import BootstrapArgs, Kcov
from revaer_tooling.external.python import Uv
from revaer_tooling.process import Completed, Invocation, ProcessRunner, RunningProcess


@pytest.mark.parametrize("soft", (resource.RLIM_INFINITY, 8192, 4096, 16))
def test_kcov_limits_only_its_child_and_preserves_the_hard_limit(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    soft: int,
) -> None:
    limits: list[tuple[int, int]] = []
    monkeypatch.setattr(resource, "getrlimit", lambda _: (soft, 65536))
    monkeypatch.setattr(resource, "setrlimit", lambda _, value: limits.append(value))
    runner = DiagnosticRunner("")
    uv = Uv(sys.executable, runner, tmp_path, {})
    adapter = Kcov(sys.executable, runner, tmp_path, {}, Path(sys.executable), uv)
    args = BootstrapArgs(tmp_path / "setup.sh", (), tmp_path, {}, tmp_path / "log")
    adapter.collect(args, Path("/bin/bash"), tmp_path / "coverage")
    expected = 4096 if soft == resource.RLIM_INFINITY else min(soft, 4096)
    assert runner.calls[-1].env["UV_RUN_RLIMIT_NOFILE"] == str(expected)
    assert "--no-project" in runner.calls[-1].argv
    assert not limits  # Only uv's child changes its limit; pytest never does.


def test_invalid_kcov_process_inputs_fail_before_exec(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    runner = DiagnosticRunner("")
    uv = Uv(sys.executable, runner, tmp_path, {})
    for arguments, python, limit in (
        ((), Path(sys.executable), 4096),
        (("relative-command",), Path(sys.executable), 4096),
        ((sys.executable,), Path("relative-python"), 4096),
        ((sys.executable,), Path(sys.executable), 0),
    ):
        with pytest.raises(ToolingError, match="absolute tools"):
            uv.limited_command(Invocation(arguments, tmp_path, {}), python, limit)
    assert not runner.calls
    monkeypatch.setattr(resource, "getrlimit", lambda _: (15, 65536))
    adapter = Kcov(sys.executable, runner, tmp_path, {}, Path(sys.executable), uv)
    with pytest.raises(ToolingError, match="at least 16"):
        adapter.collect(
            BootstrapArgs(tmp_path / "setup.sh", (), tmp_path, {}, tmp_path / "log"),
            Path("/bin/bash"),
            tmp_path / "coverage",
        )


def test_actual_uv_preserves_parent_limits_and_foreign_environment(tmp_path: Path) -> None:
    before = resource.getrlimit(resource.RLIMIT_NOFILE)
    script = tmp_path / "probe.py"
    script.write_text(
        "import json, os, resource, sys\n"
        "print(json.dumps({'limits': resource.getrlimit(resource.RLIMIT_NOFILE), "
        "'argument': sys.argv[1], 'foreign': os.environ['UV_PROJECT']}))\n"
    )
    foreign = tmp_path / "foreign"
    foreign.mkdir()
    (foreign / "keep").write_text("original")
    environment = dict(os.environ)
    for key in (
        "UV_PROJECT",
        "UV_PROJECT_ENVIRONMENT",
        "UV_WORKING_DIR",
        "UV_PYTHON",
        "VIRTUAL_ENV",
        "PYTHONHOME",
        "PYTHONPATH",
    ):
        environment[key] = str(foreign)
    uv = Uv("uv", ProcessRunner(lambda _: None), tmp_path, dict(os.environ))
    command = Invocation(
        (sys.executable, "-I", str(script), "literal $(argument)"),
        foreign,
        environment,
        capture=True,
        log_path=tmp_path / "uv.log",
    )
    result = uv.limited_command(command, Path(sys.executable), 4096)
    assert json.loads(result.stdout) == {
        "limits": [4096, before[1]],
        "argument": "literal $(argument)",
        "foreign": str(foreign),
    }
    assert resource.getrlimit(resource.RLIMIT_NOFILE) == before
    assert sorted(path.name for path in foreign.iterdir()) == ["keep"]


class DiagnosticRunner:
    def __init__(self, output: str) -> None:
        self.output = output
        self.calls: list[Invocation] = []

    def start(self, invocation: Invocation) -> RunningProcess:
        raise AssertionError("Unexpected background collection")

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        return Completed(0, "kcov 43\n" if invocation.argv[1:] == ("--version",) else self.output)


@pytest.mark.parametrize(
    "message",
    (
        "kcov: error: incomplete trace",
        "kcov: warning: missing input",
        "Failed to exchange stderr for pipe: Bad file descriptor",
        "Failed to execute script",
    ),
)
def test_zero_exit_with_a_kcov_diagnostic_is_still_a_failure(tmp_path: Path, message: str) -> None:
    runner = DiagnosticRunner(message)
    uv = Uv(sys.executable, runner, tmp_path, {})
    adapter = Kcov(sys.executable, runner, tmp_path, {}, Path(sys.executable), uv)
    args = BootstrapArgs(tmp_path / "setup.sh", (), tmp_path, {}, tmp_path / "log")
    with pytest.raises(ToolingError, match="error or warning"):
        adapter.collect(args, Path("/bin/bash"), tmp_path / "coverage")


@pytest.mark.parametrize(
    "url",
    (
        "http://example.invalid/kcov",
        "https://user:password@example.invalid/kcov",
        "https://example.invalid/kcov?pin=other",
        "https://example.invalid/kcov#other",
    ),
)
def test_kcov_distribution_overrides_require_a_plain_https_directory(url: str) -> None:
    with pytest.raises(ToolingError, match="HTTPS distribution"):
        load_kcov_settings({"REVAER_KCOV_ARCHIVES_URL": url})
