"""Global and scoped category mappings use this scenario's own indexer catalog.

Creating the definition and profile explicitly exercises the successful branch
on every shard; no scenario depends on another file having imported a catalog.
Missing-instance contracts are checked separately with fresh UUIDs.
"""

import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method
from revaer_tooling.json_data import JsonObject, string_value

from tests.support.api_assertions import requires_key, rows
from tests.support.indexers import cardigann_import, create_profile, create_torznab

TRACKER_ROUTE = "/v1/indexers/category-mappings/tracker"


def mapping_pair(api: ApiClient, identity: JsonObject, values: JsonObject, status: int) -> None:
    assert (
        api.request(ApiRequest(Method.POST, TRACKER_ROUTE, {**identity, **values})).status == status
    )
    assert api.request(ApiRequest(Method.DELETE, TRACKER_ROUTE, identity)).status == status


def test_category_mappings(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    requires_key(
        public_api,
        session,
        ApiRequest(
            Method.POST,
            TRACKER_ROUTE,
            {"tracker_category": 9001, "tracker_subcategory": 0, "torznab_cat_id": 2000},
        ),
    )
    mapping_pair(
        api,
        {"tracker_category": 9001, "tracker_subcategory": 0},
        {"torznab_cat_id": 2000, "media_domain_key": "movies"},
        204,
    )
    domain_route = "/v1/indexers/category-mappings/media-domains"
    domain: JsonObject = {"media_domain_key": "movies", "torznab_cat_id": 8000}
    assert (
        api.request(ApiRequest(Method.POST, domain_route, {**domain, "is_primary": False})).status
        == 204
    )
    assert api.request(ApiRequest(Method.DELETE, domain_route, domain)).status == 204

    slug = "category-e2e-" + uuid.uuid4().hex
    assert api.request(cardigann_import(slug)).status == 201
    assert any(
        entry["upstream_slug"] == slug
        for entry in rows(api, "/v1/indexers/definitions", "definitions")
    )
    created = api.request(
        ApiRequest(
            Method.POST,
            "/v1/indexers/instances",
            {
                "indexer_definition_upstream_slug": slug,
                "display_name": "E2E Category Mapping " + slug,
            },
        )
    )
    assert created.status == 201
    indexer_id = string_value(created.object()["indexer_instance_public_id"])
    profile_id = create_profile(api, "Category Profile " + slug)
    torznab = create_torznab(api, profile_id, "E2E Category Mapping App " + slug)
    mapping_pair(
        api,
        {
            "indexer_instance_public_id": indexer_id,
            "tracker_category": 9002,
            "tracker_subcategory": 1,
        },
        {"torznab_cat_id": 5000, "media_domain_key": "tv"},
        204,
    )
    mapping_pair(
        api,
        {
            "torznab_instance_public_id": torznab.identifier,
            "indexer_instance_public_id": indexer_id,
            "tracker_category": 9003,
            "tracker_subcategory": 2,
        },
        {"torznab_cat_id": 2030, "media_domain_key": "movies"},
        204,
    )


def test_mapping_rejects_missing_instances(api: ApiClient) -> None:
    missing: JsonObject = {
        "indexer_instance_public_id": str(uuid.uuid4()),
        "tracker_category": 9002,
        "tracker_subcategory": 1,
    }
    mapping_pair(api, missing, {"torznab_cat_id": 5000, "media_domain_key": "tv"}, 404)
    mapping_pair(
        api,
        {
            **missing,
            "torznab_instance_public_id": str(uuid.uuid4()),
            "tracker_category": 9003,
            "tracker_subcategory": 2,
        },
        {"torznab_cat_id": 2030, "media_domain_key": "movies"},
        404,
    )
