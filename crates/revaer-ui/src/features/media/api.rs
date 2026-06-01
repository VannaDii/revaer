use crate::features::media::logic::media_jobs_path;
use crate::models::{
    MediaCapabilityLatestResponse, MediaCapabilityReadinessResponse,
    MediaCapabilityRefreshResponse, MediaComplianceResponse, MediaJobListResponse,
    MediaProfileListResponse, MediaProfilePatchRequest, MediaProfileResponse,
    MediaProfileUpsertRequest, MediaYamlApplyResponse, MediaYamlExportResponse,
    MediaYamlImportRequest, MediaYamlValidationResponse,
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

pub(crate) async fn fetch_jobs_for_profiles(
    client: &ApiClient,
    profiles: &[MediaProfileResponse],
) -> Result<MediaJobListResponse, String> {
    let mut jobs = Vec::new();
    for profile in profiles {
        let path = media_jobs_path(Some(profile.media_profile_public_id));
        let mut response: MediaJobListResponse =
            client.get_api(&path).await.map_err(|err| err.to_string())?;
        jobs.append(&mut response.jobs);
    }
    Ok(MediaJobListResponse { jobs })
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
