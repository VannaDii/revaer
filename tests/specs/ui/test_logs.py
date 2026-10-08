"""Log layout and filtering retain the original five-level SSE fixtures."""

from playwright.sync_api import Page, expect

from tests.pages.app_shell import AppShell
from tests.pages.logs import LogsPage, mock_log_stream


def test_log_shell(app: AppShell, page: Page) -> None:
    mock_log_stream(page, "")
    app.goto("/logs")
    LogsPage(app).expect_loaded()
    box = page.locator(".log-terminal").bounding_box()
    assert box is not None
    assert box["height"] > 160
    expect(page.get_by_text("No log lines yet.", exact=True)).to_be_visible()


def test_log_filters_and_search(app: AppShell, page: Page) -> None:
    lines = (
        "TRACE trace details",
        "DEBUG debug details",
        "level=INFO info details",
        "level=WARN warn details",
        "level=ERROR error details",
    )
    mock_log_stream(page, "".join("data: " + line + "\n\n" for line in lines))
    app.goto("/logs")
    logs = LogsPage(app)
    logs.expect_loaded()
    expect(page.get_by_test_id("logs-level-filter").locator('input[type="radio"]')).to_have_count(6)
    expect(page.get_by_test_id("logs-search-hint")).to_have_count(0)
    terminal = page.locator(".log-terminal")
    for line in lines:
        expect(terminal).to_contain_text(line)
    logs.select_filter("Warn")
    for line in lines[3:]:
        expect(terminal).to_contain_text(line)
    for line in lines[1:3]:
        expect(terminal).not_to_contain_text(line)
    logs.select_filter("All levels")
    logs.select_filter("Info")
    for line in lines[2:]:
        expect(terminal).to_contain_text(line)
    for line in lines[:2]:
        expect(terminal).not_to_contain_text(line)
    logs.search("error")
    expect(terminal).to_contain_text(lines[4])
    expect(terminal).not_to_contain_text(lines[3])
