"""Ingestion evidence rejects role escalation, missing frames and invented identity."""

import json
import re
from copy import deepcopy
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.database.contract import BUILD_INPUTS, Contract, PostgresPin, assignments
from revaer_tooling.database.ingestion.evidence import (
    IngestionEvidence,
    comparable,
    helper_expectations,
)
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.postgres import QueryArgs
from revaer_tooling.json_data import JsonObject, array_value, object_value
from test_database_contract import contract_root
from test_database_postgres import receipt

__all__ = ["contract_root"]

SOURCE = """
BEGIN
IF fail THEN
    RAISE EXCEPTION 'proof rejection' USING ERRCODE = 'P0001',
        DETAIL = 'proof detail', HINT = 'proof hint';
END IF;
RETURN 1;
END;
"""
PARSER = IngestionEvidence("search_result_ingest_v1(boolean)", SOURCE, "proof_runtime")
ROLE = {
    "session": "proof_runtime",
    "current": "proof_runtime",
    "superuser": False,
    "create_role": False,
    "bypass_rls": False,
}


def success(helpers: bool = False) -> str:
    return (
        "\n".join(
            (
                'clock:"2026-09-10T00:00:00Z"',
                "role:" + json.dumps(ROLE),
                "before:error",
                *(("helpers:" + json.dumps(helper_expectations()),) if helpers else ()),
                '{"value":1}',
                "state: 00000",
                "after:error",
            )
        )
        + "\n"
    )


@pytest.mark.parametrize("helpers", (False, True))
def test_valid_success_frames_and_helper_known_answers(helpers: bool) -> None:
    value = PARSER.parse(success(helpers), "", role="proof_runtime", helpers_first=helpers)
    assert value["states"] == ["00000"] and value["results"] == [{"value": 1}]


@pytest.mark.parametrize(
    "old,new",
    (
        ('"superuser": false', '"superuser": true'),
        ('"current": "proof_runtime"', '"current": "postgres"'),
        ('"bypass_rls": false', '"bypass_rls": true'),
        ("before:error", "before:use_column"),
        ("after:error", "after:use_column"),
        ("state: 00000\n", ""),
        ('{"value":1}\n', ""),
        ('clock:"2026-09-10T00:00:00Z"\n', ""),
        ("after:error", "after:error\nunrecognized"),
        ('{"value":1}', '{"value":1,"value":2}'),
    ),
)
def test_incomplete_or_privileged_frames_fail(old: str, new: str) -> None:
    with pytest.raises(ToolingError):
        PARSER.parse(success().replace(old, new), "", role="proof_runtime")


def test_changed_helper_answer_is_rejected() -> None:
    with pytest.raises(ToolingError, match="known answers"):
        PARSER.parse(
            success(True).replace('"proof"', '"incorrect"'),
            "",
            role="proof_runtime",
            helpers_first=True,
        )


def test_helpers_must_precede_results() -> None:
    line = "helpers:" + json.dumps(helper_expectations())
    reordered = success().replace("after:error", line + "\nafter:error")
    with pytest.raises(ToolingError, match="before ingestion"):
        PARSER.parse(reordered, "", role="proof_runtime", helpers_first=True)


def test_comparison_rewrites_only_committed_ids_and_observed_clocks() -> None:
    torrent = "00000000-0000-4000-8000-000000000001"
    source = "00000000-0000-4000-8000-000000000002"
    value: JsonObject = {
        "after": {
            "canonical_torrent": [
                {"canonical_torrent_id": 1, "canonical_torrent_public_id": torrent}
            ],
            "canonical_torrent_source": [
                {"canonical_torrent_source_id": 2, "canonical_torrent_source_public_id": source}
            ],
        },
        "results": [
            {"canonical_torrent_public_id": torrent, "canonical_torrent_source_public_id": source}
        ],
        "clocks": ["clock-a"],
        "observed": "clock-a",
        "unrelated": "preserved",
    }
    original = deepcopy(value)
    normalized = comparable(value)
    assert value == original
    assert normalized["observed"] == "<transaction-time:0>" and "clocks" not in normalized
    assert normalized["results"] == [
        {
            "canonical_torrent_public_id": "<canonical_torrent:1>",
            "canonical_torrent_source_public_id": "<canonical_torrent_source:2>",
        }
    ]
    assert normalized["unrelated"] == "preserved"
    value["results"] = [
        {"canonical_torrent_public_id": "missing", "canonical_torrent_source_public_id": source}
    ]
    with pytest.raises(ToolingError, match="no committed row"):
        comparable(value)
    value = deepcopy(original)
    object_value(value["after"])["canonical_torrent_source"] = [
        {"canonical_torrent_source_id": 2, "canonical_torrent_source_public_id": torrent}
    ]
    with pytest.raises(ToolingError, match="duplicated"):
        comparable(value)


def test_native_diagnostic_signature_statement_and_directive_line_binding(
    contract_root: Path,
) -> None:
    context = make_context(Options())
    contract = replace(
        Contract.load(contract_root, context.fs),
        postgres=PostgresPin.load(assignments(context.root, context.fs, BUILD_INPUTS)),
    )
    runtime = context.tools.proof_databases
    with runtime.open(contract, user="postgres", database="proof") as db:
        db.sql("CREATE ROLE proof_runtime LOGIN")
        observed_lines = []
        for role, body in (
            ("postgres", SOURCE),
            ("proof_runtime", "\n#variable_conflict use_column\n" + SOURCE.lstrip("\n")),
        ):
            db.sql(
                "CREATE OR REPLACE FUNCTION public.search_result_ingest_v1(fail boolean) "
                "RETURNS integer LANGUAGE plpgsql AS $body$" + body + "$body$;"
            )
            for fail in (False, True):
                sql = f"""\n\\set VERBOSITY verbose
DO $$ BEGIN NULL; END $$;
BEGIN;
SELECT 'clock:' || to_json(transaction_timestamp())::text;
SELECT 'role:' || json_build_object('session', session_user, 'current', current_user,
    'superuser', r.rolsuper, 'create_role', r.rolcreaterole, 'bypass_rls', r.rolbypassrls)
    FROM pg_roles r WHERE r.rolname = current_user;
SELECT 'before:' || current_setting('plpgsql.variable_conflict');
SAVEPOINT operation;
\\set ON_ERROR_STOP off
SELECT json_build_object('value', public.search_result_ingest_v1({str(fail).lower()}));
\\echo state: :SQLSTATE
\\set ON_ERROR_STOP on
\\if :ERROR
ROLLBACK TO SAVEPOINT operation;
\\endif
SELECT 'after:' || current_setting('plpgsql.variable_conflict');
COMMIT;
"""
                outcome = db.docker.query(db.container, QueryArgs(role, db.database, sql))
                assert outcome.code == 0
                parsed = PARSER.parse(outcome.stdout, outcome.stderr, role=role)
                assert parsed["states"] == (["P0001"] if fail else ["00000"])
                if fail:
                    observed_lines.append(
                        object_value(array_value(parsed["diagnostics"])[0])["line"]
                    )
                    assert parsed["details"] == ["proof detail"] and parsed["hints"] == [
                        "proof hint"
                    ]
                    for invalid in (
                        outcome.stderr + "WARNING: unexpected\n",
                        re.sub(r" line [0-9]+ at", " line 99999 at", outcome.stderr),
                        outcome.stderr.replace(
                            "search_result_ingest_v1(boolean)", "search_result_ingest_v1(integer)"
                        ),
                        outcome.stderr.replace(
                            "CONTEXT:  ", 'CONTEXT:  SQL statement "SELECT invented"\n'
                        ),
                        outcome.stderr.replace(" at RAISE", " at unexpected"),
                    ):
                        with pytest.raises(ToolingError):
                            PARSER.parse(outcome.stdout, invalid, role=role)
        assert len(observed_lines) == 2 and observed_lines[0] == observed_lines[1]
    assert receipt(contract)["storage_removed"] is True
