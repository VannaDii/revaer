"""Run one ingestion case in an owned clone and retain native before/after evidence."""

import json
import re
from enum import StrEnum

from ...errors import ToolingError
from ...external.postgres import QueryArgs, identifier
from ...json_data import JsonObject, array_value, decode_unique, object_value
from ..parity import literal
from ..postgres import Connection
from .evidence import IngestionEvidence, comparable
from .inventory import TABLES


class Variant(StrEnum):
    REFERENCE = "reference"
    FINAL = "final"


class IsolatedIngestion:
    def __init__(
        self,
        connection: Connection,
        owner: str,
        runtime: str,
        seed_sql: str,
        evidence: IngestionEvidence,
    ) -> None:
        self.connection = connection
        self.owner, self.runtime = identifier(owner), identifier(runtime)
        self.seed_sql, self.evidence = seed_sql, evidence
        if not seed_sql.strip():
            raise ToolingError("Isolated ingestion requires its seed fixture")
        if evidence.runtime != runtime:
            raise ToolingError(
                "Isolated ingestion runtime differs from the verified evidence parser"
            )

    def snapshot(self, database: str) -> JsonObject:
        pairs = []
        for table in TABLES:
            pairs.append(
                f"{literal(table)}, (SELECT COALESCE(json_agg(row_to_json(t) "
                "ORDER BY to_jsonb(t)->(SELECT a.attname FROM pg_attribute a "
                f"WHERE a.attrelid = 'public.{table}'::regclass AND a.attnum = 1)), '[]') "
                f"FROM public.{table} t)"
            )
        value = object_value(
            decode_unique(
                self.connection.sql(
                    "SELECT json_build_object(" + ",".join(pairs) + ");",
                    role="postgres",
                    database=database,
                )
            )
        )
        if set(value) != set(TABLES):
            raise ToolingError("Ingestion snapshot table inventory changed")
        for rows in value.values():
            array_value(rows)
        return value

    def run(
        self, name: str, query: str, variant: Variant, *, helpers_first: bool = False
    ) -> JsonObject:
        if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name):
            raise ToolingError("Ingestion case name must be a safe lowercase artifact name")
        if variant == Variant.REFERENCE:
            source, role = "reference_proof", "postgres"
        elif variant == Variant.FINAL:
            source, role = self.connection.database, self.runtime
        else:
            raise ToolingError("Unknown ingestion proof variant")
        database = "ingestion_" + variant.value + "_proof"
        if database == source:
            raise ToolingError("Isolated ingestion cannot replace its source database")
        prefix = self.connection.output / f"{name}-{variant.value}"
        for suffix in (".sql", ".stdout", ".stderr", "-tables.json"):
            self.connection.fs.remove_owned(
                prefix.parent / (prefix.name + suffix), self.connection.output
            )
        self.connection.fs.write(prefix.with_suffix(".sql"), query, 0o600)
        created = False
        primary: BaseException | None = None
        try:
            # CREATE must succeed before we own this clone. A name collision is
            # not permission to drop a database belonging to another case.
            self.connection.sql(
                f"CREATE DATABASE {database} TEMPLATE {identifier(source)} OWNER {self.owner}",
                role="postgres",
                database="postgres",
            )
            created = True
            self.connection.sql(
                f"REVOKE ALL ON DATABASE {database} FROM PUBLIC; "
                f"GRANT CONNECT ON DATABASE {database} TO {self.runtime}",
                role="postgres",
                database=database,
            )
            self.connection.sql(self.seed_sql, role="postgres", database=database)
            before = self.snapshot(database)
            outcome = self.connection.docker.query(
                self.connection.container, QueryArgs(role, database, query)
            )
            self.connection.fs.write(prefix.with_suffix(".stdout"), outcome.stdout, 0o600)
            self.connection.fs.write(prefix.with_suffix(".stderr"), outcome.stderr, 0o600)
            if outcome.code != 0 or re.search(r"\bWARNING\b", outcome.stderr):
                raise ToolingError("Ingestion proof transport failed; retained native output")
            value = self.evidence.parse(
                outcome.stdout, outcome.stderr, role=role, helpers_first=helpers_first
            )
            after = self.snapshot(database)
            self.connection.fs.write(
                prefix.parent / (prefix.name + "-tables.json"),
                json.dumps({"before": before, "after": after}, indent=2) + "\n",
                0o600,
            )
            result = comparable(value | {"before": before, "after": after})
        except BaseException as error:
            primary = error
            raise
        finally:
            if created:
                try:
                    self.connection.sql(
                        f"DROP DATABASE {database} WITH (FORCE)",
                        role="postgres",
                        database="postgres",
                    )
                except (ToolingError, OSError) as cleanup:
                    earlier = f"; earlier {type(primary).__name__}: {primary}" if primary else ""
                    raise ToolingError(
                        f"Ingestion clone cleanup failed: {cleanup}{earlier}"
                    ) from cleanup
        # No successful case can escape before its owned clone is removed.
        return result
