"""Thin CLI boundary: parse, construct collaborators, dispatch static methods."""

import argparse
import os
import platform
import secrets
import shutil
import signal
import sys
import time
from collections.abc import Callable, Sequence
from datetime import UTC, datetime
from pathlib import Path
from types import MappingProxyType
from urllib.request import build_opener

from .bootstrap.install import kcov_command
from .bootstrap.selection import APT_PROFILES, comma_selection
from .context import Context, HostIdentity, Options, TaskResult, Tools
from .database.postgres import ProofDatabase
from .development.processes import ProcessManager
from .development.watching import FileWatcher
from .errors import ToolingError
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
from .external.http import Http, VisibleRedirects
from .external.images import Buildx, Trivy
from .external.media import Ffmpeg, Ffprobe
from .external.packages import Apk, Apt
from .external.postgres import PostgresDocker
from .external.python import Python, Uv
from .external.release import SemanticRelease
from .external.rust import Cargo, CargoUdeps, Lychee, Mdbook, MdbookMermaid, Rustup, Trunk
from .external.serving import Application, Listeners
from .external.sonar import SonarApi, SonarScanner
from .filesystem import FileSystem
from .process import ProcessRunner, Redactor, stderr_message
from .repository import checkout
from .settings import load_settings
from .sonar.install import scanner_command
from .tasks.automation import (
    VerifySupplyChainResults,
    WorkflowChartVersions,
    WorkflowMatrix,
    WorkflowMetadata,
    WorkflowReport,
)
from .tasks.bootstrap import ScriptCoverage
from .tasks.build import (
    ApiExport,
    Audit,
    Build,
    BuildRelease,
    Check,
    CheckAssets,
    Deny,
    Docs,
    DocsBuild,
    DocsIndex,
    DocsInstall,
    DocsLinks,
    DocsServe,
    Fmt,
    FmtFix,
    Licenses,
    ReleaseArtifacts,
    Sbom,
    SyncAssets,
    ToolingAudit,
    ToolingCheck,
    ToolingCoverage,
    Udeps,
    UiBuild,
    UiServe,
)
from .tasks.charts import (
    ComplianceChartTest,
    HelmAnnotationTest,
    HelmLint,
    HelmPackage,
    HelmPackageTest,
    HelmPublish,
    HelmVerify,
)
from .tasks.compliance import (
    ImageComplianceGenerate,
    ImageComplianceValidate,
    MediaComplianceGuardrails,
    TrivySarifVerify,
)
from .tasks.containers import ContainerBuild, ContainerRuntime
from .tasks.coverage import Coverage, CoverageReport
from .tasks.database import DatabaseMigrate, DatabaseReset, DatabaseSeed, DatabaseStart
from .tasks.database_lifecycle import DatabaseTestDrop, DatabaseTestInit
from .tasks.database_probes import (
    DatabaseBaselineRead,
    DatabaseCancellationProbe,
    DatabasePoolProbe,
)
from .tasks.database_rebaseline import (
    AssemblyChangedLines,
    DatabaseCandidate,
    DatabaseFinalize,
    DatabaseFreeze,
    DatabasePrefix,
    StackChangedLines,
)
from .tasks.development import Development, Zombies
from .tasks.docs import DocsGuard, DocsPrepare
from .tasks.e2e import Runbook, UiE2e, UiE2eCoverage, UiE2eShardCoverage
from .tasks.image_release import (
    ImageAttestationVerify,
    ImageBuildPush,
    ImageBuildVerify,
    ImageInventory,
    ImageManifestCreate,
    ImageManifestSign,
    ImageManifestVerify,
    ImageScan,
    ImageScanCategory,
    ImageSignAttest,
)
from .tasks.images import DockerBuild, DockerScan
from .tasks.javascript_coverage import JavaScriptCoverageMerge
from .tasks.media import (
    CleanFixtures,
    CleanTestMedia,
    DownloadFixtures,
    FixtureCacheKey,
    GenerateFixtures,
    TestFixtureScripts,
    TestMediaBrokerCodec,
    TestMediaConversion,
    TestMediaRootCatalog,
    TestMediaRootContract,
    UpdateFixtureProbes,
    VerifyFixtures,
)
from .tasks.native import SonarCompileDatabase
from .tasks.policy import InstructionDrift, Lint, Policy
from .tasks.pristine import PristineGenerate, PristineTest, PristineValidate
from .tasks.python_coverage import PythonCoverageMerge
from .tasks.quality import Ci, Lock, Validate
from .tasks.release import ReleasePreview, ReleasePublish, ReleaseResume
from .tasks.setup import Doctor, Setup
from .tasks.sonar import (
    SonarPackageReport,
    SonarPrepareScm,
    SonarPrepareSources,
    SonarScan,
    SonarVerifyInputs,
    SonarVerifyResult,
)
from .tasks.testing import (
    LintRuntimeShutdown,
    Test,
    TestFeaturesMinimal,
    TestMediaRecovery,
    TestMediaServiceRecovery,
    TestNative,
    TestRuntimeShutdown,
    UiE2eAppTest,
)

COMMANDS: dict[str, Callable[[Context], TaskResult]] = {
    "test-media-service-recovery": TestMediaServiceRecovery.run,
    "test-media-recovery": TestMediaRecovery.run,
    "db-pristine-catalog-generate": PristineGenerate.run,
    "db-pristine-catalog-validate": PristineValidate.run,
    "db-pristine-catalog-test": PristineTest.run,
    "db-rebaseline-freeze": DatabaseFreeze.run,
    "db-rebaseline-candidate": DatabaseCandidate.run,
    "db-init-prefix-check": DatabasePrefix.run,
    "db-init-finalize": DatabaseFinalize.run,
    "test-database-baseline-read": DatabaseBaselineRead.run,
    "db-init-pool-probe": DatabasePoolProbe.run,
    "db-init-cancellation-probe": DatabaseCancellationProbe.run,
    "stack-changed-lines": StackChangedLines.run,
    "db-init-assembly-changed-lines": AssemblyChangedLines.run,
    "dev": Development.run,
    "zombies": Zombies.run,
    "download-test-fixtures": DownloadFixtures.run,
    "generate-test-fixtures": GenerateFixtures.run,
    "verify-test-fixtures": VerifyFixtures.run,
    "update-test-fixture-probes": UpdateFixtureProbes.run,
    "test-media-conversion": TestMediaConversion.run,
    "test-media-root-catalog": TestMediaRootCatalog.run,
    "test-media-root-contract": TestMediaRootContract.run,
    "test-media-broker-codec": TestMediaBrokerCodec.run,
    "clean-test-fixtures": CleanFixtures.run,
    "clean-test-media": CleanTestMedia.run,
    "test-fixture-scripts": TestFixtureScripts.run,
    "fixture-cache-key": FixtureCacheKey.run,
    "workflow-metadata": WorkflowMetadata.run,
    "workflow-chart-versions": WorkflowChartVersions.run,
    "workflow-matrix": WorkflowMatrix.run,
    "workflow-report": WorkflowReport.run,
    "verify-supply-chain-results": VerifySupplyChainResults.run,
    "policy": Policy.run,
    "lint": Lint.run,
    "instruction-drift": InstructionDrift.run,
    "validate": Validate.run,
    "ci": Ci.run,
    "lock": Lock.run,
    "release-lock": Lock.run,
    "release preview": ReleasePreview.run,
    "release publish": ReleasePublish.run,
    "release resume": ReleaseResume.run,
    "helm-lint": HelmLint.run,
    "helm-annotation-test": HelmAnnotationTest.run,
    "helm-package-test": HelmPackageTest.run,
    "compliance-chart-test": ComplianceChartTest.run,
    "helm-package": HelmPackage.run,
    "helm-publish": HelmPublish.run,
    "helm-verify": HelmVerify.run,
    "setup": Setup.run,
    "doctor": Doctor.run,
    "fmt": Fmt.run,
    "fmt-fix": FmtFix.run,
    "tooling-check": ToolingCheck.run,
    "tooling-audit": ToolingAudit.run,
    "tooling-cov": ToolingCoverage.run,
    "script-coverage": ScriptCoverage.run,
    "check": Check.run,
    "test": Test.run,
    "test-native": TestNative.run,
    "test-runtime-shutdown": TestRuntimeShutdown.run,
    "lint-runtime-shutdown": LintRuntimeShutdown.run,
    "test-features-min": TestFeaturesMinimal.run,
    "db-migrate": DatabaseMigrate.run,
    "db-test-init": DatabaseTestInit.run,
    "db-test-drop": DatabaseTestDrop.run,
    "db-start": DatabaseStart.run,
    "db-reset": DatabaseReset.run,
    "db-seed": DatabaseSeed.run,
    "cov": Coverage.run,
    "cov-report": CoverageReport.run,
    "sonar-compile-db": SonarCompileDatabase.run,
    "sonar-scan": SonarScan.run,
    "sonar-verify-inputs": SonarVerifyInputs.run,
    "sonar-prepare-scm": SonarPrepareScm.run,
    "sonar-prepare-sources": SonarPrepareSources.run,
    "python-coverage-merge": PythonCoverageMerge.run,
    "js-coverage-merge": JavaScriptCoverageMerge.run,
    "sonar-package-report": SonarPackageReport.run,
    "sonar-verify-result": SonarVerifyResult.run,
    "docker-build": DockerBuild.run,
    "docker-scan": DockerScan.run,
    "trivy-sarif-verify": TrivySarifVerify.run,
    "image-compliance-generate": ImageComplianceGenerate.run,
    "image-compliance-validate": ImageComplianceValidate.run,
    "media-compliance-guardrails": MediaComplianceGuardrails.run,
    "image-build-push": ImageBuildPush.run,
    "image-build-verify": ImageBuildVerify.run,
    "image-inventory": ImageInventory.run,
    "image-scan": ImageScan.run,
    "image-sign-attest": ImageSignAttest.run,
    "image-attestation-verify": ImageAttestationVerify.run,
    "image-manifest-create": ImageManifestCreate.run,
    "image-manifest-verify": ImageManifestVerify.run,
    "image-manifest-sign": ImageManifestSign.run,
    "image-scan-category": ImageScanCategory.run,
    "container-build": ContainerBuild.run,
    "container-runtime": ContainerRuntime.run,
    "sync-assets": SyncAssets.run,
    "check-assets": CheckAssets.run,
    "build": Build.run,
    "build-release": BuildRelease.run,
    "api-export": ApiExport.run,
    "release-artifacts": ReleaseArtifacts.run,
    "audit": Audit.run,
    "deny": Deny.run,
    "udeps": Udeps.run,
    "sbom": Sbom.run,
    "licenses": Licenses.run,
    "ui-build": UiBuild.run,
    "ui-serve": UiServe.run,
    "ui-e2e-app-test": UiE2eAppTest.run,
    "ui-e2e": UiE2e.run,
    "ui-e2e-coverage": UiE2eCoverage.run,
    "ui-e2e-shard-coverage": UiE2eShardCoverage.run,
    "runbook": Runbook.run,
    "docs": Docs.run,
    "docs-install": DocsInstall.run,
    "docs-build": DocsBuild.run,
    "docs-serve": DocsServe.run,
    "docs-index": DocsIndex.run,
    "docs-link-check": DocsLinks.run,
    "docs-guard": DocsGuard.run,
    "docs-prepare": DocsPrepare.run,
}

E2E_ENVIRONMENT_TASKS = frozenset(
    ("ui-e2e", "runbook", "python-coverage-merge", "js-coverage-merge")
)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(prog="rv", description="Revaer development and automation")
    commands = result.add_subparsers(dest="command", required=True)
    release = commands.add_parser("release")
    release_commands = release.add_subparsers(dest="release_command", required=True)
    for name in COMMANDS:
        command = (
            release_commands.add_parser(name.split()[1])
            if name.startswith("release ")
            else commands.add_parser(name)
        )
        command.set_defaults(task=name)
        if name in ("container-build", "container-runtime"):
            command.set_defaults(source_archive=True)
        if name in ("db-test-init", "db-test-drop"):
            command.add_argument("database_name")
        if name == "workflow-matrix":
            command.add_argument("--release-only", dest="release_matrix", action="store_true")
        if name in ("workflow-report", "trivy-sarif-verify"):
            command.add_argument("report", type=Path)
        if name == "image-compliance-generate":
            command.add_argument("image_reference")
            command.add_argument("inventory", type=Path)
            command.add_argument("output", type=Path)
        if name == "image-compliance-validate":
            command.add_argument("bundle", type=Path)
            command.add_argument("image_reference")
        if name == "workflow-chart-versions":
            command.add_argument("--chart-version", default="")
            command.add_argument("--app-version", default="")
            command.add_argument("--pr-number", dest="pull_request", type=int)
        if name in E2E_ENVIRONMENT_TASKS:
            command.add_argument(
                "--e2e-environment-ready", action="store_true", help=argparse.SUPPRESS
            )
        if name == "ui-e2e-shard-coverage":
            command.add_argument(
                "--shards", type=int, default=3, help="Expected shard count (default: 3)"
            )
        if name in ("helm-package", "helm-publish", "helm-verify"):
            command.add_argument("chart_version")
            command.add_argument("app_version")
        if name == "setup":
            command.add_argument("--profile", choices=("dev", "ci", "python"), default="dev")
            command.add_argument("--no-launcher", action="store_true")
            command.add_argument(
                "--cargo-tools",
                type=comma_selection,
                help="Comma-separated pinned Cargo tools; empty selects none",
            )
            command.add_argument(
                "--browsers",
                type=comma_selection,
                help="Comma-separated browser engines; empty selects none",
            )
            command.add_argument("--apt-profile", choices=tuple(APT_PROFILES), default="")
            command.add_argument(
                "--apt-packages",
                type=lambda value: tuple(value.split()),
                default=(),
                help="Whitespace-separated Debian packages; overrides the apt profile",
            )
            command.add_argument(
                "--sonar-scanner",
                action="store_true",
                help="Install the pinned, signature-verified Sonar scanner",
            )
            command.add_argument(
                "--kcov", action="store_true", help="Build the pinned native kcov for Linux CI"
            )
        if name == "instruction-drift":
            command.add_argument("--base", help="Base Git revision; defaults to the branch diff")
            command.add_argument("--head", help="Head Git revision; defaults to HEAD")
        if name in ("stack-changed-lines", "db-init-assembly-changed-lines"):
            command.add_argument("--base", required=True, help="Base commit or ref")
            command.add_argument("--head", required=True, help="Head commit or ref")
        if name == "release resume":
            command.add_argument("release_tag", help="Existing release tag at the current commit")
        if name == "release publish":
            command.add_argument("--tag", dest="release_tag", help="Publish an existing stable tag")
    return result


def make_context(options: Options) -> Context:
    root = checkout(Path.cwd().resolve(), source_archive=options.source_archive)
    environment = dict(os.environ)
    runner = ProcessRunner(stderr_message)
    env = MappingProxyType(environment)
    settings = load_settings(env, linux=sys.platform == "linux")
    host = HostIdentity(os.getuid(), os.getgid(), Path.home(), sys.platform, platform.machine())
    http = Http(build_opener(VisibleRedirects()))
    rustc = Rustc(
        environment.get("RUSTC_COMMAND") or environment.get("RUSTC") or "rustc", runner, root, env
    )
    uv = Uv("uv", runner, root, env)
    git = Git(
        "git", runner, root, env, installation_hint="install Git with your system package manager"
    )
    tools = Tools(
        lifecycle_docker=LifecycleDocker("docker", runner, root, env),
        proof_databases=ProofDatabase(
            PostgresDocker("docker", runner, root, env),
            FileSystem(),
            time.sleep,
            lambda: secrets.token_hex(16),
        ),
        watcher=FileWatcher(root, git),
        processes=ProcessManager(
            root, os.getpid(), host.uid, time.monotonic, lambda: secrets.token_hex(32)
        ),
        curl=Curl(environment.get("REVAER_FIXTURE_CURL_BIN") or "curl", runner, root, env),
        ffmpeg=Ffmpeg("ffmpeg", runner, root, env),
        ffprobe=Ffprobe(
            environment.get("REVAER_FIXTURE_FFPROBE_BIN") or "ffprobe", runner, root, env
        ),
        semantic_release=SemanticRelease("semantic-release", runner, root, env),
        helm=Helm(
            "helm",
            runner,
            root,
            env,
            installation_hint="install Helm with your system package manager",
        ),
        oras=Oras(
            "oras",
            runner,
            root,
            env,
            installation_hint="install ORAS with your system package manager",
        ),
        gpg=Gpg(
            "gpg",
            runner,
            root,
            env,
            installation_hint="install GnuPG with your system package manager",
        ),
        gpgconf=GpgConf(
            "gpgconf",
            runner,
            root,
            env,
            installation_hint="install GnuPG with your system package manager",
        ),
        cargo=Cargo(
            "cargo", runner, root, env, installation_hint="install Rust from https://rustup.rs"
        ),
        rustup=Rustup(
            "rustup", runner, root, env, installation_hint="install Rust from https://rustup.rs"
        ),
        git=git,
        github=GitHub("gh", runner, root, env, installation_hint="install the GitHub CLI"),
        python=Python(sys.executable, runner, root, env),
        uv=uv,
        trunk=Trunk("trunk", runner, root, env),
        mdbook=Mdbook("mdbook", runner, root, env),
        lychee=Lychee("lychee", runner, root, env),
        cargo_audit=ExternalTool("cargo-audit", runner, root, env),
        cargo_deny=ExternalTool("cargo-deny", runner, root, env),
        cargo_llvm_cov=CargoLlvmCov("cargo-llvm-cov", runner, root, env),
        cargo_udeps=CargoUdeps("cargo-udeps", runner, root, env),
        sqlx=Sqlx("sqlx", runner, root, env),
        mdbook_mermaid=MdbookMermaid("mdbook-mermaid", runner, root, env),
        native_prerequisites=tuple(
            ExternalTool(
                name, runner, root, env, installation_hint="install this native prerequisite"
            )
            for name in ("pkg-config",)
        ),
        rustc=rustc,
        cc=ExternalTool(
            environment.get("CC") or shutil.which("clang-19", path=env.get("PATH", "")) or "clang",
            runner,
            root,
            env,
        ),
        cxx=ExternalTool(
            environment.get("CXX")
            or shutil.which("clang++-19", path=env.get("PATH", ""))
            or "clang++",
            runner,
            root,
            env,
        ),
        llvm_cov=LlvmTool("llvm-cov", runner, root, env, rustc, environment.get("LLVM_COV")),
        llvm_profdata=LlvmTool(
            "llvm-profdata", runner, root, env, rustc, environment.get("LLVM_PROFDATA")
        ),
        docker=Docker("docker", runner, root, env),
        pg_isready=PgIsReady("pg_isready", runner, root, env),
        psql=Psql("psql", runner, root, env),
        http=http,
        application=Application(runner, root, env),
        listeners=Listeners("lsof", runner, root, env),
        buildx=Buildx("docker", runner, root, env),
        trivy=Trivy(
            "trivy", runner, root, env, installation_hint="install Trivy with your package manager"
        ),
        cosign=Cosign(
            "cosign",
            runner,
            root,
            env,
            installation_hint="install Cosign with your package manager",
        ),
        sonar_scanner=SonarScanner(
            environment.get("SONAR_SCANNER_COMMAND") or scanner_command(root, settings.sonar, host),
            runner,
            root,
            env,
        ),
        sonar_api=SonarApi(http, settings.sonar, time.sleep, stderr_message),
        bash=Bash(environment.get("REVAER_KCOV_BASH") or "bash", runner, root, env),
        kcov=Kcov(
            environment.get("REVAER_KCOV_COMMAND") or kcov_command(root, settings.kcov, host),
            runner,
            root,
            env,
            Path(sys.executable),
            uv,
        ),
        cmake=Cmake("cmake", runner, root, env),
        apt=Apt(
            "apt-get",
            runner,
            root,
            env,
            ExternalTool("sudo", runner, root, env) if host.uid != 0 else None,
        ),
        sccache=ExternalTool("sccache", runner, root, env),
        apk=Apk("apk", runner, root, env),
        addgroup=AddGroup("addgroup", runner, root, env),
        adduser=AddUser("adduser", runner, root, env),
    )
    return Context(
        root=root,
        settings=settings,
        options=options,
        tools=tools,
        fs=FileSystem(),
        emit=stderr_message,
        host=host,
        invoked_at=datetime.now(UTC),
        command_names=frozenset(COMMANDS),
    )


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(argv)
    options = Options(
        database_name=getattr(args, "database_name", ""),
        chart_version=getattr(args, "chart_version", "0.0.0-dev.0"),
        app_version=getattr(args, "app_version", "v0.0.0-dev.0"),
        profile=getattr(args, "profile", "dev"),
        install_launcher=not getattr(args, "no_launcher", False),
        base=getattr(args, "base", None),
        head=getattr(args, "head", None),
        release_tag=getattr(args, "release_tag", None),
        expected_shards=getattr(args, "shards", 3),
        sonar_scanner=getattr(args, "sonar_scanner", False),
        kcov=getattr(args, "kcov", False),
        cargo_tools=getattr(args, "cargo_tools", None),
        browsers=getattr(args, "browsers", None),
        apt_profile=getattr(args, "apt_profile", ""),
        apt_packages=getattr(args, "apt_packages", ()),
        release_matrix=getattr(args, "release_matrix", False),
        report=getattr(args, "report", None),
        pull_request=getattr(args, "pull_request", None),
        image_reference=getattr(args, "image_reference", ""),
        inventory=getattr(args, "inventory", None),
        bundle=getattr(args, "bundle", None),
        output=getattr(args, "output", None),
        source_archive=getattr(args, "source_archive", False),
    )

    def interrupted(number: int, frame: object) -> None:
        raise KeyboardInterrupt

    previous = signal.signal(signal.SIGTERM, interrupted)
    try:
        # Use uv's supported dotenv parser, including its environment precedence.
        # The internal flag prevents recursion after uv has loaded tests/.env.
        if args.task in E2E_ENVIRONMENT_TASKS and not args.e2e_environment_ready:
            root = checkout(Path.cwd().resolve())
            environment_file = root / "tests/.env"
            if environment_file.is_file():
                uv = Uv("uv", ProcessRunner(stderr_message), root, dict(os.environ))
                uv.e2e_environment(
                    environment_file, tuple(argv if argv is not None else sys.argv[1:])
                )
                return 0
        context = make_context(options)
        result = COMMANDS[args.task](context)
        if result.message:
            print(result.message)
        return 0
    except ToolingError as error:
        stderr_message(f"rv: {error}")
        return error.exit_code
    except (OSError, ValueError, RuntimeError) as error:
        stderr_message(Redactor(os.environ)(f"rv: {error}"))
        return 1
    except KeyboardInterrupt:
        stderr_message("rv: interrupted; owned processes stopped")
        return 130
    finally:
        signal.signal(signal.SIGTERM, previous)


if __name__ == "__main__":
    sys.exit(main())
