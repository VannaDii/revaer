"""An imported definition drives real upstream search and persisted browser results."""

import re
from typing import Literal
from urllib.parse import parse_qs, urlsplit

import pytest
from playwright.sync_api import Page, expect
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import array_value, object_value

from tests.conftest import E2eRuntime
from tests.pages.app_shell import AppShell
from tests.specs.ui.test_search import sealed_summary, submitted_search
from tests.support.api_assertions import rows
from tests.support.cardigann import configure_cardigann, html_upstream
from tests.support.polling import eventually
from tests.support.rss import discovery_state, enable_rss, items, make_due, status


@pytest.mark.parametrize("layout", ["table", "list"])
def test_imported_cardigann_html_search(
    app: AppShell, page: Page, api: ApiClient, layout: Literal["table", "list"]
) -> None:
    before = api.request(ApiRequest(Method.GET, "/v1/torrents"))
    assert before.ok
    with (
        html_upstream(layout) as upstream,
        configure_cardigann(api, upstream) as (identifier, profile),
    ):
        assert any(
            item["search_profile_public_id"] == profile and item["is_default"]
            for item in rows(api, "/v1/indexers/search-profiles", "search_profiles")
        )
        canonical = None
        for _attempt in range(2):
            with submitted_search(app, page, api, upstream.title) as request_id:
                sealed_summary(page, request_id)
                expect(page.get_by_role("heading", name=upstream.title, exact=True)).to_be_visible()
                article = page.get_by_role("article").filter(
                    has=page.get_by_role("heading", name=upstream.title, exact=True)
                )
                expect(article).to_contain_text("S:17")
                expect(article).to_contain_text("L:2")
                expect(article).to_contain_text("1.0 MiB")
                response = api.request(
                    ApiRequest(
                        Method.GET,
                        "/v1/indexers/search-requests/{search_request_public_id}/pages/{page_number}",
                        path={"search_request_public_id": request_id, "page_number": "1"},
                    )
                )
                assert response.ok
                results = array_value(response.object()["items"])
                assert len(results) == 1
                release = object_value(results[0])
                assert release["indexer_instance_public_id"] == identifier
                assert release["size_bytes"] == 1048576
                assert release["infohash_v1"] == upstream.identity
                if canonical is not None:
                    assert release["canonical_torrent_public_id"] == canonical
                canonical = release["canonical_torrent_public_id"]
        assert len(upstream.requests) == 2
        for target in upstream.requests:
            parsed = urlsplit(target)
            assert parsed.path == upstream.search_path
            assert parse_qs(parsed.query) == {
                upstream.query_key: [upstream.title],
                "order": ["newest"],
            }
            assert "%26" in target
            assert "%2B" in target
    after = api.request(ApiRequest(Method.GET, "/v1/torrents"))
    assert after.ok
    assert after.object() == before.object()


@pytest.mark.parametrize("layout", ["table", "list"])
def test_scheduled_cardigann_discovery_in_indexer_console(
    app: AppShell,
    page: Page,
    api: ApiClient,
    runtime: E2eRuntime,
    layout: Literal["table", "list"],
) -> None:
    before = rows(api, "/v1/torrents", "torrents")
    with html_upstream(layout) as upstream, configure_cardigann(api, upstream) as (identifier, _):
        initial = discovery_state(runtime, identifier)
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        eventually(
            lambda: status(api, identifier).get("last_polled_at") is not None,
            "scheduled Cardigann discovery",
        )
        snapshot = status(api, identifier)
        assert snapshot.get("last_error_class") is None
        assert snapshot.get("next_poll_at") is not None
        first = items(api, identifier)
        assert len(first) == 1
        assert first[0]["item_guid"] == upstream.origin + "/details/" + upstream.identity
        first_poll = snapshot["last_polled_at"]
        make_due(runtime, identifier)
        eventually(
            lambda: status(api, identifier).get("last_polled_at") != first_poll,
            "repeated Cardigann poll",
        )
        assert items(api, identifier) == first
        assert len(upstream.requests) == 2
        for target in upstream.requests:
            parsed = urlsplit(target)
            assert parsed.path == upstream.search_path
            assert parse_qs(parsed.query) == {"order": ["newest"]}
        app.goto("/indexers")
        page.get_by_role("textbox", name=re.compile(r"^Instance public ID\b")).fill(identifier)
        page.get_by_role("button", name="Fetch RSS status", exact=True).click()
        record = page.locator("article").filter(
            has=page.get_by_role("heading", name="RSS subscription fetch", exact=True)
        )
        expect(record).to_contain_text('"subscription_status": "enabled"')
        page.get_by_role("button", name="Fetch recent RSS items", exact=True).click()
        recent = page.locator("article").filter(
            has=page.get_by_role("heading", name="RSS items fetch", exact=True)
        )
        expect(recent).to_contain_text(upstream.identity)
        final = discovery_state(runtime, identifier)
        assert final["search_requests"] == initial["search_requests"]
        assert final["canonical_results"] == initial["canonical_results"]
        assert final["rss_requests"] == 2
        assert final["unexpected_requests"] == 0
    assert rows(api, "/v1/torrents", "torrents") == before
