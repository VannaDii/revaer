"""Typed settings loaded once at the CLI boundary.

Defaults here preserve the existing recipe and release-script contracts. Tasks
receive these values instead of interpreting environment variables themselves.
Credentials are omitted from dataclass representations as well as process logs.
"""

import secrets
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal
from urllib.parse import quote

from .automation.settings import WorkflowSettings, load_workflow_settings
from .bootstrap.settings import KcovSettings, load_kcov_settings
from .database.lifecycle_settings import LifecycleSettings, load_lifecycle_settings
from .development.settings import DevelopmentSettings, load_development_settings
from .e2e.settings import E2eSettings, load_e2e_settings
from .errors import ToolingError
from .external.charts import RegistryCredentials
from .images.settings import (
    ContainerSettings,
    ImageReleaseSettings,
    load_container_settings,
    load_image_release_settings,
)
from .media.settings import FixtureSettings, load_fixture_settings
from .sonar.settings import SonarSettings, load_sonar_settings


@dataclass(frozen=True)
class ChartSettings:
    repository: str
    image_repository: str
    sign: bool
    private_key: str = field(repr=False)
    public_key: str = field(repr=False)
    owner_name: str
    owner_email: str
    repository_id: str
    lint_database_url: str = field(repr=False)
    registry: RegistryCredentials
    namespace: str


@dataclass(frozen=True)
class DatabaseSettings:
    """Keep migration and test URL precedence explicit at the CLI boundary."""

    test_url: str | None = field(repr=False)
    url: str = field(repr=False)
    managed: bool
    reset: bool
    container_name: str | None
    data_directory: Path | None
    shared_memory_bytes: int
    pool_proof: Path | None
    cancellation_proof: Path | None


@dataclass(frozen=True)
class CoverageSettings:
    test_threads: int
    build_jobs: int | Literal["default"]
    native_recovery_root: Path | None


@dataclass(frozen=True)
class ImageSettings:
    platforms: tuple[str, ...]
    version: str | None
    builder: str
    name: str
    scan_reference: str


@dataclass(frozen=True)
class Settings:
    lifecycle: LifecycleSettings
    development: DevelopmentSettings
    chart: ChartSettings
    helm_release_assets: bool
    workflow: WorkflowSettings
    diff_base: str | None
    diff_head: str
    udeps_toolchain: str | None
    udeps_version: str | None
    database: DatabaseSettings
    coverage: CoverageSettings
    e2e: E2eSettings
    images: ImageSettings
    image_release: ImageReleaseSettings
    container: ContainerSettings
    sonar: SonarSettings
    kcov: KcovSettings
    fixtures: FixtureSettings


def local_database_url(environment: Mapping[str, str], database: str) -> str:
    """Retain the media stack's local overrides and encode credential characters."""
    user = environment.get("REVAER_LOCAL_DB_USER") or "revaer"
    password = environment.get("REVAER_LOCAL_DB_PASSWORD") or user
    host = environment.get("REVAER_LOCAL_DB_HOST") or "localhost"
    try:
        port = int(environment.get("REVAER_LOCAL_DB_PORT") or "5432")
    except ValueError as error:
        raise ToolingError("REVAER_LOCAL_DB_PORT must be an integer") from error
    if not 1 <= port <= 65535:
        raise ToolingError("REVAER_LOCAL_DB_PORT must be between 1 and 65535")
    if ":" in host and not host.startswith("["):
        host = f"[{host}]"
    return f"postgres://{quote(user, safe='')}:{quote(password, safe='')}@{host}:{port}/{database}"


def switch(environment: Mapping[str, str], name: str, default: bool) -> bool:
    value = environment.get(name) or str(int(default))
    if value not in ("0", "1"):
        raise ToolingError(f"{name} must be 0 or 1")
    return value == "1"


def positive_integer(environment: Mapping[str, str], name: str, default: int) -> int:
    try:
        value = int(environment.get(name) or default)
    except ValueError as error:
        raise ToolingError(f"{name} must be a positive integer") from error
    if value < 1:
        raise ToolingError(f"{name} must be a positive integer")
    return value


def cargo_jobs(environment: Mapping[str, str]) -> int | Literal["default"]:
    """Preserve Cargo's negative CPU offsets and explicit default setting."""
    value = environment.get("CARGO_BUILD_JOBS") or "1"
    if value == "default":
        return "default"
    try:
        jobs = int(value)
    except ValueError as error:
        raise ToolingError("CARGO_BUILD_JOBS must be a nonzero integer or default") from error
    if jobs == 0:
        raise ToolingError("CARGO_BUILD_JOBS must be a nonzero integer or default")
    return jobs


def load_settings(environment: Mapping[str, str], *, linux: bool = False) -> Settings:
    """Resolve existing override precedence without requiring unused credentials."""
    repository = (
        environment.get("REVAER_RELEASE_REPOSITORY")
        or environment.get("GITHUB_REPOSITORY")
        or "VannaDii/Revaer"
    )
    owner = repository.split("/")[0].lower()
    host = environment.get("HELM_REGISTRY_HOST") or "ghcr.io"
    namespace_owner = (
        environment.get("GITHUB_REPOSITORY_OWNER")
        or (environment.get("GITHUB_REPOSITORY") or repository).split("/")[0]
    ).lower()
    username = environment.get("HELM_REGISTRY_USERNAME") or environment.get("HELM_API_KEY_ID") or ""
    password = (
        environment.get("HELM_REGISTRY_PASSWORD") or environment.get("HELM_API_KEY_SECRET") or ""
    )
    if host == "ghcr.io" and (not username or not password):
        username = (
            environment.get("GITHUB_ACTOR") or environment.get("GITHUB_REPOSITORY_OWNER") or ""
        )
        password = environment.get("GITHUB_TOKEN") or ""
    registry_ca = environment.get("HELM_REGISTRY_CA_FILE")
    test_url = environment.get("REVAER_TEST_DATABASE_URL") or environment.get("DATABASE_URL")
    mode = environment.get("REVAER_DB_MANAGED") or "auto"
    if mode not in ("auto", "0", "1", "true", "false"):
        raise ToolingError("REVAER_DB_MANAGED must be auto, 0, 1, true, or false")
    data_directory = environment.get("REVAER_DB_DATA_DIR")
    return Settings(
        lifecycle=load_lifecycle_settings(environment),
        development=load_development_settings(environment),
        fixtures=load_fixture_settings(environment),
        container=load_container_settings(environment),
        image_release=load_image_release_settings(environment),
        sonar=load_sonar_settings(environment, linux=linux),
        kcov=load_kcov_settings(environment),
        images=ImageSettings(
            platforms=tuple(
                part.strip()
                for part in (environment.get("PLATFORMS") or "linux/amd64,linux/arm64").split(",")
            ),
            version=environment.get("VERSION") or None,
            builder=environment.get("BUILDX_BUILDER") or "revaer-builder",
            name=environment.get("REVAER_LOCAL_IMAGE") or "revaer",
            scan_reference=environment.get("REVAER_SCAN_IMAGE") or "revaer:ci",
        ),
        chart=ChartSettings(
            repository=repository,
            image_repository=environment.get("REVAER_HELM_IMAGE_REPOSITORY")
            or f"ghcr.io/{owner}/revaer",
            sign=switch(environment, "REVAER_HELM_SIGN", True),
            private_key=environment.get("HELM_GPG_PRIVATE", ""),
            public_key=environment.get("HELM_GPG_PUBLIC", ""),
            owner_name=environment.get("ARTIFACTHUB_OWNER_NAME", ""),
            owner_email=environment.get("ARTIFACTHUB_OWNER_EMAIL", ""),
            repository_id=environment.get("ARTIFACTHUB_REPOSITORY_ID", ""),
            # Used only for Helm rendering; this does not open a database connection.
            lint_database_url=environment.get("REVAER_HELM_LINT_DATABASE_URL")
            or f"postgres://revaer:{secrets.token_hex(24)}@postgres.default.svc.cluster.local:5432/revaer",
            registry=RegistryCredentials(
                host, username, password, Path(registry_ca).resolve() if registry_ca else None
            ),
            namespace=environment.get("HELM_REGISTRY_NAMESPACE") or f"{namespace_owner}/charts",
        ),
        # The existing release engine includes Helm only when explicitly enabled.
        helm_release_assets=switch(environment, "REVAER_ENABLE_HELM_RELEASE_ASSETS", False),
        workflow=load_workflow_settings(environment),
        diff_base=environment.get("REVAER_INSTRUCTION_DIFF_BASE") or None,
        diff_head=environment.get("REVAER_INSTRUCTION_DIFF_HEAD") or "HEAD",
        udeps_toolchain=environment.get("REVAER_UDEPS_TOOLCHAIN") or None,
        udeps_version=environment.get("REVAER_UDEPS_VERSION") or None,
        database=DatabaseSettings(
            test_url=test_url,
            # Application operations share the media stack's revaer default.
            # Test commands still require an explicitly supplied disposable URL.
            url=environment.get("DATABASE_URL")
            or test_url
            or local_database_url(environment, "revaer"),
            managed=mode in ("1", "true") or (mode == "auto" and test_url is None),
            reset=switch(environment, "REVAER_DB_RESET", False),
            container_name=environment.get("PG_CONTAINER") or None,
            data_directory=Path(data_directory) if data_directory else None,
            shared_memory_bytes=database_shared_memory(environment),
            pool_proof=(
                Path(value) if (value := environment.get("REVAER_INGESTION_POOL_PROOF")) else None
            ),
            cancellation_proof=(
                Path(value)
                if (value := environment.get("REVAER_INGESTION_CANCELLATION_PROOF"))
                else None
            ),
        ),
        coverage=CoverageSettings(
            test_threads=positive_integer(environment, "RUST_TEST_THREADS", 1),
            build_jobs=cargo_jobs(environment),
            native_recovery_root=(
                Path(value) if (value := environment.get("REVAER_NATIVE_RECOVERY_ROOT")) else None
            ),
        ),
        e2e=load_e2e_settings(environment),
    )


def database_shared_memory(environment: Mapping[str, str]) -> int:
    """Accept Docker's byte/k/m/g size notation and the existing byte minimum."""
    value = (environment.get("REVAER_DB_SHM_SIZE") or "1g").lower()
    units = {"k": 1024, "m": 1024**2, "g": 1024**3}
    suffix = value[-1:]
    try:
        size = int(value[:-1]) * units[suffix] if suffix in units else int(value)
    except ValueError as error:
        raise ToolingError(
            "REVAER_DB_SHM_SIZE must be bytes or an integer with k, m, or g suffix"
        ) from error
    minimum = positive_integer(environment, "REVAER_DB_SHM_BYTES", 1024**3)
    if size < minimum:
        raise ToolingError("REVAER_DB_SHM_SIZE is smaller than REVAER_DB_SHM_BYTES")
    return size
