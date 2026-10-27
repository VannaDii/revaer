"""Public Torznab requests exercise imported HTML rules through the shared dispatcher."""

from xml.etree import ElementTree

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method

from tests.specs.api.test_indexer_torznab import API_ROUTE, bound_profile, public_instance
from tests.support.cardigann import configure_cardigann, html_upstream
from tests.support.indexers import TorznabInstance
from tests.support.torznab import configure_torznab, torznab_upstream


def search(
    public_api: ApiClient, instance: TorznabInstance, query: str, mode: str = "search"
) -> ElementTree.Element:
    response = public_api.request(
        ApiRequest(
            Method.GET,
            API_ROUTE,
            path={"torznab_instance_public_id": instance.identifier},
            query={"t": mode, "q": query, "apikey": instance.api_key},
        )
    )
    assert response.status == 200
    return ElementTree.fromstring(response.text)


def test_cardigann_public_feed_and_atomic_rejected_reimport(
    api: ApiClient, public_api: ApiClient
) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (source, _profile):
        instance = public_instance(api, source, upstream.identity)
        rejected = api.request(
            ApiRequest(
                Method.POST,
                "/v1/indexers/definitions/import/cardigann",
                {"yaml_payload": upstream.definition().replace("method: get", "method: post")},
            )
        )
        assert rejected.status == 400
        assert "cardigann_requires_supported_public_get_html_rules" in rejected.text
        assert upstream.requests == []
        document = search(public_api, instance, upstream.title)
        releases = document.findall("./channel/item")
        assert len(releases) == 1
        assert releases[0].findtext("title") == upstream.title
        attributes = {
            item.attrib["name"]: item.attrib["value"]
            for item in releases[0].findall("{http://torznab.com/schemas/2015/feed}attr")
        }
        assert releases[0].findtext("category") == "2000"
        assert attributes["seeders"] == "17"
        assert len(upstream.requests) == 1


@pytest.mark.parametrize("invalid", [False, True], ids=["empty", "invalid-size"])
def test_cardigann_empty_and_invalid_fields(
    api: ApiClient, public_api: ApiClient, invalid: bool
) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (source, _profile):
        upstream.empty = not invalid
        upstream.invalid_size = invalid
        instance = public_instance(api, source, upstream.identity)
        assert search(public_api, instance, upstream.title).findall("./channel/item") == []
        assert upstream.requests


@pytest.mark.parametrize("exclusion", ["disabled", "profile"])
def test_cardigann_disabled_and_excluded_do_not_request(
    api: ApiClient, public_api: ApiClient, exclusion: str
) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (source, _profile):
        instance = public_instance(api, source, upstream.identity)
        if exclusion == "disabled":
            response = api.request(
                ApiRequest(
                    Method.PATCH,
                    "/v1/indexers/instances/{indexer_instance_public_id}",
                    {"is_enabled": False},
                    path={"indexer_instance_public_id": source},
                )
            )
            assert response.ok
        else:
            cleared = api.request(
                ApiRequest(
                    Method.PUT,
                    "/v1/indexers/search-profiles/{search_profile_public_id}/indexers/allow",
                    {"indexer_instance_public_ids": []},
                    path={"search_profile_public_id": bound_profile(api, instance)},
                )
            )
            assert cleared.status == 204
            response = api.request(
                ApiRequest(
                    Method.PUT,
                    "/v1/indexers/search-profiles/{search_profile_public_id}/indexers/block",
                    {"indexer_instance_public_ids": [source]},
                    path={"search_profile_public_id": bound_profile(api, instance)},
                )
            )
            assert response.status == 204
        assert search(public_api, instance, upstream.title).findall("./channel/item") == []
        assert upstream.requests == []


def test_cardigann_structured_mode_is_not_a_generic_search(
    api: ApiClient, public_api: ApiClient
) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (source, _profile):
        instance = public_instance(api, source, upstream.identity)
        assert search(public_api, instance, upstream.title, "movie").findall("./channel/item") == []
        assert upstream.requests == []


def test_cardigann_and_torznab_aggregate_one_canonical_release(
    api: ApiClient, public_api: ApiClient
) -> None:
    with (
        html_upstream() as html,
        configure_cardigann(api, html) as (html_source, _profile),
        torznab_upstream() as xml,
        configure_torznab(api, xml, exclusive=False) as xml_source,
    ):
        xml.infohash = html.identity
        instance = public_instance(api, html_source, html.identity)
        response = api.request(
            ApiRequest(
                Method.PUT,
                "/v1/indexers/instances/{indexer_instance_public_id}/media-domains",
                {"media_domain_keys": ["movies"]},
                path={"indexer_instance_public_id": xml_source},
            )
        )
        assert response.status == 204
        response = api.request(
            ApiRequest(
                Method.PUT,
                "/v1/indexers/search-profiles/{search_profile_public_id}/indexers/allow",
                {"indexer_instance_public_ids": [html_source, xml_source]},
                path={"search_profile_public_id": bound_profile(api, instance)},
            )
        )
        assert response.status == 204
        releases = search(public_api, instance, html.title).findall("./channel/item")
        assert len(releases) == 1
        assert html.requests
        assert xml.requests


def test_cardigann_rate_denial_preserves_shared_budget(
    api: ApiClient, public_api: ApiClient
) -> None:
    with html_upstream() as upstream, configure_cardigann(api, upstream) as (source, _profile):
        upstream.empty = True
        instance = public_instance(api, source, upstream.identity)
        created = api.request(
            ApiRequest(
                Method.POST,
                "/v1/indexers/rate-limits",
                {
                    "display_name": "HTML budget " + upstream.identity,
                    "rpm": 1,
                    "burst": 0,
                    "concurrent": 1,
                },
            )
        )
        assert created.status == 201
        policy = created.object()["rate_limit_policy_public_id"]
        assert isinstance(policy, str)
        route = "/v1/indexers/instances/{indexer_instance_public_id}/rate-limit"
        path = {"indexer_instance_public_id": source}
        assert (
            api.request(
                ApiRequest(Method.PUT, route, {"rate_limit_policy_public_id": policy}, path=path)
            ).status
            == 204
        )
        try:
            for index in range(3):
                assert (
                    search(public_api, instance, upstream.title + f" budget {index}").findall(
                        "./channel/item"
                    )
                    == []
                )
            # Three attempts inside the existing deadline span at most two minute buckets.
            assert len(upstream.requests) <= 2
        finally:
            assert (
                api.request(
                    ApiRequest(Method.PUT, route, {"rate_limit_policy_public_id": None}, path=path)
                ).status
                == 204
            )
            assert (
                api.request(
                    ApiRequest(
                        Method.DELETE,
                        "/v1/indexers/rate-limits/{rate_limit_policy_public_id}",
                        path={"rate_limit_policy_public_id": policy},
                    )
                ).status
                == 204
            )
