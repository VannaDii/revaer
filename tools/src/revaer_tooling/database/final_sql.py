"""Reproduce only the initializer deltas already reviewed by the media work.

These exact byte substitutions and routine classifications are proof inputs,
not general SQL transformations. PostgreSQL still executes the resulting SQL.
"""

import hashlib
import re
from collections.abc import Iterator
from dataclasses import dataclass

from ..errors import ToolingError
from .contract import Contract
from .statements import Statements

HEADER = b"-- Revaer pre-v1 packaged database baseline.\n"
ASSEMBLY_HEADER = b"-- Revaer pre-v1 init candidate. Assembly-only until ADR 522 cutover.\n"
MARKER = b"\n-- ADR 551 finalization: lifecycle and explicit authored routine privileges.\n"
SECURITY_START = b"-- Generated authored routine security begins.\n"
SECURITY_END = b"-- Generated authored routine security ends.\n"
GRANTS_START = b"        -- Generated authored routine grants begin.\n"
GRANTS_END = b"        -- Generated authored routine grants end.\n"
TIMEOUTS = (b"statement_timeout", b"lock_timeout", b"idle_in_transaction_session_timeout")
RESET_HEADER = (
    b"CREATE FUNCTION revaer_config.factory_reset_without_media_defaults_v1() RETURNS void\n"
    b"    LANGUAGE plpgsql\n"
)


@dataclass(frozen=True)
class Delta:
    original: bytes
    replacement: bytes
    count: int = 1

    def apply(self, source: bytes, identity: str) -> bytes:
        if source.count(self.original) != self.count:
            raise ToolingError(
                f"Approved {identity} delta must match exactly {self.count} occurrences"
            )
        return source.replace(self.original, self.replacement)


INGESTION_DELTAS = (
    Delta(
        b"CREATE TEMP TABLE tmp_policy_rules AS",
        b"CREATE TEMP TABLE tmp_policy_rules ON COMMIT DROP AS",
    ),
    Delta(
        b"ON CONFLICT (canonical_torrent_id, id_type, id_value_text)\n        DO UPDATE SET",
        b"ON CONFLICT (canonical_torrent_id, id_type, id_value_text)\n"
        b"        WHERE id_value_text IS NOT NULL\n        DO UPDATE SET",
    ),
    Delta(
        b"ON CONFLICT (canonical_torrent_id, id_type, id_value_int)\n        DO UPDATE SET",
        b"ON CONFLICT (canonical_torrent_id, id_type, id_value_int)\n"
        b"        WHERE id_value_int IS NOT NULL\n        DO UPDATE SET",
        2,
    ),
)
CONFLICT_DELTA = Delta(
    b"    SET \"plpgsql.variable_conflict\" TO 'use_column'\n    AS $_$\n",
    b"    AS $_$\n#variable_conflict use_column\n",
)
RESET_DELTAS = (
    Delta(RESET_HEADER, RESET_HEADER + b"    SET lock_timeout TO '5s'\n"),
    Delta(b"    PERFORM set_config('lock_timeout', '5s', true);\n\n", b""),
)


def segments(source: bytes) -> Iterator[tuple[int, int, bytes]]:
    start = 0
    for boundary in Statements.parse(source).boundaries:
        yield start, boundary.byte_count, source[start : boundary.byte_count]
        start = boundary.byte_count


def replace_routine(source: bytes, identity: str, deltas: tuple[Delta, ...]) -> bytes:
    pattern = rb"^CREATE FUNCTION " + re.escape(identity.encode()) + rb"\("
    matches = [
        (start, end, statement)
        for start, end, statement in segments(source)
        if re.search(pattern, statement, re.M)
    ]
    if len(matches) != 1:
        raise ToolingError(f"Approved {identity} routine must occur exactly once")
    start, end, changed = matches[0]
    for delta in deltas:
        changed = delta.apply(changed, identity)
    return source[:start] + changed + source[end:]


def approved_legacy_deltas(source: bytes) -> bytes:
    source = replace_routine(
        source, "public.search_result_ingest_v1", (CONFLICT_DELTA, *INGESTION_DELTAS)
    )
    return replace_routine(
        source, "revaer_config.factory_reset_without_media_defaults_v1", RESET_DELTAS
    )


def replace_section(source: bytes, opening: bytes, closing: bytes, replacement: bytes) -> bytes:
    if source.count(opening) != 1 or source.count(closing) != 1:
        raise ToolingError("Generated final SQL section markers must be unique")
    start, end = source.index(opening), source.index(closing) + len(closing)
    if end <= start:
        raise ToolingError("Generated final SQL section markers must be ordered")
    return source[:start] + replacement + source[end:]


@dataclass(frozen=True)
class Routine:
    identity: str
    schema: str
    name: str
    trigger: bool
    search_path: str


class FinalSql:
    def __init__(self, contract: Contract) -> None:
        self.contract = contract

    def legacy(self, candidate: bytes) -> bytes:
        self.contract.verify_candidate(candidate)
        if not candidate.startswith(ASSEMBLY_HEADER):
            raise ToolingError("Candidate assembly header is missing")
        source = HEADER + candidate[len(ASSEMBLY_HEADER) :]
        for name in TIMEOUTS:
            source = Delta(b"SET " + name + b" = 0;\n", b"").apply(source, name.decode())
        return approved_legacy_deltas(source)

    def routines(self, candidate: bytes) -> tuple[Routine, ...]:
        self.contract.verify_candidate(candidate)
        objects = set(
            re.findall(rb"^CREATE (?:FUNCTION|TABLE|TYPE) public\.([a-z_0-9]+)", candidate, re.M)
        ) | {b"digest", b"gen_random_bytes", b"unaccent"}
        dependency = re.compile(
            rb"(?<![\w.])(?:" + b"|".join(map(re.escape, sorted(objects))) + rb")\b", re.I
        )
        result = []
        for _, _, statement in segments(candidate):
            if not re.search(rb"^CREATE FUNCTION ", statement, re.M):
                continue
            identity = re.search(
                rb"^-- Name: (.+); Type: FUNCTION; "
                rb"Schema: (public|revaer_config|revaer_runtime); Owner: -$",
                statement,
                re.M,
            )
            definition = re.search(
                rb"^CREATE FUNCTION (public|revaer_config|revaer_runtime)"
                rb"\.([a-z_0-9]+)\(.*?\) RETURNS (.+)\n",
                statement,
                re.M | re.S,
            )
            if identity is None or definition is None:
                raise ToolingError("Unrecognized authored routine envelope")
            schema, name = definition.group(1, 2)
            if identity[2] != schema or not identity[1].startswith(name + b"("):
                raise ToolingError("Routine identity does not match its definition")
            body = re.split(rb"^    AS \$\w*\$", statement, maxsplit=1, flags=re.M)
            if len(body) != 2:
                raise ToolingError("Unrecognized authored routine body")
            # Preserve the reviewed dependency classifier exactly. This is not
            # an attempted replacement for PostgreSQL name resolution.
            tokens = re.sub(rb"--[^\n]*|/\*.*?\*/|'(?:[^']|'')*'", b" ", body[1], flags=re.S)
            result.append(
                Routine(
                    (schema + b"." + identity[1]).decode(),
                    schema.decode(),
                    name.decode(),
                    bool(
                        re.search(
                            rb"^CREATE FUNCTION .* RETURNS (?:event_)?trigger$", statement, re.M
                        )
                    ),
                    "pg_catalog, public" if dependency.search(tokens) else "pg_catalog",
                )
            )
        return tuple(result)

    def security(self, candidate: bytes) -> bytes:
        lines = []
        for routine in self.routines(candidate):
            mode = "" if routine.trigger else " SECURITY DEFINER"
            lines.append(
                f"ALTER FUNCTION {routine.identity}{mode} "
                f"SET search_path TO {routine.search_path};\n"
            )
        return SECURITY_START + "".join(lines).encode() + SECURITY_END

    def grants(self, candidate: bytes) -> bytes:
        identities = (
            routine.identity.replace("'", "''")
            for routine in self.routines(candidate)
            if not routine.trigger
        )
        lines = ",\n".join(f"            '{identity}'" for identity in identities)
        return GRANTS_START + lines.encode() + b"\n" + GRANTS_END

    def verify(self, source: bytes, candidate: bytes) -> bytes:
        prefix, marker, suffix = source.partition(MARKER)
        if not marker or prefix != self.legacy(candidate):
            raise ToolingError("Final init has unauthorized legacy deltas")
        if self.security(candidate) not in suffix:
            raise ToolingError("Final init security delta changed")
        if self.grants(candidate) not in suffix:
            raise ToolingError("Final init explicit grants changed")
        if hashlib.sha256(source).hexdigest() != self.contract.review.final_sha256:
            raise ToolingError("Final init SHA-256 does not match reviewed finalization bytes")
        for _, _, statement in segments(source):
            token = re.sub(rb"^--[^\n]*\n", b"", statement, flags=re.M).strip()
            if re.match(
                rb"(?:BEGIN|COMMIT|ROLLBACK|END|VACUUM|CREATE DATABASE|ALTER SYSTEM|DISCARD)\b",
                token,
                re.I,
            ) or re.match(
                rb"(?:SET|RESET)\s+(?:LOCAL\s+|SESSION\s+)?(?:" + b"|".join(TIMEOUTS) + rb"|ALL)\b",
                token,
                re.I,
            ):
                raise ToolingError("Final init contains forbidden transaction or timeout control")
        return source

    def generate(self, source: bytes, candidate: bytes) -> bytes:
        _, marker, suffix = source.partition(MARKER)
        if not marker:
            raise ToolingError("Authored final lifecycle section is missing")
        suffix = replace_section(suffix, SECURITY_START, SECURITY_END, self.security(candidate))
        suffix = replace_section(suffix, GRANTS_START, GRANTS_END, self.grants(candidate))
        return self.legacy(candidate) + MARKER + suffix
