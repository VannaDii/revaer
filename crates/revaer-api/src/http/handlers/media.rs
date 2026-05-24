//! Media profile, job, and capability endpoints.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::media::{
    MediaCapabilityRecordParams, MediaJobCreateParams, MediaJobPhaseAppendParams,
    MediaProfileUpsertParams, MediaServiceError, MediaServiceErrorKind,
};
use crate::app::state::ApiState;
use crate::http::errors::ApiError;
use crate::http::handlers::indexers::SYSTEM_ACTOR_PUBLIC_ID;
use crate::models::{
    MediaCapabilityLatestResponse, MediaCapabilityRecordRequest, MediaCapabilityRecordResponse,
    MediaCapabilitySnapshotResponse, MediaJobCreateRequest, MediaJobCreateResponse,
    MediaJobListResponse, MediaJobPhaseAppendRequest, MediaJobResponse, MediaProfileListResponse,
    MediaProfileResponse, MediaProfileUpsertRequest, MediaYamlApplyResponse,
    MediaYamlExportResponse, MediaYamlImportRequest, MediaYamlValidationResponse,
};

const MEDIA_PROFILE_UPSERT_FAILED: &str = "failed to upsert media profile";
const MEDIA_PROFILE_LIST_FAILED: &str = "failed to list media profiles";
const MEDIA_JOB_CREATE_FAILED: &str = "failed to create media job";
const MEDIA_JOB_LIST_FAILED: &str = "failed to list media jobs";
const MEDIA_JOB_PHASE_APPEND_FAILED: &str = "failed to append media job phase";
const MEDIA_CAPABILITY_RECORD_FAILED: &str = "failed to record media capability snapshot";
const MEDIA_CAPABILITY_LATEST_FAILED: &str = "failed to load latest media capability snapshot";
const MEDIA_YAML_EXPORT_FAILED: &str = "failed to export media yaml";
const MEDIA_YAML_VALIDATE_FAILED: &str = "failed to validate media yaml";
const MEDIA_YAML_APPLY_FAILED: &str = "failed to apply media yaml";
const PROFILE_KEY_REQUIRED: &str = "profile_key is required";
const SOURCE_ROOT_REQUIRED: &str = "source_root is required";
const OUTPUT_ROOT_REQUIRED: &str = "output_root is required";
const SOURCE_PATH_REQUIRED: &str = "source_path is required";
const PHASE_NAME_REQUIRED: &str = "phase_name is required";
const PHASE_STATUS_REQUIRED: &str = "phase_status is required";
const FFMPEG_VERSION_REQUIRED: &str = "ffmpeg_version is required";
const FFPROBE_VERSION_REQUIRED: &str = "ffprobe_version is required";
const CODEC_NAME_REQUIRED: &str = "codec_name is required";
const YAML_PAYLOAD_REQUIRED: &str = "yaml_payload is required";

#[derive(Debug, Deserialize)]
pub(crate) struct MediaJobsQuery {
    media_profile_public_id: Uuid,
    status: Option<String>,
}

pub(crate) async fn upsert_media_profile(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaProfileUpsertRequest>,
) -> Result<(StatusCode, Json<MediaProfileResponse>), ApiError> {
    let profile_key = normalize_required_str_field(&request.profile_key, PROFILE_KEY_REQUIRED)?;
    let source_root = normalize_required_str_field(&request.source_root, SOURCE_ROOT_REQUIRED)?;
    let output_root = normalize_required_str_field(&request.output_root, OUTPUT_ROOT_REQUIRED)?;

    let profile_id = state
        .media
        .media_profile_upsert(MediaProfileUpsertParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            profile_key,
            source_root,
            output_root,
            dry_run_only: request.dry_run_only,
            retention_days: request.retention_days,
        })
        .await
        .map_err(|err| {
            map_media_error("media_profile_upsert", MEDIA_PROFILE_UPSERT_FAILED, &err)
        })?;

    let profile = state
        .media
        .media_profile_list()
        .await
        .map_err(|err| map_media_error("media_profile_list", MEDIA_PROFILE_LIST_FAILED, &err))?
        .into_iter()
        .find(|item| item.media_profile_public_id == profile_id)
        .ok_or_else(|| ApiError::not_found(MEDIA_PROFILE_UPSERT_FAILED))?;

    Ok((StatusCode::CREATED, Json(map_profile(profile))))
}

pub(crate) async fn list_media_profiles(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaProfileListResponse>, ApiError> {
    let profiles = state
        .media
        .media_profile_list()
        .await
        .map_err(|err| map_media_error("media_profile_list", MEDIA_PROFILE_LIST_FAILED, &err))?
        .into_iter()
        .map(map_profile)
        .collect();

    Ok(Json(MediaProfileListResponse { profiles }))
}

pub(crate) async fn create_media_job(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaJobCreateRequest>,
) -> Result<(StatusCode, Json<MediaJobCreateResponse>), ApiError> {
    let source_path = normalize_required_str_field(&request.source_path, SOURCE_PATH_REQUIRED)?;
    let output_path = trim_and_filter_empty(request.output_path.as_deref());

    let media_job_public_id = state
        .media
        .media_job_create(MediaJobCreateParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            media_profile_public_id: request.media_profile_public_id,
            source_path,
            output_path,
            dry_run: request.dry_run,
        })
        .await
        .map_err(|err| map_media_error("media_job_create", MEDIA_JOB_CREATE_FAILED, &err))?;

    Ok((
        StatusCode::CREATED,
        Json(MediaJobCreateResponse {
            media_job_public_id,
        }),
    ))
}

pub(crate) async fn list_media_jobs(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<MediaJobsQuery>,
) -> Result<Json<MediaJobListResponse>, ApiError> {
    let status = trim_and_filter_empty(query.status.as_deref());
    let jobs = state
        .media
        .media_job_list(query.media_profile_public_id, status)
        .await
        .map_err(|err| map_media_error("media_job_list", MEDIA_JOB_LIST_FAILED, &err))?
        .into_iter()
        .map(map_job)
        .collect();

    Ok(Json(MediaJobListResponse { jobs }))
}

pub(crate) async fn append_media_job_phase(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
    Json(request): Json<MediaJobPhaseAppendRequest>,
) -> Result<StatusCode, ApiError> {
    let phase_name = normalize_required_str_field(&request.phase_name, PHASE_NAME_REQUIRED)?;
    let phase_status = normalize_required_str_field(&request.phase_status, PHASE_STATUS_REQUIRED)?;

    state
        .media
        .media_job_phase_append(MediaJobPhaseAppendParams {
            media_job_public_id,
            phase_index: request.phase_index,
            phase_name,
            phase_status,
            details_text: trim_and_filter_empty(request.details_text.as_deref()),
        })
        .await
        .map_err(|err| {
            map_media_error(
                "media_job_phase_append",
                MEDIA_JOB_PHASE_APPEND_FAILED,
                &err,
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn record_media_capability(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaCapabilityRecordRequest>,
) -> Result<(StatusCode, Json<MediaCapabilityRecordResponse>), ApiError> {
    let ffmpeg_version =
        normalize_required_str_field(&request.ffmpeg_version, FFMPEG_VERSION_REQUIRED)?;
    let ffprobe_version =
        normalize_required_str_field(&request.ffprobe_version, FFPROBE_VERSION_REQUIRED)?;
    let codec_name = normalize_required_str_field(&request.codec_name, CODEC_NAME_REQUIRED)?;

    let media_capability_snapshot_id = state
        .media
        .media_capability_record(MediaCapabilityRecordParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            ffmpeg_version,
            ffprobe_version,
            codec_name,
            encode_supported: request.encode_supported,
            decode_supported: request.decode_supported,
        })
        .await
        .map_err(|err| {
            map_media_error(
                "media_capability_record",
                MEDIA_CAPABILITY_RECORD_FAILED,
                &err,
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(MediaCapabilityRecordResponse {
            media_capability_snapshot_id,
        }),
    ))
}

pub(crate) async fn latest_media_capability(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaCapabilityLatestResponse>, ApiError> {
    let snapshot = state
        .media
        .media_capability_latest()
        .await
        .map_err(|err| {
            map_media_error(
                "media_capability_latest",
                MEDIA_CAPABILITY_LATEST_FAILED,
                &err,
            )
        })?
        .map(|row| MediaCapabilitySnapshotResponse {
            media_capability_snapshot_id: row.media_capability_snapshot_id,
            ffmpeg_version: row.ffmpeg_version,
            ffprobe_version: row.ffprobe_version,
            codec_name: row.codec_name,
            encode_supported: row.encode_supported,
            decode_supported: row.decode_supported,
            observed_at: row.observed_at,
        });

    Ok(Json(MediaCapabilityLatestResponse { snapshot }))
}

pub(crate) async fn export_media_yaml(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaYamlExportResponse>, ApiError> {
    let yaml_payload = state
        .media
        .media_yaml_export()
        .await
        .map_err(|err| map_media_error("media_yaml_export", MEDIA_YAML_EXPORT_FAILED, &err))?;
    Ok(Json(MediaYamlExportResponse {
        version: "revaer.media.v1".to_string(),
        yaml_payload,
    }))
}

pub(crate) async fn validate_media_yaml(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaYamlImportRequest>,
) -> Result<Json<MediaYamlValidationResponse>, ApiError> {
    let yaml_payload = normalize_required_str_field(&request.yaml_payload, YAML_PAYLOAD_REQUIRED)?;
    let result = state
        .media
        .media_yaml_validate(yaml_payload)
        .await
        .map_err(|err| map_media_error("media_yaml_validate", MEDIA_YAML_VALIDATE_FAILED, &err))?;
    Ok(Json(MediaYamlValidationResponse {
        version: result.version,
        valid: result.valid,
        issues: result.issues,
        profile_count: result.profiles.len(),
    }))
}

pub(crate) async fn apply_media_yaml(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaYamlImportRequest>,
) -> Result<(StatusCode, Json<MediaYamlApplyResponse>), ApiError> {
    let yaml_payload = normalize_required_str_field(&request.yaml_payload, YAML_PAYLOAD_REQUIRED)?;
    let result = state
        .media
        .media_yaml_apply(SYSTEM_ACTOR_PUBLIC_ID, yaml_payload)
        .await
        .map_err(|err| map_media_error("media_yaml_apply", MEDIA_YAML_APPLY_FAILED, &err))?;
    Ok((
        StatusCode::CREATED,
        Json(MediaYamlApplyResponse {
            forced_dry_run: result.forced_dry_run,
            media_profile_public_ids: result.media_profile_public_ids,
        }),
    ))
}

fn map_profile(profile: crate::app::media::MediaProfileResponse) -> MediaProfileResponse {
    MediaProfileResponse {
        media_profile_public_id: profile.media_profile_public_id,
        profile_key: profile.profile_key,
        source_root: profile.source_root,
        output_root: profile.output_root,
        dry_run_only: profile.dry_run_only,
        retention_days: profile.retention_days,
        updated_at: profile.updated_at,
    }
}

fn map_job(job: crate::app::media::MediaJobResponse) -> MediaJobResponse {
    MediaJobResponse {
        media_job_public_id: job.media_job_public_id,
        source_path: job.source_path,
        output_path: job.output_path,
        status: job.status,
        dry_run: job.dry_run,
        queued_at: job.queued_at,
        started_at: job.started_at,
        completed_at: job.completed_at,
        last_error: job.last_error,
    }
}

fn map_media_error(
    operation: &'static str,
    detail: &'static str,
    err: &MediaServiceError,
) -> ApiError {
    let mut api_error = match err.kind() {
        MediaServiceErrorKind::Invalid => ApiError::bad_request(detail),
        MediaServiceErrorKind::NotFound => ApiError::not_found(detail),
        MediaServiceErrorKind::Conflict => ApiError::conflict(detail),
        MediaServiceErrorKind::Storage => ApiError::internal(detail),
    };

    api_error = api_error.with_context_field("operation", operation);
    if let Some(code) = err.code() {
        api_error = api_error.with_context_field("error_code", code);
    }
    if let Some(sqlstate) = err.sqlstate() {
        api_error = api_error.with_context_field("sqlstate", sqlstate);
    }
    api_error
}

fn normalize_required_str_field<'a>(
    value: &'a str,
    message: &'static str,
) -> Result<&'a str, ApiError> {
    trim_and_filter_empty(Some(value)).ok_or_else(|| ApiError::bad_request(message))
}

fn trim_and_filter_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::handlers::indexers::test_support::{RecordingIndexers, indexer_test_state};
    use crate::models::ProblemDetails;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn list_media_profiles_returns_empty_payload_with_default_facade() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = list_media_profiles(State(state)).await?;
        assert!(response.profiles.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn create_media_job_maps_noop_storage_failure_to_internal() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let request = MediaJobCreateRequest {
            media_profile_public_id: Uuid::new_v4(),
            source_path: "/input/demo.mkv".to_string(),
            output_path: Some("/output/demo.mkv".to_string()),
            dry_run: true,
        };

        let err = create_media_job(State(state), Json(request))
            .await
            .expect_err("noop media facade should fail writes");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        Ok(())
    }

    #[tokio::test]
    async fn latest_media_capability_returns_empty_snapshot_with_default_facade()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = latest_media_capability(State(state)).await?;
        assert!(response.snapshot.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn map_media_error_preserves_capability_snapshot_missing_code() -> anyhow::Result<()> {
        let err = MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_capability_snapshot_missing");
        let api_error = map_media_error("media_job_create", MEDIA_JOB_CREATE_FAILED, &err);
        let response = api_error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), 64 * 1024).await?;
        let problem: ProblemDetails = serde_json::from_slice(&body)?;
        let context = problem.context.unwrap_or_default();
        assert!(
            context.iter().any(|item| item.name == "error_code"
                && item.value == "media_capability_snapshot_missing")
        );
        Ok(())
    }

    #[tokio::test]
    async fn map_media_error_preserves_capability_snapshot_invalid_code() -> anyhow::Result<()> {
        let err = MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_capability_snapshot_invalid");
        let api_error = map_media_error("media_job_create", MEDIA_JOB_CREATE_FAILED, &err);
        let response = api_error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), 64 * 1024).await?;
        let problem: ProblemDetails = serde_json::from_slice(&body)?;
        let context = problem.context.unwrap_or_default();
        assert!(
            context.iter().any(|item| item.name == "error_code"
                && item.value == "media_capability_snapshot_invalid")
        );
        Ok(())
    }
}
