"""Fault injection for proof ownership; native PostgreSQL covers actual SQL behavior."""

import sys
from pathlib import Path

from revaer_tooling.database.contract import PostgresPin
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.postgres import DumpArgs, PostgresDocker, ProofContainerArgs, QueryArgs
from revaer_tooling.process import Completed, ProcessRunner

CONTAINER = "a" * 64
VOLUME = "b" * 64


class ProofDocker(PostgresDocker):
    def __init__(self, root: Path) -> None:
        super().__init__(sys.executable, ProcessRunner(lambda message: None), root, {})
        self.calls: list[str] = []
        self.failures: set[str] = set()
        self.exists = False
        self.is_ready = True
        self.queries: list[QueryArgs] = []
        self.query_result = Completed(0, "1\n")
        self.dumps: dict[str, str] = {}

    def hit(self, name: str) -> None:
        self.calls.append(name)
        if name in self.failures:
            raise ToolingError("Injected " + name)

    def verify_image(self, pin: PostgresPin) -> None:
        self.hit("verify_image")

    def create(self, args: ProofContainerArgs) -> str:
        self.exists = True
        self.hit("create")
        return CONTAINER

    def find_owned(self, name: str) -> str | None:
        self.hit("find_owned")
        return CONTAINER if self.exists else None

    def volumes(self, container: str) -> tuple[str, ...]:
        self.hit("volumes")
        return (VOLUME,)

    def start(self, container: str) -> None:
        self.hit("start")

    def ready(self, container: str, user: str, database: str) -> bool:
        self.hit("ready")
        return self.is_ready

    def port(self, container: str) -> int:
        self.hit("port")
        return 55001

    def query(self, container: str, args: QueryArgs) -> Completed:
        self.hit("query")
        self.queries.append(args)
        return self.query_result

    def dump(self, container: str, args: DumpArgs) -> str:
        self.hit("dump:" + args.database)
        return self.dumps[args.database]

    def create_database(self, container: str, role: str, database: str) -> None:
        self.hit("database:" + database)

    def remove(self, container: str) -> None:
        self.hit("remove")
        self.exists = False

    def require_volumes_removed(self, names: tuple[str, ...]) -> None:
        self.hit("require_volumes_removed")
        if self.exists:
            raise ToolingError("Owned data volume is still present")
