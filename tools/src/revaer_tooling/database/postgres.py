"""Own proof containers from allocation through verified storage removal."""

import json
import re
import tempfile
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Protocol
from urllib.parse import quote

from ..errors import ToolingError
from ..external.postgres import PostgresDocker, ProofContainerArgs, QueryArgs
from ..filesystem import FileSystem
from ..json_data import JsonObject
from .contract import PostgresPin


class ProofScope(Protocol):
    """Only reviewed native pins and a validated output directory are required."""

    @property
    def output(self) -> Path: ...

    @property
    def postgres(self) -> PostgresPin: ...


@dataclass(frozen=True)
class Connection:
    docker: PostgresDocker = field(repr=False)
    fs: FileSystem = field(repr=False)
    container: str
    user: str
    database: str
    password: str = field(repr=False)
    port: int | None
    output: Path

    def url(self) -> str:
        if self.port is None:
            raise ToolingError("This proof database has no published connection endpoint")
        return (
            f"postgresql://{self.user}:{quote(self.password, safe='')}@127.0.0.1:"
            f"{self.port}/{self.database}?sslmode=disable"
        )

    def sql(
        self,
        source: str,
        *,
        database: str | None = None,
        role: str | None = None,
        transaction: bool = False,
    ) -> str:
        outcome = self.docker.query(
            self.container,
            QueryArgs(
                role or self.user,
                database or self.database,
                source,
                transaction,
            ),
        )
        if outcome.code != 0 or re.search(r"\bWARNING\b", outcome.stderr):
            self.fs.write(self.output / "query-error.txt", outcome.stderr, 0o600)
            state = re.search(r"ERROR:\s+([A-Z0-9]{5})(?:\s|$)", outcome.stderr)
            detail = f" SQLSTATE={state[1]}" if state else ""
            raise ToolingError(
                f"PostgreSQL proof query failed or warned;{detail} retained query-error.txt"
            )
        return outcome.stdout.strip()


class ProofDatabase:
    def __init__(
        self,
        docker: PostgresDocker,
        fs: FileSystem,
        sleep: Callable[[float], None],
        token: Callable[[], str],
    ) -> None:
        self.docker, self.fs, self.sleep, self.token = docker, fs, sleep, token

    @contextmanager
    def open(
        self,
        contract: ProofScope,
        *,
        user: str,
        database: str,
        publish: bool = False,
        logical_wal: bool = False,
    ) -> Iterator[Connection]:
        output = contract.output
        self.fs.mkdir(output)
        output.chmod(0o700)
        token = self.token()
        if not re.fullmatch(r"[0-9a-f]{32}", token):
            raise ToolingError("Proof ownership requires a fresh 128-bit hexadecimal token")
        name = "rv-pg-proof-" + token
        password = self.token()
        if not re.fullmatch(r"[0-9a-f]{32}", password):
            raise ToolingError("Proof credentials require a fresh 128-bit hexadecimal token")
        receipt = output / f"postgres-{token}.json"
        if receipt.exists() or receipt.is_symlink():
            raise ToolingError("Proof ownership token already has a receipt; refusing to reuse it")
        evidence: JsonObject = {
            "container_name": name,
            "image": contract.postgres.image,
            "container_id": None,
            "volumes": [],
            "completed": False,
            "storage_removed": False,
        }
        # Persist the unique ownership token before Docker can create a resource.
        # An interrupted/uncertain create can be recovered by exact name + label.
        self.fs.write(receipt, json.dumps(evidence, indent=2) + "\n", 0o600)
        container: str | None = None
        volumes: tuple[str, ...] = ()
        primary: BaseException | None = None
        with tempfile.TemporaryDirectory(prefix="postgres-credentials-", dir=output) as temporary:
            credentials = Path(temporary) / "environment"
            self.fs.write(credentials, f"POSTGRES_PASSWORD={password}\n", 0o600)
            try:
                container = self.docker.create(
                    ProofContainerArgs(
                        name,
                        contract.postgres,
                        user,
                        database,
                        credentials,
                        publish,
                        logical_wal,
                    )
                )
                evidence["container_id"] = container
                volumes = self.docker.volumes(container)
                evidence["volumes"] = list(volumes)
                self.fs.write(receipt, json.dumps(evidence, indent=2) + "\n", 0o600)
                self.docker.start(container)
                for _ in range(120):
                    if self.docker.ready(container, user, database):
                        break
                    self.sleep(0.25)
                else:
                    raise ToolingError("Proof PostgreSQL did not become ready")
                port = self.docker.port(container) if publish else None
                yield Connection(
                    self.docker, self.fs, container, user, database, password, port, output
                )
                evidence["completed"] = True
            except BaseException as error:
                primary = error
                raise
            finally:
                failures = []
                try:
                    if container is None:
                        container = self.docker.find_owned(name)
                        evidence["container_id"] = container
                    if container is not None:
                        try:
                            if not volumes:
                                volumes = self.docker.volumes(container)
                                evidence["volumes"] = list(volumes)
                        finally:
                            # Even malformed inspection data must not prevent
                            # removing a container whose creation we own.
                            self.docker.remove(container)
                        self.docker.require_volumes_removed(volumes)
                    evidence["storage_removed"] = True
                except (ToolingError, OSError) as error:
                    failures.append(error)
                    evidence["cleanup_error"] = str(error)
                try:
                    self.fs.write(receipt, json.dumps(evidence, indent=2) + "\n", 0o600)
                except (ToolingError, OSError) as error:
                    failures.append(error)
                if failures:
                    details = "; ".join(str(error) for error in failures)
                    earlier = f"; earlier {type(primary).__name__}: {primary}" if primary else ""
                    raise ToolingError(
                        f"PostgreSQL proof cleanup failed: {details}{earlier}"
                    ) from failures[0]
