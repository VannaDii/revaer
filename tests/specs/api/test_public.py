"""Public health, discovery and metrics remain reachable in both auth modes."""

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method


@pytest.mark.parametrize("route", ["/health", "/health/full"])
def test_health(public_api: ApiClient, route: str) -> None:
    response = public_api.request(ApiRequest(Method.GET, route))
    assert response.ok
    body = response.object()
    assert body["status"] and body["mode"]
    if route == "/health/full":
        assert "revision" in body


def test_well_known_snapshot(public_api: ApiClient) -> None:
    response = public_api.request(ApiRequest(Method.GET, "/.well-known/revaer.json"))
    assert response.ok and response.object()["app_profile"]


def test_metrics(public_api: ApiClient) -> None:
    response = public_api.request(ApiRequest(Method.GET, "/metrics"))
    assert response.ok and response.text.strip()


def test_openapi_document(public_api: ApiClient) -> None:
    response = public_api.request(ApiRequest(Method.GET, "/docs/openapi.json"))
    assert response.ok
    assert response.object()["openapi"] and response.object()["paths"]
