"""Application-test wiring: explicit phases, native browser state, and evidence.

The rv parent owns services and database cleanup. Pytest owns each browser
context and records every retry separately, without patching browser globals.
"""

import hashlib
import json
import os
import re
import time
from collections.abc import Callable, Generator, Iterator
from contextlib import ExitStack
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol
from urllib.parse import urlsplit
from urllib.request import build_opener

import pytest
import tree_sitter_javascript
from playwright.sync_api import Browser, BrowserContext, Page, Response, Video, expect
from revaer_tooling.e2e.api import ApiClient, ApiSchema, ApiSession
from revaer_tooling.e2e.coverage import RouteCoverage
from revaer_tooling.e2e.javascript import PageCoverage, parse_inventory, write_inventory
from revaer_tooling.e2e.settings import E2eSettings, load_e2e_settings
from revaer_tooling.e2e.v8 import JavaScriptSyntax
from revaer_tooling.external.http import Http, VisibleRedirects
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.json_data import array_value, decode, object_value, string_value
from tree_sitter import Language, Parser

from tests.pages.app_shell import AppShell

REPORTS = pytest.StashKey[dict[str, pytest.TestReport]]()
SELECTION = pytest.StashKey[tuple[int, int]]()
ARTIFACTS = pytest.StashKey[Path]()


class E2eWorker(Protocol):
    """The documented xdist hook surface; xdist does not distribute type hints."""

    config: pytest.Config
    workeroutput: dict[str, object]


@dataclass(frozen=True)
class E2eRuntime:
    root: Path
    settings: E2eSettings
    phase: str
    results: Path
    filesystem: FileSystem
    session: ApiSession


def phase() -> str:
    value = os.environ.get("REVAER_E2E_PHASE", "")
    if value not in ("api-none", "api-api-key", "ui-chromium", "ui-firefox", "ui-webkit"):
        raise pytest.UsageError("Run application tests with rv ui-e2e to provision their services")
    return value


def results_root(root: Path) -> Path:
    return Path(os.environ.get("REVAER_E2E_RESULTS_ROOT", str(root / "tests/test-results")))


def worker_name() -> str:
    worker = os.environ.get("PYTEST_XDIST_WORKER", "main")
    if re.fullmatch(r"[A-Za-z0-9_-]+", worker) is None:
        raise pytest.UsageError("Invalid pytest worker identifier")
    return worker


def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]) -> None:
    if config.option.collectonly:
        return
    settings = load_e2e_settings(os.environ)
    name = phase()
    # Partition complete scenario functions deterministically. Stateful CRUD
    # lifecycles stay within a function, so shards never split their steps.
    ordered = sorted(items, key=lambda item: item.nodeid)
    chosen = [
        item
        for index, item in enumerate(ordered)
        if index % settings.shard_total == settings.shard_index - 1
    ]
    selected = set(chosen)
    deselected = [item for item in items if item not in selected]
    config.hook.pytest_deselected(items=deselected)
    items[:] = chosen
    config.stash[SELECTION] = (len(ordered), len(chosen))
    path = (
        results_root(config.rootpath)
        / f"selection-{name}{settings.shard_suffix}-{worker_name()}.json"
    )
    FileSystem().write(
        path,
        json.dumps(
            {
                "phase": name,
                "shard": settings.shard_index,
                "total_shards": settings.shard_total,
                "all": [item.nodeid for item in ordered],
                "selected": [item.nodeid for item in chosen],
            },
            indent=2,
        )
        + "\n",
    )


def pytest_sessionfinish(session: pytest.Session, exitstatus: int) -> None:
    total, selected = session.config.stash.get(SELECTION, (0, 0))
    worker_output = getattr(session.config, "workeroutput", None)
    if isinstance(worker_output, dict):
        worker_output["revaer_selection"] = (total, selected)
    # A genuinely empty suite remains an error. Only a nonempty suite assigned
    # no scenarios on this shard has an expected successful empty selection.
    if exitstatus == pytest.ExitCode.NO_TESTS_COLLECTED and total > 0 and selected == 0:
        session.exitstatus = pytest.ExitCode.OK


@pytest.hookimpl(optionalhook=True)
def pytest_testnodedown(node: E2eWorker, error: object) -> None:
    """Transfer empty-shard proof to xdist's controller, which never collects."""
    if error is not None:
        return  # xdist already reports the worker failure; it is never made successful.
    selection = node.workeroutput.get("revaer_selection")
    if not isinstance(selection, (list, tuple)) or len(selection) != 2:
        raise pytest.UsageError("An E2E worker did not provide its collection counts")
    total, selected = selection
    if not isinstance(total, int) or not isinstance(selected, int) or not 0 <= selected <= total:
        raise pytest.UsageError("An E2E worker provided invalid collection counts")
    prior = node.config.stash.get(SELECTION, (total, selected))
    if prior != (total, selected):
        raise pytest.UsageError("E2E workers disagreed on the selected scenario count")
    node.config.stash[SELECTION] = (total, selected)


@pytest.hookimpl(tryfirst=True)
def pytest_runtest_setup(item: pytest.Item) -> None:
    # pytest-rerunfailures reuses the item. A previous attempt's call report must
    # not turn a later setup failure into a successful recording decision.
    item.stash[REPORTS] = {}


@pytest.hookimpl(wrapper=True, tryfirst=True)
def pytest_runtest_makereport(
    item: pytest.Item,
    call: pytest.CallInfo[None],
) -> Generator[None, pytest.TestReport, pytest.TestReport]:
    report = yield
    item.stash.setdefault(REPORTS, {})[report.when] = report
    return report


@pytest.fixture(scope="session")
def runtime(pytestconfig: pytest.Config) -> E2eRuntime:
    name = phase()
    settings = load_e2e_settings(os.environ)
    mode = "none" if name == "api-none" else "api_key"
    key = os.environ.get("REVAER_E2E_API_KEY") or None
    if mode == "api_key" and key is None:
        raise pytest.UsageError("The rv runner did not supply the authenticated API session")
    return E2eRuntime(
        pytestconfig.rootpath,
        settings,
        name,
        results_root(pytestconfig.rootpath),
        FileSystem(),
        ApiSession(mode, key),
    )


@pytest.fixture(scope="session")
def session(runtime: E2eRuntime) -> ApiSession:
    return runtime.session


@pytest.fixture(scope="session")
def fs_root(runtime: E2eRuntime) -> Path:
    return runtime.settings.filesystem_root or runtime.root


@pytest.fixture(scope="session")
def api_coverage(runtime: E2eRuntime) -> RouteCoverage:
    return RouteCoverage(
        runtime.results
        / (f"api-coverage-{runtime.phase}{runtime.settings.shard_suffix}-{worker_name()}.json"),
        runtime.filesystem,
    )


@pytest.fixture(scope="session")
def ui_coverage(runtime: E2eRuntime) -> RouteCoverage:
    return RouteCoverage(
        runtime.results
        / (f"ui-coverage-{runtime.phase}{runtime.settings.shard_suffix}-{worker_name()}.json"),
        runtime.filesystem,
    )


@pytest.fixture(scope="session")
def schema(runtime: E2eRuntime) -> ApiSchema:
    return ApiSchema(
        object_value(decode(runtime.filesystem.read(runtime.root / "docs/api/openapi.json")))
    )


@pytest.fixture(scope="session")
def http() -> Http:
    return Http(build_opener(VisibleRedirects()))


@pytest.fixture
def api(
    runtime: E2eRuntime, http: Http, schema: ApiSchema, api_coverage: RouteCoverage
) -> ApiClient:
    return ApiClient(
        http, runtime.settings.api_url, api_coverage, schema, runtime.session.headers()
    )


@pytest.fixture
def public_api(
    runtime: E2eRuntime, http: Http, schema: ApiSchema, api_coverage: RouteCoverage
) -> ApiClient:
    return ApiClient(http, runtime.settings.api_url, api_coverage, schema)


@pytest.fixture
def javascript_pages() -> dict[Page, PageCoverage]:
    """A scenario closing a page early can call javascript_pages[page].finish()."""
    return {}


@pytest.fixture
def new_page(
    context: BrowserContext,
    runtime: E2eRuntime,
    javascript_inventory: bool,
    javascript_pages: dict[Page, PageCoverage],
    request: pytest.FixtureRequest,
) -> Callable[[], Page]:
    """Initialize instrumentation before the scenario receives a native page.

    Playwright's synchronous event callbacks can yield to the scenario during a
    protocol call. Keep page events observational and do initialization here.
    """

    def create() -> Page:
        page = context.new_page()
        if javascript_inventory:
            artifacts = request.node.stash[ARTIFACTS]
            javascript_pages[page] = PageCoverage(
                page,
                runtime.settings.ui_url,
                artifacts / f"javascript-page-{len(javascript_pages) + 1}.json",
                runtime.filesystem,
            )
        return page

    return create


@pytest.fixture
def page(new_page: Callable[[], Page]) -> Page:
    return new_page()


@pytest.fixture
def close_page(javascript_pages: dict[Page, PageCoverage]) -> Callable[[Page], None]:
    """Close a page with its final counters, across all browser configurations."""

    def close(page: Page) -> None:
        coverage = javascript_pages.get(page)
        if coverage is not None:
            coverage.finish()
        page.close()

    return close


@pytest.fixture(scope="session")
def javascript_inventory(runtime: E2eRuntime) -> bool:
    if not runtime.settings.browser_coverage or runtime.phase != "ui-chromium":
        return False
    paths = tuple(
        string_value(value)
        for value in array_value(
            decode(runtime.filesystem.read(runtime.results / "javascript-sources.json"))
        )
    )
    syntax = JavaScriptSyntax(Parser(Language(tree_sitter_javascript.language())))
    scripts = parse_inventory(syntax, runtime.root, paths)
    destination = (
        runtime.results / f"javascript-baseline{runtime.settings.shard_suffix}-{worker_name()}.json"
    )
    write_inventory(runtime.filesystem, destination, scripts)
    return True


@pytest.fixture
def context(
    browser: Browser,
    runtime: E2eRuntime,
    request: pytest.FixtureRequest,
    javascript_inventory: bool,
    javascript_pages: dict[Page, PageCoverage],
) -> Iterator[BrowserContext]:
    settings = runtime.settings
    attempt = getattr(request.node, "execution_count", 1)
    if not isinstance(attempt, int) or attempt < 1:
        raise pytest.UsageError("Invalid retry attempt from pytest")
    slug = re.sub(r"[^A-Za-z0-9_-]+", "-", request.node.name)[:80]
    identity = hashlib.sha256(request.node.nodeid.encode()).hexdigest()[:12]
    artifacts = (
        runtime.results
        / (runtime.phase + settings.shard_suffix)
        / f"{slug}-{identity}-attempt-{attempt}"
    )
    runtime.filesystem.mkdir(artifacts)
    request.node.stash[ARTIFACTS] = artifacts
    video_enabled = settings.video.enabled(attempt)
    trace_enabled = settings.trace.enabled(attempt)
    context = browser.new_context(
        base_url=settings.ui_url,
        viewport={"width": settings.width, "height": settings.height},
        record_video_dir=artifacts / "recordings" if video_enabled else None,
        storage_state={
            "cookies": [],
            "origins": [
                {
                    "origin": settings.ui_url,
                    "localStorage": [
                        {"name": "revaer.auth.mode", "value": json.dumps("api_key")},
                        {"name": "revaer.api_key", "value": json.dumps(runtime.session.api_key)},
                        {
                            "name": "revaer.api_key_expires_at",
                            "value": str(time.time_ns() // 1000000 + 86400000),
                        },
                    ],
                }
            ],
        },
    )
    pages: list[Page] = []
    server_errors: set[str] = set()
    trace_started = False

    def record_page(page: Page) -> None:
        pages.append(page)

    def record_response(response: Response) -> None:
        target = urlsplit(response.url)
        origin = f"{target.scheme}://{target.netloc}"
        if response.status >= 500 and origin in (settings.api_url, settings.ui_url):
            # Omit query strings, headers and response bodies, which may carry
            # credentials. A rendered placeholder cannot conceal a server error.
            server_errors.add(f"{response.request.method} {target.path}: {response.status}")

    try:
        # Retain closed pages too: Playwright removes them from context.pages,
        # but their videos still need saving or deletion after context closure.
        context.on("page", record_page)
        context.on("response", record_response)
        context.set_default_timeout(settings.action_timeout_ms)
        context.set_default_navigation_timeout(settings.navigation_timeout_ms)
        expect.set_options(timeout=settings.expect_timeout_ms)
        if trace_enabled:
            context.tracing.start(screenshots=True, snapshots=True, sources=True)
            trace_started = True
        yield context
    finally:
        reports = request.node.stash.get(REPORTS, {})
        missing_instrumentation = javascript_inventory and any(
            page not in javascript_pages for page in pages
        )
        failed = (
            "call" not in reports
            or any(report.failed for report in reports.values())
            or bool(server_errors)
            or missing_instrumentation
        )
        videos = [page.video for page in pages if page.video is not None]
        request.node.user_properties.append(("artifacts", str(artifacts)))

        def finish_video(video: Video, index: int) -> None:
            if settings.video.retain(failed):
                video.save_as(artifacts / f"video-{index + 1}.webm")
            # Preserve the raw recording if saving fails, so a teardown error
            # still leaves evidence that can be recovered manually.
            video.delete()

        def finish_coverage(coverage: PageCoverage) -> None:
            nonlocal failed
            try:
                coverage.finish()
            except BaseException:
                failed = True
                raise

        def finish_trace() -> None:
            destination = artifacts / "trace.zip" if settings.trace.retain(failed) else None
            context.tracing.stop(path=destination)

        def finish_screenshot(page: Page, index: int) -> None:
            if not page.is_closed() and (
                settings.screenshot == "on" or (failed and settings.screenshot == "only-on-failure")
            ):
                page.screenshot(path=artifacts / f"page-{index + 1}.png")

        # LIFO cleanup guarantees coverage -> screenshots -> trace -> context ->
        # video. A coverage failure changes retention before recording decisions.
        # ExitStack attempts every registered operation even when one fails;
        # failures remain visible as pytest teardown errors with their causes.
        with ExitStack() as cleanup:
            for index, video in reversed(list(enumerate(videos))):
                cleanup.callback(finish_video, video, index)
            cleanup.callback(context.close)
            if trace_started:
                cleanup.callback(finish_trace)
            for index, page in reversed(list(enumerate(pages))):
                cleanup.callback(finish_screenshot, page, index)
            for coverage in reversed(tuple(javascript_pages.values())):
                cleanup.callback(finish_coverage, coverage)
        # Firefox may also record internal pages used to restore storage state.
        # Once the context and all tracked videos close successfully, this
        # attempt's raw scratch directory is no longer needed.
        runtime.filesystem.remove_owned(artifacts / "recordings", runtime.root)
        if missing_instrumentation:
            raise pytest.UsageError(
                "Browser line coverage requires pages created through the new_page fixture"
            )
        if server_errors:
            raise AssertionError(
                "Unexpected E2E server responses:\n" + "\n".join(sorted(server_errors))
            )


@pytest.fixture
def app(page: Page, ui_coverage: RouteCoverage) -> AppShell:
    return AppShell(page, ui_coverage)
