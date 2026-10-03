"""Independent CRUD lifecycles for secret rotation and both tag deletion APIs."""

import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method
from revaer_tooling.json_data import string_value

from tests.support.api_assertions import find_row, requires_key, rows


def test_secret_rotation(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    route = "/v1/indexers/secrets"
    secret = "e2e-secret-" + uuid.uuid4().hex
    create = ApiRequest(Method.POST, route, {"secret_type": "api_key", "secret_value": secret})
    requires_key(public_api, session, create)
    created = api.request(create)
    assert created.status == 201
    identifier = string_value(created.object()["secret_public_id"])
    find_row(rows(api, route, "secrets"), "secret_public_id", identifier)
    rotated = api.request(
        ApiRequest(
            Method.PATCH,
            route,
            {"secret_public_id": identifier, "secret_value": secret + "-rotated"},
        )
    )
    assert rotated.ok and rotated.object()["secret_public_id"] == identifier
    assert (
        api.request(ApiRequest(Method.DELETE, route, {"secret_public_id": identifier})).status
        == 204
    )


def test_tag_lifecycle(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    route = "/v1/indexers/tags"
    suffix = uuid.uuid4().hex
    key, name = "e2e-tag-" + suffix, "E2E Tag " + suffix
    create = ApiRequest(Method.POST, route, {"tag_key": key, "display_name": name})
    requires_key(public_api, session, create)
    created = api.request(create)
    assert created.status == 201 and created.object()["display_name"] == name
    identifier = string_value(created.object()["tag_public_id"])
    find_row(rows(api, route, "tags"), "tag_key", key)
    updated = api.request(
        ApiRequest(
            Method.PATCH, route, {"tag_public_id": identifier, "display_name": name + " Updated"}
        )
    )
    assert updated.ok and updated.object()["display_name"] == name + " Updated"
    assert (
        api.request(ApiRequest(Method.DELETE, route + "/{tag_key}", path={"tag_key": key})).status
        == 204
    )
    second = api.request(
        ApiRequest(Method.POST, route, {"tag_key": key + "-body", "display_name": name + " Body"})
    )
    assert second.status == 201
    second_id = string_value(second.object()["tag_public_id"])
    assert api.request(ApiRequest(Method.DELETE, route, {"tag_public_id": second_id})).status == 204
