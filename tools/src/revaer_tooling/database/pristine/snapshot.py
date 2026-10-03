"""Read every reviewed catalog through one constrained owner's transaction."""

import json
import re

from ...errors import ToolingError
from ...json_data import JsonObject, array_value, decode_unique, object_value, string_value
from ..postgres import Connection
from .query import Query, literal
from .spec import Specification

IDENTITY: JsonObject = {
    "kind": "identity",
    "database": "<database>",
    "owner": "<database_owner>",
    "server_version_num": "160014",
    "encoding": "UTF8",
    "collate": "C",
    "ctype": "C",
    "integer_datetimes": "on",
    "standard_conforming_strings": "on",
    "data_checksums": "on",
    "owner_valid": True,
}


def records(source: str) -> list[JsonObject]:
    return [object_value(decode_unique(line)) for line in source.splitlines()]


def validate_row(row: JsonObject) -> None:
    encoded = json.dumps(row, ensure_ascii=False, allow_nan=False, separators=(",", ":"))
    if "<unresolved_reference>" in encoded:
        raise ToolingError("Unresolved catalog identity")
    library = row.get("probin")
    if library is not None and (
        not isinstance(library, str) or not re.fullmatch(r"\$libdir/[a-zA-Z0-9_-]+", library)
    ):
        raise ToolingError("Catalog requires an unrepresentable filesystem library location")
    if re.search(r"pg_(?:toast_)?temp_[0-9]+", encoded):
        raise ToolingError("Catalog output retained a raw temporary identity")
    if re.search(r"pg_toast_[0-9]+", encoded):
        raise ToolingError("Catalog output retained an OID-derived TOAST name")


class Snapshot:
    def __init__(
        self, connection: Connection, database: str, owner: str, spec: Specification
    ) -> None:
        self.connection, self.database, self.owner, self.spec = connection, database, owner, spec
        self.query = Query(spec)
        self.inventory: dict[str, tuple[tuple[str, str], ...]] = {}
        for row in records(self.sql(self.query.inventory())):
            if row.get("kind") != "columns" or row.keys() != {"kind", "catalog", "columns"}:
                raise ToolingError("Malformed pristine catalog inventory")
            catalog = string_value(row["catalog"])
            if catalog in self.inventory:
                raise ToolingError("Duplicate pristine catalog inventory")
            columns = []
            for item in array_value(row["columns"]):
                pair = array_value(item)
                if len(pair) != 2:
                    raise ToolingError("Malformed pristine catalog column")
                columns.append((string_value(pair[0]), string_value(pair[1])))
            self.query.projection(catalog, tuple(columns))
            self.inventory[catalog] = tuple(columns)
        if self.inventory.keys() != spec.columns.keys():
            raise ToolingError("Missing pristine catalog inventory")

    def sql(self, source: str) -> str:
        return self.connection.sql(source, database=self.database, role=self.owner)

    def batch(self, prefix: str = "") -> str:
        statements = []
        for catalog in self.spec.columns:
            query = self.query.catalog(catalog, self.inventory[catalog])
            if catalog in ("pg_subscription", "pg_user_mapping"):
                # Reading credential-bearing projections requires privileges the
                # owner does not have. Prove emptiness using readable identities;
                # populated catalogs attempt the full projection and must fail.
                empty = (
                    f"SELECT json_build_object('kind','rows','catalog','{catalog}',"
                    "'rows','[]'::json);"
                )
                existence = (
                    "SELECT umid FROM pg_user_mappings"
                    if catalog == "pg_user_mapping"
                    else "SELECT oid FROM pg_subscription"
                )
                statements.append(
                    f"SELECT CASE WHEN NOT EXISTS ({existence}) "
                    f"THEN {literal(empty)} ELSE {literal(query)} END\n\\gexec\n"
                )
            else:
                statements.append(query)
        return (
            "BEGIN ISOLATION LEVEL REPEATABLE READ;\nSET LOCAL search_path = pg_catalog;\n"
            "SET LOCAL statement_timeout = '120s';\nSET LOCAL lock_timeout = '120s';\n"
            "SET LOCAL idle_in_transaction_session_timeout = '30s';\n"
            + prefix
            + "\nSET TRANSACTION READ ONLY;\n"
            + self.query.identity()
            + "\n".join(statements)
            + "\nROLLBACK;\n"
        )

    def read(self, prefix: str = "") -> bytes:
        values = records(self.sql(self.batch(prefix)))
        if not values or values[0] != IDENTITY or values[0].get("owner_valid") is not True:
            raise ToolingError("Unsupported pristine PostgreSQL or owner identity")
        catalogs = values[1:]
        if tuple(row.get("catalog") for row in catalogs) != tuple(self.spec.columns):
            raise ToolingError("Incomplete pristine catalog result")
        output = ["#postgres-pristine\t16.14\t<database>\t<database_owner>\n"]
        for record in catalogs:
            if record.get("kind") != "rows" or record.keys() != {"kind", "catalog", "rows"}:
                raise ToolingError("Malformed pristine catalog rows")
            catalog = string_value(record["catalog"])
            rows = array_value(record["rows"])
            output.append(f"#catalog\t{catalog}\t{len(rows)}\n")
            for value in rows:
                row = object_value(value)
                validate_row(row)
                fields = [
                    key
                    + "="
                    + json.dumps(item, ensure_ascii=False, allow_nan=False, separators=(",", ":"))
                    for key, item in sorted(row.items())
                ]
                output.append(catalog + "\t" + "\t".join(fields) + "\n")
        if len(output) != len(set(output)):
            raise ToolingError("Duplicate catalog row evidence")
        return b"".join(sorted(line.encode("utf-8") for line in output))
