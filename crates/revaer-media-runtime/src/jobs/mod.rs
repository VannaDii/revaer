//! Media job status models.

use revaer_media_core::diff::diff_graphs;
use revaer_media_core::explain::{Explanation, explain_plan};
use revaer_media_core::model::{DesiredGraph, MediaGraph};
use revaer_media_core::plan::{OperationKind, PlannedOperation, generate_plan};
use revaer_media_core::verify::verify_plan;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::capabilities::CapabilitySnapshot;
use crate::execute::{
    BuildArgsError, ExecutionStep, build_execution_steps, build_execution_steps_with_capabilities,
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
}

/// Deterministic machine-readable error code for preflight failures.
#[must_use]
pub fn preflight_error_code(error: &JobPreflightError) -> &'static str {
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
    }
}

/// Deterministic stage label for preflight failures.
#[must_use]
pub fn preflight_failed_stage(error: &JobPreflightError) -> &'static str {
    match error {
        JobPreflightError::Inspect(_) | JobPreflightError::Plan(_) => "inspect_plan",
        JobPreflightError::Capability(_) => "capability_ready",
        JobPreflightError::Workspace(_) => "workspace_capacity",
        JobPreflightError::Build(_) => "build_steps",
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
    source_path: &str,
    output_path: &str,
    desired: &DesiredGraph,
    source_file_bytes: u64,
    capabilities: &CapabilitySnapshot,
    workspace_policy: &WorkspacePolicy,
    free_bytes: u64,
) -> Result<JobPreflightReport, JobPreflightError> {
    let planned = plan_job_from_inspect(inspector, source_path, desired, source_file_bytes)?;
    require_valid_capability_snapshot(Some(capabilities)).map_err(JobPreflightError::Capability)?;
    let capacity_report =
        workspace_policy.evaluate_capacity(free_bytes, planned.estimated_workspace_bytes);
    ensure_execution_capacity(workspace_policy, free_bytes, &planned)?;
    let steps = build_job_execution_steps_with_capabilities(
        source_path,
        output_path,
        &planned,
        capabilities,
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
    source_path: &str,
    output_path: &str,
    desired: &DesiredGraph,
    source_file_bytes: u64,
    capabilities: &CapabilitySnapshot,
    workspace_policy: &WorkspacePolicy,
    free_bytes: u64,
) -> JobPreflightEvaluation {
    match build_preflight_report(
        inspector,
        source_path,
        output_path,
        desired,
        source_file_bytes,
        capabilities,
        workspace_policy,
        free_bytes,
    ) {
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
        BuildArgsError, JobPreflightError, JobPreflightEvaluation, JobPreflightFailureReport,
        JobPreflightReport, JobPreflightRequest, PlannedJob, PreflightStageRecord,
        build_job_execution_steps, build_job_execution_steps_with_capabilities,
        build_preflight_report, ensure_execution_capacity, evaluate_preflight, plan_job,
        plan_job_from_inspect, preflight_error_code, preflight_failed_stage,
        preflight_failure_report, preflight_success_timeline, preflight_timeline_for_error,
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
            "/input/movie.mkv",
            "/output/movie.mkv",
            &desired,
            4_000,
            &capabilities,
            &policy,
            20_000,
        );
        assert!(report.is_ok());
        let Ok(report) = report else {
            return;
        };
        assert!(!report.planned.operations.is_empty());
        assert!(!report.summary.explanations.is_empty());
        assert!(!report.steps.is_empty());
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
            "/input/movie.mkv",
            "/output/movie.mkv",
            &desired,
            4_000,
            &invalid_capabilities,
            &policy,
            20_000,
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
            "/input/movie.mkv",
            "/output/movie.mkv",
            &desired,
            4_000,
            &invalid_capabilities,
            &policy,
            20_000,
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
    }
}
