"""Typed FFmpeg and ffprobe operations for the reviewed fixture recipes.

Native tools own codec discovery and media processing. Recipe arguments remain
literal argv entries; callers cannot append an embedded shell command.
"""

import re
from collections.abc import Mapping
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from enum import StrEnum
from pathlib import Path

from ..errors import ToolingError
from ..json_data import array_value, decode, object_value
from ..process import Completed, Runner
from .base import ExternalTool


class Recipe(StrEnum):
    MULTI_AUDIO = "multi-audio-mkv"
    SUBTITLES = "subtitles-mkv"
    VIDEO_ONLY = "video-only-mp4"
    AUDIO_ONLY = "audio-only-m4a"
    SILENT_AUDIO = "silent-audio-mp4"
    TRANSPORT_STREAM = "h264-aac-ts"
    QUICKTIME = "h264-aac-mov"
    AVI = "mpeg4-mp3-avi"


@dataclass(frozen=True)
class GenerateFixtureArgs:
    recipe: Recipe
    source: Path
    output: Path
    duration: str


class MediaTool(ExternalTool):
    """FFmpeg tools share a full version report and single-dash version option."""

    version_args = ("-version",)

    def _media_invoke(self, args: tuple[str, ...]) -> Completed:
        return self._invoke(args, capture=True)

    def report(self) -> bytes:
        result = self._media_invoke(self.version_args)
        if result.stderr or not result.stdout.splitlines() or not result.stdout.splitlines()[0]:
            raise ToolingError(f"{self.name} version report is empty or diagnostic-bearing")
        return result.stdout.encode("utf-8")


class Ffprobe(MediaTool):
    def __init__(
        self,
        name: str,
        runner: Runner,
        root: Path,
        environment: Mapping[str, str],
        *,
        container: str | None = None,
    ) -> None:
        super().__init__(name, runner, root, environment)
        if container is not None and re.fullmatch(r"revaer-[a-z0-9-]+", container) is None:
            raise ToolingError("Fixture ffprobe container must have a Revaer-owned name")
        self.container = container

    def _owned_container(self) -> str | None:
        if self.container is None:
            return None
        selected = self._invoke(
            (
                "container",
                "ls",
                "--all",
                "--no-trunc",
                "--quiet",
                "--filter",
                f"name=^/{self.container}$",
            ),
            capture=True,
        ).stdout.strip()
        if not selected:
            return None
        if re.fullmatch(r"[0-9a-f]{64}", selected) is None:
            raise ToolingError("Fixture probe lookup returned an ambiguous container")
        rows = array_value(
            decode(self._invoke(("container", "inspect", selected), capture=True).stdout)
        )
        if len(rows) != 1:
            raise ToolingError("Fixture probe inspection must return one container")
        row = object_value(rows[0])
        labels = object_value(object_value(row.get("Config")).get("Labels"))
        mounts = array_value(row.get("Mounts"))
        if (
            labels.get("io.revaer.project") != "revaer"
            or labels.get("io.revaer.purpose") != "fixture-probing"
        ):
            raise ToolingError("Fixture probe container ownership differs")
        if len(mounts) != 1:
            raise ToolingError("Fixture probe container must mount only this checkout read-only")
        mount = object_value(mounts[0])
        if (
            mount.get("Type") != "bind"
            or mount.get("Source") != str(self.root)
            or mount.get("Destination") != "/workspace"
            or mount.get("RW") is not False
        ):
            raise ToolingError("Fixture probe container must mount only this checkout read-only")
        return selected

    def prepare_container(self, image: str) -> None:
        if self.container is None:
            return
        if selected := self._owned_container():
            actual = self._invoke(
                ("inspect", "--format", "{{.Config.Image}}", selected), capture=True
            ).stdout.strip()
            if actual != image:
                raise ToolingError("Fixture probe container uses a different image")
            self._invoke(("container", "start", selected), capture=True)
        else:
            self._invoke(
                (
                    "run",
                    "--detach",
                    "--name",
                    self.container,
                    "--label",
                    "io.revaer.project=revaer",
                    "--label",
                    "io.revaer.purpose=fixture-probing",
                    "--mount",
                    f"type=bind,source={self.root},target=/workspace,readonly",
                    image,
                    "sleep",
                    "7200",
                ),
                capture=True,
            )
        self._invoke(
            ("exec", self.container, "apk", "add", "--no-cache", "ffmpeg=8.0.1-r1"), capture=True
        )

    def remove_container(self) -> None:
        if selected := self._owned_container():
            self._invoke(("container", "rm", "--force", selected), capture=True)

    def _media_invoke(self, args: tuple[str, ...]) -> Completed:
        if self.container is None:
            return super()._media_invoke(args)
        # The caller provisions the approved build and mounts this checkout at
        # /workspace. Only the fixture's absolute input path needs translation.
        try:
            mapped = tuple(
                str(Path("/workspace") / Path(arg).relative_to(self.root))
                if Path(arg).is_absolute()
                else arg
                for arg in args
            )
        except ValueError as error:
            raise ToolingError(
                "Fixture probe input must belong to the selected checkout"
            ) from error
        return self._invoke(("exec", self.container, "ffprobe", *mapped), capture=True)

    def streams(self, path: Path) -> Completed:
        if not path.is_absolute():
            raise ToolingError("ffprobe fixture path must be absolute")
        return self._media_invoke(("-v", "error", "-show_streams", "-of", "json", str(path)))

    def duration(self, path: Path) -> str:
        if not path.is_absolute():
            raise ToolingError("ffprobe fixture path must be absolute")
        result = self._media_invoke(
            (
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                str(path),
            ),
        )
        if result.stderr:
            raise ToolingError("ffprobe duration emitted diagnostics")
        try:
            value = Decimal(result.stdout.strip())
            if not value.is_finite() or value <= 0:
                raise ToolingError("ffprobe duration must be a positive finite number")
            rounded = value.quantize(Decimal("0.001"))
        except InvalidOperation as error:
            raise ToolingError("ffprobe duration must be a positive finite number") from error
        if rounded <= 0:
            raise ToolingError("ffprobe duration must be a positive finite number")
        return f"{value:.3f}"


class Ffmpeg(MediaTool):
    def generate(self, args: GenerateFixtureArgs) -> Completed:
        if not args.source.is_absolute() or not args.output.is_absolute():
            raise ToolingError("FFmpeg fixture paths must be absolute")
        if args.output.exists() or args.output.is_symlink():
            raise ToolingError("FFmpeg fixture output must be a new staging file")
        # The duration is computed by Ffprobe, but validate callers at this
        # public adapter boundary too. It is never interpreted as an option.
        try:
            duration = Decimal(args.duration)
        except InvalidOperation as error:
            raise ToolingError("FFmpeg duration must be positive and finite") from error
        if not duration.is_finite() or duration <= 0:
            raise ToolingError("FFmpeg duration must be positive and finite")
        command = ("-hide_banner", "-v", "error", "-nostdin", "-y")
        result = self._invoke((*command, *_recipe_arguments(args), str(args.output)), capture=True)
        if result.stderr:
            raise ToolingError(f"FFmpeg emitted diagnostics for {args.recipe}")
        return result


def _recipe_arguments(args: GenerateFixtureArgs) -> tuple[str, ...]:
    source = ("-i", str(args.source))

    def audio(generator: str) -> tuple[str, ...]:
        return ("-f", "lavfi", "-t", args.duration, "-i", generator)

    tone = "sine=frequency=440:sample_rate=48000"
    silent = "anullsrc=channel_layout=stereo:sample_rate=48000"
    maps = ("-map", "0:v:0", "-map", "1:a:0")
    codecs = ("-c:v", "copy", "-c:a", "aac")
    match args.recipe:
        case Recipe.MULTI_AUDIO:
            return (
                *source,
                *audio(tone),
                *audio("sine=frequency=554:sample_rate=48000"),
                *audio("sine=frequency=659:sample_rate=48000"),
                *audio(silent),
                *maps,
                "-map",
                "2:a:0",
                "-map",
                "3:a:0",
                "-map",
                "4:a:0",
                "-c:v",
                "copy",
                "-c:a:0",
                "aac",
                "-c:a:1",
                "libopus",
                "-c:a:2",
                "ac3",
                "-c:a:3",
                "aac",
                "-metadata:s:a:0",
                "language=eng",
                "-metadata:s:a:0",
                "title=English AAC Tone",
                "-metadata:s:a:1",
                "language=jpn",
                "-metadata:s:a:1",
                "title=Japanese Opus Tone",
                "-metadata:s:a:2",
                "language=spa",
                "-metadata:s:a:2",
                "title=Spanish AC3 Tone",
                "-metadata:s:a:3",
                "language=und",
                "-metadata:s:a:3",
                "title=Undeclared Silent AAC",
                "-shortest",
                "-f",
                "matroska",
            )
        case Recipe.SUBTITLES:
            return (
                *source,
                "-f",
                "srt",
                "-i",
                str(args.output.parent / "english-full.srt"),
                "-f",
                "srt",
                "-i",
                str(args.output.parent / "english-forced.srt"),
                "-map",
                "0:v:0",
                "-map",
                "0:a?",
                "-map",
                "1:0",
                "-map",
                "2:0",
                "-c:v",
                "copy",
                "-c:a",
                "copy",
                "-c:s",
                "srt",
                "-metadata:s:s:0",
                "language=eng",
                "-metadata:s:s:0",
                "title=English Full Subtitles",
                "-metadata:s:s:1",
                "language=eng",
                "-metadata:s:s:1",
                "title=English Forced Subtitles",
                "-disposition:s:0",
                "0",
                "-disposition:s:1",
                "forced",
                # Preserve the foundation fix: Matroska must not infer a default
                # subtitle stream when the reviewed snapshot specifies none.
                "-default_mode",
                "passthrough",
                "-f",
                "matroska",
            )
        case Recipe.VIDEO_ONLY:
            return (*source, "-map", "0:v:0", "-c:v", "copy", "-an", "-sn", "-f", "mp4")
        case Recipe.AUDIO_ONLY:
            return (*audio(tone), "-vn", "-c:a", "aac", "-movflags", "+faststart", "-f", "ipod")
        case Recipe.SILENT_AUDIO:
            return (
                *source,
                *audio(silent),
                *maps,
                *codecs,
                "-shortest",
                "-movflags",
                "+faststart",
                "-f",
                "mp4",
            )
        case Recipe.TRANSPORT_STREAM:
            return (*source, *audio(tone), *maps, *codecs, "-f", "mpegts")
        case Recipe.QUICKTIME:
            return (*source, *audio(tone), *maps, *codecs, "-f", "mov")
        case Recipe.AVI:
            return (
                *source,
                *audio(tone),
                *maps,
                "-c:v",
                "mpeg4",
                "-q:v",
                "5",
                "-c:a",
                "libmp3lame",
                "-q:a",
                "4",
                "-f",
                "avi",
            )
