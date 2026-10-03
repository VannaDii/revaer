"""Import source boundaries, observable dry runs and URL-only coexistence."""

import uuid

from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSession, Method
from revaer_tooling.json_data import array_value

from tests.support.api_assertions import requires_key
from tests.support.indexers import (
    create_import_job,
    create_profile,
    public_paths,
    reject_missing_import_secret,
    run_backup,
)


def inspect_job(api: ApiClient, identifier: str) -> None:
    path = {"import_job_public_id": identifier}
    route = "/v1/indexers/import-jobs/{import_job_public_id}"
    assert api.request(ApiRequest(Method.GET, route + "/status", path=path)).status == 200
    results = api.request(ApiRequest(Method.GET, route + "/results", path=path))
    assert results.status == 200
    array_value(results.object()["results"])


def test_import_source_contracts(
    api: ApiClient, public_api: ApiClient, session: ApiSession
) -> None:
    requires_key(
        public_api,
        session,
        ApiRequest(Method.POST, "/v1/indexers/import-jobs", {"source": "prowlarr_api"}),
    )
    api_job = create_import_job(api, "prowlarr_api")
    reject_missing_import_secret(api, api_job)
    run_backup(api, api_job, "e2e-backup", 409)
    backup_job = create_import_job(api, "prowlarr_backup")
    run_backup(api, backup_job, "e2e-backup")
    reject_missing_import_secret(api, backup_job, 409)
    inspect_job(api, api_job)


def test_final_acceptance_boundaries(
    api: ApiClient, public_api: ApiClient, session: ApiSession
) -> None:
    requires_key(
        public_api,
        session,
        ApiRequest(
            Method.POST,
            "/v1/indexers/import-jobs",
            {"source": "prowlarr_backup", "is_dry_run": True},
        ),
    )
    backup_job = create_import_job(api, "prowlarr_backup")
    run_backup(api, backup_job, "acceptance-" + uuid.uuid4().hex)
    inspect_job(api, backup_job)
    reject_missing_import_secret(api, create_import_job(api, "prowlarr_api"))
    paths = public_paths(public_api)
    assert "/torznab/{torznab_instance_public_id}/api" in paths
    assert (
        "/torznab/{torznab_instance_public_id}/download/{canonical_torrent_source_public_id}"
        in paths
    )
    for unwanted in ("/v1/apps", "radarr", "sonarr"):
        assert unwanted not in paths


def test_coexistence_and_rollback(api: ApiClient, public_api: ApiClient) -> None:
    suffix = uuid.uuid4().hex
    identifier = create_profile(api, "Rollback Profile " + suffix)
    patch = "/v1/indexers/search-profiles/{search_profile_public_id}"
    path = {"search_profile_public_id": identifier}
    assert api.request(
        ApiRequest(
            Method.PATCH,
            patch,
            {"display_name": f"Rollback Profile {suffix} Before Import", "page_size": 25},
            path=path,
        )
    ).ok
    run_backup(api, create_import_job(api, "prowlarr_backup"), "rollback-" + suffix)
    assert api.request(
        ApiRequest(
            Method.PATCH,
            patch,
            {"display_name": f"Rollback Profile {suffix} After Import", "page_size": 30},
            path=path,
        )
    ).ok
    create_profile(api, "Rollback Profile After Import " + suffix, "tv")
    paths = public_paths(public_api)
    for unwanted in ("/v1/apps", "radarr", "sonarr", "lidarr", "readarr"):
        assert unwanted not in paths
