"""Scenarios run in a child pytest session against the real application fixtures."""

import json
from collections.abc import Callable, Iterator
from pathlib import Path

import pytest
from playwright.sync_api import Browser, BrowserContext, Page, Route, expect
from revaer_tooling.e2e.api import ApiSession

from tests.pages.app_shell import AppShell

HTML = """<!doctype html><title>Fixture</title>
<nav aria-label="Navbar">Revaer</nav>
<aside id="layout-sidebar"><a href="/settings">Settings</a></aside>
<main><h1>Browser fixture</h1></main>"""


def document(route: Route) -> None:
    route.fulfill(status=200, content_type="text/html", body=HTML)


@pytest.fixture(autouse=True)
def verify_context_cleanup(browser: Browser) -> Iterator[None]:
    yield
    # This fixture is established before context, so its teardown observes the
    # production context finalizer, including the deliberately failed attempt.
    assert browser.contexts == []


def test_retry_records_both_attempts(
    page: Page,
    context: BrowserContext,
    app: AppShell,
    request: pytest.FixtureRequest,
    session: ApiSession,
    close_page: Callable[[Page], None],
) -> None:
    context.route("**/*", document)
    app.goto("/")
    expect(page.get_by_role("heading")).to_have_text("Browser fixture")
    origins = context.storage_state()["origins"]
    assert len(origins) == 1
    storage = {entry["name"]: entry["value"] for entry in origins[0]["localStorage"]}
    assert json.loads(storage["revaer.api_key"]) == session.api_key
    assert json.loads(storage["revaer.auth.mode"]) == "api_key"
    assert int(storage["revaer.api_key_expires_at"]) > 0
    popup = context.new_page()
    popup.goto("/settings")
    if popup.video is not None:
        # The scenario tests retaining an already-recording closed page. Wait
        # for Playwright's assigned video artifact before closing the popup;
        # a page closed before recording starts has no video to retain.
        assert Path(popup.video.path()).suffix == ".webm"
    close_page(popup)
    app.navigate("Settings")
    assert getattr(request.node, "execution_count", 1) == 2


def test_routes_are_accumulated(context: BrowserContext, app: AppShell) -> None:
    context.route("**/*", document)
    app.goto("/torrents/example")
    app.goto("/unknown-route")
