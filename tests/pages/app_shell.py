"""Shared navigation and overlay handling, using native Playwright locators."""

from urllib.parse import urljoin, urlsplit

from playwright.sync_api import Page, expect
from revaer_tooling.e2e.coverage import RouteCoverage


class AppShell:
    def __init__(self, page: Page, coverage: RouteCoverage) -> None:
        self.page = page
        self.coverage = coverage

    def record_route(self, path: str) -> None:
        route = urlsplit(path).path.rstrip("/") or "/"
        if route.startswith("/torrents/"):
            route = "/torrents/:id"
        elif route not in (
            "/",
            "/torrents",
            "/settings",
            "/logs",
            "/health",
            "/indexers",
            "/search",
            "/media",
        ):
            route = "/not-found"
        self.coverage.record(route)

    def goto(self, path: str = "/") -> None:
        self.page.goto(path, wait_until="domcontentloaded")
        self.record_route(path)
        # DOMContentLoaded precedes the asynchronous Wasm shell render. Use
        # Playwright's existing action bound before inspecting optional overlays.
        navbar = self.page.get_by_role("navigation", name="Navbar")
        navbar.wait_for(state="visible")
        self.dismiss_overlays()
        expect(navbar).to_be_visible()

    def navigate(self, label: str) -> None:
        self.dismiss_overlays()
        link = self.page.locator("#layout-sidebar").get_by_role("link", name=label)
        href = link.get_attribute("href")
        if not href:
            raise AssertionError(f"Sidebar link {label} has no destination")
        destination = urljoin(self.page.url, href)
        link.click()
        expect(self.page).to_have_url(destination)
        # Record the resulting route, not an assumed label-to-route mapping.
        self.record_route(self.page.url)

    def dismiss_overlays(self) -> None:
        setup = self.page.locator(".setup-overlay")
        if setup.is_visible():
            raise AssertionError("Setup is required; E2E must activate the application first")
        overlay = self.page.locator(".auth-overlay")
        if not overlay.is_visible():
            return
        # A visible container does not establish that its controls are ready.
        # click waits for the final matching action to become actionable.
        overlay.get_by_role("button", name="Dismiss").last.click()
        expect(overlay).to_be_hidden()
