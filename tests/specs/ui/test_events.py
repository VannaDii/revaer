"""Event-stream resume state survives end-of-stream, conflict and reload."""

import json

from playwright.sync_api import Page, Route

from tests.pages.app_shell import AppShell
from tests.support.polling import eventually


def event_frame(number: int) -> str:
    identifier = 99 if number == 1 else 100
    payload = json.dumps(
        {
            "id": identifier,
            "timestamp": "2026-10-08T00:00:00Z",
            "event": {"type": "health_changed", "degraded": []},
        }
    )
    # The second event ends without a blank-line terminator.
    ending = "\n\n" if number == 1 else ""
    return f"retry: 250\nid: {identifier}\ndata: {payload}" + ending


def test_event_resume_resets_conflict_and_persists_final_frame(app: AppShell, page: Page) -> None:
    resume_headers: list[str | None] = []
    exercise_resume = False

    def respond(route: Route) -> None:
        request = route.request
        headers = {
            "Access-Control-Allow-Origin": request.headers.get("origin", "*"),
            "Access-Control-Allow-Methods": "GET, OPTIONS",
            "Access-Control-Allow-Headers": request.headers.get(
                "access-control-request-headers", "authorization, x-revaer-api-key, last-event-id"
            ),
            "Content-Type": "text/event-stream",
        }
        if request.method == "OPTIONS":
            route.fulfill(status=204, headers=headers)
            return
        if not exercise_resume:
            route.fulfill(status=403, headers=headers)
            return
        resume_headers.append(request.header_value("last-event-id"))
        number = len(resume_headers)
        if number == 2:
            route.fulfill(status=409, headers=headers)
        elif number >= 4:
            route.fulfill(status=403, headers=headers)
        else:
            route.fulfill(status=200, headers=headers, body=event_frame(number))

    page.route("**/v1/torrents/events*", respond)
    app.goto("/torrents")
    # Startup health/configuration can replace the initial connection. Begin
    # the resume sequence through the operator retry after those reads settle.
    page.wait_for_load_state("networkidle")
    exercise_resume = True
    indicator = page.locator(".sse-indicator").first
    indicator.click()
    page.get_by_role("button", name="Retry now", exact=True).click()
    eventually(
        lambda: (
            indicator.get_attribute("aria-label") == "Disconnected" and len(resume_headers) >= 4
        ),
        "event resume, conflict reset and final-frame delivery",
    )
    assert resume_headers == [None, "99", None, "100"]
    stored_id = next(
        (
            entry["value"]
            for origin in page.context.storage_state()["origins"]
            for entry in origin["localStorage"]
            if entry["name"] == "revaer.sse.last_event_id"
        ),
        None,
    )
    assert stored_id is not None
    assert json.loads(stored_id) == "100"
    page.reload(wait_until="domcontentloaded")
    eventually(
        lambda: (
            indicator.get_attribute("aria-label") == "Disconnected" and len(resume_headers) >= 5
        ),
        "persisted event resume after reload",
    )
    assert resume_headers[4] == "100"
