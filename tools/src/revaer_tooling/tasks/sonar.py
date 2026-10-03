"""One scanner invocation and complete, task-bound Sonar result evidence.

Every producer invalidates its own previous success marker before execution.
Failed scans retain their log; failed result verification retains the last API
responses. Packaging keeps the full submitted binary report, not an excerpt.
"""

import json
import re
import tarfile
import tempfile
import tomllib
from pathlib import Path

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.git import ScmArgs
from ..external.sonar import ScanArgs
from ..json_data import JsonObject
from ..sonar.inputs import (
    NATIVE_SOURCE,
    verify_bootstrap_coverage,
    verify_lcov,
    verify_native_coverage,
    verify_python_coverage,
)
from ..sonar.results import PublishedResult, analysis_id, published_result, task_id
from .base import Task
from .native import verify_compilation_database
from .python_coverage import authored_python
from .workflows import SonarPolicy

RESULT_NAMES = ("ce-task", "measures", "quality-gate", "issues", "hotspots")


class SonarVerifyInputs(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        settings = context.settings.sonar
        directory = _evidence_path(context, settings.coverage)
        names = (
            "lcov.info",
            "js-lcov.info",
            "script-coverage.xml",
            "python.xml",
            "compile_commands.json",
            "llvm-cov.txt",
        )
        reports: dict[str, str] = {}
        for name in names:
            path = directory / name
            _require_nonempty(path)
            reports[name] = context.fs.read(path)
        verify_lcov(reports["lcov.info"], reports["js-lcov.info"])
        verify_bootstrap_coverage(reports["script-coverage.xml"])
        verify_python_coverage(reports["python.xml"], context.root, authored_python(context))
        verify_compilation_database(
            reports["compile_commands.json"],
            context.root,
            context.root / NATIVE_SOURCE,
            directory / "cxxbridge/include",
        )
        verify_native_coverage(reports["llvm-cov.txt"], required=settings.native_required)
        return TaskResult("Sonar coverage and native analyzer inputs verified")


class SonarPrepareScm(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        settings = context.settings.sonar
        evidence = _evidence_path(context, settings.scm_evidence)
        with context.fs.lock(_scan_lock(context)):
            context.fs.remove_owned(evidence, context.root)
            document = context.tools.git.prepare_scm(
                ScmArgs(settings.base_sha, settings.base_ref, settings.head_sha)
            )
            context.fs.write(evidence, document)
        return TaskResult(
            "Exact Sonar SCM context verified; " + str(evidence.relative_to(context.root))
        )


def _evidence_path(context: Context, configured: Path) -> Path:
    path = context.root / configured
    if path.is_symlink() or not path.resolve().is_relative_to(context.root.resolve()):
        raise ToolingError("Sonar evidence must stay inside the selected checkout")
    return path


def _require_nonempty(path: Path) -> None:
    if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
        raise ToolingError(f"Required Sonar evidence is missing or empty: {path}")


def _scan_lock(context: Context) -> Path:
    directory = _evidence_path(context, Path("artifacts/sonar"))
    context.fs.mkdir(directory)
    return directory / ".operation.lock"


class SonarPrepareSources(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        # These generated mirrors sit under scanned source directories. Removing
        # them preserves authored scope without configuring scanner exclusions.
        names = (
            "tests/node_modules",
            "release/node_modules",
            "tests/support/api/schema.ts",
            "crates/revaer-ui/dist-serve",
            "tests/test-results",
            "tests/playwright-report",
        )
        tracked = context.tools.git.files()
        for name in names:
            if any(path == name or path.startswith(name + "/") for path in tracked):
                raise ToolingError(
                    "Sonar source preparation refuses to remove tracked content: " + name
                )
        with context.fs.lock(_scan_lock(context)):
            retained = _evidence_path(context, Path("artifacts/sonar/browser-evidence"))
            for name in ("test-results", "playwright-report"):
                source = context.root / "tests" / name
                if source.exists():
                    context.fs.remove_owned(retained / name, context.root)
                    context.fs.copy_tree(source, retained / name)
            for name in names:
                context.fs.remove_owned(context.root / name, context.root)
        return TaskResult(
            "Sonar source inputs prepared; browser evidence retained in artifacts/sonar"
        )


class SonarScan(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        settings = context.settings.sonar
        log = _evidence_path(context, settings.scanner_log)
        report = _evidence_path(context, settings.report_task)
        scanner_work = _evidence_path(context, Path(".scannerwork"))
        with context.fs.lock(_scan_lock(context)):
            for path in (
                log,
                report,
                scanner_work / "scanner-report",
                scanner_work / "scanner-report.tar.xz",
            ):
                context.fs.remove_owned(path, context.root)
            # Direct CLI invocations receive the same source-scope checks as
            # CI. Validate after invalidation so rejected configuration cannot
            # leave a previous success marker looking like this run's result.
            SonarPolicy.run(context)
            version = tomllib.loads(context.fs.read(context.root / "tools/versions.toml"))["sonar"][
                "version"
            ]
            context.tools.sonar_scanner.verify(version)
            context.tools.sonar_scanner.scan(ScanArgs(log, settings.scanner_token))
            _require_nonempty(log)
            normalized = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", context.fs.read(log))
            if "WARN" in normalized:
                raise ToolingError(f"Sonar scanner emitted WARN output; review {log}")
            _require_nonempty(report)
            identifier = task_id(context.fs.read(report))
            _report_files(scanner_work / "scanner-report")
        return TaskResult("Authoritative Sonar task: " + identifier)


def _report_files(directory: Path) -> tuple[Path, ...]:
    if directory.is_symlink() or not directory.is_dir():
        raise ToolingError("The complete submitted Sonar scanner report is missing")
    paths = tuple(sorted(directory.rglob("*")))
    if any(path.is_symlink() or not (path.is_dir() or path.is_file()) for path in paths):
        raise ToolingError("Sonar report must contain only regular files and directories")
    files = tuple(path for path in paths if path.is_file())
    if not any(path.stat().st_size > 0 for path in files):
        raise ToolingError("The submitted Sonar scanner report has no nonempty evidence")
    return files


def package_report(context: Context) -> Path:
    """Publish a full archive only after all required evidence has been retained."""
    settings = context.settings.sonar
    scanner_work = _evidence_path(context, Path(".scannerwork"))
    archive = scanner_work / "scanner-report.tar.xz"
    context.fs.remove_owned(archive, context.root)
    _require_nonempty(_evidence_path(context, settings.scanner_log))
    report = _evidence_path(context, settings.report_task)
    _require_nonempty(report)
    task_id(context.fs.read(report))
    files = _report_files(scanner_work / "scanner-report")
    with tempfile.NamedTemporaryFile(prefix=".report-", dir=scanner_work, delete=False) as stream:
        temporary = Path(stream.name)
    try:
        with tarfile.open(temporary, "w:xz") as target:
            for path in files:
                target.add(path, arcname=str(path.relative_to(scanner_work)), recursive=False)
        _require_nonempty(temporary)
        temporary.replace(archive)
    finally:
        temporary.unlink(missing_ok=True)
    return archive


class SonarPackageReport(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with context.fs.lock(_scan_lock(context)):
            archive = package_report(context)
        return TaskResult(
            "Retained complete Sonar report: " + str(archive.relative_to(context.root))
        )


def _fetch_result(context: Context, identifier: str, records: dict[str, JsonObject]) -> None:
    settings = context.settings.sonar
    api = context.tools.sonar_api
    records["ce-task"] = api.get("ce/task", {"id": identifier})
    analysis = analysis_id(records["ce-task"], identifier, settings.project_key)
    scope = {"pullRequest": settings.pull_request} if settings.pull_request else {}
    records["measures"] = api.get(
        "measures/component",
        {
            **scope,
            "component": settings.project_key,
            "metricKeys": "coverage,line_coverage,lines_to_cover,uncovered_lines",
        },
    )
    records["quality-gate"] = api.get("qualitygates/project_status", {"analysisId": analysis})
    new_code = {"inNewCodePeriod": "true"} if settings.pull_request else {}
    records["issues"] = api.get(
        "issues/search",
        {
            **scope,
            **new_code,
            "componentKeys": settings.project_key,
            "resolved": "false",
            "ps": "1",
        },
    )
    # Deliberately omit a hotspot status filter. A reviewed or acknowledged
    # hotspot is still a current hotspot under the repository's strict gate.
    records["hotspots"] = api.get(
        "hotspots/search",
        {
            **scope,
            **new_code,
            "projectKey": settings.project_key,
            "ps": "1",
        },
    )


def _verify_result(context: Context, identifier: str) -> PublishedResult:
    settings = context.settings.sonar
    evidence = _evidence_path(context, settings.result_evidence)
    context.fs.mkdir(evidence)
    for name in RESULT_NAMES:
        context.fs.remove_owned(evidence / (name + ".json"), context.root)
    records: dict[str, JsonObject] = {}
    try:
        for attempt in range(1, settings.result_attempts + 1):
            records.clear()
            try:
                _fetch_result(context, identifier, records)
                return published_result(records, identifier, settings.project_key)
            except ToolingError:
                if attempt == settings.result_attempts:
                    raise
                context.emit(
                    "Sonar published result is not ready or strict; "
                    f"retrying {attempt}/{settings.result_attempts}"
                )
                context.tools.sonar_api.pause(settings.result_delay)
        raise ToolingError("Sonar result retry count must be positive")
    finally:
        # Preserve partial responses too. Never carry an earlier attempt's
        # successful object into the current attempt's evidence set.
        for name, record in records.items():
            context.fs.write(
                evidence / (name + ".json"), json.dumps(record, indent=2) + "\n", 0o600
            )


class SonarVerifyResult(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        settings = context.settings.sonar
        with context.fs.lock(_scan_lock(context)):
            package_report(context)
            identifier = task_id(context.fs.read(_evidence_path(context, settings.report_task)))
            result = _verify_result(context, identifier)
        scope = f"PR {settings.pull_request}" if settings.pull_request else "main"
        return TaskResult(
            f"Sonar {scope} verified: task={identifier} analysis={result.analysis} "
            f"coverage={result.coverage}% lines_to_cover={result.lines_to_cover} "
            "unresolved_issues=0 current_hotspots=0"
        )
