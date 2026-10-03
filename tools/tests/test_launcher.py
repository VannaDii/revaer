"""Exercise uv's real installation and dispatch, without touching user tools.

Tiny dependency-free projects expose which worktree ran. They use the same uv
build backend and source layout as Revaer, so these checks cover package entry
points and environment selection rather than only inspecting an argument list.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
import revaer_launcher
from revaer_launcher import command


def checked(argv: list[str], environment: dict[str, str]) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        argv, env=environment, capture_output=True, text=True, timeout=60, check=False
    )
    assert result.returncode == 0, result.stdout + result.stderr
    return result


@pytest.fixture
def installed(tmp_path: Path) -> tuple[Path, dict[str, str], str]:
    uv = shutil.which("uv")
    if uv is None:
        pytest.fail("Run tooling tests through uv run; uv must be available on PATH")
    environment = {
        **os.environ,
        "UV_TOOL_DIR": str(tmp_path / "tools"),
        "UV_TOOL_BIN_DIR": str(tmp_path / "bin"),
        "UV_OFFLINE": "1",
    }
    for name in ("VIRTUAL_ENV", "UV_PROJECT_ENVIRONMENT", "PYTHONPATH", "PYTHONHOME"):
        environment.pop(name, None)
    original = Path(__file__).resolve().parents[1] / "launcher"
    source = tmp_path / "install source"
    shutil.copytree(original, source, ignore=shutil.ignore_patterns("__pycache__"))
    checked(
        [uv, "tool", "install", "--python", sys.executable, "--reinstall", str(source)],
        environment,
    )
    # A user-level launcher must survive deletion of its original worktree.
    shutil.rmtree(source)
    return tmp_path / "bin/rv", environment, uv


def project(root: Path, uv: str, environment: dict[str, str]) -> None:
    package = root / "tools/src/revaer_tooling"
    package.mkdir(parents=True)
    (root / ".git").write_text("gitdir: test-worktree\n")
    (package / "__init__.py").touch()
    (root / ".python-version").write_text("3.13.12\n")
    (root / "pyproject.toml").write_text(
        '[build-system]\nrequires = ["uv_build==0.12.13"]\nbuild-backend = "uv_build"\n'
        '[project]\nname = "revaer-tooling"\nversion = "0.1.0"\n'
        'requires-python = ">=3.13,<3.14"\ndependencies = []\n'
        '[tool.uv.build-backend]\nmodule-root = "tools/src"\n'
    )
    (package / "cli.py").write_text(
        "import json, pathlib, sys\n"
        "print(json.dumps({'root': str(pathlib.Path.cwd()), 'prefix': sys.prefix, "
        "'args': sys.argv[1:]}))\n"
        "sys.exit(17 if '--fail' in sys.argv else 0)\n"
    )
    checked([uv, "lock", "--project", str(root)], environment)


def test_installed_launcher_selects_worktree_and_preserves_arguments(
    installed: tuple[Path, dict[str, str], str], tmp_path: Path
) -> None:
    executable, environment, uv = installed
    first, second = tmp_path / "first worktree", tmp_path / "second worktree"
    for root in (first, second):
        project(root, uv, environment)
    # Simulate a caller that has activated the other checkout's environment.
    environment.update(
        VIRTUAL_ENV=str(first / ".venv"),
        UV_PROJECT_ENVIRONMENT=str(first / ".venv"),
        UV_PROJECT=str(first),
    )
    arguments = ["literal space", "$(do-not-execute)", "--option=quoted'value"]
    for root in (first, second):
        result = subprocess.run(
            [str(executable), *arguments],
            cwd=root / "tools",
            env=environment,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        assert result.returncode == 0, result.stderr
        observed = json.loads(result.stdout)
        assert Path(observed["root"]).resolve() == root.resolve()
        assert Path(observed["prefix"]).resolve() == (root / ".venv").resolve()
        assert observed["args"] == arguments
    result = subprocess.run(
        [str(executable), "--fail"],
        cwd=second,
        env=environment,
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )
    assert result.returncode == 17


def test_installed_launcher_rejects_stale_lock_and_nested_repository(
    installed: tuple[Path, dict[str, str], str], tmp_path: Path
) -> None:
    executable, environment, uv = installed
    root = tmp_path / "checkout"
    project(root, uv, environment)
    manifest = root / "pyproject.toml"
    manifest.write_text(manifest.read_text().replace('version = "0.1.0"', 'version = "0.2.0"'))
    before = (root / "uv.lock").read_bytes()
    result = subprocess.run(
        [str(executable)],
        cwd=root,
        env=environment,
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )
    assert result.returncode != 0
    assert "lockfile" in result.stderr
    assert (root / "uv.lock").read_bytes() == before
    nested = root / "unrelated"
    nested.mkdir()
    (nested / ".git").mkdir()
    with pytest.raises(RuntimeError, match="no rv implementation"):
        command(nested, [], uv)
    # TMPDIR may live inside the source checkout (for container bind mounts).
    # Start at the filesystem root to exercise the absence of every ancestor.
    with pytest.raises(RuntimeError, match="Revaer checkout"):
        command(Path(tmp_path.anchor), [], uv)


def test_uv_install_refuses_unrelated_executable(
    installed: tuple[Path, dict[str, str], str], tmp_path: Path
) -> None:
    _, environment, uv = installed
    unrelated = tmp_path / "unrelated bin"
    unrelated.mkdir()
    executable = unrelated / "rv"
    executable.write_text("unrelated executable\n")
    environment.update(
        UV_TOOL_DIR=str(tmp_path / "new tool directory"), UV_TOOL_BIN_DIR=str(unrelated)
    )
    source = Path(__file__).resolve().parents[1] / "launcher"
    result = subprocess.run(
        [uv, "tool", "install", "--python", sys.executable, "--reinstall", str(source)],
        env=environment,
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )
    assert result.returncode != 0
    assert "already exist" in result.stderr
    assert executable.read_text() == "unrelated executable\n"


def test_main_forwards_to_uv_without_inheriting_another_project(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    (tmp_path / ".git").mkdir()
    package = tmp_path / "tools/src/revaer_tooling"
    package.mkdir(parents=True)
    (package / "cli.py").touch()
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("UV_PROJECT_ENVIRONMENT", "/another/checkout")
    monkeypatch.setenv("UV_CACHE_DIR", "/shared/cache")
    monkeypatch.setattr(sys, "argv", ["rv", "check"])
    uv = shutil.which("uv")
    assert uv is not None

    def replaced(executable: str, arguments: list[str], environment: dict[str, str]) -> None:
        assert executable == uv
        assert arguments[-1] == "check"
        assert "UV_PROJECT_ENVIRONMENT" not in environment
        assert environment["UV_CACHE_DIR"] == "/shared/cache"
        raise SystemExit(19)

    monkeypatch.setattr(os, "execve", replaced)
    with pytest.raises(SystemExit) as result:
        revaer_launcher.main()
    assert result.value.code == 19


def test_missing_uv_is_reported_without_a_traceback(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setenv("PATH", str(tmp_path))
    assert revaer_launcher.main() == 1
    assert "uv is missing" in capsys.readouterr().err
