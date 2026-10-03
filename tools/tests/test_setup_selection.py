"""Setup input validation and privilege/failure boundaries.

The apt process recorder deliberately performs no installation on the developer's
host. The separate disposable Linux proof exercises the actual package manager.
"""

import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest
from revaer_tooling.bootstrap.selection import apt_selection, comma_selection, native_selection
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.base import ExternalTool
from revaer_tooling.external.packages import Apt, AptInstallArgs
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.process import Completed, Invocation, RunningProcess


@pytest.fixture
def configured(tmp_path: Path) -> Path:
    (tmp_path / "tools").mkdir()
    (tmp_path / "rust-toolchain.toml").write_text(
        '[toolchain]\nchannel="1.96.0"\ncomponents=["rustfmt","rust-src"]\n'
    )
    (tmp_path / "tools/versions.toml").write_text(
        '[rust]\nudeps_toolchain="nightly-2026-06-13"\n'
        '[cargo.sqlx-cli]\nversion="0.8.6"\nfeatures=["postgres"]\nno_default_features=true\n'
        '[cargo.trunk]\nversion="0.21.14"\n'
        '[cargo.cargo-udeps]\nversion="0.1.57"\n'
    )
    return tmp_path


def test_selections_preserve_pins_and_install_only_the_requested_capabilities(
    configured: Path,
) -> None:
    fs = FileSystem()
    all_tools = native_selection(fs, configured, None, None)
    assert len(all_tools.cargo) == 3
    assert all_tools.wasm
    assert all_tools.nightly == "nightly-2026-06-13"
    minimal = native_selection(fs, configured, (), ())
    assert minimal.toolchain == all_tools.toolchain
    assert minimal.components == all_tools.components
    assert not minimal.cargo and not minimal.browsers and not minimal.wasm
    assert minimal.nightly is None
    database = native_selection(fs, configured, ("sqlx-cli",), ("firefox",))
    assert database.cargo[0].version == "0.8.6"
    assert database.cargo[0].features == ("postgres",)
    assert database.cargo[0].no_default_features
    assert not database.wasm and database.nightly is None
    assert database.browsers == ("firefox",)
    assert native_selection(fs, configured, ("trunk",), ()).wasm
    assert native_selection(fs, configured, ("cargo-udeps",), ()).nightly == all_tools.nightly
    assert comma_selection(" sqlx-cli, trunk ") == ("sqlx-cli", "trunk")
    assert comma_selection(" ") == ()


@pytest.mark.parametrize("selection", (("unknown",), ("trunk", "trunk"), ("",)))
def test_unknown_or_ambiguous_cargo_selections_fail(
    configured: Path, selection: tuple[str, ...]
) -> None:
    with pytest.raises(ToolingError, match="unique names"):
        native_selection(FileSystem(), configured, selection, ())


@pytest.mark.parametrize("selection", (("Chrome",), ("webkit", "webkit"), ("",)))
def test_unknown_or_ambiguous_browser_selections_fail(
    configured: Path, selection: tuple[str, ...]
) -> None:
    with pytest.raises(ToolingError, match="Browser selection must contain unique"):
        native_selection(FileSystem(), configured, (), selection)


def test_apt_profiles_and_explicit_package_overrides_are_deterministic() -> None:
    assert "postgresql-client" in apt_selection("db", ())
    assert "libdw-dev" in apt_selection("coverage", ())
    assert apt_selection("media", ()) == ("ca-certificates", "curl", "ffmpeg")
    assert apt_selection("base", ("libssl-dev:amd64=3.0.1-1+deb12u1",)) == (
        "libssl-dev:amd64=3.0.1-1+deb12u1",
    )
    assert not apt_selection("", ())
    with pytest.raises(ToolingError, match="Unknown apt"):
        apt_selection("typo", ())


class PackageBoundary:
    def __init__(self, fail_update: bool = False) -> None:
        self.fail_update = fail_update
        self.calls: list[Invocation] = []

    def start(self, invocation: Invocation) -> RunningProcess:
        raise AssertionError("Package setup must not background an installer")

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        if invocation.argv[-1] == "--version":
            return Completed(0, "apt 2.9.34\n")
        if self.fail_update and invocation.argv[-1] == "update":
            raise ToolingError("Repository signature verification failed")
        return Completed(0, "")


@pytest.mark.parametrize("privileged", (False, True))
def test_apt_uses_explicit_privilege_and_stops_after_failed_repository_verification(
    tmp_path: Path, privileged: bool
) -> None:
    boundary = PackageBoundary(fail_update=True)
    privilege = ExternalTool(sys.executable, boundary, tmp_path, {}) if privileged else None
    apt = Apt(sys.executable, boundary, tmp_path, {}, privilege)
    with pytest.raises(ToolingError, match="signature verification"):
        apt.install(AptInstallArgs(("ca-certificates",)))
    assert all("install" not in item.argv for item in boundary.calls)
    boundary.fail_update = False
    boundary.calls.clear()
    apt.install(AptInstallArgs(("ca-certificates",)))
    installation = boundary.calls[-1]
    assert installation.argv[-4:] == ("install", "--yes", "--", "ca-certificates")
    assert ("--non-interactive" in installation.argv) is privileged
    assert ("--preserve-env=DEBIAN_FRONTEND" in installation.argv) is privileged
    assert installation.env["DEBIAN_FRONTEND"] == "noninteractive"


@pytest.mark.parametrize(
    "packages",
    ((), ("--allow-unauthenticated",), ("foo;true",), ("/tmp/file.deb",), ("foo", "foo")),
)
def test_bad_apt_inputs_cannot_reach_a_process(tmp_path: Path, packages: tuple[str, ...]) -> None:
    boundary = PackageBoundary()
    apt = Apt(sys.executable, boundary, tmp_path, {}, None)
    with pytest.raises(ToolingError):
        apt.install(AptInstallArgs(packages))
    assert not boundary.calls


def test_linux_setup_delegates_installation_to_apt(tmp_path: Path) -> None:
    source = Path(__file__).resolve().parents[2]
    root = tmp_path / "checkout"
    root.mkdir()
    for name in (".uv-version", ".python-version", "pyproject.toml", "uv.lock"):
        shutil.copy2(source / name, root / name)
    for name in ("src", "launcher"):
        shutil.copytree(
            source / "tools" / name,
            root / "tools" / name,
            ignore=shutil.ignore_patterns("__pycache__"),
        )
    shutil.copy2(source / "tools/tests/fixtures/linux_setup.py", root / "verify-setup.py")
    (root / ".git").mkdir()
    # This is Astral's official uv 0.12.13 / Python 3.13 / Debian Trixie image.
    # The index digest supports both hosted amd64 and local Apple Silicon.
    image = (
        "ghcr.io/astral-sh/uv@sha256:"
        "c0ba49559fc5622531fd05a5747b52afb49ffa883574bbf8eb719ebd103efb84"
    )
    identifier = subprocess.run(
        [
            "docker",
            "create",
            "--workdir",
            "/work",
            "--mount",
            f"type=bind,src={root},dst=/work",
            "--env",
            "UV_LINK_MODE=copy",
            # Keep Linux packages on the container filesystem. Installing a
            # complete environment through a macOS bind mount can exhaust the
            # VM's shared-filesystem handles. uv owns this alternate location.
            "--env",
            "UV_PROJECT_ENVIRONMENT=/opt/rv-test-venv",
            image,
            "uv",
            "run",
            "--locked",
            "--",
            "python",
            "verify-setup.py",
        ],
        check=True,
        capture_output=True,
        text=True,
        timeout=300,
    ).stdout.strip()
    try:
        with (tmp_path / "setup.log").open("w") as log:
            result = subprocess.run(
                ["docker", "start", "--attach", identifier],
                stdout=log,
                stderr=subprocess.STDOUT,
                timeout=300,
                check=False,
            )
        assert result.returncode == 0, (tmp_path / "setup.log").read_text()
        proof = json.loads((root / "setup-proof.json").read_text())
        assert proof["lock_unchanged"] is True
        assert proof["missing_package_exit"] == 100
    finally:
        subprocess.run(
            ["docker", "rm", "--force", identifier], check=True, capture_output=True, timeout=30
        )
