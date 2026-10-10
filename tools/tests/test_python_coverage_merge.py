"""Real Coverage.py data keeps full paths and never-imported authored files."""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from xml.etree import ElementTree

import pytest
from revaer_tooling.cli import main, make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks.python_coverage import PythonCoverageMerge


@pytest.fixture(params=(False, True))
def coverage_context(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, request: pytest.FixtureRequest
) -> Context:
    source = Path(__file__).resolve().parents[2]
    package = tmp_path / "tools/src/revaer_tooling"
    package.mkdir(parents=True)
    (package / "cli.py").write_text("value = 1\n")
    (package / "never.py").write_text("unexecuted = 2\n")
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "cli.py").write_text("value = 3\n")
    shutil.copy2(source / "tools/coverage-report.toml", tmp_path / "tools/coverage-report.toml")
    subprocess.run(["git", "init", str(tmp_path)], check=True, capture_output=True, timeout=10)
    monkeypatch.chdir(tmp_path)
    monkeypatch.delenv("E2E_PROJECTS", raising=False)
    if request.param:
        (tmp_path / "config").mkdir()
        (tmp_path / "config/database-rebaseline.env").write_text(
            "TRANSITION_PHASE=feature-development\n"
        )
    context = make_context(Options())
    phases = context.settings.e2e.phases(media=request.param)
    for phase in ("tooling", *phases):
        data = (
            tmp_path / ".coverage"
            if phase == "tooling"
            else tmp_path / "coverage/e2e" / phase / ".coverage"
        )
        data.parent.mkdir(parents=True, exist_ok=True)
        program = "tools/src/revaer_tooling/cli.py" if phase == "tooling" else "tests/cli.py"
        subprocess.run(
            [
                sys.executable,
                "-m",
                "coverage",
                "run",
                "--rcfile=tools/coverage-report.toml",
                program,
            ],
            cwd=tmp_path,
            env={**os.environ, "COVERAGE_FILE": str(data)},
            check=True,
            capture_output=True,
            timeout=15,
        )
    context.fs.write(
        tests / "test-results/python-e2e-summary.json",
        json.dumps(
            {
                "status": "passed",
                "shard": 1,
                "total_shards": 1,
                "phases": dict.fromkeys(phases, "passed"),
            }
        ),
    )
    return context


def test_merge_uses_full_paths_and_includes_never_executed_files(coverage_context: Context) -> None:
    context = coverage_context
    assert main(["python-coverage-merge", "--e2e-environment-ready"]) == 0
    report = ElementTree.parse(context.root / "coverage/python.xml")
    entries = {
        entry.attrib["filename"]: entry
        for entry in report.findall("./packages/package/classes/class")
    }
    assert set(entries) == {
        "tools/src/revaer_tooling/cli.py",
        "tools/src/revaer_tooling/never.py",
        "tests/cli.py",
    }
    for name in ("tools/src/revaer_tooling/cli.py", "tests/cli.py"):
        assert [line.attrib["hits"] for line in entries[name].findall("./lines/line")] == ["1"]
    assert [
        line.attrib["hits"]
        for line in entries["tools/src/revaer_tooling/never.py"].findall("./lines/line")
    ] == ["0"]
    assert (context.root / ".coverage").is_file()
    assert (
        "SF:tools/src/revaer_tooling/never.py"
        in (context.root / "coverage/python.lcov").read_text()
    )
    before = (context.root / "coverage/python.lcov").read_text()
    PythonCoverageMerge.run(context)
    assert (context.root / "coverage/python.lcov").read_text() == before


@pytest.mark.parametrize("case", ("incomplete", "missing-phase", "corrupt-data"))
def test_partial_evidence_cannot_reuse_an_old_merged_report(
    coverage_context: Context, case: str
) -> None:
    context = coverage_context
    PythonCoverageMerge.run(context)
    if case == "incomplete":
        path = context.root / "tests/test-results/python-e2e-summary.json"
        path.write_text(path.read_text().replace('"passed"', '"failed"'))
    else:
        data = context.root / "coverage/e2e" / context.settings.e2e.phases()[0] / ".coverage"
        if case == "missing-phase":
            data.unlink()
        else:
            data.write_text("not a coverage database")
    with pytest.raises(ToolingError):
        PythonCoverageMerge.run(context)
    assert not (context.root / "coverage/python.xml").exists()
    assert not (context.root / "coverage/python.lcov").exists()


@pytest.mark.parametrize("lock", ("tests/.runtime/e2e.lock", "coverage/.python-coverage.lock"))
def test_merge_cannot_read_while_either_producer_is_running(
    coverage_context: Context, lock: str
) -> None:
    context = coverage_context
    PythonCoverageMerge.run(context)
    path = context.root / "coverage/python.xml"
    previous = path.read_bytes()
    with context.fs.lock(context.root / lock), pytest.raises(ToolingError, match="holds"):
        PythonCoverageMerge.run(context)
    assert path.read_bytes() == previous
