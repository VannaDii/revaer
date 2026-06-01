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
    MediaCapabilityRefreshParams, MediaDiscoveryAutomationRunParams, MediaDiscoveryPreviewParams,
    MediaDiscoveryRunParams, MediaDiscoveryRunResponse as AppMediaDiscoveryRunResponse,
    MediaJobCreateParams, MediaJobPhaseAppendParams, MediaProfilePatchParams,
    MediaProfileUpsertParams, MediaServiceError, MediaServiceErrorKind,
};
use crate::app::state::ApiState;
use crate::http::errors::ApiError;
use crate::http::handlers::indexers::SYSTEM_ACTOR_PUBLIC_ID;
use crate::models::{
    MediaCapabilityCodecResponse, MediaCapabilityFeatureResponse, MediaCapabilityLatestResponse,
    MediaCapabilityReadinessResponse, MediaCapabilityRefreshResponse,
    MediaCapabilitySnapshotResponse, MediaCompatibilityTargetListResponse,
    MediaCompatibilityTargetResponse, MediaCompatibilityTargetUpsertRequest,
    MediaComplianceResponse, MediaDiscoveryPreviewItemResponse, MediaDiscoveryPreviewRequest,
    MediaDiscoveryPreviewResponse, MediaDiscoveryQueuedJobResponse, MediaDiscoveryRunRequest,
    MediaDiscoveryRunResponse, MediaDiscoveryScheduleListResponse, MediaDiscoveryScheduleResponse,
    MediaDiscoverySkippedItemResponse, MediaDiscoveryWatcherListResponse,
    MediaDiscoveryWatcherResponse, MediaJobArtifactListResponse, MediaJobCompactAuditListResponse,
    MediaJobCreateRequest, MediaJobCreateResponse, MediaJobListResponse,
    MediaJobOperationListResponse, MediaJobPhaseAppendRequest, MediaJobPlanReasonListResponse,
    MediaJobResponse, MediaJobRetentionResponse, MediaJobRetentionUpdateRequest,
    MediaJobVerificationCheckListResponse, MediaJobViolationListResponse,
    MediaPlanningPreviewRequest, MediaPlanningPreviewResponse, MediaPolicyListResponse,
    MediaPolicyResponse, MediaPolicyUpsertRequest, MediaProfileListResponse,
    MediaProfilePatchRequest, MediaProfileResponse, MediaProfileUpsertRequest,
    MediaProfileValidationResponse, MediaYamlApplyResponse, MediaYamlExportResponse,
    MediaYamlImportRequest, MediaYamlValidationResponse,
};

const MEDIA_PROFILE_UPSERT_FAILED: &str = "failed to upsert media profile";
const MEDIA_PROFILE_LIST_FAILED: &str = "failed to list media profiles";
const MEDIA_PROFILE_NOT_FOUND: &str = "media profile not found";
const MEDIA_JOB_CREATE_FAILED: &str = "failed to create media job";
const MEDIA_DISCOVERY_PREVIEW_FAILED: &str = "failed to preview media discovery";
const MEDIA_DISCOVERY_RUN_FAILED: &str = "failed to run media discovery";
const MEDIA_DISCOVERY_SCHEDULE_RUN_FAILED: &str = "failed to run scheduled media discovery";
const MEDIA_DISCOVERY_SCHEDULE_LIST_FAILED: &str = "failed to list media discovery schedules";
const MEDIA_DISCOVERY_WATCHER_LIST_FAILED: &str = "failed to list media discovery watchers";
const MEDIA_JOB_LIST_FAILED: &str = "failed to list media jobs";
const MEDIA_JOB_GET_FAILED: &str = "failed to load media job";
const MEDIA_JOB_CANCEL_FAILED: &str = "failed to cancel media job";
const MEDIA_JOB_RETRY_FAILED: &str = "failed to retry media job";
const MEDIA_JOB_PHASE_APPEND_FAILED: &str = "failed to append media job phase";
const MEDIA_JOB_OPERATION_LIST_FAILED: &str = "failed to list media job operations";
const MEDIA_JOB_VIOLATION_LIST_FAILED: &str = "failed to list media job violations";
const MEDIA_JOB_PLAN_REASON_LIST_FAILED: &str = "failed to list media job plan reasons";
const MEDIA_JOB_VERIFICATION_CHECK_LIST_FAILED: &str =
    "failed to list media job verification checks";
const MEDIA_JOB_ARTIFACT_LIST_FAILED: &str = "failed to list media job artifacts";
const MEDIA_JOB_COMPACT_AUDIT_LIST_FAILED: &str = "failed to list media job compact audits";
const MEDIA_CAPABILITY_LATEST_FAILED: &str = "failed to load latest media capability snapshot";
const MEDIA_CAPABILITY_READINESS_FAILED: &str = "failed to determine media capability readiness";
const MEDIA_CAPABILITY_REFRESH_FAILED: &str = "failed to refresh media capability snapshot";
const MEDIA_YAML_EXPORT_FAILED: &str = "failed to export media yaml";
const MEDIA_YAML_VALIDATE_FAILED: &str = "failed to validate media yaml";
const MEDIA_YAML_APPLY_FAILED: &str = "failed to apply media yaml";
const PROFILE_KEY_REQUIRED: &str = "profile_key is required";
const SOURCE_ROOT_REQUIRED: &str = "source_root is required";
const OUTPUT_ROOT_REQUIRED: &str = "output_root is required";
const SOURCE_PATH_REQUIRED: &str = "source_path is required";
const COMPATIBILITY_TARGET_KEY_REQUIRED: &str = "compatibility_target_key is required";
const DISPLAY_NAME_REQUIRED: &str = "display_name is required";
const VIDEO_CODEC_REQUIRED: &str = "video_codec is required";
const AUDIO_CODEC_REQUIRED: &str = "audio_codec is required";
const POLICY_KEY_REQUIRED: &str = "policy_key is required";
const VIDEO_INTENT_REQUIRED: &str = "video_intent is required";
const DISCOVERY_SOURCE_PATHS_REQUIRED: &str = "source_paths must contain at least one path";
const DISCOVERY_SOURCE_PATHS_TOO_LARGE: &str = "source_paths exceeds maximum size";
const DISCOVERY_SOURCE_PATH_TOO_LARGE: &str = "source_path exceeds maximum size";
const PHASE_NAME_REQUIRED: &str = "phase_name is required";
const PHASE_STATUS_REQUIRED: &str = "phase_status is required";
const YAML_PAYLOAD_REQUIRED: &str = "yaml_payload is required";
const RETENTION_DAYS_INVALID: &str = "retention_days must be between 1 and 3650";
const VERSION_INVALID: &str = "version must be greater than zero";
const SUBTITLE_POLICY_INVALID: &str = "subtitle_policy must be one of: selected, all, none";
const VIDEO_INTENT_INVALID: &str = "video_intent must be one of: general, anime, archival";
const SCHEDULE_INTERVAL_INVALID: &str = "schedule_interval_minutes must be between 1 and 525600";
const SCHEDULE_INTERVAL_REQUIRED: &str =
    "schedule_interval_minutes is required when schedule is enabled";
const WATCHER_UNAVAILABLE: &str = "watcher discovery is unavailable until event debouncing lands";
const MEDIA_STATUS_INVALID: &str =
    "status must be one of: queued, running, verifying, completed, failed, cancelled";
const PHASE_STATUS_INVALID: &str =
    "phase_status must be one of: queued, running, verifying, completed, failed, cancelled";
const MEDIA_LICENSE_MODE: &str = "redistributable-gplv3-runtime";
const MEDIA_SOURCE_OFFER_PATH: &str = "/app/compliance/SOURCE-OFFER.txt";
const MEDIA_THIRD_PARTY_NOTICES_PATH: &str = "/app/compliance/THIRD-PARTY-NOTICES.md";
const MEDIA_SBOM_PATH: &str = "/app/compliance/media-runtime-inventory.spdx.json";
const MEDIA_INVENTORY_PATH: &str = "/app/compliance/media-runtime-inventory.spdx.json";
const MEDIA_EXIFTOOL_EXCEPTION_PATH: &str = "/app/compliance/exiftool-exception.md";
const DISCOVERY_SOURCE_PATHS_MAX_LEN: usize = 1024;
const DISCOVERY_SOURCE_PATH_MAX_BYTES: usize = 4096;
const MEDIA_LICENSE_EXCLUDED_CAPABILITIES: [&str; 5] = [
    "--enable-nonfree",
    "libfdk_aac",
    "proprietary-codecs",
    "license-incompatible-codecs",
    "unapproved-source-form-utilities",
];

#[derive(Debug, Deserialize)]
pub(crate) struct MediaJobsQuery {
    media_profile_public_id: Option<Uuid>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MediaYamlExportQuery {
    include_local_paths: Option<bool>,
}

pub(crate) async fn upsert_media_profile(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaProfileUpsertRequest>,
) -> Result<(StatusCode, Json<MediaProfileResponse>), ApiError> {
    let profile_key = normalize_required_str_field(&request.profile_key, PROFILE_KEY_REQUIRED)?;
    let source_root = normalize_required_str_field(&request.source_root, SOURCE_ROOT_REQUIRED)?;
    let output_root = normalize_required_str_field(&request.output_root, OUTPUT_ROOT_REQUIRED)?;
    validate_retention_days(request.retention_days)?;
    validate_schedule(request.schedule_enabled, request.schedule_interval_minutes)?;
    validate_watcher_disabled(request.watcher_enabled)?;

    let profile_id = state
        .media
        .media_profile_upsert(MediaProfileUpsertParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            profile_key,
            source_root,
            output_root,
            dry_run_only: true,
            retention_days: request.retention_days,
            compatibility_target_key: trim_and_filter_empty(
                request.compatibility_target_key.as_deref(),
            ),
            policy_key: request.policy_key.trim(),
            watcher_enabled: request.watcher_enabled,
            schedule_enabled: request.schedule_enabled,
            schedule_interval_minutes: request.schedule_interval_minutes,
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

pub(crate) async fn patch_media_profile(
    State(state): State<Arc<ApiState>>,
    Path(media_profile_public_id): Path<Uuid>,
    Json(request): Json<MediaProfilePatchRequest>,
) -> Result<Json<MediaProfileResponse>, ApiError> {
    if let Some(retention_days) = request.retention_days {
        validate_retention_days(retention_days)?;
    }
    if let Some(interval) = request.schedule_interval_minutes {
        validate_schedule_interval(interval)?;
    }
    if request.watcher_enabled == Some(true) {
        return Err(ApiError::bad_request(WATCHER_UNAVAILABLE));
    }
    if request.schedule_enabled == Some(true) && request.schedule_interval_minutes.is_none() {
        let current = state
            .media
            .media_profile_list()
            .await
            .map_err(|err| map_media_error("media_profile_list", MEDIA_PROFILE_LIST_FAILED, &err))?
            .into_iter()
            .find(|item| item.media_profile_public_id == media_profile_public_id)
            .ok_or_else(|| ApiError::not_found(MEDIA_PROFILE_NOT_FOUND))?;
        if current.schedule_interval_minutes.is_none() {
            return Err(ApiError::bad_request(SCHEDULE_INTERVAL_REQUIRED));
        }
    }

    state
        .media
        .media_profile_patch(MediaProfilePatchParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            media_profile_public_id,
            source_root: trim_and_filter_empty(request.source_root.as_deref()),
            output_root: trim_and_filter_empty(request.output_root.as_deref()),
            dry_run_only: request.dry_run_only,
            retention_days: request.retention_days,
            compatibility_target_key: request.compatibility_target_key.as_deref(),
            policy_key: trim_and_filter_empty(request.policy_key.as_deref()),
            watcher_enabled: request.watcher_enabled,
            schedule_enabled: request.schedule_enabled,
            schedule_interval_minutes: request.schedule_interval_minutes,
        })
        .await
        .map_err(|err| map_media_error("media_profile_patch", MEDIA_PROFILE_UPSERT_FAILED, &err))?;

    let profile = state
        .media
        .media_profile_list()
        .await
        .map_err(|err| map_media_error("media_profile_list", MEDIA_PROFILE_LIST_FAILED, &err))?
        .into_iter()
        .find(|item| item.media_profile_public_id == media_profile_public_id)
        .ok_or_else(|| ApiError::not_found(MEDIA_PROFILE_NOT_FOUND))?;

    Ok(Json(map_profile(profile)))
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

pub(crate) async fn get_media_profile(
    State(state): State<Arc<ApiState>>,
    Path(media_profile_public_id): Path<Uuid>,
) -> Result<Json<MediaProfileResponse>, ApiError> {
    let profile = state
        .media
        .media_profile_list()
        .await
        .map_err(|err| map_media_error("media_profile_list", MEDIA_PROFILE_LIST_FAILED, &err))?
        .into_iter()
        .find(|item| item.media_profile_public_id == media_profile_public_id)
        .ok_or_else(|| ApiError::not_found(MEDIA_PROFILE_NOT_FOUND))?;

    Ok(Json(map_profile(profile)))
}

pub(crate) async fn validate_media_profile(
    Json(request): Json<MediaProfileUpsertRequest>,
) -> Json<MediaProfileValidationResponse> {
    let issues = collect_profile_validation_issues(&request);
    Json(MediaProfileValidationResponse {
        valid: issues.is_empty(),
        issues,
    })
}

pub(crate) async fn list_media_compatibility_targets(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaCompatibilityTargetListResponse>, ApiError> {
    let targets = state
        .media
        .media_compatibility_target_list()
        .await
        .map_err(|err| {
            map_media_error(
                "media_compatibility_target_list",
                "failed to list media compatibility targets",
                &err,
            )
        })?
        .into_iter()
        .map(|target| MediaCompatibilityTargetResponse {
            compatibility_target_key: target.compatibility_target_key,
            version: target.version,
            display_name: target.display_name,
            video_codec: target.video_codec,
            audio_codec: target.audio_codec,
            subtitle_policy: target.subtitle_policy,
        })
        .collect();
    Ok(Json(MediaCompatibilityTargetListResponse { targets }))
}

pub(crate) async fn upsert_media_compatibility_target(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaCompatibilityTargetUpsertRequest>,
) -> Result<(StatusCode, Json<MediaCompatibilityTargetResponse>), ApiError> {
    validate_positive_version(request.version)?;
    let compatibility_target_key = normalize_required_str_field(
        &request.compatibility_target_key,
        COMPATIBILITY_TARGET_KEY_REQUIRED,
    )?;
    let display_name = normalize_required_str_field(&request.display_name, DISPLAY_NAME_REQUIRED)?;
    let video_codec = normalize_required_str_field(&request.video_codec, VIDEO_CODEC_REQUIRED)?;
    let audio_codec = normalize_required_str_field(&request.audio_codec, AUDIO_CODEC_REQUIRED)?;
    let subtitle_policy = normalize_subtitle_policy(&request.subtitle_policy)?;

    let target = state
        .media
        .media_compatibility_target_upsert(
            crate::app::media::MediaCompatibilityTargetUpsertParams {
                actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
                compatibility_target_key,
                version: request.version,
                display_name,
                video_codec,
                audio_codec,
                subtitle_policy: &subtitle_policy,
            },
        )
        .await
        .map_err(|err| {
            map_media_error(
                "media_compatibility_target_upsert",
                "failed to upsert media compatibility target",
                &err,
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(MediaCompatibilityTargetResponse {
            compatibility_target_key: target.compatibility_target_key,
            version: target.version,
            display_name: target.display_name,
            video_codec: target.video_codec,
            audio_codec: target.audio_codec,
            subtitle_policy: target.subtitle_policy,
        }),
    ))
}

pub(crate) async fn list_media_policies(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaPolicyListResponse>, ApiError> {
    let policies = state
        .media
        .media_policy_list()
        .await
        .map_err(|err| map_media_error("media_policy_list", "failed to list media policies", &err))?
        .into_iter()
        .map(|policy| MediaPolicyResponse {
            policy_key: policy.policy_key,
            version: policy.version,
            display_name: policy.display_name,
            video_intent: policy.video_intent,
        })
        .collect();
    Ok(Json(MediaPolicyListResponse { policies }))
}

pub(crate) async fn upsert_media_policy(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaPolicyUpsertRequest>,
) -> Result<(StatusCode, Json<MediaPolicyResponse>), ApiError> {
    validate_positive_version(request.version)?;
    let policy_key = normalize_required_str_field(&request.policy_key, POLICY_KEY_REQUIRED)?;
    let display_name = normalize_required_str_field(&request.display_name, DISPLAY_NAME_REQUIRED)?;
    let video_intent_input =
        normalize_required_str_field(&request.video_intent, VIDEO_INTENT_REQUIRED)?;
    let video_intent = normalize_video_intent(video_intent_input)?;

    let policy = state
        .media
        .media_policy_upsert(crate::app::media::MediaPolicyUpsertParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            policy_key,
            version: request.version,
            display_name,
            video_intent: &video_intent,
        })
        .await
        .map_err(|err| {
            map_media_error("media_policy_upsert", "failed to upsert media policy", &err)
        })?;

    Ok((
        StatusCode::CREATED,
        Json(MediaPolicyResponse {
            policy_key: policy.policy_key,
            version: policy.version,
            display_name: policy.display_name,
            video_intent: policy.video_intent,
        }),
    ))
}

pub(crate) async fn media_job_retention(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaJobRetentionResponse>, ApiError> {
    let retention = state.media.media_job_retention().await.map_err(|err| {
        map_media_error(
            "media_job_retention",
            "failed to read media job retention",
            &err,
        )
    })?;
    Ok(Json(MediaJobRetentionResponse {
        completed_retention_days: retention.completed_retention_days,
        failed_diagnostic_retention_days: retention.failed_diagnostic_retention_days,
    }))
}

pub(crate) async fn update_media_job_retention(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaJobRetentionUpdateRequest>,
) -> Result<Json<MediaJobRetentionResponse>, ApiError> {
    validate_retention_days(request.completed_retention_days)?;
    validate_retention_days(request.failed_diagnostic_retention_days)?;

    let retention = state
        .media
        .media_job_retention_update(crate::app::media::MediaJobRetentionUpdateParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            completed_retention_days: request.completed_retention_days,
            failed_diagnostic_retention_days: request.failed_diagnostic_retention_days,
        })
        .await
        .map_err(|err| {
            map_media_error(
                "media_job_retention_update",
                "failed to update media job retention",
                &err,
            )
        })?;

    Ok(Json(MediaJobRetentionResponse {
        completed_retention_days: retention.completed_retention_days,
        failed_diagnostic_retention_days: retention.failed_diagnostic_retention_days,
    }))
}

pub(crate) async fn preview_media_planning(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaPlanningPreviewRequest>,
) -> Result<Json<MediaPlanningPreviewResponse>, ApiError> {
    let source_path = normalize_required_str_field(&request.source_path, SOURCE_PATH_REQUIRED)?;
    let source_paths = vec![source_path.to_string()];
    let preview = state
        .media
        .media_discovery_preview(MediaDiscoveryPreviewParams {
            media_profile_public_id: request.media_profile_public_id,
            source_paths: &source_paths,
        })
        .await
        .map_err(|err| {
            map_media_error(
                "media_planning_preview",
                MEDIA_DISCOVERY_PREVIEW_FAILED,
                &err,
            )
        })?
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::bad_request(SOURCE_PATH_REQUIRED))?;

    Ok(Json(MediaPlanningPreviewResponse {
        accepted: preview.accepted,
        source_path: preview.source_path,
        output_path: preview.output_path,
        reason: preview.reason,
        dry_run: preview.dry_run,
    }))
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
            replace_confirmation: request.replace_confirmation.as_deref(),
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

pub(crate) async fn preview_media_discovery(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaDiscoveryPreviewRequest>,
) -> Result<Json<MediaDiscoveryPreviewResponse>, ApiError> {
    let source_paths = normalize_discovery_source_paths(&request.source_paths)?;

    let previews = state
        .media
        .media_discovery_preview(MediaDiscoveryPreviewParams {
            media_profile_public_id: request.media_profile_public_id,
            source_paths: &source_paths,
        })
        .await
        .map_err(|err| {
            map_media_error(
                "media_discovery_preview",
                MEDIA_DISCOVERY_PREVIEW_FAILED,
                &err,
            )
        })?
        .into_iter()
        .map(|item| MediaDiscoveryPreviewItemResponse {
            source_path: item.source_path,
            output_path: item.output_path,
            dry_run: item.dry_run,
            accepted: item.accepted,
            reason: item.reason,
        })
        .collect();

    Ok(Json(MediaDiscoveryPreviewResponse { previews }))
}

pub(crate) async fn run_media_discovery(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaDiscoveryRunRequest>,
) -> Result<(StatusCode, Json<MediaDiscoveryRunResponse>), ApiError> {
    run_media_discovery_for_trigger(state, request, MediaDiscoveryRunTrigger::Manual).await
}

pub(crate) async fn run_media_discovery_schedule(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaDiscoveryRunRequest>,
) -> Result<(StatusCode, Json<MediaDiscoveryRunResponse>), ApiError> {
    run_media_discovery_for_trigger(state, request, MediaDiscoveryRunTrigger::Schedule).await
}

#[derive(Debug, Clone, Copy)]
enum MediaDiscoveryRunTrigger {
    Manual,
    Schedule,
}

async fn run_media_discovery_for_trigger(
    state: Arc<ApiState>,
    request: MediaDiscoveryRunRequest,
    trigger: MediaDiscoveryRunTrigger,
) -> Result<(StatusCode, Json<MediaDiscoveryRunResponse>), ApiError> {
    let source_paths = normalize_discovery_source_paths(&request.source_paths)?;
    let response = match trigger {
        MediaDiscoveryRunTrigger::Manual => state
            .media
            .media_discovery_run(MediaDiscoveryRunParams {
                actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
                media_profile_public_id: request.media_profile_public_id,
                source_paths: &source_paths,
            })
            .await
            .map_err(|err| {
                map_media_error("media_discovery_run", MEDIA_DISCOVERY_RUN_FAILED, &err)
            })?,
        MediaDiscoveryRunTrigger::Schedule => state
            .media
            .media_discovery_schedule_run(MediaDiscoveryAutomationRunParams {
                actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
                media_profile_public_id: request.media_profile_public_id,
                source_paths: &source_paths,
            })
            .await
            .map_err(|err| {
                map_media_error(
                    "media_discovery_schedule_run",
                    MEDIA_DISCOVERY_SCHEDULE_RUN_FAILED,
                    &err,
                )
            })?,
    };

    Ok((
        StatusCode::CREATED,
        Json(map_discovery_run_response(response)),
    ))
}

fn normalize_discovery_source_paths(source_paths: &[String]) -> Result<Vec<String>, ApiError> {
    if source_paths.is_empty() {
        return Err(ApiError::bad_request(DISCOVERY_SOURCE_PATHS_REQUIRED));
    }

    if source_paths.len() > DISCOVERY_SOURCE_PATHS_MAX_LEN {
        let mut error = ApiError::bad_request(DISCOVERY_SOURCE_PATHS_TOO_LARGE);
        error = error.with_context_field("max_len", DISCOVERY_SOURCE_PATHS_MAX_LEN.to_string());
        return Err(error);
    }

    let mut normalized = Vec::new();
    for source_path in source_paths {
        let normalized_path = normalize_required_str_field(source_path, SOURCE_PATH_REQUIRED)?;
        if normalized_path.len() > DISCOVERY_SOURCE_PATH_MAX_BYTES {
            let mut error = ApiError::bad_request(DISCOVERY_SOURCE_PATH_TOO_LARGE);
            error =
                error.with_context_field("max_len", DISCOVERY_SOURCE_PATH_MAX_BYTES.to_string());
            return Err(error);
        }
        normalized.push(normalized_path.to_string());
    }
    Ok(normalized)
}

fn map_discovery_run_response(response: AppMediaDiscoveryRunResponse) -> MediaDiscoveryRunResponse {
    MediaDiscoveryRunResponse {
        queued_jobs: response
            .queued_jobs
            .into_iter()
            .map(|job| MediaDiscoveryQueuedJobResponse {
                media_job_public_id: job.media_job_public_id,
                source_path: job.source_path,
                output_path: job.output_path,
                dry_run: job.dry_run,
            })
            .collect(),
        skipped: response
            .skipped
            .into_iter()
            .map(|item| MediaDiscoverySkippedItemResponse {
                source_path: item.source_path,
                reason: item.reason,
            })
            .collect(),
    }
}

pub(crate) async fn list_media_discovery_schedules(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaDiscoveryScheduleListResponse>, ApiError> {
    let schedules = state
        .media
        .media_profile_list()
        .await
        .map_err(|err| {
            map_media_error(
                "media_discovery_schedule_list",
                MEDIA_DISCOVERY_SCHEDULE_LIST_FAILED,
                &err,
            )
        })?
        .into_iter()
        .map(|profile| MediaDiscoveryScheduleResponse {
            media_profile_public_id: profile.media_profile_public_id,
            profile_key: profile.profile_key,
            source_root: profile.source_root,
            enabled: profile.schedule_enabled,
            interval_minutes: profile.schedule_interval_minutes,
            dry_run: profile.dry_run_only,
        })
        .collect();

    Ok(Json(MediaDiscoveryScheduleListResponse { schedules }))
}

pub(crate) async fn list_media_discovery_watchers(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaDiscoveryWatcherListResponse>, ApiError> {
    let watchers = state
        .media
        .media_profile_list()
        .await
        .map_err(|err| {
            map_media_error(
                "media_discovery_watcher_list",
                MEDIA_DISCOVERY_WATCHER_LIST_FAILED,
                &err,
            )
        })?
        .into_iter()
        .map(|profile| MediaDiscoveryWatcherResponse {
            media_profile_public_id: profile.media_profile_public_id,
            profile_key: profile.profile_key,
            source_root: profile.source_root,
            enabled: false,
            dry_run: profile.dry_run_only,
        })
        .collect();

    Ok(Json(MediaDiscoveryWatcherListResponse { watchers }))
}

pub(crate) async fn list_media_jobs(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<MediaJobsQuery>,
) -> Result<Json<MediaJobListResponse>, ApiError> {
    let status = parse_media_status_optional(
        trim_and_filter_empty(query.status.as_deref()),
        MEDIA_STATUS_INVALID,
    )?;
    let Some(media_profile_public_id) = query.media_profile_public_id else {
        return Ok(Json(MediaJobListResponse { jobs: Vec::new() }));
    };
    let jobs = state
        .media
        .media_job_list(media_profile_public_id, status.as_deref())
        .await
        .map_err(|err| map_media_error("media_job_list", MEDIA_JOB_LIST_FAILED, &err))?
        .into_iter()
        .map(map_job)
        .collect();

    Ok(Json(MediaJobListResponse { jobs }))
}

pub(crate) async fn get_media_job(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<Json<MediaJobResponse>, ApiError> {
    let job = state
        .media
        .media_job_get(media_job_public_id)
        .await
        .map_err(|err| map_media_error("media_job_get", MEDIA_JOB_GET_FAILED, &err))?
        .ok_or_else(|| ApiError::not_found(MEDIA_JOB_GET_FAILED))?;

    Ok(Json(map_job(job)))
}

pub(crate) async fn cancel_media_job(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .media
        .media_job_cancel(media_job_public_id)
        .await
        .map_err(|err| map_media_error("media_job_cancel", MEDIA_JOB_CANCEL_FAILED, &err))?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn retry_media_job(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .media
        .media_job_retry(media_job_public_id)
        .await
        .map_err(|err| map_media_error("media_job_retry", MEDIA_JOB_RETRY_FAILED, &err))?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn append_media_job_phase(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
    Json(request): Json<MediaJobPhaseAppendRequest>,
) -> Result<StatusCode, ApiError> {
    let phase_name = normalize_required_str_field(&request.phase_name, PHASE_NAME_REQUIRED)?;
    let phase_status = normalize_required_str_field(&request.phase_status, PHASE_STATUS_REQUIRED)?;
    let phase_status = parse_media_status_required(phase_status, PHASE_STATUS_INVALID)?;

    state
        .media
        .media_job_phase_append(MediaJobPhaseAppendParams {
            media_job_public_id,
            phase_index: request.phase_index,
            phase_name,
            phase_status: phase_status.as_str(),
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

pub(crate) async fn list_media_job_operations(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<Json<MediaJobOperationListResponse>, ApiError> {
    let operations = state
        .media
        .media_job_operation_list(media_job_public_id)
        .await
        .map_err(|err| {
            map_media_error(
                "media_job_operation_list",
                MEDIA_JOB_OPERATION_LIST_FAILED,
                &err,
            )
        })?
        .into_iter()
        .map(|item| crate::models::MediaJobOperationResponse {
            operation_index: item.operation_index,
            operation_kind: item.operation_kind,
            stream_id: item.stream_id,
            command_bin: item.command_bin,
            arg_1: item.arg_1,
            arg_2: item.arg_2,
            arg_3: item.arg_3,
            arg_4: item.arg_4,
            arg_5: item.arg_5,
            created_at: item.created_at,
        })
        .collect();

    Ok(Json(MediaJobOperationListResponse { operations }))
}

pub(crate) async fn list_media_job_violations(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<Json<MediaJobViolationListResponse>, ApiError> {
    let violations = state
        .media
        .media_job_violation_list(media_job_public_id)
        .await
        .map_err(|err| {
            map_media_error(
                "media_job_violation_list",
                MEDIA_JOB_VIOLATION_LIST_FAILED,
                &err,
            )
        })?
        .into_iter()
        .map(|item| crate::models::MediaJobViolationResponse {
            violation_index: item.violation_index,
            violation_kind: item.violation_kind,
            severity: item.severity,
            stream_id: item.stream_id,
            created_at: item.created_at,
        })
        .collect();

    Ok(Json(MediaJobViolationListResponse { violations }))
}

pub(crate) async fn list_media_job_plan_reasons(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<Json<MediaJobPlanReasonListResponse>, ApiError> {
    let reasons = state
        .media
        .media_job_plan_reason_list(media_job_public_id)
        .await
        .map_err(|err| {
            map_media_error(
                "media_job_plan_reason_list",
                MEDIA_JOB_PLAN_REASON_LIST_FAILED,
                &err,
            )
        })?
        .into_iter()
        .map(|item| crate::models::MediaJobPlanReasonResponse {
            reason_index: item.reason_index,
            candidate_index: item.candidate_index,
            selected: item.selected,
            reason_code: item.reason_code,
            reason_text: item.reason_text,
            created_at: item.created_at,
        })
        .collect();

    Ok(Json(MediaJobPlanReasonListResponse { reasons }))
}

pub(crate) async fn list_media_job_verification_checks(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<Json<MediaJobVerificationCheckListResponse>, ApiError> {
    let checks = state
        .media
        .media_job_verification_check_list(media_job_public_id)
        .await
        .map_err(|err| {
            map_media_error(
                "media_job_verification_check_list",
                MEDIA_JOB_VERIFICATION_CHECK_LIST_FAILED,
                &err,
            )
        })?
        .into_iter()
        .map(|item| crate::models::MediaJobVerificationCheckResponse {
            check_index: item.check_index,
            check_kind: item.check_kind,
            check_status: item.check_status,
            expected_value: item.expected_value,
            actual_value: item.actual_value,
            details_text: item.details_text,
            created_at: item.created_at,
        })
        .collect();

    Ok(Json(MediaJobVerificationCheckListResponse { checks }))
}

pub(crate) async fn list_media_job_artifacts(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<Json<MediaJobArtifactListResponse>, ApiError> {
    let artifacts = state
        .media
        .media_job_artifact_list(media_job_public_id)
        .await
        .map_err(|err| {
            map_media_error(
                "media_job_artifact_list",
                MEDIA_JOB_ARTIFACT_LIST_FAILED,
                &err,
            )
        })?
        .into_iter()
        .map(|item| crate::models::MediaJobArtifactResponse {
            artifact_index: item.artifact_index,
            artifact_kind: item.artifact_kind,
            artifact_path: item.artifact_path,
            size_bytes: item.size_bytes,
            content_type: item.content_type,
            created_at: item.created_at,
        })
        .collect();

    Ok(Json(MediaJobArtifactListResponse { artifacts }))
}

pub(crate) async fn list_media_job_compact_audits(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<Json<MediaJobCompactAuditListResponse>, ApiError> {
    let audits = state
        .media
        .media_job_compact_audit_list(media_job_public_id)
        .await
        .map_err(|err| {
            map_media_error(
                "media_job_compact_audit_list",
                MEDIA_JOB_COMPACT_AUDIT_LIST_FAILED,
                &err,
            )
        })?
        .into_iter()
        .map(|item| crate::models::MediaJobCompactAuditResponse {
            audit_index: item.audit_index,
            fact_kind: item.fact_kind,
            fact_text: item.fact_text,
            created_at: item.created_at,
        })
        .collect();

    Ok(Json(MediaJobCompactAuditListResponse { audits }))
}

pub(crate) async fn refresh_media_capability(
    State(state): State<Arc<ApiState>>,
) -> Result<(StatusCode, Json<MediaCapabilityRefreshResponse>), ApiError> {
    let media_capability_snapshot_id = state
        .media
        .media_capability_refresh(MediaCapabilityRefreshParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
        })
        .await
        .map_err(|err| {
            map_media_error(
                "media_capability_refresh",
                MEDIA_CAPABILITY_REFRESH_FAILED,
                &err,
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(MediaCapabilityRefreshResponse {
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
        .map(map_capability_snapshot);

    Ok(Json(MediaCapabilityLatestResponse { snapshot }))
}

pub(crate) async fn media_capability_readiness(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaCapabilityReadinessResponse>, ApiError> {
    let readiness = state
        .media
        .media_capability_readiness()
        .await
        .map_err(|err| {
            map_media_error(
                "media_capability_readiness",
                MEDIA_CAPABILITY_READINESS_FAILED,
                &err,
            )
        })?;

    let snapshot = readiness.snapshot.map(map_capability_snapshot);

    Ok(Json(MediaCapabilityReadinessResponse {
        ready: readiness.ready,
        reason: readiness.reason,
        snapshot,
    }))
}

pub(crate) async fn media_compliance() -> Json<MediaComplianceResponse> {
    Json(MediaComplianceResponse {
        license_mode: MEDIA_LICENSE_MODE.to_string(),
        source_offer_path: MEDIA_SOURCE_OFFER_PATH.to_string(),
        third_party_notices_path: MEDIA_THIRD_PARTY_NOTICES_PATH.to_string(),
        sbom_path: MEDIA_SBOM_PATH.to_string(),
        inventory_path: MEDIA_INVENTORY_PATH.to_string(),
        exiftool_exception_path: MEDIA_EXIFTOOL_EXCEPTION_PATH.to_string(),
        license_excluded_capabilities: MEDIA_LICENSE_EXCLUDED_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_string())
            .collect(),
    })
}

pub(crate) async fn export_media_yaml(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<MediaYamlExportQuery>,
) -> Result<Json<MediaYamlExportResponse>, ApiError> {
    let yaml_payload = state
        .media
        .media_yaml_export(query.include_local_paths.unwrap_or(false))
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
        compatibility_target_key: profile.compatibility_target_key,
        policy_key: profile.policy_key,
        watcher_enabled: profile.watcher_enabled,
        schedule_enabled: profile.schedule_enabled,
        schedule_interval_minutes: profile.schedule_interval_minutes,
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

fn map_capability_snapshot(
    row: crate::app::media::MediaCapabilitySnapshotResponse,
) -> MediaCapabilitySnapshotResponse {
    MediaCapabilitySnapshotResponse {
        media_capability_snapshot_id: row.media_capability_snapshot_id,
        snapshot_run_public_id: row.snapshot_run_public_id,
        ffmpeg_version: row.ffmpeg_version,
        ffprobe_version: row.ffprobe_version,
        codecs: row
            .codecs
            .into_iter()
            .map(|codec| MediaCapabilityCodecResponse {
                codec_name: codec.codec_name,
                encode_supported: codec.encode_supported,
                decode_supported: codec.decode_supported,
            })
            .collect(),
        encoders: row.encoders,
        decoders: row.decoders,
        muxers: row.muxers,
        demuxers: row.demuxers,
        subtitle_support: row.subtitle_support,
        hardware_accelerators: row.hardware_accelerators,
        filesystem_utilities: row.filesystem_utilities,
        utility_capabilities: row.utility_capabilities,
        license_mode: row.license_mode,
        compliance_links: row.compliance_links,
        absent_capabilities: row.absent_capabilities,
        features: row
            .features
            .into_iter()
            .map(|feature| MediaCapabilityFeatureResponse {
                feature_family: feature.feature_family,
                feature_name: feature.feature_name,
                supported: feature.supported,
                detail_text: feature.detail_text,
            })
            .collect(),
        observed_at: row.observed_at,
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

fn validate_retention_days(value: i32) -> Result<(), ApiError> {
    if (1..=3650).contains(&value) {
        Ok(())
    } else {
        Err(ApiError::bad_request(RETENTION_DAYS_INVALID))
    }
}

fn validate_positive_version(value: i32) -> Result<(), ApiError> {
    if value > 0 {
        Ok(())
    } else {
        Err(ApiError::bad_request(VERSION_INVALID))
    }
}

fn normalize_subtitle_policy(value: &str) -> Result<String, ApiError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "selected" | "all" | "none" => Ok(normalized),
        _ => Err(ApiError::bad_request(SUBTITLE_POLICY_INVALID)),
    }
}

fn normalize_video_intent(value: &str) -> Result<String, ApiError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "general" | "anime" | "archival" => Ok(normalized),
        _ => Err(ApiError::bad_request(VIDEO_INTENT_INVALID)),
    }
}

fn collect_profile_validation_issues(request: &MediaProfileUpsertRequest) -> Vec<String> {
    let mut issues = Vec::new();
    if trim_and_filter_empty(Some(&request.profile_key)).is_none() {
        issues.push("media_profile_key_required".to_string());
    }
    if trim_and_filter_empty(Some(&request.source_root)).is_none() {
        issues.push("media_profile_source_root_required".to_string());
    }
    if trim_and_filter_empty(Some(&request.output_root)).is_none() {
        issues.push("media_profile_output_root_required".to_string());
    }
    if !(1..=3650).contains(&request.retention_days) {
        issues.push("media_profile_retention_days_out_of_bounds".to_string());
    }
    if request.schedule_enabled && request.schedule_interval_minutes.is_none() {
        issues.push("media_profile_schedule_interval_required".to_string());
    }
    if request
        .schedule_interval_minutes
        .is_some_and(|interval| !(1..=525_600).contains(&interval))
    {
        issues.push("media_profile_schedule_interval_out_of_bounds".to_string());
    }
    if request.watcher_enabled {
        issues.push("media_discovery_watcher_unavailable".to_string());
    }
    issues
}

fn validate_schedule(schedule_enabled: bool, interval: Option<i32>) -> Result<(), ApiError> {
    if schedule_enabled && interval.is_none() {
        return Err(ApiError::bad_request(SCHEDULE_INTERVAL_REQUIRED));
    }
    if let Some(interval) = interval {
        validate_schedule_interval(interval)?;
    }
    Ok(())
}

fn validate_schedule_interval(interval: i32) -> Result<(), ApiError> {
    if (1..=525_600).contains(&interval) {
        Ok(())
    } else {
        Err(ApiError::bad_request(SCHEDULE_INTERVAL_INVALID))
    }
}

fn validate_watcher_disabled(watcher_enabled: bool) -> Result<(), ApiError> {
    if watcher_enabled {
        Err(ApiError::bad_request(WATCHER_UNAVAILABLE))
    } else {
        Ok(())
    }
}

fn parse_media_status_required(value: &str, detail: &'static str) -> Result<String, ApiError> {
    let normalized = value.trim().to_ascii_lowercase();
    if is_supported_media_status(&normalized) {
        Ok(normalized)
    } else {
        Err(ApiError::bad_request(detail))
    }
}

fn parse_media_status_optional(
    value: Option<&str>,
    detail: &'static str,
) -> Result<Option<String>, ApiError> {
    value.map_or(Ok(None), |text| {
        parse_media_status_required(text, detail).map(Some)
    })
}

fn is_supported_media_status(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "queued" | "running" | "verifying" | "completed" | "failed" | "cancelled"
    )
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
    async fn media_configuration_reads_return_default_payloads() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;

        let Json(targets) = list_media_compatibility_targets(State(state.clone())).await?;
        assert!(targets.targets.is_empty());

        let Json(policies) = list_media_policies(State(state.clone())).await?;
        assert!(policies.policies.is_empty());

        let Json(retention) = media_job_retention(State(state)).await?;
        assert_eq!(retention.completed_retention_days, 30);
        assert_eq!(retention.failed_diagnostic_retention_days, 30);
        Ok(())
    }

    #[tokio::test]
    async fn media_configuration_writes_validate_request_shape() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;

        let target_request = MediaCompatibilityTargetUpsertRequest {
            compatibility_target_key: "plex-living-room".to_string(),
            version: 0,
            display_name: "Plex living room".to_string(),
            video_codec: "hevc".to_string(),
            audio_codec: "aac".to_string(),
            subtitle_policy: "selected".to_string(),
        };
        let target_error =
            upsert_media_compatibility_target(State(state.clone()), Json(target_request))
                .await
                .expect_err("non-positive target version should fail validation");
        assert_eq!(
            target_error.into_response().status(),
            StatusCode::BAD_REQUEST
        );

        let policy_request = MediaPolicyUpsertRequest {
            policy_key: "living-room".to_string(),
            version: 1,
            display_name: "Living room".to_string(),
            video_intent: "lossless".to_string(),
        };
        let policy_error = upsert_media_policy(State(state.clone()), Json(policy_request))
            .await
            .expect_err("unknown video intent should fail validation");
        assert_eq!(
            policy_error.into_response().status(),
            StatusCode::BAD_REQUEST
        );

        let retention_request = MediaJobRetentionUpdateRequest {
            completed_retention_days: 0,
            failed_diagnostic_retention_days: 30,
        };
        let retention_error = update_media_job_retention(State(state), Json(retention_request))
            .await
            .expect_err("retention day bounds should fail validation");
        assert_eq!(
            retention_error.into_response().status(),
            StatusCode::BAD_REQUEST
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_configuration_writes_map_default_facade_failures() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;

        let target_request = MediaCompatibilityTargetUpsertRequest {
            compatibility_target_key: "  plex-living-room  ".to_string(),
            version: 1,
            display_name: "  Plex living room  ".to_string(),
            video_codec: " HEVC ".to_string(),
            audio_codec: " AAC ".to_string(),
            subtitle_policy: " Selected ".to_string(),
        };
        let target_error =
            upsert_media_compatibility_target(State(state.clone()), Json(target_request))
                .await
                .expect_err("default media facade should reject target writes");
        assert_eq!(
            target_error.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        let policy_request = MediaPolicyUpsertRequest {
            policy_key: "  living-room  ".to_string(),
            version: 1,
            display_name: "  Living room  ".to_string(),
            video_intent: " General ".to_string(),
        };
        let policy_error = upsert_media_policy(State(state.clone()), Json(policy_request))
            .await
            .expect_err("default media facade should reject policy writes");
        assert_eq!(
            policy_error.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        let retention_request = MediaJobRetentionUpdateRequest {
            completed_retention_days: 45,
            failed_diagnostic_retention_days: 90,
        };
        let retention_error = update_media_job_retention(State(state), Json(retention_request))
            .await
            .expect_err("default media facade should reject retention writes");
        assert_eq!(
            retention_error.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_compliance_returns_release_artifact_links() -> anyhow::Result<()> {
        let Json(response) = media_compliance().await;

        assert_eq!(response.license_mode, "redistributable-gplv3-runtime");
        assert_eq!(
            response.source_offer_path,
            "/app/compliance/SOURCE-OFFER.txt"
        );
        assert_eq!(
            response.third_party_notices_path,
            "/app/compliance/THIRD-PARTY-NOTICES.md"
        );
        assert_eq!(
            response.sbom_path,
            "/app/compliance/media-runtime-inventory.spdx.json"
        );
        assert_eq!(
            response.inventory_path,
            "/app/compliance/media-runtime-inventory.spdx.json"
        );
        assert_eq!(
            response.exiftool_exception_path,
            "/app/compliance/exiftool-exception.md"
        );
        assert!(
            response
                .license_excluded_capabilities
                .iter()
                .any(|capability| capability == "--enable-nonfree")
        );
        assert!(
            response
                .license_excluded_capabilities
                .iter()
                .any(|capability| capability == "libfdk_aac")
        );
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
            replace_confirmation: None,
        };

        let err = create_media_job(State(state), Json(request))
            .await
            .expect_err("noop media facade should fail writes");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        Ok(())
    }

    #[tokio::test]
    async fn preview_media_discovery_rejects_empty_source_paths() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let request = crate::models::MediaDiscoveryPreviewRequest {
            media_profile_public_id: Uuid::new_v4(),
            source_paths: Vec::new(),
        };

        let err = preview_media_discovery(State(state), Json(request))
            .await
            .expect_err("empty discovery source paths should fail validation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn preview_media_discovery_rejects_too_many_source_paths() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let request = crate::models::MediaDiscoveryPreviewRequest {
            media_profile_public_id: Uuid::new_v4(),
            source_paths: vec!["/input/demo.mkv".to_string(); DISCOVERY_SOURCE_PATHS_MAX_LEN + 1],
        };

        let err = preview_media_discovery(State(state), Json(request))
            .await
            .expect_err("oversized discovery source path list should fail validation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), 64 * 1024).await?;
        let problem: ProblemDetails = serde_json::from_slice(&body)?;
        assert_eq!(
            problem.detail.as_deref(),
            Some(DISCOVERY_SOURCE_PATHS_TOO_LARGE)
        );
        let context = problem.context.unwrap_or_default();
        assert!(context.iter().any(|item| {
            item.name == "max_len" && item.value == DISCOVERY_SOURCE_PATHS_MAX_LEN.to_string()
        }));
        Ok(())
    }

    #[tokio::test]
    async fn run_media_discovery_rejects_empty_source_paths() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let request = crate::models::MediaDiscoveryRunRequest {
            media_profile_public_id: Uuid::new_v4(),
            source_paths: Vec::new(),
        };

        let err = run_media_discovery(State(state), Json(request))
            .await
            .expect_err("empty discovery source paths should fail validation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn run_media_discovery_rejects_too_long_source_path() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let oversized_path = format!("/{}", "a".repeat(DISCOVERY_SOURCE_PATH_MAX_BYTES));
        let request = crate::models::MediaDiscoveryRunRequest {
            media_profile_public_id: Uuid::new_v4(),
            source_paths: vec![oversized_path],
        };

        let err = run_media_discovery(State(state), Json(request))
            .await
            .expect_err("oversized discovery source path should fail validation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), 64 * 1024).await?;
        let problem: ProblemDetails = serde_json::from_slice(&body)?;
        assert_eq!(
            problem.detail.as_deref(),
            Some(DISCOVERY_SOURCE_PATH_TOO_LARGE)
        );
        let context = problem.context.unwrap_or_default();
        assert!(context.iter().any(|item| {
            item.name == "max_len" && item.value == DISCOVERY_SOURCE_PATH_MAX_BYTES.to_string()
        }));
        Ok(())
    }

    #[tokio::test]
    async fn run_media_discovery_schedule_rejects_empty_source_paths() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let request = crate::models::MediaDiscoveryRunRequest {
            media_profile_public_id: Uuid::new_v4(),
            source_paths: Vec::new(),
        };

        let err = run_media_discovery_schedule(State(state), Json(request))
            .await
            .expect_err("empty scheduled discovery source paths should fail validation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn list_media_discovery_schedules_returns_empty_payload_with_default_facade()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = list_media_discovery_schedules(State(state)).await?;
        assert!(response.schedules.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn list_media_discovery_watchers_returns_empty_payload_with_default_facade()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = list_media_discovery_watchers(State(state)).await?;
        assert!(response.watchers.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn get_media_profile_returns_not_found_with_default_facade() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;

        let err = get_media_profile(State(state), Path(Uuid::new_v4()))
            .await
            .expect_err("default facade should not contain requested profile");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), 64 * 1024).await?;
        let problem: ProblemDetails = serde_json::from_slice(&body)?;
        assert_eq!(problem.detail.as_deref(), Some("media profile not found"));
        Ok(())
    }

    #[tokio::test]
    async fn upsert_media_profile_rejects_retention_days_below_minimum() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let request = MediaProfileUpsertRequest {
            profile_key: "tv".to_string(),
            source_root: "/input/tv".to_string(),
            output_root: "/output/tv".to_string(),
            dry_run_only: true,
            retention_days: 0,
            compatibility_target_key: None,
            policy_key: "safe_dry_run".to_string(),
            watcher_enabled: false,
            schedule_enabled: false,
            schedule_interval_minutes: None,
        };

        let err = upsert_media_profile(State(state), Json(request))
            .await
            .expect_err("invalid retention should fail validation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn upsert_media_profile_rejects_retention_days_above_maximum() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let request = MediaProfileUpsertRequest {
            profile_key: "tv".to_string(),
            source_root: "/input/tv".to_string(),
            output_root: "/output/tv".to_string(),
            dry_run_only: true,
            retention_days: 3651,
            compatibility_target_key: None,
            policy_key: "safe_dry_run".to_string(),
            watcher_enabled: false,
            schedule_enabled: false,
            schedule_interval_minutes: None,
        };

        let err = upsert_media_profile(State(state), Json(request))
            .await
            .expect_err("invalid retention should fail validation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn list_media_jobs_rejects_invalid_status_filter() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let query = MediaJobsQuery {
            media_profile_public_id: Some(Uuid::new_v4()),
            status: Some("INVALID".to_string()),
        };

        let err = list_media_jobs(State(state), Query(query))
            .await
            .expect_err("invalid status should fail validation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn list_media_jobs_accepts_normalized_status_filter() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let query = MediaJobsQuery {
            media_profile_public_id: Some(Uuid::new_v4()),
            status: Some("  COMPLETED ".to_string()),
        };

        let Json(response) = list_media_jobs(State(state), Query(query)).await?;
        assert!(response.jobs.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn list_media_jobs_without_profile_filter_returns_empty_payload() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let query = MediaJobsQuery {
            media_profile_public_id: None,
            status: None,
        };

        let Json(response) = list_media_jobs(State(state), Query(query)).await?;
        assert!(response.jobs.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn get_media_job_returns_not_found_with_default_facade() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;

        let err = get_media_job(State(state), Path(Uuid::new_v4()))
            .await
            .expect_err("default facade should not contain requested job");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn cancel_media_job_maps_noop_storage_failure_to_internal() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;

        let err = cancel_media_job(State(state), Path(Uuid::new_v4()))
            .await
            .expect_err("noop media facade should fail cancellation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        Ok(())
    }

    #[tokio::test]
    async fn retry_media_job_maps_noop_storage_failure_to_internal() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;

        let err = retry_media_job(State(state), Path(Uuid::new_v4()))
            .await
            .expect_err("noop media facade should fail retry");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        Ok(())
    }

    #[tokio::test]
    async fn append_media_job_phase_rejects_invalid_phase_status() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let request = MediaJobPhaseAppendRequest {
            phase_index: 0,
            phase_name: "planning".to_string(),
            phase_status: "INVALID".to_string(),
            details_text: None,
        };

        let err = append_media_job_phase(State(state), Path(Uuid::new_v4()), Json(request))
            .await
            .expect_err("invalid phase status should fail validation");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn list_media_job_operations_returns_empty_payload_with_default_facade()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = list_media_job_operations(State(state), Path(Uuid::new_v4())).await?;
        assert!(response.operations.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn list_media_job_violations_returns_empty_payload_with_default_facade()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = list_media_job_violations(State(state), Path(Uuid::new_v4())).await?;
        assert!(response.violations.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn list_media_job_plan_reasons_returns_empty_payload_with_default_facade()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) =
            list_media_job_plan_reasons(State(state), Path(Uuid::new_v4())).await?;
        assert!(response.reasons.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn list_media_job_verification_checks_returns_empty_payload_with_default_facade()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) =
            list_media_job_verification_checks(State(state), Path(Uuid::new_v4())).await?;
        assert!(response.checks.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn list_media_job_artifacts_returns_empty_payload_with_default_facade()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = list_media_job_artifacts(State(state), Path(Uuid::new_v4())).await?;
        assert!(response.artifacts.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn list_media_job_compact_audits_returns_empty_payload_with_default_facade()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) =
            list_media_job_compact_audits(State(state), Path(Uuid::new_v4())).await?;
        assert!(response.audits.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn refresh_media_capability_maps_noop_storage_failure_to_internal() -> anyhow::Result<()>
    {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let err = refresh_media_capability(State(state))
            .await
            .expect_err("noop media facade should fail refresh");
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
    async fn media_capability_readiness_returns_missing_reason_without_snapshot()
    -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = media_capability_readiness(State(state)).await?;
        assert!(!response.ready);
        assert_eq!(
            response.reason.as_deref(),
            Some("media_capability_snapshot_missing")
        );
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

    #[test]
    fn parse_media_status_required_normalizes_case_and_whitespace() {
        let value = parse_media_status_required("  QUEUED  ", MEDIA_STATUS_INVALID);
        assert!(value.is_ok());
        let Ok(status) = value else {
            return;
        };
        assert_eq!(status, "queued");
    }

    #[test]
    fn parse_media_status_required_rejects_unknown_values() {
        let value = parse_media_status_required("mystery", MEDIA_STATUS_INVALID);
        let Err(error) = value else {
            panic!("expected invalid status to fail");
        };
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn parse_media_status_optional_normalizes_when_present() {
        let value = parse_media_status_optional(Some("  COMPLETED "), MEDIA_STATUS_INVALID);
        assert!(value.is_ok());
        let Ok(status) = value else {
            return;
        };
        assert_eq!(status.as_deref(), Some("completed"));
    }

    #[test]
    fn parse_media_status_required_accepts_all_supported_values() {
        let supported = [
            "queued",
            "running",
            "verifying",
            "completed",
            "failed",
            "cancelled",
        ];

        for status in supported {
            let value = parse_media_status_required(status, MEDIA_STATUS_INVALID);
            assert!(value.is_ok(), "expected status {status} to be accepted");
            let Ok(parsed) = value else {
                continue;
            };
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn parse_media_status_optional_treats_absent_value_as_none() {
        let value = parse_media_status_optional(None, MEDIA_STATUS_INVALID);
        assert!(value.is_ok());
        let Ok(status) = value else {
            return;
        };
        assert_eq!(status, None);
    }

    #[test]
    fn parse_media_status_optional_rejects_unknown_when_present() {
        let value = parse_media_status_optional(Some("unknown"), MEDIA_STATUS_INVALID);
        let Err(error) = value else {
            panic!("expected invalid status to fail");
        };
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validate_retention_days_rejects_out_of_bounds_values() {
        assert!(validate_retention_days(0).is_err());
        assert!(validate_retention_days(3651).is_err());
    }

    #[test]
    fn validate_retention_days_accepts_in_bounds_values() {
        assert!(validate_retention_days(1).is_ok());
        assert!(validate_retention_days(3650).is_ok());
    }
}
