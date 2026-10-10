"""Publish LCOV from a completed Chromium run and its full source inventory."""

import re
from pathlib import Path

from ..context import Context, TaskResult
from ..e2e.coverage import verify_analysis_run
from ..e2e.database import uses_single_init
from ..e2e.javascript import Script, source_text
from ..e2e.v8 import Location, line_counts, nonnegative, source_digest, source_lines, utf16_length
from ..errors import ToolingError
from ..json_data import JsonObject, array_value, decode, object_value, string_value
from ..sonar.inputs import lcov_counts
from .base import Task
from .e2e import RunPaths


def javascript_sources(context: Context) -> tuple[str, ...]:
    return tuple(
        name
        for name in context.tools.git.files(include_untracked=True)
        if name.endswith((".js", ".mjs", ".cjs")) and (context.root / name).is_file()
    )


def report(context: Context, path: Path) -> JsonObject:
    if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(context.root):
        raise ToolingError("JavaScript coverage evidence must be a regular checkout file")
    document = object_value(decode(context.fs.read(path)))
    if type(document.get("version")) is not int or document.get("version") != 1:
        raise ToolingError("Unsupported JavaScript coverage evidence version")
    return document


def read_inventory(context: Context, path: Path, expected: tuple[str, ...]) -> dict[str, Script]:
    scripts: dict[str, Script] = {}
    found: set[str] = set()
    for item in array_value(report(context, path).get("sources")):
        entry = object_value(item)
        digest = string_value(entry.get("sha256"))
        source = entry.get("source")
        if not isinstance(source, str) or source_digest(source) != digest or digest in scripts:
            raise ToolingError("JavaScript syntax source identity is invalid or duplicated")
        paths = tuple(string_value(value) for value in array_value(entry.get("paths")))
        if not paths:
            raise ToolingError("JavaScript syntax source has no checkout paths")
        for name in paths:
            if name not in expected or name in found or source_text(context.root, name) != source:
                raise ToolingError(
                    "JavaScript source changed or differs from its syntax evidence: " + name
                )
            found.add(name)
        lines, starts = source_lines(source)
        points: set[Location] = set()
        for value in array_value(entry.get("points")):
            point = object_value(value)
            location = Location(nonnegative(point.get("offset")), nonnegative(point.get("line")))
            if (
                not 1 <= location.line <= len(lines)
                or not starts[location.line - 1] <= location.offset < starts[-1]
                or location.offset
                > starts[location.line - 1]
                + utf16_length(lines[location.line - 1].rstrip("\r\n\u2028\u2029"))
                or location in points
            ):
                raise ToolingError("JavaScript syntax location is invalid or duplicated")
            points.add(location)
        scripts[digest] = Script(source, paths, tuple(sorted(points)))
    if found != set(expected) or not found:
        raise ToolingError("JavaScript syntax evidence omits checkout source files")
    return scripts


def merge_captures(context: Context, paths: tuple[Path, ...], scripts: dict[str, Script]) -> str:
    counts = {
        digest: dict.fromkeys((point.line for point in script.points), 0)
        for digest, script in scripts.items()
    }
    for path in paths:
        document = report(context, path)
        if document.get("complete") is not True:
            raise ToolingError("JavaScript page coverage was not completed: " + str(path))
        for item in array_value(document.get("captures")):
            capture = object_value(item)
            digest = string_value(capture.get("sha256"))
            if not re.fullmatch(r"[0-9a-f]{64}", digest):
                raise ToolingError("Measured JavaScript source identity must be a SHA-256")
            length = nonnegative(capture.get("length"))
            script = scripts.get(digest)
            if script and utf16_length(script.source) != length:
                raise ToolingError("Measured JavaScript source length does not match its bytes")
            hits = line_counts(
                array_value(capture.get("functions")),
                script.points if script else (),
                length,
            )
            if script:
                for line, count in hits.items():
                    counts[digest][line] += count
            # Trunk's generated loader has no authored file with those bytes.
            # Keep its raw execution record; never attribute it to another file.
    output: list[str] = []
    for name, digest in sorted(
        (name, digest) for digest, script in scripts.items() for name in script.paths
    ):
        lines = counts[digest]
        output.extend(
            (
                "SF:" + name,
                *(f"DA:{line},{count}" for line, count in sorted(lines.items())),
                f"LF:{len(lines)}",
                f"LH:{sum(count > 0 for count in lines.values())}",
                "end_of_record",
            )
        )
    return "\n".join(output) + "\n"


class JavaScriptCoverageMerge(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        paths = RunPaths.for_context(context)
        context.fs.mkdir(paths.runtime)
        target = context.root / "coverage/js-lcov.info"
        with context.fs.lock(paths.runtime / "e2e.lock"):
            context.fs.remove_owned(target, context.root)
            summary = object_value(
                decode(context.fs.read(paths.results / "python-e2e-summary.json"))
            )
            expected = context.settings.e2e.phases(media=uses_single_init(context))
            verify_analysis_run(summary, expected, browser=True)
            baselines = tuple(sorted(paths.results.glob("javascript-baseline-*.json")))
            captures = tuple(
                sorted((paths.results / "ui-chromium").glob("*/javascript-page-*.json"))
            )
            if not baselines or not captures:
                raise ToolingError(
                    "JavaScript syntax and page execution evidence are both required"
                )
            sources = javascript_sources(context)
            scripts = read_inventory(context, baselines[0], sources)
            if any(read_inventory(context, path, sources) != scripts for path in baselines[1:]):
                raise ToolingError("Chromium workers disagree on JavaScript source locations")
            document = merge_captures(context, captures, scripts)
            if not lcov_counts(document).covered:
                raise ToolingError("JavaScript coverage contains no executed authored locations")
            context.fs.write(target, document)
        return TaskResult("Merged syntax locations and browser execution: coverage/js-lcov.info")
