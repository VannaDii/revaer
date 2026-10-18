"""Collect complete Rust/native coverage and retain reports on a threshold failure."""

import os
import re
import tomllib
from pathlib import Path

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.coverage import CoverageFormat, CoverageReportArgs
from ..sonar.lcov import merge_compilations
from .base import Task
from .testing import native_test_environment

WORKSPACE_LCOV = "workspace-lcov.info"
NONPRODUCTION_PACKAGES = frozenset(("revaer-test-support", "revaer-doc-indexer", "asset_sync"))


def reset_reports(context: Context) -> None:
    """Invalidate only our reports before collecting or regenerating evidence."""
    output = context.root / "coverage"
    if output.is_symlink():
        raise ToolingError("Coverage output directory must not be a symlink")
    for name in (
        "lcov.info",
        WORKSPACE_LCOV,
        "rust-sonar-lcov.info",
        "llvm-cov.txt",
        "html",
        "crates",
    ):
        context.fs.remove_owned(output / name, context.root)
    context.fs.mkdir(output / "crates")


def coverage_environment(context: Context) -> dict[str, str]:
    pins = tomllib.loads(context.fs.read(context.root / "tools/versions.toml"))
    context.tools.cargo_llvm_cov.verify(pins["cargo"]["cargo-llvm-cov"]["version"])
    context.tools.cargo_nextest.verify(pins["cargo"]["cargo-nextest"]["version"])
    compiler = context.tools.rustc.information()
    environment = {
        "RUSTC": str(compiler.executable),
        "REVAER_RUST_LLVM_VERSION": compiler.llvm_version,
        "CARGO_INCREMENTAL": "0",
        # Never let an analysis invocation prompt for or install components.
        "CARGO_LLVM_COV_SETUP": "no",
        "RUST_TEST_THREADS": str(context.settings.coverage.test_threads),
    }
    if context.settings.coverage.build_jobs != "default":
        environment["CARGO_BUILD_JOBS"] = str(context.settings.coverage.build_jobs)
    evidence = [compiler.verbose.rstrip()]
    for name, tool in (
        ("CC", context.tools.cc),
        ("CXX", context.tools.cxx),
        ("LLVM_COV", context.tools.llvm_cov),
        ("LLVM_PROFDATA", context.tools.llvm_profdata),
    ):
        version = tool.verify()
        environment[name] = str(version.executable)
        evidence.append(f"{name}={version.executable}\n{name}_VERSION={version.version}")
    output = context.root / "coverage"
    if output.is_symlink():
        raise ToolingError("Coverage output directory must not be a symlink")
    context.fs.write(output / "toolchain.txt", "\n".join(evidence) + "\n")
    context.fs.write(output / "rust-sources.rsp", context.tools.rustc.source_mapping(compiler))
    supplied = context.tools.cargo_llvm_cov.environment.get("LLVM_COV_FLAGS", "")
    environment["LLVM_COV_FLAGS"] = (supplied + " @coverage/rust-sources.rsp").strip()
    return environment


def rust_lcov(document: str, root: Path, tracked: tuple[str, ...]) -> str:
    """Route measured authored Rust records to the Rust-only scanner sensor.

    LLVM also exports native, dependency and generated records. Those are not
    indexed authored Rust inputs. Preserve their measurements in text/HTML and
    package JSON, rather than sending them to an incompatible language sensor.
    """
    sources = {root / name for name in tracked if name.endswith(".rs")}
    compilations: dict[Path, list[list[str]]] = {}
    for record in document.split("end_of_record"):
        lines = record.strip().splitlines()
        filenames = [line[3:] for line in lines if line.startswith("SF:")]
        if not lines:
            continue
        if len(filenames) != 1:
            raise ToolingError("LLVM coverage record must identify exactly one source")
        filename = os.path.normpath(filenames[0])
        source = Path(filename)
        if not source.is_absolute():
            source = root / source
        if source in sources:
            compilations.setdefault(source, []).append(lines)
    if not compilations:
        raise ToolingError("LLVM coverage has no authored Rust records")
    selected: list[str] = []
    for source, records in compilations.items():
        lines = records[0] if len(records) == 1 else merge_compilations(str(source), records)
        selected.extend((*lines, "end_of_record"))
    return "\n".join(selected) + "\n"


class CoverageReport(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        environment = coverage_environment(context)
        output = context.root / "coverage"
        # Only replace this command's reports. Python and browser coverage belong
        # to separate producers and must survive a Rust report refresh.
        reset_reports(context)
        failures = []
        for package in context.tools.cargo.metadata().workspace_packages:
            context.emit(f"Coverage: {package} (production source scope)")
            try:
                report = output / "crates" / f"{package}.json"
                context.tools.cargo_llvm_cov.report(
                    CoverageReportArgs(CoverageFormat.JSON, report, package),
                    environment,
                )
                if package not in NONPRODUCTION_PACKAGES:
                    context.tools.cargo_llvm_cov.check_lines(package, environment)
            except ToolingError as error:
                failures.append(f"{package}: {error}")
        for format, destination in (
            (CoverageFormat.LCOV, output / WORKSPACE_LCOV),
            (CoverageFormat.TEXT, output / "llvm-cov.txt"),
            (CoverageFormat.HTML, output),
        ):
            context.tools.cargo_llvm_cov.report(
                CoverageReportArgs(format, destination), environment
            )
        lcov = context.fs.read(output / WORKSPACE_LCOV)
        lcov = rust_lcov(lcov, context.root, context.tools.git.files())
        context.fs.write(output / "lcov.info", lcov)
        if not any(line.startswith("SF:") for line in lcov.splitlines()) or not any(
            line.startswith("DA:") for line in lcov.splitlines()
        ):
            raise ToolingError("LLVM coverage report has no source and line records")
        if failures:
            raise ToolingError(
                "Coverage failed; diagnostic reports retained:\n" + "\n".join(failures)
            )
        return TaskResult("Rust/native coverage reports and per-crate gates passed")


class Coverage(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        recovery_root = context.settings.coverage.native_recovery_root
        if recovery_root is not None and context.host.system != "linux":
            raise ToolingError("Native media recovery coverage requires Linux")
        with native_test_environment(context) as database:
            return Coverage.collect(context, database)

    @staticmethod
    def collect(context: Context, database: dict[str, str]) -> TaskResult:
        recovery_root = (
            database.get("REVAER_NATIVE_RECOVERY_ROOT")
            or context.settings.coverage.native_recovery_root
        )
        environment = {**coverage_environment(context), **database, "REVAER_NATIVE_IT": "1"}
        # A compile/test failure must not leave yesterday's reports looking like
        # this run's output. Raw profiles remain engine-owned diagnostic data.
        reset_reports(context)
        context.tools.cargo_llvm_cov.clean(environment)
        context.tools.cargo_llvm_cov.collect(environment)
        if "revaer-app" in context.tools.cargo.metadata().workspace_packages:
            result = context.tools.cargo_llvm_cov.collect(
                environment,
                test_filter="media_job_runtime::tests::production_media_job_runtime_executes_and_persists_verified_replacement",
            )
            context.emit(result.stdout)
            if not re.search(r"Summary.*1 test run: 1 passed", result.stdout + result.stderr):
                raise ToolingError("Production worker coverage did not execute one passing test")
        if recovery_root is not None:
            # Keep the workspace profiles and use the same all-feature FFI instrumentation.
            result = context.tools.cargo_llvm_cov.collect(
                {**environment, "REVAER_NATIVE_RECOVERY_ROOT": str(recovery_root)},
                test_filter="bootstrap::service_recovery_tests::native_service_recovery_",
            )
            context.emit(result.stdout)
            if not re.search(r"Summary.*9 tests run: 9 passed", result.stdout + result.stderr):
                raise ToolingError(
                    "Native recovery coverage did not execute all nine passing scenarios"
                )
        return CoverageReport.run(context)
