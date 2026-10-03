"""Exercise the real root bootstrap and publish complete, measured shell evidence."""

from ..bootstrap.coverage import merge_records
from ..context import Context, TaskResult
from ..external.python import PytestArgs
from ..sonar.inputs import verify_bootstrap_coverage
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
            context.tools.python.pytest(
                PytestArgs(("tools/tests/test_bootstrap.py::test_measured_uv_bootstrap",)),
                env={"REVAER_BOOTSTRAP_COVERAGE_DIR": str(cases)},
            )
            document = merge_records(context.fs, cases, (context.root / "setup.sh").read_bytes())
            verify_bootstrap_coverage(document)
            context.fs.write(output, document)
        return TaskResult("Bootstrap execution coverage: coverage/script-coverage.xml")
