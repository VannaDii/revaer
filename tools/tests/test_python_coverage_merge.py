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
from revaer_tooling.e2e.database import uses_single_init
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
    monkeypatch.setenv("E2E_BROWSERS", "chromium")
    monkeypatch.setenv("E2E_PLAYWRIGHT_PROJECTS", "api-none,api-api-key,ui-chromium")
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
        tmp_path / ".test-tmp/e2e/test-results/python-e2e-summary.json",
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
    assert main(["python-coverage-merge", "--environment-ready"]) == 0
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
        path = context.root / ".test-tmp/e2e/test-results/python-e2e-summary.json"
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


@pytest.mark.parametrize(
    "lock", (".test-tmp/e2e/runtime/e2e.lock", "coverage/.python-coverage.lock")
)
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


def test_merge_combines_real_data_from_separate_producer_directories(
    coverage_context: Context,
) -> None:
    context = Context(
        root=coverage_context.root,
        settings=coverage_context.settings,
        options=Options(expected_shards=1),
        tools=coverage_context.tools,
        fs=coverage_context.fs,
        emit=coverage_context.emit,
        host=coverage_context.host,
        invoked_at=coverage_context.invoked_at,
        command_names=coverage_context.command_names,
    )
    producers = context.root / "coverage/producers"
    tooling = producers / "tooling-coverage"
    tooling.mkdir(parents=True)
    shutil.move(context.root / ".coverage", tooling / ".coverage")
    api_phases = tuple(
        phase
        for phase in context.settings.e2e.phases(media=uses_single_init(context))
        if phase.startswith("api-")
    )
    groups = (
        ("api-e2e-coverage", api_phases),
        ("ui-e2e-chromium", ("ui-chromium",)),
    )
    for name, phases in groups:
        directory = producers / name
        results = directory / ".test-tmp/e2e/test-results"
        results.mkdir(parents=True)
        (results / "python-e2e-summary.json").write_text(
            json.dumps(
                {
                    "status": "passed",
                    "shard": 1,
                    "total_shards": 1,
                    "phases": dict.fromkeys(phases, "passed"),
                }
            )
        )
        for phase in phases:
            coverage = directory / "coverage/e2e" / phase
            coverage.mkdir(parents=True)
            shutil.move(context.root / "coverage/e2e" / phase / ".coverage", coverage / ".coverage")
            (results / f"selection-{phase}-main.json").write_text(
                json.dumps(
                    {
                        "phase": phase,
                        "shard": 1,
                        "total_shards": 1,
                        "all": ["scenario"],
                        "selected": ["scenario"],
                    }
                )
            )
            label = "ui" if phase.startswith("ui-") else "api"
            (results / f"{label}-coverage-{phase}-main.json").write_text('["/observed"]')
    PythonCoverageMerge.run(context)
    report = ElementTree.parse(context.root / "coverage/python.xml")
    assert {
        entry.attrib["filename"] for entry in report.findall("./packages/package/classes/class")
    } == {
        "tools/src/revaer_tooling/cli.py",
        "tools/src/revaer_tooling/never.py",
        "tests/cli.py",
    }
    (producers / "api-e2e-coverage/coverage/e2e/api-api-key/.coverage").write_text("corrupt")
    with pytest.raises(ToolingError):
        PythonCoverageMerge.run(context)
    assert not (context.root / "coverage/python.xml").exists()
