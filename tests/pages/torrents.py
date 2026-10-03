"""Torrent list and modal controls use the existing accessible selectors."""

from playwright.sync_api import Page, expect


class TorrentsPage:
    def __init__(self, page: Page) -> None:
        self.page = page

    def expect_loaded(self) -> None:
        expect(self.page.get_by_label("Search torrents")).to_be_visible()
        for label in ("Add", "Create torrent"):
            expect(self.page.get_by_role("button", name=label)).to_be_visible()
        expect(self.page.get_by_role("columnheader", name="Name")).to_be_visible()
        expect(self.page.locator("#layout-topbar .breadcrumbs")).to_have_count(0)

    def open_add(self) -> None:
        self.page.get_by_role("button", name="Add").click()
        expect(self.page.get_by_role("heading", name="Add torrent")).to_be_visible()

    def open_create(self) -> None:
        self.page.get_by_role("button", name="Create torrent").click()
        expect(self.page.get_by_role("heading", name="Create torrent")).to_be_visible()

    def close_modal(self) -> None:
        close = self.page.get_by_role("button", name="Close modal")
        close.click()
        expect(close).to_be_hidden()
