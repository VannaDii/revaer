"""Retain runtime package, source-offer and image pin guardrails in Python."""

from ..context import Context
from .inputs import BuildInputs, verify_project_pins
from .validation import read_document, validate_inventory

RUNTIME_PACKAGES = frozenset(
    (
        "bento4",
        "ca-certificates",
        "curl",
        "exiftool",
        "ffmpeg",
        "ffplay",
        "font-dejavu",
        "fontconfig",
        "gnutls",
        "libass",
        "libdav1d",
        "libstdc++",
        "libtheora",
        "libtorrent-rasterbar",
        "libvorbis",
        "mediainfo",
        "mkvtoolnix",
        "openssl",
        "opus",
        "x264-libs",
        "x265-libs",
    )
)
LABELS = (
    "license_mode",
    "source_offer",
    "third_party_notices",
    "sbom",
    "inventory",
    "exiftool_exception",
    "build_inputs",
    "compliance_attestation",
)
EVIDENCE = (
    "SOURCE-OFFER.txt",
    "THIRD-PARTY-NOTICES.md",
    "media-runtime-inventory.spdx.json",
    "exiftool-exception.md",
)


def media_compliance_findings(context: Context) -> list[str]:
    inputs = BuildInputs.load(context.fs, context.root)
    verify_project_pins(context.fs, context.root, inputs)
    dockerfile = context.fs.read(context.root / "Dockerfile").splitlines()
    failures = []
    for line in (
        f"# syntax={inputs.frontend}",
        f"ARG RUST_BUILDER_IMAGE={inputs.builder}",
        f"ARG ALPINE_RUNTIME_IMAGE={inputs.runtime}",
        f"ARG UV_IMAGE={inputs.uv_image}",
        f"ARG RUST_VERSION={inputs.rust_version}",
        f"ARG ALPINE_VERSION={inputs.alpine_version}",
    ):
        if dockerfile.count(line) != 1:
            failures.append("Dockerfile must contain the exact reviewed pin: " + line)
    for label in LABELS:
        if not any(line.startswith(f"LABEL revaer.media.{label}=") for line in dockerfile):
            failures.append("Dockerfile is missing the media label: " + label)
    declared = context.root / "release/media-compliance"
    for name in EVIDENCE:
        path = declared / name
        if path.is_symlink() or not path.is_file() or not path.stat().st_size:
            failures.append("Missing, empty or linked media compliance evidence: " + name)
    inventory = validate_inventory(
        read_document(context.fs, declared / "media-runtime-inventory.spdx.json")
    )
    names = [package["name"] for package in inventory]
    if len(names) != len(set(names)) or "alpine-media-runtime-packages" in names:
        failures.append("Runtime inventory must identify individual packages without duplicates")
    pins = dict(pin.split("=", 1) for pin in inputs.runtime_packages)
    if missing := RUNTIME_PACKAGES - pins.keys():
        failures.append(
            "Runtime manifest is missing required packages: " + ", ".join(sorted(missing))
        )
    for name, version in pins.items():
        if not any(
            package["name"] == name and package["versionInfo"] == version for package in inventory
        ):
            failures.append(f"Runtime inventory must match the exact {name}={version} pin")
    for name in context.tools.git.files(include_untracked=True):
        if name in (
            "tools/src/revaer_tooling/images/policy.py",
            "scripts/media-compliance-guardrails.sh",
        ):
            continue  # Each guard contains its own forbidden-token definition.
        production = name == "Dockerfile" or (
            name.startswith(("scripts/", "release/", "tools/src/"))
            and name.endswith((".sh", ".js", ".py"))
        )
        if production and "--enable-nonfree" in context.fs.read(context.root / name):
            failures.append("Default media runtime must not enable nonfree components: " + name)
    return failures
