"""Coverage integrity tests retain zeros and require actual native inputs."""

from pathlib import Path

import pytest
from revaer_tooling.errors import ToolingError
from revaer_tooling.sonar.inputs import (
    verify_native_coverage,
    verify_python_coverage,
)

PYTHON = (
    "<coverage><sources><source>.</source></sources><packages><package><classes>"
    '<class filename="tools/module.py"><lines><line number="1" hits="1"/>'
    "</lines></class></classes></package></packages></coverage>"
)


@pytest.fixture
def python_root(tmp_path: Path) -> Path:
    (tmp_path / "tools").mkdir()
    (tmp_path / "tools/module.py").write_text("value = 1\n")
    return tmp_path


def test_python_coverage_requires_the_complete_authored_inventory(python_root: Path) -> None:
    verify_python_coverage(PYTHON, python_root, ("tools/module.py",))
    with pytest.raises(ToolingError, match="omits authored"):
        verify_python_coverage(PYTHON, python_root, ("tools/module.py", "unexecuted.py"))
    path = python_root / "tools/module.py"
    path.unlink()
    with pytest.raises(ToolingError, match="missing or linked"):
        verify_python_coverage(PYTHON, python_root, ("tools/module.py",))


@pytest.mark.parametrize(
    "before,after",
    (
        ("<source>.</source>", "<source>tools</source>"),
        ("tools/module.py", "module.py"),
        ("tools/module.py", "../module.py"),
    ),
)
def test_ambiguous_or_invalid_python_records_fail(
    python_root: Path, before: str, after: str
) -> None:
    with pytest.raises(ToolingError):
        verify_python_coverage(PYTHON.replace(before, after), python_root, ("tools/module.py",))


def test_native_report_requires_the_expected_source_only_when_configured() -> None:
    rust = "crates/example/src/lib.rs:\n  1| 1|rust\n"
    verify_native_coverage(rust, required=False)
    with pytest.raises(ToolingError, match="does not contain"):
        verify_native_coverage(rust, required=True)
