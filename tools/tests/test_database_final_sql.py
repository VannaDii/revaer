"""Mutate approved SQL deltas and independently test the final-control guard."""

import hashlib
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.database.contract import Contract, Phase
from revaer_tooling.database.final_sql import (
    ASSEMBLY_HEADER,
    GRANTS_END,
    GRANTS_START,
    MARKER,
    SECURITY_END,
    SECURITY_START,
    FinalSql,
    approved_legacy_deltas,
    replace_section,
)
from revaer_tooling.database.statements import Statements
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem
from test_database_contract import contract_root

__all__ = ["contract_root"]

INGESTION = b"""-- Name: search_result_ingest_v1(); Type: FUNCTION; Schema: public; Owner: -
CREATE FUNCTION public.search_result_ingest_v1() RETURNS void
    LANGUAGE plpgsql
    SET "plpgsql.variable_conflict" TO 'use_column'
    AS $_$
BEGIN
    CREATE TEMP TABLE tmp_policy_rules AS SELECT 1;
    INSERT INTO identity VALUES (1, 2, 'text')
    ON CONFLICT (canonical_torrent_id, id_type, id_value_text)
        DO UPDATE SET id_value_text = excluded.id_value_text;
    INSERT INTO identity VALUES (1, 2, 3)
    ON CONFLICT (canonical_torrent_id, id_type, id_value_int)
        DO UPDATE SET id_value_int = excluded.id_value_int;
    INSERT INTO identity VALUES (1, 2, 4)
    ON CONFLICT (canonical_torrent_id, id_type, id_value_int)
        DO UPDATE SET id_value_int = excluded.id_value_int;
END;
$_$;
"""
RESET = (
    b"-- Name: factory_reset_without_media_defaults_v1(); Type: FUNCTION; "
    b"Schema: revaer_config; Owner: -\n"
    + b"""
CREATE FUNCTION revaer_config.factory_reset_without_media_defaults_v1() RETURNS void
    LANGUAGE plpgsql
    AS $_$
BEGIN
    PERFORM set_config('lock_timeout', '5s', true);

    PERFORM digest('test', 'sha256');
END;
$_$;
"""
)
TRIGGER = b"""-- Name: observe(); Type: FUNCTION; Schema: public; Owner: -
CREATE FUNCTION public.observe() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    -- digest inside a comment does not require public in search_path.
    PERFORM 'digest';
    RETURN NEW;
END;
$$;
"""
CANDIDATE = (
    ASSEMBLY_HEADER + b"SET statement_timeout = 0;\nSET lock_timeout = 0;\n"
    b"SET idle_in_transaction_session_timeout = 0;\n" + INGESTION + RESET + TRIGGER
)
LIFECYCLE = (
    MARKER
    + SECURITY_START
    + SECURITY_END
    + b"DO $grants$\nBEGIN\nPERFORM ARRAY[\n"
    + GRANTS_START
    + GRANTS_END
    + b"];\nEND;\n$grants$;\n"
)


def for_candidate(final: FinalSql, source: bytes) -> FinalSql:
    review = replace(
        final.contract.review,
        candidate_sha256=hashlib.sha256(source).hexdigest(),
        statement_count=Statements.parse(source).count,
    )
    return FinalSql(replace(final.contract, review=review))


def for_final(final: FinalSql, source: bytes) -> FinalSql:
    return FinalSql(
        replace(
            final.contract,
            review=replace(
                final.contract.review,
                phase=Phase.FINALIZATION,
                final_sha256=hashlib.sha256(source).hexdigest(),
            ),
        )
    )


@pytest.fixture
def final_sql(contract_root: Path) -> FinalSql:
    return for_candidate(FinalSql(Contract.load(contract_root, FileSystem())), CANDIDATE)


def test_generation_retains_authored_lifecycle_and_exact_approved_transformations(
    final_sql: FinalSql,
) -> None:
    source = final_sql.generate(LIFECYCLE, CANDIDATE)
    checked = for_final(final_sql, source)
    assert checked.verify(source, CANDIDATE) == source
    assert checked.generate(source, CANDIDATE) == source
    assert b"#variable_conflict use_column\n" in source
    assert b"CREATE TEMP TABLE tmp_policy_rules ON COMMIT DROP AS" in source
    assert source.count(b"WHERE id_value_int IS NOT NULL") == 2
    assert source.count(b"WHERE id_value_text IS NOT NULL") == 1
    assert b"    SET lock_timeout TO '5s'\n" in source
    assert b"PERFORM set_config('lock_timeout', '5s', true)" not in source
    assert b"SET statement_timeout = 0" not in source
    assert source.endswith(b"];\nEND;\n$grants$;\n")


def test_routine_security_and_grants_follow_the_reviewed_classifier(final_sql: FinalSql) -> None:
    routines = final_sql.routines(CANDIDATE)
    assert tuple(item.search_path for item in routines) == (
        "pg_catalog",
        "pg_catalog, public",
        "pg_catalog",
    )
    assert tuple(item.trigger for item in routines) == (False, False, True)
    assert b"public.observe() SET search_path TO pg_catalog" in final_sql.security(CANDIDATE)
    assert b"public.observe()" not in final_sql.grants(CANDIDATE)
    assert b"public.search_result_ingest_v1()'" in final_sql.grants(CANDIDATE)


@pytest.mark.parametrize("change", ("missing", "duplicate", "count", "wrong-routine"))
def test_approved_routine_deltas_are_exact_and_confined(change: str) -> None:
    source = CANDIDATE
    if change == "missing":
        source = source.replace(INGESTION, b"")
    elif change == "duplicate":
        source += INGESTION
    elif change == "count":
        source = source.replace(
            b"CREATE TEMP TABLE tmp_policy_rules AS", b"CREATE TABLE different AS"
        )
    else:
        source = source.replace(
            b"CREATE FUNCTION public.search_result_ingest_v1()", b"CREATE FUNCTION public.other()"
        )
    with pytest.raises(ToolingError, match="Approved"):
        approved_legacy_deltas(source)


@pytest.mark.parametrize(
    "old,new",
    (
        (ASSEMBLY_HEADER, b"-- wrong header\n"),
        (b"SET lock_timeout = 0;\n", b""),
        (b"SET lock_timeout = 0;\n", b"SET lock_timeout = 0;\nSET lock_timeout = 0;\n"),
    ),
)
def test_header_and_dump_timeout_counts_are_not_silently_repaired(
    final_sql: FinalSql,
    old: bytes,
    new: bytes,
) -> None:
    source = CANDIDATE.replace(old, new)
    with pytest.raises(ToolingError):
        for_candidate(final_sql, source).legacy(source)


@pytest.mark.parametrize(
    "old,new",
    (
        (b"-- Name: observe();", b"-- Name: different();"),
        (b"-- Name: observe();", b"-- Wrong: observe();"),
        (b"    AS $$\n", b"    AS 'body'; --\n"),
    ),
)
def test_routine_identity_and_body_must_be_recognizable(
    final_sql: FinalSql,
    old: bytes,
    new: bytes,
) -> None:
    source = CANDIDATE.replace(old, new)
    with pytest.raises(ToolingError):
        for_candidate(final_sql, source).routines(source)


@pytest.mark.parametrize("change", ("prefix", "security", "grants", "digest"))
def test_final_verification_rejects_unreviewed_bytes(final_sql: FinalSql, change: str) -> None:
    source = final_sql.generate(LIFECYCLE, CANDIDATE)
    checked = for_final(final_sql, source)
    if change == "prefix":
        changed = b" " + source
    elif change == "security":
        changed = source.replace(b" SECURITY DEFINER", b" SECURITY INVOKER")
    elif change == "grants":
        changed = source.replace(
            b"            'public.search_result_ingest_v1()'", b"            'public.other()'"
        )
    else:
        changed = source + b"\n"
    with pytest.raises(ToolingError):
        checked.verify(changed, CANDIDATE)


@pytest.mark.parametrize(
    "statement",
    (
        b"BEGIN;",
        b"COMMIT;",
        b"ROLLBACK;",
        b"END;",
        b"VACUUM;",
        b"CREATE DATABASE other;",
        b"ALTER SYSTEM SET work_mem='1MB';",
        b"DISCARD ALL;",
        b"SET LOCAL lock_timeout='1s';",
        b"RESET SESSION statement_timeout;",
        b"RESET ALL;",
    ),
)
def test_forbidden_top_level_controls_fail_even_with_matching_fixture_hash(
    final_sql: FinalSql,
    statement: bytes,
) -> None:
    source = final_sql.generate(LIFECYCLE, CANDIDATE) + b"-- proof mutation\n" + statement + b"\n"
    with pytest.raises(ToolingError, match="forbidden transaction or timeout"):
        for_final(final_sql, source).verify(source, CANDIDATE)


def test_lifecycle_markers_must_exist_once_in_order(final_sql: FinalSql) -> None:
    with pytest.raises(ToolingError, match="lifecycle"):
        final_sql.generate(b"SELECT 1;", CANDIDATE)
    for source in (b"", SECURITY_START * 2 + SECURITY_END, SECURITY_END + SECURITY_START):
        with pytest.raises(ToolingError, match="markers"):
            replace_section(source, SECURITY_START, SECURITY_END, b"replacement")
