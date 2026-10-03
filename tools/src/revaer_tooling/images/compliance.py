"""Generate and verify the existing digest-bound compliance bundle format.

All five accompanying artifacts are hashed and retained. SPDX and source-offer
checks stay independent of hashes: hashing incomplete evidence cannot make it
complete. File paths are resolved within the bundle without following links.
"""

import json
import re
import tempfile
from datetime import UTC
from pathlib import Path

from ..artifacts import digest_file
from ..context import Context
from ..errors import ToolingError
from ..filesystem import FileSystem
from ..json_data import Json, JsonObject, array_value, object_value, string_value
from .validation import (
    PREDICATE_TYPE,
    digest_reference,
    read_document,
    require_evidence_string,
    require_sha256,
    validate_inventory,
)

BUNDLE = "final-image-compliance-bundle.json"
ARTIFACTS = {
    "spdx": "declared-runtime-inventory.spdx.json",
    "build_inputs": "build-inputs.env",
    "package_inventory": "image-package-inventory.spdx.json",
    "source_compliance": "source-compliance.json",
    "third_party_notices": "THIRD-PARTY-NOTICES.md",
}


def artifact_path(root: Path, name: str) -> Path:
    path = Path(name)
    if path.is_absolute() or ".." in name or "\\" in name:
        raise ToolingError("Compliance artifacts must be relative paths within the bundle")
    result = root / path
    if (
        result.is_symlink()
        or any((root / parent).is_symlink() for parent in path.parents)
        or not result.resolve().is_relative_to(root.resolve())
        or not result.is_file()
        or not result.stat().st_size
    ):
        raise ToolingError(
            f"Compliance artifact is missing, empty, linked or outside its bundle: {name}"
        )
    return result


def validate_bundle(fs: FileSystem, path: Path, expected_reference: str) -> None:
    digest = digest_reference(expected_reference)
    record = read_document(fs, path)
    for key, expected in (
        ("schema_version", "revaer.final-image-compliance.v2"),
        ("predicate_type", PREDICATE_TYPE),
        ("image_reference", expected_reference),
        ("image_digest", digest),
        ("package_inventory_image_reference", expected_reference),
        ("release_gate", "passed"),
    ):
        if record.get(key) != expected:
            raise ToolingError(f"Compliance bundle has an unexpected {key}")
    for key in ("revision", "source_offer_url", "generated_at"):
        string_value(record.get(key))
    verified: dict[str, Path] = {}
    for key in ARTIFACTS:
        artifact = artifact_path(path.parent, string_value(record.get(f"{key}_path")))
        expected_hash = string_value(record.get(f"{key}_sha256"))
        if (
            not re.fullmatch(r"[0-9a-f]{64}", expected_hash)
            or digest_file(artifact) != expected_hash
        ):
            raise ToolingError(f"Compliance artifact hash does not match: {key}")
        verified[key] = artifact
    validate_inventory(read_document(fs, verified["spdx"]))
    inventory = read_document(fs, verified["package_inventory"])
    validate_inventory(inventory)
    if inventory.get("comment") != "Inventory generated from " + expected_reference:
        raise ToolingError("Package inventory does not identify the exact built image")
    source = read_document(fs, verified["source_compliance"])
    if source.get("image_reference") != expected_reference or source.get("image_digest") != digest:
        raise ToolingError("Source compliance is bound to another image")
    entries = array_value(source.get("entries"))
    if not entries:
        raise ToolingError("Source compliance must include package entries")
    for value in entries:
        entry = object_value(value)
        require_evidence_string(entry.get("source_url"), "source_url")
        require_sha256(entry.get("checksums"))


def generate_bundle(context: Context, reference: str, inventory_path: Path, output: Path) -> None:
    digest = digest_reference(reference)
    inventory = read_document(context.fs, inventory_path)
    validate_inventory(inventory)
    declared_path = context.root / "release/media-compliance/media-runtime-inventory.spdx.json"
    packages = validate_inventory(read_document(context.fs, declared_path))
    if (
        output.is_symlink()
        or output.resolve() == context.root.resolve()
        or not output.resolve().is_relative_to(context.root.resolve())
    ):
        raise ToolingError("Compliance output must be an owned directory within the checkout")
    relative_path = output.relative_to(context.root)
    if ".." in relative_path.parts:
        raise ToolingError("Compliance output must use a path without parent traversal")
    relative = relative_path.as_posix()
    if any(
        name == relative or name.startswith(relative + "/") for name in context.tools.git.files()
    ):
        raise ToolingError("Compliance generation must not replace tracked source")
    if output.exists() and (
        not output.is_dir()
        or {path.name for path in output.iterdir()} - {BUNDLE, *ARTIFACTS.values()}
    ):
        raise ToolingError("Compliance output contains unrelated files")
    context.fs.mkdir(output.parent)
    # Same-filesystem staging preserves prior evidence if copying or validation
    # fails. No completion manifest is exposed while its artifacts are partial.
    with tempfile.TemporaryDirectory(prefix=".rv-compliance-", dir=output.parent) as temporary:
        stage = Path(temporary)
        inventory["comment"] = "Inventory generated from " + reference
        context.fs.write(
            stage / ARTIFACTS["package_inventory"], json.dumps(inventory, indent=2) + "\n"
        )
        for key, source in (
            ("spdx", declared_path),
            ("build_inputs", context.root / ".github/build-inputs.env"),
            (
                "third_party_notices",
                context.root / "release/media-compliance/THIRD-PARTY-NOTICES.md",
            ),
        ):
            context.fs.copy(source, stage / ARTIFACTS[key])
        entries: list[Json] = [
            {
                "name": package["name"],
                "version": package["versionInfo"],
                "source_url": package["downloadLocation"],
                "checksums": package["checksums"],
            }
            for package in packages
        ]
        source_record: JsonObject = {
            "schema_version": "revaer.source-compliance.v1",
            "image_reference": reference,
            "image_digest": digest,
            "entries": entries,
        }
        context.fs.write(
            stage / ARTIFACTS["source_compliance"], json.dumps(source_record, indent=2) + "\n"
        )
        revision = context.tools.git.revision()
        record: JsonObject = {
            "schema_version": "revaer.final-image-compliance.v2",
            "predicate_type": PREDICATE_TYPE,
            "image_reference": reference,
            "image_digest": digest,
            "package_inventory_image_reference": reference,
            "revision": revision,
            "generated_at": context.invoked_at.astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "source_offer_url": f"https://github.com/VannaDii/revaer/tree/{revision}/release/media-compliance",
            "release_gate": "passed",
        }
        for key, name in ARTIFACTS.items():
            record[f"{key}_path"] = name
            record[f"{key}_sha256"] = digest_file(stage / name)
        context.fs.write(stage / BUNDLE, json.dumps(record, indent=2) + "\n")
        validate_bundle(context.fs, stage / BUNDLE, reference)
        context.fs.remove_owned(output, context.root)
        stage.replace(output)
