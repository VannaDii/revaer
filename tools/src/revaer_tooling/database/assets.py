"""Verify the exact binary deletions covered by the existing ASSET-1 exception."""

import hashlib
import json
import re
from dataclasses import dataclass

from ..errors import ToolingError
from ..external.git import CommitFileArgs, DiffArgs, Git
from ..json_data import JsonObject, array_value, decode_unique, object_value, string_value
from .asset_history import AssetHistory, require
from .contract import Contract, owned_path, read_owned

INVENTORY_PATH = "docs/adr/support/585-binary-deletions.json"
INVENTORY_SHA256 = "3e1e4af169880c3aee57e967a01c58f9d4f368bb7ac9b72666622e7ce1918d7a"

# These are the migrated guard, dispatch and transport paths, plus the original
# decision/inventory/instruction files. The checked commit must contain their
# exact local bytes. An unpublished Python implementation cannot qualify a PR.
IMPLEMENTATION_PATHS = (
    "tools/src/revaer_tooling/database/assets.py",
    "tools/src/revaer_tooling/database/asset_history.py",
    "tools/src/revaer_tooling/database/changed_lines.py",
    "tools/src/revaer_tooling/database/contract.py",
    "tools/src/revaer_tooling/external/git.py",
    "tools/src/revaer_tooling/external/github.py",
    "tools/src/revaer_tooling/external/base.py",
    "tools/src/revaer_tooling/external/release.py",
    "tools/src/revaer_tooling/process.py",
    "tools/src/revaer_tooling/filesystem.py",
    "tools/src/revaer_tooling/json_data.py",
    "tools/src/revaer_tooling/errors.py",
    "tools/src/revaer_tooling/assignments.py",
    "tools/src/revaer_tooling/tasks/database_rebaseline.py",
    "tools/src/revaer_tooling/cli.py",
    "tools/src/revaer_tooling/context.py",
    "tools/launcher/src/revaer_launcher/__init__.py",
    "docs/adr/588-first-release-decision-package.md",
    "docs/adr/support/588-decision-details.md",
    ".github/instructions/devops.instructions.md",
    ".github/instructions/python.instructions.md",
)


@dataclass(frozen=True)
class Deletion:
    path: str
    mode: str
    blob: str
    sha256: str
    size: int

    @staticmethod
    def load(row: JsonObject) -> "Deletion":
        if row.keys() != {
            "path",
            "status",
            "priorMode",
            "priorBlobOid",
            "priorSha256",
            "priorBytes",
        }:
            raise ToolingError("ASSET-1 malformed deletion entry")
        path = string_value(row["path"])
        mode, blob = string_value(row["priorMode"]), string_value(row["priorBlobOid"])
        digest, size = string_value(row["priorSha256"]), row["priorBytes"]
        require(row["status"] == "D", "inventory contains a non-deletion")
        require(re.fullmatch(r"[0-7]{6}", mode) is not None, "invalid original file mode")
        require(re.fullmatch(r"[0-9a-f]{40}", blob) is not None, "invalid original blob ID")
        require(re.fullmatch(r"[0-9a-f]{64}", digest) is not None, "invalid original SHA-256")
        if not isinstance(size, int) or isinstance(size, bool) or size < 1:
            raise ToolingError("ASSET-1 invalid original byte count")
        return Deletion(path, mode, blob, digest, size)

    @property
    def raw_deletion(self) -> str:
        return f":{self.mode} 000000 {self.blob} {'0' * 40} D"


def inventory(contract: Contract) -> tuple[Deletion, ...]:
    try:
        document = object_value(
            decode_unique(read_owned(contract.root, contract.fs, INVENTORY_PATH).decode("utf-8"))
        )
        rows = array_value(document["binaryDeletions"])
        # Hash the existing compact JSON array in its original order. Sorting
        # keys/entries or rebuilding it from dataclasses would change its identity.
        content = json.dumps(rows, ensure_ascii=False, allow_nan=False, separators=(",", ":"))
        require(
            len(rows) == 213 and hashlib.sha256(content.encode()).hexdigest() == INVENTORY_SHA256,
            "binary inventory differs from the approved identity",
        )
        result = tuple(Deletion.load(object_value(row)) for row in rows)
        for item in result:
            owned_path(contract.root, item.path)
        require(len({item.path for item in result}) == 213, "duplicate deletion paths")
        return result
    except (KeyError, UnicodeError) as error:
        raise ToolingError("ASSET-1 inventory is malformed") from error


def raw_entries(source: str) -> dict[str, str]:
    fields = source.split("\0")
    require(fields.pop() == "" and len(fields) % 2 == 0, "raw Git diff is incomplete")
    entries = {}
    for header, path in zip(fields[::2], fields[1::2], strict=True):
        require(bool(path) and path not in entries, "raw Git paths are missing or duplicated")
        require(
            re.fullmatch(r":[0-7]{6} [0-7]{6} [0-9a-f]{40} [0-9a-f]{40} [ADMT]", header)
            is not None,
            "raw Git metadata is malformed",
        )
        entries[path] = header
    return entries


@dataclass(frozen=True)
class AssetException:
    contract: Contract
    git: Git
    history: AssetHistory

    def verify(
        self, refs: DiffArgs, binary_paths: tuple[str, ...], changed_paths: tuple[str, ...]
    ) -> JsonObject:
        require(
            len(binary_paths) == len(set(binary_paths)) == 213,
            "binary or uncountable diff is outside the complete inventory",
        )
        deletions = inventory(self.contract)
        require(
            set(binary_paths) == {item.path for item in deletions},
            "binary or uncountable diff is outside the complete inventory",
        )
        require(self.git.is_ancestor(refs), "requires an ancestor base")
        self.verify_objects(refs, deletions, changed_paths)
        for path in (*IMPLEMENTATION_PATHS, INVENTORY_PATH):
            try:
                checked = self.git.committed_file(CommitFileArgs(refs.head, path))
                local = read_owned(self.contract.root, self.contract.fs, path)
            except ToolingError as error:
                raise ToolingError(
                    "ASSET-1 checked head must contain the current guard and decision"
                ) from error
            require(
                checked == local,
                "checked head must contain this guard, decision and scoped instructions; "
                "unpublished candidates cannot receive a canonical pass",
            )
        provider = self.history.verify(refs)
        return {
            "decision": "ADR 588 ASSET-1",
            "inventory_sha256": INVENTORY_SHA256,
            "binary_deletions": len(deletions),
            "original_bytes": sum(item.size for item in deletions),
            "base": refs.base,
            "head": refs.head,
            "provider": provider,
        }

    def verify_objects(
        self, refs: DiffArgs, deletions: tuple[Deletion, ...], changed_paths: tuple[str, ...]
    ) -> None:
        entries = raw_entries(self.git.raw_diff(refs))
        require(set(entries) == set(changed_paths), "raw and changed-line Git paths disagree")
        blobs = {}
        for item in deletions:
            require(
                entries.get(item.path) == item.raw_deletion,
                "deletion path, mode or object does not match",
            )
            if item.blob not in blobs:
                blobs[item.blob] = self.git.blob(item.blob)
            source = blobs[item.blob]
            require(
                len(source) == item.size and hashlib.sha256(source).hexdigest() == item.sha256,
                "binary content does not match",
            )
