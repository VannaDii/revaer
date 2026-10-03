"""Bounded media route availability and catalog write preservation."""

import re
import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.external.http import HttpRequest
from revaer_tooling.json_data import JsonObject

from tests.support.api_assertions import rows

MISSING = "00000000-0000-0000-0000-000000000001"
OPERATIONS = (
    (Method.GET, "/v1/media/capabilities"),
    (Method.GET, "/v1/media/capabilities/readiness"),
    (Method.GET, "/v1/media/compatibility-targets"),
    (Method.GET, "/v1/media/compliance"),
    (Method.GET, "/v1/media/discovery/schedules"),
    (Method.GET, "/v1/media/discovery/watchers"),
    (Method.GET, "/v1/media/export"),
    (Method.GET, "/v1/media/job-retention"),
    (Method.GET, "/v1/media/jobs"),
    (Method.GET, "/v1/media/jobs/recent"),
    (Method.GET, "/v1/media/jobs/{media_job_public_id}"),
    (Method.GET, "/v1/media/jobs/{media_job_public_id}/artifacts"),
    (Method.GET, "/v1/media/jobs/{media_job_public_id}/compact-audits"),
    (Method.GET, "/v1/media/jobs/{media_job_public_id}/diagnostics"),
    (Method.GET, "/v1/media/jobs/{media_job_public_id}/operations"),
    (Method.GET, "/v1/media/jobs/{media_job_public_id}/plan-reasons"),
    (Method.GET, "/v1/media/jobs/{media_job_public_id}/verification-checks"),
    (Method.GET, "/v1/media/jobs/{media_job_public_id}/violations"),
    (Method.GET, "/v1/media/policies"),
    (Method.GET, "/v1/media/profiles"),
    (Method.GET, "/v1/media/profiles/{media_profile_public_id}"),
    (Method.GET, "/v1/media/targets"),
    (Method.PATCH, "/v1/media/job-retention"),
    (Method.PATCH, "/v1/media/profiles/{media_profile_public_id}"),
    (Method.PATCH, "/v1/media/profiles/{media_profile_public_id}/desired-target"),
    (Method.POST, "/v1/media/capabilities/refresh"),
    (Method.POST, "/v1/media/compatibility-targets"),
    (Method.POST, "/v1/media/discovery/preview"),
    (Method.POST, "/v1/media/discovery/runs"),
    (Method.POST, "/v1/media/discovery/schedules"),
    (Method.POST, "/v1/media/discovery/watchers"),
    (Method.POST, "/v1/media/imports/apply"),
    (Method.POST, "/v1/media/imports/validate"),
    (Method.POST, "/v1/media/jobs/{media_job_public_id}/cancel"),
    (Method.POST, "/v1/media/jobs/{media_job_public_id}/retry"),
    (Method.POST, "/v1/media/planning/preview"),
    (Method.POST, "/v1/media/policies"),
    (Method.POST, "/v1/media/profiles"),
    (Method.POST, "/v1/media/profiles/validate"),
    (Method.POST, "/v1/media/targets"),
)


def test_compatibility_target_rejected_update_preserves_persisted_value(api: ApiClient) -> None:
    target: JsonObject = {
        "compatibility_target_key": f"target-write-{uuid.uuid4()}",
        "version": 1,
        "display_name": "Original target",
        "video_codec": "hevc",
        "audio_codec": "aac",
        "audio_channels": 2,
        "audio_channel_layout": "stereo",
        "subtitle_policy": "selected",
    }
    route = "/v1/media/compatibility-targets"
    created = api.request(ApiRequest(Method.POST, route, target))
    assert created.status == 201
    assert all(created.object()[key] == value for key, value in target.items())
    updated_target = {**target, "display_name": "Updated target"}
    updated = api.request(ApiRequest(Method.POST, route, updated_target))
    assert updated.status == 201
    assert all(updated.object()[key] == value for key, value in updated_target.items())
    rejected = api.request(ApiRequest(Method.POST, route, {**target, "display_name": ""}))
    assert rejected.status == 400
    persisted = rows(api, route, "targets")
    assert [
        entry
        for entry in persisted
        if entry["compatibility_target_key"] == target["compatibility_target_key"]
    ] == [updated.object()]


def raw_status(api: ApiClient, method: Method, route: str) -> int:
    """Preserve the transport probe, including intentionally undocumented 405s.

    Typed scenario requests still validate full OpenAPI response schemas. Empty
    probes here ask only whether the router exists and rejects input without 5xx.
    """
    request = ApiRequest(
        method, route, path=dict.fromkeys(re.findall(r"\{([^{}]+)\}", route), MISSING)
    )
    api.coverage.record(f"{method} {route}")
    return api.http.request(HttpRequest(method, request.target(api.base_url), api.headers)).status


def test_bounded_empty_requests_do_not_produce_server_errors(api: ApiClient) -> None:
    for method, route in OPERATIONS:
        status = raw_status(api, method, route)
        assert status < 500, f"{method} {route} returned {status}"
        assert status != 405, f"{method} {route} is not wired"


def test_worker_owned_media_writes_remain_unavailable(api: ApiClient) -> None:
    for route in ("/v1/media/jobs", "/v1/media/jobs/{media_job_public_id}/phases"):
        assert raw_status(api, Method.POST, route) == 405
