"""Execute the unchanged root bootstrap with actual uv in disposable checkouts.

The missing-uv cases use the official unmanaged installer. Python environments
and launcher installations belong to each fixture; no user profile, native
installation, or other checkout is changed. The script-coverage task opts these
same tests into kcov, keeping behavior assertions and measured execution together.
"""

import hashlib
import json
import os
import shutil
from pathlib import Path

import pytest
from revaer_tooling.bootstrap.coverage import BOOTSTRAP_CASES
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.external.bash import BootstrapArgs


def run_bootstrap_case(case: str, tmp_path: Path, artifacts: Path | None) -> None:
    source = Path(__file__).resolve().parents[2]
    root = tmp_path / "checkout with spaces"
    root.mkdir()
    for name in ("setup.sh", ".uv-version", ".python-version", "pyproject.toml", "uv.lock"):
        shutil.copy2(source / name, root / name)
    shutil.copytree(
        source / "tools/src", root / "tools/src", ignore=shutil.ignore_patterns("__pycache__")
    )
    shutil.copytree(
        source / "tools/launcher",
        root / "tools/launcher",
        ignore=shutil.ignore_patterns("__pycache__"),
    )
    (root / ".git").mkdir()
    (root / "tests").mkdir()
    before = (root / "uv.lock").read_bytes()
    if case == "stale-lock":
        manifest = root / "pyproject.toml"
        manifest.write_text(manifest.read_text().replace('version = "0.1.0"', 'version = "0.2.0"'))

    environment = dict(os.environ)
    for name in (
        "UV_INSTALL_DIR",
        "UV_UNMANAGED_INSTALL",
        "UV_NO_MODIFY_PATH",
        "BASH_ENV",
        "ENV",
        "SHELLOPTS",
        "BASHOPTS",
        "BASH_XTRACEFD",
    ):
        environment.pop(name, None)
    foreign = tmp_path / "unrelated checkout"
    foreign.mkdir()
    marker = foreign / "keep.txt"
    marker.write_text("unrelated data")
    environment.update(
        UV_TOOL_DIR=str(tmp_path / "uv tools"),
        UV_TOOL_BIN_DIR=str(tmp_path / "tool bin"),
        # These must not redirect setup away from the script's own checkout.
        VIRTUAL_ENV=str(foreign),
        UV_PROJECT_ENVIRONMENT=str(foreign),
        UV_PROJECT=str(foreign),
        UV_WORKING_DIR=str(foreign),
        PYTHONHOME=str(foreign),
        PYTHONPATH=str(foreign),
    )
    install = tmp_path / "uv install"
    if case in ("unmanaged", "install-directory"):
        # setup-uv installs outside the system directories on CI. This creates
        # a genuinely absent-uv PATH without substituting a fake uv executable.
        environment["PATH"] = "/usr/bin:/bin"
        assert shutil.which("uv", path=environment["PATH"]) is None
        environment["UV_UNMANAGED_INSTALL"] = str(install)
        if case == "install-directory":
            # Both are supported options; UV_INSTALL_DIR has precedence.
            environment["UV_UNMANAGED_INSTALL"] = str(tmp_path / "unused install")
            environment["UV_INSTALL_DIR"] = str(install)

    context = make_context(Options())
    measured = artifacts is not None
    artifacts = artifacts or tmp_path / "artifacts"
    context.fs.mkdir(artifacts)
    arguments: tuple[str, ...] = ("--profile", "python")
    if case != "installed":
        arguments += ("--no-launcher",)
    args = BootstrapArgs(
        root / "setup.sh",
        arguments,
        foreign,
        environment,
        artifacts / "bootstrap.log",
        (BOOTSTRAP_CASES[case],),
    )
    if measured:
        bash = context.tools.bash.verify().executable
        result = context.tools.kcov.collect(args, bash, artifacts / "kcov")
    else:
        result = context.tools.bash.bootstrap(args)
    assert (root / "uv.lock").read_bytes() == before
    assert marker.read_text() == "unrelated data"
    assert tuple(foreign.iterdir()) == (marker,)
    output = result.stdout + result.stderr
    if case == "stale-lock":
        assert "lockfile" in output and "--locked" in output
        assert "Python environment ready" not in output
    else:
        assert "Python environment ready" in output
        assert "Traceback" not in output
        assert (root / ".venv/pyvenv.cfg").is_file()
        if case == "installed":
            assert (tmp_path / "tool bin/rv").is_file()
            assert (tmp_path / "uv tools/revaer-launcher/uv-receipt.toml").is_file()
        else:
            assert (install / "uv").is_file()
            assert not (tmp_path / "unused install").exists()
    if measured:
        assert (root / "setup.sh").read_bytes() == (source / "setup.sh").read_bytes()
        context.fs.write(
            artifacts / "source.json",
            json.dumps(
                {
                    "version": 1,
                    "source": str((root / "setup.sh").resolve()),
                    "sha256": hashlib.sha256((root / "setup.sh").read_bytes()).hexdigest(),
                    "exit_code": result.code,
                }
            )
            + "\n",
            0o600,
        )


@pytest.mark.parametrize("case", BOOTSTRAP_CASES)
def test_official_uv_bootstrap(case: str, tmp_path: Path) -> None:
    run_bootstrap_case(case, tmp_path, None)


@pytest.mark.parametrize("case", BOOTSTRAP_CASES)
def test_measured_uv_bootstrap(case: str, tmp_path: Path) -> None:
    output = os.environ.get("REVAER_BOOTSTRAP_COVERAGE_DIR")
    artifacts = Path(output) / case if output else tmp_path / "kcov evidence"
    run_bootstrap_case(case, tmp_path, artifacts)
