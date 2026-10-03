"""Assertions shared by dashboard, health and navigation scenarios."""

import re

from playwright.sync_api import Page, expect


def expect_dashboard(page: Page) -> None:
    for title in ("Storage Status", "Tracker Health"):
        expect(page.get_by_text(title, exact=True)).to_be_visible()
    expect(page.locator("#layout-topbar .breadcrumbs")).to_have_count(0)
    expect(page.locator("#layout-content > *").first).not_to_have_class(re.compile(r"\bmt-6\b"))


def expect_health(page: Page) -> None:
    for title in ("Metrics", "Basic", "Full"):
        expect(page.get_by_text(title, exact=True)).to_be_visible()
    expect(page.locator("#layout-topbar .breadcrumbs")).to_have_count(0)
