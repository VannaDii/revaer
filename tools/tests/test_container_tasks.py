"""Container stage failures retain their ordering and cannot mutate the host.

Native installs, account records and root directory ownership are injected here.
The unchanged Dockerfile fixture separately executes these boundaries in Alpine.
"""

import grp
import pwd
import shutil
import sys
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.context import Context
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.accounts import AddGroup, AddUser
from revaer_tooling.external.coverage import Rustc
from revaer_tooling.external.packages import Apk
from revaer_tooling.external.python import Python, Uv
from revaer_tooling.external.rust import Cargo, ContainerBuildArgs, Rustup
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.images.inputs import BuildInputs
from revaer_tooling.process import Completed, Invocation
from revaer_tooling.settings import load_settings
from revaer_tooling.tasks.containers import ContainerBuild, ContainerRuntime, container_inputs
from test_build_inputs import context as input_context
from test_external import RecordingRunner
from test_image_compliance import context as compliance_context

__all__ = ["compliance_context", "input_context"]


class ContainerFiles(FileSystem):
    def __init__(self, root: Path, alpine: str) -> None:
        self.root = root
        self.alpine = alpine
        self.directories: list[Path] = []
        self.permissions: list[tuple[Path, int, int, int]] = []

    def is_file(self, path: Path) -> bool:
        return bool(self.alpine) if path == Path("/etc/alpine-release") else super().is_file(path)

    def read(self, path: Path) -> str:
        return self.alpine if path == Path("/etc/alpine-release") else super().read(path)

    def mkdir(self, path: Path) -> None:
        if path.is_relative_to(self.root):
            super().mkdir(path)
        else:
            self.directories.append(path)

    def directory_permissions(self, path: Path, uid: int, gid: int, mode: int) -> None:
        self.permissions.append((path, uid, gid, mode))


class ContainerRunner(RecordingRunner):
    def __init__(self) -> None:
        super().__init__()
        self.fail = ""
        self.omit_binary = False

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        args = invocation.argv[1:]
        if self.fail and args[0] == self.fail:
            raise CommandError("Fixture native operation failed", 17)
        if args[:2] == ("--version", "uv"):
            return Completed(0, "uv 0.12.13")
        if args[:2] == ("--version", "rust"):
            return Completed(0, "rustc 1.96.0")
        if args == ("--version",):
            return Completed(0, "Python 3.13.12")
        if args[0] == "build" and not self.omit_binary:
            target = Path(args[args.index("--target-dir") + 1])
            binary = target / args[args.index("--target") + 1] / "release/revaer-app"
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b"compiled fixture application")
        return Completed(0, "")


class FixtureUv(Uv):
    version_args = ("--version", "uv")


class FixtureRustc(Rustc):
    version_args = ("--version", "rust")


@pytest.fixture
def runner() -> ContainerRunner:
    return ContainerRunner()


@pytest.fixture
def context(
    input_context: Context, runner: ContainerRunner, monkeypatch: pytest.MonkeyPatch
) -> Context:
    base = input_context
    source = Path(__file__).parents[2]
    shutil.copy(source / "tools/versions.toml", base.root / "tools/versions.toml")
    pins = BuildInputs.load(base.fs, base.root)
    settings = load_settings(
        {
            "RUST_BUILDER_IMAGE": pins.builder,
            "ALPINE_RUNTIME_IMAGE": pins.runtime,
            "UV_IMAGE": pins.uv_image,
            "RUST_VERSION": pins.rust_version,
            "ALPINE_VERSION": pins.alpine_version,
            "TARGETARCH": "arm64",
        }
    )

    def group(name: str) -> grp.struct_group:
        if any(call.argv[1:] == ("-S", name) for call in runner.calls):
            return grp.struct_group((name, "x", 345, []))
        raise KeyError(name)

    def user(name: str) -> pwd.struct_passwd:
        if any(call.argv[1:] == ("-S", name, "-G", name) for call in runner.calls):
            return pwd.struct_passwd((name, "x", 456, 345, "", "/home/revaer", "/sbin/nologin"))
        raise KeyError(name)

    monkeypatch.setattr(grp, "getgrnam", group)
    monkeypatch.setattr(pwd, "getpwnam", user)
    return replace(
        base,
        settings=settings,
        host=replace(base.host, uid=0, system="linux"),
        fs=ContainerFiles(base.root, pins.alpine_version),
        tools=replace(
            base.tools,
            python=Python(sys.executable, runner, base.root, {}),
            uv=FixtureUv(sys.executable, runner, base.root, {}),
            rustc=FixtureRustc(sys.executable, runner, base.root, {}),
            rustup=Rustup(sys.executable, runner, base.root, {}),
            apk=Apk(sys.executable, runner, base.root, {}),
            cargo=Cargo(sys.executable, runner, base.root, {"RUSTFLAGS": "--unrelated"}),
            addgroup=AddGroup(sys.executable, runner, base.root, {}),
            adduser=AddUser(sys.executable, runner, base.root, {}),
        ),
    )


def test_container_build_preserves_pins_component_order_and_normalized_output(
    context: Context, runner: ContainerRunner
) -> None:
    ContainerBuild.run(context)
    operations = [call.argv[1] for call in runner.calls]
    assert operations == [
        "--version",
        "--version",
        "toolchain",
        "--version",
        "add",
        "target",
        "build",
    ]
    compile_call = runner.calls[-1]
    assert (
        compile_call.env["CARGO_ENCODED_RUSTFLAGS"]
        == "-Dwarnings\x1f-C\x1ftarget-feature=-crt-static"
    )
    assert "--all-features" not in compile_call.argv
    assert (
        context.root / "target/release/revaer-app"
    ).read_bytes() == b"compiled fixture application"


@pytest.mark.parametrize("failure", ("toolchain", "add", "target", "build"))
def test_container_install_or_build_failure_prevents_normalized_completion(
    context: Context, runner: ContainerRunner, failure: str
) -> None:
    runner.fail = failure
    with pytest.raises(CommandError):
        ContainerBuild.run(context)
    assert not (context.root / "target/release/revaer-app").exists()
    assert runner.calls[-1].argv[1] == failure


def test_missing_binary_or_wrong_target_does_not_claim_success(
    context: Context, runner: ContainerRunner
) -> None:
    runner.omit_binary = True
    with pytest.raises(ToolingError, match="did not produce"):
        ContainerBuild.run(context)
    runner.calls.clear()
    altered = replace(
        context,
        settings=replace(
            context.settings,
            container=replace(context.settings.container, rust_target="x86_64-unknown-linux-musl"),
        ),
    )
    with pytest.raises(ToolingError, match="reviewed"):
        ContainerBuild.run(altered)
    assert all(call.argv[1] == "--version" for call in runner.calls)


def test_runtime_repeat_uses_the_same_account_and_directory_ownership(
    context: Context, runner: ContainerRunner
) -> None:
    ContainerRuntime.run(context)
    ContainerRuntime.run(context)
    assert len([call for call in runner.calls if call.argv[1] == "-S"]) == 2
    assert isinstance(context.fs, ContainerFiles)
    assert context.fs.permissions[:5] == context.fs.permissions[5:]
    assert context.fs.permissions[:5] == [
        (Path("/app"), 0, 0, 0o755),
        (Path("/app/compliance"), 0, 0, 0o755),
        (Path("/data"), 456, 345, 0o755),
        (Path("/config"), 456, 345, 0o755),
        (Path("/app/docs/api"), 456, 345, 0o755),
    ]


def test_runtime_pin_or_package_failure_prevents_accounts_and_permissions(
    context: Context, runner: ContainerRunner
) -> None:
    assert isinstance(context.fs, ContainerFiles)
    context.fs.alpine = "3.23.4"
    with pytest.raises(ToolingError, match="installed Alpine"):
        ContainerRuntime.run(context)
    assert not context.fs.permissions
    context.fs.alpine = context.settings.container.alpine_version
    runner.fail = "add"
    with pytest.raises(CommandError):
        ContainerRuntime.run(context)
    assert not context.fs.permissions
    assert not context.fs.directories
    context.fs.alpine = ""
    with pytest.raises(ToolingError, match="requires Alpine Linux"):
        container_inputs(context)


def test_missing_native_account_records_fail_explicitly(
    context: Context, runner: ContainerRunner, monkeypatch: pytest.MonkeyPatch
) -> None:
    def absent(name: str) -> grp.struct_group:
        raise KeyError(name)

    monkeypatch.setattr(grp, "getgrnam", absent)
    with pytest.raises(ToolingError, match="group creation"):
        context.tools.addgroup.ensure_system("revaer")

    def absent_user(name: str) -> pwd.struct_passwd:
        raise KeyError(name)

    monkeypatch.setattr(pwd, "getpwnam", absent_user)
    with pytest.raises(ToolingError, match="user creation"):
        context.tools.adduser.ensure_system("revaer", "revaer", 345)
    with pytest.raises(ToolingError, match="lowercase name"):
        context.tools.adduser.ensure_system("--root", "revaer", 345)
    with pytest.raises(ToolingError, match="musl target"):
        context.tools.cargo.container_build(
            ContainerBuildArgs("unreviewed", context.root / "target")
        )
