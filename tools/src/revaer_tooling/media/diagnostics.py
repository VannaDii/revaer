"""ADR 578 F1's existing exception, bound to exact identity and native bytes.

This is a fixture-only approval from the media stack. No version upgrade,
diagnostic variation, snapshot update, or additional fixture inherits it.
"""

import hashlib
import json
import re
import tempfile
from pathlib import Path

from ..errors import ToolingError
from ..filesystem import FileSystem
from ..json_data import JsonObject, object_value
from .model import F1_ID, Fixture, Source, digest

REVISION = "e6965e5ca666322ed93e2748a10a4f132309e005"
REPOSITORY = "https://github.com/ietf-wg-cellar/matroska-test-files"
SOURCE_SHA = "43df750a2a01a37949791b717051b41522081a266b71d113be4b713063843699"
SNAPSHOT_SHA = "7d98717b7c956c000ce4cdfdc6da9f84aebd632db51c5e8f6c4a635b2f797d25"
F1_SOURCE = Source(
    F1_ID,
    "test-fixtures/matroska/" + F1_ID + ".mkv",
    "raw",
    SOURCE_SHA,
    21313902,
    21313902,
    (
        f"https://raw.githubusercontent.com/ietf-wg-cellar/matroska-test-files/{REVISION}/test_files/test4.mkv",
    ),
)
PROFILES = {
    "8d4ba0f1aaef40839cba09f51dbcc0efbb58d1c65dc490385e46115bc8e0217c": (
        "historical-alpine-8.0.1-r1"
    ),
    "abd50a4468578ece7323bce6d220a2c909c5a38ce7328929947dda86a8bfab41": "homebrew-9.0.1",
}
DIAGNOSTIC = re.compile(
    rb"\[matroska,webm @ 0x[0-9a-f]{1,16}\] Length 5 indicated by an EBML number's "
    rb"first byte 0x0a at pos 35 \(0x23\) exceeds max length 4\.\n"
)


def verify_identity(fixture: Fixture, source: Source, lock: JsonObject, entry: JsonObject) -> None:
    upstream = object_value(object_value(lock.get("upstreams")).get("matroska"))
    expected = Fixture(
        F1_ID,
        F1_SOURCE.path,
        f"{REPOSITORY}/blob/{REVISION}/test_files/test4.mkv",
        False,
        True,
        False,
        True,
    )
    fields = {"id", "path", "encoding", "sha256", "minimumBytes", "maximumBytes", "urls"}
    if (
        fixture != expected
        or source != F1_SOURCE
        or set(entry) != fields
        or upstream.get("revision") != REVISION
        or upstream.get("repository") != REPOSITORY
    ):
        raise ToolingError("ADR578 F1 source or manifest identity expired")


def verify_snapshot(path: Path) -> None:
    if digest(path) != SNAPSHOT_SHA:
        raise ToolingError("ADR578 F1 reviewed snapshot identity expired")


def diagnostic_profile(diagnostic: bytes, version: bytes) -> str:
    if not 0 < len(diagnostic) <= 131 or DIAGNOSTIC.fullmatch(diagnostic) is None:
        raise ToolingError("ADR578 F1 exact diagnostic contract expired")
    profile = PROFILES.get(hashlib.sha256(version).hexdigest())
    if profile is None:
        raise ToolingError("ADR578 F1 full ffprobe version report is unapproved")
    return profile


def retain_evidence(
    parent: Path, diagnostic: bytes, version: bytes, fs: FileSystem
) -> tuple[Path, str]:
    profile = diagnostic_profile(diagnostic, version)
    fs.mkdir(parent)
    directory = Path(tempfile.mkdtemp(prefix="adr578-f1.", dir=parent))
    # This directory is private at creation. It intentionally survives failure:
    # incomplete evidence must be inspectable, and must never establish success.
    for name, data in (("probe.stderr", diagnostic), ("ffprobe-version.txt", version)):
        path = directory / name
        fs.write_bytes(path, data, 0o600)
        if path.read_bytes() != data:
            raise ToolingError("ADR578 F1 raw evidence retention failed")
    fs.write(
        directory / "classification.json",
        json.dumps(
            {
                "contract": "adr578-f1",
                "fixture": F1_ID,
                "classification": "exact-locked-fixture-recovery-diagnostic",
                "count": 1,
                "source_sha256": SOURCE_SHA,
                "snapshot_sha256": SNAPSHOT_SHA,
                "profile": profile,
                "version_report_sha256": hashlib.sha256(version).hexdigest(),
                "diagnostic_sha256": hashlib.sha256(diagnostic).hexdigest(),
                "diagnostic_file": "probe.stderr",
                "version_report_file": "ffprobe-version.txt",
            },
            indent=2,
        )
        + "\n",
        0o600,
    )
    return directory, profile
