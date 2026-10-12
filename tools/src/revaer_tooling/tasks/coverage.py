"""Collect complete Rust/native coverage and retain reports on a threshold failure."""

import tomllib
from pathlib import Path

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.coverage import CoverageFormat, CoverageReportArgs
from ..json_data import array_value, decode, object_value, string_value
from .base import Task
from .testing import test_environment


def reset_reports(context: Context) -> None:
    """Invalidate only our reports before collecting or regenerating evidence."""
    output = context.root / "coverage"
    if output.is_symlink():
        raise ToolingError("Coverage output directory must not be a symlink")
    for name in ("lcov.info", "workspace-lcov.info", "llvm-cov.txt", "html", "crates"):
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

    JSON, native text and HTML retain complete measurements. Sonar's Rust LCOV
    input contains authored Rust; its native sensor consumes the native text.
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
    # This internal database fixture is consumed only by dev-dependencies.
    # Its measurements remain complete; production thresholds do not apply.
    if package != "revaer-test-support" and covered * 100 < count * 90:
        raise ToolingError(summary + " is below 90%")
    return summary


def rust_lcov(document: str, root: Path, tracked: tuple[str, ...]) -> str:
    """Route measured authored Rust records to the Rust-only scanner sensor.

    LLVM also exports native, dependency and generated records. Those are not
    indexed authored Rust inputs. Preserve their measurements in text/HTML and
    package JSON, rather than sending them to an incompatible language sensor.
    """
    sources = {root / name for name in tracked if name.endswith(".rs")}
    selected: list[str] = []
    for record in document.split("end_of_record"):
        lines = record.strip().splitlines()
        filenames = [line[3:] for line in lines if line.startswith("SF:")]
        if not lines:
            continue
        if len(filenames) != 1:
            raise ToolingError("LLVM coverage record must identify exactly one source")
        source = Path(filenames[0])
        if not source.is_absolute():
            source = root / source
        if source in sources:
            selected.extend((*lines, "end_of_record"))
    if not selected:
        raise ToolingError("LLVM coverage has no authored Rust records")
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
            requirement = (
                "internal test utility measurements"
                if package == "revaer-test-support"
                else "90% Rust line minimum"
            )
            context.emit(f"Coverage: {package} ({requirement})")
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
            (CoverageFormat.LCOV, output / "workspace-lcov.info"),
            (CoverageFormat.TEXT, output / "llvm-cov.txt"),
            (CoverageFormat.HTML, output),
        ):
            context.tools.cargo_llvm_cov.report(
                CoverageReportArgs(format, destination), environment
            )
        lcov = context.fs.read(output / "workspace-lcov.info")
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
        database = test_environment(context)
        environment = {**coverage_environment(context), **database, "REVAER_NATIVE_IT": "1"}
        # A compile/test failure must not leave yesterday's reports looking like
        # this run's output. Raw profiles remain engine-owned diagnostic data.
        reset_reports(context)
        context.tools.cargo_llvm_cov.clean(environment)
        context.tools.cargo_llvm_cov.collect(environment)
        return CoverageReport.run(context)
