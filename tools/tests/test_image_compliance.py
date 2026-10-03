"""Mutate real bundle files and retain the existing fail-closed evidence gates.

The repository's declared inventory is fixture input, not a claim that an image
was scanned. Separate native Trivy tests exercise actual image findings.
"""

import json
import shutil
import subprocess
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.artifacts import digest_file
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.images.compliance import ARTIFACTS, BUNDLE
from revaer_tooling.images.validation import read_document
from revaer_tooling.json_data import Json, JsonObject, array_value, object_value
from revaer_tooling.tasks.compliance import (
    ImageComplianceGenerate,
    ImageComplianceValidate,
    TrivySarifVerify,
)

REFERENCE = "ghcr.io/vannadii/revaer@sha256:" + "a" * 64


@pytest.fixture
def context(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    source = Path(__file__).parents[2]
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    shutil.copytree(source / "release/media-compliance", tmp_path / "release/media-compliance")
    shutil.copytree(source / "scripts/tests/fixtures/trivy", tmp_path / "reports")
    (tmp_path / ".github").mkdir()
    shutil.copy(source / ".github/build-inputs.env", tmp_path / ".github/build-inputs.env")
    for arguments in (
        ("init", "--quiet"),
        ("add", "."),
        (
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "Compliance fixture",
        ),
    ):
        subprocess.run(
            ["git", *arguments], cwd=tmp_path, capture_output=True, check=True, timeout=20
        )
    monkeypatch.chdir(tmp_path)
    return make_context(
        Options(
            image_reference=REFERENCE,
            inventory=Path("release/media-compliance/media-runtime-inventory.spdx.json"),
            output=Path("image-compliance"),
            bundle=Path("image-compliance") / BUNDLE,
        )
    )


@pytest.fixture
def bundle(context: Context) -> Context:
    ImageComplianceGenerate.run(context)
    ImageComplianceValidate.run(context)
    return context


def manifest(context: Context) -> JsonObject:
    return read_document(context.fs, context.root / "image-compliance" / BUNDLE)


def write_manifest(context: Context, record: JsonObject) -> None:
    (context.root / "image-compliance" / BUNDLE).write_text(json.dumps(record))


def rehash(context: Context, key: str) -> None:
    record = manifest(context)
    record[f"{key}_sha256"] = digest_file(context.root / "image-compliance" / ARTIFACTS[key])
    write_manifest(context, record)


def test_generation_retains_all_declared_evidence_and_actual_source(bundle: Context) -> None:
    record = manifest(bundle)
    output = bundle.root / "image-compliance"
    assert record["revision"] == bundle.tools.git.revision()
    assert {path.name for path in output.iterdir()} == {BUNDLE, *ARTIFACTS.values()}
    before = {path.name: path.read_bytes() for path in output.iterdir()}
    ImageComplianceGenerate.run(bundle)
    assert before == {path.name: path.read_bytes() for path in output.iterdir()}
    assert read_document(bundle.fs, output / ARTIFACTS["spdx"]) == read_document(
        bundle.fs, bundle.root / "release/media-compliance/media-runtime-inventory.spdx.json"
    )


@pytest.mark.parametrize(
    ("key", "value"),
    (
        ("revision", ""),
        ("generated_at", ""),
        ("source_offer_url", ""),
        ("schema_version", "wrong"),
        ("predicate_type", "wrong"),
        ("release_gate", "failed"),
        ("image_reference", "wrong"),
        ("image_digest", "sha256:" + "b" * 64),
        ("package_inventory_image_reference", "wrong"),
        ("spdx_path", "/outside"),
        ("spdx_path", "../outside"),
        ("spdx_path", "..\\outside"),
        ("spdx_sha256", "invalid"),
        ("spdx_sha256", "b" * 64),
    ),
)
def test_manifest_changes_cannot_pass_existing_evidence(
    bundle: Context, key: str, value: Json
) -> None:
    record = manifest(bundle)
    record[key] = value
    write_manifest(bundle, record)
    with pytest.raises(ToolingError):
        ImageComplianceValidate.run(bundle)


@pytest.mark.parametrize("key", tuple(ARTIFACTS))
def test_every_artifact_is_required_and_hash_checked(bundle: Context, key: str) -> None:
    artifact = bundle.root / "image-compliance" / ARTIFACTS[key]
    original = artifact.read_bytes()
    artifact.write_bytes(original + b"tamper\n")
    with pytest.raises(ToolingError, match="hash does not match"):
        ImageComplianceValidate.run(bundle)
    artifact.unlink()
    with pytest.raises(ToolingError, match="missing"):
        ImageComplianceValidate.run(bundle)


@pytest.mark.parametrize(
    ("artifact", "key", "value"),
    (
        ("spdx", "spdxVersion", "SPDX-2.2"),
        ("spdx", "packages", []),
        ("package_inventory", "comment", "another image"),
        ("package_inventory", "packages", []),
        ("source_compliance", "image_reference", "wrong"),
        ("source_compliance", "image_digest", "wrong"),
        ("source_compliance", "entries", []),
        ("source_compliance", "entries", [{"source_url": "NOASSERTION", "checksums": []}]),
        (
            "source_compliance",
            "entries",
            [{"source_url": "https://example.invalid", "checksums": []}],
        ),
    ),
)
def test_rehashing_incomplete_or_unbound_artifacts_does_not_make_them_valid(
    bundle: Context, artifact: str, key: str, value: Json
) -> None:
    path = bundle.root / "image-compliance" / ARTIFACTS[artifact]
    record = read_document(bundle.fs, path)
    record[key] = value
    path.write_text(json.dumps(record))
    rehash(bundle, artifact)
    with pytest.raises(ToolingError):
        ImageComplianceValidate.run(bundle)


@pytest.mark.parametrize(
    ("key", "value"),
    (
        ("name", ""),
        ("versionInfo", "NOASSERTION"),
        ("downloadLocation", "NOASSERTION"),
        ("licenseConcluded", "NOASSERTION"),
        ("licenseDeclared", "NOASSERTION"),
        ("checksums", []),
        ("checksums", [{"algorithm": "SHA1", "checksumValue": "a" * 64}]),
        ("checksums", [{"algorithm": "SHA256", "checksumValue": "A" * 64}]),
    ),
)
def test_each_inventory_package_requires_complete_evidence(
    bundle: Context, key: str, value: Json
) -> None:
    path = bundle.root / "image-compliance" / ARTIFACTS["package_inventory"]
    record = read_document(bundle.fs, path)
    object_value(array_value(record["packages"])[0])[key] = value
    path.write_text(json.dumps(record))
    rehash(bundle, "package_inventory")
    with pytest.raises(ToolingError):
        ImageComplianceValidate.run(bundle)


@pytest.mark.parametrize("name", (BUNDLE, ARTIFACTS["spdx"], "nested"))
def test_bundle_paths_cannot_follow_links(bundle: Context, name: str) -> None:
    output = bundle.root / "image-compliance"
    if name == "nested":
        (output / name).symlink_to(
            bundle.root / "release/media-compliance", target_is_directory=True
        )
        record = manifest(bundle)
        record["spdx_path"] = "nested/media-runtime-inventory.spdx.json"
        write_manifest(bundle, record)
    else:
        target = bundle.root / "outside"
        (output / name).replace(target)
        (output / name).symlink_to(target)
    with pytest.raises(ToolingError):
        ImageComplianceValidate.run(bundle)


class FailedCopy(FileSystem):
    def copy(self, source: Path, destination: Path) -> None:
        if source.name == "THIRD-PARTY-NOTICES.md":
            raise OSError("injected copy failure")
        super().copy(source, destination)


def test_generator_protects_previous_and_unrelated_output(bundle: Context) -> None:
    before = (bundle.root / "image-compliance" / BUNDLE).read_bytes()
    with pytest.raises(OSError, match="copy failure"):
        ImageComplianceGenerate.run(replace(bundle, fs=FailedCopy()))
    assert (bundle.root / "image-compliance" / BUNDLE).read_bytes() == before
    assert not tuple(bundle.root.glob(".rv-compliance-*"))
    (bundle.root / "image-compliance/unrelated").write_text("preserve")
    with pytest.raises(ToolingError, match="unrelated files"):
        ImageComplianceGenerate.run(bundle)
    for output in (
        Path("."),
        Path("../outside"),
        Path("sub/../image-compliance"),
        Path("release/media-compliance"),
    ):
        with pytest.raises(ToolingError):
            ImageComplianceGenerate.run(
                replace(bundle, options=replace(bundle.options, output=output))
            )


@pytest.mark.parametrize(
    "reference", ("", "ghcr.io/owner/image:tag", "ghcr.io/owner/image@sha256:" + "A" * 64)
)
def test_invalid_image_references_never_generate_output(context: Context, reference: str) -> None:
    with pytest.raises(ToolingError, match="immutable lowercase"):
        ImageComplianceGenerate.run(
            replace(context, options=replace(context.options, image_reference=reference))
        )
    assert not (context.root / "image-compliance").exists()


def test_incomplete_task_inputs_fail_before_production(context: Context) -> None:
    empty = replace(context, options=Options())
    for task in (TrivySarifVerify, ImageComplianceGenerate, ImageComplianceValidate):
        with pytest.raises(ToolingError, match="Select"):
            task.run(empty)


def test_original_clean_and_vulnerable_sarif_fixtures_keep_their_outcomes(context: Context) -> None:
    clean = replace(context, options=Options(report=Path("reports/clean.sarif")))
    TrivySarifVerify.run(clean)
    vulnerable = replace(context, options=Options(report=Path("reports/vulnerable-image.sarif")))
    report = context.root / "reports/vulnerable-image.sarif"
    before = report.read_bytes()
    with pytest.raises(ToolingError, match="block verification and publication"):
        TrivySarifVerify.run(vulnerable)
    assert report.read_bytes() == before


@pytest.mark.parametrize(
    "record",
    (
        {},
        {"version": "2.0.0"},
        {"version": "2.1.0", "runs": []},
        {
            "version": "2.1.0",
            "runs": [{"tool": {"driver": {"name": "AnotherScanner"}}, "results": []}],
        },
        {"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "Trivy"}}}]},
        {"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "Trivy"}}, "results": [{}]}]},
    ),
)
def test_sarif_requires_explicit_complete_runs(context: Context, record: JsonObject) -> None:
    path = context.root / "report.sarif"
    path.write_text(json.dumps(record))
    with pytest.raises(ToolingError):
        TrivySarifVerify.run(replace(context, options=Options(report=path)))
