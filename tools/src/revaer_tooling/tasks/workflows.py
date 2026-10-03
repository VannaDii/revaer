"""Read canonical workflow inputs and apply the structured policy contracts."""

from pathlib import Path

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..json_data import decode, object_value
from ..policy.formats import Document, yaml_document
from ..policy.image_workflows import image_workflow_findings
from ..policy.release_workflows import release_workflow_findings
from ..policy.required_checks import required_check_findings
from ..policy.scanner_workflows import scanner_workflow_findings
from ..policy.setup_workflows import setup_workflow_findings
from ..policy.sonar import sonar_findings
from .base import Task


def source_inventory(context: Context) -> tuple[str, ...]:
    """Include new authored files while excluding tracked deletions and Git internals."""
    return tuple(
        name
        for name in context.tools.git.files(include_untracked=True)
        if (context.root / name).is_file()
    )


def require_no_findings(failures: list[str], label: str) -> None:
    if failures:
        raise ToolingError(label + ":\n" + "\n".join(failures))


class SonarPolicy(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        directory = context.root / ".sonar-test-scope"
        if (
            directory.is_symlink()
            or not directory.is_dir()
            or ".sonar-test-scope/.gitkeep" not in context.tools.git.files()
        ):
            raise ToolingError("Sonar test-scope sentinel must be a committed empty directory")
        sentinel = {}
        for path in directory.rglob("*"):
            if path.is_symlink() or not path.is_file():
                raise ToolingError("Sonar test-scope sentinel may contain only the empty .gitkeep")
            sentinel[str(path.relative_to(directory))] = path.read_bytes()
        require_no_findings(
            sonar_findings(
                context.fs.read(context.root / "sonar-project.properties"),
                source_inventory(context),
                sentinel,
            ),
            "Sonar policy guardrail failed",
        )
        return TaskResult("Sonar scope and analyzer policy passed")


class WorkflowPolicy(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        documents: dict[str, Document] = {}
        for name in source_inventory(context):
            if name.startswith((".github/workflows/", ".github/actions/")) and Path(
                name
            ).suffix in (".yml", ".yaml"):
                documents[name] = yaml_document(context.fs.read(context.root / name), name)
        contexts = tuple(
            line
            for line in context.fs.read(context.root / "config/required-pr-checks.txt").splitlines()
            if line and not line.startswith("#")
        )
        matrix = object_value(
            decode(context.fs.read(context.root / ".github/matrices/build-images.json"))
        )
        failures = [
            *setup_workflow_findings(documents),
            *scanner_workflow_findings(documents),
            *required_check_findings(
                documents, contexts, (context.root / "config/database-rebaseline.env").is_file()
            ),
            *image_workflow_findings(documents, matrix),
            *release_workflow_findings(documents),
        ]
        require_no_findings(failures, "Workflow contract guardrail failed")
        return TaskResult("Required-check, scanner, image and release workflow contracts passed")
