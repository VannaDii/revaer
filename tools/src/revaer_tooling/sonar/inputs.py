"""Validate executed coverage before the scanner can consume it.

These checks preserve the existing Rust, browser, shell, and native evidence
requirements. Python replaces the retired Ruby/Node tooling coverage producer;
every authored Python file must have an unambiguous checkout-relative record.
"""

from pathlib import Path, PurePosixPath
from xml.etree import ElementTree

from ..errors import ToolingError

NATIVE_SOURCE = "crates/revaer-torrent-libt/src/ffi/session.cpp"


def _xml(document: str) -> ElementTree.Element:
    try:
        # Reports need no DTD or entity expansion. Reject declarations instead
        # of letting producer-controlled input change the parser's behavior.
        if "<!DOCTYPE" in document or "<!ENTITY" in document:
            raise ToolingError("Coverage XML must not declare a DTD or entities")
        return ElementTree.fromstring(document)
    except ElementTree.ParseError as error:
        raise ToolingError("Coverage report is not valid XML") from error


def verify_python_coverage(document: str, root: Path, expected: tuple[str, ...]) -> None:
    report = _xml(document)
    if report.tag != "coverage" or not expected:
        raise ToolingError("Python coverage must describe the authored Python inventory")
    sources = report.findall("./sources/source")
    if not sources or any((source.text or "") not in ("", ".") for source in sources):
        raise ToolingError("Python coverage must use the checkout as its source root")
    found: set[str] = set()
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
        found.add(name)
    if found != set(expected):
        raise ToolingError(
            "Python coverage omits authored files: " + ", ".join(sorted(set(expected) - found))
        )


def verify_native_coverage(document: str, *, required: bool) -> None:
    """Check source binding; the upstream producer enforces native execution."""
    present = any(
        line.endswith("/" + NATIVE_SOURCE + ":") or line == NATIVE_SOURCE + ":"
        for line in document.splitlines()
    )
    if required and not present:
        raise ToolingError("Native LLVM report does not contain authored session.cpp coverage")
