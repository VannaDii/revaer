"""Child scenarios exercising the shipped fixture's JavaScript coverage wiring."""

from collections.abc import Callable
from pathlib import Path

from playwright.sync_api import BrowserContext, Page, Route


def test_collects_before_context_and_intentional_page_closure(
    context: BrowserContext,
    page: Page,
    new_page: Callable[[], Page],
    close_page: Callable[[Page], None],
) -> None:
    source = Path("fixture.js").read_text()

    def serve(route: Route) -> None:
        if route.request.url.endswith(".js"):
            route.fulfill(content_type="application/javascript; charset=utf-8", body=source)
        else:
            route.fulfill(content_type="text/html", body='<script src="/fixture.js"></script>')

    context.route("**/*", serve)
    page.goto("/")
    assert page.title() == "Collected"
    popup = new_page()
    popup.goto("/")
    close_page(popup)
