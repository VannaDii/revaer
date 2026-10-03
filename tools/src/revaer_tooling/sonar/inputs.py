"""Validate executed coverage before the scanner can consume it.

These checks preserve the existing Rust, browser, shell, and native evidence
requirements. Python replaces the retired Ruby/Node tooling coverage producer;
every authored Python file must have an unambiguous checkout-relative record.
"""

import re
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from xml.etree import ElementTree

from ..errors import ToolingError

NATIVE_SOURCE = "crates/revaer-torrent-libt/src/ffi/session.cpp"


@dataclass(frozen=True)
class LineCounts:
    total: int
    covered: int


def lcov_counts(document: str, suffix: str = "") -> LineCounts:
    """Count LCOV line records, rejecting duplicate or malformed observations."""
    records: dict[tuple[str, int], int] = {}
    source = ""
    sources: set[str] = set()
    for line in document.splitlines():
        if line.startswith("SF:"):
            if source:
                raise ToolingError("LCOV source record is missing end_of_record")
            source = line[3:]
            if not source or source in sources:
                raise ToolingError("LCOV source paths must be nonempty and unique")
            sources.add(source)
        elif line.startswith("DA:"):
            fields = line[3:].split(",")
            if (
                not source
                or len(fields) not in (2, 3)
                or not fields[0].isdecimal()
                or int(fields[0]) < 1
                or not fields[1].isdecimal()
            ):
                raise ToolingError("LCOV contains an invalid line record")
            key = source, int(fields[0])
            if key in records:
                raise ToolingError("LCOV contains a duplicate line record")
            records[key] = int(fields[1])
        elif line == "end_of_record":
            if not source:
                raise ToolingError("LCOV contains an empty source record")
            source = ""
    if source:
        raise ToolingError("LCOV source record is missing end_of_record")
    hits = [count for (path, _), count in records.items() if path.endswith(suffix)]
    return LineCounts(len(hits), sum(count > 0 for count in hits))


def verify_lcov(rust: str, browser: str) -> None:
    rust_counts = lcov_counts(rust, ".rs")
    if not rust_counts.covered:
        raise ToolingError("Rust LCOV must contain positive Rust line coverage")
    browser_counts = lcov_counts(browser)
    if browser_counts.total < 1000:
        raise ToolingError("Browser LCOV has fewer than 1000 authored line records")
    if not 0 < browser_counts.covered < browser_counts.total:
        raise ToolingError("Browser LCOV must retain covered and uncovered lines")


def _xml(document: str) -> ElementTree.Element:
    try:
        # Reports need no DTD or entity expansion. Reject declarations instead
        # of letting producer-controlled input change the parser's behavior.
        if "<!DOCTYPE" in document or "<!ENTITY" in document:
            raise ToolingError("Coverage XML must not declare a DTD or entities")
        return ElementTree.fromstring(document)
    except ElementTree.ParseError as error:
        raise ToolingError("Coverage report is not valid XML") from error


def _line_number(element: ElementTree.Element, attribute: str) -> int:
    value = element.get(attribute, "")
    if not value.isdecimal() or int(value) < 1:
        raise ToolingError("Coverage line numbers must be positive integers")
    return int(value)


def bootstrap_records(document: str, source: str) -> dict[int, bool]:
    """Read exact native kcov line observations without changing their values."""
    report = _xml(document)
    if report.tag != "coverage" or report.get("version") != "1":
        raise ToolingError("Bootstrap coverage must use Sonar's generic version 1 format")
    files = report.findall("file")
    if len(files) != 1 or files[0].get("path") != source:
        raise ToolingError("Bootstrap coverage must describe the expected setup.sh")
    observations: dict[int, bool] = {}
    for line in files[0].findall("lineToCover"):
        number = _line_number(line, "lineNumber")
        state = line.get("covered", "")
        if number in observations or state not in ("true", "false"):
            raise ToolingError("Bootstrap coverage has duplicate or invalid line records")
        observations[number] = state == "true"
    return observations


def verify_bootstrap_coverage(document: str) -> None:
    """Allow at most one measured gap in root setup.sh (ADR 592).

    Full coverage is valid too. Preserve the producer's zero-hit records;
    accepting a gap must never manufacture execution or admit empty evidence.
    """
    observations = bootstrap_records(document, "setup.sh")
    if not any(observations.values()) or sum(not hit for hit in observations.values()) > 1:
        raise ToolingError(
            "Bootstrap coverage requires executed lines and at most one uncovered executable line"
        )


def verify_python_coverage(document: str, root: Path, expected: tuple[str, ...]) -> None:
    report = _xml(document)
    if report.tag != "coverage" or not expected:
        raise ToolingError("Python coverage must describe the authored Python inventory")
    sources = report.findall("./sources/source")
    if not sources or any((source.text or "") not in ("", ".") for source in sources):
        raise ToolingError("Python coverage must use the checkout as its source root")
    found: set[str] = set()
    covered = 0
    for entry in report.findall("./packages/package/classes/class"):
        name = entry.get("filename", "")
        path = PurePosixPath(name)
        if (
            name not in expected
            or name in found
            or path.is_absolute()
            or ".." in path.parts
            or path.as_posix() != name
        ):
            raise ToolingError("Python coverage contains an ambiguous or unexpected file path")
        source = root / name
        if source.is_symlink() or not source.is_file():
            raise ToolingError("Python coverage points to a missing or linked source file")
        last_line = len(source.read_text(encoding="utf-8").splitlines())
        numbers: set[int] = set()
        for line in entry.findall("./lines/line"):
            number = _line_number(line, "number")
            hits = line.get("hits", "")
            if number in numbers or number > last_line or not hits.isdecimal():
                raise ToolingError("Python coverage has duplicate or invalid line records")
            numbers.add(number)
            covered += int(hits) > 0
        found.add(name)
    if found != set(expected):
        raise ToolingError(
            "Python coverage omits authored files: " + ", ".join(sorted(set(expected) - found))
        )
    if not covered:
        raise ToolingError("Python coverage must contain executed line records")


def verify_native_coverage(document: str, *, required: bool) -> None:
    present = active = covered = False
    for line in document.splitlines():
        if line and not line[0].isspace() and line.endswith(":"):
            active = line[:-1].endswith("/" + NATIVE_SOURCE) or line[:-1] == NATIVE_SOURCE
            present |= active
        elif active:
            match = re.fullmatch(r"\s*\d+\|\s*(\d+(?:\.\d+)?)(?:[kMG])?\|.*", line)
            if match and float(match[1]) > 0:
                covered = True
    if present and not covered:
        raise ToolingError("Native LLVM report contains session.cpp but no covered lines")
    if required and not present:
        raise ToolingError("Native LLVM report does not contain authored session.cpp coverage")
