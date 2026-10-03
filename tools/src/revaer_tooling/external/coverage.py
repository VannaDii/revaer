"""Typed cargo-llvm-cov operations and the native compiler inputs it requires.

Rust supplies the default LLVM reporting tools through llvm-tools-preview.
Clang remains a native prerequisite, with CC/CXX and LLVM_* overrides preserved.
No shell evaluation, compiler installation, or profile conversion happens here.
"""

import re
import shlex
import shutil
from collections.abc import Mapping
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path

from ..errors import ToolingError
from ..process import Completed, Runner
from .base import ExternalTool


@dataclass(frozen=True)
class RustCompilerInfo:
    executable: Path
    host: str
    llvm_version: str
    sysroot: Path
    verbose: str


class Rustc(ExternalTool):
    def source_mapping(self, compiler: RustCompilerInfo) -> str:
        fields = dict(line.split(": ", 1) for line in compiler.verbose.splitlines() if ": " in line)
        commit = fields.get("commit-hash", "")
        source = compiler.sysroot / "lib/rustlib/src/rust"
        if re.fullmatch(r"[0-9a-f]{40}", commit) is None or not (source / "library").is_dir():
            raise ToolingError("Matching Rust sources are unavailable; run rv setup for rust-src")
        if "," in str(source):
            raise ToolingError(
                "LLVM path equivalence cannot represent a Rust source path containing commas"
            )
        # LLVM's documented response-file syntax preserves spaces and quotes.
        # cargo-llvm-cov splits LLVM_COV_FLAGS on spaces before forwarding it.
        return shlex.join((f"-path-equivalence=/rustc/{commit},{source}",)) + "\n"

    def information(self) -> RustCompilerInfo:
        verbose = self._invoke(("--version", "--verbose"), capture=True).stdout
        fields = dict(line.split(": ", 1) for line in verbose.splitlines() if ": " in line)
        host = fields.get("host", "")
        llvm = fields.get("LLVM version", "")
        if not re.fullmatch(r"[\w.-]+", host) or not re.fullmatch(r"\d+\.\d+\.\d+", llvm):
            raise ToolingError("rustc did not report a host triple and LLVM version")
        sysroot = Path(self._invoke(("--print", "sysroot"), capture=True).stdout.strip())
        if not sysroot.is_absolute():
            raise ToolingError("rustc did not report an absolute sysroot")
        return RustCompilerInfo(self.locate(), host, llvm, sysroot, verbose)


class LlvmTool(ExternalTool):
    """Resolve a reporting executable from the selected Rust compiler lazily.

    CLI setup and help must work before Rust exists. Deferring this lookup also
    means a rustup installation during setup is visible to the next operation.
    """

    def __init__(
        self,
        name: str,
        runner: Runner,
        root: Path,
        environment: Mapping[str, str],
        rustc: Rustc,
        override: str | None,
    ) -> None:
        super().__init__(name, runner, root, environment)
        self.rustc = rustc
        self.override = override

    def locate(self) -> Path:
        if self.override:
            executable = shutil.which(self.override, path=self.environment.get("PATH", ""))
        else:
            compiler = self.rustc.information()
            executable = shutil.which(
                str(compiler.sysroot / "lib/rustlib" / compiler.host / "bin" / self.name)
            )
        if executable is None:
            raise ToolingError(
                f"{self.name} is missing; run rv setup for llvm-tools-preview "
                "or select a compatible LLVM tool explicitly"
            )
        return Path(executable)


class CoverageFormat(StrEnum):
    JSON = "json"
    LCOV = "lcov"
    HTML = "html"
    TEXT = "text"


@dataclass(frozen=True)
class CoverageReportArgs:
    format: CoverageFormat
    output: Path
    package: str | None = None


class CargoLlvmCov(ExternalTool):
    version_args = ("llvm-cov", "--version")

    def clean(self, environment: Mapping[str, str]) -> Completed:
        return self._invoke(("llvm-cov", "clean", "--workspace"), env=environment)

    def collect(self, environment: Mapping[str, str]) -> Completed:
        # Preserve caller flags and append the warning gate using Cargo's exact
        # encoded-argument interface, which also handles paths containing spaces.
        supplied = self.environment.get("CARGO_ENCODED_RUSTFLAGS")
        flags = (
            {"CARGO_ENCODED_RUSTFLAGS": "\x1f".join(filter(None, (supplied, "-Dwarnings")))}
            if supplied is not None
            else {"RUSTFLAGS": f"{self.environment.get('RUSTFLAGS', '')} -Dwarnings".strip()}
        )
        return self._invoke(
            (
                "llvm-cov",
                "--workspace",
                "--all-features",
                "--locked",
                "--include-ffi",
                "--no-report",
            ),
            env={**environment, **flags},
        )

    def report(self, args: CoverageReportArgs, environment: Mapping[str, str]) -> Completed:
        # report consumes the collected profiles. In 0.8.7 its parser rejects
        # --all-features despite listing that flag in report --help.
        command = ["llvm-cov", "report", "--locked", "--include-ffi"]
        if args.package:
            # Retain every measurement in the package report. The task checks
            # the existing Rust-only threshold from these engine counts; newly
            # instrumented C++ remains visible without changing that criterion.
            command.extend(("--package", args.package, "--summary-only"))
        else:
            command.extend(("--no-default-ignore-filename-regex", "--include-build-script"))
        command.extend(
            (
                f"--{args.format}",
                "--output-dir" if args.format == CoverageFormat.HTML else "--output-path",
                str(args.output),
            )
        )
        return self._invoke(tuple(command), env=environment)
