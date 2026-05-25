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

/// Record capability snapshot parameters.
#[derive(Debug, Clone)]
pub struct MediaCapabilityRecordParams<'a> {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
    /// ffmpeg version.
    pub ffmpeg_version: &'a str,
    /// ffprobe version.
    pub ffprobe_version: &'a str,
    /// codec name.
    pub codec_name: &'a str,
    /// encode support.
    pub encode_supported: bool,
    /// decode support.
    pub decode_supported: bool,
}

/// Refresh capability snapshot parameters.
#[derive(Debug, Clone)]
pub struct MediaCapabilityRefreshParams {
    /// Actor performing the operation.
    pub actor_user_public_id: Uuid,
}

/// Profile row used in YAML import/export payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Result of YAML validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaYamlValidationResult {
    /// Schema version string.
    pub version: String,
    /// Validation pass/fail.
    pub valid: bool,
    /// Diagnostic issues.
    pub issues: Vec<String>,
    /// Parsed profile rows.
    pub profiles: Vec<MediaYamlProfile>,
}

/// Result of YAML apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaYamlApplyResult {
    /// Whether dry-run was forced for imported profiles.
    pub forced_dry_run: bool,
    /// Imported profile ids.
    pub media_profile_public_ids: Vec<Uuid>,
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
    /// Updated timestamp.
    pub updated_at: DateTime<Utc>,
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

/// Media capability snapshot response row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaCapabilitySnapshotResponse {
    /// Snapshot id.
    pub media_capability_snapshot_id: i64,
    /// ffmpeg version.
    pub ffmpeg_version: String,
    /// ffprobe version.
    pub ffprobe_version: String,
    /// codec name.
    pub codec_name: String,
    /// encode support.
    pub encode_supported: bool,
    /// decode support.
    pub decode_supported: bool,
    /// observed timestamp.
    pub observed_at: DateTime<Utc>,
}

/// Media capability readiness response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaCapabilityReadinessResponse {
    /// Whether media execution can proceed.
    pub ready: bool,
    /// Reason code when not ready.
    pub reason: Option<String>,
    /// Latest snapshot when available.
    pub snapshot: Option<MediaCapabilitySnapshotResponse>,
}

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

    /// List active profiles.
    async fn media_profile_list(&self) -> Result<Vec<MediaProfileResponse>, MediaServiceError>;

    /// Create media job.
    async fn media_job_create(
        &self,
        params: MediaJobCreateParams<'_>,
    ) -> Result<Uuid, MediaServiceError>;

    /// List media jobs for profile.
    async fn media_job_list(
        &self,
        media_profile_public_id: Uuid,
        status: Option<&str>,
    ) -> Result<Vec<MediaJobResponse>, MediaServiceError>;

    /// Append media job phase.
    async fn media_job_phase_append(
        &self,
        params: MediaJobPhaseAppendParams<'_>,
    ) -> Result<(), MediaServiceError>;

    /// Record capability snapshot row.
    async fn media_capability_record(
        &self,
        params: MediaCapabilityRecordParams<'_>,
    ) -> Result<i64, MediaServiceError>;

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
    async fn media_yaml_export(&self) -> Result<String, MediaServiceError>;

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

    async fn media_job_create(
        &self,
        _params: MediaJobCreateParams<'_>,
    ) -> Result<Uuid, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_job_list(
        &self,
        _media_profile_public_id: Uuid,
        _status: Option<&str>,
    ) -> Result<Vec<MediaJobResponse>, MediaServiceError> {
        Ok(Vec::new())
    }

    async fn media_job_phase_append(
        &self,
        _params: MediaJobPhaseAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
    }

    async fn media_capability_record(
        &self,
        _params: MediaCapabilityRecordParams<'_>,
    ) -> Result<i64, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Storage).with_code("media_unavailable"))
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

    async fn media_yaml_export(&self) -> Result<String, MediaServiceError> {
        Ok("version: revaer.media.v1\nprofiles: []\n".to_string())
    }

    async fn media_yaml_validate(
        &self,
        _yaml_payload: &str,
    ) -> Result<MediaYamlValidationResult, MediaServiceError> {
        Ok(MediaYamlValidationResult {
            version: "revaer.media.v1".to_string(),
            valid: false,
            issues: vec!["media_unavailable".to_string()],
            profiles: Vec::new(),
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
