"""Typed Docker/PostgreSQL operations for task-owned disposable proof databases."""

import re
import stat
from dataclasses import dataclass, field
from pathlib import Path

from ..database.contract import PostgresPin
from ..errors import ToolingError
from ..json_data import JsonObject, array_value, decode, object_value, string_value
from ..process import Completed
from .base import ExternalTool


def identifier(value: str) -> str:
    if not re.fullmatch(r"[a-z][a-z0-9_]{0,62}", value):
        raise ToolingError("Proof database and role names must be lowercase SQL identifiers")
    return value


def container_id(value: str) -> str:
    if not re.fullmatch(r"[0-9a-f]{64}", value):
        raise ToolingError("Docker proof operations require a complete container ID")
    return value


@dataclass(frozen=True)
class ProofContainerArgs:
    name: str
    pin: PostgresPin
    user: str
    database: str
    credentials: Path
    publish: bool = False
    logical_wal: bool = False


@dataclass(frozen=True)
class QueryArgs:
    role: str
    database: str
    sql: str = field(repr=False)
    transaction: bool = False


@dataclass(frozen=True)
class DumpArgs:
    role: str
    database: str
    no_privileges: bool = False
    exclude_schemas: tuple[str, ...] = ()
    exclude_tables: tuple[str, ...] = ()


class PostgresDocker(ExternalTool):
    def image_identity(self, pin: PostgresPin) -> JsonObject:
        raw = self._invoke(("image", "inspect", pin.image), capture=True).stdout
        rows = array_value(decode(raw))
        if len(rows) != 1:
            raise ToolingError("Proof image inspection must identify exactly one image")
        row = object_value(rows[0])
        identity = string_value(row.get("Id"))
        if not re.fullmatch(r"sha256:[0-9a-f]{64}", identity):
            raise ToolingError("Proof image inspection returned an invalid image ID")
        digests = array_value(row.get("RepoDigests"))
        if not digests or any(not isinstance(digest, str) for digest in digests):
            raise ToolingError("Proof image inspection is missing registry digests")
        return {
            "Id": identity,
            "RepoDigests": digests,
            "Architecture": string_value(row.get("Architecture")),
            "Os": string_value(row.get("Os")),
        }

    def find_owned(self, name: str) -> str | None:
        if not re.fullmatch(r"rv-pg-proof-[0-9a-f]{32}", name):
            raise ToolingError("Proof lookup requires the exact generated container name")
        result = self._invoke(
            (
                "container",
                "ls",
                "--all",
                "--no-trunc",
                "--quiet",
                "--filter",
                f"name=^/{name}$",
                "--filter",
                f"label=io.revaer.rv.database-proof={name}",
            ),
            capture=True,
        ).stdout.split()
        if len(result) > 1:
            raise ToolingError("Docker returned multiple matches for one proof ownership token")
        return container_id(result[0]) if result else None

    def verify_image(self, pin: PostgresPin) -> None:
        # Docker resolves and verifies the reviewed registry digest. Each native
        # version command is independent; there is no shell embedded in the image.
        for executable in ("pg_dump", "psql", "postgres"):
            actual = self._invoke(
                (
                    "run",
                    "--rm",
                    "--network=none",
                    "--entrypoint",
                    executable,
                    pin.image,
                    "--version",
                ),
                capture=True,
                timeout=120,
            ).stdout.strip()
            expected = f"{executable} (PostgreSQL) {pin.version}"
            if actual not in (expected, expected + " (Debian 18.6-1.pgdg12+2)"):
                raise ToolingError(f"Pinned proof image has an unexpected {executable} version")

    def create(self, args: ProofContainerArgs) -> str:
        if not re.fullmatch(r"rv-pg-proof-[0-9a-f]{32}", args.name):
            raise ToolingError("Proof containers require a fresh rv-owned name")
        metadata = args.credentials.lstat()
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
            raise ToolingError("PostgreSQL credentials must be a regular, unlinked file")
        if stat.S_IMODE(metadata.st_mode) != 0o600:
            raise ToolingError("PostgreSQL credential file must have mode 0600")
        network = ("--publish", "127.0.0.1::5432") if args.publish else ("--network=none",)
        return container_id(
            self._invoke(
                (
                    "container",
                    "create",
                    "--name",
                    args.name,
                    "--shm-size=1g",
                    "--label",
                    f"io.revaer.rv.database-proof={args.name}",
                    *network,
                    "--env-file",
                    str(args.credentials),
                    "--env",
                    f"POSTGRES_USER={identifier(args.user)}",
                    "--env",
                    f"POSTGRES_DB={identifier(args.database)}",
                    "--env",
                    "PGDATA=/var/lib/postgresql/18/docker",
                    "--env",
                    "POSTGRES_INITDB_ARGS=--locale=C --encoding=UTF8 --data-checksums",
                    "--env",
                    "TZ=UTC",
                    args.pin.image,
                    *(
                        ("postgres", "-c", "timezone=UTC", "-c", "wal_level=logical")
                        if args.logical_wal
                        else ()
                    ),
                ),
                capture=True,
            ).stdout.strip()
        )

    def volumes(self, container: str) -> tuple[str, ...]:
        raw = self._invoke(
            ("container", "inspect", "--format", "{{json .Mounts}}", container_id(container)),
            capture=True,
        ).stdout
        mounts = array_value(decode(raw))
        if len(mounts) != 1:
            raise ToolingError("Proof container must have exactly one disposable data volume")
        mount = object_value(mounts[0])
        if (
            mount.get("Type") != "volume"
            or mount.get("RW") is not True
            or mount.get("Destination") != "/var/lib/postgresql"
        ):
            raise ToolingError("Proof container data mount differs from the owned volume contract")
        name = string_value(mount.get("Name"))
        if not re.fullmatch(r"[0-9a-f]{64}", name):
            raise ToolingError("Proof PostgreSQL storage must be a Docker-owned anonymous volume")
        return (name,)

    def start(self, container: str) -> None:
        self._invoke(("container", "start", container_id(container)), capture=True)

    def ready(self, container: str, user: str, database: str) -> bool:
        outcome = self._invoke(
            (
                "exec",
                container_id(container),
                "pg_isready",
                "--quiet",
                "--timeout=1",
                "--host=127.0.0.1",
                f"--username={identifier(user)}",
                f"--dbname={identifier(database)}",
            ),
            capture=True,
            timeout=5,
            accepted_codes=(0, 1, 2),
        )
        return outcome.code == 0

    def port(self, container: str) -> int:
        value = self._invoke(("port", container_id(container), "5432/tcp"), capture=True).stdout
        match = re.fullmatch(r"127\.0\.0\.1:([0-9]+)\n?", value)
        if match is None or not 1 <= int(match[1]) <= 65535:
            raise ToolingError("Proof PostgreSQL must publish exactly one loopback port")
        return int(match[1])

    def query(self, container: str, args: QueryArgs) -> Completed:
        return self._invoke(
            (
                "exec",
                "-i",
                container_id(container),
                "psql",
                "--no-psqlrc",
                "--no-password",
                "--quiet",
                "--tuples-only",
                "--no-align",
                "--set=ON_ERROR_STOP=1",
                "--set=VERBOSITY=sqlstate",
                f"--username={identifier(args.role)}",
                f"--dbname={identifier(args.database)}",
                *(("--single-transaction",) if args.transaction else ()),
            ),
            capture=True,
            input_text=args.sql,
            accepted_codes=(0, 1, 2, 3),
        )

    def create_database(self, container: str, role: str, database: str) -> None:
        self._invoke(
            (
                "exec",
                container_id(container),
                "createdb",
                f"--username={identifier(role)}",
                "--template=template0",
                "--locale=C",
                "--encoding=UTF8",
                identifier(database),
            ),
            capture=True,
        )

    def dump(self, container: str, args: DumpArgs) -> str:
        return self._invoke(
            (
                "exec",
                container_id(container),
                "pg_dump",
                f"--username={identifier(args.role)}",
                f"--dbname={identifier(args.database)}",
                "--schema-only",
                "--no-owner",
                "--no-tablespaces",
                *(("--no-privileges",) if args.no_privileges else ()),
                *(f"--exclude-schema={name}" for name in args.exclude_schemas),
                *(f"--exclude-table={name}" for name in args.exclude_tables),
            ),
            capture=True,
        ).stdout

    def remove(self, container: str) -> None:
        self._invoke(
            ("container", "rm", "--force", "--volumes", container_id(container)), capture=True
        )

    def require_volumes_removed(self, names: tuple[str, ...]) -> None:
        for name in names:
            if not re.fullmatch(r"[0-9a-f]{64}", name):
                raise ToolingError("Proof cleanup requires exact owned volume IDs")
            remaining = self._invoke(
                ("volume", "ls", "--quiet", "--filter", f"name=^{name}$"),
                capture=True,
            ).stdout.strip()
            if remaining:
                raise ToolingError("Docker retained an owned PostgreSQL data volume after cleanup")
