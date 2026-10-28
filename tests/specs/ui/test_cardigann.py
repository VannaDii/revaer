"""An imported definition drives real upstream search and persisted browser results."""

from typing import Literal
from urllib.parse import parse_qs, urlsplit

import pytest
from playwright.sync_api import Page, expect
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import array_value, object_value

from tests.pages.app_shell import AppShell
from tests.specs.ui.test_search import sealed_summary, submitted_search
from tests.support.api_assertions import rows
from tests.support.cardigann import configure_cardigann, html_upstream


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
