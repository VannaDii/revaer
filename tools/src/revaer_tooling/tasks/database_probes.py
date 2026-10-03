"""Expose the existing Rust qualifications; their caller owns the proof databases."""

from pathlib import Path

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.rust import CargoArgs, CargoOperation, DatabaseProbe, DatabaseProbeArgs
from .base import Task
from .testing import test_environment


def qualify(context: Context, kind: DatabaseProbe, path: Path | None) -> TaskResult:
    if path is None:
        raise ToolingError(
            f"Set REVAER_INGESTION_{kind.upper()}_PROOF to the explicit disposable proof input file"
        )
    # Rust retains authority over the input schema, loopback/role/database
    # identity and fresh report paths. No default database or input is invented.
    selected = path if path.is_absolute() else context.root / path
    result = context.tools.cargo.database_probe(DatabaseProbeArgs(kind, selected))
    context.emit(result.stdout.rstrip())
    return TaskResult(f"Database {kind} qualification passed")


class DatabasePoolProbe(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return qualify(context, DatabaseProbe.POOL, context.settings.database.pool_proof)


class DatabaseCancellationProbe(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return qualify(
            context, DatabaseProbe.CANCELLATION, context.settings.database.cancellation_proof
        )


class DatabaseBaselineRead(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.execute(
            CargoArgs(CargoOperation.TEST, test_filter="baseline::"), test_environment(context)
        )
        return TaskResult("Database baseline read tests passed")
