//! In-process media job runtime.
//!
//! # Design
//! - Claims queued media jobs through stored procedures.
//! - Persists phase, operation, verification, and compact-audit rows before terminal status.
//! - Keeps runtime adapters injected so tests avoid real `ffmpeg` execution.

use std::collections::BTreeSet;
use std::fs;
use std::num::TryFromIntError;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use revaer_data::DataError;
use revaer_data::media::jobs::{
    AppendMediaJobCompactAuditInput, AppendMediaJobVerificationCheckInput, ClaimedMediaJobRow,
    MediaJobDesiredTargetStreamRow,
};
use revaer_events::{Event, EventBus};
use revaer_media_core::classify::SemanticRole;
use revaer_media_core::model::DesiredGraph;
use revaer_media_core::model::{MediaGraph, MediaStream, StreamKind};
use revaer_media_core::normalize::normalize_container_format;
use revaer_media_core::plan::{OperationKind, PlannedOperation};
use revaer_media_core::target::{
    CompiledDesiredTarget, DesiredSidecarOutput, DesiredTarget, ImageSubtitleAction,
    SidecarSubtitleInput, SubtitlePlacement, TargetStream, UnmatchedStreamPolicy,
    compile_desired_target_with_sidecars_at,
};
use revaer_media_runtime::capabilities::{CapabilitySnapshot, CodecCapability};
use revaer_media_runtime::execute::{
    AudioStreamConstraints, CommandRunner, ExecuteSequenceError, ExecuteStepError,
    ExecutionControl, ExecutionStep, ProcessCommandRunner, VideoStreamConstraints,
    VideoTranscodeIntent, VideoTranscodePolicy, execute_filesystem_step,
    execute_step_sequence_controlled,
};
use revaer_media_runtime::inspect::{
    ChapterInspection, FfprobeInspectAdapter, InspectAdapter, MediaInspection, MetadataEntry,
    StreamInspection, SystemInspectProbeExecutor,
};
use revaer_media_runtime::jobs::{
    JobPreflightEvaluation, JobPreflightReport, PreflightBuildTemplate, PreflightPolicyInput,
    build_preflight_input, evaluate_preflight_from_compiled_target, preflight_compact_audit_facts,
};
use revaer_media_runtime::replacement::{
    CommittedReplacement, ReplacementArtifactRequest, ReplacementBundleRequest,
    ReplacementCommitter, ReplacementError, ReplacementRecoveryAction, SystemReplacementCommitter,
};
use revaer_media_runtime::sidecar::{SidecarFormat, SidecarRole, SidecarSubtitle};
use revaer_media_runtime::verification::{
    SystemVerificationExecutor, VerificationChecks, VerificationExecutionError,
    VerificationExecutor, VerificationPolicy, VerificationReport, verify_candidate_controlled,
};
use revaer_media_runtime::workspace::{
    ManagedWorkspaceError, TerminalWorkspaceCleanupPolicy, TerminalWorkspaceState, WorkspacePolicy,
    cleanup_terminal_workspace, create_managed_workspace,
};
use revaer_runtime::media::MediaStore;
use revaer_telemetry::Metrics;
use thiserror::Error;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval, sleep};
use tracing::{info, warn};
use uuid::Uuid;

const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(1);
const DEFAULT_WORKSPACE_MAX_BYTES: u64 = 20 * 1024 * 1024 * 1024;
const DEFAULT_WORKSPACE_RESERVE_BYTES: u64 = 1024 * 1024 * 1024;
const DEFAULT_WORKSPACE_ROOT_NAME: &str = "revaer-media-runtime";
const FAILURE_PHASE_INDEX: i32 = 99;
const FAILURE_CHECK_INDEX: i32 = 99;
const CANCELLATION_PHASE_INDEX: i32 = 98;
const CANCELLATION_CHECK_INDEX: i32 = 98;
const CONTROL_POLL_INTERVAL: Duration = Duration::from_millis(250);
const DIALOG_NORMALIZED_TARGET_LUFS: f64 = -16.0;
const DIALOG_NORMALIZED_LUFS_TOLERANCE: f64 = 1.0;
const DIALOG_NORMALIZED_TRUE_PEAK_MAX_DBFS: f64 = -1.0;
const SPEECH_DYNAMIC_RANGE_MAX_LU: f64 = 12.0;

type RuntimeInspector = dyn InspectAdapter + Send + Sync;
type RuntimeCommandRunner = dyn CommandRunner + Send + Sync;
type RuntimeCapacityProbe = dyn FilesystemCapacityProbe + Send + Sync;
type RuntimeReplacementCommitter = dyn ReplacementCommitter + Send + Sync;
type RuntimeVerificationExecutor = dyn VerificationExecutor + Send + Sync;
type RuntimeAudioAnalyzer = dyn AudioAnalysisAdapter + Send + Sync;

struct MediaJobRuntimeComponents {
    inspector: Arc<RuntimeInspector>,
    command_runner: Arc<RuntimeCommandRunner>,
    capacity_probe: Arc<RuntimeCapacityProbe>,
    replacement_committer: Arc<RuntimeReplacementCommitter>,
    verification_executor: Arc<RuntimeVerificationExecutor>,
    audio_analyzer: Arc<RuntimeAudioAnalyzer>,
    events: EventBus,
    telemetry: Metrics,
    tick_interval: Duration,
    workspace_policy: WorkspacePolicy,
    workspace_root: PathBuf,
}

struct RuntimePreflightEvaluation {
    evaluation: JobPreflightEvaluation,
    desired: DesiredGraph,
    desired_target: Option<DesiredTargetSnapshot>,
    expected_chapters: Vec<ChapterInspection>,
}

struct DesiredVerificationContext<'a> {
    desired: &'a DesiredGraph,
    desired_target: Option<&'a DesiredTargetSnapshot>,
    expected_chapters: &'a [ChapterInspection],
}

struct SidecarPublicationContext<'a> {
    outputs: &'a [DesiredSidecarOutput],
    removals: &'a [String],
}

#[derive(Debug, Default)]
struct CancellationSignal {
    requested: AtomicBool,
}

impl CancellationSignal {
    fn request(&self) {
        self.requested.store(true, Ordering::Release);
    }
}

impl ExecutionControl for CancellationSignal {
    fn cancellation_requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct AudioMeasurement {
    integrated_lufs: f64,
    loudness_range_lu: f64,
    true_peak_dbfs: Option<f64>,
}

trait AudioAnalysisAdapter {
    fn measure(&self, source_path: &str, stream_id: u32) -> Result<AudioMeasurement, String>;
}

#[derive(Debug, Clone)]
struct SystemFfmpegAudioAnalysisAdapter {
    ffmpeg_bin: String,
}

impl Default for SystemFfmpegAudioAnalysisAdapter {
    fn default() -> Self {
        Self {
            ffmpeg_bin: "ffmpeg".to_string(),
        }
    }
}

impl AudioAnalysisAdapter for SystemFfmpegAudioAnalysisAdapter {
    fn measure(&self, source_path: &str, stream_id: u32) -> Result<AudioMeasurement, String> {
        let args = vec![
            "-hide_banner".to_string(),
            "-nostats".to_string(),
            "-nostdin".to_string(),
            "-i".to_string(),
            source_path.to_string(),
            "-map".to_string(),
            format!("0:{stream_id}"),
            "-filter:a".to_string(),
            "ebur128=peak=true".to_string(),
            "-f".to_string(),
            "null".to_string(),
            "-".to_string(),
        ];
        let output = Command::new(&self.ffmpeg_bin)
            .args(&args)
            .output()
            .map_err(|error| format!("audio analyzer command spawn failed: {error}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let detail = if stderr.is_empty() {
                format!("status {}", output.status)
            } else {
                format!("status {}: {stderr}", output.status)
            };
            return Err(format!("audio analyzer command failed: {detail}"));
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        parse_ebur128_summary(&stderr)
    }
}

/// Runtime worker that progresses queued media jobs to terminal status.
pub(crate) struct MediaJobRuntime {
    store: MediaStore,
    inspector: Arc<RuntimeInspector>,
    command_runner: Arc<RuntimeCommandRunner>,
    capacity_probe: Arc<RuntimeCapacityProbe>,
    replacement_committer: Arc<RuntimeReplacementCommitter>,
    verification_executor: Arc<RuntimeVerificationExecutor>,
    audio_analyzer: Arc<RuntimeAudioAnalyzer>,
    events: EventBus,
    telemetry: Metrics,
    tick_interval: Duration,
    workspace_policy: WorkspacePolicy,
    workspace_root: PathBuf,
}

impl MediaJobRuntime {
    /// Construct a production media job runtime.
    pub(crate) fn new(store: MediaStore, events: EventBus, telemetry: Metrics) -> Self {
        Self::with_components(
            store,
            MediaJobRuntimeComponents {
                inspector: Arc::new(FfprobeInspectAdapter::new(
                    Arc::new(SystemInspectProbeExecutor),
                    "ffprobe",
                )),
                command_runner: Arc::new(ProcessCommandRunner),
                capacity_probe: Arc::new(SystemFilesystemCapacityProbe),
                replacement_committer: Arc::new(SystemReplacementCommitter),
                verification_executor: Arc::new(SystemVerificationExecutor),
                audio_analyzer: Arc::new(SystemFfmpegAudioAnalysisAdapter::default()),
                events,
                telemetry,
                tick_interval: DEFAULT_TICK_INTERVAL,
                workspace_policy: WorkspacePolicy {
                    max_bytes: DEFAULT_WORKSPACE_MAX_BYTES,
                    reserve_bytes: DEFAULT_WORKSPACE_RESERVE_BYTES,
                },
                workspace_root: std::env::temp_dir().join(DEFAULT_WORKSPACE_ROOT_NAME),
            },
        )
    }

    fn with_components(store: MediaStore, components: MediaJobRuntimeComponents) -> Self {
        Self {
            store,
            inspector: components.inspector,
            command_runner: components.command_runner,
            capacity_probe: components.capacity_probe,
            replacement_committer: components.replacement_committer,
            verification_executor: components.verification_executor,
            audio_analyzer: components.audio_analyzer,
            events: components.events,
            telemetry: components.telemetry,
            tick_interval: components.tick_interval,
            workspace_policy: components.workspace_policy,
            workspace_root: components.workspace_root,
        }
    }

    /// Spawn the media worker loop.
    pub(crate) fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run_loop().await;
        })
    }

    async fn run_loop(self) {
        while let Err(error) = self.recover_interrupted_replacements().await {
            warn!(error = %error, "media job replacement recovery failed; worker remains paused");
            sleep(self.tick_interval).await;
        }
        let mut ticker = interval(self.tick_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            if let Err(error) = self.run_tick().await {
                warn!(error = %error, "media job runtime tick failed");
            }
        }
    }

    async fn recover_interrupted_replacements(&self) -> Result<(), MediaJobRuntimeError> {
        let source_roots = self
            .store
            .list_profiles()
            .await?
            .into_iter()
            .map(|profile| PathBuf::from(profile.source_root))
            .collect::<BTreeSet<_>>();
        for source_root in source_roots {
            let replacement_backend = Arc::clone(&self.replacement_committer);
            let recovered =
                tokio::task::spawn_blocking(move || replacement_backend.recover(&source_root))
                    .await
                    .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))??;
            for transaction in recovered {
                if transaction.action == ReplacementRecoveryAction::Finalized {
                    info!(
                        media_job_public_id = transaction.job_key,
                        source_path = %transaction.source_path.display(),
                        "media job replacement recovery finalized verified transaction"
                    );
                    continue;
                }
                let media_job_public_id = Uuid::parse_str(&transaction.job_key).map_err(|_| {
                    MediaJobRuntimeError::InvalidRecoveryJobKey(transaction.job_key.clone())
                })?;
                let detail = "media_job_recovered_interrupted_replacement";
                self.store
                    .mark_job_status(media_job_public_id, "failed", Some(detail))
                    .await?;
                self.publish_event(Event::MediaJobFailed {
                    media_job_public_id,
                    error_code: detail.to_string(),
                });
                warn!(
                    %media_job_public_id,
                    source_path = %transaction.source_path.display(),
                    recovery_action = ?transaction.action,
                    "media job replacement recovery marked interrupted job failed"
                );
            }
        }
        Ok(())
    }

    async fn run_tick(&self) -> Result<(), DataError> {
        let claimed = self.store.claim_next_job().await?;
        if let Some(job) = claimed {
            self.process_job(job).await;
        }
        Ok(())
    }

    async fn process_job(&self, job: ClaimedMediaJobRow) {
        let started_at = Instant::now();
        let workspace =
            create_managed_workspace(&self.workspace_root, &job.media_job_public_id.to_string());
        let workspace = match workspace {
            Ok(value) => value,
            Err(error) => {
                warn!(media_job_public_id = %job.media_job_public_id, error = %error, "media job runtime failed to create workspace");
                self.telemetry.inc_media_job_failure("workspace");
                self.telemetry.inc_media_job_outcome("failed", job.dry_run);
                self.telemetry.observe_media_job_duration(
                    "failed",
                    job.dry_run,
                    started_at.elapsed(),
                );
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
            Err(MediaJobRuntimeError::Cancelled) => {
                info!(media_job_public_id = %job.media_job_public_id, "media job runtime acknowledged cancellation");
                match self.persist_cancellation(&job).await {
                    Ok(()) => TerminalWorkspaceState::Cancelled,
                    Err(error) => {
                        warn!(media_job_public_id = %job.media_job_public_id, error = %error, "media job runtime failed to persist cancellation");
                        self.telemetry.inc_media_job_failure(error.category());
                        self.persist_failure(&job, error).await;
                        TerminalWorkspaceState::Failed
                    }
                }
            }
            Err(error) => {
                warn!(media_job_public_id = %job.media_job_public_id, error = %error, "media job runtime failed job processing");
                self.telemetry.inc_media_job_failure(error.category());
                self.persist_failure(&job, error).await;
                TerminalWorkspaceState::Failed
            }
        };
        let outcome = terminal_workspace_outcome(terminal_state);
        self.telemetry.inc_media_job_outcome(outcome, job.dry_run);
        self.telemetry
            .observe_media_job_duration(outcome, job.dry_run, started_at.elapsed());

        if let Err(error) = cleanup_terminal_workspace(
            &workspace,
            terminal_state,
            TerminalWorkspaceCleanupPolicy {
                retain_diagnostics: terminal_state != TerminalWorkspaceState::Completed,
            },
        ) {
            warn!(media_job_public_id = %job.media_job_public_id, error = %error, "media job runtime failed workspace cleanup");
            self.telemetry.inc_media_workspace_cleanup("failed");
        } else {
            self.telemetry.inc_media_workspace_cleanup("completed");
        }
    }

    async fn process_claimed_job(
        &self,
        job: &ClaimedMediaJobRow,
        workspace: &revaer_media_runtime::workspace::ManagedWorkspace,
    ) -> Result<TerminalWorkspaceState, MediaJobRuntimeError> {
        self.ensure_not_cancelled(job).await?;
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
        self.ensure_not_cancelled(job).await?;
        self.publish_event(Event::MediaJobInspected {
            media_job_public_id: job.media_job_public_id,
        });
        self.persist_preflight_audits(job.media_job_public_id, &preflight.evaluation)
            .await?;
        let RuntimePreflightEvaluation {
            evaluation,
            desired,
            desired_target,
            expected_chapters,
        } = preflight;

        match evaluation {
            JobPreflightEvaluation::Failed(report) => {
                self.handle_preflight_failed(job, report).await
            }
            JobPreflightEvaluation::Ready(report) => {
                self.handle_preflight_ready(
                    job,
                    *report,
                    &desired,
                    desired_target.as_ref(),
                    &expected_chapters,
                )
                .await
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
        self.telemetry.inc_media_job_failure("preflight");
        self.mark_failed(job, report.error_code).await?;
        Ok(TerminalWorkspaceState::Failed)
    }

    async fn handle_preflight_ready(
        &self,
        job: &ClaimedMediaJobRow,
        report: JobPreflightReport,
        desired: &DesiredGraph,
        desired_target: Option<&DesiredTargetSnapshot>,
        expected_chapters: &[ChapterInspection],
    ) -> Result<TerminalWorkspaceState, MediaJobRuntimeError> {
        let verification_context = DesiredVerificationContext {
            desired,
            desired_target,
            expected_chapters,
        };
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
        self.publish_event(Event::MediaJobPlanned {
            media_job_public_id: job.media_job_public_id,
            operation_count: usize_to_u64_saturating(report.summary.total_operations),
            dry_run: job.dry_run,
        });
        if job.dry_run {
            self.complete_dry_run(job).await?;
            return Ok(TerminalWorkspaceState::Completed);
        }
        if planned_job_is_noop(&report) {
            self.complete_noop_job(job, &verification_context).await?;
            return Ok(TerminalWorkspaceState::Completed);
        }

        self.store
            .mark_job_status(job.media_job_public_id, "verifying", None)
            .await?;
        self.publish_event(Event::MediaJobExecutionStarted {
            media_job_public_id: job.media_job_public_id,
        });
        self.append_phase(job.media_job_public_id, 1, "execute", "running", None)
            .await?;
        let sidecar_outputs = report.planned.sidecar_outputs.clone();
        let sidecar_removals = report.planned.sidecar_removals.clone();
        self.execute_verified_replacement(
            job,
            report.steps,
            &verification_context,
            &SidecarPublicationContext {
                outputs: &sidecar_outputs,
                removals: &sidecar_removals,
            },
        )
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
        self.complete_or_cancel(job).await?;
        self.publish_event(Event::MediaJobCompleted {
            media_job_public_id: job.media_job_public_id,
        });
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
        self.complete_or_cancel(job).await?;
        self.publish_event(Event::MediaJobCompleted {
            media_job_public_id: job.media_job_public_id,
        });
        Ok(())
    }

    async fn complete_noop_job(
        &self,
        job: &ClaimedMediaJobRow,
        verification_context: &DesiredVerificationContext<'_>,
    ) -> Result<(), MediaJobRuntimeError> {
        self.store
            .mark_job_status(job.media_job_public_id, "verifying", None)
            .await?;
        self.append_phase(job.media_job_public_id, 1, "verify_noop", "running", None)
            .await?;
        self.verify_graph_check(
            job.media_job_public_id,
            1,
            "source_graph",
            &job.source_path,
            verification_context,
        )
        .await?;
        self.append_phase(job.media_job_public_id, 1, "verify_noop", "completed", None)
            .await?;
        self.complete_or_cancel(job).await?;
        self.publish_event(Event::MediaJobCompleted {
            media_job_public_id: job.media_job_public_id,
        });
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
        let desired_target_streams = self
            .store
            .list_job_desired_target_streams(job.media_job_public_id)
            .await?;
        let desired_target = desired_target_from_job(job, desired_target_streams)?;
        let base_video_policy =
            video_policy_from_policy_intent(job.policy_video_intent.as_deref())?;
        let workspace_output_path = workspace.output_path.clone();
        let diagnostics_root = path_to_string(&workspace.diagnostics_path, "diagnostics_path")?;
        tokio::task::spawn_blocking(move || {
            let inspection = inspector
                .inspect_full(&source_path)
                .map_err(|error| MediaJobRuntimeError::Inspect(error.to_string()))?;
            let expected_chapters = inspection.chapters.clone();
            let source_file_bytes = source_artifact_bytes(&source_path, &inspection.sidecars)?;
            let source_graph = inspection.graph;
            let compiled = match desired_target.as_ref() {
                Some(snapshot) => compile_desired_target_with_sidecars_at(
                    &source_graph,
                    &output_path,
                    &source_path,
                    &snapshot.target,
                    snapshot.unmatched_stream_policy,
                    &sidecar_inputs(&inspection.sidecars)?,
                )
                .map_err(|_| {
                    MediaJobRuntimeError::InvalidDesiredGraph(
                        "media_job_desired_target_compile_failed",
                    )
                })?,
                None => CompiledDesiredTarget {
                    graph: DesiredGraph {
                        output_path: output_path.clone(),
                        container_format: None,
                        streams: source_graph.streams.clone(),
                    },
                    sidecar_embeddings: Vec::new(),
                    sidecar_outputs: Vec::new(),
                    sidecar_removals: Vec::new(),
                },
            };
            let video_policy = video_policy_from_target_snapshot(
                base_video_policy,
                desired_target.as_ref(),
                &compiled.graph,
            )?;
            let free_bytes = capacity_probe
                .available_bytes(&workspace_output_path)
                .map_err(MediaJobRuntimeError::Capacity)?;
            let input = build_preflight_input(
                PreflightBuildTemplate {
                    source_path: &source_path,
                    output_path: &output_path,
                    desired: &compiled.graph,
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
            let evaluation = evaluate_preflight_from_compiled_target(
                &source_graph,
                &compiled,
                input.as_borrowed(),
            );
            Ok(RuntimePreflightEvaluation {
                evaluation,
                desired: compiled.graph,
                desired_target,
                expected_chapters,
            })
        })
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?
    }

    async fn execute_steps(
        &self,
        job: &ClaimedMediaJobRow,
        steps: Vec<ExecutionStep>,
    ) -> Result<(), MediaJobRuntimeError> {
        self.ensure_not_cancelled(job).await?;
        let runner = Arc::clone(&self.command_runner);
        let signal = Arc::new(CancellationSignal::default());
        let execution_signal = Arc::clone(&signal);
        let monitor_store = self.store.clone();
        let media_job_public_id = job.media_job_public_id;
        let observed_cancel_generation = job.cancel_generation;
        let (stop_tx, stop_rx) = watch::channel(false);
        let monitor = tokio::spawn(monitor_job_control(
            monitor_store,
            media_job_public_id,
            observed_cancel_generation,
            Arc::clone(&signal),
            stop_rx,
        ));
        let execution = tokio::task::spawn_blocking(move || {
            execute_step_sequence_controlled(&steps, &*runner, &*execution_signal)
        })
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        drop(stop_tx);
        let cancellation_requested = monitor
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))??;
        if cancellation_requested {
            return Err(MediaJobRuntimeError::Cancelled);
        }
        match execution {
            Err(error) if matches!(error.failed, ExecuteStepError::Cancelled) => {
                Err(MediaJobRuntimeError::Cancelled)
            }
            result => result.map_err(MediaJobRuntimeError::Execute),
        }
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
        verification_context: &DesiredVerificationContext<'_>,
        sidecar_publication: &SidecarPublicationContext<'_>,
    ) -> Result<(), MediaJobRuntimeError> {
        self.ensure_not_cancelled(job).await?;
        let candidate_output_path = self
            .execute_and_verify_candidate(job, &steps, verification_context)
            .await?;
        self.ensure_not_cancelled(job).await?;
        let replace_step = steps
            .iter()
            .find(|step| matches!(step, ExecutionStep::AtomicReplace { .. }))
            .ok_or(MediaJobRuntimeError::Verification(
                "media_job_atomic_replace_step_missing",
            ))?;
        let ExecutionStep::AtomicReplace {
            source_path,
            output_path,
        } = replace_step
        else {
            return Err(MediaJobRuntimeError::Verification(
                "media_job_atomic_replace_step_invalid",
            ));
        };
        if source_path != &job.source_path || output_path != &candidate_output_path {
            return Err(MediaJobRuntimeError::Verification(
                "media_job_atomic_replace_path_mismatch",
            ));
        }

        let committed = self
            .prepare_and_commit_replacement(
                job,
                source_path,
                output_path,
                sidecar_publication.outputs,
                sidecar_publication.removals,
            )
            .await?;
        let verification = self
            .verify_final_desired_state(job, verification_context, sidecar_publication)
            .await;
        let verification = match verification {
            Ok(()) => self.ensure_not_cancelled(job).await,
            Err(error) => Err(error),
        };
        if let Err(error) = verification {
            let replacement_backend = Arc::clone(&self.replacement_committer);
            let rollback =
                tokio::task::spawn_blocking(move || replacement_backend.rollback(committed))
                    .await
                    .map_err(|join_error| MediaJobRuntimeError::Join(join_error.to_string()))?;
            if rollback.is_err() {
                self.recover_replacement_root(PathBuf::from(&job.source_root))
                    .await?;
            }
            rollback.map_err(MediaJobRuntimeError::ReplacementRollback)?;
            return Err(error);
        }
        let replacement_backend = Arc::clone(&self.replacement_committer);
        let finalization =
            tokio::task::spawn_blocking(move || replacement_backend.finalize(committed))
                .await
                .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        if finalization.is_err() {
            self.recover_replacement_root(PathBuf::from(&job.source_root))
                .await?;
        }
        finalization?;
        Ok(())
    }

    async fn execute_and_verify_candidate(
        &self,
        job: &ClaimedMediaJobRow,
        steps: &[ExecutionStep],
        verification_context: &DesiredVerificationContext<'_>,
    ) -> Result<String, MediaJobRuntimeError> {
        let candidate_output_path = replacement_output_path(steps)?;
        let source_inspection = self.inspect_full(job.source_path.clone()).await?;
        self.ensure_not_cancelled(job).await?;
        let pre_replace_steps = steps
            .iter()
            .filter(|step| !matches!(step, ExecutionStep::AtomicReplace { .. }))
            .filter(|step| !matches!(step, ExecutionStep::QuarantineFailedOutput { .. }))
            .cloned()
            .collect::<Vec<_>>();
        self.execute_steps(job, pre_replace_steps).await?;
        self.ensure_not_cancelled(job).await?;
        let candidate_inspection = match self.inspect_full(candidate_output_path.clone()).await {
            Ok(inspection) => inspection,
            Err(error) => {
                self.quarantine_candidate(steps).await?;
                return Err(error);
            }
        };
        let graph_verification = self
            .verify_graph_inspection(
                job.media_job_public_id,
                10,
                "candidate_graph",
                &candidate_inspection,
                verification_context,
            )
            .await;
        if let Err(error) = graph_verification {
            self.quarantine_candidate(steps).await?;
            return Err(error);
        }
        self.ensure_not_cancelled(job).await?;
        let safety_report = self
            .verify_candidate_safety(
                job,
                source_inspection,
                candidate_inspection,
                candidate_output_path.clone(),
            )
            .await?;
        if let Err(error) = self
            .persist_safety_verification(job.media_job_public_id, &safety_report)
            .await
        {
            self.quarantine_candidate(steps).await?;
            return Err(error);
        }
        self.ensure_not_cancelled(job).await?;
        Ok(candidate_output_path)
    }

    async fn verify_candidate_safety(
        &self,
        job: &ClaimedMediaJobRow,
        source_inspection: MediaInspection,
        candidate_inspection: MediaInspection,
        candidate_output_path: String,
    ) -> Result<VerificationReport, MediaJobRuntimeError> {
        self.ensure_not_cancelled(job).await?;
        let verification_policy = verification_policy_from_job(job)?;
        let executor = Arc::clone(&self.verification_executor);
        let signal = Arc::new(CancellationSignal::default());
        let verification_signal = Arc::clone(&signal);
        let monitor_store = self.store.clone();
        let media_job_public_id = job.media_job_public_id;
        let observed_cancel_generation = job.cancel_generation;
        let (stop_tx, stop_rx) = watch::channel(false);
        let monitor = tokio::spawn(monitor_job_control(
            monitor_store,
            media_job_public_id,
            observed_cancel_generation,
            Arc::clone(&signal),
            stop_rx,
        ));
        let verification = tokio::task::spawn_blocking(move || {
            verify_candidate_controlled(
                &source_inspection,
                &candidate_inspection,
                &candidate_output_path,
                verification_policy,
                executor.as_ref(),
                verification_signal.as_ref(),
            )
        })
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        drop(stop_tx);
        let cancellation_requested = monitor
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))??;
        if cancellation_requested
            || matches!(verification, Err(VerificationExecutionError::Cancelled))
        {
            return Err(MediaJobRuntimeError::Cancelled);
        }
        verification.map_err(|error| match error {
            VerificationExecutionError::Cancelled => MediaJobRuntimeError::Cancelled,
            VerificationExecutionError::Failed(_) => MediaJobRuntimeError::Verification(
                "media_job_candidate_verification_executor_failed",
            ),
        })
    }

    async fn prepare_and_commit_replacement(
        &self,
        job: &ClaimedMediaJobRow,
        source_path: &str,
        candidate_path: &str,
        sidecar_outputs: &[DesiredSidecarOutput],
        sidecar_removals: &[String],
    ) -> Result<CommittedReplacement, MediaJobRuntimeError> {
        self.ensure_not_cancelled(job).await?;
        let replacement_backend = Arc::clone(&self.replacement_committer);
        let job_key = job.media_job_public_id.to_string();
        let source_root = PathBuf::from(&job.source_root);
        let source = PathBuf::from(source_path);
        let candidate = PathBuf::from(candidate_path);
        let artifact_paths = replacement_artifact_paths(sidecar_outputs, sidecar_removals);
        let preparation = tokio::task::spawn_blocking(move || {
            let artifacts = artifact_paths
                .iter()
                .map(|(destination, candidate)| ReplacementArtifactRequest {
                    destination_path: destination,
                    candidate_path: candidate.as_deref(),
                })
                .collect::<Vec<_>>();
            replacement_backend.prepare_bundle(ReplacementBundleRequest {
                job_key: &job_key,
                source_root: &source_root,
                source_path: &source,
                candidate_path: &candidate,
                artifacts: &artifacts,
            })
        })
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        let prepared = preparation?;
        if let Err(error) = self.ensure_not_cancelled(job).await {
            let replacement_backend = Arc::clone(&self.replacement_committer);
            tokio::task::spawn_blocking(move || replacement_backend.discard_prepared(prepared))
                .await
                .map_err(|join_error| MediaJobRuntimeError::Join(join_error.to_string()))??;
            return Err(error);
        }
        let replacement_backend = Arc::clone(&self.replacement_committer);
        let commit = tokio::task::spawn_blocking(move || replacement_backend.commit(prepared))
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        match commit {
            Ok(value) => Ok(value),
            Err(error) => {
                self.recover_replacement_root(PathBuf::from(&job.source_root))
                    .await?;
                Err(MediaJobRuntimeError::Replacement(error))
            }
        }
    }

    async fn recover_replacement_root(
        &self,
        source_root: PathBuf,
    ) -> Result<(), MediaJobRuntimeError> {
        let replacement_backend = Arc::clone(&self.replacement_committer);
        let recovery =
            tokio::task::spawn_blocking(move || replacement_backend.recover(&source_root))
                .await
                .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        match recovery {
            Ok(recovered) => {
                drop(recovered);
                Ok(())
            }
            Err(error) => Err(MediaJobRuntimeError::ReplacementRollback(error)),
        }
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
        verification_context: &DesiredVerificationContext<'_>,
    ) -> Result<(), MediaJobRuntimeError> {
        let inspection = self.inspect_full(output_path.to_string()).await?;
        self.verify_graph_inspection(
            media_job_public_id,
            check_index,
            check_kind,
            &inspection,
            verification_context,
        )
        .await
    }

    async fn verify_final_desired_state(
        &self,
        job: &ClaimedMediaJobRow,
        verification_context: &DesiredVerificationContext<'_>,
        sidecar_publication: &SidecarPublicationContext<'_>,
    ) -> Result<(), MediaJobRuntimeError> {
        let inspection = self.inspect_full(job.source_path.clone()).await?;
        self.verify_graph_inspection(
            job.media_job_public_id,
            20,
            "final_graph",
            &inspection,
            verification_context,
        )
        .await?;
        let matched = sidecar_state_matches(
            &inspection,
            sidecar_publication.outputs,
            sidecar_publication.removals,
        );
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id: job.media_job_public_id,
            check_index: 21,
            check_kind: "final_sidecar_state",
            check_status: if matched { "passed" } else { "failed" },
            expected_value: Some("desired_sidecar_state"),
            actual_value: Some(if matched { "matched" } else { "mismatched" }),
            details_text: Some(&inspection.graph.source_path),
        })
        .await?;
        if matched {
            Ok(())
        } else {
            self.telemetry.inc_media_job_failure("verification");
            self.publish_event(Event::MediaJobVerificationFailed {
                media_job_public_id: job.media_job_public_id,
                check_kind: "final_sidecar_state".to_string(),
                error_code: "media_job_output_sidecar_mismatch".to_string(),
            });
            Err(MediaJobRuntimeError::Verification(
                "media_job_output_sidecar_mismatch",
            ))
        }
    }

    async fn verify_graph_inspection(
        &self,
        media_job_public_id: Uuid,
        check_index: i32,
        check_kind: &'static str,
        inspection: &MediaInspection,
        verification_context: &DesiredVerificationContext<'_>,
    ) -> Result<(), MediaJobRuntimeError> {
        let matched = media_graph_matches_desired(&inspection.graph, verification_context.desired);
        let check_status = if matched { "passed" } else { "failed" };
        let actual_value = if matched { "matched" } else { "mismatched" };
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id,
            check_index,
            check_kind,
            check_status,
            expected_value: Some("desired_graph"),
            actual_value: Some(actual_value),
            details_text: Some(&inspection.graph.source_path),
        })
        .await?;
        if matched {
            self.verify_video_constraints_inspection(
                media_job_public_id,
                video_constraint_check_index(check_kind, check_index),
                video_constraint_check_kind(check_kind),
                inspection,
                verification_context.desired,
                verification_context.desired_target,
            )
            .await?;
            self.verify_audio_constraints_inspection(
                media_job_public_id,
                audio_constraint_check_index(check_kind, check_index),
                audio_constraint_check_kind(check_kind),
                inspection,
                verification_context.desired,
                verification_context.desired_target,
            )
            .await?;
            self.verify_chapter_timeline_inspection(
                media_job_public_id,
                chapter_timeline_check_index(check_kind, check_index),
                chapter_timeline_check_kind(check_kind),
                inspection,
                verification_context.expected_chapters,
            )
            .await
        } else {
            self.telemetry.inc_media_job_failure("verification");
            self.publish_event(Event::MediaJobVerificationFailed {
                media_job_public_id,
                check_kind: check_kind.to_string(),
                error_code: "media_job_output_graph_mismatch".to_string(),
            });
            Err(MediaJobRuntimeError::Verification(
                "media_job_output_graph_mismatch",
            ))
        }
    }

    async fn verify_video_constraints_inspection(
        &self,
        media_job_public_id: Uuid,
        check_index: i32,
        check_kind: &'static str,
        inspection: &MediaInspection,
        desired: &DesiredGraph,
        desired_target: Option<&DesiredTargetSnapshot>,
    ) -> Result<(), MediaJobRuntimeError> {
        let verification = video_constraints_match_inspection(inspection, desired, desired_target);
        let check_status = if verification.matched {
            "passed"
        } else {
            "failed"
        };
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id,
            check_index,
            check_kind,
            check_status,
            expected_value: Some(verification.expected.as_str()),
            actual_value: Some(verification.actual.as_str()),
            details_text: verification.details.as_deref(),
        })
        .await?;
        if verification.matched {
            Ok(())
        } else {
            self.telemetry.inc_media_job_failure("verification");
            self.publish_event(Event::MediaJobVerificationFailed {
                media_job_public_id,
                check_kind: check_kind.to_string(),
                error_code: "media_job_output_video_constraints_mismatch".to_string(),
            });
            Err(MediaJobRuntimeError::Verification(
                "media_job_output_video_constraints_mismatch",
            ))
        }
    }

    async fn verify_audio_constraints_inspection(
        &self,
        media_job_public_id: Uuid,
        check_index: i32,
        check_kind: &'static str,
        inspection: &MediaInspection,
        desired: &DesiredGraph,
        desired_target: Option<&DesiredTargetSnapshot>,
    ) -> Result<(), MediaJobRuntimeError> {
        let verification = self
            .audio_constraints_match_inspection(inspection, desired, desired_target)
            .await?;
        let check_status = if verification.matched {
            "passed"
        } else {
            "failed"
        };
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id,
            check_index,
            check_kind,
            check_status,
            expected_value: Some(verification.expected.as_str()),
            actual_value: Some(verification.actual.as_str()),
            details_text: verification.details.as_deref(),
        })
        .await?;
        if verification.matched {
            Ok(())
        } else {
            self.telemetry.inc_media_job_failure("verification");
            self.publish_event(Event::MediaJobVerificationFailed {
                media_job_public_id,
                check_kind: check_kind.to_string(),
                error_code: "media_job_output_audio_constraints_mismatch".to_string(),
            });
            Err(MediaJobRuntimeError::Verification(
                "media_job_output_audio_constraints_mismatch",
            ))
        }
    }

    async fn verify_chapter_timeline_inspection(
        &self,
        media_job_public_id: Uuid,
        check_index: i32,
        check_kind: &'static str,
        inspection: &MediaInspection,
        expected_chapters: &[ChapterInspection],
    ) -> Result<(), MediaJobRuntimeError> {
        let verification = chapter_timeline_matches_inspection(inspection, expected_chapters);
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id,
            check_index,
            check_kind,
            check_status: if verification.matched {
                "passed"
            } else {
                "failed"
            },
            expected_value: Some(verification.expected.as_str()),
            actual_value: Some(verification.actual.as_str()),
            details_text: verification.details.as_deref(),
        })
        .await?;
        if verification.matched {
            Ok(())
        } else {
            self.telemetry.inc_media_job_failure("verification");
            self.publish_event(Event::MediaJobVerificationFailed {
                media_job_public_id,
                check_kind: check_kind.to_string(),
                error_code: "media_job_output_chapter_timeline_mismatch".to_string(),
            });
            Err(MediaJobRuntimeError::Verification(
                "media_job_output_chapter_timeline_mismatch",
            ))
        }
    }

    async fn audio_constraints_match_inspection(
        &self,
        inspection: &MediaInspection,
        desired: &DesiredGraph,
        target: Option<&DesiredTargetSnapshot>,
    ) -> Result<AudioConstraintVerification, MediaJobRuntimeError> {
        audio_constraints_match_inspection(
            inspection,
            desired,
            target,
            Arc::clone(&self.audio_analyzer),
        )
        .await
    }

    async fn persist_safety_verification(
        &self,
        media_job_public_id: Uuid,
        report: &VerificationReport,
    ) -> Result<(), MediaJobRuntimeError> {
        for (offset, check) in report.checks.iter().enumerate() {
            let check_index = i32::try_from(offset)
                .map_err(index_error("verification check index"))?
                .checked_add(11)
                .ok_or(MediaJobRuntimeError::Verification(
                    "media_job_verification_check_index_overflow",
                ))?;
            self.append_verification_check(&AppendMediaJobVerificationCheckInput {
                media_job_public_id,
                check_index,
                check_kind: check.kind,
                check_status: if check.passed { "passed" } else { "failed" },
                expected_value: Some(&check.expected),
                actual_value: Some(&check.actual),
                details_text: check.details.as_deref(),
            })
            .await?;
        }
        if report.passed() {
            return Ok(());
        }
        self.telemetry.inc_media_job_failure("verification");
        self.publish_event(Event::MediaJobVerificationFailed {
            media_job_public_id,
            check_kind: "candidate_safety".to_string(),
            error_code: "media_job_candidate_safety_verification_failed".to_string(),
        });
        Err(MediaJobRuntimeError::Verification(
            "media_job_candidate_safety_verification_failed",
        ))
    }

    async fn inspect_full(
        &self,
        source_path: String,
    ) -> Result<MediaInspection, MediaJobRuntimeError> {
        let inspector = Arc::clone(&self.inspector);
        tokio::task::spawn_blocking(move || {
            inspector
                .inspect_full(&source_path)
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
        let utilities = feature_names(&snapshot.features, "utility", true);
        if snapshot.ffmpeg_version.trim().is_empty()
            || snapshot.ffprobe_version.trim().is_empty()
            || snapshot.codecs.is_empty()
            || snapshot.encoders.is_empty()
            || feature_names(&snapshot.features, "decoder", true).is_empty()
            || feature_names(&snapshot.features, "muxer", true).is_empty()
            || feature_names(&snapshot.features, "demuxer", true).is_empty()
            || feature_names(&snapshot.features, "filesystem", true).is_empty()
            || !["ffmpeg", "ffprobe", "ffplay"]
                .iter()
                .all(|required| utilities.iter().any(|actual| actual == required))
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
        let ffmpeg_license_mode = feature_names(&snapshot.features, "ffmpeg_license", true)
            .into_iter()
            .next()
            .unwrap_or_else(|| license_mode.clone());
        let ffmpeg_enable_gpl =
            feature_supported(&snapshot.features, "ffmpeg_build_flag", "--enable-gpl");
        let ffmpeg_enable_version3 =
            feature_supported(&snapshot.features, "ffmpeg_build_flag", "--enable-version3");
        let ffmpeg_enable_nonfree =
            feature_supported(&snapshot.features, "ffmpeg_build_flag", "--enable-nonfree");
        let compliance_links = feature_names(&snapshot.features, "compliance", true);
        let absent_capabilities = feature_names(&snapshot.features, "absent", false);

        let runtime_snapshot = CapabilitySnapshot {
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
            ffmpeg_license_mode,
            ffmpeg_enable_gpl,
            ffmpeg_enable_version3,
            ffmpeg_enable_nonfree,
            compliance_links,
            absent_capabilities,
        };
        if !runtime_snapshot.is_valid() {
            return Err(MediaJobRuntimeError::InvalidCapability(
                "media_capability_snapshot_invalid",
            ));
        }
        Ok(runtime_snapshot)
    }

    async fn persist_ready_plan(
        &self,
        media_job_public_id: Uuid,
        report: &JobPreflightReport,
    ) -> Result<(), MediaJobRuntimeError> {
        for (index, operation) in report.planned.operations.iter().enumerate() {
            let (command_bin, args) = command_fields_for_operation(&report.steps, index);
            let operation_kind = operation_kind_code(operation.kind);
            self.store
                .append_job_operation(
                    media_job_public_id,
                    usize_to_i32(index, "operation_index")?,
                    operation_kind,
                    stream_id_to_i32(operation)?,
                    command_bin,
                    args,
                )
                .await?;
            self.telemetry
                .inc_media_job_operation(operation_kind, "planned");
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
        let result = self
            .store
            .append_job_phase(
                media_job_public_id,
                phase_index,
                phase_name,
                phase_status,
                details_text,
            )
            .await;
        if result.is_ok() {
            self.telemetry.inc_media_job_phase(phase_name, phase_status);
        }
        result
    }

    async fn append_verification_check(
        &self,
        input: &AppendMediaJobVerificationCheckInput<'_>,
    ) -> Result<(), DataError> {
        let result = self.store.append_job_verification_check(input).await;
        if result.is_ok() {
            self.telemetry
                .inc_media_job_verification_check(input.check_kind, input.check_status);
        }
        result
    }

    async fn ensure_not_cancelled(
        &self,
        job: &ClaimedMediaJobRow,
    ) -> Result<(), MediaJobRuntimeError> {
        let control = self
            .store
            .poll_job_control(job.media_job_public_id, job.cancel_generation)
            .await?;
        if control.cancel_requested {
            Err(MediaJobRuntimeError::Cancelled)
        } else {
            Ok(())
        }
    }

    async fn complete_or_cancel(
        &self,
        job: &ClaimedMediaJobRow,
    ) -> Result<(), MediaJobRuntimeError> {
        let cancelled = self
            .store
            .complete_job(job.media_job_public_id, job.cancel_generation)
            .await?;
        if cancelled {
            Err(MediaJobRuntimeError::Cancelled)
        } else {
            Ok(())
        }
    }

    async fn persist_cancellation(
        &self,
        job: &ClaimedMediaJobRow,
    ) -> Result<(), MediaJobRuntimeError> {
        self.append_phase(
            job.media_job_public_id,
            CANCELLATION_PHASE_INDEX,
            "runtime_cancellation",
            "cancelled",
            Some("media_job_cancelled_by_operator"),
        )
        .await?;
        self.append_verification_check(&AppendMediaJobVerificationCheckInput {
            media_job_public_id: job.media_job_public_id,
            check_index: CANCELLATION_CHECK_INDEX,
            check_kind: "cancellation",
            check_status: "skipped",
            expected_value: Some("continue"),
            actual_value: Some("operator_cancelled"),
            details_text: Some("worker acknowledged durable cancellation generation"),
        })
        .await?;
        let acknowledged_generation = self
            .store
            .acknowledge_job_cancel(job.media_job_public_id, job.cancel_generation)
            .await?;
        info!(
            media_job_public_id = %job.media_job_public_id,
            observed_cancel_generation = job.cancel_generation,
            acknowledged_cancel_generation = acknowledged_generation,
            "media job cancellation persisted"
        );
        Ok(())
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
            .await?;
        self.publish_event(Event::MediaJobFailed {
            media_job_public_id: job.media_job_public_id,
            error_code: detail.to_string(),
        });
        Ok(())
    }

    fn publish_event(&self, event: Event) {
        if let Err(error) = self.events.publish(event) {
            warn!(
                event_id = error.event_id(),
                event_kind = error.event_kind(),
                error = %error,
                "media job runtime failed to publish event"
            );
        }
    }
}

fn verification_policy_from_job(
    job: &ClaimedMediaJobRow,
) -> Result<VerificationPolicy, MediaJobRuntimeError> {
    match job.verification_strictness.as_str() {
        "strict" | "balanced" | "fast" => {}
        _ => {
            return Err(MediaJobRuntimeError::Verification(
                "media_job_verification_strictness_invalid",
            ));
        }
    }
    let duration_tolerance_millis = u64::try_from(job.verification_duration_tolerance_millis)
        .map_err(|_| {
            MediaJobRuntimeError::Verification("media_job_verification_duration_tolerance_invalid")
        })?;
    if duration_tolerance_millis > 60_000 {
        return Err(MediaJobRuntimeError::Verification(
            "media_job_verification_duration_tolerance_invalid",
        ));
    }
    let selected_checks_valid = match job.verification_strictness.as_str() {
        "strict" => {
            job.verification_mux_validation.enabled()
                && job.verification_decode_all_streams.enabled()
                && job.verification_keyframe_seek.enabled()
                && job.verification_playback_probe.enabled()
        }
        "balanced" => true,
        "fast" => {
            !job.verification_decode_all_streams.enabled()
                && !job.verification_keyframe_seek.enabled()
                && !job.verification_playback_probe.enabled()
        }
        _ => false,
    };
    if !selected_checks_valid {
        return Err(MediaJobRuntimeError::Verification(
            "media_job_verification_checks_invalid",
        ));
    }
    Ok(VerificationPolicy {
        duration_tolerance_millis,
        checks: VerificationChecks::none()
            .with_mux_validation(job.verification_mux_validation.enabled())
            .with_decode_all_streams(job.verification_decode_all_streams.enabled())
            .with_keyframe_seek(job.verification_keyframe_seek.enabled())
            .with_playback_probe(job.verification_playback_probe.enabled()),
    })
}

async fn monitor_job_control(
    store: MediaStore,
    media_job_public_id: Uuid,
    observed_cancel_generation: i64,
    signal: Arc<CancellationSignal>,
    mut stop: watch::Receiver<bool>,
) -> Result<bool, DataError> {
    let mut ticker = interval(CONTROL_POLL_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let control = store
                    .poll_job_control(media_job_public_id, observed_cancel_generation)
                    .await;
                match control {
                    Ok(control) if control.cancel_requested => {
                        signal.request();
                        return Ok(true);
                    }
                    Ok(_) => {}
                    Err(error) => {
                        signal.request();
                        return Err(error);
                    }
                }
            }
            changed = stop.changed() => {
                match changed {
                    Ok(()) if *stop.borrow() => return Ok(false),
                    Ok(()) => {}
                    Err(_) => return Ok(false),
                }
            }
        }
    }
}

#[derive(Debug, Error)]
enum MediaJobRuntimeError {
    #[error("media job runtime data error: {0}")]
    Data(#[from] DataError),
    #[error("media job runtime cancelled by operator")]
    Cancelled,
    #[error("media job runtime join error: {0}")]
    Join(String),
    #[error("media job runtime inspect error: {0}")]
    Inspect(String),
    #[error("media job runtime source metadata failed for {path}: {source}")]
    SourceMetadata {
        path: String,
        source: std::io::Error,
    },
    #[error("media job runtime source artifact byte count overflowed")]
    SourceSizeOverflow,
    #[error("media job runtime execute error: {0}")]
    Execute(ExecuteSequenceError),
    #[error("media job runtime execute step error: {0}")]
    ExecuteStep(ExecuteStepError),
    #[error("media job runtime replacement error: {0}")]
    Replacement(#[from] ReplacementError),
    #[error("media job runtime replacement rollback error: {0}")]
    ReplacementRollback(ReplacementError),
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
    #[error("media job runtime replacement manifest has invalid job key: {0}")]
    InvalidRecoveryJobKey(String),
}

impl MediaJobRuntimeError {
    const fn code(&self) -> &'static str {
        match self {
            Self::Data(_) => "media_job_runtime_storage_failed",
            Self::Cancelled => "media_job_cancelled_by_operator",
            Self::Join(_) => "media_job_runtime_join_failed",
            Self::Inspect(_) => "media_job_runtime_inspect_failed",
            Self::SourceMetadata { .. } => "media_job_runtime_source_metadata_failed",
            Self::SourceSizeOverflow => "media_job_runtime_source_size_overflow",
            Self::Execute(_) | Self::ExecuteStep(_) => "media_job_runtime_execute_failed",
            Self::Replacement(_) => "media_job_runtime_replacement_failed",
            Self::ReplacementRollback(_) => "media_job_runtime_replacement_rollback_failed",
            Self::Workspace(_) => "media_job_runtime_workspace_failed",
            Self::Capacity(_) => "media_job_runtime_capacity_probe_failed",
            Self::InvalidPath(code)
            | Self::InvalidDesiredGraph(code)
            | Self::Verification(code)
            | Self::InvalidCapability(code)
            | Self::IndexTooLarge(code) => code,
            Self::InvalidRecoveryJobKey(_) => "media_job_runtime_recovery_job_key_invalid",
        }
    }

    const fn category(&self) -> &'static str {
        match self {
            Self::Data(_) => "storage",
            Self::Cancelled => "cancellation",
            Self::Join(_) => "join",
            Self::Inspect(_) | Self::SourceMetadata { .. } | Self::SourceSizeOverflow => {
                "inspection"
            }
            Self::Execute(_) | Self::ExecuteStep(_) => "execution",
            Self::Replacement(_)
            | Self::ReplacementRollback(_)
            | Self::InvalidRecoveryJobKey(_) => "replacement",
            Self::Workspace(_) => "workspace",
            Self::Capacity(_) => "disk_reserve",
            Self::InvalidPath(_) | Self::InvalidDesiredGraph(_) | Self::IndexTooLarge(_) => {
                "planning"
            }
            Self::Verification(_) => "verification",
            Self::InvalidCapability(_) => "capability",
        }
    }
}

const fn terminal_workspace_outcome(terminal_state: TerminalWorkspaceState) -> &'static str {
    match terminal_state {
        TerminalWorkspaceState::Completed => "completed",
        TerminalWorkspaceState::Failed => "failed",
        TerminalWorkspaceState::Cancelled => "cancelled",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DesiredTargetSnapshot {
    target: DesiredTarget,
    unmatched_stream_policy: UnmatchedStreamPolicy,
}

fn desired_target_from_job(
    job: &ClaimedMediaJobRow,
    rows: Vec<MediaJobDesiredTargetStreamRow>,
) -> Result<Option<DesiredTargetSnapshot>, MediaJobRuntimeError> {
    let Some(target_key) = job
        .desired_target_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        if !rows.is_empty()
            || job.desired_target_version.is_some()
            || job.desired_container_format.is_some()
        {
            return Err(MediaJobRuntimeError::InvalidDesiredGraph(
                "media_job_desired_target_snapshot_incomplete",
            ));
        }
        return Ok(None);
    };

    let version = u32::try_from(job.desired_target_version.unwrap_or_default()).map_err(|_| {
        MediaJobRuntimeError::InvalidDesiredGraph("media_job_desired_target_version_invalid")
    })?;
    if version == 0 {
        return Err(MediaJobRuntimeError::InvalidDesiredGraph(
            "media_job_desired_target_version_invalid",
        ));
    }
    let container = normalized_snapshot_field(
        job.desired_container_format.as_deref(),
        "media_job_desired_container_missing",
    )?;
    let unmatched_stream_policy =
        unmatched_stream_policy_from_snapshot(job.unmatched_stream_policy.as_deref())?;
    let streams = rows
        .into_iter()
        .map(target_stream_from_snapshot)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(DesiredTargetSnapshot {
        target: DesiredTarget {
            target_key: target_key.to_ascii_lowercase(),
            version,
            container,
            streams,
        },
        unmatched_stream_policy,
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

fn target_stream_from_snapshot(
    row: MediaJobDesiredTargetStreamRow,
) -> Result<TargetStream, MediaJobRuntimeError> {
    let kind = stream_kind_from_snapshot(&row.stream_kind)?;
    let role = row
        .semantic_role
        .as_deref()
        .map(semantic_role_from_snapshot)
        .transpose()?;
    let channels = row
        .channel_count
        .map(|value| {
            u32::try_from(value).map_err(|_| {
                MediaJobRuntimeError::InvalidDesiredGraph(
                    "media_job_desired_target_channel_count_invalid",
                )
            })
        })
        .transpose()?;
    let mut dispositions = Vec::with_capacity(2);
    if row.default_disposition {
        dispositions.push("default".to_string());
    }
    if row.forced_disposition {
        dispositions.push("forced".to_string());
    }
    let subtitle_placement =
        subtitle_placement_from_snapshot(kind, row.subtitle_placement.as_deref())?;
    let image_subtitle_action =
        image_subtitle_action_from_snapshot(kind, row.image_subtitle_action.as_deref())?;
    Ok(TargetStream {
        stream_key: row.stream_key,
        kind,
        role,
        language: row.language_code,
        optional: row.optional,
        codec: row.codec,
        channels,
        channel_layout: row.channel_layout,
        audio_bitrate_bps: row
            .audio_bitrate_bps
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    MediaJobRuntimeError::InvalidDesiredGraph(
                        "media_job_desired_target_audio_bitrate_invalid",
                    )
                })
            })
            .transpose()?,
        audio_sample_rate_hz: row
            .audio_sample_rate_hz
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    MediaJobRuntimeError::InvalidDesiredGraph(
                        "media_job_desired_target_audio_sample_rate_invalid",
                    )
                })
            })
            .transpose()?,
        audio_loudness_profile: row.audio_loudness_profile,
        audio_dynamic_range: row.audio_dynamic_range,
        video_profile: row.video_profile,
        video_level: row.video_level,
        video_bitrate_bps: row
            .video_bitrate_bps
            .map(|value| {
                u32::try_from(value).map_err(|_| {
                    MediaJobRuntimeError::InvalidDesiredGraph(
                        "media_job_desired_target_video_bitrate_invalid",
                    )
                })
            })
            .transpose()?,
        color_primaries: row.color_primaries,
        color_transfer: row.color_transfer,
        color_space: row.color_space,
        hdr_format: row.hdr_format,
        title: row.title,
        dispositions,
        subtitle_placement,
        image_subtitle_action,
    })
}

fn subtitle_placement_from_snapshot(
    kind: StreamKind,
    value: Option<&str>,
) -> Result<Option<SubtitlePlacement>, MediaJobRuntimeError> {
    if kind != StreamKind::Subtitle {
        return value.map_or(Ok(None), |_| {
            Err(MediaJobRuntimeError::InvalidDesiredGraph(
                "media_job_desired_target_subtitle_shape_invalid",
            ))
        });
    }
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("embedded") => Ok(Some(SubtitlePlacement::Embedded)),
        Some("sidecar") => Ok(Some(SubtitlePlacement::Sidecar)),
        Some("both") => Ok(Some(SubtitlePlacement::Both)),
        Some("none") => Ok(Some(SubtitlePlacement::None)),
        _ => Err(MediaJobRuntimeError::InvalidDesiredGraph(
            "media_job_desired_target_subtitle_placement_invalid",
        )),
    }
}

fn image_subtitle_action_from_snapshot(
    kind: StreamKind,
    value: Option<&str>,
) -> Result<Option<ImageSubtitleAction>, MediaJobRuntimeError> {
    if kind != StreamKind::Subtitle {
        return value.map_or(Ok(None), |_| {
            Err(MediaJobRuntimeError::InvalidDesiredGraph(
                "media_job_desired_target_subtitle_shape_invalid",
            ))
        });
    }
    match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
        Some("preserve") => Ok(Some(ImageSubtitleAction::Preserve)),
        Some("remove") => Ok(Some(ImageSubtitleAction::Remove)),
        Some("fail") => Ok(Some(ImageSubtitleAction::Fail)),
        _ => Err(MediaJobRuntimeError::InvalidDesiredGraph(
            "media_job_desired_target_image_subtitle_action_invalid",
        )),
    }
}

fn stream_kind_from_snapshot(value: &str) -> Result<StreamKind, MediaJobRuntimeError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "video" => Ok(StreamKind::Video),
        "audio" => Ok(StreamKind::Audio),
        "subtitle" => Ok(StreamKind::Subtitle),
        "attachment" => Ok(StreamKind::Attachment),
        "chapter" => Ok(StreamKind::Chapter),
        _ => Err(MediaJobRuntimeError::InvalidDesiredGraph(
            "media_job_desired_target_stream_kind_unknown",
        )),
    }
}

fn semantic_role_from_snapshot(value: &str) -> Result<SemanticRole, MediaJobRuntimeError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "primary" => Ok(SemanticRole::Primary),
        "forced" => Ok(SemanticRole::Forced),
        "commentary" => Ok(SemanticRole::Commentary),
        "descriptive_audio" => Ok(SemanticRole::DescriptiveAudio),
        "sdh" => Ok(SemanticRole::Sdh),
        "signs_songs" => Ok(SemanticRole::SignsSongs),
        "karaoke" => Ok(SemanticRole::Karaoke),
        "unknown" => Ok(SemanticRole::Unknown),
        _ => Err(MediaJobRuntimeError::InvalidDesiredGraph(
            "media_job_desired_target_stream_role_unknown",
        )),
    }
}

fn unmatched_stream_policy_from_snapshot(
    value: Option<&str>,
) -> Result<UnmatchedStreamPolicy, MediaJobRuntimeError> {
    match normalized_snapshot_field(value, "media_job_unmatched_stream_policy_missing")?.as_str() {
        "remove" => Ok(UnmatchedStreamPolicy::Remove),
        "preserve" => Ok(UnmatchedStreamPolicy::Preserve),
        "reject" => Ok(UnmatchedStreamPolicy::Reject),
        _ => Err(MediaJobRuntimeError::InvalidDesiredGraph(
            "media_job_unmatched_stream_policy_unknown",
        )),
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubtitlePolicy {
    Selected,
    All,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DesiredGraphTarget {
    video_codec: String,
    audio_codec: String,
    audio_channels: Option<u32>,
    audio_channel_layout: Option<String>,
    subtitle_policy: SubtitlePolicy,
}

#[cfg(test)]
fn compile_desired_graph(
    source: &MediaGraph,
    output_path: &str,
    target: Option<&DesiredGraphTarget>,
) -> DesiredGraph {
    let Some(target) = target else {
        return DesiredGraph {
            output_path: output_path.to_string(),
            container_format: None,
            streams: source.streams.clone(),
        };
    };
    let selected_subtitle = source.streams.iter().find(|stream| {
        stream.kind == StreamKind::Subtitle
            && infer_test_role(stream) != SemanticRole::Commentary
            && stream.dispositions.iter().any(|value| {
                value.eq_ignore_ascii_case("default") || value.eq_ignore_ascii_case("forced")
            })
    });
    let fallback_subtitle = source.streams.iter().find(|stream| {
        stream.kind == StreamKind::Subtitle
            && infer_test_role(stream) != SemanticRole::Commentary
            && stream
                .language
                .as_deref()
                .is_some_and(|language| language.eq_ignore_ascii_case("eng"))
    });
    let selected_subtitle_id = selected_subtitle
        .or(fallback_subtitle)
        .map(|stream| stream.stream_id);
    let mut default_subtitle_assigned = false;
    let streams = source
        .streams
        .iter()
        .filter_map(|stream| match stream.kind {
            StreamKind::Video => Some(MediaStream {
                codec: target.video_codec.clone(),
                ..stream.clone()
            }),
            StreamKind::Audio => Some(MediaStream {
                codec: target.audio_codec.clone(),
                channels: target.audio_channels,
                channel_layout: target.audio_channel_layout.clone(),
                ..stream.clone()
            }),
            StreamKind::Subtitle
                if target.subtitle_policy == SubtitlePolicy::Selected
                    && selected_subtitle_id != Some(stream.stream_id) =>
            {
                None
            }
            StreamKind::Subtitle => {
                let commentary = infer_test_role(stream) == SemanticRole::Commentary;
                let mut dispositions = stream
                    .dispositions
                    .iter()
                    .map(|value| value.trim().to_ascii_lowercase())
                    .filter(|value| !value.is_empty())
                    .filter(|value| {
                        value != "default" || (!commentary && !default_subtitle_assigned)
                    })
                    .collect::<Vec<_>>();
                if dispositions.iter().any(|value| value == "default") {
                    default_subtitle_assigned = true;
                }
                dispositions.sort();
                dispositions.dedup();
                Some(MediaStream {
                    dispositions,
                    ..stream.clone()
                })
            }
            StreamKind::Attachment | StreamKind::Chapter | StreamKind::Data => Some(stream.clone()),
        })
        .collect();
    DesiredGraph {
        output_path: output_path.to_string(),
        container_format: None,
        streams,
    }
}

#[cfg(test)]
fn infer_test_role(stream: &MediaStream) -> SemanticRole {
    revaer_media_core::classify::infer_role(stream)
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

fn video_policy_from_target_snapshot(
    mut policy: VideoTranscodePolicy,
    target: Option<&DesiredTargetSnapshot>,
    desired: &DesiredGraph,
) -> Result<VideoTranscodePolicy, MediaJobRuntimeError> {
    let Some(target) = target else {
        return Ok(policy);
    };
    let mut consumed_stream_ids = BTreeSet::new();
    for target_stream in &target.target.streams {
        if target_stream.kind != StreamKind::Video
            || !target_stream_has_video_constraints(target_stream)
        {
            continue;
        }
        let stream =
            desired_target_stream_for_constraints(desired, target_stream, &consumed_stream_ids)
                .ok_or(MediaJobRuntimeError::InvalidDesiredGraph(
                    "media_job_target_constraint_stream_unmatched",
                ))?;
        consumed_stream_ids.insert(stream.stream_id);
        policy.stream_constraints.push(VideoStreamConstraints {
            stream_id: stream.stream_id,
            profile: target_stream.video_profile.clone(),
            level: target_stream.video_level.clone(),
            bitrate_bps: target_stream.video_bitrate_bps,
            color_primaries: target_stream.color_primaries.clone(),
            color_transfer: target_stream.color_transfer.clone(),
            color_space: target_stream.color_space.clone(),
            hdr_format: target_stream.hdr_format.clone(),
        });
    }
    consumed_stream_ids.clear();
    for target_stream in &target.target.streams {
        if target_stream.kind != StreamKind::Audio
            || !target_stream_has_audio_constraints(target_stream)
        {
            continue;
        }
        let stream =
            desired_target_stream_for_constraints(desired, target_stream, &consumed_stream_ids)
                .ok_or(MediaJobRuntimeError::InvalidDesiredGraph(
                    "media_job_target_constraint_stream_unmatched",
                ))?;
        consumed_stream_ids.insert(stream.stream_id);
        policy
            .audio_stream_constraints
            .push(AudioStreamConstraints {
                stream_id: stream.stream_id,
                bitrate_bps: target_stream.audio_bitrate_bps,
                sample_rate_hz: target_stream.audio_sample_rate_hz,
                loudness_profile: target_stream.audio_loudness_profile.clone(),
                dynamic_range: target_stream.audio_dynamic_range.clone(),
            });
    }
    Ok(policy)
}

fn desired_target_stream_for_constraints<'a>(
    desired: &'a DesiredGraph,
    target_stream: &TargetStream,
    consumed_stream_ids: &BTreeSet<u32>,
) -> Option<&'a MediaStream> {
    desired.streams.iter().find(|stream| {
        stream.kind == target_stream.kind
            && !consumed_stream_ids.contains(&stream.stream_id)
            && stream
                .codec
                .trim()
                .eq_ignore_ascii_case(target_stream.codec.trim())
    })
}

struct VideoConstraintVerification {
    matched: bool,
    expected: String,
    actual: String,
    details: Option<String>,
}

fn video_constraints_match_inspection(
    inspection: &MediaInspection,
    desired: &DesiredGraph,
    target: Option<&DesiredTargetSnapshot>,
) -> VideoConstraintVerification {
    let constraints = match expected_video_constraints(target, desired) {
        Ok(constraints) => constraints,
        Err(mismatch) => return mismatch,
    };
    if constraints.is_empty() {
        return VideoConstraintVerification {
            matched: true,
            expected: "desired_video_constraints".to_string(),
            actual: "not_configured".to_string(),
            details: None,
        };
    }
    for constraint in &constraints {
        let Some(stream) = inspection
            .streams
            .iter()
            .find(|stream| stream.stream_id == constraint.stream_id)
        else {
            return video_constraint_mismatch(constraint.stream_id, "stream", "present", "missing");
        };
        if let Some(mismatch) = video_constraint_stream_mismatch(constraint, stream) {
            return mismatch;
        }
    }
    VideoConstraintVerification {
        matched: true,
        expected: format!("{} constrained_video_streams", constraints.len()),
        actual: "matched".to_string(),
        details: None,
    }
}

fn expected_video_constraints(
    target: Option<&DesiredTargetSnapshot>,
    desired: &DesiredGraph,
) -> Result<Vec<VideoStreamConstraints>, VideoConstraintVerification> {
    let Some(target) = target else {
        return Ok(Vec::new());
    };
    let mut constraints = Vec::new();
    let mut consumed_stream_ids = BTreeSet::new();
    for target_stream in &target.target.streams {
        if target_stream.kind != StreamKind::Video
            || !target_stream_has_video_constraints(target_stream)
        {
            continue;
        }
        let Some(stream) =
            desired_target_stream_for_constraints(desired, target_stream, &consumed_stream_ids)
        else {
            return Err(video_target_constraint_unmatched(target_stream));
        };
        consumed_stream_ids.insert(stream.stream_id);
        constraints.push(VideoStreamConstraints {
            stream_id: stream.stream_id,
            profile: target_stream.video_profile.clone(),
            level: target_stream.video_level.clone(),
            bitrate_bps: target_stream.video_bitrate_bps,
            color_primaries: target_stream.color_primaries.clone(),
            color_transfer: target_stream.color_transfer.clone(),
            color_space: target_stream.color_space.clone(),
            hdr_format: target_stream.hdr_format.clone(),
        });
    }
    Ok(constraints)
}

fn video_target_constraint_unmatched(target_stream: &TargetStream) -> VideoConstraintVerification {
    VideoConstraintVerification {
        matched: false,
        expected: format!(
            "target_stream:{}:video_constraint=mapped",
            target_stream.stream_key
        ),
        actual: format!("unmatched:{:?}:{}", target_stream.kind, target_stream.codec),
        details: Some(format!(
            "target stream {} video constraints could not be mapped onto the compiled desired graph",
            target_stream.stream_key
        )),
    }
}

fn video_constraint_stream_mismatch(
    constraint: &VideoStreamConstraints,
    stream: &StreamInspection,
) -> Option<VideoConstraintVerification> {
    if let Some(expected) = constraint.profile.as_deref()
        && normalized_compact_text(stream.profile.as_deref())
            .is_none_or(|actual| actual != normalized_compact_value(expected))
    {
        return Some(video_constraint_mismatch(
            constraint.stream_id,
            "profile",
            expected,
            stream.profile.as_deref().unwrap_or("missing"),
        ));
    }
    if let Some(expected) = constraint.level.as_deref()
        && !video_level_matches(expected, stream_level_metadata(stream))
    {
        return Some(video_constraint_mismatch(
            constraint.stream_id,
            "level",
            expected,
            stream_level_metadata(stream).unwrap_or("missing"),
        ));
    }
    if let Some(expected) = constraint.bitrate_bps
        && !stream
            .bit_rate
            .is_some_and(|actual| bitrate_within_tolerance(u64::from(expected), actual))
    {
        let expected_text = expected.to_string();
        let actual_text = stream
            .bit_rate
            .map_or_else(|| "missing".to_string(), |value| value.to_string());
        return Some(video_constraint_mismatch(
            constraint.stream_id,
            "bitrate_bps",
            &expected_text,
            &actual_text,
        ));
    }
    let (color_primaries, color_transfer, color_space) =
        normalized_expected_color_constraints(constraint);
    if let Some(expected) = color_primaries.as_deref()
        && normalized_constraint_text(stream.color_primaries.as_deref())
            .is_none_or(|actual| actual != normalized_constraint_value(expected))
    {
        return Some(video_constraint_mismatch(
            constraint.stream_id,
            "color_primaries",
            expected,
            stream.color_primaries.as_deref().unwrap_or("missing"),
        ));
    }
    if let Some(expected) = color_transfer.as_deref()
        && normalized_constraint_text(stream.color_transfer.as_deref())
            .is_none_or(|actual| actual != normalized_constraint_value(expected))
    {
        return Some(video_constraint_mismatch(
            constraint.stream_id,
            "color_transfer",
            expected,
            stream.color_transfer.as_deref().unwrap_or("missing"),
        ));
    }
    if let Some(expected) = color_space.as_deref()
        && normalized_constraint_text(stream.color_space.as_deref())
            .is_none_or(|actual| actual != normalized_constraint_value(expected))
    {
        return Some(video_constraint_mismatch(
            constraint.stream_id,
            "color_space",
            expected,
            stream.color_space.as_deref().unwrap_or("missing"),
        ));
    }
    if let Some(expected) = constraint.hdr_format.as_deref()
        && !video_hdr_constraint_matches(expected, stream)
    {
        return Some(video_constraint_mismatch(
            constraint.stream_id,
            "hdr_format",
            expected,
            "unverified",
        ));
    }
    None
}

fn video_constraint_mismatch(
    stream_id: u32,
    field: &str,
    expected: &str,
    actual: &str,
) -> VideoConstraintVerification {
    VideoConstraintVerification {
        matched: false,
        expected: format!("stream:{stream_id}:{field}={expected}"),
        actual: actual.to_string(),
        details: Some(format!("stream {stream_id} video {field} mismatch")),
    }
}

fn stream_level_metadata(stream: &StreamInspection) -> Option<&str> {
    stream
        .metadata
        .iter()
        .find(|entry| normalized_constraint_value(&entry.key) == "level")
        .map(|entry| entry.value.as_str())
}

fn video_level_matches(expected: &str, actual: Option<&str>) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    match (parse_video_level(expected), parse_video_level(actual)) {
        (Some(expected), Some(actual)) => expected == actual,
        _ => normalized_constraint_value(actual) == normalized_constraint_value(expected),
    }
}

fn parse_video_level(value: &str) -> Option<u16> {
    let candidate = first_video_level_candidate(value.trim())?;
    if candidate.contains('.') {
        parse_dotted_video_level(candidate)
    } else {
        parse_integer_video_level(candidate)
    }
}

fn first_video_level_candidate(value: &str) -> Option<&str> {
    let lowercase = value.to_ascii_lowercase();
    if let Some(level_index) = lowercase.find("level") {
        let after_level = level_index.checked_add("level".len())?;
        if let Some(candidate) = first_numeric_video_level_candidate(&value[after_level..]) {
            return Some(candidate);
        }
    }
    first_numeric_video_level_candidate(value)
}

fn first_numeric_video_level_candidate(value: &str) -> Option<&str> {
    let start = value
        .char_indices()
        .find_map(|(index, item)| item.is_ascii_digit().then_some(index))?;
    let end = value[start..]
        .char_indices()
        .find_map(|(offset, item)| {
            (!item.is_ascii_digit() && item != '.').then_some(start + offset)
        })
        .unwrap_or(value.len());
    Some(&value[start..end])
}

fn parse_dotted_video_level(value: &str) -> Option<u16> {
    let (major, minor) = value.split_once('.')?;
    if major.is_empty() || minor.len() != 1 || minor.contains('.') {
        return None;
    }
    let major = major.parse::<u16>().ok()?;
    let minor = minor.parse::<u16>().ok()?;
    major.checked_mul(10)?.checked_add(minor)
}

fn parse_integer_video_level(value: &str) -> Option<u16> {
    if value.is_empty() {
        return None;
    }
    let parsed = value.parse::<u16>().ok()?;
    if value.len() == 1 {
        parsed.checked_mul(10)
    } else {
        Some(parsed)
    }
}

fn normalized_expected_color_constraints(
    constraint: &VideoStreamConstraints,
) -> (Option<String>, Option<String>, Option<String>) {
    let hdr_format = constraint
        .hdr_format
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if hdr_format.eq_ignore_ascii_case("hdr10") {
        return (
            constraint
                .color_primaries
                .clone()
                .or_else(|| Some("bt2020".to_string())),
            constraint
                .color_transfer
                .clone()
                .or_else(|| Some("smpte2084".to_string())),
            constraint
                .color_space
                .clone()
                .or_else(|| Some("bt2020nc".to_string())),
        );
    }
    (
        constraint.color_primaries.clone(),
        constraint.color_transfer.clone(),
        constraint.color_space.clone(),
    )
}

fn video_hdr_constraint_matches(expected: &str, stream: &StreamInspection) -> bool {
    let expected = normalized_constraint_value(expected);
    if expected != "hdr10" {
        return false;
    }
    normalized_constraint_text(stream.color_primaries.as_deref()).as_deref() == Some("bt2020")
        && normalized_constraint_text(stream.color_transfer.as_deref()).as_deref()
            == Some("smpte2084")
        && normalized_constraint_text(stream.color_space.as_deref()).as_deref() == Some("bt2020nc")
        && stream_has_side_data(stream, "mastering display metadata")
        && stream_has_side_data(stream, "content light level metadata")
}

fn stream_has_side_data(stream: &StreamInspection, expected: &str) -> bool {
    let expected = normalized_constraint_value(expected);
    stream
        .side_data_types
        .iter()
        .any(|actual| normalized_constraint_value(actual) == expected)
}

fn normalized_compact_text(value: Option<&str>) -> Option<String> {
    value
        .map(normalized_compact_value)
        .filter(|value| !value.is_empty())
}

fn normalized_compact_value(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|item| !item.is_ascii_whitespace() && *item != '-' && *item != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalized_constraint_text(value: Option<&str>) -> Option<String> {
    value
        .map(normalized_constraint_value)
        .filter(|value| !value.is_empty())
}

fn normalized_constraint_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn bitrate_within_tolerance(expected: u64, actual: u64) -> bool {
    const BITRATE_TOLERANCE_PERCENT: u128 = 5;
    let expected = u128::from(expected);
    let actual = u128::from(actual);
    let delta = expected.abs_diff(actual);
    delta.saturating_mul(100) <= expected.saturating_mul(BITRATE_TOLERANCE_PERCENT)
}

fn video_constraint_check_kind(graph_check_kind: &'static str) -> &'static str {
    match graph_check_kind {
        "candidate_graph" => "candidate_video_constraints",
        "final_graph" => "final_video_constraints",
        "source_graph" => "source_video_constraints",
        _ => "video_constraints",
    }
}

fn video_constraint_check_index(graph_check_kind: &'static str, fallback: i32) -> i32 {
    match graph_check_kind {
        "source_graph" => 2,
        "candidate_graph" => 16,
        "final_graph" => 22,
        _ => fallback,
    }
}

const fn target_stream_has_video_constraints(stream: &TargetStream) -> bool {
    stream.video_profile.is_some()
        || stream.video_level.is_some()
        || stream.video_bitrate_bps.is_some()
        || stream.color_primaries.is_some()
        || stream.color_transfer.is_some()
        || stream.color_space.is_some()
        || stream.hdr_format.is_some()
}

struct AudioConstraintVerification {
    matched: bool,
    expected: String,
    actual: String,
    details: Option<String>,
}

async fn audio_constraints_match_inspection(
    inspection: &MediaInspection,
    desired: &DesiredGraph,
    target: Option<&DesiredTargetSnapshot>,
    analyzer: Arc<RuntimeAudioAnalyzer>,
) -> Result<AudioConstraintVerification, MediaJobRuntimeError> {
    let constraints = match expected_audio_constraints(target, desired) {
        Ok(constraints) => constraints,
        Err(mismatch) => return Ok(mismatch),
    };
    if constraints.is_empty() {
        return Ok(AudioConstraintVerification {
            matched: true,
            expected: "desired_audio_constraints".to_string(),
            actual: "not_configured".to_string(),
            details: None,
        });
    }
    for constraint in &constraints {
        let Some(stream) = inspection
            .streams
            .iter()
            .find(|stream| stream.stream_id == constraint.stream_id)
        else {
            return Ok(audio_constraint_mismatch(
                constraint.stream_id,
                "stream",
                "present",
                "missing",
            ));
        };
        if let Some(mismatch) = audio_constraint_stream_mismatch(constraint, stream) {
            return Ok(mismatch);
        }
        if audio_constraint_requires_measurement(constraint) {
            let measurement = match measure_audio_stream(
                Arc::clone(&analyzer),
                inspection.graph.source_path.clone(),
                constraint.stream_id,
            )
            .await?
            {
                Ok(value) => value,
                Err(error) => {
                    return Ok(AudioConstraintVerification {
                        matched: false,
                        expected: format!(
                            "stream:{}:audio_measurement=present",
                            constraint.stream_id
                        ),
                        actual: "analysis_failed".to_string(),
                        details: Some(error),
                    });
                }
            };
            if let Some(mismatch) = audio_measurement_mismatch(constraint, measurement) {
                return Ok(mismatch);
            }
        }
    }
    Ok(AudioConstraintVerification {
        matched: true,
        expected: format!("{} constrained_audio_streams", constraints.len()),
        actual: "matched".to_string(),
        details: None,
    })
}

fn expected_audio_constraints(
    target: Option<&DesiredTargetSnapshot>,
    desired: &DesiredGraph,
) -> Result<Vec<AudioStreamConstraints>, AudioConstraintVerification> {
    let Some(target) = target else {
        return Ok(Vec::new());
    };
    let mut constraints = Vec::new();
    let mut consumed_stream_ids = BTreeSet::new();
    for target_stream in &target.target.streams {
        if target_stream.kind != StreamKind::Audio
            || !target_stream_has_audio_constraints(target_stream)
        {
            continue;
        }
        let Some(stream) =
            desired_target_stream_for_constraints(desired, target_stream, &consumed_stream_ids)
        else {
            return Err(audio_target_constraint_unmatched(target_stream));
        };
        consumed_stream_ids.insert(stream.stream_id);
        constraints.push(AudioStreamConstraints {
            stream_id: stream.stream_id,
            bitrate_bps: target_stream.audio_bitrate_bps,
            sample_rate_hz: target_stream.audio_sample_rate_hz,
            loudness_profile: target_stream.audio_loudness_profile.clone(),
            dynamic_range: target_stream.audio_dynamic_range.clone(),
        });
    }
    Ok(constraints)
}

fn audio_target_constraint_unmatched(target_stream: &TargetStream) -> AudioConstraintVerification {
    AudioConstraintVerification {
        matched: false,
        expected: format!(
            "target_stream:{}:audio_constraint=mapped",
            target_stream.stream_key
        ),
        actual: format!("unmatched:{:?}:{}", target_stream.kind, target_stream.codec),
        details: Some(format!(
            "target stream {} audio constraints could not be mapped onto the compiled desired graph",
            target_stream.stream_key
        )),
    }
}

fn audio_constraint_stream_mismatch(
    constraint: &AudioStreamConstraints,
    stream: &StreamInspection,
) -> Option<AudioConstraintVerification> {
    if let Some(expected) = constraint.bitrate_bps
        && !stream
            .bit_rate
            .is_some_and(|actual| bitrate_within_tolerance(u64::from(expected), actual))
    {
        let expected_text = expected.to_string();
        let actual_text = stream
            .bit_rate
            .map_or_else(|| "missing".to_string(), |value| value.to_string());
        return Some(audio_constraint_mismatch(
            constraint.stream_id,
            "bitrate_bps",
            &expected_text,
            &actual_text,
        ));
    }
    if let Some(expected) = constraint.sample_rate_hz
        && stream.sample_rate != Some(expected)
    {
        let expected_text = expected.to_string();
        let actual_text = stream
            .sample_rate
            .map_or_else(|| "missing".to_string(), |value| value.to_string());
        return Some(audio_constraint_mismatch(
            constraint.stream_id,
            "sample_rate_hz",
            &expected_text,
            &actual_text,
        ));
    }
    None
}

async fn measure_audio_stream(
    analyzer: Arc<RuntimeAudioAnalyzer>,
    source_path: String,
    stream_id: u32,
) -> Result<Result<AudioMeasurement, String>, MediaJobRuntimeError> {
    tokio::task::spawn_blocking(move || analyzer.measure(&source_path, stream_id))
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))
}

fn audio_constraint_requires_measurement(constraint: &AudioStreamConstraints) -> bool {
    constraint
        .loudness_profile
        .as_deref()
        .is_some_and(|profile| profile.trim().eq_ignore_ascii_case("dialog-normalized"))
        || constraint
            .dynamic_range
            .as_deref()
            .is_some_and(|range| range.trim().eq_ignore_ascii_case("speech"))
}

fn audio_measurement_mismatch(
    constraint: &AudioStreamConstraints,
    measurement: AudioMeasurement,
) -> Option<AudioConstraintVerification> {
    if constraint
        .loudness_profile
        .as_deref()
        .is_some_and(|profile| profile.trim().eq_ignore_ascii_case("dialog-normalized"))
    {
        let min_lufs = DIALOG_NORMALIZED_TARGET_LUFS - DIALOG_NORMALIZED_LUFS_TOLERANCE;
        let max_lufs = DIALOG_NORMALIZED_TARGET_LUFS + DIALOG_NORMALIZED_LUFS_TOLERANCE;
        if measurement.integrated_lufs < min_lufs || measurement.integrated_lufs > max_lufs {
            return Some(audio_constraint_mismatch(
                constraint.stream_id,
                "integrated_lufs",
                "-17.0..=-15.0",
                &format!("{:.1}", measurement.integrated_lufs),
            ));
        }
        if !measurement
            .true_peak_dbfs
            .is_some_and(|peak| peak <= DIALOG_NORMALIZED_TRUE_PEAK_MAX_DBFS)
        {
            let actual = measurement
                .true_peak_dbfs
                .map_or_else(|| "missing".to_string(), |peak| format!("{peak:.1}"));
            return Some(audio_constraint_mismatch(
                constraint.stream_id,
                "true_peak_dbfs",
                "<=-1.0",
                &actual,
            ));
        }
    }
    if constraint
        .dynamic_range
        .as_deref()
        .is_some_and(|range| range.trim().eq_ignore_ascii_case("speech"))
        && measurement.loudness_range_lu > SPEECH_DYNAMIC_RANGE_MAX_LU
    {
        return Some(audio_constraint_mismatch(
            constraint.stream_id,
            "loudness_range_lu",
            "<=12.0",
            &format!("{:.1}", measurement.loudness_range_lu),
        ));
    }
    None
}

fn parse_ebur128_summary(output: &str) -> Result<AudioMeasurement, String> {
    let mut integrated_lufs = None;
    let mut loudness_range_lu = None;
    let mut true_peak_dbfs = None;
    for line in output.lines().map(str::trim) {
        if let Some(value) = parse_prefixed_float(line, "I:", "LUFS") {
            integrated_lufs = Some(value);
        } else if let Some(value) = parse_prefixed_float(line, "LRA:", "LU") {
            loudness_range_lu = Some(value);
        } else if let Some(value) = parse_prefixed_float(line, "Peak:", "dBFS") {
            true_peak_dbfs = Some(value);
        }
    }
    let integrated_lufs = integrated_lufs
        .ok_or_else(|| "audio analyzer output missing integrated LUFS".to_string())?;
    let loudness_range_lu = loudness_range_lu
        .ok_or_else(|| "audio analyzer output missing loudness range".to_string())?;
    Ok(AudioMeasurement {
        integrated_lufs,
        loudness_range_lu,
        true_peak_dbfs,
    })
}

fn parse_prefixed_float(line: &str, prefix: &str, suffix: &str) -> Option<f64> {
    let value = line.strip_prefix(prefix)?.trim();
    if !value.ends_with(suffix) {
        return None;
    }
    value.trim_end_matches(suffix).trim().parse::<f64>().ok()
}

fn audio_constraint_mismatch(
    stream_id: u32,
    field: &str,
    expected: &str,
    actual: &str,
) -> AudioConstraintVerification {
    AudioConstraintVerification {
        matched: false,
        expected: format!("stream:{stream_id}:{field}={expected}"),
        actual: actual.to_string(),
        details: Some(format!("stream {stream_id} audio {field} mismatch")),
    }
}

fn audio_constraint_check_kind(graph_check_kind: &'static str) -> &'static str {
    match graph_check_kind {
        "candidate_graph" => "candidate_audio_constraints",
        "final_graph" => "final_audio_constraints",
        "source_graph" => "source_audio_constraints",
        _ => "audio_constraints",
    }
}

fn audio_constraint_check_index(graph_check_kind: &'static str, fallback: i32) -> i32 {
    match graph_check_kind {
        "source_graph" => 3,
        "candidate_graph" => 17,
        "final_graph" => 23,
        _ => fallback,
    }
}

const fn target_stream_has_audio_constraints(stream: &TargetStream) -> bool {
    stream.audio_bitrate_bps.is_some()
        || stream.audio_sample_rate_hz.is_some()
        || stream.audio_loudness_profile.is_some()
        || stream.audio_dynamic_range.is_some()
}

struct ChapterTimelineVerification {
    matched: bool,
    expected: String,
    actual: String,
    details: Option<String>,
}

fn chapter_timeline_matches_inspection(
    inspection: &MediaInspection,
    expected_chapters: &[ChapterInspection],
) -> ChapterTimelineVerification {
    if expected_chapters.is_empty() {
        return ChapterTimelineVerification {
            matched: true,
            expected: "source_chapters".to_string(),
            actual: "not_present".to_string(),
            details: None,
        };
    }
    let expected = normalized_chapter_timeline(expected_chapters);
    let actual = normalized_chapter_timeline(&inspection.chapters);
    let matched = expected == actual;
    ChapterTimelineVerification {
        matched,
        expected: format!("{} source_chapters", expected.len()),
        actual: if matched {
            "matched".to_string()
        } else {
            format!("{} output_chapters", actual.len())
        },
        details: (!matched).then(|| {
            format!(
                "chapter timeline mismatch for {}; expected {} but found {}",
                inspection.graph.source_path,
                expected.len(),
                actual.len()
            )
        }),
    }
}

fn normalized_chapter_timeline(chapters: &[ChapterInspection]) -> Vec<NormalizedChapter> {
    chapters
        .iter()
        .map(|chapter| NormalizedChapter {
            start_millis: chapter.start_millis,
            end_millis: chapter.end_millis,
            metadata: normalized_metadata_entries(&chapter.metadata),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedChapter {
    start_millis: i64,
    end_millis: i64,
    metadata: Vec<(String, String)>,
}

fn normalized_metadata_entries(entries: &[MetadataEntry]) -> Vec<(String, String)> {
    let mut normalized = entries
        .iter()
        .filter_map(|entry| {
            let key = normalized_optional_text(Some(&entry.key))?.to_ascii_lowercase();
            let value = normalized_optional_text(Some(&entry.value))?;
            Some((key, value))
        })
        .collect::<Vec<_>>();
    normalized.sort();
    normalized
}

fn chapter_timeline_check_kind(graph_check_kind: &'static str) -> &'static str {
    match graph_check_kind {
        "source_graph" => "source_chapters",
        "candidate_graph" => "candidate_chapters",
        "final_graph" => "final_chapters",
        _ => "chapter_timeline",
    }
}

fn chapter_timeline_check_index(graph_check_kind: &'static str, fallback: i32) -> i32 {
    match graph_check_kind {
        "source_graph" => 4,
        "candidate_graph" => 18,
        "final_graph" => 24,
        _ => fallback,
    }
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

fn feature_supported(
    features: &[revaer_data::media::capabilities::CapabilityFeatureRow],
    family: &str,
    name: &str,
) -> bool {
    features.iter().any(|feature| {
        feature.supported
            && feature.feature_family.eq_ignore_ascii_case(family)
            && feature.feature_name.eq_ignore_ascii_case(name)
    })
}

fn sidecar_inputs(
    sidecars: &[SidecarSubtitle],
) -> Result<Vec<SidecarSubtitleInput>, MediaJobRuntimeError> {
    sidecars.iter().map(sidecar_input).collect()
}

fn sidecar_input(sidecar: &SidecarSubtitle) -> Result<SidecarSubtitleInput, MediaJobRuntimeError> {
    Ok(SidecarSubtitleInput {
        path: path_to_string(&sidecar.path, "media_sidecar_path_invalid")?,
        companion_path: sidecar
            .companion_path
            .as_deref()
            .map(|path| path_to_string(path, "media_sidecar_companion_path_invalid"))
            .transpose()?,
        language: sidecar.language.clone(),
        role: sidecar.role.map(sidecar_semantic_role),
        codec: sidecar_codec(sidecar.format).to_string(),
        image_based: sidecar.format.image_based(),
    })
}

const fn sidecar_semantic_role(role: SidecarRole) -> SemanticRole {
    match role {
        SidecarRole::Forced => SemanticRole::Forced,
        SidecarRole::Commentary => SemanticRole::Commentary,
        SidecarRole::Sdh => SemanticRole::Sdh,
        SidecarRole::SignsSongs => SemanticRole::SignsSongs,
        SidecarRole::Karaoke => SemanticRole::Karaoke,
    }
}

const fn sidecar_codec(format: SidecarFormat) -> &'static str {
    match format {
        SidecarFormat::Srt => "subrip",
        SidecarFormat::Ass => "ass",
        SidecarFormat::Vtt => "webvtt",
        SidecarFormat::Sup => "hdmv_pgs_subtitle",
        SidecarFormat::Sub => "microdvd",
        SidecarFormat::VobSub => "dvd_subtitle",
    }
}

fn replacement_artifact_paths(
    outputs: &[DesiredSidecarOutput],
    removals: &[String],
) -> Vec<(PathBuf, Option<PathBuf>)> {
    let mut artifacts = Vec::with_capacity(outputs.len() * 2 + removals.len());
    for output in outputs {
        artifacts.push((
            PathBuf::from(&output.destination_path),
            Some(PathBuf::from(&output.path)),
        ));
        if let (Some(destination), Some(candidate)) = (
            output.destination_companion_path.as_deref(),
            output.companion_path.as_deref(),
        ) {
            artifacts.push((PathBuf::from(destination), Some(PathBuf::from(candidate))));
        }
    }
    artifacts.extend(removals.iter().map(|path| (PathBuf::from(path), None)));
    artifacts
}

fn sidecar_state_matches(
    inspection: &MediaInspection,
    outputs: &[DesiredSidecarOutput],
    removals: &[String],
) -> bool {
    let actual = inspection
        .sidecars
        .iter()
        .flat_map(|sidecar| {
            std::iter::once(sidecar.path.as_path()).chain(sidecar.companion_path.as_deref())
        })
        .collect::<BTreeSet<_>>();
    outputs.iter().all(|output| {
        actual.contains(Path::new(&output.destination_path))
            && output
                .destination_companion_path
                .as_deref()
                .is_none_or(|path| actual.contains(Path::new(path)))
    }) && removals
        .iter()
        .all(|path| !actual.contains(Path::new(path)))
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
    desired_container_matches(actual, desired)
        && actual.streams.len() == desired.streams.len()
        && actual
            .streams
            .iter()
            .zip(&desired.streams)
            .all(stream_matches_desired)
}

fn desired_container_matches(actual: &MediaGraph, desired: &DesiredGraph) -> bool {
    desired
        .container_format
        .as_deref()
        .is_none_or(|desired_format| {
            let desired = normalize_container_format(desired_format);
            actual
                .container_formats
                .iter()
                .any(|actual_format| normalize_container_format(actual_format) == desired)
        })
}

fn stream_matches_desired((actual_stream, desired_stream): (&MediaStream, &MediaStream)) -> bool {
    actual_stream.kind == desired_stream.kind
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
        && desired_channel_count_matches(actual_stream.channels, desired_stream.channels)
        && desired_channel_layout_matches(
            actual_stream.channel_layout.as_deref(),
            desired_stream.channel_layout.as_deref(),
        )
}

fn normalized_language(value: Option<&str>) -> Option<String> {
    normalized_optional_text(value).map(|item| item.to_ascii_lowercase())
}

fn desired_channel_count_matches(actual: Option<u32>, desired: Option<u32>) -> bool {
    desired.is_none_or(|desired_channels| actual == Some(desired_channels))
}

fn desired_channel_layout_matches(actual: Option<&str>, desired: Option<&str>) -> bool {
    normalized_channel_layout(desired)
        .is_none_or(|desired_layout| normalized_channel_layout(actual) == Some(desired_layout))
}

fn normalized_channel_layout(value: Option<&str>) -> Option<String> {
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

fn source_artifact_bytes(
    source_path: &str,
    sidecars: &[SidecarSubtitle],
) -> Result<u64, MediaJobRuntimeError> {
    let paths = std::iter::once(Path::new(source_path))
        .chain(sidecars.iter().flat_map(|sidecar| {
            std::iter::once(sidecar.path.as_path()).chain(sidecar.companion_path.as_deref())
        }))
        .collect::<BTreeSet<_>>();
    paths.into_iter().try_fold(0_u64, |total, path| {
        let bytes = fs::metadata(path)
            .map(|metadata| metadata.len())
            .map_err(|source| MediaJobRuntimeError::SourceMetadata {
                path: path.to_string_lossy().into_owned(),
                source,
            })?;
        total
            .checked_add(bytes)
            .ok_or(MediaJobRuntimeError::SourceSizeOverflow)
    })
}

const fn operation_kind_code(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::NoOp => "no_op",
        OperationKind::Remux => "remux",
        OperationKind::MetadataRewrite => "metadata_rewrite",
        OperationKind::DispositionRewrite => "disposition_rewrite",
        OperationKind::LabelRewrite => "label_rewrite",
        OperationKind::StreamReorder => "stream_reorder",
        OperationKind::EmbedSubtitle => "embed_subtitle",
        OperationKind::ExtractSubtitle => "extract_subtitle",
        OperationKind::CopySidecarSubtitle => "copy_sidecar_subtitle",
        OperationKind::RemoveSidecarSubtitle => "remove_sidecar_subtitle",
        OperationKind::SubtitleTranscode => "subtitle_transcode",
        OperationKind::AudioTranscode => "audio_transcode",
        OperationKind::VideoTranscode => "video_transcode",
    }
}

fn planned_job_is_noop(report: &JobPreflightReport) -> bool {
    matches!(
        report.planned.operations.as_slice(),
        [PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None
        }]
    )
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

fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, |converted| converted)
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
        AudioAnalysisAdapter, AudioMeasurement, AudioStreamConstraints, DesiredTargetSnapshot,
        FilesystemCapacityProbe, MediaJobRuntime, MediaJobRuntimeComponents, RuntimeAudioAnalyzer,
        RuntimeCapacityProbe, RuntimeCommandRunner, RuntimeInspector, RuntimeReplacementCommitter,
        RuntimeVerificationExecutor, SystemFfmpegAudioAnalysisAdapter, VideoStreamConstraints,
        audio_measurement_mismatch, expected_audio_constraints, parse_ebur128_summary,
        verification_policy_from_job, video_constraint_stream_mismatch,
        video_policy_from_policy_intent, video_policy_from_target_snapshot,
    };
    use revaer_data::indexers::app_users::{app_user_create, app_user_verify_email};
    use revaer_data::media::capabilities::{
        RecordCapabilityEncoderInput, RecordCapabilityFeatureInput, RecordCapabilitySnapshotInput,
        complete_capability_snapshot_run_with_executor, record_capability_encoder_with_executor,
        record_capability_feature_with_executor, record_capability_snapshot_with_executor,
        start_capability_snapshot_run_with_executor,
    };
    use revaer_data::media::configuration::{
        AppendMediaDesiredTargetStreamInput, CreateMediaDesiredTargetInput,
        append_media_desired_target_stream, create_media_desired_target,
        set_media_profile_desired_target,
    };
    use revaer_data::media::jobs::{ClaimedMediaJobRow, CreateMediaJobInput};
    use revaer_data::media::profiles::{UpdateMediaProfileInput, UpsertMediaProfileInput};
    use revaer_events::{Event as CoreEvent, EventBus};
    use revaer_media_core::classify::SemanticRole;
    use revaer_media_core::compliance::{Status, report_for_status};
    use revaer_media_core::explain::Explanation;
    use revaer_media_core::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};
    use revaer_media_core::plan::{OperationKind, PlannedOperation};
    use revaer_media_core::target::{
        DesiredSidecarOutput, DesiredTarget, SidecarOutputSource, TargetStream,
        UnmatchedStreamPolicy,
    };
    use revaer_media_runtime::execute::{
        CommandRunner, ExecuteStepError, ExecutionControl, ExecutionStep,
    };
    use revaer_media_runtime::inspect::{
        ChapterInspection, ContainerInspection, InspectAdapter, InspectError, MediaInspection,
        MetadataEntry, StreamInspection,
    };
    use revaer_media_runtime::jobs::{JobPreflightReport, PlannedJob, PlannedJobSummary};
    use revaer_media_runtime::replacement::{
        CommittedReplacement, PreparedReplacement, RecoveredReplacement, ReplacementCommitter,
        ReplacementError, ReplacementRequest, SystemReplacementCommitter,
    };
    use revaer_media_runtime::sidecar::{SidecarFormat, SidecarRole, SidecarSubtitle};
    use revaer_media_runtime::verification::{VerificationExecutionError, VerificationExecutor};
    use revaer_media_runtime::workspace::{
        WorkspaceCapacityReport, WorkspacePolicy, WorkspaceRejectionReason,
    };
    use revaer_runtime::media::MediaStore;
    use revaer_telemetry::Metrics;
    use revaer_test_support::postgres::TestDatabase;
    use revaer_test_support::postgres::start_postgres;
    use sqlx::postgres::PgPoolOptions;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::TempDir;
    use tokio_stream::StreamExt;
    use uuid::Uuid;

    #[derive(Clone)]
    struct StaticInspector;

    impl InspectAdapter for StaticInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            Ok(video_graph(source_path, "h264"))
        }

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            let mut inspection = self.inspect(source_path).map(complete_test_inspection)?;
            for stream in &mut inspection.streams {
                if stream.sample_rate == Some(48_000) {
                    stream.bit_rate = Some(160_000);
                }
            }
            Ok(inspection)
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

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            let mut inspection = self.inspect(source_path).map(complete_test_inspection)?;
            for stream in &mut inspection.streams {
                if stream.sample_rate == Some(48_000) {
                    stream.bit_rate = Some(160_000);
                }
            }
            Ok(inspection)
        }
    }

    #[derive(Clone)]
    struct SuccessfulTranscodeInspector;

    impl InspectAdapter for SuccessfulTranscodeInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            let codec = match fs::read(source_path) {
                Ok(bytes) if bytes.as_slice() == b"output" => "hevc",
                Ok(_) | Err(_) => "h264",
            };
            Ok(video_graph(source_path, codec))
        }

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            self.inspect(source_path).map(complete_test_inspection)
        }
    }

    #[derive(Clone)]
    struct CandidateDropsChaptersInspector;

    impl InspectAdapter for CandidateDropsChaptersInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            let codec = match fs::read(source_path) {
                Ok(bytes) if bytes.as_slice() == b"output" => "hevc",
                Ok(_) | Err(_) => "h264",
            };
            Ok(video_graph(source_path, codec))
        }

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            let inspection = self.inspect(source_path).map(complete_test_inspection)?;
            if source_path.contains("/workspace/") {
                Ok(inspection)
            } else {
                Ok(with_test_chapters(inspection))
            }
        }
    }

    #[derive(Clone)]
    struct PostCommitDropsChaptersInspector;

    impl InspectAdapter for PostCommitDropsChaptersInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            let codec = if source_path.contains("/workspace/") {
                "hevc"
            } else {
                match fs::read(source_path) {
                    Ok(bytes) if bytes.as_slice() == b"output" => "hevc",
                    Ok(_) | Err(_) => "h264",
                }
            };
            Ok(video_graph(source_path, codec))
        }

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            let inspection = self.inspect(source_path).map(complete_test_inspection)?;
            if source_path.contains("/workspace/")
                || fs::read(source_path).is_ok_and(|bytes| bytes.as_slice() == b"source")
            {
                Ok(with_test_chapters(inspection))
            } else {
                Ok(inspection)
            }
        }
    }

    #[derive(Clone)]
    struct VideoConstraintMismatchInspector;

    impl InspectAdapter for VideoConstraintMismatchInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            let codec = match fs::read(source_path) {
                Ok(bytes) if bytes.as_slice() == b"output" => "hevc",
                Ok(_) | Err(_) => "h264",
            };
            Ok(video_graph(source_path, codec))
        }

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            let mut inspection = self.inspect(source_path).map(complete_test_inspection)?;
            for stream in &mut inspection.streams {
                if stream.profile.as_deref() == Some("Main 10") {
                    stream.color_transfer = Some("bt709".to_string());
                }
            }
            Ok(inspection)
        }
    }

    #[derive(Clone)]
    struct HdrSideDataMismatchInspector;

    impl InspectAdapter for HdrSideDataMismatchInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            let codec = match fs::read(source_path) {
                Ok(bytes) if bytes.as_slice() == b"output" => "hevc",
                Ok(_) | Err(_) => "h264",
            };
            Ok(video_graph(source_path, codec))
        }

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            let mut inspection = self.inspect(source_path).map(complete_test_inspection)?;
            for stream in &mut inspection.streams {
                if stream.profile.as_deref() == Some("Main 10") {
                    stream.side_data_types.clear();
                }
            }
            Ok(inspection)
        }
    }

    #[derive(Clone)]
    struct AudioConstraintMismatchInspector;

    impl InspectAdapter for AudioConstraintMismatchInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            let (video_codec, audio_codec) = match fs::read(source_path) {
                Ok(bytes) if bytes.as_slice() == b"source" => ("h264", "mp3"),
                Ok(_) | Err(_) => ("hevc", "aac"),
            };
            Ok(av_graph(source_path, video_codec, audio_codec))
        }

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            let mut inspection = self.inspect(source_path).map(complete_test_inspection)?;
            for stream in &mut inspection.streams {
                if stream.sample_rate == Some(48_000) {
                    stream.bit_rate = Some(160_000);
                    stream.sample_rate = Some(44_100);
                }
            }
            Ok(inspection)
        }
    }

    #[derive(Clone)]
    struct AudioPolicyInspector;

    impl InspectAdapter for AudioPolicyInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            let (video_codec, audio_codec) = match fs::read(source_path) {
                Ok(bytes) if bytes.as_slice() == b"source" => ("h264", "mp3"),
                Ok(_) | Err(_) => ("hevc", "aac"),
            };
            Ok(av_graph(source_path, video_codec, audio_codec))
        }

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            let mut inspection = self.inspect(source_path).map(complete_test_inspection)?;
            for stream in &mut inspection.streams {
                if stream.sample_rate == Some(48_000) {
                    stream.bit_rate = Some(160_000);
                }
            }
            Ok(inspection)
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct StaticAudioAnalyzer {
        measurement: Result<AudioMeasurement, &'static str>,
    }

    impl AudioAnalysisAdapter for StaticAudioAnalyzer {
        fn measure(&self, _source_path: &str, _stream_id: u32) -> Result<AudioMeasurement, String> {
            self.measurement.map_err(str::to_string)
        }
    }

    #[derive(Clone)]
    struct PostCommitMismatchInspector;

    impl InspectAdapter for PostCommitMismatchInspector {
        fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
            let codec = if source_path.contains("/workspace/") {
                "hevc"
            } else {
                match fs::read(source_path) {
                    Ok(bytes) if bytes.as_slice() == b"output" => "vp9",
                    Ok(_) | Err(_) => "h264",
                }
            };
            Ok(video_graph(source_path, codec))
        }

        fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
            self.inspect(source_path).map(complete_test_inspection)
        }
    }

    fn complete_test_inspection(mut graph: MediaGraph) -> MediaInspection {
        graph.container_formats = vec!["matroska".to_string()];
        let streams = graph
            .streams
            .iter()
            .map(|stream| StreamInspection {
                stream_id: stream.stream_id,
                profile: constrained_test_video_profile(stream),
                duration_millis: Some(1_000),
                bit_rate: constrained_test_video_bitrate(stream).or(Some(8_192)),
                sample_rate: (stream.kind == StreamKind::Audio).then_some(48_000),
                width: (stream.kind == StreamKind::Video).then_some(1920),
                height: (stream.kind == StreamKind::Video).then_some(1080),
                pixel_format: (stream.kind == StreamKind::Video).then(|| "yuv420p".to_string()),
                sample_aspect_ratio: None,
                display_aspect_ratio: None,
                average_frame_rate: None,
                color_range: None,
                color_space: constrained_test_video_color(stream, "bt2020nc"),
                color_transfer: constrained_test_video_color(stream, "smpte2084"),
                color_primaries: constrained_test_video_color(stream, "bt2020"),
                chroma_location: None,
                field_order: None,
                metadata: constrained_test_video_metadata(stream),
                side_data_types: constrained_test_video_side_data(stream),
            })
            .collect();
        MediaInspection {
            graph,
            container: ContainerInspection {
                formats: vec!["matroska".to_string()],
                duration_millis: Some(1_000),
                start_time_millis: Some(0),
                size_bytes: Some(6),
                bit_rate: Some(8_192),
                metadata: Vec::new(),
            },
            streams,
            chapters: Vec::new(),
            sidecars: Vec::new(),
        }
    }

    fn with_test_chapters(mut inspection: MediaInspection) -> MediaInspection {
        inspection.chapters = vec![
            ChapterInspection {
                chapter_id: 0,
                start_millis: 0,
                end_millis: 500,
                metadata: vec![MetadataEntry {
                    key: "title".to_string(),
                    value: "Opening".to_string(),
                }],
            },
            ChapterInspection {
                chapter_id: 1,
                start_millis: 500,
                end_millis: 1_000,
                metadata: vec![MetadataEntry {
                    key: "title".to_string(),
                    value: "Main".to_string(),
                }],
            },
        ];
        inspection
    }

    fn constrained_test_video_profile(stream: &MediaStream) -> Option<String> {
        (stream.kind == StreamKind::Video && stream.codec == "hevc").then(|| "Main 10".to_string())
    }

    fn constrained_test_video_bitrate(stream: &MediaStream) -> Option<u64> {
        (stream.kind == StreamKind::Video && stream.codec == "hevc").then_some(7_900_000)
    }

    fn constrained_test_video_color(stream: &MediaStream, value: &str) -> Option<String> {
        (stream.kind == StreamKind::Video && stream.codec == "hevc").then(|| value.to_string())
    }

    fn constrained_test_video_metadata(stream: &MediaStream) -> Vec<MetadataEntry> {
        if stream.kind == StreamKind::Video && stream.codec == "hevc" {
            vec![MetadataEntry {
                key: "level".to_string(),
                value: "5.1".to_string(),
            }]
        } else {
            Vec::new()
        }
    }

    fn constrained_test_video_side_data(stream: &MediaStream) -> Vec<String> {
        if stream.kind == StreamKind::Video && stream.codec == "hevc" {
            vec![
                "content light level metadata".to_string(),
                "mastering display metadata".to_string(),
            ]
        } else {
            Vec::new()
        }
    }

    fn constrained_target_snapshot(stream: TargetStream) -> DesiredTargetSnapshot {
        DesiredTargetSnapshot {
            target: DesiredTarget {
                target_key: "test-target".to_string(),
                version: 1,
                container: "matroska".to_string(),
                streams: vec![stream],
            },
            unmatched_stream_policy: UnmatchedStreamPolicy::Preserve,
        }
    }

    fn target_stream(stream_key: &str, kind: StreamKind, codec: &str) -> TargetStream {
        TargetStream {
            stream_key: stream_key.to_string(),
            kind,
            role: None,
            language: None,
            optional: false,
            codec: codec.to_string(),
            channels: None,
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
            dispositions: Vec::new(),
            subtitle_placement: None,
            image_subtitle_action: None,
        }
    }

    fn video_level_constraint(expected_level: &str) -> VideoStreamConstraints {
        VideoStreamConstraints {
            stream_id: 0,
            profile: None,
            level: Some(expected_level.to_string()),
            bitrate_bps: None,
            color_primaries: None,
            color_transfer: None,
            color_space: None,
            hdr_format: None,
        }
    }

    fn video_stream_inspection_with_level(actual_level: &str) -> StreamInspection {
        StreamInspection {
            stream_id: 0,
            profile: None,
            duration_millis: None,
            bit_rate: None,
            sample_rate: None,
            width: None,
            height: None,
            pixel_format: None,
            sample_aspect_ratio: None,
            display_aspect_ratio: None,
            average_frame_rate: None,
            color_range: None,
            color_space: None,
            color_transfer: None,
            color_primaries: None,
            chroma_location: None,
            field_order: None,
            metadata: vec![MetadataEntry {
                key: "level".to_string(),
                value: actual_level.to_string(),
            }],
            side_data_types: Vec::new(),
        }
    }

    #[test]
    fn video_level_constraint_accepts_ffprobe_integer_equivalent() {
        let constraint = video_level_constraint("5.1");
        let stream = video_stream_inspection_with_level("51");

        assert!(video_constraint_stream_mismatch(&constraint, &stream).is_none());
    }

    #[test]
    fn video_level_constraint_accepts_labeled_level_equivalent() {
        let constraint = video_level_constraint("51");
        let stream = video_stream_inspection_with_level("Level 5.1");

        assert!(video_constraint_stream_mismatch(&constraint, &stream).is_none());
    }

    #[test]
    fn video_level_constraint_prefers_labeled_level_over_codec_digits() {
        let constraint = video_level_constraint("51");
        let stream = video_stream_inspection_with_level("h264 level 5.1");

        assert!(video_constraint_stream_mismatch(&constraint, &stream).is_none());
    }

    #[test]
    fn video_level_constraint_rejects_non_equivalent_level() {
        let constraint = video_level_constraint("5.2");
        let stream = video_stream_inspection_with_level("51");
        let mismatch = video_constraint_stream_mismatch(&constraint, &stream)
            .expect("different video levels should fail verification");

        assert_eq!(mismatch.expected, "stream:0:level=5.2");
        assert_eq!(mismatch.actual, "51");
    }

    #[test]
    fn video_policy_rejects_unmatched_target_constraint_stream() -> anyhow::Result<()> {
        let desired = DesiredGraph {
            output_path: "/tmp/output.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            streams: video_graph("/tmp/source.mkv", "h264").streams,
        };
        let mut stream = target_stream("main-video", StreamKind::Video, "hevc");
        stream.video_profile = Some("main10".to_string());
        let snapshot = constrained_target_snapshot(stream);
        let base_policy = video_policy_from_policy_intent(Some("general"))?;

        let error = video_policy_from_target_snapshot(base_policy, Some(&snapshot), &desired)
            .expect_err("unmatched constrained target video stream should fail closed");

        assert_eq!(error.code(), "media_job_target_constraint_stream_unmatched");
        Ok(())
    }

    #[test]
    fn audio_constraint_mapping_rejects_unmatched_target_constraint_stream() {
        let desired = DesiredGraph {
            output_path: "/tmp/output.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            streams: video_graph("/tmp/source.mkv", "h264").streams,
        };
        let mut stream = target_stream("dialog-audio", StreamKind::Audio, "aac");
        stream.audio_loudness_profile = Some("dialog-normalized".to_string());
        let snapshot = constrained_target_snapshot(stream);

        let mismatch = expected_audio_constraints(Some(&snapshot), &desired)
            .expect_err("unmatched constrained target audio stream should fail closed");

        assert!(!mismatch.matched);
        assert_eq!(
            mismatch.expected,
            "target_stream:dialog-audio:audio_constraint=mapped"
        );
        assert_eq!(mismatch.actual, "unmatched:Audio:aac");
    }

    #[derive(Default)]
    struct RecordingCommandRunner {
        commands: Mutex<Vec<Vec<String>>>,
    }

    #[derive(Debug, Default)]
    struct CancellationAwareCommandRunner {
        started: AtomicBool,
        cancellation_observed: AtomicBool,
    }

    impl CommandRunner for CancellationAwareCommandRunner {
        fn run(&self, bin: &str, _argv: &[String]) -> Result<(), ExecuteStepError> {
            Err(ExecuteStepError::CommandFailed {
                bin: format!("uncontrolled_test_runner:{bin}"),
                status_code: None,
            })
        }

        fn run_controlled(
            &self,
            bin: &str,
            _argv: &[String],
            control: &dyn ExecutionControl,
        ) -> Result<(), ExecuteStepError> {
            self.started.store(true, Ordering::Release);
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if control.cancellation_requested() {
                    self.cancellation_observed.store(true, Ordering::Release);
                    return Err(ExecuteStepError::Cancelled);
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(ExecuteStepError::CommandFailed {
                bin: format!("cancellation_timeout:{bin}"),
                status_code: None,
            })
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct PassingVerificationExecutor;

    impl VerificationExecutor for PassingVerificationExecutor {
        fn run(&self, _bin: &str, _argv: &[String]) -> Result<(), String> {
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct FailingDecodeVerificationExecutor;

    impl VerificationExecutor for FailingDecodeVerificationExecutor {
        fn run(&self, _bin: &str, argv: &[String]) -> Result<(), String> {
            let full_decode = argv.windows(2).any(|pair| pair == ["-map", "0"]);
            if full_decode {
                Err("truncated packet detected".to_string())
            } else {
                Ok(())
            }
        }
    }

    #[derive(Debug, Default)]
    struct CancellationAwareVerificationExecutor {
        started: AtomicBool,
        cancellation_observed: AtomicBool,
    }

    impl VerificationExecutor for CancellationAwareVerificationExecutor {
        fn run(&self, bin: &str, _argv: &[String]) -> Result<(), String> {
            Err(format!("uncontrolled verification invocation: {bin}"))
        }

        fn run_controlled(
            &self,
            _bin: &str,
            _argv: &[String],
            control: &dyn ExecutionControl,
        ) -> Result<(), VerificationExecutionError> {
            self.started.store(true, Ordering::Release);
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if control.cancellation_requested() {
                    self.cancellation_observed.store(true, Ordering::Release);
                    return Err(VerificationExecutionError::Cancelled);
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(VerificationExecutionError::Failed(
                "verification cancellation timeout".to_string(),
            ))
        }
    }

    #[derive(Debug, Default)]
    struct PreCommitCancellationCommitter {
        inner: SystemReplacementCommitter,
        prepared: AtomicBool,
        release_prepare: AtomicBool,
        discarded: AtomicBool,
        committed: AtomicBool,
    }

    impl ReplacementCommitter for PreCommitCancellationCommitter {
        fn prepare(
            &self,
            request: ReplacementRequest<'_>,
        ) -> Result<PreparedReplacement, ReplacementError> {
            let prepared = self.inner.prepare(request)?;
            self.prepared.store(true, Ordering::Release);
            let deadline = Instant::now() + Duration::from_secs(5);
            while !self.release_prepare.load(Ordering::Acquire) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            Ok(prepared)
        }

        fn commit(
            &self,
            prepared: PreparedReplacement,
        ) -> Result<CommittedReplacement, ReplacementError> {
            self.committed.store(true, Ordering::Release);
            self.inner.commit(prepared)
        }

        fn discard_prepared(&self, prepared: PreparedReplacement) -> Result<(), ReplacementError> {
            self.discarded.store(true, Ordering::Release);
            self.inner.discard_prepared(prepared)
        }

        fn rollback(&self, committed: CommittedReplacement) -> Result<(), ReplacementError> {
            self.inner.rollback(committed)
        }

        fn finalize(&self, committed: CommittedReplacement) -> Result<(), ReplacementError> {
            self.inner.finalize(committed)
        }

        fn recover(
            &self,
            source_root: &Path,
        ) -> Result<Vec<RecoveredReplacement>, ReplacementError> {
            self.inner.recover(source_root)
        }
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
        events: EventBus,
        telemetry: Metrics,
        command_runner: Arc<RecordingCommandRunner>,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum RuntimeJobTarget {
        SourceGraph,
        Hevc,
        HevcAudio,
    }

    async fn create_runtime_target(
        store: &MediaStore,
        actor: Uuid,
        job_target: RuntimeJobTarget,
    ) -> anyhow::Result<Option<(&'static str, i32)>> {
        match job_target {
            RuntimeJobTarget::SourceGraph => Ok(None),
            RuntimeJobTarget::Hevc | RuntimeJobTarget::HevcAudio => {
                let target_id = create_media_desired_target(
                    store.pool(),
                    CreateMediaDesiredTargetInput {
                        actor_public_id: actor,
                        target_key: "runtime-hevc",
                        version: 1,
                        display_name: "Runtime HEVC",
                        container_format: "matroska",
                    },
                )
                .await?;
                append_media_desired_target_stream(
                    store.pool(),
                    AppendMediaDesiredTargetStreamInput {
                        media_desired_target_profile_public_id: target_id,
                        stream_key: "video-main",
                        stream_kind: "video",
                        semantic_role: None,
                        language_code: None,
                        optional: false,
                        sort_order: 0,
                        codec: "hevc",
                        channel_count: None,
                        channel_layout: None,
                        audio_bitrate_bps: None,
                        audio_sample_rate_hz: None,
                        audio_loudness_profile: None,
                        audio_dynamic_range: None,
                        video_profile: Some("main10"),
                        video_level: Some("5.1"),
                        video_bitrate_bps: Some(8_000_000),
                        color_primaries: Some("bt2020"),
                        color_transfer: Some("smpte2084"),
                        color_space: Some("bt2020nc"),
                        hdr_format: Some("hdr10"),
                        title: None,
                        default_disposition: false,
                        forced_disposition: false,
                        subtitle_placement: None,
                        image_subtitle_action: None,
                    },
                )
                .await?;
                if job_target == RuntimeJobTarget::HevcAudio {
                    append_media_desired_target_stream(
                        store.pool(),
                        AppendMediaDesiredTargetStreamInput {
                            media_desired_target_profile_public_id: target_id,
                            stream_key: "audio-main",
                            stream_kind: "audio",
                            semantic_role: Some("primary"),
                            language_code: Some("eng"),
                            optional: false,
                            sort_order: 1,
                            codec: "aac",
                            channel_count: Some(2),
                            channel_layout: Some("stereo"),
                            audio_bitrate_bps: Some(160_000),
                            audio_sample_rate_hz: Some(48_000),
                            audio_loudness_profile: Some("dialog-normalized"),
                            audio_dynamic_range: Some("speech"),
                            video_profile: None,
                            video_level: None,
                            video_bitrate_bps: None,
                            color_primaries: None,
                            color_transfer: None,
                            color_space: None,
                            hdr_format: None,
                            title: None,
                            default_disposition: true,
                            forced_disposition: false,
                            subtitle_placement: None,
                            image_subtitle_action: None,
                        },
                    )
                    .await?;
                }
                Ok(Some(("runtime-hevc", 1)))
            }
        }
    }

    async fn pin_runtime_target(
        store: &MediaStore,
        actor: Uuid,
        profile_id: Uuid,
        desired_target: Option<(&str, i32)>,
    ) -> anyhow::Result<()> {
        if let Some((target_key, version)) = desired_target {
            set_media_profile_desired_target(
                store.pool(),
                actor,
                profile_id,
                Some(target_key),
                Some(version),
            )
            .await?;
        }
        Ok(())
    }

    fn video_graph(source_path: &str, codec: &str) -> MediaGraph {
        MediaGraph {
            source_path: source_path.to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: codec.to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        }
    }

    fn av_graph(source_path: &str, video_codec: &str, audio_codec: &str) -> MediaGraph {
        let mut graph = video_graph(source_path, video_codec);
        graph.streams.push(MediaStream {
            stream_id: 1,
            kind: StreamKind::Audio,
            codec: audio_codec.to_string(),
            channels: Some(2),
            channel_layout: Some("stereo".to_string()),
            language: Some("eng".to_string()),
            title: None,
            dispositions: vec!["default".to_string()],
        });
        graph
    }

    async fn setup_runtime(
        dry_run: bool,
        record_capability: bool,
        job_target: RuntimeJobTarget,
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
        let desired_target = create_runtime_target(&store, actor, job_target).await?;
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
        pin_runtime_target(&store, actor, profile_id, desired_target).await?;
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
        let events = EventBus::with_capacity(16);
        let telemetry = Metrics::new()?;
        let runtime = test_runtime(
            store.clone(),
            Arc::clone(&command_runner),
            events.clone(),
            telemetry.clone(),
            workspace_root,
        );
        Ok(Some(RuntimeFixture {
            _postgres: postgres,
            _temp: temp,
            runtime,
            store,
            job_id,
            events,
            telemetry,
            command_runner,
        }))
    }

    fn test_runtime(
        store: MediaStore,
        command_runner: Arc<RecordingCommandRunner>,
        events: EventBus,
        telemetry: Metrics,
        workspace_root: PathBuf,
    ) -> MediaJobRuntime {
        MediaJobRuntime::with_components(
            store,
            MediaJobRuntimeComponents {
                inspector: Arc::new(StaticInspector) as Arc<RuntimeInspector>,
                command_runner: command_runner as Arc<RuntimeCommandRunner>,
                capacity_probe: Arc::new(StaticCapacityProbe {
                    available_bytes: 1024 * 1024 * 1024,
                }) as Arc<RuntimeCapacityProbe>,
                replacement_committer: Arc::new(SystemReplacementCommitter)
                    as Arc<RuntimeReplacementCommitter>,
                verification_executor: Arc::new(PassingVerificationExecutor)
                    as Arc<RuntimeVerificationExecutor>,
                audio_analyzer: Arc::new(StaticAudioAnalyzer {
                    measurement: Ok(AudioMeasurement {
                        integrated_lufs: -16.0,
                        loudness_range_lu: 8.0,
                        true_peak_dbfs: Some(-1.5),
                    }),
                }) as Arc<RuntimeAudioAnalyzer>,
                events,
                telemetry,
                tick_interval: Duration::from_mins(1),
                workspace_policy: WorkspacePolicy {
                    max_bytes: 1024 * 1024 * 1024,
                    reserve_bytes: 1024,
                },
                workspace_root,
            },
        )
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
            ("utility", "ffprobe", true),
            ("utility", "ffplay", true),
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
        let Some(fixture) = setup_runtime(true, true, RuntimeJobTarget::SourceGraph).await? else {
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
        let rendered = fixture.telemetry.render()?;
        assert!(rendered_has_metric_labels(
            &rendered,
            "media_job_outcomes_total",
            &[("outcome", "completed"), ("dry_run", "true")]
        ));
        assert!(rendered_has_metric_labels(
            &rendered,
            "media_job_phases_total",
            &[("phase", "inspect_plan"), ("status", "completed")]
        ));
        assert!(rendered_has_metric_labels(
            &rendered,
            "media_job_operations_total",
            &[("operation", "no_op"), ("outcome", "planned")]
        ));
        assert!(rendered_has_metric_labels(
            &rendered,
            "media_job_verification_checks_total",
            &[("check", "dry_run_preflight"), ("status", "passed")]
        ));
        assert!(rendered.contains("media_workspace_cleanup_total"));
        Ok(())
    }

    fn rendered_has_metric_labels(
        rendered: &str,
        metric_name: &str,
        labels: &[(&str, &str)],
    ) -> bool {
        rendered.lines().any(|line| {
            line.starts_with(metric_name)
                && labels
                    .iter()
                    .all(|(name, value)| line.contains(&format!("{name}=\"{value}\"")))
        })
    }

    async fn wait_for_flag(flag: &AtomicBool, label: &'static str) -> anyhow::Result<()> {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !flag.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("timed out waiting for {label}"))?;
        Ok(())
    }

    async fn assert_runtime_cancelled(
        store: &MediaStore,
        job_id: Uuid,
        source_path: &Path,
        workspace_output: &Path,
    ) -> anyhow::Result<()> {
        let job = store
            .get_job(job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("cancelled media job missing"))?;
        assert_eq!(job.status_text, "cancelled");
        assert_eq!(job.last_error, None);
        assert_eq!(fs::read(source_path)?, b"source");
        assert!(!workspace_output.exists());
        assert!(
            store
                .list_job_verification_checks(job_id)
                .await?
                .iter()
                .any(|check| {
                    check.check_kind == "cancellation"
                        && check.check_status == "skipped"
                        && check.actual_value.as_deref() == Some("operator_cancelled")
                })
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_executes_non_dry_run_with_injected_runner() -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(SuccessfulTranscodeInspector) as Arc<RuntimeInspector>;

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
            assert!(
                first_command
                    .windows(2)
                    .any(|pair| pair == ["-profile:0", "main10"])
            );
            assert!(
                first_command
                    .windows(2)
                    .any(|pair| pair == ["-level:0", "5.1"])
            );
            assert!(
                first_command
                    .windows(2)
                    .any(|pair| pair == ["-b:0", "8000000"])
            );
            assert!(
                first_command
                    .windows(2)
                    .any(|pair| pair == ["-color_primaries:0", "bt2020"])
            );
            assert!(
                first_command
                    .windows(2)
                    .any(|pair| pair == ["-color_trc:0", "smpte2084"])
            );
            assert!(
                first_command
                    .windows(2)
                    .any(|pair| pair == ["-colorspace:0", "bt2020nc"])
            );
            commands.len()
        };
        assert_eq!(command_count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_rejects_candidate_video_constraint_mismatch() -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector =
            Arc::new(VideoConstraintMismatchInspector) as Arc<RuntimeInspector>;
        let source_path = PathBuf::from(
            fixture
                .store
                .get_job(fixture.job_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("media job missing"))?
                .source_path,
        );

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("media_job_output_video_constraints_mismatch")
        );
        assert_eq!(fs::read(source_path)?, b"source");
        let checks = fixture
            .store
            .list_job_verification_checks(fixture.job_id)
            .await?;
        assert!(checks.iter().any(|check| {
            check.check_kind == "candidate_graph" && check.check_status == "passed"
        }));
        assert!(checks.iter().any(|check| {
            check.check_kind == "candidate_video_constraints"
                && check.check_status == "failed"
                && check.expected_value.as_deref() == Some("stream:0:color_transfer=smpte2084")
                && check.actual_value.as_deref() == Some("bt709")
        }));
        assert!(!checks.iter().any(|check| {
            check.check_kind == "output_replacement" && check.check_status == "passed"
        }));
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_rejects_candidate_hdr10_missing_side_data() -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(HdrSideDataMismatchInspector) as Arc<RuntimeInspector>;
        let source_path = PathBuf::from(
            fixture
                .store
                .get_job(fixture.job_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("media job missing"))?
                .source_path,
        );

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("media_job_output_video_constraints_mismatch")
        );
        assert_eq!(fs::read(source_path)?, b"source");
        let checks = fixture
            .store
            .list_job_verification_checks(fixture.job_id)
            .await?;
        assert!(checks.iter().any(|check| {
            check.check_kind == "candidate_graph" && check.check_status == "passed"
        }));
        assert!(checks.iter().any(|check| {
            check.check_kind == "candidate_video_constraints"
                && check.check_status == "failed"
                && check.expected_value.as_deref() == Some("stream:0:hdr_format=hdr10")
                && check.actual_value.as_deref() == Some("unverified")
        }));
        assert!(!checks.iter().any(|check| {
            check.check_kind == "output_replacement" && check.check_status == "passed"
        }));
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_rejects_candidate_audio_constraint_mismatch() -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::HevcAudio).await?
        else {
            return Ok(());
        };
        fixture.runtime.inspector =
            Arc::new(AudioConstraintMismatchInspector) as Arc<RuntimeInspector>;
        let source_path = PathBuf::from(
            fixture
                .store
                .get_job(fixture.job_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("media job missing"))?
                .source_path,
        );

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("media_job_output_audio_constraints_mismatch")
        );
        assert_eq!(fs::read(source_path)?, b"source");
        let checks = fixture
            .store
            .list_job_verification_checks(fixture.job_id)
            .await?;
        assert!(checks.iter().any(|check| {
            check.check_kind == "candidate_graph" && check.check_status == "passed"
        }));
        assert!(checks.iter().any(|check| {
            check.check_kind == "candidate_audio_constraints"
                && check.check_status == "failed"
                && check.expected_value.as_deref() == Some("stream:1:sample_rate_hz=48000")
                && check.actual_value.as_deref() == Some("44100")
        }));
        assert!(!checks.iter().any(|check| {
            check.check_kind == "output_replacement" && check.check_status == "passed"
        }));
        let first_command = {
            let commands = fixture
                .command_runner
                .commands
                .lock()
                .map_err(|error| anyhow::anyhow!("command lock poisoned: {error}"))?;
            let first_command = commands
                .first()
                .ok_or_else(|| anyhow::anyhow!("command missing"))?
                .clone();
            drop(commands);
            first_command
        };
        let has_bitrate_arg = first_command
            .windows(2)
            .any(|pair| pair == ["-b:a:0", "160000"]);
        let has_sample_rate_arg = first_command
            .windows(2)
            .any(|pair| pair == ["-ar:0", "48000"]);
        assert!(has_bitrate_arg);
        assert!(has_sample_rate_arg);
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_rejects_candidate_audio_loudness_mismatch() -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::HevcAudio).await?
        else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(AudioPolicyInspector) as Arc<RuntimeInspector>;
        fixture.runtime.audio_analyzer = Arc::new(StaticAudioAnalyzer {
            measurement: Ok(AudioMeasurement {
                integrated_lufs: -20.0,
                loudness_range_lu: 14.0,
                true_peak_dbfs: Some(-2.0),
            }),
        }) as Arc<RuntimeAudioAnalyzer>;
        let source_path = PathBuf::from(
            fixture
                .store
                .get_job(fixture.job_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("media job missing"))?
                .source_path,
        );

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("media_job_output_audio_constraints_mismatch")
        );
        assert_eq!(fs::read(source_path)?, b"source");
        let checks = fixture
            .store
            .list_job_verification_checks(fixture.job_id)
            .await?;
        let check_summary = checks
            .iter()
            .map(|check| {
                format!(
                    "{}:{}:{:?}:{:?}",
                    check.check_kind,
                    check.check_status,
                    check.expected_value.as_deref(),
                    check.actual_value.as_deref()
                )
            })
            .collect::<Vec<_>>()
            .join("|");
        assert!(
            checks.iter().any(|check| {
                let expected = check.expected_value.as_deref().map_or("", |value| value);
                check.check_kind == "candidate_audio_constraints"
                    && check.check_status == "failed"
                    && (expected == "stream:1:integrated_lufs=-17.0..=-15.0"
                        || expected == "stream:1:loudness_range_lu=<=12.0"
                        || expected == "stream:1:true_peak_dbfs=<=-1.0")
            }),
            "{check_summary}"
        );
        assert!(!checks.iter().any(|check| {
            check.check_kind == "output_replacement" && check.check_status == "passed"
        }));
        Ok(())
    }

    #[test]
    fn ebur128_summary_parser_reads_loudness_range_and_peak() -> anyhow::Result<()> {
        let output = "
Integrated loudness:
    I:         -16.2 LUFS
Loudness range:
    LRA:         8.5 LU
True peak:
    Peak:       -1.4 dBFS
";
        let measurement = parse_ebur128_summary(output).map_err(anyhow::Error::msg)?;
        assert!((measurement.integrated_lufs - -16.2).abs() < 0.01);
        assert!((measurement.loudness_range_lu - 8.5).abs() < 0.01);
        assert_eq!(measurement.true_peak_dbfs, Some(-1.4));
        Ok(())
    }

    #[test]
    fn ebur128_summary_parser_requires_integrated_loudness_and_range() {
        let missing_lufs = parse_ebur128_summary(
            "
Loudness range:
    LRA:         8.5 LU
",
        );
        assert_eq!(
            missing_lufs.err().as_deref(),
            Some("audio analyzer output missing integrated LUFS")
        );

        let missing_range = parse_ebur128_summary(
            "
Integrated loudness:
    I:         -16.2 LUFS
",
        );
        assert_eq!(
            missing_range.err().as_deref(),
            Some("audio analyzer output missing loudness range")
        );
    }

    #[test]
    fn audio_measurement_policy_checks_fail_missing_peak_and_excess_lra() {
        let constraint = AudioStreamConstraints {
            stream_id: 1,
            bitrate_bps: None,
            sample_rate_hz: None,
            loudness_profile: Some("dialog-normalized".to_string()),
            dynamic_range: Some("speech".to_string()),
        };

        let missing_peak = audio_measurement_mismatch(
            &constraint,
            AudioMeasurement {
                integrated_lufs: -16.0,
                loudness_range_lu: 8.0,
                true_peak_dbfs: None,
            },
        )
        .expect("missing true peak should fail dialog-normalized verification");
        assert_eq!(
            missing_peak.expected,
            "stream:1:true_peak_dbfs=<=-1.0".to_string()
        );
        assert_eq!(missing_peak.actual, "missing".to_string());

        let high_lra = audio_measurement_mismatch(
            &AudioStreamConstraints {
                loudness_profile: None,
                dynamic_range: Some(" speech ".to_string()),
                ..constraint
            },
            AudioMeasurement {
                integrated_lufs: -16.0,
                loudness_range_lu: 12.5,
                true_peak_dbfs: Some(-2.0),
            },
        )
        .expect("speech dynamic range above policy maximum should fail");
        assert_eq!(
            high_lra.expected,
            "stream:1:loudness_range_lu=<=12.0".to_string()
        );
        assert_eq!(high_lra.actual, "12.5".to_string());
    }

    #[test]
    fn system_audio_analyzer_reports_spawn_failure() {
        let analyzer = SystemFfmpegAudioAnalysisAdapter {
            ffmpeg_bin: format!("missing-ffmpeg-{}", Uuid::new_v4()),
        };

        let error = analyzer
            .measure("/tmp/nonexistent-media-input.mkv", 1)
            .expect_err("missing analyzer binary should fail closed");

        assert!(error.starts_with("audio analyzer command spawn failed:"));
    }

    #[tokio::test]
    async fn media_job_runtime_cancels_active_transcode_and_removes_candidate() -> anyhow::Result<()>
    {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(SuccessfulTranscodeInspector) as Arc<RuntimeInspector>;
        let runner = Arc::new(CancellationAwareCommandRunner::default());
        fixture.runtime.command_runner = Arc::clone(&runner) as Arc<RuntimeCommandRunner>;
        let source_path = PathBuf::from(
            fixture
                .store
                .get_job(fixture.job_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("media job missing"))?
                .source_path,
        );
        let workspace_output = fixture
            .runtime
            .workspace_root
            .join(fixture.job_id.to_string())
            .join("output");
        let store = fixture.store.clone();
        let job_id = fixture.job_id;
        let runtime = fixture.runtime;
        let tick = tokio::spawn(async move { runtime.run_tick().await });

        wait_for_flag(&runner.started, "transcode start").await?;
        store.cancel_job(job_id).await?;
        let tick_result = tokio::time::timeout(Duration::from_secs(5), tick).await??;
        tick_result?;

        assert!(runner.cancellation_observed.load(Ordering::Acquire));
        assert_runtime_cancelled(&store, job_id, &source_path, &workspace_output).await?;
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_cancels_active_verification_before_replacement() -> anyhow::Result<()>
    {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(SuccessfulTranscodeInspector) as Arc<RuntimeInspector>;
        let verifier = Arc::new(CancellationAwareVerificationExecutor::default());
        fixture.runtime.verification_executor =
            Arc::clone(&verifier) as Arc<RuntimeVerificationExecutor>;
        let source_path = PathBuf::from(
            fixture
                .store
                .get_job(fixture.job_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("media job missing"))?
                .source_path,
        );
        let workspace_output = fixture
            .runtime
            .workspace_root
            .join(fixture.job_id.to_string())
            .join("output");
        let store = fixture.store.clone();
        let job_id = fixture.job_id;
        let runtime = fixture.runtime;
        let tick = tokio::spawn(async move { runtime.run_tick().await });

        wait_for_flag(&verifier.started, "verification start").await?;
        store.cancel_job(job_id).await?;
        let tick_result = tokio::time::timeout(Duration::from_secs(5), tick).await??;
        tick_result?;

        assert!(verifier.cancellation_observed.load(Ordering::Acquire));
        assert_runtime_cancelled(&store, job_id, &source_path, &workspace_output).await?;
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_discards_prepared_replacement_when_cancelled_before_commit()
    -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(SuccessfulTranscodeInspector) as Arc<RuntimeInspector>;
        let committer = Arc::new(PreCommitCancellationCommitter::default());
        fixture.runtime.replacement_committer =
            Arc::clone(&committer) as Arc<RuntimeReplacementCommitter>;
        let source_path = PathBuf::from(
            fixture
                .store
                .get_job(fixture.job_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("media job missing"))?
                .source_path,
        );
        let workspace_output = fixture
            .runtime
            .workspace_root
            .join(fixture.job_id.to_string())
            .join("output");
        let store = fixture.store.clone();
        let job_id = fixture.job_id;
        let runtime = fixture.runtime;
        let tick = tokio::spawn(async move { runtime.run_tick().await });

        wait_for_flag(&committer.prepared, "replacement preparation").await?;
        store.cancel_job(job_id).await?;
        committer.release_prepare.store(true, Ordering::Release);
        let tick_result = tokio::time::timeout(Duration::from_secs(5), tick).await??;
        tick_result?;

        assert!(committer.discarded.load(Ordering::Acquire));
        assert!(!committer.committed.load(Ordering::Acquire));
        assert_runtime_cancelled(&store, job_id, &source_path, &workspace_output).await?;
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_completes_non_dry_run_noop_without_command_execution()
    -> anyhow::Result<()> {
        let Some(fixture) = setup_runtime(false, true, RuntimeJobTarget::SourceGraph).await? else {
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
                .any(|check| check.check_kind == "source_graph" && check.check_status == "passed")
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
    async fn media_job_runtime_publishes_non_dry_run_lifecycle_events() -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(SuccessfulTranscodeInspector) as Arc<RuntimeInspector>;
        let mut stream = fixture.events.subscribe(None);

        fixture.runtime.run_tick().await?;

        let mut event_kinds = Vec::new();
        while event_kinds.len() < 4 {
            let envelope = tokio::time::timeout(Duration::from_secs(1), stream.next())
                .await?
                .ok_or_else(|| anyhow::anyhow!("event stream closed"))??;
            match envelope.event {
                CoreEvent::MediaJobInspected {
                    media_job_public_id,
                }
                | CoreEvent::MediaJobExecutionStarted {
                    media_job_public_id,
                }
                | CoreEvent::MediaJobCompleted {
                    media_job_public_id,
                } if media_job_public_id == fixture.job_id => {
                    event_kinds.push(envelope.event.kind());
                }
                CoreEvent::MediaJobPlanned {
                    media_job_public_id,
                    ..
                } if media_job_public_id == fixture.job_id => {
                    event_kinds.push(envelope.event.kind());
                }
                _ => {}
            }
        }

        assert_eq!(
            event_kinds,
            vec![
                "media_job_inspected",
                "media_job_planned",
                "media_job_execution_started",
                "media_job_completed",
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_rejects_low_workspace_capacity_before_execution()
    -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
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
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
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
    async fn media_job_runtime_rejects_candidate_that_drops_source_chapters() -> anyhow::Result<()>
    {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector =
            Arc::new(CandidateDropsChaptersInspector) as Arc<RuntimeInspector>;

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("media_job_output_chapter_timeline_mismatch")
        );
        assert_eq!(fs::read(&job.source_path)?, b"source");
        let checks = fixture
            .store
            .list_job_verification_checks(fixture.job_id)
            .await?;
        assert!(checks.iter().any(|check| {
            check.check_kind == "candidate_chapters" && check.check_status == "failed"
        }));
        assert!(checks.iter().all(|check| check.check_kind != "final_graph"));
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_rejects_probeable_truncated_candidate_before_replacement()
    -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(SuccessfulTranscodeInspector) as Arc<RuntimeInspector>;
        fixture.runtime.verification_executor =
            Arc::new(FailingDecodeVerificationExecutor) as Arc<RuntimeVerificationExecutor>;

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("media_job_candidate_safety_verification_failed")
        );
        assert_eq!(fs::read(&job.source_path)?, b"source");
        let checks = fixture
            .store
            .list_job_verification_checks(fixture.job_id)
            .await?;
        assert!(checks.iter().any(|check| {
            check.check_kind == "decode_corruption" && check.check_status == "failed"
        }));
        assert!(checks.iter().all(|check| check.check_kind != "final_graph"));
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_rolls_back_mismatched_committed_replacement() -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector = Arc::new(PostCommitMismatchInspector) as Arc<RuntimeInspector>;

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
                |check| check.check_kind == "candidate_graph" && check.check_status == "passed"
            )
        );
        assert!(
            checks
                .iter()
                .any(|check| check.check_kind == "final_graph" && check.check_status == "failed")
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_rolls_back_committed_replacement_that_drops_chapters()
    -> anyhow::Result<()> {
        let Some(mut fixture) = setup_runtime(false, true, RuntimeJobTarget::Hevc).await? else {
            return Ok(());
        };
        fixture.runtime.inspector =
            Arc::new(PostCommitDropsChaptersInspector) as Arc<RuntimeInspector>;

        fixture.runtime.run_tick().await?;

        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("media job missing"))?;
        assert_eq!(job.status_text, "failed");
        assert_eq!(
            job.last_error.as_deref(),
            Some("media_job_output_chapter_timeline_mismatch")
        );
        assert_eq!(fs::read(&job.source_path)?, b"source");
        let checks = fixture
            .store
            .list_job_verification_checks(fixture.job_id)
            .await?;
        assert!(checks.iter().any(|check| {
            check.check_kind == "candidate_chapters" && check.check_status == "passed"
        }));
        assert!(checks.iter().any(
            |check| check.check_kind == "final_chapters" && check.check_status == "failed"
        ));
        Ok(())
    }

    #[tokio::test]
    async fn media_job_runtime_marks_missing_capability_failed() -> anyhow::Result<()> {
        let Some(fixture) = setup_runtime(true, false, RuntimeJobTarget::SourceGraph).await? else {
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
                    codec: "dts".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 2,
                    kind: StreamKind::Subtitle,
                    codec: "subrip".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 3,
                    kind: StreamKind::Subtitle,
                    codec: "ass".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("jpn".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };

        let target = super::DesiredGraphTarget {
            video_codec: "hevc".to_string(),
            audio_codec: "aac".to_string(),
            audio_channels: None,
            audio_channel_layout: None,
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
    fn selected_subtitle_policy_prefers_full_subtitles_over_commentary_default() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
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
                    channels: Some(2),
                    channel_layout: Some("stereo".to_string()),
                    language: Some("eng".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 2,
                    kind: StreamKind::Subtitle,
                    codec: "subrip".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("Director Commentary".to_string()),
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 3,
                    kind: StreamKind::Subtitle,
                    codec: "subrip".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("English Full".to_string()),
                    dispositions: Vec::new(),
                },
            ],
        };
        let target = super::DesiredGraphTarget {
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
            audio_channels: None,
            audio_channel_layout: None,
            subtitle_policy: super::SubtitlePolicy::Selected,
        };

        let desired = super::compile_desired_graph(&source, "/workspace/movie.mkv", Some(&target));
        let subtitle_ids = desired
            .streams
            .iter()
            .filter(|stream| stream.kind == StreamKind::Subtitle)
            .map(|stream| stream.stream_id)
            .collect::<Vec<_>>();

        assert_eq!(subtitle_ids, vec![3]);
    }

    #[test]
    fn desired_graph_clears_commentary_subtitle_default_when_retaining_all_subtitles() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
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
                    channels: Some(2),
                    channel_layout: Some("stereo".to_string()),
                    language: Some("eng".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 2,
                    kind: StreamKind::Subtitle,
                    codec: "subrip".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("English Full".to_string()),
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 3,
                    kind: StreamKind::Subtitle,
                    codec: "subrip".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("Director Commentary".to_string()),
                    dispositions: vec!["default".to_string()],
                },
            ],
        };
        let target = super::DesiredGraphTarget {
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
            audio_channels: None,
            audio_channel_layout: None,
            subtitle_policy: super::SubtitlePolicy::All,
        };

        let desired = super::compile_desired_graph(&source, "/workspace/movie.mkv", Some(&target));
        let subtitle_dispositions = desired
            .streams
            .iter()
            .filter(|stream| stream.kind == StreamKind::Subtitle)
            .map(|stream| (stream.stream_id, stream.dispositions.clone()))
            .collect::<Vec<_>>();

        assert_eq!(
            subtitle_dispositions,
            vec![(2, vec!["default".to_string()]), (3, Vec::new())]
        );
    }

    #[test]
    fn desired_graph_keeps_only_first_subtitle_default_when_retaining_all_subtitles() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
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
                    channels: Some(2),
                    channel_layout: Some("stereo".to_string()),
                    language: Some("eng".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 2,
                    kind: StreamKind::Subtitle,
                    codec: "subrip".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("English Full".to_string()),
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 3,
                    kind: StreamKind::Subtitle,
                    codec: "subrip".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("English SDH".to_string()),
                    dispositions: vec!["default".to_string()],
                },
            ],
        };
        let target = super::DesiredGraphTarget {
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
            audio_channels: None,
            audio_channel_layout: None,
            subtitle_policy: super::SubtitlePolicy::All,
        };

        let desired = super::compile_desired_graph(&source, "/workspace/movie.mkv", Some(&target));
        let subtitle_dispositions = desired
            .streams
            .iter()
            .filter(|stream| stream.kind == StreamKind::Subtitle)
            .map(|stream| (stream.stream_id, stream.dispositions.clone()))
            .collect::<Vec<_>>();

        assert_eq!(
            subtitle_dispositions,
            vec![(2, vec!["default".to_string()]), (3, Vec::new())]
        );
    }

    #[test]
    fn desired_graph_applies_snapshotted_audio_channel_policy() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
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
                    channels: Some(6),
                    channel_layout: Some("5.1(side)".to_string()),
                    language: Some("eng".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: vec!["default".to_string()],
                },
            ],
        };
        let target = super::DesiredGraphTarget {
            video_codec: "hevc".to_string(),
            audio_codec: "aac".to_string(),
            audio_channels: Some(2),
            audio_channel_layout: Some("stereo".to_string()),
            subtitle_policy: super::SubtitlePolicy::Selected,
        };

        let desired = super::compile_desired_graph(&source, "/workspace/movie.mkv", Some(&target));
        let Some(audio) = desired
            .streams
            .iter()
            .find(|stream| stream.kind == StreamKind::Audio)
        else {
            panic!("expected desired audio stream");
        };

        assert_eq!(audio.codec, "aac");
        assert_eq!(audio.channels, Some(2));
        assert_eq!(audio.channel_layout.as_deref(), Some("stereo"));

        let diff = revaer_media_core::diff::diff_graphs(&source, &desired);
        assert_eq!(diff.audio_channel_mismatched_streams, vec![1]);
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
            ]
        );
    }

    #[test]
    fn final_graph_verification_matches_ordered_stream_content_after_index_compaction() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: Some("mkv".to_string()),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "hevc".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 2,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: vec!["default".to_string(), "forced".to_string()],
                },
            ],
        };
        let matching = MediaGraph {
            source_path: "/output/movie.mkv".to_string(),
            container_formats: vec!["matroska".to_string(), "webm".to_string()],
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "HEVC".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                    codec: "AAC".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("ENG".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: vec!["forced".to_string(), "default".to_string()],
                },
            ],
        };
        assert!(super::media_graph_matches_desired(&matching, &desired));

        let mismatched = MediaGraph {
            source_path: "/output/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "hevc".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: vec!["default".to_string()],
                },
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("jpn".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: vec!["default".to_string(), "forced".to_string()],
                },
            ],
        };
        assert!(!super::media_graph_matches_desired(&mismatched, &desired));

        let mut channel_desired = desired;
        channel_desired.streams[1].channels = Some(2);
        channel_desired.streams[1].channel_layout = Some("stereo".to_string());

        let mut channel_matching = matching.clone();
        channel_matching.streams[1].channels = Some(2);
        channel_matching.streams[1].channel_layout = Some("STEREO".to_string());
        assert!(super::media_graph_matches_desired(
            &channel_matching,
            &channel_desired
        ));

        let mut channel_mismatched = matching;
        channel_mismatched.streams[1].channels = Some(6);
        channel_mismatched.streams[1].channel_layout = Some("5.1(side)".to_string());
        assert!(!super::media_graph_matches_desired(
            &channel_mismatched,
            &channel_desired
        ));
    }

    #[test]
    fn final_graph_verification_rejects_wrong_container() {
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: Some("mkv".to_string()),
            streams: Vec::new(),
        };
        let inspected = MediaGraph {
            source_path: "/output/movie.mkv".to_string(),
            container_formats: vec!["mp4".to_string()],
            streams: Vec::new(),
        };

        assert!(!super::media_graph_matches_desired(&inspected, &desired));
    }

    #[test]
    fn sidecar_helpers_preserve_semantics_and_validate_published_artifacts() -> anyhow::Result<()> {
        let sidecars = vec![
            SidecarSubtitle {
                path: PathBuf::from("/media/movie.eng.forced.srt"),
                companion_path: None,
                language: Some("eng".to_string()),
                role: Some(SidecarRole::Forced),
                format: SidecarFormat::Srt,
            },
            SidecarSubtitle {
                path: PathBuf::from("/media/movie.jpn.idx"),
                companion_path: Some(PathBuf::from("/media/movie.jpn.sub")),
                language: Some("jpn".to_string()),
                role: None,
                format: SidecarFormat::VobSub,
            },
        ];
        let inputs = super::sidecar_inputs(&sidecars)?;
        assert_eq!(inputs[0].codec, "subrip");
        assert_eq!(inputs[0].role, Some(SemanticRole::Forced));
        assert!(!inputs[0].image_based);
        assert_eq!(inputs[1].codec, "dvd_subtitle");
        assert!(inputs[1].image_based);
        assert_eq!(
            inputs[1].companion_path.as_deref(),
            Some("/media/movie.jpn.sub")
        );

        let outputs = vec![DesiredSidecarOutput {
            path: "/workspace/movie.jpn.idx".to_string(),
            companion_path: Some("/workspace/movie.jpn.sub".to_string()),
            destination_path: "/media/movie.jpn.idx".to_string(),
            destination_companion_path: Some("/media/movie.jpn.sub".to_string()),
            source: SidecarOutputSource::ExistingSidecar {
                path: "/input/movie.jpn.idx".to_string(),
                companion_path: Some("/input/movie.jpn.sub".to_string()),
                codec: "dvd_subtitle".to_string(),
            },
            codec: "dvd_subtitle".to_string(),
        }];
        let removals = vec!["/media/movie.eng.forced.srt".to_string()];
        assert_eq!(
            super::replacement_artifact_paths(&outputs, &removals),
            vec![
                (
                    PathBuf::from("/media/movie.jpn.idx"),
                    Some(PathBuf::from("/workspace/movie.jpn.idx")),
                ),
                (
                    PathBuf::from("/media/movie.jpn.sub"),
                    Some(PathBuf::from("/workspace/movie.jpn.sub")),
                ),
                (PathBuf::from("/media/movie.eng.forced.srt"), None),
            ]
        );

        let mut inspection = complete_test_inspection(video_graph("/media/movie.mkv", "h264"));
        inspection.sidecars = vec![sidecars[1].clone()];
        assert!(super::sidecar_state_matches(
            &inspection,
            &outputs,
            &removals
        ));
        inspection.sidecars.push(sidecars[0].clone());
        assert!(!super::sidecar_state_matches(
            &inspection,
            &outputs,
            &removals
        ));
        inspection.sidecars.clear();
        assert!(!super::sidecar_state_matches(
            &inspection,
            &outputs,
            &removals
        ));
        Ok(())
    }

    #[test]
    fn source_artifact_bytes_counts_container_and_unique_sidecar_files() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("movie.mkv");
        let index = temp.path().join("movie.eng.idx");
        let companion = temp.path().join("movie.eng.sub");
        fs::write(&source, b"container")?;
        fs::write(&index, b"index")?;
        fs::write(&companion, b"subtitle")?;
        let sidecars = [SidecarSubtitle {
            path: index,
            companion_path: Some(companion),
            language: Some("eng".to_string()),
            role: None,
            format: SidecarFormat::VobSub,
        }];

        assert_eq!(
            super::source_artifact_bytes(source.to_string_lossy().as_ref(), &sidecars)?,
            22
        );
        Ok(())
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
            super::source_artifact_bytes("/definitely/missing/revaer/media.mkv", &[])
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
        let Some(fixture) = setup_runtime(true, true, RuntimeJobTarget::SourceGraph).await? else {
            return Ok(());
        };
        let source = MediaGraph {
            source_path: "/media/in.mkv".to_string(),
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
            output_path: "/media/out.mkv".to_string(),
            container_format: None,
            streams: source.streams.clone(),
        };
        let report = JobPreflightReport {
            planned: PlannedJob {
                source: Box::new(source),
                desired: Box::new(desired),
                sidecar_embeddings: Vec::new(),
                sidecar_outputs: Vec::new(),
                sidecar_removals: Vec::new(),
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
                embed_subtitle_operations: 0,
                extract_subtitle_operations: 0,
                copy_sidecar_subtitle_operations: 0,
                remove_sidecar_subtitle_operations: 0,
                subtitle_transcode_operations: 0,
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
            target_audio_channels: None,
            target_audio_channel_layout: None,
            target_subtitle_policy: None,
            policy_video_intent: Some("general".to_string()),
            desired_target_key: None,
            desired_target_version: None,
            desired_container_format: None,
            unmatched_stream_policy: Some("remove".to_string()),
            verification_strictness: "strict".to_string(),
            verification_duration_tolerance_millis: 100,
            verification_mux_validation: true.into(),
            verification_decode_all_streams: true.into(),
            verification_keyframe_seek: true.into(),
            verification_playback_probe: true.into(),
            cancel_generation: 0,
        }
    }

    #[test]
    fn verification_policy_rejects_relaxed_strict_snapshot() {
        let mut job = claimed_job_with_paths(
            "/input/movie.mkv",
            Some("/output/movie.mkv".to_string()),
            false,
            "/input",
            "/output",
        );
        job.verification_playback_probe = false.into();
        let result = verification_policy_from_job(&job);
        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.code(), "media_job_verification_checks_invalid");
        }
    }
}
