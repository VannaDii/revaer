"""Generate only the derived fixtures declared by the selected checkout."""

import tempfile
from collections.abc import Callable
from pathlib import Path

from ..errors import ToolingError
from ..external.media import Ffmpeg, Ffprobe, GenerateFixtureArgs, Recipe
from .catalog import Catalog
from .model import owned_path, regular_file
from .settings import FixtureSettings

SUBTITLES = {
    "english-full.srt": "1\n00:00:00,000 --> 00:00:03,000\nEnglish full subtitle line one.\n\n"
    "2\n00:00:03,000 --> 00:00:07,000\nEnglish full subtitle line two.\n",
    "english-forced.srt": "1\n00:00:01,000 --> 00:00:04,000\nEnglish forced subtitle.\n",
}


def generate(
    catalog: Catalog,
    root: Path,
    settings: FixtureSettings,
    ffmpeg: Ffmpeg,
    ffprobe: Ffprobe,
    emit: Callable[[str], None],
) -> None:
    selected = tuple(item for item in catalog.fixtures if item.generate)
    if not selected:
        raise ToolingError("Fixture manifest declares no generation recipes")
    try:
        recipes = tuple((fixture, Recipe(fixture.id)) for fixture in selected)
    except ValueError as error:
        raise ToolingError("Fixture manifest selects an unknown generation recipe") from error
    source = catalog.source("bbb-h264-mp4")
    source_path = owned_path(root, source.path)
    source.verify(source_path)
    initial = ffprobe.streams(source_path)
    if initial.stderr:
        raise ToolingError("Generation source emitted unapproved probe diagnostics")
    duration = ffprobe.duration(source_path)
    for fixture, recipe in recipes:
        destination = owned_path(root, fixture.path)
        if destination.exists():
            regular_file(destination)
            if destination.stat().st_size > 0 and not settings.force_generate:
                emit(f"Derived fixture {fixture.id} already exists")
                continue
        destination.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix=".generate-", dir=destination.parent) as name:
            staging = Path(name)
            if recipe == Recipe.SUBTITLES:
                for filename, content in SUBTITLES.items():
                    (staging / filename).write_text(content, encoding="utf-8")
            output = staging / "generated"
            ffmpeg.generate(GenerateFixtureArgs(recipe, source_path, output, duration))
            regular_file(output)
            if output.stat().st_size == 0:
                raise ToolingError(f"FFmpeg produced an empty fixture for {fixture.id}")
            output.chmod(0o644)
            output.replace(destination)
        emit(f"Generated fixture {fixture.id}")
