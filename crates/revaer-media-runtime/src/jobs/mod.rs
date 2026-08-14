//! Media job status models.

use revaer_media_core::compliance::{Report as ComplianceReport, score_diff};
use revaer_media_core::diff::diff_graphs;
use revaer_media_core::explain::{Explanation, explain_plan};
use revaer_media_core::model::{DesiredGraph, MediaGraph};
use revaer_media_core::pipeline::PlanningOutcome;
use revaer_media_core::plan::{
    OperationKind, PlanGenerationError, PlanSelection, PlannedOperation, generate_plan,
};
use revaer_media_core::target::{
    CompiledDesiredTarget, DesiredSidecarOutput, SidecarEmbedding, SidecarOutputSource,
};
use revaer_media_core::verify::{verify_plan_against_graphs, verify_unique_stream_ids};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

use crate::capabilities::CapabilitySnapshot;
use crate::execute::{
    BuildArgsError, DesiredGraphBuildContext, ExecutionStep, ExecutionStepAudit,
    SubtitleArtifactPlan, VideoTranscodePolicy, build_desired_graph_execution_steps_with_sidecars,
    compile_execution_step_audits,
};
use crate::inspect::{InspectAdapter, InspectError};
use crate::workspace::{
    WorkspaceCapacityReport, WorkspaceError, WorkspacePolicy, WorkspaceRejectionReason,
};

/// Execution phase for a media job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobPhase {
    /// Planning has started.
    Planning,
    /// Active execution.
    Running,
    /// Final verification stage.
    Verifying,
    /// Job completed.
    Completed,
    /// Job failed.
    Failed,
}

/// Runtime media job state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaJob {
    /// Stable job id.
    pub job_id: String,
    /// Source path.
    pub source_path: String,
    /// Current phase.
    pub phase: JobPhase,
}

/// Normalized planning output for one media job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedJob {
    /// Inspected source graph used to generate the plan.
    pub source: Box<MediaGraph>,
    /// Desired output graph used to generate the plan.
    pub desired: Box<DesiredGraph>,
    /// Existing sidecars selected as embedded media inputs.
    pub sidecar_embeddings: Vec<SidecarEmbedding>,
    /// Managed sidecar outputs selected by the target.
    pub sidecar_outputs: Vec<DesiredSidecarOutput>,
    /// Existing sidecars removed only after verified replacement.
    pub sidecar_removals: Vec<String>,
    /// Generated deterministic operations.
    pub operations: Vec<PlannedOperation>,
    /// Diff-based compliance report.
    pub compliance: ComplianceReport,
    /// Inspected source duration used for bitrate-derived workspace planning.
    pub source_duration_millis: Option<u64>,
    /// Estimated temporary workspace usage in bytes.
    pub estimated_workspace_bytes: u64,
}

/// Deterministic summary for planned operations and explainability rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedJobSummary {
    /// Total planned operation count.
    pub total_operations: usize,
    /// Count of remux operations.
    pub remux_operations: usize,
    /// Count of metadata rewrite operations.
    pub metadata_rewrite_operations: usize,
    /// Count of disposition rewrite operations.
    pub disposition_rewrite_operations: usize,
    /// Count of label rewrite operations.
    pub label_rewrite_operations: usize,
    /// Count of stream reorder operations.
    pub stream_reorder_operations: usize,
    /// Count of sidecar-to-container subtitle embeds.
    pub embed_subtitle_operations: usize,
    /// Count of embedded-to-sidecar subtitle extractions.
    pub extract_subtitle_operations: usize,
    /// Count of managed sidecar copies or conversions.
    pub copy_sidecar_subtitle_operations: usize,
    /// Count of verified source-sidecar removals.
    pub remove_sidecar_subtitle_operations: usize,
    /// Count of subtitle codec conversions.
    pub subtitle_transcode_operations: usize,
    /// Count of audio transcode operations.
    pub audio_transcode_operations: usize,
    /// Count of video transcode operations.
    pub video_transcode_operations: usize,
    /// Deterministic operation explanations.
    pub explanations: Vec<Explanation>,
}

/// Deterministic preflight report for one planned media job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPreflightReport {
    /// Planned job operations and workspace estimate.
    pub planned: PlannedJob,
    /// Structured operation summary and explanations.
    pub summary: PlannedJobSummary,
    /// Deterministic execution steps validated against capabilities.
    pub steps: Vec<ExecutionStep>,
    /// Stable compiled-step to logical-operation audit mapping.
    pub step_audits: Vec<ExecutionStepAudit>,
    /// Stage-by-stage deterministic preflight timeline.
    pub timeline: Vec<PreflightStageRecord>,
    /// Structured workspace capacity decision used during preflight.
    pub capacity_report: WorkspaceCapacityReport,
}

/// Deterministic structured preflight failure payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPreflightFailureReport {
    /// Stage where preflight failed.
    pub failed_stage: &'static str,
    /// Machine-readable failure code.
    pub error_code: &'static str,
    /// Deterministic human-readable detail.
    pub error_detail: &'static str,
    /// Stage timeline projected from the failure.
    pub timeline: Vec<PreflightStageRecord>,
}

/// Deterministic preflight evaluation outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobPreflightEvaluation {
    /// Successful preflight report.
    Ready(Box<JobPreflightReport>),
    /// Failed preflight report with structured diagnostics.
    Failed(JobPreflightFailureReport),
}

impl JobPreflightEvaluation {
    /// Returns true when preflight evaluation is ready for execution.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// Borrow the ready report when preflight evaluation succeeded.
    #[must_use]
    pub const fn as_ready(&self) -> Option<&JobPreflightReport> {
        match self {
            Self::Ready(report) => Some(report),
            Self::Failed(_) => None,
        }
    }

    /// Borrow the failed report when preflight evaluation failed.
    #[must_use]
    pub const fn as_failed(&self) -> Option<&JobPreflightFailureReport> {
        match self {
            Self::Ready(_) => None,
            Self::Failed(report) => Some(report),
        }
    }

    /// Return the failed stage when preflight evaluation failed.
    #[must_use]
    pub const fn failed_stage(&self) -> Option<&'static str> {
        match self {
            Self::Ready(_) => None,
            Self::Failed(report) => Some(report.failed_stage),
        }
    }

    /// Return the machine-readable error code when preflight evaluation failed.
    #[must_use]
    pub const fn error_code(&self) -> Option<&'static str> {
        match self {
            Self::Ready(_) => None,
            Self::Failed(report) => Some(report.error_code),
        }
    }

    /// Return the deterministic human-readable error detail when preflight evaluation failed.
    #[must_use]
    pub const fn error_detail(&self) -> Option<&'static str> {
        match self {
            Self::Ready(_) => None,
            Self::Failed(report) => Some(report.error_detail),
        }
    }

    /// Return the machine-readable code for the failed stage when preflight evaluation failed.
    #[must_use]
    pub fn failed_stage_code(&self) -> Option<&'static str> {
        self.as_failed()
            .and_then(|report| report.timeline.last())
            .and_then(|record| record.code)
    }

    /// Return the final timeline stage for ready or failed preflight outcomes.
    #[must_use]
    pub fn final_stage(&self) -> Option<&'static str> {
        self.timeline().last().map(|record| record.stage)
    }

    /// Return the final timeline stage code for ready or failed preflight outcomes.
    #[must_use]
    pub fn final_stage_code(&self) -> Option<&'static str> {
        self.timeline().last().and_then(|record| record.code)
    }

    /// Return true when preflight failed at the build-steps stage.
    #[must_use]
    pub fn failed_at_build_steps(&self) -> bool {
        matches!(self.failed_stage(), Some("build_steps"))
    }

    /// Borrow the stage timeline for both ready and failed outcomes.
    #[must_use]
    pub fn timeline(&self) -> &[PreflightStageRecord] {
        match self {
            Self::Ready(report) => &report.timeline,
            Self::Failed(report) => &report.timeline,
        }
    }

    /// Borrow planned job when preflight evaluation succeeded.
    #[must_use]
    pub const fn planned(&self) -> Option<&PlannedJob> {
        match self {
            Self::Ready(report) => Some(&report.planned),
            Self::Failed(_) => None,
        }
    }

    /// Borrow planned job summary when preflight evaluation succeeded.
    #[must_use]
    pub const fn summary(&self) -> Option<&PlannedJobSummary> {
        match self {
            Self::Ready(report) => Some(&report.summary),
            Self::Failed(_) => None,
        }
    }

    /// Borrow deterministic execution steps when preflight evaluation succeeded.
    #[must_use]
    pub fn steps(&self) -> Option<&[ExecutionStep]> {
        match self {
            Self::Ready(report) => Some(&report.steps),
            Self::Failed(_) => None,
        }
    }

    /// Borrow workspace capacity report when preflight evaluation succeeded.
    #[must_use]
    pub const fn capacity_report(&self) -> Option<&WorkspaceCapacityReport> {
        match self {
            Self::Ready(report) => Some(&report.capacity_report),
            Self::Failed(_) => None,
        }
    }
}

/// Deterministic stage record for preflight explainability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightStageRecord {
    /// Stage identifier.
    pub stage: &'static str,
    /// Whether this stage succeeded.
    pub ok: bool,
    /// Optional machine-readable stage code.
    pub code: Option<&'static str>,
}

/// Compact audit fact ready for durable media-job audit persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactAuditFact {
    /// Stable fact order index within a job.
    pub audit_index: i32,
    /// Machine-readable fact kind.
    pub fact_kind: &'static str,
    /// Bounded deterministic fact text.
    pub fact_text: String,
}

/// Preflight request inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPreflightRequest {
    /// Actual inspected source graph.
    pub source: MediaGraph,
    /// Desired compiled output graph.
    pub desired: DesiredGraph,
    /// Source file size in bytes.
    pub source_file_bytes: u64,
}

/// Preflight failure when inspection/planning cannot complete.
#[derive(Debug, Error)]
pub enum JobPreflightError {
    /// Source inspection failed.
    #[error(transparent)]
    Inspect(#[from] InspectError),
    /// Planning verification failed.
    #[error("plan verification failed: {0}")]
    Plan(&'static str),
    /// Capability snapshot failed readiness checks.
    #[error("capability preflight failed: {0}")]
    Capability(&'static str),
    /// Workspace reserve or capacity check failed.
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    /// Execution-step construction failed.
    #[error(transparent)]
    Build(#[from] BuildArgsError),
    /// Backup path could not be resolved from configured backup root and source path.
    #[error("backup preflight failed: {0}")]
    BackupPath(BackupPathError),
    /// Quarantine path could not be resolved from configured quarantine root and source path.
    #[error("quarantine preflight failed: {0}")]
    QuarantinePath(QuarantinePathError),
}

/// Deterministic backup-path validation errors for preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupPathError {
    /// Backup root was configured but source path has no file name.
    SourceFileNameMissing,
    /// Backup path resolves to source path.
    MatchesSourcePath,
    /// Backup path resolves to output path.
    MatchesOutputPath,
}

impl std::fmt::Display for BackupPathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::SourceFileNameMissing => "configured backup root requires a source file name",
            Self::MatchesSourcePath => "backup path must not match source path",
            Self::MatchesOutputPath => "backup path must not match output path",
        };
        formatter.write_str(message)
    }
}

/// Deterministic quarantine-path validation errors for preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuarantinePathError {
    /// Quarantine root was configured but source path has no file name.
    SourceFileNameMissing,
    /// Quarantine path resolves to source path.
    MatchesSourcePath,
    /// Quarantine path resolves to output path.
    MatchesOutputPath,
    /// Quarantine path resolves to backup path.
    MatchesBackupPath,
}

impl std::fmt::Display for QuarantinePathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::SourceFileNameMissing => "configured quarantine root requires a source file name",
            Self::MatchesSourcePath => "quarantine path must not match source path",
            Self::MatchesOutputPath => "quarantine path must not match output path",
            Self::MatchesBackupPath => "quarantine path must not match backup path",
        };
        formatter.write_str(message)
    }
}

/// Inputs required to build/evaluate a preflight report.
#[derive(Debug, Clone)]
pub struct PreflightBuildInput<'a> {
    /// Source path used for inspection.
    pub source_path: &'a str,
    /// Output path used for execution-step generation.
    pub output_path: &'a str,
    /// Optional backup path used before replacement.
    pub backup_path: Option<&'a str>,
    /// Optional quarantine path for failed generated output.
    pub quarantine_path: Option<&'a str>,
    /// Desired output graph.
    pub desired: &'a DesiredGraph,
    /// Source file size in bytes.
    pub source_file_bytes: u64,
    /// Capability snapshot used to validate required codecs.
    pub capabilities: &'a CapabilitySnapshot,
    /// Workspace policy used for capacity checks.
    pub workspace_policy: &'a WorkspacePolicy,
    /// Free disk bytes available at preflight time.
    pub free_bytes: u64,
    /// Video transcode policy derived from profile/target settings.
    pub video_policy: VideoTranscodePolicy,
}

/// Policy-derived inputs used to construct preflight requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightPolicyInput<'a> {
    /// Optional backup root configured by policy/profile.
    pub backup_root: Option<&'a str>,
    /// Optional quarantine root configured by policy/profile.
    pub quarantine_root: Option<&'a str>,
    /// Video transcode policy configured by profile/target settings.
    pub video_policy: VideoTranscodePolicy,
}

/// Owned preflight input payload that can safely produce borrowed preflight view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedPreflightBuildInput<'a> {
    /// Source path used for inspection.
    pub source_path: &'a str,
    /// Output path used for execution-step generation.
    pub output_path: &'a str,
    /// Optional backup path used before replacement.
    pub backup_path: Option<String>,
    /// Optional quarantine path for failed generated output.
    pub quarantine_path: Option<String>,
    /// Desired output graph.
    pub desired: &'a DesiredGraph,
    /// Source file size in bytes.
    pub source_file_bytes: u64,
    /// Capability snapshot used to validate required codecs.
    pub capabilities: &'a CapabilitySnapshot,
    /// Workspace policy used for capacity checks.
    pub workspace_policy: &'a WorkspacePolicy,
    /// Free disk bytes available at preflight time.
    pub free_bytes: u64,
    /// Video transcode policy derived from profile/target settings.
    pub video_policy: VideoTranscodePolicy,
}

/// Template fields for building owned preflight input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreflightBuildTemplate<'a> {
    /// Source path used for inspection.
    pub source_path: &'a str,
    /// Output path used for execution-step generation.
    pub output_path: &'a str,
    /// Desired output graph.
    pub desired: &'a DesiredGraph,
    /// Source file size in bytes.
    pub source_file_bytes: u64,
    /// Capability snapshot used to validate required codecs.
    pub capabilities: &'a CapabilitySnapshot,
    /// Workspace policy used for capacity checks.
    pub workspace_policy: &'a WorkspacePolicy,
    /// Free disk bytes available at preflight time.
    pub free_bytes: u64,
}

impl<'a> OwnedPreflightBuildInput<'a> {
    /// Borrow as [`PreflightBuildInput`] for preflight evaluation functions.
    #[must_use]
    pub fn as_borrowed(&'a self) -> PreflightBuildInput<'a> {
        PreflightBuildInput {
            source_path: self.source_path,
            output_path: self.output_path,
            backup_path: self.backup_path.as_deref(),
            quarantine_path: self.quarantine_path.as_deref(),
            desired: self.desired,
            source_file_bytes: self.source_file_bytes,
            capabilities: self.capabilities,
            workspace_policy: self.workspace_policy,
            free_bytes: self.free_bytes,
            video_policy: self.video_policy.clone(),
        }
    }
}

/// Resolve deterministic backup output path from optional backup root and source path.
#[must_use]
pub fn resolve_backup_path(backup_root: Option<&str>, source_path: &str) -> Option<String> {
    resolve_managed_artifact_path(backup_root, source_path)
}

/// Resolve deterministic quarantine output path from optional quarantine root and source path.
#[must_use]
pub fn resolve_quarantine_path(quarantine_root: Option<&str>, source_path: &str) -> Option<String> {
    resolve_managed_artifact_path(quarantine_root, source_path)
}

fn resolve_managed_artifact_path(root: Option<&str>, source_path: &str) -> Option<String> {
    let root = root.map(str::trim).filter(|value| !value.is_empty())?;
    let file_name = Path::new(source_path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(format!("{root}/{file_name}"))
}

/// Build preflight input while resolving backup path from policy.
#[must_use]
pub fn build_preflight_input<'a>(
    template: PreflightBuildTemplate<'a>,
    policy_input: PreflightPolicyInput<'a>,
) -> OwnedPreflightBuildInput<'a> {
    let backup_path = resolve_backup_path(policy_input.backup_root, template.source_path);
    let quarantine_path =
        resolve_quarantine_path(policy_input.quarantine_root, template.source_path);
    OwnedPreflightBuildInput {
        source_path: template.source_path,
        output_path: template.output_path,
        backup_path,
        quarantine_path,
        desired: template.desired,
        source_file_bytes: template.source_file_bytes,
        capabilities: template.capabilities,
        workspace_policy: template.workspace_policy,
        free_bytes: template.free_bytes,
        video_policy: policy_input.video_policy,
    }
}

/// Deterministic machine-readable error code for preflight failures.
#[must_use]
pub fn preflight_error_code(error: &JobPreflightError) -> &'static str {
    preflight_error_metadata(error).code
}

/// Deterministic human-readable detail for preflight failures.
#[must_use]
pub fn preflight_error_detail(error: &JobPreflightError) -> &'static str {
    preflight_error_metadata(error).detail
}

struct PreflightErrorMetadata {
    code: &'static str,
    detail: &'static str,
}

fn preflight_error_metadata(error: &JobPreflightError) -> PreflightErrorMetadata {
    match error {
        JobPreflightError::Inspect(_) => PreflightErrorMetadata {
            code: "preflight_inspect_failed",
            detail: "source inspection failed",
        },
        JobPreflightError::Plan(plan_error) => preflight_plan_metadata(plan_error),
        JobPreflightError::Capability(_) => PreflightErrorMetadata {
            code: "preflight_capability_failed",
            detail: "capability snapshot is missing or invalid",
        },
        JobPreflightError::Workspace(error) => workspace_error_metadata(error),
        JobPreflightError::Build(error) => build_error_metadata(error),
        JobPreflightError::BackupPath(error) => backup_path_metadata(*error),
        JobPreflightError::QuarantinePath(error) => quarantine_path_metadata(*error),
    }
}

fn preflight_plan_metadata(plan_error: &str) -> PreflightErrorMetadata {
    match plan_error {
        "duplicate_source_stream_id" => PreflightErrorMetadata {
            code: "preflight_plan_duplicate_source_stream_id",
            detail: "source graph contains duplicate stream ids",
        },
        "duplicate_desired_stream_id" => PreflightErrorMetadata {
            code: "preflight_plan_duplicate_desired_stream_id",
            detail: "desired graph contains duplicate stream ids",
        },
        "missing_desired_stream" => PreflightErrorMetadata {
            code: "preflight_plan_missing_desired_stream",
            detail: "desired graph references a missing source stream",
        },
        "unsupported_recode_stream_kind" => PreflightErrorMetadata {
            code: "preflight_plan_unsupported_recode_stream_kind",
            detail: "plan includes unsupported recode stream kind",
        },
        _ => PreflightErrorMetadata {
            code: "preflight_plan_failed",
            detail: "plan verification failed",
        },
    }
}

const fn workspace_error_metadata(error: &WorkspaceError) -> PreflightErrorMetadata {
    match error {
        WorkspaceError::InvalidPolicy => PreflightErrorMetadata {
            code: "preflight_workspace_invalid_policy",
            detail: "workspace policy is invalid",
        },
        WorkspaceError::InsufficientReserve => PreflightErrorMetadata {
            code: "preflight_workspace_insufficient_reserve",
            detail: "free disk is below workspace reserve",
        },
        WorkspaceError::InsufficientCapacity => PreflightErrorMetadata {
            code: "preflight_workspace_insufficient_capacity",
            detail: "free disk cannot satisfy workspace demand above reserve",
        },
        WorkspaceError::ExceedsMaxWorkspace => PreflightErrorMetadata {
            code: "preflight_workspace_exceeds_max",
            detail: "workspace demand exceeds configured max",
        },
    }
}

const fn build_error_metadata(error: &BuildArgsError) -> PreflightErrorMetadata {
    match error {
        BuildArgsError::MissingStreamId => PreflightErrorMetadata {
            code: "preflight_build_missing_stream_id",
            detail: "stream-scoped operation is missing stream id",
        },
        BuildArgsError::DesiredStreamMissing(_) => PreflightErrorMetadata {
            code: "preflight_build_desired_stream_missing",
            detail: "desired graph references a missing source stream",
        },
        BuildArgsError::DesiredStreamKindMismatch(_) => PreflightErrorMetadata {
            code: "preflight_build_desired_stream_kind_mismatch",
            detail: "desired graph stream kind does not match source stream",
        },
        BuildArgsError::DuplicateSourceStreamIds => PreflightErrorMetadata {
            code: "preflight_build_duplicate_source_stream_id",
            detail: "source graph contains duplicate stream ids",
        },
        BuildArgsError::DuplicateDesiredStreamIds => PreflightErrorMetadata {
            code: "preflight_build_duplicate_desired_stream_id",
            detail: "desired graph contains duplicate stream ids",
        },
        BuildArgsError::UnsupportedCodec(_) => PreflightErrorMetadata {
            code: "preflight_build_unsupported_codec",
            detail: "required transcode codec is unavailable",
        },
        BuildArgsError::UnsupportedMuxer(_) => PreflightErrorMetadata {
            code: "preflight_build_unsupported_muxer",
            detail: "required output muxer is unavailable",
        },
        BuildArgsError::UnsupportedChapterMuxer(_) => PreflightErrorMetadata {
            code: "preflight_build_unsupported_chapter_muxer",
            detail: "required output muxer cannot author replacement chapters",
        },
        BuildArgsError::UnsupportedContainerMetadataPolicy(_) => PreflightErrorMetadata {
            code: "preflight_build_unsupported_container_metadata_policy",
            detail: "required container metadata policy is unavailable",
        },
        BuildArgsError::InvalidContainerMetadataValues(_) => PreflightErrorMetadata {
            code: "preflight_build_invalid_container_metadata_values",
            detail: "desired container metadata values do not match the selected policy",
        },
        BuildArgsError::UnsupportedContainerChapterPolicy(_) => PreflightErrorMetadata {
            code: "preflight_build_unsupported_container_chapter_policy",
            detail: "required container chapter policy is unavailable",
        },
        BuildArgsError::UnsupportedContainerAttachmentPolicy(_) => PreflightErrorMetadata {
            code: "preflight_build_unsupported_container_attachment_policy",
            detail: "required container attachment policy is unavailable",
        },
        BuildArgsError::InvalidContainerChapterValues(_) => PreflightErrorMetadata {
            code: "preflight_build_invalid_container_chapter_values",
            detail: "desired container chapter values do not match the selected policy",
        },
        BuildArgsError::UnsupportedMetadataRewrite => PreflightErrorMetadata {
            code: "preflight_build_unsupported_metadata_rewrite",
            detail: "metadata rewrite requires a verified desired metadata contract",
        },
        BuildArgsError::ContextlessStreamRewrite => PreflightErrorMetadata {
            code: "preflight_build_contextless_stream_rewrite",
            detail: "stream rewrite requires complete desired graph context",
        },
        BuildArgsError::UnsupportedDesiredStreamKind { .. } => PreflightErrorMetadata {
            code: "preflight_build_unsupported_desired_stream_kind",
            detail: "desired graph stream kind requires an unsupported materialization contract",
        },
        BuildArgsError::EmptyOperations => PreflightErrorMetadata {
            code: "preflight_build_empty_operations",
            detail: "at least one operation is required",
        },
        BuildArgsError::InvalidOperations(_) => PreflightErrorMetadata {
            code: "preflight_build_invalid_operations",
            detail: "operation list is invalid",
        },
        BuildArgsError::NoOpCommand => PreflightErrorMetadata {
            code: "preflight_build_noop_command",
            detail: "no-op operation does not require command construction",
        },
        BuildArgsError::SubtitleArtifactContextRequired => PreflightErrorMetadata {
            code: "preflight_build_subtitle_artifact_context_required",
            detail: "subtitle artifact operation requires complete desired-target context",
        },
        BuildArgsError::SidecarCompanionMismatch => PreflightErrorMetadata {
            code: "preflight_build_sidecar_companion_mismatch",
            detail: "paired sidecar companion paths are inconsistent",
        },
    }
}

const fn backup_path_metadata(error: BackupPathError) -> PreflightErrorMetadata {
    match error {
        BackupPathError::SourceFileNameMissing => PreflightErrorMetadata {
            code: "preflight_backup_path_source_filename_missing",
            detail: "configured backup root requires a source file name",
        },
        BackupPathError::MatchesSourcePath => PreflightErrorMetadata {
            code: "preflight_backup_path_matches_source",
            detail: "backup path must not match source path",
        },
        BackupPathError::MatchesOutputPath => PreflightErrorMetadata {
            code: "preflight_backup_path_matches_output",
            detail: "backup path must not match output path",
        },
    }
}

const fn quarantine_path_metadata(error: QuarantinePathError) -> PreflightErrorMetadata {
    match error {
        QuarantinePathError::SourceFileNameMissing => PreflightErrorMetadata {
            code: "preflight_quarantine_path_source_filename_missing",
            detail: "configured quarantine root requires a source file name",
        },
        QuarantinePathError::MatchesSourcePath => PreflightErrorMetadata {
            code: "preflight_quarantine_path_matches_source",
            detail: "quarantine path must not match source path",
        },
        QuarantinePathError::MatchesOutputPath => PreflightErrorMetadata {
            code: "preflight_quarantine_path_matches_output",
            detail: "quarantine path must not match output path",
        },
        QuarantinePathError::MatchesBackupPath => PreflightErrorMetadata {
            code: "preflight_quarantine_path_matches_backup",
            detail: "quarantine path must not match backup path",
        },
    }
}

/// Deterministic stage label for preflight failures.
#[must_use]
pub const fn preflight_failed_stage(error: &JobPreflightError) -> &'static str {
    match error {
        JobPreflightError::Inspect(_) | JobPreflightError::Plan(_) => "inspect_plan",
        JobPreflightError::Capability(_) => "capability_ready",
        JobPreflightError::Workspace(_) => "workspace_capacity",
        JobPreflightError::Build(_)
        | JobPreflightError::BackupPath(_)
        | JobPreflightError::QuarantinePath(_) => "build_steps",
    }
}

const PREFLIGHT_STAGE_ORDER: [&str; 5] = [
    "inspect_plan",
    "capability_ready",
    "workspace_capacity",
    "build_steps",
    "summarize",
];

/// Build deterministic stage records for a successful preflight run.
#[must_use]
pub fn preflight_success_timeline() -> Vec<PreflightStageRecord> {
    PREFLIGHT_STAGE_ORDER
        .iter()
        .map(|stage| PreflightStageRecord {
            stage,
            ok: true,
            code: None,
        })
        .collect()
}

/// Build a deterministic stage timeline for a failed preflight result.
#[must_use]
pub fn preflight_timeline_for_error(error: &JobPreflightError) -> Vec<PreflightStageRecord> {
    let failed_stage = preflight_failed_stage(error);
    let failed_code = preflight_error_code(error);
    let mut timeline = Vec::new();
    for stage in PREFLIGHT_STAGE_ORDER {
        if stage == failed_stage {
            timeline.push(PreflightStageRecord {
                stage,
                ok: false,
                code: Some(failed_code),
            });
            break;
        }
        timeline.push(PreflightStageRecord {
            stage,
            ok: true,
            code: None,
        });
    }
    timeline
}

/// Build deterministic failure report details from a preflight error.
#[must_use]
pub fn preflight_failure_report(error: &JobPreflightError) -> JobPreflightFailureReport {
    JobPreflightFailureReport {
        failed_stage: preflight_failed_stage(error),
        error_code: preflight_error_code(error),
        error_detail: preflight_error_detail(error),
        timeline: preflight_timeline_for_error(error),
    }
}

/// Project preflight evaluation details into compact deterministic audit facts.
#[must_use]
pub fn preflight_compact_audit_facts(evaluation: &JobPreflightEvaluation) -> Vec<CompactAuditFact> {
    let mut facts = Vec::new();
    let mut audit_index = 0_i32;

    for record in evaluation.timeline() {
        push_compact_audit_fact(
            &mut facts,
            &mut audit_index,
            "preflight_timeline",
            format!(
                "stage={} ok={} code={}",
                record.stage,
                record.ok,
                preflight_stage_code_text(record.code)
            ),
        );
    }

    match evaluation {
        JobPreflightEvaluation::Ready(report) => {
            push_compact_audit_fact(
                &mut facts,
                &mut audit_index,
                "preflight_plan",
                format!(
                    "operations={} remux={} metadata_rewrite={} disposition_rewrite={} label_rewrite={} stream_reorder={} audio_transcode={} video_transcode={} estimated_workspace_bytes={}",
                    report.summary.total_operations,
                    report.summary.remux_operations,
                    report.summary.metadata_rewrite_operations,
                    report.summary.disposition_rewrite_operations,
                    report.summary.label_rewrite_operations,
                    report.summary.stream_reorder_operations,
                    report.summary.audio_transcode_operations,
                    report.summary.video_transcode_operations,
                    report.planned.estimated_workspace_bytes
                ),
            );
            push_compact_audit_fact(
                &mut facts,
                &mut audit_index,
                "workspace_capacity",
                format!(
                    "accepted={} reason={} available_after_reserve_bytes={} required_workspace_bytes={}",
                    report.capacity_report.accepted,
                    workspace_rejection_reason_code(report.capacity_report.reason),
                    report.capacity_report.available_after_reserve_bytes,
                    report.capacity_report.required_workspace_bytes
                ),
            );
        }
        JobPreflightEvaluation::Failed(report) => {
            push_compact_audit_fact(
                &mut facts,
                &mut audit_index,
                "preflight_failure",
                format!(
                    "stage={} code={} detail={}",
                    report.failed_stage, report.error_code, report.error_detail
                ),
            );
        }
    }

    facts
}

fn push_compact_audit_fact(
    facts: &mut Vec<CompactAuditFact>,
    audit_index: &mut i32,
    fact_kind: &'static str,
    fact_text: String,
) {
    facts.push(CompactAuditFact {
        audit_index: *audit_index,
        fact_kind,
        fact_text,
    });
    *audit_index += 1;
}

const fn preflight_stage_code_text(code: Option<&'static str>) -> &'static str {
    match code {
        Some(value) => value,
        None => "none",
    }
}

const fn workspace_rejection_reason_code(reason: Option<WorkspaceRejectionReason>) -> &'static str {
    match reason {
        None => "none",
        Some(WorkspaceRejectionReason::InvalidPolicy) => "invalid_policy",
        Some(WorkspaceRejectionReason::InsufficientReserve) => "insufficient_reserve",
        Some(WorkspaceRejectionReason::ExceedsMaxWorkspace) => "exceeds_max_workspace",
        Some(WorkspaceRejectionReason::InsufficientCapacity) => "insufficient_capacity",
    }
}

/// Build a deterministic plan and estimate workspace usage.
///
/// # Errors
///
/// Returns an error when generated operations fail semantic verification.
pub fn plan_job(request: &JobPreflightRequest) -> Result<PlannedJob, &'static str> {
    plan_job_from_source_graph(&request.desired, request.source_file_bytes, &request.source)
}

/// Inspect source media, then build deterministic plan output.
///
/// # Errors
///
/// Returns [`JobPreflightError::Inspect`] when source inspection fails and
/// [`JobPreflightError::Plan`] when generated operations fail semantic verification.
pub fn plan_job_from_inspect(
    inspector: &dyn InspectAdapter,
    source_path: &str,
    desired: &DesiredGraph,
    source_file_bytes: u64,
) -> Result<PlannedJob, JobPreflightError> {
    let inspection = inspector.inspect(Path::new(source_path))?;
    let mut planned = plan_job_from_source_graph(desired, source_file_bytes, &inspection.graph)
        .map_err(JobPreflightError::Plan)?;
    planned.source_duration_millis = inspection.container.duration_millis;
    Ok(planned)
}

/// Build a deterministic plan from already-inspected source graph.
///
/// # Errors
///
/// Returns an error when generated operations fail semantic verification.
pub fn plan_job_from_source_graph(
    desired: &DesiredGraph,
    source_file_bytes: u64,
    source: &MediaGraph,
) -> Result<PlannedJob, &'static str> {
    plan_job_from_source_graph_with_artifacts(
        desired,
        source_file_bytes,
        source,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

/// Build a deterministic plan from a complete desired-target compilation.
///
/// # Errors
///
/// Returns an error when external stream bindings or generated operations are inconsistent.
pub fn plan_job_from_compiled_target(
    compiled: &CompiledDesiredTarget,
    source_file_bytes: u64,
    source: &MediaGraph,
) -> Result<PlannedJob, &'static str> {
    plan_job_from_source_graph_with_artifacts(
        &compiled.graph,
        source_file_bytes,
        source,
        compiled.sidecar_embeddings.clone(),
        compiled.sidecar_outputs.clone(),
        compiled.sidecar_removals.clone(),
    )
}

/// Build a deterministic plan from the production planning pipeline outcome.
///
/// # Errors
///
/// Returns an error when the outcome does not match the compiled target or its operations are
/// inconsistent with the source and desired graphs.
pub fn plan_job_from_planning_outcome(
    compiled: &CompiledDesiredTarget,
    outcome: &PlanningOutcome,
    source_file_bytes: u64,
    source: &MediaGraph,
) -> Result<PlannedJob, &'static str> {
    if outcome.desired_graph != compiled.graph {
        return Err("planning_outcome_desired_graph_mismatch");
    }
    plan_job_from_selection_with_artifacts(
        &outcome.selection,
        source_file_bytes,
        source,
        &compiled.graph,
        compiled.sidecar_embeddings.clone(),
        compiled.sidecar_outputs.clone(),
        compiled.sidecar_removals.clone(),
    )
}

fn plan_job_from_source_graph_with_artifacts(
    desired: &DesiredGraph,
    source_file_bytes: u64,
    source: &MediaGraph,
    sidecar_embeddings: Vec<SidecarEmbedding>,
    sidecar_outputs: Vec<DesiredSidecarOutput>,
    sidecar_removals: Vec<String>,
) -> Result<PlannedJob, &'static str> {
    verify_unique_stream_ids(&source.streams).map_err(|_| "duplicate_source_stream_id")?;
    verify_unique_stream_ids(&desired.streams).map_err(|_| "duplicate_desired_stream_id")?;
    validate_sidecar_bindings(source, desired, &sidecar_embeddings)?;
    let diff = diff_graphs(source, desired);
    reject_unbound_desired_streams(&diff, &sidecar_embeddings)?;
    reject_unsupported_recoded_stream_kinds(&diff)?;
    let selection = generate_plan(&diff).map_err(|error| plan_generation_error_code(&error))?;
    plan_job_from_selection_with_artifacts(
        &selection,
        source_file_bytes,
        source,
        desired,
        sidecar_embeddings,
        sidecar_outputs,
        sidecar_removals,
    )
}

fn plan_job_from_selection_with_artifacts(
    selection: &PlanSelection,
    source_file_bytes: u64,
    source: &MediaGraph,
    desired: &DesiredGraph,
    sidecar_embeddings: Vec<SidecarEmbedding>,
    sidecar_outputs: Vec<DesiredSidecarOutput>,
    sidecar_removals: Vec<String>,
) -> Result<PlannedJob, &'static str> {
    verify_unique_stream_ids(&source.streams).map_err(|_| "duplicate_source_stream_id")?;
    verify_unique_stream_ids(&desired.streams).map_err(|_| "duplicate_desired_stream_id")?;
    validate_sidecar_bindings(source, desired, &sidecar_embeddings)?;
    let diff = diff_graphs(source, desired);
    reject_unbound_desired_streams(&diff, &sidecar_embeddings)?;
    reject_unsupported_recoded_stream_kinds(&diff)?;
    let compliance = score_diff(&diff);
    let mut operations = selection.selected.operations.clone();
    append_sidecar_operations(
        &mut operations,
        &sidecar_embeddings,
        &sidecar_outputs,
        &sidecar_removals,
    );
    verify_plan_against_graphs(source, desired, &operations)?;

    Ok(PlannedJob {
        source: Box::new(source.clone()),
        desired: Box::new(desired.clone()),
        sidecar_embeddings,
        sidecar_outputs,
        sidecar_removals,
        compliance,
        source_duration_millis: None,
        estimated_workspace_bytes: estimate_workspace_bytes(source_file_bytes, &operations),
        operations,
    })
}

const fn plan_generation_error_code(error: &PlanGenerationError) -> &'static str {
    match error {
        PlanGenerationError::MissingDesiredStream => "missing_desired_stream",
        PlanGenerationError::NoValidCandidate { .. } => "no_valid_candidate",
    }
}

fn reject_unbound_desired_streams(
    diff: &revaer_media_core::diff::GraphDiff,
    sidecar_embeddings: &[SidecarEmbedding],
) -> Result<(), &'static str> {
    if diff.missing_desired_streams.iter().all(|stream_id| {
        sidecar_embeddings
            .iter()
            .any(|binding| binding.output_stream.stream_id == *stream_id)
    }) {
        Ok(())
    } else {
        Err("missing_desired_stream")
    }
}

fn validate_sidecar_bindings(
    source: &MediaGraph,
    desired: &DesiredGraph,
    embeddings: &[SidecarEmbedding],
) -> Result<(), &'static str> {
    let mut ids = std::collections::BTreeSet::new();
    for binding in embeddings {
        if !ids.insert(binding.output_stream.stream_id) {
            return Err("duplicate_sidecar_embedding_stream_id");
        }
        if source
            .streams
            .iter()
            .any(|stream| stream.stream_id == binding.output_stream.stream_id)
        {
            return Err("sidecar_embedding_stream_id_overlaps_source");
        }
        if !desired
            .streams
            .iter()
            .any(|stream| stream == &binding.output_stream)
        {
            return Err("sidecar_embedding_missing_from_desired_graph");
        }
    }
    Ok(())
}

fn append_sidecar_operations(
    operations: &mut Vec<PlannedOperation>,
    embeddings: &[SidecarEmbedding],
    outputs: &[DesiredSidecarOutput],
    removals: &[String],
) {
    if !embeddings.is_empty() || !outputs.is_empty() || !removals.is_empty() {
        operations.retain(|operation| operation.kind != OperationKind::NoOp);
    }
    operations.extend(embeddings.iter().map(|binding| PlannedOperation {
        kind: OperationKind::EmbedSubtitle,
        stream_id: None,
        output_stream_id: Some(binding.output_stream.stream_id),
    }));
    operations.extend(outputs.iter().map(|output| PlannedOperation {
        kind: match &output.source {
            SidecarOutputSource::EmbeddedStream { .. } => OperationKind::ExtractSubtitle,
            SidecarOutputSource::ExistingSidecar { .. } => OperationKind::CopySidecarSubtitle,
        },
        stream_id: match &output.source {
            SidecarOutputSource::EmbeddedStream { stream_id } => Some(*stream_id),
            SidecarOutputSource::ExistingSidecar { .. } => None,
        },
        output_stream_id: None,
    }));
    operations.extend(removals.iter().map(|_| PlannedOperation {
        kind: OperationKind::RemoveSidecarSubtitle,
        stream_id: None,
        output_stream_id: None,
    }));
    if operations.is_empty() {
        operations.push(PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
            output_stream_id: None,
        });
    }
}

fn reject_unsupported_recoded_stream_kinds(
    diff: &revaer_media_core::diff::GraphDiff,
) -> Result<(), &'static str> {
    for stream in &diff.recoded_streams {
        match stream.kind {
            revaer_media_core::model::StreamKind::Audio
            | revaer_media_core::model::StreamKind::Video
            | revaer_media_core::model::StreamKind::Subtitle => {}
            revaer_media_core::model::StreamKind::Attachment
            | revaer_media_core::model::StreamKind::Chapter
            | revaer_media_core::model::StreamKind::Data => {
                return Err("unsupported_recode_stream_kind");
            }
        }
    }
    Ok(())
}

/// Validate workspace reserve/capacity before execution.
///
/// # Errors
///
/// Returns [`crate::workspace::WorkspaceError`] when reserve or capacity checks fail.
pub fn ensure_execution_capacity(
    policy: &WorkspacePolicy,
    free_bytes: u64,
    planned: &PlannedJob,
) -> Result<(), crate::workspace::WorkspaceError> {
    policy.ensure_capacity(free_bytes, planned.estimated_workspace_bytes)
}

/// Build deterministic execution steps from planned job output.
///
/// # Errors
///
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_job_execution_steps(
    input_path: &str,
    output_path: &str,
    planned: &PlannedJob,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    build_desired_graph_execution_steps_with_sidecars(
        planned_build_context(
            input_path,
            output_path,
            planned,
            None,
            VideoTranscodePolicy::default(),
        ),
        planned_subtitle_artifacts(planned),
    )
}

/// Build deterministic execution steps from planned job output, validating required capabilities.
///
/// # Errors
///
/// Returns [`BuildArgsError`] when a required codec or muxer capability is missing.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_job_execution_steps_with_capabilities(
    input_path: &str,
    output_path: &str,
    planned: &PlannedJob,
    capabilities: &CapabilitySnapshot,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    build_desired_graph_execution_steps_with_sidecars(
        planned_build_context(
            input_path,
            output_path,
            planned,
            Some(capabilities),
            VideoTranscodePolicy::default(),
        ),
        planned_subtitle_artifacts(planned),
    )
}

/// Build deterministic execution steps with optional backup and final atomic replacement.
///
/// # Errors
///
/// Returns [`BuildArgsError`] when a required codec or muxer capability is missing.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_job_execution_steps_with_replacement(
    source_path: &str,
    output_path: &str,
    planned: &PlannedJob,
    capabilities: &CapabilitySnapshot,
    backup_path: Option<&str>,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    build_job_execution_steps_with_replacement_video_policy(
        source_path,
        output_path,
        planned,
        capabilities,
        backup_path,
        None,
        VideoTranscodePolicy::default(),
    )
}

/// Build deterministic execution steps with optional backup, quarantine, and replacement.
///
/// # Errors
///
/// Returns [`BuildArgsError`] when a required codec or muxer capability is missing.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_job_execution_steps_with_replacement_policy(
    source_path: &str,
    output_path: &str,
    planned: &PlannedJob,
    capabilities: &CapabilitySnapshot,
    backup_path: Option<&str>,
    quarantine_path: Option<&str>,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    build_job_execution_steps_with_replacement_video_policy(
        source_path,
        output_path,
        planned,
        capabilities,
        backup_path,
        quarantine_path,
        VideoTranscodePolicy::default(),
    )
}

/// Build deterministic execution steps with replacement and video transcode policy.
///
/// # Errors
///
/// Returns [`BuildArgsError`] when a required codec or muxer capability is missing.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_job_execution_steps_with_replacement_video_policy(
    source_path: &str,
    output_path: &str,
    planned: &PlannedJob,
    capabilities: &CapabilitySnapshot,
    backup_path: Option<&str>,
    quarantine_path: Option<&str>,
    video_policy: VideoTranscodePolicy,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    if operations_are_noop(&planned.operations) {
        return build_desired_graph_execution_steps_with_sidecars(
            planned_build_context(
                source_path,
                output_path,
                planned,
                Some(capabilities),
                video_policy,
            ),
            planned_subtitle_artifacts(planned),
        );
    }

    let mut steps = Vec::new();
    if let Some(path) = backup_path {
        steps.push(ExecutionStep::BackupSource {
            source_path: source_path.to_string(),
            backup_path: path.to_string(),
        });
    }
    steps.extend(build_desired_graph_execution_steps_with_sidecars(
        planned_build_context(
            source_path,
            output_path,
            planned,
            Some(capabilities),
            video_policy,
        ),
        planned_subtitle_artifacts(planned),
    )?);
    if let Some(path) = quarantine_path {
        steps.push(ExecutionStep::QuarantineFailedOutput {
            output_path: output_path.to_string(),
            quarantine_path: path.to_string(),
        });
    }
    steps.push(ExecutionStep::AtomicReplace {
        source_path: source_path.to_string(),
        output_path: output_path.to_string(),
    });
    Ok(steps)
}

fn planned_build_context<'a>(
    input_path: &'a str,
    output_path: &'a str,
    planned: &'a PlannedJob,
    capabilities: Option<&'a CapabilitySnapshot>,
    policy: VideoTranscodePolicy,
) -> DesiredGraphBuildContext<'a> {
    DesiredGraphBuildContext {
        input_path,
        output_path,
        source: &planned.source,
        desired: &planned.desired,
        operations: &planned.operations,
        capabilities,
        policy,
    }
}

fn planned_subtitle_artifacts(planned: &PlannedJob) -> SubtitleArtifactPlan<'_> {
    SubtitleArtifactPlan {
        embeddings: &planned.sidecar_embeddings,
        outputs: &planned.sidecar_outputs,
        removals: &planned.sidecar_removals,
    }
}

/// Build a deterministic summary of planned operations.
#[must_use]
pub fn summarize_planned_job(planned: &PlannedJob) -> PlannedJobSummary {
    let mut remux_operations = 0_usize;
    let mut metadata_rewrite_operations = 0_usize;
    let mut disposition_rewrite_operations = 0_usize;
    let mut label_rewrite_operations = 0_usize;
    let mut stream_reorder_operations = 0_usize;
    let mut embed_subtitle_operations = 0_usize;
    let mut extract_subtitle_operations = 0_usize;
    let mut copy_sidecar_subtitle_operations = 0_usize;
    let mut remove_sidecar_subtitle_operations = 0_usize;
    let mut subtitle_transcode_operations = 0_usize;
    let mut audio_transcode_operations = 0_usize;
    let mut video_transcode_operations = 0_usize;

    for operation in &planned.operations {
        match operation.kind {
            OperationKind::NoOp => {}
            OperationKind::Remux => remux_operations += 1,
            OperationKind::MetadataRewrite => metadata_rewrite_operations += 1,
            OperationKind::DispositionRewrite => disposition_rewrite_operations += 1,
            OperationKind::LabelRewrite => label_rewrite_operations += 1,
            OperationKind::StreamReorder => stream_reorder_operations += 1,
            OperationKind::EmbedSubtitle => embed_subtitle_operations += 1,
            OperationKind::ExtractSubtitle => extract_subtitle_operations += 1,
            OperationKind::CopySidecarSubtitle => copy_sidecar_subtitle_operations += 1,
            OperationKind::RemoveSidecarSubtitle => remove_sidecar_subtitle_operations += 1,
            OperationKind::SubtitleTranscode => subtitle_transcode_operations += 1,
            OperationKind::AudioTranscode => audio_transcode_operations += 1,
            OperationKind::VideoTranscode => video_transcode_operations += 1,
        }
    }

    PlannedJobSummary {
        total_operations: planned.operations.len(),
        remux_operations,
        metadata_rewrite_operations,
        disposition_rewrite_operations,
        label_rewrite_operations,
        stream_reorder_operations,
        embed_subtitle_operations,
        extract_subtitle_operations,
        copy_sidecar_subtitle_operations,
        remove_sidecar_subtitle_operations,
        subtitle_transcode_operations,
        audio_transcode_operations,
        video_transcode_operations,
        explanations: explain_plan(&planned.operations),
    }
}

/// Build a deterministic end-to-end preflight report.
///
/// # Errors
///
/// Returns [`JobPreflightError`] when inspection, plan verification, capability checks,
/// workspace checks, or step construction fails.
pub fn build_preflight_report(
    inspector: &dyn InspectAdapter,
    input: PreflightBuildInput<'_>,
) -> Result<JobPreflightReport, JobPreflightError> {
    let planned = plan_job_from_inspect(
        inspector,
        input.source_path,
        input.desired,
        input.source_file_bytes,
    )?;
    build_preflight_report_for_planned(planned, input)
}

/// Build preflight from one complete target compilation and its source inspection.
///
/// # Errors
///
/// Returns [`JobPreflightError`] when planning, capability, workspace, or execution-step
/// validation fails.
pub fn build_preflight_report_from_compiled_target(
    source: &crate::inspect::MediaInspection,
    compiled: &CompiledDesiredTarget,
    input: PreflightBuildInput<'_>,
) -> Result<JobPreflightReport, JobPreflightError> {
    if input.desired != &compiled.graph {
        return Err(JobPreflightError::Plan("compiled_desired_graph_mismatch"));
    }
    let mut planned =
        plan_job_from_compiled_target(compiled, input.source_file_bytes, &source.graph)
            .map_err(JobPreflightError::Plan)?;
    planned.source_duration_millis = source.container.duration_millis;
    build_preflight_report_for_planned(planned, input)
}

/// Build preflight from the production planning pipeline outcome and complete target compilation.
///
/// # Errors
///
/// Returns [`JobPreflightError`] when the outcome, planning, workspace, capabilities, or execution
/// steps are invalid.
pub fn build_preflight_report_from_planning_outcome(
    source: &MediaGraph,
    compiled: &CompiledDesiredTarget,
    outcome: &PlanningOutcome,
    input: PreflightBuildInput<'_>,
) -> Result<JobPreflightReport, JobPreflightError> {
    if input.desired != &compiled.graph {
        return Err(JobPreflightError::Plan("compiled_desired_graph_mismatch"));
    }
    let planned =
        plan_job_from_planning_outcome(compiled, outcome, input.source_file_bytes, source)
            .map_err(JobPreflightError::Plan)?;
    build_preflight_report_for_planned(planned, input)
}

/// Evaluate preflight from the production planning pipeline outcome.
#[must_use]
pub fn evaluate_preflight_from_planning_outcome(
    source: &MediaGraph,
    compiled: &CompiledDesiredTarget,
    outcome: &PlanningOutcome,
    input: PreflightBuildInput<'_>,
) -> JobPreflightEvaluation {
    match build_preflight_report_from_planning_outcome(source, compiled, outcome, input) {
        Ok(report) => JobPreflightEvaluation::Ready(Box::new(report)),
        Err(error) => JobPreflightEvaluation::Failed(preflight_failure_report(&error)),
    }
}

fn build_preflight_report_for_planned(
    mut planned: PlannedJob,
    input: PreflightBuildInput<'_>,
) -> Result<JobPreflightReport, JobPreflightError> {
    require_valid_capability_snapshot(Some(input.capabilities))
        .map_err(JobPreflightError::Capability)?;
    planned.estimated_workspace_bytes =
        estimate_live_workspace_demand(&planned, input.source_file_bytes, &input.video_policy);
    let capacity_report = input
        .workspace_policy
        .evaluate_capacity(input.free_bytes, planned.estimated_workspace_bytes);
    ensure_execution_capacity(input.workspace_policy, input.free_bytes, &planned)?;
    let steps = build_job_execution_steps_with_replacement_video_policy(
        input.source_path,
        input.output_path,
        &planned,
        input.capabilities,
        input.backup_path,
        input.quarantine_path,
        input.video_policy,
    )?;
    let summary = summarize_planned_job(&planned);
    let step_audits = compile_execution_step_audits(&planned.operations, &steps);
    let timeline = preflight_success_timeline();
    Ok(JobPreflightReport {
        planned,
        summary,
        steps,
        step_audits,
        timeline,
        capacity_report,
    })
}

/// Evaluate preflight from a complete target compilation and source inspection.
#[must_use]
pub fn evaluate_preflight_from_compiled_target(
    source: &crate::inspect::MediaInspection,
    compiled: &CompiledDesiredTarget,
    input: PreflightBuildInput<'_>,
) -> JobPreflightEvaluation {
    match build_preflight_report_from_compiled_target(source, compiled, input) {
        Ok(report) => JobPreflightEvaluation::Ready(Box::new(report)),
        Err(error) => JobPreflightEvaluation::Failed(preflight_failure_report(&error)),
    }
}

/// Evaluate preflight and always return a structured outcome payload.
#[must_use]
pub fn evaluate_preflight(
    inspector: &dyn InspectAdapter,
    input: PreflightBuildInput<'_>,
) -> JobPreflightEvaluation {
    match build_preflight_report(inspector, input) {
        Ok(report) => JobPreflightEvaluation::Ready(Box::new(report)),
        Err(error) => JobPreflightEvaluation::Failed(preflight_failure_report(&error)),
    }
}

/// Build owned preflight input from template/policy and return a preflight report result.
///
/// # Errors
///
/// Returns [`JobPreflightError::BackupPath`] when backup path resolution fails for configured
/// backup policy.
/// Returns other [`JobPreflightError`] variants from preflight report construction.
pub fn build_preflight_report_from_template(
    inspector: &dyn InspectAdapter,
    template: PreflightBuildTemplate<'_>,
    policy_input: PreflightPolicyInput<'_>,
) -> Result<JobPreflightReport, JobPreflightError> {
    let backup_root_configured = policy_input
        .backup_root
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let resolved_backup_path = resolve_backup_path(policy_input.backup_root, template.source_path);
    let quarantine_root_configured = policy_input
        .quarantine_root
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let resolved_quarantine_path =
        resolve_quarantine_path(policy_input.quarantine_root, template.source_path);

    if backup_root_configured {
        let Some(backup_path) = resolved_backup_path.as_deref() else {
            return Err(JobPreflightError::BackupPath(
                BackupPathError::SourceFileNameMissing,
            ));
        };

        if backup_path == template.source_path {
            return Err(JobPreflightError::BackupPath(
                BackupPathError::MatchesSourcePath,
            ));
        }
        if backup_path == template.output_path {
            return Err(JobPreflightError::BackupPath(
                BackupPathError::MatchesOutputPath,
            ));
        }
    }
    if quarantine_root_configured {
        let Some(quarantine_path) = resolved_quarantine_path.as_deref() else {
            return Err(JobPreflightError::QuarantinePath(
                QuarantinePathError::SourceFileNameMissing,
            ));
        };

        if quarantine_path == template.source_path {
            return Err(JobPreflightError::QuarantinePath(
                QuarantinePathError::MatchesSourcePath,
            ));
        }
        if quarantine_path == template.output_path {
            return Err(JobPreflightError::QuarantinePath(
                QuarantinePathError::MatchesOutputPath,
            ));
        }
        if Some(quarantine_path) == resolved_backup_path.as_deref() {
            return Err(JobPreflightError::QuarantinePath(
                QuarantinePathError::MatchesBackupPath,
            ));
        }
    }
    let input = build_preflight_input(template, policy_input);
    build_preflight_report(inspector, input.as_borrowed())
}

/// Build owned preflight input from template/policy and evaluate preflight.
#[must_use]
pub fn evaluate_preflight_from_template(
    inspector: &dyn InspectAdapter,
    template: PreflightBuildTemplate<'_>,
    policy_input: PreflightPolicyInput<'_>,
) -> JobPreflightEvaluation {
    match build_preflight_report_from_template(inspector, template, policy_input) {
        Ok(report) => JobPreflightEvaluation::Ready(Box::new(report)),
        Err(error) => JobPreflightEvaluation::Failed(preflight_failure_report(&error)),
    }
}

/// Ensure media execution can proceed with a valid capability snapshot.
///
/// # Errors
///
/// Returns an error when no capability snapshot is available or when snapshot data is invalid.
pub fn require_valid_capability_snapshot(
    snapshot: Option<&CapabilitySnapshot>,
) -> Result<(), &'static str> {
    let Some(snapshot) = snapshot else {
        return Err("media capability snapshot is missing");
    };
    if !snapshot.is_valid() {
        return Err("media capability snapshot is invalid");
    }
    Ok(())
}

fn estimate_workspace_bytes(source_file_bytes: u64, operations: &[PlannedOperation]) -> u64 {
    if operations_are_noop(operations) {
        return 0;
    }

    // Conservative fixed multipliers for current foundation implementation.
    let mut max_multiplier_num: u64 = 1;
    let mut max_multiplier_den: u64 = 1;

    for op in operations {
        let candidate = match op.kind {
            revaer_media_core::plan::OperationKind::NoOp => (0_u64, 1_u64),
            revaer_media_core::plan::OperationKind::Remux
            | revaer_media_core::plan::OperationKind::MetadataRewrite
            | revaer_media_core::plan::OperationKind::DispositionRewrite
            | revaer_media_core::plan::OperationKind::LabelRewrite
            | revaer_media_core::plan::OperationKind::StreamReorder
            | revaer_media_core::plan::OperationKind::EmbedSubtitle
            | revaer_media_core::plan::OperationKind::ExtractSubtitle
            | revaer_media_core::plan::OperationKind::CopySidecarSubtitle
            | revaer_media_core::plan::OperationKind::RemoveSidecarSubtitle => (6_u64, 5_u64), // 1.2x
            revaer_media_core::plan::OperationKind::AudioTranscode
            | revaer_media_core::plan::OperationKind::SubtitleTranscode => (3_u64, 2_u64), // 1.5x
            revaer_media_core::plan::OperationKind::VideoTranscode => (5_u64, 2_u64), // 2.5x
        };
        if candidate.0.saturating_mul(max_multiplier_den)
            > max_multiplier_num.saturating_mul(candidate.1)
        {
            max_multiplier_num = candidate.0;
            max_multiplier_den = candidate.1;
        }
    }

    // Use saturating math for deterministic overflow-safe behavior.
    source_file_bytes.saturating_mul(max_multiplier_num) / max_multiplier_den
}

fn estimate_live_workspace_demand(
    planned: &PlannedJob,
    source_file_bytes: u64,
    video_policy: &VideoTranscodePolicy,
) -> u64 {
    let fallback = estimate_workspace_bytes(source_file_bytes, &planned.operations);
    let constrained_bitrate_bps = video_policy
        .stream_constraints
        .iter()
        .filter_map(|constraint| constraint.max_bitrate_bps)
        .map(|bitrate| u64::from(bitrate.get()))
        .chain(
            video_policy
                .audio_stream_constraints
                .iter()
                .filter_map(|constraint| constraint.bitrate_bps)
                .map(u64::from),
        )
        .fold(0_u64, u64::saturating_add);
    let bitrate_output_bytes = planned.source_duration_millis.map_or(0, |duration_millis| {
        constrained_bitrate_bps
            .saturating_mul(duration_millis)
            .div_ceil(8_000)
    });
    let primary_output = fallback.max(bitrate_output_bytes);
    let sidecar_and_attachment_count = planned.sidecar_outputs.len().saturating_add(
        planned
            .desired
            .streams
            .iter()
            .filter(|stream| stream.kind == revaer_media_core::model::StreamKind::Attachment)
            .count(),
    );
    let auxiliary_bytes = source_file_bytes.saturating_mul(
        u64::try_from(sidecar_and_attachment_count).map_or(u64::MAX, |value| value),
    ) / 10;
    let container_temporary_and_log_overhead = primary_output / 4;
    primary_output
        .saturating_add(auxiliary_bytes)
        .saturating_add(container_temporary_and_log_overhead)
}

fn operations_are_noop(operations: &[PlannedOperation]) -> bool {
    matches!(
        operations,
        [PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
            output_stream_id: None
        }]
    )
}

#[cfg(test)]
mod tests {
    use super::{
        BackupPathError, BuildArgsError, CompactAuditFact, DesiredSidecarOutput, ExecutionStep,
        JobPreflightError, JobPreflightEvaluation, JobPreflightFailureReport, JobPreflightReport,
        JobPreflightRequest, OwnedPreflightBuildInput, PlannedJob, PreflightBuildInput,
        PreflightBuildTemplate, PreflightPolicyInput, PreflightStageRecord, SidecarOutputSource,
        build_job_execution_steps, build_job_execution_steps_with_capabilities,
        build_job_execution_steps_with_replacement,
        build_job_execution_steps_with_replacement_policy, build_preflight_input,
        build_preflight_report, build_preflight_report_from_template, ensure_execution_capacity,
        estimate_live_workspace_demand, evaluate_preflight, evaluate_preflight_from_template,
        plan_job, plan_job_from_inspect, plan_job_from_source_graph, preflight_compact_audit_facts,
        preflight_error_code, preflight_error_detail, preflight_failed_stage,
        preflight_failure_report, preflight_success_timeline, preflight_timeline_for_error,
        require_valid_capability_snapshot, resolve_backup_path, resolve_quarantine_path,
        summarize_planned_job,
    };
    use crate::capabilities::{CapabilitySnapshot, CodecCapability};
    use crate::execute::{
        HdrColorPolicy, MaxBitrateBps, VideoTranscodeIntent, VideoTranscodePolicy,
    };
    use crate::inspect::{
        ContainerInspection, InspectAdapter, InspectCancellation, InspectError, MediaInspection,
    };
    use crate::workspace::{WorkspaceError, WorkspacePolicy};
    use revaer_media_core::compliance::{Status, report_for_status};
    use revaer_media_core::model::{
        ContainerChapterEntry, DesiredGraph, MediaGraph, MediaStream, StreamKind,
    };
    use revaer_media_core::plan::{OperationKind, PlannedOperation};

    fn capability_snapshot_for_tests(codecs: &[&str], encoders: &[&str]) -> CapabilitySnapshot {
        CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: codecs.iter().map(|codec| (*codec).to_string()).collect(),
            codec_support: codecs
                .iter()
                .map(|codec| CodecCapability {
                    name: (*codec).to_string(),
                    encode_supported: true,
                    decode_supported: true,
                })
                .collect(),
            encoders: encoders
                .iter()
                .map(|encoder| (*encoder).to_string())
                .collect(),
            ..CapabilitySnapshot::default()
        }
    }

    #[test]
    fn plan_job_builds_operations_and_estimate() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };

        let planned_result = plan_job(&JobPreflightRequest {
            desired,
            source_file_bytes: 1_000,
            source,
        });
        assert!(
            planned_result.is_ok(),
            "expected plan to succeed, got: {planned_result:?}"
        );
        let Ok(planned) = planned_result else {
            return;
        };
        assert!(!planned.operations.is_empty());
        assert_eq!(planned.compliance.status, Status::NonCompliant);
        assert!(!planned.compliance.violations.is_empty());
        assert!(planned.estimated_workspace_bytes > 1_000);
    }

    #[test]
    fn strip_chapter_command_reinspection_replans_as_noop() {
        let stream = MediaStream {
            stream_id: 0,
            kind: StreamKind::Video,
            codec: "h264".to_string(),
            channels: None,
            channel_layout: None,
            language: None,
            title: None,
            dispositions: Vec::new(),
        };
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: vec![ContainerChapterEntry {
                start_millis: 0,
                end_millis: 1_000,
                metadata: Vec::new(),
            }],
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream.clone()],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: Some("strip".to_string()),
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            stream_bindings: Vec::new(),
            streams: vec![stream.clone()],
        };
        let initial = plan_job_from_source_graph(&desired, 1_000, &source);
        assert!(initial.is_ok(), "expected initial strip plan: {initial:?}");
        let Ok(initial) = initial else {
            return;
        };
        let steps = build_job_execution_steps("/input/movie.mkv", "/output/movie.mkv", &initial);
        assert!(steps.is_ok(), "expected strip execution steps: {steps:?}");
        let Ok(steps) = steps else {
            return;
        };
        assert!(steps.iter().any(|step| matches!(
            step,
            ExecutionStep::Command { argv, .. }
                if argv.windows(2).any(|pair| pair == ["-map_chapters", "-1"])
        )));

        let reinspected = MediaGraph {
            source_path: "/output/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream],
        };
        let replanned = plan_job_from_source_graph(&desired, 1_000, &reinspected);
        assert!(
            replanned.is_ok(),
            "expected compliant replan: {replanned:?}"
        );
        let Ok(replanned) = replanned else {
            return;
        };
        assert_eq!(
            replanned.operations,
            vec![PlannedOperation {
                kind: OperationKind::NoOp,
                stream_id: None,
                output_stream_id: None,
            }]
        );
    }

    #[test]
    fn preflight_capacity_check_fails_when_demand_exceeds_reserve_budget() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let planned_result = plan_job(&JobPreflightRequest {
            desired,
            source_file_bytes: 10_000,
            source,
        });
        assert!(
            planned_result.is_ok(),
            "expected plan to succeed, got: {planned_result:?}"
        );
        let Ok(planned) = planned_result else {
            return;
        };

        let policy = WorkspacePolicy {
            max_bytes: 100_000,
            reserve_bytes: 5_000,
        };
        assert_eq!(
            ensure_execution_capacity(&policy, 20_000, &planned),
            Err(WorkspaceError::InsufficientCapacity)
        );
    }

    #[test]
    fn build_job_execution_steps_adds_verify_step() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let planned_result = plan_job(&JobPreflightRequest {
            desired,
            source_file_bytes: 2_000,
            source,
        });
        assert!(planned_result.is_ok());
        let Ok(planned) = planned_result else {
            return;
        };

        let steps_result =
            build_job_execution_steps("/input/movie.mkv", "/output/movie.mkv", &planned);
        assert!(steps_result.is_ok());
        let Ok(steps) = steps_result else {
            return;
        };
        assert!(!steps.is_empty());
    }

    #[test]
    fn job_execution_steps_map_only_desired_streams() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 2,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("und".to_string()),
                    title: Some("Silent".to_string()),
                    dispositions: Vec::new(),
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: source.streams[..2].to_vec(),
        };
        let planned_result = plan_job(&JobPreflightRequest {
            desired,
            source_file_bytes: 1_000,
            source,
        });
        assert!(
            planned_result.is_ok(),
            "expected plan to succeed, got: {planned_result:?}"
        );
        let Ok(planned) = planned_result else {
            return;
        };

        let steps_result =
            build_job_execution_steps("/input/movie.mkv", "/output/movie.mkv", &planned);
        assert!(
            steps_result.is_ok(),
            "expected step build to succeed, got: {steps_result:?}"
        );
        let Ok(steps) = steps_result else {
            return;
        };
        let Some(ExecutionStep::Command { argv, .. }) = steps.first() else {
            panic!("expected first step to be an ffmpeg command");
        };

        assert!(argv.windows(2).any(|pair| pair == ["-map", "0:0"]));
        assert!(argv.windows(2).any(|pair| pair == ["-map", "0:1"]));
        assert!(!argv.windows(2).any(|pair| pair == ["-map", "0"]));
        assert!(!argv.windows(2).any(|pair| pair == ["-map", "0:2"]));
    }

    #[test]
    fn plan_job_rejects_unsupported_recode_stream_kind() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 3,
                kind: StreamKind::Attachment,
                codec: "ttf".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: Some("Font".to_string()),
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 3,
                kind: StreamKind::Attachment,
                codec: "otf".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: Some("Font".to_string()),
                dispositions: Vec::new(),
            }],
        };

        let result = plan_job(&JobPreflightRequest {
            source,
            desired,
            source_file_bytes: 1_024,
        });

        assert_eq!(result, Err("unsupported_recode_stream_kind"));
    }

    #[test]
    fn plan_job_rejects_missing_desired_stream() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 7,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: Some(2),
                    channel_layout: Some("stereo".to_string()),
                    language: Some("eng".to_string()),
                    title: Some("Generated".to_string()),
                    dispositions: Vec::new(),
                },
            ],
        };

        let result = plan_job(&JobPreflightRequest {
            source,
            desired,
            source_file_bytes: 1_024,
        });

        assert_eq!(result, Err("missing_desired_stream"));
    }

    #[test]
    fn plan_job_rejects_duplicate_source_stream_ids() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: Some(2),
                    channel_layout: Some("stereo".to_string()),
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };

        let result = plan_job(&JobPreflightRequest {
            source,
            desired,
            source_file_bytes: 1_024,
        });

        assert_eq!(result, Err("duplicate_source_stream_id"));
    }

    #[test]
    fn plan_job_rejects_duplicate_desired_stream_ids() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: Some("Duplicate".to_string()),
                    dispositions: Vec::new(),
                },
            ],
        };

        let result = plan_job(&JobPreflightRequest {
            source,
            desired,
            source_file_bytes: 1_024,
        });

        assert_eq!(result, Err("duplicate_desired_stream_id"));
    }

    #[test]
    fn require_capability_snapshot_rejects_missing_or_invalid_state() {
        assert_eq!(
            require_valid_capability_snapshot(None),
            Err("media capability snapshot is missing")
        );

        let invalid = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: Vec::new(),
            codec_support: Vec::new(),
            encoders: Vec::new(),
            ..CapabilitySnapshot::default()
        };
        assert_eq!(
            require_valid_capability_snapshot(Some(&invalid)),
            Err("media capability snapshot is invalid")
        );
    }

    #[test]
    fn require_capability_snapshot_accepts_valid_snapshot() {
        let valid = capability_snapshot_for_tests(&["h264"], &["libx265"]);
        assert!(require_valid_capability_snapshot(Some(&valid)).is_ok());
    }

    #[test]
    fn build_job_execution_steps_with_capabilities_rejects_unsupported_codec() {
        let planned = planned_job_for_tests(
            vec![PlannedOperation {
                kind: revaer_media_core::plan::OperationKind::VideoTranscode,
                stream_id: Some(0),
                output_stream_id: Some(0),
            }],
            100,
        );
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        assert_eq!(
            build_job_execution_steps_with_capabilities(
                "/input/movie.mkv",
                "/output/movie.mkv",
                &planned,
                &capabilities
            ),
            Err(BuildArgsError::UnsupportedCodec("libx265"))
        );
    }

    #[test]
    fn build_job_execution_steps_with_replacement_includes_backup_and_replace() {
        let planned = planned_job_for_tests(
            vec![PlannedOperation {
                kind: revaer_media_core::plan::OperationKind::Remux,
                stream_id: None,
                output_stream_id: None,
            }],
            100,
        );
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let steps_result = build_job_execution_steps_with_replacement(
            "/input/movie.mkv",
            "/output/movie.mkv",
            &planned,
            &capabilities,
            Some("/backup/movie.mkv"),
        );
        assert!(steps_result.is_ok());
        let Ok(steps) = steps_result else {
            return;
        };
        assert!(matches!(
            steps.first(),
            Some(ExecutionStep::BackupSource { .. })
        ));
        assert!(matches!(
            steps.last(),
            Some(ExecutionStep::AtomicReplace { .. })
        ));
    }

    struct StubInspectAdapter {
        graph: Option<MediaGraph>,
        error: Option<&'static str>,
    }

    impl InspectAdapter for StubInspectAdapter {
        fn inspect_with_cancellation(
            &self,
            _source_path: &std::path::Path,
            _cancellation: &dyn InspectCancellation,
        ) -> Result<MediaInspection, InspectError> {
            if let Some(message) = self.error {
                return Err(InspectError::ProbeFailed(message.to_string()));
            }
            let graph = self
                .graph
                .clone()
                .ok_or_else(|| InspectError::ProbeFailed("missing graph".to_string()))?;
            Ok(MediaInspection {
                container: ContainerInspection {
                    formats: graph.container_formats.clone(),
                    duration_millis: None,
                    start_time_millis: None,
                    size_bytes: None,
                    bit_rate: None,
                    metadata: Vec::new(),
                },
                graph,
                streams: Vec::new(),
                chapters: Vec::new(),
                sidecars: Vec::new(),
            })
        }
    }

    fn single_video_graphs(source_codec: &str, desired_codec: &str) -> (MediaGraph, DesiredGraph) {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: source_codec.to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: desired_codec.to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        (source, desired)
    }

    fn planned_job_for_tests(
        operations: Vec<PlannedOperation>,
        estimated_workspace_bytes: u64,
    ) -> PlannedJob {
        let (source, desired) = single_video_graphs("h264", "h264");
        PlannedJob {
            source: Box::new(source),
            desired: Box::new(desired),
            sidecar_embeddings: Vec::new(),
            sidecar_outputs: Vec::new(),
            sidecar_removals: Vec::new(),
            operations,
            compliance: report_for_status(Status::Compliant),
            source_duration_millis: None,
            estimated_workspace_bytes,
        }
    }

    #[test]
    fn duration_and_aggregate_output_rates_drive_workspace_demand() {
        let mut planned = planned_job_for_tests(
            vec![PlannedOperation {
                kind: OperationKind::VideoTranscode,
                stream_id: Some(0),
                output_stream_id: Some(0),
            }],
            2_500_000,
        );
        planned.source_duration_millis = Some(8 * 60 * 60 * 1_000);
        let policy = VideoTranscodePolicy {
            stream_constraints: vec![
                crate::execute::VideoStreamConstraints {
                    stream_id: 0,
                    profile: None,
                    level: None,
                    max_bitrate_bps: MaxBitrateBps::new(100_000),
                    color_primaries: None,
                    color_transfer: None,
                    color_space: None,
                    hdr_format: None,
                },
                crate::execute::VideoStreamConstraints {
                    stream_id: 1,
                    profile: None,
                    level: None,
                    max_bitrate_bps: MaxBitrateBps::new(20_000_000),
                    color_primaries: None,
                    color_transfer: None,
                    color_space: None,
                    hdr_format: None,
                },
            ],
            audio_stream_constraints: vec![crate::execute::AudioStreamConstraints {
                stream_id: 2,
                channel_count: None,
                channel_layout: None,
                bitrate_bps: Some(320_000),
                sample_rate_hz: None,
                loudness_profile: None,
                dynamic_range: None,
            }],
            ..VideoTranscodePolicy::default()
        };

        let demand = estimate_live_workspace_demand(&planned, 1_000_000, &policy);

        assert!(demand > 70_000_000_000);
    }

    #[test]
    fn sidecars_and_attachments_increase_workspace_demand() {
        let mut planned = planned_job_for_tests(
            vec![PlannedOperation {
                kind: OperationKind::Remux,
                stream_id: None,
                output_stream_id: None,
            }],
            1_200_000,
        );
        let baseline =
            estimate_live_workspace_demand(&planned, 1_000_000, &VideoTranscodePolicy::default());
        planned.desired.streams.push(MediaStream {
            stream_id: 9,
            kind: StreamKind::Attachment,
            codec: "ttf".to_string(),
            channels: None,
            channel_layout: None,
            language: None,
            title: None,
            dispositions: Vec::new(),
        });
        planned.sidecar_outputs.push(DesiredSidecarOutput {
            path: "/workspace/movie.srt".to_string(),
            companion_path: None,
            destination_path: "/library/movie.srt".to_string(),
            destination_companion_path: None,
            source: SidecarOutputSource::EmbeddedStream { stream_id: 1 },
            codec: "subrip".to_string(),
        });

        let expanded =
            estimate_live_workspace_demand(&planned, 1_000_000, &VideoTranscodePolicy::default());

        assert_eq!(expanded, baseline + 200_000);
    }

    #[test]
    fn plan_job_from_inspect_uses_inspected_graph() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: vec![MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            }),
            error: None,
        };

        let planned = plan_job_from_inspect(&inspector, "/input/movie.mkv", &desired, 5_000);
        assert!(planned.is_ok());
        let Ok(planned) = planned else {
            return;
        };
        assert!(!planned.operations.is_empty());
    }

    #[test]
    fn plan_job_from_inspect_propagates_inspect_error() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: None,
            error: Some("probe failed"),
        };

        let err = plan_job_from_inspect(&inspector, "/input/movie.mkv", &desired, 5_000).err();
        assert!(matches!(err, Some(JobPreflightError::Inspect(_))));
    }

    #[test]
    fn summarize_planned_job_counts_kinds_and_includes_explanations() {
        let planned = planned_job_for_tests(
            vec![
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::Remux,
                    stream_id: None,
                    output_stream_id: None,
                },
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::MetadataRewrite,
                    stream_id: None,
                    output_stream_id: None,
                },
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::DispositionRewrite,
                    stream_id: Some(2),
                    output_stream_id: Some(2),
                },
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::LabelRewrite,
                    stream_id: Some(2),
                    output_stream_id: Some(2),
                },
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::StreamReorder,
                    stream_id: None,
                    output_stream_id: None,
                },
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::AudioTranscode,
                    stream_id: Some(1),
                    output_stream_id: Some(1),
                },
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::VideoTranscode,
                    stream_id: Some(0),
                    output_stream_id: Some(0),
                },
            ],
            123,
        );

        let summary = summarize_planned_job(&planned);
        assert_eq!(summary.total_operations, 7);
        assert_eq!(summary.remux_operations, 1);
        assert_eq!(summary.metadata_rewrite_operations, 1);
        assert_eq!(summary.disposition_rewrite_operations, 1);
        assert_eq!(summary.label_rewrite_operations, 1);
        assert_eq!(summary.stream_reorder_operations, 1);
        assert_eq!(summary.audio_transcode_operations, 1);
        assert_eq!(summary.video_transcode_operations, 1);
        assert_eq!(summary.explanations.len(), 7);
    }

    #[test]
    fn build_preflight_report_returns_summary_and_steps() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: vec![MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            }),
            error: None,
        };
        let capabilities = capability_snapshot_for_tests(&["libx265"], &["libx265"]);
        let policy = WorkspacePolicy {
            max_bytes: 100_000,
            reserve_bytes: 1_000,
        };

        let report = build_preflight_report(
            &inspector,
            PreflightBuildInput {
                source_path: "/input/movie.mkv",
                output_path: "/output/movie.mkv",
                backup_path: Some("/backup/movie.mkv"),
                quarantine_path: None,
                desired: &desired,
                source_file_bytes: 4_000,
                capabilities: &capabilities,
                workspace_policy: &policy,
                free_bytes: 20_000,
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        assert!(report.is_ok());
        let Ok(report) = report else {
            return;
        };
        assert!(!report.planned.operations.is_empty());
        assert!(!report.summary.explanations.is_empty());
        assert!(!report.steps.is_empty());
        assert!(matches!(
            report.steps.first(),
            Some(ExecutionStep::BackupSource { .. })
        ));
        assert!(matches!(
            report.steps.last(),
            Some(ExecutionStep::AtomicReplace { .. })
        ));
        assert_eq!(report.timeline.len(), 5);
        assert_eq!(report.timeline[0].stage, "inspect_plan");
        assert!(report.timeline.iter().all(|item| item.ok));
        assert!(report.capacity_report.accepted);
        assert_eq!(report.capacity_report.reason, None);
    }

    #[test]
    fn build_preflight_report_for_compliant_graph_uses_noop_without_replacement_steps() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: vec![MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            }),
            error: None,
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx265", "aac"]);
        let policy = WorkspacePolicy {
            max_bytes: 100_000,
            reserve_bytes: 1_000,
        };

        let report = build_preflight_report(
            &inspector,
            PreflightBuildInput {
                source_path: "/input/movie.mkv",
                output_path: "/output/movie.mkv",
                backup_path: Some("/backup/movie.mkv"),
                quarantine_path: Some("/quarantine/movie.mkv"),
                desired: &desired,
                source_file_bytes: 4_000,
                capabilities: &capabilities,
                workspace_policy: &policy,
                free_bytes: 20_000,
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        assert!(report.is_ok());
        let Ok(report) = report else {
            return;
        };

        assert_eq!(report.planned.operations.len(), 1);
        assert_eq!(report.planned.operations[0].kind, OperationKind::NoOp);
        assert_eq!(report.summary.total_operations, 1);
        assert!(report.steps.iter().all(|step| !matches!(
            step,
            ExecutionStep::Command { .. }
                | ExecutionStep::BackupSource { .. }
                | ExecutionStep::QuarantineFailedOutput { .. }
                | ExecutionStep::AtomicReplace { .. }
        )));
    }

    #[test]
    fn build_preflight_report_rejects_invalid_capabilities() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: vec![MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            }),
            error: None,
        };
        let invalid_capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: Vec::new(),
            codec_support: Vec::new(),
            encoders: Vec::new(),
            ..CapabilitySnapshot::default()
        };
        let policy = WorkspacePolicy {
            max_bytes: 100_000,
            reserve_bytes: 1_000,
        };

        let err = build_preflight_report(
            &inspector,
            PreflightBuildInput {
                source_path: "/input/movie.mkv",
                output_path: "/output/movie.mkv",
                backup_path: None,
                quarantine_path: None,
                desired: &desired,
                source_file_bytes: 4_000,
                capabilities: &invalid_capabilities,
                workspace_policy: &policy,
                free_bytes: 20_000,
                video_policy: VideoTranscodePolicy::default(),
            },
        )
        .err();
        assert!(matches!(
            err,
            Some(JobPreflightError::Capability(
                "media capability snapshot is invalid"
            ))
        ));
    }

    #[test]
    fn preflight_stage_record_shape_is_stable() {
        let record = PreflightStageRecord {
            stage: "build_steps",
            ok: true,
            code: None,
        };
        assert_eq!(record.stage, "build_steps");
        assert!(record.ok);
        assert_eq!(record.code, None);
    }

    #[test]
    fn preflight_error_classification_is_deterministic() {
        let err = JobPreflightError::Workspace(WorkspaceError::ExceedsMaxWorkspace);
        assert_eq!(
            preflight_error_code(&err),
            "preflight_workspace_exceeds_max"
        );
        assert_eq!(preflight_failed_stage(&err), "workspace_capacity");

        let err = JobPreflightError::Build(BuildArgsError::UnsupportedCodec("libx265"));
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_unsupported_codec"
        );
        assert_eq!(preflight_failed_stage(&err), "build_steps");

        let err = JobPreflightError::Plan("unsupported_recode_stream_kind");
        assert_eq!(
            preflight_error_code(&err),
            "preflight_plan_unsupported_recode_stream_kind"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "plan includes unsupported recode stream kind"
        );
        assert_eq!(preflight_failed_stage(&err), "inspect_plan");

        let err = JobPreflightError::Plan("missing_desired_stream");
        assert_eq!(
            preflight_error_code(&err),
            "preflight_plan_missing_desired_stream"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "desired graph references a missing source stream"
        );
        assert_eq!(preflight_failed_stage(&err), "inspect_plan");

        let err = JobPreflightError::Plan("duplicate_source_stream_id");
        assert_eq!(
            preflight_error_code(&err),
            "preflight_plan_duplicate_source_stream_id"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "source graph contains duplicate stream ids"
        );
        assert_eq!(preflight_failed_stage(&err), "inspect_plan");

        let err = JobPreflightError::Plan("duplicate_desired_stream_id");
        assert_eq!(
            preflight_error_code(&err),
            "preflight_plan_duplicate_desired_stream_id"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "desired graph contains duplicate stream ids"
        );
        assert_eq!(preflight_failed_stage(&err), "inspect_plan");

        let err = JobPreflightError::Build(BuildArgsError::DuplicateSourceStreamIds);
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_duplicate_source_stream_id"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "source graph contains duplicate stream ids"
        );
        assert_eq!(preflight_failed_stage(&err), "build_steps");

        let err = JobPreflightError::Build(BuildArgsError::DuplicateDesiredStreamIds);
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_duplicate_desired_stream_id"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "desired graph contains duplicate stream ids"
        );
        assert_eq!(preflight_failed_stage(&err), "build_steps");

        let err = JobPreflightError::Build(BuildArgsError::DesiredStreamKindMismatch(0));
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_desired_stream_kind_mismatch"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "desired graph stream kind does not match source stream"
        );
        assert_eq!(preflight_failed_stage(&err), "build_steps");

        let err = JobPreflightError::Build(BuildArgsError::InvalidOperations(
            "no-op operation must not be combined with mutating operations",
        ));
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_invalid_operations"
        );
        assert_eq!(preflight_error_detail(&err), "operation list is invalid");
        assert_eq!(preflight_failed_stage(&err), "build_steps");
    }

    #[test]
    fn unsupported_metadata_rewrite_preflight_classification_is_stable() {
        let err = JobPreflightError::Build(BuildArgsError::UnsupportedMetadataRewrite);
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_unsupported_metadata_rewrite"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "metadata rewrite requires a verified desired metadata contract"
        );
        assert_eq!(preflight_failed_stage(&err), "build_steps");
    }

    #[test]
    fn contextless_stream_rewrite_preflight_classification_is_stable() {
        let err = JobPreflightError::Build(BuildArgsError::ContextlessStreamRewrite);
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_contextless_stream_rewrite"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "stream rewrite requires complete desired graph context"
        );
        assert_eq!(preflight_failed_stage(&err), "build_steps");
    }

    #[test]
    fn unsupported_desired_stream_kind_preflight_classification_is_stable() {
        let err =
            JobPreflightError::Build(BuildArgsError::UnsupportedDesiredStreamKind { stream_id: 4 });
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_unsupported_desired_stream_kind"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "desired graph stream kind requires an unsupported materialization contract"
        );
        assert_eq!(preflight_failed_stage(&err), "build_steps");
    }

    #[test]
    fn unsupported_muxer_preflight_classification_is_stable() {
        let err = JobPreflightError::Build(BuildArgsError::UnsupportedMuxer("mp4".to_string()));
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_unsupported_muxer"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "required output muxer is unavailable"
        );
        assert_eq!(preflight_failed_stage(&err), "build_steps");
    }

    #[test]
    fn unsupported_chapter_muxer_preflight_classification_is_stable() {
        let err =
            JobPreflightError::Build(BuildArgsError::UnsupportedChapterMuxer("mp4".to_string()));
        assert_eq!(
            preflight_error_code(&err),
            "preflight_build_unsupported_chapter_muxer"
        );
        assert_eq!(
            preflight_error_detail(&err),
            "required output muxer cannot author replacement chapters"
        );
        assert_eq!(preflight_failed_stage(&err), "build_steps");
    }

    #[test]
    fn preflight_timeline_for_error_marks_prior_stages_successful() {
        let err = JobPreflightError::Workspace(WorkspaceError::InsufficientCapacity);
        let timeline = preflight_timeline_for_error(&err);
        assert_eq!(timeline.len(), 3);
        assert_eq!(timeline[0].stage, "inspect_plan");
        assert!(timeline[0].ok);
        assert_eq!(timeline[0].code, None);
        assert_eq!(timeline[1].stage, "capability_ready");
        assert!(timeline[1].ok);
        assert_eq!(timeline[1].code, None);
        assert_eq!(timeline[2].stage, "workspace_capacity");
        assert!(!timeline[2].ok);
        assert_eq!(
            timeline[2].code,
            Some("preflight_workspace_insufficient_capacity")
        );
    }

    #[test]
    fn preflight_success_timeline_marks_all_stages_successful() {
        let timeline = preflight_success_timeline();
        assert_eq!(timeline.len(), 5);
        assert_eq!(timeline[0].stage, "inspect_plan");
        assert_eq!(timeline[4].stage, "summarize");
        assert!(timeline.iter().all(|row| row.ok));
        assert!(timeline.iter().all(|row| row.code.is_none()));
    }

    #[test]
    fn preflight_failure_report_projects_stage_code_and_timeline() {
        let err = JobPreflightError::Build(BuildArgsError::UnsupportedCodec("libx265"));
        let report = preflight_failure_report(&err);
        assert_eq!(report.failed_stage, "build_steps");
        assert_eq!(report.error_code, "preflight_build_unsupported_codec");
        assert_eq!(
            report.error_detail,
            "required transcode codec is unavailable"
        );
        assert_eq!(report.timeline.len(), 4);
        assert!(report.timeline[0].ok);
        assert!(!report.timeline[3].ok);
        assert_eq!(
            report.timeline[3].code,
            Some("preflight_build_unsupported_codec")
        );
    }

    #[test]
    fn evaluate_preflight_returns_structured_failed_outcome() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: vec![MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            }),
            error: None,
        };
        let invalid_capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: Vec::new(),
            codec_support: Vec::new(),
            encoders: Vec::new(),
            ..CapabilitySnapshot::default()
        };
        let policy = WorkspacePolicy {
            max_bytes: 100_000,
            reserve_bytes: 1_000,
        };

        let outcome = evaluate_preflight(
            &inspector,
            PreflightBuildInput {
                source_path: "/input/movie.mkv",
                output_path: "/output/movie.mkv",
                backup_path: None,
                quarantine_path: None,
                desired: &desired,
                source_file_bytes: 4_000,
                capabilities: &invalid_capabilities,
                workspace_policy: &policy,
                free_bytes: 20_000,
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        let JobPreflightEvaluation::Failed(failure) = outcome else {
            panic!("expected failed preflight outcome");
        };
        assert_eq!(failure.failed_stage, "capability_ready");
        assert_eq!(failure.error_code, "preflight_capability_failed");
    }

    #[test]
    fn preflight_evaluation_ready_flag_is_deterministic() {
        let ready = JobPreflightEvaluation::Ready(Box::new(JobPreflightReport {
            planned: planned_job_for_tests(Vec::new(), 0),
            summary: super::PlannedJobSummary {
                total_operations: 0,
                remux_operations: 0,
                metadata_rewrite_operations: 0,
                disposition_rewrite_operations: 0,
                label_rewrite_operations: 0,
                stream_reorder_operations: 0,
                embed_subtitle_operations: 0,
                extract_subtitle_operations: 0,
                copy_sidecar_subtitle_operations: 0,
                remove_sidecar_subtitle_operations: 0,
                subtitle_transcode_operations: 0,
                audio_transcode_operations: 0,
                video_transcode_operations: 0,
                explanations: Vec::new(),
            },
            steps: Vec::new(),
            step_audits: Vec::new(),
            timeline: Vec::new(),
            capacity_report: crate::workspace::WorkspaceCapacityReport {
                accepted: true,
                reason: None,
                available_after_reserve_bytes: 0,
                required_workspace_bytes: 0,
            },
        }));
        assert!(ready.is_ready());

        let failed = JobPreflightEvaluation::Failed(JobPreflightFailureReport {
            failed_stage: "capability_ready",
            error_code: "preflight_capability_failed",
            error_detail: "capability snapshot is missing or invalid",
            timeline: vec![PreflightStageRecord {
                stage: "capability_ready",
                ok: false,
                code: Some("preflight_capability_failed"),
            }],
        });
        assert!(!failed.is_ready());
        assert!(failed.as_ready().is_none());
        assert!(failed.as_failed().is_some());
        assert!(ready.as_ready().is_some());
        assert!(ready.as_failed().is_none());
        assert!(ready.planned().is_some());
        assert!(ready.summary().is_some());
        assert!(ready.steps().is_some());
        assert!(ready.capacity_report().is_some());
        assert_eq!(ready.failed_stage(), None);
        assert_eq!(ready.error_code(), None);
        assert_eq!(ready.error_detail(), None);
        assert_eq!(ready.failed_stage_code(), None);
        assert_eq!(ready.final_stage(), None);
        assert_eq!(ready.final_stage_code(), None);
        assert!(failed.planned().is_none());
        assert!(failed.summary().is_none());
        assert!(failed.steps().is_none());
        assert!(failed.capacity_report().is_none());
        assert_eq!(failed.failed_stage(), Some("capability_ready"));
        assert_eq!(failed.error_code(), Some("preflight_capability_failed"));
        assert_eq!(
            failed.error_detail(),
            Some("capability snapshot is missing or invalid")
        );
        assert_eq!(
            failed.failed_stage_code(),
            Some("preflight_capability_failed")
        );
        assert_eq!(failed.final_stage(), Some("capability_ready"));
        assert_eq!(
            failed.final_stage_code(),
            Some("preflight_capability_failed")
        );
        assert!(!failed.failed_at_build_steps());
        assert_eq!(ready.timeline().len(), 0);
        assert_eq!(failed.timeline().len(), 1);
    }

    #[test]
    fn preflight_evaluation_final_stage_accessors_follow_timeline_tail() {
        let ready = JobPreflightEvaluation::Ready(Box::new(JobPreflightReport {
            planned: planned_job_for_tests(Vec::new(), 0),
            summary: super::PlannedJobSummary {
                total_operations: 0,
                remux_operations: 0,
                metadata_rewrite_operations: 0,
                disposition_rewrite_operations: 0,
                label_rewrite_operations: 0,
                stream_reorder_operations: 0,
                embed_subtitle_operations: 0,
                extract_subtitle_operations: 0,
                copy_sidecar_subtitle_operations: 0,
                remove_sidecar_subtitle_operations: 0,
                subtitle_transcode_operations: 0,
                audio_transcode_operations: 0,
                video_transcode_operations: 0,
                explanations: Vec::new(),
            },
            steps: Vec::new(),
            step_audits: Vec::new(),
            timeline: vec![
                PreflightStageRecord {
                    stage: "inspect_source",
                    ok: true,
                    code: None,
                },
                PreflightStageRecord {
                    stage: "ready",
                    ok: true,
                    code: None,
                },
            ],
            capacity_report: crate::workspace::WorkspaceCapacityReport {
                accepted: true,
                reason: None,
                available_after_reserve_bytes: 0,
                required_workspace_bytes: 0,
            },
        }));
        assert_eq!(ready.final_stage(), Some("ready"));
        assert_eq!(ready.final_stage_code(), None);

        let failed = JobPreflightEvaluation::Failed(JobPreflightFailureReport {
            failed_stage: "build_steps",
            error_code: "preflight_build_missing_stream_id",
            error_detail: "stream-scoped operation is missing stream id",
            timeline: vec![
                PreflightStageRecord {
                    stage: "inspect_source",
                    ok: true,
                    code: None,
                },
                PreflightStageRecord {
                    stage: "build_steps",
                    ok: false,
                    code: Some("preflight_build_missing_stream_id"),
                },
            ],
        });
        assert_eq!(failed.final_stage(), Some("build_steps"));
        assert_eq!(
            failed.final_stage_code(),
            Some("preflight_build_missing_stream_id")
        );
        assert!(failed.failed_at_build_steps());
    }

    #[test]
    fn resolve_backup_path_returns_none_when_root_or_source_file_is_missing() {
        assert_eq!(resolve_backup_path(None, "/input/movie.mkv"), None);
        assert_eq!(resolve_backup_path(Some(""), "/input/movie.mkv"), None);
        assert_eq!(resolve_backup_path(Some("/backup"), ""), None);
        assert_eq!(resolve_backup_path(Some("/backup"), "/"), None);
    }

    #[test]
    fn resolve_backup_path_joins_root_and_source_file_name() {
        let path = resolve_backup_path(Some("/backup/media"), "/input/tv/show.s01e01.mkv");
        assert_eq!(path.as_deref(), Some("/backup/media/show.s01e01.mkv"));
    }

    #[test]
    fn resolve_quarantine_path_joins_root_and_source_file_name() {
        let path = resolve_quarantine_path(Some("/quarantine/media"), "/input/tv/show.s01e01.mkv");
        assert_eq!(path.as_deref(), Some("/quarantine/media/show.s01e01.mkv"));
    }

    #[test]
    fn build_preflight_input_resolves_backup_path_from_policy() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };

        let owned = build_preflight_input(
            PreflightBuildTemplate {
                source_path: "/input/tv/show.s01e01.mkv",
                output_path: "/output/tv/show.s01e01.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 100_000,
            },
            PreflightPolicyInput {
                backup_root: Some("/backup/tv"),
                quarantine_root: Some("/quarantine/tv"),
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        assert_eq!(
            owned.backup_path.as_deref(),
            Some("/backup/tv/show.s01e01.mkv")
        );
        assert_eq!(
            owned.quarantine_path.as_deref(),
            Some("/quarantine/tv/show.s01e01.mkv")
        );
    }

    #[test]
    fn owned_preflight_input_as_borrowed_exposes_managed_paths() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };
        let owned = OwnedPreflightBuildInput {
            source_path: "/input/movie.mkv",
            output_path: "/output/movie.mkv",
            backup_path: Some("/backup/movie.mkv".to_string()),
            quarantine_path: Some("/quarantine/movie.mkv".to_string()),
            desired: &desired,
            source_file_bytes: 20_000,
            capabilities: &capabilities,
            workspace_policy: &workspace_policy,
            free_bytes: 500_000,
            video_policy: VideoTranscodePolicy::default(),
        };
        let borrowed = owned.as_borrowed();
        assert_eq!(borrowed.backup_path, Some("/backup/movie.mkv"));
        assert_eq!(borrowed.quarantine_path, Some("/quarantine/movie.mkv"));
        assert_eq!(borrowed.source_path, "/input/movie.mkv");
    }

    #[test]
    fn build_job_execution_steps_with_replacement_policy_includes_quarantine() {
        let planned = planned_job_for_tests(
            vec![PlannedOperation {
                kind: revaer_media_core::plan::OperationKind::Remux,
                stream_id: None,
                output_stream_id: None,
            }],
            1024,
        );
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);

        let steps = build_job_execution_steps_with_replacement_policy(
            "/input/movie.mkv",
            "/workspace/output/movie.mkv",
            &planned,
            &capabilities,
            Some("/backup/movie.mkv"),
            Some("/quarantine/movie.mkv"),
        );
        assert!(steps.as_ref().is_ok());
        let Ok(steps) = steps else {
            return;
        };
        assert!(steps.iter().any(|step| {
            matches!(
                step,
                ExecutionStep::QuarantineFailedOutput { quarantine_path, .. }
                    if quarantine_path == "/quarantine/movie.mkv"
            )
        }));
    }

    #[test]
    fn build_preflight_report_from_template_applies_video_policy_to_steps() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: vec![MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            }),
            error: None,
        };
        let capabilities =
            capability_snapshot_for_tests(&["libx265", "hevc_nvenc"], &["libx265", "hevc_nvenc"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };

        let report = build_preflight_report_from_template(
            &inspector,
            PreflightBuildTemplate {
                source_path: "/input/movie.mkv",
                output_path: "/output/movie.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 500_000,
            },
            PreflightPolicyInput {
                backup_root: None,
                quarantine_root: None,
                video_policy: VideoTranscodePolicy {
                    intent: VideoTranscodeIntent::Anime,
                    hdr_color: HdrColorPolicy::PreserveHdr10,
                    ..VideoTranscodePolicy::default()
                },
            },
        );

        assert!(report.as_ref().is_ok());
        let Ok(report) = report else {
            return;
        };
        let Some(ExecutionStep::Command { argv, .. }) = report.steps.first() else {
            return;
        };
        assert!(argv.iter().any(|item| item == "libx265"));
        assert!(!argv.iter().any(|item| item == "hevc_nvenc"));
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["-color_primaries", "bt2020"])
        );
    }

    #[test]
    fn evaluate_preflight_from_template_builds_and_evaluates_ready_path() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: vec![MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            }),
            error: None,
        };
        let capabilities =
            capability_snapshot_for_tests(&["h264", "libx265"], &["libx264", "libx265"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };

        let outcome = evaluate_preflight_from_template(
            &inspector,
            PreflightBuildTemplate {
                source_path: "/input/movie.mkv",
                output_path: "/output/movie.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 500_000,
            },
            PreflightPolicyInput {
                backup_root: Some("/backup/media"),
                quarantine_root: Some("/quarantine/media"),
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        let JobPreflightEvaluation::Ready(report) = outcome else {
            panic!("expected ready preflight outcome, got {outcome:?}");
        };
        assert!(matches!(
            report.steps.first(),
            Some(ExecutionStep::BackupSource { .. })
        ));
        assert!(
            report
                .steps
                .iter()
                .any(|step| { matches!(step, ExecutionStep::QuarantineFailedOutput { .. }) })
        );
        assert!(matches!(
            report.steps.last(),
            Some(ExecutionStep::AtomicReplace { .. })
        ));
    }

    #[test]
    fn evaluate_preflight_from_template_rejects_quarantine_path_matching_backup() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: Vec::new(),
            }),
            error: None,
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };

        let outcome = evaluate_preflight_from_template(
            &inspector,
            PreflightBuildTemplate {
                source_path: "/input/movie.mkv",
                output_path: "/output/movie.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 500_000,
            },
            PreflightPolicyInput {
                backup_root: Some("/managed"),
                quarantine_root: Some("/managed"),
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        let JobPreflightEvaluation::Failed(report) = outcome else {
            panic!("expected failed preflight outcome");
        };
        assert_eq!(
            report.error_code,
            "preflight_quarantine_path_matches_backup"
        );
        assert_eq!(report.failed_stage, "build_steps");
    }

    #[test]
    fn evaluate_preflight_from_template_rejects_unresolvable_backup_path() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: Vec::new(),
            }),
            error: None,
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };

        let outcome = evaluate_preflight_from_template(
            &inspector,
            PreflightBuildTemplate {
                source_path: "/",
                output_path: "/output/movie.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 500_000,
            },
            PreflightPolicyInput {
                backup_root: Some("/backup/media"),
                quarantine_root: None,
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        let JobPreflightEvaluation::Failed(report) = outcome else {
            panic!("expected failed preflight outcome");
        };
        assert_eq!(
            report.error_code,
            "preflight_backup_path_source_filename_missing"
        );
        assert_eq!(report.failed_stage, "build_steps");
    }

    #[test]
    fn evaluate_preflight_from_template_rejects_backup_path_matching_source() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: Vec::new(),
            }),
            error: None,
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };
        let outcome = evaluate_preflight_from_template(
            &inspector,
            PreflightBuildTemplate {
                source_path: "/input/movie.mkv",
                output_path: "/output/movie.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 500_000,
            },
            PreflightPolicyInput {
                backup_root: Some("/input"),
                quarantine_root: None,
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        let JobPreflightEvaluation::Failed(report) = outcome else {
            panic!("expected failed preflight outcome");
        };
        assert_eq!(report.error_code, "preflight_backup_path_matches_source");
        assert_eq!(report.failed_stage, "build_steps");
    }

    #[test]
    fn evaluate_preflight_from_template_rejects_backup_path_matching_output() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: Vec::new(),
            }),
            error: None,
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };
        let outcome = evaluate_preflight_from_template(
            &inspector,
            PreflightBuildTemplate {
                source_path: "/input/movie.mkv",
                output_path: "/backup/movie.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 500_000,
            },
            PreflightPolicyInput {
                backup_root: Some("/backup"),
                quarantine_root: None,
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        let JobPreflightEvaluation::Failed(report) = outcome else {
            panic!("expected failed preflight outcome");
        };
        assert_eq!(report.error_code, "preflight_backup_path_matches_output");
        assert_eq!(report.failed_stage, "build_steps");
    }

    #[test]
    fn build_preflight_report_from_template_returns_backup_path_error() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: Vec::new(),
            }),
            error: None,
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };

        let result = build_preflight_report_from_template(
            &inspector,
            PreflightBuildTemplate {
                source_path: "/",
                output_path: "/output/movie.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 500_000,
            },
            PreflightPolicyInput {
                backup_root: Some("/backup/media"),
                quarantine_root: None,
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        assert!(matches!(
            result,
            Err(JobPreflightError::BackupPath(
                BackupPathError::SourceFileNameMissing
            ))
        ));
    }

    #[test]
    fn build_preflight_report_from_template_rejects_backup_path_equal_to_source() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: Vec::new(),
            }),
            error: None,
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };
        let result = build_preflight_report_from_template(
            &inspector,
            PreflightBuildTemplate {
                source_path: "/input/movie.mkv",
                output_path: "/output/movie.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 500_000,
            },
            PreflightPolicyInput {
                backup_root: Some("/input"),
                quarantine_root: None,
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        assert!(matches!(
            result,
            Err(JobPreflightError::BackupPath(
                BackupPathError::MatchesSourcePath
            ))
        ));
    }

    #[test]
    fn build_preflight_report_from_template_rejects_backup_path_equal_to_output() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            stream_bindings: Vec::new(),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                container_metadata: Vec::new(),
                container_chapters: Vec::new(),
                container_formats: Vec::new(),
                streams: Vec::new(),
            }),
            error: None,
        };
        let capabilities = capability_snapshot_for_tests(&["h264"], &["libx264"]);
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };
        let result = build_preflight_report_from_template(
            &inspector,
            PreflightBuildTemplate {
                source_path: "/input/movie.mkv",
                output_path: "/backup/movie.mkv",
                desired: &desired,
                source_file_bytes: 50_000,
                capabilities: &capabilities,
                workspace_policy: &workspace_policy,
                free_bytes: 500_000,
            },
            PreflightPolicyInput {
                backup_root: Some("/backup"),
                quarantine_root: None,
                video_policy: VideoTranscodePolicy::default(),
            },
        );
        assert!(matches!(
            result,
            Err(JobPreflightError::BackupPath(
                BackupPathError::MatchesOutputPath
            ))
        ));
    }

    #[test]
    fn preflight_error_code_maps_backup_path_variants_deterministically() {
        assert_eq!(
            preflight_error_code(&JobPreflightError::BackupPath(
                BackupPathError::SourceFileNameMissing
            )),
            "preflight_backup_path_source_filename_missing"
        );
        assert_eq!(
            preflight_error_code(&JobPreflightError::BackupPath(
                BackupPathError::MatchesSourcePath
            )),
            "preflight_backup_path_matches_source"
        );
        assert_eq!(
            preflight_error_code(&JobPreflightError::BackupPath(
                BackupPathError::MatchesOutputPath
            )),
            "preflight_backup_path_matches_output"
        );
    }

    #[test]
    fn preflight_failure_report_projects_backup_path_subcode_on_build_stage() {
        let report = preflight_failure_report(&JobPreflightError::BackupPath(
            BackupPathError::MatchesOutputPath,
        ));
        assert_eq!(report.failed_stage, "build_steps");
        assert_eq!(report.error_code, "preflight_backup_path_matches_output");
        assert_eq!(
            report.error_detail,
            "backup path must not match output path"
        );
        assert_eq!(report.timeline.len(), 4);
        assert!(report.timeline[0].ok);
        assert!(!report.timeline[3].ok);
        assert_eq!(
            report.timeline[3].code,
            Some("preflight_backup_path_matches_output")
        );
    }

    #[test]
    fn preflight_error_detail_maps_backup_path_variants_deterministically() {
        assert_eq!(
            preflight_error_detail(&JobPreflightError::BackupPath(
                BackupPathError::SourceFileNameMissing
            )),
            "configured backup root requires a source file name"
        );
        assert_eq!(
            preflight_error_detail(&JobPreflightError::BackupPath(
                BackupPathError::MatchesSourcePath
            )),
            "backup path must not match source path"
        );
        assert_eq!(
            preflight_error_detail(&JobPreflightError::BackupPath(
                BackupPathError::MatchesOutputPath
            )),
            "backup path must not match output path"
        );
    }

    #[test]
    fn preflight_compact_audit_facts_project_ready_timeline_summary_and_capacity() {
        let outcome = JobPreflightEvaluation::Ready(Box::new(JobPreflightReport {
            planned: planned_job_for_tests(
                vec![PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::Remux,
                    stream_id: None,
                    output_stream_id: None,
                }],
                4_096,
            ),
            summary: super::PlannedJobSummary {
                total_operations: 1,
                remux_operations: 1,
                metadata_rewrite_operations: 0,
                disposition_rewrite_operations: 0,
                label_rewrite_operations: 0,
                stream_reorder_operations: 0,
                embed_subtitle_operations: 0,
                extract_subtitle_operations: 0,
                copy_sidecar_subtitle_operations: 0,
                remove_sidecar_subtitle_operations: 0,
                subtitle_transcode_operations: 0,
                audio_transcode_operations: 0,
                video_transcode_operations: 0,
                explanations: Vec::new(),
            },
            steps: Vec::new(),
            step_audits: Vec::new(),
            timeline: vec![
                PreflightStageRecord {
                    stage: "inspect_plan",
                    ok: true,
                    code: None,
                },
                PreflightStageRecord {
                    stage: "workspace_capacity",
                    ok: true,
                    code: None,
                },
            ],
            capacity_report: crate::workspace::WorkspaceCapacityReport {
                accepted: true,
                reason: None,
                available_after_reserve_bytes: 8_192,
                required_workspace_bytes: 4_096,
            },
        }));

        let facts = preflight_compact_audit_facts(&outcome);

        assert_eq!(
            facts,
            vec![
                CompactAuditFact {
                    audit_index: 0,
                    fact_kind: "preflight_timeline",
                    fact_text: "stage=inspect_plan ok=true code=none".to_string(),
                },
                CompactAuditFact {
                    audit_index: 1,
                    fact_kind: "preflight_timeline",
                    fact_text: "stage=workspace_capacity ok=true code=none".to_string(),
                },
                CompactAuditFact {
                    audit_index: 2,
                    fact_kind: "preflight_plan",
                    fact_text: "operations=1 remux=1 metadata_rewrite=0 disposition_rewrite=0 label_rewrite=0 stream_reorder=0 audio_transcode=0 video_transcode=0 estimated_workspace_bytes=4096".to_string(),
                },
                CompactAuditFact {
                    audit_index: 3,
                    fact_kind: "workspace_capacity",
                    fact_text: "accepted=true reason=none available_after_reserve_bytes=8192 required_workspace_bytes=4096".to_string(),
                },
            ]
        );
    }

    #[test]
    fn preflight_compact_audit_facts_project_failure_stage_and_detail() {
        let outcome = JobPreflightEvaluation::Failed(JobPreflightFailureReport {
            failed_stage: "workspace_capacity",
            error_code: "preflight_workspace_insufficient_capacity",
            error_detail: "free disk cannot satisfy workspace demand above reserve",
            timeline: vec![
                PreflightStageRecord {
                    stage: "inspect_plan",
                    ok: true,
                    code: None,
                },
                PreflightStageRecord {
                    stage: "workspace_capacity",
                    ok: false,
                    code: Some("preflight_workspace_insufficient_capacity"),
                },
            ],
        });

        let facts = preflight_compact_audit_facts(&outcome);

        assert_eq!(
            facts,
            vec![
                CompactAuditFact {
                    audit_index: 0,
                    fact_kind: "preflight_timeline",
                    fact_text: "stage=inspect_plan ok=true code=none".to_string(),
                },
                CompactAuditFact {
                    audit_index: 1,
                    fact_kind: "preflight_timeline",
                    fact_text: "stage=workspace_capacity ok=false code=preflight_workspace_insufficient_capacity".to_string(),
                },
                CompactAuditFact {
                    audit_index: 2,
                    fact_kind: "preflight_failure",
                    fact_text: "stage=workspace_capacity code=preflight_workspace_insufficient_capacity detail=free disk cannot satisfy workspace demand above reserve".to_string(),
                },
            ]
        );
    }
}
