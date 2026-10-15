"""Generate the frozen candidate through SQLx and two native PostgreSQL replays."""

import hashlib
import re
from collections.abc import Callable
from dataclasses import dataclass

from ..errors import ToolingError
from ..external.database import Sqlx
from ..external.postgres import DumpArgs
from .contract import MIGRATIONS, Contract, Corpus, Phase
from .final_sql import ASSEMBLY_HEADER, FinalSql
from .postgres import Connection, ProofDatabase

USER = "revaer_rebaseline"
SOURCE = "revaer_source"
NORMALIZATION = "revaer_normalization"
CANDIDATE = "revaer_candidate"
PREFIX = "revaer_prefix"
RESTRICT = re.compile(rb"^\\(?:un)?restrict [A-Za-z0-9]+[ \t\r\v\f]*(?:\n|$)", re.M)
TRANSIENT = re.compile(
    r"connection refused|unexpected response from SSLRequest|got 0 bytes at EOF|"
    r"database system is starting up",
    re.I,
)
SEED = (
    b"\n-- Initialize the repository's existing canonical default state.\n"
    b"SET search_path = public, revaer_config, revaer_runtime;\n"
    b"SELECT revaer_config.factory_reset();\nRESET search_path;\n"
)


def normalize_dump(source: bytes, version: str) -> bytes:
    if len(RESTRICT.findall(source)) != 2:
        raise ToolingError("PostgreSQL dump must contain exactly two restrict metadata lines")
    normalized = RESTRICT.sub(b"", source)
    if re.search(
        rb"^-- Name: _sqlx_migrations; Type:|^CREATE TABLE public\._sqlx_migrations\b",
        normalized,
        re.M,
    ):
        raise ToolingError("PostgreSQL dump contains the SQLx migration metadata table")
    for forbidden in (SOURCE.encode(), USER.encode()):
        if forbidden in normalized:
            raise ToolingError("PostgreSQL dump contains migration metadata or proof identities")
    for prefix in (b"-- Dumped from database version ", b"-- Dumped by pg_dump version "):
        headers = (
            prefix + version.encode() + b"\n",
            prefix + version.encode() + b" (Debian 18.6-1.pgdg12+2)\n",
        )
        if not any(header in normalized for header in headers):
            raise ToolingError("PostgreSQL dump does not identify the pinned server/client version")
    if not re.search(
        rb"CREATE (?:OR REPLACE )?FUNCTION revaer_config\.factory_reset\(\)", normalized
    ):
        raise ToolingError("PostgreSQL dump is missing the canonical factory_reset function")
    return normalized


def build_candidate(schema: bytes, corpus: Corpus, contract: Contract) -> bytes:
    return (
        ASSEMBLY_HEADER
        + f"-- Frozen migration corpus SHA-256: {corpus.sha256}\n".encode()
        + (
            f"-- Generated with PostgreSQL {contract.postgres.version} "
            f"from {contract.postgres.image}\n\n"
        ).encode()
        + schema.rstrip()
        + b"\n"
        + SEED
    )


@dataclass(frozen=True)
class CandidateEvidence:
    candidate: bytes
    mapping: str
    report: str


class CandidateBuilder:
    def __init__(
        self, contract: Contract, databases: ProofDatabase, sqlx: Sqlx, emit: Callable[[str], None]
    ) -> None:
        self.contract, self.databases, self.sqlx, self.emit = contract, databases, sqlx, emit

    def generate(self) -> CandidateEvidence:
        corpus = self.contract.freeze()
        output = self.contract.output
        self.contract.fs.mkdir(output)
        output.chmod(0o700)
        # Positive evidence is published last, after replay AND resource cleanup.
        # A failed rerun must not leave a previous success report in its place.
        for name in ("init-candidate.sql", "statement-boundaries.tsv", "evidence.env"):
            self.contract.fs.remove_owned(output / name, self.contract.root)
        self.sqlx.verify("0.8.6")
        self.databases.docker.verify_image(self.contract.postgres)
        with self.databases.open(
            self.contract, user=USER, database=SOURCE, publish=True
        ) as connection:
            evidence = self._prove(connection, corpus)
        self.contract.fs.write_bytes(output / "init-candidate.sql", evidence.candidate, 0o600)
        self.contract.fs.write(output / "statement-boundaries.tsv", evidence.mapping, 0o600)
        self.contract.fs.write(output / "evidence.env", evidence.report, 0o600)
        return evidence

    def _migrate(self, connection: Connection) -> None:
        for _ in range(120):
            outcome = self.sqlx.proof_migration(connection.url(), self.contract.root / MIGRATIONS)
            if outcome.code == 0:
                return
            if not TRANSIENT.search(outcome.stdout + outcome.stderr):
                break
            self.databases.sleep(0.25)
        self.contract.fs.write(connection.output / "migration-error.txt", outcome.stderr, 0o600)
        raise ToolingError("Frozen SQLx migration failed; retained migration-error.txt")

    def _dump(self, connection: Connection, database: str) -> bytes:
        source = connection.docker.dump(
            connection.container,
            DumpArgs(
                USER,
                database,
                exclude_tables=("public._sqlx_migrations",),
            ),
        ).encode("utf-8")
        return normalize_dump(source, self.contract.postgres.version)

    @staticmethod
    def _apply(connection: Connection, database: str, source: bytes) -> None:
        connection.docker.create_database(connection.container, USER, database)
        connection.sql(source.decode("utf-8"), database=database, transaction=True)

    def _prove(self, connection: Connection, corpus: Corpus) -> CandidateEvidence:
        self.emit("Replaying the frozen migration corpus with SQLx")
        self._migrate(connection)
        source_schema = self._dump(connection, SOURCE)
        self._apply(
            connection, NORMALIZATION, build_candidate(source_schema, corpus, self.contract)
        )
        normalized = self._dump(connection, NORMALIZATION)
        candidate = build_candidate(normalized, corpus, self.contract)
        self._apply(connection, CANDIDATE, candidate)
        observed = self._dump(connection, CANDIDATE)
        if observed != normalized:
            self.contract.fs.write_bytes(
                connection.output / "expected-schema.sql", normalized, 0o600
            )
            self.contract.fs.write_bytes(connection.output / "observed-schema.sql", observed, 0o600)
            raise ToolingError("Fresh candidate schema redump differs; retained both schema files")
        statements = self.contract.verify_candidate(candidate)
        if self.contract.review.phase == Phase.FINALIZATION:
            FinalSql(self.contract).verify(self.contract.initializer(), candidate)
        elif self.contract.review.phase == Phase.ASSEMBLY:
            self._apply(connection, PREFIX, self.contract.verify_prefix(candidate))
        report = (
            "candidate_path=target/database-rebaseline/init-candidate.sql\n"
            f"candidate_sha256={hashlib.sha256(candidate).hexdigest()}\n"
            f"candidate_statement_count={statements.count}\n"
            f"migration_file_count={corpus.file_count}\nmigration_corpus_sha256={corpus.sha256}\n"
            f"postgres_image={self.contract.postgres.image}\npostgres_version={self.contract.postgres.version}\n"
            f"migrated_schema_sha256={hashlib.sha256(source_schema).hexdigest()}\n"
            f"normalized_schema_sha256={hashlib.sha256(normalized).hexdigest()}\n"
            "normalization_apply=passed\nfresh_candidate_apply=passed\nschema_redump_match=passed\n"
        )
        return CandidateEvidence(candidate, statements.mapping(), report)
