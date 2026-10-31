"""Scheduled HTML discovery reuses real imports, claims and fenced RSS completion."""

import threading
from typing import Literal

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method

from tests.conftest import E2eRuntime
from tests.support.cardigann import configure_cardigann, html_upstream
from tests.support.indexers import create_profile
from tests.support.polling import eventually
from tests.support.rss import RSS_ROUTE, enable_rss, items, make_due, status


@pytest.mark.parametrize("links", ["details", "magnet", "unsafe", "download"])
def test_cardigann_rss_safe_identifiers(
    api: ApiClient,
    runtime: E2eRuntime,
    links: Literal["details", "magnet", "unsafe", "download"],
) -> None:
    with html_upstream() as upstream:
        upstream.links = links
        with configure_cardigann(api, upstream) as (identifier, _):
            enable_rss(api, identifier)
            make_due(runtime, identifier)
            eventually(
                lambda: status(api, identifier).get("last_polled_at") is not None,
                "Cardigann identifier poll",
            )
            assert status(api, identifier).get("last_error_class") is None
            assert status(api, identifier).get("next_poll_at") is not None
            seen = items(api, identifier)
            assert len(seen) == (1 if links in ("details", "magnet") else 0)
            if links == "details":
                assert seen[0]["item_guid"] == upstream.origin + "/details/" + upstream.identity
            elif links == "magnet":
                assert seen[0]["magnet_hash"] is not None
                assert seen[0].get("item_guid") is None
            assert "magnet:?" not in str(seen)
            assert "apikey=" not in str(seen)


@pytest.mark.parametrize("http_status,error", [(429, "http_429"), (503, "http_5xx")])
def test_cardigann_rss_http_backoff(
    api: ApiClient,
    runtime: E2eRuntime,
    http_status: int,
    error: str,
) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (identifier, _):
        upstream.status_code = http_status
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        eventually(
            lambda: status(api, identifier).get("last_error_class") == error,
            "Cardigann HTTP failure",
        )
        assert status(api, identifier).get("backoff_seconds") == 60
        assert status(api, identifier).get("next_poll_at") is not None
        assert items(api, identifier) == []


@pytest.mark.parametrize("invalid", [False, True], ids=["empty", "malformed"])
def test_cardigann_rss_empty_or_invalid_listing(
    api: ApiClient,
    runtime: E2eRuntime,
    invalid: bool,
) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (identifier, _):
        upstream.invalid_size = invalid
        upstream.empty = not invalid
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        eventually(
            lambda: (
                status(api, identifier).get("last_error_class") == "parse_error"
                if invalid
                else status(api, identifier).get("last_polled_at") is not None
            ),
            "Cardigann listing outcome",
        )
        snapshot = status(api, identifier)
        assert snapshot.get("last_error_class") == ("parse_error" if invalid else None)
        assert (snapshot.get("next_poll_at") is None) == invalid
        assert items(api, identifier) == []


@pytest.mark.parametrize("selection", ["future", "subscription", "instance", "rss"])
def test_cardigann_rss_enablement_and_due_time(
    api: ApiClient,
    runtime: E2eRuntime,
    selection: str,
) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (identifier, _):
        enable_rss(api, identifier)
        if selection == "subscription":
            assert api.request(
                ApiRequest(
                    Method.PUT,
                    RSS_ROUTE,
                    {"is_enabled": False},
                    path={"indexer_instance_public_id": identifier},
                )
            ).ok
        elif selection != "future":
            field = "is_enabled" if selection == "instance" else "enable_rss"
            assert api.request(
                ApiRequest(
                    Method.PATCH,
                    "/v1/indexers/instances/{indexer_instance_public_id}",
                    {"indexer_instance_public_id": identifier, field: False},
                    path={"indexer_instance_public_id": identifier},
                )
            ).ok
        make_due(runtime, identifier, due=selection != "future")
        assert not upstream.requested.wait(1.5)
        assert items(api, identifier) == []


def test_cardigann_rss_ignores_search_profile_exclusion(
    api: ApiClient, runtime: E2eRuntime
) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (identifier, _):
        profile = create_profile(api, "RSS independent " + upstream.identity)
        assert (
            api.request(
                ApiRequest(
                    Method.PUT,
                    "/v1/indexers/search-profiles/{search_profile_public_id}/indexers/block",
                    {"indexer_instance_public_ids": [identifier]},
                    path={"search_profile_public_id": profile},
                )
            ).status
            == 204
        )
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        eventually(
            lambda: status(api, identifier).get("last_polled_at") is not None,
            "Cardigann RSS despite search exclusion",
        )
        assert len(items(api, identifier)) == 1


def test_cardigann_rss_disable_during_response(api: ApiClient, runtime: E2eRuntime) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (identifier, _):
        upstream.response_gate = threading.Event()
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        assert upstream.requested.wait(5)
        assert api.request(
            ApiRequest(
                Method.PUT,
                RSS_ROUTE,
                {"is_enabled": False},
                path={"indexer_instance_public_id": identifier},
            )
        ).ok
        upstream.response_gate.set()
        assert upstream.responded.wait(5)
        # The guarded fixture waits for the production RSS job to settle.
        make_due(runtime, identifier, due=False)
        snapshot = status(api, identifier)
        assert snapshot["subscription_status"] == "disabled"
        assert snapshot.get("next_poll_at") is None
        assert items(api, identifier) == []
