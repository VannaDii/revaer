"""Exercise uv setup and the actual Python quality tools in an isolated project."""

import json
import shutil
import sys
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import main, make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.packages import Apt
from revaer_tooling.external.python import Python
from revaer_tooling.external.rust import Cargo, Rustup
from revaer_tooling.process import Completed, Invocation, Runner, RunningProcess
from revaer_tooling.settings import load_settings
from revaer_tooling.tasks.build import ToolingCheck, ToolingCoverage
from revaer_tooling.tasks.quality import Lock
from revaer_tooling.tasks.setup import Setup


@pytest.fixture
def python_project(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    source = Path(__file__).resolve().parents[2]
    for name in ("pyproject.toml", "uv.lock", ".uv-version", ".python-version"):
        shutil.copy2(source / name, tmp_path / name)
    (tmp_path / ".git").mkdir()
    package = tmp_path / "tools/src/revaer_tooling"
    package.mkdir(parents=True)
    (package / "__init__.py").touch()
    (package / "cli.py").write_text(
        '"""Small coverage fixture."""\n\n\n'
        "def double(value: int) -> int:\n"
        '    if value < 0:\n        raise ValueError("negative")\n'
        "    return value * 2\n"
    )
    shutil.copytree(
        source / "tools/launcher",
        tmp_path / "tools/launcher",
        ignore=shutil.ignore_patterns("__pycache__"),
    )
    tests = tmp_path / "tools/tests"
    tests.mkdir()
    (tmp_path / "tests").mkdir()
    (tmp_path / "tests/__init__.py").touch()
    (tests / "test_value.py").write_text(
        '"""Exercise the fixture."""\n\n'
        "from revaer_tooling.cli import double\n\n\n"
        "def test_positive() -> None:\n    assert double(3) == 6\n"
    )
    # This child project imports its own fixture implementation for the coverage
    # test; the parent continues using the real rv implementation under test.
    manifest = tmp_path / "pyproject.toml"
    manifest.write_text(
        manifest.read_text()
        .replace(
            'pythonpath = ["tools/launcher/src"]',
            'pythonpath = ["tools/src", "tools/launcher/src"]',
        )
        .replace(
            'source = ["tools/src/revaer_tooling", '
            '"tools/launcher/src/revaer_launcher", "tools/tests"]',
            'source = ["tools/src/revaer_tooling", "tools/tests"]',
        )
    )
    monkeypatch.setenv("UV_TOOL_DIR", str(tmp_path / "uv-tools"))
    monkeypatch.setenv("UV_TOOL_BIN_DIR", str(tmp_path / "bin"))
    monkeypatch.chdir(tmp_path)
    return make_context(Options(profile="python"))


def test_setup_uses_uv_installation_and_repeat_sync_is_locked(python_project: Context) -> None:
    context = python_project
    before = (context.root / "uv.lock").read_bytes()
    Setup.run(context)
    assert (context.root / "bin/rv").is_file()
    assert (context.root / "uv-tools/revaer-launcher/uv-receipt.toml").is_file()
    assert (
        Setup.run(
            replace(context, options=replace(context.options, install_launcher=False))
        ).message
        == "Python environment ready"
    )
    assert (context.root / "uv.lock").read_bytes() == before


def test_lock_uses_uv_resolution_and_propagates_an_invalid_manifest(
    python_project: Context,
) -> None:
    lock = python_project.root / "uv.lock"
    before = lock.read_bytes()
    assert Lock.run(python_project).message == "Project dependencies locked with uv"
    assert lock.read_bytes() == before
    manifest = python_project.root / "pyproject.toml"
    manifest.write_text(manifest.read_text().replace('"PyYAML==6.0.3"', '"PyYAML==not-a-version"'))
    with pytest.raises(ToolingError):
        Lock.run(python_project)
    assert lock.read_bytes() == before


def test_quality_task_runs_format_lint_types_and_tests(python_project: Context) -> None:
    assert ToolingCheck.run(python_project).message == "Python tooling checks passed"
    (python_project.root / "tools/tests/test_value.py").write_text("invalid syntax!\n")
    with pytest.raises(ToolingError):
        ToolingCheck.run(python_project)


def test_coverage_floor_fails_with_evidence_then_passes_with_real_branch_coverage(
    python_project: Context,
) -> None:
    with pytest.raises(ToolingError):
        ToolingCoverage.run(python_project)
    assert (python_project.root / "coverage/python.xml").stat().st_size > 0
    assert (python_project.root / "coverage/python.lcov").stat().st_size > 0
    assert (
        "SF:tools/tests/test_value.py" in (python_project.root / "coverage/python.lcov").read_text()
    )
    tests = python_project.root / "tools/tests/test_value.py"
    tests.write_text(
        tests.read_text().replace(
            "from revaer_tooling.cli import double",
            "import pytest\nfrom revaer_tooling.cli import double",
        )
        + "\n\ndef test_negative() -> None:\n"
        + '    with pytest.raises(ValueError, match="negative"):\n        double(-1)\n'
    )
    ToolingCoverage.run(python_project)


def test_cli_reports_failure_as_exit_status(
    python_project: Context, capsys: pytest.CaptureFixture[str]
) -> None:
    assert main(["setup", "--profile", "python", "--no-launcher"]) == 0
    assert "Python environment ready" in capsys.readouterr().out
    (python_project.root / ".uv-version").write_text("0.0.1\n")
    assert main(["setup", "--profile", "python", "--no-launcher"]) == 1
    assert "required" in capsys.readouterr().err


def test_tooling_coverage_cannot_replace_data_while_a_merge_reads_it(
    python_project: Context,
) -> None:
    context = python_project
    report = context.root / "coverage/python.xml"
    context.fs.write(report, "current report")
    with (
        context.fs.lock(report.parent / ".python-coverage.lock"),
        pytest.raises(ToolingError, match="holds"),
    ):
        ToolingCoverage.run(context)
    assert report.read_text() == "current report"


@pytest.mark.parametrize(
    "command", ("ui-e2e", "runbook", "python-coverage-merge", "js-coverage-merge")
)
def test_e2e_reentry_uses_uv_dotenv_precedence_and_keeps_the_lockfile(
    python_project: Context, monkeypatch: pytest.MonkeyPatch, command: str
) -> None:
    root = python_project.root
    before = (root / "uv.lock").read_bytes()
    for name in ("VIRTUAL_ENV", "UV_PROJECT_ENVIRONMENT", "UV_PROJECT"):
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv("RV_FIXTURE_OVERRIDE", "caller")
    (root / "tests/.env").write_text(
        "RV_FIXTURE_OVERRIDE=file\nRV_FIXTURE_TEXT='spaces and # literal text'\n"
        'RV_FIXTURE_EXPANSION="${RV_FIXTURE_OVERRIDE}/expanded"\n'
    )
    # This isolated, installed module observes the child process after uv has
    # parsed dotenv. The parent calls rv's actual CLI reentry boundary.
    (root / "tools/src/revaer_tooling/cli.py").write_text(
        "import json, os, sys\nfrom pathlib import Path\n"
        "Path('observed-env.json').write_text(json.dumps("
        "{'override': os.environ['RV_FIXTURE_OVERRIDE'], "
        "'text': os.environ['RV_FIXTURE_TEXT'], 'expansion': os.environ['RV_FIXTURE_EXPANSION'], "
        "'args': sys.argv[1:]}))\n"
    )
    assert main([command]) == 0
    observed = json.loads((root / "observed-env.json").read_text())
    assert observed == {
        "override": "caller",
        "text": "spaces and # literal text",
        "expansion": "caller/expanded",
        "args": [command, "--e2e-environment-ready"],
    }
    assert (root / "uv.lock").read_bytes() == before


class InstallationBoundary:
    """Record install requests; allow only version probes to reach native tools.

    Package-manager idempotence belongs to Cargo/Rustup. The task test checks
    installation order, selected inputs, and failure propagation without changing
    the developer's compiler, global executables, or browser cache.
    """

    def __init__(self, delegate: Runner, fail_install: bool) -> None:
        self.delegate = delegate
        self.fail_install = fail_install
        self.installs: list[Invocation] = []

    def start(self, invocation: Invocation) -> RunningProcess:
        raise AssertionError("Setup must not launch a background installer")

    def run(self, invocation: Invocation) -> Completed:
        arguments = invocation.argv[1:]
        if arguments == ("--version",):
            return self.delegate.run(invocation)
        if (
            arguments[:2] in (("toolchain", "install"), ("target", "add"))
            or arguments[:1] in (("install",), ("update",))
            or arguments[:3] == ("-m", "playwright", "install")
        ):
            self.installs.append(invocation)
            if self.fail_install and arguments[:1] == ("install",):
                raise ToolingError("Injected package-manager failure")
            return Completed(0, "")
        raise AssertionError(f"Unreviewed setup operation: {arguments}")


@pytest.mark.parametrize("profile", ["dev", "ci"])
@pytest.mark.parametrize("fail_install", [False, True])
def test_native_setup_respects_pins_order_and_package_manager_failures(
    python_project: Context, profile: str, fail_install: bool
) -> None:
    source = Path(__file__).resolve().parents[2]
    shutil.copy2(source / "rust-toolchain.toml", python_project.root / "rust-toolchain.toml")
    # Distinct fixture inputs prove the task reads its own checkout's pins. The
    # installation boundary prevents these values reaching a real package manager.
    (python_project.root / "tools/versions.toml").write_text(
        '[rust]\nudeps_toolchain = "nightly-2000-01-01"\n'
        '[cargo.example-tool]\nversion = "1.2.3"\nfeatures = ["selected"]\n'
        "no_default_features = true\n"
    )
    messages: list[str] = []
    template = python_project.tools.cargo
    runner = InstallationBoundary(template.runner, fail_install)
    tools = replace(
        python_project.tools,
        cargo=Cargo(template.name, runner, template.root, template.environment),
        rustup=Rustup("rustup", runner, template.root, template.environment),
        python=Python(
            python_project.tools.python.name, runner, template.root, template.environment
        ),
    )
    context = replace(
        python_project,
        options=Options(profile=profile, install_launcher=False),
        tools=tools,
        emit=messages.append,
    )
    if fail_install:
        with pytest.raises(ToolingError, match="package-manager failure"):
            Setup.run(context)
        assert len(runner.installs) == 4
        assert not messages
    else:
        assert Setup.run(context).message == "Development tools ready"
        assert len(runner.installs) == 5
        browser_arguments = runner.installs[-1].argv
        assert ("--with-deps" in browser_arguments) is (profile == "ci")
        assert browser_arguments[-3:] == ("chromium", "firefox", "webkit")
    assert runner.installs[0].argv[1:3] == ("toolchain", "install")
    assert runner.installs[1].argv[1:] == ("target", "add", "wasm32-unknown-unknown")
    assert runner.installs[2].argv[1:4] == ("toolchain", "install", "nightly-2000-01-01")
    package_arguments = runner.installs[3].argv
    assert package_arguments[1:] == (
        "install",
        "--locked",
        "--version",
        "1.2.3",
        "example-tool",
        "--features",
        "selected",
        "--no-default-features",
    )
    assert not (context.root / "bin/rv").exists()


def test_doctor_reports_all_missing_tools_and_changes_no_files(python_project: Context) -> None:
    from revaer_tooling.external.base import ExternalTool
    from revaer_tooling.tasks.setup import Doctor

    template = python_project.tools.git
    missing = tuple(
        ExternalTool(name, template.runner, template.root, {"PATH": str(template.root)})
        for name in ("missing-first-tool", "missing-second-tool")
    )
    context = replace(
        python_project, tools=replace(python_project.tools, native_prerequisites=missing)
    )
    before = (context.root / "uv.lock").read_bytes()
    with pytest.raises(ToolingError) as failure:
        Doctor.run(context)
    assert "missing-first-tool" in str(failure.value)
    assert "missing-second-tool" in str(failure.value)
    assert (context.root / "uv.lock").read_bytes() == before
    assert not (context.root / "bin/rv").exists()


def test_cli_handles_signals_and_filesystem_errors_without_tracebacks(
    python_project: Context,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    import signal

    from revaer_tooling.cli import COMMANDS
    from revaer_tooling.context import TaskResult

    def interrupted(context: Context) -> TaskResult:
        signal.raise_signal(signal.SIGTERM)
        raise AssertionError("SIGTERM handler must interrupt dispatch")

    original = signal.getsignal(signal.SIGTERM)
    monkeypatch.setitem(COMMANDS, "doctor", interrupted)
    assert main(["doctor"]) == 130
    assert signal.getsignal(signal.SIGTERM) is original
    assert "interrupted" in capsys.readouterr().err
    (python_project.root / ".uv-version").unlink()
    assert main(["setup", "--profile", "python", "--no-launcher"]) == 1
    assert "Traceback" not in capsys.readouterr().err
    (python_project.root / ".git").rmdir()
    # Temporary test roots can be nested under the real checkout. The CLI must
    # be outside every repository to exercise its missing-checkout error.
    monkeypatch.chdir(python_project.root.anchor)
    assert main(["setup", "--profile", "python"]) == 1
    assert "Run rv from a Revaer checkout" in capsys.readouterr().err


def test_selective_setup_avoids_unrequested_native_installations(python_project: Context) -> None:
    source = Path(__file__).resolve().parents[2]
    for name in ("rust-toolchain.toml", "tools/versions.toml"):
        shutil.copy2(source / name, python_project.root / name)
    template = python_project.tools.rustup
    boundary = InstallationBoundary(template.runner, False)
    tools = replace(
        python_project.tools,
        rustup=Rustup(template.name, boundary, template.root, template.environment),
    )
    context = replace(
        python_project,
        tools=tools,
        options=Options(profile="ci", install_launcher=False, cargo_tools=(), browsers=()),
    )
    assert Setup.run(context).message == "Selected development tools ready"
    assert len(boundary.installs) == 1
    assert boundary.installs[0].argv[1:3] == ("toolchain", "install")


def test_ci_apt_selection_exports_cache_settings_after_installation(
    python_project: Context,
) -> None:
    # Only the apt install is simulated. uv checks the real lock, and the actual
    # command-file writer must preserve the runner's existing environment file.
    template = python_project.tools.python
    boundary = InstallationBoundary(template.runner, False)
    output = python_project.root / "github-environment"
    output.write_text("existing=value\n")
    context = replace(
        python_project,
        options=Options(profile="python", install_launcher=False, apt_profile="base"),
        host=replace(python_project.host, system="linux", home=python_project.root / "host"),
        settings=load_settings({"GITHUB_ENV": str(output)}),
        tools=replace(
            python_project.tools,
            apt=Apt(sys.executable, boundary, template.root, template.environment, None),
        ),
    )
    assert Setup.run(context).message == "Python environment ready"
    assert len(boundary.installs) == 2
    assert output.read_text().startswith("existing=value\nRUSTC_WRAPPER<<")
    assert str(context.host.home / ".cache/sccache") in output.read_text()
    assert (context.host.home / ".cache/sccache").is_dir()


def test_setup_rejects_wrong_platform_and_incompatible_profile_before_installing(
    python_project: Context,
) -> None:
    context = replace(
        python_project,
        host=replace(python_project.host, system="darwin"),
        options=Options(profile="python", apt_packages=("ca-certificates",)),
    )
    with pytest.raises(ToolingError, match="Debian/Ubuntu"):
        Setup.run(context)
    context = replace(context, options=Options(profile="python", browsers=("chromium",)))
    with pytest.raises(ToolingError, match="dev or ci"):
        Setup.run(context)
