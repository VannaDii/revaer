"""Image workflow stages with independent build, scan and publication gates.

Tasks retain failed reports and publish completion outputs only after validation.
The workflow controls publication eligibility; external tools own registry and
Sigstore protocols. A mutable tag is resolved before any signature is requested.
"""

import json
import tempfile
from pathlib import Path
from typing import Literal

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.cosign import AttestArgs, VerifyAttestationArgs
from ..external.images import ImageEvidenceArgs, ManifestArgs, ReleaseBuildArgs
from ..images.compliance import validate_bundle
from ..images.model import (
    ARCHITECTURES,
    ManifestPlan,
    build_architecture,
    ghcr_reference,
    image_name,
    image_tag,
    sha256_digest,
)
from ..images.validation import read_document
from ..json_data import string_value
from .automation import outputs, source_values
from .base import Task


def image_output(context: Context, path: Path) -> Path:
    """Reject source/metadata destinations before removing stale evidence."""
    result = context.root / path
    try:
        relative = result.relative_to(context.root)
    except ValueError as error:
        raise ToolingError("Image evidence must belong to the current checkout") from error
    if (
        not relative.parts
        or any(part.startswith(".") for part in relative.parts)
        or any((context.root / parent).is_symlink() for parent in relative.parents)
        or result.is_symlink()
        or not result.resolve().is_relative_to(context.root.resolve())
        or (result.exists() and not result.is_file())
    ):
        raise ToolingError("Image evidence must use a regular owned path without links")
    if relative.as_posix() in context.tools.git.files():
        raise ToolingError("Image evidence must not replace tracked source")
    return result


def build_image(context: Context, *, publish: bool) -> TaskResult:
    settings = context.settings.image_release
    item = build_architecture(settings.platform, settings.arch_tag, settings.rust_target)
    image_name(settings.name)
    image_tag(settings.version)
    base = settings.base() if publish else settings.name
    revision = source_values(context)["short_sha"]
    tag = f"{base}:{revision}-{item.name}" if publish else f"{base}:verify-{item.name}"
    metadata = image_output(context, Path(f"artifacts/image-build-{item.name}.json"))
    context.fs.mkdir(metadata.parent)
    with context.fs.lock(metadata.with_suffix(".lock")):
        context.fs.remove_owned(metadata, context.root)
        context.tools.buildx.release_build(
            ReleaseBuildArgs(
                context.settings.images.builder,
                item.platform,
                item.name,
                item.rust_target,
                tag,
                settings.version,
                revision,
                context.invoked_at,
                metadata,
                publish,
            )
        )
        record = read_document(context.fs, metadata)
        digest = sha256_digest(string_value(record.get("containerimage.digest")))
        values = {"image_tag": tag}
        if publish:
            resolved = context.tools.buildx.resolve_digest(tag)
            if digest != resolved:
                raise ToolingError("Build metadata digest does not match the registry digest")
            values.update(image_digest=resolved, image_reference=f"{base}@{resolved}")
        return outputs(context, values)


class ImageBuildPush(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return build_image(context, publish=True)


class ImageBuildVerify(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return build_image(context, publish=False)


def image_evidence(context: Context, format: Literal["spdx-json", "sarif"]) -> TaskResult:
    settings = context.settings.image_release
    if settings.source not in ("remote", "docker"):
        raise ToolingError("IMAGE_SOURCE must be remote or docker")
    if settings.source == "remote":
        ghcr_reference(settings.reference)
    source: Literal["remote", "docker"] = "remote" if settings.source == "remote" else "docker"
    default = (
        "image-package-inventory.spdx.json" if format == "spdx-json" else "trivy-results.sarif"
    )
    report = image_output(context, settings.report or Path(default))
    context.fs.mkdir(report.parent)
    with context.fs.lock(report.with_suffix(report.suffix + ".lock")):
        with tempfile.TemporaryDirectory(prefix=".rv-image-scan-", dir=report.parent) as work:
            ignores = Path(work) / "empty.ignore"
            context.fs.write(ignores, "")
            args = ImageEvidenceArgs(
                settings.reference, settings.platform, source, format, report, ignores
            )
            args.validate()
            context.fs.remove_owned(report, context.root)
            # Never remove the current attempt's report on failure: the workflow
            # uploads SARIF under always(), before the next publication stage.
            context.tools.trivy.evidence(args)
        read_document(context.fs, report)
    return TaskResult(f"Image {format} evidence retained: {report}")


class ImageInventory(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return image_evidence(context, "spdx-json")


class ImageScan(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        return image_evidence(context, "sarif")


class ImageSignAttest(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        settings = context.settings.image_release
        ghcr_reference(settings.reference)
        if settings.predicate is None:
            raise ToolingError("COMPLIANCE_PREDICATE must select the image's validated bundle")
        predicate = context.root / settings.predicate
        # Direct CLI calls must satisfy the same bundle gate as workflow calls.
        # This check completes before either irreversible registry operation.
        validate_bundle(context.fs, predicate, settings.reference)
        context.tools.cosign.sign(settings.reference)
        context.tools.cosign.attest(AttestArgs(settings.reference, predicate))
        return TaskResult(f"Signed image and compliance evidence: {settings.reference}")


class ImageAttestationVerify(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        settings = context.settings.image_release
        args = VerifyAttestationArgs(settings.reference, settings.repository)
        args.identity()
        if settings.attestation_output is None:
            raise ToolingError("ATTESTATION_OUTPUT must select the verified evidence file")
        output = image_output(context, settings.attestation_output)
        context.fs.mkdir(output.parent)
        with context.fs.lock(output.with_suffix(".lock")):
            context.fs.remove_owned(output, context.root)
            result = context.tools.cosign.verify_attestation(args)
            if not result.stdout.strip():
                raise ToolingError("Cosign did not return verified attestation evidence")
            # Cosign versions emit either a JSON array or successive envelopes.
            # Validate the entire stream while retaining its original bytes.
            decoder = json.JSONDecoder()
            remaining = result.stdout.strip()
            while remaining:
                value, end = decoder.raw_decode(remaining)
                if not value or not isinstance(value, (dict, list)):
                    raise ToolingError("Cosign returned invalid attestation evidence")
                remaining = remaining[end:].lstrip()
            context.fs.write(output, result.stdout)
        return TaskResult(f"Verified image attestation: {output}")


def manifest_inputs(context: Context) -> ManifestPlan:
    settings = context.settings.image_release
    base = settings.base()
    revision = source_values(context)["short_sha"]
    return ManifestPlan(
        settings.tags(revision),
        tuple(f"{base}:{revision}-{item.name}" for item in ARCHITECTURES),
    )


def common_digest(context: Context, tags: tuple[str, ...]) -> str:
    digests = {context.tools.buildx.resolve_digest(tag) for tag in tags}
    if len(digests) != 1:
        raise ToolingError("Manifest tags do not identify the same image digest")
    return digests.pop()


class ImageManifestCreate(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        planned = manifest_inputs(context)
        base = context.settings.image_release.base()
        # Resolve both architecture tags first. Changes to a mutable source tag
        # during publication cannot alter the supplied manifest inputs.
        sources = tuple(
            f"{base}@{context.tools.buildx.resolve_digest(tag)}" for tag in planned.source_tags
        )
        context.tools.buildx.create_manifest(ManifestArgs(planned.tags, sources))
        digest = common_digest(context, planned.tags)
        return outputs(context, {"image_digest": digest, "image_reference": f"{base}@{digest}"})


class ImageManifestVerify(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        # Verification-only PR jobs load images locally on separate runners.
        # Their manifests have no registry sources to inspect: preserve the
        # existing input check and describe exactly what it establishes.
        planned = manifest_inputs(context)
        return TaskResult(
            "Verified manifest inputs without publishing:\n"
            + "\n".join(f"  {value}" for value in (*planned.tags, *planned.source_tags))
        )


class ImageManifestSign(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        planned = manifest_inputs(context)
        digest = common_digest(context, planned.tags)
        reference = f"{context.settings.image_release.base()}@{digest}"
        context.tools.cosign.sign(reference)
        return TaskResult(f"Signed multi-architecture manifest: {reference}")


class ImageScanCategory(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        settings = context.settings.image_release
        item = build_architecture(settings.platform, settings.arch_tag, settings.rust_target)
        if (
            settings.matrix_name != item.name
            or settings.runner != item.runner
            or settings.needs_qemu
        ):
            raise ToolingError("Code scanning identity must match the audited image matrix")
        # Keep GitHub's existing analysis identity across the workflow refactor.
        category = (
            f".github/workflows/ci.yml:build-images/arch_tag:{item.name}/name:{item.name}"
            f"/needs_qemu:/platform:{item.platform}/runner:{item.runner}"
            f"/rust_target:{item.rust_target}"
        )
        return outputs(context, {"value": category})
