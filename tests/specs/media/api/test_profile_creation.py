"""Profile persistence and dry-run admission preserve sources and automation guards."""

import tempfile
import uuid
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import JsonObject, array_value, object_value, string_value

PROFILE = "/v1/media/profiles/{media_profile_public_id}"
CREATE_HEADERS = {"If-None-Match": "*"}


@dataclass(frozen=True)
class ProfileFixture:
    source: Path
    content: bytes
    request: JsonObject

    def unchanged(self) -> None:
        assert self.source.read_bytes() == self.content


@pytest.fixture
def profile(tmp_path: Path) -> Iterator[ProfileFixture]:
    # pytest retains its tmp_path on failure; own a nested context so media
    # source files are removed immediately after every outcome.
    with tempfile.TemporaryDirectory(prefix="media-profile-", dir=tmp_path) as directory:
        source = Path(directory) / "source"
        output = Path(directory) / "output"
        source.mkdir()
        output.mkdir()
        path = source / "source.mkv"
        content = b"profile-create source must remain unchanged"
        path.write_bytes(content)
        yield ProfileFixture(
            path,
            content,
            {
                "profile_key": f"profile-create-{uuid.uuid4()}",
                "source_root": str(source),
                "output_root": str(output),
                "dry_run_only": True,
                "retention_days": 30,
                "schedule_enabled": False,
                "watcher_enabled": False,
            },
        )


def create(api: ApiClient, profile: ProfileFixture) -> JsonObject:
    response = api.request(
        ApiRequest(Method.POST, "/v1/media/profiles", profile.request, headers=CREATE_HEADERS)
    )
    assert response.status == 201
    return response.object()


def path_for(profile: JsonObject) -> dict[str, str]:
    return {"media_profile_public_id": string_value(profile["media_profile_public_id"])}


def read(api: ApiClient, profile: JsonObject) -> JsonObject:
    response = api.request(ApiRequest(Method.GET, PROFILE, path=path_for(profile)))
    assert response.status == 200
    return response.object()


def test_creation_forces_dry_run_and_duplicate_preserves_state(
    api: ApiClient,
    profile: ProfileFixture,
) -> None:
    response = api.request(
        ApiRequest(
            Method.POST,
            "/v1/media/profiles",
            {**profile.request, "dry_run_only": False},
            headers=CREATE_HEADERS,
        )
    )
    assert response.status == 201
    created = response.object()
    assert all(created[key] == value for key, value in profile.request.items())
    assert created["policy_key"] == "safe_dry_run"
    assert created.get("schedule_interval_minutes") is None
    assert read(api, created) == created
    duplicate = api.request(
        ApiRequest(
            Method.POST,
            "/v1/media/profiles",
            {**profile.request, "retention_days": 31},
            headers=CREATE_HEADERS,
        )
    )
    assert duplicate.status == 409
    assert {"name": "error_code", "value": "media_profile_key_conflict"} in array_value(
        duplicate.object()["context"]
    )
    assert read(api, created) == created
    profile.unchanged()


@pytest.mark.parametrize("automation", ("schedule", "watcher"))
def test_creation_requires_verified_identity_for_automation(
    api: ApiClient,
    profile: ProfileFixture,
    automation: str,
) -> None:
    body: JsonObject = {**profile.request, f"{automation}_enabled": True}
    if automation == "schedule":
        body["schedule_interval_minutes"] = 60
    response = api.request(
        ApiRequest(Method.POST, "/v1/media/profiles", body, headers=CREATE_HEADERS)
    )
    assert response.status == 400
    assert response.object()["context"] == [
        {"name": "operation", "value": "media_profile_upsert"},
        {"name": "error_code", "value": "media_profile_filesystem_identity_required"},
        {"name": "sqlstate", "value": "P0001"},
    ]
    listed = api.request(ApiRequest(Method.GET, "/v1/media/profiles"))
    assert listed.status == 200
    assert not any(
        object_value(item)["profile_key"] == profile.request["profile_key"]
        for item in array_value(listed.object()["profiles"])
    )
    profile.unchanged()


def test_metadata_update_preserves_roots_and_disabled_automation(
    api: ApiClient,
    profile: ProfileFixture,
) -> None:
    created = create(api, profile)
    body = {key: value for key, value in profile.request.items() if key != "profile_key"}
    body["retention_days"] = 31
    response = api.request(ApiRequest(Method.PATCH, PROFILE, body, path=path_for(created)))
    assert response.status == 200
    expected = {**profile.request, "retention_days": 31}
    assert all(response.object()[key] == value for key, value in expected.items())
    assert response.object().get("schedule_interval_minutes") is None
    assert read(api, created) == response.object()
    profile.unchanged()


@pytest.mark.parametrize(
    "update",
    (
        {"schedule_enabled": True, "schedule_interval_minutes": 60},
        {"schedule_interval_minutes": 120},
        {"schedule_enabled": False, "schedule_interval_minutes": 120},
        {"watcher_enabled": True},
    ),
    ids=("schedule", "interval-alone", "disabled-interval", "watcher"),
)
def test_unverified_updates_preserve_existing_profile(
    api: ApiClient,
    profile: ProfileFixture,
    update: JsonObject,
) -> None:
    created = create(api, profile)
    response = api.request(
        ApiRequest(Method.PATCH, PROFILE, {**update, "retention_days": 31}, path=path_for(created))
    )
    assert response.status == 400
    assert response.object()["context"] == [
        {"name": "operation", "value": "media_profile_patch"},
        {"name": "error_code", "value": "media_profile_filesystem_identity_required"},
        {"name": "sqlstate", "value": "P0001"},
    ]
    assert read(api, created) == created
    profile.unchanged()


def test_manual_discovery_and_diagnostics_preserve_dry_run(
    api: ApiClient,
    profile: ProfileFixture,
) -> None:
    created = create(api, profile)
    path = path_for(created)
    readiness = api.request(ApiRequest(Method.GET, PROFILE + "/readiness", path=path))
    assert readiness.status == 200
    assert (
        object_value(readiness.object()["profile"])["media_profile_public_id"]
        == path["media_profile_public_id"]
    )
    assert isinstance(readiness.object()["ready"], bool)
    planning = api.request(
        ApiRequest(
            Method.POST, "/v1/media/planning/preview", {**path, "source_path": str(profile.source)}
        )
    )
    assert planning.status == 200 and planning.object()["accepted"] is True
    body: JsonObject = {**path, "source_paths": [str(profile.source)]}
    preview = api.request(ApiRequest(Method.POST, "/v1/media/discovery/preview", body))
    assert preview.status == 200
    assert object_value(array_value(preview.object()["previews"])[0])["accepted"] is True
    run = api.request(ApiRequest(Method.POST, "/v1/media/discovery/runs", body))
    assert run.status == 201 and run.object()["skipped"] == []
    queued = array_value(run.object()["queued_jobs"])
    assert len(queued) == 1
    job = object_value(queued[0])
    assert job["source_path"] == str(profile.source) and job["dry_run"] is True
    job_id = string_value(job["media_job_public_id"])
    duplicate = api.request(ApiRequest(Method.POST, "/v1/media/discovery/runs", body))
    assert duplicate.status == 201 and duplicate.object()["queued_jobs"] == []
    assert duplicate.object()["skipped"] == [
        {"source_path": str(profile.source), "reason": "media_discovery_source_unchanged"}
    ]
    jobs = api.request(ApiRequest(Method.GET, "/v1/media/jobs", query=path))
    assert jobs.status == 200
    assert [
        object_value(row)["media_job_public_id"] for row in array_value(jobs.object()["jobs"])
    ] == [job_id]
    route = "/v1/media/jobs/{media_job_public_id}"
    job_path = {"media_job_public_id": job_id}
    detail = api.request(ApiRequest(Method.GET, route, path=job_path))
    assert detail.status == 200 and detail.object()["source_path"] == str(profile.source)
    for suffix, collection in (
        ("phases", "phases"),
        ("operations", "operations"),
        ("violations", "violations"),
        ("plan-reasons", "reasons"),
        ("verification-checks", "checks"),
        ("artifacts", "artifacts"),
        ("compact-audits", "audits"),
    ):
        response = api.request(ApiRequest(Method.GET, route + "/" + suffix, path=job_path))
        assert response.status == 200
        array_value(response.object()[collection])
    for action in ("cancel", "retry"):
        assert api.request(ApiRequest(Method.POST, route + "/" + action, path=job_path)).status in (
            204,
            409,
        )
    assert read(api, created) == created
    profile.unchanged()


@pytest.mark.parametrize("automation", ("schedules", "watchers"))
def test_disabled_automation_admits_no_jobs(
    api: ApiClient,
    profile: ProfileFixture,
    automation: str,
) -> None:
    created = create(api, profile)
    path = path_for(created)
    response = api.request(
        ApiRequest(
            Method.POST,
            f"/v1/media/discovery/{automation}",
            {**path, "source_paths": [str(profile.source)]},
        )
    )
    assert response.status == 400
    assert {
        "name": "error_code",
        "value": f"media_discovery_{automation[:-1]}_disabled",
    } in array_value(response.object()["context"])
    jobs = api.request(ApiRequest(Method.GET, "/v1/media/jobs", query=path))
    assert jobs.status == 200 and jobs.object()["jobs"] == []
    profile.unchanged()
