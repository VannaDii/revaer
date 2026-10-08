"""Image publication coordination against an explicitly simulated tool boundary.

Git identity and compliance files are real. The recording runner supplies build,
registry, scan and signature outcomes; it never publishes an external image.
Real Buildx/Trivy compatibility checks live in test_images.py.
"""

import json
import re
import sys
from collections.abc import Callable
from dataclasses import replace
from pathlib import Path
from typing import TypedDict, Unpack

import pytest
from revaer_tooling.context import Context, TaskResult
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.cosign import Cosign, VerifyAttestationArgs
from revaer_tooling.external.images import Buildx, Trivy
from revaer_tooling.images.compliance import BUNDLE
from revaer_tooling.process import Completed, Invocation
from revaer_tooling.settings import load_settings
from revaer_tooling.tasks.compliance import ImageComplianceGenerate, TrivySarifVerify
from revaer_tooling.tasks.image_release import (
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
    image_output,
)
from test_external import RecordingRunner
from test_image_compliance import REFERENCE
from test_image_compliance import context as compliance_context

__all__ = ["compliance_context"]

DIGEST = "sha256:" + "a" * 64
SECOND_DIGEST = "sha256:" + "b" * 64


class ImageRunner(RecordingRunner):
    def __init__(self) -> None:
        super().__init__()
        self.build_digest = DIGEST
        self.registry_digest = DIGEST
        self.tag_digests: dict[str, str] = {}
        self.scan_content = '{"version":"2.1.0","runs":[]}'
        self.fail = ""
        self.attestation = '{"payload":"verified fixture envelope"}\n'

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        args = invocation.argv[1:]
        if args[:2] == ("buildx", "ls"):
            return Completed(0, "fixture-builder\n")
        if args[:2] == ("buildx", "build"):
            Path(args[args.index("--metadata-file") + 1]).write_text(
                json.dumps({"containerimage.digest": self.build_digest})
            )
            if self.fail == "build":
                raise CommandError("Fixture build failed", 17)
        if args[:3] == ("buildx", "imagetools", "inspect"):
            if self.fail == "registry":
                raise CommandError("Fixture registry failed", 18)
            return Completed(0, self.tag_digests.get(args[3], self.registry_digest))
        if args[:3] == ("buildx", "imagetools", "create") and self.fail == "manifest":
            raise CommandError("Fixture manifest publication failed", 19)
        if args[0] == "image":
            ignore = Path(args[args.index("--ignorefile") + 1])
            assert ignore.is_file()
            assert ignore.read_bytes() == b""
            if self.fail != "missing-report":
                Path(args[args.index("--output") + 1]).write_text(self.scan_content)
            if self.fail == "scan":
                raise CommandError("Fixture scanner failed", 20)
        if args[0] == "verify-attestation":
            if self.fail == "verification":
                raise CommandError("Fixture certificate check failed", 21)
            return Completed(0, self.attestation)
        if args[0] == "attest" and self.fail == "attest":
            raise CommandError("Fixture attestation upload failed", 22)
        return Completed(0, "")


@pytest.fixture
def runner() -> ImageRunner:
    return ImageRunner()


@pytest.fixture
def context(compliance_context: Context, runner: ImageRunner) -> Context:
    base = compliance_context
    environment = {
        "IMAGE_NAME": "revaer",
        "VERSION_TAG": "dev",
        "REPOSITORY_OWNER": "VannaDii",
        "PLATFORM": "linux/amd64",
        "ARCH_TAG": "amd64",
        "RUST_TARGET": "x86_64-unknown-linux-musl",
        "IMAGE_REFERENCE": REFERENCE,
        "IMAGE_SOURCE": "remote",
        "BUILDX_BUILDER": "fixture-builder",
        "GITHUB_OUTPUT": str(base.root / "github-output"),
        "GITHUB_REPOSITORY": "VannaDii/Revaer",
        "COMPLIANCE_PREDICATE": f"image-compliance/{BUNDLE}",
        "ATTESTATION_OUTPUT": "image-compliance/verified-attestation.json",
        "MATRIX_NAME": "amd64",
        "RUNNER": "ubuntu-latest",
    }
    return replace(
        base,
        settings=load_settings(environment),
        tools=replace(
            base.tools,
            buildx=Buildx(sys.executable, runner, base.root, {}),
            trivy=Trivy(sys.executable, runner, base.root, {}),
            cosign=Cosign(sys.executable, runner, base.root, {}),
        ),
    )


class ImageChanges(TypedDict, total=False):
    name: str
    version: str
    owner: str
    alias: str
    include_sha: str
    platform: str
    arch_tag: str
    rust_target: str
    source: str
    reference: str
    matrix_name: str


def configured(context: Context, **changes: Unpack[ImageChanges]) -> Context:
    return replace(
        context,
        settings=replace(
            context.settings, image_release=replace(context.settings.image_release, **changes)
        ),
    )


def test_build_outputs_follow_registry_verification_and_keep_source_identity(
    context: Context, runner: ImageRunner
) -> None:
    result = json.loads(ImageBuildPush.run(context).message)
    revision = context.tools.git.revision()[:7]
    assert result == {
        "image_tag": f"ghcr.io/vannadii/revaer:{revision}-amd64",
        "image_digest": DIGEST,
        "image_reference": REFERENCE,
    }
    build = next(call.argv for call in runner.calls if call.argv[1:3] == ("buildx", "build"))
    assert f"REVISION={revision}" in build
    assert "type=provenance,mode=max" in build
    assert "type=sbom" in build
    assert "--push" in build
    assert "--load" not in build
    assert (context.root / "artifacts/image-build-amd64.json").is_file()
    output = context.root / "github-output"
    before = output.read_bytes()
    runner.registry_digest = SECOND_DIGEST
    with pytest.raises(ToolingError, match="does not match"):
        ImageBuildPush.run(context)
    assert output.read_bytes() == before
    runner.registry_digest = "invalid"
    with pytest.raises(ToolingError, match="invalid SHA-256"):
        ImageBuildPush.run(context)
    assert output.read_bytes() == before


@pytest.mark.parametrize("failure", ("build", "registry"))
def test_failed_build_or_registry_never_emits_completion(
    context: Context, runner: ImageRunner, failure: str
) -> None:
    runner.fail = failure
    with pytest.raises(CommandError):
        ImageBuildPush.run(context)
    assert not (context.root / "github-output").exists()


def test_verification_build_loads_and_uses_no_registry_or_signing(
    context: Context, runner: ImageRunner
) -> None:
    result = json.loads(ImageBuildVerify.run(configured(context, owner="")).message)
    assert result == {"image_tag": "revaer:verify-amd64"}
    assert len(runner.calls) == 2
    assert "--load" in runner.calls[-1].argv
    assert "--push" not in runner.calls[-1].argv


@pytest.mark.parametrize(
    "changes",
    (
        {"name": "../outside"},
        {"version": "--bad tag"},
        {"owner": "bad/owner"},
        {"alias": "bad tag"},
        {"include_sha": "1"},
        {"platform": "linux/386"},
        {"arch_tag": "arm64"},
        {"rust_target": "x86_64-unknown-linux-gnu"},
    ),
)
def test_invalid_build_inputs_have_no_external_side_effects(
    context: Context, runner: ImageRunner, changes: ImageChanges
) -> None:
    with pytest.raises(ToolingError):
        ImageBuildPush.run(configured(context, **changes))
    assert not runner.calls


def test_scan_retains_findings_and_scanner_failure_reports(
    context: Context, runner: ImageRunner
) -> None:
    runner.scan_content = json.dumps(
        {
            "version": "2.1.0",
            "runs": [{"tool": {"driver": {"name": "Trivy"}}, "results": [{"ruleId": "TEST"}]}],
        }
    )
    ImageScan.run(context)
    report = context.root / "trivy-results.sarif"
    gate = replace(context, options=replace(context.options, report=report))
    with pytest.raises(ToolingError, match="HIGH or CRITICAL"):
        TrivySarifVerify.run(gate)
    before = report.read_bytes()
    runner.fail = "scan"
    with pytest.raises(CommandError):
        ImageScan.run(context)
    assert report.read_bytes() == before
    runner.fail = "missing-report"
    with pytest.raises(ToolingError, match="nonempty"):
        ImageScan.run(context)
    assert not report.exists()


@pytest.mark.parametrize(
    "task,changes",
    (
        (ImageInventory.run, {"source": "docker"}),
        (ImageScan.run, {"source": "auto"}),
        (ImageScan.run, {"reference": "ghcr.io/vannadii/revaer:dev"}),
        (ImageScan.run, {"reference": "--help", "source": "docker"}),
        (ImageInventory.run, {"platform": "linux/386"}),
    ),
)
def test_scan_selection_fails_before_invocation(
    context: Context,
    runner: ImageRunner,
    task: Callable[[Context], TaskResult],
    changes: ImageChanges,
) -> None:
    with pytest.raises(ToolingError):
        task(configured(context, **changes))
    assert not runner.calls


def test_inventory_preserves_the_native_output_and_explicit_source(
    context: Context, runner: ImageRunner
) -> None:
    runner.scan_content = '{"spdxVersion":"SPDX-2.3","packages":[]}'
    ImageInventory.run(context)
    report = context.root / "image-package-inventory.spdx.json"
    assert report.read_text() == runner.scan_content
    args = runner.calls[-1].argv
    assert args[args.index("--image-src") + 1] == "remote"
    assert args[args.index("--platform") + 1] == "linux/amd64"
    # Completeness remains an independent compliance gate, not an invented
    # package entry or a claim that this fixture scanned an actual image.
    assert not (context.root / "image-compliance" / BUNDLE).exists()


@pytest.mark.parametrize("name", ("../outside", ".git/config", "reports", "reports/clean.sarif"))
def test_reports_cannot_overwrite_metadata_directories_or_tracked_files(
    context: Context, name: str
) -> None:
    with pytest.raises(ToolingError):
        image_output(context, Path(name))


def test_reports_reject_links_and_outside_destinations(context: Context) -> None:
    with pytest.raises(ToolingError):
        image_output(context, context.root.parent / "outside")
    target = context.root / "target"
    target.mkdir()
    (context.root / "linked").symlink_to(target, target_is_directory=True)
    with pytest.raises(ToolingError):
        image_output(context, Path("linked/report.json"))
    (target / "linked").symlink_to(context.root / "missing")
    with pytest.raises(ToolingError):
        image_output(context, Path("target/linked"))


def test_signing_requires_the_complete_bound_bundle_before_any_registry_operation(
    context: Context, runner: ImageRunner
) -> None:
    with pytest.raises(ToolingError):
        ImageSignAttest.run(context)
    assert not runner.calls
    ImageComplianceGenerate.run(context)
    ImageSignAttest.run(context)
    assert [call.argv[1] for call in runner.calls] == ["sign", "attest"]
    assert all(call.argv[-1] == REFERENCE for call in runner.calls)
    runner.calls.clear()
    predicate = context.root / "image-compliance" / BUNDLE
    record = json.loads(predicate.read_text())
    record["image_digest"] = SECOND_DIGEST
    predicate.write_text(json.dumps(record))
    with pytest.raises(ToolingError):
        ImageSignAttest.run(context)
    assert not runner.calls


@pytest.mark.parametrize("content", ('{"payload":"first"}\n{"payload":"second"}\n', '[{"ok":1}]'))
def test_verified_attestations_keep_original_cosign_bytes(
    context: Context, runner: ImageRunner, content: str
) -> None:
    runner.attestation = content
    ImageAttestationVerify.run(context)
    output = context.root / "image-compliance/verified-attestation.json"
    assert output.read_text() == content
    runner.fail = "verification"
    with pytest.raises(CommandError):
        ImageAttestationVerify.run(context)
    assert not output.exists()


@pytest.mark.parametrize("content", ("", "[]", "null", "{}", "invalid", '{"ok":1} garbage'))
def test_invalid_verified_output_cannot_become_evidence(
    context: Context, runner: ImageRunner, content: str
) -> None:
    runner.attestation = content
    with pytest.raises((ToolingError, ValueError)):
        ImageAttestationVerify.run(context)
    assert not (context.root / "image-compliance/verified-attestation.json").exists()


def test_certificate_identity_treats_the_repository_and_github_path_literally() -> None:
    pattern = VerifyAttestationArgs(REFERENCE, "owner.name/repo.name").identity()
    for workflow in ("pr", "ci"):
        assert re.search(
            pattern,
            f"https://github.com/owner.name/repo.name/.github/workflows/{workflow}.yml@refs/heads/main",
        )
    for wrong in (
        "https://githubXcom/owner.name/repo.name/.github/workflows/ci.yml@main",
        "https://github.com/ownerXname/repo.name/.github/workflows/ci.yml@main",
        "https://github.com/owner.name/repo.name/Xgithub/workflows/ci.yml@main",
        "https://github.com/owner.name/repo.name/.github/workflows/other.yml@main",
    ):
        assert not re.search(pattern, wrong)


def test_manifest_resolves_both_sources_then_compares_tags_before_signing(
    context: Context, runner: ImageRunner
) -> None:
    context = configured(context, alias="latest", include_sha="true")
    revision = context.tools.git.revision()[:7]
    runner.tag_digests[f"ghcr.io/vannadii/revaer:{revision}-arm64"] = SECOND_DIGEST
    result = json.loads(ImageManifestCreate.run(context).message)
    assert result == {"image_digest": DIGEST, "image_reference": REFERENCE}
    create = next(
        call.argv for call in runner.calls if call.argv[1:4] == ("buildx", "imagetools", "create")
    )
    assert create[-2:] == (REFERENCE, "ghcr.io/vannadii/revaer@" + SECOND_DIGEST)
    assert len(runner.calls) == 6  # Two source resolutions, create, three tag resolutions.
    runner.calls.clear()
    ImageManifestSign.run(context)
    assert runner.calls[-1].argv[1:] == ("sign", "--yes", REFERENCE)
    assert len(runner.calls) == 4
    runner.calls.clear()
    runner.tag_digests["ghcr.io/vannadii/revaer:latest"] = SECOND_DIGEST
    with pytest.raises(ToolingError, match="same image digest"):
        ImageManifestSign.run(context)
    assert all(call.argv[1] != "sign" for call in runner.calls)


def test_failed_manifest_creation_never_emits_completion(
    context: Context, runner: ImageRunner
) -> None:
    revision = context.tools.git.revision()[:7]
    runner.tag_digests[f"ghcr.io/vannadii/revaer:{revision}-arm64"] = SECOND_DIGEST
    runner.fail = "manifest"
    with pytest.raises(CommandError):
        ImageManifestCreate.run(context)
    assert not (context.root / "github-output").exists()


def test_verification_only_manifest_and_category_preserve_existing_ci_identity(
    context: Context, runner: ImageRunner
) -> None:
    plan = ImageManifestVerify.run(configured(context, alias="dev", include_sha="true")).message
    assert plan.count("ghcr.io/vannadii/revaer:dev\n") == 1
    assert "-amd64" in plan
    assert "-arm64" in plan
    result = json.loads(ImageScanCategory.run(context).message)
    assert result["value"] == (
        ".github/workflows/ci.yml:build-images/arch_tag:amd64/name:amd64/needs_qemu:"
        "/platform:linux/amd64/runner:ubuntu-latest/rust_target:x86_64-unknown-linux-musl"
    )
    assert not runner.calls
    with pytest.raises(ToolingError, match="audited image matrix"):
        ImageScanCategory.run(configured(context, matrix_name="unknown"))
