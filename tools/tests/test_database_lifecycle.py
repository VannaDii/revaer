"""Run the migrated single-init lifecycle against an owned native PostgreSQL server."""

import hashlib
import secrets
import shutil
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import COMMANDS, make_context, parser
from revaer_tooling.context import Context, Options
from revaer_tooling.database.lifecycle_settings import LifecycleSettings
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks.database_lifecycle import SCRIPTS, DatabaseTestDrop, DatabaseTestInit
from test_database import PostgresFixture
from test_database import postgres as postgres

# A small real initializer isolates lifecycle ordering, transactional rollback,
# digest binding and roles. Full media-init acceptance is a separate integration.
INITIALIZER = """
CREATE SCHEMA revaer_system;
CREATE TABLE revaer_system.seal(digest bytea NOT NULL);
CREATE FUNCTION revaer_system.seal_database_baseline_v1(smallint, bytea, text)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO revaer_system.seal VALUES ($2);
    EXECUTE format('GRANT USAGE ON SCHEMA revaer_system TO %I', $3);
    EXECUTE format('GRANT SELECT ON revaer_system.seal TO %I', $3);
END;
$$;
"""


@pytest.fixture
def lifecycle_context(tmp_path: Path) -> Context:
    context = make_context(Options(database_name=f"revaer_test_1_{secrets.randbelow(10**12)}"))
    for script in SCRIPTS:
        target = tmp_path / "scripts/tests" / script
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(context.root / "scripts/tests" / script, target)
    initializer = tmp_path / "crates/revaer-data/init.sql"
    initializer.parent.mkdir(parents=True)
    initializer.write_text(INITIALIZER)
    manifest = tmp_path / ".github/build-inputs.env"
    manifest.parent.mkdir()
    manifest.write_text("POSTGRES_REBASELINE_IMAGE=postgres:16-alpine\n")
    settings = LifecycleSettings(
        "not-selected", "fixture", secrets.token_hex(24), secrets.token_hex(32)
    )
    return replace(context, root=tmp_path, settings=replace(context.settings, lifecycle=settings))


def selected(context: Context, server: PostgresFixture) -> Context:
    settings = replace(
        context.settings.lifecycle, container=server.name, password=server.environment["PGPASSWORD"]
    )
    return replace(context, settings=replace(context.settings, lifecycle=settings))


def test_native_init_seals_digest_disables_owner_and_drop_removes_roles(
    lifecycle_context: Context,
    postgres: PostgresFixture,
) -> None:
    context = selected(lifecycle_context, postgres)
    name = context.options.database_name
    DatabaseTestInit.run(context)
    try:
        assert (
            postgres.query(f"SELECT rolcanlogin FROM pg_roles WHERE rolname='{name}_owner'") == "f"
        )
        runtime = replace(
            postgres,
            environment={
                **postgres.environment,
                "PGDATABASE": name,
                "PGUSER": name + "_runtime",
                "PGPASSWORD": context.settings.lifecycle.runtime_password,
            },
        )

        expected = hashlib.sha256(INITIALIZER.encode()).hexdigest()
        assert runtime.query("SELECT encode(digest, 'hex') FROM revaer_system.seal") == expected
        assert (
            runtime.query(
                "SELECT rolsuper OR rolcreatedb OR rolcreaterole OR rolbypassrls "
                "FROM pg_roles WHERE rolname=current_user"
            )
            == "f"
        )
    finally:
        DatabaseTestDrop.run(context)
    assert postgres.query(f"SELECT count(*) FROM pg_database WHERE datname='{name}'") == "0"
    assert (
        postgres.query(
            f"SELECT count(*) FROM pg_roles WHERE rolname IN ('{name}_owner','{name}_runtime')"
        )
        == "0"
    )


def test_failed_init_removes_only_created_database_and_roles(
    lifecycle_context: Context,
    postgres: PostgresFixture,
) -> None:
    context = selected(lifecycle_context, postgres)
    (context.root / "crates/revaer-data/init.sql").write_text(
        INITIALIZER + "SELECT missing_function();"
    )
    with pytest.raises(ToolingError):
        DatabaseTestInit.run(context)
    name = context.options.database_name
    assert postgres.query(f"SELECT count(*) FROM pg_database WHERE datname='{name}'") == "0"
    assert (
        postgres.query(
            f"SELECT count(*) FROM pg_roles WHERE rolname IN ('{name}_owner','{name}_runtime')"
        )
        == "0"
    )
    # The same name can be retried only because both SQL staging and roles were cleaned.
    (context.root / "crates/revaer-data/init.sql").write_text(INITIALIZER)
    DatabaseTestInit.run(context)
    DatabaseTestDrop.run(context)


def test_existing_unowned_database_survives_init_and_drop(
    lifecycle_context: Context,
    postgres: PostgresFixture,
) -> None:
    context = selected(lifecycle_context, postgres)
    name = context.options.database_name
    postgres.query(f'CREATE DATABASE "{name}"')
    try:
        for task in (DatabaseTestInit, DatabaseTestDrop):
            with pytest.raises(ToolingError):
                task.run(context)
            assert postgres.query(f"SELECT count(*) FROM pg_database WHERE datname='{name}'") == "1"
    finally:
        postgres.query(f'DROP DATABASE "{name}"')


@pytest.mark.parametrize("name", ("postgres", "revaer_test_1_x", "revaer_test_1_" + "2" * 50))
def test_invalid_names_fail_before_docker(lifecycle_context: Context, name: str) -> None:
    context = replace(
        lifecycle_context, options=replace(lifecycle_context.options, database_name=name)
    )
    with pytest.raises(ToolingError, match="unique owned"):
        DatabaseTestInit.run(context)


def test_credentials_and_runtime_password_are_required(lifecycle_context: Context) -> None:
    for field in ("container", "admin", "password", "runtime_password"):
        settings = replace(lifecycle_context.settings.lifecycle, **{field: ""})
        context = replace(
            lifecycle_context, settings=replace(lifecycle_context.settings, lifecycle=settings)
        )
        with pytest.raises(ToolingError, match=r"credentials|runtime password"):
            DatabaseTestInit.run(context)


def test_image_mismatch_prevents_database_creation(
    lifecycle_context: Context,
    postgres: PostgresFixture,
) -> None:
    context = selected(lifecycle_context, postgres)
    (context.root / ".github/build-inputs.env").write_text(
        "POSTGRES_REBASELINE_IMAGE=wrong-image\n"
    )
    with pytest.raises(ToolingError, match="pinned PostgreSQL"):
        DatabaseTestInit.run(context)
    assert (
        postgres.query(
            f"SELECT count(*) FROM pg_database WHERE datname='{context.options.database_name}'"
        )
        == "0"
    )


def test_lifecycle_commands_have_static_dispatch_and_required_name() -> None:
    for name, task in (("db-test-init", DatabaseTestInit), ("db-test-drop", DatabaseTestDrop)):
        assert COMMANDS[name] == task.run
        assert parser().parse_args([name, "revaer_test_1_2"]).database_name == "revaer_test_1_2"
        with pytest.raises(SystemExit):
            parser().parse_args([name])


def test_existing_role_survives_failed_role_transaction(
    lifecycle_context: Context,
    postgres: PostgresFixture,
) -> None:
    context = selected(lifecycle_context, postgres)
    name = context.options.database_name
    postgres.query(f'CREATE ROLE "{name}_runtime"')
    try:
        with pytest.raises(ToolingError):
            DatabaseTestInit.run(context)
        assert postgres.query(f"SELECT count(*) FROM pg_database WHERE datname='{name}'") == "0"
        assert postgres.query(f"SELECT count(*) FROM pg_roles WHERE rolname='{name}_owner'") == "0"
        assert (
            postgres.query(f"SELECT count(*) FROM pg_roles WHERE rolname='{name}_runtime'") == "1"
        )
    finally:
        postgres.query(f'DROP ROLE "{name}_runtime"')


def test_existing_staging_directory_is_never_removed(
    lifecycle_context: Context,
    postgres: PostgresFixture,
) -> None:
    context = selected(lifecycle_context, postgres)
    docker = context.tools.lifecycle_docker
    container = docker.select(postgres.name, "postgres:16-alpine")
    directory = f"/tmp/{context.options.database_name}-init"
    docker.stage(container, directory)
    try:
        with pytest.raises(ToolingError):
            DatabaseTestInit.run(context)
    finally:
        # Native rm (without -f) proves that rejected initialization left it intact.
        docker.cleanup(container, directory)
