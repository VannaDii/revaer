"""Bind isolated setup executions to the exact root script and merge native data."""

import hashlib
from pathlib import Path
from xml.etree import ElementTree

from ..errors import ToolingError
from ..filesystem import FileSystem
from ..json_data import decode, object_value, string_value
from ..sonar.inputs import bootstrap_records

# Each case has observable assertions in test_bootstrap.py. The stale-lock
# failure is required behavior, so its measured lines remain useful evidence.
BOOTSTRAP_CASES = {"installed": 0, "unmanaged": 0, "install-directory": 0, "stale-lock": 1}


def case_records(
    filesystem: FileSystem, directory: Path, source: bytes, expected_exit: int
) -> dict[int, bool]:
    identity = object_value(decode(filesystem.read(directory / "source.json")))
    script = string_value(identity.get("source"))
    if (
        type(identity.get("version")) is not int
        or identity.get("version") != 1
        or type(identity.get("exit_code")) is not int
        or identity.get("exit_code") != expected_exit
        or identity.get("sha256") != hashlib.sha256(source).hexdigest()
        or not Path(script).is_absolute()
        or Path(script).name != "setup.sh"
    ):
        raise ToolingError("Bootstrap execution does not match the current setup.sh or outcome")
    reports = {path.resolve() for path in (directory / "kcov").rglob("sonarqube.xml")}
    if len(reports) != 1:
        raise ToolingError("Each bootstrap execution must retain exactly one kcov report")
    report = reports.pop()
    if not report.is_relative_to(directory.resolve()) or not report.is_file():
        raise ToolingError("Kcov report must remain inside its owned case directory")
    records = bootstrap_records(filesystem.read(report), script)
    if not records or max(records) > len(source.splitlines()):
        raise ToolingError("Bootstrap coverage is empty or exceeds its source length")
    return records


def merge_records(filesystem: FileSystem, directory: Path, source: bytes) -> str:
    measurements = [
        case_records(filesystem, directory / name, source, code)
        for name, code in BOOTSTRAP_CASES.items()
    ]
    expected = set(measurements[0])
    if any(set(records) != expected for records in measurements):
        raise ToolingError("Bootstrap executions disagree on the executable-line inventory")
    document = ElementTree.Element("coverage", {"version": "1"})
    script = ElementTree.SubElement(document, "file", {"path": "setup.sh"})
    for number in sorted(expected):
        ElementTree.SubElement(
            script,
            "lineToCover",
            {
                "lineNumber": str(number),
                "covered": str(any(records[number] for records in measurements)).lower(),
            },
        )
    ElementTree.indent(document)
    return ElementTree.tostring(document, encoding="unicode") + "\n"
