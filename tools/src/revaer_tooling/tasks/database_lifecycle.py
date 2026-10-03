"""Initialize and remove explicitly owned single-init application test databases."""

import hashlib
import re

from ..assignments import literal_assignments
from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.database_lifecycle import LifecycleQuery
from .base import Task

SCRIPTS = (
    "database-e2e-create.sql",
    "database-e2e-roles.sql",
    "database-e2e-seal.sql",
    "database-e2e-drop.sql",
    "database-runtime-fixture-roles.sql",
    "database-runtime-fixture-owner-disable.sql",
)


def lifecycle(context: Context, *, initialize: bool) -> TaskResult:
    name = context.options.database_name
    settings = context.settings.lifecycle
    if not re.fullmatch(r"revaer_test_[0-9]+_[0-9]+", name) or len(name) > 50:
        raise ToolingError("Expected a unique owned test database name")
    if not settings.container or not settings.admin or not settings.password:
        raise ToolingError("Select PG_CONTAINER and explicit test-service admin credentials")
    if initialize and not re.fullmatch(r"[0-9a-f]{64}", settings.runtime_password):
        raise ToolingError("A 64-hex-character temporary runtime password is required")
    sources = tuple(context.root / "scripts/tests" / script for script in SCRIPTS)
    initializer = context.root / "crates/revaer-data/init.sql"
    for source in (*sources, *((initializer,) if initialize else ())):
        if source.is_symlink() or not source.is_file():
            raise ToolingError("Test database SQL must be regular checkout files")
    pins = literal_assignments(
        context.fs.read(context.root / ".github/build-inputs.env"), "build inputs"
    )
    image = pins.get("POSTGRES_REBASELINE_IMAGE")
    if not image:
        raise ToolingError("Test database lifecycle requires the pinned PostgreSQL image")
    docker = context.tools.lifecycle_docker
    container = docker.select(settings.container, image)
    scratch = f"/tmp/{name}-init"
    database_created = roles_created = completed = False

    def admin(script: str, database: str = "postgres", **variables: str) -> None:
        docker.query(
            container,
            LifecycleQuery(
                settings.admin,
                database,
                name,
                f"{scratch}/{script}",
                settings.password,
                settings.runtime_password,
                tuple(variables.items()),
            ),
        )

    # Exclusive staging establishes ownership before any cleanup is permitted.
    docker.stage(container, scratch)
    try:
        for source in sources:
            docker.copy(container, source, f"{scratch}/{source.name}")
        if not initialize:
            admin("database-e2e-drop.sql", verify_owner="true", drop_roles="true")
            completed = True
            return TaskResult("Owned test database removed")
        digest = hashlib.sha256(initializer.read_bytes()).hexdigest()
        docker.copy(container, initializer, f"{scratch}/init.sql")
        if docker.digest(container, f"{scratch}/init.sql") != digest:
            raise ToolingError("Initializer changed while staging the test database")
        admin("database-e2e-create.sql")
        database_created = True
        admin(
            "database-e2e-roles.sql",
            name,
            roles_file=f"{scratch}/database-runtime-fixture-roles.sql",
        )
        roles_created = True
        docker.query(
            container,
            LifecycleQuery(
                f"{name}_owner",
                name,
                name,
                f"{scratch}/database-e2e-seal.sql",
                settings.runtime_password,
                variables=(
                    ("init_file", f"{scratch}/init.sql"),
                    ("init_digest", digest),
                    ("runtime_role", f"{name}_runtime"),
                ),
            ),
        )
        admin("database-runtime-fixture-owner-disable.sql", name)
        completed = True
        return TaskResult("Owned test database initialized and sealed")
    finally:
        try:
            if initialize and database_created and not completed:
                admin(
                    "database-e2e-drop.sql",
                    verify_owner="false",
                    drop_roles="true" if roles_created else "false",
                )
        finally:
            # Attempt staging cleanup even when database cleanup itself fails.
            docker.cleanup(container, scratch)


class DatabaseTestInit(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return lifecycle(context, initialize=True)


class DatabaseTestDrop(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return lifecycle(context, initialize=False)
