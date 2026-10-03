"""Corrupt native evidence must fail instead of becoming a successful report.

These small XML documents model report corruption only. The real executable
locations and execution counts are exercised by test_bootstrap.py using kcov.
"""

import hashlib
import json
from pathlib import Path
from xml.etree import ElementTree

import pytest
from revaer_tooling.bootstrap.coverage import BOOTSTRAP_CASES, merge_records
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.sonar.inputs import bootstrap_records

SOURCE = b"#!/usr/bin/env bash\nif false; then\n    printf never\nfi\n"


@pytest.fixture
def records(tmp_path: Path) -> Path:
    filesystem = FileSystem()
    for name, code in BOOTSTRAP_CASES.items():
        directory = tmp_path / name
        script = str(tmp_path / name / "source/setup.sh")
        filesystem.write(
            directory / "source.json",
            json.dumps(
                {
                    "version": 1,
                    "source": script,
                    "sha256": hashlib.sha256(SOURCE).hexdigest(),
                    "exit_code": code,
                }
            ),
        )
        document = ElementTree.Element("coverage", {"version": "1"})
        file = ElementTree.SubElement(document, "file", {"path": script})
        ElementTree.SubElement(file, "lineToCover", {"lineNumber": "2", "covered": "true"})
        ElementTree.SubElement(file, "lineToCover", {"lineNumber": "3", "covered": "false"})
        filesystem.write(
            directory / "kcov/run/sonarqube.xml", ElementTree.tostring(document, encoding="unicode")
        )
    return tmp_path


def test_merge_retains_lines_that_no_execution_covered(records: Path) -> None:
    result = merge_records(FileSystem(), records, SOURCE)
    assert bootstrap_records(result, "setup.sh") == {2: True, 3: False}
    assert merge_records(FileSystem(), records, SOURCE) == result


@pytest.mark.parametrize(
    "case",
    ("hash", "exit", "source", "missing", "outside", "line", "inventory", "duplicate", "dtd"),
)
def test_incomplete_or_changed_measurements_fail(records: Path, case: str) -> None:
    directory = records / "installed"
    identity_path = directory / "source.json"
    identity = json.loads(identity_path.read_text())
    report = directory / "kcov/run/sonarqube.xml"
    if case in ("hash", "exit", "source"):
        key, value = {
            "hash": ("sha256", "0" * 64),
            "exit": ("exit_code", True),
            "source": ("source", "relative/setup.sh"),
        }[case]
        identity[key] = value
        identity_path.write_text(json.dumps(identity))
    elif case == "missing":
        report.unlink()
    elif case == "outside":
        outside = records / "outside.xml"
        report.rename(outside)
        report.symlink_to(outside)
    elif case == "dtd":
        report.write_text('<!DOCTYPE coverage><coverage version="1"/>')
    else:
        document = ElementTree.parse(report)
        file = document.getroot().find("file")
        assert file is not None
        first = file.find("lineToCover")
        assert first is not None
        if case == "line":
            first.set("lineNumber", "999")
        elif case == "inventory":
            file.remove(first)
        else:
            ElementTree.SubElement(file, "lineToCover", dict(first.attrib))
        document.write(report)
    with pytest.raises(ToolingError):
        merge_records(FileSystem(), records, SOURCE)
