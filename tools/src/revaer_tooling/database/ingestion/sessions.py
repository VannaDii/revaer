"""Build ingestion qualification sessions and prove their transaction semantics."""

import re
from collections.abc import Callable, Mapping
from dataclasses import dataclass
from types import MappingProxyType

from ...errors import ToolingError
from ...external.postgres import QueryArgs
from ...process import Completed
from ..postgres import Connection

ARGUMENTS: Mapping[str, str] = MappingProxyType(
    {
        "search_request_public_id_input": "'56900000-0000-4000-8000-000000000002'::uuid",
        "indexer_instance_public_id_input": "'56900000-0000-4000-8000-000000000001'::uuid",
        "source_guid_input": "'ingestion-proof-source'::varchar",
        "details_url_input": "NULL::varchar",
        "download_url_input": "NULL::varchar",
        "magnet_uri_input": "NULL::varchar",
        "title_raw_input": "'Ingestion proof title'::varchar",
        "size_bytes_input": "1024::bigint",
        "infohash_v1_input": "repeat('a', 40)::char(40)",
        "infohash_v2_input": "NULL::char(64)",
        "magnet_hash_input": "NULL::char(64)",
        "seeders_input": "5",
        "leechers_input": "2",
        "published_at_input": "NULL::timestamptz",
        "uploader_input": "NULL::varchar",
        "observed_at_input": "'2026-09-10T00:00:00Z'::timestamptz",
        "attr_keys_input": "NULL::public.observation_attr_key[]",
        "attr_types_input": "NULL::public.attr_value_type[]",
        "attr_value_text_input": "NULL::varchar[]",
        "attr_value_int_input": "NULL::integer[]",
        "attr_value_bigint_input": "NULL::bigint[]",
        "attr_value_numeric_input": "NULL::numeric[]",
        "attr_value_bool_input": "NULL::boolean[]",
        "attr_value_uuid_input": "NULL::uuid[]",
    }
)


def ingestion_call(changes: Mapping[str, str]) -> str:
    """Accept only reviewed argument names; values are internal SQL fixtures."""
    if changes.keys() - ARGUMENTS.keys():
        raise ToolingError("Unknown ingestion fixture argument")
    arguments = ",\n".join(
        f"{name} => {value}" for name, value in (dict(ARGUMENTS) | dict(changes)).items()
    )
    return f"SELECT row_to_json(r) FROM public.search_result_ingest_v1({arguments}) r;"


@dataclass(frozen=True)
class IngestionCase:
    name: str
    changes: Mapping[str, str]
    expected: tuple[str, ...]
    helpers_first: bool = False
    repeat: bool = False


def cases() -> tuple[IngestionCase, ...]:
    cold = (
        IngestionCase(
            "request-missing", {"search_request_public_id_input": "NULL::uuid"}, ("P0001",)
        ),
        IngestionCase(
            "instance-missing", {"indexer_instance_public_id_input": "NULL::uuid"}, ("P0001",)
        ),
        IngestionCase("new-v1", {}, ("00000",)),
        IngestionCase("new-v2", {"infohash_v2_input": "repeat('b', 64)::char(64)"}, ("00000",)),
        IngestionCase(
            "new-magnet",
            {
                "infohash_v1_input": "NULL::char(40)",
                "magnet_uri_input": "'magnet:?dn=Proof&xt=opaque'::varchar",
            },
            ("00000",),
        ),
        IngestionCase("new-title-size", {"infohash_v1_input": "NULL::char(40)"}, ("00000",)),
    )
    return (
        cold
        + tuple(
            IngestionCase("helpers-first-" + case.name, case.changes, case.expected, True)
            for case in cold
        )
        + (IngestionCase("warm-committed", {}, ("00000", "00000"), repeat=True),)
    )


def session(
    changes: Mapping[str, str], *, repeat: bool = False, helper_sql: str | None = None
) -> str:
    call = ingestion_call(changes)
    if helper_sql is not None and not helper_sql.strip():
        raise ToolingError("Helper-first ingestion requires its SQL fixture")
    second = (
        f"""COMMIT;
BEGIN;
SELECT 'clock:' || to_json(transaction_timestamp())::text;
SAVEPOINT ingestion_second;
\\set ON_ERROR_STOP off
{call}
\\echo state: :SQLSTATE
\\set ON_ERROR_STOP on
\\if :ERROR
ROLLBACK TO SAVEPOINT ingestion_second;
\\endif
"""
        if repeat
        else "\\if :ERROR\nROLLBACK TO SAVEPOINT ingestion_call;\n\\endif\n"
    )
    return f"""\\set VERBOSITY verbose
SET statement_timeout = '120s';
DO $$ BEGIN NULL; END $$;
BEGIN;
SELECT 'clock:' || to_json(transaction_timestamp())::text;
SELECT 'role:' || json_build_object('session', session_user, 'current', current_user,
  'superuser', r.rolsuper, 'create_role', r.rolcreaterole, 'bypass_rls', r.rolbypassrls)::text
  FROM pg_roles r WHERE r.rolname = current_user;
SELECT 'before:' || COALESCE(current_setting('plpgsql.variable_conflict', true), '<unloaded>');
{helper_sql or ""}
SAVEPOINT ingestion_call;
\\set ON_ERROR_STOP off
{call}
\\echo state: :SQLSTATE
\\set ON_ERROR_STOP on
{second}
SELECT 'after:' || COALESCE(current_setting('plpgsql.variable_conflict', true), '<unloaded>');
COMMIT;
"""


def control_records(outcome: Completed, *, failure_expected: bool) -> list[str]:
    if outcome.code != 0:
        raise ToolingError("Ingestion session control failed")
    lines = [line.strip() for line in outcome.stdout.splitlines()]
    expected = ["state: 00000", "state: 22012" if failure_expected else "state: 00000"]
    if [line for line in lines if line.startswith("state:")] != expected:
        raise ToolingError("Ingestion session control SQLSTATE mismatch")
    diagnostic_matches = (
        re.fullmatch(
            r"ERROR:\s+22012: division by zero\nLOCATION:\s+int4div, int\.c:[0-9]+\n",
            outcome.stderr,
        )
        is not None
        if failure_expected
        else not outcome.stderr
    )
    if not diagnostic_matches:
        raise ToolingError("Ingestion session control unexpected diagnostic")
    return [line for line in lines if line.startswith("writes:")]


class SessionControls:
    def __init__(self, connection: Connection, check: Callable[[str, bool], None]) -> None:
        self.connection, self.check = connection, check

    def verify(self) -> None:
        # Exercise the exact warm-session producer. Different second writes and
        # a real division error expose swallowed writes or accidental rollbacks.
        parts = session({}, repeat=True).split(ingestion_call({}))
        if len(parts) != 3:
            raise ToolingError("Ingestion warm control must replace exactly two calls")
        observations = []
        for second in ("2", "3", "1/0"):
            query = (
                "CREATE TEMP TABLE ingestion_harness_counter (value integer NOT NULL);\n"
                + parts[0]
                + "\nINSERT INTO ingestion_harness_counter VALUES (1);\n"
                + parts[1]
                + f"\nINSERT INTO ingestion_harness_counter VALUES ({second});\n"
                + parts[2]
                + "\nSELECT 'writes:' || COALESCE(string_agg(value::text, ',' ORDER BY value), '') "
                "FROM ingestion_harness_counter;\n"
            )
            prefix = self.connection.output / ("session-control-" + second.replace("/", "-"))
            self.connection.fs.write(prefix.with_suffix(".sql"), query, 0o600)
            # Invalidate diagnostics before launch; a failed transport cannot
            # leave the previous run's stdout beside newly generated SQL.
            for suffix in (".stdout", ".stderr"):
                self.connection.fs.remove_owned(prefix.with_suffix(suffix), self.connection.output)
            result = self.connection.docker.query(
                self.connection.container, QueryArgs("postgres", "reference_proof", query)
            )
            self.connection.fs.write(prefix.with_suffix(".stdout"), result.stdout, 0o600)
            self.connection.fs.write(prefix.with_suffix(".stderr"), result.stderr, 0o600)
            observations.append(control_records(result, failure_expected=second == "1/0"))
        self.check(
            "ingestion warm control commits successful second writes",
            observations[0] == ["writes:1,2"],
        )
        self.check(
            "ingestion warm control exposes different second writes",
            observations[1] == ["writes:1,3"] and observations[0] != observations[1],
        )
        self.check(
            "ingestion warm control rolls back only failed second writes",
            observations[2] == ["writes:1"],
        )
