"""Intentional setup and timeout failures executed only by the tooling tests."""

import threading
from collections.abc import Iterator
from typing import NoReturn

import pytest
from playwright.sync_api import Browser, Page, expect


@pytest.fixture(autouse=True)
def verify_context_cleanup(browser: Browser) -> Iterator[None]:
    yield
    assert browser.contexts == []


@pytest.fixture
def failed_setup(page: Page) -> NoReturn:
    page.set_content("<h1>Setup reached the page</h1>")
    expect(page.get_by_role("heading")).to_be_visible()
    raise RuntimeError("injected setup failure")


@pytest.mark.usefixtures("failed_setup")
def test_setup_failure() -> None:
    pytest.fail("The injected setup failure did not prevent the test body")


@pytest.mark.timeout(1, func_only=True)
def test_timeout_failure(page: Page) -> None:
    page.set_content("<h1>Waiting for the test timeout</h1>")
    expect(page.get_by_role("heading")).to_be_visible()
    threading.Event().wait(20)
    pytest.fail("The configured test timeout did not interrupt the test")
