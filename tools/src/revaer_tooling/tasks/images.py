"""Local image tasks preserve single-platform loading and multi-platform export."""

import json
import re
import tempfile
from pathlib import Path

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.images import ImageBuildArgs, ImageScanArgs
from .base import Task


class DockerBuild(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        settings = context.settings.images
        version = settings.version or (
            f"dev.{context.invoked_at:%y%m%d}.{context.tools.git.revision()[:7]}"
        )
        output = context.root / "artifacts"
        metadata = output / "image-build.json"
        archive = (
            output / f"{settings.name.rsplit('/', 1)[-1]}-{version}.oci"
            if len(settings.platforms) > 1
            else None
        )
        args = ImageBuildArgs(
            settings.builder, settings.platforms, settings.name, version, metadata, archive
        )
        # Validate before constructing cleanup paths from an operator's version.
        args.validate()
        context.fs.mkdir(output)
        with context.fs.lock(output / ".image-build.lock"):
            context.fs.remove_owned(metadata, context.root)
            if archive:
                context.fs.remove_owned(archive, context.root)
            try:
                context.tools.buildx.build(args)
                result = json.loads(context.fs.read(metadata))
                digest = result.get("containerimage.digest") if isinstance(result, dict) else None
                if not isinstance(digest, str) or not re.fullmatch(r"sha256:[a-f0-9]{64}", digest):
                    raise ToolingError("Buildx did not retain a complete image digest")
                if archive and (not archive.is_file() or archive.stat().st_size == 0):
                    raise ToolingError("Buildx did not produce its OCI archive")
            except BaseException:
                # A partial tarball or stale metadata cannot be a completed build.
                context.fs.remove_owned(metadata, context.root)
                if archive:
                    context.fs.remove_owned(archive, context.root)
                raise
        return TaskResult(f"Image built: {settings.name}:{version} ({digest})")


class DockerScan(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        report = context.root / "artifacts/image-scan.json"
        context.fs.mkdir(report.parent)
        with context.fs.lock(report.parent / ".image-scan.lock"):
            context.fs.remove_owned(report, context.root)
            # No advisory-ignore file can silently change the gate. Trivy still
            # owns policy parsing and all configured scanners; findings retain
            # their JSON evidence even when its nonzero exit fails this task.
            with tempfile.TemporaryDirectory(prefix="image-scan-", dir=report.parent) as work:
                ignores = Path(work) / "empty.ignore"
                context.fs.write(ignores, "")
                context.tools.trivy.image(
                    ImageScanArgs(context.settings.images.scan_reference, report, ignores)
                )
            result = json.loads(context.fs.read(report))
            if not isinstance(result, dict) or not result.get("ArtifactName"):
                raise ToolingError("Trivy did not produce a complete image report")
        return TaskResult(f"Image scan passed: {report}")
