"""Docker operations for persistent, explicitly owned development databases."""

import csv
import io
import json
import re
from dataclasses import dataclass, field
from pathlib import Path

from ..errors import ToolingError
from .base import ExternalTool

OWNER_LABEL = "io.revaer.rv.database-owner"


@dataclass(frozen=True)
class PostgresContainerArgs:
    name: str
    owner: str
    directory: Path
    port: int
    shared_memory_bytes: int
    user: str
    password: str = field(repr=False)
    uid: int
    gid: int


@dataclass(frozen=True)
class DatabaseContainer:
    identifier: str
    owner: str
    directory: Path
    port: int
    shared_memory_bytes: int
    running: bool


class Docker(ExternalTool):
    def database(self, name: str) -> DatabaseContainer | None:
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]+", name):
            raise ToolingError("PG_CONTAINER is not a valid Docker container name")
        identifiers = self._invoke(
            (
                "container",
                "ls",
                "--all",
                "--no-trunc",
                "--filter",
                f"name=^/{re.escape(name)}$",
                "--format",
                "{{.ID}}",
            ),
            capture=True,
        ).stdout.split()
        if not identifiers:
            return None
        if len(identifiers) != 1:
            raise ToolingError("Docker returned more than one exact database container match")
        identifier = self._identifier(identifiers[0])
        document = self._invoke(("container", "inspect", identifier), capture=True).stdout
        try:
            items = json.loads(document)
            item = items[0]
            owner = (item["Config"]["Labels"] or {}).get(OWNER_LABEL, "")
            # Ownership is checked before interpreting other container details.
            # An unrelated container can have any image, ports, or mount layout.
            if not owner:
                raise ToolingError(f"Container {name} is not managed by rv")
            mounts = [
                mount
                for mount in item["Mounts"]
                if mount["Destination"] == "/var/lib/postgresql/data" and mount["Type"] == "bind"
            ]
            bindings = item["HostConfig"]["PortBindings"]["5432/tcp"]
            if (
                len(items) != 1
                or len(mounts) != 1
                or len(bindings) != 1
                or bindings[0]["HostIp"] != "127.0.0.1"
            ):
                raise ToolingError(
                    "Managed database container has unexpected storage or port bindings"
                )
            running = item["State"]["Running"]
            if (
                not isinstance(running, bool)
                or item["Id"] != identifier
                or not isinstance(owner, str)
            ):
                raise ToolingError("Managed database container has invalid identity or state")
            return DatabaseContainer(
                identifier,
                owner,
                Path(mounts[0]["Source"]),
                int(bindings[0]["HostPort"]),
                int(item["HostConfig"]["ShmSize"]),
                running,
            )
        except (ValueError, KeyError, TypeError, IndexError) as error:
            raise ToolingError("Docker returned invalid database container metadata") from error

    def create_database(self, args: PostgresContainerArgs) -> str:
        # PGDATA is a child of the bind mount. The outer directory holds rv's
        # ownership record/lock without interfering with initdb's empty-dir rule.
        mount = io.StringIO()
        csv.writer(mount, lineterminator="").writerow(
            ("type=bind", f"source={args.directory}", "target=/var/lib/postgresql/data")
        )
        result = self._invoke(
            (
                "container",
                "create",
                "--read-only",
                "--name",
                args.name,
                "--label",
                f"{OWNER_LABEL}={args.owner}",
                "--user",
                f"{args.uid}:{args.gid}",
                "--mount",
                mount.getvalue(),
                "--tmpfs",
                "/var/run/postgresql:rw,size=16m,mode=1777",
                "--tmpfs",
                "/tmp:rw,size=16m,mode=1777",
                "--shm-size",
                str(args.shared_memory_bytes),
                "--publish",
                f"127.0.0.1:{args.port}:5432",
                "--env",
                "POSTGRES_USER",
                "--env",
                "POSTGRES_PASSWORD",
                "--env",
                "POSTGRES_DB=postgres",
                "--env",
                "PGDATA=/var/lib/postgresql/data/pgdata",
                "postgres:16-alpine",
                "postgres",
                "-c",
                f"revaer.rv_owner={args.owner}",
            ),
            env={"POSTGRES_USER": args.user, "POSTGRES_PASSWORD": args.password},
            capture=True,
        )
        return self._identifier(result.stdout.strip())

    @staticmethod
    def _identifier(value: str) -> str:
        if not re.fullmatch(r"[a-f0-9]{64}", value):
            raise ToolingError("Docker did not return a complete container ID")
        return value

    def start_database(self, identifier: str) -> None:
        self._invoke(("container", "start", self._identifier(identifier)), capture=True)

    def remove_database_container(self, identifier: str) -> None:
        """Remove the proven owned container; its bind-mounted data remains."""
        self._invoke(("container", "rm", "--force", self._identifier(identifier)), capture=True)
