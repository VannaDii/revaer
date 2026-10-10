"""Collect complete Rust/native coverage and retain reports on a threshold failure."""

import re
import tomllib

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.coverage import CoverageFormat, CoverageReportArgs
from ..json_data import array_value, decode, object_value, string_value
from .base import Task
from .testing import native_test_environment


def reset_reports(context: Context) -> None:
    """Invalidate only our reports before collecting or regenerating evidence."""
    output = context.root / "coverage"
    if output.is_symlink():
        raise ToolingError("Coverage output directory must not be a symlink")
    for name in ("lcov.info", "llvm-cov.txt", "html", "crates"):
        context.fs.remove_owned(output / name, context.root)
    context.fs.mkdir(output / "crates")


def coverage_environment(context: Context) -> dict[str, str]:
    pins = tomllib.loads(context.fs.read(context.root / "tools/versions.toml"))
    context.tools.cargo_llvm_cov.verify(pins["cargo"]["cargo-llvm-cov"]["version"])
    compiler = context.tools.rustc.information()
    environment = {
        "RUSTC": str(compiler.executable),
        "REVAER_RUST_LLVM_VERSION": compiler.llvm_version,
        # Never let an analysis invocation prompt for or install components.
        "CARGO_LLVM_COV_SETUP": "no",
        "RUST_TEST_THREADS": str(context.settings.coverage.test_threads),
        "CARGO_BUILD_JOBS": str(context.settings.coverage.build_jobs),
    }
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


def rust_line_gate(document: str, package: str) -> str:
    """Preserve the original Rust gate using the engine's unrounded line counts.

    This does not remove C++ or other files from the JSON, LCOV, text, or HTML
    evidence. Before native instrumentation was added, this gate measured Rust.
    """
    report = object_value(decode(document))
    count = covered = 0
    for unit in array_value(report.get("data")):
        for item in array_value(object_value(unit).get("files")):
            source = object_value(item)
            if string_value(source.get("filename")).endswith(".rs"):
                lines = object_value(object_value(source.get("summary")).get("lines"))
                total, hits = lines.get("count"), lines.get("covered")
                if type(total) is not int or type(hits) is not int or not 0 <= hits <= total:
                    raise ToolingError(f"Invalid coverage line counts for {package}")
                count += total
                covered += hits
    if count == 0:
        raise ToolingError(f"Rust coverage has no line records for {package}")
    summary = f"{package}: {covered}/{count} Rust lines ({100 * covered / count:.2f}%)"
    if covered * 100 < count * 90:
        raise ToolingError(summary + " is below 90%")
    return summary


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
            context.emit(f"Coverage: {package} (90% Rust line minimum)")
            try:
                report = output / "crates" / f"{package}.json"
                context.tools.cargo_llvm_cov.report(
                    CoverageReportArgs(CoverageFormat.JSON, report, package),
                    environment,
                )
                context.emit(rust_line_gate(context.fs.read(report), package))
            except ToolingError as error:
                failures.append(f"{package}: {error}")
        for format, destination in (
            (CoverageFormat.LCOV, output / "lcov.info"),
            (CoverageFormat.TEXT, output / "llvm-cov.txt"),
            (CoverageFormat.HTML, output),
        ):
            context.tools.cargo_llvm_cov.report(
                CoverageReportArgs(format, destination), environment
            )
        lcov = context.fs.read(output / "lcov.info")
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
            if not re.search(
                r"(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;", result.stdout
            ):
                raise ToolingError("Production worker coverage did not execute one passing test")
        if recovery_root is not None:
            # Keep the workspace profiles and use the same all-feature FFI instrumentation.
            result = context.tools.cargo_llvm_cov.collect(
                {**environment, "REVAER_NATIVE_RECOVERY_ROOT": str(recovery_root)},
                test_filter="bootstrap::service_recovery_tests::native_service_shutdown_resumes_active_ffmpeg",
            )
            context.emit(result.stdout)
            if not re.search(
                r"(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;", result.stdout
            ):
                raise ToolingError("Native recovery coverage did not execute one passing test")
        return CoverageReport.run(context)
