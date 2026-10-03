"""Torrent controls, modal behavior and native pointer hit-testing for menus."""

import re
import uuid

from playwright.sync_api import Page, expect

from tests.pages.app_shell import AppShell
from tests.pages.torrents import TorrentsPage
from tests.support.polling import eventually


def test_list_controls(app: AppShell, page: Page) -> None:
    app.goto("/torrents")
    TorrentsPage(page).expect_loaded()


def test_add_and_create_modals(app: AppShell, page: Page) -> None:
    app.goto("/torrents")
    torrents = TorrentsPage(page)
    torrents.expect_loaded()
    torrents.open_add()
    torrents.close_modal()
    torrents.open_create()
    torrents.close_modal()


def test_topbar_menus_receive_pointer_above_bulk_bar(app: AppShell, page: Page) -> None:
    app.goto("/torrents")
    TorrentsPage(page).expect_loaded()
    page.get_by_test_id("torrents-bulk-action-bar").scroll_into_view_if_needed()
    for label, selector in (
        ("Locale", ".locale-menu__content"),
        ("Server menu", ".server-menu__content"),
    ):
        page.get_by_label(label, exact=True).click()
        menu = page.locator(selector)
        expect(menu).to_be_visible()
        box = menu.bounding_box()
        assert box is not None
        # Playwright's trial click performs visibility, stability and pointer
        # interception checks without dispatching the click or evaluating JS.
        menu.click(trial=True, position={"x": box["width"] / 2, "y": min(8, box["height"] / 2)})


def test_detail_route(app: AppShell, page: Page) -> None:
    identifier = str(uuid.uuid4())
    app.goto("/torrents/" + identifier)
    expect(page).to_have_url(re.compile("/torrents/" + identifier + "$"))
    # Toast notifications also have status roles. Only the loading indicator
    # is evidence that the detail drawer has begun opening.
    spinner = page.locator('.loading[role="status"]')
    overview = page.get_by_role("tab", name="Overview")
    eventually(
        lambda: spinner.is_visible() or overview.is_visible(), "torrent detail loading or overview"
    )


def test_not_found(app: AppShell, page: Page) -> None:
    app.goto("/definitely-not-a-route")
    expect(page.get_by_text("Not found", exact=True)).to_be_visible()
