"""Strict native ingestion framing and narrowly approved comparison normalization."""

import hashlib
import re
from dataclasses import dataclass

from ...errors import ToolingError
from ...json_data import Json, JsonObject, array_value, decode_unique, object_value, string_value
from ..parity import UUID4

DIAGNOSTIC = re.compile(
    r"ERROR:  (?P<error>[0-9A-Z]{5}: [^\n]+)\n"
    r"(?:DETAIL:  (?P<detail>[^\n]+)\n)?(?:HINT:  (?P<hint>[^\n]+)\n)?"
    r'CONTEXT:  (?:(?P<statement>SQL statement "[^"]*")\n)?'
    r"(?P<routine>PL/pgSQL function search_result_ingest_v1\([^\n]+\)) "
    r"line (?P<line>[1-9][0-9]*) at (?P<operation>RAISE|SQL statement)\n"
    r"LOCATION:  (?P<location>[A-Za-z0-9_]+, [A-Za-z0-9_]+\.c:[1-9][0-9]*)\n"
)


def helper_expectations() -> JsonObject:
    """Independent known answers for the existing helper-first SQL fixture."""
    return {
        "normalize_title_v1": [None, "proof"],
        "normalize_magnet_uri_v1": [
            None,
            None,
            "https://example.invalid/proof",
            "magnet:?",
            "magnet:?dn=Proof&xt=opaque",
            "magnet:?",
            "magnet:?",
            "magnet:?dn=Proof&xt",
        ],
        "derive_magnet_hash_v1": [
            None,
            hashlib.sha256(bytes.fromhex("b" * 64)).hexdigest(),
            hashlib.sha256(bytes.fromhex("a" * 40)).hexdigest(),
            hashlib.sha256(b"magnet:?dn=Proof&xt=opaque").hexdigest(),
        ],
        "compute_title_size_hash_v1": [None, None, hashlib.sha256(b"proof|1024").hexdigest()],
        "policy_text_match_v1": [
            False,
            True,
            True,
            True,
            True,
            True,
            False,
            True,
            False,
            False,
            False,
        ],
        "policy_uuid_match_v1": [False, True, False, False],
        "policy_int_match_v1": [False, True, False, False],
        "policy_release_group_match_v1": [True, False, False],
        "policy_action_to_decision_type": [
            "drop_canonical",
            "drop_source",
            "downrank",
            "flag",
            "flag",
            "flag",
        ],
    }


@dataclass(frozen=True)
class IngestionEvidence:
    """The independently verified reference body/signature bind every diagnostic."""

    signature: str
    source: str
    runtime: str

    def diagnostics(self, stderr: str, role: str) -> list[Json]:
        entries: list[Json] = []
        if not stderr:
            return entries
        for record in re.split(r"(?=^ERROR:  )", stderr, flags=re.M):
            if not record:
                continue
            match = DIAGNOSTIC.fullmatch(record)
            if match is None:
                raise ToolingError("Ingestion diagnostic contains an unrecognized record")
            entry: JsonObject = dict(match.groupdict())
            if entry["routine"] != "PL/pgSQL function " + self.signature:
                raise ToolingError("Ingestion diagnostic signature differs from the frozen routine")
            if entry["statement"] is not None:
                statement = string_value(entry["statement"])[len('SQL statement "') : -1]
                if not statement.strip() or statement not in self.source:
                    raise ToolingError("Ingestion diagnostic SQL is outside the frozen routine")
            # The independently verified local variable-conflict directive adds
            # exactly one line to the final function body.
            line = int(match["line"]) - (1 if role == self.runtime else 0)
            if not 1 <= line <= len(self.source.splitlines()):
                raise ToolingError("Ingestion diagnostic line is outside the frozen routine")
            entry["line"] = line
            entries.append(entry)
        return entries

    def parse(
        self, stdout: str, stderr: str, *, role: str, helpers_first: bool = False
    ) -> JsonObject:
        lines = [line.strip() for line in stdout.splitlines()]
        if not all(
            re.fullmatch(
                r"(?:(?:clock:|role:|before:|helpers:|after:|\{).*|state: [0-9A-Z]{5})", line
            )
            for line in lines
        ):
            raise ToolingError("Ingestion stdout contains an unrecognized record")
        states: list[Json] = [line[7:] for line in lines if line.startswith("state: ")]
        results: list[Json] = [
            object_value(decode_unique(line)) for line in lines if line.startswith("{")
        ]
        roles = [
            object_value(decode_unique(line[5:])) for line in lines if line.startswith("role:")
        ]
        if len(roles) != 1 or roles[0].get("session") != role or roles[0].get("current") != role:
            raise ToolingError("Ingestion must connect directly without role substitution")
        if role == self.runtime and any(
            roles[0].get(key) is not False for key in ("superuser", "create_role", "bypass_rls")
        ):
            raise ToolingError("Ingestion final runtime gained a forbidden capability")
        diagnostics = self.diagnostics(stderr, role)
        errors: list[Json] = [object_value(entry)["error"] for entry in diagnostics]
        details: list[Json] = [
            object_value(entry)["detail"]
            for entry in diagnostics
            if object_value(entry)["detail"] is not None
        ]
        hints: list[Json] = [
            object_value(entry)["hint"]
            for entry in diagnostics
            if object_value(entry)["hint"] is not None
        ]
        if (
            not states
            or len(results) != states.count("00000")
            or [string_value(error)[:5] for error in errors]
            != [state for state in states if state != "00000"]
        ):
            raise ToolingError("Ingestion result/error framing is incomplete")
        before: list[Json] = [line[7:] for line in lines if line.startswith("before:")]
        after: list[Json] = [line[6:] for line in lines if line.startswith("after:")]
        if before != ["error"] or after != before:
            raise ToolingError("Ingestion caller variable-conflict scope changed")
        clocks: list[Json] = [
            string_value(decode_unique(line[6:])) for line in lines if line.startswith("clock:")
        ]
        if len(clocks) != len(states):
            raise ToolingError("Ingestion transaction clock evidence missing")
        helpers: list[Json] = [
            decode_unique(line[8:]) for line in lines if line.startswith("helpers:")
        ]
        if helpers != ([helper_expectations()] if helpers_first else []):
            raise ToolingError("Ingestion helper-first known answers changed or missing")
        if helpers_first:
            helper_position = next(i for i, line in enumerate(lines) if line.startswith("helpers:"))
            before_position = next(i for i, line in enumerate(lines) if line.startswith("before:"))
            result_position = next(
                i for i, line in enumerate(lines) if line.startswith(("state:", "{"))
            )
            if not before_position < helper_position < result_position:
                raise ToolingError("Ingestion helpers were not observed before ingestion")
        return {
            "states": states,
            "results": results,
            "errors": errors,
            "details": details,
            "hints": hints,
            "diagnostics": diagnostics,
            "clocks": clocks,
            "helpers": helpers,
            "caller_settings": {"before": before, "after": after},
        }


def normalize(value: Json, identities: dict[str, str], clocks: list[Json]) -> Json:
    """Replace only proven committed identities and observed transaction clocks."""
    if isinstance(value, dict):
        return {key: normalize(item, identities, clocks) for key, item in value.items()}
    if isinstance(value, list):
        return [normalize(item, identities, clocks) for item in value]
    if isinstance(value, str):
        if value in identities:
            return identities[value]
        if value in clocks:
            return f"<transaction-time:{clocks.index(value)}>"
    return value


def comparable(value: JsonObject) -> JsonObject:
    after = object_value(value.get("after"))
    identities: dict[str, str] = {}
    for table in ("canonical_torrent", "canonical_torrent_source"):
        for row_value in array_value(after.get(table)):
            row = object_value(row_value)
            uuid = string_value(row.get(table + "_public_id"))
            if UUID4.fullmatch(uuid) is None or uuid in identities:
                raise ToolingError("Ingestion generated identity is invalid or duplicated")
            key = row.get(table + "_id")
            if not isinstance(key, int) or isinstance(key, bool):
                raise ToolingError("Ingestion committed identity lacks its integer key")
            identities[uuid] = f"<{table}:{key}>"
    for row_value in array_value(value.get("results")):
        row = object_value(row_value)
        for key in ("canonical_torrent_public_id", "canonical_torrent_source_public_id"):
            if string_value(row.get(key)) not in identities:
                raise ToolingError("Ingestion result identity has no committed row")
    result = object_value(normalize(value, identities, array_value(value.get("clocks"))))
    del result["clocks"]
    return result
