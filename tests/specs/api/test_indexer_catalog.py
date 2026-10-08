"""Indexer definitions, Cardigann import and missing-instance error contracts."""

import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method, QueryValue
from revaer_tooling.json_data import JsonObject, object_value, string_value

from tests.support.api_assertions import requires_key, rows
from tests.support.indexers import cardigann_import


def test_definitions(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    route = "/v1/indexers/definitions"
    requires_key(public_api, session, ApiRequest(Method.GET, route))
    for definition in rows(api, route, "definitions"):
        assert definition["upstream_source"]
        assert definition["upstream_slug"]
        assert definition["display_name"]
        assert len(string_value(definition["definition_hash"])) == 64


def test_cardigann_import(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    slug = "cardigann-e2e-" + uuid.uuid4().hex
    request = cardigann_import(slug)
    requires_key(public_api, session, request)
    imported = api.request(request)
    assert imported.status == 201
    definition = object_value(imported.object()["definition"])
    assert definition["upstream_source"] == "cardigann"
    assert definition["upstream_slug"] == slug
    assert imported.object()["field_count"] == 2
    assert imported.object()["option_count"] == 2
    assert any(
        entry["upstream_source"] == "cardigann" and entry["upstream_slug"] == slug
        for entry in rows(api, "/v1/indexers/definitions", "definitions")
    )


def test_missing_instance_endpoints(
    api: ApiClient, public_api: ApiClient, session: ApiSession
) -> None:
    route = "/v1/indexers/instances"
    body: JsonObject = {
        "indexer_definition_upstream_slug": "missing-definition",
        "display_name": "E2E Instance " + uuid.uuid4().hex,
    }
    requires_key(public_api, session, ApiRequest(Method.GET, route))
    requires_key(public_api, session, ApiRequest(Method.POST, route, body))
    rows(api, route, "indexer_instances")
    assert api.request(ApiRequest(Method.POST, route, body)).status == 404
    identifier = str(uuid.uuid4())
    path = {"indexer_instance_public_id": identifier}
    route += "/{indexer_instance_public_id}"
    mutations: tuple[tuple[Method, str, JsonObject], ...] = (
        (Method.PATCH, "", {"indexer_instance_public_id": identifier, "display_name": "Updated"}),
        (Method.PUT, "/media-domains", {"media_domain_keys": ["movies"]}),
        (Method.PUT, "/tags", {"tag_keys": ["e2e-tag"]}),
        (Method.PATCH, "/fields/value", {"field_name": "api_key", "value_plain": "e2e-value"}),
        (
            Method.PATCH,
            "/fields/secret",
            {"field_name": "api_key", "secret_public_id": str(uuid.uuid4())},
        ),
        (Method.POST, "/cf-state/reset", {"reason": "e2e reset"}),
        (Method.PUT, "/rss", {"is_enabled": True, "interval_seconds": 900}),
        (Method.POST, "/rss/items", {"item_guid": "e2e-guid"}),
    )
    for method, suffix, value in mutations:
        assert api.request(ApiRequest(method, route + suffix, value, path=path)).status == 404
    for suffix in ("/cf-state", "/connectivity-profile", "/rss"):
        assert api.request(ApiRequest(Method.GET, route + suffix, path=path)).status == 404
    for suffix in ("/reputation", "/health-events", "/rss/items"):
        query: dict[str, QueryValue] = (
            {"limit": 10, "window_key": "1h"} if suffix == "/reputation" else {"limit": 10}
        )
        assert (
            api.request(ApiRequest(Method.GET, route + suffix, path=path, query=query)).status
            == 404
        )
