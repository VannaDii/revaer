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
import yaml
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import JsonObject, array_value, object_value, string_value

from tests.support.api_assertions import find_row, rows
from tests.support.media_profiles import native_profile_request

PROFILE = "/v1/media/profiles/{media_profile_public_id}"


@dataclass(frozen=True)
class Lifecycle:
    suffix: str
    source: Path
    body: JsonObject
    profile: JsonObject

    @property
    def path(self) -> dict[str, str]:
        return {"media_profile_public_id": string_value(self.profile["media_profile_public_id"])}

    def input(self, name: str) -> str:
        return f"{self.source.name}/{name}/source.mkv"

    def unchanged(self) -> None:
        for name in ("movie", "watcher", "manual", "schedule"):
            assert (
                self.source / name / "source.mkv"
            ).read_bytes() == f"{name}-{self.suffix}".encode()


def document(api: ApiClient, request: ApiRequest, status: int = 200) -> JsonObject:
    response = api.request(request)
    assert response.status == status, (
        f"{request.method} {request.route}: {response.status} "
        f"{response.object().get('context', [])}"
    )
    return response.object()


@pytest.fixture
def lifecycle(api: ApiClient, fs_root: Path) -> Iterator[Lifecycle]:
    suffix = uuid.uuid4().hex[:8]
    with tempfile.TemporaryDirectory(
        prefix="media-lifecycle-", dir=fs_root / "source"
    ) as directory:
        source = Path(directory)
        for name in ("movie", "watcher", "manual", "schedule"):
            (source / name).mkdir()
            (source / name / "source.mkv").write_bytes(f"{name}-{suffix}".encode())
        body = native_profile_request(api, suffix)
        created = document(
            api,
            ApiRequest(Method.POST, "/v1/media/profiles", body, headers={"If-None-Match": "*"}),
            201,
        )
        fixture = Lifecycle(suffix, source, body, created)
        try:
            yield fixture
        finally:
            fixture.unchanged()


def replace_profile(api: ApiClient, lifecycle: Lifecycle, body: JsonObject) -> JsonObject:
    current = api.request(ApiRequest(Method.GET, PROFILE, path=lifecycle.path))
    assert current.status == 200
    return document(
        api,
        ApiRequest(
            Method.PUT,
            PROFILE,
            body,
            path=lifecycle.path,
            headers={"If-Match": current.headers["etag"]},
        ),
    )


def bind_association(api: ApiClient, lifecycle: Lifecycle, prefix: str = "") -> JsonObject:
    return document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/discovery-associations",
            {
                "association_key": f"lifecycle-{lifecycle.suffix}",
                "media_profile_public_id": lifecycle.profile["media_profile_public_id"],
                "profile_version": lifecycle.profile["latest_version"],
                "source_root_key": "ui-source",
                "root_relative_path": lifecycle.source.name + prefix,
                "manual_enabled": True,
                "schedule_enabled": False,
                "watcher_enabled": False,
            },
            headers={"If-None-Match": "*"},
        ),
        201,
    )


def test_profile_target_policy_validation(api: ApiClient, lifecycle: Lifecycle) -> None:
    profile_id = lifecycle.path["media_profile_public_id"]
    assert find_row(
        rows(api, "/v1/media/profiles", "profiles"), "media_profile_public_id", profile_id
    )
    profile = document(api, ApiRequest(Method.GET, PROFILE, path=lifecycle.path))
    assert profile["output_root_key"] == "ui-source"
    patched = replace_profile(api, lifecycle, {**lifecycle.body, "description": "Updated metadata"})
    assert patched["description"] == "Updated metadata"
    assert patched["latest_version"] == 2
    legacy_validation: JsonObject = {
        "profile_key": f"validated-{lifecycle.suffix}",
        "source_root": str(lifecycle.source),
        "output_root": str(lifecycle.source),
        "dry_run_only": True,
        "retention_days": 30,
        "policy_key": lifecycle.body["policy_key"],
    }
    valid = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/profiles/validate",
            {
                **legacy_validation,
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
    assert target["audio_channels"] == 2
    assert target["audio_channel_layout"] == "stereo"
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
    pinned = replace_profile(
        api,
        lifecycle,
        {
            **lifecycle.body,
            "description": "Updated metadata",
            "desired_target_key": desired_key,
            "desired_target_version": 1,
        },
    )
    assert pinned["desired_target_key"] == desired_key
    assert pinned["desired_target_version"] == 1
    assert pinned["latest_version"] == 3
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
                "output": {
                    "dry_run": True,
                    "replacement_mode": "disabled",
                    "quarantine_enabled": False,
                    "preserve_permissions": True,
                    "preserve_ownership": True,
                },
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
    assert policy["policy_key"] == policy_key
    assert policy["verification_playback_probe"] is True
    invalid: JsonObject = {
        **legacy_validation,
        "profile_key": f"invalid-{lifecycle.suffix}",
        "compatibility_target_key": f"missing-target-{lifecycle.suffix}",
        "policy_key": f"missing-policy-{lifecycle.suffix}",
    }
    validation = document(api, ApiRequest(Method.POST, "/v1/media/profiles/validate", invalid))
    assert validation["valid"] is False
    issues = array_value(validation["issues"])
    assert "media_profile_compatibility_target_not_found" in issues
    assert "media_profile_policy_profile_not_found" in issues
    updated = replace_profile(
        api,
        lifecycle,
        {
            **lifecycle.body,
            "desired_target_key": desired_key,
            "desired_target_version": 1,
            "policy_key": policy_key,
            "policy_version": 1,
        },
    )
    assert updated["policy_key"] == policy_key
    assert updated["latest_version"] == 4
    for field, code in (
        ("desired_target_key", "media_desired_target_not_found"),
        ("policy_key", "media_policy_profile_not_found"),
    ):
        invalid_key = f"invalid-{field.replace('_', '-')}-{lifecycle.suffix}"
        rejected = api.request(
            ApiRequest(
                Method.POST,
                "/v1/media/profiles",
                {
                    **lifecycle.body,
                    "profile_key": invalid_key,
                    field: f"missing-{lifecycle.suffix}",
                },
                headers={"If-None-Match": "*"},
            )
        )
        assert rejected.status == 400
        assert {"name": "error_code", "value": code} in array_value(rejected.object()["context"])
        assert not any(
            row["profile_key"] == invalid_key for row in rows(api, "/v1/media/profiles", "profiles")
        )
    lifecycle.unchanged()


def test_job_retention_settings(api: ApiClient) -> None:
    route = "/v1/media/job-retention"
    original = document(api, ApiRequest(Method.GET, route))
    assert original["completed_enabled"] is False
    assert original["completed_mode"] == "age"
    assert isinstance(original["completed_limit"], int)
    assert original["completed_limit"] > 0
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
    assert profile["binding_ready"] is True
    assert isinstance(profile["destructive_ready"], bool)
    assert profile["active_profile"] == lifecycle.body
    refresh = api.request(ApiRequest(Method.POST, "/v1/media/capabilities/refresh"))
    assert refresh.status in (201, 500, 503)
    compliance = document(api, ApiRequest(Method.GET, "/v1/media/compliance"))
    assert compliance["license_mode"] == "redistributable-gplv3-runtime"


def test_profile_yaml_import_export(api: ApiClient, lifecycle: Lifecycle) -> None:
    exported = document(api, ApiRequest(Method.GET, "/v1/media/export"))
    payload = string_value(exported["yaml_payload"])
    bundle = object_value(yaml.safe_load(payload))
    assert bundle["kind"] == "revaer.media.profile_bundle"
    assert str(lifecycle.source) not in payload
    validated = document(
        api, ApiRequest(Method.POST, "/v1/media/imports/validate", {"yaml_payload": payload})
    )
    assert validated["valid"] is True
    exported_profile = find_row(
        [object_value(row) for row in array_value(bundle["profiles"])],
        "profile_key",
        lifecycle.body["profile_key"],
    )
    assert all(exported_profile[key] == value for key, value in lifecycle.body.items())
    invalid_key = f"yaml-invalid-{lifecycle.suffix}"
    invalid_profile: JsonObject = {
        **lifecycle.body,
        "version": 1,
        "profile_key": invalid_key,
        "desired_target_key": f"missing-target-{lifecycle.suffix}",
        "policy_key": f"missing-policy-{lifecycle.suffix}",
    }
    invalid_bundle: JsonObject = {
        "format_version": 1,
        "kind": "revaer.media.profile_bundle",
        "metadata": {"name": "Invalid catalog references"},
        "profiles": [invalid_profile],
    }
    invalid = yaml.safe_dump(invalid_bundle)
    invalidated = document(
        api, ApiRequest(Method.POST, "/v1/media/imports/validate", {"yaml_payload": invalid})
    )
    assert invalidated["valid"] is False
    issues = [object_value(item) for item in array_value(invalidated["issues"])]
    for code, pointer in (
        ("media_yaml_desired_target_not_found", "/profiles/0/desired_target_key"),
        ("media_yaml_policy_profile_not_found", "/profiles/0/policy_key"),
    ):
        assert any(
            item["code"] == code and item["pointer"] == pointer and item["blocking"] is True
            for item in issues
        )
    rejected = api.request(
        ApiRequest(
            Method.POST,
            "/v1/media/imports/apply",
            {
                "yaml_payload": invalid,
                "preconditions": [{"intent": "create", "kind": "profiles", "key": invalid_key}],
            },
        )
    )
    assert rejected.status == 400
    copy_key = f"yaml-copy-{lifecycle.suffix}"
    copy_profile: JsonObject = {**exported_profile, "profile_key": copy_key, "dry_run_only": False}
    portable_bundle: JsonObject = {
        "format_version": 1,
        "kind": "revaer.media.profile_bundle",
        "metadata": {"name": "Explicit fixture copy"},
        "profiles": [copy_profile],
    }
    portable_payload = yaml.safe_dump(portable_bundle)
    portable = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/imports/apply",
            {
                "yaml_payload": portable_payload,
                "preconditions": [{"intent": "create", "kind": "profiles", "key": copy_key}],
            },
        ),
        201,
    )
    assert portable["forced_dry_run"] is True
    copied = find_row(rows(api, "/v1/media/profiles", "profiles"), "profile_key", copy_key)
    assert all(
        copied[key] == value for key, value in lifecycle.body.items() if key != "profile_key"
    )
    assert copied["dry_run_only"] is True
    assert copied["latest_version"] == 1
    matched = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/imports/apply",
            {
                "yaml_payload": yaml.safe_dump(
                    {**portable_bundle, "profiles": [{**copy_profile, "dry_run_only": True}]}
                ),
                "preconditions": [
                    {
                        "intent": "match",
                        "kind": "profiles",
                        "key": copy_key,
                        "expected_version": copied["latest_version"],
                    }
                ],
            },
        ),
        201,
    )
    assert matched["forced_dry_run"] is True
    assert find_row(rows(api, "/v1/media/profiles", "profiles"), "profile_key", copy_key) == copied
    local = document(
        api, ApiRequest(Method.GET, "/v1/media/export", query={"include_local_paths": True})
    )
    local_payload = string_value(local["yaml_payload"])
    local_bundle = object_value(yaml.safe_load(local_payload))
    assert local_bundle["kind"] == "revaer.media.local_snapshot"
    assert array_value(local_bundle["local_root_paths"])
    # Local snapshots retain diagnostic paths, never portable write authority.
    for route in ("validate", "apply"):
        body: JsonObject = {"yaml_payload": local_payload}
        if route == "apply":
            body["preconditions"] = []
        response = api.request(ApiRequest(Method.POST, f"/v1/media/imports/{route}", body))
        assert response.status == 400
        assert {"name": "error_code", "value": "media_yaml_invalid"} in array_value(
            response.object()["context"]
        )
    restored = replace_profile(
        api, lifecycle, {**lifecycle.body, "description": "After portable round trip"}
    )
    assert restored["output_root_key"] == "ui-source"
    assert restored["workspace_root_key"] == "ui-workspace"
    lifecycle.unchanged()


def discovery(
    api: ApiClient, lifecycle: Lifecycle, association: JsonObject, kind: str, name: str
) -> JsonObject:
    return document(
        api,
        ApiRequest(
            Method.POST,
            f"/v1/media/discovery/{kind}",
            {
                "media_discovery_association_public_id": association[
                    "media_discovery_association_public_id"
                ],
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
def test_manual_discovery_and_job_diagnostics(api: ApiClient, lifecycle: Lifecycle) -> None:
    profile_id = lifecycle.path["media_profile_public_id"]
    association = bind_association(api, lifecycle)
    planning = document(
        api,
        ApiRequest(
            Method.POST,
            "/v1/media/planning/preview",
            {
                "media_discovery_association_public_id": association[
                    "media_discovery_association_public_id"
                ],
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
                "media_discovery_association_public_id": association[
                    "media_discovery_association_public_id"
                ],
                "source_paths": [lifecycle.input("movie")],
            },
        ),
    )
    assert object_value(array_value(preview["previews"])[0])["accepted"] is True
    manual = discovery(api, lifecycle, association, "runs", "manual")
    reasons = ("media_discovery_source_unchanged", "media_discovery_source_unstable")
    queued = one_discovery_outcome(manual, reasons)
    duplicate = discovery(api, lifecycle, association, "runs", "manual")
    if queued:
        assert not array_value(duplicate["queued_jobs"])
        assert object_value(array_value(duplicate["skipped"])[0])["reason"] == reasons[0]
    else:
        one_discovery_outcome(duplicate, reasons)
    jobs = [
        object_value(job)
        for result in (manual, duplicate)
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


@pytest.mark.parametrize("automation", ("schedule", "watcher"))
@pytest.mark.timeout(60)
def test_automated_association_discovery(
    api: ApiClient, lifecycle: Lifecycle, automation: str
) -> None:
    association = bind_association(api, lifecycle, "/manual")
    association_id = string_value(association["media_discovery_association_public_id"])
    collection = automation + "s"
    listed = find_row(
        rows(api, f"/v1/media/discovery/{collection}", collection),
        "media_discovery_association_public_id",
        association_id,
    )
    assert listed[f"{automation}_enabled"] is False
    body: JsonObject = {
        key: association[key]
        for key in (
            "association_key",
            "media_profile_public_id",
            "profile_version",
            "source_root_key",
            "root_relative_path",
            "manual_enabled",
            "schedule_enabled",
            "watcher_enabled",
        )
    }
    body["association_key"] = f"automated-{lifecycle.suffix}"
    body["root_relative_path"] = f"{lifecycle.source.name}/{automation}"
    body[f"{automation}_enabled"] = True
    # Retain positive activation. A held runtime contract is a gate failure,
    # never permission to fabricate attestation or accept a rejection as success.
    enabled = document(
        api,
        ApiRequest(
            Method.POST, "/v1/media/discovery-associations", body, headers={"If-None-Match": "*"}
        ),
        201,
    )
    association_id = string_value(enabled["media_discovery_association_public_id"])
    route = "/v1/media/discovery-associations/{media_discovery_association_public_id}"
    path = {"media_discovery_association_public_id": association_id}
    assert enabled[f"{automation}_enabled"] is True
    listed = find_row(
        rows(api, f"/v1/media/discovery/{collection}", collection),
        "media_discovery_association_public_id",
        association_id,
    )
    assert listed[f"{automation}_enabled"] is True
    if automation == "schedule":
        cadence = document(
            api,
            ApiRequest(
                Method.POST,
                route + "/schedule",
                {
                    "association_version": enabled["latest_version"],
                    "interval_quantity": 120,
                    "interval_unit": "minutes",
                },
                path=path,
                headers={"If-None-Match": "*"},
            ),
            201,
        )
        assert cadence["interval_quantity"] == 120
        assert cadence["interval_unit"] == "minutes"
    result = discovery(api, lifecycle, enabled, collection, automation)
    one_discovery_outcome(result, ("media_discovery_source_unchanged",))
    lifecycle.unchanged()
