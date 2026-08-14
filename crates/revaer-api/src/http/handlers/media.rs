//! Media profile, job, and capability endpoints.

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use revaer_events::Event as CoreEvent;
use revaer_media_core::{
    normalize::{
        audio_channel_count_for_layout,
        normalize_audio_channel_layout as normalize_supported_audio_channel_layout,
    },
    target::{
        MAX_CONTAINER_CHAPTER_METADATA_ENTRIES, MAX_CONTAINER_CHAPTER_METADATA_TOTAL_BYTES,
        MAX_CONTAINER_CHAPTERS, MAX_CONTAINER_METADATA_ENTRIES, MAX_CONTAINER_METADATA_KEY_BYTES,
        MAX_CONTAINER_METADATA_TOTAL_BYTES, MAX_CONTAINER_METADATA_VALUE_BYTES,
        MAX_DESIRED_TARGET_STREAMS, is_known_color_primaries, is_known_color_space,
        is_known_color_transfer, is_known_video_level,
    },
};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::media::{
    MediaCapabilityRefreshParams, MediaDesiredTargetChapterParams, MediaDesiredTargetCreateParams,
    MediaDesiredTargetMetadataParams, MediaDesiredTargetResponse as AppMediaDesiredTargetResponse,
    MediaDesiredTargetStreamParams, MediaDiscoveryAutomationRunParams, MediaDiscoveryPreviewParams,
    MediaDiscoveryRunParams, MediaDiscoveryRunResponse as AppMediaDiscoveryRunResponse,
    MediaProfileDesiredTargetParams, MediaProfilePatchParams, MediaProfileUpsertParams,
    MediaServiceError, MediaServiceErrorKind,
};
use crate::app::state::ApiState;
use crate::http::errors::ApiError;
use crate::http::handlers::indexers::SYSTEM_ACTOR_PUBLIC_ID;
use crate::models::{
    MediaCapabilityLatestResponse, MediaCapabilityReadinessResponse,
    MediaCapabilityRefreshResponse, MediaCompatibilityTargetListResponse,
    MediaCompatibilityTargetResponse, MediaCompatibilityTargetUpsertRequest,
    MediaComplianceResponse, MediaDesiredTargetChapterEntry, MediaDesiredTargetCreateRequest,
    MediaDesiredTargetListResponse, MediaDesiredTargetMetadataEntry, MediaDesiredTargetResponse,
    MediaDesiredTargetStream, MediaDiscoveryPreviewItemResponse, MediaDiscoveryPreviewRequest,
    MediaDiscoveryPreviewResponse, MediaDiscoveryQueuedJobResponse, MediaDiscoveryRunRequest,
    MediaDiscoveryRunResponse, MediaDiscoveryScheduleListResponse, MediaDiscoveryScheduleResponse,
    MediaDiscoverySkippedItemResponse, MediaDiscoveryWatcherListResponse,
    MediaDiscoveryWatcherResponse, MediaJobArtifactListResponse, MediaJobCompactAuditListResponse,
    MediaJobDiagnosticCounts, MediaJobDiagnosticsResponse, MediaJobListResponse,
    MediaJobOperationListResponse, MediaJobPhaseListResponse, MediaJobPlanReasonListResponse,
    MediaJobResponse, MediaJobRetentionResponse, MediaJobRetentionUpdateRequest,
    MediaJobVerificationCheckListResponse, MediaJobViolationListResponse,
    MediaPlanningPreviewRequest, MediaPlanningPreviewResponse, MediaPolicyListResponse,
    MediaPolicyResponse, MediaPolicyUpsertRequest, MediaProfileDesiredTargetRequest,
    MediaProfileListResponse, MediaProfilePatchRequest, MediaProfileReadinessResponse,
    MediaProfileResponse, MediaProfileUpsertRequest, MediaProfileValidationResponse,
    MediaRecentJobPageResponse, MediaRecentJobSummaryResponse, MediaTextValidationError,
    MediaYamlApplyResponse, MediaYamlExportResponse, MediaYamlImportRequest,
    MediaYamlIssueResponse, MediaYamlValidationResponse, validate_media_display,
    validate_media_key,
};

const MEDIA_PROFILE_UPSERT_FAILED: &str = "failed to upsert media profile";
const MEDIA_PROFILE_LIST_FAILED: &str = "failed to list media profiles";
const MEDIA_PROFILE_READINESS_FAILED: &str = "failed to determine media profile readiness";
const MEDIA_PROFILE_NOT_FOUND: &str = "media profile not found";
const MEDIA_DISCOVERY_PREVIEW_FAILED: &str = "failed to preview media discovery";
const MEDIA_DISCOVERY_RUN_FAILED: &str = "failed to run media discovery";
const MEDIA_DISCOVERY_SCHEDULE_RUN_FAILED: &str = "failed to run scheduled media discovery";
const MEDIA_DISCOVERY_SCHEDULE_LIST_FAILED: &str = "failed to list media discovery schedules";
const MEDIA_DISCOVERY_WATCHER_RUN_FAILED: &str = "failed to run watcher media discovery";
const MEDIA_DISCOVERY_WATCHER_LIST_FAILED: &str = "failed to list media discovery watchers";
const MEDIA_JOB_LIST_FAILED: &str = "failed to list media jobs";
const MEDIA_JOB_GET_FAILED: &str = "failed to load media job";
const MEDIA_JOB_RECENT_FAILED: &str = "failed to list recent media jobs";
const MEDIA_JOB_DIAGNOSTICS_FAILED: &str = "failed to load media job diagnostics";
const MEDIA_JOB_DIAGNOSTIC_LIMIT: usize = 1_024;
const MEDIA_JOB_CANCEL_FAILED: &str = "failed to cancel media job";
const MEDIA_JOB_RETRY_FAILED: &str = "failed to retry media job";
const MEDIA_JOB_PHASE_LIST_FAILED: &str = "failed to list media job phases";
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
const SOURCE_ROOT_REQUIRED: &str = "source_root is required";
const OUTPUT_ROOT_REQUIRED: &str = "output_root is required";
const SOURCE_PATH_REQUIRED: &str = "source_path is required";
const CONTAINER_FORMAT_REQUIRED: &str = "container_format is required";
const CONTAINER_METADATA_POLICY_INVALID: &str =
    "container_metadata_policy must be preserve, strip, or replace";
const CONTAINER_METADATA_INVALID: &str =
    "container_metadata must be non-empty only when container_metadata_policy is replace";
const CONTAINER_CHAPTER_POLICY_INVALID: &str =
    "container_chapter_policy must be preserve, strip, or replace";
const CONTAINER_CHAPTERS_INVALID: &str =
    "container_chapters must be non-empty only when container_chapter_policy is replace";
const DESIRED_TARGET_STREAMS_REQUIRED: &str = "streams must contain at least one stream";
const VIDEO_CODEC_REQUIRED: &str = "video_codec is required";
const AUDIO_CODEC_REQUIRED: &str = "audio_codec is required";
const VIDEO_INTENT_REQUIRED: &str = "video_intent is required";
const DISCOVERY_SOURCE_PATHS_REQUIRED: &str = "source_paths must contain at least one path";
const DISCOVERY_SOURCE_PATHS_TOO_LARGE: &str = "source_paths exceeds maximum size";
const DISCOVERY_SOURCE_PATH_TOO_LARGE: &str = "source_path exceeds maximum size";
const YAML_PAYLOAD_REQUIRED: &str = "yaml_payload is required";
const RETENTION_DAYS_INVALID: &str = "retention_days must be between 1 and 3650";
const RETENTION_LIMIT_INVALID: &str = "retention limit must be between 1 and 3650";
const RETENTION_MODE_INVALID: &str = "retention mode must be one of: age, count";
const VERSION_INVALID: &str = "version must be greater than zero";
const AUDIO_CHANNELS_INVALID: &str = "audio_channels must be greater than zero";
const AUDIO_CHANNEL_LAYOUT_INVALID: &str = "audio_channel_layout must not be empty";
const SUBTITLE_POLICY_INVALID: &str = "subtitle_policy must be one of: selected, all, none";
const VIDEO_INTENT_INVALID: &str = "video_intent must be one of: general, anime, archival";
const VERIFICATION_STRICTNESS_REQUIRED: &str = "verification_strictness is required";
const VERIFICATION_STRICTNESS_INVALID: &str =
    "verification_strictness must be one of: strict, balanced, fast";
const VERIFICATION_DURATION_TOLERANCE_INVALID: &str =
    "verification_duration_tolerance_millis must be between 0 and 60000";
const SCHEDULE_INTERVAL_INVALID: &str = "schedule_interval_minutes must be between 1 and 525600";
const SCHEDULE_INTERVAL_REQUIRED: &str =
    "schedule_interval_minutes is required when schedule is enabled";
const MEDIA_STATUS_INVALID: &str =
    "status must be one of: queued, running, verifying, completed, failed, cancelled";
const MEDIA_LICENSE_MODE: &str = "redistributable-gplv3-runtime";
const MEDIA_SOURCE_OFFER_PATH: &str = "/app/compliance/SOURCE-OFFER.txt";
const MEDIA_SOURCE_OFFER_URL: &str = "/app/compliance/SOURCE-OFFER.txt";
const MEDIA_THIRD_PARTY_NOTICES_PATH: &str = "/app/compliance/THIRD-PARTY-NOTICES.md";
const MEDIA_THIRD_PARTY_NOTICES_URL: &str = "/app/compliance/THIRD-PARTY-NOTICES.md";
const MEDIA_SBOM_PATH: &str = "/app/compliance/media-runtime-inventory.spdx.json";
const MEDIA_SBOM_URL: &str = "/app/compliance/media-runtime-inventory.spdx.json";
const MEDIA_INVENTORY_PATH: &str = "/app/compliance/media-runtime-inventory.spdx.json";
const MEDIA_EXIFTOOL_EXCEPTION_PATH: &str = "/app/compliance/exiftool-exception.md";
const MEDIA_SOURCE_COMPLIANCE_BUNDLE_PATH: &str =
    "/app/compliance/final-image-compliance-bundle.json";
const MEDIA_SOURCE_COMPLIANCE_BUNDLE_DIGEST_UNAVAILABLE: &str =
    "unavailable-until-final-image-bundle-is-present";
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
pub(crate) struct MediaRecentJobsQuery {
    limit: Option<i32>,
    cursor: Option<String>,
    media_profile_public_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MediaYamlExportQuery {
    include_local_paths: Option<bool>,
}

pub(crate) async fn upsert_media_profile(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaProfileUpsertRequest>,
) -> Result<(StatusCode, Json<MediaProfileResponse>), ApiError> {
    let profile_key = normalize_media_key(&request.profile_key, "profile_key")?;
    let source_root = normalize_required_str_field(&request.source_root, SOURCE_ROOT_REQUIRED)?;
    let output_root = normalize_required_str_field(&request.output_root, OUTPUT_ROOT_REQUIRED)?;
    validate_retention_days(request.retention_days)?;
    validate_schedule(request.schedule_enabled, request.schedule_interval_minutes)?;

    let profile_id = state
        .media
        .media_profile_upsert(MediaProfileUpsertParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            profile_key,
            source_root,
            output_root,
            dry_run_only: true,
            retention_days: request.retention_days,
            compatibility_target_key: normalize_optional_media_key(
                request.compatibility_target_key.as_deref(),
                "compatibility_target_key",
            )?,
            policy_key: normalize_media_key(&request.policy_key, "policy_key")?,
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

    let response = map_profile(profile);
    state.publish_event(CoreEvent::MediaProfileChanged {
        media_profile_public_id: response.media_profile_public_id,
        profile_key: response.profile_key.clone(),
    });

    Ok((StatusCode::CREATED, Json(response)))
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
            compatibility_target_key: normalize_optional_media_key(
                request.compatibility_target_key.as_deref(),
                "compatibility_target_key",
            )?,
            policy_key: normalize_optional_media_key(request.policy_key.as_deref(), "policy_key")?,
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

    let response = map_profile(profile);
    state.publish_event(CoreEvent::MediaProfileChanged {
        media_profile_public_id: response.media_profile_public_id,
        profile_key: response.profile_key.clone(),
    });

    Ok(Json(response))
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

pub(crate) async fn get_media_profile_readiness(
    State(state): State<Arc<ApiState>>,
    Path(media_profile_public_id): Path<Uuid>,
) -> Result<Json<MediaProfileReadinessResponse>, ApiError> {
    let readiness = state
        .media
        .media_profile_readiness(media_profile_public_id)
        .await
        .map_err(|err| {
            map_media_error(
                "media_profile_readiness",
                MEDIA_PROFILE_READINESS_FAILED,
                &err,
            )
        })?
        .ok_or_else(|| ApiError::not_found(MEDIA_PROFILE_NOT_FOUND))?;

    Ok(Json(readiness))
}

pub(crate) async fn validate_media_profile(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaProfileUpsertRequest>,
) -> Result<Json<MediaProfileValidationResponse>, ApiError> {
    let mut issues = collect_profile_validation_issues(&request);
    append_profile_catalog_issues(
        &state,
        request.compatibility_target_key.as_deref(),
        Some(request.policy_key.as_str()),
        "media_profile_compatibility_target_not_found",
        "media_profile_policy_profile_not_found",
        &mut issues,
    )
    .await?;
    Ok(Json(MediaProfileValidationResponse {
        valid: issues.is_empty(),
        issues,
    }))
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
            audio_channels: target.audio_channels,
            audio_channel_layout: target.audio_channel_layout,
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
    let compatibility_target_key = normalize_media_key(
        &request.compatibility_target_key,
        "compatibility_target_key",
    )?;
    let display_name = normalize_media_display(&request.display_name, "display_name")?;
    let video_codec = normalize_required_str_field(&request.video_codec, VIDEO_CODEC_REQUIRED)?;
    let audio_codec = normalize_required_str_field(&request.audio_codec, AUDIO_CODEC_REQUIRED)?;
    let audio_channels = validate_audio_channels(request.audio_channels)?;
    let audio_channel_layout =
        normalize_audio_channel_layout(request.audio_channel_layout.as_deref())?;
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
                audio_channels,
                audio_channel_layout: audio_channel_layout.as_deref(),
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
            audio_channels: target.audio_channels,
            audio_channel_layout: target.audio_channel_layout,
            subtitle_policy: target.subtitle_policy,
        }),
    ))
}

pub(crate) async fn list_media_desired_targets(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaDesiredTargetListResponse>, ApiError> {
    let targets = state
        .media
        .media_desired_target_list()
        .await
        .map_err(|err| {
            map_media_error(
                "media_desired_target_list",
                "failed to list media desired targets",
                &err,
            )
        })?
        .into_iter()
        .map(map_desired_target_response)
        .collect();
    Ok(Json(MediaDesiredTargetListResponse { targets }))
}

pub(crate) async fn create_media_desired_target(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaDesiredTargetCreateRequest>,
) -> Result<(StatusCode, Json<MediaDesiredTargetResponse>), ApiError> {
    validate_positive_version(request.version)?;
    let target_key = normalize_media_key(&request.target_key, "target_key")?;
    let display_name = normalize_media_display(&request.display_name, "display_name")?;
    let container_format =
        normalize_required_str_field(&request.container_format, CONTAINER_FORMAT_REQUIRED)?;
    let container_metadata_policy =
        normalize_container_metadata_policy(request.container_metadata_policy.as_deref())?;
    let container_metadata =
        normalize_container_metadata(&container_metadata_policy, &request.container_metadata)?;
    let container_chapter_policy =
        normalize_container_chapter_policy(request.container_chapter_policy.as_deref())?;
    let container_chapters =
        normalize_container_chapters(&container_chapter_policy, &request.container_chapters)?;
    validate_desired_target_streams(&request.streams)?;
    let params = MediaDesiredTargetCreateParams {
        actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
        target_key: target_key.to_string(),
        version: request.version,
        display_name: display_name.to_string(),
        container_format: container_format.to_ascii_lowercase(),
        container_metadata_policy,
        container_metadata,
        container_chapter_policy,
        container_chapters,
        streams: request
            .streams
            .iter()
            .map(map_desired_target_stream_params)
            .collect(),
    };
    let target = state
        .media
        .media_desired_target_create(params)
        .await
        .map_err(|err| {
            map_media_error(
                "media_desired_target_create",
                "failed to create media desired target",
                &err,
            )
        })?;
    Ok((
        StatusCode::CREATED,
        Json(map_desired_target_response(target)),
    ))
}

pub(crate) async fn set_media_profile_desired_target(
    State(state): State<Arc<ApiState>>,
    Path(media_profile_public_id): Path<Uuid>,
    Json(request): Json<MediaProfileDesiredTargetRequest>,
) -> Result<StatusCode, ApiError> {
    let target_key = normalize_optional_media_key(request.target_key.as_deref(), "target_key")?
        .map(str::to_string);
    match (&target_key, request.version) {
        (Some(_), Some(version)) => validate_positive_version(version)?,
        (None, None) => {}
        _ => {
            return Err(ApiError::bad_request(
                "target_key and version must be supplied together",
            ));
        }
    }
    state
        .media
        .media_profile_desired_target_set(MediaProfileDesiredTargetParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            media_profile_public_id,
            target_key,
            version: request.version,
        })
        .await
        .map_err(|err| {
            map_media_error(
                "media_profile_desired_target_set",
                "failed to set media profile desired target",
                &err,
            )
        })?;
    Ok(StatusCode::NO_CONTENT)
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
            verification_strictness: policy.verification_strictness,
            verification_duration_tolerance_millis: policy.verification_duration_tolerance_millis,
            verification_mux_validation: policy.verification_mux_validation,
            verification_decode_all_streams: policy.verification_decode_all_streams,
            verification_keyframe_seek: policy.verification_keyframe_seek,
            verification_playback_probe: policy.verification_playback_probe,
        })
        .collect();
    Ok(Json(MediaPolicyListResponse { policies }))
}

pub(crate) async fn upsert_media_policy(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaPolicyUpsertRequest>,
) -> Result<(StatusCode, Json<MediaPolicyResponse>), ApiError> {
    validate_positive_version(request.version)?;
    let policy_key = normalize_media_key(&request.policy_key, "policy_key")?;
    let display_name = normalize_media_display(&request.display_name, "display_name")?;
    let video_intent_input =
        normalize_required_str_field(&request.video_intent, VIDEO_INTENT_REQUIRED)?;
    let video_intent = normalize_video_intent(video_intent_input)?;
    let verification_strictness_input = normalize_required_str_field(
        &request.verification_strictness,
        VERIFICATION_STRICTNESS_REQUIRED,
    )?;
    let verification_strictness = normalize_verification_strictness(verification_strictness_input)?;
    validate_verification_duration_tolerance(request.verification_duration_tolerance_millis)?;
    validate_verification_check_selection(&request, &verification_strictness)?;

    let policy = state
        .media
        .media_policy_upsert(crate::app::media::MediaPolicyUpsertParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            policy_key,
            version: request.version,
            display_name,
            video_intent: &video_intent,
            verification_strictness: &verification_strictness,
            verification_duration_tolerance_millis: request.verification_duration_tolerance_millis,
            verification_mux_validation: request.verification_mux_validation,
            verification_decode_all_streams: request.verification_decode_all_streams,
            verification_keyframe_seek: request.verification_keyframe_seek,
            verification_playback_probe: request.verification_playback_probe,
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
            verification_strictness: policy.verification_strictness,
            verification_duration_tolerance_millis: policy.verification_duration_tolerance_millis,
            verification_mux_validation: policy.verification_mux_validation,
            verification_decode_all_streams: policy.verification_decode_all_streams,
            verification_keyframe_seek: policy.verification_keyframe_seek,
            verification_playback_probe: policy.verification_playback_probe,
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
        completed_enabled: retention.completed_enabled,
        completed_mode: retention.completed_mode,
        completed_limit: retention.completed_limit,
        failed_diagnostic_enabled: retention.failed_diagnostic_enabled,
        failed_diagnostic_mode: retention.failed_diagnostic_mode,
        failed_diagnostic_limit: retention.failed_diagnostic_limit,
    }))
}

pub(crate) async fn update_media_job_retention(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaJobRetentionUpdateRequest>,
) -> Result<Json<MediaJobRetentionResponse>, ApiError> {
    let completed_mode = validate_retention_mode(&request.completed_mode)?;
    let failed_diagnostic_mode = validate_retention_mode(&request.failed_diagnostic_mode)?;
    validate_retention_limit(request.completed_limit)?;
    validate_retention_limit(request.failed_diagnostic_limit)?;

    let retention = state
        .media
        .media_job_retention_update(crate::app::media::MediaJobRetentionUpdateParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
            completed_enabled: request.completed_enabled,
            completed_mode,
            completed_limit: request.completed_limit,
            failed_diagnostic_enabled: request.failed_diagnostic_enabled,
            failed_diagnostic_mode,
            failed_diagnostic_limit: request.failed_diagnostic_limit,
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
        completed_enabled: retention.completed_enabled,
        completed_mode: retention.completed_mode,
        completed_limit: retention.completed_limit,
        failed_diagnostic_enabled: retention.failed_diagnostic_enabled,
        failed_diagnostic_mode: retention.failed_diagnostic_mode,
        failed_diagnostic_limit: retention.failed_diagnostic_limit,
    }))
}

fn validate_retention_mode(value: &str) -> Result<&'static str, ApiError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "age" => Ok("age"),
        "count" => Ok("count"),
        _ => Err(ApiError::bad_request(RETENTION_MODE_INVALID)),
    }
}

fn validate_retention_limit(value: i32) -> Result<(), ApiError> {
    if (1..=3650).contains(&value) {
        Ok(())
    } else {
        Err(ApiError::bad_request(RETENTION_LIMIT_INVALID))
    }
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
        })?;
    let accepted_count = previews.iter().filter(|item| item.accepted).count();
    state.publish_event(CoreEvent::MediaDiscoveryPreviewed {
        media_profile_public_id: request.media_profile_public_id,
        candidate_count: usize_to_u64_saturating(previews.len()),
        accepted_count: usize_to_u64_saturating(accepted_count),
    });
    let previews = previews
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

pub(crate) async fn run_media_discovery_watcher(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<MediaDiscoveryRunRequest>,
) -> Result<(StatusCode, Json<MediaDiscoveryRunResponse>), ApiError> {
    run_media_discovery_for_trigger(state, request, MediaDiscoveryRunTrigger::Watcher).await
}

#[derive(Debug, Clone, Copy)]
enum MediaDiscoveryRunTrigger {
    Manual,
    Schedule,
    Watcher,
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
        MediaDiscoveryRunTrigger::Watcher => state
            .media
            .media_discovery_watcher_run(MediaDiscoveryAutomationRunParams {
                actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
                media_profile_public_id: request.media_profile_public_id,
                source_paths: &source_paths,
            })
            .await
            .map_err(|err| {
                map_media_error(
                    "media_discovery_watcher_run",
                    MEDIA_DISCOVERY_WATCHER_RUN_FAILED,
                    &err,
                )
            })?,
    };
    for job in &response.queued_jobs {
        state.publish_event(CoreEvent::MediaJobQueued {
            media_job_public_id: job.media_job_public_id,
            media_profile_public_id: request.media_profile_public_id,
            dry_run: job.dry_run,
        });
    }

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
            enabled: profile.watcher_enabled,
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

pub(crate) async fn list_recent_media_jobs(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<MediaRecentJobsQuery>,
) -> Result<Json<MediaRecentJobPageResponse>, ApiError> {
    let limit = query.limit.unwrap_or(10);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::bad_request("limit must be between 1 and 100"));
    }
    let cursor = query
        .cursor
        .as_deref()
        .map(decode_recent_job_cursor)
        .transpose()?;
    let page = state
        .media
        .media_job_recent(limit, cursor, query.media_profile_public_id)
        .await
        .map_err(|err| map_media_error("media_job_recent", MEDIA_JOB_RECENT_FAILED, &err))?;
    let jobs = page
        .jobs
        .into_iter()
        .map(|summary| MediaRecentJobSummaryResponse {
            job: map_job(summary.job),
            media_profile_public_id: summary.media_profile_public_id,
            diagnostic_counts: MediaJobDiagnosticCounts {
                operations: summary.operation_count,
                violations: summary.violation_count,
                plan_reasons: summary.plan_reason_count,
                verification_checks: summary.verification_check_count,
                artifacts: summary.artifact_count,
                compact_audits: summary.compact_audit_count,
            },
        })
        .collect();
    Ok(Json(MediaRecentJobPageResponse {
        jobs,
        next_cursor: page.next_cursor.map(encode_recent_job_cursor),
    }))
}

pub(crate) async fn get_media_job_diagnostics(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<Json<MediaJobDiagnosticsResponse>, ApiError> {
    if state
        .media
        .media_job_get(media_job_public_id)
        .await
        .map_err(|err| map_media_error("media_job_get", MEDIA_JOB_GET_FAILED, &err))?
        .is_none()
    {
        return Err(ApiError::not_found(MEDIA_JOB_GET_FAILED));
    }
    let (operations, violations, plan_reasons, verification_checks, artifacts, compact_audits) =
        tokio::try_join!(
            state.media.media_job_operation_list(media_job_public_id),
            state.media.media_job_violation_list(media_job_public_id),
            state.media.media_job_plan_reason_list(media_job_public_id),
            state
                .media
                .media_job_verification_check_list(media_job_public_id),
            state.media.media_job_artifact_list(media_job_public_id),
            state
                .media
                .media_job_compact_audit_list(media_job_public_id),
        )
        .map_err(|err| {
            map_media_error("media_job_diagnostics", MEDIA_JOB_DIAGNOSTICS_FAILED, &err)
        })?;
    if [
        operations.len(),
        violations.len(),
        plan_reasons.len(),
        verification_checks.len(),
        artifacts.len(),
        compact_audits.len(),
    ]
    .into_iter()
    .any(|count| count > MEDIA_JOB_DIAGNOSTIC_LIMIT)
    {
        return Err(ApiError::internal(
            "media job diagnostic collection exceeds limit",
        ));
    }
    Ok(Json(MediaJobDiagnosticsResponse {
        operations,
        violations,
        plan_reasons,
        verification_checks,
        artifacts,
        compact_audits,
    }))
}

fn encode_recent_job_cursor((queued_at, id): (DateTime<Utc>, Uuid)) -> String {
    URL_SAFE_NO_PAD.encode(format!("{}|{id}", queued_at.to_rfc3339()))
}

fn decode_recent_job_cursor(cursor: &str) -> Result<(DateTime<Utc>, Uuid), ApiError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| ApiError::bad_request("cursor is invalid"))?;
    let text =
        std::str::from_utf8(&decoded).map_err(|_| ApiError::bad_request("cursor is invalid"))?;
    let (timestamp, id) = text
        .split_once('|')
        .ok_or_else(|| ApiError::bad_request("cursor is invalid"))?;
    Ok((
        timestamp
            .parse::<DateTime<Utc>>()
            .map_err(|_| ApiError::bad_request("cursor is invalid"))?,
        id.parse()
            .map_err(|_| ApiError::bad_request("cursor is invalid"))?,
    ))
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

pub(crate) async fn list_media_job_phases(
    State(state): State<Arc<ApiState>>,
    Path(media_job_public_id): Path<Uuid>,
) -> Result<Json<MediaJobPhaseListResponse>, ApiError> {
    let phases = state
        .media
        .media_job_phase_list(media_job_public_id)
        .await
        .map_err(|err| map_media_error("media_job_phase_list", MEDIA_JOB_PHASE_LIST_FAILED, &err))?
        .into_iter()
        .collect();

    Ok(Json(MediaJobPhaseListResponse { phases }))
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
        .collect();

    Ok(Json(MediaJobCompactAuditListResponse { audits }))
}

pub(crate) async fn refresh_media_capability(
    State(state): State<Arc<ApiState>>,
) -> Result<(StatusCode, Json<MediaCapabilityRefreshResponse>), ApiError> {
    let media_capability_snapshot_id = match state
        .media
        .media_capability_refresh(MediaCapabilityRefreshParams {
            actor_user_public_id: SYSTEM_ACTOR_PUBLIC_ID,
        })
        .await
    {
        Ok(snapshot_id) => snapshot_id,
        Err(err) => {
            state.publish_event(CoreEvent::MediaCapabilitiesRefreshFailed {
                reason: err
                    .code()
                    .unwrap_or("media_capability_refresh_failed")
                    .to_string(),
            });
            return Err(map_media_error(
                "media_capability_refresh",
                MEDIA_CAPABILITY_REFRESH_FAILED,
                &err,
            ));
        }
    };
    state.publish_event(CoreEvent::MediaCapabilitiesRefreshed {
        media_capability_snapshot_id,
    });

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
    let snapshot = state.media.media_capability_latest().await.map_err(|err| {
        map_media_error(
            "media_capability_latest",
            MEDIA_CAPABILITY_LATEST_FAILED,
            &err,
        )
    })?;

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

    Ok(Json(readiness))
}

pub(crate) async fn media_compliance(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<MediaComplianceResponse>, ApiError> {
    let snapshot = state.media.media_capability_latest().await.map_err(|err| {
        map_media_error(
            "media_capability_latest",
            MEDIA_CAPABILITY_LATEST_FAILED,
            &err,
        )
    })?;
    let ffmpeg_license_mode = snapshot
        .as_ref()
        .map(|item| item.ffmpeg_license_mode.trim())
        .filter(|mode| !mode.is_empty())
        .unwrap_or("unknown")
        .to_string();
    let ffmpeg_enable_gpl = snapshot.as_ref().is_some_and(|item| item.ffmpeg_enable_gpl);
    let ffmpeg_enable_version3 = snapshot
        .as_ref()
        .is_some_and(|item| item.ffmpeg_enable_version3);
    let ffmpeg_enable_nonfree = snapshot
        .as_ref()
        .is_some_and(|item| item.ffmpeg_enable_nonfree);
    let absent_license_excluded_capabilities = snapshot
        .as_ref()
        .map(|item| {
            item.absent_capabilities
                .iter()
                .filter(|capability| {
                    MEDIA_LICENSE_EXCLUDED_CAPABILITIES
                        .iter()
                        .any(|excluded| capability.eq_ignore_ascii_case(excluded))
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(Json(MediaComplianceResponse {
        license_mode: MEDIA_LICENSE_MODE.to_string(),
        image_license_mode: MEDIA_LICENSE_MODE.to_string(),
        ffmpeg_license_mode,
        ffmpeg_enable_gpl,
        ffmpeg_enable_version3,
        ffmpeg_enable_nonfree,
        source_offer_path: MEDIA_SOURCE_OFFER_PATH.to_string(),
        source_offer_url: MEDIA_SOURCE_OFFER_URL.to_string(),
        third_party_notices_path: MEDIA_THIRD_PARTY_NOTICES_PATH.to_string(),
        third_party_notices_url: MEDIA_THIRD_PARTY_NOTICES_URL.to_string(),
        sbom_path: MEDIA_SBOM_PATH.to_string(),
        sbom_url: MEDIA_SBOM_URL.to_string(),
        inventory_path: MEDIA_INVENTORY_PATH.to_string(),
        exiftool_exception_path: MEDIA_EXIFTOOL_EXCEPTION_PATH.to_string(),
        source_compliance_bundle_digest: source_compliance_bundle_digest(),
        source_compliance_bundle_path: MEDIA_SOURCE_COMPLIANCE_BUNDLE_PATH.to_string(),
        license_excluded_capabilities: MEDIA_LICENSE_EXCLUDED_CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_string())
            .collect(),
        absent_license_excluded_capabilities,
    }))
}

fn source_compliance_bundle_digest() -> String {
    std::fs::read_to_string(MEDIA_SOURCE_COMPLIANCE_BUNDLE_PATH)
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok())
        .and_then(|manifest| {
            manifest
                .get("source_compliance_sha256")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|digest| !digest.is_empty())
                .map(|digest| format!("sha256:{digest}"))
        })
        .unwrap_or_else(|| MEDIA_SOURCE_COMPLIANCE_BUNDLE_DIGEST_UNAVAILABLE.to_string())
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
        version: "1".to_string(),
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
        issues: result
            .issues
            .into_iter()
            .map(|issue| MediaYamlIssueResponse {
                code: issue.code,
                pointer: issue.pointer,
                blocking: issue.blocking,
            })
            .collect(),
        profile_count: result.bundle.profiles.len(),
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
            media_profile_import_draft_public_ids: result.media_profile_import_draft_public_ids,
        }),
    ))
}

fn map_desired_target_stream_params(
    stream: &MediaDesiredTargetStream,
) -> MediaDesiredTargetStreamParams {
    MediaDesiredTargetStreamParams {
        stream_key: stream.stream_key.trim().to_string(),
        stream_kind: stream.stream_kind.trim().to_ascii_lowercase(),
        semantic_role: trim_and_filter_empty(stream.semantic_role.as_deref())
            .map(str::to_ascii_lowercase),
        language_code: trim_and_filter_empty(stream.language_code.as_deref())
            .map(str::to_ascii_lowercase),
        optional: stream.optional,
        sort_order: stream.sort_order,
        codec: stream.codec.trim().to_ascii_lowercase(),
        channel_count: stream.channel_count,
        channel_layout: trim_and_filter_empty(stream.channel_layout.as_deref())
            .and_then(normalize_supported_audio_channel_layout)
            .map(str::to_string),
        audio_bitrate_bps: stream.audio_bitrate_bps,
        audio_sample_rate_hz: stream.audio_sample_rate_hz,
        audio_loudness_profile: trim_and_filter_empty(stream.audio_loudness_profile.as_deref())
            .map(str::to_ascii_lowercase),
        audio_dynamic_range: trim_and_filter_empty(stream.audio_dynamic_range.as_deref())
            .map(str::to_ascii_lowercase),
        video_profile: trim_and_filter_empty(stream.video_profile.as_deref())
            .map(str::to_ascii_lowercase),
        video_level: trim_and_filter_empty(stream.video_level.as_deref()).map(str::to_string),
        video_bitrate_bps: stream.video_bitrate_bps,
        color_primaries: trim_and_filter_empty(stream.color_primaries.as_deref())
            .map(str::to_ascii_lowercase),
        color_transfer: trim_and_filter_empty(stream.color_transfer.as_deref())
            .map(str::to_ascii_lowercase),
        color_space: trim_and_filter_empty(stream.color_space.as_deref())
            .map(str::to_ascii_lowercase),
        hdr_format: trim_and_filter_empty(stream.hdr_format.as_deref())
            .map(str::to_ascii_lowercase),
        title: trim_and_filter_empty(stream.title.as_deref()).map(str::to_string),
        default_disposition: stream.default_disposition,
        forced_disposition: stream.forced_disposition,
        subtitle_placement: subtitle_placement_for_stream(stream),
        image_subtitle_action: image_subtitle_action_for_stream(stream),
    }
}

fn map_desired_target_response(
    target: AppMediaDesiredTargetResponse,
) -> MediaDesiredTargetResponse {
    MediaDesiredTargetResponse {
        media_desired_target_profile_public_id: target.media_desired_target_profile_public_id,
        target_key: target.target_key,
        version: target.version,
        display_name: target.display_name,
        container_format: target.container_format,
        container_metadata_policy: target.container_metadata_policy,
        container_metadata: target.container_metadata,
        container_chapter_policy: target.container_chapter_policy,
        container_chapters: target.container_chapters,
        streams: target.streams,
    }
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
        desired_target_key: profile.desired_target_key,
        desired_target_version: profile.desired_target_version,
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

fn normalize_media_key<'a>(value: &'a str, field: &str) -> Result<&'a str, ApiError> {
    validate_media_key(value).map_err(|error| media_text_validation_error(field, error))
}

fn normalize_optional_media_key<'a>(
    value: Option<&'a str>,
    field: &str,
) -> Result<Option<&'a str>, ApiError> {
    let Some(value) = trim_and_filter_empty(value) else {
        return Ok(None);
    };
    normalize_media_key(value, field).map(Some)
}

fn normalize_media_display<'a>(value: &'a str, field: &str) -> Result<&'a str, ApiError> {
    validate_media_display(value).map_err(|error| media_text_validation_error(field, error))
}

fn media_text_validation_error(field: &str, error: MediaTextValidationError) -> ApiError {
    ApiError::bad_request(format!("{field} {error}"))
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

fn normalize_container_metadata_policy(value: Option<&str>) -> Result<String, ApiError> {
    normalize_container_policy(value, CONTAINER_METADATA_POLICY_INVALID, true)
}

fn normalize_container_chapter_policy(value: Option<&str>) -> Result<String, ApiError> {
    normalize_container_policy(value, CONTAINER_CHAPTER_POLICY_INVALID, true)
}

fn normalize_container_policy(
    value: Option<&str>,
    invalid_message: &'static str,
    allow_replace: bool,
) -> Result<String, ApiError> {
    let policy = trim_and_filter_empty(value)
        .unwrap_or("preserve")
        .to_ascii_lowercase();
    if matches!(policy.as_str(), "preserve" | "strip") || (allow_replace && policy == "replace") {
        Ok(policy)
    } else {
        Err(ApiError::bad_request(invalid_message))
    }
}

fn normalize_container_metadata(
    policy: &str,
    metadata: &[MediaDesiredTargetMetadataEntry],
) -> Result<Vec<MediaDesiredTargetMetadataParams>, ApiError> {
    let replace = policy.eq_ignore_ascii_case("replace");
    if replace == metadata.is_empty() {
        return Err(ApiError::bad_request(CONTAINER_METADATA_INVALID));
    }
    if metadata.len() > MAX_CONTAINER_METADATA_ENTRIES {
        return Err(ApiError::bad_request(CONTAINER_METADATA_INVALID));
    }
    let mut keys = BTreeSet::new();
    let mut normalized = Vec::with_capacity(metadata.len());
    let mut total_bytes = 0_usize;
    for entry in metadata {
        let key = entry.key.trim().to_ascii_lowercase();
        let value = entry.value.trim().to_string();
        if key.is_empty()
            || value.is_empty()
            || key.len() > MAX_CONTAINER_METADATA_KEY_BYTES
            || value.len() > MAX_CONTAINER_METADATA_VALUE_BYTES
            || !keys.insert(key.clone())
        {
            return Err(ApiError::bad_request(CONTAINER_METADATA_INVALID));
        }
        total_bytes = total_bytes
            .checked_add(key.len())
            .and_then(|bytes| bytes.checked_add(value.len()))
            .ok_or_else(|| ApiError::bad_request(CONTAINER_METADATA_INVALID))?;
        if total_bytes > MAX_CONTAINER_METADATA_TOTAL_BYTES {
            return Err(ApiError::bad_request(CONTAINER_METADATA_INVALID));
        }
        normalized.push(MediaDesiredTargetMetadataParams { key, value });
    }
    normalized.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(normalized)
}

fn normalize_container_chapters(
    policy: &str,
    chapters: &[MediaDesiredTargetChapterEntry],
) -> Result<Vec<MediaDesiredTargetChapterParams>, ApiError> {
    let replace = policy.eq_ignore_ascii_case("replace");
    if replace == chapters.is_empty() || chapters.len() > MAX_CONTAINER_CHAPTERS {
        return Err(ApiError::bad_request(CONTAINER_CHAPTERS_INVALID));
    }
    let mut normalized = Vec::with_capacity(chapters.len());
    let mut total_metadata_bytes = 0_usize;
    for chapter in chapters {
        if chapter.start_millis < 0 || chapter.end_millis <= chapter.start_millis {
            return Err(ApiError::bad_request(CONTAINER_CHAPTERS_INVALID));
        }
        normalized.push(MediaDesiredTargetChapterParams {
            start_millis: chapter.start_millis,
            end_millis: chapter.end_millis,
            metadata: normalize_container_chapter_metadata(
                &chapter.metadata,
                &mut total_metadata_bytes,
            )?,
        });
    }
    normalized.sort_by(|left, right| {
        left.start_millis
            .cmp(&right.start_millis)
            .then(left.end_millis.cmp(&right.end_millis))
    });
    let mut previous_end = None;
    for chapter in &normalized {
        if previous_end.is_some_and(|end| chapter.start_millis < end) {
            return Err(ApiError::bad_request(CONTAINER_CHAPTERS_INVALID));
        }
        previous_end = Some(chapter.end_millis);
    }
    Ok(normalized)
}

fn normalize_container_chapter_metadata(
    metadata: &[MediaDesiredTargetMetadataEntry],
    total_bytes: &mut usize,
) -> Result<Vec<MediaDesiredTargetMetadataParams>, ApiError> {
    if metadata.len() > MAX_CONTAINER_CHAPTER_METADATA_ENTRIES {
        return Err(ApiError::bad_request(CONTAINER_CHAPTERS_INVALID));
    }
    let mut keys = BTreeSet::new();
    let mut normalized = Vec::with_capacity(metadata.len());
    for entry in metadata {
        let key = entry.key.trim().to_ascii_lowercase();
        let value = entry.value.trim().to_string();
        if key.is_empty()
            || value.is_empty()
            || key.len() > MAX_CONTAINER_METADATA_KEY_BYTES
            || value.len() > MAX_CONTAINER_METADATA_VALUE_BYTES
            || !keys.insert(key.clone())
        {
            return Err(ApiError::bad_request(CONTAINER_CHAPTERS_INVALID));
        }
        *total_bytes = total_bytes
            .checked_add(key.len())
            .and_then(|bytes| bytes.checked_add(value.len()))
            .filter(|bytes| *bytes <= MAX_CONTAINER_CHAPTER_METADATA_TOTAL_BYTES)
            .ok_or_else(|| ApiError::bad_request(CONTAINER_CHAPTERS_INVALID))?;
        normalized.push(MediaDesiredTargetMetadataParams { key, value });
    }
    normalized.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(normalized)
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

fn validate_desired_target_streams(streams: &[MediaDesiredTargetStream]) -> Result<(), ApiError> {
    if streams.is_empty() {
        return Err(ApiError::bad_request(DESIRED_TARGET_STREAMS_REQUIRED));
    }
    if streams.len() > MAX_DESIRED_TARGET_STREAMS {
        return Err(ApiError::bad_request(
            "streams exceeds the maximum desired target stream count",
        ));
    }
    let mut keys = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for stream in streams {
        validate_desired_target_stream_identity(stream, &mut keys, &mut orders)?;
        let kind = validate_desired_target_stream_kind(&stream.stream_kind)?;
        normalize_required_str_field(&stream.codec, "codec is required")?;
        if let Some(title) = trim_and_filter_empty(stream.title.as_deref()) {
            normalize_media_display(title, "title")?;
        }
        validate_desired_target_semantic_role(stream.semantic_role.as_deref())?;
        validate_desired_target_stream_shape(stream, &kind)?;
    }
    if orders.iter().copied().ne((0_i32..).take(streams.len())) {
        return Err(ApiError::bad_request(
            "sort_order must be contiguous from zero",
        ));
    }
    Ok(())
}

fn validate_desired_target_stream_identity(
    stream: &MediaDesiredTargetStream,
    keys: &mut BTreeSet<String>,
    orders: &mut BTreeSet<i32>,
) -> Result<(), ApiError> {
    let key = normalize_media_key(&stream.stream_key, "stream_key")?;
    if !keys.insert(key.to_ascii_lowercase()) {
        return Err(ApiError::bad_request("stream_key must be unique"));
    }
    if stream.sort_order < 0 || !orders.insert(stream.sort_order) {
        return Err(ApiError::bad_request(
            "sort_order must be unique and nonnegative",
        ));
    }
    Ok(())
}

fn validate_desired_target_stream_kind(value: &str) -> Result<String, ApiError> {
    const STREAM_KINDS: [&str; 3] = ["video", "audio", "subtitle"];

    let kind = normalize_required_str_field(value, "stream_kind is required")?.to_ascii_lowercase();
    if STREAM_KINDS.contains(&kind.as_str()) {
        Ok(kind)
    } else {
        Err(ApiError::bad_request("stream_kind is invalid"))
    }
}

fn validate_desired_target_semantic_role(value: Option<&str>) -> Result<(), ApiError> {
    const SEMANTIC_ROLES: [&str; 8] = [
        "primary",
        "forced",
        "commentary",
        "descriptive_audio",
        "sdh",
        "signs_songs",
        "karaoke",
        "unknown",
    ];

    if let Some(role) = trim_and_filter_empty(value)
        && !SEMANTIC_ROLES.contains(&role.to_ascii_lowercase().as_str())
    {
        return Err(ApiError::bad_request("semantic_role is invalid"));
    }
    Ok(())
}

fn validate_desired_target_stream_shape(
    stream: &MediaDesiredTargetStream,
    kind: &str,
) -> Result<(), ApiError> {
    validate_target_scalar_values(stream)?;
    validate_audio_target_shape_scope(stream, kind)?;
    validate_video_target_shape_scope(stream, kind)?;
    validate_subtitle_target_shape_scope(stream, kind)?;
    validate_subtitle_target_shape(stream, kind)?;
    Ok(())
}

fn validate_target_scalar_values(stream: &MediaDesiredTargetStream) -> Result<(), ApiError> {
    if stream.channel_count.is_some_and(|channels| channels <= 0) {
        return Err(ApiError::bad_request(
            "channel_count must be greater than zero",
        ));
    }
    if stream.audio_bitrate_bps.is_some_and(|bitrate| bitrate <= 0) {
        return Err(ApiError::bad_request(
            "audio_bitrate_bps must be greater than zero",
        ));
    }
    if stream
        .audio_sample_rate_hz
        .is_some_and(|sample_rate| sample_rate <= 0)
    {
        return Err(ApiError::bad_request(
            "audio_sample_rate_hz must be greater than zero",
        ));
    }
    if stream
        .audio_loudness_profile
        .as_deref()
        .is_some_and(|profile| !profile.trim().eq_ignore_ascii_case("dialog-normalized"))
    {
        return Err(ApiError::bad_request("audio_loudness_profile is invalid"));
    }
    if stream.audio_dynamic_range.as_deref().is_some_and(|range| {
        !["preserve", "speech"].contains(&range.trim().to_ascii_lowercase().as_str())
    }) {
        return Err(ApiError::bad_request("audio_dynamic_range is invalid"));
    }
    if stream.video_bitrate_bps.is_some_and(|bitrate| bitrate <= 0) {
        return Err(ApiError::bad_request(
            "video_bitrate_bps must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_audio_target_shape_scope(
    stream: &MediaDesiredTargetStream,
    kind: &str,
) -> Result<(), ApiError> {
    if kind != "audio" && has_audio_target_shape_fields(stream) {
        return Err(ApiError::bad_request(
            "audio shape is only valid for audio streams",
        ));
    }
    if kind == "audio" {
        validate_audio_target_layout(stream)?;
    }
    Ok(())
}

const fn has_audio_target_shape_fields(stream: &MediaDesiredTargetStream) -> bool {
    stream.channel_count.is_some()
        || stream.channel_layout.is_some()
        || stream.audio_bitrate_bps.is_some()
        || stream.audio_sample_rate_hz.is_some()
        || stream.audio_loudness_profile.is_some()
        || stream.audio_dynamic_range.is_some()
}

fn validate_video_target_shape_scope(
    stream: &MediaDesiredTargetStream,
    kind: &str,
) -> Result<(), ApiError> {
    if kind != "video" && has_video_target_shape_fields(stream) {
        return Err(ApiError::bad_request(
            "video shape is only valid for video streams",
        ));
    }
    if kind == "video" {
        validate_video_target_shape(stream)?;
    }
    Ok(())
}

const fn has_video_target_shape_fields(stream: &MediaDesiredTargetStream) -> bool {
    stream.video_profile.is_some()
        || stream.video_level.is_some()
        || stream.video_bitrate_bps.is_some()
        || stream.color_primaries.is_some()
        || stream.color_transfer.is_some()
        || stream.color_space.is_some()
        || stream.hdr_format.is_some()
}

fn validate_subtitle_target_shape_scope(
    stream: &MediaDesiredTargetStream,
    kind: &str,
) -> Result<(), ApiError> {
    if stream.forced_disposition && kind != "subtitle" {
        return Err(ApiError::bad_request(
            "forced disposition is only valid for subtitle streams",
        ));
    }
    if kind == "subtitle"
        && trim_and_filter_empty(stream.semantic_role.as_deref())
            .is_some_and(|role| role.eq_ignore_ascii_case("descriptive_audio"))
    {
        return Err(ApiError::bad_request(
            "descriptive_audio is not a subtitle semantic role",
        ));
    }
    Ok(())
}

fn validate_audio_target_layout(stream: &MediaDesiredTargetStream) -> Result<(), ApiError> {
    let Some(raw_layout) = trim_and_filter_empty(stream.channel_layout.as_deref()) else {
        return Ok(());
    };
    let Some(canonical_layout) = normalize_supported_audio_channel_layout(raw_layout) else {
        return Err(ApiError::bad_request("channel_layout is invalid"));
    };
    let Some(layout_channels) = audio_channel_count_for_layout(canonical_layout) else {
        return Err(ApiError::bad_request("channel_layout is invalid"));
    };
    if let Some(channels) = stream.channel_count
        && u32::try_from(channels).ok() != Some(layout_channels)
    {
        return Err(ApiError::bad_request(
            "channel_count must match channel_layout",
        ));
    }
    Ok(())
}

fn validate_video_target_shape(stream: &MediaDesiredTargetStream) -> Result<(), ApiError> {
    validate_video_level(stream)?;
    validate_video_color_value("color_primaries", stream.color_primaries.as_deref())?;
    validate_video_color_value("color_transfer", stream.color_transfer.as_deref())?;
    validate_video_color_value("color_space", stream.color_space.as_deref())?;
    if trim_and_filter_empty(stream.hdr_format.as_deref())
        .is_some_and(|format| !format.eq_ignore_ascii_case("hdr10"))
    {
        return Err(ApiError::bad_request("hdr_format is invalid"));
    }
    Ok(())
}

fn validate_video_level(stream: &MediaDesiredTargetStream) -> Result<(), ApiError> {
    let Some(video_level) = trim_and_filter_empty(stream.video_level.as_deref()) else {
        return Ok(());
    };
    let codec = normalize_required_str_field(&stream.codec, "codec is required")?;
    if is_known_video_level(codec, video_level) {
        Ok(())
    } else {
        Err(ApiError::bad_request("video_level is invalid"))
    }
}

fn validate_video_color_value(field: &'static str, value: Option<&str>) -> Result<(), ApiError> {
    let Some(value) = trim_and_filter_empty(value) else {
        return Ok(());
    };
    let valid = match field {
        "color_primaries" => is_known_color_primaries(value),
        "color_transfer" => is_known_color_transfer(value),
        "color_space" => is_known_color_space(value),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!("{field} is invalid")))
    }
}

fn validate_subtitle_target_shape(
    stream: &MediaDesiredTargetStream,
    kind: &str,
) -> Result<(), ApiError> {
    let placement = trim_and_filter_empty(stream.subtitle_placement.as_deref());
    let image_action = trim_and_filter_empty(stream.image_subtitle_action.as_deref());
    if kind != "subtitle" {
        if placement.is_some() || image_action.is_some() {
            return Err(ApiError::bad_request(
                "subtitle placement and image action are only valid for subtitle streams",
            ));
        }
        return Ok(());
    }
    if placement.is_some_and(|value| {
        !["embedded", "sidecar", "both", "none"].contains(&value.to_ascii_lowercase().as_str())
    }) {
        return Err(ApiError::bad_request("subtitle_placement is invalid"));
    }
    if image_action.is_some_and(|value| {
        !["preserve", "remove", "fail"].contains(&value.to_ascii_lowercase().as_str())
    }) {
        return Err(ApiError::bad_request("image_subtitle_action is invalid"));
    }
    Ok(())
}

fn subtitle_placement_for_stream(stream: &MediaDesiredTargetStream) -> Option<String> {
    (stream.stream_kind.trim().eq_ignore_ascii_case("subtitle")).then(|| {
        trim_and_filter_empty(stream.subtitle_placement.as_deref())
            .unwrap_or("embedded")
            .to_ascii_lowercase()
    })
}

fn image_subtitle_action_for_stream(stream: &MediaDesiredTargetStream) -> Option<String> {
    (stream.stream_kind.trim().eq_ignore_ascii_case("subtitle")).then(|| {
        trim_and_filter_empty(stream.image_subtitle_action.as_deref())
            .unwrap_or("fail")
            .to_ascii_lowercase()
    })
}

fn validate_audio_channels(value: Option<i32>) -> Result<Option<i32>, ApiError> {
    if value.is_none_or(|channels| channels > 0) {
        Ok(value)
    } else {
        Err(ApiError::bad_request(AUDIO_CHANNELS_INVALID))
    }
}

fn normalize_audio_channel_layout(value: Option<&str>) -> Result<Option<String>, ApiError> {
    value
        .map(str::trim)
        .map(|item| {
            if item.is_empty() {
                Err(ApiError::bad_request(AUDIO_CHANNEL_LAYOUT_INVALID))
            } else {
                Ok(item.to_ascii_lowercase())
            }
        })
        .transpose()
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

fn normalize_verification_strictness(value: &str) -> Result<String, ApiError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "strict" | "balanced" | "fast" => Ok(normalized),
        _ => Err(ApiError::bad_request(VERIFICATION_STRICTNESS_INVALID)),
    }
}

fn validate_verification_duration_tolerance(value: i64) -> Result<(), ApiError> {
    if (0..=60_000).contains(&value) {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            VERIFICATION_DURATION_TOLERANCE_INVALID,
        ))
    }
}

fn validate_verification_check_selection(
    request: &MediaPolicyUpsertRequest,
    strictness: &str,
) -> Result<(), ApiError> {
    let valid = match strictness {
        "strict" => {
            request.verification_mux_validation.enabled()
                && request.verification_decode_all_streams.enabled()
                && request.verification_keyframe_seek.enabled()
                && request.verification_playback_probe.enabled()
        }
        "balanced" => true,
        "fast" => {
            !request.verification_decode_all_streams.enabled()
                && !request.verification_keyframe_seek.enabled()
                && !request.verification_playback_probe.enabled()
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "verification checks do not match the selected strictness",
        ))
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
    issues
}

async fn append_profile_catalog_issues(
    state: &ApiState,
    compatibility_target_key: Option<&str>,
    policy_key: Option<&str>,
    compatibility_issue: &str,
    policy_issue: &str,
    issues: &mut Vec<String>,
) -> Result<(), ApiError> {
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
        })?;
    let policies = state.media.media_policy_list().await.map_err(|err| {
        map_media_error("media_policy_list", "failed to list media policies", &err)
    })?;

    if compatibility_target_key
        .and_then(|value| trim_and_filter_empty(Some(value)))
        .is_some_and(|target| {
            let normalized = target.trim().replace('_', "-");
            !targets.iter().any(|item| {
                item.compatibility_target_key
                    .replace('_', "-")
                    .eq_ignore_ascii_case(&normalized)
            })
        })
    {
        issues.push(compatibility_issue.to_string());
    }
    let normalized_policy = policy_key
        .and_then(|value| trim_and_filter_empty(Some(value)))
        .unwrap_or("safe_dry_run");
    if !policies
        .iter()
        .any(|item| item.policy_key.eq_ignore_ascii_case(normalized_policy))
    {
        issues.push(policy_issue.to_string());
    }
    issues.sort();
    issues.dedup();
    Ok(())
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

fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, |converted| converted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::handlers::indexers::test_support::{RecordingIndexers, indexer_test_state};
    use crate::models::ProblemDetails;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;
    use chrono::TimeZone;

    #[test]
    fn container_metadata_policy_normalizes_supported_values() -> anyhow::Result<()> {
        assert_eq!(normalize_container_metadata_policy(None)?, "preserve");
        assert_eq!(
            normalize_container_metadata_policy(Some(" Strip "))?,
            "strip"
        );
        assert_eq!(
            normalize_container_metadata_policy(Some(" Replace "))?,
            "replace"
        );
        assert!(normalize_container_metadata_policy(Some("rewrite")).is_err());
        Ok(())
    }

    #[test]
    fn container_metadata_values_normalize_only_for_replace_policy() -> anyhow::Result<()> {
        let metadata = vec![
            MediaDesiredTargetMetadataEntry {
                key: " Title ".to_string(),
                value: " Canonical Cut ".to_string(),
            },
            MediaDesiredTargetMetadataEntry {
                key: "COMMENT".to_string(),
                value: "Verified".to_string(),
            },
        ];
        let normalized = normalize_container_metadata("replace", &metadata)?;
        assert_eq!(
            normalized
                .iter()
                .map(|entry| (entry.key.as_str(), entry.value.as_str()))
                .collect::<Vec<_>>(),
            vec![("comment", "Verified"), ("title", "Canonical Cut")]
        );
        assert!(normalize_container_metadata("preserve", &metadata).is_err());
        assert!(normalize_container_metadata("strip", &metadata).is_err());
        assert!(normalize_container_metadata("replace", &[]).is_err());
        assert!(
            normalize_container_metadata(
                "replace",
                &[
                    MediaDesiredTargetMetadataEntry {
                        key: "title".to_string(),
                        value: "one".to_string(),
                    },
                    MediaDesiredTargetMetadataEntry {
                        key: " Title ".to_string(),
                        value: "two".to_string(),
                    },
                ],
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn container_metadata_values_enforce_shared_resource_budget() {
        let at_maximum = (0..MAX_CONTAINER_METADATA_ENTRIES)
            .map(|index| MediaDesiredTargetMetadataEntry {
                key: format!("key-{index}"),
                value: "value".to_string(),
            })
            .collect::<Vec<_>>();
        assert!(normalize_container_metadata("replace", &at_maximum).is_ok());

        let mut above_maximum = at_maximum;
        above_maximum.push(MediaDesiredTargetMetadataEntry {
            key: "overflow".to_string(),
            value: "value".to_string(),
        });
        assert!(normalize_container_metadata("replace", &above_maximum).is_err());

        let aggregate_overflow = (0..17)
            .map(|index| MediaDesiredTargetMetadataEntry {
                key: format!("key-{index}"),
                value: "x".repeat(MAX_CONTAINER_METADATA_VALUE_BYTES),
            })
            .collect::<Vec<_>>();
        assert!(normalize_container_metadata("replace", &aggregate_overflow).is_err());
    }

    #[test]
    fn container_chapter_policy_normalizes_supported_values() -> anyhow::Result<()> {
        assert_eq!(normalize_container_chapter_policy(None)?, "preserve");
        assert_eq!(
            normalize_container_chapter_policy(Some(" Strip "))?,
            "strip"
        );
        assert_eq!(
            normalize_container_chapter_policy(Some(" Replace "))?,
            "replace"
        );
        assert!(normalize_container_chapter_policy(Some("rewrite")).is_err());
        Ok(())
    }

    #[test]
    fn container_chapter_values_normalize_only_for_replace_policy() -> anyhow::Result<()> {
        let chapters = vec![
            MediaDesiredTargetChapterEntry {
                start_millis: 60_000,
                end_millis: 120_000,
                metadata: vec![MediaDesiredTargetMetadataEntry {
                    key: "TITLE".to_string(),
                    value: "Act Two".to_string(),
                }],
            },
            MediaDesiredTargetChapterEntry {
                start_millis: 0,
                end_millis: 60_000,
                metadata: vec![MediaDesiredTargetMetadataEntry {
                    key: " Title ".to_string(),
                    value: " Act One ".to_string(),
                }],
            },
        ];

        let normalized = normalize_container_chapters("replace", &chapters)?;
        assert_eq!(normalized[0].start_millis, 0);
        assert_eq!(normalized[0].end_millis, 60_000);
        assert_eq!(normalized[0].metadata[0].key, "title");
        assert_eq!(normalized[0].metadata[0].value, "Act One");
        assert_eq!(normalized[1].start_millis, 60_000);

        assert!(normalize_container_chapters("preserve", &chapters).is_err());
        assert!(normalize_container_chapters("strip", &chapters).is_err());
        assert!(normalize_container_chapters("replace", &[]).is_err());
        assert!(
            normalize_container_chapters(
                "replace",
                &[MediaDesiredTargetChapterEntry {
                    start_millis: 1000,
                    end_millis: 1000,
                    metadata: Vec::new(),
                }],
            )
            .is_err()
        );
        assert!(
            normalize_container_chapters(
                "replace",
                &[
                    MediaDesiredTargetChapterEntry {
                        start_millis: 0,
                        end_millis: 2000,
                        metadata: Vec::new(),
                    },
                    MediaDesiredTargetChapterEntry {
                        start_millis: 1000,
                        end_millis: 3000,
                        metadata: Vec::new(),
                    },
                ],
            )
            .is_err()
        );
        assert!(
            normalize_container_chapters(
                "replace",
                &[MediaDesiredTargetChapterEntry {
                    start_millis: 0,
                    end_millis: 1000,
                    metadata: vec![
                        MediaDesiredTargetMetadataEntry {
                            key: "title".to_string(),
                            value: "one".to_string(),
                        },
                        MediaDesiredTargetMetadataEntry {
                            key: " Title ".to_string(),
                            value: "two".to_string(),
                        },
                    ],
                }],
            )
            .is_err()
        );

        Ok(())
    }

    #[tokio::test]
    async fn list_media_profiles_returns_empty_payload_with_default_facade() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = list_media_profiles(State(state)).await?;
        assert!(response.profiles.is_empty());
        Ok(())
    }

    #[test]
    fn recent_job_cursor_round_trips_and_rejects_malformed_values() -> anyhow::Result<()> {
        let queued_at = Utc
            .timestamp_opt(1_700_000_000, 123)
            .single()
            .ok_or_else(|| anyhow::anyhow!("timestamp unavailable"))?;
        let id = Uuid::new_v4();
        let encoded = encode_recent_job_cursor((queued_at, id));
        assert_eq!(decode_recent_job_cursor(&encoded)?, (queued_at, id));
        assert!(decode_recent_job_cursor("not-base64!").is_err());
        assert!(decode_recent_job_cursor(&URL_SAFE_NO_PAD.encode("incomplete")).is_err());
        Ok(())
    }

    #[tokio::test]
    async fn recent_jobs_defaults_to_empty_bounded_page() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = list_recent_media_jobs(
            State(state),
            Query(MediaRecentJobsQuery {
                limit: None,
                cursor: None,
                media_profile_public_id: None,
            }),
        )
        .await?;
        assert!(response.jobs.is_empty());
        assert!(response.next_cursor.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn media_configuration_reads_return_default_payloads() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;

        let Json(targets) = list_media_compatibility_targets(State(state.clone())).await?;
        assert!(targets.targets.is_empty());

        let Json(desired_targets) = list_media_desired_targets(State(state.clone())).await?;
        assert!(desired_targets.targets.is_empty());

        let Json(policies) = list_media_policies(State(state.clone())).await?;
        assert!(policies.policies.is_empty());

        let Json(retention) = media_job_retention(State(state)).await?;
        assert!(!retention.completed_enabled);
        assert_eq!(retention.completed_mode, "age");
        assert_eq!(retention.completed_limit, 30);
        assert!(retention.failed_diagnostic_enabled);
        assert_eq!(retention.failed_diagnostic_mode, "age");
        assert_eq!(retention.failed_diagnostic_limit, 30);
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
            audio_channels: None,
            audio_channel_layout: None,
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

        let target_request = MediaCompatibilityTargetUpsertRequest {
            compatibility_target_key: "plex-living-room".to_string(),
            version: 1,
            display_name: "Plex living room".to_string(),
            video_codec: "hevc".to_string(),
            audio_codec: "aac".to_string(),
            audio_channels: Some(0),
            audio_channel_layout: None,
            subtitle_policy: "selected".to_string(),
        };
        let target_error =
            upsert_media_compatibility_target(State(state.clone()), Json(target_request))
                .await
                .expect_err("non-positive target audio channels should fail validation");
        assert_eq!(
            target_error.into_response().status(),
            StatusCode::BAD_REQUEST
        );

        let policy_request = MediaPolicyUpsertRequest {
            policy_key: "living-room".to_string(),
            version: 1,
            display_name: "Living room".to_string(),
            video_intent: "lossless".to_string(),
            verification_strictness: "strict".to_string(),
            verification_duration_tolerance_millis: 100,
            verification_mux_validation: true.into(),
            verification_decode_all_streams: true.into(),
            verification_keyframe_seek: true.into(),
            verification_playback_probe: true.into(),
        };
        let policy_error = upsert_media_policy(State(state.clone()), Json(policy_request))
            .await
            .expect_err("unknown video intent should fail validation");
        assert_eq!(
            policy_error.into_response().status(),
            StatusCode::BAD_REQUEST
        );

        let relaxed_strict_policy = MediaPolicyUpsertRequest {
            policy_key: "relaxed-strict".to_string(),
            version: 1,
            display_name: "Relaxed strict".to_string(),
            video_intent: "general".to_string(),
            verification_strictness: "strict".to_string(),
            verification_duration_tolerance_millis: 100,
            verification_mux_validation: true.into(),
            verification_decode_all_streams: true.into(),
            verification_keyframe_seek: true.into(),
            verification_playback_probe: false.into(),
        };
        let strict_error = upsert_media_policy(State(state.clone()), Json(relaxed_strict_policy))
            .await
            .expect_err("strict verification must require every strict check");
        assert_eq!(
            strict_error.into_response().status(),
            StatusCode::BAD_REQUEST
        );

        let retention_request = MediaJobRetentionUpdateRequest {
            completed_enabled: true,
            completed_mode: "age".to_string(),
            completed_limit: 0,
            failed_diagnostic_enabled: true,
            failed_diagnostic_mode: "age".to_string(),
            failed_diagnostic_limit: 30,
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
    async fn desired_target_writes_validate_complete_graph_and_profile_pin() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let valid_stream = desired_target_valid_video_stream();
        let invalid = create_media_desired_target(
            State(state.clone()),
            Json(MediaDesiredTargetCreateRequest {
                target_key: "target".to_string(),
                version: 1,
                display_name: "Target".to_string(),
                container_format: "matroska".to_string(),
                container_metadata_policy: None,
                container_metadata: Vec::new(),
                container_chapter_policy: None,
                container_chapters: Vec::new(),
                streams: Vec::new(),
            }),
        )
        .await;
        assert!(invalid.is_err());

        let invalid_metadata_policy = create_media_desired_target(
            State(state.clone()),
            Json(MediaDesiredTargetCreateRequest {
                target_key: "target".to_string(),
                version: 1,
                display_name: "Target".to_string(),
                container_format: "matroska".to_string(),
                container_metadata_policy: Some("rewrite".to_string()),
                container_metadata: Vec::new(),
                container_chapter_policy: None,
                container_chapters: Vec::new(),
                streams: vec![valid_stream.clone()],
            }),
        )
        .await;
        assert!(invalid_metadata_policy.is_err());

        let unavailable = create_media_desired_target(
            State(state.clone()),
            Json(MediaDesiredTargetCreateRequest {
                target_key: " target ".to_string(),
                version: 1,
                display_name: " Target ".to_string(),
                container_format: " Matroska ".to_string(),
                container_metadata_policy: Some(" Preserve ".to_string()),
                container_metadata: Vec::new(),
                container_chapter_policy: Some(" Preserve ".to_string()),
                container_chapters: Vec::new(),
                streams: vec![valid_stream.clone()],
            }),
        )
        .await;
        assert!(unavailable.is_err());

        let invalid_pin = set_media_profile_desired_target(
            State(state.clone()),
            Path(Uuid::new_v4()),
            Json(MediaProfileDesiredTargetRequest {
                target_key: Some("target".to_string()),
                version: None,
            }),
        )
        .await;
        assert!(invalid_pin.is_err());
        let unavailable_pin = set_media_profile_desired_target(
            State(state),
            Path(Uuid::new_v4()),
            Json(MediaProfileDesiredTargetRequest {
                target_key: Some("target".to_string()),
                version: Some(1),
            }),
        )
        .await;
        assert!(unavailable_pin.is_err());
        assert_desired_target_audio_validation_rejects_invalid_shapes(&valid_stream);
        assert_desired_target_video_validation_rejects_invalid_shapes(&valid_stream);
        assert_desired_target_subtitle_validation_rejects_invalid_shapes(valid_stream);
        Ok(())
    }

    #[test]
    fn desired_target_stream_validation_enforces_count_boundaries() {
        fn streams(count: usize) -> Vec<MediaDesiredTargetStream> {
            (0_i32..)
                .zip(0..count)
                .map(|(sort_order, index)| {
                    let mut stream = desired_target_valid_video_stream();
                    stream.stream_key = format!("video-{index}");
                    stream.sort_order = sort_order;
                    stream
                })
                .collect()
        }

        assert!(validate_desired_target_streams(&[]).is_err());
        assert!(validate_desired_target_streams(&streams(1)).is_ok());
        assert!(validate_desired_target_streams(&streams(MAX_DESIRED_TARGET_STREAMS)).is_ok());
        assert!(validate_desired_target_streams(&streams(MAX_DESIRED_TARGET_STREAMS + 1)).is_err());
    }

    fn desired_target_valid_video_stream() -> MediaDesiredTargetStream {
        MediaDesiredTargetStream {
            stream_key: "video-main".to_string(),
            stream_kind: "video".to_string(),
            semantic_role: None,
            language_code: None,
            optional: false,
            sort_order: 0,
            codec: "hevc".to_string(),
            channel_count: None,
            channel_layout: None,
            audio_bitrate_bps: None,
            audio_sample_rate_hz: None,
            audio_loudness_profile: None,
            audio_dynamic_range: None,
            video_profile: Some("main10".to_string()),
            video_level: Some("5.1".to_string()),
            video_bitrate_bps: Some(8_000_000),
            color_primaries: Some("bt2020".to_string()),
            color_transfer: Some("smpte2084".to_string()),
            color_space: Some("bt2020nc".to_string()),
            hdr_format: Some("hdr10".to_string()),
            title: None,
            default_disposition: true,
            forced_disposition: false,
            subtitle_placement: None,
            image_subtitle_action: None,
        }
    }

    fn assert_desired_target_audio_validation_rejects_invalid_shapes(
        valid_stream: &MediaDesiredTargetStream,
    ) {
        let mut invalid_stream = valid_stream.clone();
        invalid_stream.channel_count = Some(2);
        assert!(validate_desired_target_streams(&[invalid_stream]).is_err());

        let mut invalid_audio_bitrate = valid_stream.clone();
        invalid_audio_bitrate.stream_kind = "audio".to_string();
        invalid_audio_bitrate.audio_bitrate_bps = Some(0);
        assert!(validate_desired_target_streams(&[invalid_audio_bitrate]).is_err());

        let mut invalid_audio_sample_rate = valid_stream.clone();
        invalid_audio_sample_rate.stream_kind = "audio".to_string();
        invalid_audio_sample_rate.audio_sample_rate_hz = Some(0);
        assert!(validate_desired_target_streams(&[invalid_audio_sample_rate]).is_err());

        let mut cross_kind_audio_bitrate = valid_stream.clone();
        cross_kind_audio_bitrate.audio_bitrate_bps = Some(160_000);
        assert!(validate_desired_target_streams(&[cross_kind_audio_bitrate]).is_err());

        let mut cross_kind_audio_sample_rate = valid_stream.clone();
        cross_kind_audio_sample_rate.audio_sample_rate_hz = Some(48_000);
        assert!(validate_desired_target_streams(&[cross_kind_audio_sample_rate]).is_err());

        let mut unsupported_layout = valid_stream.clone();
        unsupported_layout.stream_kind = "audio".to_string();
        unsupported_layout.channel_layout = Some("ambisonic".to_string());
        assert!(validate_desired_target_streams(&[unsupported_layout]).is_err());

        let mut mismatched_layout = valid_stream.clone();
        mismatched_layout.stream_kind = "audio".to_string();
        mismatched_layout.channel_count = Some(2);
        mismatched_layout.channel_layout = Some("5.1".to_string());
        assert!(validate_desired_target_streams(&[mismatched_layout]).is_err());
    }

    fn assert_desired_target_video_validation_rejects_invalid_shapes(
        valid_stream: &MediaDesiredTargetStream,
    ) {
        let mut cross_kind_video_level = valid_stream.clone();
        cross_kind_video_level.stream_kind = "audio".to_string();
        assert!(validate_desired_target_streams(&[cross_kind_video_level]).is_err());

        let mut invalid_video_bitrate = valid_stream.clone();
        invalid_video_bitrate.video_bitrate_bps = Some(0);
        assert!(validate_desired_target_streams(&[invalid_video_bitrate]).is_err());

        let mut invalid_video_level = valid_stream.clone();
        invalid_video_level.video_level = Some("7.9".to_string());
        assert!(validate_desired_target_streams(&[invalid_video_level]).is_err());

        let mut invalid_color_primaries = valid_stream.clone();
        invalid_color_primaries.color_primaries = Some("unknown".to_string());
        assert!(validate_desired_target_streams(&[invalid_color_primaries]).is_err());

        let mut invalid_color_transfer = valid_stream.clone();
        invalid_color_transfer.color_transfer = Some("unknown".to_string());
        assert!(validate_desired_target_streams(&[invalid_color_transfer]).is_err());

        let mut invalid_color_space = valid_stream.clone();
        invalid_color_space.color_space = Some("unknown".to_string());
        assert!(validate_desired_target_streams(&[invalid_color_space]).is_err());

        let mut invalid_hdr_format = valid_stream.clone();
        invalid_hdr_format.hdr_format = Some("dolby_vision".to_string());
        assert!(validate_desired_target_streams(&[invalid_hdr_format]).is_err());
    }

    fn assert_desired_target_subtitle_validation_rejects_invalid_shapes(
        valid_stream: MediaDesiredTargetStream,
    ) {
        let mut invalid_placement = valid_stream.clone();
        invalid_placement.stream_kind = "subtitle".to_string();
        invalid_placement.subtitle_placement = Some("elsewhere".to_string());
        assert!(validate_desired_target_streams(&[invalid_placement]).is_err());

        let mut cross_kind_subtitle_shape = valid_stream.clone();
        cross_kind_subtitle_shape.subtitle_placement = Some("embedded".to_string());
        cross_kind_subtitle_shape.image_subtitle_action = Some("fail".to_string());
        assert!(validate_desired_target_streams(&[cross_kind_subtitle_shape]).is_err());

        let mut invalid_subtitle_role = valid_stream;
        invalid_subtitle_role.stream_kind = "subtitle".to_string();
        invalid_subtitle_role.semantic_role = Some("descriptive_audio".to_string());
        assert!(validate_desired_target_streams(&[invalid_subtitle_role]).is_err());
    }

    #[test]
    fn desired_target_stream_validation_rejects_unsupported_kinds() {
        let unsupported = ["attachment", "chapter", "data"].map(|kind| MediaDesiredTargetStream {
            stream_key: format!("{kind}-main"),
            stream_kind: kind.to_string(),
            semantic_role: None,
            language_code: None,
            optional: false,
            sort_order: 0,
            codec: "copy".to_string(),
            channel_count: None,
            channel_layout: None,
            audio_bitrate_bps: None,
            audio_sample_rate_hz: None,
            audio_loudness_profile: None,
            audio_dynamic_range: None,
            video_profile: None,
            video_level: None,
            video_bitrate_bps: None,
            color_primaries: None,
            color_transfer: None,
            color_space: None,
            hdr_format: None,
            title: None,
            default_disposition: false,
            forced_disposition: false,
            subtitle_placement: None,
            image_subtitle_action: None,
        });

        for stream in unsupported {
            assert!(validate_desired_target_streams(&[stream]).is_err());
        }
    }

    #[test]
    fn desired_target_stream_deserialization_rejects_unknown_fields() {
        let payload = r#"{
            "stream_key": "video-main",
            "stream_kind": "video",
            "sort_order": 0,
            "codec": "hevc",
            "video_profile": "main10",
            "video_levle": "5.1"
        }"#;

        let result = serde_json::from_str::<MediaDesiredTargetStream>(payload);

        assert!(result.is_err());
    }

    #[test]
    fn desired_target_audio_constraints_are_normalized_and_mapped() {
        let request_stream = MediaDesiredTargetStream {
            stream_key: " audio-primary ".to_string(),
            stream_kind: " AUDIO ".to_string(),
            semantic_role: Some(" Primary ".to_string()),
            language_code: Some(" ENG ".to_string()),
            optional: false,
            sort_order: 0,
            codec: " AAC ".to_string(),
            channel_count: Some(2),
            channel_layout: Some(" Stereo ".to_string()),
            audio_bitrate_bps: Some(160_000),
            audio_sample_rate_hz: Some(48_000),
            audio_loudness_profile: Some(" Dialog-Normalized ".to_string()),
            audio_dynamic_range: Some(" Speech ".to_string()),
            video_profile: None,
            video_level: None,
            video_bitrate_bps: None,
            color_primaries: None,
            color_transfer: None,
            color_space: None,
            hdr_format: None,
            title: Some(" Main audio ".to_string()),
            default_disposition: true,
            forced_disposition: false,
            subtitle_placement: None,
            image_subtitle_action: None,
        };

        validate_desired_target_streams(std::slice::from_ref(&request_stream))
            .expect("valid audio target stream should pass validation");
        let params = map_desired_target_stream_params(&request_stream);
        assert_eq!(params.stream_key, "audio-primary");
        assert_eq!(params.stream_kind, "audio");
        assert_eq!(params.semantic_role.as_deref(), Some("primary"));
        assert_eq!(params.language_code.as_deref(), Some("eng"));
        assert_eq!(params.codec, "aac");
        assert_eq!(params.channel_count, Some(2));
        assert_eq!(params.channel_layout.as_deref(), Some("stereo"));
        assert_eq!(params.audio_bitrate_bps, Some(160_000));
        assert_eq!(params.audio_sample_rate_hz, Some(48_000));
        assert_eq!(
            params.audio_loudness_profile.as_deref(),
            Some("dialog-normalized")
        );
        assert_eq!(params.audio_dynamic_range.as_deref(), Some("speech"));
        assert_eq!(params.title.as_deref(), Some("Main audio"));

        let response = map_desired_target_response(AppMediaDesiredTargetResponse {
            media_desired_target_profile_public_id: Uuid::new_v4(),
            target_key: "living-room".to_string(),
            version: 3,
            display_name: "Living room".to_string(),
            container_format: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            streams: vec![params],
        });
        assert_eq!(response.container_metadata_policy, "preserve");
        assert_eq!(response.container_chapter_policy, "preserve");
        let response_stream = response
            .streams
            .first()
            .expect("mapped response should contain the audio stream");
        assert_eq!(response_stream.audio_bitrate_bps, Some(160_000));
        assert_eq!(response_stream.audio_sample_rate_hz, Some(48_000));
        assert_eq!(
            response_stream.audio_loudness_profile.as_deref(),
            Some("dialog-normalized")
        );
        assert_eq!(
            response_stream.audio_dynamic_range.as_deref(),
            Some("speech")
        );
        assert_eq!(response_stream.channel_layout.as_deref(), Some("stereo"));
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
            audio_channels: Some(2),
            audio_channel_layout: Some(" Stereo ".to_string()),
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
            verification_strictness: " Strict ".to_string(),
            verification_duration_tolerance_millis: 100,
            verification_mux_validation: true.into(),
            verification_decode_all_streams: true.into(),
            verification_keyframe_seek: true.into(),
            verification_playback_probe: true.into(),
        };
        let policy_error = upsert_media_policy(State(state.clone()), Json(policy_request))
            .await
            .expect_err("default media facade should reject policy writes");
        assert_eq!(
            policy_error.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        let retention_request = MediaJobRetentionUpdateRequest {
            completed_enabled: true,
            completed_mode: "count".to_string(),
            completed_limit: 45,
            failed_diagnostic_enabled: true,
            failed_diagnostic_mode: "age".to_string(),
            failed_diagnostic_limit: 90,
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
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = media_compliance(State(state)).await?;

        assert_eq!(response.license_mode, "redistributable-gplv3-runtime");
        assert_eq!(response.image_license_mode, "redistributable-gplv3-runtime");
        assert_eq!(response.ffmpeg_license_mode, "unknown");
        assert!(!response.ffmpeg_enable_gpl);
        assert!(!response.ffmpeg_enable_version3);
        assert!(!response.ffmpeg_enable_nonfree);
        assert_eq!(
            response.source_offer_path,
            "/app/compliance/SOURCE-OFFER.txt"
        );
        assert_eq!(
            response.source_offer_url,
            "/app/compliance/SOURCE-OFFER.txt"
        );
        assert_eq!(
            response.third_party_notices_path,
            "/app/compliance/THIRD-PARTY-NOTICES.md"
        );
        assert_eq!(
            response.third_party_notices_url,
            "/app/compliance/THIRD-PARTY-NOTICES.md"
        );
        assert_eq!(
            response.sbom_path,
            "/app/compliance/media-runtime-inventory.spdx.json"
        );
        assert_eq!(
            response.sbom_url,
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
        assert_eq!(
            response.source_compliance_bundle_path,
            "/app/compliance/final-image-compliance-bundle.json"
        );
        assert_eq!(
            response.source_compliance_bundle_digest,
            "unavailable-until-final-image-bundle-is-present"
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
    async fn run_media_discovery_watcher_rejects_empty_source_paths() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let request = crate::models::MediaDiscoveryRunRequest {
            media_profile_public_id: Uuid::new_v4(),
            source_paths: Vec::new(),
        };

        let err = run_media_discovery_watcher(State(state), Json(request))
            .await
            .expect_err("empty watcher discovery source paths should fail validation");
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

    #[test]
    fn media_profile_validation_accepts_enabled_watcher_shape() {
        let request = MediaProfileUpsertRequest {
            profile_key: "tv".to_string(),
            source_root: "/input/tv".to_string(),
            output_root: "/output/tv".to_string(),
            dry_run_only: true,
            retention_days: 30,
            compatibility_target_key: None,
            policy_key: "safe_dry_run".to_string(),
            watcher_enabled: true,
            schedule_enabled: false,
            schedule_interval_minutes: None,
        };

        let issues = collect_profile_validation_issues(&request);

        assert!(!issues.contains(&"media_discovery_watcher_unavailable".to_string()));
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
    async fn list_media_job_phases_returns_empty_payload_with_default_facade() -> anyhow::Result<()>
    {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let Json(response) = list_media_job_phases(State(state), Path(Uuid::new_v4())).await?;
        assert!(response.phases.is_empty());
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
    async fn media_profile_readiness_returns_not_found_for_missing_profile() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let err = get_media_profile_readiness(State(state), Path(Uuid::new_v4()))
            .await
            .expect_err("missing profile readiness should return not found");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn map_media_error_preserves_capability_snapshot_missing_code() -> anyhow::Result<()> {
        let err = MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_capability_snapshot_missing");
        let api_error = map_media_error("media_job_create", "failed to create media job", &err);
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
        let api_error = map_media_error("media_job_create", "failed to create media job", &err);
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

    #[test]
    fn validate_retention_policy_modes_and_limits() {
        assert_eq!(validate_retention_mode(" AGE ").ok(), Some("age"));
        assert_eq!(validate_retention_mode("Count").ok(), Some("count"));
        assert!(validate_retention_mode("unbounded").is_err());
        assert!(validate_retention_limit(0).is_err());
        assert!(validate_retention_limit(1).is_ok());
        assert!(validate_retention_limit(3650).is_ok());
        assert!(validate_retention_limit(3651).is_err());
    }

    #[test]
    fn media_request_keys_use_shared_exact_boundary_contract() {
        let exact = format!("a{}z", "b".repeat(126));
        assert_eq!(
            normalize_media_key(&exact, "profile_key").ok(),
            Some(exact.as_str())
        );

        let oversized = format!("a{}z", "b".repeat(127));
        assert!(normalize_media_key(&oversized, "profile_key").is_err());
        assert!(normalize_media_key("Profile Key", "profile_key").is_err());
        assert_eq!(
            normalize_optional_media_key(Some("  "), "policy_key").ok(),
            Some(None)
        );
    }

    #[test]
    fn bounded_profile_key_bounds_replay_payload_size() -> anyhow::Result<()> {
        let profile_key = format!("a{}z", "b".repeat(126));
        let profile_key = normalize_media_key(&profile_key, "profile_key")?;
        let event = CoreEvent::MediaProfileChanged {
            media_profile_public_id: Uuid::nil(),
            profile_key: profile_key.to_string(),
        };
        let encoded_event_bytes = serde_json::to_vec(&event)?.len();
        let maximum_default_replay_bytes = encoded_event_bytes
            .checked_mul(revaer_events::DEFAULT_REPLAY_CAPACITY)
            .ok_or_else(|| anyhow::anyhow!("replay byte bound overflowed"))?;

        assert!(encoded_event_bytes <= 256);
        assert!(maximum_default_replay_bytes <= 256 * 1024);
        Ok(())
    }
}
