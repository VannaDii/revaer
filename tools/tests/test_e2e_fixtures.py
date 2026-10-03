"""Real browser sessions prove storage, retries, artifacts and shard collection.

Each child pytest process loads the shipped conftest from a disposable checkout.
HTTP interception supplies a small page; no Revaer service or developer database
is needed to verify fixture ownership and browser recording semantics.
"""

import hashlib
import json
import os
import shutil
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import pytest
from revaer_tooling.e2e.settings import E2eSettings, load_e2e_settings
from revaer_tooling.errors import CommandError
from revaer_tooling.external.python import E2ePytestArgs, Python
from revaer_tooling.process import ProcessRunner


@pytest.fixture
def browser_project(tmp_path: Path) -> Python:
    source = Path(__file__).resolve().parents[2]
    for relative in (
        "tests/__init__.py",
        "tests/conftest.py",
        "tests/pages/__init__.py",
        "tests/pages/app_shell.py",
    ):
        destination = tmp_path / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source / relative, destination)
    scenarios = tmp_path / "tests/specs/ui/test_browser.py"
    scenarios.parent.mkdir(parents=True)
    shutil.copy2(source / "tools/tests/fixtures/browser_scenarios.py", scenarios)
    shutil.copytree(
        source / "tools/src/revaer_tooling",
        tmp_path / "tools/src/revaer_tooling",
        ignore=shutil.ignore_patterns("__pycache__"),
    )
    (tmp_path / "pyproject.toml").write_text(
        '[tool.pytest.ini_options]\naddopts="--strict-config --strict-markers"\n'
        'filterwarnings=["error"]\npythonpath=["tools/src"]\n'
    )
    environment = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith(("E2E_", "REVAER_E2E_", "PLAYWRIGHT_SHARD_", "PYTEST_"))
    }
    # This child tests the installed implementation and selected fixture files;
    # it must not inherit a parent coverage target or pytest selection.
    environment.pop("COVERAGE_PROCESS_START", None)
    environment["REVAER_E2E_API_KEY"] = "fixture-local-key"
    environment["REVAER_E2E_RESULTS_ROOT"] = str(tmp_path / "results")
    return Python(sys.executable, ProcessRunner(print), tmp_path, environment)


def run_browser(python: Python, settings: E2eSettings, browser: str = "chromium") -> Path:
    phase = "ui-" + browser
    name = phase + settings.shard_suffix
    result = python.root / "results" / name
    for directory in (result, python.root / "report" / name, python.root / "coverage"):
        directory.mkdir(parents=True, exist_ok=True)
    python.e2e(
        E2ePytestArgs(
            phase, settings, result, python.root / "report" / name, python.root / "coverage"
        ),
        {
            "E2E_RETRIES": str(settings.retries),
            "E2E_TRACE": settings.trace.value,
            "E2E_VIDEO": settings.video.value,
            "E2E_SCREENSHOT": settings.screenshot,
            "E2E_UI_WORKERS": str(settings.workers),
            "PLAYWRIGHT_SHARD_INDEX": str(settings.shard_index),
            "PLAYWRIGHT_SHARD_TOTAL": str(settings.shard_total),
            "E2E_BROWSER_COVERAGE": str(settings.browser_coverage),
        },
    )
    return result


def test_browser_line_coverage_uses_the_shipped_pytest_fixture(browser_project: Python) -> None:
    source = Path(__file__).parent / "fixtures/browser_javascript.py"
    shutil.copy2(source, browser_project.root / "tests/specs/ui/test_browser.py")
    script = 'document.title = "Collected";\n'
    (browser_project.root / "fixture.js").write_text(script)
    results = browser_project.root / "results"
    results.mkdir()
    (results / "javascript-sources.json").write_text('["fixture.js"]\n')
    settings = load_e2e_settings(
        {"E2E_BROWSER_COVERAGE": "true", "E2E_RETRIES": "0", "E2E_TRACE": "off", "E2E_VIDEO": "off"}
    )
    run_browser(browser_project, settings)
    baseline = json.loads((results / "javascript-baseline-main.json").read_text())
    assert baseline["sources"][0]["source"] == script
    pages = sorted((results / "ui-chromium").glob("*/javascript-page-*.json"))
    assert len(pages) == 2
    for path in pages:
        evidence = json.loads(path.read_text())
        assert evidence["complete"] is True
        assert any(
            capture["sha256"] == hashlib.sha256(script.encode()).hexdigest()
            for capture in evidence["captures"]
        )


def test_browser_coverage_failure_keeps_trace_video_and_partial_data(
    browser_project: Python,
) -> None:
    source = Path(__file__).parent / "fixtures/browser_javascript_failure.py"
    shutil.copy2(source, browser_project.root / "tests/specs/ui/test_browser.py")
    (browser_project.root / "fixture.js").write_text('document.title = "Collected";\n')
    results = browser_project.root / "results"
    results.mkdir()
    (results / "javascript-sources.json").write_text('["fixture.js"]\n')
    settings = load_e2e_settings(
        {
            "E2E_BROWSER_COVERAGE": "true",
            "E2E_RETRIES": "0",
            "E2E_TRACE": "retain-on-failure",
            "E2E_VIDEO": "retain-on-failure",
        }
    )
    with pytest.raises(CommandError):
        run_browser(browser_project, settings)
    attempts = list((results / "ui-chromium").glob("*attempt-1"))
    assert len(attempts) == 1
    assert (attempts[0] / "trace.zip").stat().st_size > 0
    assert (attempts[0] / "video-1.webm").stat().st_size > 0
    assert json.loads((attempts[0] / "javascript-page-1.json").read_text())["complete"] is False


@pytest.mark.parametrize("browser", ["chromium", "firefox", "webkit"])
def test_browser_retries_keep_evidence_closed_page_video_and_route_union(
    browser_project: Python,
    browser: str,
) -> None:
    settings = load_e2e_settings({"E2E_RETRIES": "1"})
    results = run_browser(browser_project, settings, browser)
    attempts = sorted(results.glob("*attempt-*"))
    failed = next(path for path in attempts if "retry" in path.name and "attempt-1" in path.name)
    retry = next(path for path in attempts if "retry" in path.name and "attempt-2" in path.name)
    assert len(list(failed.glob("*.webm"))) == 2
    assert (failed / "page-1.png").stat().st_size > 0
    assert not (failed / "trace.zip").exists()
    assert (retry / "trace.zip").stat().st_size > 0
    assert not list(retry.rglob("*.webm"))
    assert not list(retry.glob("*.png"))
    assert all(path.stat().st_size > 0 for path in failed.glob("*.webm"))
    assert json.loads((results.parent / f"ui-coverage-ui-{browser}-main.json").read_text()) == [
        "/",
        "/not-found",
        "/settings",
        "/torrents/:id",
    ]
    report = ET.parse(results / "junit.xml").getroot()
    suite = report.find("testsuite")
    assert suite is not None and suite.get("failures") == "0" and suite.get("tests") == "2"
    assert len(report.findall(".//property[@name='artifacts']")) >= 2
    assert (browser_project.root / f"report/ui-{browser}/index.html").stat().st_size > 0
    assert (browser_project.root / "coverage/python.xml").stat().st_size > 0


def test_shards_partition_complete_scenarios_and_accept_only_expected_empty_shards(
    browser_project: Python,
) -> None:
    universe: set[str] = set()
    selected: list[set[str]] = []
    for index in (1, 2, 3):
        settings = load_e2e_settings(
            {
                "E2E_RETRIES": "1",
                "E2E_TRACE": "off",
                "E2E_VIDEO": "off",
                "PLAYWRIGHT_SHARD_INDEX": str(index),
                "PLAYWRIGHT_SHARD_TOTAL": "3",
            }
        )
        run_browser(browser_project, settings)
        path = browser_project.root / f"results/selection-ui-chromium-shard-{index}-main.json"
        evidence = json.loads(path.read_text())
        universe = set(evidence["all"])
        selected.append(set(evidence["selected"]))
    assert len(universe) == 2
    assert set.union(*selected) == universe
    assert sum(map(len, selected)) == len(universe)
    assert not selected[-1]


def test_xdist_workers_preserve_route_union_and_expected_empty_shards(
    browser_project: Python,
) -> None:
    settings = load_e2e_settings({"E2E_RETRIES": "1", "E2E_UI_WORKERS": "2", "E2E_VIDEO": "off"})
    run_browser(browser_project, settings)
    files = sorted((browser_project.root / "results").glob("ui-coverage-ui-chromium-gw*.json"))
    assert len(files) == 2
    assert set.union(*(set(json.loads(path.read_text())) for path in files)) == {
        "/",
        "/settings",
        "/torrents/:id",
        "/not-found",
    }
    empty = load_e2e_settings(
        {
            "E2E_RETRIES": "1",
            "E2E_UI_WORKERS": "2",
            "E2E_VIDEO": "off",
            "PLAYWRIGHT_SHARD_INDEX": "3",
            "PLAYWRIGHT_SHARD_TOTAL": "3",
        }
    )
    results = run_browser(browser_project, empty)
    assert len(list(results.parent.glob("selection-ui-chromium-shard-3-gw*.json"))) == 2
    # Deleting every scenario must still fail, including in a parallel run.
    (browser_project.root / "tests/specs/ui/test_browser.py").unlink()
    with pytest.raises(CommandError):
        run_browser(browser_project, empty)


def test_setup_and_timeout_failures_close_contexts_and_keep_artifacts(
    browser_project: Python,
) -> None:
    source = Path(__file__).resolve().parent / "fixtures/browser_failures.py"
    shutil.copy2(source, browser_project.root / "tests/specs/ui/test_browser.py")
    settings = load_e2e_settings({"E2E_RETRIES": "0", "E2E_TRACE": "retain-on-failure"})
    with pytest.raises(CommandError):
        run_browser(browser_project, settings)
    results = browser_project.root / "results/ui-chromium"
    attempts = sorted(results.glob("*attempt-1"))
    assert len(attempts) == 2
    for path in attempts:
        assert (path / "trace.zip").stat().st_size > 0
        assert (path / "page-1.png").stat().st_size > 0
        assert (path / "video-1.webm").stat().st_size > 0
        assert not (path / "recordings").exists()
    report = ET.parse(results / "junit.xml").getroot()
    errors = [element.get("message", "") for element in report.findall(".//error")]
    failures = [element.get("message", "") for element in report.findall(".//failure")]
    assert len(errors) == 1 and "injected setup failure" in errors[0]
    assert len(failures) == 1 and "Timeout" in failures[0]


@pytest.mark.parametrize("phase", ("api-none", "ui-chromium"))
@pytest.mark.parametrize("include_media", (False, True))
def test_media_selection_adds_to_foundation_cases(
    browser_project: Python,
    include_media: bool,
    phase: str,
) -> None:
    kind = "ui" if phase.startswith("ui-") else "api"
    if kind == "ui":
        # Keep this probe about selection; native page behavior is covered above.
        (browser_project.root / "tests/specs/ui/test_browser.py").unlink()
    for relative, name in ((kind, "foundation"), (f"media/{kind}", "media")):
        destination = browser_project.root / "tests/specs" / relative
        destination.mkdir(parents=True, exist_ok=True)
        (destination / f"test_{name}.py").write_text(f"def test_{name}():\n    assert 2 + 2 == 4\n")
    output = browser_project.root / "selection-results"
    for name in ("results", "report", "coverage"):
        (output / name).mkdir(parents=True)
    browser_project.e2e(
        E2ePytestArgs(
            phase,
            load_e2e_settings({"E2E_RETRIES": "0"}),
            output / "results",
            output / "report",
            output / "coverage",
            include_media=include_media,
        ),
        {},
    )
    cases = {
        case.attrib["name"] for case in ET.parse(output / "results/junit.xml").iter("testcase")
    }
    assert cases == ({"test_foundation", "test_media"} if include_media else {"test_foundation"})
