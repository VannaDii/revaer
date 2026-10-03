"""Route-controlled presentation checks; these do not prove root attestation.

Transport interception preserves the original desktop/mobile regression cases.
Only the DOM Range measurement executes in the browser: Playwright exposes box
geometry in Python, but not text-range geometry.
"""

from pathlib import Path

import pytest
from playwright.sync_api import Page, Route, expect
from revaer_tooling.json_data import Json

from tests.conftest import ARTIFACTS
from tests.pages.app_shell import AppShell

READINESS = "**/v1/media/root-catalog/readiness"
KINDS = ("source", "output", "workspace", "backup", "quarantine")


def ready_catalog(count: int) -> dict[str, Json]:
    """Supply identical counts per kind while keeping destructive readiness off."""
    return {
        "format_version": 1,
        "source_state": "ready",
        "attestation_state": "ready",
        "generation": "1",
        "kinds": [
            {
                "kind": kind,
                "attested_slot_count": count,
                "binding_ready_slot_count": count,
                "destructive_ready_slot_count": 0,
            }
            for kind in KINDS
        ],
    }


def test_unavailable_endpoint_refreshes_to_empty_catalog(app: AppShell, page: Page) -> None:
    status = 404

    def respond(route: Route) -> None:
        route.fulfill(
            status=status,
            json=ready_catalog(0)
            if status == 200
            else {
                "title": "Not found",
                "detail": "/private/root/must-not-appear",
                "status": status,
            },
        )

    page.route(READINESS, respond)
    app.goto("/media")
    panel = page.get_by_test_id("media-root-readiness")
    expect(panel.get_by_role("alert")).to_have_text("Root readiness is unavailable on this server.")
    expect(panel).not_to_contain_text("/private/root")
    status = 200
    panel.get_by_role("button", name="Refresh root readiness").click()
    expect(panel.get_by_role("status")).to_have_text("The catalog contains no attested root slots.")
    expect(panel.get_by_role("row")).to_have_count(6)


def test_binding_and_destructive_readiness_desktop_and_mobile(
    app: AppShell,
    page: Page,
    request: pytest.FixtureRequest,
) -> None:
    page.route(READINESS, lambda route: route.fulfill(json=ready_catalog(2)))
    app.goto("/media")
    panel = page.get_by_test_id("media-root-readiness")
    for width in (1280, 390):
        page.set_viewport_size({"width": width, "height": 844})
        expect(panel.get_by_role("columnheader", name="Binding ready", exact=True)).to_be_visible()
        header = panel.get_by_role("columnheader", name="Destructive ready", exact=True)
        expect(header).to_be_visible()
        bounds = header.bounding_box()
        assert bounds is not None and bounds["x"] + bounds["width"] <= width
        # DOM Range has no Python locator equivalent; keep this measurement
        # local to the element, without embedding automation or application logic.
        assert (
            header.evaluate("""header => {
            const range = header.ownerDocument.createRange();
            range.selectNodeContents(header);
            return range.getBoundingClientRect().right <= header.getBoundingClientRect().right;
        }""")
            is True
        )
        icon = panel.get_by_role("button", name="Refresh root readiness").locator("svg")
        expect(icon).to_be_visible()
        icon_bounds = icon.bounding_box()
        assert icon_bounds is not None and 0 < icon_bounds["width"] <= 32
        for kind in KINDS:
            row = panel.get_by_role("row").filter(
                has=page.get_by_role("rowheader", name=kind.capitalize(), exact=True),
            )
            expect(row.get_by_role("cell")).to_have_text(["2", "2", "0"])
        panel_bounds = panel.bounding_box()
        assert panel_bounds is not None and panel_bounds["x"] + panel_bounds["width"] <= width
        destination: Path = request.node.stash[ARTIFACTS] / f"root-readiness-{width}.png"
        panel.screenshot(path=destination, animations="disabled")


def test_access_denied_removes_previously_ready_counts(app: AppShell, page: Page) -> None:
    status = 200

    def respond(route: Route) -> None:
        route.fulfill(
            status=status,
            json=ready_catalog(2) if status == 200 else {"title": "Forbidden", "status": status},
        )

    page.route(READINESS, respond)
    app.goto("/media")
    panel = page.get_by_test_id("media-root-readiness")
    expect(panel.get_by_role("table")).to_be_visible()
    status = 403
    panel.get_by_role("button", name="Refresh root readiness").click()
    expect(panel.get_by_role("alert")).to_have_text("Access to root readiness was denied.")
    expect(panel.get_by_role("table")).to_have_count(0)
