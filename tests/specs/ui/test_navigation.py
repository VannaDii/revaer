"""Overview routes, sidebar navigation and icon-only topbar controls."""

import re

from playwright.sync_api import Page, expect

from tests.pages.app_shell import AppShell
from tests.pages.overview import expect_dashboard, expect_health
from tests.pages.settings import SettingsPage
from tests.pages.torrents import TorrentsPage


def test_dashboard(app: AppShell, page: Page) -> None:
    app.goto("/")
    expect_dashboard(page)


def test_health(app: AppShell, page: Page) -> None:
    app.goto("/health")
    expect_health(page)


def test_sidebar_routes(app: AppShell, page: Page) -> None:
    app.goto("/")
    expect_dashboard(page)
    app.navigate("Torrents")
    expect(page).to_have_url(re.compile(r"/torrents$"))
    TorrentsPage(page).expect_loaded()
    app.navigate("Settings")
    expect(page).to_have_url(re.compile(r"/settings$"))
    SettingsPage(page).expect_loaded()


def test_icon_controls(app: AppShell, page: Page) -> None:
    app.goto("/")
    expect(page.get_by_test_id("server-menu-icon").locator("svg rect")).to_have_count(3)
    locale = page.locator('[aria-label="Locale"]')
    expect(locale.locator("img")).to_have_attribute("src", re.compile(r"/gb\.svg$"))
    expect(locale.locator("img")).to_have_class(re.compile("rounded-full"))
    locale.click()
    menu = page.locator(".locale-menu__content")
    expect(menu).to_be_visible()
    expect(menu.locator("img").first).to_have_class(re.compile("rounded-full"))
    expect(menu.locator('img[src*="/gb.svg"]')).to_have_count(1)
    indicator = page.locator("#layout-sidebar .sse-indicator")
    expect(indicator).to_have_attribute("aria-label", re.compile(".+"))
    expect(indicator).to_have_class(re.compile("btn-circle"))
    expect(indicator).to_have_class(re.compile("btn-sm"))
    expect(page.locator("#layout-sidebar .sse-indicator__label")).to_have_count(0)
    logout = page.get_by_role("button", name="Logout")
    expect(logout).to_be_visible()
    expect(logout).to_have_attribute("data-tip", "Logout")
    expect(logout).not_to_have_text(re.compile("Logout"))
    sidebar_box = page.locator("#layout-sidebar").bounding_box()
    logout_box, indicator_box = logout.bounding_box(), indicator.bounding_box()
    assert sidebar_box is not None
    assert logout_box is not None
    assert indicator_box is not None
    assert logout_box["x"] + logout_box["width"] <= sidebar_box["x"] + sidebar_box["width"] + 1
    assert abs(logout_box["width"] - indicator_box["width"]) <= 1
    assert abs(logout_box["height"] - indicator_box["height"]) <= 1


def test_settings_tabs(app: AppShell, page: Page) -> None:
    app.goto("/settings")
    settings = SettingsPage(page)
    settings.expect_loaded()
    for label in ("Downloads", "Seeding", "Network", "Storage", "Labels", "System"):
        settings.select_tab(label)
