"""Exercise bootstrap behavior and retain raw upstream measurements without a coverage target."""

from ..context import Context, TaskResult
from ..external.python import PytestArgs
from .base import Task


class ScriptCoverage(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        output = context.root / "coverage/script-coverage.xml"
        cases = context.root / "coverage/scripts/cases"
        context.fs.mkdir(output.parent)
        with context.fs.lock(output.parent / ".bootstrap-coverage.lock"):
            context.fs.remove_owned(output, context.root)
            context.fs.remove_owned(cases, context.root)
            context.tools.bash.verify()
            context.tools.kcov.verify()
            temporary = context.fs.temporary_directory(output.parent, ".bootstrap-pytest-")
            try:
                # Keep this invocation out of earlier pytest sessions' cleanup pool.
                context.tools.python.pytest(
                    PytestArgs(
                        (
                            "tools/tests/test_bootstrap.py::test_measured_uv_bootstrap",
                            "--basetemp",
                            str(temporary),
                        )
                    ),
                    env={"REVAER_BOOTSTRAP_COVERAGE_DIR": str(cases)},
                )
            finally:
                context.fs.remove_owned(temporary, context.root)
        return TaskResult("Bootstrap behavior passed; raw kcov reports: coverage/scripts/cases")
