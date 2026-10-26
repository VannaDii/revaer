"""The indexer administration console exposes every existing management panel."""

import re

from playwright.sync_api import Page, expect
from revaer_tooling.e2e.api import ApiClient

from tests.conftest import E2eRuntime
from tests.pages.app_shell import AppShell
from tests.support.polling import eventually
from tests.support.rss import enable_rss, make_due, status
from tests.support.torznab import configure_torznab, torznab_upstream


def test_indexer_console(app: AppShell, page: Page) -> None:
    app.goto("/indexers")
    headings = (
        "Indexers",
        "Catalog",
        "Cardigann import",
        "Health notifications",
        "RSS management",
        "Category overrides",
        "Connectivity & reputation",
        "App sync",
        "Connectivity profile",
        "Source reputation",
        "Health events",
        "Import status",
        "Import results",
        "Search-profile inventory",
        "Policy-set inventory",
        "Torznab inventory",
        "Source conflict resolution",
        "Backup & restore",
        "Activity log",
    )
    for title in headings:
        expect(page.get_by_role("heading", name=title, exact=True)).to_be_visible()
    buttons = (
        "Refresh definitions",
        "Import Cardigann YAML",
        "Fetch RSS status",
        "Upsert tracker mapping",
        "Mark RSS item seen",
        "Fetch connectivity profile",
        "Fetch routing policy",
        "Fetch routing inventory",
        "Fetch rate limits",
        "Fetch instances",
        "Fetch source reputation",
        "Fetch health events",
        "Fetch notification hooks",
        "Fetch tags",
        "Fetch secrets",
        "Fetch search profiles",
        "Fetch policy sets",
        "Fetch Torznab instances",
        "Provision app sync",
    )
    for label in buttons:
        expect(page.get_by_role("button", name=label, exact=True)).to_be_visible()
    for label in ("RSS enabled", "Automatic search enabled"):
        expect(page.get_by_text(label, exact=True)).to_be_visible()


def test_scheduled_rss_items_visible_in_existing_console(
    app: AppShell,
    page: Page,
    api: ApiClient,
    runtime: E2eRuntime,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        eventually(
            lambda: status(api, identifier).get("last_polled_at") is not None,
            "scheduled RSS discovery",
        )
        snapshot = status(api, identifier)
        app.goto("/indexers")
        page.get_by_role("textbox", name=re.compile(r"^Instance public ID\b")).fill(identifier)
        page.get_by_role("button", name="Fetch RSS status", exact=True).click()
        record = page.locator("article").filter(
            has=page.get_by_role("heading", name="RSS subscription fetch", exact=True)
        )
        expect(record).to_contain_text(str(snapshot.get("last_polled_at")))
        expect(record).to_contain_text('"subscription_status": "enabled"')
        page.get_by_role("button", name="Fetch recent RSS items", exact=True).click()
        recent = page.locator("article").filter(
            has=page.get_by_role("heading", name="RSS items fetch", exact=True)
        )
        expect(recent).to_contain_text(upstream.infohash)
