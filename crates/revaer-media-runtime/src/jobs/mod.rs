//! Media job status models.

use revaer_media_core::diff::diff_graphs;
use revaer_media_core::explain::{Explanation, explain_plan};
use revaer_media_core::model::{DesiredGraph, MediaGraph};
use revaer_media_core::plan::{OperationKind, PlannedOperation, generate_plan};
use revaer_media_core::verify::verify_plan;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

use crate::capabilities::CapabilitySnapshot;
use crate::execute::{
    BuildArgsError, ExecutionStep, build_execution_steps, build_execution_steps_with_capabilities,
    build_execution_steps_with_replacement,
};
use crate::inspect::{InspectAdapter, InspectError};
use crate::workspace::{WorkspaceCapacityReport, WorkspaceError, WorkspacePolicy};

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
    /// Generated deterministic operations.
    pub operations: Vec<PlannedOperation>,
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
    /// Stage timeline projected from the failure.
    pub timeline: Vec<PreflightStageRecord>,
}

/// Deterministic preflight evaluation outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobPreflightEvaluation {
    /// Successful preflight report.
    Ready(JobPreflightReport),
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

    /// Borrow the stage timeline for both ready and failed outcomes.
    #[must_use]
    pub fn timeline(&self) -> &[PreflightStageRecord] {
        match self {
            Self::Ready(report) => &report.timeline,
            Self::Failed(report) => &report.timeline,
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
    BackupPath(&'static str),
}

/// Inputs required to build/evaluate a preflight report.
#[derive(Debug, Clone, Copy)]
pub struct PreflightBuildInput<'a> {
    /// Source path used for inspection.
    pub source_path: &'a str,
    /// Output path used for execution-step generation.
    pub output_path: &'a str,
    /// Optional backup path used before replacement.
    pub backup_path: Option<&'a str>,
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

/// Policy-derived inputs used to construct preflight requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreflightPolicyInput<'a> {
    /// Optional backup root configured by policy/profile.
    pub backup_root: Option<&'a str>,
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
            desired: self.desired,
            source_file_bytes: self.source_file_bytes,
            capabilities: self.capabilities,
            workspace_policy: self.workspace_policy,
            free_bytes: self.free_bytes,
        }
    }
}

/// Resolve deterministic backup output path from optional backup root and source path.
#[must_use]
pub fn resolve_backup_path(backup_root: Option<&str>, source_path: &str) -> Option<String> {
    let root = backup_root.map(str::trim).filter(|value| !value.is_empty())?;
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
    OwnedPreflightBuildInput {
        source_path: template.source_path,
        output_path: template.output_path,
        backup_path,
        desired: template.desired,
        source_file_bytes: template.source_file_bytes,
        capabilities: template.capabilities,
        workspace_policy: template.workspace_policy,
        free_bytes: template.free_bytes,
    }
}

/// Deterministic machine-readable error code for preflight failures.
#[must_use]
pub const fn preflight_error_code(error: &JobPreflightError) -> &'static str {
    match error {
        JobPreflightError::Inspect(_) => "preflight_inspect_failed",
        JobPreflightError::Plan(_) => "preflight_plan_failed",
        JobPreflightError::Capability(_) => "preflight_capability_failed",
        JobPreflightError::Workspace(WorkspaceError::InvalidPolicy) => {
            "preflight_workspace_invalid_policy"
        }
        JobPreflightError::Workspace(WorkspaceError::InsufficientReserve) => {
            "preflight_workspace_insufficient_reserve"
        }
        JobPreflightError::Workspace(WorkspaceError::InsufficientCapacity) => {
            "preflight_workspace_insufficient_capacity"
        }
        JobPreflightError::Workspace(WorkspaceError::ExceedsMaxWorkspace) => {
            "preflight_workspace_exceeds_max"
        }
        JobPreflightError::Build(BuildArgsError::MissingStreamId) => {
            "preflight_build_missing_stream_id"
        }
        JobPreflightError::Build(BuildArgsError::UnsupportedCodec(_)) => {
            "preflight_build_unsupported_codec"
        }
        JobPreflightError::Build(BuildArgsError::CompositionRequired) => {
            "preflight_build_composition_required"
        }
        JobPreflightError::BackupPath(_) => "preflight_backup_path_invalid",
    }
}

/// Deterministic stage label for preflight failures.
#[must_use]
pub const fn preflight_failed_stage(error: &JobPreflightError) -> &'static str {
    match error {
        JobPreflightError::Inspect(_) | JobPreflightError::Plan(_) => "inspect_plan",
        JobPreflightError::Capability(_) => "capability_ready",
        JobPreflightError::Workspace(_) => "workspace_capacity",
        JobPreflightError::Build(_) | JobPreflightError::BackupPath(_) => "build_steps",
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
        timeline: preflight_timeline_for_error(error),
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
    let source = inspector.inspect(source_path)?;
    plan_job_from_source_graph(desired, source_file_bytes, &source).map_err(JobPreflightError::Plan)
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
    let diff = diff_graphs(source, desired);
    let operations = generate_plan(&diff);
    verify_plan(&operations)?;

    Ok(PlannedJob {
        estimated_workspace_bytes: estimate_workspace_bytes(source_file_bytes, &operations),
        operations,
    })
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
    build_execution_steps(input_path, output_path, &planned.operations)
}

/// Build deterministic execution steps from planned job output, validating required codecs.
///
/// # Errors
///
/// Returns [`BuildArgsError::UnsupportedCodec`] when required transcode codec support is missing.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_job_execution_steps_with_capabilities(
    input_path: &str,
    output_path: &str,
    planned: &PlannedJob,
    capabilities: &CapabilitySnapshot,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    build_execution_steps_with_capabilities(
        input_path,
        output_path,
        &planned.operations,
        capabilities,
    )
}

/// Build deterministic execution steps with optional backup and final atomic replacement.
///
/// # Errors
///
/// Returns [`BuildArgsError::UnsupportedCodec`] when required transcode codec support is missing.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
/// Returns [`BuildArgsError::CompositionRequired`] when composition planning is required.
pub fn build_job_execution_steps_with_replacement(
    source_path: &str,
    output_path: &str,
    planned: &PlannedJob,
    capabilities: &CapabilitySnapshot,
    backup_path: Option<&str>,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    build_execution_steps_with_replacement(
        source_path,
        output_path,
        &planned.operations,
        capabilities,
        backup_path,
    )
}

/// Build a deterministic summary of planned operations.
#[must_use]
pub fn summarize_planned_job(planned: &PlannedJob) -> PlannedJobSummary {
    let mut remux_operations = 0_usize;
    let mut audio_transcode_operations = 0_usize;
    let mut video_transcode_operations = 0_usize;

    for operation in &planned.operations {
        match operation.kind {
            OperationKind::Remux => remux_operations += 1,
            OperationKind::AudioTranscode => audio_transcode_operations += 1,
            OperationKind::VideoTranscode => video_transcode_operations += 1,
        }
    }

    PlannedJobSummary {
        total_operations: planned.operations.len(),
        remux_operations,
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
    require_valid_capability_snapshot(Some(input.capabilities))
        .map_err(JobPreflightError::Capability)?;
    let capacity_report = input
        .workspace_policy
        .evaluate_capacity(input.free_bytes, planned.estimated_workspace_bytes);
    ensure_execution_capacity(input.workspace_policy, input.free_bytes, &planned)?;
    let steps = build_job_execution_steps_with_replacement(
        input.source_path,
        input.output_path,
        &planned,
        input.capabilities,
        input.backup_path,
    )?;
    let summary = summarize_planned_job(&planned);
    let timeline = preflight_success_timeline();
    Ok(JobPreflightReport {
        planned,
        summary,
        steps,
        timeline,
        capacity_report,
    })
}

/// Evaluate preflight and always return a structured outcome payload.
#[must_use]
pub fn evaluate_preflight(
    inspector: &dyn InspectAdapter,
    input: PreflightBuildInput<'_>,
) -> JobPreflightEvaluation {
    match build_preflight_report(inspector, input) {
        Ok(report) => JobPreflightEvaluation::Ready(report),
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
    if policy_input
        .backup_root
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        && resolve_backup_path(policy_input.backup_root, template.source_path).is_none()
    {
        return Err(JobPreflightError::BackupPath(
            "configured backup root requires a source file name",
        ));
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
        Ok(report) => JobPreflightEvaluation::Ready(report),
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
    // Conservative fixed multipliers for current foundation implementation.
    let mut max_multiplier_num: u64 = 1;
    let mut max_multiplier_den: u64 = 1;

    for op in operations {
        let candidate = match op.kind {
            revaer_media_core::plan::OperationKind::Remux => (6_u64, 5_u64), // 1.2x
            revaer_media_core::plan::OperationKind::AudioTranscode => (3_u64, 2_u64), // 1.5x
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

#[cfg(test)]
mod tests {
    use super::{
        BuildArgsError, ExecutionStep, JobPreflightError, JobPreflightEvaluation,
        JobPreflightFailureReport, JobPreflightReport, JobPreflightRequest,
        OwnedPreflightBuildInput, PlannedJob, PreflightBuildInput, PreflightBuildTemplate,
        PreflightPolicyInput,
        PreflightStageRecord, build_job_execution_steps,
        build_preflight_input,
        build_preflight_report_from_template,
        build_job_execution_steps_with_capabilities, build_job_execution_steps_with_replacement,
        build_preflight_report,
        ensure_execution_capacity, evaluate_preflight, evaluate_preflight_from_template, plan_job,
        plan_job_from_inspect,
        preflight_error_code, preflight_failed_stage, preflight_failure_report,
        preflight_success_timeline, preflight_timeline_for_error, resolve_backup_path,
        require_valid_capability_snapshot, summarize_planned_job,
    };
    use crate::capabilities::CapabilitySnapshot;
    use crate::inspect::{InspectAdapter, InspectError};
    use crate::workspace::{WorkspaceError, WorkspacePolicy};
    use revaer_media_core::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};
    use revaer_media_core::plan::PlannedOperation;

    #[test]
    fn plan_job_builds_operations_and_estimate() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
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
        assert!(planned.estimated_workspace_bytes > 1_000);
    }

    #[test]
    fn preflight_capacity_check_fails_when_demand_exceeds_reserve_budget() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
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
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
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
    fn require_capability_snapshot_rejects_missing_or_invalid_state() {
        assert_eq!(
            require_valid_capability_snapshot(None),
            Err("media capability snapshot is missing")
        );

        let invalid = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: Vec::new(),
        };
        assert_eq!(
            require_valid_capability_snapshot(Some(&invalid)),
            Err("media capability snapshot is invalid")
        );
    }

    #[test]
    fn require_capability_snapshot_accepts_valid_snapshot() {
        let valid = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
        };
        assert!(require_valid_capability_snapshot(Some(&valid)).is_ok());
    }

    #[test]
    fn build_job_execution_steps_with_capabilities_rejects_unsupported_codec() {
        let planned = PlannedJob {
            operations: vec![PlannedOperation {
                kind: revaer_media_core::plan::OperationKind::VideoTranscode,
                stream_id: Some(0),
            }],
            estimated_workspace_bytes: 100,
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
        };
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
        let planned = PlannedJob {
            operations: vec![PlannedOperation {
                kind: revaer_media_core::plan::OperationKind::Remux,
                stream_id: None,
            }],
            estimated_workspace_bytes: 100,
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
        };
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
        error: Option<InspectError>,
    }

    impl InspectAdapter for StubInspectAdapter {
        fn inspect(&self, _source_path: &str) -> Result<MediaGraph, InspectError> {
            if let Some(err) = &self.error {
                return Err(InspectError::Adapter(err.to_string()));
            }
            self.graph
                .clone()
                .ok_or_else(|| InspectError::Adapter("missing graph".to_string()))
        }
    }

    #[test]
    fn plan_job_from_inspect_uses_inspected_graph() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                streams: vec![MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
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
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: None,
            error: Some(InspectError::Adapter("probe failed".to_string())),
        };

        let err = plan_job_from_inspect(&inspector, "/input/movie.mkv", &desired, 5_000).err();
        assert!(matches!(err, Some(JobPreflightError::Inspect(_))));
    }

    #[test]
    fn summarize_planned_job_counts_kinds_and_includes_explanations() {
        let planned = PlannedJob {
            operations: vec![
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::Remux,
                    stream_id: None,
                },
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::AudioTranscode,
                    stream_id: Some(1),
                },
                PlannedOperation {
                    kind: revaer_media_core::plan::OperationKind::VideoTranscode,
                    stream_id: Some(0),
                },
            ],
            estimated_workspace_bytes: 123,
        };

        let summary = summarize_planned_job(&planned);
        assert_eq!(summary.total_operations, 3);
        assert_eq!(summary.remux_operations, 1);
        assert_eq!(summary.audio_transcode_operations, 1);
        assert_eq!(summary.video_transcode_operations, 1);
        assert_eq!(summary.explanations.len(), 3);
    }

    #[test]
    fn build_preflight_report_returns_summary_and_steps() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                streams: vec![MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            }),
            error: None,
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["libx265".to_string()],
        };
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
                desired: &desired,
                source_file_bytes: 4_000,
                capabilities: &capabilities,
                workspace_policy: &policy,
                free_bytes: 20_000,
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
    fn build_preflight_report_rejects_invalid_capabilities() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                streams: vec![MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
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
                desired: &desired,
                source_file_bytes: 4_000,
                capabilities: &invalid_capabilities,
                workspace_policy: &policy,
                free_bytes: 20_000,
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
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                streams: vec![MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
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
                desired: &desired,
                source_file_bytes: 4_000,
                capabilities: &invalid_capabilities,
                workspace_policy: &policy,
                free_bytes: 20_000,
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
        let ready = JobPreflightEvaluation::Ready(JobPreflightReport {
            planned: PlannedJob {
                operations: Vec::new(),
                estimated_workspace_bytes: 0,
            },
            summary: super::PlannedJobSummary {
                total_operations: 0,
                remux_operations: 0,
                audio_transcode_operations: 0,
                video_transcode_operations: 0,
                explanations: Vec::new(),
            },
            steps: Vec::new(),
            timeline: Vec::new(),
            capacity_report: crate::workspace::WorkspaceCapacityReport {
                accepted: true,
                reason: None,
                available_after_reserve_bytes: 0,
                required_workspace_bytes: 0,
            },
        });
        assert!(ready.is_ready());

        let failed = JobPreflightEvaluation::Failed(JobPreflightFailureReport {
            failed_stage: "capability_ready",
            error_code: "preflight_capability_failed",
            timeline: Vec::new(),
        });
        assert!(!failed.is_ready());
        assert!(failed.as_ready().is_none());
        assert!(failed.as_failed().is_some());
        assert!(ready.as_ready().is_some());
        assert!(ready.as_failed().is_none());
        assert_eq!(ready.failed_stage(), None);
        assert_eq!(ready.error_code(), None);
        assert_eq!(failed.failed_stage(), Some("capability_ready"));
        assert_eq!(failed.error_code(), Some("preflight_capability_failed"));
        assert_eq!(ready.timeline().len(), 0);
        assert_eq!(failed.timeline().len(), 0);
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
    fn build_preflight_input_resolves_backup_path_from_policy() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: Vec::new(),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
        };
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
            },
        );
        assert_eq!(
            owned.backup_path.as_deref(),
            Some("/backup/tv/show.s01e01.mkv")
        );
    }

    #[test]
    fn owned_preflight_input_as_borrowed_exposes_backup_path() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: Vec::new(),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
        };
        let workspace_policy = WorkspacePolicy {
            max_bytes: 1_000_000,
            reserve_bytes: 10_000,
        };
        let owned = OwnedPreflightBuildInput {
            source_path: "/input/movie.mkv",
            output_path: "/output/movie.mkv",
            backup_path: Some("/backup/movie.mkv".to_string()),
            desired: &desired,
            source_file_bytes: 20_000,
            capabilities: &capabilities,
            workspace_policy: &workspace_policy,
            free_bytes: 500_000,
        };
        let borrowed = owned.as_borrowed();
        assert_eq!(borrowed.backup_path, Some("/backup/movie.mkv"));
        assert_eq!(borrowed.source_path, "/input/movie.mkv");
    }

    #[test]
    fn evaluate_preflight_from_template_builds_and_evaluates_ready_path() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/input/movie.mkv".to_string(),
                streams: vec![MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            }),
            error: None,
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
        };
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
            },
        );
        let JobPreflightEvaluation::Ready(report) = outcome else {
            panic!("expected ready preflight outcome");
        };
        assert!(matches!(
            report.steps.first(),
            Some(ExecutionStep::BackupSource { .. })
        ));
        assert!(matches!(
            report.steps.last(),
            Some(ExecutionStep::AtomicReplace { .. })
        ));
    }

    #[test]
    fn evaluate_preflight_from_template_rejects_unresolvable_backup_path() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/".to_string(),
                streams: Vec::new(),
            }),
            error: None,
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
        };
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
            },
        );
        let JobPreflightEvaluation::Failed(report) = outcome else {
            panic!("expected failed preflight outcome");
        };
        assert_eq!(report.error_code, "preflight_backup_path_invalid");
        assert_eq!(report.failed_stage, "build_steps");
    }

    #[test]
    fn build_preflight_report_from_template_returns_backup_path_error() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: Vec::new(),
        };
        let inspector = StubInspectAdapter {
            graph: Some(MediaGraph {
                source_path: "/".to_string(),
                streams: Vec::new(),
            }),
            error: None,
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
        };
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
            },
        );
        assert!(matches!(result, Err(JobPreflightError::BackupPath(_))));
    }
}
