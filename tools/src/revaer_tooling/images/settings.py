"""Existing image workflow inputs, read only at the CLI boundary."""

import re
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from .model import image_name, image_tag


@dataclass(frozen=True)
class ImageReleaseSettings:
    name: str
    version: str
    owner: str
    alias: str
    include_sha: str
    platform: str
    arch_tag: str
    rust_target: str
    reference: str
    source: str
    report: Path | None
    predicate: Path | None
    attestation_output: Path | None
    repository: str
    matrix_name: str
    runner: str
    needs_qemu: str

    def base(self) -> str:
        image_name(self.name)
        image_tag(self.version)
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]*", self.owner):
            raise ToolingError("REPOSITORY_OWNER must be a registry namespace")
        if self.alias:
            image_tag(self.alias)
        if self.include_sha not in ("true", "false"):
            raise ToolingError("INCLUDE_SHA_TAG must be true or false")
        return f"ghcr.io/{self.owner.lower()}/{self.name}"

    def tags(self, source: str) -> tuple[str, ...]:
        base = self.base()
        values = [self.version]
        if self.alias:
            values.append(self.alias)
        if self.include_sha == "true":
            values.append(image_tag(source))
        # An alias equal to the version needs only one registry operation.
        return tuple(f"{base}:{value}" for value in dict.fromkeys(values))


@dataclass(frozen=True)
class ContainerSettings:
    builder_image: str
    runtime_image: str
    uv_image: str
    rust_version: str
    alpine_version: str
    target_arch: str
    rust_target: str


def load_container_settings(environment: Mapping[str, str]) -> ContainerSettings:
    return ContainerSettings(
        environment.get("RUST_BUILDER_IMAGE", ""),
        environment.get("ALPINE_RUNTIME_IMAGE", ""),
        environment.get("UV_IMAGE", ""),
        environment.get("RUST_VERSION", ""),
        environment.get("ALPINE_VERSION", ""),
        environment.get("TARGETARCH", ""),
        environment.get("RUST_TARGET", ""),
    )


def load_image_release_settings(environment: Mapping[str, str]) -> ImageReleaseSettings:
    def path(name: str) -> Path | None:
        value = environment.get(name)
        return Path(value) if value else None

    return ImageReleaseSettings(
        name=environment.get("IMAGE_NAME", ""),
        version=environment.get("VERSION_TAG", ""),
        owner=environment.get("REPOSITORY_OWNER", ""),
        alias=environment.get("ALIAS_TAG", ""),
        include_sha=environment.get("INCLUDE_SHA_TAG") or "false",
        platform=environment.get("PLATFORM", ""),
        arch_tag=environment.get("ARCH_TAG", ""),
        rust_target=environment.get("RUST_TARGET", ""),
        reference=environment.get("IMAGE_REFERENCE", ""),
        source=environment.get("IMAGE_SOURCE", ""),
        report=path("TRIVY_OUTPUT_PATH"),
        predicate=path("COMPLIANCE_PREDICATE"),
        attestation_output=path("ATTESTATION_OUTPUT"),
        repository=environment.get("GITHUB_REPOSITORY", ""),
        matrix_name=environment.get("MATRIX_NAME", ""),
        runner=environment.get("RUNNER", ""),
        needs_qemu=environment.get("NEEDS_QEMU", ""),
    )
