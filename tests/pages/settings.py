"""Settings affordances and accessible tab/panel linkage."""

import re

from playwright.sync_api import Page, expect


class SettingsPage:
    def __init__(self, page: Page) -> None:
        self.page = page

    def expect_loaded(self) -> None:
        content = self.page.locator("#layout-content")
        tablist = content.get_by_role("tablist")
        expect(tablist).to_be_visible()
        expect(tablist).to_have_class(re.compile("tabs-lift"))
        tabs = tablist.get_by_role("tab")
        expect(tabs).to_have_count(7)
        expect(tabs.locator("svg")).to_have_count(7)
        expect(content.get_by_text("Connection / Auth", exact=True)).to_be_visible()
        panel = content.get_by_role("tabpanel")
        expect(panel).to_be_visible()
        expect(panel).to_have_class(re.compile("tab-content"))
        for label in ("Save", "Test connection"):
            expect(content.get_by_role("button", name=label)).to_be_visible()
        expect(self.page.locator("#layout-topbar .breadcrumbs")).to_have_count(0)
        expect(
            content.get_by_text("Configure authentication, engine behavior, and storage policies.")
        ).to_have_count(0)

    def select_tab(self, label: str) -> None:
        tab = self.page.get_by_role("tab", name=label)
        tab.click()
        expect(tab).to_have_attribute("aria-selected", "true")
        identifier = tab.get_attribute("id")
        assert identifier, "Settings tabs must identify their accessible panel"
        panel = self.page.get_by_role("tabpanel")
        expect(panel).to_be_visible()
        expect(panel).to_have_attribute("aria-labelledby", identifier)
