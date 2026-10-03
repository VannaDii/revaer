"""SQLx owns migration execution and validates its stored migration history."""

import time
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import parse_qsl, quote, unquote, urlencode, urlsplit, urlunsplit

from ..errors import ToolingError
from ..process import Completed
from .base import ExternalTool


def with_database(url: str, name: str) -> str:
    """Select a database on the same server without losing libpq URI options.

    A second database name in the query could override the path in a client.
    Reject that ambiguity before a caller creates, migrates, or drops anything.
    """
    parsed = urlsplit(url)
    if parsed.scheme not in ("postgres", "postgresql") or parsed.fragment:
        raise ToolingError("Database selection requires a PostgreSQL URI")
    if any(key == "dbname" for key, _ in parse_qsl(parsed.query)):
        raise ToolingError("Database URLs must identify the database in their path")
    if not name or "\x00" in name:
        raise ToolingError("Database name must be nonempty and contain no NUL")
    return urlunsplit(parsed._replace(path="/" + quote(name, safe="")))


class Sqlx(ExternalTool):
    def proof_migration(self, database_url: str, source: Path) -> Completed:
        """Return a captured attempt so disposable proofs can classify startup retries."""
        return self._invoke(
            ("migrate", "run", "--no-dotenv", "--source", str(source)),
            env={"DATABASE_URL": database_url},
            capture=True,
            accepted_codes=(0, 1),
        )

    def drop_database(self, database_url: str) -> Completed:
        """Used only with a database created by the calling resource lifecycle."""
        return self._invoke(
            ("database", "drop", "-y", "--no-dotenv"), env={"DATABASE_URL": database_url}
        )

    def create_database(self, database_url: str) -> Completed:
        return self._invoke(
            ("database", "create", "--no-dotenv"), env={"DATABASE_URL": database_url}
        )

    def reset_database(self, database_url: str, source: Path) -> Completed:
        return self._invoke(
            ("database", "reset", "-y", "--no-dotenv", "--source", str(source)),
            env={"DATABASE_URL": database_url},
        )

    def migrate(self, database_url: str, source: Path) -> Completed:
        # The caller selected the database. Do not allow a child tool's dotenv
        # lookup to select another endpoint; keep credentials out of argv.
        return self._invoke(
            ("migrate", "run", "--no-dotenv", "--source", str(source)),
            env={"DATABASE_URL": database_url},
        )


@dataclass(frozen=True)
class LibpqConnection:
    """A libpq URI with its password moved into the supported environment input."""

    uri: str
    password: str | None = field(repr=False)

    @staticmethod
    def from_url(database_url: str) -> "LibpqConnection":
        parsed = urlsplit(database_url)
        if parsed.scheme not in ("postgres", "postgresql") or parsed.fragment:
            raise ToolingError("Expected a PostgreSQL connection URI without a fragment")
        password = unquote(parsed.password) if parsed.password is not None else None
        userinfo, separator, host = parsed.netloc.rpartition("@")
        netloc = f"{userinfo.split(':', 1)[0]}@{host}" if separator else parsed.netloc
        query = []
        for key, value in parse_qsl(parsed.query, keep_blank_values=True):
            if key == "password":
                password = value
            else:
                query.append((key, value))
        uri = urlunsplit((parsed.scheme, netloc, parsed.path, urlencode(query), ""))
        return LibpqConnection(uri, password)

    def environment(self) -> dict[str, str]:
        return {"PGPASSWORD": self.password} if self.password is not None else {}


class PgIsReady(ExternalTool):
    def wait(self, database_urls: tuple[str, ...], timeout: float = 60) -> str:
        if not database_urls:
            raise ToolingError("PostgreSQL readiness needs at least one endpoint")
        connections = tuple((url, LibpqConnection.from_url(url)) for url in database_urls)
        deadline = time.monotonic() + timeout
        while True:
            for url, connection in connections:
                result = self._invoke(
                    ("--quiet", "--timeout=1", "--dbname", connection.uri),
                    env=connection.environment(),
                    capture=True,
                    timeout=3,
                    accepted_codes=(0, 1, 2),
                )
                if result.code == 0:
                    return url
            if time.monotonic() >= deadline:
                raise ToolingError("PostgreSQL did not become ready before the timeout")
            time.sleep(min(0.2, max(0, deadline - time.monotonic())))


class Psql(ExternalTool):
    def server_owner(self, database_url: str) -> str | None:
        connection = LibpqConnection.from_url(database_url)
        value = self._invoke(
            (
                "--no-psqlrc",
                "--no-password",
                "--set=ON_ERROR_STOP=1",
                "--tuples-only",
                "--no-align",
                "--dbname",
                connection.uri,
                "--command",
                "SELECT current_setting('revaer.rv_owner', true)",
            ),
            env=connection.environment(),
            capture=True,
            timeout=10,
        ).stdout.strip()
        return value or None

    def seed(self, database_url: str, source: Path) -> Completed:
        connection = LibpqConnection.from_url(database_url)
        return self._invoke(
            (
                "--no-psqlrc",
                "--no-password",
                "--set=ON_ERROR_STOP=1",
                "--single-transaction",
                "--dbname",
                connection.uri,
                "--file",
                str(source),
            ),
            env=connection.environment(),
        )
