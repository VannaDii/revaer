"""Validate the selected checkout's manifests before acquiring or probing files.

The foundation has a smaller fixture inventory than the media stack. Commands
operate on these records rather than embedding either checkout's fixture count.
Hashes, bounded sizes, unique paths and generation ownership remain mandatory.
"""

import hashlib
import re
import stat
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

from ..errors import ToolingError
from ..json_data import Json, JsonObject, array_value, decode, object_value, string_value

F1_ID = "mkv-theora-vorbis-live-style"
BOUNDED_ID = "chromium-bear-1280x720-av-frag-mp4"
FIXTURE_DIRECTORIES = ("source", "matroska", "chromium", "derived")


def https_url(value: str) -> str:
    try:
        parsed = urlsplit(value)
        port = parsed.port
    except ValueError as error:
        raise ToolingError("Fixture URL has an invalid authority or port") from error
    if (
        parsed.scheme != "https"
        or not parsed.hostname
        or parsed.username is not None
        or parsed.password is not None
        or parsed.fragment
        or port == 0
        or any(character.isspace() or ord(character) < 32 for character in value)
    ):
        raise ToolingError("Fixture URLs must use credential-free HTTPS")
    return value


def owned_path(root: Path, path: Path | str) -> Path:
    """Reject linked components and traversal before reading or mutating fixtures."""
    candidate = root / path
    if ".." in candidate.parts or not candidate.is_relative_to(root) or candidate == root:
        raise ToolingError(f"Fixture path must stay inside the selected checkout: {path}")
    for component in (candidate, *candidate.parents):
        if component == root:
            break
        if component.is_symlink():
            raise ToolingError(f"Fixture paths must not follow symlinks: {component}")
    return candidate


def regular_file(path: Path) -> None:
    if path.is_symlink() or not path.is_file():
        raise ToolingError(f"Fixture file is missing or linked: {path}")
    information = path.stat()
    if not stat.S_ISREG(information.st_mode) or information.st_nlink != 1:
        raise ToolingError(f"Fixture file must be regular with a single link: {path}")


def digest(path: Path) -> str:
    regular_file(path)
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


@dataclass(frozen=True)
class Source:
    id: str
    path: str
    encoding: str
    sha256: str
    minimum: int
    maximum: int
    urls: tuple[str, ...]

    def verify(self, path: Path) -> None:
        regular_file(path)
        if not self.minimum <= path.stat().st_size <= self.maximum:
            raise ToolingError(f"Fixture {self.id} violates its locked byte bounds")
        if digest(path) != self.sha256:
            raise ToolingError(f"Fixture {self.id} violates its locked SHA-256")

    @property
    def encoded_maximum(self) -> int:
        return ((self.maximum + 2) // 3) * 4 + 4 if self.encoding == "base64" else self.maximum


@dataclass(frozen=True)
class Fixture:
    id: str
    path: str
    source: str
    generate: bool
    download: bool
    bounded_diagnostics: bool
    exact_diagnostic: bool


def _identifier(value: Json) -> str:
    name = string_value(value)
    if re.fullmatch(r"[a-z0-9][a-z0-9-]*", name) is None:
        raise ToolingError("Fixture IDs must contain lowercase letters, digits and hyphens")
    return name


def _path(value: Json, *, generated: bool) -> str:
    path = string_value(value)
    directories = "derived" if generated else "source|matroska|chromium"
    if re.fullmatch(rf"test-fixtures/({directories})/[A-Za-z0-9._-]+", path) is None:
        raise ToolingError(f"Invalid fixture destination: {path}")
    return path


def _positive(value: Json) -> int:
    if type(value) is not int or value < 1:
        raise ToolingError("Fixture byte bounds must be positive integers")
    return value


def _boolean(document: JsonObject, key: str, *, optional: bool = False) -> bool:
    value = document.get(key, False if optional else None)
    if not isinstance(value, bool):
        raise ToolingError(f"Fixture {key} must be a boolean")
    return value


def document(path: Path, field: str) -> tuple[JsonObject, tuple[JsonObject, ...]]:
    regular_file(path)
    data = object_value(decode(path.read_text(encoding="utf-8")))
    if type(data.get("schemaVersion")) is not int or data["schemaVersion"] != 1:
        raise ToolingError("Fixture manifests require schemaVersion 1")
    entries = tuple(object_value(item) for item in array_value(data.get(field)))
    if not entries:
        raise ToolingError(f"Fixture {field} must not be empty")
    for key in ("id", "path"):
        values = tuple(string_value(item.get(key)) for item in entries)
        if len(set(values)) != len(values):
            raise ToolingError(f"Fixture {field} has duplicate {key} values")
    return data, entries


def sources(entries: tuple[JsonObject, ...]) -> tuple[Source, ...]:
    parsed: list[Source] = []
    for item in entries:
        encoding = string_value(item.get("encoding"))
        sha256 = string_value(item.get("sha256"))
        low, high = _positive(item.get("minimumBytes")), _positive(item.get("maximumBytes"))
        urls = tuple(https_url(string_value(url)) for url in array_value(item.get("urls")))
        if encoding not in ("raw", "base64") or re.fullmatch("[a-f0-9]{64}", sha256) is None:
            raise ToolingError("Fixture sources need raw/base64 encoding and a SHA-256")
        if high < low or not urls:
            raise ToolingError("Fixture sources need ordered byte bounds and HTTPS URLs")
        parsed.append(
            Source(
                _identifier(item.get("id")),
                _path(item.get("path"), generated=False),
                encoding,
                sha256,
                low,
                high,
                urls,
            )
        )
    return tuple(parsed)


def fixtures(entries: tuple[JsonObject, ...]) -> tuple[Fixture, ...]:
    parsed: list[Fixture] = []
    for item in entries:
        name = _identifier(item.get("id"))
        generated, download, generate = (
            _boolean(item, key) for key in ("generated", "shouldDownload", "shouldGenerate")
        )
        bounded = _boolean(item, "allowProbeDiagnostics", optional=True)
        contract = item.get("probeDiagnosticContract")
        if generated != generate or download == generate:
            raise ToolingError("Fixtures must select exactly one source or generation owner")
        if bounded and name != BOUNDED_ID:
            raise ToolingError("Only the reviewed Chromium fixture permits bounded diagnostics")
        if (name == F1_ID and "allowProbeDiagnostics" in item) or (
            "probeDiagnosticContract" in item and (contract != "adr578-f1" or name != F1_ID)
        ):
            raise ToolingError("Fixture probe diagnostic contract is invalid")
        parsed.append(
            Fixture(
                name,
                _path(item.get("path"), generated=generated),
                string_value(item.get("source")),
                generate,
                download,
                bounded,
                contract == "adr578-f1",
            )
        )
    return tuple(parsed)
