"""Real Git and adversarial evidence bind ASSET-1 to its exact deletion set."""

import json
import os
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import COMMANDS, make_context, parser
from revaer_tooling.context import Options
from revaer_tooling.database.asset_history import AssetHistory
from revaer_tooling.database.assets import (
    IMPLEMENTATION_PATHS,
    INVENTORY_PATH,
    INVENTORY_SHA256,
    AssetException,
    Deletion,
    inventory,
)
from revaer_tooling.database.changed_lines import ChangedLineGuard, Counts, Scope, totals
from revaer_tooling.database.contract import Contract
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.git import CommitFileArgs, DiffArgs, Git
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.process import ProcessRunner
from test_asset_history import REFS, Provider, page
from test_database_contract import contract_root
from test_git import git

__all__ = ["contract_root"]

ROOT = Path(__file__).parents[2]
FIXTURES = Path(__file__).parent / "fixtures/assets"


@pytest.fixture
def asset_contract(contract_root: Path) -> Contract:
    fs = FileSystem()
    config = contract_root / "config/database-rebaseline.env"
    config.write_text(
        config.read_text()
        .replace("STACK_CHANGED_LINE_MAX=9\n", "STACK_CHANGED_LINE_MAX=9999\n")
        .replace("INIT_ASSEMBLY_CHANGED_LINE_MAX=5\n", "INIT_ASSEMBLY_CHANGED_LINE_MAX=8500\n")
    )
    for name in (*IMPLEMENTATION_PATHS, INVENTORY_PATH):
        # Existing decision text is historical fixture input. Synthetic provider
        # responses below cannot establish acceptance for an actual PR.
        source = FIXTURES / name if (FIXTURES / name).exists() else ROOT / name
        fs.write_bytes(contract_root / name, source.read_bytes())
    return Contract.load(contract_root, fs)


@pytest.fixture(scope="module")
def blobs() -> dict[str, bytes]:
    document = json.loads((FIXTURES / INVENTORY_PATH).read_text())
    native = Git("git", ProcessRunner(lambda message: None), ROOT, os.environ)
    result: dict[str, bytes] = {}
    for entry in document["binaryDeletions"]:
        oid = entry["priorBlobOid"]
        if oid not in result:
            result[oid] = native.blob(oid)
    return result


class AssetGit(Git):
    def __init__(self, contract: Contract, blobs: dict[str, bytes]) -> None:
        super().__init__("git", ProcessRunner(lambda message: None), contract.root, os.environ)
        self.objects = blobs
        self.entries = inventory(contract)
        self.numstat_output = "3\t4\ttext.md\0" + "".join(
            f"-\t-\t{item.path}\0" for item in self.entries
        )
        self.raw_output = f":000000 100644 {'0' * 40} {'a' * 40} A\0text.md\0" + "".join(
            f"{item.raw_deletion}\0{item.path}\0" for item in self.entries
        )
        self.ancestor = True
        self.changed_blob = False
        self.missing_file: str | None = None

    def exact_commit(self, ref: str) -> str:
        return ref

    def numstat(self, args: DiffArgs) -> str:
        return self.numstat_output

    def raw_diff(self, args: DiffArgs) -> str:
        return self.raw_output

    def is_ancestor(self, args: DiffArgs) -> bool:
        return self.ancestor

    def blob(self, oid: str) -> bytes:
        return self.objects[oid] + (b"changed" if self.changed_blob else b"")

    def committed_file(self, args: CommitFileArgs) -> bytes:
        if args.path == self.missing_file:
            raise ToolingError("Missing committed file")
        return (self.root / args.path).read_bytes()


def test_exact_inventory_and_text_limit(asset_contract: Contract, blobs: dict[str, bytes]) -> None:
    native, provider = AssetGit(asset_contract, blobs), Provider([page()])
    guard = ChangedLineGuard(asset_contract, native, provider)
    proof = guard.verify(Scope.STACK, REFS.base, REFS.head)
    assert (proof.counts.additions, proof.counts.deletions, proof.counts.total, proof.maximum) == (
        3,
        4,
        7,
        9999,
    )
    assert proof.asset_exception is not None
    assert proof.asset_exception["binary_deletions"] == 213
    assert proof.asset_exception["original_bytes"] == 21_064_113
    assert proof.asset_exception["inventory_sha256"] == INVENTORY_SHA256
    native.numstat_output = native.numstat_output.replace("3\t4\t", "9999\t0\t")
    provider.pages.append(json.dumps(page()))
    assert guard.verify(Scope.STACK, REFS.base, REFS.head).counts.total == 9999
    native.numstat_output = native.numstat_output.replace("9999\t0\t", "10000\t0\t")
    requests = len(provider.requests)
    with pytest.raises(ToolingError, match="10000 changed lines"):
        guard.verify(Scope.STACK, REFS.base, REFS.head)
    assert len(provider.requests) == requests


@pytest.mark.parametrize(
    "change",
    (
        "missing",
        "extra",
        "wrong",
        "status",
        "mode",
        "oid",
        "missing_raw",
        "truncated",
        "metadata",
        "missing_text",
        "duplicate",
        "bytes",
        "ancestor",
    ),
)
def test_binary_or_raw_diff_drift_fails_before_provider(
    asset_contract: Contract,
    blobs: dict[str, bytes],
    change: str,
) -> None:
    native, provider = AssetGit(asset_contract, blobs), Provider([])
    first = native.entries[0]
    match change:
        case "missing":
            native.numstat_output = native.numstat_output.replace(f"-\t-\t{first.path}\0", "")
        case "extra":
            native.numstat_output += "-\t-\textra.bin\0"
        case "wrong":
            native.numstat_output = native.numstat_output.replace(first.path, "wrong.bin")
        case "status":
            native.raw_output = native.raw_output.replace(" D\0", " A\0", 1)
        case "mode":
            native.raw_output = native.raw_output.replace(":100644", ":120000", 1)
        case "oid":
            native.raw_output = native.raw_output.replace(first.blob, "f" * 40, 1)
        case "missing_raw":
            native.raw_output = native.raw_output.split("\0", 2)[2]
        case "truncated":
            native.raw_output = native.raw_output[:-1]
        case "metadata":
            native.raw_output = native.raw_output.replace(":000000", ":broken", 1)
        case "missing_text":
            native.numstat_output = native.numstat_output.removeprefix("3\t4\ttext.md\0")
        case "duplicate":
            native.raw_output += "\0".join(native.raw_output.split("\0")[:2]) + "\0"
        case "bytes":
            native.changed_blob = True
        case "ancestor":
            native.ancestor = False
    with pytest.raises(ToolingError, match="ASSET-1"):
        ChangedLineGuard(asset_contract, native, provider).verify(Scope.STACK, REFS.base, REFS.head)
    assert not provider.requests


@pytest.mark.parametrize("path", IMPLEMENTATION_PATHS)
def test_every_guard_transport_and_decision_file_must_be_committed(
    asset_contract: Contract,
    blobs: dict[str, bytes],
    path: str,
) -> None:
    native, provider = AssetGit(asset_contract, blobs), Provider([])
    native.missing_file = path
    with pytest.raises(ToolingError, match="checked head"):
        ChangedLineGuard(asset_contract, native, provider).verify(Scope.STACK, REFS.base, REFS.head)
    assert not provider.requests


@pytest.mark.parametrize(
    "source",
    (
        "1\t2\tpath",
        "1\t2\t\0",
        "-1\t2\tpath\0",
        "-\t2\tpath\0",
        "1\t2\tx\0" * 2,
        "\u0661\t2\tx\0",
    ),
)
def test_malformed_numstat_cannot_hide_uncountable_lines(source: str) -> None:
    with pytest.raises(ToolingError, match="Git"):
        totals(source)


def test_empty_diff_and_unusual_text_names_preserve_counts() -> None:
    assert totals("") == Counts(0, 0, (), ())
    name = "with\ttab\nand-newline"
    assert totals(f"1\t2\t{name}\0") == Counts(1, 2, (), (name,))


@pytest.mark.parametrize(
    "source",
    (
        "null",
        "[]",
        "{}",
        "invalid",
        '{"binaryDeletions":[],"binaryDeletions":[]}',
    ),
)
def test_malformed_inventory_fails_without_git(asset_contract: Contract, source: str) -> None:
    (asset_contract.root / INVENTORY_PATH).write_text(source)
    with pytest.raises(ToolingError):
        inventory(asset_contract)


def test_every_inventory_field_order_and_duplicate_are_bound(asset_contract: Contract) -> None:
    path = asset_contract.root / INVENTORY_PATH
    original = path.read_text()
    document = json.loads(original)
    for key in document["binaryDeletions"][0]:
        changed = json.loads(original)
        changed["binaryDeletions"][0][key] = 0 if key == "priorBytes" else "changed"
        path.write_text(json.dumps(changed))
        with pytest.raises(ToolingError, match="approved identity"):
            inventory(asset_contract)
    document["binaryDeletions"].reverse()
    path.write_text(json.dumps(document))
    with pytest.raises(ToolingError, match="approved identity"):
        inventory(asset_contract)
    path.write_text(original.replace('"priorBytes":', '"priorBytes":0,"priorBytes":', 1))
    with pytest.raises(ToolingError, match="unique-key JSON"):
        inventory(asset_contract)


def test_real_git_blobs_complete_heads_and_stale_provider(
    asset_contract: Contract,
    blobs: dict[str, bytes],
) -> None:
    root = asset_contract.root
    deletions = inventory(asset_contract)
    for item in deletions:
        asset_contract.fs.write_bytes(root / item.path, blobs[item.blob])
    text = root / "counted.txt"
    text.write_text("one\n")
    git(root, "init", "--initial-branch=main")
    git(root, "config", "user.name", "Revaer test")
    git(root, "config", "user.email", "revaer-test@example.invalid")
    git(root, "add", ".")
    git(root, "-c", "commit.gpgsign=false", "commit", "-m", "Stage fixture inputs")
    native = Git("git", ProcessRunner(lambda message: None), root, os.environ)
    base = native.exact_commit("HEAD")
    for item in deletions:
        (root / item.path).unlink()
    text.write_text("one\ntwo\n")
    git(root, "add", "--all")
    git(root, "-c", "commit.gpgsign=false", "commit", "-m", "Delete exact fixture inventory")
    head = native.exact_commit("HEAD")
    refs = DiffArgs(base, head)
    provider = Provider([page(refs=refs)])
    guard = ChangedLineGuard(asset_contract, native, provider)
    result = guard.verify(Scope.STACK, base, head)
    assert result.counts.total == 1
    assert result.asset_exception is not None
    assert result.asset_exception["binary_deletions"] == 213
    with pytest.raises(ToolingError, match="binary or uncountable"):
        guard.verify(Scope.ASSEMBLY, base, head)
    text.write_text("one\ntwo\nthree\n")
    git(root, "add", "counted.txt")
    git(root, "-c", "commit.gpgsign=false", "commit", "-m", "Count later text change")
    next_head = native.exact_commit("HEAD")
    provider.pages.append(json.dumps(page(refs=refs)))
    with pytest.raises(ToolingError, match="refs"):
        guard.verify(Scope.STACK, base, next_head)
    provider.pages.append(json.dumps(page(refs=DiffArgs(base, next_head))))
    assert guard.verify(Scope.STACK, base, next_head).counts.total == 2
    (root / IMPLEMENTATION_PATHS[0]).write_text("unpublished guard")
    with pytest.raises(ToolingError, match="checked head"):
        guard.verify(Scope.STACK, base, next_head)
    assert not provider.pages


def test_static_commands_require_exact_refs_and_use_the_checkout(
    asset_contract: Contract,
    blobs: dict[str, bytes],
) -> None:
    context = replace(make_context(Options()), root=asset_contract.root)
    native, provider = AssetGit(asset_contract, blobs), Provider([])
    native.numstat_output = "1\t2\tordinary\0"
    context = replace(context, tools=replace(context.tools, git=native, github=provider))
    resolved = replace(context, options=Options(base=REFS.base, head=REFS.head))
    for name in ("stack-changed-lines", "db-init-assembly-changed-lines"):
        with pytest.raises(ToolingError, match="explicit --base"):
            COMMANDS[name](context)
        parsed = parser().parse_args([name, "--base", REFS.base, "--head", REFS.head])
        assert (parsed.base, parsed.head) == (REFS.base, REFS.head)
        assert "1 additions and 2 deletions" in COMMANDS[name](resolved).message
    native.numstat_output = "8501\t0\tordinary\0"
    with pytest.raises(ToolingError, match="8501 changed lines"):
        COMMANDS["db-init-assembly-changed-lines"](resolved)
    assert not provider.requests


def test_blob_validation_requires_original_size_and_digest(
    asset_contract: Contract,
    blobs: dict[str, bytes],
) -> None:
    native = AssetGit(asset_contract, blobs)
    exception = AssetException(asset_contract, native, AssetHistory(Provider([])))
    original = native.entries[0]
    for changed in (replace(original, size=original.size + 1), replace(original, sha256="0" * 64)):
        with pytest.raises(ToolingError, match="content does not match"):
            exception.verify_objects(
                REFS, (changed,), ("text.md", *(item.path for item in native.entries))
            )


@pytest.mark.parametrize(
    "field,value",
    (
        ("priorBytes", True),
        ("priorBytes", -1),
        ("priorMode", "invalid"),
        ("priorBlobOid", "not-oid"),
    ),
)
def test_deletion_projection_rejects_invalid_metadata(field: str, value: str | int) -> None:
    document = json.loads((FIXTURES / INVENTORY_PATH).read_text())["binaryDeletions"][0]
    document[field] = value
    with pytest.raises(ToolingError):
        Deletion.load(document)
