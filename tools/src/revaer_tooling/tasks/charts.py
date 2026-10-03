"""Chart packaging and publication preserve one verified artifact across steps."""

import re
import shutil
import tempfile
from dataclasses import replace
from pathlib import Path

import yaml

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.charts import PackageArgs, RegistryCredentials
from ..external.python import PytestArgs
from .base import Task


def validate_version(value: str) -> str:
    # Helm versions follow SemVer 2.0. Empty identifiers and numeric leading
    # zeroes are invalid even though the old workflow's loose regex accepted them.
    number = r"(?:0|[1-9][0-9]*)"
    identifiers = r"[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*"
    match = re.fullmatch(
        rf"{number}\.{number}\.{number}(?:-({identifiers}))?(?:\+{identifiers})?", value
    )
    if not match or any(
        part.isdecimal() and len(part) > 1 and part.startswith("0")
        for part in (match[1] or "").split(".")
    ):
        raise ToolingError(f"Invalid chart version: {value!r}")
    return value


def validate_app_version(value: str) -> str:
    if not re.fullmatch(r"[0-9A-Za-z._+-]+", value):
        raise ToolingError("Invalid chart application version")
    return value


def release_repository(context: Context) -> str:
    value = context.settings.chart.repository
    if not re.fullmatch(r"[\w.-]+/[\w.-]+", value):
        raise ToolingError("Release repository must be owner/name")
    return value


def registry(context: Context) -> tuple[RegistryCredentials, str]:
    credentials = context.settings.chart.registry
    namespace = context.settings.chart.namespace
    if not credentials.username or not credentials.password:
        raise ToolingError("Chart registry credentials are missing")
    if not re.fullmatch(r"[\w.-]+(?::\d+)?", credentials.host) or not re.fullmatch(
        r"[\w./-]+", namespace
    ):
        raise ToolingError("Invalid chart registry host or namespace")
    return credentials, namespace


def signing_identity(context: Context, home: Path, output: Path) -> tuple[str, str]:
    private = context.settings.chart.private_key
    public = context.settings.chart.public_key
    if not private or not public:
        raise ToolingError("HELM_GPG_PRIVATE and HELM_GPG_PUBLIC are required for chart signing")
    context.tools.gpg.import_key(home, public)
    context.tools.gpg.import_key(home, private)
    context.fs.write(output / "revaer-helm-public.asc", public.rstrip() + "\n")
    context.tools.gpg.export(home, output / "revaer-helm-public.gpg")
    secret = home / "secring.gpg"
    context.fs.write(secret, "", 0o600)
    context.tools.gpg.export(home, secret, secret=True)
    records = [line.split(":") for line in context.tools.gpg.identities(home).splitlines()]
    uid = next((fields[9] for fields in records if len(fields) > 9 and fields[0] == "uid"), "")
    fingerprint = next(
        (fields[9] for fields in records if len(fields) > 9 and fields[0] == "fpr"), ""
    )
    if not uid or not fingerprint:
        raise ToolingError("Unable to resolve imported chart signing identity")
    return uid, fingerprint


def render_annotations(source: str, annotations: dict[str, str]) -> str:
    """Replace exactly one standalone release marker without rewriting the template.

    A substring search can mistake an annotation value for the marker or replace
    duplicate placeholders. Preserve the original renderer's unique-comment
    contract and leave neighboring authored YAML byte-for-byte unchanged.
    """
    markers = tuple(
        re.finditer(r"^[ \t]*#[ \t]*__RELEASE_HELM_ANNOTATIONS__[ \t]*\r?$", source, re.MULTILINE)
    )
    if len(markers) != 1:
        raise ToolingError("Chart requires exactly one standalone release annotation marker")
    rendered = "\n".join(
        "  " + line for line in yaml.safe_dump(annotations, sort_keys=False).splitlines()
    )
    marker = markers[0]
    return source[: marker.start()] + rendered + source[marker.end() :]


def package_chart(context: Context, staging: Path, home: Path) -> None:
    version = validate_version(context.options.chart_version)
    app_version = validate_app_version(context.options.app_version)
    settings = context.settings.chart
    # Build and verify in private staging before replacing the previous output.
    # A bad key, malformed metadata, or Helm failure leaves existing assets intact.
    output = staging / "output"
    context.fs.mkdir(output)
    chart = staging / "revaer"
    shutil.copytree(context.root / "charts/revaer", chart)
    try:
        metadata = yaml.safe_load(context.fs.read(chart / "artifacthub-repo.yml"))
    except yaml.YAMLError as error:
        raise ToolingError("Artifact Hub metadata contains invalid YAML") from error
    if not isinstance(metadata, dict):
        raise ToolingError("Artifact Hub metadata must be a mapping")
    (chart / "artifacthub-repo.yml").unlink()
    repository = release_repository(context)
    image_repository = settings.image_repository
    annotations = {
        "artifacthub.io/prerelease": "true" if "-" in version else "false",
        "artifacthub.io/images": yaml.safe_dump(
            [
                {
                    "name": "revaer",
                    "image": f"{image_repository}:{app_version}",
                    "platforms": ["linux/amd64", "linux/arm64"],
                }
            ],
            sort_keys=False,
        ),
    }
    owner_name = settings.owner_name
    owner_email = settings.owner_email
    uid: str | None = None
    keyring: Path | None = None
    if settings.sign:
        uid, fingerprint = signing_identity(context, home, output)
        keyring = home / "secring.gpg"
        annotations["artifacthub.io/signKey"] = yaml.safe_dump(
            {
                "fingerprint": fingerprint,
                "url": f"https://github.com/{repository}/releases/download/{app_version}/revaer-helm-public.asc",
            },
            sort_keys=False,
        )
        identity = re.fullmatch(r"(.*?) <([^>]+)>", uid)
        if identity:
            owner_name = owner_name or identity[1]
            owner_email = owner_email or identity[2]
    if settings.repository_id:
        metadata.setdefault("repositoryID", settings.repository_id)
    if owner_name and owner_email:
        if any("\n" in value or "\r" in value for value in (owner_name, owner_email)):
            raise ToolingError("Artifact Hub owner values must not contain newlines")
        metadata.setdefault("owners", [{"name": owner_name, "email": owner_email}])
    context.fs.write(output / "artifacthub-repo.yml", yaml.safe_dump(metadata, sort_keys=False))
    source = context.fs.read(chart / "Chart.yaml")
    context.fs.write(chart / "Chart.yaml", render_annotations(source, annotations))
    values = staging / "lint-values.yaml"
    # This is an ephemeral rendering-only database endpoint; no connection is made.
    lint_url = settings.lint_database_url
    lint_values: dict[str, object] = {"database": {"url": lint_url}}
    source_values = yaml.safe_load(context.fs.read(chart / "values.yaml"))
    if isinstance(source_values, dict) and "compliance" in source_values:
        # The media chart requires immutable image and prepared-volume bindings.
        # Supply synthetic identities only to lint, in a file outside the chart;
        # published defaults must still require real deployment evidence.
        lint_values.update(
            image={"digest": "sha256:" + "a" * 64, "architecture": "amd64", "tag": ""},
            compliance={
                "existingClaim": "lint-only-not-prepared",
                "manifestDigest": "sha256:" + "b" * 64,
            },
        )
    context.fs.write(values, yaml.safe_dump(lint_values), 0o600)
    context.tools.helm.lint(chart, values)
    context.tools.helm.package(PackageArgs(chart, output, version, app_version, uid, keyring))
    if settings.sign:
        context.tools.helm.verify_chart(
            output / f"revaer-{version}.tgz", output / "revaer-helm-public.gpg"
        )
    destination = context.root / "dist/helm"
    context.fs.remove_owned(destination, context.root)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(output), destination)


class HelmPackage(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        with (
            tempfile.TemporaryDirectory(prefix="revaer-chart-") as staging,
            # Agent socket paths have a much smaller limit than filesystem
            # paths. A private short home works even when TMPDIR is a worktree.
            tempfile.TemporaryDirectory(prefix="rv-signing-", dir="/tmp") as home,
        ):
            try:
                package_chart(context, Path(staging), Path(home))
            finally:
                if context.settings.chart.sign:
                    context.tools.gpgconf.stop_agent(Path(home))
        message = (
            "Signed chart packaged and verified"
            if context.settings.chart.sign
            else "Chart packaged"
        )
        return TaskResult(message)


class ComplianceChartTest(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        """Validate the selected checkout's chart, never the reference fixture."""
        context.tools.python.pytest(
            PytestArgs(("tools/tests/test_chart_compliance.py",)),
            {"REVAER_TEST_CHART": str(context.root / "charts/revaer")},
        )
        return TaskResult("Selected chart compliance regressions passed")


class HelmAnnotationTest(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.python.pytest(PytestArgs(("tools/tests/test_chart_annotations.py",)))
        return TaskResult("Chart annotation regressions passed")


class HelmPackageTest(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        # These tests call HelmPackage directly, never this prerequisite's parent
        # HelmLint command. Native signing/registry fixtures remain disposable.
        context.tools.python.pytest(
            PytestArgs(("tools/tests/test_charts.py", "tools/tests/test_chart_versions.py"))
        )
        return TaskResult("Chart package regressions passed")


class HelmLint(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        HelmAnnotationTest.run(context)
        # The media chart introduces immutable compliance bindings. Preserve its
        # existing lint prerequisite while the foundation chart lacks that API.
        values = yaml.safe_load(context.fs.read(context.root / "charts/revaer/values.yaml"))
        if isinstance(values, dict) and "compliance" in values:
            ComplianceChartTest.run(context)
        HelmPackageTest.run(context)
        unsigned = replace(
            context,
            settings=replace(context.settings, chart=replace(context.settings.chart, sign=False)),
        )
        return HelmPackage.run(unsigned)


def verified_chart(context: Context, *, signed: bool) -> tuple[Path, Path]:
    """Read the existing package through Helm and verify the requested identity."""
    version = validate_version(context.options.chart_version)
    app = validate_app_version(context.options.app_version)
    output = context.root / "dist/helm"
    chart = output / f"revaer-{version}.tgz"
    metadata = output / "artifacthub-repo.yml"
    keyring = output / "revaer-helm-public.gpg"
    required = (
        (chart, metadata, keyring, chart.with_suffix(".tgz.prov")) if signed else (chart, metadata)
    )
    for path in required:
        if (
            output.is_symlink()
            or path.is_symlink()
            or not path.resolve().is_relative_to(context.root.resolve())
            or not path.is_file()
            or not path.stat().st_size
        ):
            raise ToolingError(f"Missing or invalid previously packaged release artifact: {path}")
    if signed:
        context.tools.helm.verify_chart(chart, keyring)
    try:
        package = yaml.safe_load(context.tools.helm.show_chart(chart).stdout)
        repository_metadata = yaml.safe_load(context.fs.read(metadata))
    except yaml.YAMLError as error:
        raise ToolingError("Packaged chart metadata contains invalid YAML") from error
    if not isinstance(package, dict) or (
        package.get("name"),
        package.get("version"),
        package.get("appVersion"),
    ) != ("revaer", version, app):
        raise ToolingError("Packaged chart does not match the requested chart/application versions")
    if not isinstance(repository_metadata, dict) or not repository_metadata.get("repositoryID"):
        raise ToolingError("Packaged Artifact Hub metadata must retain its repository ID")
    return chart, metadata


class HelmVerify(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        verified_chart(context, signed=context.settings.chart.sign)
        return TaskResult("Packaged chart and Artifact Hub metadata verified")


class HelmPublish(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        credentials, namespace = registry(context)
        chart, metadata = verified_chart(context, signed=True)
        # Both CLIs support a scoped Docker-format registry configuration. Keep
        # this operation's credentials private and out of the developer's store.
        with tempfile.TemporaryDirectory(prefix="rv-registry-") as work:
            configuration = Path(work) / "config.json"
            context.fs.write(configuration, '{"auths": {}}\n', 0o600)
            context.tools.helm.login(credentials, configuration)
            context.tools.oras.login(credentials, configuration)
            context.tools.helm.push(
                chart, f"oci://{credentials.host}/{namespace}", configuration, credentials.ca_file
            )
            context.tools.oras.push_metadata(
                f"{credentials.host}/{namespace}/revaer:artifacthub.io",
                metadata,
                configuration,
                credentials.ca_file,
            )
        return TaskResult("Chart and Artifact Hub metadata published")
