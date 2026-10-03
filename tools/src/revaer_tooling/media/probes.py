"""Compare exact native probe records without rewriting reviewed snapshots."""

import difflib
import json
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from ..external.media import Ffprobe
from ..filesystem import FileSystem
from ..json_data import Json, array_value, decode, object_value
from .catalog import Catalog
from .diagnostics import retain_evidence, verify_snapshot
from .model import Fixture, owned_path, regular_file
from .settings import FixtureSettings


def _fallback(value: Json, default: Json) -> Json:
    """Match jq's // semantics; empty strings and zero are real field values."""
    return default if value is None or value is False else value


def canonical_probe(output: str) -> str:
    streams = array_value(object_value(decode(output)).get("streams"))
    if not streams:
        raise ToolingError("ffprobe must identify at least one fixture stream")
    records: list[Json] = []
    for value in streams:
        stream = object_value(value)
        disposition = object_value(stream.get("disposition") or {})
        tags = object_value(stream.get("tags") or {})
        index = stream.get("index")
        if type(index) is not int or index < 0:
            raise ToolingError("ffprobe stream index must be a nonnegative integer")
        records.append(
            {
                "codec_name": stream.get("codec_name"),
                "codec_type": stream.get("codec_type"),
                "disposition": {
                    "default": _fallback(disposition.get("default"), 0),
                    "forced": _fallback(disposition.get("forced"), 0),
                },
                "index": index,
                "tags": {
                    "language": _fallback(tags.get("language"), None),
                    "title": _fallback(tags.get("title"), None),
                },
            }
        )
    # Match jq's reviewed projection, field order, Unicode and terminal LF.
    return json.dumps({"streams": records}, indent=2, ensure_ascii=False) + "\n"


@dataclass(frozen=True)
class ProbeCounts:
    snapshots: int
    bounded: int
    exact: int


def _compare(reviewed: Path, generated: str, fixture: Fixture, emit: Callable[[str], None]) -> None:
    regular_file(reviewed)
    # Reading bytes avoids newline translation of a manually changed snapshot.
    original = reviewed.read_bytes()
    if original != generated.encode("utf-8"):
        emit(
            "".join(
                difflib.unified_diff(
                    original.decode("utf-8").splitlines(keepends=True),
                    generated.splitlines(keepends=True),
                    fromfile=str(reviewed),
                    tofile=fixture.id + " (current)",
                )
            )
        )
        raise ToolingError(f"Probe snapshot drift detected for {fixture.id}")


def probe_catalog(
    catalog: Catalog,
    root: Path,
    settings: FixtureSettings,
    ffprobe: Ffprobe,
    fs: FileSystem,
    emit: Callable[[str], None],
    *,
    update: bool = False,
) -> ProbeCounts:
    parent = owned_path(root, str(settings.report) + ".probe-evidence")
    fs.mkdir(parent)
    attempt = Path(tempfile.mkdtemp(prefix="probe-", dir=parent))
    replacements: list[tuple[Path, str]] = []
    bounded, exact = 0, 0
    for fixture in catalog.fixtures:
        path = owned_path(root, fixture.path)
        reviewed = owned_path(root, settings.probes / (fixture.id + ".json"))
        regular_file(path)
        if path.stat().st_size == 0:
            raise ToolingError(f"Fixture {fixture.id} is empty")
        if fixture.exact_diagnostic:
            verify_snapshot(reviewed)
            catalog.source(fixture.id).verify(path)
        result = ffprobe.streams(path)
        diagnostic = result.stderr.encode("utf-8")
        fs.write(attempt / (fixture.id + ".stderr"), result.stderr, 0o600)
        if len(diagnostic) > 4096:
            raise ToolingError(f"Fixture {fixture.id} diagnostics exceed the 4096-byte limit")
        generated = canonical_probe(result.stdout)
        fs.write(attempt / (fixture.id + ".json"), generated, 0o600)
        if not update or fixture.exact_diagnostic:
            _compare(reviewed, generated, fixture, emit)
        else:
            replacements.append((reviewed, generated))
        if diagnostic:
            if fixture.exact_diagnostic:
                version = ffprobe.report()
                directory, profile = retain_evidence(parent, diagnostic, version, fs)
                emit(
                    f"ADR578 F1 exact diagnostic accepted: fixture={fixture.id}; "
                    f"count=1; profile={profile}; evidence={directory}"
                )
                emit(version.decode("utf-8"))
                exact += 1
            elif fixture.bounded_diagnostics:
                emit(f"Accepted bounded diagnostics for {fixture.id}: {len(diagnostic)} bytes")
                bounded += 1
            else:
                raise ToolingError(f"Unapproved diagnostics emitted for {fixture.id}")
    # Ordinary snapshot updates wait until every fixture passes. The exact F1
    # snapshot is always compared and is never included in this publication list.
    for path, content in replacements:
        fs.write(path, content)
    counts = ProbeCounts(len(catalog.fixtures), bounded, exact)
    fs.write(
        attempt / "summary.json",
        json.dumps(
            {
                "snapshots": counts.snapshots,
                "bounded": counts.bounded,
                "exact": counts.exact,
                "updated": len(replacements),
            },
            indent=2,
        )
        + "\n",
        0o600,
    )
    emit(f"Verified {counts.snapshots} snapshots; bounded diagnostics={bounded}; ADR578 F1={exact}")
    return counts
