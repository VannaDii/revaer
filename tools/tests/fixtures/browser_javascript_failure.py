"""Child scenario proving coverage teardown errors retain browser evidence."""

from pathlib import Path

from playwright.sync_api import Page


def test_closed_without_capture(page: Page) -> None:
    source = Path("fixture.js").read_text()
    page.route(
        "**/*",
        lambda route: route.fulfill(content_type="text/html", body=f"<script>{source}</script>"),
    )
    page.goto("/")
    if page.video is not None:
        # Await the assigned recording before closing, as the video fixture does.
        assert Path(page.video.path()).suffix == ".webm"
    page.close()
