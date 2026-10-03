"""Exact ADR 588 D4/D5 counterexamples; these are not general error tolerances."""

import hashlib
from dataclasses import dataclass

from ...errors import ToolingError
from ...json_data import Json, JsonObject, array_value, object_value
from .inventory import TABLES


@dataclass(frozen=True)
class Rejection:
    error: str
    line: int
    location: str
    statement_sha256: str


D4 = Rejection(
    '42P07: relation "tmp_policy_rules" already exists',
    1026,
    "CreateTableAsRelExists, createas.c:406",
    "b55bc5a5393cb31760dd3d1f0d60e1c1bad9403b4572a013d50d9f766813856e",
)
D5 = Rejection(
    "42P10: there is no unique or exclusion constraint matching the ON CONFLICT specification",
    2107,
    "infer_arbiter_indexes, plancat.c:920",
    "6fa4994788151a9ac4c4984689bac6626146e6cca2d82e9920a182f09df1b7a6",
)
REFRESH = {
    "canonical_torrent": "updated_at",
    "canonical_torrent_source": "updated_at",
    "canonical_torrent_source_context_score": "computed_at",
    "canonical_size_rollup": "updated_at",
}


def exact(left: Json, right: Json) -> bool:
    # Preserve Ruby's numeric equality while keeping booleans distinct from
    # numbers: Python's True == 1 must not widen an approved evidence shape.
    if isinstance(left, bool) or isinstance(right, bool):
        return isinstance(left, bool) and isinstance(right, bool) and left is right
    if isinstance(left, dict):
        return (
            isinstance(right, dict)
            and left.keys() == right.keys()
            and all(exact(value, right[key]) for key, value in left.items())
        )
    if isinstance(left, list):
        return (
            isinstance(right, list)
            and len(left) == len(right)
            and all(exact(a, b) for a, b in zip(left, right, strict=True))
        )
    return left == right


def result(created: bool) -> JsonObject:
    return {
        "canonical_torrent_public_id": "<canonical_torrent:1>",
        "canonical_torrent_source_public_id": "<canonical_torrent_source:1>",
        "observation_created": created,
        "durable_source_created": created,
        "canonical_changed": created,
    }


def success(value: JsonObject, results: list[Json], after: JsonObject) -> JsonObject:
    return value | {
        "states": ["00000"] * len(results),
        "results": results,
        "errors": [],
        "details": [],
        "hints": [],
        "diagnostics": [],
        "after": after,
    }


def refresh(tables: JsonObject, clock: str) -> JsonObject:
    if set(tables) != set(TABLES):
        raise ToolingError("Approved ingestion table inventory changed")
    changed = dict(tables)
    for table, value in tables.items():
        rows = array_value(value)
        column = REFRESH.get(table)
        if column is not None:
            if len(rows) != 1 or column not in object_value(rows[0]):
                raise ToolingError("Approved ingestion refresh requires exactly one existing row")
            changed[table] = [object_value(rows[0]) | {column: clock}]
    return changed


def external_tables(before: JsonObject) -> JsonObject:
    clock, observed = "<tested-transaction-time>", "2026-09-10T00:01:00+00:00"
    tables = refresh(before, clock)
    updates: dict[str, JsonObject] = {
        "canonical_torrent": {"imdb_id": "tt1234567"},
        "canonical_torrent_source": {"last_seen_at": observed, "last_seen_seeders": 17},
        "canonical_size_rollup": {"sample_count": 2},
    }
    for table, values in updates.items():
        tables[table] = [object_value(array_value(tables[table])[0]) | values]
    observations = array_value(before.get("search_request_source_observation"))
    if len(observations) != 1:
        raise ToolingError("Approved external-id fixture requires one observation")
    tables["search_request_source_observation"] = [
        object_value(observations[0]) | {"observed_at": observed, "seeders": 17}
    ]
    tables["canonical_torrent_source_attr"] = [
        *array_value(before.get("canonical_torrent_source_attr")),
        {
            "canonical_torrent_source_attr_id": 3,
            "canonical_torrent_source_id": 1,
            "attr_key": "imdb_id",
            "value_text": "tt1234567",
            "value_int": None,
            "value_bigint": None,
            "value_numeric": None,
            "value_bool": None,
        },
    ]
    tables["search_request_source_observation_attr"] = [
        *array_value(before.get("search_request_source_observation_attr")),
        {
            "observation_attr_id": 4,
            "observation_id": 1,
            "attr_key": "imdb_id",
            "value_text": "tt1234567",
            "value_int": None,
            "value_bigint": None,
            "value_numeric": None,
            "value_bool": None,
            "value_uuid": None,
            "created_at": clock,
        },
    ]
    if array_value(before.get("canonical_external_id")):
        raise ToolingError("Approved external-id fixture must not contain an identifier")
    tables["canonical_external_id"] = [
        {
            "canonical_external_id_id": 1,
            "canonical_torrent_id": 1,
            "id_type": "imdb",
            "id_value_text": "tt1234567",
            "id_value_int": None,
            "source_canonical_torrent_source_id": 1,
            "trust_tier_rank": 10,
            "first_seen_at": observed,
            "last_seen_at": observed,
        }
    ]
    tables["canonical_size_sample"] = [
        *array_value(before.get("canonical_size_sample")),
        {
            "canonical_size_sample_id": 2,
            "canonical_torrent_id": 1,
            "observed_at": observed,
            "size_bytes": 1024,
        },
    ]
    return tables


def external_outcome(stages: JsonObject) -> bool:
    """The existing-case semantic assertion required in addition to exact D5 equality."""
    fixture, tested = object_value(stages.get("fixture")), object_value(stages.get("tested"))
    first_rows, second_rows = (
        array_value(fixture.get("results")),
        array_value(tested.get("results")),
    )
    if (
        fixture.get("states") != ["00000"]
        or len(first_rows) != 1
        or tested.get("states") != ["00000"]
        or len(second_rows) != 1
    ):
        return False
    first, second = object_value(first_rows[0]), object_value(second_rows[0])
    flags = ("observation_created", "durable_source_created", "canonical_changed")
    if any(first.get(key) is not True or second.get(key) is not False for key in flags):
        return False
    if any(array_value(rows) for rows in object_value(fixture.get("before")).values()):
        return False
    if any(
        first.get(key) != second.get(key)
        for key in ("canonical_torrent_public_id", "canonical_torrent_source_public_id")
    ):
        return False
    after = object_value(tested.get("after"))
    sources = array_value(after.get("canonical_torrent_source"))
    if (
        len(sources) != 1
        or object_value(sources[0]).get("infohash_v1") != "a" * 40
        or object_value(sources[0]).get("last_seen_seeders") != 17
    ):
        return False
    return (
        len(array_value(after.get("search_request_source_observation"))) == 1
        and object_value(array_value(after.get("canonical_torrent"))[0]).get("imdb_id")
        == "tt1234567"
        and any(
            object_value(row).get("id_type") == "imdb"
            and object_value(row).get("id_value_text") == "tt1234567"
            for row in array_value(after.get("canonical_external_id"))
        )
    )


@dataclass(frozen=True)
class ApprovedCorrections:
    signature: str

    def rejection(self, value: JsonObject, expected: Rejection) -> bool:
        if (
            value.get("errors") != [expected.error]
            or value.get("details") != []
            or value.get("hints") != []
        ):
            return False
        diagnostics = array_value(value.get("diagnostics"))
        if len(diagnostics) != 1:
            return False
        diagnostic = object_value(diagnostics[0])
        statement = diagnostic.get("statement")
        if (
            not isinstance(statement, str)
            or hashlib.sha256(statement.encode()).hexdigest() != expected.statement_sha256
        ):
            return False
        return exact(
            diagnostic,
            {
                "error": expected.error,
                "line": expected.line,
                "location": expected.location,
                "detail": None,
                "hint": None,
                "statement": statement,
                "routine": "PL/pgSQL function " + self.signature,
                "operation": "SQL statement",
            },
        )

    def match(self, name: str, reference: JsonObject, final: JsonObject) -> str | None:
        if name == "warm-committed" and self.warm(reference, final):
            return "ADR 588 D4"
        if name == "existing-external-id" and self.external(reference, final):
            return "ADR 588 D5"
        return None

    def warm(self, reference: JsonObject, final: JsonObject) -> bool:
        if reference.get("states") != ["00000", "42P07"] or not self.rejection(reference, D4):
            return False
        if not exact(reference.get("results"), [result(True)]) or any(
            array_value(rows) for rows in object_value(reference.get("before")).values()
        ):
            return False
        after = refresh(object_value(reference.get("after")), "<transaction-time:1>")
        return exact(final, success(reference, [result(True), result(False)], after))

    def external(self, reference: JsonObject, final: JsonObject) -> bool:
        if (
            list(reference) != ["fixture", "tested"]
            or list(final) != list(reference)
            or not exact(reference["fixture"], final["fixture"])
        ):
            return False
        old = object_value(reference["tested"])
        if (
            old.get("states") != ["42P10"]
            or old.get("results") != []
            or not self.rejection(old, D5)
            or not exact(old.get("before"), old.get("after"))
        ):
            return False
        if not exact(object_value(reference["fixture"]).get("after"), old.get("before")):
            return False
        expected = success(old, [result(False)], external_tables(object_value(old.get("before"))))
        return exact(final["tested"], expected) and external_outcome(final)
