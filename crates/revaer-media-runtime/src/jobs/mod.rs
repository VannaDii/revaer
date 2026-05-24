//! Media job status models.

use revaer_media_core::diff::diff_graphs;
use revaer_media_core::model::{DesiredGraph, MediaGraph};
use revaer_media_core::plan::{PlannedOperation, generate_plan};
use revaer_media_core::verify::verify_plan;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::capabilities::CapabilitySnapshot;
use crate::execute::{BuildArgsError, ExecutionStep, build_execution_steps};
use crate::inspect::{InspectAdapter, InspectError};
use crate::workspace::WorkspacePolicy;

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
        JobPreflightError, JobPreflightRequest, build_job_execution_steps,
        ensure_execution_capacity, plan_job, plan_job_from_inspect,
        require_valid_capability_snapshot,
    };
    use crate::capabilities::CapabilitySnapshot;
    use crate::inspect::{InspectAdapter, InspectError};
    use crate::workspace::{WorkspaceError, WorkspacePolicy};
    use revaer_media_core::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};

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
}
