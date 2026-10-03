"""Count the complete exact-commit diff before considering any binary exception."""

import re
from dataclasses import dataclass
from enum import StrEnum

from ..errors import ToolingError
from ..external.git import DiffArgs, Git
from ..external.github import GitHub
from ..json_data import JsonObject
from .asset_history import AssetHistory
from .assets import AssetException
from .contract import Contract


class Scope(StrEnum):
    STACK = "stack"
    ASSEMBLY = "init-assembly"


@dataclass(frozen=True)
class Counts:
    additions: int
    deletions: int
    binaries: tuple[str, ...]
    paths: tuple[str, ...]

    @property
    def total(self) -> int:
        return self.additions + self.deletions


def totals(source: str) -> Counts:
    if not source:
        return Counts(0, 0, (), ())
    records = source.split("\0")
    if records.pop() != "":
        raise ToolingError("Git changed-line output is incomplete")
    additions, deletions = 0, 0
    paths: dict[str, None] = {}
    binaries = []
    for record in records:
        fields = record.split("\t", 2)
        if len(fields) != 3 or not fields[2] or fields[2] in paths:
            raise ToolingError("Git changed-line paths are missing or duplicated")
        added, removed, path = fields
        paths[path] = None
        if added == removed == "-":
            binaries.append(path)
        elif re.fullmatch(r"[0-9]+", added) and re.fullmatch(r"[0-9]+", removed):
            additions += int(added)
            deletions += int(removed)
        else:
            raise ToolingError("Git reported a binary or uncountable changed-line entry")
    return Counts(additions, deletions, tuple(binaries), tuple(paths))


@dataclass(frozen=True)
class LineProof:
    counts: Counts
    maximum: int
    asset_exception: JsonObject | None


@dataclass(frozen=True)
class ChangedLineGuard:
    contract: Contract
    git: Git
    github: GitHub

    def verify(self, scope: Scope, base: str, head: str) -> LineProof:
        refs = DiffArgs(self.git.exact_commit(base), self.git.exact_commit(head))
        counts = totals(self.git.numstat(refs))
        maximum = (
            self.contract.review.stack_limit
            if scope == Scope.STACK
            else self.contract.review.assembly_limit
        )
        if counts.total > maximum:
            raise ToolingError(
                f"{scope} diff has {counts.total} changed lines; maximum is {maximum}"
            )
        evidence = None
        if counts.binaries:
            if scope != Scope.STACK:
                raise ToolingError("Git reported a binary or uncountable changed-line entry")
            evidence = AssetException(self.contract, self.git, AssetHistory(self.github)).verify(
                refs, counts.binaries, counts.paths
            )
        return LineProof(counts, maximum, evidence)
