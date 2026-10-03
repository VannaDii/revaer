"""Build real disposable images and inspect Docker/OCI output and scan evidence.

The fixture has no application runtime and executes no image commands. Distinct
tags/builders prevent replacing the developer's images or selected builder.
"""

import json
import secrets
import shutil
import subprocess
import tarfile
from collections.abc import Iterator
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.images import ImageBuildArgs
from revaer_tooling.settings import load_settings
from revaer_tooling.tasks.compliance import TrivySarifVerify
from revaer_tooling.tasks.image_release import ImageBuildVerify, ImageScan
from revaer_tooling.tasks.images import DockerBuild, DockerScan


def docker(*arguments: str) -> str:
    return subprocess.run(
        ["docker", *arguments], capture_output=True, text=True, check=True, timeout=120
    ).stdout.strip()


def selected_builders() -> set[str]:
    return {
        entry["Name"]
        for line in docker("buildx", "ls", "--format", "{{json .}}").splitlines()
        if (entry := json.loads(line)).get("Current")
    }


@pytest.fixture
def image_context(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Iterator[Context]:
    source = Path(__file__).resolve().parents[2]
    root = tmp_path / "image, fixture"
    (root / ".git").mkdir(parents=True)
    (root / "tools/src/revaer_tooling").mkdir(parents=True)
    (root / "tools/src/revaer_tooling/cli.py").touch()
    (root / "payload").write_text("fixture bytes\n")
    (root / "Dockerfile").write_text("FROM scratch\nCOPY payload /payload\nUSER 65532\n")
    shutil.copy2(source / "trivy.yaml", root / "trivy.yaml")
    for arguments in (
        ("init", "--quiet"),
        ("add", "."),
        (
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "Image fixture",
        ),
    ):
        subprocess.run(["git", *arguments], cwd=root, capture_output=True, check=True, timeout=20)
    name = "rv-tooling-image-" + secrets.token_hex(8)
    builder = name + "-builder"
    monkeypatch.chdir(root)
    context = make_context(Options())
    context = replace(
        context,
        settings=load_settings(
            {
                "VERSION": "fixture",
                "BUILDX_BUILDER": builder,
                "REVAER_LOCAL_IMAGE": name,
                "REVAER_SCAN_IMAGE": name + ":fixture",
                "PLATFORMS": "linux/arm64",
            }
        ),
    )
    try:
        yield context
    finally:
        # These tags are unique to this fixture; removal never touches user tags.
        identifiers = docker("image", "ls", "--quiet", name).splitlines()
        if identifiers:
            docker("image", "rm", "--force", *sorted(set(identifiers)))
        if builder in docker("buildx", "ls", "--format", "{{.Name}}").splitlines():
            docker("buildx", "rm", "--force", builder)


def test_load_and_oci_exports_retain_platforms_bytes_and_builder_selection(
    image_context: Context,
) -> None:
    context = image_context
    selected = selected_builders()
    DockerBuild.run(context)
    metadata = json.loads((context.root / "artifacts/image-build.json").read_text())
    assert metadata["containerimage.digest"].startswith("sha256:")
    loaded = json.loads(docker("image", "inspect", context.settings.images.name + ":fixture"))[0]
    assert loaded["Os"] == "linux" and loaded["Architecture"] == "arm64"
    assert loaded["Config"]["User"] == "65532"
    assert selected_builders() == selected
    multi = replace(
        context,
        settings=replace(
            context.settings,
            images=replace(context.settings.images, platforms=("linux/amd64", "linux/arm64")),
        ),
    )
    DockerBuild.run(multi)
    archive = context.root / f"artifacts/{context.settings.images.name}-fixture.oci"
    with tarfile.open(archive) as output:
        stream = output.extractfile("index.json")
        assert stream is not None
        with stream:
            index = json.load(stream)
        descriptor = index["manifests"][0]
        stream = output.extractfile("blobs/sha256/" + descriptor["digest"].removeprefix("sha256:"))
        assert stream is not None
        with stream:
            manifest = json.load(stream)
        platforms = {item["platform"]["architecture"] for item in manifest["manifests"]}
        assert {"amd64", "arm64"} <= platforms
    assert selected_builders() == selected
    (context.root / "Dockerfile").write_text("INVALID fixture\n")
    with pytest.raises(ToolingError):
        DockerBuild.run(multi)
    assert not archive.exists()
    assert not (context.root / "artifacts/image-build.json").exists()


def test_real_trivy_scan_preserves_failed_findings_and_rejects_missing_images(
    image_context: Context,
) -> None:
    context = image_context
    subprocess.run(
        [
            "openssl",
            "genpkey",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            "rsa_keygen_bits:2048",
            "-out",
            str(context.root / "payload"),
        ],
        capture_output=True,
        check=True,
        timeout=30,
    )
    DockerBuild.run(context)
    # A generated, disposable private key produces a real scanner finding. It
    # never represents a credential for any service or developer account.
    with pytest.raises(ToolingError):
        DockerScan.run(context)
    report = context.root / "artifacts/image-scan.json"
    result = json.loads(report.read_text())
    assert any(finding.get("Secrets") for finding in result.get("Results", []))
    workflow = replace(
        context,
        settings=replace(
            context.settings,
            image_release=load_settings(
                {
                    "IMAGE_REFERENCE": context.settings.images.scan_reference,
                    "IMAGE_SOURCE": "docker",
                    "PLATFORM": "linux/arm64",
                }
            ).image_release,
        ),
        options=Options(report=Path("trivy-results.sarif")),
    )
    ImageScan.run(workflow)
    with pytest.raises(ToolingError, match="HIGH or CRITICAL"):
        TrivySarifVerify.run(workflow)
    sarif = context.root / "trivy-results.sarif"
    assert sarif.is_file() and sarif.stat().st_size > 0
    missing = replace(
        context,
        settings=replace(
            context.settings,
            images=replace(context.settings.images, scan_reference="rv-no-such-image:fixture"),
        ),
    )
    with pytest.raises(ToolingError):
        DockerScan.run(missing)
    assert not report.exists()


def test_real_workflow_build_loads_the_requested_architecture_and_source_labels(
    image_context: Context,
) -> None:
    context = image_context
    (context.root / "Dockerfile").write_text(
        "FROM scratch\n"
        "ARG RUST_TARGET\nARG BUILD_DATE\nARG VERSION\nARG REVISION\n"
        'LABEL fixture.target="$RUST_TARGET" fixture.date="$BUILD_DATE" '
        'fixture.version="$VERSION" fixture.revision="$REVISION"\n'
        "COPY payload /payload\nUSER 65532\n"
    )
    workflow = replace(
        context,
        settings=replace(
            context.settings,
            image_release=load_settings(
                {
                    "IMAGE_NAME": context.settings.images.name,
                    "VERSION_TAG": "pr-61-fixture",
                    "PLATFORM": "linux/arm64",
                    "ARCH_TAG": "arm64",
                    "RUST_TARGET": "aarch64-unknown-linux-musl",
                }
            ).image_release,
        ),
    )
    before = selected_builders()
    result = json.loads(ImageBuildVerify.run(workflow).message)
    loaded = json.loads(docker("image", "inspect", result["image_tag"]))[0]
    assert (loaded["Os"], loaded["Architecture"]) == ("linux", "arm64")
    assert loaded["Config"]["Labels"] == {
        "fixture.target": "aarch64-unknown-linux-musl",
        "fixture.date": f"{context.invoked_at:%Y-%m-%dT%H:%M:%SZ}",
        "fixture.version": "pr-61-fixture",
        "fixture.revision": context.tools.git.revision()[:7],
    }
    assert selected_builders() == before


@pytest.mark.parametrize(
    "field,value",
    [
        ("builder", "--help"),
        ("name", "../outside"),
        ("version", "../outside"),
        ("version", "a,b"),
        ("platforms", ()),
        ("platforms", ("linux/amd64", "linux/amd64")),
        ("platforms", ("--help",)),
        ("platforms", ("linux/amd64", "linux/arm64")),
    ],
)
def test_bad_build_inputs_fail_before_any_docker_operation(
    tmp_path: Path, field: str, value: str | tuple[str, ...]
) -> None:
    args = ImageBuildArgs("test", ("linux/amd64",), "fixture", "test", tmp_path / "metadata", None)
    if field == "platforms":
        assert isinstance(value, tuple)
        args = replace(args, platforms=value)
    else:
        assert isinstance(value, str)
        if field == "builder":
            args = replace(args, builder=value)
        elif field == "name":
            args = replace(args, name=value)
        else:
            args = replace(args, version=value)
    with pytest.raises(ToolingError):
        args.validate()
