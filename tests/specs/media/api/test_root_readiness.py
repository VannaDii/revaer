"""Retain persisted missing-root state and fail-closed catalog paging."""

from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method


def test_catalog_does_not_invent_slots(api: ApiClient) -> None:
    response = api.request(ApiRequest(Method.GET, "/v1/media/root-catalog"))
    assert response.status == 200
    assert response.headers["cache-control"] == "no-store"
    assert response.object() == {
        "format_version": 1,
        "source_state": "missing",
        "source_reason": "media_root_catalog_source_missing",
        "attestation_state": "not_evaluated",
        "slots": [],
    }


def test_invalid_page_returns_problem_without_paths(api: ApiClient) -> None:
    response = api.request(ApiRequest(Method.GET, "/v1/media/root-catalog", query={"limit": 0}))
    assert response.status == 400
    assert response.headers["cache-control"] == "no-store"
    assert response.headers["content-type"] == "application/problem+json"
    assert response.object()["context"] == [
        {"name": "error_code", "value": "media_configuration_invalid"}
    ]


def test_readiness_does_not_invent_attestation(api: ApiClient) -> None:
    response = api.request(ApiRequest(Method.GET, "/v1/media/root-catalog/readiness"))
    assert response.status == 200
    assert response.headers["cache-control"] == "no-store"
    assert response.object() == {
        "format_version": 1,
        "source_state": "missing",
        "source_reason": "media_root_catalog_source_missing",
        "attestation_state": "not_evaluated",
        "kinds": [
            {
                "kind": kind,
                "attested_slot_count": 0,
                "binding_ready_slot_count": 0,
                "destructive_ready_slot_count": 0,
            }
            for kind in ("source", "output", "workspace", "backup", "quarantine")
        ],
    }
