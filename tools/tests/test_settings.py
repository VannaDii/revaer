"""Configuration tests protect defaults and the existing override precedence."""

import secrets
from pathlib import Path

import pytest
from revaer_tooling.errors import ToolingError
from revaer_tooling.settings import load_settings


def test_release_defaults_preserve_optional_helm_assets() -> None:
    settings = load_settings({})
    assert settings.coverage.build_jobs == "default"
    assert not settings.helm_release_assets
    assert settings.chart.sign
    assert settings.chart.repository == "VannaDii/Revaer"
    assert settings.chart.namespace == "vannadii/charts"
    assert settings.diff_base is None
    assert settings.diff_head == "HEAD"


def test_registry_overrides_take_precedence_over_github_credentials() -> None:
    settings = load_settings(
        {
            "GITHUB_REPOSITORY": "Example/Revaer",
            "GITHUB_ACTOR": "test-actor",
            "GITHUB_TOKEN": "job-token",
            "HELM_API_KEY_ID": "registry-user",
            "HELM_API_KEY_SECRET": "registry-secret",
            "HELM_REGISTRY_NAMESPACE": "custom/charts",
            "HELM_GPG_PRIVATE": "private-test-key",
        }
    )
    assert settings.chart.registry.username == "registry-user"
    assert settings.chart.registry.password == "registry-secret"
    assert settings.chart.namespace == "custom/charts"
    assert settings.chart.image_repository == "ghcr.io/example/revaer"
    assert "registry-secret" not in repr(settings)
    assert "private-test-key" not in repr(settings)


def test_empty_values_use_existing_shell_default_semantics() -> None:
    settings = load_settings(
        {
            "REVAER_RELEASE_REPOSITORY": "",
            "GITHUB_REPOSITORY": "Example/Revaer",
            "GITHUB_ACTOR": "test-actor",
            "GITHUB_TOKEN": "job-token",
            "HELM_REGISTRY_HOST": "",
            "HELM_REGISTRY_PASSWORD": "",
            "REVAER_ENABLE_HELM_RELEASE_ASSETS": "",
        }
    )
    assert settings.chart.repository == "Example/Revaer"
    assert settings.chart.registry.host == "ghcr.io"
    assert settings.chart.registry.password == "job-token"
    assert not settings.helm_release_assets


def test_unknown_switch_value_is_not_silently_interpreted() -> None:
    with pytest.raises(ToolingError, match="REVAER_HELM_SIGN"):
        load_settings({"REVAER_HELM_SIGN": "treu"})


def test_database_url_precedence_and_local_credentials_are_preserved() -> None:
    from urllib.parse import unquote, urlsplit

    password = secrets.token_hex(12) + " :/@?#$\u2603 "
    settings = load_settings(
        {
            "REVAER_LOCAL_DB_USER": "local user",
            "REVAER_LOCAL_DB_PASSWORD": password,
            "REVAER_LOCAL_DB_HOST": "::1",
            "REVAER_LOCAL_DB_PORT": "55432",
        }
    )
    url = urlsplit(settings.database.require_url())
    assert url.hostname == "::1"
    assert url.port == 55432
    assert unquote(url.username or "") == "local user"
    assert unquote(url.password or "") == password
    assert url.path == "/revaer"
    assert settings.database.test_url is None
    assert settings.database.managed
    assert password not in repr(settings)
    explicit = load_settings(
        {
            "DATABASE_URL": "postgres://application.invalid/selected",
            "REVAER_TEST_DATABASE_URL": "postgres://tests.invalid/admin",
        }
    )
    assert explicit.database.url == "postgres://application.invalid/selected"
    assert explicit.database.test_url == "postgres://tests.invalid/admin"
    assert not explicit.database.managed
    application_only = load_settings({"DATABASE_URL": "postgres://tests.invalid/selected"})
    assert application_only.database.url == application_only.database.test_url


@pytest.mark.parametrize("port", ["zero", "0", "65536"])
def test_invalid_local_database_port_is_actionable(port: str) -> None:
    with pytest.raises(ToolingError, match="REVAER_LOCAL_DB_PORT"):
        load_settings({"REVAER_LOCAL_DB_PORT": port})


@pytest.mark.parametrize("jobs", ["1", "-2", "default"])
def test_coverage_preserves_cargos_supported_parallelism_options(jobs: str) -> None:
    settings = load_settings({"CARGO_BUILD_JOBS": jobs, "RUST_TEST_THREADS": "3"})
    assert str(settings.coverage.build_jobs) == jobs
    assert settings.coverage.test_threads == 3


@pytest.mark.parametrize(
    "name,value",
    [
        ("RUST_TEST_THREADS", "0"),
        ("RUST_TEST_THREADS", "zero"),
        ("CARGO_BUILD_JOBS", "0"),
        ("CARGO_BUILD_JOBS", "lots"),
    ],
)
def test_invalid_coverage_parallelism_is_reported(name: str, value: str) -> None:
    with pytest.raises(ToolingError, match=name):
        load_settings({name: value})


@pytest.mark.parametrize("value", ["true", "1", "false", "0", "auto"])
def test_database_lifecycle_mode_is_explicit(value: str) -> None:
    settings = load_settings({"REVAER_DB_MANAGED": value, "DATABASE_URL": "postgres://example/db"})
    assert settings.database.managed == (value in ("1", "true"))


@pytest.mark.parametrize(
    "environment",
    [
        {"REVAER_DB_MANAGED": "sometimes"},
        {"REVAER_DB_SHM_SIZE": "lots"},
        {"REVAER_DB_SHM_SIZE": "1m"},
    ],
)
def test_database_lifecycle_rejects_invalid_configuration(environment: dict[str, str]) -> None:
    with pytest.raises(ToolingError, match="REVAER_DB_"):
        load_settings(environment)


def test_database_shared_memory_accepts_docker_size_units_and_a_byte_minimum() -> None:
    for value in ("1g", "1024m", "1048576k", "1073741824"):
        assert load_settings({"REVAER_DB_SHM_SIZE": value}).database.shared_memory_bytes == 1024**3
    assert (
        load_settings(
            {
                "REVAER_DB_SHM_SIZE": "32m",
                "REVAER_DB_SHM_BYTES": "33554432",
            }
        ).database.shared_memory_bytes
        == 32 * 1024**2
    )


@pytest.mark.parametrize("value", (None, ""))
def test_missing_local_password_is_absent_until_a_database_is_required(value: str | None) -> None:
    environment = {} if value is None else {"REVAER_LOCAL_DB_PASSWORD": value}
    settings = load_settings(environment)
    assert settings.database.url is None
    assert settings.e2e.admin_url is None
    with pytest.raises(
        ToolingError, match=r"Copy \.env.template to \.env and set REVAER_LOCAL_DB_PASSWORD"
    ):
        settings.database.require_url()


def test_local_admin_connection_uses_the_single_supplied_password() -> None:
    from urllib.parse import unquote, urlsplit

    password = secrets.token_urlsafe(24)
    environment = {"REVAER_LOCAL_DB_PASSWORD": password}
    settings = load_settings(environment)
    from revaer_tooling.external.database import with_database

    assert settings.e2e.admin_url is None
    parsed = urlsplit(with_database(settings.database.require_url(), "postgres"))
    assert unquote(parsed.password or "") == password
    assert parsed.path == "/postgres"
    assert settings.database.test_url is None
    selected = "postgres://external.invalid/admin"
    explicit = load_settings({**environment, "E2E_DB_ADMIN_URL": selected})
    assert explicit.e2e.admin_url == selected


def test_database_start_requires_configuration_before_provisioning(
    tmp_path: Path,
) -> None:
    from revaer_tooling.cli import make_context
    from revaer_tooling.context import Context, Options
    from revaer_tooling.tasks.database import DatabaseStart

    context = make_context(Options())
    settings = load_settings({"REVAER_DB_DATA_DIR": str(tmp_path / "database")})
    context = Context(
        root=context.root,
        settings=settings,
        options=context.options,
        tools=context.tools,
        fs=context.fs,
        emit=context.emit,
        host=context.host,
        invoked_at=context.invoked_at,
        command_names=context.command_names,
    )
    with pytest.raises(ToolingError, match="REVAER_LOCAL_DB_PASSWORD"):
        DatabaseStart.run(context)
    assert not (tmp_path / "database").exists()


@pytest.mark.parametrize("configured", (False, True))
def test_local_postgres_url_script_encodes_or_reports_missing_credentials(configured: bool) -> None:
    import os
    import subprocess
    from pathlib import Path
    from urllib.parse import unquote, urlsplit

    password = secrets.token_hex(16) + " :/@?#$ " if configured else ""
    root = Path(__file__).parents[2]
    result = subprocess.run(
        [str(root / "scripts/local-postgres-url.sh"), "selected"],
        cwd=root,
        env={**os.environ, "REVAER_LOCAL_DB_PASSWORD": password, "REVAER_LOCAL_DB_HOST": "::1"},
        text=True,
        capture_output=True,
        timeout=30,
        check=False,
    )
    if configured:
        assert result.returncode == 0
        assert unquote(urlsplit(result.stdout).password or "") == password
        assert urlsplit(result.stdout).hostname == "::1"
    else:
        assert result.returncode != 0
        assert not result.stdout
        assert "REVAER_LOCAL_DB_PASSWORD" in result.stderr
