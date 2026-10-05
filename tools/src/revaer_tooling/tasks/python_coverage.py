"""Combine actual tooling and E2E execution for one complete Python report."""

from pathlib import Path

from ..context import Context, TaskResult
from ..e2e.coverage import verify_analysis_run
from ..e2e.database import uses_single_init
from ..errors import ToolingError
from ..external.python import PythonCoverageArgs
from ..json_data import decode, object_value
from ..sonar.inputs import verify_python_coverage
from .base import Task
from .e2e import RunPaths


def authored_python(context: Context) -> tuple[str, ...]:
    return tuple(
        name
        for name in context.tools.git.files(include_untracked=True)
        if name.endswith(".py") and (context.root / name).is_file()
    )


def coverage_inputs(context: Context) -> tuple[Path, ...]:
    """Select only the completed run's named phases, never glob stale phase data."""
    path = context.root / "tests/test-results/python-e2e-summary.json"
    summary = object_value(decode(context.fs.read(path)))
    expected = context.settings.e2e.phases(media=uses_single_init(context))
    verify_analysis_run(summary, expected)
    inputs = (
        context.root / ".coverage",
        *(context.root / "coverage/e2e" / phase / ".coverage" for phase in expected),
    )
    for item in inputs:
        if item.is_symlink() or not item.is_file() or item.stat().st_size == 0:
            raise ToolingError("Python execution coverage is missing or empty: " + str(item))
    return inputs


class PythonCoverageMerge(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        output = context.root / "coverage"
        if output.is_symlink():
            raise ToolingError("Python coverage directory must not be a symlink")
        context.fs.mkdir(output)
        paths = RunPaths.for_context(context)
        context.fs.mkdir(paths.runtime)
        # Read the E2E summary/data and tooling database while their producers
        # cannot replace them. Always acquire E2E before the output lock.
        with (
            context.fs.lock(paths.runtime / "e2e.lock"),
            context.fs.lock(output / ".python-coverage.lock"),
        ):
            for name in (".coverage", "python.xml", "python.lcov"):
                context.fs.remove_owned(output / name, context.root)
            inputs = coverage_inputs(context)
            sources = authored_python(context)
            if not sources:
                raise ToolingError("Python coverage requires authored source files")
            try:
                context.tools.python.merge_coverage(PythonCoverageArgs(inputs, sources, output))
                verify_python_coverage(
                    context.fs.read(output / "python.xml"), context.root, sources
                )
            except BaseException:
                # Keep raw databases for diagnosis; a partial XML/LCOV pair
                # must not look like a completed analysis report on the next run.
                for name in ("python.xml", "python.lcov"):
                    context.fs.remove_owned(output / name, context.root)
                raise
        return TaskResult("Merged tooling and API/browser Python coverage: coverage/python.xml")
