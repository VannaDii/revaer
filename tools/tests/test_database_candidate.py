"""Fail candidate stages independently and prevent stale positive evidence."""

import hashlib
import sys
from dataclasses import replace
from pathlib import Path

import pytest
from fixtures.database import ProofDocker
from revaer_tooling.cli import COMMANDS, make_context
from revaer_tooling.context import Options
from revaer_tooling.database.candidate import (
    CANDIDATE,
    NORMALIZATION,
    SOURCE,
    CandidateBuilder,
    build_candidate,
    normalize_dump,
)
from revaer_tooling.database.contract import INITIALIZER, Contract, Phase
from revaer_tooling.database.final_sql import FinalSql
from revaer_tooling.database.postgres import ProofDatabase
from revaer_tooling.database.statements import Statements
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.base import ToolVersion
from revaer_tooling.external.database import Sqlx
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.process import Completed, ProcessRunner
from revaer_tooling.tasks.database_rebaseline import DatabaseFreeze
from test_database_contract import contract_root
from test_database_final_sql import CANDIDATE as FINAL_CANDIDATE
from test_database_final_sql import LIFECYCLE

__all__ = ["contract_root"]

DUMP = b"""-- PostgreSQL database dump
\\restrict a123
-- Dumped from database version 16.14
-- Dumped by pg_dump version 16.14
CREATE TABLE public.item (id bigint);
CREATE FUNCTION revaer_config.factory_reset() RETURNS void
    LANGUAGE sql
    AS $$ SELECT NULL; $$;
\\unrestrict a123
"""


class ProofSqlx(Sqlx):
    def __init__(self, root: Path) -> None:
        super().__init__(sys.executable, ProcessRunner(lambda message: None), root, {})
        self.attempts = 0
        self.outcomes: list[Completed] = [Completed(0, "")]

    def verify(self, required: str | None = None) -> ToolVersion:
        assert required == "0.8.6"
        return ToolVersion(Path(sys.executable), "0.8.6")

    def proof_migration(self, database_url: str, source: Path) -> Completed:
        self.attempts += 1
        assert "@127.0.0.1:55001/" in database_url
        return self.outcomes.pop(0) if len(self.outcomes) > 1 else self.outcomes[0]


@pytest.fixture
def builder(contract_root: Path) -> CandidateBuilder:
    fs = FileSystem()
    contract = Contract.load(contract_root, fs)
    candidate = build_candidate(normalize_dump(DUMP, "16.14"), contract.review.corpus, contract)
    contract = replace(
        contract,
        review=replace(
            contract.review,
            candidate_sha256=hashlib.sha256(candidate).hexdigest(),
            statement_count=Statements.parse(candidate).count,
        ),
    )
    docker = ProofDocker(contract_root)
    docker.dumps = dict.fromkeys((SOURCE, NORMALIZATION, CANDIDATE), DUMP.decode())
    tokens = iter(("a" * 32, "b" * 32))
    runtime = ProofDatabase(docker, fs, lambda duration: None, lambda: next(tokens))
    return CandidateBuilder(contract, runtime, ProofSqlx(contract_root), lambda message: None)


def fake_docker(builder: CandidateBuilder) -> ProofDocker:
    docker = builder.databases.docker
    assert isinstance(docker, ProofDocker)
    return docker


def test_candidate_publishes_matching_mapping_and_evidence_after_cleanup(
    builder: CandidateBuilder,
) -> None:
    result = builder.generate()
    output = builder.contract.output
    assert result.candidate == (output / "init-candidate.sql").read_bytes()
    assert result.mapping == Statements.parse(result.candidate).mapping()
    assert result.report == (output / "evidence.env").read_text()
    assert "candidate_path=target/database-rebaseline/init-candidate.sql\n" in result.report
    assert "migrated_schema_sha256=" in result.report
    docker = fake_docker(builder)
    assert docker.calls[-2:] == ["remove", "require_volumes_removed"]
    assert tuple(query.database for query in docker.queries) == (NORMALIZATION, CANDIDATE)
    assert all(query.transaction for query in docker.queries)
    assert all(
        (output / name).stat().st_mode & 0o777 == 0o600
        for name in ("init-candidate.sql", "statement-boundaries.tsv", "evidence.env")
    )


@pytest.mark.parametrize(
    "failure",
    (
        "verify_image",
        "start",
        "database:revaer_normalization",
        "dump:revaer_candidate",
        "query",
        "remove",
    ),
)
def test_failure_at_any_native_stage_removes_previous_success_evidence(
    builder: CandidateBuilder,
    failure: str,
) -> None:
    output = builder.contract.output
    for name in ("init-candidate.sql", "statement-boundaries.tsv", "evidence.env"):
        (output / name).write_text("stale success")
    fake_docker(builder).failures.add(failure)
    with pytest.raises(ToolingError, match=failure):
        builder.generate()
    assert not any(
        (output / name).exists()
        for name in ("init-candidate.sql", "statement-boundaries.tsv", "evidence.env")
    )


def test_schema_mismatch_retains_both_native_inputs_and_no_positive_report(
    builder: CandidateBuilder,
) -> None:
    fake_docker(builder).dumps[CANDIDATE] = DUMP.replace(b"id bigint", b"id integer").decode()
    with pytest.raises(ToolingError, match="redump differs"):
        builder.generate()
    assert (builder.contract.output / "expected-schema.sql").is_file()
    assert (builder.contract.output / "observed-schema.sql").is_file()
    assert not (builder.contract.output / "evidence.env").exists()


@pytest.mark.parametrize(
    "diagnostic",
    (
        "connection refused",
        "unexpected response from SSLRequest",
        "got 0 bytes at EOF",
        "database system is starting up",
    ),
)
def test_only_known_postgresql_startup_failures_are_retried(
    builder: CandidateBuilder,
    diagnostic: str,
) -> None:
    sqlx = builder.sqlx
    assert isinstance(sqlx, ProofSqlx)
    sqlx.outcomes = [Completed(1, "", diagnostic), Completed(0, "")]
    builder.generate()
    assert sqlx.attempts == 2


@pytest.mark.parametrize(
    "diagnostic,attempts", (("checksum changed", 1), ("got 0 bytes at EOF", 120))
)
def test_migration_failure_is_bounded_and_retains_diagnostics(
    builder: CandidateBuilder,
    diagnostic: str,
    attempts: int,
) -> None:
    sqlx = builder.sqlx
    assert isinstance(sqlx, ProofSqlx)
    sqlx.outcomes = [Completed(1, "", diagnostic)]
    with pytest.raises(ToolingError, match="migration failed"):
        builder.generate()
    assert sqlx.attempts == attempts
    assert (builder.contract.output / "migration-error.txt").read_text() == diagnostic
    assert not (builder.contract.output / "evidence.env").exists()


def test_assembly_prefix_is_applied_before_evidence_is_published(builder: CandidateBuilder) -> None:
    builder.contract = replace(
        builder.contract, review=replace(builder.contract.review, phase=Phase.ASSEMBLY)
    )
    candidate = build_candidate(
        normalize_dump(DUMP, "16.14"), builder.contract.review.corpus, builder.contract
    )
    first = Statements.parse(candidate).boundaries[0].byte_count
    prefix = candidate[:first]
    builder.contract.fs.write_bytes(builder.contract.root / INITIALIZER, prefix)
    builder.generate()
    assert fake_docker(builder).queries[-1].sql.encode() == prefix
    assert fake_docker(builder).queries[-1].database == "revaer_prefix"


@pytest.mark.parametrize(
    "source",
    (
        DUMP.replace(b"\\restrict a123\n", b""),
        DUMP.replace(b"CREATE TABLE public.item", b"CREATE TABLE public._sqlx_migrations"),
        DUMP + b"-- revaer_source\n",
        DUMP + b"-- revaer_rebaseline\n",
        DUMP.replace(b"16.14", b"16.15"),
        DUMP.replace(b"factory_reset", b"other"),
    ),
)
def test_dump_metadata_and_source_identity_cannot_drift(source: bytes) -> None:
    with pytest.raises(ToolingError):
        normalize_dump(source, "16.14")


def test_sqlx_table_name_inside_routine_is_not_mistaken_for_metadata() -> None:
    source = DUMP + b"-- caller inspects '_sqlx_migrations' by name\n"
    assert b"'_sqlx_migrations'" in normalize_dump(source, "16.14")


def test_static_freeze_and_finalization_commands_use_the_selected_checkout(
    contract_root: Path,
) -> None:
    context = replace(make_context(Options()), root=contract_root)
    assert COMMANDS["db-rebaseline-freeze"] is DatabaseFreeze.run
    assert "verified" in COMMANDS["db-rebaseline-freeze"](context).message
    contract = Contract.load(contract_root, context.fs)
    config = contract_root / "config/database-rebaseline.env"
    source = config.read_text().replace(
        contract.review.candidate_sha256, hashlib.sha256(FINAL_CANDIDATE).hexdigest()
    )
    source = source.replace(
        "CANDIDATE_STATEMENT_COUNT=2",
        f"CANDIDATE_STATEMENT_COUNT={Statements.parse(FINAL_CANDIDATE).count}",
    )
    config.write_text(source)
    context.fs.write_bytes(contract.output / "init-candidate.sql", FINAL_CANDIDATE)
    context.fs.write_bytes(contract_root / INITIALIZER, LIFECYCLE)
    before = config.read_bytes()
    COMMANDS["db-init-finalize"](context)
    actual = (contract_root / INITIALIZER).read_bytes()
    assert actual == FinalSql(Contract.load(contract_root, context.fs)).generate(
        LIFECYCLE, FINAL_CANDIDATE
    )
    assert config.read_bytes() == before
