//! In-process media job runtime.
//!
//! # Design
//! - Claims queued media jobs through stored procedures.
//! - Persists phase, operation, verification, and compact-audit rows before terminal status.
//! - Keeps runtime adapters injected so tests avoid real `ffmpeg` execution.

use std::fs;
use std::num::TryFromIntError;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use revaer_data::DataError;
use revaer_data::media::jobs::{
    AppendMediaJobCompactAuditInput, AppendMediaJobVerificationCheckInput, ClaimedMediaJobRow,
};
use revaer_media_core::model::DesiredGraph;
use revaer_media_core::model::{MediaGraph, MediaStream, StreamKind};
use revaer_media_core::plan::{OperationKind, PlannedOperation};
use revaer_media_runtime::capabilities::{CapabilitySnapshot, CodecCapability};
use revaer_media_runtime::execute::{
    CommandRunner, ExecuteSequenceError, ExecuteStepError, ExecutionStep, ProcessCommandRunner,
    VideoTranscodeIntent, VideoTranscodePolicy, execute_filesystem_step, execute_step_sequence,
};
use revaer_media_runtime::inspect::{
    FfprobeInspectAdapter, InspectAdapter, SystemInspectProbeExecutor,
};
use revaer_media_runtime::jobs::{
    JobPreflightEvaluation, JobPreflightReport, PreflightBuildTemplate, PreflightPolicyInput,
    evaluate_preflight_from_template, preflight_compact_audit_facts,
};
use revaer_media_runtime::workspace::{
    ManagedWorkspaceError, TerminalWorkspaceCleanupPolicy, TerminalWorkspaceState, WorkspacePolicy,
    cleanup_terminal_workspace, create_managed_workspace,
};
use revaer_runtime::media::MediaStore;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};
use uuid::Uuid;

const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(1);
const DEFAULT_WORKSPACE_MAX_BYTES: u64 = 20 * 1024 * 1024 * 1024;
const DEFAULT_WORKSPACE_RESERVE_BYTES: u64 = 1024 * 1024 * 1024;
const DEFAULT_WORKSPACE_ROOT_NAME: &str = "revaer-media-runtime";
const FAILURE_PHASE_INDEX: i32 = 99;
const FAILURE_CHECK_INDEX: i32 = 99;

type RuntimeInspector = dyn InspectAdapter + Send + Sync;
type RuntimeCommandRunner = dyn CommandRunner + Send + Sync;
type RuntimeCapacityProbe = dyn FilesystemCapacityProbe + Send + Sync;

struct RuntimePreflightEvaluation {
    evaluation: JobPreflightEvaluation,
    desired: DesiredGraph,
}

trait FilesystemCapacityProbe {
    fn available_bytes(&self, path: &Path) -> Result<u64, String>;
}

#[derive(Debug, Default, Clone, Copy)]
struct SystemFilesystemCapacityProbe;

impl FilesystemCapacityProbe for SystemFilesystemCapacityProbe {
    fn available_bytes(&self, path: &Path) -> Result<u64, String> {
        revaer_fsops::available_bytes(path).map_err(|error| error.to_string())
    }
}

/// Runtime worker that progresses queued media jobs to terminal status.
pub(crate) struct MediaJobRuntime {
    store: MediaStore,
    inspector: Arc<RuntimeInspector>,
    command_runner: Arc<RuntimeCommandRunner>,
    capacity_probe: Arc<RuntimeCapacityProbe>,
    tick_interval: Duration,
    workspace_policy: WorkspacePolicy,
    workspace_root: PathBuf,
}

impl MediaJobRuntime {
    /// Construct a production media job runtime.
    pub(crate) fn new(store: MediaStore) -> Self {
        Self::with_components(
            store,
            Arc::new(FfprobeInspectAdapter::new(
                Arc::new(SystemInspectProbeExecutor),
                "ffprobe",
            )),
            Arc::new(ProcessCommandRunner),
            Arc::new(SystemFilesystemCapacityProbe),
            DEFAULT_TICK_INTERVAL,
            WorkspacePolicy {
                max_bytes: DEFAULT_WORKSPACE_MAX_BYTES,
                reserve_bytes: DEFAULT_WORKSPACE_RESERVE_BYTES,
            },
            std::env::temp_dir().join(DEFAULT_WORKSPACE_ROOT_NAME),
        )
    }

    fn with_components(
        store: MediaStore,
        inspector: Arc<RuntimeInspector>,
        command_runner: Arc<RuntimeCommandRunner>,
        capacity_probe: Arc<RuntimeCapacityProbe>,
        tick_interval: Duration,
        workspace_policy: WorkspacePolicy,
        workspace_root: PathBuf,
    ) -> Self {
        Self {
            store,
            inspector,
            command_runner,
            capacity_probe,
            tick_interval,
            workspace_policy,
            workspace_root,
        }
    }

    /// Spawn the media worker loop.
    pub(crate) fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run_loop().await;
        })
    }

    async fn run_loop(self) {
        let mut ticker = interval(self.tick_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            if let Err(error) = self.run_tick().await {
                warn!(error = %error, "media job runtime tick failed");
            }
        }
    }

    async fn run_tick(&self) -> Result<(), DataError> {
        let claimed = self.store.claim_next_job().await?;
        if let Some(job) = claimed {
            self.process_job(job).await;
        }
        Ok(())
    }

    async fn process_job(&self, job: ClaimedMediaJobRow) {
        let workspace =
            create_managed_workspace(&self.workspace_root, &job.media_job_public_id.to_string());
        let workspace = match workspace {
            Ok(value) => value,
            Err(error) => {
                warn!(media_job_public_id = %job.media_job_public_id, error = %error, "media job runtime failed to create workspace");
                self.persist_failure(&job, MediaJobRuntimeError::Workspace(error))
                    .await;
                return;
            }
        };

        let terminal_state = match self.process_claimed_job(&job, &workspace).await {
            Ok(state) => {
                info!(media_job_public_id = %job.media_job_public_id, "media job runtime processed job");
                state
            }
            Err(error) => {
                warn!(media_job_public_id = %job.media_job_public_id, error = %error, "media job runtime failed job processing");
                self.persist_failure(&job, error).await;
                TerminalWorkspaceState::Failed
            }
        };

        if let Err(error) = cleanup_terminal_workspace(
            &workspace,
            terminal_state,
            TerminalWorkspaceCleanupPolicy {
                retain_diagnostics: terminal_state != TerminalWorkspaceState::Completed,
            },
        ) {
            warn!(media_job_public_id = %job.media_job_public_id, error = %error, "media job runtime failed workspace cleanup");
        }
    }

    async fn process_claimed_job(
        &self,
        job: &ClaimedMediaJobRow,
        workspace: &revaer_media_runtime::workspace::ManagedWorkspace,
    ) -> Result<TerminalWorkspaceState, MediaJobRuntimeError> {
        self.store.heartbeat_job(job.media_job_public_id).await?;
        self.append_phase(job.media_job_public_id, 0, "inspect_plan", "running", None)
            .await?;

        let final_output_path = resolve_output_path(job)?;
        let output_path = resolve_workspace_output_path(&final_output_path, workspace)?;
        if !job.dry_run {
            validate_existing_ancestor_bounds(&job.source_path, &job.source_root, "source_path")?;
            validate_existing_ancestor_bounds(&final_output_path, &job.output_root, "output_path")?;
        }

        let capabilities = self.load_capability_snapshot().await?;
        let preflight = self
            .build_preflight_evaluation(job, output_path.clone(), workspace, capabilities)
            .await?;
        self.persist_preflight_audits(job.media_job_public_id, &preflight.evaluation)
            .await?;
        let RuntimePreflightEvaluation {
            evaluation,
            desired,
        } = preflight;

        match evaluation {
            JobPreflightEvaluation::Failed(report) => {
                self.handle_preflight_failed(job, report).await
            }
            JobPreflightEvaluation::Ready(report) => {
                self.handle_preflight_ready(job, report, &desired).await
            }
        }
    }

    async fn handle_preflight_failed(
        &self,
        job: &ClaimedMediaJobRow,
        report: revaer_media_runtime::jobs::JobPreflightFailureReport,
    ) -> Result<TerminalWorkspaceState, MediaJobRuntimeError> {
        self.append_phase(
            job.media_job_public_id,
            0,
            report.failed_stage,
            "failed",
            Some(report.error_code),
        )
        .await?;
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id: job.media_job_public_id,
            check_index: 0,
            check_kind: "preflight",
            check_status: "failed",
            expected_value: Some("ready"),
            actual_value: Some(report.error_code),
            details_text: Some(report.error_detail),
        })
        .await?;
        self.mark_failed(job, report.error_code).await?;
        Ok(TerminalWorkspaceState::Failed)
    }

    async fn handle_preflight_ready(
        &self,
        job: &ClaimedMediaJobRow,
        report: JobPreflightReport,
        desired: &DesiredGraph,
    ) -> Result<TerminalWorkspaceState, MediaJobRuntimeError> {
        self.append_phase(
            job.media_job_public_id,
            0,
            "inspect_plan",
            "completed",
            None,
        )
        .await?;
        self.persist_ready_plan(job.media_job_public_id, &report)
            .await?;
        if job.dry_run {
            self.complete_dry_run(job).await?;
            return Ok(TerminalWorkspaceState::Completed);
        }

        self.store
            .mark_job_status(job.media_job_public_id, "verifying", None)
            .await?;
        self.append_phase(job.media_job_public_id, 1, "execute", "running", None)
            .await?;
        self.execute_verified_replacement(job, report.steps, desired)
            .await?;
        self.append_phase(job.media_job_public_id, 1, "execute", "completed", None)
            .await?;
        self.append_phase(
            job.media_job_public_id,
            2,
            "verify_replace",
            "completed",
            None,
        )
        .await?;
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id: job.media_job_public_id,
            check_index: 0,
            check_kind: "output_replacement",
            check_status: "passed",
            expected_value: Some("verified_atomic_replace"),
            actual_value: Some("completed"),
            details_text: None,
        })
        .await?;
        self.store
            .mark_job_status(job.media_job_public_id, "completed", None)
            .await?;
        Ok(TerminalWorkspaceState::Completed)
    }

    async fn complete_dry_run(&self, job: &ClaimedMediaJobRow) -> Result<(), MediaJobRuntimeError> {
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id: job.media_job_public_id,
            check_index: 0,
            check_kind: "dry_run_preflight",
            check_status: "passed",
            expected_value: Some("execution_skipped"),
            actual_value: Some("dry_run"),
            details_text: Some("dry-run job completed without destructive execution"),
        })
        .await?;
        self.store
            .mark_job_status(job.media_job_public_id, "completed", None)
            .await?;
        Ok(())
    }

    async fn build_preflight_evaluation(
        &self,
        job: &ClaimedMediaJobRow,
        output_path: String,
        workspace: &revaer_media_runtime::workspace::ManagedWorkspace,
        capabilities: CapabilitySnapshot,
    ) -> Result<RuntimePreflightEvaluation, MediaJobRuntimeError> {
        let inspector = Arc::clone(&self.inspector);
        let workspace_policy = self.workspace_policy.clone();
        let capacity_probe = Arc::clone(&self.capacity_probe);
        let source_path = job.source_path.clone();
        let desired_target = desired_target_from_job(job)?;
        let video_policy = video_policy_from_policy_intent(job.policy_video_intent.as_deref())?;
        let workspace_output_path = workspace.output_path.clone();
        let diagnostics_root = path_to_string(&workspace.diagnostics_path, "diagnostics_path")?;
        tokio::task::spawn_blocking(move || {
            let source_graph = inspector
                .inspect(&source_path)
                .map_err(|error| MediaJobRuntimeError::Inspect(error.to_string()))?;
            let source_file_bytes = source_file_bytes(&source_path)?;
            let desired =
                compile_desired_graph(&source_graph, &output_path, desired_target.as_ref());
            let free_bytes = capacity_probe
                .available_bytes(&workspace_output_path)
                .map_err(MediaJobRuntimeError::Capacity)?;
            let evaluation = evaluate_preflight_from_template(
                &*inspector,
                PreflightBuildTemplate {
                    source_path: &source_path,
                    output_path: &output_path,
                    desired: &desired,
                    source_file_bytes,
                    capabilities: &capabilities,
                    workspace_policy: &workspace_policy,
                    free_bytes,
                },
                PreflightPolicyInput {
                    backup_root: None,
                    quarantine_root: Some(&diagnostics_root),
                    video_policy,
                },
            );
            Ok(RuntimePreflightEvaluation {
                evaluation,
                desired,
            })
        })
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?
    }

    async fn execute_steps(&self, steps: Vec<ExecutionStep>) -> Result<(), MediaJobRuntimeError> {
        let runner = Arc::clone(&self.command_runner);
        tokio::task::spawn_blocking(move || execute_step_sequence(&steps, &*runner))
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?
            .map_err(MediaJobRuntimeError::Execute)
    }

    async fn execute_filesystem_step_direct(
        &self,
        step: ExecutionStep,
    ) -> Result<(), MediaJobRuntimeError> {
        tokio::task::spawn_blocking(move || execute_filesystem_step(&step))
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?
            .map_err(MediaJobRuntimeError::ExecuteStep)
    }

    async fn execute_verified_replacement(
        &self,
        job: &ClaimedMediaJobRow,
        steps: Vec<ExecutionStep>,
        desired: &DesiredGraph,
    ) -> Result<(), MediaJobRuntimeError> {
        let candidate_output_path = replacement_output_path(&steps)?;
        let pre_replace_steps = steps
            .iter()
            .filter(|step| !matches!(step, ExecutionStep::AtomicReplace { .. }))
            .filter(|step| !matches!(step, ExecutionStep::QuarantineFailedOutput { .. }))
            .cloned()
            .collect::<Vec<_>>();
        self.execute_steps(pre_replace_steps).await?;
        if let Err(error) = self
            .verify_graph_check(
                job.media_job_public_id,
                1,
                "candidate_graph",
                &candidate_output_path,
                desired,
            )
            .await
        {
            self.quarantine_candidate(&steps).await?;
            return Err(error);
        }

        let replace_step = steps
            .iter()
            .find(|step| matches!(step, ExecutionStep::AtomicReplace { .. }))
            .cloned()
            .ok_or(MediaJobRuntimeError::Verification(
                "media_job_atomic_replace_step_missing",
            ))?;
        self.execute_steps(vec![replace_step]).await?;
        self.verify_graph_check(
            job.media_job_public_id,
            2,
            "final_graph",
            &job.source_path,
            desired,
        )
        .await?;
        Ok(())
    }

    async fn quarantine_candidate(
        &self,
        steps: &[ExecutionStep],
    ) -> Result<(), MediaJobRuntimeError> {
        let Some(step) = steps
            .iter()
            .find(|step| matches!(step, ExecutionStep::QuarantineFailedOutput { .. }))
            .cloned()
        else {
            return Ok(());
        };
        self.execute_filesystem_step_direct(step).await
    }

    async fn verify_graph_check(
        &self,
        media_job_public_id: Uuid,
        check_index: i32,
        check_kind: &'static str,
        output_path: &str,
        desired: &DesiredGraph,
    ) -> Result<(), MediaJobRuntimeError> {
        let actual = self.inspect_graph(output_path.to_string()).await?;
        let matched = media_graph_matches_desired(&actual, desired);
        let check_status = if matched { "passed" } else { "failed" };
        let actual_value = if matched { "matched" } else { "mismatched" };
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id,
            check_index,
            check_kind,
            check_status,
            expected_value: Some("desired_graph"),
            actual_value: Some(actual_value),
            details_text: Some(output_path),
        })
        .await?;
        if matched {
            Ok(())
        } else {
            Err(MediaJobRuntimeError::Verification(
                "media_job_output_graph_mismatch",
            ))
        }
    }

    async fn inspect_graph(&self, source_path: String) -> Result<MediaGraph, MediaJobRuntimeError> {
        let inspector = Arc::clone(&self.inspector);
        tokio::task::spawn_blocking(move || {
            inspector
                .inspect(&source_path)
                .map_err(|error| MediaJobRuntimeError::Inspect(error.to_string()))
        })
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?
    }

    async fn load_capability_snapshot(&self) -> Result<CapabilitySnapshot, MediaJobRuntimeError> {
        let snapshot = self.store.latest_capability().await?;
        let Some(snapshot) = snapshot else {
            return Err(MediaJobRuntimeError::InvalidCapability(
                "media_capability_snapshot_missing",
            ));
        };
        if snapshot.ffmpeg_version.trim().is_empty()
            || snapshot.ffprobe_version.trim().is_empty()
            || snapshot.codecs.is_empty()
            || snapshot.encoders.is_empty()
            || feature_names(&snapshot.features, "decoder", true).is_empty()
            || feature_names(&snapshot.features, "muxer", true).is_empty()
            || feature_names(&snapshot.features, "demuxer", true).is_empty()
            || feature_names(&snapshot.features, "filesystem", true).is_empty()
            || feature_names(&snapshot.features, "utility", true).is_empty()
            || feature_names(&snapshot.features, "license", true).is_empty()
        {
            return Err(MediaJobRuntimeError::InvalidCapability(
                "media_capability_snapshot_invalid",
            ));
        }

        let codecs = snapshot
            .codecs
            .iter()
            .map(|codec| codec.codec_name.clone())
            .collect::<Vec<_>>();
        let codec_support = snapshot
            .codecs
            .iter()
            .map(|codec| CodecCapability {
                name: codec.codec_name.clone(),
                encode_supported: codec.encode_supported,
                decode_supported: codec.decode_supported,
            })
            .collect::<Vec<_>>();
        let encoders = snapshot.encoders;
        let decoders = feature_names(&snapshot.features, "decoder", true);
        let muxers = feature_names(&snapshot.features, "muxer", true);
        let demuxers = feature_names(&snapshot.features, "demuxer", true);
        let hardware_accelerators = feature_names(&snapshot.features, "hardware", true);
        let subtitle_support = feature_names(&snapshot.features, "subtitle", true);
        let filesystem_utilities = feature_names(&snapshot.features, "filesystem", true);
        let utility_capabilities = feature_names(&snapshot.features, "utility", true);
        let license_mode = feature_names(&snapshot.features, "license", true)
            .into_iter()
            .next()
            .unwrap_or_default();
        let compliance_links = feature_names(&snapshot.features, "compliance", true);
        let absent_capabilities = feature_names(&snapshot.features, "absent", false);

        Ok(CapabilitySnapshot {
            ffmpeg_version: snapshot.ffmpeg_version,
            ffprobe_version: snapshot.ffprobe_version,
            codecs,
            codec_support,
            encoders,
            decoders,
            muxers,
            demuxers,
            hardware_accelerators,
            subtitle_support,
            filesystem_utilities,
            utility_capabilities,
            license_mode,
            compliance_links,
            absent_capabilities,
        })
    }

    async fn persist_ready_plan(
        &self,
        media_job_public_id: Uuid,
        report: &JobPreflightReport,
    ) -> Result<(), MediaJobRuntimeError> {
        for (index, operation) in report.planned.operations.iter().enumerate() {
            let (command_bin, args) = command_fields_for_operation(&report.steps, index);
            self.store
                .append_job_operation(
                    media_job_public_id,
                    usize_to_i32(index, "operation_index")?,
                    operation_kind_code(operation.kind),
                    stream_id_to_i32(operation)?,
                    command_bin,
                    args,
                )
                .await?;
        }

        for (index, explanation) in report.summary.explanations.iter().enumerate() {
            self.store
                .append_job_plan_reason(
                    media_job_public_id,
                    usize_to_i32(index, "reason_index")?,
                    Some(usize_to_i32(index, "candidate_index")?),
                    true,
                    "selected_operation",
                    &explanation.message,
                )
                .await?;
        }
        Ok(())
    }

    async fn persist_preflight_audits(
        &self,
        media_job_public_id: Uuid,
        evaluation: &JobPreflightEvaluation,
    ) -> Result<(), DataError> {
        for fact in preflight_compact_audit_facts(evaluation) {
            self.store
                .append_job_compact_audit(&AppendMediaJobCompactAuditInput {
                    media_job_public_id,
                    audit_index: fact.audit_index,
                    fact_kind: fact.fact_kind,
                    fact_text: &fact.fact_text,
                })
                .await?;
        }
        Ok(())
    }

    async fn append_phase(
        &self,
        media_job_public_id: Uuid,
        phase_index: i32,
        phase_name: &str,
        phase_status: &str,
        details_text: Option<&str>,
    ) -> Result<(), DataError> {
        self.store
            .append_job_phase(
                media_job_public_id,
                phase_index,
                phase_name,
                phase_status,
                details_text,
            )
            .await
    }

    async fn append_verification_check(
        &self,
        input: &AppendMediaJobVerificationCheckInput<'_>,
    ) -> Result<(), DataError> {
        self.store.append_job_verification_check(input).await
    }

    async fn persist_failure(&self, job: &ClaimedMediaJobRow, error: MediaJobRuntimeError) {
        let code = error.code();
        if let Err(phase_error) = self
            .append_phase(
                job.media_job_public_id,
                FAILURE_PHASE_INDEX,
                "runtime_failure",
                "failed",
                Some(code),
            )
            .await
        {
            warn!(media_job_public_id = %job.media_job_public_id, error = %phase_error, "media job runtime failed to persist failure phase");
        }
        if let Err(check_error) = self
            .append_verification_check(&AppendMediaJobVerificationCheckInput {
                media_job_public_id: job.media_job_public_id,
                check_index: FAILURE_CHECK_INDEX,
                check_kind: "runtime_failure",
                check_status: "failed",
                expected_value: Some("success"),
                actual_value: Some(code),
                details_text: Some(code),
            })
            .await
        {
            warn!(media_job_public_id = %job.media_job_public_id, error = %check_error, "media job runtime failed to persist failure check");
        }
        if let Err(mark_error) = self.mark_failed(job, code).await {
            warn!(media_job_public_id = %job.media_job_public_id, error = %mark_error, "media job runtime failed to mark job failed");
        }
    }

    async fn mark_failed(&self, job: &ClaimedMediaJobRow, detail: &str) -> Result<(), DataError> {
        self.store
            .mark_job_status(job.media_job_public_id, "failed", Some(detail))
            .await
    }
}

#[derive(Debug, Error)]
enum MediaJobRuntimeError {
    #[error("media job runtime data error: {0}")]
    Data(#[from] DataError),
    #[error("media job runtime join error: {0}")]
    Join(String),
    #[error("media job runtime inspect error: {0}")]
    Inspect(String),
    #[error("media job runtime source metadata failed for {path}: {source}")]
    SourceMetadata {
        path: String,
        source: std::io::Error,
    },
    #[error("media job runtime execute error: {0}")]
    Execute(ExecuteSequenceError),
    #[error("media job runtime execute step error: {0}")]
    ExecuteStep(ExecuteStepError),
    #[error("media job runtime workspace error: {0}")]
    Workspace(ManagedWorkspaceError),
    #[error("media job runtime capacity probe error: {0}")]
    Capacity(String),
    #[error("media job runtime invalid path: {0}")]
    InvalidPath(&'static str),
    #[error("media job runtime invalid desired graph: {0}")]
    InvalidDesiredGraph(&'static str),
    #[error("media job runtime verification failed: {0}")]
    Verification(&'static str),
    #[error("media job runtime capability invalid: {0}")]
    InvalidCapability(&'static str),
    #[error("media job runtime index too large: {0}")]
    IndexTooLarge(&'static str),
}

impl MediaJobRuntimeError {
    const fn code(&self) -> &'static str {
        match self {
            Self::Data(_) => "media_job_runtime_storage_failed",
            Self::Join(_) => "media_job_runtime_join_failed",
            Self::Inspect(_) => "media_job_runtime_inspect_failed",
            Self::SourceMetadata { .. } => "media_job_runtime_source_metadata_failed",
            Self::Execute(_) | Self::ExecuteStep(_) => "media_job_runtime_execute_failed",
            Self::Workspace(_) => "media_job_runtime_workspace_failed",
            Self::Capacity(_) => "media_job_runtime_capacity_probe_failed",
            Self::InvalidPath(code)
            | Self::InvalidDesiredGraph(code)
            | Self::Verification(code)
            | Self::InvalidCapability(code)
            | Self::IndexTooLarge(code) => code,
        }
    }
}

fn compile_desired_graph(
    source_graph: &MediaGraph,
    output_path: &str,
    target: Option<&DesiredGraphTarget>,
) -> DesiredGraph {
    let Some(target) = target else {
        return DesiredGraph {
            output_path: output_path.to_string(),
            streams: source_graph.streams.clone(),
        };
    };

    let selected_subtitle_ids = selected_subtitle_stream_ids(&source_graph.streams);
    let streams = source_graph
        .streams
        .iter()
        .filter_map(|stream| compile_desired_stream(stream, &selected_subtitle_ids, target))
        .collect::<Vec<_>>();
    DesiredGraph {
        output_path: output_path.to_string(),
        streams,
    }
}

fn compile_desired_stream(
    stream: &MediaStream,
    selected_subtitle_ids: &std::collections::BTreeSet<u32>,
    target: &DesiredGraphTarget,
) -> Option<MediaStream> {
    match stream.kind {
        StreamKind::Video => Some(MediaStream {
            codec: target.video_codec.clone(),
            ..stream.clone()
        }),
        StreamKind::Audio => Some(MediaStream {
            codec: target.audio_codec.clone(),
            ..stream.clone()
        }),
        StreamKind::Subtitle if target.subtitle_policy == SubtitlePolicy::All => {
            Some(stream.clone())
        }
        StreamKind::Subtitle if target.subtitle_policy == SubtitlePolicy::None => None,
        StreamKind::Subtitle if selected_subtitle_ids.contains(&stream.stream_id) => {
            Some(stream.clone())
        }
        StreamKind::Subtitle => None,
        StreamKind::Attachment | StreamKind::Chapter => Some(stream.clone()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubtitlePolicy {
    Selected,
    All,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DesiredGraphTarget {
    video_codec: String,
    audio_codec: String,
    subtitle_policy: SubtitlePolicy,
}

fn desired_target_from_job(
    job: &ClaimedMediaJobRow,
) -> Result<Option<DesiredGraphTarget>, MediaJobRuntimeError> {
    if job
        .compatibility_target_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Ok(None);
    }

    let video_codec = normalized_snapshot_field(
        job.target_video_codec.as_deref(),
        "media_job_target_snapshot_missing",
    )?;
    let audio_codec = normalized_snapshot_field(
        job.target_audio_codec.as_deref(),
        "media_job_target_snapshot_missing",
    )?;
    let subtitle_policy = subtitle_policy_from_snapshot(job.target_subtitle_policy.as_deref())?;
    Ok(Some(DesiredGraphTarget {
        video_codec,
        audio_codec,
        subtitle_policy,
    }))
}

fn normalized_snapshot_field(
    value: Option<&str>,
    error_code: &'static str,
) -> Result<String, MediaJobRuntimeError> {
    value
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_ascii_lowercase)
        .ok_or(MediaJobRuntimeError::InvalidDesiredGraph(error_code))
}

fn subtitle_policy_from_snapshot(
    value: Option<&str>,
) -> Result<SubtitlePolicy, MediaJobRuntimeError> {
    match normalized_snapshot_field(value, "media_job_target_snapshot_missing")?.as_str() {
        "selected" => Ok(SubtitlePolicy::Selected),
        "all" => Ok(SubtitlePolicy::All),
        "none" => Ok(SubtitlePolicy::None),
        _ => Err(MediaJobRuntimeError::InvalidDesiredGraph(
            "media_job_subtitle_policy_unknown",
        )),
    }
}

fn selected_subtitle_stream_ids(streams: &[MediaStream]) -> std::collections::BTreeSet<u32> {
    let subtitle_streams = streams
        .iter()
        .filter(|stream| stream.kind == StreamKind::Subtitle)
        .collect::<Vec<_>>();
    let mut selected = subtitle_streams
        .iter()
        .filter(|stream| {
            stream.dispositions.iter().any(|value| {
                let normalized = value.trim().to_ascii_lowercase();
                normalized == "default" || normalized == "forced"
            })
        })
        .map(|stream| stream.stream_id)
        .collect::<std::collections::BTreeSet<_>>();
    if !selected.is_empty() {
        return selected;
    }
    if let Some(stream) = subtitle_streams.iter().find(|stream| {
        stream
            .language
            .as_deref()
            .is_some_and(|language| language.eq_ignore_ascii_case("eng"))
    }) {
        selected.insert(stream.stream_id);
        return selected;
    }
    if let Some(stream) = subtitle_streams.first() {
        selected.insert(stream.stream_id);
    }
    selected
}

fn video_policy_from_policy_intent(
    policy_video_intent: Option<&str>,
) -> Result<VideoTranscodePolicy, MediaJobRuntimeError> {
    let normalized =
        normalized_snapshot_field(policy_video_intent, "media_job_policy_snapshot_missing")?;
    let intent = match normalized.as_str() {
        "general" => VideoTranscodeIntent::General,
        "anime" => VideoTranscodeIntent::Anime,
        "archival" => VideoTranscodeIntent::Archival,
        _ => {
            return Err(MediaJobRuntimeError::InvalidDesiredGraph(
                "media_job_policy_intent_unknown",
            ));
        }
    };
    Ok(VideoTranscodePolicy {
        intent,
        ..VideoTranscodePolicy::default()
    })
}

fn feature_names(
    features: &[revaer_data::media::capabilities::CapabilityFeatureRow],
    family: &str,
    supported: bool,
) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    for feature in features {
        if feature.supported == supported && feature.feature_family.eq_ignore_ascii_case(family) {
            names.insert(feature.feature_name.clone());
        }
    }
    names.into_iter().collect()
}

fn resolve_output_path(job: &ClaimedMediaJobRow) -> Result<String, MediaJobRuntimeError> {
    if !path_is_within_root(&job.source_path, &job.source_root) {
        return Err(MediaJobRuntimeError::InvalidPath(
            "media_job_source_path_outside_profile_root",
        ));
    }

    let output = match job.output_path.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => clean_absolute_path(value),
        _ => derive_profile_output_path(&job.source_path, &job.source_root, &job.output_root),
    };
    let Some(output) = output else {
        return Err(MediaJobRuntimeError::InvalidPath(
            "media_job_output_path_invalid",
        ));
    };
    if !path_is_within_root_path(&output, &job.output_root) {
        return Err(MediaJobRuntimeError::InvalidPath(
            "media_job_output_path_outside_profile_root",
        ));
    }
    output
        .to_str()
        .map(str::to_string)
        .ok_or(MediaJobRuntimeError::InvalidPath(
            "media_job_output_path_invalid",
        ))
}

fn resolve_workspace_output_path(
    final_output_path: &str,
    workspace: &revaer_media_runtime::workspace::ManagedWorkspace,
) -> Result<String, MediaJobRuntimeError> {
    let Some(file_name) = Path::new(final_output_path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(MediaJobRuntimeError::InvalidPath(
            "media_job_output_filename_invalid",
        ));
    };
    path_to_string(
        &workspace.output_path.join(file_name),
        "workspace_output_path",
    )
}

fn path_to_string(path: &Path, label: &'static str) -> Result<String, MediaJobRuntimeError> {
    path.to_str()
        .map(str::to_string)
        .ok_or(MediaJobRuntimeError::InvalidPath(label))
}

fn replacement_output_path(steps: &[ExecutionStep]) -> Result<String, MediaJobRuntimeError> {
    steps
        .iter()
        .find_map(|step| match step {
            ExecutionStep::AtomicReplace { output_path, .. } => Some(output_path.clone()),
            ExecutionStep::CopySidecarSubtitle { .. }
            | ExecutionStep::BackupSource { .. }
            | ExecutionStep::Command { .. }
            | ExecutionStep::VerifyOutput { .. }
            | ExecutionStep::QuarantineFailedOutput { .. } => None,
        })
        .ok_or(MediaJobRuntimeError::Verification(
            "media_job_atomic_replace_step_missing",
        ))
}

fn media_graph_matches_desired(actual: &MediaGraph, desired: &DesiredGraph) -> bool {
    actual.streams.len() == desired.streams.len()
        && actual
            .streams
            .iter()
            .zip(&desired.streams)
            .all(stream_matches_desired)
}

fn stream_matches_desired((actual_stream, desired_stream): (&MediaStream, &MediaStream)) -> bool {
    actual_stream.stream_id == desired_stream.stream_id
        && actual_stream.kind == desired_stream.kind
        && actual_stream
            .codec
            .trim()
            .eq_ignore_ascii_case(desired_stream.codec.trim())
        && normalized_language(actual_stream.language.as_deref())
            == normalized_language(desired_stream.language.as_deref())
        && normalized_optional_text(actual_stream.title.as_deref())
            == normalized_optional_text(desired_stream.title.as_deref())
        && normalized_dispositions(&actual_stream.dispositions)
            == normalized_dispositions(&desired_stream.dispositions)
}

fn normalized_language(value: Option<&str>) -> Option<String> {
    normalized_optional_text(value).map(|item| item.to_ascii_lowercase())
}

fn normalized_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

fn normalized_dispositions(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn path_is_within_root(path: &str, root: &str) -> bool {
    let Some(normalized_path) = clean_absolute_path(path) else {
        return false;
    };
    path_is_within_root_path(&normalized_path, root)
}

fn path_is_within_root_path(path: &Path, root: &str) -> bool {
    let Some(normalized_root) = clean_absolute_path(root) else {
        return false;
    };
    path == normalized_root.as_path() || path.starts_with(normalized_root)
}

fn clean_absolute_path(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let input = Path::new(trimmed);
    if !input.is_absolute() {
        return None;
    }

    let mut normalized = PathBuf::new();
    for component in input.components() {
        match component {
            Component::RootDir => normalized = PathBuf::from("/"),
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

fn derive_profile_output_path(
    source_path: &str,
    source_root: &str,
    output_root: &str,
) -> Option<PathBuf> {
    let normalized_source_path = clean_absolute_path(source_path)?;
    let normalized_source_root = clean_absolute_path(source_root)?;
    let normalized_output_root = clean_absolute_path(output_root)?;
    if normalized_source_path != normalized_source_root
        && !normalized_source_path.starts_with(&normalized_source_root)
    {
        return None;
    }
    if normalized_source_path == normalized_source_root {
        return Some(normalized_output_root);
    }

    let suffix = normalized_source_path
        .strip_prefix(&normalized_source_root)
        .ok()?;
    Some(normalized_output_root.join(suffix))
}

fn validate_existing_ancestor_bounds(
    path: &str,
    root: &str,
    label: &'static str,
) -> Result<(), MediaJobRuntimeError> {
    let Some(normalized_path) = clean_absolute_path(path) else {
        return Err(MediaJobRuntimeError::InvalidPath(label));
    };
    let Some(normalized_root) = clean_absolute_path(root) else {
        return Err(MediaJobRuntimeError::InvalidPath(label));
    };
    let Some(existing_path) = first_existing_ancestor(&normalized_path) else {
        return Err(MediaJobRuntimeError::InvalidPath(label));
    };
    let Some(existing_root) = first_existing_ancestor(&normalized_root) else {
        return Err(MediaJobRuntimeError::InvalidPath(label));
    };
    let canonical_path = existing_path
        .canonicalize()
        .map_err(|_| MediaJobRuntimeError::InvalidPath(label))?;
    let canonical_root = existing_root
        .canonicalize()
        .map_err(|_| MediaJobRuntimeError::InvalidPath(label))?;
    if canonical_path == canonical_root || canonical_path.starts_with(canonical_root) {
        Ok(())
    } else {
        Err(MediaJobRuntimeError::InvalidPath(label))
    }
}

fn first_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    loop {
        if current.exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

fn source_file_bytes(source_path: &str) -> Result<u64, MediaJobRuntimeError> {
    fs::metadata(source_path)
        .map(|metadata| metadata.len())
        .map_err(|source| MediaJobRuntimeError::SourceMetadata {
            path: source_path.to_string(),
            source,
        })
}

const fn operation_kind_code(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Remux => "remux",
        OperationKind::MetadataRewrite => "metadata_rewrite",
        OperationKind::DispositionRewrite => "disposition_rewrite",
        OperationKind::LabelRewrite => "label_rewrite",
        OperationKind::StreamReorder => "stream_reorder",
        OperationKind::AudioTranscode => "audio_transcode",
        OperationKind::VideoTranscode => "video_transcode",
    }
}

fn stream_id_to_i32(operation: &PlannedOperation) -> Result<Option<i32>, MediaJobRuntimeError> {
    operation
        .stream_id
        .map(i32::try_from)
        .transpose()
        .map_err(index_error("stream_id"))
}

fn usize_to_i32(value: usize, label: &'static str) -> Result<i32, MediaJobRuntimeError> {
    i32::try_from(value).map_err(index_error(label))
}

fn index_error(label: &'static str) -> impl FnOnce(TryFromIntError) -> MediaJobRuntimeError {
    move |_| MediaJobRuntimeError::IndexTooLarge(label)
}

fn command_fields_for_operation(
    steps: &[ExecutionStep],
    operation_index: usize,
) -> (&str, [Option<&str>; 5]) {
    let selected = steps
        .iter()
        .filter_map(command_step)
        .nth(operation_index)
        .or_else(|| steps.iter().find_map(command_step));
    selected.map_or(
        ("filesystem", [None, None, None, None, None]),
        |(bin, argv)| {
            (
                bin.as_str(),
                [
                    argv.first().map(String::as_str),
                    argv.get(1).map(String::as_str),
                    argv.get(2).map(String::as_str),
                    argv.get(3).map(String::as_str),
                    argv.get(4).map(String::as_str),
                ],
            )
        },
    )
}

const fn command_step(step: &ExecutionStep) -> Option<(&String, &Vec<String>)> {
    match step {
        ExecutionStep::Command { bin, argv } => Some((bin, argv)),
        ExecutionStep::CopySidecarSubtitle { .. }
        | ExecutionStep::BackupSource { .. }
        | ExecutionStep::VerifyOutput { .. }
        | ExecutionStep::QuarantineFailedOutput { .. }
        | ExecutionStep::AtomicReplace { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FilesystemCapacityProbe, MediaJobRuntime, RuntimeCapacityProbe, RuntimeCommandRunner,
        RuntimeInspector,
    };
    use revaer_data::indexers::app_users::{app_user_create, app_user_verify_email};
    use revaer_data::media::capabilities::{
        RecordCapabilityEncoderInput, RecordCapabilityFeatureInput, RecordCapabilitySnapshotInput,
        complete_capability_snapshot_run_with_executor, record_capability_encoder_with_executor,
        record_capability_feature_with_executor, record_capability_snapshot_with_executor,
        start_capability_snapshot_run_with_executor,
    };
    use revaer_data::media::jobs::{ClaimedMediaJobRow, CreateMediaJobInput};
    use revaer_data::media::profiles::{UpdateMediaProfileInput, UpsertMediaProfileInput};
    use revaer_media_core::compliance::{Status, report_for_status};
    use revaer_media_core::explain::Explanation;
    use revaer_media_core::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};
    use revaer_media_core::plan::{OperationKind, PlannedOperation};
    use revaer_media_runtime::execute::{CommandRunner, ExecuteStepError, ExecutionStep};
    use revaer_media_runtime::inspect::{InspectAdapter, InspectError};
    use revaer_media_runtime::jobs::{JobPreflightReport, PlannedJob, PlannedJobSummary};
    use revaer_media_runtime::workspace::{
        WorkspaceCapacityReport, WorkspacePolicy, WorkspaceRejectionReason,
    };
    use revaer_runtime::media::MediaStore;
    use revaer_test_support::postgres::TestDatabase;
    use revaer_test_support::postgres::start_postgres;
    use sqlx::postgres::PgPoolOptions;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[derive(Clone)]
    struct StaticInspector;

    impl InspectAdapter for StaticInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            Ok(video_graph(source_path, "h264"))
        }
    }

    #[derive(Clone)]
    struct CandidateMismatchInspector;

    impl InspectAdapter for CandidateMismatchInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            if source_path.contains("/workspace/") {
                Ok(video_graph(source_path, "vp9"))
            } else {
                Ok(video_graph(source_path, "h264"))
            }
        }
    }

    #[derive(Default)]
    struct RecordingCommandRunner {
        commands: Mutex<Vec<Vec<String>>>,
    }

    #[derive(Clone, Copy)]
    struct StaticCapacityProbe {
        available_bytes: u64,
    }

    impl FilesystemCapacityProbe for StaticCapacityProbe {
        fn available_bytes(&self, _path: &Path) -> Result<u64, String> {
            Ok(self.available_bytes)
        }
    }

    impl CommandRunner for RecordingCommandRunner {
        fn run(&self, bin: &str, argv: &[String]) -> Result<(), ExecuteStepError> {
            let mut row = Vec::with_capacity(argv.len() + 1);
            row.push(bin.to_string());
            row.extend(argv.iter().cloned());
            match self.commands.lock() {
                Ok(mut commands) => commands.push(row),
                Err(error) => {
                    return Err(ExecuteStepError::CommandFailed {
                        bin: format!("mutex_poisoned:{error}"),
                        status_code: None,
                    });
                }
            }
            if let Some(output_path) = argv.last() {
                fs::write(output_path, b"output").map_err(|source| ExecuteStepError::Io {
                    operation: "test.output_write",
                    path: PathBuf::from(output_path),
                    source,
                })?;
            }
            Ok(())
        }
    }

    struct RuntimeFixture {
        _postgres: TestDatabase,
        _temp: TempDir,
        runtime: MediaJobRuntime,
        store: MediaStore,
        job_id: Uuid,
        command_runner: Arc<RecordingCommandRunner>,
    }

    fn video_graph(source_path: &str, codec: &str) -> MediaGraph {
        MediaGraph {
            source_path: source_path.to_string(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: codec.to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        }
    }

    async fn setup_runtime(
        dry_run: bool,
        record_capability: bool,
    ) -> anyhow::Result<Option<RuntimeFixture>> {
        let Ok(postgres) = start_postgres() else {
            return Ok(None);
        };
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(postgres.connection_string())
            .await?;
        let mut migrator = sqlx::migrate!("../revaer-data/migrations");
        migrator.set_ignore_missing(true);
        migrator.run(&pool).await?;
        let store = MediaStore::new(pool);

        let temp = tempfile::tempdir()?;
        let input_root = temp.path().join("input");
        let output_root = temp.path().join("output");
        let workspace_root = temp.path().join("workspace");
        fs::create_dir_all(&input_root)?;
        fs::create_dir_all(&output_root)?;
        let source_path = input_root.join("movie.mkv");
        let output_path = output_root.join("movie.mkv");
        let input_root_text = input_root.to_string_lossy().to_string();
        let output_root_text = output_root.to_string_lossy().to_string();
        let source_path_text = source_path.to_string_lossy().to_string();
        let output_path_text = output_path.to_string_lossy().to_string();
        fs::write(&source_path, b"source")?;

        let email = format!("media-worker-{}@example.invalid", Uuid::new_v4());
        let actor = app_user_create(store.pool(), &email, "Media Worker").await?;
        app_user_verify_email(store.pool(), actor).await?;
        let profile_id = store
            .upsert_profile(&UpsertMediaProfileInput {
                actor_public_id: actor,
                profile_key: "worker",
                source_root: &input_root_text,
                output_root: &output_root_text,
                dry_run_only: dry_run,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            })
            .await?;
        if !dry_run {
            store
                .update_profile(&UpdateMediaProfileInput {
                    actor_public_id: actor,
                    media_profile_public_id: profile_id,
                    source_root: None,
                    output_root: None,
                    dry_run_only: Some(false),
                    retention_days: None,
                    compatibility_target_key: None,
                    policy_key: None,
                    watcher_enabled: None,
                    schedule_enabled: None,
                    schedule_interval_minutes: None,
                })
                .await?;
        }
        let job_id = store
            .create_job(&CreateMediaJobInput {
                actor_public_id: actor,
                media_profile_public_id: profile_id,
                source_path: &source_path_text,
                output_path: Some(&output_path_text),
                dry_run,
            })
            .await?;
        if record_capability {
            record_runtime_capability(&store, actor).await?;
        }

        let command_runner = Arc::new(RecordingCommandRunner::default());
        let runtime = MediaJobRuntime::with_components(
            store.clone(),
            Arc::new(StaticInspector) as Arc<RuntimeInspector>,
            Arc::clone(&command_runner) as Arc<RuntimeCommandRunner>,
            Arc::new(StaticCapacityProbe {
                available_bytes: 1024 * 1024 * 1024,
            }) as Arc<RuntimeCapacityProbe>,
            Duration::from_secs(60),
            WorkspacePolicy {
                max_bytes: 1024 * 1024 * 1024,
                reserve_bytes: 1024,
            },
            workspace_root,
        );
        Ok(Some(RuntimeFixture {
            _postgres: postgres,
            _temp: temp,
            runtime,
            store,
            job_id,
            command_runner,
        }))
    }

    async fn record_runtime_capability(store: &MediaStore, actor: Uuid) -> anyhow::Result<()> {
        let snapshot_run_public_id = Uuid::new_v4();
        let mut transaction = store.pool().begin().await?;
        start_capability_snapshot_run_with_executor(
            &mut *transaction,
            actor,
            snapshot_run_public_id,
        )
        .await?;
        record_capability_snapshot_with_executor(
            &mut *transaction,
            &RecordCapabilitySnapshotInput {
                actor_public_id: actor,
                snapshot_run_public_id: Some(snapshot_run_public_id),
                ffmpeg_version: "7.1",
                ffprobe_version: "7.1",
                codec_name: "h264",
                encode_supported: true,
                decode_supported: true,
            },
        )
        .await?;
        record_capability_encoder_with_executor(
            &mut *transaction,
            &RecordCapabilityEncoderInput {
                actor_public_id: actor,
                snapshot_run_public_id,
                encoder_name: "libx265",
            },
        )
        .await?;
        record_capability_encoder_with_executor(
            &mut *transaction,
            &RecordCapabilityEncoderInput {
                actor_public_id: actor,
                snapshot_run_public_id,
                encoder_name: "aac",
            },
        )
        .await?;
        for (feature_family, feature_name, supported) in [
            ("decoder", "h264", true),
            ("muxer", "matroska", true),
            ("demuxer", "matroska", true),
            ("filesystem", "atomic_rename", true),
            ("utility", "ffmpeg", true),
            ("license", "gpl", true),
            ("absent", "--enable-nonfree", false),
        ] {
            record_capability_feature_with_executor(
                &mut *transaction,
                &RecordCapabilityFeatureInput {
                    actor_public_id: actor,
                    snapshot_run_public_id,
                    feature_family,
                    feature_name,
                    supported,
                    detail_text: None,
                },
            )
            .await?;
        }
        complete_capability_snapshot_run_with_executor(&mut *transaction, snapshot_run_public_id)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_completes_dry_run_without_command_execution() -> anyhow::Result<()> {
        let Some(fixture) = setup_runtime(true, true).await? else {
            return Ok(());
        };

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "completed");
        assert_eq!(
            fixture
                .store
                .list_job_operations(fixture.job_id)
                .await?
                .len(),
            1
        );
        assert!(
            !fixture
                .store
                .list_job_compact_audits(fixture.job_id)
                .await?
                .is_empty()
        );
        assert!(
            !fixture
                .store
                .list_job_verification_checks(fixture.job_id)
                .await?
                .is_empty()
        );
        let command_count = {
            let commands = fixture
                .command_runner
                .commands
                .lock()
                .map_err(|error| anyhow::anyhow!("command lock poisoned: {error}"))?;
            commands.len()
        };
        assert_eq!(command_count, 0);
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_executes_non_dry_run_with_injected_runner() -> anyhow::Result<()> {
        let Some(fixture) = setup_runtime(false, true).await? else {
            return Ok(());
        };

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "completed");
        assert_eq!(job.last_error, None);
        assert!(
            fixture
                .store
                .list_job_verification_checks(fixture.job_id)
                .await?
                .iter()
                .any(|check| check.check_kind == "output_replacement"
                    && check.check_status == "passed")
        );
        assert!(
            fixture
                .store
                .list_job_verification_checks(fixture.job_id)
                .await?
                .iter()
                .any(
                    |check| check.check_kind == "candidate_graph" && check.check_status == "passed"
                )
        );
        assert!(
            fixture
                .store
                .list_job_verification_checks(fixture.job_id)
                .await?
                .iter()
                .any(|check| check.check_kind == "final_graph" && check.check_status == "passed")
        );
        let command_count = {
            let commands = fixture
                .command_runner
                .commands
                .lock()
                .map_err(|error| anyhow::anyhow!("command lock poisoned: {error}"))?;
            let first_command = commands
                .first()
                .ok_or_else(|| anyhow::anyhow!("command missing"))?;
            let candidate_path = first_command
                .last()
                .ok_or_else(|| anyhow::anyhow!("command output missing"))?;
            assert!(Path::new(candidate_path).starts_with(&fixture.runtime.workspace_root));
            commands.len()
        };
        assert_eq!(command_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_rejects_low_workspace_capacity_before_execution()
    -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true).await? else {
            return Ok(());
        };
        fixture.runtime.capacity_probe =
            Arc::new(StaticCapacityProbe { available_bytes: 0 }) as Arc<RuntimeCapacityProbe>;

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("preflight_workspace_insufficient_reserve")
        );
        let command_count = {
            let commands = fixture
                .command_runner
                .commands
                .lock()
                .map_err(|error| anyhow::anyhow!("command lock poisoned: {error}"))?;
            commands.len()
        };
        assert_eq!(command_count, 0);
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_quarantines_mismatched_candidate_before_replacement()
    -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true).await? else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(CandidateMismatchInspector) as Arc<RuntimeInspector>;

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("media_job_output_graph_mismatch")
        );
        assert_eq!(fs::read(&job.source_path)?, b"source");
        let checks = fixture
            .store
            .list_job_verification_checks(fixture.job_id)
            .await?;
        assert!(
            checks.iter().any(
                |check| check.check_kind == "candidate_graph" && check.check_status == "failed"
            )
        );
        assert!(checks.iter().all(|check| check.check_kind != "final_graph"));
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_marks_missing_capability_failed() -> anyhow::Result<()> {
        let Some(fixture) = setup_runtime(true, false).await? else {
            return Ok(());
        };

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("media_capability_snapshot_missing")
        );
        assert!(
            fixture
                .store
                .list_job_verification_checks(fixture.job_id)
                .await?
                .iter()
                .any(|check| check.check_kind == "runtime_failure"
                    && check.actual_value.as_deref() == Some("media_capability_snapshot_missing"))
        );
        Ok(())
    }

    #[test]
    fn desired_graph_compiles_from_snapshotted_target_and_selected_subtitles() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                    codec: "dts".to_string(),
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 2,
                    kind: StreamKind::Subtitle,
                    codec: "subrip".to_string(),
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 3,
                    kind: StreamKind::Subtitle,
                    codec: "ass".to_string(),
                    language: Some("jpn".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };

        let target = super::DesiredGraphTarget {
            video_codec: "hevc".to_string(),
            audio_codec: "aac".to_string(),
            subtitle_policy: super::SubtitlePolicy::Selected,
        };
        let desired = super::compile_desired_graph(&source, "/workspace/movie.mkv", Some(&target));
        assert_eq!(
            desired
                .streams
                .iter()
                .map(|stream| (stream.stream_id, stream.kind, stream.codec.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (0, StreamKind::Video, "hevc"),
                (1, StreamKind::Audio, "aac"),
                (2, StreamKind::Subtitle, "subrip"),
            ]
        );
        let diff = revaer_media_core::diff::diff_graphs(&source, &desired);
        let operations = revaer_media_core::plan::generate_plan(&diff);
        assert_eq!(
            operations,
            vec![
                PlannedOperation {
                    kind: OperationKind::VideoTranscode,
                    stream_id: Some(0),
                },
                PlannedOperation {
                    kind: OperationKind::AudioTranscode,
                    stream_id: Some(1),
                },
                PlannedOperation {
                    kind: OperationKind::Remux,
                    stream_id: None,
                },
            ]
        );
    }

    #[test]
    fn final_graph_verification_matches_stream_identity_and_metadata() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 2,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                language: Some("eng".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["default".to_string(), "forced".to_string()],
            }],
        };
        let matching = MediaGraph {
            source_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 2,
                kind: StreamKind::Audio,
                codec: "AAC".to_string(),
                language: Some("ENG".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["forced".to_string(), "default".to_string()],
            }],
        };
        assert!(super::media_graph_matches_desired(&matching, &desired));

        let mismatched = MediaGraph {
            source_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 2,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                language: Some("jpn".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["default".to_string(), "forced".to_string()],
            }],
        };
        assert!(!super::media_graph_matches_desired(&mismatched, &desired));
    }

    #[test]
    fn path_helpers_normalize_derive_and_validate_bounds() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let input_root = temp.path().join("input");
        let output_root = temp.path().join("output");
        let sibling_root = temp.path().join("input-sibling");
        fs::create_dir_all(input_root.join("nested"))?;
        fs::create_dir_all(&output_root)?;
        fs::create_dir_all(&sibling_root)?;

        let source_path = input_root.join("nested").join("movie.mkv");
        fs::write(&source_path, b"source")?;
        let source_text = source_path.to_string_lossy().to_string();
        let input_text = input_root.to_string_lossy().to_string();
        let output_text = output_root.to_string_lossy().to_string();
        let sibling_text = sibling_root.to_string_lossy().to_string();

        let derived = super::derive_profile_output_path(&source_text, &input_text, &output_text)
            .ok_or_else(|| anyhow::anyhow!("derived path missing"))?;
        assert_eq!(derived, output_root.join("nested").join("movie.mkv"));
        assert!(super::path_is_within_root(&source_text, &input_text));
        assert!(!super::path_is_within_root(&source_text, &sibling_text));

        let job = claimed_job_with_paths(&source_text, None, true, &input_text, &output_text);
        assert_eq!(
            super::resolve_output_path(&job).ok().as_deref(),
            Some(derived.to_string_lossy().as_ref())
        );

        let outside_output = sibling_root.join("movie.mkv").to_string_lossy().to_string();
        let outside_job = claimed_job_with_paths(
            &source_text,
            Some(outside_output),
            true,
            &input_text,
            &output_text,
        );
        assert_eq!(
            super::resolve_output_path(&outside_job)
                .err()
                .map(|error| error.code()),
            Some("media_job_output_path_outside_profile_root")
        );
        assert!(
            super::validate_existing_ancestor_bounds(&source_text, &input_text, "source_path")
                .is_ok()
        );
        assert_eq!(
            super::validate_existing_ancestor_bounds(&source_text, &sibling_text, "source_path")
                .err()
                .map(|error| error.code()),
            Some("source_path")
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn ancestor_validation_rejects_symlink_escape() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("root");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&root)?;
        fs::create_dir_all(&outside)?;
        let link_path = root.join("link");
        std::os::unix::fs::symlink(&outside, &link_path)?;
        let escaped_path = link_path.join("movie.mkv").to_string_lossy().to_string();
        let root_text = root.to_string_lossy().to_string();

        assert_eq!(
            super::validate_existing_ancestor_bounds(&escaped_path, &root_text, "output_path")
                .err()
                .map(|error| error.code()),
            Some("output_path")
        );
        Ok(())
    }

    #[test]
    fn command_and_error_helpers_cover_runtime_variants() {
        let steps = vec![
            ExecutionStep::AtomicReplace {
                source_path: "/media/in.mkv".to_string(),
                output_path: "/media/out.mkv".to_string(),
            },
            ExecutionStep::Command {
                bin: "ffmpeg".to_string(),
                argv: vec![
                    "-i".to_string(),
                    "/media/in.mkv".to_string(),
                    "/media/out.mkv".to_string(),
                    "ignored".to_string(),
                    "extra".to_string(),
                    "truncated".to_string(),
                ],
            },
        ];
        let (bin, args) = super::command_fields_for_operation(&steps, 1);
        assert_eq!(bin, "ffmpeg");
        assert_eq!(args[0], Some("-i"));
        assert_eq!(args[4], Some("extra"));
        let (fallback_bin, fallback_args) = super::command_fields_for_operation(&steps, 5);
        assert_eq!(fallback_bin, "ffmpeg");
        assert_eq!(fallback_args[0], Some("-i"));
        let (filesystem_bin, filesystem_args) = super::command_fields_for_operation(&steps[..1], 0);
        assert_eq!(filesystem_bin, "filesystem");
        assert_eq!(filesystem_args, [None, None, None, None, None]);

        assert_eq!(
            super::operation_kind_code(OperationKind::MetadataRewrite),
            "metadata_rewrite"
        );
        assert_eq!(
            super::operation_kind_code(OperationKind::DispositionRewrite),
            "disposition_rewrite"
        );
        assert_eq!(
            super::operation_kind_code(OperationKind::LabelRewrite),
            "label_rewrite"
        );
        assert_eq!(
            super::operation_kind_code(OperationKind::StreamReorder),
            "stream_reorder"
        );
        assert_eq!(
            super::operation_kind_code(OperationKind::AudioTranscode),
            "audio_transcode"
        );
        assert_eq!(
            super::operation_kind_code(OperationKind::VideoTranscode),
            "video_transcode"
        );
        assert_eq!(
            super::stream_id_to_i32(&PlannedOperation {
                kind: OperationKind::AudioTranscode,
                stream_id: Some(7),
            })
            .ok(),
            Some(Some(7))
        );
        assert_eq!(
            super::usize_to_i32(usize::MAX, "huge_index")
                .err()
                .map(|error| error.code()),
            Some("huge_index")
        );
        assert_eq!(
            super::source_file_bytes("/definitely/missing/revaer/media.mkv")
                .err()
                .map(|error| error.code()),
            Some("media_job_runtime_source_metadata_failed")
        );
        assert_eq!(
            super::MediaJobRuntimeError::InvalidCapability("capability_bad").code(),
            "capability_bad"
        );
    }

    #[tokio::test]
    async fn persist_ready_plan_records_filesystem_fallback_operations() -> anyhow::Result<()> {
        let Some(fixture) = setup_runtime(true, true).await? else {
            return Ok(());
        };
        let source = MediaGraph {
            source_path: "/media/in.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/media/out.mkv".to_string(),
            streams: source.streams.clone(),
        };
        let report = JobPreflightReport {
            planned: PlannedJob {
                source: Box::new(source),
                desired: Box::new(desired),
                operations: vec![PlannedOperation {
                    kind: OperationKind::MetadataRewrite,
                    stream_id: None,
                }],
                compliance: report_for_status(Status::DryRunPlanned),
                estimated_workspace_bytes: 1,
            },
            summary: PlannedJobSummary {
                total_operations: 1,
                remux_operations: 0,
                metadata_rewrite_operations: 1,
                disposition_rewrite_operations: 0,
                label_rewrite_operations: 0,
                stream_reorder_operations: 0,
                audio_transcode_operations: 0,
                video_transcode_operations: 0,
                explanations: vec![Explanation {
                    message: "metadata rewrite selected".to_string(),
                }],
            },
            steps: vec![ExecutionStep::AtomicReplace {
                source_path: "/media/in.mkv".to_string(),
                output_path: "/media/out.mkv".to_string(),
            }],
            timeline: Vec::new(),
            capacity_report: WorkspaceCapacityReport {
                accepted: true,
                reason: Some(WorkspaceRejectionReason::InvalidPolicy),
                available_after_reserve_bytes: 1,
                required_workspace_bytes: 1,
            },
        };

        fixture
            .runtime
            .persist_ready_plan(fixture.job_id, &report)
            .await?;
        let operations = fixture.store.list_job_operations(fixture.job_id).await?;
        assert!(
            operations
                .iter()
                .any(|operation| operation.command_bin == "filesystem"
                    && operation.operation_kind == "metadata_rewrite")
        );
        Ok(())
    }

    fn claimed_job_with_paths(
        source_path: &str,
        output_path: Option<String>,
        dry_run: bool,
        source_root: &str,
        output_root: &str,
    ) -> ClaimedMediaJobRow {
        ClaimedMediaJobRow {
            media_job_public_id: Uuid::new_v4(),
            media_profile_public_id: Uuid::new_v4(),
            source_path: source_path.to_string(),
            output_path,
            dry_run,
            source_root: source_root.to_string(),
            output_root: output_root.to_string(),
            compatibility_target_key: None,
            policy_key: "safe_dry_run".to_string(),
            target_video_codec: None,
            target_audio_codec: None,
            target_subtitle_policy: None,
            policy_video_intent: Some("general".to_string()),
        }
    }
}
