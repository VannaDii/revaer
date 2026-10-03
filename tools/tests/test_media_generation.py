"""Exercise all eight recipes with actual FFmpeg and reviewed stream snapshots."""

import json
import os
import shutil
from dataclasses import replace
from pathlib import Path

import pytest
from fixtures.media import DATA, ProbeRunner, media_context
from revaer_tooling.context import Context
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.curl import Curl
from revaer_tooling.external.media import Ffmpeg, Ffprobe, Recipe
from revaer_tooling.media.acquire import acquire
from revaer_tooling.media.catalog import Catalog, load_catalog
from revaer_tooling.media.generate import generate
from revaer_tooling.media.model import Fixture, document, sources
from revaer_tooling.media.probes import canonical_probe
from revaer_tooling.media.settings import load_fixture_settings
from revaer_tooling.process import Completed, Invocation, ProcessRunner, RunningProcess

__all__ = ["media_context"]


@pytest.fixture(scope="module")
def native_source(tmp_path_factory: pytest.TempPathFactory) -> Path:
    root = tmp_path_factory.mktemp("native-media")
    source = next(
        item
        for item in sources(document(DATA / "lock.json", "sources")[1])
        if item.id == "bbb-h264-mp4"
    )
    curl = Curl("curl", ProcessRunner(print), root, dict(os.environ))
    acquire(source, root, load_fixture_settings({}), curl, print)
    return root / source.path


def test_every_native_recipe_matches_reviewed_snapshots_and_reuses_cache(
    media_context: Context,
    native_source: Path,
) -> None:
    context = media_context
    catalog = load_catalog(context.root, context.settings.fixtures)
    source = next(
        item
        for item in sources(document(DATA / "lock.json", "sources")[1])
        if item.id == "bbb-h264-mp4"
    )
    destination = context.root / source.path
    shutil.copy2(native_source, destination)
    recorded = json.loads((DATA / "manifest.json").read_text())["fixtures"]
    generated = tuple(
        Fixture(item["id"], item["path"], item["source"], True, False, False, False)
        for item in recorded
        if item["shouldGenerate"]
    )
    catalog = Catalog(generated, (source,))
    generate(
        catalog,
        context.root,
        context.settings.fixtures,
        context.tools.ffmpeg,
        context.tools.ffprobe,
        context.emit,
    )
    assert {item.id for item in generated} == set(Recipe)
    times = {}
    for fixture in generated:
        path = context.root / fixture.path
        expected = (DATA / "probe" / (fixture.id + ".json")).read_bytes()
        result = context.tools.ffprobe.streams(path)
        assert not result.stderr
        assert canonical_probe(result.stdout).encode() == expected
        assert path.stat().st_mode & 0o777 == 0o644
        times[fixture.id] = path.stat().st_mtime_ns
    generate(
        catalog,
        context.root,
        context.settings.fixtures,
        context.tools.ffmpeg,
        context.tools.ffprobe,
        context.emit,
    )
    assert {item.id: (context.root / item.path).stat().st_mtime_ns for item in generated} == times
    assert not tuple((context.root / "test-fixtures/derived").glob(".generate-*"))


class EncoderRunner:
    def __init__(self, *, fail: bool = False, empty: bool = False, diagnostic: str = "") -> None:
        self.fail, self.empty, self.diagnostic = fail, empty, diagnostic
        self.calls: list[Invocation] = []

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        if self.fail:
            raise CommandError("simulated FFmpeg failure", 1)
        Path(invocation.argv[-1]).write_bytes(b"" if self.empty else b"generated fixture")
        return Completed(0, "", self.diagnostic)

    def start(self, invocation: Invocation) -> RunningProcess:
        raise AssertionError("Encoding runs to completion")


def generation_case(context: Context) -> tuple[Catalog, ProbeRunner]:
    source = load_catalog(context.root, context.settings.fixtures).sources[0]
    fixture = Fixture(
        "subtitles-mkv",
        "test-fixtures/derived/subtitles.mkv",
        "generated",
        True,
        False,
        False,
        False,
    )
    return Catalog((fixture,), (source,)), ProbeRunner()


@pytest.mark.parametrize("failure", ("status", "empty", "diagnostics"))
def test_failed_regeneration_keeps_previous_bytes(media_context: Context, failure: str) -> None:
    context = media_context
    catalog, probe_runner = generation_case(context)
    destination = context.root / catalog.fixtures[0].path
    destination.parent.mkdir()
    destination.write_bytes(b"previous generated fixture")
    runner = EncoderRunner(
        fail=failure == "status",
        empty=failure == "empty",
        diagnostic="error\n" if failure == "diagnostics" else "",
    )
    encoder = Ffmpeg("ffmpeg", runner, context.root, context.tools.ffmpeg.environment)
    probe = Ffprobe("ffprobe", probe_runner, context.root, context.tools.ffprobe.environment)
    with pytest.raises(ToolingError):
        generate(
            catalog,
            context.root,
            replace(context.settings.fixtures, force_generate=True),
            encoder,
            probe,
            context.emit,
        )
    assert destination.read_bytes() == b"previous generated fixture"
    assert not tuple(destination.parent.glob(".generate-*"))


@pytest.mark.parametrize("duration", ("0", "-1", "NaN", "Infinity", "invalid", "0.0001", "1e99999"))
def test_invalid_source_duration_prevents_encoding(media_context: Context, duration: str) -> None:
    context = media_context
    catalog, probe_runner = generation_case(context)
    probe_runner.duration = duration
    runner = EncoderRunner()
    encoder = Ffmpeg("ffmpeg", runner, context.root, context.tools.ffmpeg.environment)
    probe = Ffprobe("ffprobe", probe_runner, context.root, context.tools.ffprobe.environment)
    with pytest.raises(ToolingError, match="duration"):
        generate(catalog, context.root, context.settings.fixtures, encoder, probe, context.emit)
    assert not runner.calls


def test_unknown_recipe_and_invalid_source_fail_before_probing(media_context: Context) -> None:
    context = media_context
    catalog, probe_runner = generation_case(context)
    probe = Ffprobe("ffprobe", probe_runner, context.root, context.tools.ffprobe.environment)
    unknown = replace(catalog, fixtures=(replace(catalog.fixtures[0], id="unknown"),))
    with pytest.raises(ToolingError, match="unknown generation"):
        generate(
            unknown,
            context.root,
            context.settings.fixtures,
            context.tools.ffmpeg,
            probe,
            context.emit,
        )
    (context.root / catalog.sources[0].path).write_bytes(b"changed")
    with pytest.raises(ToolingError, match="locked"):
        generate(
            catalog,
            context.root,
            context.settings.fixtures,
            context.tools.ffmpeg,
            probe,
            context.emit,
        )
    assert not probe_runner.calls
