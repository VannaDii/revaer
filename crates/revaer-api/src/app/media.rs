//! Media application facade and error types.
//!
//! # Design
//! - Expose a narrow async trait for media profile/job/capability operations.
//! - Keep API-facing error mapping stable via typed error kinds and optional codes.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use revaer_api_models::{
    MediaCapabilityCodecResponse as SharedMediaCapabilityCodecResponse,
    MediaCapabilityFeatureResponse as SharedMediaCapabilityFeatureResponse,
    MediaCapabilityReadinessResponse as SharedMediaCapabilityReadinessResponse,
    MediaCapabilitySnapshotResponse as SharedMediaCapabilitySnapshotResponse,
    MediaDesiredTargetStream as SharedMediaDesiredTargetStream,
    MediaJobArtifactResponse as SharedMediaJobArtifactResponse,
    MediaJobCompactAuditResponse as SharedMediaJobCompactAuditResponse,
    MediaJobOperationResponse as SharedMediaJobOperationResponse,
    MediaJobPhaseResponse as SharedMediaJobPhaseResponse,
    MediaJobPlanReasonResponse as SharedMediaJobPlanReasonResponse,
    MediaJobVerificationCheckResponse as SharedMediaJobVerificationCheckResponse,
    MediaJobViolationResponse as SharedMediaJobViolationResponse, MediaVerificationToggle,
};

/// Create/update media profile parameters.
#[derive(Debug, Clone)]
pub struct MediaProfileUpsertParams<'a> {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Stable profile key.
    pub profile_key: &'a str,
    /// Source root path.
    pub source_root: &'a str,
    /// Output root path.
    pub output_root: &'a str,
    /// Dry-run only policy.
    pub dry_run_only: bool,
    /// Retention days.
    pub retention_days: i32,
    /// Optional compatibility target key.
    pub compatibility_target_key: Option<&'a str>,
    /// Operational policy key.
    pub policy_key: &'a str,
    /// Whether filesystem watching is enabled.
    pub watcher_enabled: bool,
    /// Whether scheduled discovery is enabled.
    pub schedule_enabled: bool,
    /// Scheduled discovery interval in minutes.
    pub schedule_interval_minutes: Option<i32>,
}

/// Patch media profile parameters.
#[derive(Debug, Clone)]
pub struct MediaProfilePatchParams<'a> {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Profile public id.
    pub media_profile_public_id: Uuid,
    /// Source root path override.
    pub source_root: Option<&'a str>,
    /// Output root path override.
    pub output_root: Option<&'a str>,
    /// Dry-run only policy override.
    pub dry_run_only: Option<bool>,
    /// Retention days override.
    pub retention_days: Option<i32>,
    /// Optional compatibility target key override.
    pub compatibility_target_key: Option<&'a str>,
    /// Operational policy key override.
    pub policy_key: Option<&'a str>,
    /// Filesystem watcher override.
    pub watcher_enabled: Option<bool>,
    /// Scheduled discovery override.
    pub schedule_enabled: Option<bool>,
    /// Scheduled discovery interval in minutes.
    pub schedule_interval_minutes: Option<i32>,
}

/// Create media job parameters.
#[derive(Debug, Clone)]
pub struct MediaJobCreateParams<'a> {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Owning profile id.
    pub media_profile_public_id: Uuid,
    /// Source path.
    pub source_path: &'a str,
    /// Output path (optional).
    pub output_path: Option<&'a str>,
    /// Dry-run flag.
    pub dry_run: bool,
    /// Exact confirmation phrase required to override a dry-run profile.
    pub replace_confirmation: Option<&'a str>,
}

/// Preview media discovery for source paths under one profile association.
#[derive(Debug, Clone)]
pub struct MediaDiscoveryPreviewParams<'a> {
    /// Profile association used for discovery.
    pub media_profile_public_id: Uuid,
    /// Candidate source paths to preview.
    pub source_paths: &'a [String],
}

/// Run manual media discovery for source paths under one profile association.
#[derive(Debug, Clone)]
pub struct MediaDiscoveryRunParams<'a> {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Profile association used for discovery.
    pub media_profile_public_id: Uuid,
    /// Candidate source paths to discover.
    pub source_paths: &'a [String],
}

/// Run automated media discovery for source paths under one profile association.
#[derive(Debug, Clone)]
pub struct MediaDiscoveryAutomationRunParams<'a> {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Profile association used for discovery.
    pub media_profile_public_id: Uuid,
    /// Candidate source paths discovered by automation.
    pub source_paths: &'a [String],
}

/// Append media job phase parameters.
#[derive(Debug, Clone)]
pub struct MediaJobPhaseAppendParams<'a> {
    /// Job id.
    pub media_job_public_id: Uuid,
    /// Ordered phase index.
    pub phase_index: i32,
    /// Phase name.
    pub phase_name: &'a str,
    /// Phase status (`queued`,`running`,...).
    pub phase_status: &'a str,
    /// Optional detail text.
    pub details_text: Option<&'a str>,
}

/// Append media job operation parameters.
#[derive(Debug, Clone)]
pub struct MediaJobOperationAppendParams<'a> {
    /// Job id.
    pub media_job_public_id: Uuid,
    /// Ordered operation index.
    pub operation_index: i32,
    /// Operation kind.
    pub operation_kind: &'a str,
    /// Optional stream id.
    pub stream_id: Option<i32>,
    /// Command binary.
    pub command_bin: &'a str,
    /// Up to five deterministic arguments.
    pub args: [Option<&'a str>; 5],
}

/// Append media job violation parameters.
#[derive(Debug, Clone)]
pub struct MediaJobViolationAppendParams<'a> {
    /// Job id.
    pub media_job_public_id: Uuid,
    /// Ordered violation index.
    pub violation_index: i32,
    /// Violation kind.
    pub violation_kind: &'a str,
    /// Violation severity.
    pub severity: &'a str,
    /// Optional stream id.
    pub stream_id: Option<i32>,
}

/// Append media job plan-reason parameters.
#[derive(Debug, Clone)]
pub struct MediaJobPlanReasonAppendParams<'a> {
    /// Job id.
    pub media_job_public_id: Uuid,
    /// Ordered reason index.
    pub reason_index: i32,
    /// Optional candidate index.
    pub candidate_index: Option<i32>,
    /// Whether this reason describes the selected plan.
    pub selected: bool,
    /// Stable reason code.
    pub reason_code: &'a str,
    /// Human-readable reason text.
    pub reason_text: &'a str,
}

/// Append media job verification-check parameters.
#[derive(Debug, Clone)]
pub struct MediaJobVerificationCheckAppendParams<'a> {
    /// Job id.
    pub media_job_public_id: Uuid,
    /// Ordered check index.
    pub check_index: i32,
    /// Verification check kind.
    pub check_kind: &'a str,
    /// Verification check status.
    pub check_status: &'a str,
    /// Expected value text.
    pub expected_value: Option<&'a str>,
    /// Actual value text.
    pub actual_value: Option<&'a str>,
    /// Optional detail text.
    pub details_text: Option<&'a str>,
}

/// Append media job artifact parameters.
#[derive(Debug, Clone)]
pub struct MediaJobArtifactAppendParams<'a> {
    /// Job id.
    pub media_job_public_id: Uuid,
    /// Ordered artifact index.
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

/// Append media job compact-audit parameters.
#[derive(Debug, Clone)]
pub struct MediaJobCompactAuditAppendParams<'a> {
    /// Job id.
    pub media_job_public_id: Uuid,
    /// Ordered audit index.
    pub audit_index: i32,
    /// Audit fact kind.
    pub fact_kind: &'a str,
    /// Audit fact text.
    pub fact_text: &'a str,
}

/// Refresh capability snapshot parameters.
#[derive(Debug, Clone)]
pub struct MediaCapabilityRefreshParams {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
}

/// Upsert compatibility target parameters.
#[derive(Debug, Clone)]
pub struct MediaCompatibilityTargetUpsertParams<'a> {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Stable compatibility target key.
    pub compatibility_target_key: &'a str,
    /// Target version.
    pub version: i32,
    /// Operator-facing display name.
    pub display_name: &'a str,
    /// Desired video codec.
    pub video_codec: &'a str,
    /// Desired audio codec.
    pub audio_codec: &'a str,
    /// Optional desired audio channel count.
    pub audio_channels: Option<i32>,
    /// Optional desired audio channel layout.
    pub audio_channel_layout: Option<&'a str>,
    /// Subtitle retention policy.
    pub subtitle_policy: &'a str,
}

/// Ordered desired-target stream parameters.
pub type MediaDesiredTargetStreamParams = SharedMediaDesiredTargetStream;

/// Immutable desired-target version creation parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDesiredTargetCreateParams {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Stable desired-target key.
    pub target_key: String,
    /// Positive immutable version.
    pub version: i32,
    /// Operator-facing display name.
    pub display_name: String,
    /// Desired output container format.
    pub container_format: String,
    /// Complete ordered stream graph.
    pub streams: Vec<MediaDesiredTargetStreamParams>,
}

/// Profile desired-target assignment parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaProfileDesiredTargetParams {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Profile public id.
    pub media_profile_public_id: Uuid,
    /// Desired-target key, or `None` to clear.
    pub target_key: Option<String>,
    /// Exact immutable version.
    pub version: Option<i32>,
}

/// Upsert policy profile parameters.
#[derive(Debug, Clone)]
pub struct MediaPolicyUpsertParams<'a> {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Stable policy key.
    pub policy_key: &'a str,
    /// Policy version.
    pub version: i32,
    /// Operator-facing display name.
    pub display_name: &'a str,
    /// Worker video intent.
    pub video_intent: &'a str,
    /// Verification strictness.
    pub verification_strictness: &'a str,
    /// Maximum source/candidate duration delta in milliseconds.
    pub verification_duration_tolerance_millis: i64,
    /// Whether normalized mux-structure validation is selected.
    pub verification_mux_validation: MediaVerificationToggle,
    /// Whether every candidate stream must decode without errors.
    pub verification_decode_all_streams: MediaVerificationToggle,
    /// Whether midpoint video keyframe seeking must succeed.
    pub verification_keyframe_seek: MediaVerificationToggle,
    /// Whether noninteractive playback smoke verification is selected.
    pub verification_playback_probe: MediaVerificationToggle,
}

/// Update media job retention parameters.
#[derive(Debug, Clone, Copy)]
pub struct MediaJobRetentionUpdateParams {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// Whether completed-job deletion is enabled.
    pub completed_enabled: bool,
    /// Completed-job retention mode.
    pub completed_mode: &'static str,
    /// Completed-job age in days or retained-job count.
    pub completed_limit: i32,
    /// Whether failed-terminal diagnostic pruning is enabled.
    pub failed_diagnostic_enabled: bool,
    /// Failed-terminal diagnostic retention mode.
    pub failed_diagnostic_mode: &'static str,
    /// Failed-terminal diagnostic age in days or retained-job count.
    pub failed_diagnostic_limit: i32,
}

/// Profile row used in YAML import/export payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaYamlProfile {
    /// Stable profile key.
    pub profile_key: String,
    /// Source root path.
    pub source_root: String,
    /// Output root path.
    pub output_root: String,
    /// Dry-run policy.
    pub dry_run_only: bool,
    /// Retention in days.
    pub retention_days: i32,
    /// Optional compatibility target key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_target_key: Option<String>,
    /// Optional immutable desired-target key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_target_key: Option<String>,
    /// Optional immutable desired-target version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_target_version: Option<i32>,
    /// Operational policy key.
    #[serde(default = "default_media_policy_key")]
    pub policy_key: String,
    /// Whether filesystem watching is enabled.
    #[serde(default)]
    pub watcher_enabled: bool,
    /// Whether scheduled discovery is enabled.
    #[serde(default)]
    pub schedule_enabled: bool,
    /// Scheduled discovery interval in minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_interval_minutes: Option<i32>,
}

/// Human-readable metadata for a versioned media configuration bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaYamlMetadata {
    /// Bundle name.
    pub name: String,
    /// Optional bundle description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Compatibility catalog row carried by a media configuration bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaYamlCompatibilityTarget {
    /// Stable catalog key.
    pub compatibility_target_key: String,
    /// Positive catalog version.
    pub version: i32,
    /// Operator-facing label.
    pub display_name: String,
    /// Desired video codec.
    pub video_codec: String,
    /// Desired audio codec.
    pub audio_codec: String,
    /// Optional desired channel count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_channels: Option<i32>,
    /// Optional desired channel layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_channel_layout: Option<String>,
    /// Subtitle retention policy.
    pub subtitle_policy: String,
}

/// Immutable desired target carried by a media configuration bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaYamlDesiredTarget {
    /// Stable target key.
    pub target_key: String,
    /// Positive immutable version.
    pub version: i32,
    /// Operator-facing label.
    pub display_name: String,
    /// Desired output container.
    pub container_format: String,
    /// Complete ordered stream graph.
    pub streams: Vec<MediaDesiredTargetStreamParams>,
}

/// Policy catalog row carried by a media configuration bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaYamlPolicy {
    /// Stable policy key.
    pub policy_key: String,
    /// Positive catalog version.
    pub version: i32,
    /// Operator-facing label.
    pub display_name: String,
    /// Video transcode intent.
    pub video_intent: String,
    /// Verification strictness.
    pub verification_strictness: String,
    /// Maximum source/candidate duration delta in milliseconds.
    pub verification_duration_tolerance_millis: i64,
    /// Whether normalized mux validation is required.
    pub verification_mux_validation: MediaVerificationToggle,
    /// Whether every output stream must decode.
    pub verification_decode_all_streams: MediaVerificationToggle,
    /// Whether midpoint keyframe seeking is required.
    pub verification_keyframe_seek: MediaVerificationToggle,
    /// Whether a playback smoke probe is required.
    pub verification_playback_probe: MediaVerificationToggle,
}

/// Complete versioned media configuration exchange bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaYamlBundle {
    /// Numeric schema format version.
    pub format_version: u32,
    /// Stable bundle kind discriminator.
    pub kind: String,
    /// Human-readable bundle metadata.
    pub metadata: MediaYamlMetadata,
    /// Referenced compatibility catalog rows.
    #[serde(default)]
    pub compatibility_targets: Vec<MediaYamlCompatibilityTarget>,
    /// Referenced immutable desired targets.
    #[serde(default)]
    pub targets: Vec<MediaYamlDesiredTarget>,
    /// Referenced policy catalog rows.
    #[serde(default)]
    pub policies: Vec<MediaYamlPolicy>,
    /// Profile definitions.
    #[serde(default)]
    pub profiles: Vec<MediaYamlProfile>,
}

/// Pointer-addressable YAML validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaYamlIssue {
    /// Stable machine-readable issue code.
    pub code: String,
    /// JSON Pointer location in the parsed bundle.
    pub pointer: String,
    /// Whether the issue prevents import.
    pub blocking: bool,
}

fn default_media_policy_key() -> String {
    "safe_dry_run".to_string()
}

/// Result of YAML validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaYamlValidationResult {
    /// Schema version string.
    pub version: String,
    /// Validation pass/fail.
    pub valid: bool,
    /// Diagnostic issues.
    pub issues: Vec<MediaYamlIssue>,
    /// Parsed bundle.
    pub bundle: MediaYamlBundle,
}

/// Result of YAML apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaYamlApplyResult {
    /// Whether dry-run was forced for imported profiles.
    pub forced_dry_run: bool,
    /// Imported profile ids.
    pub media_profile_public_ids: Vec<Uuid>,
    /// Persisted disabled draft ids awaiting local path mapping.
    pub media_profile_import_draft_public_ids: Vec<Uuid>,
}

/// Media profile response row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaProfileResponse {
    /// Profile public id.
    pub media_profile_public_id: Uuid,
    /// Profile key.
    pub profile_key: String,
    /// Source root.
    pub source_root: String,
    /// Output root.
    pub output_root: String,
    /// Dry-run only flag.
    pub dry_run_only: bool,
    /// Retention days.
    pub retention_days: i32,
    /// Optional compatibility target key.
    pub compatibility_target_key: Option<String>,
    /// Optional pinned desired-target key.
    pub desired_target_key: Option<String>,
    /// Optional pinned desired-target version.
    pub desired_target_version: Option<i32>,
    /// Operational policy key.
    pub policy_key: String,
    /// Whether filesystem watching is enabled.
    pub watcher_enabled: bool,
    /// Whether scheduled discovery is enabled.
    pub schedule_enabled: bool,
    /// Scheduled discovery interval in minutes.
    pub schedule_interval_minutes: Option<i32>,
    /// Updated timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Versioned compatibility target response row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaCompatibilityTargetResponse {
    /// Stable target key.
    pub compatibility_target_key: String,
    /// Catalog version.
    pub version: i32,
    /// Display label.
    pub display_name: String,
    /// Desired video codec.
    pub video_codec: String,
    /// Desired audio codec.
    pub audio_codec: String,
    /// Optional desired audio channel count.
    pub audio_channels: Option<i32>,
    /// Optional desired audio channel layout.
    pub audio_channel_layout: Option<String>,
    /// Subtitle retention policy.
    pub subtitle_policy: String,
}

/// Complete immutable desired-target version response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDesiredTargetResponse {
    /// Desired-target version public id.
    pub media_desired_target_profile_public_id: Uuid,
    /// Stable target key.
    pub target_key: String,
    /// Immutable version.
    pub version: i32,
    /// Display label.
    pub display_name: String,
    /// Desired output container format.
    pub container_format: String,
    /// Complete ordered stream graph.
    pub streams: Vec<MediaDesiredTargetStreamParams>,
}

/// Versioned policy profile response row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaPolicyResponse {
    /// Stable policy key.
    pub policy_key: String,
    /// Catalog version.
    pub version: i32,
    /// Display label.
    pub display_name: String,
    /// Video transcode intent.
    pub video_intent: String,
    /// Verification strictness.
    pub verification_strictness: String,
    /// Maximum source/candidate duration delta in milliseconds.
    pub verification_duration_tolerance_millis: i64,
    /// Whether normalized mux-structure validation is selected.
    pub verification_mux_validation: MediaVerificationToggle,
    /// Whether every candidate stream must decode without errors.
    pub verification_decode_all_streams: MediaVerificationToggle,
    /// Whether midpoint video keyframe seeking must succeed.
    pub verification_keyframe_seek: MediaVerificationToggle,
    /// Whether noninteractive playback smoke verification is selected.
    pub verification_playback_probe: MediaVerificationToggle,
}

/// Media job retention policy response row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaJobRetentionResponse {
    /// Whether completed-job deletion is enabled.
    pub completed_enabled: bool,
    /// Completed-job retention mode.
    pub completed_mode: String,
    /// Completed-job age in days or retained-job count.
    pub completed_limit: i32,
    /// Whether failed-terminal diagnostic pruning is enabled.
    pub failed_diagnostic_enabled: bool,
    /// Failed-terminal diagnostic retention mode.
    pub failed_diagnostic_mode: String,
    /// Failed-terminal diagnostic age in days or retained-job count.
    pub failed_diagnostic_limit: i32,
}

/// Media job response row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaJobResponse {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Source path.
    pub source_path: String,
    /// Output path.
    pub output_path: Option<String>,
    /// Status text.
    pub status: String,
    /// Dry-run flag.
    pub dry_run: bool,
    /// Queued timestamp.
    pub queued_at: DateTime<Utc>,
    /// Started timestamp.
    pub started_at: Option<DateTime<Utc>>,
    /// Completed timestamp.
    pub completed_at: Option<DateTime<Utc>>,
    /// Last error.
    pub last_error: Option<String>,
}

/// Bounded recent-job summary with diagnostic collection counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaRecentJobSummaryResponse {
    /// Existing job fields.
    pub job: MediaJobResponse,
    /// Owning profile.
    pub media_profile_public_id: Uuid,
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

/// Application-level recent-job keyset page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaRecentJobPageResponse {
    /// Bounded page rows.
    pub jobs: Vec<MediaRecentJobSummaryResponse>,
    /// Next keyset position.
    pub next_cursor: Option<(DateTime<Utc>, Uuid)>,
}

/// Discovery preview row for one candidate source path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDiscoveryPreviewResponse {
    /// Candidate source path.
    pub source_path: String,
    /// Derived output path when the candidate matches the profile.
    pub output_path: Option<String>,
    /// Whether a discovered job would run in dry-run mode.
    pub dry_run: bool,
    /// Whether discovery accepted the candidate.
    pub accepted: bool,
    /// Stable rejection reason when rejected.
    pub reason: Option<String>,
}

/// Queued job created by manual media discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDiscoveryQueuedJobResponse {
    /// Queued media job id.
    pub media_job_public_id: Uuid,
    /// Source path selected by discovery.
    pub source_path: String,
    /// Derived output path under the profile output root.
    pub output_path: String,
    /// Whether the queued job will run in dry-run mode.
    pub dry_run: bool,
}

/// Candidate skipped by manual media discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDiscoverySkippedItemResponse {
    /// Candidate source path.
    pub source_path: String,
    /// Stable rejection reason when available.
    pub reason: Option<String>,
}

/// Manual media discovery run response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDiscoveryRunResponse {
    /// Jobs queued for accepted candidates.
    pub queued_jobs: Vec<MediaDiscoveryQueuedJobResponse>,
    /// Candidates skipped before queueing.
    pub skipped: Vec<MediaDiscoverySkippedItemResponse>,
}

/// Media job operation response row.
pub type MediaJobOperationResponse = SharedMediaJobOperationResponse;

/// Media job phase response row.
pub type MediaJobPhaseResponse = SharedMediaJobPhaseResponse;

/// Media job compliance violation response row.
pub type MediaJobViolationResponse = SharedMediaJobViolationResponse;

/// Media job plan-reason response row.
pub type MediaJobPlanReasonResponse = SharedMediaJobPlanReasonResponse;

/// Media job verification check response row.
pub type MediaJobVerificationCheckResponse = SharedMediaJobVerificationCheckResponse;

/// Media job artifact response row.
pub type MediaJobArtifactResponse = SharedMediaJobArtifactResponse;

/// Media job compact-audit response row.
pub type MediaJobCompactAuditResponse = SharedMediaJobCompactAuditResponse;

/// Codec row within a media capability snapshot run.
pub type MediaCapabilityCodecResponse = SharedMediaCapabilityCodecResponse;

/// Additional capability feature row within a media capability snapshot run.
pub type MediaCapabilityFeatureResponse = SharedMediaCapabilityFeatureResponse;

/// Media capability snapshot response row.
pub type MediaCapabilitySnapshotResponse = SharedMediaCapabilitySnapshotResponse;

/// Media capability readiness response.
pub type MediaCapabilityReadinessResponse = SharedMediaCapabilityReadinessResponse;

/// Media service error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaServiceErrorKind {
    /// Input validation or semantic failure.
    Invalid,
    /// Referenced resource not found.
    NotFound,
    /// Conflicting state.
    Conflict,
    /// Persistence or unknown backend failure.
    Storage,
}

/// Typed media service error.
#[derive(Debug, Clone)]
pub struct MediaServiceError {
    kind: MediaServiceErrorKind,
    code: Option<String>,
    sqlstate: Option<String>,
}

impl MediaServiceError {
    /// Construct from a kind.
    #[must_use]
    pub const fn new(kind: MediaServiceErrorKind) -> Self {
        Self {
            kind,
            code: None,
            sqlstate: None,
        }
    }

    /// Attach stable error code.
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Attach SQLSTATE.
    #[must_use]
    pub fn with_sqlstate(mut self, sqlstate: impl Into<String>) -> Self {
        self.sqlstate = Some(sqlstate.into());
        self
    }

    /// Error kind.
    #[must_use]
    pub const fn kind(&self) -> MediaServiceErrorKind {
        self.kind
    }

    /// Optional stable code.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Optional SQLSTATE.
    #[must_use]
    pub fn sqlstate(&self) -> Option<&str> {
        self.sqlstate.as_deref()
    }
}

impl Display for MediaServiceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("media service error")
    }
}

impl Error for MediaServiceError {}

/// Facade for media API operations.
#[async_trait]
pub trait MediaFacade: Send + Sync {
    /// Upsert profile and return profile id.
    async fn media_profile_upsert(
        &self,
        params: MediaProfileUpsertParams<'_>,
    ) -> Result<Uuid, MediaServiceError>;

    /// Patch profile and return profile id.
    async fn media_profile_patch(
        &self,
        params: MediaProfilePatchParams<'_>,
    ) -> Result<Uuid, MediaServiceError>;

    /// List active profiles.
    async fn media_profile_list(&self) -> Result<Vec<MediaProfileResponse>, MediaServiceError>;

    /// List active compatibility target versions.
    async fn media_compatibility_target_list(
        &self,
    ) -> Result<Vec<MediaCompatibilityTargetResponse>, MediaServiceError>;

    /// Create or replace a compatibility target version.
    async fn media_compatibility_target_upsert(
        &self,
        params: MediaCompatibilityTargetUpsertParams<'_>,
    ) -> Result<MediaCompatibilityTargetResponse, MediaServiceError>;

    /// List complete immutable desired-target versions.
    async fn media_desired_target_list(
        &self,
    ) -> Result<Vec<MediaDesiredTargetResponse>, MediaServiceError>;

    /// Atomically create one immutable desired-target version and its stream graph.
    async fn media_desired_target_create(
        &self,
        params: MediaDesiredTargetCreateParams,
    ) -> Result<MediaDesiredTargetResponse, MediaServiceError>;

    /// Pin or clear the immutable desired-target version for one profile.
    async fn media_profile_desired_target_set(
        &self,
        params: MediaProfileDesiredTargetParams,
    ) -> Result<Uuid, MediaServiceError>;

    /// List active policy profile versions.
    async fn media_policy_list(&self) -> Result<Vec<MediaPolicyResponse>, MediaServiceError>;

    /// Create or replace a policy profile version.
    async fn media_policy_upsert(
        &self,
        params: MediaPolicyUpsertParams<'_>,
    ) -> Result<MediaPolicyResponse, MediaServiceError>;

    /// Read active job retention policy.
    async fn media_job_retention(&self) -> Result<MediaJobRetentionResponse, MediaServiceError>;

    /// Update active job retention policy.
    async fn media_job_retention_update(
        &self,
        params: MediaJobRetentionUpdateParams,
    ) -> Result<MediaJobRetentionResponse, MediaServiceError>;

    /// Create media job.
    async fn media_job_create(
        &self,
        params: MediaJobCreateParams<'_>,
    ) -> Result<Uuid, MediaServiceError>;

    /// Preview manual discovery for candidate source paths.
    async fn media_discovery_preview(
        &self,
        params: MediaDiscoveryPreviewParams<'_>,
    ) -> Result<Vec<MediaDiscoveryPreviewResponse>, MediaServiceError>;

    /// Run manual discovery and queue jobs for accepted candidates.
    async fn media_discovery_run(
        &self,
        params: MediaDiscoveryRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError>;

    /// Run enabled scheduled discovery and queue jobs for accepted candidates.
    async fn media_discovery_schedule_run(
        &self,
        params: MediaDiscoveryAutomationRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError>;

    /// Run enabled watcher discovery and queue jobs for accepted candidates.
    async fn media_discovery_watcher_run(
        &self,
        params: MediaDiscoveryAutomationRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError>;

    /// List media jobs for profile.
    async fn media_job_list(
        &self,
        media_profile_public_id: Uuid,
        status: Option<&str>,
    ) -> Result<Vec<MediaJobResponse>, MediaServiceError>;

    /// Read one bounded recent-job page.
    async fn media_job_recent(
        &self,
        limit: i32,
        cursor: Option<(DateTime<Utc>, Uuid)>,
        media_profile_public_id: Option<Uuid>,
    ) -> Result<MediaRecentJobPageResponse, MediaServiceError>;

    /// Read one media job by public id.
    async fn media_job_get(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Option<MediaJobResponse>, MediaServiceError>;

    /// Cancel a queued, running, or verifying media job.
    async fn media_job_cancel(&self, media_job_public_id: Uuid) -> Result<(), MediaServiceError>;

    /// Retry a failed or cancelled media job.
    async fn media_job_retry(&self, media_job_public_id: Uuid) -> Result<(), MediaServiceError>;

    /// Append media job phase.
    async fn media_job_phase_append(
        &self,
        params: MediaJobPhaseAppendParams<'_>,
    ) -> Result<(), MediaServiceError>;

    /// List persisted media job phases.
    async fn media_job_phase_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobPhaseResponse>, MediaServiceError>;

    /// Append media job operation.
    async fn media_job_operation_append(
        &self,
        params: MediaJobOperationAppendParams<'_>,
    ) -> Result<(), MediaServiceError>;

    /// List persisted media job operations.
    async fn media_job_operation_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobOperationResponse>, MediaServiceError>;

    /// Append media job violation.
    async fn media_job_violation_append(
        &self,
        params: MediaJobViolationAppendParams<'_>,
    ) -> Result<(), MediaServiceError>;

    /// List persisted media job violations.
    async fn media_job_violation_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobViolationResponse>, MediaServiceError>;

    /// Append media job plan reason.
    async fn media_job_plan_reason_append(
        &self,
        params: MediaJobPlanReasonAppendParams<'_>,
    ) -> Result<(), MediaServiceError>;

    /// List persisted media job plan reasons.
    async fn media_job_plan_reason_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobPlanReasonResponse>, MediaServiceError>;

    /// Append media job verification check.
    async fn media_job_verification_check_append(
        &self,
        params: MediaJobVerificationCheckAppendParams<'_>,
    ) -> Result<(), MediaServiceError>;

    /// List persisted media job verification checks.
    async fn media_job_verification_check_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobVerificationCheckResponse>, MediaServiceError>;

    /// Append media job artifact reference.
    async fn media_job_artifact_append(
        &self,
        params: MediaJobArtifactAppendParams<'_>,
    ) -> Result<(), MediaServiceError>;

    /// List persisted media job artifact references.
    async fn media_job_artifact_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobArtifactResponse>, MediaServiceError>;

    /// Append media job compact audit fact.
    async fn media_job_compact_audit_append(
        &self,
        params: MediaJobCompactAuditAppendParams<'_>,
    ) -> Result<(), MediaServiceError>;

    /// List persisted media job compact audit facts.
    async fn media_job_compact_audit_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobCompactAuditResponse>, MediaServiceError>;

    /// Refresh capability snapshot from runtime detector.
    async fn media_capability_refresh(
        &self,
        params: MediaCapabilityRefreshParams,
    ) -> Result<i64, MediaServiceError>;

    /// Read latest capability snapshot row when available.
    async fn media_capability_latest(
        &self,
    ) -> Result<Option<MediaCapabilitySnapshotResponse>, MediaServiceError>;

    /// Read current capability readiness.
    async fn media_capability_readiness(
        &self,
    ) -> Result<MediaCapabilityReadinessResponse, MediaServiceError>;

    /// Export active media profiles as versioned YAML.
    async fn media_yaml_export(
        &self,
        include_local_paths: bool,
    ) -> Result<String, MediaServiceError>;

    /// Validate YAML import payload and return parsed semantics.
    async fn media_yaml_validate(
        &self,
        yaml_payload: &str,
    ) -> Result<MediaYamlValidationResult, MediaServiceError>;

    /// Apply YAML import payload to profile storage.
    async fn media_yaml_apply(
        &self,
        actor_user_public_id: Uuid,
        yaml_payload: &str,
    ) -> Result<MediaYamlApplyResult, MediaServiceError>;
}

#[derive(Default)]
pub(crate) struct NoopMedia;

#[async_trait]
impl MediaFacade for NoopMedia {
    async fn media_profile_upsert(
        &self,
        _params: MediaProfileUpsertParams<'_>,
    ) -> Result<Uuid, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_profile_list(&self) -> Result<Vec<MediaProfileResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_job_recent(
        &self,
        _limit: i32,
        _cursor: Option<(DateTime<Utc>, Uuid)>,
        _media_profile_public_id: Option<Uuid>,
    ) -> Result<MediaRecentJobPageResponse, MediaServiceError> {
        Ok(MediaRecentJobPageResponse {
            jobs: Vec::new(),
            next_cursor: None,
        })
    }

    async fn media_compatibility_target_list(
        &self,
    ) -> Result<Vec<MediaCompatibilityTargetResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_compatibility_target_upsert(
        &self,
        _params: MediaCompatibilityTargetUpsertParams<'_>,
    ) -> Result<MediaCompatibilityTargetResponse, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_desired_target_list(
        &self,
    ) -> Result<Vec<MediaDesiredTargetResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_desired_target_create(
        &self,
        _params: MediaDesiredTargetCreateParams,
    ) -> Result<MediaDesiredTargetResponse, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_profile_desired_target_set(
        &self,
        _params: MediaProfileDesiredTargetParams,
    ) -> Result<Uuid, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_policy_list(&self) -> Result<Vec<MediaPolicyResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_policy_upsert(
        &self,
        _params: MediaPolicyUpsertParams<'_>,
    ) -> Result<MediaPolicyResponse, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_retention(&self) -> Result<MediaJobRetentionResponse, MediaServiceError> {
        Ok(MediaJobRetentionResponse {
            completed_enabled: false,
            completed_mode: "age".to_string(),
            completed_limit: 30,
            failed_diagnostic_enabled: true,
            failed_diagnostic_mode: "age".to_string(),
            failed_diagnostic_limit: 30,
        })
    }

    async fn media_job_retention_update(
        &self,
        _params: MediaJobRetentionUpdateParams,
    ) -> Result<MediaJobRetentionResponse, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_profile_patch(
        &self,
        _params: MediaProfilePatchParams<'_>,
    ) -> Result<Uuid, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_create(
        &self,
        _params: MediaJobCreateParams<'_>,
    ) -> Result<Uuid, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_discovery_preview(
        &self,
        _params: MediaDiscoveryPreviewParams<'_>,
    ) -> Result<Vec<MediaDiscoveryPreviewResponse>, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_discovery_run(
        &self,
        _params: MediaDiscoveryRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_discovery_schedule_run(
        &self,
        _params: MediaDiscoveryAutomationRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_discovery_watcher_run(
        &self,
        _params: MediaDiscoveryAutomationRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_list(
        &self,
        _media_profile_public_id: Uuid,
        _status: Option<&str>,
    ) -> Result<Vec<MediaJobResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_job_get(
        &self,
        _media_job_public_id: Uuid,
    ) -> Result<Option<MediaJobResponse>, MediaServiceError> {
        Ok(None)
    }

    async fn media_job_cancel(&self, _media_job_public_id: Uuid) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_retry(&self, _media_job_public_id: Uuid) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_phase_append(
        &self,
        _params: MediaJobPhaseAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_phase_list(
        &self,
        _media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobPhaseResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_job_operation_append(
        &self,
        _params: MediaJobOperationAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_operation_list(
        &self,
        _media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobOperationResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_job_violation_append(
        &self,
        _params: MediaJobViolationAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_violation_list(
        &self,
        _media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobViolationResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_job_plan_reason_append(
        &self,
        _params: MediaJobPlanReasonAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_plan_reason_list(
        &self,
        _media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobPlanReasonResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_job_verification_check_append(
        &self,
        _params: MediaJobVerificationCheckAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_verification_check_list(
        &self,
        _media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobVerificationCheckResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_job_artifact_append(
        &self,
        _params: MediaJobArtifactAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_artifact_list(
        &self,
        _media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobArtifactResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_job_compact_audit_append(
        &self,
        _params: MediaJobCompactAuditAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_compact_audit_list(
        &self,
        _media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobCompactAuditResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_capability_refresh(
        &self,
        _params: MediaCapabilityRefreshParams,
    ) -> Result<i64, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_capability_latest(
        &self,
    ) -> Result<Option<MediaCapabilitySnapshotResponse>, MediaServiceError> {
        Ok(None)
    }

    async fn media_capability_readiness(
        &self,
    ) -> Result<MediaCapabilityReadinessResponse, MediaServiceError> {
        Ok(MediaCapabilityReadinessResponse {
            ready: false,
            reason: Some("media_capability_snapshot_missing".to_string()),
            snapshot: None,
        })
    }

    async fn media_yaml_export(
        &self,
        _include_local_paths: bool,
    ) -> Result<String, MediaServiceError> {
        Ok("format_version: 1\nkind: revaer.media.profile_bundle\nmetadata:\n  name: Revaer media configuration\nprofiles: []\n".to_string())
    }

    async fn media_yaml_validate(
        &self,
        _yaml_payload: &str,
    ) -> Result<MediaYamlValidationResult, MediaServiceError> {
        Ok(MediaYamlValidationResult {
            version: "1".to_string(),
            valid: false,
            issues: vec![MediaYamlIssue {
                code: "media_unavailable".to_string(),
                pointer: String::new(),
                blocking: true,
            }],
            bundle: MediaYamlBundle {
                format_version: 1,
                kind: "revaer.media.profile_bundle".to_string(),
                metadata: MediaYamlMetadata {
                    name: "Revaer media configuration".to_string(),
                    description: None,
                },
                compatibility_targets: Vec::new(),
                targets: Vec::new(),
                policies: Vec::new(),
                profiles: Vec::new(),
            },
        })
    }

    async fn media_yaml_apply(
        &self,
        _actor_user_public_id: Uuid,
        _yaml_payload: &str,
    ) -> Result<MediaYamlApplyResult, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }
}

pub(crate) fn noop_media() -> std::sync::Arc<dyn MediaFacade> {
    std::sync::Arc::new(NoopMedia)
}
