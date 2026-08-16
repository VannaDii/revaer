use crate::features::media::logic::{
    media_discovery_preview_path, media_job_diagnostics_path, media_recent_jobs_path,
};
use crate::models::{
    MediaCapabilityLatestResponse, MediaCapabilityReadinessResponse,
    MediaCapabilityRefreshResponse, MediaCompatibilityTargetListResponse,
    MediaCompatibilityTargetResponse, MediaCompatibilityTargetUpsertRequest,
    MediaComplianceResponse, MediaDiscoveryPreviewRequest, MediaDiscoveryPreviewResponse,
    MediaJobDiagnosticsResponse, MediaPolicyListResponse, MediaPolicyResponse,
    MediaPolicyUpsertRequest, MediaProfileListResponse, MediaProfilePatchRequest,
    MediaProfileResponse, MediaProfileUpsertRequest, MediaRecentJobPageResponse,
    MediaYamlApplyResponse, MediaYamlExportResponse, MediaYamlImportRequest,
    MediaYamlValidationResponse,
};
use crate::services::api::ApiClient;
use uuid::Uuid;

pub(crate) async fn fetch_profiles(client: &ApiClient) -> Result<MediaProfileListResponse, String> {
    client
        .get_api("/v1/media/profiles")
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn create_profile(
    client: &ApiClient,
    request: &MediaProfileUpsertRequest,
) -> Result<MediaProfileResponse, String> {
    client
        .post_api("/v1/media/profiles", request)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn patch_profile(
    client: &ApiClient,
    media_profile_public_id: Uuid,
    request: &MediaProfilePatchRequest,
) -> Result<MediaProfileResponse, String> {
    client
        .patch_api(
            &format!("/v1/media/profiles/{media_profile_public_id}"),
            request,
        )
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn fetch_recent_jobs(
    client: &ApiClient,
    limit: u16,
    cursor: Option<&str>,
    media_profile_public_id: Option<Uuid>,
) -> Result<MediaRecentJobPageResponse, String> {
    client
        .get_api(&media_recent_jobs_path(
            limit,
            cursor,
            media_profile_public_id,
        ))
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn fetch_job_diagnostics(
    client: &ApiClient,
    media_job_public_id: Uuid,
) -> Result<MediaJobDiagnosticsResponse, String> {
    client
        .get_api(&media_job_diagnostics_path(media_job_public_id))
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn preview_discovery(
    client: &ApiClient,
    request: &MediaDiscoveryPreviewRequest,
) -> Result<MediaDiscoveryPreviewResponse, String> {
    client
        .post_api(media_discovery_preview_path(), request)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn fetch_readiness(
    client: &ApiClient,
) -> Result<MediaCapabilityReadinessResponse, String> {
    client
        .get_api("/v1/media/capabilities/readiness")
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn fetch_latest_capability(
    client: &ApiClient,
) -> Result<MediaCapabilityLatestResponse, String> {
    client
        .get_api("/v1/media/capabilities")
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn fetch_compliance(
    client: &ApiClient,
) -> Result<MediaComplianceResponse, String> {
    client
        .get_api("/v1/media/compliance")
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn fetch_compatibility_targets(
    client: &ApiClient,
) -> Result<MediaCompatibilityTargetListResponse, String> {
    client
        .get_api("/v1/media/compatibility-targets")
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn upsert_compatibility_target(
    client: &ApiClient,
    request: &MediaCompatibilityTargetUpsertRequest,
) -> Result<MediaCompatibilityTargetResponse, String> {
    client
        .post_api("/v1/media/compatibility-targets", request)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn fetch_policies(client: &ApiClient) -> Result<MediaPolicyListResponse, String> {
    client
        .get_api("/v1/media/policies")
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn upsert_policy(
    client: &ApiClient,
    request: &MediaPolicyUpsertRequest,
) -> Result<MediaPolicyResponse, String> {
    client
        .post_api("/v1/media/policies", request)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn refresh_capability(
    client: &ApiClient,
) -> Result<MediaCapabilityRefreshResponse, String> {
    let empty = serde_json::json!({});
    client
        .post_api("/v1/media/capabilities/refresh", &empty)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn export_yaml(client: &ApiClient) -> Result<MediaYamlExportResponse, String> {
    client
        .get_api("/v1/media/export")
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn validate_yaml(
    client: &ApiClient,
    yaml_payload: String,
) -> Result<MediaYamlValidationResponse, String> {
    let request = MediaYamlImportRequest { yaml_payload };
    client
        .post_api("/v1/media/imports/validate", &request)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn apply_yaml(
    client: &ApiClient,
    yaml_payload: String,
) -> Result<MediaYamlApplyResponse, String> {
    let request = MediaYamlImportRequest { yaml_payload };
    client
        .post_api("/v1/media/imports/apply", &request)
        .await
        .map_err(|err| err.to_string())
}
