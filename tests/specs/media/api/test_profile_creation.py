"""Profile persistence and dry-run admission preserve sources and automation guards."""

import tempfile
import uuid
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import JsonObject, array_value, object_value, string_value

from tests.support.media_profiles import native_profile_request

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
def profile(api: ApiClient, fs_root: Path) -> Iterator[ProfileFixture]:
    suffix = uuid.uuid4().hex
    request = native_profile_request(api, suffix)
    # Own only files within the real catalog's source slot. The fixture never
    # creates catalog authority or manufactures runtime attestation.
    with tempfile.TemporaryDirectory(prefix="media-profile-", dir=fs_root / "source") as directory:
        path = Path(directory) / "source.mkv"
        content = b"profile-create source must remain unchanged"
        path.write_bytes(content)
        yield ProfileFixture(
            path,
            content,
            request,
        )


def association(api: ApiClient, profile: ProfileFixture, created: JsonObject) -> JsonObject:
    response = api.request(
        ApiRequest(
            Method.POST,
            "/v1/media/discovery-associations",
            {
                "association_key": f"association-{uuid.uuid4().hex}",
                "media_profile_public_id": created["media_profile_public_id"],
                "profile_version": created["latest_version"],
                "source_root_key": "ui-source",
                "root_relative_path": profile.source.parent.name,
                "manual_enabled": True,
                "watcher_enabled": False,
                "schedule_enabled": False,
            },
            headers=CREATE_HEADERS,
        )
    )
    assert response.status == 201, response.object()
    assert response.object()["binding_ready"] is True
    return response.object()


def discovery_body(association: JsonObject, profile: ProfileFixture) -> JsonObject:
    return {
        "media_discovery_association_public_id": association[
            "media_discovery_association_public_id"
        ],
        "source_paths": [f"{profile.source.parent.name}/{profile.source.name}"],
    }


def replace_headers(api: ApiClient, created: JsonObject) -> dict[str, str]:
    response = api.request(ApiRequest(Method.GET, PROFILE, path=path_for(created)))
    assert response.status == 200
    return {"If-Match": response.headers["etag"]}


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


def test_policy_forces_dry_run_and_duplicate_preserves_state(
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
    assert all(
        created[key] == value for key, value in profile.request.items() if key != "dry_run_only"
    )
    assert created["dry_run_only"] is False
    bound = association(api, profile, created)
    preview = api.request(
        ApiRequest(Method.POST, "/v1/media/discovery/preview", discovery_body(bound, profile))
    )
    assert preview.status == 200
    assert object_value(array_value(preview.object()["previews"])[0])["dry_run"] is True
    assert read(api, created) == created
    duplicate = api.request(
        ApiRequest(
            Method.POST,
            "/v1/media/profiles",
            {**profile.request, "description": "Duplicate must preserve state"},
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
def test_retired_profile_automation_body_is_rejected_without_writes(
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
        {"name": "error_code", "value": "media_configuration_invalid"},
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
    expected: JsonObject = {**profile.request, "description": "Updated metadata"}
    response = api.request(
        ApiRequest(
            Method.PUT,
            PROFILE,
            expected,
            path=path_for(created),
            headers=replace_headers(api, created),
        )
    )
    assert response.status == 200
    assert all(response.object()[key] == value for key, value in expected.items())
    assert response.object()["latest_version"] == 2
    assert read(api, created) == response.object()
    bound = association(api, profile, response.object())
    assert bound["schedule_enabled"] is False and bound["watcher_enabled"] is False
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
        ApiRequest(
            Method.PUT,
            PROFILE,
            {**profile.request, **update},
            path=path_for(created),
            headers=replace_headers(api, created),
        )
    )
    assert response.status == 400
    assert response.object()["context"] == [
        {"name": "error_code", "value": "media_configuration_invalid"},
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
    assert readiness.object()["binding_ready"] is True
    bound = association(api, profile, created)
    body = discovery_body(bound, profile)
    relative_source = string_value(array_value(body["source_paths"])[0])
    planning = api.request(
        ApiRequest(
            Method.POST,
            "/v1/media/planning/preview",
            {
                "media_discovery_association_public_id": bound[
                    "media_discovery_association_public_id"
                ],
                "source_path": relative_source,
            },
        )
    )
    assert planning.status == 200 and planning.object()["accepted"] is True
    preview = api.request(ApiRequest(Method.POST, "/v1/media/discovery/preview", body))
    assert preview.status == 200
    assert object_value(array_value(preview.object()["previews"])[0])["accepted"] is True
    run = api.request(ApiRequest(Method.POST, "/v1/media/discovery/runs", body))
    assert run.status == 201 and run.object()["skipped"] == []
    queued = array_value(run.object()["queued_jobs"])
    assert len(queued) == 1
    job = object_value(queued[0])
    assert job["source_path"] == relative_source and job["dry_run"] is True
    job_id = string_value(job["media_job_public_id"])
    duplicate = api.request(ApiRequest(Method.POST, "/v1/media/discovery/runs", body))
    assert duplicate.status == 201 and duplicate.object()["queued_jobs"] == []
    assert duplicate.object()["skipped"] == [
        {"source_path": relative_source, "reason": "media_discovery_source_unchanged"}
    ]
    jobs = api.request(ApiRequest(Method.GET, "/v1/media/jobs", query=path))
    assert jobs.status == 200
    assert [
        object_value(row)["media_job_public_id"] for row in array_value(jobs.object()["jobs"])
    ] == [job_id]
    route = "/v1/media/jobs/{media_job_public_id}"
    job_path = {"media_job_public_id": job_id}
    detail = api.request(ApiRequest(Method.GET, route, path=job_path))
    assert detail.status == 200 and detail.object()["source_path"] == relative_source
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
    bound = association(api, profile, created)
    response = api.request(
        ApiRequest(Method.POST, f"/v1/media/discovery/{automation}", discovery_body(bound, profile))
    )
    assert response.status == 400
    assert {
        "name": "error_code",
        "value": f"media_discovery_{automation[:-1]}_disabled",
    } in array_value(response.object()["context"])
    jobs = api.request(ApiRequest(Method.GET, "/v1/media/jobs", query=path))
    assert jobs.status == 200 and jobs.object()["jobs"] == []
    profile.unchanged()
