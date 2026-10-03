"""Create scenario-owned records; missing IDs and setup failures are test failures.

These helpers deliberately do not return random fallback identifiers. A failed
Torznab creation must not silently bypass all subsequent compatibility checks.
"""

import uuid
from dataclasses import dataclass, field

from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import object_value, string_value


@dataclass(frozen=True)
class TorznabInstance:
    identifier: str
    api_key: str = field(repr=False)


def cardigann_import(slug: str) -> ApiRequest:
    return ApiRequest(
        Method.POST,
        "/v1/indexers/definitions/import/cardigann",
        {
            "is_deprecated": False,
            "yaml_payload": f"""id: {slug}
name: Cardigann E2E {slug}
caps:
  search:
    - q
settings:
  - name: apiKey
    label: API key
    type: apikey
    required: true
  - name: sort
    type: select
    default: seeders
    options:
      - value: seeders
        label: Seeders
      - date
""",
        },
    )


def create_profile(api: ApiClient, name: str, domain: str = "movies") -> str:
    created = api.request(
        ApiRequest(
            Method.POST,
            "/v1/indexers/search-profiles",
            {
                "display_name": name,
                "page_size": 20,
                "default_media_domain_key": domain,
            },
        )
    )
    assert created.status == 201
    return string_value(created.object()["search_profile_public_id"])


def create_torznab(api: ApiClient, profile: str, name: str) -> TorznabInstance:
    created = api.request(
        ApiRequest(
            Method.POST,
            "/v1/indexers/torznab-instances",
            {
                "search_profile_public_id": profile,
                "display_name": name,
            },
        )
    )
    assert created.status == 201
    return TorznabInstance(
        string_value(created.object()["torznab_instance_public_id"]),
        string_value(created.object()["api_key_plaintext"]),
    )


def create_import_job(api: ApiClient, source: str) -> str:
    created = api.request(
        ApiRequest(Method.POST, "/v1/indexers/import-jobs", {"source": source, "is_dry_run": True})
    )
    assert created.status == 201
    return string_value(created.object()["import_job_public_id"])


def run_backup(api: ApiClient, identifier: str, reference: str, status: int = 204) -> None:
    assert (
        api.request(
            ApiRequest(
                Method.POST,
                "/v1/indexers/import-jobs/{import_job_public_id}/run/prowlarr-backup",
                {"backup_blob_ref": reference},
                path={"import_job_public_id": identifier},
            )
        ).status
        == status
    )


def reject_missing_import_secret(api: ApiClient, identifier: str, status: int = 404) -> None:
    assert (
        api.request(
            ApiRequest(
                Method.POST,
                "/v1/indexers/import-jobs/{import_job_public_id}/run/prowlarr-api",
                {
                    "prowlarr_url": "http://prowlarr.local",
                    "prowlarr_api_key_secret_public_id": str(uuid.uuid4()),
                },
                path={"import_job_public_id": identifier},
            )
        ).status
        == status
    )


def public_paths(api: ApiClient) -> str:
    document = api.request(ApiRequest(Method.GET, "/docs/openapi.json"))
    assert document.status == 200
    return "\n".join(object_value(document.object()["paths"]))
