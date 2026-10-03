"""Log presentation, filtering and native HTTP interception for SSE fixtures."""

import re

from playwright.sync_api import Page, Route, expect

from .app_shell import AppShell


def mock_log_stream(page: Page, body: str) -> None:
    def respond(route: Route) -> None:
        request = route.request
        headers = {
            "Access-Control-Allow-Origin": request.headers.get("origin", "*"),
            "Access-Control-Allow-Methods": "GET, OPTIONS",
            "Access-Control-Allow-Headers": request.headers.get(
                "access-control-request-headers", "authorization, x-revaer-api-key, content-type"
            ),
            "Access-Control-Max-Age": "600",
        }
        if request.method == "OPTIONS":
            route.fulfill(status=204, headers=headers)
        else:
            route.fulfill(
                status=200,
                headers={
                    **headers,
                    "Content-Type": "text/event-stream",
                    "Cache-Control": "no-cache",
                },
                body=body,
            )

    page.route("**/v1/logs/stream", respond)


class LogsPage:
    def __init__(self, app: AppShell) -> None:
        self.app = app
        self.page = app.page

    def expect_loaded(self) -> None:
        content = self.page.locator("#layout-content")
        expect(
            content.locator("span.badge").filter(has_text=re.compile("^(Connecting|Error|Live)$"))
        ).to_be_visible()
        expect(content.locator(".log-terminal")).to_be_visible()
        expect(self.page.locator("#layout-topbar .breadcrumbs")).to_have_count(0)
        expect(
            content.get_by_text("Live server output streamed on demand.", exact=True)
        ).to_have_count(0)

    def select_filter(self, label: str) -> None:
        self.app.dismiss_overlays()
        # These styled radio inputs are visually represented by their labels.
        # Preserve the existing force-check of the underlying native input.
        self.page.get_by_test_id("logs-level-filter").get_by_role("radio", name=label).check(
            force=True
        )

    def search(self, text: str) -> None:
        self.app.dismiss_overlays()
        self.page.get_by_role("searchbox").fill(text)
