"""Collect real Chromium coverage through Playwright's supported CDP session.

The syntax parser is injected by pytest. Source locations start at zero. Test
pages supply precise V8 execution counters, matched to sources by their complete
bytes. Raw ranges and source locations remain reviewable, including unused code.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING
from urllib.parse import urlsplit

from ..errors import ToolingError
from ..filesystem import FileSystem
from ..json_data import JsonObject, array_value, object_value, string_value
from .v8 import JavaScriptSyntax, Location, nonnegative, source_digest

if TYPE_CHECKING:
    # Import browser APIs only in the test process. Container build commands
    # still use the same CLI with its core dependency set.
    from playwright.sync_api import CDPSession, Page


@dataclass(frozen=True)
class Script:
    source: str
    paths: tuple[str, ...]
    points: tuple[Location, ...]


def source_text(root: Path, name: str) -> str:
    relative = PurePosixPath(name)
    path = root / name
    if (
        relative.is_absolute()
        or ".." in relative.parts
        or relative.as_posix() != name
        or relative.suffix not in (".js", ".mjs", ".cjs")
        or "\n" in name
        or "\r" in name
        or path.is_symlink()
        or not path.resolve().is_relative_to(root.resolve())
        or not path.is_file()
    ):
        raise ToolingError("JavaScript coverage requires a regular checkout source: " + name)
    return path.read_bytes().decode("utf-8")


def parse_inventory(
    syntax: JavaScriptSyntax, root: Path, paths: tuple[str, ...]
) -> dict[str, Script]:
    """Parse every source, including unexecuted modules, without resolving imports."""
    grouped: dict[str, list[str]] = {}
    contents: dict[str, str] = {}
    for name in paths:
        source = source_text(root, name)
        digest = source_digest(source)
        grouped.setdefault(digest, []).append(name)
        contents[digest] = source
    result: dict[str, Script] = {}
    for digest, names in grouped.items():
        source = contents[digest]
        try:
            points = syntax.locations(source)
        except ToolingError as error:
            raise ToolingError(f"{names[0]}: {error}") from error
        result[digest] = Script(source, tuple(names), points)
    return result


def write_inventory(filesystem: FileSystem, path: Path, scripts: dict[str, Script]) -> None:
    document = {
        "version": 1,
        "sources": [
            {
                "sha256": digest,
                "source": script.source,
                "paths": script.paths,
                "points": [{"offset": point.offset, "line": point.line} for point in script.points],
            }
            for digest, script in scripts.items()
        ],
    }
    filesystem.write(path, json.dumps(document) + "\n", mode=0o600)


class PageCoverage:
    """One page's counters; capture before intentional page closure.

    The fixture calls ``finish`` before it closes contexts. A scenario closing a
    script-bearing page earlier must call this method first; a closed target
    cannot supply its last counters and must never produce a complete report.
    """

    def __init__(self, page: Page, origin: str, path: Path, filesystem: FileSystem) -> None:
        self.page = page
        self.origin = origin
        self.path = path
        self.filesystem = filesystem
        self.local_scripts: dict[str, tuple[str, int]] = {}
        self.finished = False
        self.session: CDPSession = page.context.new_cdp_session(page)
        self.session.on("Debugger.scriptParsed", self._parsed)
        self.session.send("Profiler.enable")
        self.session.send("Profiler.startPreciseCoverage", {"callCount": True, "detailed": True})
        self.session.send("Debugger.enable")

    def _parsed(self, event: JsonObject) -> None:
        if event.get("scriptLanguage") == "WebAssembly":
            return  # Rust/Wasm coverage belongs to the Rust collection pipeline.
        url = event.get("url")
        if isinstance(url, str):
            target = urlsplit(url)
            if f"{target.scheme}://{target.netloc}" == self.origin:
                identifier = string_value(event.get("scriptId"))
                # The event includes the source's UTF-8 SHA-256 and its UTF-16
                # length. Keeping them here survives navigation without a later
                # source lookup. Event handlers make no reentrant protocol calls.
                digest = string_value(event.get("hash"))
                if not re.fullmatch(r"[0-9a-f]{64}", digest):
                    raise ToolingError("Chromium did not provide a source SHA-256")
                self.local_scripts[identifier] = (digest, nonnegative(event.get("length")))

    def finish(self) -> None:
        if self.finished:
            return
        records: list[JsonObject] = []
        self._write(records, False)
        if self.page.is_closed():
            if self.local_scripts:
                raise ToolingError(
                    "Capture JavaScript coverage before closing a script-bearing page"
                )
        else:
            response = object_value(self.session.send("Profiler.takePreciseCoverage"))
            for item in array_value(response.get("result")):
                script = object_value(item)
                identifier = string_value(script.get("scriptId"))
                if identifier not in self.local_scripts:
                    continue
                digest, length = self.local_scripts[identifier]
                records.append(
                    {
                        "sha256": digest,
                        "length": length,
                        "functions": script.get("functions"),
                    }
                )
                self._write(records, False)
            self.session.send("Profiler.stopPreciseCoverage")
            self.session.send("Profiler.disable")
            self.session.send("Debugger.disable")
            self.session.detach()
        self._write(records, True)
        self.finished = True

    def _write(self, records: list[JsonObject], complete: bool) -> None:
        self.filesystem.write(
            self.path,
            json.dumps({"version": 1, "complete": complete, "captures": records}) + "\n",
            mode=0o600,
        )
