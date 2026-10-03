"""Validate retained scanner and package evidence without invoking a shell."""

import re
from pathlib import Path

from ..errors import ToolingError
from ..filesystem import FileSystem
from ..json_data import Json, JsonObject, array_value, decode, object_value, string_value

PREDICATE_TYPE = "https://revaer.com/attestations/final-image-compliance/v2"


def digest_reference(value: str) -> str:
    if not re.fullmatch(r"[a-z0-9][a-z0-9._:/-]*@sha256:[0-9a-f]{64}", value):
        raise ToolingError("Image reference must contain an immutable lowercase SHA-256 digest")
    return value.rsplit("@", 1)[1]


def read_document(fs: FileSystem, path: Path) -> JsonObject:
    if path.is_symlink() or not path.is_file() or not path.stat().st_size:
        raise ToolingError(f"Evidence must be a nonempty regular file: {path}")
    return object_value(decode(fs.read(path)))


def verify_sarif(record: JsonObject) -> None:
    """Trivy is configured for HIGH/CRITICAL; every reported result blocks.

    Validate the whole report before counting results. An absent results array
    or a different scanner is not equivalent to a clean scan. Keep the original
    file available to Actions even when this independent gate fails.
    """
    if record.get("version") != "2.1.0":
        raise ToolingError("Trivy report must use SARIF 2.1.0")
    runs = array_value(record.get("runs"))
    if not runs:
        raise ToolingError("Trivy SARIF must contain at least one run")
    findings = []
    for value in runs:
        run = object_value(value)
        driver = object_value(object_value(run.get("tool")).get("driver"))
        if driver.get("name") != "Trivy":
            raise ToolingError("SARIF must identify the Trivy scanner")
        for result in array_value(run.get("results")):
            result = object_value(result)
            findings.append(str(result.get("ruleId") or "unknown"))
    if findings:
        raise ToolingError(
            f"{len(findings)} HIGH or CRITICAL finding(s) block verification and publication: "
            + ", ".join(findings)
        )


def require_evidence_string(value: Json, label: str) -> str:
    result = string_value(value)
    if result == "NOASSERTION":
        raise ToolingError(f"{label} must contain asserted package evidence")
    return result


def require_sha256(value: Json) -> None:
    checksums = array_value(value)
    for value in checksums:
        checksum = object_value(value)
        if str(checksum.get("algorithm", "")).lower() == "sha256" and re.fullmatch(
            r"[0-9a-f]{64}", str(checksum.get("checksumValue", ""))
        ):
            return
    raise ToolingError("Package evidence must contain a lowercase SHA-256 checksum")


def validate_inventory(record: JsonObject) -> tuple[JsonObject, ...]:
    if record.get("spdxVersion") != "SPDX-2.3":
        raise ToolingError("Package inventory must use SPDX 2.3")
    packages = tuple(object_value(value) for value in array_value(record.get("packages")))
    if not packages:
        raise ToolingError("Package inventory must contain at least one package")
    for package in packages:
        for name in (
            "name",
            "versionInfo",
            "downloadLocation",
            "licenseConcluded",
            "licenseDeclared",
        ):
            require_evidence_string(package.get(name), name)
        require_sha256(package.get("checksums"))
    return packages
