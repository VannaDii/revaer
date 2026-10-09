"""Static media tasks preserve fixture ownership and fresh conversion evidence."""

import hashlib
import re
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import replace
from pathlib import Path

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.python import PytestArgs
from ..external.rust import CargoArgs, CargoOperation
from ..media.acquire import acquire
from ..media.catalog import Catalog, load_catalog
from ..media.generate import generate
from ..media.model import FIXTURE_DIRECTORIES, owned_path, regular_file
from ..media.probes import probe_catalog
from .automation import outputs
from .base import Task
from .testing import test_environment


@contextmanager
def fixture_operation(context: Context) -> Iterator[None]:
    target = owned_path(context.root, "target")
    context.fs.mkdir(target)
    with context.fs.lock(target / ".fixture-operation.lock"):
        yield


def protect_tracked(context: Context, paths: tuple[Path, ...]) -> None:
    tracked = tuple(context.root / name for name in context.tools.git.files())
    for path in paths:
        owned_path(context.root, path)
        if any(item == path or item.is_relative_to(path) for item in tracked):
            raise ToolingError(f"Fixture operation would change tracked source: {path}")


class DownloadFixtures(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with fixture_operation(context):
            catalog = load_catalog(context.root, context.settings.fixtures)
            protect_tracked(
                context, tuple(context.root / source.path for source in catalog.sources)
            )
            for source in catalog.sources:
                acquire(
                    source,
                    context.root,
                    context.settings.fixtures,
                    context.tools.curl,
                    context.emit,
                )
        return TaskResult(f"Verified {len(catalog.sources)} locked source fixtures")


class GenerateFixtures(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with fixture_operation(context):
            catalog = load_catalog(context.root, context.settings.fixtures)
            protect_tracked(
                context,
                tuple(context.root / item.path for item in catalog.fixtures if item.generate),
            )
            generate(
                catalog,
                context.root,
                context.settings.fixtures,
                context.tools.ffmpeg,
                context.tools.ffprobe,
                context.emit,
            )
        return TaskResult("Declared media fixtures generated")


def invalidate_report(context: Context) -> Path:
    report = owned_path(context.root, context.settings.fixtures.report)
    protect_tracked(context, (report,))
    context.fs.remove_owned(report, context.root)
    return report


def verify(context: Context, catalog: Catalog) -> TaskResult:
    settings = context.settings.fixtures
    report = invalidate_report(context)
    for source in catalog.sources:
        source.verify(owned_path(context.root, source.path))
    counts = probe_catalog(
        catalog, context.root, settings, context.tools.ffprobe, context.fs, context.emit
    )
    bounded = sum(item.bounded_diagnostics for item in catalog.fixtures)
    if not catalog.sources or counts.snapshots < 1 or bounded < 1:
        raise ToolingError("Fixture verification report counts must be positive")
    context.fs.write(
        report,
        "# Media Conversion Fixture Report\n\n## Summary\n"
        "- Outcome: passed\n"
        f"- Locked source fixtures: {len(catalog.sources)}\n"
        f"- Generated and reviewed fixtures: {counts.snapshots}\n"
        f"- Explicitly bounded diagnostic fixtures: {bounded}\n"
        "- Verification: source integrity, FFmpeg-derived generation, "
        "and canonical ffprobe snapshots\n",
    )
    return TaskResult(f"Source integrity and reviewed probe snapshots verified: {report}")


class VerifyFixtures(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with fixture_operation(context):
            invalidate_report(context)
            return verify(context, load_catalog(context.root, context.settings.fixtures))


class UpdateFixtureProbes(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with fixture_operation(context):
            catalog = load_catalog(context.root, context.settings.fixtures)
            probe_catalog(
                catalog,
                context.root,
                context.settings.fixtures,
                context.tools.ffprobe,
                context.fs,
                context.emit,
                update=True,
            )
        return TaskResult("Reviewed snapshots updated; ADR578 F1 snapshot verified unchanged")


def verify_conversion_report(report: Path) -> None:
    regular_file(report)
    lines = report.read_text(encoding="utf-8").splitlines()
    for name in (
        "Outcome",
        "Pipeline actions",
        "Video transcodes",
        "Audio transcodes",
        "Pipeline failures",
        "Suite failures",
    ):
        selected = tuple(line for line in lines if line.startswith(f"- {name}:"))
        pattern = (
            "passed" if name == "Outcome" else "0" if name.endswith("failures") else "[1-9][0-9]*"
        )
        if len(selected) != 1 or re.fullmatch(rf"- {name}: {pattern}", selected[0]) is None:
            raise ToolingError(f"Media conversion report has missing or invalid {name}")


class TestMediaConversion(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with fixture_operation(context):
            invalidate_report(context)
            catalog = load_catalog(context.root, context.settings.fixtures)
            packages = context.tools.cargo.metadata().workspace_packages
            if "revaer-media-runtime" not in packages:
                # The foundation predates this crate. Its existing contract is
                # fixture verification; never describe that as a transcoding run.
                return verify(context, catalog)
            report = owned_path(context.root, context.settings.fixtures.report)
            temporary = owned_path(context.root, "target/media-conversion/tmp")
            protect_tracked(context, (report, temporary))
            context.fs.remove_owned(report, context.root)
            preparation = replace(
                context,
                settings=replace(
                    context.settings,
                    fixtures=replace(
                        context.settings.fixtures, report=Path(str(report) + ".preparation")
                    ),
                ),
            )
            verify(preparation, catalog)
            context.fs.mkdir(temporary)
            if "revaer-app" in packages:
                result = context.tools.cargo.execute(
                    CargoArgs(
                        CargoOperation.TEST,
                        packages=("revaer-app",),
                        test_filter="media_job_runtime::tests::production_media_job_runtime_executes_and_persists_verified_replacement",
                        include_ignored=True,
                        test_threads=1,
                    ),
                    test_environment(context),
                    capture=True,
                )
                context.emit(result.stdout)
                if not re.search(
                    r"(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;", result.stdout
                ):
                    raise ToolingError(
                        "Media conversion did not execute one production worker test"
                    )
            context.tools.cargo.execute(
                CargoArgs(
                    CargoOperation.TEST,
                    packages=("revaer-media-runtime",),
                    test_binary="media_fixtures",
                    include_ignored=True,
                ),
                env={"REVAER_MEDIA_CONVERSION_REPORT": str(report), "TMPDIR": str(temporary)},
            )
            verify_conversion_report(report)
        return TaskResult(f"Media conversion passed with positive audio/video actions: {report}")


class TestMediaRootCatalog(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.execute(
            CargoArgs(
                CargoOperation.TEST, packages=("revaer-media-runtime",), test_filter="root_catalog"
            )
        )
        return TaskResult("Media root catalog tests passed")


class TestMediaRootContract(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.execute(
            CargoArgs(
                CargoOperation.TEST,
                packages=("revaer-api-models",),
                test_filter="media_root_contract",
            )
        )
        return TaskResult("Media root contract tests passed")


class TestMediaBrokerCodec(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.execute(
            CargoArgs(
                CargoOperation.TEST,
                packages=("revaer-media-runtime",),
                test_filter="process::broker",
            )
        )
        return TaskResult("Media broker codec tests passed")


class CleanFixtures(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with fixture_operation(context):
            paths = tuple(
                owned_path(context.root, f"test-fixtures/{name}") for name in FIXTURE_DIRECTORIES
            )
            protect_tracked(context, paths)
            # Preflight every tree before the first removal. Nested links must
            # not hide data owned by another checkout or an operator.
            for path in paths:
                for item in path.rglob("*"):
                    owned_path(context.root, item)
            for path in paths:
                context.fs.remove_owned(path, context.root)
        return TaskResult("Removed the selected checkout's acquired and derived fixtures")


class CleanTestMedia(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with fixture_operation(context):
            path = owned_path(context.root, "target/media-conversion")
            protect_tracked(context, (path,))
            context.fs.remove_owned(path, context.root)
        return TaskResult("Removed temporary media owned by this checkout")


class TestFixtureScripts(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.python.pytest(
            PytestArgs(
                (
                    "tools/tests/test_media_sources.py",
                    "tools/tests/test_media_probes.py",
                    "tools/tests/test_media_generation.py",
                    "tools/tests/test_media_tasks.py",
                )
            )
        )
        return TaskResult("Media fixture tooling tests passed")


class FixtureCacheKey(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        # Full native version strings remain available in normal process output;
        # preserve the existing cache contract of hashing their first lines.
        ffmpeg = context.tools.ffmpeg.report()
        probe = context.tools.ffprobe.report()
        native = hashlib.sha256(
            ffmpeg.splitlines()[0] + b"\n" + probe.splitlines()[0] + b"\n"
        ).hexdigest()
        paths = [
            owned_path(context.root, context.settings.fixtures.lock),
            owned_path(context.root, context.settings.fixtures.manifest),
        ]
        paths.extend(sorted((context.root / "tools/src/revaer_tooling/media").glob("*.py")))
        paths.extend(
            context.root / path
            for path in (
                "tools/src/revaer_tooling/tasks/media.py",
                "tools/src/revaer_tooling/external/media.py",
                "tools/src/revaer_tooling/external/curl.py",
                "tools/src/revaer_tooling/process.py",
            )
        )
        code = hashlib.sha256()
        for path in paths:
            regular_file(path)
            code.update(
                path.relative_to(context.root).as_posix().encode()
                + b"\0"
                + path.read_bytes()
                + b"\0"
            )
        return outputs(context, {"tool_version": native, "fixtures": code.hexdigest()})
