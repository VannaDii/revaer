use crate::models::{
    MediaCapabilityLatestResponse, MediaCapabilityReadinessResponse,
    MediaCapabilityRefreshResponse, MediaJobListResponse, MediaProfileListResponse,
    MediaYamlApplyResponse, MediaYamlExportResponse, MediaYamlImportRequest,
    MediaYamlValidationResponse,
};
use crate::services::api::ApiClient;

pub(crate) async fn fetch_profiles(client: &ApiClient) -> Result<MediaProfileListResponse, String> {
    client
        .get_api("/v1/media/profiles")
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn fetch_jobs(client: &ApiClient) -> Result<MediaJobListResponse, String> {
    client
        .get_api("/v1/media/jobs")
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
