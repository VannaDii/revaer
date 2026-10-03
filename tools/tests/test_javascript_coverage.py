"""Real Chromium proves source identity, branch counts, navigation and closure."""

import json
import subprocess
from pathlib import Path

import pytest
import tree_sitter_javascript
from playwright.sync_api import Route, sync_playwright
from revaer_tooling.cli import main, make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.e2e.javascript import PageCoverage, parse_inventory, write_inventory
from revaer_tooling.e2e.v8 import JavaScriptSyntax, source_digest
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.tasks.javascript_coverage import JavaScriptCoverageMerge
from tree_sitter import Language, Parser

# Source fixtures are measured data. The parser sees the astral character and
# CRLF bytes that would invalidate an offset conversion based on Python indices.
SOURCE = """const label = "🌊";
function choose(flag) {
    if (flag) {
        document.body.dataset.chosen = label;
    } else {
        document.body.dataset.unused = "not executed";
    }
}
document.querySelector("button").addEventListener("click", () => choose(true));
""".replace("\n", "\r\n")
UNUSED = (
    'import * as dependency from "https://example.invalid/never-fetch.mjs";\n'
    'throw new Error("inventory parsing must not execute this script");\n'
)


@pytest.fixture(params=("classic", "module"))
def measured(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, request: pytest.FixtureRequest
) -> Context:
    subprocess.run(["git", "init", str(tmp_path)], check=True, capture_output=True, timeout=10)
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    (tmp_path / ".gitignore").write_text("coverage/\ntests/test-results/\n")
    script_source = SOURCE + ("export {};\r\n" if request.param == "module" else "")
    script_type = ' type="module"' if request.param == "module" else ""
    for name, source in (
        ("app.js", script_source),
        ("mirror.js", script_source),
        ("unused.js", UNUSED),
    ):
        (tmp_path / name).write_bytes(source.encode())
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("E2E_BROWSER_COVERAGE", "1")
    context = make_context(Options())
    results = tmp_path / "tests/test-results"

    def serve(route: Route) -> None:
        if route.request.url.endswith(".js"):
            route.fulfill(content_type="application/javascript; charset=utf-8", body=script_source)
        else:
            route.fulfill(
                content_type="text/html; charset=utf-8",
                body=f'<button>Run</button><script{script_type} src="/app.js"></script>',
            )

    with sync_playwright() as playwright, playwright.chromium.launch() as browser:
        syntax = JavaScriptSyntax(Parser(Language(tree_sitter_javascript.language())))
        scripts = parse_inventory(syntax, tmp_path, ("app.js", "mirror.js", "unused.js"))
        assert len(scripts) == 2
        assert scripts[source_digest(UNUSED)].points
        write_inventory(context.fs, results / "javascript-baseline-main.json", scripts)
        with browser.new_context() as web:
            page = web.new_page()
            capture = PageCoverage(
                page,
                context.settings.e2e.ui_url,
                results / "ui-chromium/scenario/javascript-page-1.json",
                context.fs,
            )
            page.route("**/*", serve)
            page.goto(context.settings.e2e.ui_url)
            page.get_by_role("button").click()
            # Old execution contexts must remain available after navigation.
            page.goto(context.settings.e2e.ui_url + "/next")
            page.get_by_role("button").click()
            capture.finish()
            capture.finish()  # Explicit early capture and fixture cleanup compose.
    context.fs.write(
        results / "python-e2e-summary.json",
        json.dumps(
            {
                "status": "passed",
                "phases": dict.fromkeys(context.settings.e2e.phases(), "passed"),
                "shard": 1,
                "total_shards": 1,
                "browser_coverage": True,
            }
        ),
    )
    return context


def lcov_lines(document: str) -> dict[str, dict[int, int]]:
    result: dict[str, dict[int, int]] = {}
    source = ""
    for line in document.splitlines():
        if line.startswith("SF:"):
            source = line[3:]
            result[source] = {}
        if line.startswith("DA:"):
            number, count = line[3:].split(",")
            result[source][int(number)] = int(count)
    return result


def test_real_parser_and_execution_keep_unexecuted_code_at_zero(measured: Context) -> None:
    assert main(["js-coverage-merge"]) == 0
    path = measured.root / "coverage/js-lcov.info"
    original = path.read_text()
    lines = lcov_lines(original)
    assert set(lines) == {"app.js", "mirror.js", "unused.js"}
    assert lines["app.js"] == lines["mirror.js"]
    assert lines["app.js"][4] >= 2
    assert lines["app.js"][6] == 0
    assert lines["unused.js"] == {1: 0, 2: 0}
    # The implicit EOF return after CRLF must not create a nonexistent line.
    assert max(lines["app.js"]) <= len((measured.root / "app.js").read_text().splitlines())
    JavaScriptCoverageMerge.run(measured)
    assert path.read_text() == original


@pytest.mark.parametrize(
    "case", ("source", "missing-source", "partial", "failed-run", "counter", "location")
)
def test_incomplete_or_changed_evidence_never_reuses_a_report(measured: Context, case: str) -> None:
    JavaScriptCoverageMerge.run(measured)
    results = measured.root / "tests/test-results"
    if case == "source":
        (measured.root / "app.js").write_text("changed source\n")
    elif case == "missing-source":
        (measured.root / "additional.js").write_text("const extra = 1;\n")
    else:
        name = {
            "partial": "ui-chromium/scenario/javascript-page-1.json",
            "failed-run": "python-e2e-summary.json",
            "counter": "ui-chromium/scenario/javascript-page-1.json",
            "location": "javascript-baseline-main.json",
        }[case]
        path = results / name
        record = json.loads(path.read_text())
        if case == "partial":
            record["complete"] = False
        elif case == "failed-run":
            record["status"] = "failed"
        elif case == "counter":
            record["captures"][0]["functions"][0]["ranges"][0]["count"] = -1
        else:
            record["sources"][0]["points"][0]["line"] = 0
        path.write_text(json.dumps(record))
    with pytest.raises(ToolingError):
        JavaScriptCoverageMerge.run(measured)
    assert not (measured.root / "coverage/js-lcov.info").exists()


def test_early_closed_script_page_fails_and_retains_partial_evidence(tmp_path: Path) -> None:
    with (
        sync_playwright() as playwright,
        playwright.chromium.launch() as browser,
        browser.new_context() as context,
    ):
        page = context.new_page()
        path = tmp_path / "capture.json"
        capture = PageCoverage(page, "http://localhost:8080", path, FileSystem())
        page.route(
            "**/*",
            lambda route: route.fulfill(
                content_type="text/html",
                body='<script>document.title = "recorded";</script>',
            ),
        )
        page.goto("http://localhost:8080")
        page.close()
        with pytest.raises(ToolingError, match="before closing"):
            capture.finish()
    assert json.loads(path.read_text())["complete"] is False


def test_invalid_script_does_not_turn_into_a_zero_coverage_success(tmp_path: Path) -> None:
    (tmp_path / "invalid.js").write_text("function invalid(\n")
    syntax = JavaScriptSyntax(Parser(Language(tree_sitter_javascript.language())))
    with pytest.raises(ToolingError, match="parse error"):
        parse_inventory(syntax, tmp_path, ("invalid.js",))
