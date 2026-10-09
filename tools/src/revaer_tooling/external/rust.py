"""Rust build, analysis, documentation and toolchain operations."""

import json
import re
from collections.abc import Mapping
from dataclasses import dataclass, field
from enum import StrEnum
from pathlib import Path

from ..errors import ToolingError
from ..images.model import ARCHITECTURES
from ..process import Completed, Invocation, RunningProcess
from .base import ExternalTool
from .serving import ServingExecutable, ServingKind, select_executable


class CargoOperation(StrEnum):
    BUILD = "build"
    CHECK = "check"
    TEST = "test"


class AppRegression(StrEnum):
    """The three existing media application bootstrap regression groups."""

    LAUNCH = "bootstrap::runtime_tests::e2e_"
    COMPLIANCE = "bootstrap::compliance_tests"
    BOOTSTRAP = "bootstrap"


class DatabaseProbe(StrEnum):
    POOL = "pool"
    CANCELLATION = "cancellation"


@dataclass(frozen=True)
class DatabaseProbeArgs:
    kind: DatabaseProbe
    input_file: Path

    @property
    def variable(self) -> str:
        return f"REVAER_INGESTION_{self.kind.upper()}_PROOF"

    @property
    def test_name(self) -> str:
        return (
            f"indexers::search_results::{self.kind}_proof_tests::"
            f"application_{self.kind}_qualification"
        )


@dataclass(frozen=True)
class CargoArgs:
    operation: CargoOperation
    packages: tuple[str, ...] = ()
    all_features: bool = True
    all_targets: bool = False
    release: bool = False
    no_default_features: bool = False
    test_threads: int | None = None
    test_binary: str | None = None
    test_filter: str | None = None
    include_ignored: bool = False


@dataclass(frozen=True)
class CargoInstallArgs:
    package: str
    version: str
    features: tuple[str, ...] = ()
    no_default_features: bool = False


@dataclass(frozen=True)
class ContainerBuildArgs:
    target: str
    target_directory: Path


@dataclass(frozen=True)
class CargoMetadata:
    document: str
    target_directory: Path
    workspace_packages: tuple[str, ...]


@dataclass(frozen=True)
class DevelopmentArgs:
    database_url: str = field(repr=False)
    rust_log: str
    log_path: Path
    ownership_token: str = field(repr=False)


class CargoUdeps(ExternalTool):
    # Cargo extensions can require the subcommand name even when invoked by path.
    version_args = ("udeps", "--version")


class Cargo(ExternalTool):
    def database_probe(self, args: DatabaseProbeArgs) -> Completed:
        result = self._invoke(
            (
                "--config",
                'build.rustflags=["-Dwarnings"]',
                "test",
                "--locked",
                "--package",
                "revaer-data",
                "--all-features",
                "--lib",
                args.test_name,
                "--",
                "--exact",
                "--show-output",
            ),
            env={args.variable: str(args.input_file)},
            capture=True,
        )
        # Cargo succeeds when an exact selector matches nothing. A transition
        # checkout missing the requested qualification must not report success.
        if not re.search(r"(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;", result.stdout):
            raise ToolingError("Database qualification did not execute exactly one passing test")
        return result

    def app_regressions(self, group: AppRegression, env: Mapping[str, str]) -> Completed:
        """Retain default features and require actual tests in every recipe group."""
        library = group != AppRegression.BOOTSTRAP
        result = self._invoke(
            (
                "--config",
                'build.rustflags=["-Dwarnings"]',
                "test",
                "--locked",
                "--package",
                "revaer-app",
                *(("--lib", group.value) if library else ("--test", group.value)),
                *(("--", "--test-threads=1") if library else ()),
            ),
            env=env,
            capture=True,
        )
        # Cargo succeeds when a renamed filter matches nothing. That is missing
        # regression coverage, not a passing launch/compliance qualification.
        if not re.search(
            r"(?m)^test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;",
            result.stdout,
        ):
            raise ToolingError(f"Application regression group {group.name} ran no passing tests")
        return result

    def shutdown_tests(self, minimal: bool) -> Completed:
        """Run the media runtime's library checks in one explicit feature mode."""
        result = self._invoke(
            (
                "--config",
                'build.rustflags=["-Dwarnings"]',
                "test",
                "--locked",
                "--package",
                "revaer-app",
                "--no-default-features" if minimal else "--all-features",
                "--lib",
                "bootstrap::shutdown_tests",
                "--",
                "--show-output",
            ),
            capture=True,
        )
        # A removed/renamed module otherwise produces Cargo's successful zero-test
        # result. Require positive execution in each feature mode independently.
        if not re.search(
            r"(?m)^test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;",
            result.stdout,
        ):
            raise ToolingError("Runtime shutdown qualification did not execute passing tests")
        return result

    def development(self, args: DevelopmentArgs) -> RunningProcess:
        """Cargo owns compile/run behavior; watchfiles only schedules restarts."""
        return self.runner.start(
            Invocation(
                (
                    str(self.locate()),
                    "--config",
                    'build.rustflags=["-Dwarnings"]',
                    "run",
                    "--locked",
                    "--package",
                    "revaer-app",
                ),
                self.root,
                {
                    **self.environment,
                    "DATABASE_URL": args.database_url,
                    "RUST_LOG": args.rust_log,
                    "RV_DEV_SESSION": args.ownership_token,
                },
                log_path=args.log_path,
                stream_log=True,
            )
        )

    def container_build(self, args: ContainerBuildArgs) -> Completed:
        if args.target not in {item.rust_target for item in ARCHITECTURES}:
            raise ToolingError("Container builds require a reviewed musl target")
        return self._invoke(
            (
                "build",
                "--release",
                "--locked",
                "--package",
                "revaer-app",
                "--target",
                args.target,
                "--target-dir",
                str(args.target_directory),
            ),
            # Match the runtime's shared libtorrent/OpenSSL ABI. Cargo's encoded
            # flags take precedence over inherited RUSTFLAGS and retain -Dwarnings.
            env={"CARGO_ENCODED_RUSTFLAGS": "-Dwarnings\x1f-C\x1ftarget-feature=-crt-static"},
        )

    def serving_executable(self, kind: ServingKind) -> ServingExecutable:
        target = (
            ("test", "--lib", "--no-run")
            if kind == ServingKind.LIBRARY_TEST
            else ("build", "--bin", "revaer-app")
        )
        output = self._invoke(
            (
                "--config",
                'build.rustflags=["-Dwarnings"]',
                *target,
                "--package",
                "revaer-app",
                "--locked",
                "--message-format=json-render-diagnostics",
            ),
            capture=True,
        ).stdout
        return select_executable(output, self.root, kind)

    def execute(
        self, args: CargoArgs, env: Mapping[str, str] | None = None, *, capture: bool = False
    ) -> Completed:
        selectors = (args.test_binary, args.test_filter)
        if (
            any(value is not None for value in selectors) or args.include_ignored
        ) and args.operation != CargoOperation.TEST:
            raise ToolingError("Test selectors require a Cargo test operation")
        if any(
            value is not None and (not value or value.startswith("-") or "\x00" in value)
            for value in selectors
        ):
            raise ToolingError("Cargo test selectors must be nonempty literal names")
        command = ["--config", 'build.rustflags=["-Dwarnings"]', args.operation.value, "--locked"]
        if args.packages:
            for package in args.packages:
                command.extend(("--package", package))
        else:
            command.append("--workspace")
        if args.all_features:
            command.append("--all-features")
        if args.all_targets:
            command.append("--all-targets")
        if args.release:
            command.append("--release")
        if args.no_default_features:
            command.append("--no-default-features")
        if args.test_binary is not None:
            command.extend(("--test", args.test_binary))
        if args.test_filter is not None:
            command.append(args.test_filter)
        harness: list[str] = []
        if args.include_ignored:
            harness.append("--include-ignored")
        if args.test_threads is not None:
            if args.operation != CargoOperation.TEST or args.test_threads < 1:
                raise ToolingError("Test threads require a test operation and a positive count")
            harness.append(f"--test-threads={args.test_threads}")
        if harness:
            command.extend(("--", *harness))
        return self._invoke(tuple(command), env=env, capture=capture)

    def fmt(self, fix: bool = False) -> Completed:
        return self._invoke(("fmt", "--all") + (() if fix else ("--check",)))

    def clippy(
        self, production: bool = False, *, packages: tuple[str, ...] = (), minimal: bool = False
    ) -> Completed:
        targets = ("--lib", "--bins", "--examples") if production else ("--all-targets",)
        lints = (
            (
                "-D",
                "warnings",
                *tuple(
                    item
                    for lint in (
                        "expect_used",
                        "panic",
                        "todo",
                        "unimplemented",
                        "unreachable",
                        "unwrap_used",
                    )
                    for item in ("-W", f"clippy::{lint}")
                ),
            )
            if production
            else (
                "-D",
                "warnings",
                "-W",
                "clippy::cargo",
                "-W",
                "clippy::nursery",
                "-A",
                "clippy::multiple_crate_versions",
                "-A",
                "clippy::redundant_pub_crate",
            )
        )
        return self._invoke(
            (
                "clippy",
                *(
                    tuple(item for name in packages for item in ("--package", name))
                    if packages
                    else ("--workspace",)
                ),
                "--locked",
                *targets,
                "--no-default-features" if minimal else "--all-features",
                "--",
                *lints,
            )
        )

    def run_binary(
        self, package: str, binary: str | None = None, release: bool = False
    ) -> Completed:
        args = ["run", "--locked", "--package", package]
        if binary:
            args.extend(("--bin", binary))
        if release:
            args.append("--release")
        return self._invoke(tuple(args))

    def install(self, args: CargoInstallArgs) -> Completed:
        # Cargo tracks package versions, features and installed binaries. Let it
        # skip an identical install and reject collisions instead of forcing it.
        command = [
            "install",
            "--locked",
            "--version",
            args.version,
            args.package,
        ]
        if args.features:
            command.extend(("--features", ",".join(args.features)))
        if args.no_default_features:
            command.append("--no-default-features")
        return self._invoke(tuple(command))

    def audit(self) -> Completed:
        return self._invoke(("audit", "--deny", "warnings"))

    def deny(self) -> Completed:
        return self._invoke(("deny", "check"))

    def udeps(self, toolchain: str) -> Completed:
        return self._invoke((f"+{toolchain}", "udeps", "--workspace", "--all-targets"))

    def metadata(self) -> CargoMetadata:
        document = self._invoke(
            ("metadata", "--format-version", "1", "--all-features", "--locked"), capture=True
        ).stdout
        try:
            metadata = json.loads(document)
        except ValueError as error:
            raise ToolingError("Cargo returned invalid metadata") from error
        target = metadata.get("target_directory") if isinstance(metadata, dict) else None
        if not isinstance(target, str) or not Path(target).is_absolute():
            raise ToolingError("Cargo metadata has no absolute target directory")
        members = metadata.get("workspace_members")
        packages = metadata.get("packages")
        if not isinstance(members, list) or not isinstance(packages, list) or not members:
            raise ToolingError("Cargo metadata has no workspace members")
        # Cargo expands member globs and knows implicit path members. Its opaque
        # package IDs are identity keys, not strings for us to parse or recreate.
        names = []
        for member in members:
            matches = [
                item for item in packages if isinstance(item, dict) and item.get("id") == member
            ]
            if len(matches) != 1 or not isinstance(matches[0].get("name"), str):
                raise ToolingError("Cargo metadata has an unresolved workspace member")
            names.append(matches[0]["name"])
        return CargoMetadata(document, Path(target), tuple(sorted(names)))

    def licenses(self) -> str:
        return self._invoke(("deny", "list", "--format", "json"), capture=True).stdout


class Rustup(ExternalTool):
    def install(self, channel: str, components: tuple[str, ...]) -> Completed:
        return self._invoke(
            (
                "toolchain",
                "install",
                channel,
                "--no-self-update",
                "--profile",
                "minimal",
                *(("--component", ",".join(components)) if components else ()),
            )
        )

    def target_add(self, target: str) -> Completed:
        return self._invoke(("target", "add", target))

    def compiler_version(self, channel: str) -> str:
        return self._invoke(
            ("run", channel, "rustc", "--version", "--verbose"), capture=True
        ).stdout


@dataclass(frozen=True)
class TrunkServeArgs:
    port: int
    api_url: str
    log_path: Path


class Trunk(ExternalTool):
    def development(self, args: DevelopmentArgs) -> RunningProcess:
        return self.runner.start(
            Invocation(
                (str(self.locate()), "serve", "--dist", "dist-serve"),
                self.root / "crates/revaer-ui",
                {
                    **self.environment,
                    "NO_COLOR": "true",
                    "DATABASE_URL": args.database_url,
                    "RUST_LOG": args.rust_log,
                    "RV_DEV_SESSION": args.ownership_token,
                },
                log_path=args.log_path,
                stream_log=True,
            )
        )

    def start(self, args: "TrunkServeArgs") -> RunningProcess:
        return self.runner.start(
            Invocation(
                (str(self.locate()), "serve", "--dist", "dist-serve", "--port", str(args.port)),
                self.root / "crates/revaer-ui",
                {**self.environment, "NO_COLOR": "true", "REVAER_UI_API_BASE_URL": args.api_url},
                log_path=args.log_path,
            )
        )

    def build(self) -> Completed:
        return self._invoke(
            ("build", "--release"), cwd=self.root / "crates/revaer-ui", env={"NO_COLOR": "true"}
        )

    def serve(self, open_browser: bool = False) -> Completed:
        return self._invoke(
            ("serve", "--dist", "dist-serve", *(("--open",) if open_browser else ())),
            cwd=self.root / "crates/revaer-ui",
            env={"NO_COLOR": "true"},
        )


class Mdbook(ExternalTool):
    def build(self, serve: bool = False) -> Completed:
        return self._invoke(("serve", "--open") if serve else ("build",), cwd=self.root / "docs")


class MdbookMermaid(ExternalTool):
    def install(self) -> Completed:
        """Let the pinned preprocessor install its supported book integration."""
        return self._invoke(("install", "docs"))


class Lychee(ExternalTool):
    def check(self) -> Completed:
        return self._invoke(("--verbose", "--no-progress", "docs"))
