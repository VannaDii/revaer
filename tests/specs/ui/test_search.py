"""Live manual search traverses the production UI, executor and database."""

import json
from collections.abc import Iterator
from contextlib import ExitStack, contextmanager
from urllib.parse import urlsplit

import pytest
from playwright.sync_api import Page, Response, expect
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import JsonObject, array_value, object_value, string_value

from tests.pages.app_shell import AppShell
from tests.support.polling import eventually
from tests.support.torznab import (
    TorznabUpstream,
    configure_torznab,
    require_status,
    torznab_upstream,
)


@contextmanager
def submitted_search(app: AppShell, page: Page, api: ApiClient, query: str) -> Iterator[str]:
    app.goto("/search")
    page.get_by_label("Query text", exact=True).fill(query)
    with page.expect_response(
        lambda response: (
            response.request.method == "POST"
            and urlsplit(response.url).path == "/v1/indexers/search-requests"
        )
    ) as submitted:
        page.get_by_role("button", name="Search", exact=True).click()
    assert submitted.value.status == 201
    request_id = string_value(object_value(submitted.value.json())["search_request_public_id"])
    try:
        yield request_id
    finally:
        require_status(
            api,
            ApiRequest(
                Method.POST,
                "/v1/indexers/search-requests/{search_request_public_id}/cancel",
                path={"search_request_public_id": request_id},
            ),
            204,
        )


def refresh_pages(page: Page, request_id: str) -> JsonObject:
    expect(page.get_by_text(request_id, exact=True)).to_be_visible()
    with page.expect_response(
        lambda response: (
            urlsplit(response.url).path == f"/v1/indexers/search-requests/{request_id}/pages"
        )
    ) as refreshed:
        page.get_by_role("button", name="Refresh pages", exact=True).click()
    assert refreshed.value.status == 200
    return object_value(refreshed.value.json())


def sealed_summary(page: Page, request_id: str) -> JsonObject:
    latest: JsonObject = {}

    def sealed() -> bool:
        nonlocal latest
        latest = refresh_pages(page, request_id)
        pages = array_value(latest["pages"])
        return bool(pages) and all(
            object_value(item).get("sealed_at") is not None for item in pages
        )

    eventually(sealed, "completed manual search page")
    return latest


@pytest.mark.parametrize("duplicate", [False, True], ids=["matching", "duplicate"])
def test_live_torznab_manual_search(
    app: AppShell, page: Page, api: ApiClient, duplicate: bool
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
        upstream.scenario = "duplicate" if duplicate else "matching"
        assert_live_results(app, page, api, upstream, identifier)


def test_live_torznab_private_download_stays_server_side(
    app: AppShell, page: Page, api: ApiClient
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
        upstream.private_download = True
        assert_live_results(app, page, api, upstream, identifier)


def assert_live_results(
    app: AppShell, page: Page, api: ApiClient, upstream: TorznabUpstream, identifier: str
) -> None:
    pages: list[JsonObject] = []

    def collect_page(response: Response) -> None:
        path = urlsplit(response.url).path
        if "/search-requests/" in path and "/pages/" in path:
            assert response.status == 200
            pages.append(object_value(response.json()))

    page.on("response", collect_page)
    with submitted_search(app, page, api, upstream.title) as request_id:
        results = page.get_by_role("heading", name=upstream.title, exact=True)
        sealed_summary(page, request_id)
        expect(results).to_be_visible()
        row = page.get_by_role("article").filter(has=results)
        expect(row).to_contain_text("Live search " + upstream.infohash[:12])
        expect(row).to_contain_text(upstream.details_url)
        items = [object_value(item) for result in pages for item in array_value(result["items"])]
        assert any(
            item.get("infohash_v1") == upstream.infohash
            and item.get("indexer_instance_public_id") == identifier
            for item in items
        )
        assert (upstream.title, True) in upstream.requests
        # Repeated reads and duplicate RSS items must retain canonical identity and position.
        request = ApiRequest(
            Method.GET,
            "/v1/indexers/search-requests/{search_request_public_id}/pages/{page_number}",
            path={"search_request_public_id": request_id, "page_number": "1"},
        )
        first = api.request(request)
        repeated = api.request(request)
        assert first.status == repeated.status == 200
        assert first.object() == repeated.object()
        assert len(array_value(first.object()["items"])) == 1
        if upstream.private_download:
            item = object_value(array_value(first.object()["items"])[0])
            omitted = item.get("download_url") is None
            assert omitted, "Private download URL must be omitted"
            assert item.get("details_url") == upstream.details_url
        private = (
            upstream.api_key not in json.dumps(first.object())
            and upstream.api_key not in page.content()
        )
        assert private, "Upstream credentials must remain private"


@pytest.mark.parametrize("scenario", ["empty", "unauthorized", "malformed", "oversized"])
def test_live_torznab_zero_result_outcomes(
    app: AppShell, page: Page, api: ApiClient, scenario: str
) -> None:
    with (
        torznab_upstream() as unrelated,
        configure_torznab(api, unrelated),
        torznab_upstream() as upstream,
        configure_torznab(api, upstream),
    ):
        unrelated.scenario = "unauthorized"
        if scenario == "empty":
            upstream.scenario = "empty"
        elif scenario == "unauthorized":
            upstream.scenario = "unauthorized"
        elif scenario == "oversized":
            upstream.scenario = "oversized"
        else:
            upstream.scenario = "malformed"
        with submitted_search(app, page, api, upstream.title) as request_id:
            summary = sealed_summary(page, request_id)
            assert all(
                object_value(item)["item_count"] == 0 for item in array_value(summary["pages"])
            )
            explainability = object_value(summary["explainability"])
            assert explainability["zero_runnable_indexers"] is False
            expected_failures = 0 if scenario == "empty" else 1
            assert explainability["skipped_failed_indexers"] == expected_failures
            expect(page.get_by_role("button", name="Page 1 (0)", exact=True)).to_be_visible()
            metric = page.get_by_text("Skipped (failed)", exact=True).locator("..")
            expect(metric).to_have_text("Skipped (failed)" + str(expected_failures))
            assert len(upstream.requests) == (2 if scenario in {"malformed", "oversized"} else 1)
            assert all(
                query == upstream.title and authenticated
                for query, authenticated in upstream.requests
            )
            assert not unrelated.requests
            private = (
                upstream.api_key not in json.dumps(summary)
                and upstream.api_key not in page.content()
            )
            assert private, "Failed searches must not expose credentials"


def test_live_torznab_rate_denial_does_not_contact_upstream(
    app: AppShell, page: Page, api: ApiClient
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as source:
        upstream.scenario = "empty"
        created = api.request(
            ApiRequest(
                Method.POST,
                "/v1/indexers/rate-limits",
                {
                    "display_name": "Live search " + upstream.infohash,
                    "rpm": 1,
                    "burst": 0,
                    "concurrent": 1,
                },
            )
        )
        assert created.status == 201
        policy = string_value(created.object()["rate_limit_policy_public_id"])
        assignment = "/v1/indexers/instances/{indexer_instance_public_id}/rate-limit"
        source_path = {"indexer_instance_public_id": source}
        with ExitStack() as cleanup:
            cleanup.callback(
                require_status,
                api,
                ApiRequest(
                    Method.DELETE,
                    "/v1/indexers/rate-limits/{rate_limit_policy_public_id}",
                    path={"rate_limit_policy_public_id": policy},
                ),
                204,
            )
            cleanup.callback(
                require_status,
                api,
                ApiRequest(
                    Method.PUT, assignment, {"rate_limit_policy_public_id": None}, path=source_path
                ),
                204,
            )
            require_status(
                api,
                ApiRequest(
                    Method.PUT,
                    assignment,
                    {"rate_limit_policy_public_id": policy},
                    path=source_path,
                ),
                204,
            )
            app.goto("/search")
            denied: list[str] = []
            # Three attempts within the existing 30-second test deadline cannot all
            # fit a one-per-minute budget, even when the minute changes between them.
            for index in range(3):
                query = upstream.title + f" rate {index}"
                page.get_by_label("Query text", exact=True).fill(query)
                with page.expect_response(
                    lambda response: (
                        response.request.method == "POST"
                        and urlsplit(response.url).path == "/v1/indexers/search-requests"
                    )
                ) as submitted:
                    page.get_by_role("button", name="Search", exact=True).click()
                assert submitted.value.status == 201
                request_id = string_value(
                    object_value(submitted.value.json())["search_request_public_id"]
                )
                cancel = ApiRequest(
                    Method.POST,
                    "/v1/indexers/search-requests/{search_request_public_id}/cancel",
                    path={"search_request_public_id": request_id},
                )
                with ExitStack() as request_cleanup:
                    request_cleanup.callback(require_status, api, cancel, 204)
                    summary: JsonObject = {}

                    def classified(selected_request_id: str = request_id) -> bool:
                        nonlocal summary
                        summary = refresh_pages(page, selected_request_id)
                        explain = object_value(summary["explainability"])
                        return explain["rate_limited_indexers"] == 1 or any(
                            object_value(item).get("sealed_at") is not None
                            for item in array_value(summary["pages"])
                        )

                    eventually(classified, "allowed or deferred manual attempt")
                    if object_value(summary["explainability"])["rate_limited_indexers"] == 1:
                        denied.append(query)
                        metric = page.get_by_text("Rate-limited indexers", exact=True).locator("..")
                        expect(metric).to_contain_text("1")
            assert denied, "The configured budget must defer an attempt"
            observed_queries = {query for query, _ in upstream.requests}
            assert all(query not in observed_queries for query in denied)
            assert all(authenticated for _, authenticated in upstream.requests)


@pytest.mark.parametrize("authenticated", [False, True], ids=["open", "authenticated"])
def test_live_torznab_uses_configured_http_proxy(
    app: AppShell, page: Page, api: ApiClient, authenticated: bool
) -> None:
    with torznab_upstream() as upstream, ExitStack() as cleanup:
        proxy = urlsplit(upstream.endpoint)
        route = api.request(
            ApiRequest(
                Method.POST,
                "/v1/indexers/routing-policies",
                {
                    "display_name": "Search proxy " + upstream.infohash,
                    "mode": "http_proxy",
                },
            )
        )
        assert route.status == 201
        route_id = string_value(route.object()["routing_policy_public_id"])
        for key, values in (
            ("proxy_host", {"value_plain": proxy.hostname}),
            ("proxy_port", {"value_int": proxy.port}),
        ):
            body: JsonObject = {"param_key": key, **values}
            require_status(
                api,
                ApiRequest(
                    Method.POST,
                    "/v1/indexers/routing-policies/{routing_policy_public_id}/params",
                    body,
                    path={"routing_policy_public_id": route_id},
                ),
                204,
            )
        if authenticated:
            upstream.require_proxy_auth = True
            require_status(
                api,
                ApiRequest(
                    Method.POST,
                    "/v1/indexers/routing-policies/{routing_policy_public_id}/params",
                    {"param_key": "proxy_username", "value_plain": "search-proxy"},
                    path={"routing_policy_public_id": route_id},
                ),
                204,
            )
            secret = api.request(
                ApiRequest(
                    Method.POST,
                    "/v1/indexers/secrets",
                    {
                        "secret_type": "password",
                        "secret_value": upstream.proxy_password,
                    },
                )
            )
            assert secret.status == 201
            secret_id = string_value(secret.object()["secret_public_id"])
            cleanup.callback(
                require_status,
                api,
                ApiRequest(Method.DELETE, "/v1/indexers/secrets", {"secret_public_id": secret_id}),
                204,
            )
            require_status(
                api,
                ApiRequest(
                    Method.POST,
                    "/v1/indexers/routing-policies/{routing_policy_public_id}/secrets",
                    {"param_key": "http_proxy_auth", "secret_public_id": secret_id},
                    path={"routing_policy_public_id": route_id},
                ),
                204,
            )
        # Both endpoints belong to the loopback fixture. Absolute-form requests
        # distinguish configured proxy routing from a direct connection.
        with configure_torznab(api, upstream, route_id) as identifier:
            assert_live_results(app, page, api, upstream, identifier)
            assert upstream.proxied_requests
            assert all(upstream.proxied_requests)
            assert all(upstream.proxy_authenticated)
            private = upstream.proxy_password not in page.content()
            assert private, "Proxy credentials must stay private"
