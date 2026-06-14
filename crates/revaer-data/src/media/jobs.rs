//! Stored-procedure access for media job lifecycle.

use crate::error::{Result, try_op};
use sqlx::PgPool;
use uuid::Uuid;

use super::configuration::MediaVerificationToggle;

const MEDIA_JOB_CREATE_V1: &str = "SELECT media_job_create_v1(actor_public_id_input => $1, media_profile_public_id_input => $2, source_path_input => $3, output_path_input => $4, dry_run_input => $5)";
const MEDIA_JOB_PHASE_APPEND_V1: &str = "SELECT media_job_phase_append_v1(media_job_public_id_input => $1, phase_index_input => $2, phase_name_input => $3, phase_status_input => $4, details_text_input => $5)";
const MEDIA_JOB_OPERATION_APPEND_V1: &str = "SELECT media_job_operation_append_v1(media_job_public_id_input => $1, operation_index_input => $2, operation_kind_input => $3, stream_id_input => $4, command_bin_input => $5, arg_1_input => $6, arg_2_input => $7, arg_3_input => $8, arg_4_input => $9, arg_5_input => $10)";
const MEDIA_JOB_OPERATION_LIST_V1: &str = "SELECT operation_index, operation_kind, stream_id, command_bin, arg_1, arg_2, arg_3, arg_4, arg_5, created_at FROM media_job_operation_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_VIOLATION_APPEND_V1: &str = "SELECT media_job_violation_append_v1(media_job_public_id_input => $1, violation_index_input => $2, violation_kind_input => $3, severity_input => $4, stream_id_input => $5)";
const MEDIA_JOB_VIOLATION_LIST_V1: &str = "SELECT violation_index, violation_kind, severity, stream_id, created_at FROM media_job_violation_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_PLAN_REASON_APPEND_V1: &str = "SELECT media_job_plan_reason_append_v1(media_job_public_id_input => $1, reason_index_input => $2, candidate_index_input => $3, selected_input => $4, reason_code_input => $5, reason_text_input => $6)";
const MEDIA_JOB_PLAN_REASON_LIST_V1: &str = "SELECT reason_index, candidate_index, selected, reason_code, reason_text, created_at FROM media_job_plan_reason_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_VERIFICATION_CHECK_APPEND_V1: &str = "SELECT media_job_verification_check_append_v1(media_job_public_id_input => $1, check_index_input => $2, check_kind_input => $3, check_status_input => $4, expected_value_input => $5, actual_value_input => $6, details_text_input => $7)";
const MEDIA_JOB_VERIFICATION_CHECK_LIST_V1: &str = "SELECT check_index, check_kind, check_status, expected_value, actual_value, details_text, created_at FROM media_job_verification_check_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_ARTIFACT_APPEND_V1: &str = "SELECT media_job_artifact_append_v1(media_job_public_id_input => $1, artifact_index_input => $2, artifact_kind_input => $3, artifact_path_input => $4, size_bytes_input => $5, content_type_input => $6)";
const MEDIA_JOB_ARTIFACT_LIST_V1: &str = "SELECT artifact_index, artifact_kind, artifact_path, size_bytes, content_type, created_at FROM media_job_artifact_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_COMPACT_AUDIT_APPEND_V1: &str = "SELECT media_job_compact_audit_append_v1(media_job_public_id_input => $1, audit_index_input => $2, fact_kind_input => $3, fact_text_input => $4)";
const MEDIA_JOB_COMPACT_AUDIT_LIST_V1: &str = "SELECT audit_index, fact_kind, fact_text, created_at FROM media_job_compact_audit_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_LIST_V1: &str = "SELECT media_job_public_id, source_path, output_path, status::text AS status_text, dry_run, queued_at, started_at, completed_at, last_error FROM media_job_list_v1(media_profile_public_id_input => $1, status_input => $2::media_job_status)";
const MEDIA_JOB_GET_V1: &str = "SELECT media_job_public_id, source_path, output_path, status::text AS status_text, dry_run, queued_at, started_at, completed_at, last_error FROM media_job_get_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_CANCEL_V2: &str = "SELECT media_job_cancel_v2(media_job_public_id_input => $1)";
const MEDIA_JOB_RETRY_V1: &str = "SELECT media_job_retry_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_MARK_COMPLETED_V1: &str =
    "SELECT media_job_mark_completed_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_RETENTION_RUN_V1: &str = "SELECT completed_jobs_deleted, failed_jobs_pruned, failed_detail_rows_deleted FROM media_job_retention_run_v1(as_of_input => $1)";
const MEDIA_JOB_WORKER_CLAIM_NEXT_V2: &str = "SELECT media_job_public_id, media_profile_public_id, source_path, output_path, dry_run, source_root, output_root, compatibility_target_key, policy_key, target_video_codec, target_audio_codec, target_audio_channels, target_audio_channel_layout, target_subtitle_policy, policy_video_intent, desired_target_key, desired_target_version, desired_container_format, unmatched_stream_policy, verification_strictness, verification_duration_tolerance_millis, verification_mux_validation, verification_decode_all_streams, verification_keyframe_seek, verification_playback_probe, cancel_generation FROM media_job_worker_claim_next_v2()";
const MEDIA_JOB_WORKER_HEARTBEAT_V1: &str =
    "SELECT media_job_worker_heartbeat_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_WORKER_MARK_STATUS_V1: &str = "SELECT media_job_worker_mark_status_v1(media_job_public_id_input => $1, status_input => $2::media_job_status, last_error_input => $3)";
const MEDIA_JOB_WORKER_POLL_CONTROL_V1: &str = "SELECT cancel_requested, cancel_generation FROM media_job_worker_poll_control_v1(media_job_public_id_input => $1, observed_cancel_generation_input => $2)";
const MEDIA_JOB_WORKER_ACKNOWLEDGE_CANCEL_V1: &str = "SELECT media_job_worker_acknowledge_cancel_v1(media_job_public_id_input => $1, observed_cancel_generation_input => $2)";
const MEDIA_JOB_WORKER_COMPLETE_V1: &str = "SELECT media_job_worker_complete_v1(media_job_public_id_input => $1, observed_cancel_generation_input => $2)";
const MEDIA_JOB_DESIRED_TARGET_STREAM_LIST_V5: &str = "SELECT stream_key, stream_kind, semantic_role, language_code, optional, sort_order, codec, channel_count, channel_layout, audio_bitrate_bps, audio_sample_rate_hz, audio_loudness_profile, audio_dynamic_range, video_profile, video_level, video_bitrate_bps, color_primaries, color_transfer, color_space, hdr_format, title, default_disposition, forced_disposition, subtitle_placement, image_subtitle_action FROM media_job_desired_target_stream_list_v5(media_job_public_id_input => $1)";
const MEDIA_DISCOVERY_JOB_ENQUEUE_V1: &str = "SELECT media_discovery_job_enqueue_v1(actor_public_id_input => $1, media_profile_public_id_input => $2, source_path_input => $3, output_path_input => $4, source_size_bytes_input => $5, source_modified_ns_input => $6, source_sha256_input => $7)";

/// Create media job payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMediaJobInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Owning media profile public id.
    pub media_profile_public_id: Uuid,
    /// Source media path.
    pub source_path: &'a str,
    /// Optional output path.
    pub output_path: Option<&'a str>,
    /// Dry-run execution flag.
    pub dry_run: bool,
}

/// Atomic discovery fingerprint claim and job creation input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueueDiscoveredMediaJobInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Profile public id.
    pub media_profile_public_id: Uuid,
    /// Canonical source path.
    pub source_path: &'a str,
    /// Derived output path.
    pub output_path: &'a str,
    /// Stable file size observed while hashing.
    pub source_size_bytes: i64,
    /// Stable nanosecond modification timestamp observed while hashing.
    pub source_modified_ns: i64,
    /// Lowercase SHA-256 content fingerprint.
    pub source_sha256: &'a str,
}

/// Append media job verification check payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendMediaJobVerificationCheckInput<'a> {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Verification check ordering index.
    pub check_index: i32,
    /// Verification check kind.
    pub check_kind: &'a str,
    /// Verification check status.
    pub check_status: &'a str,
    /// Expected value text.
    pub expected_value: Option<&'a str>,
    /// Actual value text.
    pub actual_value: Option<&'a str>,
    /// Optional check details.
    pub details_text: Option<&'a str>,
}

/// Append media job artifact reference payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendMediaJobArtifactInput<'a> {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Artifact ordering index.
    pub artifact_index: i32,
    /// Artifact kind.
    pub artifact_kind: &'a str,
    /// Managed artifact path.
    pub artifact_path: &'a str,
    /// Artifact size in bytes.
    pub size_bytes: Option<i64>,
    /// Optional content type.
    pub content_type: Option<&'a str>,
}

/// Append media job compact audit fact payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendMediaJobCompactAuditInput<'a> {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Audit fact ordering index.
    pub audit_index: i32,
    /// Audit fact kind.
    pub fact_kind: &'a str,
    /// Audit fact text.
    pub fact_text: &'a str,
}

/// Media job listing row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobRow {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Source path.
    pub source_path: String,
    /// Output path.
    pub output_path: Option<String>,
    /// Status text.
    pub status_text: String,
    /// Dry-run job flag.
    pub dry_run: bool,
    /// Queue timestamp.
    pub queued_at: chrono::DateTime<chrono::Utc>,
    /// Start timestamp.
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Completion timestamp.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last error text.
    pub last_error: Option<String>,
}

/// Media job operation row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobOperationRow {
    /// Operation ordering index.
    pub operation_index: i32,
    /// Operation kind.
    pub operation_kind: String,
    /// Optional stream id for stream-scoped operations.
    pub stream_id: Option<i32>,
    /// Command binary.
    pub command_bin: String,
    /// Optional argument 1.
    pub arg_1: Option<String>,
    /// Optional argument 2.
    pub arg_2: Option<String>,
    /// Optional argument 3.
    pub arg_3: Option<String>,
    /// Optional argument 4.
    pub arg_4: Option<String>,
    /// Optional argument 5.
    pub arg_5: Option<String>,
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Media job compliance violation row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobViolationRow {
    /// Violation ordering index.
    pub violation_index: i32,
    /// Violation kind.
    pub violation_kind: String,
    /// Violation severity.
    pub severity: String,
    /// Optional stream id for stream-scoped violations.
    pub stream_id: Option<i32>,
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Media job plan explanation row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobPlanReasonRow {
    /// Reason ordering index.
    pub reason_index: i32,
    /// Optional candidate index for rejected/selected candidates.
    pub candidate_index: Option<i32>,
    /// Whether this reason describes the selected plan.
    pub selected: bool,
    /// Stable reason code.
    pub reason_code: String,
    /// Human-readable reason text.
    pub reason_text: String,
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Media job verification check row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobVerificationCheckRow {
    /// Verification check ordering index.
    pub check_index: i32,
    /// Verification check kind.
    pub check_kind: String,
    /// Verification check status.
    pub check_status: String,
    /// Expected value text.
    pub expected_value: Option<String>,
    /// Actual value text.
    pub actual_value: Option<String>,
    /// Optional check details.
    pub details_text: Option<String>,
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Media job artifact reference row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobArtifactRow {
    /// Artifact ordering index.
    pub artifact_index: i32,
    /// Artifact kind.
    pub artifact_kind: String,
    /// Managed artifact path.
    pub artifact_path: String,
    /// Artifact size in bytes.
    pub size_bytes: Option<i64>,
    /// Optional content type.
    pub content_type: Option<String>,
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Media job compact audit fact row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobCompactAuditRow {
    /// Audit fact ordering index.
    pub audit_index: i32,
    /// Audit fact kind.
    pub fact_kind: String,
    /// Audit fact text.
    pub fact_text: String,
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Result of one policy-driven media retention janitor run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobRetentionRunRow {
    /// Completed job shells deleted by the configured policy.
    pub completed_jobs_deleted: i32,
    /// Failed or cancelled job shells whose bulky details were pruned.
    pub failed_jobs_pruned: i32,
    /// Total bulky child rows deleted from failed or cancelled jobs.
    pub failed_detail_rows_deleted: i32,
}

/// Claimed media job row for worker processing.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct ClaimedMediaJobRow {
    /// Claimed job public id.
    pub media_job_public_id: Uuid,
    /// Owning media profile public id.
    pub media_profile_public_id: Uuid,
    /// Source media path.
    pub source_path: String,
    /// Optional generated output path.
    pub output_path: Option<String>,
    /// Whether execution must remain dry-run only.
    pub dry_run: bool,
    /// Profile source root.
    pub source_root: String,
    /// Profile output root.
    pub output_root: String,
    /// Optional compatibility target key used by the worker planner.
    pub compatibility_target_key: Option<String>,
    /// Operational policy key used by the worker planner.
    pub policy_key: String,
    /// Desired target video codec snapshotted when queued.
    pub target_video_codec: Option<String>,
    /// Desired target audio codec snapshotted when queued.
    pub target_audio_codec: Option<String>,
    /// Desired target audio channel count snapshotted when queued.
    pub target_audio_channels: Option<i32>,
    /// Desired target audio channel layout snapshotted when queued.
    pub target_audio_channel_layout: Option<String>,
    /// Desired subtitle policy snapshotted when queued.
    pub target_subtitle_policy: Option<String>,
    /// Policy video intent snapshotted when queued.
    pub policy_video_intent: Option<String>,
    /// Optional immutable desired-target key snapshotted when queued.
    pub desired_target_key: Option<String>,
    /// Optional immutable desired-target version snapshotted when queued.
    pub desired_target_version: Option<i32>,
    /// Optional desired output container format snapshotted when queued.
    pub desired_container_format: Option<String>,
    /// Unmatched-stream policy snapshotted when queued.
    pub unmatched_stream_policy: Option<String>,
    /// Verification strictness snapshotted when queued.
    pub verification_strictness: String,
    /// Maximum source/candidate duration delta snapshotted when queued.
    pub verification_duration_tolerance_millis: i64,
    /// Whether normalized mux validation was selected when queued.
    pub verification_mux_validation: MediaVerificationToggle,
    /// Whether every stream must decode without errors.
    pub verification_decode_all_streams: MediaVerificationToggle,
    /// Whether midpoint video keyframe seeking must succeed.
    pub verification_keyframe_seek: MediaVerificationToggle,
    /// Whether noninteractive playback smoke verification was selected.
    pub verification_playback_probe: MediaVerificationToggle,
    /// Durable cancellation generation observed when the worker claimed the job.
    pub cancel_generation: i64,
}

/// Worker heartbeat and cancellation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobControlRow {
    /// Whether a cancellation newer than the worker's claim is pending.
    pub cancel_requested: bool,
    /// Current durable cancellation generation.
    pub cancel_generation: i64,
}

/// Ordered desired-target stream snapshotted for one job.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobDesiredTargetStreamRow {
    /// Stable target stream key.
    pub stream_key: String,
    /// Media stream kind.
    pub stream_kind: String,
    /// Optional semantic role selector.
    pub semantic_role: Option<String>,
    /// Optional language selector.
    pub language_code: Option<String>,
    /// Whether a missing source match is acceptable.
    pub optional: bool,
    /// Final mux ordering position.
    pub sort_order: i32,
    /// Desired codec.
    pub codec: String,
    /// Optional desired audio channel count.
    pub channel_count: Option<i32>,
    /// Optional desired audio channel layout.
    pub channel_layout: Option<String>,
    /// Optional desired average audio bitrate in bits per second.
    pub audio_bitrate_bps: Option<i32>,
    /// Optional desired audio sample rate in hertz.
    pub audio_sample_rate_hz: Option<i32>,
    /// Optional desired audio loudness processing profile.
    pub audio_loudness_profile: Option<String>,
    /// Optional desired audio dynamic-range behavior.
    pub audio_dynamic_range: Option<String>,
    /// Optional desired video profile.
    pub video_profile: Option<String>,
    /// Optional desired video level.
    pub video_level: Option<String>,
    /// Optional desired average video bitrate in bits per second.
    pub video_bitrate_bps: Option<i32>,
    /// Optional desired video color primaries.
    pub color_primaries: Option<String>,
    /// Optional desired video transfer characteristic.
    pub color_transfer: Option<String>,
    /// Optional desired video color space.
    pub color_space: Option<String>,
    /// Optional desired HDR format label.
    pub hdr_format: Option<String>,
    /// Optional desired title.
    pub title: Option<String>,
    /// Desired default disposition.
    pub default_disposition: bool,
    /// Desired forced disposition.
    pub forced_disposition: bool,
    /// Desired subtitle placement for subtitle streams.
    pub subtitle_placement: Option<String>,
    /// Image-subtitle behavior for subtitle streams.
    pub image_subtitle_action: Option<String>,
}

/// Create media job row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn create_media_job(pool: &PgPool, input: &CreateMediaJobInput<'_>) -> Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(MEDIA_JOB_CREATE_V1)
        .bind(input.actor_public_id)
        .bind(input.media_profile_public_id)
        .bind(input.source_path)
        .bind(input.output_path.unwrap_or_default())
        .bind(input.dry_run)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job create"))
}

/// Atomically enqueue a discovered source only when its durable identity changed.
///
/// Returns `None` when the persisted size, modification time, and SHA-256 fingerprint are
/// unchanged. The fingerprint claim and job insert share one database transaction.
///
/// # Errors
///
/// Returns an error when validation, fingerprint persistence, or job creation fails.
pub async fn enqueue_discovered_media_job(
    pool: &PgPool,
    input: &EnqueueDiscoveredMediaJobInput<'_>,
) -> Result<Option<Uuid>> {
    sqlx::query_scalar::<_, Option<Uuid>>(MEDIA_DISCOVERY_JOB_ENQUEUE_V1)
        .bind(input.actor_public_id)
        .bind(input.media_profile_public_id)
        .bind(input.source_path)
        .bind(input.output_path)
        .bind(input.source_size_bytes)
        .bind(input.source_modified_ns)
        .bind(input.source_sha256)
        .fetch_one(pool)
        .await
        .map_err(try_op("media discovery job enqueue"))
}

/// List the immutable ordered desired-target stream snapshot for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_desired_target_streams(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobDesiredTargetStreamRow>> {
    sqlx::query_as::<_, MediaJobDesiredTargetStreamRow>(MEDIA_JOB_DESIRED_TARGET_STREAM_LIST_V5)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job desired target stream list"))
}

/// Append or update a media job phase row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_phase(
    pool: &PgPool,
    media_job_public_id: Uuid,
    phase_index: i32,
    phase_name: &str,
    phase_status_text: &str,
    details_text: Option<&str>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_PHASE_APPEND_V1)
        .bind(media_job_public_id)
        .bind(phase_index)
        .bind(phase_name)
        .bind(phase_status_text)
        .bind(details_text.unwrap_or_default())
        .execute(pool)
        .await
        .map_err(try_op("media job phase append"))?;

    Ok(())
}

/// Append or update a media job operation row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_operation(
    pool: &PgPool,
    media_job_public_id: Uuid,
    operation_index: i32,
    operation_kind: &str,
    stream_id: Option<i32>,
    command_bin: &str,
    args: [Option<&str>; 5],
) -> Result<()> {
    sqlx::query(MEDIA_JOB_OPERATION_APPEND_V1)
        .bind(media_job_public_id)
        .bind(operation_index)
        .bind(operation_kind)
        .bind(stream_id)
        .bind(command_bin)
        .bind(args[0].unwrap_or_default())
        .bind(args[1].unwrap_or_default())
        .bind(args[2].unwrap_or_default())
        .bind(args[3].unwrap_or_default())
        .bind(args[4].unwrap_or_default())
        .execute(pool)
        .await
        .map_err(try_op("media job operation append"))?;
    Ok(())
}

/// Append or update a media job compliance violation row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_violation(
    pool: &PgPool,
    media_job_public_id: Uuid,
    violation_index: i32,
    violation_kind: &str,
    severity: &str,
    stream_id: Option<i32>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_VIOLATION_APPEND_V1)
        .bind(media_job_public_id)
        .bind(violation_index)
        .bind(violation_kind)
        .bind(severity)
        .bind(stream_id)
        .execute(pool)
        .await
        .map_err(try_op("media job violation append"))?;
    Ok(())
}

/// Append or update a media job plan-reason row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_plan_reason(
    pool: &PgPool,
    media_job_public_id: Uuid,
    reason_index: i32,
    candidate_index: Option<i32>,
    selected: bool,
    reason_code: &str,
    reason_text: &str,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_PLAN_REASON_APPEND_V1)
        .bind(media_job_public_id)
        .bind(reason_index)
        .bind(candidate_index)
        .bind(selected)
        .bind(reason_code)
        .bind(reason_text)
        .execute(pool)
        .await
        .map_err(try_op("media job plan reason append"))?;
    Ok(())
}

/// Append or update a media job verification check row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_verification_check(
    pool: &PgPool,
    input: &AppendMediaJobVerificationCheckInput<'_>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_VERIFICATION_CHECK_APPEND_V1)
        .bind(input.media_job_public_id)
        .bind(input.check_index)
        .bind(input.check_kind)
        .bind(input.check_status)
        .bind(input.expected_value.unwrap_or_default())
        .bind(input.actual_value.unwrap_or_default())
        .bind(input.details_text.unwrap_or_default())
        .execute(pool)
        .await
        .map_err(try_op("media job verification check append"))?;
    Ok(())
}

/// Append or update a media job artifact reference row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_artifact(
    pool: &PgPool,
    input: &AppendMediaJobArtifactInput<'_>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_ARTIFACT_APPEND_V1)
        .bind(input.media_job_public_id)
        .bind(input.artifact_index)
        .bind(input.artifact_kind)
        .bind(input.artifact_path)
        .bind(input.size_bytes)
        .bind(input.content_type.unwrap_or_default())
        .execute(pool)
        .await
        .map_err(try_op("media job artifact append"))?;
    Ok(())
}

/// Append or update a media job compact audit fact row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_compact_audit(
    pool: &PgPool,
    input: &AppendMediaJobCompactAuditInput<'_>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_COMPACT_AUDIT_APPEND_V1)
        .bind(input.media_job_public_id)
        .bind(input.audit_index)
        .bind(input.fact_kind)
        .bind(input.fact_text)
        .execute(pool)
        .await
        .map_err(try_op("media job compact audit append"))?;
    Ok(())
}

/// List media jobs for profile and optional status.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_jobs(
    pool: &PgPool,
    media_profile_public_id: Uuid,
    status_text: Option<&str>,
) -> Result<Vec<MediaJobRow>> {
    sqlx::query_as::<_, MediaJobRow>(MEDIA_JOB_LIST_V1)
        .bind(media_profile_public_id)
        .bind(status_text)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job list"))
}

/// List media job operations for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_operations(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobOperationRow>> {
    sqlx::query_as::<_, MediaJobOperationRow>(MEDIA_JOB_OPERATION_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job operation list"))
}

/// List media job compliance violations for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_violations(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobViolationRow>> {
    sqlx::query_as::<_, MediaJobViolationRow>(MEDIA_JOB_VIOLATION_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job violation list"))
}

/// List media job plan reasons for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_plan_reasons(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobPlanReasonRow>> {
    sqlx::query_as::<_, MediaJobPlanReasonRow>(MEDIA_JOB_PLAN_REASON_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job plan reason list"))
}

/// List media job verification checks for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_verification_checks(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobVerificationCheckRow>> {
    sqlx::query_as::<_, MediaJobVerificationCheckRow>(MEDIA_JOB_VERIFICATION_CHECK_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job verification check list"))
}

/// List media job artifact references for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_artifacts(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobArtifactRow>> {
    sqlx::query_as::<_, MediaJobArtifactRow>(MEDIA_JOB_ARTIFACT_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job artifact list"))
}

/// List media job compact audit facts for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_compact_audits(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobCompactAuditRow>> {
    sqlx::query_as::<_, MediaJobCompactAuditRow>(MEDIA_JOB_COMPACT_AUDIT_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job compact audit list"))
}

/// Get one media job by public id.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn get_media_job(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Option<MediaJobRow>> {
    sqlx::query_as::<_, MediaJobRow>(MEDIA_JOB_GET_V1)
        .bind(media_job_public_id)
        .fetch_optional(pool)
        .await
        .map_err(try_op("media job get"))
}

/// Cancel one queued/running/verifying media job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn cancel_media_job(pool: &PgPool, media_job_public_id: Uuid) -> Result<i64> {
    sqlx::query_scalar::<_, i64>(MEDIA_JOB_CANCEL_V2)
        .bind(media_job_public_id)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job cancel"))
}

/// Retry one failed/cancelled media job by requeueing it.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn retry_media_job(pool: &PgPool, media_job_public_id: Uuid) -> Result<()> {
    sqlx::query(MEDIA_JOB_RETRY_V1)
        .bind(media_job_public_id)
        .execute(pool)
        .await
        .map_err(try_op("media job retry"))?;
    Ok(())
}

/// Mark one queued/running/verifying media job completed.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn mark_media_job_completed(pool: &PgPool, media_job_public_id: Uuid) -> Result<()> {
    sqlx::query(MEDIA_JOB_MARK_COMPLETED_V1)
        .bind(media_job_public_id)
        .execute(pool)
        .await
        .map_err(try_op("media job mark completed"))?;
    Ok(())
}

/// Run the active completed-job and failed-diagnostic retention policies atomically.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn run_media_job_retention(
    pool: &PgPool,
    as_of: chrono::DateTime<chrono::Utc>,
) -> Result<MediaJobRetentionRunRow> {
    sqlx::query_as::<_, MediaJobRetentionRunRow>(MEDIA_JOB_RETENTION_RUN_V1)
        .bind(as_of)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job retention run"))
}

/// Claim the next queued media job for worker processing.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn media_job_worker_claim_next(pool: &PgPool) -> Result<Option<ClaimedMediaJobRow>> {
    sqlx::query_as::<_, ClaimedMediaJobRow>(MEDIA_JOB_WORKER_CLAIM_NEXT_V2)
        .fetch_optional(pool)
        .await
        .map_err(try_op("media job worker claim next"))
}

/// Refresh the worker heartbeat and read durable cancellation state.
///
/// # Errors
///
/// Returns an error when the job is no longer worker-owned or execution fails.
pub async fn media_job_worker_poll_control(
    pool: &PgPool,
    media_job_public_id: Uuid,
    observed_cancel_generation: i64,
) -> Result<MediaJobControlRow> {
    sqlx::query_as::<_, MediaJobControlRow>(MEDIA_JOB_WORKER_POLL_CONTROL_V1)
        .bind(media_job_public_id)
        .bind(observed_cancel_generation)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job worker poll control"))
}

/// Acknowledge a cancellation newer than the worker's claim and mark the job cancelled.
///
/// # Errors
///
/// Returns an error when no cancellation is pending or execution fails.
pub async fn media_job_worker_acknowledge_cancel(
    pool: &PgPool,
    media_job_public_id: Uuid,
    observed_cancel_generation: i64,
) -> Result<i64> {
    sqlx::query_scalar::<_, i64>(MEDIA_JOB_WORKER_ACKNOWLEDGE_CANCEL_V1)
        .bind(media_job_public_id)
        .bind(observed_cancel_generation)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job worker acknowledge cancel"))
}

/// Atomically complete a job or acknowledge a cancellation that won the terminal-state race.
///
/// Returns `true` when the job was cancelled and `false` when it completed.
///
/// # Errors
///
/// Returns an error when the job is no longer worker-owned or execution fails.
pub async fn media_job_worker_complete(
    pool: &PgPool,
    media_job_public_id: Uuid,
    observed_cancel_generation: i64,
) -> Result<bool> {
    sqlx::query_scalar::<_, bool>(MEDIA_JOB_WORKER_COMPLETE_V1)
        .bind(media_job_public_id)
        .bind(observed_cancel_generation)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job worker complete"))
}

/// Update the heartbeat timestamp for a running media job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn media_job_worker_heartbeat(pool: &PgPool, media_job_public_id: Uuid) -> Result<()> {
    sqlx::query(MEDIA_JOB_WORKER_HEARTBEAT_V1)
        .bind(media_job_public_id)
        .execute(pool)
        .await
        .map_err(try_op("media job worker heartbeat"))?;
    Ok(())
}

/// Mark a claimed media job with the next worker-visible status.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn media_job_worker_mark_status(
    pool: &PgPool,
    media_job_public_id: Uuid,
    status_text: &str,
    last_error: Option<&str>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_WORKER_MARK_STATUS_V1)
        .bind(media_job_public_id)
        .bind(status_text)
        .bind(last_error.unwrap_or_default())
        .execute(pool)
        .await
        .map_err(try_op("media job worker mark status"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        AppendMediaJobArtifactInput, AppendMediaJobCompactAuditInput,
        AppendMediaJobVerificationCheckInput, CreateMediaJobInput, EnqueueDiscoveredMediaJobInput,
        append_media_job_artifact, append_media_job_compact_audit, append_media_job_operation,
        append_media_job_phase, append_media_job_plan_reason, append_media_job_verification_check,
        append_media_job_violation, cancel_media_job, create_media_job,
        enqueue_discovered_media_job, get_media_job, list_media_job_artifacts,
        list_media_job_compact_audits, list_media_job_operations, list_media_job_plan_reasons,
        list_media_job_verification_checks, list_media_job_violations, list_media_jobs,
        mark_media_job_completed, media_job_worker_acknowledge_cancel, media_job_worker_claim_next,
        media_job_worker_mark_status, media_job_worker_poll_control, run_media_job_retention,
    };
    use crate::DataError;
    use crate::media::configuration::{
        UpdateMediaJobRetentionPolicyInput, UpsertMediaCompatibilityTargetInput,
        UpsertMediaPolicyProfileInput, update_media_job_retention_policy,
        upsert_media_compatibility_target, upsert_media_policy_profile,
    };
    use crate::media::profiles::{
        UpdateMediaProfileInput, UpsertMediaProfileInput, update_media_profile,
        upsert_media_profile,
    };
    use crate::media::schema_tests::{MediaTestDb, setup_media_db};
    use chrono::{Duration, Utc};
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };
    use std::{fs, path::Path};
    use uuid::Uuid;

    fn closed_pool_options() -> PgConnectOptions {
        PgConnectOptions::new()
            .host("127.0.0.1")
            .port(9)
            .username("revaer")
            .password(
                &['r', 'e', 'v', 'a', 'e', 'r']
                    .into_iter()
                    .collect::<String>(),
            )
            .database("revaer")
    }

    async fn closed_pool() -> sqlx::PgPool {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy_with(closed_pool_options());
        pool.close().await;
        pool
    }

    async fn append_and_assert_plan_reason(pool: &PgPool, job_id: Uuid) -> anyhow::Result<()> {
        append_media_job_plan_reason(
            pool,
            job_id,
            0,
            Some(0),
            true,
            "least_cost_selected",
            "Selected the least-cost compliant candidate.",
        )
        .await?;
        let plan_reasons = list_media_job_plan_reasons(pool, job_id).await?;
        assert_eq!(plan_reasons.len(), 1);
        assert_eq!(plan_reasons[0].reason_index, 0);
        assert_eq!(plan_reasons[0].candidate_index, Some(0));
        assert!(plan_reasons[0].selected);
        assert_eq!(plan_reasons[0].reason_code, "least_cost_selected");
        assert_eq!(
            plan_reasons[0].reason_text,
            "Selected the least-cost compliant candidate."
        );
        Ok(())
    }

    async fn append_and_assert_verification_check(
        pool: &PgPool,
        job_id: Uuid,
    ) -> anyhow::Result<()> {
        append_media_job_verification_check(
            pool,
            &AppendMediaJobVerificationCheckInput {
                media_job_public_id: job_id,
                check_index: 0,
                check_kind: "duration",
                check_status: "passed",
                expected_value: Some("3600.0"),
                actual_value: Some("3599.9"),
                details_text: Some("within tolerance"),
            },
        )
        .await?;
        let verification_checks = list_media_job_verification_checks(pool, job_id).await?;
        assert_eq!(verification_checks.len(), 1);
        assert_eq!(verification_checks[0].check_index, 0);
        assert_eq!(verification_checks[0].check_kind, "duration");
        assert_eq!(verification_checks[0].check_status, "passed");
        assert_eq!(
            verification_checks[0].expected_value.as_deref(),
            Some("3600.0")
        );
        assert_eq!(
            verification_checks[0].actual_value.as_deref(),
            Some("3599.9")
        );
        assert_eq!(
            verification_checks[0].details_text.as_deref(),
            Some("within tolerance")
        );
        Ok(())
    }

    async fn append_and_assert_artifact_and_audit(
        pool: &PgPool,
        job_id: Uuid,
    ) -> anyhow::Result<()> {
        append_media_job_artifact(
            pool,
            &AppendMediaJobArtifactInput {
                media_job_public_id: job_id,
                artifact_index: 0,
                artifact_kind: "ffprobe_json",
                artifact_path: "jobs/abc/ffprobe.json",
                size_bytes: Some(2048),
                content_type: Some("application/json"),
            },
        )
        .await?;
        let artifacts = list_media_job_artifacts(pool, job_id).await?;
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].artifact_kind, "ffprobe_json");
        assert_eq!(artifacts[0].artifact_path, "jobs/abc/ffprobe.json");
        let oversized_artifact_path = format!("jobs/{}", "x".repeat(1020));
        let oversized_artifact = append_media_job_artifact(
            pool,
            &AppendMediaJobArtifactInput {
                media_job_public_id: job_id,
                artifact_index: 1,
                artifact_kind: "ffprobe_json",
                artifact_path: &oversized_artifact_path,
                size_bytes: Some(2048),
                content_type: Some("application/json"),
            },
        )
        .await;
        assert!(oversized_artifact.is_err());
        let unmanaged_artifact = append_media_job_artifact(
            pool,
            &AppendMediaJobArtifactInput {
                media_job_public_id: job_id,
                artifact_index: 2,
                artifact_kind: "ffprobe_json",
                artifact_path: "../escape.json",
                size_bytes: Some(2048),
                content_type: Some("application/json"),
            },
        )
        .await;
        assert!(unmanaged_artifact.is_err());

        append_media_job_compact_audit(
            pool,
            &AppendMediaJobCompactAuditInput {
                media_job_public_id: job_id,
                audit_index: 0,
                fact_kind: "replacement",
                fact_text: "source preserved before replace",
            },
        )
        .await?;
        let audits = list_media_job_compact_audits(pool, job_id).await?;
        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].fact_kind, "replacement");
        assert_eq!(audits[0].fact_text, "source preserved before replace");
        let oversized_fact_text = "x".repeat(1025);
        let oversized_audit = append_media_job_compact_audit(
            pool,
            &AppendMediaJobCompactAuditInput {
                media_job_public_id: job_id,
                audit_index: 1,
                fact_kind: "replacement",
                fact_text: &oversized_fact_text,
            },
        )
        .await;
        assert!(oversized_audit.is_err());
        Ok(())
    }

    async fn upsert_retention_profile(
        db: &MediaTestDb,
        profile_key: &str,
        root: &str,
    ) -> anyhow::Result<Uuid> {
        let source_root = format!("/input/{root}");
        let output_root = format!("/output/{root}");
        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key,
                source_root: &source_root,
                output_root: &output_root,
                dry_run_only: true,
                retention_days: 3650,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        Ok(profile_id)
    }

    async fn create_cancelled_job_with_audit(
        db: &MediaTestDb,
        profile_id: Uuid,
        root: &str,
        file_name: &str,
    ) -> anyhow::Result<Uuid> {
        let source_path = format!("/input/{root}/{file_name}");
        let output_path = format!("/output/{root}/{file_name}");
        let job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: &source_path,
                output_path: Some(&output_path),
                dry_run: true,
            },
        )
        .await?;
        append_and_assert_artifact_and_audit(db.pool(), job_id).await?;
        cancel_media_job(db.pool(), job_id).await?;
        Ok(job_id)
    }

    async fn assert_create_job_path_rejected(
        pool: &PgPool,
        actor_public_id: Uuid,
        profile_id: Uuid,
        source_path: &str,
        output_path: Option<&str>,
        expected_detail: &str,
    ) -> anyhow::Result<()> {
        let err = create_media_job(
            pool,
            &CreateMediaJobInput {
                actor_public_id,
                media_profile_public_id: profile_id,
                source_path,
                output_path,
                dry_run: true,
            },
        )
        .await
        .expect_err("media job path should be rejected");
        assert!(matches!(err, DataError::QueryFailed { .. }));
        assert_eq!(err.database_detail(), Some(expected_detail));
        Ok(())
    }

    #[tokio::test]
    async fn create_media_job_rejects_paths_outside_profile_roots() -> anyhow::Result<()> {
        let db = match setup_media_db("create_media_job_rejects_paths_outside_profile_roots").await
        {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
            }
        };
        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "job-path-bounds",
                source_root: "/input/bounds",
                output_root: "/output/bounds",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;

        assert_create_job_path_rejected(
            db.pool(),
            db.system_user_public_id,
            profile_id,
            "/input/other/movie.mkv",
            Some("/output/bounds/movie.mkv"),
            "media_job_source_path_outside_profile_root",
        )
        .await?;
        assert_create_job_path_rejected(
            db.pool(),
            db.system_user_public_id,
            profile_id,
            "/input/bounds/movie.mkv",
            Some("/output/other/movie.mkv"),
            "media_job_output_path_outside_profile_root",
        )
        .await?;
        assert_create_job_path_rejected(
            db.pool(),
            db.system_user_public_id,
            profile_id,
            "/input/bounds/../outside/movie.mkv",
            Some("/output/bounds/movie.mkv"),
            "media_job_source_path_outside_profile_root",
        )
        .await?;

        let wildcard_profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "job-path-wildcard-bounds",
                source_root: "/input/bounds_1",
                output_root: "/output/bounds_1",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        assert_create_job_path_rejected(
            db.pool(),
            db.system_user_public_id,
            wildcard_profile_id,
            "/input/boundsx1/movie.mkv",
            Some("/output/bounds_1/movie.mkv"),
            "media_job_source_path_outside_profile_root",
        )
        .await?;
        assert_create_job_path_rejected(
            db.pool(),
            db.system_user_public_id,
            wildcard_profile_id,
            "/input/bounds_1/movie.mkv",
            Some("/output/boundsx1/movie.mkv"),
            "media_job_output_path_outside_profile_root",
        )
        .await?;

        Ok(())
    }

    #[test]
    fn migration_guards_media_job_path_bounds_validation() {
        let migration_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let migration_text = fs::read_dir(migration_root)
            .into_iter()
            .flat_map(|entries| entries.filter_map(Result::ok))
            .filter_map(|entry| fs::read_to_string(entry.path()).ok())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            migration_text.contains("media_job_normalized_absolute_path_v1"),
            "media job path normalization helper must be present in migrations"
        );
        assert!(
            migration_text.contains("media_job_validate_path_within_root_v1"),
            "media job path root-bound validator must be present in migrations"
        );
        assert!(
            migration_text.contains("media_job_source_path_outside_profile_root"),
            "source-path rejection detail must be present in migrations"
        );
        assert!(
            migration_text.contains("media_job_output_path_outside_profile_root"),
            "output-path rejection detail must be present in migrations"
        );
    }

    #[tokio::test]
    async fn create_and_list_media_job() -> anyhow::Result<()> {
        let db = match setup_media_db("create_and_list_media_job").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
            }
        };
        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-jobs",
                source_root: "/input/tv",
                output_root: "/output/tv",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;

        let job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/tv/show.mkv",
                output_path: Some("/output/tv/show.mkv"),
                dry_run: true,
            },
        )
        .await?;

        append_media_job_phase(
            db.pool(),
            job_id,
            0,
            "planning",
            "queued",
            Some("scheduled"),
        )
        .await?;

        append_media_job_operation(
            db.pool(),
            job_id,
            0,
            "remux",
            None,
            "ffmpeg",
            [
                Some("-i"),
                Some("/input/tv/show.mkv"),
                Some("-c"),
                Some("copy"),
                None,
            ],
        )
        .await?;

        append_media_job_violation(
            db.pool(),
            job_id,
            0,
            "video_codec_mismatch",
            "high",
            Some(0),
        )
        .await?;

        let rows = list_media_jobs(db.pool(), profile_id, Some("queued")).await?;
        assert!(rows.iter().any(|item| item.media_job_public_id == job_id));

        let job = get_media_job(db.pool(), job_id).await?;
        assert!(job.is_some());
        let Some(job) = job else {
            return Ok(());
        };
        assert_eq!(job.media_job_public_id, job_id);

        let operations = list_media_job_operations(db.pool(), job_id).await?;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].operation_index, 0);
        assert_eq!(operations[0].operation_kind, "remux");
        assert_eq!(operations[0].command_bin, "ffmpeg");

        let violations = list_media_job_violations(db.pool(), job_id).await?;
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_index, 0);
        assert_eq!(violations[0].violation_kind, "video_codec_mismatch");
        assert_eq!(violations[0].severity, "high");
        assert_eq!(violations[0].stream_id, Some(0));

        append_and_assert_plan_reason(db.pool(), job_id).await?;

        append_and_assert_verification_check(db.pool(), job_id).await?;
        append_and_assert_artifact_and_audit(db.pool(), job_id).await?;
        Ok(())
    }

    #[tokio::test]
    async fn running_job_cancellation_is_durable_and_worker_acknowledged() -> anyhow::Result<()> {
        let db = match setup_media_db("running_job_cancellation_is_durable").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
            }
        };
        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "cancel-running",
                source_root: "/input/cancel-running",
                output_root: "/output/cancel-running",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        let job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/cancel-running/movie.mkv",
                output_path: Some("/output/cancel-running/movie.mkv"),
                dry_run: true,
            },
        )
        .await?;

        let claimed = media_job_worker_claim_next(db.pool()).await?;
        let Some(claimed) = claimed else {
            return Err(anyhow::anyhow!("worker did not claim queued job"));
        };
        assert_eq!(
            claimed.media_job_public_id, job_id,
            "worker should claim the queued job before cancellation"
        );
        assert_eq!(claimed.cancel_generation, 0);

        let requested_generation = cancel_media_job(db.pool(), job_id).await?;
        assert_eq!(requested_generation, 1);
        let control =
            media_job_worker_poll_control(db.pool(), job_id, claimed.cancel_generation).await?;
        assert!(control.cancel_requested);
        assert_eq!(control.cancel_generation, requested_generation);
        assert!(
            media_job_worker_mark_status(db.pool(), job_id, "completed", None)
                .await
                .is_err(),
            "a pending cancellation must fence successful completion"
        );
        let acknowledged_generation =
            media_job_worker_acknowledge_cancel(db.pool(), job_id, claimed.cancel_generation)
                .await?;
        assert_eq!(acknowledged_generation, requested_generation);
        let job = get_media_job(db.pool(), job_id).await?;
        let Some(job) = job else {
            return Err(anyhow::anyhow!("cancelled job missing"));
        };
        assert_eq!(job.status_text, "cancelled");
        Ok(())
    }

    async fn upsert_strict_snapshot_policy(
        pool: &sqlx::PgPool,
        actor_public_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        upsert_media_policy_profile(
            pool,
            UpsertMediaPolicyProfileInput {
                actor_public_id,
                policy_key: "safe_dry_run",
                version: 1,
                display_name: "Snapshot strict",
                video_intent: "general",
                verification_strictness: "strict",
                verification_duration_tolerance_millis: 25,
                verification_mux_validation: true.into(),
                verification_decode_all_streams: true.into(),
                verification_keyframe_seek: true.into(),
                verification_playback_probe: true.into(),
            },
        )
        .await?;
        Ok(())
    }

    async fn replace_with_fast_policy(
        pool: &sqlx::PgPool,
        actor_public_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        upsert_media_policy_profile(
            pool,
            UpsertMediaPolicyProfileInput {
                actor_public_id,
                policy_key: "safe_dry_run",
                version: 1,
                display_name: "Catalog changed after enqueue",
                video_intent: "general",
                verification_strictness: "fast",
                verification_duration_tolerance_millis: 5_000,
                verification_mux_validation: false.into(),
                verification_decode_all_streams: false.into(),
                verification_keyframe_seek: false.into(),
                verification_playback_probe: false.into(),
            },
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn worker_claim_uses_enqueue_time_profile_intent() -> anyhow::Result<()> {
        let db = match setup_media_db("worker_claim_uses_enqueue_time_profile_intent").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
            }
        };
        upsert_media_compatibility_target(
            db.pool(),
            UpsertMediaCompatibilityTargetInput {
                actor_public_id: db.system_user_public_id,
                compatibility_target_key: "intent-stereo",
                version: 1,
                display_name: "Intent Stereo",
                video_codec: "hevc",
                audio_codec: "aac",
                audio_channels: Some(2),
                audio_channel_layout: Some("stereo"),
                subtitle_policy: "selected",
            },
        )
        .await?;
        upsert_strict_snapshot_policy(db.pool(), db.system_user_public_id).await?;
        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "intent-snapshot",
                source_root: "/input/original",
                output_root: "/output/original",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: Some("intent-stereo"),
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        let job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/original/movie.mkv",
                output_path: Some("/output/original/movie.mkv"),
                dry_run: true,
            },
        )
        .await?;

        replace_with_fast_policy(db.pool(), db.system_user_public_id).await?;

        update_media_profile(
            db.pool(),
            &UpdateMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_root: Some("/input/changed"),
                output_root: Some("/output/changed"),
                dry_run_only: None,
                retention_days: None,
                compatibility_target_key: Some("plex-general-hevc-aac"),
                policy_key: Some("archival"),
                watcher_enabled: None,
                schedule_enabled: None,
                schedule_interval_minutes: None,
            },
        )
        .await?;

        let claimed = media_job_worker_claim_next(db.pool()).await?;
        let Some(claimed) = claimed else {
            return Err(anyhow::anyhow!("expected queued job to be claimed"));
        };
        assert_eq!(claimed.media_job_public_id, job_id);
        assert_eq!(claimed.source_root, "/input/original");
        assert_eq!(claimed.output_root, "/output/original");
        assert_eq!(
            claimed.compatibility_target_key.as_deref(),
            Some("intent-stereo")
        );
        assert_eq!(claimed.policy_key, "safe_dry_run");
        assert_eq!(claimed.target_video_codec.as_deref(), Some("hevc"));
        assert_eq!(claimed.target_audio_codec.as_deref(), Some("aac"));
        assert_eq!(claimed.target_audio_channels, Some(2));
        assert_eq!(
            claimed.target_audio_channel_layout.as_deref(),
            Some("stereo")
        );
        assert_eq!(claimed.target_subtitle_policy.as_deref(), Some("selected"));
        assert_eq!(claimed.policy_video_intent.as_deref(), Some("general"));
        assert_eq!(claimed.verification_strictness, "strict");
        assert_eq!(claimed.verification_duration_tolerance_millis, 25);
        assert!(claimed.verification_mux_validation.enabled());
        assert!(claimed.verification_decode_all_streams.enabled());
        assert!(claimed.verification_keyframe_seek.enabled());
        assert!(claimed.verification_playback_probe.enabled());
        Ok(())
    }

    #[tokio::test]
    async fn retention_count_mode_deletes_only_completed_jobs_beyond_limit_and_preserves_audit()
    -> anyhow::Result<()> {
        let Some(db) = setup_media_db("cleanup_completed_media_jobs").await? else {
            return Ok(());
        };

        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "count-retention",
                source_root: "/input/count",
                output_root: "/output/count",
                dry_run_only: true,
                retention_days: 1,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        let expired_job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/count/older.mkv",
                output_path: Some("/output/count/older.mkv"),
                dry_run: true,
            },
        )
        .await?;
        let retained_completed_job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/count/newer.mkv",
                output_path: Some("/output/count/newer.mkv"),
                dry_run: true,
            },
        )
        .await?;
        let queued_job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/count/queued.mkv",
                output_path: Some("/output/count/queued.mkv"),
                dry_run: true,
            },
        )
        .await?;

        mark_media_job_completed(db.pool(), expired_job_id).await?;
        mark_media_job_completed(db.pool(), retained_completed_job_id).await?;
        append_media_job_compact_audit(
            db.pool(),
            &AppendMediaJobCompactAuditInput {
                media_job_public_id: expired_job_id,
                audit_index: 0,
                fact_kind: "replacement",
                fact_text: "source replaced after verified candidate",
            },
        )
        .await?;
        update_media_job_retention_policy(
            db.pool(),
            UpdateMediaJobRetentionPolicyInput {
                actor_public_id: db.system_user_public_id,
                completed_enabled: true,
                completed_mode: "count".to_string(),
                completed_limit: 1,
                failed_diagnostic_enabled: false,
                failed_diagnostic_mode: "age".to_string(),
                failed_diagnostic_limit: 30,
            },
        )
        .await?;

        let outcome = run_media_job_retention(db.pool(), Utc::now()).await?;

        assert_eq!(outcome.completed_jobs_deleted, 1);
        assert_eq!(outcome.failed_jobs_pruned, 0);
        assert!(get_media_job(db.pool(), expired_job_id).await?.is_none());
        assert!(
            get_media_job(db.pool(), retained_completed_job_id)
                .await?
                .is_some()
        );
        assert!(get_media_job(db.pool(), queued_job_id).await?.is_some());
        assert_eq!(
            list_media_job_compact_audits(db.pool(), expired_job_id)
                .await?
                .len(),
            1
        );
        Ok(())
    }

    #[tokio::test]
    async fn cleanup_failed_terminal_media_diagnostics_removes_only_expired_diagnostics()
    -> anyhow::Result<()> {
        let Some(db) = setup_media_db("cleanup_failed_terminal_media_diagnostics").await? else {
            return Ok(());
        };
        let profile_id =
            upsert_retention_profile(&db, "diagnostic-retention", "diagnostics").await?;
        let cancelled_job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/diagnostics/cancelled.mkv",
                output_path: Some("/output/diagnostics/cancelled.mkv"),
                dry_run: true,
            },
        )
        .await?;
        append_media_job_violation(
            db.pool(),
            cancelled_job_id,
            0,
            "video_codec_mismatch",
            "high",
            Some(0),
        )
        .await?;
        append_and_assert_plan_reason(db.pool(), cancelled_job_id).await?;
        append_and_assert_verification_check(db.pool(), cancelled_job_id).await?;
        append_and_assert_artifact_and_audit(db.pool(), cancelled_job_id).await?;
        cancel_media_job(db.pool(), cancelled_job_id).await?;

        let completed_job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/diagnostics/completed.mkv",
                output_path: Some("/output/diagnostics/completed.mkv"),
                dry_run: true,
            },
        )
        .await?;
        append_and_assert_artifact_and_audit(db.pool(), completed_job_id).await?;
        mark_media_job_completed(db.pool(), completed_job_id).await?;

        update_media_job_retention_policy(
            db.pool(),
            UpdateMediaJobRetentionPolicyInput {
                actor_public_id: db.system_user_public_id,
                completed_enabled: false,
                completed_mode: "age".to_string(),
                completed_limit: 30,
                failed_diagnostic_enabled: true,
                failed_diagnostic_mode: "age".to_string(),
                failed_diagnostic_limit: 30,
            },
        )
        .await?;
        let outcome = run_media_job_retention(db.pool(), Utc::now() + Duration::days(31)).await?;

        assert_eq!(outcome.completed_jobs_deleted, 0);
        assert_eq!(outcome.failed_jobs_pruned, 1);
        assert_eq!(outcome.failed_detail_rows_deleted, 4);
        assert!(get_media_job(db.pool(), cancelled_job_id).await?.is_some());
        assert!(
            list_media_job_violations(db.pool(), cancelled_job_id)
                .await?
                .is_empty()
        );
        assert!(
            list_media_job_plan_reasons(db.pool(), cancelled_job_id)
                .await?
                .is_empty()
        );
        assert!(
            list_media_job_verification_checks(db.pool(), cancelled_job_id)
                .await?
                .is_empty()
        );
        assert!(
            list_media_job_artifacts(db.pool(), cancelled_job_id)
                .await?
                .is_empty()
        );
        assert_eq!(
            list_media_job_compact_audits(db.pool(), cancelled_job_id)
                .await?
                .len(),
            1
        );
        assert_eq!(
            list_media_job_artifacts(db.pool(), completed_job_id)
                .await?
                .len(),
            1
        );
        assert_eq!(
            list_media_job_compact_audits(db.pool(), completed_job_id)
                .await?
                .len(),
            1
        );
        Ok(())
    }

    #[tokio::test]
    async fn retention_disabled_is_a_noop_and_failed_count_mode_prunes_only_excess_diagnostics()
    -> anyhow::Result<()> {
        let Some(db) = setup_media_db("retention_disabled_and_failed_count").await? else {
            return Ok(());
        };
        let profile_id =
            upsert_retention_profile(&db, "failed-count-retention", "failed-count").await?;
        let older_job_id =
            create_cancelled_job_with_audit(&db, profile_id, "failed-count", "older.mkv").await?;
        let newer_job_id =
            create_cancelled_job_with_audit(&db, profile_id, "failed-count", "newer.mkv").await?;

        update_media_job_retention_policy(
            db.pool(),
            UpdateMediaJobRetentionPolicyInput {
                actor_public_id: db.system_user_public_id,
                completed_enabled: false,
                completed_mode: "count".to_string(),
                completed_limit: 1,
                failed_diagnostic_enabled: false,
                failed_diagnostic_mode: "count".to_string(),
                failed_diagnostic_limit: 1,
            },
        )
        .await?;
        let disabled_outcome = run_media_job_retention(db.pool(), Utc::now()).await?;
        assert_eq!(disabled_outcome.completed_jobs_deleted, 0);
        assert_eq!(disabled_outcome.failed_jobs_pruned, 0);
        assert_eq!(disabled_outcome.failed_detail_rows_deleted, 0);
        assert_eq!(
            list_media_job_artifacts(db.pool(), older_job_id)
                .await?
                .len(),
            1
        );
        assert_eq!(
            list_media_job_artifacts(db.pool(), newer_job_id)
                .await?
                .len(),
            1
        );

        update_media_job_retention_policy(
            db.pool(),
            UpdateMediaJobRetentionPolicyInput {
                actor_public_id: db.system_user_public_id,
                completed_enabled: false,
                completed_mode: "count".to_string(),
                completed_limit: 1,
                failed_diagnostic_enabled: true,
                failed_diagnostic_mode: "count".to_string(),
                failed_diagnostic_limit: 1,
            },
        )
        .await?;
        let count_outcome = run_media_job_retention(db.pool(), Utc::now()).await?;
        assert_eq!(count_outcome.completed_jobs_deleted, 0);
        assert_eq!(count_outcome.failed_jobs_pruned, 1);
        assert_eq!(count_outcome.failed_detail_rows_deleted, 1);
        assert!(
            list_media_job_artifacts(db.pool(), older_job_id)
                .await?
                .is_empty()
        );
        assert_eq!(
            list_media_job_artifacts(db.pool(), newer_job_id)
                .await?
                .len(),
            1
        );
        assert_eq!(
            list_media_job_compact_audits(db.pool(), older_job_id)
                .await?
                .len(),
            1
        );
        assert_eq!(
            list_media_job_compact_audits(db.pool(), newer_job_id)
                .await?
                .len(),
            1
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_job_queries_surface_query_errors_without_database() {
        let pool = closed_pool().await;
        let profile_id = Uuid::new_v4();
        let job_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();

        let create = create_media_job(
            &pool,
            &CreateMediaJobInput {
                actor_public_id: actor_id,
                media_profile_public_id: profile_id,
                source_path: "/input/movie.mkv",
                output_path: None,
                dry_run: true,
            },
        )
        .await;
        assert!(create.is_err());

        let append = append_media_job_phase(&pool, job_id, 0, "plan", "queued", None).await;
        assert!(append.is_err());

        let list = list_media_jobs(&pool, profile_id, Some("queued")).await;
        assert!(list.is_err());

        let get = get_media_job(&pool, job_id).await;
        assert!(get.is_err());

        let operations = list_media_job_operations(&pool, job_id).await;
        assert!(operations.is_err());

        let append_violation =
            append_media_job_violation(&pool, job_id, 0, "codec_mismatch", "high", Some(0)).await;
        assert!(append_violation.is_err());

        let violations = list_media_job_violations(&pool, job_id).await;
        assert!(violations.is_err());

        let append_reason = append_media_job_plan_reason(
            &pool,
            job_id,
            0,
            Some(0),
            true,
            "least_cost_selected",
            "Selected candidate.",
        )
        .await;
        assert!(append_reason.is_err());

        let reasons = list_media_job_plan_reasons(&pool, job_id).await;
        assert!(reasons.is_err());

        let append_check = append_media_job_verification_check(
            &pool,
            &AppendMediaJobVerificationCheckInput {
                media_job_public_id: job_id,
                check_index: 0,
                check_kind: "duration",
                check_status: "passed",
                expected_value: Some("3600.0"),
                actual_value: Some("3599.9"),
                details_text: Some("within tolerance"),
            },
        )
        .await;
        assert!(append_check.is_err());

        let checks = list_media_job_verification_checks(&pool, job_id).await;
        assert!(checks.is_err());

        let append_artifact = append_media_job_artifact(
            &pool,
            &AppendMediaJobArtifactInput {
                media_job_public_id: job_id,
                artifact_index: 0,
                artifact_kind: "ffprobe_json",
                artifact_path: "jobs/abc/ffprobe.json",
                size_bytes: Some(2048),
                content_type: Some("application/json"),
            },
        )
        .await;
        assert!(append_artifact.is_err());
        assert!(list_media_job_artifacts(&pool, job_id).await.is_err());

        let append_audit = append_media_job_compact_audit(
            &pool,
            &AppendMediaJobCompactAuditInput {
                media_job_public_id: job_id,
                audit_index: 0,
                fact_kind: "replacement",
                fact_text: "source preserved before replace",
            },
        )
        .await;
        assert!(append_audit.is_err());
        assert!(list_media_job_compact_audits(&pool, job_id).await.is_err());
    }

    #[tokio::test]
    async fn discovery_fingerprints_are_atomic_durable_and_change_sensitive() -> anyhow::Result<()>
    {
        let Some(db) = setup_media_db("discovery_fingerprints").await? else {
            return Ok(());
        };
        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "fingerprint-profile",
                source_root: "/input/fingerprint",
                output_root: "/output/fingerprint",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: true,
                schedule_enabled: true,
                schedule_interval_minutes: Some(60),
            },
        )
        .await?;
        let source_path = "/input/fingerprint/movie.webm";
        let output_path = "/output/fingerprint/movie.webm";
        let first = EnqueueDiscoveredMediaJobInput {
            actor_public_id: db.system_user_public_id,
            media_profile_public_id: profile_id,
            source_path,
            output_path,
            source_size_bytes: 10,
            source_modified_ns: 100,
            source_sha256: &"a".repeat(64),
        };

        assert!(
            enqueue_discovered_media_job(db.pool(), &first)
                .await?
                .is_some()
        );
        assert_eq!(enqueue_discovered_media_job(db.pool(), &first).await?, None);

        let modified_time = EnqueueDiscoveredMediaJobInput {
            source_modified_ns: 101,
            ..first.clone()
        };
        assert!(
            enqueue_discovered_media_job(db.pool(), &modified_time)
                .await?
                .is_some()
        );
        let modified_content = EnqueueDiscoveredMediaJobInput {
            source_sha256: &"b".repeat(64),
            ..modified_time
        };
        assert!(
            enqueue_discovered_media_job(db.pool(), &modified_content)
                .await?
                .is_some()
        );

        let failed_claim = EnqueueDiscoveredMediaJobInput {
            source_modified_ns: 102,
            output_path: "/outside/movie.webm",
            ..modified_content.clone()
        };
        assert!(
            enqueue_discovered_media_job(db.pool(), &failed_claim)
                .await
                .is_err()
        );
        let retry = EnqueueDiscoveredMediaJobInput {
            output_path,
            ..failed_claim
        };
        assert!(
            enqueue_discovered_media_job(db.pool(), &retry)
                .await?
                .is_some()
        );

        let jobs = list_media_jobs(db.pool(), profile_id, None).await?;
        assert_eq!(jobs.len(), 4);
        assert!(jobs.iter().all(|job| job.dry_run));
        Ok(())
    }
}
