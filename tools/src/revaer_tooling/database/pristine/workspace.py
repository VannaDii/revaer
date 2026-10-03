"""Pristine proofs need the PostgreSQL pin, not an application transition state."""

import hashlib
import json
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from ...errors import ToolingError
from ...filesystem import FileSystem
from ...json_data import Json, JsonObject
from ..contract import BUILD_INPUTS, PostgresPin, assignments, owned_path
from ..postgres import Connection
from .snapshot import IDENTITY, Snapshot

EXPECTED = "config/postgres-pristine-16.14.tsv"


@dataclass(frozen=True)
class Workspace:
    root: Path
    postgres: PostgresPin

    @staticmethod
    def load(root: Path, fs: FileSystem) -> "Workspace":
        pin = PostgresPin.load(assignments(root, fs, BUILD_INPUTS))
        if pin.version != "16.14":
            raise ToolingError("Pristine PostgreSQL input must pin the approved 16.14 release")
        return Workspace(root, pin)

    @property
    def output(self) -> Path:
        path = owned_path(self.root, "target/postgres-pristine")
        if path.is_symlink() or (path.exists() and not path.is_dir()):
            raise ToolingError("Pristine evidence output must be an unlinked directory")
        return path


def provision(connection: Connection, suffix: str) -> tuple[str, str]:
    # Names are derived from the already recorded random ownership token. Native
    # psql still receives values through stdin, never through shell evaluation.
    if (
        not suffix
        or any(character not in "0123456789abcdef" for character in suffix)
        or len(suffix) > 32
    ):
        raise ToolingError("Pristine fixture identity requires a bounded hexadecimal suffix")
    owner, database = "pristine_owner_" + suffix, "pristine_database_" + suffix
    connection.sql(
        f"CREATE ROLE {owner} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE "
        "NOREPLICATION NOBYPASSRLS;\n"
        f"CREATE DATABASE {database} OWNER {owner} TEMPLATE template0 "
        "ENCODING 'UTF8' LC_COLLATE 'C' LC_CTYPE 'C';\n"
    )
    return database, owner


def provenance(
    workspace: Workspace, snapshot: Snapshot, source: bytes, image: JsonObject, invoked_at: datetime
) -> JsonObject:
    columns: dict[str, Json] = {
        catalog: [[column, kind] for column, kind in entries]
        for catalog, entries in snapshot.inventory.items()
    }
    excluded: dict[str, Json] = dict(snapshot.spec.excluded)
    return {
        "postgres_image": workspace.postgres.image,
        "postgres_version": workspace.postgres.version,
        "image_identity": image,
        "database_identity": IDENTITY,
        "snapshot_sha256": hashlib.sha256(source).hexdigest(),
        "snapshot_bytes": len(source),
        "snapshot_lines": source.count(b"\n"),
        "catalog_count": len(snapshot.spec.columns),
        "columns": columns,
        "excluded_columns": excluded,
        "oid_handling": (
            "row OIDs omitted; references resolved; TOAST identities derive from parent relation"
        ),
        "reader": (
            "direct constrained database owner connection; no role substitution or extra grants"
        ),
        "transaction": "REPEATABLE READ READ ONLY; fixed pg_catalog search_path",
        "generated_at_utc": invoked_at.strftime("%Y-%m-%dT%H:%M:%SZ"),
    }


def publish(workspace: Workspace, fs: FileSystem, source: bytes, evidence: JsonObject) -> None:
    fs.write_bytes(workspace.output / "postgres-pristine-16.14.tsv", source, 0o600)
    fs.write(workspace.output / "provenance.json", json.dumps(evidence, indent=2) + "\n", 0o600)
