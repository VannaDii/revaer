"""Reviewed image platforms and validated names shared by tasks and policy.

These are Revaer's two shipped Linux architectures. Changing this table changes
the build matrix contract as well as the CLI, and requires native validation.
"""

import re
from dataclasses import dataclass

from ..errors import ToolingError
from ..json_data import JsonObject
from .validation import digest_reference


@dataclass(frozen=True)
class Architecture:
    name: str
    runner: str
    platform: str
    rust_target: str


@dataclass(frozen=True)
class ManifestPlan:
    """Validated destination tags and source tags that still need resolution."""

    tags: tuple[str, ...]
    source_tags: tuple[str, ...]


ARCHITECTURES = (
    Architecture("amd64", "ubuntu-latest", "linux/amd64", "x86_64-unknown-linux-musl"),
    Architecture("arm64", "ubuntu-24.04-arm", "linux/arm64", "aarch64-unknown-linux-musl"),
)

IMAGE_MATRIX: JsonObject = {
    "include": [
        {
            "name": item.name,
            "runner": item.runner,
            "platform": item.platform,
            "rust_target": item.rust_target,
            "arch_tag": item.name,
            "needs_qemu": False,
        }
        for item in ARCHITECTURES
    ]
}


def architecture(platform: str) -> Architecture:
    for item in ARCHITECTURES:
        if item.platform == platform:
            return item
    raise ToolingError("PLATFORM must be linux/amd64 or linux/arm64")


def build_architecture(platform: str, arch_tag: str, rust_target: str) -> Architecture:
    item = architecture(platform)
    if item.name != arch_tag or item.rust_target != rust_target:
        raise ToolingError("Image platform, architecture tag and Rust target do not match")
    return item


def image_name(value: str) -> str:
    if not re.fullmatch(r"[a-z0-9]+(?:[._/-][a-z0-9]+)*", value):
        raise ToolingError("IMAGE_NAME must be a lowercase image name")
    return value


def image_tag(value: str) -> str:
    if not re.fullmatch(r"[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}", value):
        raise ToolingError("Image version must be an OCI tag")
    return value


def ghcr_reference(value: str) -> str:
    digest = digest_reference(value)
    if not re.fullmatch(r"ghcr\.io/[a-z0-9][a-z0-9._/-]*@sha256:[0-9a-f]{64}", value):
        raise ToolingError("IMAGE_REFERENCE must be a digest-qualified GHCR reference")
    return digest


def sha256_digest(value: str) -> str:
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", value):
        raise ToolingError("Image tool returned an invalid SHA-256 digest")
    return value


def image_locator(value: str) -> str:
    """Accept a registry or local image reference, never a command-line option."""
    if not re.fullmatch(r"[a-z0-9][a-zA-Z0-9._:/@-]*", value):
        raise ToolingError("Image operation requires a valid image reference")
    return value
