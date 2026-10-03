"""Authentication and activation boundaries match the selected phase."""

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method


def test_refresh_matches_auth_mode(api: ApiClient, session: ApiSession) -> None:
    response = api.request(ApiRequest(Method.POST, "/v1/auth/refresh"))
    if session.auth_mode == "api_key":
        assert response.ok and response.object()["api_key_expires_at"]
    else:
        assert response.status == 401


def test_protected_endpoint_requires_key(public_api: ApiClient, session: ApiSession) -> None:
    response = public_api.request(ApiRequest(Method.GET, "/v1/config"))
    if session.auth_mode == "api_key":
        assert response.status == 401
    else:
        assert response.ok


def test_setup_rejects_after_activation(public_api: ApiClient) -> None:
    for route in ("/admin/setup/start", "/admin/setup/complete"):
        assert public_api.request(ApiRequest(Method.POST, route, {})).status == 409
