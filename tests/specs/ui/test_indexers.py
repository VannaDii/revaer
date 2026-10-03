"""The indexer administration console exposes every existing management panel."""

from playwright.sync_api import Page, expect

from tests.pages.app_shell import AppShell


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
