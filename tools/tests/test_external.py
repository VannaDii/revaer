"""Tool adapters emit exact typed operations without shell interpretation."""

import json
import os
import sys
from pathlib import Path

import pytest
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.base import ExternalTool
from revaer_tooling.external.coverage import CargoLlvmCov
from revaer_tooling.external.github import GitHub
from revaer_tooling.external.media import Ffprobe
from revaer_tooling.external.mount import MkfsExt4, Mount, Unmount
from revaer_tooling.external.python import Python
from revaer_tooling.external.release import ReleaseRepository, SemanticRelease
from revaer_tooling.external.rust import Cargo, CargoArgs, CargoInstallArgs, CargoOperation
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.process import Completed, Invocation, RunningProcess


class RecordingRunner:
    def __init__(self, stdout: str = "") -> None:
        self.calls: list[Invocation] = []
        self.stdout = stdout

    def start(self, invocation: Invocation) -> RunningProcess:
        raise AssertionError("Unexpected background process")

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        return Completed(0, self.stdout)


def test_ext4_fixture_preserves_private_image_and_explicit_mount_lifecycle(tmp_path: Path) -> None:
    runner = RecordingRunner()
    fs = FileSystem()
    image = tmp_path / "media.ext4"
    root = tmp_path / "roots"
    fs.create_disk_image(image, 1024 * 1024)
    root.mkdir(mode=0o700)
    uid, gid = os.getuid(), os.getgid()
    formatter = MkfsExt4(sys.executable, runner, tmp_path, {})
    formatter.image(image, uid, gid)
    assert runner.calls[-1].argv[1:] == ("-q", "-F", "-E", f"root_owner={uid}:{gid}", str(image))
    Mount(sys.executable, runner, tmp_path, {}).temporary(root, image, uid, gid, None)
    assert runner.calls[-1].argv[1:] == (
        "-t",
        "ext4",
        "-o",
        "loop,nodev,nosuid,noexec",
        str(image),
        str(root),
    )
    Unmount(sys.executable, runner, tmp_path, {}).temporary(root, None)
    assert runner.calls[-1].argv[1:] == ("--", str(root))
    image.chmod(0o644)
    with pytest.raises(ToolingError, match="private owned backing file"):
        formatter.image(image, uid, gid)
    assert len(runner.calls) == 3
    with pytest.raises(FileExistsError):
        fs.create_disk_image(image, 1024)
    assert image.stat().st_size == 1024 * 1024
    with pytest.raises(ToolingError, match="positive"):
        fs.create_disk_image(tmp_path / "invalid.ext4", 0)


def test_container_ffprobe_reads_only_the_selected_checkout(tmp_path: Path) -> None:
    runner = RecordingRunner("ffprobe version 8.0.1\n")
    tool = Ffprobe(sys.executable, runner, tmp_path, {}, container="revaer-fixture-ffprobe")
    assert tool.report() == b"ffprobe version 8.0.1\n"
    tool.streams(tmp_path / "test-fixtures/source/input.mkv")
    assert runner.calls[-1].argv[-1] == "/workspace/test-fixtures/source/input.mkv"
    assert runner.calls[-1].argv[1:4] == ("exec", "revaer-fixture-ffprobe", "ffprobe")
    with pytest.raises(ToolingError, match="selected checkout"):
        tool.streams(tmp_path.parent / "outside.mkv")


@pytest.mark.parametrize("newline", ("\n", "\r\n"))
def test_github_release_headers_accept_native_line_endings(tmp_path: Path, newline: str) -> None:
    response = newline.join(("HTTP/2.0 200 OK", "Content-Type: application/json", "", ""))
    response += '{\n"tag_name": "v1.2.3", "draft": false, "assets": []\n}\n'
    runner = RecordingRunner(response)
    tool = GitHub(sys.executable, runner, tmp_path, {})
    tool.check_existing_assets(ReleaseRepository("github.com", "example/revaer"), "v1.2.3", ())
    assert len(runner.calls) == 1


@pytest.mark.parametrize("response", ("HTTP/2.0 200 OK\r\n", "HTTP/2.0 500 Failed\r\n\r\n{}"))
def test_github_incomplete_or_failed_response_cannot_establish_release_state(
    tmp_path: Path,
    response: str,
) -> None:
    tool = GitHub(sys.executable, RecordingRunner(response), tmp_path, {})
    with pytest.raises(ToolingError, match="Cannot inspect"):
        tool.check_existing_assets(ReleaseRepository("github.com", "example/revaer"), "v1.2.3", ())


@pytest.mark.parametrize(
    "key,flags,separator",
    [
        ("CARGO_ENCODED_RUSTFLAGS", "--remap-path-prefix\x1f/path with spaces=/mapped", "\x1f"),
        ("RUSTFLAGS", "--cfg feature_test", " "),
    ],
)
def test_coverage_preserves_compiler_arguments_and_appends_the_warning_gate(
    tmp_path: Path,
    key: str,
    flags: str,
    separator: str,
) -> None:
    runner = RecordingRunner()
    tool = CargoLlvmCov(sys.executable, runner, tmp_path, {key: flags})
    tool.collect({})
    assert runner.calls[0].env[key] == flags + separator + "-Dwarnings"


def test_cargo_build_has_locked_complete_workspace(tmp_path: Path) -> None:
    runner = RecordingRunner()
    cargo = Cargo(sys.executable, runner, tmp_path, {})
    cargo.execute(CargoArgs(CargoOperation.BUILD, all_targets=True, release=True))
    assert runner.calls[0].argv[1:] == (
        "--config",
        'build.rustflags=["-Dwarnings"]',
        "build",
        "--locked",
        "--workspace",
        "--all-features",
        "--all-targets",
        "--release",
    )


def test_cargo_minimal_features_does_not_enable_all_features(tmp_path: Path) -> None:
    runner = RecordingRunner()
    cargo = Cargo(sys.executable, runner, tmp_path, {})
    cargo.execute(
        CargoArgs(
            CargoOperation.TEST, ("revaer-api",), all_features=False, no_default_features=True
        )
    )
    assert "--all-features" not in runner.calls[0].argv
    assert "--no-default-features" in runner.calls[0].argv
    assert "--workspace" not in runner.calls[0].argv


def test_native_test_serialization_is_explicit_and_invalid_combinations_fail(
    tmp_path: Path,
) -> None:
    runner = RecordingRunner()
    cargo = Cargo(sys.executable, runner, tmp_path, {})
    cargo.execute(CargoArgs(CargoOperation.TEST, test_threads=1))
    assert runner.calls[0].argv[-2:] == ("--", "--test-threads=1")
    with pytest.raises(ToolingError, match="Test threads"):
        cargo.execute(CargoArgs(CargoOperation.BUILD, test_threads=1))
    assert len(runner.calls) == 1


def test_tool_requires_exact_configured_version(tmp_path: Path) -> None:
    runner = RecordingRunner("example 1.2.3\n")
    tool = ExternalTool(sys.executable, runner, tmp_path, {})
    assert tool.verify("1.2.3").version == "1.2.3"
    with pytest.raises(ToolingError, match="required"):
        tool.verify("1.2.4")


@pytest.mark.parametrize("output", ["tool v0.5.4", "v0.5.4", 'Version: "v0.5.4"'])
def test_v_prefix_does_not_drop_the_major_version(tmp_path: Path, output: str) -> None:
    tool = ExternalTool(sys.executable, RecordingRunner(output), tmp_path, {})
    assert tool.verify().version == "0.5.4"


def test_missing_executable_is_actionable(tmp_path: Path) -> None:
    tool = ExternalTool(
        "missing-revaer-test-tool", RecordingRunner(), tmp_path, {"PATH": str(tmp_path)}
    )
    with pytest.raises(ToolingError, match="rv setup"):
        tool.locate()


def test_cargo_owns_installation_tracking_and_collision_checks(tmp_path: Path) -> None:
    runner = RecordingRunner()
    cargo = Cargo(sys.executable, runner, tmp_path, {})
    cargo.install(CargoInstallArgs("sqlx-cli", "0.8.6", ("postgres",), True))
    call = runner.calls[0].argv
    assert "--force" not in call
    assert "--locked" in call
    assert call[call.index("--version") + 1] == "0.8.6"
    assert call[call.index("--features") + 1] == "postgres"
    assert "--no-default-features" in call


@pytest.mark.parametrize(
    "url",
    [
        "https://github.com/example/revaer.git",
        "git@github.com:example/revaer.git",
        "ssh://git@github.com/example/revaer.git",
    ],
)
def test_release_destination_uses_the_engines_supported_git_url_parser(
    tmp_path: Path, url: str
) -> None:
    engine = SemanticRelease(sys.executable, RecordingRunner(), tmp_path, {})
    assert engine.repository(url, {"type": "github"}) == ReleaseRepository(
        "github.com", "example/revaer"
    )


@pytest.mark.parametrize(
    "remote",
    [
        {"name": "upstream"},
        {"domain": "http://github.com"},
        {"domain": "https://elsewhere.invalid"},
        {"api_domain": "https://elsewhere.invalid"},
        {"url": "file:///tmp/example/revaer.git"},
    ],
)
def test_inconsistent_release_configuration_fails_closed(
    tmp_path: Path, remote: dict[str, object]
) -> None:
    engine = SemanticRelease(sys.executable, RecordingRunner(), tmp_path, {})
    with pytest.raises(ToolingError):
        engine.repository("https://github.com/example/revaer.git", {"type": "github", **remote})


@pytest.mark.parametrize(
    "dependencies",
    [
        [],
        [{"name": "example", "skip_reason": "URL requirement"}],
        [{"name": "example", "version": "1", "vulns": [{"id": "test-advisory"}]}],
        [{"name": "example", "version": "1"}],
        [{"name": "", "version": "1", "vulns": []}],
    ],
)
def test_successful_audit_process_cannot_hide_missing_or_skipped_evidence(
    tmp_path: Path, dependencies: list[dict[str, object]]
) -> None:
    runner = RecordingRunner(json.dumps({"dependencies": dependencies}))
    tool = Python(sys.executable, runner, tmp_path, {})
    with pytest.raises(ToolingError):
        tool.audit(tmp_path / "pylock.toml")


def test_versioned_source_package_can_be_fully_audited(tmp_path: Path) -> None:
    runner = RecordingRunner(
        json.dumps(
            {
                "dependencies": [
                    {"name": "python-semantic-release", "version": "10.6.2", "vulns": []}
                ]
            }
        )
    )
    result = Python(sys.executable, runner, tmp_path, {}).audit(tmp_path / "pylock.toml")
    assert result.code == 0
    assert "--locked" in runner.calls[0].argv
    assert "--strict" in runner.calls[0].argv
