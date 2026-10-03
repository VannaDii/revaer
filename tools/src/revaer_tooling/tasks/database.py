"""Local database ownership, readiness, migration, reset, and seed coordination.

A supplied connection is caller-owned unless managed mode is selected. Docker
names locate candidates; labels, directory ownership, and a server-side identity
must agree before a managed operation can write or reset application data.
"""

import hashlib
import json
from collections.abc import Iterator
from contextlib import contextmanager, nullcontext
from dataclasses import replace
from pathlib import Path
from urllib.parse import parse_qsl, unquote, urlsplit, urlunsplit

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.docker import PostgresContainerArgs
from .base import Task

OWNER_FILE = ".rv-owner.json"


def verify_directory(context: Context, directory: Path, owner: str) -> None:
    marker = directory / OWNER_FILE
    if directory.is_symlink() or marker.is_symlink():
        raise ToolingError("Managed database ownership paths must not be symlinks")
    if not marker.is_file():
        raise ToolingError(f"No rv ownership record for database directory {directory}")
    try:
        record = json.loads(context.fs.read(marker))
    except ValueError as error:
        raise ToolingError("Database directory ownership record is invalid") from error
    if record != {"owner": owner, "checkout": str(context.root)}:
        raise ToolingError("Database directory belongs to another checkout")


def managed_options(context: Context, owner: str) -> PostgresContainerArgs:
    settings = context.settings.database
    parsed = urlsplit(settings.url)
    if parsed.scheme not in ("postgres", "postgresql") or parsed.fragment:
        raise ToolingError("Managed databases require a PostgreSQL connection URI")
    if parsed.hostname not in ("localhost", "127.0.0.1", "::1", "host.docker.internal"):
        raise ToolingError("Managed databases require a local Docker host endpoint")
    overrides = {"host", "hostaddr", "port", "user", "password", "dbname", "service", "servicefile"}
    if any(key in overrides for key, _ in parse_qsl(parsed.query, keep_blank_values=True)):
        raise ToolingError("Managed database identity must be in the URI authority and path")
    if parsed.username is None or parsed.password is None or not parsed.path.strip("/"):
        raise ToolingError("Managed database URI must specify its user, password, and database")
    port = parsed.port if parsed.port is not None else 5432
    if not 1 <= port <= 65535:
        raise ToolingError("Managed database port must be between 1 and 65535")
    directory = settings.data_directory or context.root / ".server_root/postgres-data"
    if not directory.is_absolute():
        directory = context.root / directory
    if directory.is_symlink():
        raise ToolingError("Managed database directory must not be a symlink")
    directory = directory.resolve()
    return PostgresContainerArgs(
        settings.container_name or f"revaer-db-{owner[:12]}",
        owner,
        directory,
        port,
        settings.shared_memory_bytes,
        unquote(parsed.username),
        unquote(parsed.password),
        context.host.uid,
        context.host.gid,
    )


def prepare_container(context: Context, args: PostgresContainerArgs) -> None:
    marker = args.directory / OWNER_FILE
    if marker.exists() or marker.is_symlink():
        verify_directory(context, args.directory, args.owner)
    else:
        # The lock is the only entry permitted before an initial ownership claim.
        # Existing PostgreSQL data is never adopted by guessing from a path/name.
        if any(path.name != ".rv.lock" for path in args.directory.iterdir()):
            raise ToolingError("Refusing to adopt an existing unowned database directory")
        context.fs.write(
            marker,
            json.dumps({"owner": args.owner, "checkout": str(context.root)}) + "\n",
            mode=0o600,
        )
    container = context.tools.docker.database(args.name)
    if container is not None:
        if container.owner != args.owner:
            raise ToolingError("Database container belongs to another checkout")
        verify_directory(context, container.directory, args.owner)
        if (
            container.directory != args.directory
            or container.port != args.port
            or container.shared_memory_bytes < args.shared_memory_bytes
        ):
            context.emit("Recreating the owned database container with its requested configuration")
            context.tools.docker.remove_database_container(container.identifier)
            container = None
    if container is None:
        identifier = context.tools.docker.create_database(args)
        context.tools.docker.start_database(identifier)
    elif not container.running:
        context.tools.docker.start_database(container.identifier)


def managed_connection(context: Context, args: PostgresContainerArgs) -> str:
    parsed = urlsplit(context.settings.database.url)
    userinfo = parsed.netloc.rpartition("@")[0]
    hosts = (
        ("host.docker.internal", "127.0.0.1")
        if parsed.hostname == "host.docker.internal"
        else ("127.0.0.1", "host.docker.internal")
    )
    candidates = tuple(
        urlunsplit((parsed.scheme, f"{userinfo}@{host}:{args.port}", "/postgres", parsed.query, ""))
        for host in hosts
    )
    selected = context.tools.pg_isready.wait(candidates)
    # A ready TCP/PG endpoint alone cannot prove ownership, especially when rv
    # runs inside a container and localhost refers to a different network.
    if context.tools.psql.server_owner(selected) != args.owner:
        raise ToolingError("PostgreSQL endpoint does not identify this checkout's managed server")
    return urlunsplit(urlsplit(selected)._replace(path=parsed.path))


@contextmanager
def database_connection(context: Context) -> Iterator[str]:
    """Keep server ownership locked while a composed operation uses its URL."""
    settings = context.settings.database
    owner = hashlib.sha256(str(context.root).encode()).hexdigest()
    options = managed_options(context, owner) if settings.managed else None
    if options is not None:
        context.fs.mkdir(options.directory)
    lock = context.fs.lock(options.directory / ".rv.lock") if options else nullcontext()
    with lock:
        if options is not None:
            prepare_container(context, options)
            url = managed_connection(context, options)
        else:
            url = context.tools.pg_isready.wait((settings.url,))
        yield url


@contextmanager
def prepared_database(context: Context, *, seed: bool = False) -> Iterator[str]:
    """Hold the ownership lock through migration and the caller's service lifetime."""
    settings = context.settings.database
    if settings.reset and not settings.managed:
        raise ToolingError("Refusing to reset a caller-owned database")
    if settings.reset and unquote(urlsplit(settings.url).path).strip("/") in (
        "postgres",
        "template0",
        "template1",
    ):
        raise ToolingError("Refusing to reset a PostgreSQL administrative database")
    with database_connection(context) as url:
        source = context.root / "crates/revaer-data/migrations"
        context.tools.sqlx.create_database(url)
        if settings.reset:
            context.tools.sqlx.reset_database(url, source)
        else:
            # A migration failure is evidence to inspect. Only the explicit
            # reset operation may drop data; a retry never silently resets it.
            context.tools.sqlx.migrate(url, source)
        if seed:
            context.tools.psql.seed(url, context.root / "scripts/dev-seed.sql")
        yield url


def start_database(context: Context, *, seed: bool = False) -> TaskResult:
    with prepared_database(context, seed=seed):
        return TaskResult("Database ready" + (" and seeded" if seed else ""))


class DatabaseStart(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return start_database(context)


class DatabaseReset(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        settings = replace(context.settings.database, managed=True, reset=True)
        return start_database(
            replace(context, settings=replace(context.settings, database=settings))
        )


class DatabaseSeed(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return start_database(context, seed=True)


class DatabaseMigrate(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.sqlx.migrate(
            context.settings.database.url,
            context.root / "crates/revaer-data/migrations",
        )
        return TaskResult("Database migrations applied")
