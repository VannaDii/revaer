"""Torznab management and wire compatibility use successfully created instances."""

import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method, QueryValue

from tests.support.api_assertions import find_row, requires_key, rows
from tests.support.indexers import (
    create_import_job,
    create_profile,
    create_torznab,
    reject_missing_import_secret,
    run_backup,
)

API_ROUTE = "/torznab/{torznab_instance_public_id}/api"
DOWNLOAD_ROUTE = (
    "/torznab/{torznab_instance_public_id}/download/{canonical_torrent_source_public_id}"
)
INSTANCE_ROUTE = "/v1/indexers/torznab-instances/{torznab_instance_public_id}"


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
    assert caps.status == 200
    assert "<caps>" in caps.text
    invalid = public_api.request(
        ApiRequest(
            Method.GET,
            API_ROUTE,
            path=path,
            query={"apikey": instance.api_key, "t": "tvsearch", "ep": "2"},
        )
    )
    assert invalid.status == 200
    assert 'torznab:response offset="0" total="0"' in invalid.text
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
