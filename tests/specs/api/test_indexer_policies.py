"""Policy sets and rules retain their ordering, activation and validation APIs."""

import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method
from revaer_tooling.json_data import JsonObject, array_value, object_value, string_value

from tests.support.api_assertions import find_row, requires_key, rows


def test_policy_set_and_rules(api: ApiClient, public_api: ApiClient, session: ApiSession) -> None:
    route = "/v1/indexers/policies"
    name = "E2E Policy Set " + uuid.uuid4().hex
    create = ApiRequest(
        Method.POST, route, {"display_name": name, "scope": "global", "enabled": True}
    )
    requires_key(public_api, session, create)
    created = api.request(create)
    assert created.status == 201
    identifier = string_value(created.object()["policy_set_public_id"])
    path = {"policy_set_public_id": identifier}
    item = route + "/{policy_set_public_id}"
    assert api.request(
        ApiRequest(Method.PATCH, item, {"display_name": name + " Updated"}, path=path)
    ).ok
    policy = find_row(rows(api, route, "policy_sets"), "policy_set_public_id", identifier)
    assert policy["display_name"] == name + " Updated"
    for action in ("enable", "disable"):
        assert api.request(ApiRequest(Method.POST, item + "/" + action, path=path)).status == 204
    assert (
        api.request(
            ApiRequest(
                Method.POST, route + "/reorder", {"ordered_policy_set_public_ids": [identifier]}
            )
        ).status
        == 204
    )

    rule: JsonObject = {
        "rule_type": "block_title_regex",
        "match_field": "title",
        "match_operator": "regex",
        "sort_order": 10,
        "match_value_text": "sample",
        "action": "drop_canonical",
        "severity": "hard",
        "is_case_insensitive": True,
        "rationale": "e2e test",
    }
    created_rule = api.request(ApiRequest(Method.POST, item + "/rules", rule, path=path))
    assert created_rule.status == 201
    rule_id = string_value(created_rule.object()["policy_rule_public_id"])
    policy = find_row(rows(api, route, "policy_sets"), "policy_set_public_id", identifier)
    find_row(
        [object_value(entry) for entry in array_value(policy["rules"])],
        "policy_rule_public_id",
        rule_id,
    )
    for action in ("enable", "disable"):
        assert (
            api.request(
                ApiRequest(
                    Method.POST,
                    route + "/rules/{policy_rule_public_id}/" + action,
                    path={"policy_rule_public_id": rule_id},
                )
            ).status
            == 204
        )
    assert (
        api.request(
            ApiRequest(
                Method.POST,
                item + "/rules/reorder",
                {"ordered_policy_rule_public_ids": [rule_id]},
                path=path,
            )
        ).status
        == 204
    )
    invalid_rule = {**rule, "sort_order": 20, "expires_at": "not-a-date"}
    assert (
        api.request(ApiRequest(Method.POST, item + "/rules", invalid_rule, path=path)).status == 400
    )
