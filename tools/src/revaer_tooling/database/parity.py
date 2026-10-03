"""Compare native schema and seed bytes using only the reviewed normalizations."""

import json
import re
from collections.abc import Callable

from ..errors import ToolingError
from ..external.postgres import DumpArgs, identifier
from ..json_data import array_value, decode_unique, object_value, string_value
from .candidate import RESTRICT
from .final_sql import approved_legacy_deltas
from .postgres import Connection

PUBLIC_COMMENT = (
    b"--\n-- Name: public; Type: SCHEMA; Schema: -; Owner: -\n--\n\n"
    b"-- *not* creating schema, since initdb creates it\n\n\n"
)
UUID4 = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}")


def normalize_routine_security(source: bytes) -> bytes:
    """Remove only separately-proven routine security headers, never body text."""
    source = source.replace(PUBLIC_COMMENT, b"", 1)

    def header(match: re.Match[bytes]) -> bytes:
        changed = re.sub(rb" SECURITY DEFINER\b", b"", match[0])
        return re.sub(rb"^    SET search_path TO [^\n]*\n", b"", changed, flags=re.M)

    return re.sub(rb"^CREATE FUNCTION .*?^    AS ", header, source, flags=re.M | re.S)


def normalize_seed_identities(source: str) -> str:
    """Normalize only the two generated UUIDv4 identifiers approved by the proof."""
    result = []
    for line in source.splitlines(keepends=True):
        if not line.startswith("public.rate_limit_policy:"):
            result.append(line)
            continue
        table, encoded = line.split(":", 1)
        rows = [object_value(row) for row in array_value(decode_unique(encoded))]
        ids = [string_value(row.get("rate_limit_policy_public_id")) for row in rows]
        if (
            len(ids) != 2
            or len(set(ids)) != 2
            or any(UUID4.fullmatch(value) is None for value in ids)
        ):
            raise ToolingError("Canonical seed public identities are invalid")
        for row in rows:
            row["rate_limit_policy_public_id"] = f"<{string_value(row.get('display_name'))}>"
        result.append(
            table + ":" + json.dumps(rows, ensure_ascii=False, separators=(",", ":")) + "\n"
        )
    return "".join(result)


def literal(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


class DatabaseParity:
    def __init__(
        self, connection: Connection, owner: str, check: Callable[[str, bool], None]
    ) -> None:
        self.connection, self.owner, self.check = connection, identifier(owner), check

    def schema(self, database: str) -> bytes:
        source = self.connection.docker.dump(
            self.connection.container,
            DumpArgs("postgres", database, no_privileges=True, exclude_schemas=("revaer_system",)),
        ).encode()
        return normalize_routine_security(RESTRICT.sub(b"", source))

    def seed_query(self) -> str:
        # JSON avoids psql delimiter ambiguity in catalog names. PostgreSQL quotes
        # qualified identifiers; column names become escaped SQL string literals.
        tables = array_value(
            decode_unique(
                self.connection.sql(
                    """
            SELECT COALESCE(json_agg(row_to_json(r) ORDER BY r.name), '[]') FROM (
              SELECT format('%I.%I', n.nspname, c.relname) AS name,
                COALESCE(json_agg(a.attname ORDER BY a.attnum)
                  FILTER (WHERE a.atttypid IN ('timestamptz'::regtype, 'timestamp'::regtype)),
                  '[]') AS clocks
              FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
              LEFT JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped
              WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime') AND c.relkind = 'r'
              GROUP BY n.nspname, c.relname
            ) r;
        """,
                    role=self.owner,
                )
            )
        )
        queries = []
        for value in tables:
            table = object_value(value)
            name = string_value(table.get("name"))
            clocks = ",".join(
                literal(string_value(item)) for item in array_value(table.get("clocks"))
            )
            queries.append(
                f"SELECT {literal(name)} || ':' || "
                "COALESCE(jsonb_agg(v ORDER BY v::text)::text, '[]') "
                f"FROM (SELECT to_jsonb(t) - ARRAY[{clocks}]::text[] AS v FROM {name} t) rows;"
            )
        if not queries:
            raise ToolingError("Seed parity requires a nonempty native table inventory")
        return "\n".join(queries)

    def verify(self) -> None:
        output = self.connection.output
        for kind in ("schema.sql", "seeds.jsonl"):
            for variant in ("reference", "observed"):
                self.connection.fs.remove_owned(output / f"final-{variant}-{kind}", output)
        reference = approved_legacy_deltas(self.schema("reference_proof"))
        observed = self.schema(self.connection.database)
        for variant, source in (("reference", reference), ("observed", observed)):
            self.connection.fs.write_bytes(output / f"final-{variant}-schema.sql", source, 0o600)
        self.check("two-way legacy schema and extension parity", observed == reference)
        query = self.seed_query()
        reference_seeds = normalize_seed_identities(
            self.connection.sql(query, role="postgres", database="reference_proof")
        )
        observed_seeds = normalize_seed_identities(self.connection.sql(query, role=self.owner))
        for variant, source_text in (("reference", reference_seeds), ("observed", observed_seeds)):
            self.connection.fs.write(output / f"final-{variant}-seeds.jsonl", source_text, 0o600)
        self.check(
            "two-way seed parity excluding generated timestamp columns",
            observed_seeds == reference_seeds,
        )
