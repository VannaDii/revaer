"""Proxy credentials, rate limits and notification-hook lifecycles."""

import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method
from revaer_tooling.json_data import string_value

from tests.support.api_assertions import find_row, requires_key, rows


def test_routing_credentials(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    route = "/v1/indexers/routing-policies"
    suffix = uuid.uuid4().hex
    name = "E2E Routing " + suffix
    create = ApiRequest(Method.POST, route, {"display_name": name, "mode": "http_proxy"})
    requires_key(public_api, session, create)
    created = api.request(create)
    assert created.status == 201 and created.object()["display_name"] == name
    identifier = string_value(created.object()["routing_policy_public_id"])
    policy = find_row(rows(api, route, "routing_policies"), "routing_policy_public_id", identifier)
    assert policy["display_name"] == name
    path = {"routing_policy_public_id": identifier}
    item = route + "/{routing_policy_public_id}"
    assert (
        api.request(
            ApiRequest(
                Method.POST,
                item + "/params",
                {"param_key": "proxy_host", "value_plain": "localhost"},
                path=path,
            )
        ).status
        == 204
    )
    secret = api.request(
        ApiRequest(
            Method.POST,
            "/v1/indexers/secrets",
            {"secret_type": "password", "secret_value": "routing-secret-" + suffix},
        )
    )
    assert secret.status == 201, secret.object().get("context")
    secret_id = string_value(secret.object()["secret_public_id"])
    bind = ApiRequest(
        Method.POST,
        item + "/secrets",
        {"param_key": "http_proxy_auth", "secret_public_id": secret_id},
        path=path,
    )
    assert api.request(bind).status == 204
    assert (
        api.request(
            ApiRequest(Method.DELETE, "/v1/indexers/secrets", {"secret_public_id": secret_id})
        ).status
        == 204
    )
    assert api.request(bind).status == 404


def test_rate_limit_policy(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    route = "/v1/indexers/rate-limits"
    name = "E2E Rate Limit " + uuid.uuid4().hex
    create = ApiRequest(
        Method.POST, route, {"display_name": name, "rpm": 120, "burst": 30, "concurrent": 2}
    )
    requires_key(public_api, session, create)
    created = api.request(create)
    assert created.status == 201
    identifier = string_value(created.object()["rate_limit_policy_public_id"])
    policy = find_row(
        rows(api, route, "rate_limit_policies"), "rate_limit_policy_public_id", identifier
    )
    assert policy["display_name"] == name
    path = {"rate_limit_policy_public_id": identifier}
    item = route + "/{rate_limit_policy_public_id}"
    assert (
        api.request(
            ApiRequest(
                Method.PATCH, item, {"display_name": name + " Updated", "rpm": 240}, path=path
            )
        ).status
        == 204
    )
    for collection, field in (
        ("instances", "indexer_instance_public_id"),
        ("routing-policies", "routing_policy_public_id"),
    ):
        assignment = f"/v1/indexers/{collection}/{{{field}}}/rate-limit"
        assert (
            api.request(
                ApiRequest(
                    Method.PUT,
                    assignment,
                    {"rate_limit_policy_public_id": identifier},
                    path={field: str(uuid.uuid4())},
                )
            ).status
            == 404
        )
    assert api.request(ApiRequest(Method.DELETE, item, path=path)).status == 204


def test_health_notifications(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    route = "/v1/indexers/health-notifications"
    requires_key(public_api, session, ApiRequest(Method.GET, route))
    created = api.request(
        ApiRequest(
            Method.POST,
            route,
            {
                "channel": "webhook",
                "display_name": "E2E Hook " + uuid.uuid4().hex,
                "status_threshold": "failing",
                "webhook_url": "https://hooks.example.test/indexers",
            },
        )
    )
    assert created.status == 201 and created.object()["channel"] == "webhook"
    assert created.object()["webhook_url"] == "https://hooks.example.test/indexers"
    field = "indexer_health_notification_hook_public_id"
    identifier = string_value(created.object()[field])
    find_row(rows(api, route, "hooks"), field, identifier)
    updated = api.request(
        ApiRequest(
            Method.PATCH,
            route,
            {
                field: identifier,
                "display_name": "E2E Hook Updated",
                "status_threshold": "quarantined",
                "webhook_url": "https://hooks.example.test/escalation",
                "is_enabled": False,
            },
        )
    )
    assert updated.status == 200
    assert updated.object()["display_name"] == "E2E Hook Updated"
    assert updated.object()["status_threshold"] == "quarantined"
    assert updated.object()["is_enabled"] is False
    assert api.request(ApiRequest(Method.DELETE, route, {field: identifier})).status == 204
