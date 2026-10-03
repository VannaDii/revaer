"""Bind ingestion evidence to exact native bodies, signatures and reviewed deltas."""

from collections.abc import Callable
from dataclasses import dataclass

from ...errors import ToolingError
from ...json_data import JsonObject, array_value, decode_unique, object_value, string_value
from ..final_sql import INGESTION_DELTAS
from ..parity import literal
from ..postgres import Connection
from .evidence import IngestionEvidence

TABLES = (
    "canonical_torrent",
    "canonical_torrent_source",
    "canonical_torrent_source_attr",
    "canonical_torrent_source_context_score",
    "canonical_torrent_best_source_context",
    "search_request_source_observation",
    "search_request_source_observation_attr",
    "canonical_torrent_signal",
    "canonical_external_id",
    "canonical_size_sample",
    "canonical_size_rollup",
    "search_request_canonical",
    "search_page",
    "search_page_item",
    "search_filter_decision",
    "source_metadata_conflict",
    "source_metadata_conflict_audit_log",
    "indexer_health_event",
)
HELPERS = (
    "search_result_ingest",
    "search_result_ingest_v1",
    "normalize_title_v1",
    "normalize_magnet_uri_v1",
    "derive_magnet_hash_v1",
    "compute_title_size_hash_v1",
    "policy_text_match_v1",
    "policy_uuid_match_v1",
    "policy_int_match_v1",
    "policy_release_group_match_v1",
    "log_source_metadata_conflict_v1",
    "policy_action_to_decision_type",
)


@dataclass(frozen=True)
class VerifiedInventory:
    reference: JsonObject
    final: JsonObject
    evidence: IngestionEvidence


def routines(snapshot: JsonObject) -> dict[str, JsonObject]:
    if snapshot.get("version") != "160014":
        raise ToolingError("Ingestion proof requires PostgreSQL 16.14")
    rows = [object_value(row) for row in array_value(snapshot.get("routines"))]
    names = [string_value(row.get("name")) for row in rows]
    if sorted(names) != sorted(HELPERS):
        raise ToolingError("Ingestion reviewed routine inventory changed")
    return dict(zip(names, rows, strict=True))


def settings(row: JsonObject) -> list[str]:
    value = row.get("settings")
    return [] if value is None else [string_value(item) for item in array_value(value)]


def verify(reference: JsonObject, final: JsonObject, runtime: str) -> VerifiedInventory:
    old, fresh = routines(reference), routines(final)
    for name in HELPERS:
        before, after = old[name], fresh[name]
        expected = string_value(before.get("source"))
        if name == "search_result_ingest_v1":
            body = expected.encode()
            for delta in INGESTION_DELTAS:
                body = delta.apply(body, name)
            expected = "\n#variable_conflict use_column\n" + body.decode().removeprefix("\n")
            if "plpgsql.variable_conflict=use_column" not in settings(before) or any(
                item.startswith("plpgsql.variable_conflict=") for item in settings(after)
            ):
                raise ToolingError(
                    "Ingestion reference must retain its GUC and final only its local directive"
                )
        if expected != string_value(after.get("source")) or string_value(
            before.get("signature")
        ) != string_value(after.get("signature")):
            raise ToolingError("Ingestion function body or signature changed beyond D3/D4/D5")
    ingestion = old["search_result_ingest_v1"]
    return VerifiedInventory(
        reference,
        final,
        IngestionEvidence(
            string_value(ingestion.get("signature")),
            string_value(ingestion.get("source")),
            runtime,
        ),
    )


class IngestionInventory:
    def __init__(
        self, connection: Connection, runtime: str, check: Callable[[str, bool], None]
    ) -> None:
        self.connection, self.runtime, self.check = connection, runtime, check

    def collect(self) -> VerifiedInventory:
        query = f"""
            SELECT json_build_object('version', current_setting('server_version_num'),
              'routines', (SELECT json_agg(row_to_json(r) ORDER BY r.name) FROM (
                SELECT p.proname AS name, p.oid::regprocedure::text AS signature,
                  p.prosrc AS source, p.proconfig AS settings, p.prosecdef AS definer,
                  pg_get_userbyid(p.proowner) AS owner
                FROM pg_proc p WHERE p.pronamespace = 'public'::regnamespace
                  AND p.proname IN ({",".join(map(literal, HELPERS))})
              ) r),
              'triggers', (SELECT COALESCE(json_agg(pg_get_triggerdef(t.oid) ORDER BY t.oid), '[]')
                FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid
                WHERE c.relnamespace = 'public'::regnamespace AND NOT t.tgisinternal
                  AND c.relname IN ({",".join(map(literal, TABLES))})));
        """
        databases = ("reference_proof", self.connection.database)
        if databases[0] == databases[1]:
            raise ToolingError(
                "Ingestion inventory requires distinct reference and final databases"
            )
        for database in databases:
            self.connection.fs.remove_owned(
                self.connection.output / f"{database}-inventory.json", self.connection.output
            )
        snapshots = []
        for database in databases:
            value = self.connection.sql(query, role="postgres", database=database)
            self.connection.fs.write(
                self.connection.output / f"{database}-inventory.json", value + "\n", 0o600
            )
            snapshots.append(object_value(decode_unique(value)))
        result = verify(snapshots[0], snapshots[1], self.runtime)
        self.check("ingestion frozen body retained independently of approved D3/D4/D5 deltas", True)
        return result
