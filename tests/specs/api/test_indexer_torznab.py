"""Torznab management and wire compatibility use successfully created instances."""

import hmac
import time
import uuid
from typing import Literal
from urllib.parse import parse_qs, urlsplit
from xml.etree import ElementTree

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method, QueryValue
from revaer_tooling.json_data import JsonObject

from tests.support.api_assertions import find_row, requires_key, rows
from tests.support.indexers import (
    TorznabInstance,
    create_import_job,
    create_profile,
    create_torznab,
    reject_missing_import_secret,
    run_backup,
)
from tests.support.torznab import configure_torznab, torznab_upstream

API_ROUTE = "/torznab/{torznab_instance_public_id}/api"
DOWNLOAD_ROUTE = (
    "/torznab/{torznab_instance_public_id}/download/{canonical_torrent_source_public_id}"
)
INSTANCE_ROUTE = "/v1/indexers/torznab-instances/{torznab_instance_public_id}"


def public_instance(
    api: ApiClient, source: str, suffix: str, domain: str = "movies"
) -> TorznabInstance:
    domains = api.request(
        ApiRequest(
            Method.PUT,
            "/v1/indexers/instances/{indexer_instance_public_id}/media-domains",
            {"media_domain_keys": [domain]},
            path={"indexer_instance_public_id": source},
        )
    )
    assert domains.status == 204
    profile = create_profile(api, "Public live search " + suffix, domain)
    allowed = api.request(
        ApiRequest(
            Method.PUT,
            "/v1/indexers/search-profiles/{search_profile_public_id}/indexers/allow",
            {"indexer_instance_public_ids": [source]},
            path={"search_profile_public_id": profile},
        )
    )
    assert allowed.status == 204
    return create_torznab(api, profile, "Public live " + suffix)


def bound_profile(api: ApiClient, instance: TorznabInstance) -> str:
    inventory = find_row(
        rows(api, "/v1/indexers/torznab-instances", "torznab_instances"),
        "torznab_instance_public_id",
        instance.identifier,
    )
    profile = inventory["search_profile_public_id"]
    assert isinstance(profile, str)
    return profile


@pytest.mark.parametrize("private_download", [False, True], ids=["magnet", "private-download"])
def test_public_torznab_returns_live_upstream_results(
    api: ApiClient,
    public_api: ApiClient,
    private_download: bool,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as source:
        upstream.private_download = private_download
        instance = public_instance(api, source, upstream.infohash)
        response = public_api.request(
            ApiRequest(
                Method.GET,
                API_ROUTE,
                path={"torznab_instance_public_id": instance.identifier},
                query={"t": "search", "q": upstream.title, "apikey": instance.api_key},
            )
        )
        assert response.status == 200
        document = ElementTree.fromstring(response.text)
        items = document.findall("./channel/item")
        assert len(items) == 1, "The public request must wait for the real upstream release"
        item = items[0]
        assert item.findtext("title") == upstream.title
        attributes = {
            attribute.attrib["name"]: attribute.attrib["value"]
            for attribute in item.findall("{http://torznab.com/schemas/2015/feed}attr")
        }
        assert attributes["infohash"] == upstream.infohash
        source_id = str(uuid.UUID(item.findtext("guid") or ""))
        link = item.findtext("link") or ""
        parsed = urlsplit(link)
        assert parsed.path == f"/torznab/{instance.identifier}/download/{source_id}"
        linked_key = parse_qs(parsed.query).get("apikey", [""])[0]
        link_authenticated = hmac.compare_digest(linked_key, instance.api_key)
        assert link_authenticated, "The generated link must use the Revaer instance credential"
        download = public_api.request(
            ApiRequest(
                Method.GET,
                DOWNLOAD_ROUTE,
                path={
                    "torznab_instance_public_id": instance.identifier,
                    "canonical_torrent_source_public_id": source_id,
                },
                query={"apikey": instance.api_key},
            )
        )
        assert download.status == 302
        expected_target = upstream.download_url if private_download else upstream.magnet
        acquisition_preserved = download.headers.get("location") == expected_target
        assert acquisition_preserved, "The authorized redirect must retain its original target"
        assert (upstream.title, True) in upstream.requests
        credentials_redacted = upstream.api_key not in response.text
        assert credentials_redacted, "Upstream credentials must stay out of feed metadata"


@pytest.mark.parametrize(
    ("parameters", "expected", "domain"),
    [
        (
            {"t": "tvsearch", "tvdbid": "0007", "season": "1", "ep": "2"},
            {"t": ["tvsearch"], "tvdbid": ["7"], "season": ["1"], "ep": ["2"]},
            "tv",
        ),
        ({"t": "tvsearch", "season": "0"}, {"t": ["tvsearch"], "season": ["0"]}, "tv"),
        (
            {"t": "movie", "imdbid": "tt0019798", "q": "tmdb:999"},
            {"t": ["movie"], "imdbid": ["0019798"]},
            "movies",
        ),
        ({"t": "movie", "tmdbid": "00042", "q": ""}, {"t": ["movie"], "tmdbid": ["42"]}, "movies"),
    ],
)
def test_public_torznab_preserves_structured_queries(
    api: ApiClient,
    public_api: ApiClient,
    parameters: dict[str, QueryValue],
    expected: dict[str, list[str]],
    domain: str,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as source:
        instance = public_instance(api, source, upstream.infohash, domain)
        query = {"q": upstream.title, **parameters}
        response = public_api.request(
            ApiRequest(
                Method.GET,
                API_ROUTE,
                path={"torznab_instance_public_id": instance.identifier},
                query={**query, "apikey": instance.api_key},
            )
        )
        assert response.status == 200
        assert upstream.parameters, "The structured request must execute against the upstream"
        observed = upstream.parameters[-1]
        assert all(observed.get(key) == value for key, value in expected.items())
        assert observed.get("q", [""]) == [str(query["q"])]
        assert len(ElementTree.fromstring(response.text).findall("./channel/item")) == 1
        assert all(authenticated for _, authenticated in upstream.requests)


@pytest.mark.parametrize("offset", [5, 18, 34, 50])
def test_public_torznab_paginates_across_internal_pages(
    api: ApiClient,
    public_api: ApiClient,
    offset: int,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as source:
        upstream.release_count = 35
        instance = public_instance(api, source, upstream.infohash)
        response = public_api.request(
            ApiRequest(
                Method.GET,
                API_ROUTE,
                path={"torznab_instance_public_id": instance.identifier},
                query={
                    "t": "search",
                    "q": upstream.title,
                    "offset": offset,
                    "limit": 10,
                    "apikey": instance.api_key,
                },
            )
        )
        assert response.status == 200
        document = ElementTree.fromstring(response.text)
        titles = [item.findtext("title") for item in document.findall("./channel/item")]
        assert titles == [
            f"{upstream.title} item {index:03}" for index in range(offset, min(offset + 10, 35))
        ]
        metadata = document.find("./channel/{http://torznab.com/schemas/2015/feed}response")
        assert metadata is not None
        assert metadata.attrib["offset"] == str(offset)
        assert int(metadata.attrib["total"]) >= min(offset + 10, 35)


def test_public_torznab_profile_excludes_enabled_upstreams(
    api: ApiClient,
    public_api: ApiClient,
) -> None:
    with (
        torznab_upstream() as included,
        torznab_upstream() as excluded,
        configure_torznab(api, included) as source,
        configure_torznab(api, excluded, exclusive=False) as excluded_source,
    ):
        instance = public_instance(api, source, included.infohash)
        other = public_instance(api, excluded_source, excluded.infohash)
        response = public_api.request(
            ApiRequest(
                Method.GET,
                API_ROUTE,
                path={"torznab_instance_public_id": instance.identifier},
                query={"t": "search", "q": included.title, "apikey": instance.api_key},
            )
        )
        assert response.status == 200
        document = ElementTree.fromstring(response.text)
        items = document.findall("./channel/item")
        assert [item.findtext("title") for item in items] == [included.title]
        assert not excluded.requests
        source_id = items[0].findtext("guid") or ""
        denied = public_api.request(
            ApiRequest(
                Method.GET,
                DOWNLOAD_ROUTE,
                path={
                    "torznab_instance_public_id": other.identifier,
                    "canonical_torrent_source_public_id": source_id,
                },
                query={"apikey": other.api_key},
            )
        )
        assert denied.status == 404
        assert not excluded.requests


@pytest.mark.parametrize("scenario", ["empty", "unauthorized", "malformed"])
def test_public_torznab_empty_or_failed_upstream(
    api: ApiClient,
    public_api: ApiClient,
    scenario: Literal["empty", "unauthorized", "malformed"],
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as source:
        assert scenario in {"empty", "unauthorized", "malformed"}
        upstream.scenario = scenario
        instance = public_instance(api, source, upstream.infohash)
        response = public_api.request(
            ApiRequest(
                Method.GET,
                API_ROUTE,
                path={"torznab_instance_public_id": instance.identifier},
                query={"t": "search", "q": upstream.title, "apikey": instance.api_key},
            )
        )
        assert response.status == 200
        document = ElementTree.fromstring(response.text)
        assert not document.findall("./channel/item")
        assert (upstream.title, True) in upstream.requests
        credentials_redacted = upstream.api_key not in response.text
        assert credentials_redacted


def test_public_torznab_bounds_wait_without_aborting_upstream(
    api: ApiClient,
    public_api: ApiClient,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as source:
        upstream.delay_seconds = 8
        instance = public_instance(api, source, upstream.infohash)
        started = time.monotonic()
        response = public_api.request(
            ApiRequest(
                Method.GET,
                API_ROUTE,
                path={"torznab_instance_public_id": instance.identifier},
                query={"t": "search", "q": upstream.title, "apikey": instance.api_key},
            )
        )
        elapsed = time.monotonic() - started
        assert response.status == 200
        assert 5 <= elapsed < 8, "The five-second result wait must fit the existing HTTP deadline"
        assert not ElementTree.fromstring(response.text).findall("./channel/item")
        assert (upstream.title, True) in upstream.requests
        assert not upstream.responded.is_set(), "The downstream deadline preceded the response"
        assert upstream.responded.wait(timeout=5), "The original upstream request must continue"


def test_public_torznab_deadline_returns_ordered_partial_results(
    api: ApiClient,
    public_api: ApiClient,
) -> None:
    with (
        torznab_upstream() as fast,
        torznab_upstream() as slow,
        configure_torznab(api, fast) as first,
        configure_torznab(api, slow, exclusive=False) as second,
    ):
        fast.release_count = 35
        slow.delay_seconds = 8
        instance = public_instance(api, first, fast.infohash)
        assert (
            api.request(
                ApiRequest(
                    Method.PUT,
                    "/v1/indexers/instances/{indexer_instance_public_id}/media-domains",
                    {"media_domain_keys": ["movies"]},
                    path={"indexer_instance_public_id": second},
                )
            ).status
            == 204
        )
        assert (
            api.request(
                ApiRequest(
                    Method.PUT,
                    "/v1/indexers/search-profiles/{search_profile_public_id}/indexers/allow",
                    {"indexer_instance_public_ids": [first, second]},
                    path={"search_profile_public_id": bound_profile(api, instance)},
                )
            ).status
            == 204
        )
        started = time.monotonic()
        response = public_api.request(
            ApiRequest(
                Method.GET,
                API_ROUTE,
                path={"torznab_instance_public_id": instance.identifier},
                query={"t": "search", "q": fast.title, "limit": 200, "apikey": instance.api_key},
            )
        )
        elapsed = time.monotonic() - started
        assert response.status == 200
        assert 5 <= elapsed < 8
        document = ElementTree.fromstring(response.text)
        assert [item.findtext("title") for item in document.findall("./channel/item")] == [
            f"{fast.title} item {index:03}" for index in range(20)
        ]
        assert (fast.title, True) in slow.requests
        assert not slow.responded.is_set()
        assert slow.responded.wait(timeout=10), (
            "The pending upstream must outlive the HTTP deadline"
        )


def test_public_torznab_rejected_queries_never_execute(
    api: ApiClient,
    public_api: ApiClient,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as source:
        instance = public_instance(api, source, upstream.infohash)
        path = {"torznab_instance_public_id": instance.identifier}
        for parameters in (
            {"t": "unsupported"},
            {"t": "tvsearch", "ep": "2"},
            {"t": "movie", "imdbid": "tt0019798", "tmdbid": "42"},
            {"t": "search", "cat": "999999"},
            {"t": "search", "offset": "-1"},
        ):
            response = public_api.request(
                ApiRequest(
                    Method.GET,
                    API_ROUTE,
                    path=path,
                    query={"q": upstream.title, **parameters, "apikey": instance.api_key},
                )
            )
            assert response.status == 200
            assert not ElementTree.fromstring(response.text).findall("./channel/item")
        for key in ("", "invalid"):
            response = public_api.request(
                ApiRequest(
                    Method.GET,
                    API_ROUTE,
                    path=path,
                    query={"t": "search", "q": upstream.title, "apikey": key},
                )
            )
            assert response.status == 401
        # The real serial dispatcher drains earlier work before this valid search.
        response = public_api.request(
            ApiRequest(
                Method.GET,
                API_ROUTE,
                path=path,
                query={"t": "search", "q": upstream.title, "apikey": instance.api_key},
            )
        )
        assert response.status == 200
        assert len(ElementTree.fromstring(response.text).findall("./channel/item")) == 1
        assert upstream.requests == [(upstream.title, True)]


@pytest.mark.parametrize(
    ("category", "expected_upstream"),
    [("2030,5000,999999", "2030"), ("8000", "8000"), ("5000", None)],
)
def test_public_torznab_categories_respect_profile_and_mapping(
    api: ApiClient,
    public_api: ApiClient,
    category: str,
    expected_upstream: str | None,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as source:
        instance = public_instance(api, source, upstream.infohash)
        profile = bound_profile(api, instance)
        assert (
            api.request(
                ApiRequest(
                    Method.PUT,
                    "/v1/indexers/search-profiles/{search_profile_public_id}/media-domains",
                    {"media_domain_keys": ["movies"]},
                    path={"search_profile_public_id": profile},
                )
            ).status
            == 204
        )
        identity: JsonObject = {
            "indexer_instance_public_id": source,
            "tracker_category": 2000,
            "tracker_subcategory": 0,
        }
        mapping_route = "/v1/indexers/category-mappings/tracker"
        assert (
            api.request(
                ApiRequest(
                    Method.POST,
                    mapping_route,
                    {**identity, "torznab_cat_id": 2030, "media_domain_key": "movies"},
                )
            ).status
            == 204
        )
        try:
            response = public_api.request(
                ApiRequest(
                    Method.GET,
                    API_ROUTE,
                    path={"torznab_instance_public_id": instance.identifier},
                    query={
                        "t": "search",
                        "q": upstream.title,
                        "cat": category,
                        "apikey": instance.api_key,
                    },
                )
            )
            assert response.status == 200
            items = ElementTree.fromstring(response.text).findall("./channel/item")
            if expected_upstream is None:
                assert not items
                assert not upstream.requests
            else:
                assert upstream.parameters[-1].get("cat") == [expected_upstream]
                assert len(items) == 1
                assert {element.text for element in items[0].findall("category")} == {
                    "2000",
                    "2030",
                }
        finally:
            assert api.request(ApiRequest(Method.DELETE, mapping_route, identity)).status == 204


def test_public_torznab_rate_denial_never_contacts_upstream(
    api: ApiClient,
    public_api: ApiClient,
) -> None:
    with torznab_upstream() as upstream, configure_torznab(api, upstream) as source:
        instance = public_instance(api, source, upstream.infohash)
        policy = api.request(
            ApiRequest(
                Method.POST,
                "/v1/indexers/rate-limits",
                {
                    "display_name": "Public rate " + upstream.infohash,
                    "rpm": 1,
                    "burst": 0,
                    "concurrent": 1,
                },
            )
        )
        assert policy.status == 201
        identifier = policy.object()["rate_limit_policy_public_id"]
        assert isinstance(identifier, str)
        route = "/v1/indexers/instances/{indexer_instance_public_id}/rate-limit"
        path = {"indexer_instance_public_id": source}
        assert (
            api.request(
                ApiRequest(
                    Method.PUT, route, {"rate_limit_policy_public_id": identifier}, path=path
                )
            ).status
            == 204
        )
        try:
            empty_queries = set()
            submitted_queries = set()
            for index in range(3):
                query = upstream.title + f" rate {index}"
                submitted_queries.add(query)
                response = public_api.request(
                    ApiRequest(
                        Method.GET,
                        API_ROUTE,
                        path={"torznab_instance_public_id": instance.identifier},
                        query={"t": "search", "q": query, "apikey": instance.api_key},
                    )
                )
                assert response.status == 200
                if not ElementTree.fromstring(response.text).findall("./channel/item"):
                    empty_queries.add(query)
            observed = {query for query, _ in upstream.requests}
            # The unchanged 30-second test bound spans at most two minute buckets.
            # A deferred search may legitimately retry after the bucket resets.
            assert len(upstream.requests) <= 2
            uncontacted = submitted_queries - observed
            assert uncontacted
            assert uncontacted <= empty_queries
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
                        path={"rate_limit_policy_public_id": identifier},
                    )
                ).status
                == 204
            )


def test_torznab_instance_lifecycle(
    api: ApiClient, public_api: ApiClient, session: ApiSession
) -> None:
    suffix = uuid.uuid4().hex
    profile = create_profile(api, "Torznab Profile " + suffix)
    requires_key(
        public_api,
        session,
        ApiRequest(
            Method.POST,
            "/v1/indexers/torznab-instances",
            {"search_profile_public_id": profile, "display_name": "Torznab " + suffix},
        ),
    )
    instance = create_torznab(api, profile, "Torznab " + suffix)
    entry = find_row(
        rows(api, "/v1/indexers/torznab-instances", "torznab_instances"),
        "torznab_instance_public_id",
        instance.identifier,
    )
    assert entry["search_profile_public_id"] == profile
    path = {"torznab_instance_public_id": instance.identifier}
    for query in ({"t": "caps"}, {"t": "caps", "apikey": "invalid"}):
        assert (
            public_api.request(ApiRequest(Method.GET, API_ROUTE, path=path, query=query)).status
            == 401
        )
    searches: tuple[tuple[dict[str, QueryValue], tuple[str, ...]], ...] = (
        ({"t": "caps"}, ("<caps>",)),
        ({"t": "invalid-query"}, ("<rss",)),
        (
            {"t": "search", "q": "example", "offset": "5", "limit": "2"},
            ("<rss", 'torznab:response offset="5"'),
        ),
        ({"t": "tvsearch", "ep": "2"}, ('torznab:response offset="0" total="0"',)),
        ({"t": "search", "cat": "999999"}, ('torznab:response offset="0" total="0"',)),
    )
    for parameters, expected in searches:
        response = public_api.request(
            ApiRequest(
                Method.GET, API_ROUTE, path=path, query={**parameters, "apikey": instance.api_key}
            )
        )
        assert response.status == 200
        for text in expected:
            assert text in response.text
    missing_path = {"torznab_instance_public_id": str(uuid.uuid4())}
    assert (
        api.request(ApiRequest(Method.PATCH, INSTANCE_ROUTE + "/rotate", path=missing_path)).status
        == 404
    )
    assert (
        api.request(
            ApiRequest(
                Method.PUT, INSTANCE_ROUTE + "/state", {"is_enabled": True}, path=missing_path
            )
        ).status
        == 404
    )
    assert (
        public_api.request(
            ApiRequest(
                Method.GET, API_ROUTE, path=missing_path, query={"apikey": "invalid", "t": "caps"}
            )
        ).status
        == 404
    )
    source_id = str(uuid.uuid4())
    download_path = {**missing_path, "canonical_torrent_source_public_id": source_id}
    if session.auth_mode == "api_key":
        assert (
            public_api.request(ApiRequest(Method.GET, DOWNLOAD_ROUTE, path=download_path)).status
            == 401
        )
    assert (
        public_api.request(
            ApiRequest(Method.GET, DOWNLOAD_ROUTE, path=download_path, query={"apikey": "invalid"})
        ).status
        == 404
    )

    assert (
        api.request(
            ApiRequest(Method.PUT, INSTANCE_ROUTE + "/state", {"is_enabled": False}, path=path)
        ).status
        == 204
    )
    disabled = find_row(
        rows(api, "/v1/indexers/torznab-instances", "torznab_instances"),
        "torznab_instance_public_id",
        instance.identifier,
    )
    assert disabled["is_enabled"] is False
    download_path = {**path, "canonical_torrent_source_public_id": source_id}
    downloads: tuple[tuple[dict[str, QueryValue], int], ...] = (
        ({}, 401),
        ({"apikey": "invalid"}, 404),
        ({"apikey": instance.api_key}, 404),
    )
    for parameters, status in downloads:
        assert (
            public_api.request(
                ApiRequest(Method.GET, DOWNLOAD_ROUTE, path=download_path, query=parameters)
            ).status
            == status
        )
    assert api.request(ApiRequest(Method.DELETE, INSTANCE_ROUTE, path=missing_path)).status == 404


def test_migration_parity(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    suffix = uuid.uuid4().hex
    profile = create_profile(api, "Parity Profile " + suffix)
    instance = create_torznab(api, profile, "Parity Torznab " + suffix)
    path = {"torznab_instance_public_id": instance.identifier}
    caps = public_api.request(
        ApiRequest(
            Method.GET, API_ROUTE, path=path, query={"apikey": instance.api_key, "t": "caps"}
        )
    )
    assert caps.status == 200 and "<caps>" in caps.text
    invalid = public_api.request(
        ApiRequest(
            Method.GET,
            API_ROUTE,
            path=path,
            query={"apikey": instance.api_key, "t": "tvsearch", "ep": "2"},
        )
    )
    assert invalid.status == 200 and 'torznab:response offset="0" total="0"' in invalid.text
    download_path = {**path, "canonical_torrent_source_public_id": str(uuid.uuid4())}
    assert (
        public_api.request(
            ApiRequest(
                Method.GET, DOWNLOAD_ROUTE, path=download_path, query={"apikey": instance.api_key}
            )
        ).status
        == 404
    )
    if session.auth_mode == "api_key":
        assert (
            public_api.request(ApiRequest(Method.GET, DOWNLOAD_ROUTE, path=download_path)).status
            == 401
        )
    reject_missing_import_secret(api, create_import_job(api, "prowlarr_api"))
    run_backup(api, create_import_job(api, "prowlarr_backup"), "parity-e2e-backup")
