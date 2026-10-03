"""Select the media single-init lifecycle without adopting a caller database."""

import secrets
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import replace
from urllib.parse import urlsplit, urlunsplit

from ..assignments import literal_assignments
from ..context import Context
from ..database.contract import Phase, PostgresPin
from ..errors import ToolingError
from ..tasks.database_lifecycle import DatabaseTestDrop, DatabaseTestInit


def uses_single_init(context: Context) -> bool:
    source = context.root / "config/database-rebaseline.env"
    if source.is_symlink():
        raise ToolingError("Database phase configuration must be a regular checkout file")
    if not source.exists():
        return False
    values = literal_assignments(context.fs.read(source), "database phase")
    try:
        phase = Phase(values.get("TRANSITION_PHASE", ""))
    except ValueError as error:
        raise ToolingError(
            "Database phase configuration has no recognized transition phase"
        ) from error
    return phase == Phase.FEATURE_DEVELOPMENT


def verify_endpoint(admin_url: str, binding: str) -> None:
    parsed = urlsplit(admin_url)
    if (
        parsed.scheme not in ("postgres", "postgresql")
        or parsed.hostname not in ("localhost", "127.0.0.1", "host.docker.internal")
        or parsed.query
        or parsed.fragment
    ):
        raise ToolingError(
            "Test database requires an explicit local PostgreSQL URL without overrides"
        )
    if binding.strip() != f"127.0.0.1:{parsed.port or 5432}":
        raise ToolingError("Test database URL does not match the selected container loopback port")


@contextmanager
def single_init_database(context: Context) -> Iterator[str]:
    settings = context.settings.lifecycle
    admin_url = context.settings.e2e.admin_url
    if not admin_url or not settings.container:
        raise ToolingError("Media E2E requires an explicit test database URL and PG_CONTAINER")
    pins = PostgresPin.load(
        literal_assignments(
            context.fs.read(context.root / ".github/build-inputs.env"), "build inputs"
        )
    )
    docker = context.tools.lifecycle_docker
    # Resolve a name once, then bind every operation to that immutable ID.
    container = docker.select(settings.container, pins.image)
    verify_endpoint(admin_url, docker.binding(container))
    context.tools.pg_isready.wait((admin_url,))
    name = f"revaer_test_{int(context.invoked_at.timestamp() * 1000)}_{secrets.randbits(32)}"
    password = secrets.token_hex(32)
    selected = replace(
        context,
        options=replace(context.options, database_name=name),
        settings=replace(
            context.settings,
            lifecycle=replace(
                settings,
                container=container,
                runtime_password=password,
            ),
        ),
    )
    parsed = urlsplit(admin_url)
    # Endpoint verification forbids query overrides; only the restricted role
    # and newly owned database are handed to the application service.
    host = parsed.hostname
    runtime_url = urlunsplit(
        (
            parsed.scheme,
            f"{name}_runtime:{password}@{host}:{parsed.port or 5432}",
            "/" + name,
            "",
            "",
        )
    )
    DatabaseTestInit.run(selected)
    try:
        yield runtime_url
    finally:
        DatabaseTestDrop.run(selected)
