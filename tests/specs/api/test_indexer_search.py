"""Search validation, page retrieval and cancellation of missing requests."""

import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method
from revaer_tooling.json_data import array_value, object_value, string_value

from tests.support.api_assertions import requires_key


def test_search_request(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    route = "/v1/indexers/search-requests"
    create = ApiRequest(Method.POST, route, {"query_text": "Dune", "query_type": "free_text"})
    requires_key(public_api, session, create)
    assert (
        api.request(
            ApiRequest(Method.POST, route, {"query_text": "Dune", "query_type": "   "})
        ).status
        == 400
    )
    created = api.request(create)
    assert created.status == 201
    identifier = string_value(created.object()["search_request_public_id"])
    path = {"search_request_public_id": identifier}
    pages_route = route + "/{search_request_public_id}/pages"
    pages = api.request(ApiRequest(Method.GET, pages_route, path=path))
    assert pages.status == 200
    entries = array_value(pages.object()["pages"])
    number = object_value(entries[0])["page_number"] if entries else 1
    assert isinstance(number, int) and not isinstance(number, bool)
    assert (
        api.request(
            ApiRequest(
                Method.GET,
                pages_route + "/{page_number}",
                path={**path, "page_number": str(number)},
            )
        ).status
        == 200
    )
    cancel = ApiRequest(
        Method.POST,
        route + "/{search_request_public_id}/cancel",
        path={"search_request_public_id": str(uuid.uuid4())},
    )
    requires_key(public_api, session, cancel)
    assert api.request(cancel).status == 404
