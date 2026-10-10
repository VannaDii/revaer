"""Coverage integrity tests retain zeros and require actual native inputs."""

from pathlib import Path

import pytest
from revaer_tooling.errors import ToolingError
from revaer_tooling.sonar.inputs import (
    NATIVE_SOURCE,
    LineCounts,
    lcov_counts,
    verify_bootstrap_coverage,
    verify_lcov,
    verify_native_coverage,
    verify_python_coverage,
)


def lcov(source: str, count: int, covered: int) -> str:
    return (
        f"SF:{source}\n"
        + "".join(f"DA:{number},{int(number <= covered)}\n" for number in range(1, count + 1))
        + "end_of_record\n"
    )


def test_coverage_requires_execution_of_production_sources_only() -> None:
    rust = lcov("crates/example/src/lib.rs", 2, 1)
    browser = lcov("crates/example/static/app.js", 2, 1)
    assert lcov_counts(rust) == LineCounts(2, 1)
    verify_lcov(rust, browser)
    verify_lcov(rust, lcov("app.js", 2, 2))
    verify_lcov(rust, lcov("docs/diagram.js", 2, 0))
    with pytest.raises(ToolingError, match="real source line records"):
        verify_lcov(rust, "")
    with pytest.raises(ToolingError, match="production JavaScript"):
        verify_lcov(rust, lcov("app.js", 2, 0))
    for bad in (lcov("app.rs", 2, 0), lcov("not-rust.cpp", 2, 2), ""):
        with pytest.raises(ToolingError, match="Rust"):
            verify_lcov(bad, browser)


@pytest.mark.parametrize(
    "document",
    (
        "SF:a.rs\nDA:1,1\n",
        "SF:a.rs\nSF:b.rs\n",
        "SF:\n",
        "DA:1,1\nend_of_record\n",
        "SF:a.rs\nDA:0,1\nend_of_record\n",
        "SF:a.rs\nDA:1,-1\nend_of_record\n",
        "SF:a.rs\nDA:1,1,checksum,extra\nend_of_record\n",
        "SF:a.rs\nDA:1,1\nDA:1,2\nend_of_record\n",
        "SF:a.rs\nend_of_record\nSF:a.rs\nend_of_record\n",
        "end_of_record\n",
    ),
)
def test_invalid_lcov_is_not_mistaken_for_execution(document: str) -> None:
    with pytest.raises(ToolingError):
        lcov_counts(document)


BOOTSTRAP = (
    '<coverage version="1"><file path="setup.sh">'
    '<lineToCover lineNumber="2" covered="true"/>'
    '<lineToCover lineNumber="3" covered="false"/></file></coverage>'
)


def test_bootstrap_generic_coverage_keeps_unexecuted_lines() -> None:
    verify_bootstrap_coverage(BOOTSTRAP)


def test_fully_executed_bootstrap_does_not_require_an_artificial_gap() -> None:
    verify_bootstrap_coverage(BOOTSTRAP.replace('covered="false"', 'covered="true"'))


@pytest.mark.parametrize(
    "document",
    (
        "broken XML",
        "<!DOCTYPE coverage><coverage/>",
        BOOTSTRAP.replace('version="1"', 'version="2"'),
        BOOTSTRAP.replace("setup.sh", "retired.sh"),
        BOOTSTRAP.replace('lineNumber="3"', 'lineNumber="2"'),
        BOOTSTRAP.replace('lineNumber="3"', 'lineNumber="0"'),
        BOOTSTRAP.replace('covered="false"', 'covered="unknown"'),
        BOOTSTRAP.replace('covered="true"', 'covered="false"'),
        '<coverage version="1"><file path="setup.sh"/></coverage>',
        BOOTSTRAP.replace("</file>", '<lineToCover lineNumber="4" covered="false"/></file>'),
    ),
)
def test_missing_or_fabricated_bootstrap_records_fail(document: str) -> None:
    with pytest.raises(ToolingError):
        verify_bootstrap_coverage(document)


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
        ('number="1"', 'number="2"'),
        ('hits="1"', 'hits="-1"'),
        ('hits="1"', 'hits="0"'),
        ('<line number="1" hits="1"/>', '<line number="1" hits="1"/><line number="1" hits="1"/>'),
    ),
)
def test_ambiguous_or_invalid_python_records_fail(
    python_root: Path, before: str, after: str
) -> None:
    with pytest.raises(ToolingError):
        verify_python_coverage(PYTHON.replace(before, after), python_root, ("tools/module.py",))


@pytest.mark.parametrize("count", ("1", "123", "1.2k", "1M", "2G"))
def test_native_report_accepts_real_llvm_count_formats(count: str) -> None:
    verify_native_coverage(f"/checkout/{NATIVE_SOURCE}:\n    1|  {count}|code\n", required=True)


def test_optional_native_omission_is_distinct_from_uncovered_native_source() -> None:
    rust = "crates/example/src/lib.rs:\n  1| 1|rust\n"
    verify_native_coverage(rust, required=False)
    with pytest.raises(ToolingError, match="does not contain"):
        verify_native_coverage(rust, required=True)
    for required in (True, False):
        with pytest.raises(ToolingError, match="no covered lines"):
            verify_native_coverage(f"{NATIVE_SOURCE}:\n  1| 0|native\n" + rust, required=required)
