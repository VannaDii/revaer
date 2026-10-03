"""Common API assertions retain typed requests and useful operation failures."""

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method
from revaer_tooling.json_data import Json, JsonObject, array_value, object_value


def requires_key(public_api: ApiClient, session: ApiSession, request: ApiRequest) -> None:
    if session.auth_mode == "api_key":
        assert public_api.request(request).status == 401


def rows(api: ApiClient, route: str, collection: str) -> list[JsonObject]:
    response = api.request(ApiRequest(Method.GET, route))
    assert response.status == 200
    return [object_value(entry) for entry in array_value(response.object()[collection])]


def find_row(entries: list[JsonObject], field: str, value: Json) -> JsonObject:
    matches = [entry for entry in entries if entry.get(field) == value]
    assert len(matches) == 1, f"Expected exactly one matching {field}"
    return matches[0]
