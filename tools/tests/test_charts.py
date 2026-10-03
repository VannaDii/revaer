"""Package the real chart with Helm and disposable signing keys."""

import hashlib
import json
import shutil
import subprocess
import tarfile
import tempfile
from dataclasses import replace
from pathlib import Path

import pytest
import yaml
from fixtures.registry import registry
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options, TaskResult
from revaer_tooling.errors import ToolingError
from revaer_tooling.settings import load_settings
from revaer_tooling.tasks.charts import HelmPackage, HelmPublish, HelmVerify


@pytest.fixture
def chart_context(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    source = Path(__file__).resolve().parents[2]
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    shutil.copytree(source / "charts", tmp_path / "charts")
    monkeypatch.chdir(tmp_path)
    return replace(
        make_context(Options(chart_version="1.2.3-dev.1", app_version="v1.2.3-dev.1")),
        settings=load_settings({}),
    )


def package_unsigned(context: Context) -> TaskResult:
    """Exercise packaging directly so the lint prerequisite cannot invoke itself."""
    return HelmPackage.run(
        replace(
            context,
            settings=replace(context.settings, chart=replace(context.settings.chart, sign=False)),
        )
    )


def chart_metadata(root: Path) -> dict[str, object]:
    with tarfile.open(root / "dist/helm/revaer-1.2.3-dev.1.tgz") as archive:
        assert "revaer/artifacthub-repo.yml" not in archive.getnames()
        stream = archive.extractfile("revaer/Chart.yaml")
        assert stream is not None
        with stream:
            result = yaml.safe_load(stream)
    assert isinstance(result, dict)
    return result


def test_unsigned_packaging_retains_owner_and_source_template(chart_context: Context) -> None:
    context = replace(
        chart_context,
        settings=replace(
            chart_context.settings,
            chart=replace(
                chart_context.settings.chart,
                owner_name="Test's owner",
                owner_email="test@example.invalid",
            ),
        ),
    )
    source = (context.root / "charts/revaer/Chart.yaml").read_bytes()
    result = package_unsigned(context)
    assert "verified" not in result.message
    chart = chart_metadata(context.root)
    assert chart["version"] == "1.2.3-dev.1"
    assert chart["appVersion"] == "v1.2.3-dev.1"
    assert (context.root / "charts/revaer/Chart.yaml").read_bytes() == source
    metadata = yaml.safe_load((context.root / "dist/helm/artifacthub-repo.yml").read_text())
    assert metadata["owners"] == [{"name": "Test's owner", "email": "test@example.invalid"}]
    assert not tuple((context.root / "dist/helm").glob("*.prov"))
    unsigned = replace(
        context,
        settings=replace(context.settings, chart=replace(context.settings.chart, sign=False)),
    )
    HelmVerify.run(unsigned)
    with pytest.raises(ToolingError, match="chart/application versions"):
        HelmVerify.run(replace(unsigned, options=replace(unsigned.options, app_version="wrong")))
    metadata_path = context.root / "dist/helm/artifacthub-repo.yml"
    metadata_path.write_text("{}")
    with pytest.raises(ToolingError, match="repository ID"):
        HelmVerify.run(unsigned)
    metadata_path.write_text("[invalid")
    with pytest.raises(ToolingError, match="invalid YAML"):
        HelmVerify.run(unsigned)


@pytest.fixture(params=(False, True), ids=("default-temporary-root", "long-temporary-root"))
def signed_chart_context(
    chart_context: Context, request: pytest.FixtureRequest, monkeypatch: pytest.MonkeyPatch
) -> Context:
    if request.param:
        temporary_root = chart_context.root / ("long-temporary-directory-" * 4)
        temporary_root.mkdir()
        monkeypatch.setattr(tempfile, "tempdir", str(temporary_root))
    # GnuPG uses Unix sockets. Both pytest's directory and an overridden TMPDIR
    # can exceed their path limit, so keep this process-only fixture under /tmp.
    with tempfile.TemporaryDirectory(prefix="rv-test-key-", dir="/tmp") as temporary:
        home = Path(temporary)
        gpg = chart_context.tools.gpg.locate()
        try:
            subprocess.run(
                [
                    str(gpg),
                    "--homedir",
                    str(home),
                    "--batch",
                    "--pinentry-mode",
                    "loopback",
                    "--passphrase",
                    "",
                    "--quick-generate-key",
                    "Rv packaging tests <tests@example.invalid>",
                    "rsa2048",
                    "sign",
                    "1d",
                ],
                check=True,
                capture_output=True,
                timeout=30,
            )

            def export(secret: bool) -> str:
                return subprocess.run(
                    [
                        str(gpg),
                        "--homedir",
                        str(home),
                        "--batch",
                        "--armor",
                        "--export-secret-keys" if secret else "--export",
                    ],
                    check=True,
                    capture_output=True,
                    text=True,
                    timeout=30,
                ).stdout

            context = replace(
                chart_context,
                settings=replace(
                    chart_context.settings,
                    chart=replace(
                        chart_context.settings.chart,
                        private_key=export(True),
                        public_key=export(False),
                    ),
                ),
            )
            result = HelmPackage.run(context)
        finally:
            chart_context.tools.gpgconf.stop_agent(home)
    assert "verified" in result.message
    return context


def test_signed_chart_verifies_and_tampering_is_rejected(signed_chart_context: Context) -> None:
    context = signed_chart_context
    chart = chart_metadata(context.root)
    annotations = chart["annotations"]
    assert isinstance(annotations, dict)
    assert "artifacthub.io/signKey" in annotations
    directory = context.root / "dist/helm"
    archive = directory / "revaer-1.2.3-dev.1.tgz"
    assert archive.with_suffix(".tgz.prov").stat().st_size > 0
    keyring = directory / "revaer-helm-public.gpg"
    context.tools.helm.verify_chart(archive, keyring)
    archive.write_bytes(archive.read_bytes() + b"tampered")
    with pytest.raises(ToolingError):
        context.tools.helm.verify_chart(archive, keyring)


@pytest.mark.parametrize("version", ["../outside", "--help", "1.2", "1.2.3; echo unsafe"])
def test_bad_version_does_not_delete_existing_artifacts(
    chart_context: Context, version: str
) -> None:
    output = chart_context.root / "dist/helm"
    output.mkdir(parents=True)
    sentinel = output / "retained"
    sentinel.write_text("keep")
    context = replace(chart_context, options=replace(chart_context.options, chart_version=version))
    with pytest.raises(ToolingError, match="version"):
        package_unsigned(context)
    assert sentinel.read_text() == "keep"


def test_missing_signing_material_preserves_previous_package(chart_context: Context) -> None:
    package_unsigned(chart_context)
    package = chart_context.root / "dist/helm/revaer-1.2.3-dev.1.tgz"
    original = package.read_bytes()
    with pytest.raises(ToolingError, match="HELM_GPG_PRIVATE"):
        HelmPackage.run(chart_context)
    assert package.read_bytes() == original


def test_publication_requires_existing_signed_artifacts(chart_context: Context) -> None:
    settings = load_settings(
        {"HELM_REGISTRY_USERNAME": "test", "HELM_REGISTRY_PASSWORD": "test-only"}
    )
    with pytest.raises(ToolingError, match="previously packaged"):
        HelmPublish.run(replace(chart_context, settings=settings))


def test_real_registry_preserves_signed_chart_and_metadata_and_rejects_bad_auth(
    signed_chart_context: Context,
) -> None:
    context = signed_chart_context
    with registry(context.root / "registry") as service:
        context = replace(
            context,
            settings=replace(
                context.settings,
                chart=replace(
                    context.settings.chart,
                    registry=service.credentials,
                    namespace="rv-tests/charts",
                ),
            ),
        )
        assert HelmPublish.run(context).message == "Chart and Artifact Hub metadata published"
        prefix = "/v2/rv-tests/charts/revaer"
        manifest = json.loads(service.fetch(prefix + "/manifests/1.2.3-dev.1"))
        output = context.root / "dist/helm"
        expected = {
            "application/vnd.cncf.helm.chart.content.v1.tar+gzip": output
            / "revaer-1.2.3-dev.1.tgz",
            "application/vnd.cncf.helm.chart.provenance.v1.prov": output
            / "revaer-1.2.3-dev.1.tgz.prov",
        }
        assert {layer["mediaType"] for layer in manifest["layers"]} == set(expected)
        for layer in manifest["layers"]:
            content = service.fetch(prefix + "/blobs/" + layer["digest"])
            assert content == expected[layer["mediaType"]].read_bytes()
            assert layer["digest"] == "sha256:" + hashlib.sha256(content).hexdigest()
        metadata = json.loads(service.fetch(prefix + "/manifests/artifacthub.io"))
        assert len(metadata["layers"]) == 1
        assert (
            service.fetch(prefix + "/blobs/" + metadata["layers"][0]["digest"])
            == (output / "artifacthub-repo.yml").read_bytes()
        )
        for credentials in (
            replace(service.credentials, password="incorrect-fixture-password"),
            replace(service.credentials, ca_file=None),
        ):
            invalid = replace(
                context,
                settings=replace(
                    context.settings, chart=replace(context.settings.chart, registry=credentials)
                ),
            )
            with pytest.raises(ToolingError):
                HelmPublish.run(invalid)
        # Failed authentication/trust checks cannot change a published manifest.
        assert json.loads(service.fetch(prefix + "/manifests/1.2.3-dev.1")) == manifest


def test_native_helm_warning_prevents_packaging(chart_context: Context) -> None:
    """The media packaging contract treats Helm warnings as gate failures."""
    template = chart_context.root / "charts/revaer/templates/deprecated-api.yaml"
    template.write_text(
        "apiVersion: extensions/v1beta1\nkind: Deployment\n"
        "metadata:\n  name: deprecated-api\nspec:\n"
        "  selector:\n    matchLabels:\n      app: warning-fixture\n"
        "  template:\n    metadata:\n      labels:\n        app: warning-fixture\n"
        "    spec:\n      containers:\n        - name: fixture\n          image: fixture:1\n"
    )
    # Keep the control valid for both chart schemas. Synthetic deployment
    # bindings live outside the chart and are never part of a published package.
    values = {"database": {"url": "postgres://localhost/revaer"}}
    source_values = yaml.safe_load((chart_context.root / "charts/revaer/values.yaml").read_text())
    if "compliance" in source_values:
        values.update(
            image={"digest": "sha256:" + "a" * 64, "architecture": "amd64", "tag": ""},
            compliance={
                "existingClaim": "warning-control-not-prepared",
                "manifestDigest": "sha256:" + "b" * 64,
            },
        )
    control_values = chart_context.root / "warning-control-values.yaml"
    control_values.write_text(yaml.safe_dump(values))
    # Establish that this fixture is valid without --strict, so a syntax error
    # cannot masquerade as coverage of the warning gate.
    control = subprocess.run(
        [
            chart_context.tools.helm.locate(),
            "lint",
            str(chart_context.root / "charts/revaer"),
            "--values",
            str(control_values),
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert control.returncode == 0, control.stdout + control.stderr
    assert "[WARNING]" in control.stdout
    with pytest.raises(ToolingError):
        package_unsigned(chart_context)
    assert not (chart_context.root / "dist/helm").exists()


def test_media_packaging_uses_lint_only_bindings_without_publishing_them(
    chart_context: Context,
) -> None:
    fixture = Path(__file__).parent / "fixtures/media-chart/revaer"
    chart = chart_context.root / "charts/revaer"
    shutil.rmtree(chart)
    shutil.copytree(fixture, chart)
    original = (chart / "values.yaml").read_bytes()
    unsigned = replace(
        chart_context,
        settings=replace(
            chart_context.settings, chart=replace(chart_context.settings.chart, sign=False)
        ),
    )
    HelmPackage.run(unsigned)
    assert (chart / "values.yaml").read_bytes() == original
    with tarfile.open(chart_context.root / "dist/helm/revaer-1.2.3-dev.1.tgz") as archive:
        stream = archive.extractfile("revaer/values.yaml")
        assert stream is not None
        with stream:
            assert stream.read() == original
        assert not any("lint-values" in name for name in archive.getnames())
    HelmVerify.run(unsigned)


@pytest.mark.parametrize("mutation", ("missing", "duplicate", "embedded"))
def test_annotation_marker_failure_preserves_previous_package(
    chart_context: Context,
    mutation: str,
) -> None:
    chart = chart_context.root / "charts/revaer/Chart.yaml"
    source = chart.read_text()
    marker = "# __RELEASE_HELM_ANNOTATIONS__"
    assert marker in source
    if mutation == "missing":
        source = source.replace(marker, "# no release marker")
    elif mutation == "duplicate":
        source += "\n" + marker + "\n"
    else:
        source = source.replace(marker, "note: __RELEASE_HELM_ANNOTATIONS__")
    chart.write_text(source)
    previous = chart_context.root / "dist/helm/previous.tgz"
    previous.parent.mkdir(parents=True)
    previous.write_bytes(b"preserve previous verified artifact")
    with pytest.raises(ToolingError, match="exactly one standalone"):
        package_unsigned(chart_context)
    assert previous.read_bytes() == b"preserve previous verified artifact"
    assert tuple(previous.parent.iterdir()) == (previous,)
    assert chart.read_text() == source
