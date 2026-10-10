"""Explicit context shared by static task entrypoints."""

from collections.abc import Callable
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from .database.postgres import ProofDatabase
from .development.processes import ProcessManager
from .development.watching import FileWatcher
from .external.accounts import AddGroup, AddUser
from .external.base import ExternalTool
from .external.bash import Bash, Kcov
from .external.charts import Gpg, GpgConf, Helm, Oras
from .external.cmake import Cmake
from .external.cosign import Cosign
from .external.coverage import CargoLlvmCov, LlvmTool, Rustc
from .external.curl import Curl
from .external.database import PgIsReady, Psql, Sqlx
from .external.database_lifecycle import LifecycleDocker
from .external.docker import Docker
from .external.git import Git
from .external.github import GitHub
from .external.http import Http
from .external.images import Buildx, Trivy
from .external.media import Ffmpeg, Ffprobe
from .external.mount import MkfsExt4, Mount, Unmount
from .external.packages import Apk, Apt
from .external.python import Python, Uv
from .external.release import SemanticRelease
from .external.rust import Cargo, Lychee, Mdbook, MdbookMermaid, Rustup, Trunk
from .external.serving import Application, Listeners
from .external.sonar import SonarApi, SonarScanner
from .filesystem import FileSystem
from .settings import Settings


@dataclass(frozen=True)
class Options:
    """Validated command inputs; additional values require explicit typed fields."""

    profile: str = "dev"
    install_launcher: bool = True
    chart_version: str = "0.0.0-dev.0"
    app_version: str = "v0.0.0-dev.0"
    base: str | None = None
    head: str | None = None
    release_tag: str | None = None
    expected_shards: int = 3
    sonar_scanner: bool = False
    kcov: bool = False
    cargo_tools: tuple[str, ...] | None = None
    browsers: tuple[str, ...] | None = None
    apt_profile: str = ""
    apt_packages: tuple[str, ...] = ()
    release_matrix: bool = False
    report: Path | None = None
    pull_request: int | None = None
    image_reference: str = ""
    inventory: Path | None = None
    bundle: Path | None = None
    output: Path | None = None
    database_name: str = ""
    source_archive: bool = False


@dataclass(frozen=True)
class Tools:
    mount: Mount
    mkfs_ext4: MkfsExt4
    unmount: Unmount
    privilege: ExternalTool | None
    lifecycle_docker: LifecycleDocker
    proof_databases: ProofDatabase
    watcher: FileWatcher
    processes: ProcessManager
    curl: Curl
    ffmpeg: Ffmpeg
    ffprobe: Ffprobe
    semantic_release: SemanticRelease
    helm: Helm
    oras: Oras
    gpg: Gpg
    gpgconf: GpgConf
    cargo: Cargo
    rustup: Rustup
    git: Git
    github: GitHub
    python: Python
    uv: Uv
    trunk: Trunk
    mdbook: Mdbook
    lychee: Lychee
    cargo_audit: ExternalTool
    cargo_deny: ExternalTool
    cargo_llvm_cov: CargoLlvmCov
    cargo_udeps: ExternalTool
    sqlx: Sqlx
    mdbook_mermaid: MdbookMermaid
    native_prerequisites: tuple[ExternalTool, ...]
    rustc: Rustc
    cc: ExternalTool
    cxx: ExternalTool
    llvm_cov: LlvmTool
    llvm_profdata: LlvmTool
    docker: Docker
    pg_isready: PgIsReady
    psql: Psql
    http: Http
    application: Application
    listeners: Listeners
    buildx: Buildx
    trivy: Trivy
    cosign: Cosign
    sonar_scanner: SonarScanner
    sonar_api: SonarApi
    bash: Bash
    kcov: Kcov
    cmake: Cmake
    apt: Apt
    sccache: ExternalTool
    apk: Apk
    addgroup: AddGroup
    adduser: AddUser


@dataclass(frozen=True)
class HostIdentity:
    """Host file ownership supplied only by CLI wiring."""

    uid: int
    gid: int
    home: Path
    system: str
    machine: str


@dataclass(frozen=True)
class Context:
    root: Path
    settings: Settings
    options: Options
    tools: Tools
    fs: FileSystem
    emit: Callable[[str], None]
    host: HostIdentity
    invoked_at: datetime
    command_names: frozenset[str]


@dataclass(frozen=True)
class TaskResult:
    message: str = ""
