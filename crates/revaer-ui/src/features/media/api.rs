use crate::features::media::logic::{media_job_diagnostics_path, media_recent_jobs_path};
use crate::models::{
    MediaCapabilityLatestResponse, MediaCapabilityReadinessResponse,
    MediaCapabilityRefreshResponse, MediaCompatibilityTargetListResponse,
    MediaCompatibilityTargetResponse, MediaCompatibilityTargetUpsertRequest,
    MediaComplianceResponse, MediaJobDiagnosticsResponse, MediaPolicyListResponse,
    MediaPolicyResponse, MediaPolicyUpsertRequest, MediaRecentJobPageResponse,
    MediaYamlApplyResponse, MediaYamlExportResponse, MediaYamlImportRequest,
    MediaYamlValidationResponse,
};
use crate::services::api::ApiClient;
use uuid::Uuid;

pub(crate) use super::root_catalog_api::fetch_root_catalog;

pub(crate) async fn fetch_profile_edit(
    client: &ApiClient,
    id: Uuid,
) -> Result<super::profile_authoring::ProfileEdit, &'static str> {
    let (response, etag) = client
        .get_api_versioned(&format!("/v1/media/profiles/{id}"))
        .await
        .map_err(|_| "Profile could not be loaded. The draft is preserved.")?;
    super::profile_authoring::ProfileEdit::loaded(id, &response, etag)
}

pub(crate) async fn replace_profile_version(
    client: &ApiClient,
    edit: &super::profile_authoring::ProfileEdit,
    request: &revaer_api_models::media_root_contract::ProfileVersionRequest,
) -> Result<i32, &'static str> {
    let (response, etag) = client
        .replace_api(
            &format!("/v1/media/profiles/{}", edit.id),
            &edit.etag,
            request,
        )
        .await
        .map_err(|error| super::profile_authoring::mutation_failure(error.status))?;
    edit.confirm_replacement(request, &response, &etag)
}

pub(crate) async fn create_profile_version(
    client: &ApiClient,
    request: &revaer_api_models::media_root_contract::ProfileVersionRequest,
) -> Result<(), &'static str> {
    let (response, etag) = client
        .create_api::<_, revaer_api_models::media_root_contract::ProfileVersionResponse>(
            "/v1/media/profiles",
            request,
        )
        .await
        .map_err(|error| super::profile_authoring::mutation_failure(error.status))?;
    super::profile_authoring::confirm_creation(&response, &etag, request)
}

pub(crate) async fn fetch_profile_choices(
    client: &ApiClient,
    cursor: Option<&str>,
) -> Result<super::association::ProfileChoices, &'static str> {
    let mut path = "/v1/media/profiles?limit=200".to_owned();
    if let Some(cursor) = cursor {
        path.push_str("&cursor=");
        path.push_str(&urlencoding::encode(cursor));
    }
    let page = client.get_api::<crate::models::media_configuration::ProfileHeadPage>(&path)
        .await.map_err(|_| "Active profile versions could not be loaded. This workflow requires the immutable profile contract.")?;
    super::association::ProfileChoices::try_from(page)
}

pub(crate) async fn create_association(
    client: &ApiClient,
    request: &revaer_api_models::media_root_contract::DiscoveryAssociationRequest,
) -> Result<(String, Option<Uuid>), &'static str> {
    let (response, etag) = client
        .create_api::<_, crate::models::media_configuration::AssociationCreated>(
            "/v1/media/discovery-associations",
            request,
        )
        .await
        .map_err(|error| super::association::mutation_failure(error.status))?;
    let message = super::association::confirm_creation(&response, &etag, request)?;
    let manual = (response.manual_enabled && response.active_version == Some(1))
        .then_some(response.media_discovery_association_public_id);
    Ok((message, manual))
}

pub(crate) async fn preview_manual_discovery(
    client: &ApiClient,
    request: &revaer_api_models::MediaDiscoveryPreviewRequest,
) -> Result<revaer_api_models::MediaDiscoveryPreviewResponse, &'static str> {
    let response = client
        .post_api("/v1/media/discovery/preview", request)
        .await
        .map_err(|_| "Discovery preview failed. The candidate draft was preserved.")?;
    super::manual_discovery::confirm_preview(request, &response)?;
    Ok(response)
}

pub(crate) async fn fetch_association_page(
    client: &ApiClient,
    cursor: Option<&str>,
) -> Result<super::manual_discovery::AssociationPage, &'static str> {
    let mut path = "/v1/media/discovery-associations?limit=50".to_owned();
    if let Some(cursor) = cursor {
        path.push_str("&cursor=");
        path.push_str(&urlencoding::encode(cursor));
    }
    client.get_api_private::<revaer_api_models::media_root_contract::DiscoveryAssociationPageResponse>(&path)
        .await.map(super::manual_discovery::AssociationPage::from)
        .map_err(|_| "Discovery associations could not be loaded.")
}

pub(crate) async fn run_manual_discovery(
    client: &ApiClient,
    request: &revaer_api_models::MediaDiscoveryPreviewRequest,
) -> Result<revaer_api_models::MediaDiscoveryRunResponse, &'static str> {
    let response = client
        .post_api("/v1/media/discovery/runs", request)
        .await
        .map_err(|_| "Discovery was not confirmed. Check recent jobs before resubmitting.")?;
    super::manual_discovery::confirm_run(request, &response)?;
    Ok(response)
}

pub(crate) async fn fetch_root_readiness(
    client: &ApiClient,
) -> Result<super::root_readiness::RootSummary, super::root_readiness::RootLoadFailure> {
    client
        .get_api::<revaer_api_models::media_root_contract::RootCatalogReadinessResponse>(
            "/v1/media/root-catalog/readiness",
        )
        .await
        .map(super::root_readiness::RootSummary::from)
        .map_err(|error| super::root_readiness::RootLoadFailure::from_status(error.status))
}

pub(crate) async fn fetch_profile_page(
    client: &ApiClient,
    cursor: Option<&str>,
) -> Result<super::profile_list::ProfilePage, &'static str> {
    let mut path = "/v1/media/profiles?limit=50".to_owned();
    if let Some(cursor) = cursor {
        path.push_str("&cursor=");
        path.push_str(&urlencoding::encode(cursor));
    }
    client
        .get_api::<revaer_api_models::media_root_contract::ProfileVersionPageResponse>(&path)
        .await
        .map(super::profile_list::ProfilePage::from)
        .map_err(|_| "Profiles could not be loaded.")
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
    let response: MediaPolicyResponse = client
        .post_api("/v1/media/policies", request)
        .await
        .map_err(|err| err.to_string())?;
    if response.policy_key != request.policy_key.trim()
        || response.version != request.version
        || response.display_name != request.display_name.trim()
        || response.output != request.output
        || response.video_intent != request.video_intent.trim().to_ascii_lowercase()
        || response.verification_strictness
            != request.verification_strictness.trim().to_ascii_lowercase()
        || response.verification_duration_tolerance_millis
            != request.verification_duration_tolerance_millis
        || response.verification_mux_validation != request.verification_mux_validation
        || response.verification_decode_all_streams != request.verification_decode_all_streams
        || response.verification_keyframe_seek != request.verification_keyframe_seek
        || response.verification_playback_probe != request.verification_playback_probe
    {
        return Err("Policy response could not be confirmed. Refresh before retrying.".into());
    }
    Ok(response)
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
    let request = MediaYamlImportRequest {
        yaml_payload,
        preconditions: None,
    };
    client
        .post_api("/v1/media/imports/validate", &request)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn apply_yaml(
    client: &ApiClient,
    yaml_payload: String,
) -> Result<MediaYamlApplyResponse, String> {
    let request = MediaYamlImportRequest {
        yaml_payload,
        preconditions: None,
    };
    client
        .post_api("/v1/media/imports/apply", &request)
        .await
        .map_err(|err| err.to_string())
}
