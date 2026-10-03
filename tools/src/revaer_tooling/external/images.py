"""Typed Buildx and Trivy interfaces for local container tooling.

Docker retains ownership of builders, exporters, and image storage. Trivy reads
the versioned repository policy; rv preserves its report and failing exit status.
"""

from __future__ import annotations

import csv
import io
import re
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal

from ..errors import ToolingError
from ..images.model import (
    architecture,
    build_architecture,
    image_locator,
    image_tag,
    sha256_digest,
)
from ..images.validation import digest_reference
from ..process import Completed
from .base import ExternalTool


@dataclass(frozen=True)
class ImageBuildArgs:
    builder: str
    platforms: tuple[str, ...]
    name: str
    version: str
    metadata: Path
    archive: Path | None

    def validate(self) -> None:
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", self.builder):
            raise ToolingError("BUILDX_BUILDER must be a builder name")
        if not re.fullmatch(r"[a-z0-9]+(?:[._/-][a-z0-9]+)*", self.name):
            raise ToolingError("REVAER_LOCAL_IMAGE must be a lowercase image name")
        if not re.fullmatch(r"[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}", self.version):
            raise ToolingError("VERSION must be an OCI tag")
        if (
            not self.platforms
            or len(set(self.platforms)) != len(self.platforms)
            or any(
                not re.fullmatch(r"[a-z0-9_-]+/[a-z0-9_-]+(?:/[a-z0-9_-]+)?", platform)
                for platform in self.platforms
            )
        ):
            raise ToolingError("PLATFORMS must contain distinct OS/architecture[/variant] values")
        if self.archive is None and len(self.platforms) != 1:
            raise ToolingError("Multiple platforms require the OCI archive exporter")


class Buildx(ExternalTool):
    version_args = ("buildx", "version")

    def prepare(self, builder: str) -> None:
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", builder):
            raise ToolingError("BUILDX_BUILDER must be a builder name")
        # Listing distinguishes absence from an inspect/connection failure.
        # Explicit --builder preserves the caller's globally selected builder.
        names = self._invoke(("buildx", "ls", "--format", "{{.Name}}"), capture=True).stdout
        if builder not in names.splitlines():
            self._invoke(("buildx", "create", "--name", builder, "--driver", "docker-container"))

    def build(self, args: ImageBuildArgs) -> Completed:
        args.validate()
        self.prepare(args.builder)
        exporter = io.StringIO()
        csv.writer(exporter, lineterminator="").writerow(("type=oci", f"dest={args.archive}"))
        return self._invoke(
            (
                "buildx",
                "build",
                "--builder",
                args.builder,
                "--platform",
                ",".join(args.platforms),
                "--tag",
                f"{args.name}:latest",
                "--tag",
                f"{args.name}:{args.version}",
                "--metadata-file",
                str(args.metadata),
                # Docker parses output options as CSV, including quoted paths.
                *(("--output", exporter.getvalue()) if args.archive else ("--load",)),
                ".",
            )
        )

    def release_build(self, args: ReleaseBuildArgs) -> Completed:
        args.validate()
        self.prepare(args.builder)
        return self._invoke(
            (
                "buildx",
                "build",
                "--builder",
                args.builder,
                "--platform",
                args.platform,
                "--tag",
                args.tag,
                *(
                    ("--push", "--attest", "type=provenance,mode=max", "--attest", "type=sbom")
                    if args.publish
                    else ("--load",)
                ),
                "--metadata-file",
                str(args.metadata),
                "--build-arg",
                f"RUST_TARGET={args.rust_target}",
                "--build-arg",
                f"BUILD_DATE={args.invoked_at.astimezone(UTC):%Y-%m-%dT%H:%M:%SZ}",
                "--build-arg",
                f"VERSION={args.version}",
                "--build-arg",
                f"REVISION={args.revision}",
                ".",
            )
        )

    def resolve_digest(self, reference: str) -> str:
        image_locator(reference)
        result = self._invoke(
            ("buildx", "imagetools", "inspect", reference, "--format", "{{.Manifest.Digest}}"),
            capture=True,
        )
        return sha256_digest(result.stdout.strip())

    def create_manifest(self, args: ManifestArgs) -> Completed:
        args.validate()
        tags = tuple(part for tag in args.tags for part in ("--tag", tag))
        return self._invoke(("buildx", "imagetools", "create", *tags, *args.sources))


@dataclass(frozen=True)
class ReleaseBuildArgs:
    builder: str
    platform: str
    arch_tag: str
    rust_target: str
    tag: str
    version: str
    revision: str
    invoked_at: datetime
    metadata: Path
    publish: bool

    def validate(self) -> None:
        build_architecture(self.platform, self.arch_tag, self.rust_target)
        image_locator(self.tag)
        image_tag(self.version)
        if not re.fullmatch(r"[0-9a-f]{7,40}", self.revision):
            raise ToolingError("Image revision must identify the source commit")
        if self.invoked_at.tzinfo is None:
            raise ToolingError("Image build date must include its time zone")


@dataclass(frozen=True)
class ManifestArgs:
    tags: tuple[str, ...]
    sources: tuple[str, ...]

    def validate(self) -> None:
        if not self.tags or len(set(self.tags)) != len(self.tags):
            raise ToolingError("Manifest tags must be nonempty and distinct")
        if len(self.sources) != 2 or len(set(self.sources)) != 2:
            raise ToolingError("Manifest requires two distinct immutable architecture images")
        for tag in self.tags:
            image_locator(tag)
        for source in self.sources:
            digest_reference(source)


@dataclass(frozen=True)
class ImageScanArgs:
    reference: str
    report: Path
    ignore_file: Path


class Trivy(ExternalTool):
    def evidence(self, args: ImageEvidenceArgs) -> Completed:
        args.validate()
        return self._invoke(
            (
                "image",
                "--config",
                str(self.root / "trivy.yaml"),
                "--image-src",
                args.source,
                "--platform",
                args.platform,
                "--format",
                args.format,
                "--output",
                str(args.report),
                "--severity",
                "HIGH,CRITICAL",
                "--ignore-unfixed=false",
                "--ignorefile",
                str(args.ignore_file),
                # Findings stay in the report for the independent SARIF gate.
                # Transport, database and scanner failures still exit nonzero.
                "--exit-code",
                "0",
                args.reference,
            )
        )

    def image(self, args: ImageScanArgs) -> Completed:
        if (
            not args.reference
            or args.reference.startswith("-")
            or any(character.isspace() for character in args.reference)
        ):
            raise ToolingError("Image scan requires an image reference")
        return self._invoke(
            (
                "image",
                "--config",
                str(self.root / "trivy.yaml"),
                "--exit-code",
                "1",
                "--severity",
                "HIGH,CRITICAL",
                "--ignore-unfixed=false",
                "--ignorefile",
                str(args.ignore_file),
                "--format",
                "json",
                "--output",
                str(args.report),
                args.reference,
            )
        )


@dataclass(frozen=True)
class ImageEvidenceArgs:
    reference: str
    platform: str
    source: Literal["remote", "docker"]
    format: Literal["spdx-json", "sarif"]
    report: Path
    ignore_file: Path

    def validate(self) -> None:
        image_locator(self.reference)
        architecture(self.platform)
        if self.source not in ("remote", "docker") or self.format not in ("spdx-json", "sarif"):
            raise ToolingError("Unsupported image evidence source or format")
        if self.source == "remote":
            digest_reference(self.reference)
        if self.format == "spdx-json" and self.source != "remote":
            raise ToolingError("Package inventory requires a remote immutable image")
