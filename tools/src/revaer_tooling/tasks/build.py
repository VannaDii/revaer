"""Rust build and static asset tasks."""

import hashlib
import json
import tomllib

from ..artifacts import record_artifacts
from ..bootstrap.selection import native_selection
from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.python import PytestArgs
from ..external.rust import CargoArgs, CargoOperation
from .base import Task
from .policy import AdvisoryPolicy


class Fmt(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.python.format()
        context.tools.cargo.fmt()
        return TaskResult()


class FmtFix(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.python.format(fix=True)
        context.tools.cargo.fmt(fix=True)
        return TaskResult()


class ToolingCheck(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.python.format()
        context.tools.python.lint()
        context.tools.python.types()
        context.tools.python.pytest(PytestArgs(("tools/tests",)))
        return TaskResult("Python tooling checks passed")


class ToolingCoverage(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        output = context.root / "coverage"
        if output.is_symlink():
            raise ToolingError("Python coverage directory must not be a symlink")
        context.fs.mkdir(output)
        # The merge command reads the raw tooling database and replaces these
        # reports. Keep both operations under one lock for their full duration.
        with context.fs.lock(output / ".python-coverage.lock"):
            for path in (context.root / ".coverage", output / "python.xml", output / "python.lcov"):
                context.fs.remove_owned(path, context.root)
            context.tools.python.pytest(PytestArgs(("tools/tests",), coverage=True))
            context.tools.python.runtime_coverage()
        return TaskResult()


class Check(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.execute(CargoArgs(CargoOperation.CHECK, all_targets=True))
        return TaskResult()


class SyncAssets(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.run_binary("asset_sync")
        return TaskResult()


class CheckAssets(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        SyncAssets.run(context)
        context.tools.git.check_assets()
        return TaskResult()


class Build(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        SyncAssets.run(context)
        context.tools.cargo.execute(CargoArgs(CargoOperation.BUILD, all_targets=True))
        return TaskResult()


class BuildRelease(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.execute(CargoArgs(CargoOperation.BUILD, all_targets=True, release=True))
        return TaskResult()


class ApiExport(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        destination = context.root / "docs/api/openapi.json"
        # The Rust exporter embeds this file. Invalid merge input can otherwise
        # compile successfully and be replaced by an empty fallback document.
        if destination.exists():
            ApiExport.validate(context.fs.read(destination))
        context.tools.cargo.run_binary("revaer-api", "generate_openapi")
        ApiExport.validate(context.fs.read(destination))
        return TaskResult()

    @staticmethod
    def validate(source: str) -> None:
        try:
            document = json.loads(source)
        except json.JSONDecodeError as error:
            raise ToolingError("OpenAPI export must contain valid JSON") from error
        if (
            not isinstance(document, dict)
            or not isinstance(document.get("openapi"), str)
            or not document["openapi"].startswith("3.")
            or not isinstance(document.get("paths"), dict)
            or not document["paths"]
        ):
            raise ToolingError("OpenAPI export requires a version and nonempty API paths")


class ReleaseArtifacts(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.git.require_clean()
        source = context.tools.git.revision()
        BuildRelease.run(context)
        ApiExport.run(context)
        context.tools.git.require_clean()
        if context.tools.git.revision() != source:
            raise ToolingError("Source commit changed during artifact preparation")
        binary = context.root / "dist/revaer-app"
        # Cargo owns target-directory resolution, including CARGO_TARGET_DIR and
        # .cargo/config.toml. Never package a stale binary from a guessed location.
        target = context.tools.cargo.metadata().target_directory
        context.fs.copy(target / "release/revaer-app", binary)
        with binary.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        context.fs.write(binary.with_suffix(".sha256"), f"{digest}  revaer-app\n")
        context.fs.copy(context.root / "docs/api/openapi.json", context.root / "dist/openapi.json")
        record_artifacts(context, source)
        return TaskResult()


class Audit(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        AdvisoryPolicy.run(context)
        context.tools.cargo.audit()
        return ToolingAudit.run(context)


class ToolingAudit(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        """Audit all groups through uv's standard lock export without resolving again."""
        requirements = context.root / "artifacts/python-audit/pylock.toml"
        context.fs.mkdir(requirements.parent)
        context.tools.uv.export(requirements)
        report = requirements.parent / "audit.json"
        context.fs.remove_owned(report, context.root)
        result = context.tools.python.audit(requirements)
        context.fs.write(report, result.stdout)
        return TaskResult("Python dependency audit passed")


class Deny(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        AdvisoryPolicy.run(context)
        context.tools.cargo.deny()
        return TaskResult()


class Udeps(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        pins = tomllib.loads(context.fs.read(context.root / "tools/versions.toml"))
        channel = pins["rust"]["udeps_toolchain"]
        version = pins["cargo"]["cargo-udeps"]["version"]
        for name, configured, required in (
            ("REVAER_UDEPS_TOOLCHAIN", context.settings.udeps_toolchain, channel),
            ("REVAER_UDEPS_VERSION", context.settings.udeps_version, version),
        ):
            if configured is not None and configured != required:
                raise ToolingError(f"{name} must equal the repository pin {required}")
        installed = context.tools.cargo_udeps.verify(version)
        compiler = context.tools.rustup.compiler_version(channel)
        evidence = (
            f"cargo-udeps-version={installed.version}\ntoolchain={channel}\n{compiler}"
            f"command=cargo +{channel} udeps --workspace --all-targets\n"
        )
        context.fs.write(context.root / "target/udeps-toolchain-evidence.txt", evidence)
        context.emit(evidence.rstrip())
        context.tools.cargo.udeps(channel)
        return TaskResult()


class Sbom(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        metadata = context.tools.cargo.metadata()
        context.fs.write(context.root / "artifacts/sbom.json", metadata.document)
        return TaskResult()


class Licenses(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        # License reporting is also a standalone recipe. Preserve its native
        # pinned-tool prerequisite instead of assuming a prior full setup.
        selection = native_selection(context.fs, context.root, ("cargo-deny",), ())
        for package in selection.cargo:
            context.tools.cargo.install(package)
        content = context.tools.cargo.licenses()
        json.loads(content)
        context.fs.write(context.root / "artifacts/licenses.json", content)
        return TaskResult()


class UiBuild(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        SyncAssets.run(context)
        context.fs.mkdir(context.root / "crates/revaer-ui/dist/.stage")
        context.tools.trunk.build()
        return TaskResult()


class UiServe(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        SyncAssets.run(context)
        context.fs.mkdir(context.root / "crates/revaer-ui/dist-serve/.stage")
        context.tools.trunk.serve(open_browser=True)
        return TaskResult()


def verify_documentation_tools(context: Context) -> None:
    pins = tomllib.loads(context.fs.read(context.root / "tools/versions.toml"))["cargo"]
    # The mermaid preprocessor checks its mdBook protocol version. Reject a
    # shadowing PATH installation before it can produce a warning-bearing build.
    for name, tool in (
        ("mdbook", context.tools.mdbook),
        ("mdbook-mermaid", context.tools.mdbook_mermaid),
    ):
        tool.verify(str(pins[name]["version"]))


class DocsInstall(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        selection = native_selection(context.fs, context.root, ("mdbook", "mdbook-mermaid"), ())
        for package in selection.cargo:
            context.tools.cargo.install(package)
        verify_documentation_tools(context)
        context.tools.mdbook_mermaid.install()
        return TaskResult("Pinned documentation tools and Mermaid integration ready")


class DocsBuild(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        verify_documentation_tools(context)
        context.tools.mdbook.build()
        return TaskResult()


class DocsServe(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        verify_documentation_tools(context)
        context.tools.mdbook.build(serve=True)
        return TaskResult()


class DocsIndex(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.run_binary("revaer-doc-indexer", release=True)
        return TaskResult()


class DocsLinks(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        selection = native_selection(context.fs, context.root, ("lychee",), ())
        for package in selection.cargo:
            context.tools.cargo.install(package)
        context.tools.lychee.check()
        return TaskResult()


class Docs(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        DocsInstall.run(context)
        DocsBuild.run(context)
        DocsIndex.run(context)
        return TaskResult()
