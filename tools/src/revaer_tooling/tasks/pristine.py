"""Generate or validate the complete pristine catalog through a constrained owner."""

import hashlib
from pathlib import Path

from ..context import Context, TaskResult
from ..database.contract import read_owned
from ..database.pristine.snapshot import Snapshot
from ..database.pristine.spec import Specification
from ..database.pristine.workspace import EXPECTED, Workspace, provenance, provision, publish
from ..errors import ToolingError
from ..external.python import PytestArgs
from .base import Task


def run(context: Context, *, validate: bool) -> TaskResult:
    workspace = Workspace.load(context.root, context.fs)
    context.fs.mkdir(workspace.output)
    with context.fs.lock(workspace.output / ".operation.lock"):
        for name in ("postgres-pristine-16.14.tsv", "provenance.json"):
            context.fs.remove_owned(workspace.output / name, context.root)
        spec = Specification.load(
            context.fs, Path(__file__).parents[1] / "database/pristine/spec.json"
        )
        native = context.tools.proof_databases
        native.docker.verify_image(workspace.postgres)
        image = native.docker.image_identity(workspace.postgres)
        with native.open(
            workspace, user="postgres", database="postgres", logical_wal=True
        ) as connection:
            database, owner = provision(connection, connection.container[:16])
            snapshot = Snapshot(connection, database, owner, spec)
            source = snapshot.read()
            evidence = provenance(workspace, snapshot, source, image, context.invoked_at)
        # Keep generated evidence even when the committed snapshot differs. It
        # describes this native read; validation still exits unsuccessfully.
        publish(workspace, context.fs, source, evidence)
        if validate and source != read_owned(context.root, context.fs, EXPECTED):
            raise ToolingError("Committed pristine snapshot differs from the pinned image")
    lines = source.count(b"\n")
    if lines > 9_999:
        context.emit(
            "Snapshot exceeds the 9,999-line PR ceiling; do not commit an incomplete snapshot"
        )
    return TaskResult(
        f"Pristine catalog {'validated' if validate else 'generated'}: "
        f"{lines} lines, {len(source)} bytes, SHA-256 {hashlib.sha256(source).hexdigest()}"
    )


class PristineGenerate(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return run(context, validate=False)


class PristineValidate(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return run(context, validate=True)


class PristineTest(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.python.pytest(PytestArgs(("tools/tests/test_pristine.py",)))
        return TaskResult("Pristine catalog mutation and constrained-owner proofs passed")
