"""Mutate the frozen contract, source bytes and artifact ownership boundaries."""

import hashlib
import os
from pathlib import Path

import pytest
from revaer_tooling.assignments import literal_assignments
from revaer_tooling.database.contract import (
    BUILD_INPUTS,
    CONFIG,
    INITIALIZER,
    MIGRATIONS,
    OUTPUT,
    Contract,
    Phase,
    owned_path,
)
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem

CANDIDATE = b"CREATE TABLE item (id bigint PRIMARY KEY);\nSELECT '\xe9\x9b\xaa;';\n"
MIGRATION = "0001_initial.sql"


@pytest.fixture
def contract_root(tmp_path: Path) -> Path:
    fs = FileSystem()
    relative = MIGRATIONS + "/" + MIGRATION
    fs.write_bytes(tmp_path / relative, CANDIDATE)
    corpus = hashlib.sha256(relative.encode() + b"\x00" + CANDIDATE + b"\x00").hexdigest()
    fs.write(
        tmp_path / BUILD_INPUTS,
        "POSTGRES_REBASELINE_IMAGE=docker.io/library/postgres@sha256:"
        + "a" * 64
        + "\nPOSTGRES_REBASELINE_VERSION=16.14\nRUST_VERSION=1.96.0\n",
    )
    fs.write(
        tmp_path / CONFIG,
        f"TRANSITION_PHASE=freeze\nMIGRATION_FILE_COUNT=1\nMIGRATION_LAST_FILE={relative}\n"
        f"MIGRATION_CORPUS_SHA256={corpus}\n"
        f"CANDIDATE_SHA256={hashlib.sha256(CANDIDATE).hexdigest()}\n"
        "CANDIDATE_STATEMENT_COUNT=2\nSTACK_CHANGED_LINE_MAX=9\n"
        "INIT_ASSEMBLY_CHANGED_LINE_MAX=5\n",
    )
    fs.write_bytes(tmp_path / OUTPUT / "init-candidate.sql", CANDIDATE, 0o600)
    return tmp_path


def load(root: Path) -> Contract:
    return Contract.load(root, FileSystem())


def change(root: Path, old: str, new: str, name: str = CONFIG) -> None:
    path = root / name
    original = path.read_text()
    assert old in original
    path.write_text(original.replace(old, new))


def test_frozen_inputs_validate_without_rewriting_any_bytes(contract_root: Path) -> None:
    before = {path: path.read_bytes() for path in contract_root.rglob("*") if path.is_file()}
    contract = load(contract_root)
    assert contract.freeze() == contract.review.corpus
    assert contract.review.phase is Phase.FREEZE
    assert contract.postgres.version == "16.14"
    assert contract.verify_candidate(contract.candidate()).count == 2
    assert before == {path: path.read_bytes() for path in before}


@pytest.mark.parametrize("mutation", ("bytes", "name", "extra", "empty", "directory", "link"))
def test_corpus_drift_and_invalid_entries_fail(contract_root: Path, mutation: str) -> None:
    path = contract_root / MIGRATIONS / MIGRATION
    match mutation:
        case "bytes":
            path.write_bytes(CANDIDATE.replace(b"\n", b"\r\n"))
        case "name":
            path.rename(path.with_name("0002_initial.sql"))
        case "extra":
            path.with_name("notes.txt").write_text("unexpected")
        case "empty":
            path.unlink()
        case "directory":
            path.unlink()
            path.mkdir()
        case "link":
            path.unlink()
            path.symlink_to(contract_root / OUTPUT / "init-candidate.sql")
    with pytest.raises(ToolingError):
        load(contract_root).freeze()


@pytest.mark.parametrize(
    "old,new",
    (
        ("TRANSITION_PHASE=freeze", "TRANSITION_PHASE=unknown"),
        ("MIGRATION_FILE_COUNT=1", "MIGRATION_FILE_COUNT=0"),
        ("MIGRATION_FILE_COUNT=1", "MIGRATION_FILE_COUNT=-1"),
        ("MIGRATION_FILE_COUNT=1", "MIGRATION_FILE_COUNT=1_0"),
        ("CANDIDATE_STATEMENT_COUNT=2", "CANDIDATE_STATEMENT_COUNT="),
        ("STACK_CHANGED_LINE_MAX=9", "STACK_CHANGED_LINE_MAX=latest"),
        ("STACK_CHANGED_LINE_MAX=9\n", ""),
        ("STACK_CHANGED_LINE_MAX=9", "STACK_CHANGED_LINE_MAX=9\nUNKNOWN=1"),
    ),
)
def test_contract_malformed_missing_and_unknown_fields_fail(
    contract_root: Path, old: str, new: str
) -> None:
    change(contract_root, old, new)
    with pytest.raises(ToolingError):
        load(contract_root)


@pytest.mark.parametrize(
    "source",
    (
        "A=1\nA=2",
        'A="$HOME"',
        'A="`false`"',
        "A=1 B=2",
        'A="unterminated',
        "wrong=1",
        "A=\x00",
        "A",
    ),
)
def test_literal_manifest_never_expands_or_accepts_ambiguous_input(source: str) -> None:
    with pytest.raises(ToolingError):
        literal_assignments(source, "fixture")


def test_literal_manifest_accepts_comments_and_quoted_package_lists() -> None:
    assert literal_assignments('# comment\nA="one=1 two=2" # more\n', "fixture") == {
        "A": "one=1 two=2"
    }


def test_prefix_requires_exact_bytes_and_a_complete_statement(contract_root: Path) -> None:
    change(contract_root, "TRANSITION_PHASE=freeze", "TRANSITION_PHASE=assembly")
    initializer = contract_root / INITIALIZER
    first = CANDIDATE.splitlines(keepends=True)[0]
    for contents in (b"", first[:-1], first + b"SEL", b"SELECT 1;\n"):
        initializer.write_bytes(contents)
        with pytest.raises(ToolingError):
            load(contract_root).verify_prefix(CANDIDATE)
    initializer.write_bytes(first)
    assert load(contract_root).verify_prefix(CANDIDATE) == first
    assert load(contract_root).freeze().file_count == 1
    with pytest.raises(ToolingError, match="Candidate SHA-256"):
        load(contract_root).verify_prefix(CANDIDATE + b" ")
    change(contract_root, "CANDIDATE_STATEMENT_COUNT=2", "CANDIDATE_STATEMENT_COUNT=3")
    with pytest.raises(ToolingError, match="statement count"):
        load(contract_root).verify_candidate(CANDIDATE)


def test_phase_requires_absence_or_reviewed_final_bytes(contract_root: Path) -> None:
    initializer = contract_root / INITIALIZER
    initializer.symlink_to(contract_root / "missing")
    with pytest.raises(ToolingError, match="remain absent"):
        load(contract_root).freeze()
    initializer.unlink()
    change(contract_root, "TRANSITION_PHASE=freeze", "TRANSITION_PHASE=assembly")
    with pytest.raises(ToolingError, match="Cannot read"):
        load(contract_root).freeze()
    change(contract_root, "TRANSITION_PHASE=assembly", "TRANSITION_PHASE=finalization")
    with pytest.raises(ToolingError, match="FINAL_INIT_SHA256"):
        load(contract_root)
    path = contract_root / CONFIG
    path.write_text(
        path.read_text() + f"FINAL_INIT_SHA256={hashlib.sha256(CANDIDATE).hexdigest()}\n"
    )
    initializer.write_bytes(CANDIDATE)
    assert load(contract_root).freeze().file_count == 1
    initializer.write_bytes(CANDIDATE + b" ")
    with pytest.raises(ToolingError, match="Final init SHA-256"):
        load(contract_root).freeze()


@pytest.mark.parametrize("kind", ("symlink", "hardlink", "fifo"))
def test_input_link_and_nonregular_file_rejection(contract_root: Path, kind: str) -> None:
    path = contract_root / OUTPUT / "init-candidate.sql"
    path.unlink()
    source = contract_root / MIGRATIONS / MIGRATION
    if kind == "symlink":
        path.symlink_to(source)
    elif kind == "hardlink":
        os.link(source, path)
    else:
        os.mkfifo(path)
    with pytest.raises(ToolingError):
        load(contract_root).candidate()


def test_output_parent_and_relative_traversal_rejection(contract_root: Path) -> None:
    directory = contract_root / OUTPUT
    directory.rename(contract_root / "retained")
    directory.symlink_to(contract_root / "retained", target_is_directory=True)
    with pytest.raises(ToolingError, match="unlinked"):
        load(contract_root).candidate()
    for name in ("../foreign", "/foreign"):
        with pytest.raises(ToolingError, match="relative"):
            owned_path(contract_root, name)


@pytest.mark.parametrize(
    "old,new",
    (("16.14", "18.1"), ("16.14", "latest"), ("docker.io/library/postgres@sha256:", "postgres:")),
)
def test_postgresql_inputs_require_the_reviewed_major_and_digest(
    contract_root: Path, old: str, new: str
) -> None:
    change(contract_root, old, new, BUILD_INPUTS)
    with pytest.raises(ToolingError):
        load(contract_root)


def test_feature_development_allows_initializer_changes_but_keeps_corpus_frozen(
    contract_root: Path,
) -> None:
    change(contract_root, "TRANSITION_PHASE=freeze", "TRANSITION_PHASE=feature-development")
    manifest = contract_root / CONFIG
    manifest.write_text(manifest.read_text() + "FINAL_INIT_SHA256=" + "0" * 64 + "\n")
    initializer = contract_root / INITIALIZER
    initializer.write_bytes(b"CREATE TABLE new_feature (id bigint PRIMARY KEY);\n")
    before = manifest.read_bytes()
    contract = load(contract_root)
    assert contract.review.phase is Phase.FEATURE_DEVELOPMENT
    assert contract.freeze() == contract.review.corpus
    assert manifest.read_bytes() == before
    # Candidate verification remains exact when explicitly requested.
    with pytest.raises(ToolingError, match="Candidate SHA-256"):
        contract.verify_candidate(initializer.read_bytes())
    (contract_root / MIGRATIONS / MIGRATION).write_bytes(CANDIDATE + b"-- drift\n")
    with pytest.raises(ToolingError, match="corpus drifted"):
        contract.freeze()


@pytest.mark.parametrize("contents", (None, b"", b"SELECT 'unfinished;", b"SELECT 1;\x00"))
def test_feature_development_requires_a_regular_scannable_initializer(
    contract_root: Path,
    contents: bytes | None,
) -> None:
    change(contract_root, "TRANSITION_PHASE=freeze", "TRANSITION_PHASE=feature-development")
    if contents is not None:
        (contract_root / INITIALIZER).write_bytes(contents)
    with pytest.raises(ToolingError):
        load(contract_root).freeze()
