"""Recent items must originate in the scheduled production upstream request."""

import threading

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method

from tests.conftest import E2eRuntime
from tests.support.api_assertions import rows
from tests.support.polling import eventually
from tests.support.rss import RSS_ROUTE, enable_rss, items, make_due, status
from tests.support.torznab import configure_torznab, torznab_upstream


def test_scheduled_rss_discovers_upstream_items(api: ApiClient, runtime: E2eRuntime) -> None:
    before = [torrent["id"] for torrent in rows(api, "/v1/torrents", "torrents")]
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        assert upstream.responded.wait(5), "The production scheduler must execute a due RSS poll"
        assert upstream.requests == [("", True)]
        assert upstream.parameters[0]["t"] == ["search"]
        eventually(
            lambda: status(api, identifier).get("last_polled_at") is not None,
            "RSS outcome to commit",
        )
        snapshot = status(api, identifier)
        assert snapshot.get("next_poll_at") is not None
        assert snapshot.get("last_error_class") is None
        assert any(item["item_guid"] == upstream.infohash for item in items(api, identifier))
        assert [torrent["id"] for torrent in rows(api, "/v1/torrents", "torrents")] == before


@pytest.mark.parametrize(
    "http_status, expected, retryable",
    [
        (401, "auth_error", False),
        (429, "http_429", True),
        (503, "http_5xx", True),
    ],
)
def test_scheduled_rss_classifies_http_failure(
    api: ApiClient,
    runtime: E2eRuntime,
    http_status: int,
    expected: str,
    retryable: bool,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
        upstream.status_code = http_status
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        eventually(
            lambda: status(api, identifier).get("last_error_class") == expected, "RSS HTTP failure"
        )
        snapshot = status(api, identifier)
        assert upstream.requests == [("", True)]
        assert (snapshot.get("next_poll_at") is not None) == retryable
        assert (snapshot.get("backoff_seconds") == 60) == retryable
        assert items(api, identifier) == []


@pytest.mark.parametrize("empty", [False, True], ids=["repeated", "empty"])
def test_scheduled_rss_repeat_and_empty_feed(
    api: ApiClient,
    runtime: E2eRuntime,
    empty: bool,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
        upstream.scenario = "empty" if empty else "matching"
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        eventually(
            lambda: status(api, identifier).get("last_polled_at") is not None, "first RSS poll"
        )
        before = items(api, identifier)
        first_poll = status(api, identifier).get("last_polled_at")
        make_due(runtime, identifier)
        eventually(
            lambda: status(api, identifier).get("last_polled_at") != first_poll, "second RSS poll"
        )
        assert len(upstream.requests) == 2
        assert items(api, identifier) == before
        assert len(before) == (0 if empty else 1)


def test_scheduled_rss_malformed_feed_disables_subscription(
    api: ApiClient, runtime: E2eRuntime
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
        upstream.scenario = "malformed"
        enable_rss(api, identifier)
        make_due(runtime, identifier)
        eventually(
            lambda: status(api, identifier).get("last_error_class") == "parse_error",
            "RSS parse failure",
        )
        assert status(api, identifier).get("next_poll_at") is None
        assert items(api, identifier) == []


def test_scheduled_rss_operator_disable_during_request(api: ApiClient, runtime: E2eRuntime) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
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
        snapshot = status(api, identifier)
        assert snapshot["subscription_status"] == "disabled"
        assert snapshot.get("next_poll_at") is None
        assert items(api, identifier) == []


def test_scheduled_rss_ignores_search_profile_exclusion(
    api: ApiClient, runtime: E2eRuntime
) -> None:
    from tests.support.indexers import create_profile

    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
        profile = create_profile(api, "RSS independent " + upstream.infohash)
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
            "RSS despite search exclusion",
        )
        assert upstream.requests == [("", True)]
        assert len(items(api, identifier)) == 1


@pytest.mark.parametrize("selection", ["future", "subscription", "instance", "rss"])
def test_scheduled_rss_requires_all_enablement_and_due_time(
    api: ApiClient,
    runtime: E2eRuntime,
    selection: str,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as identifier:
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
