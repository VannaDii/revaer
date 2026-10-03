"""Media lifecycle coverage preserved from the combined TypeScript scenario.

Independent subsystems get separate scenarios so one regression cannot conceal
other results. Every case owns real source files and removes them on failure.
The automation case retains the original positive admission requirements; a
missing capability is a failure, never permission to manufacture attestation.
"""

import tempfile
import uuid
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import JsonObject, array_value, object_value, string_value

from tests.support.api_assertions import find_row, rows

PROFILE = "/v1/media/profiles/{media_profile_public_id}"


@dataclass(frozen=True)
class Lifecycle:
    suffix: str
    source: Path
    output: Path
    body: JsonObject
    profile: JsonObject

    @property
    def path(self) -> dict[str, str]:
        return {"media_profile_public_id": string_value(self.profile["media_profile_public_id"])}

    def input(self, name: str) -> str:
        return str(self.source / f"{name}.mkv")


def document(api: ApiClient, request: ApiRequest, status: int = 200) -> JsonObject:
    response = api.request(request)
    assert response.status == status, f"{request.method} {request.route}: {response.status}"
    return response.object()


@pytest.fixture
def lifecycle(api: ApiClient, tmp_path: Path) -> Iterator[Lifecycle]:
    suffix = uuid.uuid4().hex[:8]
    with tempfile.TemporaryDirectory(prefix="media-lifecycle-", dir=tmp_path) as directory:
        root = Path(directory)
        source, output = root / "source", root / "output"
        source.mkdir()
        output.mkdir()
        for name in ("movie", "watcher", "manual", "schedule"):
            (source / f"{name}.mkv").write_bytes(f"{name}-{suffix}".encode())
        body: JsonObject = {
            "profile_key": f"e2e-media-{suffix}",
            "source_root": str(source),
            "output_root": str(output),
            "dry_run_only": True,
            "retention_days": 30,
            "schedule_enabled": False,
            "watcher_enabled": False,
        }
        created = document(
            api,
            ApiRequest(Method.POST, "/v1/media/profiles", body, headers={"If-None-Match": "*"}),
            201,
        )
        yield Lifecycle(suffix, source, output, body, created)


def test_profile_target_policy_validation(api: ApiClient, lifecycle: Lifecycle) -> None:
    profile_id = lifecycle.path["media_profile_public_id"]
    assert find_row(
        rows(api, "/v1/media/profiles", "profiles"), "media_profile_public_id", profile_id
    )
    profile = document(api, ApiRequest(Method.GET, PROFILE, path=lifecycle.path))
    assert profile["source_root"] == str(lifecycle.source)
    patched = document(
        api, ApiRequest(Method.PATCH, PROFILE, {"retention_days": 31}, path=lifecycle.path)
    )
    assert patched["retention_days"] == 31
    valid = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/profiles/validate",
            {
                **lifecycle.body,
                "profile_key": f"validated-{lifecycle.suffix}",
            },
        ),
    )
    assert valid["valid"] is True
    assert find_row(
        rows(api, "/v1/media/compatibility-targets", "targets"),
        "compatibility_target_key",
        "hevc-aac",
    )
    target_key = f"e2e-target-{lifecycle.suffix}"
    target = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/compatibility-targets",
            {
                "compatibility_target_key": target_key,
                "version": 1,
                "display_name": f"E2E target {lifecycle.suffix}",
                "video_codec": "hevc",
                "audio_codec": "aac",
                "audio_channels": 2,
                "audio_channel_layout": "stereo",
                "subtitle_policy": "selected",
            },
        ),
        201,
    )
    assert target["compatibility_target_key"] == target_key
    assert target["audio_channels"] == 2 and target["audio_channel_layout"] == "stereo"
    desired_key = f"e2e-desired-target-{lifecycle.suffix}"
    desired = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/targets",
            {
                "target_key": desired_key,
                "version": 1,
                "display_name": "E2E desired target",
                "container_format": "matroska",
                "streams": [
                    {
                        "stream_key": "video-main",
                        "stream_kind": "video",
                        "optional": False,
                        "sort_order": 0,
                        "codec": "hevc",
                        "default_disposition": True,
                        "forced_disposition": False,
                    },
                    {
                        "stream_key": "audio-main",
                        "stream_kind": "audio",
                        "semantic_role": "primary",
                        "language_code": "eng",
                        "optional": False,
                        "sort_order": 1,
                        "codec": "aac",
                        "channel_count": 2,
                        "channel_layout": "stereo",
                        "default_disposition": True,
                        "forced_disposition": False,
                    },
                    {
                        "stream_key": "subtitle-full",
                        "stream_kind": "subtitle",
                        "semantic_role": "primary",
                        "language_code": "eng",
                        "optional": True,
                        "sort_order": 2,
                        "codec": "subrip",
                        "default_disposition": False,
                        "forced_disposition": False,
                    },
                ],
            },
        ),
        201,
    )
    assert desired["target_key"] == desired_key
    assert [object_value(item)["stream_key"] for item in array_value(desired["streams"])] == [
        "video-main",
        "audio-main",
        "subtitle-full",
    ]
    assert find_row(rows(api, "/v1/media/targets", "targets"), "target_key", desired_key)
    assert (
        api.request(
            ApiRequest(
                Method.PATCH,
                PROFILE + "/desired-target",
                {"target_key": desired_key, "version": 1},
                path=lifecycle.path,
            )
        ).status
        == 204
    )
    pinned = document(api, ApiRequest(Method.GET, PROFILE, path=lifecycle.path))
    assert pinned["desired_target_key"] == desired_key and pinned["desired_target_version"] == 1
    assert find_row(rows(api, "/v1/media/policies", "policies"), "policy_key", "safe_dry_run")
    policy_key = f"e2e-policy-{lifecycle.suffix}"
    policy = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/policies",
            {
                "policy_key": policy_key,
                "version": 1,
                "display_name": "E2E policy",
                "video_intent": "general",
                "verification_strictness": "strict",
                "verification_duration_tolerance_millis": 100,
                "verification_mux_validation": True,
                "verification_decode_all_streams": True,
                "verification_keyframe_seek": True,
                "verification_playback_probe": True,
            },
        ),
        201,
    )
    assert policy["policy_key"] == policy_key and policy["verification_playback_probe"] is True
    invalid: JsonObject = {
        **lifecycle.body,
        "profile_key": f"invalid-{lifecycle.suffix}",
        "source_root": str(lifecycle.source / "invalid-validation"),
        "output_root": str(lifecycle.output / "invalid-validation"),
        "compatibility_target_key": f"missing-target-{lifecycle.suffix}",
        "policy_key": f"missing-policy-{lifecycle.suffix}",
    }
    validation = document(api, ApiRequest(Method.POST, "/v1/media/profiles/validate", invalid))
    assert validation["valid"] is False
    issues = array_value(validation["issues"])
    assert "media_profile_compatibility_target_not_found" in issues
    assert "media_profile_policy_profile_not_found" in issues
    rejected = api.request(
        ApiRequest(
            Method.POST,
            "/v1/media/profiles",
            {
                **invalid,
                "profile_key": f"invalid-create-{lifecycle.suffix}",
                "source_root": str(lifecycle.source / "invalid-create"),
                "output_root": str(lifecycle.output / "invalid-create"),
            },
            headers={"If-None-Match": "*"},
        )
    )
    assert rejected.status == 400


def test_job_retention_settings(api: ApiClient) -> None:
    route = "/v1/media/job-retention"
    original = document(api, ApiRequest(Method.GET, route))
    assert original["completed_enabled"] is False
    assert original["completed_mode"] == "age"
    assert isinstance(original["completed_limit"], int) and original["completed_limit"] > 0
    assert original["failed_diagnostic_enabled"] is True
    settings: JsonObject = {
        "completed_enabled": True,
        "completed_mode": "count",
        "completed_limit": 32,
        "failed_diagnostic_enabled": True,
        "failed_diagnostic_mode": "age",
        "failed_diagnostic_limit": 33,
    }
    try:
        updated = document(api, ApiRequest(Method.PATCH, route, settings))
        assert all(updated[key] == value for key, value in settings.items())
    finally:
        # Both auth phases share the server; restore the fetched state even if
        # the write assertion fails, so the next phase sees the same baseline.
        restored = document(
            api, ApiRequest(Method.PATCH, route, {key: original[key] for key in settings})
        )
        assert all(restored[key] == original[key] for key in settings)


def test_capabilities_readiness_and_compliance(api: ApiClient, lifecycle: Lifecycle) -> None:
    latest = document(api, ApiRequest(Method.GET, "/v1/media/capabilities"))
    snapshot = latest.get("snapshot")
    if snapshot is not None:
        evidence = object_value(snapshot)
        array_value(evidence["features"])
        array_value(evidence["muxers"])
        assert string_value(evidence["license_mode"])
    ready = document(api, ApiRequest(Method.GET, "/v1/media/capabilities/readiness"))
    assert isinstance(ready["ready"], bool)
    profile = document(api, ApiRequest(Method.GET, PROFILE + "/readiness", path=lifecycle.path))
    assert (
        object_value(profile["profile"])["media_profile_public_id"]
        == lifecycle.path["media_profile_public_id"]
    )
    assert isinstance(profile["ready"], bool)
    refresh = api.request(ApiRequest(Method.POST, "/v1/media/capabilities/refresh"))
    assert refresh.status in (201, 500, 503)
    compliance = document(api, ApiRequest(Method.GET, "/v1/media/compliance"))
    assert compliance["license_mode"] == "redistributable-gplv3-runtime"


def test_profile_yaml_import_export(api: ApiClient, lifecycle: Lifecycle) -> None:
    exported = document(api, ApiRequest(Method.GET, "/v1/media/export"))
    yaml = string_value(exported["yaml_payload"])
    assert yaml
    validated = document(
        api, ApiRequest(Method.POST, "/v1/media/imports/validate", {"yaml_payload": yaml})
    )
    assert validated["valid"] is True
    invalid = "\n".join(
        (
            "format_version: 1",
            "kind: revaer.media.profile_bundle",
            "metadata:",
            "  name: Invalid catalog references",
            "profiles:",
            f"  - profile_key: e2e-yaml-invalid-{lifecycle.suffix}",
            f"    source_root: {lifecycle.source}/yaml-invalid",
            f"    output_root: {lifecycle.output}/yaml-invalid",
            "    dry_run_only: true",
            "    retention_days: 30",
            f"    compatibility_target_key: missing-target-{lifecycle.suffix}",
            f"    policy_key: missing-policy-{lifecycle.suffix}",
        )
    )
    invalidated = document(
        api, ApiRequest(Method.POST, "/v1/media/imports/validate", {"yaml_payload": invalid})
    )
    assert invalidated["valid"] is False
    issues = [object_value(item) for item in array_value(invalidated["issues"])]
    for code, pointer in (
        ("media_yaml_compatibility_target_not_found", "/profiles/0/compatibility_target_key"),
        ("media_yaml_policy_profile_not_found", "/profiles/0/policy_key"),
    ):
        assert any(
            item["code"] == code and item["pointer"] == pointer and item["blocking"] is True
            for item in issues
        )
    assert (
        api.request(
            ApiRequest(Method.POST, "/v1/media/imports/apply", {"yaml_payload": invalid})
        ).status
        == 400
    )
    portable = document(
        api, ApiRequest(Method.POST, "/v1/media/imports/apply", {"yaml_payload": yaml}), 201
    )
    assert portable["forced_dry_run"] is True
    local = document(
        api, ApiRequest(Method.GET, "/v1/media/export", query={"include_local_paths": True})
    )
    local_yaml = string_value(local["yaml_payload"])
    assert local_yaml
    applied = document(
        api, ApiRequest(Method.POST, "/v1/media/imports/apply", {"yaml_payload": local_yaml}), 201
    )
    assert applied["forced_dry_run"] is True
    restored = document(
        api,
        ApiRequest(
            Method.PATCH, PROFILE, {**lifecycle.body, "retention_days": 31}, path=lifecycle.path
        ),
    )
    assert restored["source_root"] == str(lifecycle.source)
    assert restored["output_root"] == str(lifecycle.output)


def discovery(api: ApiClient, lifecycle: Lifecycle, kind: str, name: str) -> JsonObject:
    return document(
        api,
        ApiRequest(
            Method.POST,
            f"/v1/media/discovery/{kind}",
            {
                "media_profile_public_id": lifecycle.path["media_profile_public_id"],
                "source_paths": [lifecycle.input(name)],
            },
        ),
        201,
    )


def one_discovery_outcome(result: JsonObject, reasons: tuple[str, ...]) -> bool:
    queued, skipped = array_value(result["queued_jobs"]), array_value(result["skipped"])
    assert len(queued) + len(skipped) == 1
    if not queued:
        assert object_value(skipped[0])["reason"] in reasons
    return len(queued) == 1


@pytest.mark.timeout(60)
def test_automated_discovery_and_job_diagnostics(api: ApiClient, lifecycle: Lifecycle) -> None:
    profile_id = lifecycle.path["media_profile_public_id"]
    schedule = find_row(
        rows(api, "/v1/media/discovery/schedules", "schedules"),
        "media_profile_public_id",
        profile_id,
    )
    assert schedule["enabled"] is False
    scheduled = document(
        api,
        ApiRequest(
            Method.PATCH,
            PROFILE,
            {"schedule_enabled": True, "schedule_interval_minutes": 120},
            path=lifecycle.path,
        ),
    )
    assert scheduled["schedule_enabled"] is True
    watcher = find_row(
        rows(api, "/v1/media/discovery/watchers", "watchers"), "media_profile_public_id", profile_id
    )
    assert watcher["enabled"] is False
    watched = document(
        api, ApiRequest(Method.PATCH, PROFILE, {"watcher_enabled": True}, path=lifecycle.path)
    )
    assert watched["watcher_enabled"] is True
    enabled = find_row(
        rows(api, "/v1/media/discovery/watchers", "watchers"), "media_profile_public_id", profile_id
    )
    assert enabled["enabled"] is True
    watcher_run = discovery(api, lifecycle, "watchers", "watcher")
    one_discovery_outcome(watcher_run, ("media_discovery_source_unchanged",))
    planning = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/planning/preview",
            {
                "media_profile_public_id": profile_id,
                "source_path": lifecycle.input("movie"),
            },
        ),
    )
    assert planning["accepted"] is True
    preview = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/discovery/preview",
            {
                "media_profile_public_id": profile_id,
                "source_paths": [lifecycle.input("movie")],
            },
        ),
    )
    assert object_value(array_value(preview["previews"])[0])["accepted"] is True
    manual = discovery(api, lifecycle, "runs", "manual")
    reasons = ("media_discovery_source_unchanged", "media_discovery_source_unstable")
    queued = one_discovery_outcome(manual, reasons)
    duplicate = discovery(api, lifecycle, "runs", "manual")
    if queued:
        assert not array_value(duplicate["queued_jobs"])
        assert object_value(array_value(duplicate["skipped"])[0])["reason"] == reasons[0]
    else:
        one_discovery_outcome(duplicate, reasons)
    schedule_run = discovery(api, lifecycle, "schedules", "schedule")
    jobs = [
        object_value(job)
        for result in (manual, schedule_run, watcher_run)
        for job in array_value(result["queued_jobs"])
    ]
    assert jobs, "Missing media job public id"
    job_id = string_value(jobs[0]["media_job_public_id"])
    listed = document(
        api, ApiRequest(Method.GET, "/v1/media/jobs", query={"media_profile_public_id": profile_id})
    )
    assert job_id in [
        object_value(job)["media_job_public_id"] for job in array_value(listed["jobs"])
    ]
    route, path = "/v1/media/jobs/{media_job_public_id}", {"media_job_public_id": job_id}
    job = document(api, ApiRequest(Method.GET, route, path=path))
    assert job["source_path"] in [
        lifecycle.input(name) for name in ("manual", "schedule", "watcher")
    ]
    for endpoint, collection in (
        ("phases", "phases"),
        ("operations", "operations"),
        ("violations", "violations"),
        ("plan-reasons", "reasons"),
        ("verification-checks", "checks"),
        ("artifacts", "artifacts"),
        ("compact-audits", "audits"),
    ):
        diagnostics = document(api, ApiRequest(Method.GET, route + "/" + endpoint, path=path))
        array_value(diagnostics[collection])
    for action in ("cancel", "retry"):
        assert api.request(ApiRequest(Method.POST, route + "/" + action, path=path)).status in (
            204,
            409,
        )
