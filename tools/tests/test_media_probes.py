"""Port the media stack's adversarial F1 cases without broadening its exception.

Identity tests use the recorded real manifests and version/snapshot bytes.
Probe coordination uses an explicitly synthetic source with its own real hash,
plus a process double. Native decoding is verified separately.
"""

import hashlib
import json
import shutil
from dataclasses import replace
from pathlib import Path

import pytest
from fixtures.media import DATA, ProbeRunner, media_context
from revaer_tooling.context import Context
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.media import Ffprobe
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.media.catalog import Catalog, load_catalog
from revaer_tooling.media.diagnostics import (
    F1_SOURCE,
    diagnostic_profile,
    retain_evidence,
    verify_snapshot,
)
from revaer_tooling.media.model import F1_ID
from revaer_tooling.media.probes import ProbeCounts, canonical_probe, probe_catalog

__all__ = ["media_context"]
DIAGNOSTIC = (
    b"[matroska,webm @ 0x1] Length 5 indicated by an EBML number's first byte 0x0a "
    b"at pos 35 (0x23) exceeds max length 4.\n"
)


def recorded(context: Context) -> Catalog:
    for name in ("manifest.json", "lock.json"):
        shutil.copy2(DATA / name, context.root / "test-fixtures" / name)
    return load_catalog(context.root, context.settings.fixtures)


@pytest.fixture
def f1(media_context: Context) -> tuple[Context, Catalog, ProbeRunner]:
    context = media_context
    catalog = recorded(context)
    fixture = next(item for item in catalog.fixtures if item.id == F1_ID)
    payload = b"synthetic source for F1 coordination"
    source = replace(
        F1_SOURCE,
        sha256=hashlib.sha256(payload).hexdigest(),
        minimum=len(payload),
        maximum=len(payload),
    )
    path = context.root / source.path
    path.parent.mkdir(parents=True)
    path.write_bytes(payload)
    snapshot = context.root / context.settings.fixtures.probes / (F1_ID + ".json")
    shutil.copy2(DATA / "probe" / snapshot.name, snapshot)
    runner = ProbeRunner()
    runner.output = snapshot.read_text()
    runner.diagnostic = DIAGNOSTIC.decode()
    probe = Ffprobe("ffprobe", runner, context.root, context.tools.ffprobe.environment)
    return (
        replace(context, tools=replace(context.tools, ffprobe=probe)),
        Catalog((fixture,), (source,)),
        runner,
    )


def check(case: tuple[Context, Catalog, ProbeRunner], *, update: bool = False) -> ProbeCounts:
    context, catalog, _ = case
    return probe_catalog(
        catalog,
        context.root,
        context.settings.fixtures,
        context.tools.ffprobe,
        context.fs,
        context.emit,
        update=update,
    )


@pytest.mark.parametrize("profile", ("f1-host-version.txt", "f1-linux-version.txt"))
@pytest.mark.parametrize("update", (False, True))
def test_exact_profiles_retain_native_bytes_and_never_replace_f1(
    f1: tuple[Context, Catalog, ProbeRunner],
    profile: str,
    update: bool,
) -> None:
    context, _, runner = f1
    version = (DATA / profile).read_bytes()
    runner.version = version.decode()
    snapshot = context.root / context.settings.fixtures.probes / (F1_ID + ".json")
    before = snapshot.stat().st_mtime_ns
    assert check(f1, update=update) == ProbeCounts(1, 0, 1)
    assert snapshot.stat().st_mtime_ns == before
    parent = context.root / (str(context.settings.fixtures.report) + ".probe-evidence")
    evidence = next(parent.glob("adr578-f1.*"))
    assert (evidence / "probe.stderr").read_bytes() == DIAGNOSTIC
    assert (evidence / "ffprobe-version.txt").read_bytes() == version
    metadata = json.loads((evidence / "classification.json").read_text())
    assert metadata["count"] == 1
    assert metadata["contract"] == "adr578-f1"
    assert metadata["version_report_sha256"] == hashlib.sha256(version).hexdigest()
    assert evidence.stat().st_mode & 0o777 == 0o700
    assert all(path.stat().st_mode & 0o777 == 0o600 for path in evidence.iterdir())


@pytest.mark.parametrize(
    "field,value",
    (
        ("probeDiagnosticContract", "unknown"),
        ("probeDiagnosticContract", None),
        ("probeDiagnosticContract", True),
        ("allowProbeDiagnostics", False),
        ("allowProbeDiagnostics", True),
        ("id", "other-fixture"),
        ("path", "scripts/other"),
        ("source", "https://example.invalid/changed"),
        ("shouldDownload", False),
        ("generated", True),
        ("shouldGenerate", True),
    ),
)
def test_f1_manifest_policy_and_identity_expire(
    media_context: Context, field: str, value: object
) -> None:
    recorded(media_context)
    path = media_context.root / "test-fixtures/manifest.json"
    data = json.loads(path.read_text())
    fixture = next(item for item in data["fixtures"] if item["id"] == F1_ID)
    fixture[field] = value
    path.write_text(json.dumps(data))
    with pytest.raises(ToolingError):
        load_catalog(media_context.root, media_context.settings.fixtures)


@pytest.mark.parametrize(
    "field,value",
    (
        ("sha256", "0" * 64),
        ("minimumBytes", 21313901),
        ("maximumBytes", 21313903),
        ("encoding", "base64"),
        ("urls", ["https://example.invalid/other"]),
        ("extra", True),
        ("revision", "other"),
        ("repository", "https://example.invalid/other"),
    ),
)
def test_f1_lock_identity_expires(media_context: Context, field: str, value: object) -> None:
    recorded(media_context)
    path = media_context.root / "test-fixtures/lock.json"
    data = json.loads(path.read_text())
    selected = (
        data["upstreams"]["matroska"]
        if field in ("revision", "repository")
        else next(item for item in data["sources"] if item["id"] == F1_ID)
    )
    selected[field] = value
    path.write_text(json.dumps(data))
    with pytest.raises(ToolingError):
        load_catalog(media_context.root, media_context.settings.fixtures)


INVALID_DIAGNOSTICS = (
    *(
        DIAGNOSTIC.replace(b"0x1]", value + b"]")
        for value in (b"0xA", b"0x12345678901234567", b"0x", b"0X1")
    ),
    *(
        DIAGNOSTIC.replace(old, new)
        for old, new in (
            (b"pos 35", b"pos 36"),
            (b"Length 5", b"Length 4"),
            (b"0x0a", b"0x0b"),
            (b"0x23", b"0x24"),
            (b"max length 4", b"max length 5"),
        )
    ),
    DIAGNOSTIC[:-1],
    DIAGNOSTIC[:-1] + b"\r\n",
    DIAGNOSTIC + b"\n",
    DIAGNOSTIC + b"extra\n",
    b"\n" + DIAGNOSTIC,
    DIAGNOSTIC.replace(b"0x1]", b"0x1\0]"),
    DIAGNOSTIC[:-1] + b"\0\n",
    DIAGNOSTIC[:-1] + "é\n".encode(),
    b"x" * 4097,
)


@pytest.mark.parametrize("diagnostic", INVALID_DIAGNOSTICS)
def test_f1_rejects_every_unapproved_diagnostic_byte_sequence(
    f1: tuple[Context, Catalog, ProbeRunner],
    diagnostic: bytes,
) -> None:
    f1[2].diagnostic = diagnostic.decode()
    with pytest.raises(ToolingError):
        check(f1)


@pytest.mark.parametrize(
    "version",
    (
        "",
        "ffprobe version 9.0.2\n",
        "ffprobe version 8.0.1\n",
        (DATA / "f1-host-version.txt").read_text() + "\n",
    ),
)
def test_f1_rejects_incomplete_or_changed_tool_profile(
    f1: tuple[Context, Catalog, ProbeRunner],
    version: str,
) -> None:
    f1[2].version = version
    with pytest.raises(ToolingError):
        check(f1)


def test_f1_version_diagnostics_fail(f1: tuple[Context, Catalog, ProbeRunner]) -> None:
    f1[2].version_diagnostic = "unexpected warning\n"
    with pytest.raises(ToolingError, match="diagnostic-bearing"):
        check(f1)


@pytest.mark.parametrize("output", ("", "invalid JSON", '{"streams":[]}', "{}"))
def test_malformed_probe_output_fails(
    f1: tuple[Context, Catalog, ProbeRunner], output: str
) -> None:
    f1[2].output = output
    with pytest.raises(ToolingError):
        check(f1)


@pytest.mark.parametrize("update", (False, True))
@pytest.mark.parametrize("change", ("missing", "changed", "linked"))
def test_f1_snapshot_identity_cannot_be_updated_away(
    f1: tuple[Context, Catalog, ProbeRunner],
    update: bool,
    change: str,
) -> None:
    context, _, runner = f1
    path = context.root / context.settings.fixtures.probes / (F1_ID + ".json")
    if change == "changed":
        path.write_bytes(path.read_bytes() + b"\n")
    else:
        path.unlink()
        if change == "linked":
            path.symlink_to(DATA / "probe" / path.name)
    with pytest.raises(ToolingError):
        check(f1, update=update)
    assert not runner.calls


@pytest.mark.parametrize("change", ("missing", "size", "hash", "linked"))
def test_f1_source_validation_precedes_probe(
    f1: tuple[Context, Catalog, ProbeRunner],
    change: str,
) -> None:
    context, catalog, runner = f1
    path = context.root / catalog.sources[0].path
    if change in ("missing", "linked"):
        path.unlink()
        if change == "linked":
            path.symlink_to(context.root / "elsewhere")
    else:
        path.write_bytes(b"small" if change == "size" else b"x" * path.stat().st_size)
    with pytest.raises(ToolingError):
        check(f1)
    assert not runner.calls


def test_empty_f1_diagnostics_need_no_tool_profile(
    f1: tuple[Context, Catalog, ProbeRunner],
) -> None:
    f1[2].diagnostic = ""
    f1[2].version = "unknown"
    assert check(f1) == ProbeCounts(1, 0, 0)
    assert len(f1[2].calls) == 1


def test_removing_contract_reverts_to_strict_diagnostics(
    f1: tuple[Context, Catalog, ProbeRunner],
) -> None:
    context, catalog, runner = f1
    strict = replace(catalog, fixtures=(replace(catalog.fixtures[0], exact_diagnostic=False),))
    with pytest.raises(ToolingError, match="Unapproved diagnostics"):
        check((context, strict, runner))


@pytest.mark.parametrize("name", ("probe.stderr", "ffprobe-version.txt"))
@pytest.mark.parametrize("change", ("missing", "changed"))
def test_lost_evidence_cannot_establish_success(tmp_path: Path, name: str, change: str) -> None:
    class DamagedEvidence(FileSystem):
        def write_bytes(self, path: Path, value: bytes, mode: int = 0o644) -> None:
            if path.name == name and change == "missing":
                return
            super().write_bytes(path, b"changed" if path.name == name else value, mode)

    with pytest.raises((OSError, ToolingError)):
        retain_evidence(
            tmp_path / "evidence",
            DIAGNOSTIC,
            (DATA / "f1-host-version.txt").read_bytes(),
            DamagedEvidence(),
        )
    assert not tuple(tmp_path.rglob("classification.json"))


def test_evidence_destination_failure_is_not_ignored(tmp_path: Path) -> None:
    parent = tmp_path / "file"
    parent.write_text("retained")
    with pytest.raises(OSError):
        retain_evidence(
            parent / "evidence",
            DIAGNOSTIC,
            (DATA / "f1-host-version.txt").read_bytes(),
            FileSystem(),
        )
    assert parent.read_text() == "retained"


def test_empty_optional_tags_keep_the_reviewed_json_projection() -> None:
    source = {
        "streams": [
            {"index": 0, "tags": {"title": "", "language": None}, "disposition": {"default": False}}
        ]
    }
    result = json.loads(canonical_probe(json.dumps(source)))["streams"][0]
    assert result["tags"] == {"title": "", "language": None}
    assert result["disposition"] == {"default": 0, "forced": 0}


def test_pinned_snapshot_and_diagnostic_profile_are_exact() -> None:
    verify_snapshot(DATA / "probe" / (F1_ID + ".json"))
    assert (
        diagnostic_profile(DIAGNOSTIC, (DATA / "f1-linux-version.txt").read_bytes())
        == "historical-alpine-8.0.1-r1"
    )


def test_snapshot_updates_wait_for_all_probes_to_pass(media_context: Context) -> None:
    context = media_context
    catalog = load_catalog(context.root, context.settings.fixtures)
    runner = ProbeRunner()
    runner.diagnostic = "unapproved"
    first = context.root / context.settings.fixtures.probes / (catalog.fixtures[0].id + ".json")
    before = first.read_bytes()
    probe = Ffprobe("ffprobe", runner, context.root, context.tools.ffprobe.environment)
    with pytest.raises(ToolingError, match="Unapproved"):
        probe_catalog(
            catalog,
            context.root,
            context.settings.fixtures,
            probe,
            context.fs,
            context.emit,
            update=True,
        )
    assert first.read_bytes() == before
