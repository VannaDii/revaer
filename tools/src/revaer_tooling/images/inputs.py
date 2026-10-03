"""Read the reviewed literal build manifest without executing shell content.

This file is a fixed set of literal assignments, not a general dotenv file.
Shell expansion, duplicate keys and unknown fields fail. Runtime dotenv loading
elsewhere remains uv's responsibility.
"""

from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

from ..assignments import literal_assignments
from ..database.contract import PostgresPin
from ..errors import ToolingError
from ..external.packages import ApkInstallArgs
from ..filesystem import FileSystem
from .validation import digest_reference

FIELDS = frozenset(
    (
        "RUST_VERSION",
        "UV_VERSION",
        "PYTHON_VERSION",
        "DOCKERFILE_FRONTEND_IMAGE",
        "RUST_BUILDER_IMAGE",
        "ALPINE_RUNTIME_IMAGE",
        "UV_IMAGE",
        "ALPINE_VERSION",
        "ALPINE_BUILDER_PACKAGES",
        "ALPINE_RUNTIME_PACKAGES",
    )
)
PROOF_FIELDS = frozenset(("POSTGRES_REBASELINE_IMAGE", "POSTGRES_REBASELINE_VERSION"))
NATIVE_HASH_FIELDS = (
    "POSTGRES_NATIVE_APK_METADATA_SHA256",
    "POSTGRES_NATIVE_GDB_SHA256",
    "POSTGRES_NATIVE_MUSL_SHA256",
    "POSTGRES_NATIVE_MUSL_DEBUG_SHA256",
    "POSTGRES_NATIVE_POSTGRES_SHA256",
    "POSTGRES_NATIVE_MUSL_SOURCE_SHA256",
    "POSTGRES_NATIVE_MUSL_SYSCALL_CP_SHA256",
)
NATIVE_PROOF_FIELDS = frozenset(
    (
        *NATIVE_HASH_FIELDS,
        "POSTGRES_NATIVE_DEBUGGER_IMAGE",
        "POSTGRES_NATIVE_ALPINE_VERSION",
        "POSTGRES_NATIVE_APK_PACKAGES",
        "POSTGRES_NATIVE_MUSL_SOURCE_URL",
    )
)


def validate_native_proof(values: dict[str, str]) -> None:
    """Validate the optional, complete native debugger block in the shared manifest.

    Container builds do not install these tools. They still reject partial or
    unpinned debugger declarations instead of silently accepting unrelated keys.
    """
    missing = NATIVE_PROOF_FIELDS - values.keys()
    if missing:
        raise ToolingError("Missing native PostgreSQL inputs: " + ", ".join(sorted(missing)))
    for name in NATIVE_HASH_FIELDS:
        if not re.fullmatch(r"[a-f0-9]{64}", values[name]):
            raise ToolingError(f"{name} must pin a SHA-256 digest")
    if not re.fullmatch(r"sha256:[a-f0-9]{64}", values["POSTGRES_NATIVE_DEBUGGER_IMAGE"]):
        raise ToolingError("POSTGRES_NATIVE_DEBUGGER_IMAGE must pin an image ID")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", values["POSTGRES_NATIVE_ALPINE_VERSION"]):
        raise ToolingError("POSTGRES_NATIVE_ALPINE_VERSION must pin an exact release")
    ApkInstallArgs(tuple(values["POSTGRES_NATIVE_APK_PACKAGES"].split())).validate()
    try:
        source = urlsplit(values["POSTGRES_NATIVE_MUSL_SOURCE_URL"])
        valid = (
            source.scheme == "https"
            and source.hostname
            and source.path
            and (source.port is None or source.port > 0)
            and not (source.username or source.password or source.fragment or source.query)
        )
    except ValueError as error:
        raise ToolingError("Invalid native musl source URL") from error
    if not valid:
        raise ToolingError("Native musl source must be an HTTPS URL without credentials or query")


@dataclass(frozen=True)
class BuildInputs:
    rust_version: str
    uv_version: str
    python_version: str
    frontend: str
    builder: str
    runtime: str
    uv_image: str
    alpine_version: str
    builder_packages: tuple[str, ...]
    runtime_packages: tuple[str, ...]

    @staticmethod
    def load(fs: FileSystem, root: Path) -> BuildInputs:
        values = literal_assignments(fs.read(root / ".github/build-inputs.env"), "Build input")
        unknown = values.keys() - FIELDS - PROOF_FIELDS - NATIVE_PROOF_FIELDS
        if unknown:
            raise ToolingError("Unknown build inputs: " + ", ".join(sorted(unknown)))
        if values.keys() & NATIVE_PROOF_FIELDS:
            validate_native_proof(values)
        if values.keys() & PROOF_FIELDS:
            # The shared manifest also carries the database proof's pins. Its
            # typed owner validates them; image builds consume only their fields.
            PostgresPin.load(values)
        if values.keys() & FIELDS != FIELDS:
            raise ToolingError(
                "Build manifest is missing inputs: " + ", ".join(sorted(FIELDS - values.keys()))
            )
        for name in ("RUST_VERSION", "UV_VERSION", "PYTHON_VERSION", "ALPINE_VERSION"):
            if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", values[name]):
                raise ToolingError(f"{name} must pin an exact release")
        for name in (
            "DOCKERFILE_FRONTEND_IMAGE",
            "RUST_BUILDER_IMAGE",
            "ALPINE_RUNTIME_IMAGE",
            "UV_IMAGE",
        ):
            digest_reference(values[name])
        builder = tuple(values["ALPINE_BUILDER_PACKAGES"].split())
        runtime = tuple(values["ALPINE_RUNTIME_PACKAGES"].split())
        ApkInstallArgs(builder).validate()
        ApkInstallArgs(runtime).validate()
        return BuildInputs(
            values["RUST_VERSION"],
            values["UV_VERSION"],
            values["PYTHON_VERSION"],
            values["DOCKERFILE_FRONTEND_IMAGE"],
            values["RUST_BUILDER_IMAGE"],
            values["ALPINE_RUNTIME_IMAGE"],
            values["UV_IMAGE"],
            values["ALPINE_VERSION"],
            builder,
            runtime,
        )


def verify_project_pins(fs: FileSystem, root: Path, inputs: BuildInputs) -> None:
    try:
        project = tomllib.loads(fs.read(root / "pyproject.toml"))
        rust = tomllib.loads(fs.read(root / "rust-toolchain.toml"))
        uv_version = project["tool"]["uv"]["required-version"]
        rust_version = rust["toolchain"]["channel"]
    except (tomllib.TOMLDecodeError, KeyError, TypeError) as error:
        raise ToolingError("Project metadata is missing valid uv/Rust pins") from error
    if (
        uv_version != "==" + inputs.uv_version
        or fs.read(root / ".uv-version").strip() != inputs.uv_version
        or fs.read(root / ".python-version").strip() != inputs.python_version
        or rust_version != inputs.rust_version
    ):
        raise ToolingError("Build manifest must match the project's uv, Python and Rust pins")
