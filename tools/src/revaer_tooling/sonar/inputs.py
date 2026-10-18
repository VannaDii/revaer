"""Validate executed coverage before the scanner can consume it.

These checks preserve the existing Rust, browser, shell, and native evidence
requirements. Python replaces the retired Ruby/Node tooling coverage producer;
every authored Python file must have an unambiguous checkout-relative record.
"""

import os
import re
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from xml.etree import ElementTree

from ..errors import ToolingError
from ..policy.sonar import vendor_source

NATIVE_SOURCE = "crates/revaer-torrent-libt/src/ffi/session.cpp"


@dataclass(frozen=True)
class LineCounts:
    total: int
    covered: int


def rust_source(path: Path, root: Path, inventory: set[str]) -> str:
    absolute = Path(os.path.normpath(path if path.is_absolute() else root / path))
    source = absolute.relative_to(root).as_posix() if absolute.is_relative_to(root) else ""
    if source.endswith(".rs") and source not in inventory and not source.startswith("target/"):
        raise ToolingError("Rust LCOV contains an untracked checkout source: " + source)
    return source


def rust_branch(line: str) -> str:
    if not line.startswith("BRDA:"):
        return line
    fields = line[5:].split(",")
    if (
        len(fields) != 4
        or not all(value.isdecimal() for value in fields[:3])
        or not (fields[3].isdecimal() or fields[3] == "-")
    ):
        raise ToolingError("Rust LCOV contains an invalid branch record")
    # Preserve unexecuted branches and their uncovered denominator.
    if fields[3] == "-":
        return "BRDA:" + ",".join((*fields[:3], "0"))
    return line


def rust_sonar_lcov(document: str, root: Path, tracked: tuple[str, ...]) -> str:
    """Adapt complete LLVM records to Sonar without changing authored measurements."""
    lcov_counts(document)
    inventory = {name for name in tracked if name.endswith(".rs")}
    output: list[str] = []
    record: list[str] = []
    source = ""
    for line in document.splitlines():
        if line.startswith("SF:"):
            source = rust_source(Path(line[3:]), root, inventory)
            record = ["SF:" + source]
        elif line == "end_of_record":
            if source in inventory and not vendor_source(source):
                output.extend((*record, line))
            record = []
            source = ""
        elif source in inventory:
            record.append(rust_branch(line))
    result = "\n".join(output) + "\n"
    if not lcov_counts(result, ".rs").covered:
        raise ToolingError("Sonar Rust LCOV must retain executed authored Rust records")
    return result


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
    """Validate genuine records without imposing a target on setup.sh."""
    observations = bootstrap_records(document, "setup.sh")
    if not observations:
        raise ToolingError("Bootstrap coverage contains no executable-line records")


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
