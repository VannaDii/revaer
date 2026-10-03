"""Static CLI entry points for the reviewed, checkout-local database transition."""

import json
from collections.abc import Iterator
from contextlib import contextmanager

from ..context import Context, TaskResult
from ..database.candidate import CandidateBuilder
from ..database.changed_lines import ChangedLineGuard, Scope
from ..database.contract import INITIALIZER, Contract, Phase
from ..database.final_sql import FinalSql
from ..errors import ToolingError
from .base import Task


@contextmanager
def operation(context: Context) -> Iterator[Contract]:
    contract = Contract.load(context.root, context.fs)
    context.fs.mkdir(contract.output)
    with context.fs.lock(contract.output / ".operation.lock"):
        yield contract


class DatabaseFreeze(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with operation(context) as contract:
            corpus = contract.freeze()
        return TaskResult(
            f"Frozen database corpus verified: {corpus.file_count} files, {corpus.sha256}"
        )


class DatabaseCandidate(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with operation(context) as contract:
            CandidateBuilder(
                contract, context.tools.proof_databases, context.tools.sqlx, context.emit
            ).generate()
        return TaskResult(
            "Frozen candidate passed native replay and schema parity; evidence retained"
        )


class DatabasePrefix(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with operation(context) as contract:
            contract.freeze()
            candidate = contract.candidate()
            if contract.review.phase == Phase.FINALIZATION:
                FinalSql(contract).verify(contract.initializer(), candidate)
            else:
                contract.verify_prefix(candidate)
        return TaskResult("Database initializer matches the reviewed transition contract")


class DatabaseFinalize(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with operation(context) as contract:
            original = contract.initializer()
            generated = FinalSql(contract).generate(original, contract.candidate())
            if original != generated:
                context.fs.write_bytes(context.root / INITIALIZER, generated)
        return TaskResult("Generated the approved SQL deltas; reviewed hash pins were not changed")


def changed_lines(context: Context, scope: Scope) -> TaskResult:
    if context.options.base is None or context.options.head is None:
        raise ToolingError("Changed-line validation requires explicit --base and --head revisions")
    contract = Contract.load(context.root, context.fs)
    result = ChangedLineGuard(contract, context.tools.git, context.tools.github).verify(
        scope, context.options.base, context.options.head
    )
    if result.asset_exception is not None:
        context.emit(
            "Approved asset exception " + json.dumps(result.asset_exception, separators=(",", ":"))
        )
    return TaskResult(
        f"{scope} diff has {result.counts.additions} additions and "
        f"{result.counts.deletions} deletions ({result.counts.total}/{result.maximum})"
    )


class StackChangedLines(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return changed_lines(context, Scope.STACK)


class AssemblyChangedLines(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return changed_lines(context, Scope.ASSEMBLY)
