"""Validate reviewed inputs without blessing new hashes or generated evidence."""

import hashlib
import re
from dataclasses import dataclass, field
from enum import StrEnum
from pathlib import Path

from ..assignments import literal_assignments
from ..errors import ToolingError
from ..filesystem import FileSystem
from .statements import Statements

CONFIG = "config/database-rebaseline.env"
BUILD_INPUTS = ".github/build-inputs.env"
MIGRATIONS = "crates/revaer-data/migrations"
INITIALIZER = "crates/revaer-data/init.sql"
OUTPUT = "target/database-rebaseline"
CONFIG_KEYS = frozenset(
    (
        "TRANSITION_PHASE",
        "MIGRATION_FILE_COUNT",
        "MIGRATION_LAST_FILE",
        "MIGRATION_CORPUS_SHA256",
        "CANDIDATE_SHA256",
        "CANDIDATE_STATEMENT_COUNT",
        "FINAL_INIT_SHA256",
        "STACK_CHANGED_LINE_MAX",
        "INIT_ASSEMBLY_CHANGED_LINE_MAX",
    )
)


def required(values: dict[str, str], name: str) -> str:
    try:
        return values[name]
    except KeyError as error:
        raise ToolingError(f"Database transition input is missing {name}") from error


def sha256(values: dict[str, str], name: str) -> str:
    value = required(values, name)
    if not re.fullmatch(r"[0-9a-f]{64}", value):
        raise ToolingError(f"{name} must be a lowercase SHA-256 digest")
    return value


def positive(values: dict[str, str], name: str) -> int:
    value = required(values, name)
    if not re.fullmatch(r"[0-9]+", value) or int(value) < 1:
        raise ToolingError(f"{name} must be a positive base-10 integer")
    return int(value)


def owned_path(root: Path, relative: str) -> Path:
    """A fixed checkout path whose existing parent directories contain no links."""
    if Path(relative).is_absolute() or ".." in Path(relative).parts:
        raise ToolingError("Database paths must be relative to the selected checkout")
    path = root / relative
    for parent in path.parents:
        if parent == root:
            return path
        if parent.is_symlink() or (parent.exists() and not parent.is_dir()):
            raise ToolingError(
                f"Database input/output parent must be an unlinked directory: {parent}"
            )
    raise ToolingError("Database input/output must remain in the selected checkout")


def read_owned(root: Path, fs: FileSystem, relative: str) -> bytes:
    try:
        return fs.read_regular_bytes(owned_path(root, relative))
    except OSError as error:
        raise ToolingError(f"Cannot read regular database input: {relative}") from error


def assignments(root: Path, fs: FileSystem, relative: str) -> dict[str, str]:
    try:
        source = read_owned(root, fs, relative).decode("utf-8")
    except UnicodeDecodeError as error:
        raise ToolingError(f"Database manifest must use UTF-8: {relative}") from error
    return literal_assignments(source, relative)


class Phase(StrEnum):
    FREEZE = "freeze"
    ASSEMBLY = "assembly"
    FINALIZATION = "finalization"
    FEATURE_DEVELOPMENT = "feature-development"


@dataclass(frozen=True)
class Corpus:
    file_count: int
    last_file: str
    sha256: str


@dataclass(frozen=True)
class PostgresPin:
    image: str
    version: str

    @staticmethod
    def load(values: dict[str, str]) -> "PostgresPin":
        image = required(values, "POSTGRES_REBASELINE_IMAGE")
        version = required(values, "POSTGRES_REBASELINE_VERSION")
        if not re.fullmatch(r"docker\.io/library/postgres@sha256:[0-9a-f]{64}", image):
            raise ToolingError("POSTGRES_REBASELINE_IMAGE must pin a fully qualified SHA-256 image")
        if not re.fullmatch(r"16\.[0-9]+", version):
            raise ToolingError(
                "POSTGRES_REBASELINE_VERSION must pin an exact PostgreSQL 16 release"
            )
        return PostgresPin(image, version)


@dataclass(frozen=True)
class Review:
    phase: Phase
    corpus: Corpus
    candidate_sha256: str
    statement_count: int
    final_sha256: str | None
    stack_limit: int
    assembly_limit: int

    @staticmethod
    def load(values: dict[str, str]) -> "Review":
        if unknown := values.keys() - CONFIG_KEYS:
            raise ToolingError("Unknown database contract inputs: " + ", ".join(sorted(unknown)))
        try:
            phase = Phase(required(values, "TRANSITION_PHASE"))
        except ValueError as error:
            raise ToolingError("Unsupported database transition phase") from error
        final = (
            sha256(values, "FINAL_INIT_SHA256")
            if phase == Phase.FINALIZATION or "FINAL_INIT_SHA256" in values
            else None
        )
        return Review(
            phase,
            Corpus(
                positive(values, "MIGRATION_FILE_COUNT"),
                required(values, "MIGRATION_LAST_FILE"),
                sha256(values, "MIGRATION_CORPUS_SHA256"),
            ),
            sha256(values, "CANDIDATE_SHA256"),
            positive(values, "CANDIDATE_STATEMENT_COUNT"),
            final,
            positive(values, "STACK_CHANGED_LINE_MAX"),
            positive(values, "INIT_ASSEMBLY_CHANGED_LINE_MAX"),
        )


@dataclass(frozen=True)
class Contract:
    root: Path
    fs: FileSystem = field(repr=False)
    review: Review
    postgres: PostgresPin

    @staticmethod
    def load(root: Path, fs: FileSystem) -> "Contract":
        root = root.resolve()
        return Contract(
            root,
            fs,
            Review.load(assignments(root, fs, CONFIG)),
            PostgresPin.load(assignments(root, fs, BUILD_INPUTS)),
        )

    @property
    def output(self) -> Path:
        path = owned_path(self.root, OUTPUT)
        if path.is_symlink() or (path.exists() and not path.is_dir()):
            raise ToolingError("Database evidence output must be an unlinked directory")
        return path

    def corpus(self) -> Corpus:
        directory = owned_path(self.root, MIGRATIONS)
        if directory.is_symlink() or not directory.is_dir():
            raise ToolingError("Frozen migration directory must be an unlinked directory")
        entries = sorted(directory.iterdir())
        if not entries:
            raise ToolingError("Frozen migration directory is empty")
        digest = hashlib.sha256()
        for path in entries:
            if not re.fullmatch(r"[0-9]{4}_[a-z0-9_]+\.sql", path.name):
                raise ToolingError(f"Unexpected migration entry: {path.name}")
            relative = path.relative_to(self.root).as_posix()
            digest.update(relative.encode("utf-8") + b"\x00")
            digest.update(read_owned(self.root, self.fs, relative))
            digest.update(b"\x00")
        return Corpus(
            len(entries), entries[-1].relative_to(self.root).as_posix(), digest.hexdigest()
        )

    def freeze(self) -> Corpus:
        actual = self.corpus()
        if actual != self.review.corpus:
            raise ToolingError(
                f"Frozen migration corpus drifted: observed {actual}, reviewed {self.review.corpus}"
            )
        path = owned_path(self.root, INITIALIZER)
        if self.review.phase == Phase.FREEZE:
            if path.exists() or path.is_symlink():
                raise ToolingError("Initializer must remain absent during the freeze phase")
        else:
            initializer = self.initializer()
            if (
                self.review.phase == Phase.FINALIZATION
                and hashlib.sha256(initializer).hexdigest() != self.review.final_sha256
            ):
                raise ToolingError("Final init SHA-256 does not match reviewed finalization bytes")
            if self.review.phase == Phase.FEATURE_DEVELOPMENT:
                # Feature work evolves the initializer after finalization. The
                # historical corpus stays frozen; its final hash is not a pin
                # on new feature SQL. Preserve the source contract's token check.
                Statements.parse(initializer)
        return actual

    def initializer(self) -> bytes:
        return read_owned(self.root, self.fs, INITIALIZER)

    def candidate(self) -> bytes:
        relative = (self.output / "init-candidate.sql").relative_to(self.root).as_posix()
        return read_owned(self.root, self.fs, relative)

    def verify_candidate(self, source: bytes) -> Statements:
        actual = hashlib.sha256(source).hexdigest()
        if actual != self.review.candidate_sha256:
            raise ToolingError(
                f"Candidate SHA-256 is {actual}, expected {self.review.candidate_sha256}"
            )
        statements = Statements.parse(source)
        if statements.count != self.review.statement_count:
            raise ToolingError(
                f"Candidate statement count is {statements.count}, "
                f"expected {self.review.statement_count}"
            )
        return statements

    def verify_prefix(self, candidate: bytes) -> bytes:
        statements = self.verify_candidate(candidate)
        prefix = self.initializer()
        if not candidate.startswith(prefix):
            raise ToolingError("Initializer is not an exact prefix of the pinned candidate")
        if not statements.complete_prefix(len(prefix)):
            raise ToolingError("Initializer does not end at a complete SQL statement boundary")
        return prefix
