"""Exercise asynchronous shell and authentication-control rendering."""

import pytest
from playwright.sync_api import Page, Route, TimeoutError, expect

from tests.pages.app_shell import AppShell

HTML_CONTENT_TYPE = "text/html"


@pytest.mark.parametrize("missing", ("shell", "dismiss"))
def test_missing_startup_controls_still_fail(page: Page, app: AppShell, missing: str) -> None:
    def serve(route: Route) -> None:
        body = "<main>Uninitialized</main>"
        if missing == "dismiss":
            body = (
                '<nav aria-label="Navbar">Ready</nav>'
                '<div class="auth-overlay" style="min-height:100px">Authentication</div>'
            )
        route.fulfill(content_type=HTML_CONTENT_TYPE, body=body)

    page.set_default_timeout(100)
    page.route("**/*", serve)
    with pytest.raises(TimeoutError):
        app.goto("/")


def test_shell_startup_precedes_overlay_handling(page: Page, app: AppShell) -> None:
    def serve(route: Route) -> None:
        route.fulfill(
            content_type=HTML_CONTENT_TYPE,
            body="""<script>
            setTimeout(() => {
                document.body.innerHTML = '<nav aria-label="Navbar">Ready</nav>';
            }, 300);
            </script>""",
        )

    # Model startup exceeding the short assertion timeout but remaining within
    # the existing action bound; do not change either application default.
    expect.set_options(timeout=100)
    page.set_default_timeout(2000)
    page.route("**/*", serve)
    app.goto("/")


def test_overlay_controls_can_finish_rendering(page: Page, app: AppShell) -> None:
    def serve(route: Route) -> None:
        route.fulfill(
            content_type=HTML_CONTENT_TYPE,
            body="""<nav aria-label="Navbar">Ready</nav>
            <div class="auth-overlay" role="dialog" style="min-height:100px">Authentication</div>
            <script>
            setTimeout(() => {
                const overlay = document.querySelector('.auth-overlay');
                const button = document.createElement('button');
                button.textContent = 'Dismiss';
                button.addEventListener('click', () => overlay.remove());
                overlay.append(button);
            }, 1000);
            </script>""",
        )

    expect.set_options(timeout=100)
    page.set_default_timeout(2000)
    page.route("**/*", serve)
    app.goto("/")
    expect(page.locator(".auth-overlay")).to_be_hidden()
