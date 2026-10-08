"""Exercise SQLx against one owned PostgreSQL container with disposable data.

The server uses temporary memory-backed storage and an automatically assigned
loopback port. It does not inspect, start, reset, or remove a developer database.
"""

import os
import secrets
import socket
import subprocess
import time
from collections.abc import Iterator
from dataclasses import dataclass, field, replace
from pathlib import Path
from urllib.parse import quote, urlsplit, urlunsplit

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.database import LibpqConnection
from revaer_tooling.process import Completed
from revaer_tooling.tasks.database import (
    DatabaseMigrate,
    DatabaseReset,
    DatabaseSeed,
    DatabaseStart,
)


@dataclass(frozen=True)
class PostgresFixture:
    url: str = field(repr=False)
    environment: dict[str, str] = field(repr=False)
    name: str

    def query(self, statement: str) -> str:
        return subprocess.run(
            ["psql", "--no-psqlrc", "--set=ON_ERROR_STOP=1", "-At", "--command", statement],
            env=self.environment,
            text=True,
            capture_output=True,
            check=True,
            timeout=10,
        ).stdout.strip()


@pytest.fixture
def postgres() -> Iterator[PostgresFixture]:
    environment = {
        **os.environ,
        "POSTGRES_USER": "fixture",
        "POSTGRES_PASSWORD": secrets.token_hex(24),
        "POSTGRES_DB": "fixture",
    }
    # Create returns the exact ID owned by this fixture, even if starting or
    # connecting subsequently fails. Cleanup is registered before either action.
    name = f"rv-db-test-{secrets.token_hex(8)}"
    identifier = subprocess.run(
        [
            "docker",
            "create",
            "--read-only",
            "--name",
            name,
            "--tmpfs",
            "/var/lib/postgresql/data:rw,size=256m",
            "--tmpfs",
            "/var/run/postgresql:rw,size=16m",
            "--tmpfs",
            "/tmp:rw,size=16m",
            "--publish",
            "127.0.0.1::5432",
            "--env",
            "POSTGRES_USER",
            "--env",
            "POSTGRES_PASSWORD",
            "--env",
            "POSTGRES_DB",
            "postgres:16-alpine",
        ],
        env=environment,
        capture_output=True,
        text=True,
        check=True,
        timeout=60,
    ).stdout.strip()
    try:
        subprocess.run(["docker", "start", identifier], capture_output=True, check=True, timeout=20)
        binding = subprocess.run(
            ["docker", "port", identifier, "5432/tcp"],
            capture_output=True,
            text=True,
            check=True,
            timeout=10,
        ).stdout.strip()
        port = int(binding.rsplit(":", 1)[1])
        url = f"postgres://fixture:{environment['POSTGRES_PASSWORD']}@127.0.0.1:{port}/fixture"
        connection = {
            **environment,
            "PGHOST": "127.0.0.1",
            "PGPORT": str(port),
            "PGUSER": "fixture",
            "PGPASSWORD": environment["POSTGRES_PASSWORD"],
            "PGDATABASE": "fixture",
            "PGCONNECT_TIMEOUT": "2",
        }
        deadline = time.monotonic() + 20
        while True:
            result = subprocess.run(
                ["pg_isready", "--quiet"],
                env=connection,
                capture_output=True,
                timeout=5,
            )
            if result.returncode == 0:
                break
            if result.returncode not in (1, 2) or time.monotonic() >= deadline:
                logs = subprocess.run(
                    ["docker", "logs", identifier],
                    capture_output=True,
                    text=True,
                    check=True,
                    timeout=10,
                )
                raise RuntimeError(
                    "Disposable PostgreSQL did not become ready:\n" + logs.stdout + logs.stderr
                )
            time.sleep(0.1)
        yield PostgresFixture(url, connection, name)
    finally:
        subprocess.run(
            ["docker", "rm", "--force", identifier],
            capture_output=True,
            check=True,
            timeout=20,
        )


def test_migrations_replay_and_fail_without_resetting_the_database(
    postgres: PostgresFixture, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("DATABASE_URL", postgres.url)
    # Explicit DATABASE_URL takes precedence over the test fallback and dotenv.
    monkeypatch.setenv("REVAER_TEST_DATABASE_URL", "postgres://unreachable.invalid/postgres")
    (tmp_path / ".env").write_text("DATABASE_URL=postgres://unreachable.invalid/postgres\n")
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    source = tmp_path / "crates/revaer-data/migrations"
    source.mkdir(parents=True)
    initial = source / "1_initial.sql"
    initial.write_text(
        "CREATE TABLE fixture_rows (value integer NOT NULL);\n"
        "INSERT INTO fixture_rows VALUES (42);\n"
    )
    context = make_context(Options())
    assert DatabaseMigrate.run(context).message == "Database migrations applied"
    DatabaseMigrate.run(context)
    assert postgres.query("SELECT value FROM fixture_rows") == "42"
    initial.write_text(initial.read_text() + "-- changed history\n")
    with pytest.raises(ToolingError):
        DatabaseMigrate.run(context)
    assert postgres.query("SELECT value FROM fixture_rows") == "42"
    initial.write_text(initial.read_text().removesuffix("-- changed history\n"))
    (source / "2_invalid.sql").write_text(
        "CREATE TABLE must_rollback (id integer);\nSELECT missing_fixture_function();\n"
    )
    with pytest.raises(ToolingError):
        DatabaseMigrate.run(context)
    assert postgres.query("SELECT to_regclass('must_rollback') IS NULL") == "t"
    assert postgres.query("SELECT count(*) FROM _sqlx_migrations") == "1"


@pytest.fixture
def managed_database(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> Iterator[tuple[Context, PostgresFixture]]:
    password = f"{secrets.token_hex(16)}:p@ss/word"
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    url = f"postgres://fixture:{quote(password, safe='')}@127.0.0.1:{port}/rv_fixture"
    name = f"rv-managed-test-{secrets.token_hex(8)}"
    monkeypatch.setenv("DATABASE_URL", url)
    monkeypatch.setenv("REVAER_DB_MANAGED", "1")
    monkeypatch.setenv("PG_CONTAINER", name)
    # Commas need Docker's documented CSV mount encoding; spaces need literal
    # argv handling. Exercise both with actual storage, not an argument snapshot.
    monkeypatch.setenv("REVAER_DB_DATA_DIR", str(tmp_path / "data, with spaces"))
    monkeypatch.chdir(tmp_path)
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    migrations = tmp_path / "crates/revaer-data/migrations"
    migrations.mkdir(parents=True)
    (migrations / "1_initial.sql").write_text(
        "CREATE TABLE fixture_rows (value integer PRIMARY KEY);\n"
        "INSERT INTO fixture_rows VALUES (42);\n"
    )
    (tmp_path / "scripts").mkdir()
    (tmp_path / "scripts/dev-seed.sql").write_text(
        "INSERT INTO fixture_rows VALUES (43) ON CONFLICT DO NOTHING;\n"
    )
    context = make_context(Options())
    fixture = PostgresFixture(
        url,
        {
            **os.environ,
            "PGHOST": "127.0.0.1",
            "PGPORT": str(port),
            "PGUSER": "fixture",
            "PGPASSWORD": password,
            "PGDATABASE": "rv_fixture",
            "PGCONNECT_TIMEOUT": "2",
        },
        name,
    )
    try:
        yield context, fixture
    finally:
        container = context.tools.docker.database(name)
        if container is not None:
            marker = container.directory / ".rv-owner.json"
            assert container.owner in marker.read_text()
            try:
                # Pytest retains captured output on failure. Inspect only state,
                # never Config.Env (which contains the generated password), and
                # collect logs before the owned container is removed. This makes
                # intermittent startup exits distinguishable from TCP failures.
                for arguments in (
                    ("inspect", "--format", "{{json .State}}", container.identifier),
                    ("logs", "--tail", "100", container.identifier),
                ):
                    result = subprocess.run(
                        ["docker", *arguments],
                        capture_output=True,
                        text=True,
                        check=False,
                        timeout=10,
                    )
                    diagnostic = (
                        (result.stdout + result.stderr)
                        .replace(password, "[redacted-test-password]")
                        .replace(quote(password, safe=""), "[redacted-test-password]")
                    )
                    print(f"Managed PostgreSQL {arguments[0]} exit={result.returncode}:")
                    print(diagnostic)
            finally:
                context.tools.docker.remove_database_container(container.identifier)


def test_managed_database_preserves_data_and_resets_only_when_requested(
    managed_database: tuple[Context, PostgresFixture],
) -> None:
    context, database = managed_database
    DatabaseStart.run(context)
    initial = context.tools.docker.database(database.name)
    assert initial is not None
    assert (initial.directory / "pgdata/PG_VERSION").read_text().strip() == "16"
    DatabaseStart.run(context)
    assert context.tools.docker.database(database.name) == initial
    DatabaseSeed.run(context)
    DatabaseSeed.run(context)
    assert database.query("SELECT sum(value) FROM fixture_rows") == "85"
    seed = context.root / "scripts/dev-seed.sql"
    seed.write_text("INSERT INTO fixture_rows VALUES (99); SELECT missing_seed_function();\n")
    with pytest.raises(ToolingError):
        DatabaseSeed.run(context)
    assert database.query("SELECT sum(value) FROM fixture_rows") == "85"
    migration = context.root / "crates/revaer-data/migrations/1_initial.sql"
    migration.write_text(migration.read_text() + "-- checksum changed\n")
    with pytest.raises(ToolingError):
        DatabaseStart.run(context)
    assert database.query("SELECT sum(value) FROM fixture_rows") == "85"
    DatabaseReset.run(context)
    assert database.query("SELECT sum(value) FROM fixture_rows") == "42"
    # Configuration changes recreate only the proven container, retaining its
    # PostgreSQL data. A removed container can likewise be restored from that data.
    changed = replace(
        context,
        settings=replace(
            context.settings,
            database=replace(
                context.settings.database,
                shared_memory_bytes=2 * 1024**3,
            ),
        ),
    )
    DatabaseStart.run(changed)
    replacement = context.tools.docker.database(database.name)
    assert replacement is not None and replacement.identifier != initial.identifier
    assert database.query("SELECT sum(value) FROM fixture_rows") == "42"
    context.tools.docker.remove_database_container(replacement.identifier)
    DatabaseStart.run(context)
    assert database.query("SELECT sum(value) FROM fixture_rows") == "42"


def test_managed_database_requires_directory_container_and_server_ownership(
    managed_database: tuple[Context, PostgresFixture],
    tmp_path: Path,
) -> None:
    context, database = managed_database
    DatabaseStart.run(context)
    other = replace(context, root=tmp_path / "another-checkout")
    with pytest.raises(ToolingError, match="directory belongs to another checkout"):
        DatabaseStart.run(other)
    other = replace(
        other,
        settings=replace(
            other.settings,
            database=replace(
                other.settings.database,
                data_directory=tmp_path / "other-data",
            ),
        ),
    )
    with pytest.raises(ToolingError, match="container belongs to another checkout"):
        DatabaseStart.run(other)
    database.query("ALTER ROLE fixture SET revaer.rv_owner TO 'another-checkout'")
    with pytest.raises(ToolingError, match="does not identify this checkout"):
        DatabaseStart.run(context)
    database.query("ALTER ROLE fixture RESET revaer.rv_owner")
    assert database.query("SELECT sum(value) FROM fixture_rows") == "42"


def test_caller_database_is_used_without_adopting_or_resetting_its_container(
    postgres: PostgresFixture,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("DATABASE_URL", postgres.url)
    monkeypatch.setenv("PG_CONTAINER", postgres.name)
    monkeypatch.delenv("REVAER_DB_MANAGED", raising=False)
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    migrations = tmp_path / "crates/revaer-data/migrations"
    migrations.mkdir(parents=True)
    (migrations / "1_initial.sql").write_text(
        "CREATE TABLE caller_rows (value int);\nINSERT INTO caller_rows VALUES (42);\n"
    )
    context = make_context(Options())
    DatabaseStart.run(context)
    with pytest.raises(ToolingError, match="not managed by rv"):
        DatabaseReset.run(context)
    assert postgres.query("SELECT value FROM caller_rows") == "42"


@pytest.mark.parametrize(
    "url",
    [
        "postgres://fixture:password@remote.invalid/example",
        "mysql://fixture:password@localhost/example",
        "postgres://localhost/example",
        "postgres://fixture:password@localhost:0/example",
        "postgres://fixture:password@localhost/example?host=remote.invalid",
    ],
)
def test_managed_database_rejects_ambiguous_or_nonlocal_connections_before_creating_data(
    managed_database: tuple[Context, PostgresFixture],
    url: str,
) -> None:
    context, _ = managed_database
    context = replace(
        context,
        settings=replace(
            context.settings,
            database=replace(
                context.settings.database,
                url=url,
            ),
        ),
    )
    with pytest.raises(ToolingError):
        DatabaseStart.run(context)
    directory = context.settings.database.data_directory
    assert directory is not None and not directory.exists()


def test_database_data_and_locks_cannot_be_adopted_by_accident(
    managed_database: tuple[Context, PostgresFixture],
    tmp_path: Path,
) -> None:
    context, _ = managed_database
    directory = context.settings.database.data_directory
    assert directory is not None
    directory.mkdir()
    existing = directory / "PG_VERSION"
    existing.write_text("16\n")
    with pytest.raises(ToolingError, match="existing unowned database directory"):
        DatabaseStart.run(context)
    assert existing.read_text() == "16\n"
    assert not (directory / ".rv-owner.json").exists()
    existing.unlink()
    with (
        context.fs.lock(directory / ".rv.lock"),
        pytest.raises(ToolingError, match="Another operation holds"),
    ):
        DatabaseStart.run(context)
    # A released lock can be taken again. An invalid marker never becomes a new
    # claim, and a symlink never moves ownership checks to an unrelated directory.
    with context.fs.lock(directory / ".rv.lock"):
        pass
    marker = directory / ".rv-owner.json"
    marker.write_text("invalid JSON")
    with pytest.raises(ToolingError, match="ownership record is invalid"):
        DatabaseStart.run(context)
    alternate = tmp_path / "alternate"
    alternate.symlink_to(directory, target_is_directory=True)
    linked = replace(
        context,
        settings=replace(
            context.settings,
            database=replace(
                context.settings.database,
                data_directory=alternate,
            ),
        ),
    )
    with pytest.raises(ToolingError, match="must not be a symlink"):
        DatabaseStart.run(linked)


def test_reset_rejects_caller_owned_and_administrative_databases(
    managed_database: tuple[Context, PostgresFixture],
) -> None:
    context, _ = managed_database
    caller = replace(
        context,
        settings=replace(
            context.settings,
            database=replace(
                context.settings.database,
                managed=False,
                reset=True,
            ),
        ),
    )
    with pytest.raises(ToolingError, match="caller-owned database"):
        DatabaseStart.run(caller)
    admin_url = urlunsplit(urlsplit(context.settings.database.url)._replace(path="/postgres"))
    admin = replace(
        context,
        settings=replace(
            context.settings,
            database=replace(
                context.settings.database,
                url=admin_url,
            ),
        ),
    )
    with pytest.raises(ToolingError, match="administrative database"):
        DatabaseReset.run(admin)


def test_libpq_receives_query_passwords_without_exposing_them_in_arguments(
    postgres: PostgresFixture,
) -> None:
    context = make_context(Options())
    password = postgres.environment["PGPASSWORD"]
    url = postgres.url + f"?password={quote(password, safe='')}&application_name=rv-test"
    connection = LibpqConnection.from_url(url)
    assert password not in connection.uri
    assert "password=" not in connection.uri
    assert connection.environment() == {"PGPASSWORD": password}
    assert "application_name=rv-test" in connection.uri
    # A caller-owned PostgreSQL has no rv identity, but the authenticated query
    # must still succeed through libpq's actual URI/environment interfaces.
    assert context.tools.psql.server_owner(url) is None
    with pytest.raises(ToolingError, match="connection URI"):
        LibpqConnection.from_url("postgres://localhost/example#fragment")


def test_postgres_readiness_has_a_bounded_failure() -> None:
    context = make_context(Options())
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        port = listener.getsockname()[1]
        with pytest.raises(ToolingError, match="before the timeout"):
            context.tools.pg_isready.wait(
                (f"postgres://fixture@127.0.0.1:{port}/fixture",), timeout=0.01
            )
    with pytest.raises(ToolingError, match="at least one endpoint"):
        context.tools.pg_isready.wait(())


@pytest.mark.parametrize("code", (124, 1))
def test_postgres_readiness_retries_only_timed_out_candidates(
    monkeypatch: pytest.MonkeyPatch, code: int
) -> None:
    context = make_context(Options())
    calls = []

    def probe(*args: object, **kwargs: object) -> Completed:
        calls.append(args)
        if len(calls) == 1:
            raise CommandError("probe failed", code)
        return Completed(0, "")

    monkeypatch.setattr(context.tools.pg_isready, "_invoke", probe)
    endpoints = ("postgres://fixture@first.invalid/database", "postgres://fixture@localhost/db")
    if code == 124:
        assert context.tools.pg_isready.wait(endpoints) == endpoints[1]
        assert len(calls) == 2
    else:
        with pytest.raises(CommandError, match="probe failed"):
            context.tools.pg_isready.wait(endpoints)
        assert len(calls) == 1
