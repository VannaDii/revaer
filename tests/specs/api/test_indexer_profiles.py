"""Search profiles, domain defaults and tag/indexer selection policies."""

import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method
from revaer_tooling.json_data import JsonObject, array_value, string_value

from tests.support.api_assertions import find_row, requires_key, rows


def test_search_profile_lifecycle(
    api: ApiClient, public_api: ApiClient, session: ApiSession
) -> None:
    route = "/v1/indexers/search-profiles"
    suffix = uuid.uuid4().hex
    name = "E2E Search Profile " + suffix
    requires_key(public_api, session, ApiRequest(Method.POST, route, {"display_name": name}))
    created = api.request(
        ApiRequest(
            Method.POST,
            route,
            {"display_name": name, "page_size": 20, "default_media_domain_key": "movies"},
        )
    )
    assert created.status == 201
    identifier = string_value(created.object()["search_profile_public_id"])
    path = {"search_profile_public_id": identifier}
    item = route + "/{search_profile_public_id}"
    assert api.request(
        ApiRequest(
            Method.PATCH, item, {"display_name": name + " Updated", "page_size": 25}, path=path
        )
    ).ok
    profile = find_row(rows(api, route, "search_profiles"), "search_profile_public_id", identifier)
    assert profile["display_name"] == name + " Updated"
    changes: tuple[tuple[Method, str, JsonObject], ...] = (
        (Method.POST, "/default", {"page_size": 30}),
        (Method.PUT, "/default-domain", {"default_media_domain_key": "tv"}),
        (Method.PUT, "/media-domains", {"media_domain_keys": ["movies", "tv"]}),
    )
    for method, suffix_route, body in changes:
        assert api.request(ApiRequest(method, item + suffix_route, body, path=path)).status == 204
    keys: dict[str, str] = {}
    for letter, selection in (("a", "allow"), ("b", "block"), ("c", "prefer")):
        key = f"e2e-tag-{letter}-{suffix}"
        tag = api.request(
            ApiRequest(
                Method.POST,
                "/v1/indexers/tags",
                {"tag_key": key, "display_name": f"E2E Tag {letter.upper()} {suffix}"},
            )
        )
        assert tag.status == 201
        tag_id = string_value(tag.object()["tag_public_id"])
        keys[selection] = key
        assert (
            api.request(
                ApiRequest(
                    Method.PUT, item + "/tags/" + selection, {"tag_public_ids": [tag_id]}, path=path
                )
            ).status
            == 204
        )
    profile = find_row(rows(api, route, "search_profiles"), "search_profile_public_id", identifier)
    for selection, key in keys.items():
        assert key in array_value(profile[selection + "_tag_keys"])
    for method in (Method.POST, Method.DELETE):
        assert (
            api.request(
                ApiRequest(
                    method,
                    item + "/policy-sets",
                    {"policy_set_public_id": str(uuid.uuid4())},
                    path=path,
                )
            ).status
            == 404
        )
    for selection in ("allow", "block"):
        assert (
            api.request(
                ApiRequest(
                    Method.PUT,
                    item + "/indexers/" + selection,
                    {"indexer_instance_public_ids": [str(uuid.uuid4())]},
                    path=path,
                )
            ).status
            == 404
        )
