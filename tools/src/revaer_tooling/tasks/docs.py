"""Documentation guardrails and the complete, source-bound Pages payload.

mdBook and the Rust indexer remain the content producers. These tasks replace
the workflow's grep, rsync and metadata shell blocks; the pinned Pages action
continues to publish the prepared directory to the existing gh-pages branch.
"""

import re
import tempfile
from datetime import UTC
from pathlib import Path

from ..context import Context, TaskResult
from ..errors import ToolingError
from .automation import outputs, source_values
from .base import Task

SECRET_PATTERN = re.compile(r"AKIA[0-9A-Z]{16}|SECRET_KEY|token=|password=|x-api-key")
FORBIDDEN_SUFFIXES = frozenset((".py", ".ipynb", ".html"))


class DocsGuard(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        failures: list[str] = []
        for path in sorted((context.root / "docs").rglob("*.md")):
            if path.is_symlink() or not path.resolve().is_relative_to(context.root.resolve()):
                raise ToolingError("Documentation source must belong to this checkout")
            for number, line in enumerate(context.fs.read(path).splitlines(), 1):
                if SECRET_PATTERN.search(line):
                    # Report the location without echoing a potential credential.
                    failures.append(
                        f"{path.relative_to(context.root)}:{number}: secret-like pattern"
                    )
        for name in context.tools.git.files():
            path = Path(name)
            if path.parts[0] == "docs" and path.suffix in FORBIDDEN_SUFFIXES:
                failures.append(f"{name}: forbidden committed documentation file type")
        if failures:
            raise ToolingError("Documentation guardrails failed:\n" + "\n".join(failures))
        return TaskResult("Documentation guardrails passed")


class DocsPrepare(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        DocsGuard.run(context)
        destination = context.root / "deploy"
        if destination.is_symlink() or any(
            name == "deploy" or name.startswith("deploy/") for name in context.tools.git.files()
        ):
            raise ToolingError("Documentation deployment output must be an untracked directory")
        if destination.exists() and not (destination / ".deployment-info").is_file():
            raise ToolingError("Existing deploy directory has no deployment ownership record")
        book = context.root / "docs/book"
        manifests = context.root / "docs/llm"
        if not (book / "index.html").is_file() or not (book / "index.html").stat().st_size:
            raise ToolingError("Build documentation before preparing deployment")
        for name in ("manifest.json", "schema.json", "summaries.json"):
            if not (manifests / name).is_file() or not (manifests / name).stat().st_size:
                raise ToolingError("Generate documentation manifests before preparing deployment")
        values = source_values(context)
        values["generated_at"] = context.invoked_at.astimezone(UTC).strftime(
            "%Y-%m-%d %H:%M:%S UTC"
        )
        run = context.settings.workflow
        metadata = (
            f"Generated on: {values['generated_at']}\n"
            f"Source commit: {values['sha']}\n"
            f"Workflow run: {run.run_id or 'local'}\n"
            f"Trigger: {run.event_name or 'local'}\n"
        )
        # Finish and validate all copies before removing the previous payload.
        # Replacing the complete tree also removes files deleted from the book.
        with (
            context.fs.lock(context.root / ".rv-docs-deploy.lock"),
            tempfile.TemporaryDirectory(prefix=".rv-docs-", dir=context.root) as temporary,
        ):
            stage = Path(temporary) / "deploy"
            context.fs.copy_tree(book, stage)
            context.fs.remove_owned(stage / "llm", stage)
            context.fs.copy_tree(manifests, stage / "llm")
            cname = context.root / "docs/CNAME"
            if cname.is_symlink():
                raise ToolingError("Documentation CNAME must be a regular file")
            if cname.exists():
                context.fs.copy(cname, stage / "CNAME")
            context.fs.write(stage / ".deployment-info", metadata)
            context.fs.remove_owned(destination, context.root)
            stage.replace(destination)
        return outputs(context, values)
