"""Synthetic media boundaries; recorded native profiles are separate fixture data."""

import hashlib
import json
import os
import subprocess
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.json_data import JsonObject
from revaer_tooling.media.model import BOUNDED_ID
from revaer_tooling.media.probes import canonical_probe
from revaer_tooling.process import Completed, Invocation, RunningProcess

DATA = Path(__file__).with_suffix("")
STREAM = {"streams": [{"codec_name": "h264", "codec_type": "video", "index": 0}]}


def write_catalog(root: Path, *, payload: bytes = b"fixture-data") -> None:
    fixtures: list[JsonObject] = []
    sources: list[JsonObject] = []
    for name, directory in (("bbb-h264-mp4", "source"), (BOUNDED_ID, "chromium")):
        path = f"test-fixtures/{directory}/{name}.mp4"
        fixture: JsonObject = {
            "id": name,
            "path": path,
            "source": "https://fixtures.invalid/source",
            "generated": False,
            "shouldGenerate": False,
            "shouldDownload": True,
        }
        if name == BOUNDED_ID:
            fixture["allowProbeDiagnostics"] = True
        fixtures.append(fixture)
        sources.append(
            {
                "id": name,
                "path": path,
                "encoding": "raw",
                "sha256": hashlib.sha256(payload).hexdigest(),
                "minimumBytes": len(payload),
                "maximumBytes": len(payload),
                "urls": ["https://fixtures.invalid/source"],
            }
        )
        destination = root / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(payload)
        snapshot = root / f"test-fixtures/probe/{name}.json"
        snapshot.parent.mkdir(parents=True, exist_ok=True)
        snapshot.write_text(canonical_probe(json.dumps(STREAM)))
    (root / "test-fixtures/manifest.json").write_text(
        json.dumps({"schemaVersion": 1, "fixtures": fixtures})
    )
    (root / "test-fixtures/lock.json").write_text(
        json.dumps({"schemaVersion": 1, "sources": sources})
    )


@pytest.fixture
def media_context(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    root = tmp_path / "checkout"
    (root / "tools/src/revaer_tooling").mkdir(parents=True)
    (root / "tools/src/revaer_tooling/cli.py").touch()
    subprocess.run(
        ["git", "init", "--quiet", str(root)], check=True, capture_output=True, timeout=10
    )
    for name in tuple(os.environ):
        if name.startswith("REVAER_FIXTURE_") or name == "REVAER_MEDIA_CONVERSION_REPORT":
            monkeypatch.delenv(name)
    monkeypatch.chdir(root)
    write_catalog(root)
    return make_context(Options())


class ProbeRunner:
    """Explicit ffprobe process double; it does not validate media decoding."""

    def __init__(self) -> None:
        self.calls: list[Invocation] = []
        self.output = json.dumps(STREAM)
        self.diagnostic = ""
        self.version = (DATA / "f1-host-version.txt").read_bytes().decode()
        self.version_diagnostic = ""
        self.duration = "10.000\n"

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        if "-version" in invocation.argv:
            return Completed(0, self.version, self.version_diagnostic)
        if "-show_entries" in invocation.argv:
            return Completed(0, self.duration)
        return Completed(0, self.output, self.diagnostic)

    def start(self, invocation: Invocation) -> RunningProcess:
        raise AssertionError("Media probes do not start persistent processes")
