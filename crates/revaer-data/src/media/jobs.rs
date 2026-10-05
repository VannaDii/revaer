//! Stored-procedure access for media job lifecycle.

use crate::error::{Result, try_op};
use sqlx::PgPool;
use uuid::Uuid;

use super::configuration::MediaVerificationToggle;

const MEDIA_JOB_CREATE_V1: &str = "SELECT media_job_create_v1(actor_public_id_input => $1, media_profile_public_id_input => $2, source_path_input => $3, output_path_input => $4, dry_run_input => $5)";
const MEDIA_JOB_PHASE_APPEND_V1: &str = "SELECT media_job_phase_append_v1(media_job_public_id_input => $1, claim_generation_input => $2, phase_index_input => $3, phase_name_input => $4, phase_status_input => $5, details_text_input => $6)";
const MEDIA_JOB_PHASE_LIST_V1: &str = "SELECT phase_index, phase_name, phase_status::text AS phase_status, details_text, created_at FROM media_job_phase_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_OPERATION_APPEND_V1: &str = "SELECT media_job_operation_append_v1(media_job_public_id_input => $1, claim_generation_input => $2, operation_index_input => $3, operation_kind_input => $4, stream_id_input => $5, command_bin_input => $6, arg_1_input => $7, arg_2_input => $8, arg_3_input => $9, arg_4_input => $10, arg_5_input => $11)";
const MEDIA_JOB_OPERATION_LIST_V1: &str = "SELECT operation_index, operation_kind, stream_id, command_bin, arg_1, arg_2, arg_3, arg_4, arg_5, created_at FROM media_job_operation_list_v1(media_job_public_id_input => $1) LIMIT 1025";
const MEDIA_JOB_VIOLATION_APPEND_V1: &str = "SELECT media_job_violation_append_v1(media_job_public_id_input => $1, claim_generation_input => $2, violation_index_input => $3, violation_kind_input => $4, severity_input => $5, stream_id_input => $6)";
const MEDIA_JOB_VIOLATION_LIST_V1: &str = "SELECT violation_index, violation_kind, severity, stream_id, created_at FROM media_job_violation_list_v1(media_job_public_id_input => $1) LIMIT 1025";
const MEDIA_JOB_PLAN_REASON_APPEND_V1: &str = "SELECT media_job_plan_reason_append_v1(media_job_public_id_input => $1, claim_generation_input => $2, reason_index_input => $3, candidate_index_input => $4, selected_input => $5, reason_code_input => $6, reason_text_input => $7)";
const MEDIA_JOB_PLAN_REASON_LIST_V1: &str = "SELECT reason_index, candidate_index, selected, reason_code, reason_text, created_at FROM media_job_plan_reason_list_v1(media_job_public_id_input => $1) LIMIT 1025";
const MEDIA_JOB_VERIFICATION_CHECK_APPEND_V1: &str = "SELECT media_job_verification_check_append_v1(media_job_public_id_input => $1, claim_generation_input => $2, check_index_input => $3, check_kind_input => $4, check_status_input => $5, expected_value_input => $6, actual_value_input => $7, details_text_input => $8)";
const MEDIA_JOB_VERIFICATION_CHECK_LIST_V1: &str = "SELECT check_index, check_kind, check_status, expected_value, actual_value, details_text, created_at FROM media_job_verification_check_list_v1(media_job_public_id_input => $1) LIMIT 1025";
const MEDIA_JOB_ARTIFACT_APPEND_V1: &str = "SELECT media_job_artifact_append_v1(media_job_public_id_input => $1, claim_generation_input => $2, artifact_index_input => $3, artifact_kind_input => $4, artifact_path_input => $5, size_bytes_input => $6, content_type_input => $7)";
const MEDIA_JOB_ARTIFACT_LIST_V1: &str = "SELECT artifact_index, artifact_kind, artifact_path, size_bytes, content_type, created_at FROM media_job_artifact_list_v1(media_job_public_id_input => $1) LIMIT 1025";
const MEDIA_JOB_COMPACT_AUDIT_APPEND_V1: &str = "SELECT media_job_compact_audit_append_v1(media_job_public_id_input => $1, claim_generation_input => $2, audit_index_input => $3, fact_kind_input => $4, fact_text_input => $5)";
const MEDIA_JOB_ATTEMPT_IDENTITY_LIST_V1: &str = "SELECT attempt_number, claim_generation, is_current FROM media_job_attempt_list_v1(media_job_public_id_input => $1) LIMIT 1025";
const MEDIA_JOB_COMPACT_AUDIT_LIST_V1: &str = "SELECT attempt_number, is_current, audit_index, fact_kind, fact_text, created_at FROM media_job_compact_audit_list_v1(media_job_public_id_input => $1) LIMIT 1025";
const MEDIA_JOB_LIST_V1: &str = "SELECT media_job_public_id, source_path, output_path, status::text AS status_text, dry_run, queued_at, started_at, completed_at, last_error FROM media_job_list_v1(media_profile_public_id_input => $1, status_input => $2::media_job_status)";
const MEDIA_JOB_GET_V1: &str = "SELECT media_job_public_id, source_path, output_path, status::text AS status_text, dry_run, queued_at, started_at, completed_at, last_error FROM media_job_get_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_RECENT_PAGE_V1: &str = "SELECT media_job_public_id, media_profile_public_id, source_path, output_path, status_text, dry_run, queued_at, started_at, completed_at, last_error, operation_count, violation_count, plan_reason_count, verification_check_count, artifact_count, compact_audit_count FROM media_job_recent_page_v1(limit_input => $1, cursor_queued_at_input => $2, cursor_public_id_input => $3, media_profile_public_id_input => $4)";
const MEDIA_JOB_CANCEL_V2: &str = "SELECT media_job_cancel_v2(media_job_public_id_input => $1)";
const MEDIA_JOB_RETRY_V1: &str = "SELECT media_job_retry_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_MARK_COMPLETED_V1: &str =
    "SELECT media_job_mark_completed_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_RETENTION_RUN_V1: &str = "SELECT completed_jobs_deleted, failed_jobs_pruned, failed_detail_rows_deleted FROM media_job_retention_run_v1(as_of_input => $1)";
const MEDIA_JOB_WORKER_CLAIM_NEXT_V4: &str = "SELECT media_job_public_id, media_profile_public_id, source_path, output_path, dry_run, source_root, output_root, source_identity, source_size_bytes, source_modified_ns, source_changed_ns, source_sha256, compatibility_target_key, policy_key, target_video_codec, target_audio_codec, target_audio_channels, target_audio_channel_layout, target_subtitle_policy, policy_video_intent, desired_target_key, desired_target_version, desired_container_format, unmatched_stream_policy, verification_strictness, verification_duration_tolerance_millis, verification_mux_validation, verification_decode_all_streams, verification_keyframe_seek, verification_playback_probe, attempt_number, claim_generation, cancel_generation FROM media_job_worker_claim_next_v4()";
const MEDIA_WORKSPACE_RETENTION_SNAPSHOT_V1: &str = "SELECT media_job_public_id, attempt_number, claim_generation, workspace_retention_seconds, diagnostic_workspace_retention_seconds, max_entries_per_tick FROM media_workspace_retention_snapshot_v1()";
const MEDIA_JOB_WORKER_HEARTBEAT_V1: &str = "SELECT media_job_worker_heartbeat_v1(media_job_public_id_input => $1, claim_generation_input => $2)";
const MEDIA_JOB_WORKER_RESUME_INTERRUPTED_V1: &str = "SELECT media_job_public_id, status::text AS status_text, last_error FROM media_job_worker_resume_interrupted_v1(workspace_root_input => $1)";
const MEDIA_JOB_WORKER_INTERRUPT_V1: &str = "SELECT media_job_worker_interrupt_v1(media_job_public_id_input => $1, claim_generation_input => $2)";
const MEDIA_JOB_WORKER_MARK_STATUS_V1: &str = "SELECT media_job_worker_mark_status_v1(media_job_public_id_input => $1, claim_generation_input => $2, status_input => $3::media_job_status, last_error_input => $4)";
const MEDIA_JOB_WORKER_POLL_CONTROL_V1: &str = "SELECT cancel_requested, cancel_generation FROM media_job_worker_poll_control_v1(media_job_public_id_input => $1, claim_generation_input => $2, observed_cancel_generation_input => $3)";
const MEDIA_JOB_WORKER_ACKNOWLEDGE_CANCEL_V1: &str = "SELECT media_job_worker_acknowledge_cancel_v1(media_job_public_id_input => $1, claim_generation_input => $2, observed_cancel_generation_input => $3)";
const MEDIA_JOB_WORKER_COMPLETE_V1: &str = "SELECT media_job_worker_complete_v1(media_job_public_id_input => $1, claim_generation_input => $2, observed_cancel_generation_input => $3)";
const MEDIA_JOB_WORKER_COMMIT_REPLACEMENT_TERMINAL_V1: &str = "SELECT media_job_worker_commit_replacement_terminal_v1(media_job_public_id_input => $1, claim_generation_input => $2, observed_cancel_generation_input => $3)";
const MEDIA_JOB_TERMINAL_OUTBOX_LIST_UNPUBLISHED_V1: &str = "SELECT media_job_public_id, claim_generation, event_kind FROM media_job_terminal_outbox_list_unpublished_v1()";
const MEDIA_JOB_TERMINAL_OUTBOX_MARK_PUBLISHED_V1: &str =
    "SELECT media_job_terminal_outbox_mark_published_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_DESIRED_TARGET_STREAM_LIST_V5: &str = "SELECT stream_key, stream_kind, semantic_role, language_code, optional, sort_order, codec, channel_count, channel_layout, audio_bitrate_bps, audio_sample_rate_hz, audio_loudness_profile, audio_dynamic_range, video_profile, video_level, video_bitrate_bps, color_primaries, color_transfer, color_space, hdr_format, title, default_disposition, forced_disposition, subtitle_placement, image_subtitle_action FROM media_job_desired_target_stream_list_v5(media_job_public_id_input => $1)";
const MEDIA_DISCOVERY_JOB_ENQUEUE_V3: &str = "SELECT media_job_public_id, dry_run FROM media_discovery_job_enqueue_v3(actor_public_id_input => $1, media_profile_public_id_input => $2, source_path_input => $3, output_path_input => $4, dry_run_input => $5, source_identity_input => $6, source_size_bytes_input => $7, source_modified_ns_input => $8, source_changed_ns_input => $9, source_sha256_input => $10)";

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
    pub output_path: Option<&'a str>,
    /// Requested execution mode. A dry-run-only profile always forces this to true.
    pub dry_run: bool,
    /// Stable device and inode identity observed from the opened source handle.
    pub source_identity: &'a str,
    /// Stable file size observed while hashing.
    pub source_size_bytes: i64,
    /// Stable nanosecond modification timestamp observed while hashing.
    pub source_modified_ns: i64,
    /// Stable nanosecond change timestamp observed while hashing.
    pub source_changed_ns: i64,
    /// Lowercase SHA-256 content fingerprint.
    pub source_sha256: &'a str,
}

/// Durable job admission selected under the profile-row lock.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct EnqueuedMediaJobRow {
    /// Created job public id.
    pub media_job_public_id: Uuid,
    /// Effective persisted execution mode.
    pub dry_run: bool,
}

/// Append media job operation payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendMediaJobOperationInput<'a> {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Operation ordering index.
    pub operation_index: i32,
    /// Stable operation kind.
    pub operation_kind: &'a str,
    /// Optional stream id for stream-scoped operations.
    pub stream_id: Option<i32>,
    /// Command binary.
    pub command_bin: &'a str,
    /// Bounded command arguments.
    pub args: [Option<&'a str>; 5],
}

/// Append media job plan reason payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendMediaJobPlanReasonInput<'a> {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Reason ordering index.
    pub reason_index: i32,
    /// Optional candidate index.
    pub candidate_index: Option<i32>,
    /// Whether the reason describes the selected plan.
    pub selected: bool,
    /// Stable reason code.
    pub reason_code: &'a str,
    /// Human-readable reason text.
    pub reason_text: &'a str,
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

/// Recent-job summary with set-wise diagnostic counts.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaRecentJobRow {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Profile public id.
    pub media_profile_public_id: Uuid,
    /// Source path.
    pub source_path: String,
    /// Output path.
    pub output_path: Option<String>,
    /// Status text.
    pub status_text: String,
    /// Dry-run flag.
    pub dry_run: bool,
    /// Queued timestamp.
    pub queued_at: chrono::DateTime<chrono::Utc>,
    /// Started timestamp.
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Completed timestamp.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last error.
    pub last_error: Option<String>,
    /// Operation count.
    pub operation_count: i64,
    /// Violation count.
    pub violation_count: i64,
    /// Plan-reason count.
    pub plan_reason_count: i64,
    /// Verification-check count.
    pub verification_check_count: i64,
    /// Artifact count.
    pub artifact_count: i64,
    /// Compact-audit count.
    pub compact_audit_count: i64,
}

/// One row in the coherent workspace-retention policy and active-job snapshot.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaWorkspaceRetentionSnapshotRow {
    /// Active job public id, absent only when the active set is empty.
    pub media_job_public_id: Option<Uuid>,
    /// Protected workspace attempt, absent only with the job identity.
    pub attempt_number: Option<i32>,
    /// Protected workspace claim generation, absent only with the job identity.
    pub claim_generation: Option<i64>,
    /// Full-workspace retention duration in seconds.
    pub workspace_retention_seconds: i64,
    /// Diagnostics-only workspace retention duration in seconds.
    pub diagnostic_workspace_retention_seconds: i64,
    /// Maximum root entries inspected during one janitor tick.
    pub max_entries_per_tick: i32,
}

/// Media job phase row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobPhaseRow {
    /// Phase ordering index.
    pub phase_index: i32,
    /// Phase name.
    pub phase_name: String,
    /// Phase status.
    pub phase_status: String,
    /// Optional phase details.
    pub details_text: Option<String>,
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
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

/// Unpublished durable terminal event for a media job.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobTerminalOutboxRow {
    /// Job public id used as the idempotency key.
    pub media_job_public_id: Uuid,
    /// Attempt claim generation that owns the replacement transaction.
    pub claim_generation: i64,
    /// Stable terminal event kind.
    pub event_kind: String,
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

/// Persisted identity of one media job attempt, including terminal attempts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobAttemptIdentityRow {
    /// Attempt number within the job.
    pub attempt_number: i32,
    /// Stable identity allocated when the attempt is created.
    pub claim_generation: i64,
    /// Whether this is the job's current attempt.
    pub is_current: bool,
}

/// Media job compact audit fact row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobCompactAuditRow {
    /// Attempt that recorded this observation.
    pub attempt_number: i32,
    /// Whether this observation belongs to the current attempt.
    pub is_current: bool,
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
    /// Stable device and inode identity snapshotted when queued.
    pub source_identity: String,
    /// Aggregate source byte count snapshotted when queued.
    pub source_size_bytes: i64,
    /// Latest aggregate modification timestamp snapshotted when queued.
    pub source_modified_ns: i64,
    /// Main source change timestamp snapshotted when queued.
    pub source_changed_ns: i64,
    /// Aggregate source and sidecar hash snapshotted when queued.
    pub source_sha256: String,
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
    /// Monotonic attempt number for this job.
    pub attempt_number: i32,
    /// Fencing generation that must accompany worker-owned mutations.
    pub claim_generation: i64,
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

/// Media job recovered from an abandoned worker heartbeat.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct RecoveredMediaJobRow {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Terminal status assigned by recovery.
    pub status_text: String,
    /// Machine-readable failure detail for failed stale jobs.
    pub last_error: Option<String>,
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
) -> Result<Option<EnqueuedMediaJobRow>> {
    sqlx::query_as::<_, EnqueuedMediaJobRow>(MEDIA_DISCOVERY_JOB_ENQUEUE_V3)
        .bind(input.actor_public_id)
        .bind(input.media_profile_public_id)
        .bind(input.source_path)
        .bind(input.output_path.unwrap_or_default())
        .bind(input.dry_run)
        .bind(input.source_identity)
        .bind(input.source_size_bytes)
        .bind(input.source_modified_ns)
        .bind(input.source_changed_ns)
        .bind(input.source_sha256)
        .fetch_optional(pool)
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
    claim_generation: i64,
    phase_index: i32,
    phase_name: &str,
    phase_status_text: &str,
    details_text: Option<&str>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_PHASE_APPEND_V1)
        .bind(media_job_public_id)
        .bind(claim_generation)
        .bind(phase_index)
        .bind(phase_name)
        .bind(phase_status_text)
        .bind(details_text.unwrap_or_default())
        .execute(pool)
        .await
        .map_err(try_op("media job phase append"))?;

    Ok(())
}

/// List media job phases for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_phases(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobPhaseRow>> {
    sqlx::query_as::<_, MediaJobPhaseRow>(MEDIA_JOB_PHASE_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job phase list"))
}

/// Append or update a media job operation row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_operation(
    pool: &PgPool,
    claim_generation: i64,
    input: &AppendMediaJobOperationInput<'_>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_OPERATION_APPEND_V1)
        .bind(input.media_job_public_id)
        .bind(claim_generation)
        .bind(input.operation_index)
        .bind(input.operation_kind)
        .bind(input.stream_id)
        .bind(input.command_bin)
        .bind(input.args[0].unwrap_or_default())
        .bind(input.args[1].unwrap_or_default())
        .bind(input.args[2].unwrap_or_default())
        .bind(input.args[3].unwrap_or_default())
        .bind(input.args[4].unwrap_or_default())
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
    claim_generation: i64,
    violation_index: i32,
    violation_kind: &str,
    severity: &str,
    stream_id: Option<i32>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_VIOLATION_APPEND_V1)
        .bind(media_job_public_id)
        .bind(claim_generation)
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
    claim_generation: i64,
    input: &AppendMediaJobPlanReasonInput<'_>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_PLAN_REASON_APPEND_V1)
        .bind(input.media_job_public_id)
        .bind(claim_generation)
        .bind(input.reason_index)
        .bind(input.candidate_index)
        .bind(input.selected)
        .bind(input.reason_code)
        .bind(input.reason_text)
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
    claim_generation: i64,
    input: &AppendMediaJobVerificationCheckInput<'_>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_VERIFICATION_CHECK_APPEND_V1)
        .bind(input.media_job_public_id)
        .bind(claim_generation)
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
    claim_generation: i64,
    input: &AppendMediaJobArtifactInput<'_>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_ARTIFACT_APPEND_V1)
        .bind(input.media_job_public_id)
        .bind(claim_generation)
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
    claim_generation: i64,
    input: &AppendMediaJobCompactAuditInput<'_>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_COMPACT_AUDIT_APPEND_V1)
        .bind(input.media_job_public_id)
        .bind(claim_generation)
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

/// Read one bounded keyset page of recent jobs with diagnostic counts.
///
/// # Errors
///
/// Returns an error when the page or cursor is invalid or stored-procedure execution fails.
pub async fn list_recent_media_jobs(
    pool: &PgPool,
    limit: i32,
    cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
    media_profile_public_id: Option<Uuid>,
) -> Result<Vec<MediaRecentJobRow>> {
    let (cursor_queued_at, cursor_public_id) = cursor.unzip();
    sqlx::query_as::<_, MediaRecentJobRow>(MEDIA_JOB_RECENT_PAGE_V1)
        .bind(limit)
        .bind(cursor_queued_at)
        .bind(cursor_public_id)
        .bind(media_profile_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media recent job page"))
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

/// List persisted attempt identities for one job, including terminal outcomes.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_attempt_identities(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobAttemptIdentityRow>> {
    sqlx::query_as::<_, MediaJobAttemptIdentityRow>(MEDIA_JOB_ATTEMPT_IDENTITY_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job attempt identity list"))
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

/// Read one operator job with root-relative paths, not worker filesystem paths.
///
/// # Errors
/// Propagates privilege, snapshot, procedure and decoding failures.
pub async fn get_operator_media_job(pool: &PgPool, id: Uuid) -> Result<Option<MediaJobRow>> {
    sqlx::query_as("SELECT media_job_public_id, source_path, output_path, status::text AS status_text, dry_run, queued_at, started_at, completed_at, last_error FROM media_job_operator_get_v1($1)")
        .bind(id).fetch_optional(pool).await.map_err(try_op("operator media job get"))
}

/// List operator jobs without exposing worker filesystem paths.
///
/// # Errors
/// Propagates privilege, snapshot, procedure and decoding failures.
pub async fn list_operator_media_jobs(
    pool: &PgPool,
    profile: Uuid,
    status: Option<&str>,
) -> Result<Vec<MediaJobRow>> {
    sqlx::query_as("SELECT media_job_public_id, source_path, output_path, status::text AS status_text, dry_run, queued_at, started_at, completed_at, last_error FROM media_job_operator_list_v1($1, $2::media_job_status)")
        .bind(profile).bind(status).fetch_all(pool).await.map_err(try_op("operator media job list"))
}

/// Read the canonical recent page with root-relative operator paths.
///
/// # Errors
/// Propagates cursor, privilege, snapshot, procedure and decoding failures.
pub async fn list_operator_recent_media_jobs(
    pool: &PgPool,
    limit: i32,
    cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
    profile: Option<Uuid>,
) -> Result<Vec<MediaRecentJobRow>> {
    let (queued_at, id) = cursor.unzip();
    sqlx::query_as("SELECT * FROM media_job_operator_recent_page_v1($1, $2, $3, $4)")
        .bind(limit)
        .bind(queued_at)
        .bind(id)
        .bind(profile)
        .fetch_all(pool)
        .await
        .map_err(try_op("operator media recent job page"))
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

/// Read persisted workspace-retention bounds and active job keys in one database snapshot.
///
/// The procedure always returns one policy row when no jobs are active and repeats the same policy
/// values for each active job otherwise.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn load_media_workspace_retention_snapshot(
    pool: &PgPool,
) -> Result<Vec<MediaWorkspaceRetentionSnapshotRow>> {
    sqlx::query_as::<_, MediaWorkspaceRetentionSnapshotRow>(MEDIA_WORKSPACE_RETENTION_SNAPSHOT_V1)
        .fetch_all(pool)
        .await
        .map_err(try_op("media workspace retention snapshot"))
}

/// Claim the next queued media job for worker processing.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn media_job_worker_claim_next(pool: &PgPool) -> Result<Option<ClaimedMediaJobRow>> {
    sqlx::query_as::<_, ClaimedMediaJobRow>(MEDIA_JOB_WORKER_CLAIM_NEXT_V4)
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
    claim_generation: i64,
    observed_cancel_generation: i64,
) -> Result<MediaJobControlRow> {
    sqlx::query_as::<_, MediaJobControlRow>(MEDIA_JOB_WORKER_POLL_CONTROL_V1)
        .bind(media_job_public_id)
        .bind(claim_generation)
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
    claim_generation: i64,
    observed_cancel_generation: i64,
) -> Result<i64> {
    sqlx::query_scalar::<_, i64>(MEDIA_JOB_WORKER_ACKNOWLEDGE_CANCEL_V1)
        .bind(media_job_public_id)
        .bind(claim_generation)
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
    claim_generation: i64,
    observed_cancel_generation: i64,
) -> Result<bool> {
    sqlx::query_scalar::<_, bool>(MEDIA_JOB_WORKER_COMPLETE_V1)
        .bind(media_job_public_id)
        .bind(claim_generation)
        .bind(observed_cancel_generation)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job worker complete"))
}

/// Atomically persist replacement verification, terminal completion, and an outbox event.
///
/// # Errors
///
/// Returns an error when the job is not worker-owned or execution fails.
pub async fn media_job_worker_commit_replacement_terminal(
    pool: &PgPool,
    media_job_public_id: Uuid,
    claim_generation: i64,
    observed_cancel_generation: i64,
) -> Result<bool> {
    sqlx::query_scalar::<_, bool>(MEDIA_JOB_WORKER_COMMIT_REPLACEMENT_TERMINAL_V1)
        .bind(media_job_public_id)
        .bind(claim_generation)
        .bind(observed_cancel_generation)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job worker commit replacement terminal"))
}

/// List bounded unpublished terminal events in commit order.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_terminal_outbox_unpublished(
    pool: &PgPool,
) -> Result<Vec<MediaJobTerminalOutboxRow>> {
    sqlx::query_as::<_, MediaJobTerminalOutboxRow>(MEDIA_JOB_TERMINAL_OUTBOX_LIST_UNPUBLISHED_V1)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job terminal outbox list unpublished"))
}

/// Mark one durable terminal event published.
///
/// # Errors
///
/// Returns an error when the row does not exist or execution fails.
pub async fn mark_media_job_terminal_outbox_published(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_TERMINAL_OUTBOX_MARK_PUBLISHED_V1)
        .bind(media_job_public_id)
        .execute(pool)
        .await
        .map_err(try_op("media job terminal outbox mark published"))?;
    Ok(())
}

/// Update the heartbeat timestamp for a running media job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn media_job_worker_heartbeat(
    pool: &PgPool,
    media_job_public_id: Uuid,
    claim_generation: i64,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_WORKER_HEARTBEAT_V1)
        .bind(media_job_public_id)
        .bind(claim_generation)
        .execute(pool)
        .await
        .map_err(try_op("media job worker heartbeat"))?;
    Ok(())
}

/// Resume a bounded startup batch after root admission and replacement reconciliation.
///
/// # Errors
/// Returns persistence errors. This is never a live-worker takeover operation.
pub async fn media_job_worker_resume_interrupted(
    pool: &PgPool,
    workspace_root: &str,
) -> Result<Vec<RecoveredMediaJobRow>> {
    sqlx::query_as(MEDIA_JOB_WORKER_RESUME_INTERRUPTED_V1)
        .bind(workspace_root)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job worker resume interrupted"))
}

/// Requeue stopped work on the same attempt, acknowledging pending cancellation instead.
///
/// Call only after the local worker and its children have stopped.
///
/// # Errors
/// Returns stale-claim or persistence errors. The returned flag is true for cancellation.
pub async fn media_job_worker_interrupt(
    pool: &PgPool,
    media_job_public_id: Uuid,
    claim_generation: i64,
) -> Result<bool> {
    sqlx::query_scalar(MEDIA_JOB_WORKER_INTERRUPT_V1)
        .bind(media_job_public_id)
        .bind(claim_generation)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job worker interrupt"))
}

/// Mark a claimed media job with the next worker-visible status.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn media_job_worker_mark_status(
    pool: &PgPool,
    media_job_public_id: Uuid,
    claim_generation: i64,
    status_text: &str,
    last_error: Option<&str>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_WORKER_MARK_STATUS_V1)
        .bind(media_job_public_id)
        .bind(claim_generation)
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
        AppendMediaJobArtifactInput, AppendMediaJobCompactAuditInput, AppendMediaJobOperationInput,
        AppendMediaJobPlanReasonInput, AppendMediaJobVerificationCheckInput, ClaimedMediaJobRow,
        CreateMediaJobInput, EnqueueDiscoveredMediaJobInput, append_media_job_artifact,
        append_media_job_compact_audit, append_media_job_operation, append_media_job_phase,
        append_media_job_plan_reason, append_media_job_verification_check,
        append_media_job_violation, cancel_media_job,
        create_media_job as create_unfingerprinted_media_job, enqueue_discovered_media_job,
        get_media_job, list_media_job_artifacts, list_media_job_compact_audits,
        list_media_job_operations, list_media_job_phases, list_media_job_plan_reasons,
        list_media_job_terminal_outbox_unpublished, list_media_job_verification_checks,
        list_media_job_violations, list_media_jobs, list_recent_media_jobs,
        load_media_workspace_retention_snapshot, mark_media_job_completed,
        mark_media_job_terminal_outbox_published, media_job_worker_acknowledge_cancel,
        media_job_worker_claim_next, media_job_worker_commit_replacement_terminal,
        media_job_worker_mark_status, media_job_worker_poll_control, run_media_job_retention,
    };
    use crate::DataError;
    use crate::media::association_jobs::{
        AssociationFingerprint, AssociationJobInput, enqueue_association_job,
    };
    use crate::media::configuration::{
        AppendMediaDesiredTargetStreamInput, CreateMediaDesiredTargetInput, MediaPolicyOutputRow,
        UpdateMediaJobRetentionPolicyInput, UpsertMediaPolicyProfileInput,
        append_media_desired_target_stream, create_media_desired_target,
        update_media_job_retention_policy, upsert_media_policy_profile,
    };
    use crate::media::profile_versions::{CreateProfileVersionInput, replace_profile_version};
    use crate::media::profiles::{UpsertMediaProfileInput, upsert_media_profile};
    use crate::media::schema_tests::{MediaTestDb, setup_media_db};
    use chrono::{Duration, Utc};
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };
    use std::{
        fs,
        path::Path,
        sync::atomic::{AtomicI64, Ordering},
    };
    use uuid::Uuid;

    static TEST_FINGERPRINT_VERSION: AtomicI64 = AtomicI64::new(1);

    async fn create_media_job(
        pool: &PgPool,
        input: &CreateMediaJobInput<'_>,
    ) -> anyhow::Result<Uuid> {
        let version = TEST_FINGERPRINT_VERSION.fetch_add(1, Ordering::Relaxed);
        let source_identity = format!("{version:016x}:{version:016x}");
        let source_sha256 = format!("{version:064x}");
        enqueue_discovered_media_job(
            pool,
            &EnqueueDiscoveredMediaJobInput {
                actor_public_id: input.actor_public_id,
                media_profile_public_id: input.media_profile_public_id,
                source_path: input.source_path,
                output_path: input.output_path,
                dry_run: input.dry_run,
                source_identity: &source_identity,
                source_size_bytes: version,
                source_modified_ns: version,
                source_changed_ns: version,
                source_sha256: &source_sha256,
            },
        )
        .await?
        .map(|job| job.media_job_public_id)
        .ok_or_else(|| anyhow::anyhow!("test media job fingerprint was unchanged"))
    }

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

    async fn claim_job(pool: &PgPool, expected_job_id: Uuid) -> anyhow::Result<ClaimedMediaJobRow> {
        let claimed = media_job_worker_claim_next(pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("test media job was not claimable"))?;
        if claimed.media_job_public_id != expected_job_id {
            return Err(anyhow::anyhow!(
                "worker claimed an unexpected test media job"
            ));
        }
        Ok(claimed)
    }

    async fn append_and_assert_phase_transition(
        pool: &PgPool,
        job_id: Uuid,
        claim_generation: i64,
    ) -> anyhow::Result<()> {
        append_media_job_phase(
            pool,
            job_id,
            claim_generation,
            0,
            "planning",
            "running",
            Some("scheduled"),
        )
        .await?;
        append_media_job_phase(
            pool,
            job_id,
            claim_generation,
            0,
            "planning",
            "completed",
            None,
        )
        .await?;
        let phase_status = sqlx::query_scalar::<_, String>(
            "SELECT phase_status::text FROM media_job_phase_list_v1($1) WHERE phase_index = 0",
        )
        .bind(job_id)
        .fetch_one(pool)
        .await?;
        assert_eq!(phase_status, "completed");
        Ok(())
    }

    async fn append_and_assert_plan_reason(
        pool: &PgPool,
        job_id: Uuid,
        claim_generation: i64,
    ) -> anyhow::Result<()> {
        append_media_job_plan_reason(
            pool,
            claim_generation,
            &AppendMediaJobPlanReasonInput {
                media_job_public_id: job_id,
                reason_index: 0,
                candidate_index: Some(0),
                selected: true,
                reason_code: "least_cost_selected",
                reason_text: "Selected the least-cost compliant candidate.",
            },
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
        claim_generation: i64,
    ) -> anyhow::Result<()> {
        append_media_job_verification_check(
            pool,
            claim_generation,
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
        claim_generation: i64,
    ) -> anyhow::Result<()> {
        append_media_job_artifact(
            pool,
            claim_generation,
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
            claim_generation,
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
            claim_generation,
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
            claim_generation,
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
            claim_generation,
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

    async fn cancel_job_with_audit(db: &MediaTestDb, job_id: Uuid) -> anyhow::Result<()> {
        let claimed = claim_job(db.pool(), job_id).await?;
        append_and_assert_artifact_and_audit(db.pool(), job_id, claimed.claim_generation).await?;
        cancel_media_job(db.pool(), job_id).await?;
        media_job_worker_acknowledge_cancel(
            db.pool(),
            job_id,
            claimed.claim_generation,
            claimed.cancel_generation,
        )
        .await?;
        Ok(())
    }

    async fn cancel_diagnostic_job(db: &MediaTestDb, job_id: Uuid) -> anyhow::Result<()> {
        let claim = claim_job(db.pool(), job_id).await?;
        append_media_job_violation(
            db.pool(),
            job_id,
            claim.claim_generation,
            0,
            "video_codec_mismatch",
            "high",
            Some(0),
        )
        .await?;
        append_and_assert_plan_reason(db.pool(), job_id, claim.claim_generation).await?;
        append_and_assert_verification_check(db.pool(), job_id, claim.claim_generation).await?;
        append_and_assert_artifact_and_audit(db.pool(), job_id, claim.claim_generation).await?;
        cancel_media_job(db.pool(), job_id).await?;
        media_job_worker_acknowledge_cancel(
            db.pool(),
            job_id,
            claim.claim_generation,
            claim.cancel_generation,
        )
        .await?;
        Ok(())
    }

    async fn complete_diagnostic_job(db: &MediaTestDb, job_id: Uuid) -> anyhow::Result<()> {
        let claim = claim_job(db.pool(), job_id).await?;
        append_and_assert_artifact_and_audit(db.pool(), job_id, claim.claim_generation).await?;
        media_job_worker_mark_status(db.pool(), job_id, claim.claim_generation, "completed", None)
            .await?;
        Ok(())
    }

    async fn assert_create_job_path_rejected(
        pool: &PgPool,
        actor_public_id: Uuid,
        profile_id: Uuid,
        source_path: &str,
        output_path: Option<&str>,
        expected_detail: &str,
    ) -> anyhow::Result<()> {
        let err = create_unfingerprinted_media_job(
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
        let db = setup_media_db().await?;
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

    #[test]
    fn migration_guards_bounded_recent_job_read_model() {
        let migration = include_str!("../../migrations/0187_media_bounded_read_models.sql");
        assert!(migration.contains("media_job_recent_page_v1"));
        assert!(migration.contains("limit_input + 1"));
        assert!(migration.contains("operation_count"));
        assert!(migration.contains("compact_audit_count"));
        assert!(migration.contains("media_desired_target_graph_page_v1"));
        assert!(migration.contains("LIMIT 1025"));
        assert!(migration.contains("media_job_worker_claim_next_v4()"));
        assert!(migration.contains("FOR UPDATE OF job, attempt SKIP LOCKED"));
        assert!(migration.contains("attempt.attempt_number, attempt.claim_generation"));
        assert!(migration.contains("job.intent_source_identity IS NOT NULL"));
    }

    #[test]
    fn migration_guards_media_job_fingerprint_requirement() {
        let migration_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let migration_text = fs::read_dir(migration_root)
            .into_iter()
            .flat_map(|entries| entries.filter_map(Result::ok))
            .filter_map(|entry| fs::read_to_string(entry.path()).ok())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            migration_text.contains("media_discovery_source_fingerprint"),
            "source fingerprint table must be present in migrations"
        );
        assert!(
            migration_text.contains("media_job_source_fingerprint_required"),
            "direct job creation must require persisted source fingerprints"
        );
    }

    #[tokio::test]
    async fn create_media_job_requires_persisted_source_fingerprint() -> anyhow::Result<()> {
        let db = setup_media_db().await?;

        let profile_id =
            upsert_retention_profile(&db, "fingerprint-required", "fingerprint-required").await?;
        let err = create_unfingerprinted_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/fingerprint-required/video.mkv",
                output_path: Some("/output/fingerprint-required/video.mkv"),
                dry_run: true,
            },
        )
        .await
        .expect_err("direct job creation without fingerprint should fail");

        assert!(matches!(err, DataError::QueryFailed { .. }));
        assert_eq!(
            err.database_detail(),
            Some("media_job_source_fingerprint_required")
        );
        Ok(())
    }

    #[tokio::test]
    async fn create_and_list_media_job() -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        let (_roots, job_id) =
            crate::media::tests::native_job(db.database(), db.pool(), "tv-jobs", true).await?;
        let claimed = claim_job(db.pool(), job_id).await?;
        let profile_id = claimed.media_profile_public_id;
        append_and_assert_phase_transition(db.pool(), job_id, claimed.claim_generation).await?;

        append_media_job_operation(
            db.pool(),
            claimed.claim_generation,
            &AppendMediaJobOperationInput {
                media_job_public_id: job_id,
                operation_index: 0,
                operation_kind: "remux",
                stream_id: None,
                command_bin: "ffmpeg",
                args: [
                    Some("-i"),
                    Some(&claimed.source_path),
                    Some("-c"),
                    Some("copy"),
                    None,
                ],
            },
        )
        .await?;

        append_media_job_violation(
            db.pool(),
            job_id,
            claimed.claim_generation,
            0,
            "video_codec_mismatch",
            "high",
            Some(0),
        )
        .await?;

        let rows = list_media_jobs(db.pool(), profile_id, Some("running")).await?;
        assert!(rows.iter().any(|item| item.media_job_public_id == job_id));

        let job = get_media_job(db.pool(), job_id).await?;
        assert!(job.is_some());
        let Some(job) = job else {
            return Ok(());
        };
        assert_eq!(job.media_job_public_id, job_id);

        let phases = list_media_job_phases(db.pool(), job_id).await?;
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].phase_index, 0);
        assert_eq!(phases[0].phase_name, "planning");
        assert_eq!(phases[0].phase_status, "completed");
        assert_eq!(phases[0].details_text, None);

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

        append_and_assert_plan_reason(db.pool(), job_id, claimed.claim_generation).await?;

        append_and_assert_verification_check(db.pool(), job_id, claimed.claim_generation).await?;
        append_and_assert_artifact_and_audit(db.pool(), job_id, claimed.claim_generation).await?;
        Ok(())
    }

    #[tokio::test]
    async fn recent_job_page_is_keyset_bounded_and_counts_diagnostics_set_wise()
    -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        let key = "recent-jobs";
        let (_roots, first_job) =
            crate::media::tests::native_job(db.database(), db.pool(), key, true).await?;
        let mut ids = vec![first_job];
        for index in 1..12 {
            ids.push(
                crate::media::tests::additional_native_job(
                    db.pool(),
                    key,
                    &format!("{index}.mkv"),
                    true,
                )
                .await?,
            );
        }
        let mut latest_claim = None;
        for expected_id in &ids {
            let claim = media_job_worker_claim_next(db.pool())
                .await?
                .ok_or_else(|| anyhow::anyhow!("expected queued media job"))?;
            assert_eq!(claim.media_job_public_id, *expected_id);
            latest_claim = Some(claim);
        }
        let latest_claim = latest_claim.ok_or_else(|| anyhow::anyhow!("expected media claim"))?;
        append_media_job_operation(
            db.pool(),
            latest_claim.claim_generation,
            &AppendMediaJobOperationInput {
                media_job_public_id: ids[11],
                operation_index: 0,
                operation_kind: "copy",
                stream_id: None,
                command_bin: "ffmpeg",
                args: [None, None, None, None, None],
            },
        )
        .await?;
        let profile_id = latest_claim.media_profile_public_id;
        let first = list_recent_media_jobs(db.pool(), 10, None, Some(profile_id)).await?;
        assert_eq!(first.len(), 11);
        assert_eq!(first[0].operation_count, 1);
        let cursor = first
            .get(9)
            .map(|row| (row.queued_at, row.media_job_public_id));
        let second = list_recent_media_jobs(db.pool(), 10, cursor, Some(profile_id)).await?;
        assert_eq!(second.len(), 2);
        assert!(
            list_recent_media_jobs(db.pool(), 0, None, None)
                .await
                .is_err()
        );
        assert!(
            list_recent_media_jobs(db.pool(), 101, None, None)
                .await
                .is_err()
        );
        Ok(())
    }

    #[tokio::test]
    async fn running_job_cancellation_is_durable_and_worker_acknowledged() -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        let (_roots, job_id) =
            crate::media::tests::native_job(db.database(), db.pool(), "cancel-running", true)
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
        let control = media_job_worker_poll_control(
            db.pool(),
            job_id,
            claimed.claim_generation,
            claimed.cancel_generation,
        )
        .await?;
        assert!(control.cancel_requested);
        assert_eq!(control.cancel_generation, requested_generation);
        assert!(
            media_job_worker_mark_status(
                db.pool(),
                job_id,
                claimed.claim_generation,
                "completed",
                None,
            )
            .await
            .is_err(),
            "a pending cancellation must fence successful completion"
        );
        let acknowledged_generation = media_job_worker_acknowledge_cancel(
            db.pool(),
            job_id,
            claimed.claim_generation,
            claimed.cancel_generation,
        )
        .await?;
        assert_eq!(acknowledged_generation, requested_generation);
        let job = get_media_job(db.pool(), job_id).await?;
        let Some(job) = job else {
            return Err(anyhow::anyhow!("cancelled job missing"));
        };
        assert_eq!(job.status_text, "cancelled");
        Ok(())
    }

    #[tokio::test]
    async fn replacement_terminal_commit_is_atomic_idempotent_and_outboxed() -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        let (_roots, job_id) = crate::media::tests::native_job(
            db.database(),
            db.pool(),
            "replacement-terminal",
            false,
        )
        .await?;
        let Some(claimed) = media_job_worker_claim_next(db.pool()).await? else {
            return Err(anyhow::anyhow!("replacement job was not claimable"));
        };
        assert_eq!(claimed.media_job_public_id, job_id);

        assert!(
            !media_job_worker_commit_replacement_terminal(
                db.pool(),
                job_id,
                claimed.claim_generation,
                claimed.cancel_generation,
            )
            .await?
        );
        assert!(
            !media_job_worker_commit_replacement_terminal(
                db.pool(),
                job_id,
                claimed.claim_generation,
                claimed.cancel_generation,
            )
            .await?
        );

        let Some(job) = get_media_job(db.pool(), job_id).await? else {
            return Err(anyhow::anyhow!("terminal job missing"));
        };
        assert_eq!(job.status_text, "completed");
        let checks = list_media_job_verification_checks(db.pool(), job_id).await?;
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].check_kind, "output_replacement");
        assert_eq!(checks[0].check_status, "passed");
        let unpublished = list_media_job_terminal_outbox_unpublished(db.pool()).await?;
        assert_eq!(unpublished.len(), 1);
        assert_eq!(unpublished[0].media_job_public_id, job_id);
        assert_eq!(unpublished[0].claim_generation, claimed.claim_generation);
        assert_eq!(unpublished[0].event_kind, "completed");

        mark_media_job_terminal_outbox_published(db.pool(), job_id).await?;
        assert!(
            list_media_job_terminal_outbox_unpublished(db.pool())
                .await?
                .is_empty()
        );
        Ok(())
    }

    #[tokio::test]
    async fn replacement_terminal_commit_honors_cancellation_and_fences_terminal_attempt()
    -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        let (_roots, job_id) = crate::media::tests::native_job(
            db.database(),
            db.pool(),
            "replacement-terminal-cancel",
            false,
        )
        .await?;
        let Some(claimed) = media_job_worker_claim_next(db.pool()).await? else {
            return Err(anyhow::anyhow!("cancellation-race job was not claimable"));
        };
        assert_eq!(claimed.media_job_public_id, job_id);
        let requested_generation = cancel_media_job(db.pool(), job_id).await?;
        assert!(requested_generation > claimed.cancel_generation);

        assert!(
            media_job_worker_commit_replacement_terminal(
                db.pool(),
                job_id,
                claimed.claim_generation,
                claimed.cancel_generation,
            )
            .await?
        );

        let Some(job) = get_media_job(db.pool(), job_id).await? else {
            return Err(anyhow::anyhow!("cancelled replacement job missing"));
        };
        assert_eq!(job.status_text, "cancelled");
        let checks = list_media_job_verification_checks(db.pool(), job_id).await?;
        assert!(checks.iter().any(|check| {
            check.check_index == 98
                && check.check_kind == "cancellation"
                && check.check_status == "skipped"
        }));
        assert!(
            list_media_job_terminal_outbox_unpublished(db.pool())
                .await?
                .is_empty()
        );
        assert!(
            media_job_worker_commit_replacement_terminal(
                db.pool(),
                job_id,
                claimed.claim_generation,
                claimed.cancel_generation,
            )
            .await
            .is_err()
        );
        Ok(())
    }

    #[tokio::test]
    async fn replacement_terminal_commit_rejects_late_cancel_and_remains_completed()
    -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        let (_roots, job_id) = crate::media::tests::native_job(
            db.database(),
            db.pool(),
            "replacement-terminal-late-cancel",
            false,
        )
        .await?;
        let claimed = media_job_worker_claim_next(db.pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("replacement job was not claimable"))?;
        assert_eq!(claimed.media_job_public_id, job_id);

        assert!(
            !media_job_worker_commit_replacement_terminal(
                db.pool(),
                job_id,
                claimed.claim_generation,
                claimed.cancel_generation,
            )
            .await?
        );
        assert!(cancel_media_job(db.pool(), job_id).await.is_err());
        assert!(
            !media_job_worker_commit_replacement_terminal(
                db.pool(),
                job_id,
                claimed.claim_generation,
                claimed.cancel_generation,
            )
            .await?
        );

        let job = get_media_job(db.pool(), job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("completed replacement job missing"))?;
        assert_eq!(job.status_text, "completed");
        assert_eq!(job.last_error, None);
        let unpublished = list_media_job_terminal_outbox_unpublished(db.pool()).await?;
        assert_eq!(unpublished.len(), 1);
        assert_eq!(unpublished[0].media_job_public_id, job_id);
        assert_eq!(unpublished[0].claim_generation, claimed.claim_generation);
        assert_eq!(unpublished[0].event_kind, "completed");
        Ok(())
    }

    async fn upsert_strict_snapshot_policy(
        pool: &sqlx::PgPool,
        actor_public_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        upsert_media_policy_profile(
            pool,
            UpsertMediaPolicyProfileInput {
                output: MediaPolicyOutputRow {
                    dry_run: true,
                    replacement_mode: "disabled".to_string(),
                    quarantine_enabled: false,
                    ..MediaPolicyOutputRow::default()
                },
                actor_public_id,
                policy_key: "snapshot-policy",
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
                output: MediaPolicyOutputRow {
                    dry_run: true,
                    replacement_mode: "disabled".to_string(),
                    quarantine_enabled: false,
                    ..MediaPolicyOutputRow::default()
                },
                actor_public_id,
                policy_key: "snapshot-policy",
                version: 2,
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

    async fn create_snapshot_target(db: &MediaTestDb, version: i32) -> anyhow::Result<()> {
        let target = create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: db.system_user_public_id,
                target_key: "intent-stereo",
                version,
                display_name: "Intent stereo",
                container_format: "matroska",
            },
        )
        .await?;
        let video = AppendMediaDesiredTargetStreamInput {
            media_desired_target_profile_public_id: target,
            stream_key: "video-main",
            stream_kind: "video",
            semantic_role: None,
            language_code: None,
            optional: false,
            sort_order: 0,
            codec: if version == 1 { "hevc" } else { "h264" },
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
        };
        for stream in [
            video,
            AppendMediaDesiredTargetStreamInput {
                stream_key: "audio-main",
                stream_kind: "audio",
                sort_order: 1,
                codec: if version == 1 { "aac" } else { "opus" },
                channel_count: Some(2),
                channel_layout: Some("stereo"),
                ..video
            },
            AppendMediaDesiredTargetStreamInput {
                stream_key: "subtitle-selected",
                stream_kind: "subtitle",
                sort_order: 2,
                optional: true,
                codec: "webvtt",
                subtitle_placement: Some("embedded"),
                image_subtitle_action: Some("preserve"),
                ..video
            },
        ] {
            append_media_desired_target_stream(db.pool(), stream).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn worker_claim_uses_enqueue_time_profile_intent() -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        create_snapshot_target(&db, 1).await?;
        upsert_strict_snapshot_policy(db.pool(), db.system_user_public_id).await?;
        let input = CreateProfileVersionInput {
            actor_public_id: db.system_user_public_id,
            profile_key: "intent-snapshot",
            display_name: "Intent snapshot",
            description: "Synthetic native admission",
            enabled: true,
            dry_run_only: true,
            desired_target_key: "intent-stereo",
            desired_target_version: 1,
            policy_key: "snapshot-policy",
            policy_version: 1,
            output_root_key: "worker-source",
            workspace_root_key: "worker-workspace",
            backup_root_key: None,
            quarantine_root_key: None,
        };
        let (_roots, profile_id, job_id) =
            crate::media::tests::native_configured_job(db.database(), db.pool(), &input).await?;
        let admitted = get_media_job(db.pool(), job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("admitted job missing"))?;
        let (original_root, _) = admitted
            .source_path
            .rsplit_once('/')
            .ok_or_else(|| anyhow::anyhow!("admitted source root missing"))?;
        create_snapshot_target(&db, 2).await?;
        replace_with_fast_policy(db.pool(), db.system_user_public_id).await?;
        let replacement = CreateProfileVersionInput {
            desired_target_version: 2,
            policy_version: 2,
            ..input
        };
        let heads = replace_profile_version(db.pool(), &replacement, profile_id, 1).await?;
        assert!(
            heads
                .iter()
                .all(|row| row.latest_version == 2 && row.active_version == Some(2))
        );

        let claimed = media_job_worker_claim_next(db.pool()).await?;
        let Some(claimed) = claimed else {
            return Err(anyhow::anyhow!("expected queued job to be claimed"));
        };
        assert_eq!(claimed.media_job_public_id, job_id);
        assert_eq!(claimed.source_path, admitted.source_path);
        assert_eq!(claimed.output_path, admitted.output_path);
        assert_eq!(claimed.source_root, original_root);
        assert_eq!(claimed.output_root, original_root);
        assert_eq!(claimed.desired_target_key.as_deref(), Some("intent-stereo"));
        assert_eq!(claimed.desired_target_version, Some(1));
        assert_eq!(claimed.policy_key, "snapshot-policy");
        let streams = super::list_media_job_desired_target_streams(db.pool(), job_id).await?;
        assert_eq!(streams.len(), 3);
        assert_eq!(streams[0].codec, "hevc");
        assert_eq!(streams[1].codec, "aac");
        assert_eq!(streams[1].channel_count, Some(2));
        assert_eq!(streams[1].channel_layout.as_deref(), Some("stereo"));
        assert_eq!(streams[2].stream_key, "subtitle-selected");
        assert!(streams[2].optional);
        assert_eq!(streams[2].subtitle_placement.as_deref(), Some("embedded"));
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
        let db = setup_media_db().await?;

        let key = "count-retention";
        let (_roots, expired_job_id) =
            crate::media::tests::native_job(db.database(), db.pool(), key, true).await?;
        let retained_completed_job_id =
            crate::media::tests::additional_native_job(db.pool(), key, "newer.mkv", true).await?;
        let queued_job_id =
            crate::media::tests::additional_native_job(db.pool(), key, "queued.mkv", true).await?;

        db.database()
            .apply_fixture_script(
                include_str!("../../../../scripts/tests/media-root-snapshot-retention-guard.sql"),
                &[("revaer_test.job_id", &expired_job_id.to_string())],
            )
            .await?;
        let expired_claim = claim_job(db.pool(), expired_job_id).await?;
        append_media_job_compact_audit(
            db.pool(),
            expired_claim.claim_generation,
            &AppendMediaJobCompactAuditInput {
                media_job_public_id: expired_job_id,
                audit_index: 0,
                fact_kind: "replacement",
                fact_text: "source replaced after verified candidate",
            },
        )
        .await?;
        media_job_worker_mark_status(
            db.pool(),
            expired_job_id,
            expired_claim.claim_generation,
            "completed",
            None,
        )
        .await?;
        mark_media_job_completed(db.pool(), retained_completed_job_id).await?;
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
        let db = setup_media_db().await?;
        let key = "diagnostic-retention";
        let (_roots, cancelled_job_id) =
            crate::media::tests::native_job(db.database(), db.pool(), key, true).await?;
        cancel_diagnostic_job(&db, cancelled_job_id).await?;
        let completed_job_id =
            crate::media::tests::additional_native_job(db.pool(), key, "completed.mkv", true)
                .await?;
        complete_diagnostic_job(&db, completed_job_id).await?;

        assert_eq!(
            super::list_media_job_desired_target_streams(db.pool(), cancelled_job_id)
                .await?
                .len(),
            1
        );

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
        assert_eq!(outcome.failed_detail_rows_deleted, 5);
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
    async fn workspace_retention_snapshot_includes_queued_and_claimed_jobs() -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        let key = "workspace-retention-snapshot";
        let (_roots, running_id) =
            crate::media::tests::native_job(db.database(), db.pool(), key, true).await?;
        let queued_id =
            crate::media::tests::additional_native_job(db.pool(), key, "queued.mkv", true).await?;
        let completed_id =
            crate::media::tests::additional_native_job(db.pool(), key, "completed.mkv", true)
                .await?;
        let claimed = media_job_worker_claim_next(db.pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("expected a claimed media job"))?;
        assert_eq!(claimed.media_job_public_id, running_id);
        mark_media_job_completed(db.pool(), completed_id).await?;

        let snapshot = load_media_workspace_retention_snapshot(db.pool()).await?;
        let active_ids = snapshot
            .iter()
            .filter_map(|row| row.media_job_public_id)
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(active_ids, [running_id, queued_id].into_iter().collect());
        assert!(!active_ids.contains(&completed_id));
        assert!(snapshot.iter().all(|row| {
            row.workspace_retention_seconds == 86_400
                && row.diagnostic_workspace_retention_seconds == 2_592_000
                && row.max_entries_per_tick == 128
        }));
        let protected = snapshot
            .iter()
            .find(|row| row.media_job_public_id == Some(running_id))
            .ok_or_else(|| anyhow::anyhow!("claimed workspace missing"))?;
        assert_eq!(protected.attempt_number, Some(claimed.attempt_number));
        assert_eq!(protected.claim_generation, Some(claimed.claim_generation));
        assert!(
            !super::media_job_worker_interrupt(db.pool(), running_id, claimed.claim_generation)
                .await?
        );
        let interrupted_snapshot = load_media_workspace_retention_snapshot(db.pool()).await?;
        let interrupted = interrupted_snapshot
            .iter()
            .find(|row| row.media_job_public_id == Some(running_id))
            .ok_or_else(|| anyhow::anyhow!("interrupted workspace missing"))?;
        assert_eq!(interrupted.attempt_number, protected.attempt_number);
        assert_eq!(interrupted.claim_generation, protected.claim_generation);
        let interrupted_ids = interrupted_snapshot
            .iter()
            .filter_map(|row| row.media_job_public_id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(interrupted_ids, active_ids);
        Ok(())
    }

    #[tokio::test]
    async fn retention_disabled_is_a_noop_and_failed_count_mode_prunes_only_excess_diagnostics()
    -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        let key = "failed-count-retention";
        let (_roots, older_job_id) =
            crate::media::tests::native_job(db.database(), db.pool(), key, true).await?;
        cancel_job_with_audit(&db, older_job_id).await?;
        let newer_job_id =
            crate::media::tests::additional_native_job(db.pool(), key, "newer.mkv", true).await?;
        cancel_job_with_audit(&db, newer_job_id).await?;

        assert_eq!(
            super::list_media_job_desired_target_streams(db.pool(), older_job_id)
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
        assert_eq!(count_outcome.failed_detail_rows_deleted, 2);
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

        let append = append_media_job_phase(&pool, job_id, 0, 0, "plan", "queued", None).await;
        assert!(append.is_err());

        let list = list_media_jobs(&pool, profile_id, Some("queued")).await;
        assert!(list.is_err());

        let get = get_media_job(&pool, job_id).await;
        assert!(get.is_err());

        let operations = list_media_job_operations(&pool, job_id).await;
        assert!(operations.is_err());

        let append_violation =
            append_media_job_violation(&pool, job_id, 0, 0, "codec_mismatch", "high", Some(0))
                .await;
        assert!(append_violation.is_err());

        let violations = list_media_job_violations(&pool, job_id).await;
        assert!(violations.is_err());

        let append_reason = append_media_job_plan_reason(
            &pool,
            0,
            &AppendMediaJobPlanReasonInput {
                media_job_public_id: job_id,
                reason_index: 0,
                candidate_index: Some(0),
                selected: true,
                reason_code: "least_cost_selected",
                reason_text: "Selected candidate.",
            },
        )
        .await;
        assert!(append_reason.is_err());

        let reasons = list_media_job_plan_reasons(&pool, job_id).await;
        assert!(reasons.is_err());

        let append_check = append_media_job_verification_check(
            &pool,
            0,
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
            0,
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
            0,
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
        let db = setup_media_db().await?;
        let digest_a = "a".repeat(64);
        let digest_b = "b".repeat(64);
        let key = "fingerprint-profile";
        let fingerprint = AssociationFingerprint {
            identity: "000000000000000a:000000000000000a",
            size_bytes: 10,
            modified_ns: 100,
            changed_ns: 100,
            sha256: &digest_a,
        };
        let (_roots, first_job_id) = crate::media::tests::native_job_fingerprint(
            db.database(),
            db.pool(),
            key,
            true,
            fingerprint,
        )
        .await?;
        let association =
            crate::media::associations::read_association_page(db.pool(), 100, None, None)
                .await?
                .into_iter()
                .find(|row| row.association_key == key)
                .ok_or_else(|| anyhow::anyhow!("native association missing"))?;
        let generation = crate::media::root_catalog::read_root_catalog_readiness(db.pool())
            .await?
            .first()
            .and_then(|row| row.attestation_generation)
            .ok_or_else(|| anyhow::anyhow!("native generation missing"))?;
        let profile_id = association.media_profile_public_id;
        let mut input = AssociationJobInput {
            actor_public_id: db.system_user_public_id,
            association_public_id: association.media_discovery_association_public_id,
            association_version: association.latest_version,
            relative_path: "source.mkv",
            dry_run: true,
            trigger: "manual",
            generation,
            generation_sha256: [0x33; 32],
            fingerprint: AssociationFingerprint {
                identity: "000000000000000a:000000000000000a",
                size_bytes: 10,
                modified_ns: 100,
                changed_ns: 100,
                sha256: &digest_a,
            },
        };
        let claimed = media_job_worker_claim_next(db.pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("discovered job was not claimable"))?;
        assert_eq!(claimed.media_job_public_id, first_job_id);
        assert_eq!(claimed.source_size_bytes, 10);
        assert_eq!(claimed.source_modified_ns, 100);
        assert_eq!(claimed.source_sha256, digest_a);
        assert_eq!(enqueue_association_job(db.pool(), &input).await?, None);

        input.fingerprint.modified_ns = 101;
        input.fingerprint.changed_ns = 101;
        assert!(enqueue_association_job(db.pool(), &input).await?.is_some());
        input.fingerprint.sha256 = &digest_b;
        assert!(enqueue_association_job(db.pool(), &input).await?.is_some());

        // Reject the job INSERT after its fingerprint update, inside this owned database.
        db.database()
            .apply_fixture_script(
                include_str!("../../../../scripts/tests/media-fingerprint-inject-failure.sql"),
                &[],
            )
            .await?;
        input.fingerprint.modified_ns = 102;
        input.fingerprint.changed_ns = 102;
        let failure = enqueue_association_job(db.pool(), &input).await;
        match failure {
            Err(error) => assert_eq!(error.database_code().as_deref(), Some("23514")),
            Ok(_) => return Err(anyhow::anyhow!("injected admission failure was accepted")),
        }
        assert_eq!(list_media_jobs(db.pool(), profile_id, None).await?.len(), 3);
        db.database()
            .apply_fixture_script(
                include_str!("../../../../scripts/tests/media-fingerprint-clear-failure.sql"),
                &[],
            )
            .await?;
        assert!(enqueue_association_job(db.pool(), &input).await?.is_some());
        assert_eq!(enqueue_association_job(db.pool(), &input).await?, None);

        let jobs = list_media_jobs(db.pool(), profile_id, None).await?;
        assert_eq!(jobs.len(), 4);
        assert!(jobs.iter().all(|job| job.dry_run));
        Ok(())
    }

    #[tokio::test]
    async fn claimed_job_returns_immutable_source_fingerprint_snapshot() -> anyhow::Result<()> {
        let db = setup_media_db().await?;
        let source_sha256 = "c".repeat(64);
        let input = crate::media::association_jobs::AssociationFingerprint {
            identity: "000000000000000c:000000000000001c",
            size_bytes: 4_096,
            modified_ns: 5_000,
            changed_ns: 6_000,
            sha256: &source_sha256,
        };
        let (_roots, job_id) = crate::media::tests::native_job_fingerprint(
            db.database(),
            db.pool(),
            "claim-fingerprint-profile",
            true,
            input,
        )
        .await?;

        let claimed = media_job_worker_claim_next(db.pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("fingerprinted job should be claimable"))?;

        assert_eq!(claimed.media_job_public_id, job_id);
        assert_eq!(claimed.source_identity, "000000000000000c:000000000000001c");
        assert_eq!(claimed.source_size_bytes, 4_096);
        assert_eq!(claimed.source_modified_ns, 5_000);
        assert_eq!(claimed.source_changed_ns, 6_000);
        assert_eq!(claimed.source_sha256, source_sha256);
        Ok(())
    }
}
