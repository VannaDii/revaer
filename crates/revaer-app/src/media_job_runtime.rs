//! In-process media job runtime.
//!
//! # Design
//! - Claims queued media jobs through stored procedures.
//! - Persists phase, operation, verification, and compact-audit rows before terminal status.
//! - Keeps runtime adapters injected so tests avoid real `ffmpeg` execution.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read};
use std::num::TryFromIntError;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use revaer_data::DataError;
use revaer_data::media::jobs::{
    AppendMediaJobCompactAuditInput, AppendMediaJobOperationInput, AppendMediaJobPlanReasonInput,
    AppendMediaJobVerificationCheckInput, ClaimedMediaJobRow, MediaJobDesiredTargetStreamRow,
};
use revaer_events::{Event, EventBus};
use revaer_media_core::classify::SemanticRole;
use revaer_media_core::model::{
    DesiredGraph, DesiredStreamBinding, MediaGraph, MediaStream, StreamKind,
};
use revaer_media_core::normalize::{normalize_audio_channel_layout, normalize_container_format};
use revaer_media_core::pipeline::{
    PlanningConstraints, PlanningOutcome, compile_and_plan_with_policy,
};
use revaer_media_core::plan::{CandidateRejectionReason, OperationKind, PlannedOperation};
use revaer_media_core::target::{
    CompiledDesiredTarget, DesiredSidecarOutput, DesiredTarget, ImageSubtitleAction, LanguageToken,
    SidecarSubtitleInput, SubtitlePlacement, TargetStream, UnmatchedStreamPolicy,
    compile_desired_target_with_sidecars_at,
};
use revaer_media_runtime::capabilities::{CapabilitySnapshot, CodecCapability};
use revaer_media_runtime::execute::{
    AudioStreamConstraints, CommandRunner, ExecuteSequenceError, ExecuteStepError,
    ExecutionControl, ExecutionStep, MaxBitrateBps, ProcessCommandRunner, VideoStreamConstraints,
    VideoTranscodeIntent, VideoTranscodePolicy, execute_filesystem_step,
};
use revaer_media_runtime::inspect::{
    ChapterInspection, FfprobeInspectAdapter, InspectAdapter, InspectCancellation, InspectError,
    MediaInspection, MetadataEntry, StreamInspection, SupervisedInspectProbeExecutor,
};
use revaer_media_runtime::jobs::{
    JobPreflightEvaluation, JobPreflightReport, PreflightBuildTemplate, PreflightPolicyInput,
    build_preflight_input, evaluate_preflight_from_compiled_target,
    evaluate_preflight_from_planning_outcome, preflight_compact_audit_facts,
};
use revaer_media_runtime::process::NativeProcessSupervisor;
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
    ManagedWorkspace, ManagedWorkspaceError, TerminalWorkspaceCleanupPolicy,
    TerminalWorkspaceState, WorkspacePaths, WorkspacePolicy, cleanup_terminal_workspace,
    create_or_resume_managed_workspace, project_managed_workspace,
};
use revaer_runtime::media::MediaStore;
use revaer_telemetry::Metrics;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};
use uuid::Uuid;

use crate::media_discovery_fingerprint::{
    FingerprintError, MediaAggregateFingerprint, fingerprint_media_aggregate_cancellable,
};
use crate::runtime_shutdown::{self, RuntimeShutdownReceiver};

const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(1);
const DEFAULT_WORKSPACE_MAX_BYTES: u64 = 20 * 1024 * 1024 * 1024;
const DEFAULT_WORKSPACE_RESERVE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_IN_PROCESS_MEDIA_JOBS: usize = 256;
const FAILURE_PHASE_INDEX: i32 = 99;
const FAILURE_CHECK_INDEX: i32 = 99;
const CANCELLATION_PHASE_INDEX: i32 = 98;
const CANCELLATION_CHECK_INDEX: i32 = 98;
const CONTROL_POLL_INTERVAL: Duration = Duration::from_millis(250);
const DIALOG_NORMALIZED_TARGET_LUFS: f64 = -16.0;
const DIALOG_NORMALIZED_LUFS_TOLERANCE: f64 = 1.0;
const DIALOG_NORMALIZED_TRUE_PEAK_MAX_DBFS: f64 = -1.0;
const SPEECH_DYNAMIC_RANGE_MAX_LU: f64 = 12.0;
const MAX_AUDIO_ANALYSIS_STDERR_BYTES: usize = 64 * 1024;
const AUDIO_ANALYSIS_TRUNCATION_MARKER: &str = "...[truncated]\n";
const AUDIO_ANALYSIS_TIMEOUT: Duration = Duration::from_mins(30);

type RuntimeInspector = dyn InspectAdapter + Send + Sync;
type RuntimeCommandRunner = dyn CommandRunner + Send + Sync;
type RuntimeCapacityProbe = dyn FilesystemCapacityProbe + Send + Sync;
type RuntimeReplacementCommitter = dyn ReplacementCommitter + Send + Sync;
type RuntimeVerificationExecutor = dyn VerificationExecutor + Send + Sync;
type RuntimeAudioAnalyzer = dyn AudioAnalysisAdapter + Send + Sync;
type RuntimeSourceFingerprintProbe = dyn SourceFingerprintProbe + Send + Sync;

struct MediaJobRuntimeComponents {
    inspector: Arc<RuntimeInspector>,
    command_runner: Arc<RuntimeCommandRunner>,
    capacity_probe: Arc<RuntimeCapacityProbe>,
    replacement_committer: Arc<RuntimeReplacementCommitter>,
    verification_executor: Arc<RuntimeVerificationExecutor>,
    audio_analyzer: Arc<RuntimeAudioAnalyzer>,
    source_fingerprint_probe: Arc<RuntimeSourceFingerprintProbe>,
    events: EventBus,
    telemetry: Metrics,
    tick_interval: Duration,
    workspace_policy: WorkspacePolicy,
    workspace_root: PathBuf,
    scratch_reservations: Arc<Mutex<BTreeMap<PathBuf, u64>>>,
}

struct RuntimePreflightEvaluation {
    evaluation: JobPreflightEvaluation,
    desired: DesiredGraph,
    desired_target: Option<DesiredTargetSnapshot>,
    expected_container_metadata: Vec<MetadataEntry>,
    expected_chapters: Vec<ChapterInspection>,
    planning_outcome: Option<PlanningOutcome>,
}

struct PreflightReadyContext<'a> {
    desired: &'a DesiredGraph,
    desired_target: Option<&'a DesiredTargetSnapshot>,
    expected_container_metadata: &'a [MetadataEntry],
    expected_chapters: &'a [ChapterInspection],
    planning_outcome: Option<&'a PlanningOutcome>,
    source_fingerprint: &'a MediaAggregateFingerprint,
    managed_workspace: Option<&'a revaer_media_runtime::workspace::ManagedWorkspace>,
    shutdown: Option<&'a RuntimeShutdownReceiver>,
}

mod checkpoints;
mod policy;

struct RuntimePreflightBuildInput {
    source_path: String,
    output_path: String,
    workspace_root_path: PathBuf,
    diagnostics_root: String,
    inspection: MediaInspection,
    desired_target: Option<DesiredTargetSnapshot>,
    base_video_policy: VideoTranscodePolicy,
    capabilities: CapabilitySnapshot,
    workspace_policy: WorkspacePolicy,
    capacity_probe: Arc<RuntimeCapacityProbe>,
    operation_costs: revaer_media_core::policy::OperationCosts,
}

struct DesiredVerificationContext<'a> {
    desired: &'a DesiredGraph,
    desired_target: Option<&'a DesiredTargetSnapshot>,
    expected_container_metadata: &'a [MetadataEntry],
    expected_chapters: &'a [ChapterInspection],
}

struct VerificationCheckOutcome<'a> {
    matched: bool,
    expected: &'a str,
    actual: &'a str,
    details: Option<&'a str>,
    error_code: &'static str,
}

struct SidecarPublicationContext<'a> {
    outputs: &'a [DesiredSidecarOutput],
    removals: &'a [String],
}

struct ReplacementExecutionContext<'a> {
    verification: &'a DesiredVerificationContext<'a>,
    workspace: &'a revaer_media_runtime::workspace::ManagedWorkspace,
    sidecars: SidecarPublicationContext<'a>,
    source_fingerprint: &'a MediaAggregateFingerprint,
    shutdown: Option<&'a RuntimeShutdownReceiver>,
}

#[derive(Debug, Default)]
struct CancellationSignal {
    requested: AtomicBool,
    interrupted: AtomicBool,
}

impl CancellationSignal {
    fn interrupt(&self) {
        self.interrupted.store(true, Ordering::Release);
        self.request();
    }

    fn request(&self) {
        self.requested.store(true, Ordering::Release);
    }
}

impl ExecutionControl for CancellationSignal {
    fn cancellation_requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
}

impl InspectCancellation for CancellationSignal {
    fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
}

trait FilesystemCapacityProbe {
    fn available_bytes(&self, path: &Path) -> Result<u64, String>;

    fn workspace_bytes(&self, path: &Path) -> Result<u64, String> {
        revaer_media_runtime::workspace::WorkspaceBudgetProbe::sample(
            &revaer_media_runtime::workspace::SystemWorkspaceBudgetProbe,
            path,
        )
        .map(|sample| sample.workspace_bytes)
        .map_err(|error| error.to_string())
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct SystemFilesystemCapacityProbe;

impl FilesystemCapacityProbe for SystemFilesystemCapacityProbe {
    fn available_bytes(&self, path: &Path) -> Result<u64, String> {
        revaer_fsops::available_bytes(path).map_err(|error| error.to_string())
    }
}

trait SourceFingerprintProbe {
    fn fingerprint(
        &self,
        media_path: &Path,
        source_root: &Path,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<MediaAggregateFingerprint>, FingerprintError>;
}

#[derive(Debug, Default, Clone, Copy)]
struct SystemSourceFingerprintProbe;

impl SourceFingerprintProbe for SystemSourceFingerprintProbe {
    fn fingerprint(
        &self,
        media_path: &Path,
        source_root: &Path,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
        fingerprint_media_aggregate_cancellable(media_path, source_root, cancelled)
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

#[derive(Debug, Clone)]
struct AudioAnalysisProcessOutput {
    status: ExitStatus,
    stderr: String,
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
        let output = run_audio_analysis_process(&self.ffmpeg_bin, &args)?;
        if !output.status.success() {
            let detail = if output.stderr.is_empty() {
                format!("status {}", output.status)
            } else {
                format!("status {}: {}", output.status, output.stderr)
            };
            return Err(format!("audio analyzer command failed: {detail}"));
        }
        parse_ebur128_summary(&output.stderr)
    }
}

fn run_audio_analysis_process(
    ffmpeg_bin: &str,
    args: &[String],
) -> Result<AudioAnalysisProcessOutput, String> {
    run_audio_analysis_process_with_timeout(ffmpeg_bin, args, AUDIO_ANALYSIS_TIMEOUT)
}

fn run_audio_analysis_process_with_timeout(
    ffmpeg_bin: &str,
    args: &[String],
    timeout: Duration,
) -> Result<AudioAnalysisProcessOutput, String> {
    let mut child = Command::new(ffmpeg_bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("audio analyzer command spawn failed: {error}"))?;
    let Some(stderr) = child.stderr.take() else {
        terminate_audio_analysis_process(&mut child)?;
        return Err("audio analyzer stderr pipe unavailable".to_string());
    };
    let stderr_reader = thread::spawn(move || read_bounded_audio_analysis_stderr(stderr));
    let started_at = Instant::now();
    let status = loop {
        if started_at.elapsed() >= timeout {
            terminate_audio_analysis_process(&mut child)?;
            let _stderr = join_audio_analysis_stderr_reader(stderr_reader)?;
            return Err(format!(
                "audio analyzer command timed out after {timeout:?}"
            ));
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("audio analyzer command wait failed: {error}"))?
        {
            break status;
        }
        thread::sleep(Duration::from_millis(100));
    };
    let stderr = join_audio_analysis_stderr_reader(stderr_reader)?;
    Ok(AudioAnalysisProcessOutput { status, stderr })
}

fn terminate_audio_analysis_process(child: &mut std::process::Child) -> Result<(), String> {
    match child.kill() {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {}
        Err(error) => return Err(format!("audio analyzer command kill failed: {error}")),
    }
    child
        .wait()
        .map_err(|error| format!("audio analyzer command reap failed: {error}"))?;
    Ok(())
}

fn join_audio_analysis_stderr_reader(
    stderr_reader: thread::JoinHandle<io::Result<String>>,
) -> Result<String, String> {
    let stderr = stderr_reader
        .join()
        .map_err(|_| "audio analyzer stderr reader thread panicked".to_string())?
        .map_err(|error| format!("audio analyzer stderr read failed: {error}"))?;
    Ok(stderr)
}

fn read_bounded_audio_analysis_stderr(mut stderr: impl Read) -> io::Result<String> {
    let mut retained = Vec::with_capacity(MAX_AUDIO_ANALYSIS_STDERR_BYTES);
    let mut scratch = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = stderr.read(&mut scratch)?;
        if read == 0 {
            break;
        }
        retain_audio_analysis_tail(&mut retained, &scratch[..read], &mut truncated);
    }
    let mut detail = String::from_utf8_lossy(&retained).trim().to_string();
    if truncated {
        detail.insert_str(0, AUDIO_ANALYSIS_TRUNCATION_MARKER);
    }
    Ok(detail)
}

fn retain_audio_analysis_tail(retained: &mut Vec<u8>, chunk: &[u8], truncated: &mut bool) {
    if chunk.len() >= MAX_AUDIO_ANALYSIS_STDERR_BYTES {
        retained.clear();
        retained.extend_from_slice(&chunk[chunk.len() - MAX_AUDIO_ANALYSIS_STDERR_BYTES..]);
        *truncated = true;
        return;
    }
    let overflow = retained
        .len()
        .saturating_add(chunk.len())
        .saturating_sub(MAX_AUDIO_ANALYSIS_STDERR_BYTES);
    if overflow > 0 {
        retained.drain(..overflow);
        *truncated = true;
    }
    retained.extend_from_slice(chunk);
}

/// Runtime worker that progresses queued media jobs to terminal status.
#[derive(Clone)]
pub(crate) struct MediaJobRuntime {
    store: MediaStore,
    inspector: Arc<RuntimeInspector>,
    command_runner: Arc<RuntimeCommandRunner>,
    capacity_probe: Arc<RuntimeCapacityProbe>,
    replacement_committer: Arc<RuntimeReplacementCommitter>,
    verification_executor: Arc<RuntimeVerificationExecutor>,
    audio_analyzer: Arc<RuntimeAudioAnalyzer>,
    source_fingerprint_probe: Arc<RuntimeSourceFingerprintProbe>,
    events: EventBus,
    telemetry: Metrics,
    tick_interval: Duration,
    workspace_policy: WorkspacePolicy,
    workspace_root: PathBuf,
    scratch_reservations: Arc<Mutex<BTreeMap<PathBuf, u64>>>,
}

impl MediaJobRuntime {
    /// Construct a production media job runtime.
    pub(crate) fn new(
        store: MediaStore,
        events: EventBus,
        telemetry: Metrics,
        workspace_root: PathBuf,
        native_process_supervisor: Arc<dyn NativeProcessSupervisor>,
    ) -> Self {
        Self::with_components(
            store,
            MediaJobRuntimeComponents {
                inspector: Arc::new(FfprobeInspectAdapter::new(
                    Arc::new(SupervisedInspectProbeExecutor::new(
                        native_process_supervisor,
                    )),
                    "ffprobe",
                )),
                command_runner: Arc::new(ProcessCommandRunner),
                capacity_probe: Arc::new(SystemFilesystemCapacityProbe),
                replacement_committer: Arc::new(SystemReplacementCommitter),
                verification_executor: Arc::new(SystemVerificationExecutor),
                audio_analyzer: Arc::new(SystemFfmpegAudioAnalysisAdapter::default()),
                source_fingerprint_probe: Arc::new(SystemSourceFingerprintProbe),
                events,
                telemetry,
                tick_interval: DEFAULT_TICK_INTERVAL,
                workspace_policy: WorkspacePolicy {
                    max_bytes: DEFAULT_WORKSPACE_MAX_BYTES,
                    reserve_bytes: DEFAULT_WORKSPACE_RESERVE_BYTES,
                },
                workspace_root,
                scratch_reservations: Arc::new(Mutex::new(BTreeMap::new())),
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
            source_fingerprint_probe: components.source_fingerprint_probe,
            events: components.events,
            telemetry: components.telemetry,
            tick_interval: components.tick_interval,
            workspace_policy: components.workspace_policy,
            workspace_root: components.workspace_root,
            scratch_reservations: components.scratch_reservations,
        }
    }

    /// Spawn the media worker loop.
    pub(crate) fn spawn(self, shutdown: RuntimeShutdownReceiver) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run_loop(shutdown).await;
        })
    }

    async fn run_loop(self, mut shutdown: RuntimeShutdownReceiver) {
        if runtime_shutdown::requested(&shutdown) {
            return;
        }
        while let Err(error) = self.recover_interrupted_replacements(Some(&shutdown)).await {
            if runtime_shutdown::requested(&shutdown) {
                return;
            }
            warn!(error = %error, "media job replacement recovery failed; worker remains paused");
            if runtime_shutdown::sleep_or_requested(self.tick_interval, &mut shutdown).await {
                return;
            }
        }
        while let Err(error) = self.resume_interrupted_jobs().await {
            warn!(error = %error, "media job interruption recovery failed; worker remains paused");
            if runtime_shutdown::sleep_or_requested(self.tick_interval, &mut shutdown).await {
                return;
            }
        }
        let mut ticker = interval(self.tick_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut workers = JoinSet::new();

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    while workers.len() < MAX_IN_PROCESS_MEDIA_JOBS {
                        match self.store.claim_next_job().await {
                            Ok(Some(job)) => {
                                let runtime = self.clone();
                                let worker_shutdown = shutdown.clone();
                                workers.spawn(async move {
                                    runtime.process_claimed_job_with_shutdown(job, worker_shutdown).await
                                });
                            }
                            Ok(None) => break,
                            Err(error) => {
                                warn!(error = %error, "media job runtime tick failed to claim work");
                                break;
                            }
                        }
                    }
                }
                joined = workers.join_next(), if !workers.is_empty() => {
                    match joined {
                        Some(Ok(Ok(()))) | None => {}
                        Some(Ok(Err(error))) => {
                            warn!(error = %error, "media job worker task failed");
                        }
                        Some(Err(error)) => {
                            warn!(error = %error, "media job worker task failed to join");
                        }
                    }
                }
                () = runtime_shutdown::changed(&mut shutdown) => {
                    while let Some(joined) = workers.join_next().await {
                        match joined {
                            Ok(Ok(())) => {}
                            Ok(Err(error)) => {
                                warn!(error = %error, "media job worker failed during shutdown");
                            }
                            Err(error) => {
                                warn!(error = %error, "media job worker task failed to join during shutdown");
                            }
                        }
                    }
                    return;
                }
            }
        }
    }

    async fn recover_interrupted_replacements(
        &self,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<(), MediaJobRuntimeError> {
        if shutdown.is_some_and(runtime_shutdown::requested) {
            return Err(MediaJobRuntimeError::Interrupted);
        }
        let terminal_events = self.store.list_unpublished_terminal_events().await?;
        let mut cursor = 0;
        loop {
            let candidates = self
                .store
                .list_replacement_recovery_candidates(cursor)
                .await?;
            if candidates.is_empty() {
                break;
            }
            for candidate in candidates {
                if shutdown.is_some_and(runtime_shutdown::requested) {
                    return Err(MediaJobRuntimeError::Interrupted);
                }
                cursor = candidate.attempt_id;
                let source_root = PathBuf::from(candidate.source_root);
                let recovery_root = source_root.clone();
                let replacement_backend = Arc::clone(&self.replacement_committer);
                let job_key =
                    replacement_job_key(candidate.media_job_public_id, candidate.claim_generation);
                let recovered = tokio::task::spawn_blocking(move || {
                    replacement_backend.recover_job(
                        &recovery_root,
                        &job_key,
                        candidate.terminal_committed,
                    )
                })
                .await
                .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))??;
                if let Some(mut transaction) = recovered {
                    if transaction.action == ReplacementRecoveryAction::Quarantined {
                        return Err(MediaJobRuntimeError::Verification(
                            "media_job_replacement_recovery_unresolved",
                        ));
                    }
                    if transaction.action == ReplacementRecoveryAction::Finalized {
                        info!(
                            media_job_public_id = transaction.job_key,
                            source_path = %transaction.source_path.display(),
                            "media job replacement recovery finalized verified transaction"
                        );
                        continue;
                    }
                    let (media_job_public_id, claim_generation) =
                        parse_replacement_job_key(&transaction.job_key)?;
                    self.finish_recovered_rollback(&mut transaction, &source_root, shutdown)
                        .await?;
                    let job = self.store.get_job(media_job_public_id).await?.ok_or(
                        MediaJobRuntimeError::Verification("media_job_recovery_job_missing"),
                    )?;
                    let resumed = if matches!(job.status_text.as_str(), "running" | "verifying") {
                        !self
                            .store
                            .interrupt_job(media_job_public_id, claim_generation)
                            .await?
                    } else {
                        false
                    };
                    info!(
                        %media_job_public_id,
                        source_path = %transaction.source_path.display(),
                        recovery_action = ?transaction.action,
                        resumed,
                        "media job replacement reconciled before interrupted work resumes"
                    );
                }
            }
        }
        for event in terminal_events {
            if shutdown.is_some_and(runtime_shutdown::requested) {
                return Err(MediaJobRuntimeError::Interrupted);
            }
            if event.event_kind != "completed" {
                return Err(MediaJobRuntimeError::InvalidTerminalEventKind(
                    event.event_kind,
                ));
            }
            self.publish_terminal_event(event.media_job_public_id)
                .await?;
        }
        Ok(())
    }

    async fn resume_interrupted_jobs(&self) -> Result<(), MediaJobRuntimeError> {
        let workspace_root =
            self.workspace_root
                .to_str()
                .ok_or(MediaJobRuntimeError::InvalidPath(
                    "media_job_workspace_path_invalid",
                ))?;
        loop {
            let resumed = self.store.resume_interrupted_jobs(workspace_root).await?;
            if resumed.is_empty() {
                return Ok(());
            }
            for job in resumed {
                info!(media_job_public_id = %job.media_job_public_id, outcome = %job.status_text,
                    "media job startup reconciled interruption without a failure retry");
            }
        }
    }

    async fn validate_claimed_roots(
        &self,
        job: &ClaimedMediaJobRow,
    ) -> Result<(), MediaJobRuntimeError> {
        let rows = revaer_data::media::job_roots::read_job_roots(
            self.store.pool(),
            job.media_job_public_id,
            job.attempt_number,
            job.claim_generation,
        )
        .await?;
        let expected = [
            Path::new(&job.source_root),
            Path::new(&job.output_root),
            self.workspace_root.as_path(),
        ];
        for (row, path) in rows.iter().take(3).zip(expected) {
            if row.binding_state != "bound"
                || row.canonical_path.as_deref().map(Path::new) != Some(path)
            {
                return Err(MediaJobRuntimeError::InvalidPath(
                    "media_root_identity_mismatch",
                ));
            }
        }
        Ok(())
    }

    async fn process_job(
        &self,
        job: ClaimedMediaJobRow,
        shutdown: Option<RuntimeShutdownReceiver>,
    ) {
        let started_at = Instant::now();
        let Some((workspace_paths, managed_workspace)) =
            self.prepare_job_workspace(&job, started_at).await
        else {
            return;
        };

        let terminal_state = match self
            .process_claimed_job(
                &job,
                &workspace_paths,
                managed_workspace.as_ref(),
                shutdown.as_ref(),
            )
            .await
        {
            Ok(state) => {
                info!(media_job_public_id = %job.media_job_public_id, "media job runtime processed job");
                state
            }
            Err(
                error @ (MediaJobRuntimeError::ScratchCapacityDeferred
                | MediaJobRuntimeError::Interrupted),
            ) => {
                let deferred = matches!(error, MediaJobRuntimeError::ScratchCapacityDeferred);
                match self
                    .store
                    .interrupt_job(job.media_job_public_id, job.claim_generation)
                    .await
                {
                    Ok(false) => {
                        let (outcome, message) = if deferred {
                            (
                                "deferred",
                                "media job returned to the queue until scratch capacity is available",
                            )
                        } else {
                            (
                                "interrupted",
                                "stopped media job remains resumable on its current attempt",
                            )
                        };
                        self.telemetry.inc_media_job_outcome(outcome, job.dry_run);
                        info!(media_job_public_id = %job.media_job_public_id, "{message}");
                        self.release_scratch_capacity(&workspace_paths.job_path);
                        return;
                    }
                    Ok(true) => TerminalWorkspaceState::Cancelled,
                    Err(error) => {
                        let message = if deferred {
                            "media job scratch deferral could not be persisted; workspace retained for startup recovery"
                        } else {
                            "stopped media job could not persist interruption; retained for startup recovery"
                        };
                        warn!(media_job_public_id = %job.media_job_public_id, error = %error, "{message}");
                        self.release_scratch_capacity(&workspace_paths.job_path);
                        return;
                    }
                }
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

        if let Some(workspace) = managed_workspace {
            self.cleanup_job_workspace(&job, &workspace, terminal_state);
        }
        self.release_scratch_capacity(&workspace_paths.job_path);
    }

    async fn prepare_job_workspace(
        &self,
        job: &ClaimedMediaJobRow,
        started_at: Instant,
    ) -> Option<(WorkspacePaths, Option<ManagedWorkspace>)> {
        if let Err(error) = self.validate_claimed_roots(job).await {
            warn!(media_job_public_id = %job.media_job_public_id, error = %error, "media job immutable root admission failed");
            self.telemetry.inc_media_job_failure(error.category());
            self.telemetry.inc_media_job_outcome("failed", job.dry_run);
            self.persist_failure(job, error).await;
            return None;
        }
        let workspace_key = crate::media_workspace_identity::workspace_key(
            job.media_job_public_id,
            job.attempt_number,
            job.claim_generation,
        );
        let workspace = if job.dry_run {
            project_managed_workspace(&self.workspace_root, &workspace_key)
                .map(|paths| (paths, None))
        } else {
            create_or_resume_managed_workspace(&self.workspace_root, &workspace_key)
                .map(|workspace| (workspace.paths.clone(), Some(workspace)))
        };
        let (workspace_paths, managed_workspace) = match workspace {
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
                self.persist_failure(job, MediaJobRuntimeError::Workspace(error))
                    .await;
                return None;
            }
        };
        Some((workspace_paths, managed_workspace))
    }

    fn cleanup_job_workspace(
        &self,
        job: &ClaimedMediaJobRow,
        workspace: &revaer_media_runtime::workspace::ManagedWorkspace,
        terminal_state: TerminalWorkspaceState,
    ) {
        if let Err(error) = cleanup_terminal_workspace(
            workspace,
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

    fn reserve_scratch_capacity(
        &self,
        job_workspace_path: &Path,
        required_peak_bytes: u64,
    ) -> Result<bool, MediaJobRuntimeError> {
        let workspace_bytes = self
            .capacity_probe
            .workspace_bytes(&self.workspace_root)
            .map_err(MediaJobRuntimeError::Capacity)?;
        let free_bytes = self
            .capacity_probe
            .available_bytes(&self.workspace_root)
            .map_err(MediaJobRuntimeError::Capacity)?;
        let job_workspace_bytes = self
            .capacity_probe
            .workspace_bytes(job_workspace_path)
            .map_err(MediaJobRuntimeError::Capacity)?;
        let mut reservations = self.scratch_reservations.lock().map_err(|_| {
            MediaJobRuntimeError::Capacity("scratch_reservation_lock_poisoned".into())
        })?;
        let mut reserved_future_bytes = 0_u64;
        for (path, peak_bytes) in reservations.iter() {
            if path == job_workspace_path {
                continue;
            }
            let current_bytes = self
                .capacity_probe
                .workspace_bytes(path)
                .map_err(MediaJobRuntimeError::Capacity)?;
            reserved_future_bytes =
                reserved_future_bytes.saturating_add(peak_bytes.saturating_sub(current_bytes));
        }
        let new_future_bytes = required_peak_bytes.saturating_sub(job_workspace_bytes);
        let projected_workspace_bytes = workspace_bytes
            .saturating_add(reserved_future_bytes)
            .saturating_add(new_future_bytes);
        let projected_free_demand = self
            .workspace_policy
            .reserve_bytes
            .saturating_add(reserved_future_bytes)
            .saturating_add(new_future_bytes);
        if projected_workspace_bytes > self.workspace_policy.max_bytes
            || free_bytes < projected_free_demand
        {
            return Ok(false);
        }
        reservations.insert(job_workspace_path.to_path_buf(), required_peak_bytes);
        drop(reservations);
        Ok(true)
    }

    fn release_scratch_capacity(&self, job_workspace_path: &Path) {
        match self.scratch_reservations.lock() {
            Ok(mut reservations) => {
                reservations.remove(job_workspace_path);
            }
            Err(error) => {
                warn!(error = %error, path = %job_workspace_path.display(), "media job scratch reservation could not be released");
            }
        }
    }

    async fn process_claimed_job_with_shutdown(
        &self,
        job: ClaimedMediaJobRow,
        shutdown: RuntimeShutdownReceiver,
    ) -> Result<(), MediaJobRuntimeError> {
        if runtime_shutdown::requested(&shutdown) {
            self.interrupt_claimed_job_for_shutdown(&job).await?;
        } else {
            self.process_job(job, Some(shutdown)).await;
        }
        Ok(())
    }

    async fn interrupt_claimed_job_for_shutdown(
        &self,
        job: &ClaimedMediaJobRow,
    ) -> Result<(), MediaJobRuntimeError> {
        let cancelled = self
            .store
            .interrupt_job(job.media_job_public_id, job.claim_generation)
            .await?;
        let outcome = if cancelled {
            "cancelled"
        } else {
            "interrupted"
        };
        self.telemetry.inc_media_job_outcome(outcome, job.dry_run);
        info!(media_job_public_id = %job.media_job_public_id, outcome, "media job stopped before shutdown");
        Ok(())
    }

    async fn process_claimed_job(
        &self,
        job: &ClaimedMediaJobRow,
        workspace: &WorkspacePaths,
        managed_workspace: Option<&revaer_media_runtime::workspace::ManagedWorkspace>,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<TerminalWorkspaceState, MediaJobRuntimeError> {
        self.ensure_not_cancelled(job).await?;
        let source_fingerprint = self.capture_source_fingerprint(job, shutdown).await?;
        self.append_phase(job, 0, "inspect_plan", "running", None)
            .await?;

        let final_output_path = resolve_output_path(job)?;
        let output_path = resolve_workspace_output_path(&final_output_path, workspace)?;
        if !job.dry_run {
            validate_existing_ancestor_bounds(&job.source_path, &job.source_root, "source_path")?;
            validate_existing_ancestor_bounds(&final_output_path, &job.output_root, "output_path")?;
        }

        let capabilities = self.load_capability_snapshot().await?;
        let preflight = self
            .build_preflight_evaluation(job, output_path.clone(), workspace, capabilities, shutdown)
            .await?;
        self.ensure_not_cancelled(job).await?;
        self.publish_event(Event::MediaJobInspected {
            media_job_public_id: job.media_job_public_id,
        });
        self.persist_preflight_audits(job, &preflight.evaluation)
            .await?;
        let RuntimePreflightEvaluation {
            evaluation,
            desired,
            desired_target,
            expected_container_metadata,
            expected_chapters,
            planning_outcome,
        } = preflight;

        match evaluation {
            JobPreflightEvaluation::Failed(report)
                if report.failed_stage == "workspace_capacity" =>
            {
                Err(MediaJobRuntimeError::ScratchCapacityDeferred)
            }
            JobPreflightEvaluation::Failed(report) => {
                self.handle_preflight_failed(job, report).await
            }
            JobPreflightEvaluation::Ready(report) => {
                let required_peak_bytes = report.planned.estimated_workspace_bytes;
                if !job.dry_run
                    && !self.reserve_scratch_capacity(&workspace.job_path, required_peak_bytes)?
                {
                    warn!(
                        media_job_public_id = %job.media_job_public_id,
                        required_peak_bytes,
                        "media job deferred because concurrent scratch demand exceeds the configured budget"
                    );
                    return Err(MediaJobRuntimeError::ScratchCapacityDeferred);
                }
                self.handle_preflight_ready(
                    job,
                    *report,
                    PreflightReadyContext {
                        desired: &desired,
                        desired_target: desired_target.as_ref(),
                        expected_container_metadata: &expected_container_metadata,
                        expected_chapters: &expected_chapters,
                        planning_outcome: planning_outcome.as_ref(),
                        source_fingerprint: &source_fingerprint,
                        managed_workspace,
                        shutdown,
                    },
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
            job,
            0,
            report.failed_stage,
            "failed",
            Some(report.error_code),
        )
        .await?;
        self.append_verification_check(
            job,
            &AppendMediaJobVerificationCheckInput {
                media_job_public_id: job.media_job_public_id,
                check_index: 0,
                check_kind: "preflight",
                check_status: "failed",
                expected_value: Some("ready"),
                actual_value: Some(report.error_code),
                details_text: Some(report.error_detail),
            },
        )
        .await?;
        self.telemetry.inc_media_job_failure("preflight");
        self.mark_failed(job, report.error_code).await?;
        Ok(TerminalWorkspaceState::Failed)
    }

    async fn handle_preflight_ready(
        &self,
        job: &ClaimedMediaJobRow,
        report: JobPreflightReport,
        context: PreflightReadyContext<'_>,
    ) -> Result<TerminalWorkspaceState, MediaJobRuntimeError> {
        let verification_context = DesiredVerificationContext {
            desired: context.desired,
            desired_target: context.desired_target,
            expected_container_metadata: context.expected_container_metadata,
            expected_chapters: context.expected_chapters,
        };
        self.append_phase(job, 0, "inspect_plan", "completed", None)
            .await?;
        self.persist_ready_plan(job, &report, context.planning_outcome)
            .await?;
        self.publish_event(Event::MediaJobPlanned {
            media_job_public_id: job.media_job_public_id,
            operation_count: usize_to_u64_saturating(report.summary.total_operations),
            dry_run: job.dry_run,
        });
        if job.dry_run {
            return self
                .complete_dry_run(job, context.source_fingerprint, context.shutdown)
                .await;
        }
        let managed_workspace =
            context
                .managed_workspace
                .ok_or(MediaJobRuntimeError::Verification(
                    "media_job_managed_workspace_missing",
                ))?;
        managed_workspace.validate()?;
        if planned_job_is_noop(&report) {
            return self
                .complete_noop_job(
                    job,
                    &verification_context,
                    context.source_fingerprint,
                    context.shutdown,
                )
                .await;
        }

        self.store
            .mark_job_status(
                job.media_job_public_id,
                job.claim_generation,
                "verifying",
                None,
            )
            .await?;
        self.publish_event(Event::MediaJobExecutionStarted {
            media_job_public_id: job.media_job_public_id,
        });
        self.append_phase(job, 1, "execute", "running", None)
            .await?;
        let sidecar_outputs = report.planned.sidecar_outputs.clone();
        let sidecar_removals = report.planned.sidecar_removals.clone();
        let committed = self
            .execute_verified_replacement(
                job,
                report.steps,
                ReplacementExecutionContext {
                    verification: &verification_context,
                    workspace: managed_workspace,
                    sidecars: SidecarPublicationContext {
                        outputs: &sidecar_outputs,
                        removals: &sidecar_removals,
                    },
                    source_fingerprint: context.source_fingerprint,
                    shutdown: context.shutdown,
                },
            )
            .await?;
        managed_workspace.validate()?;
        let cancelled = self
            .store
            .commit_replacement_terminal(
                job.media_job_public_id,
                job.claim_generation,
                job.cancel_generation,
            )
            .await?;
        if cancelled {
            drop(committed);
            self.recover_replacement_job(job, context.shutdown).await?;
            return Ok(TerminalWorkspaceState::Cancelled);
        }
        let replacement_backend = Arc::clone(&self.replacement_committer);
        tokio::task::spawn_blocking(move || replacement_backend.finalize(committed))
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))??;
        self.publish_terminal_event(job.media_job_public_id).await?;
        Ok(TerminalWorkspaceState::Completed)
    }

    async fn complete_dry_run(
        &self,
        job: &ClaimedMediaJobRow,
        source_fingerprint: &MediaAggregateFingerprint,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<TerminalWorkspaceState, MediaJobRuntimeError> {
        self.append_verification_check(
            job,
            &AppendMediaJobVerificationCheckInput {
                media_job_public_id: job.media_job_public_id,
                check_index: 0,
                check_kind: "dry_run_preflight",
                check_status: "passed",
                expected_value: Some("execution_skipped"),
                actual_value: Some("dry_run"),
                details_text: Some("dry-run job completed without destructive execution"),
            },
        )
        .await?;
        self.ensure_not_cancelled(job).await?;
        self.revalidate_source_fingerprint(job, source_fingerprint, shutdown)
            .await?;
        if self.complete_or_cancel(job).await? {
            return Ok(TerminalWorkspaceState::Cancelled);
        }
        self.publish_event(Event::MediaJobCompleted {
            media_job_public_id: job.media_job_public_id,
        });
        Ok(TerminalWorkspaceState::Completed)
    }

    async fn publish_terminal_event(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<(), MediaJobRuntimeError> {
        self.events
            .publish(Event::MediaJobCompleted {
                media_job_public_id,
            })
            .map_err(|error| MediaJobRuntimeError::EventPublish(error.to_string()))?;
        self.store
            .mark_terminal_event_published(media_job_public_id)
            .await?;
        Ok(())
    }

    async fn complete_noop_job(
        &self,
        job: &ClaimedMediaJobRow,
        verification_context: &DesiredVerificationContext<'_>,
        source_fingerprint: &MediaAggregateFingerprint,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<TerminalWorkspaceState, MediaJobRuntimeError> {
        self.revalidate_source_fingerprint(job, source_fingerprint, shutdown)
            .await?;
        self.store
            .mark_job_status(
                job.media_job_public_id,
                job.claim_generation,
                "verifying",
                None,
            )
            .await?;
        self.append_phase(job, 1, "verify_noop", "running", None)
            .await?;
        self.verify_graph_check(
            job,
            1,
            "source_graph",
            &job.source_path,
            verification_context,
            shutdown,
        )
        .await?;
        self.append_phase(job, 1, "verify_noop", "completed", None)
            .await?;
        self.ensure_not_cancelled(job).await?;
        self.revalidate_source_fingerprint(job, source_fingerprint, shutdown)
            .await?;
        if self.complete_or_cancel(job).await? {
            return Ok(TerminalWorkspaceState::Cancelled);
        }
        self.publish_event(Event::MediaJobCompleted {
            media_job_public_id: job.media_job_public_id,
        });
        Ok(TerminalWorkspaceState::Completed)
    }

    async fn capture_source_fingerprint(
        &self,
        job: &ClaimedMediaJobRow,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<MediaAggregateFingerprint, MediaJobRuntimeError> {
        let observed = self.observe_source_fingerprint(job, shutdown).await?;
        validate_claimed_source_fingerprint(job, observed)
    }

    async fn revalidate_source_fingerprint(
        &self,
        job: &ClaimedMediaJobRow,
        expected: &MediaAggregateFingerprint,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<(), MediaJobRuntimeError> {
        match self.observe_source_fingerprint(job, shutdown).await? {
            Some(observed) if observed == *expected => Ok(()),
            Some(_) | None => Err(MediaJobRuntimeError::SourceFingerprint(
                "media_job_source_fingerprint_changed",
            )),
        }
    }

    async fn finish_recovered_rollback(
        &self,
        transaction: &mut revaer_media_runtime::replacement::RecoveredReplacement,
        source_root: &Path,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<(), MediaJobRuntimeError> {
        if transaction.action != ReplacementRecoveryAction::RolledBack {
            return Ok(());
        }
        let (job_id, generation) = parse_replacement_job_key(&transaction.job_key)?;
        let cleanup =
            transaction
                .pending_cleanup
                .take()
                .ok_or(MediaJobRuntimeError::Verification(
                    "media_job_recovery_backup_not_retained",
                ))?;
        self.refresh_rolled_back_source(
            job_id,
            generation,
            &transaction.source_path,
            source_root,
            shutdown,
        )
        .await?;
        let backend = Arc::clone(&self.replacement_committer);
        tokio::task::spawn_blocking(move || backend.discard_prepared(cleanup))
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))??;
        Ok(())
    }

    async fn refresh_rolled_back_source(
        &self,
        job_id: Uuid,
        generation: i64,
        source_path: &Path,
        source_root: &Path,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<(), MediaJobRuntimeError> {
        let probe = Arc::clone(&self.source_fingerprint_probe);
        let media = source_path.to_path_buf();
        let root = source_root.to_path_buf();
        let reader_shutdown = shutdown.cloned();
        let observed = tokio::task::spawn_blocking(move || {
            probe.fingerprint(&media, &root, &|| {
                reader_shutdown
                    .as_ref()
                    .is_some_and(runtime_shutdown::requested)
            })
        })
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?
        .map_err(|error| match error {
            FingerprintError::Cancelled => MediaJobRuntimeError::Interrupted,
            other => MediaJobRuntimeError::Fingerprint(other),
        })?
        .ok_or(MediaJobRuntimeError::SourceFingerprint(
            "media_job_source_fingerprint_unavailable",
        ))?;
        if shutdown.is_some_and(runtime_shutdown::requested) {
            return Err(MediaJobRuntimeError::Interrupted);
        }
        self.store
            .refresh_restored_source(&revaer_data::media::job_roots::RestoredSourceInput {
                job_id,
                generation,
                source_path: source_path
                    .to_str()
                    .ok_or(MediaJobRuntimeError::InvalidPath(
                        "media_job_recovery_source_encoding",
                    ))?,
                identity: &observed.identity,
                size_bytes: observed.size_bytes,
                modified_ns: observed.modified_ns,
                changed_ns: observed.changed_ns,
                sha256: &observed.sha256,
            })
            .await?;
        Ok(())
    }

    async fn observe_source_fingerprint(
        &self,
        job: &ClaimedMediaJobRow,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<Option<MediaAggregateFingerprint>, MediaJobRuntimeError> {
        let probe = Arc::clone(&self.source_fingerprint_probe);
        let media_path = PathBuf::from(&job.source_path);
        let source_root = PathBuf::from(&job.source_root);
        let signal = Arc::new(CancellationSignal::default());
        let read_signal = Arc::clone(&signal);
        let (stop_tx, stop_rx) = watch::channel(false);
        let monitor = tokio::spawn(monitor_job_control(
            self.store.clone(),
            job.media_job_public_id,
            job.claim_generation,
            job.cancel_generation,
            Arc::clone(&signal),
            stop_rx,
            shutdown.cloned(),
        ));
        let observed = tokio::task::spawn_blocking(move || {
            probe.fingerprint(&media_path, &source_root, &|| {
                read_signal.requested.load(Ordering::Acquire)
            })
        })
        .await;
        drop(stop_tx);
        let cancellation_requested = monitor
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))??;
        let observed = observed.map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        if signal.interrupted.load(Ordering::Acquire)
            && matches!(observed, Ok(_) | Err(FingerprintError::Cancelled))
        {
            return Err(MediaJobRuntimeError::Interrupted);
        }
        if cancellation_requested || matches!(observed, Err(FingerprintError::Cancelled)) {
            return Err(MediaJobRuntimeError::Cancelled);
        }
        observed.map_err(MediaJobRuntimeError::Fingerprint)
    }

    async fn build_preflight_evaluation(
        &self,
        job: &ClaimedMediaJobRow,
        output_path: String,
        workspace: &WorkspacePaths,
        capabilities: CapabilitySnapshot,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<RuntimePreflightEvaluation, MediaJobRuntimeError> {
        let workspace_policy = self.workspace_policy.clone();
        let capacity_probe = Arc::clone(&self.capacity_probe);
        let source_path = job.source_path.clone();
        let desired_target_streams = self
            .store
            .list_job_desired_target_streams(job.media_job_public_id)
            .await?;
        let desired_target = desired_target_from_job(job, desired_target_streams)?;
        let operation_costs = policy::compile_costs(
            self.store
                .list_job_operation_costs(job.media_job_public_id)
                .await?,
        )?;
        let base_video_policy =
            video_policy_from_policy_intent(job.policy_video_intent.as_deref())?;
        let workspace_root_path = workspace.root_path.clone();
        let diagnostics_root = path_to_string(&workspace.diagnostics_path, "diagnostics_path")?;
        let inspection = self
            .inspect_media(job, source_path.clone(), shutdown)
            .await?;
        tokio::task::spawn_blocking(move || {
            compile_runtime_preflight(RuntimePreflightBuildInput {
                source_path,
                output_path,
                workspace_root_path,
                diagnostics_root,
                inspection,
                desired_target,
                base_video_policy,
                capabilities,
                workspace_policy,
                capacity_probe,
                operation_costs,
            })
        })
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?
    }

    async fn execute_steps(
        &self,
        job: &ClaimedMediaJobRow,
        steps: Vec<ExecutionStep>,
        workspace: &revaer_media_runtime::workspace::ManagedWorkspace,
        source_fingerprint: &MediaAggregateFingerprint,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<(), MediaJobRuntimeError> {
        self.ensure_not_cancelled(job).await?;
        workspace.validate()?;
        self.revalidate_source_fingerprint(job, source_fingerprint, shutdown)
            .await?;
        let signal = Arc::new(CancellationSignal::default());
        let monitor_store = self.store.clone();
        let media_job_public_id = job.media_job_public_id;
        let observed_cancel_generation = job.cancel_generation;
        let (stop_tx, stop_rx) = watch::channel(false);
        let monitor = tokio::spawn(monitor_job_control(
            monitor_store,
            media_job_public_id,
            job.claim_generation,
            observed_cancel_generation,
            Arc::clone(&signal),
            stop_rx,
            shutdown.cloned(),
        ));
        let execution = self
            .execute_checkpointed_steps(
                job,
                &steps,
                workspace,
                source_fingerprint,
                Arc::clone(&signal),
                shutdown,
            )
            .await;
        drop(stop_tx);
        let cancellation_requested = monitor
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))??;
        if signal.interrupted.load(Ordering::Acquire) {
            return match execution {
                Ok(()) | Err(MediaJobRuntimeError::Cancelled) => {
                    Err(MediaJobRuntimeError::Interrupted)
                }
                Err(error) => Err(error),
            };
        }
        if cancellation_requested {
            return Err(MediaJobRuntimeError::Cancelled);
        }
        workspace.validate()?;
        execution
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
        context: ReplacementExecutionContext<'_>,
    ) -> Result<CommittedReplacement, MediaJobRuntimeError> {
        let ReplacementExecutionContext {
            verification,
            workspace,
            sidecars,
            source_fingerprint,
            shutdown,
        } = context;
        self.ensure_not_cancelled(job).await?;
        let candidate_output_path = self
            .execute_and_verify_candidate(
                job,
                &steps,
                verification,
                workspace,
                source_fingerprint,
                shutdown,
            )
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
                output_path,
                sidecars.outputs,
                sidecars.removals,
                source_fingerprint,
                shutdown,
            )
            .await?;
        let verification = self
            .verify_final_desired_state(job, verification, &sidecars, shutdown)
            .await;
        let verification = match verification {
            Ok(()) => self.ensure_not_cancelled(job).await,
            Err(error) => Err(error),
        };
        if let Err(error) = verification {
            drop(committed);
            self.recover_replacement_job(job, shutdown).await?;
            return Err(error);
        }
        Ok(committed)
    }

    async fn execute_and_verify_candidate(
        &self,
        job: &ClaimedMediaJobRow,
        steps: &[ExecutionStep],
        verification_context: &DesiredVerificationContext<'_>,
        workspace: &revaer_media_runtime::workspace::ManagedWorkspace,
        source_fingerprint: &MediaAggregateFingerprint,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<String, MediaJobRuntimeError> {
        let candidate_output_path = replacement_output_path(steps)?;
        self.revalidate_source_fingerprint(job, source_fingerprint, shutdown)
            .await?;
        let source_inspection = self
            .inspect_media(job, job.source_path.clone(), shutdown)
            .await?;
        self.ensure_not_cancelled(job).await?;
        let pre_replace_steps = steps
            .iter()
            .filter(|step| !matches!(step, ExecutionStep::AtomicReplace { .. }))
            .filter(|step| !matches!(step, ExecutionStep::QuarantineFailedOutput { .. }))
            .cloned()
            .collect::<Vec<_>>();
        self.execute_steps(
            job,
            pre_replace_steps,
            workspace,
            source_fingerprint,
            shutdown,
        )
        .await?;
        self.ensure_not_cancelled(job).await?;
        let candidate_inspection = match self
            .inspect_media(job, candidate_output_path.clone(), shutdown)
            .await
        {
            Ok(inspection) => inspection,
            Err(error) => {
                self.quarantine_candidate(steps).await?;
                return Err(error);
            }
        };
        let graph_verification = self
            .verify_graph_inspection(
                job,
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
                shutdown,
            )
            .await?;
        if let Err(error) = self.persist_safety_verification(job, &safety_report).await {
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
        shutdown: Option<&RuntimeShutdownReceiver>,
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
            job.claim_generation,
            observed_cancel_generation,
            Arc::clone(&signal),
            stop_rx,
            shutdown.cloned(),
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
        if signal.interrupted.load(Ordering::Acquire) {
            return match verification {
                Ok(_) | Err(VerificationExecutionError::Cancelled) => {
                    Err(MediaJobRuntimeError::Interrupted)
                }
                Err(VerificationExecutionError::Failed(_)) => {
                    Err(MediaJobRuntimeError::Verification(
                        "media_job_candidate_verification_executor_failed",
                    ))
                }
            };
        }
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
        candidate_path: &str,
        sidecar_outputs: &[DesiredSidecarOutput],
        sidecar_removals: &[String],
        source_fingerprint: &MediaAggregateFingerprint,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<CommittedReplacement, MediaJobRuntimeError> {
        self.ensure_not_cancelled(job).await?;
        self.revalidate_source_fingerprint(job, source_fingerprint, shutdown)
            .await?;
        let replacement_backend = Arc::clone(&self.replacement_committer);
        let job_key = replacement_job_key(job.media_job_public_id, job.claim_generation);
        let source_root = PathBuf::from(&job.source_root);
        let source = PathBuf::from(&job.source_path);
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
        let pre_commit_gate = match self.ensure_not_cancelled(job).await {
            Ok(()) => {
                self.revalidate_source_fingerprint(job, source_fingerprint, shutdown)
                    .await
            }
            Err(error) => Err(error),
        };
        if let Err(error) = pre_commit_gate {
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
                self.recover_replacement_job(job, shutdown).await?;
                Err(MediaJobRuntimeError::Replacement(error))
            }
        }
    }

    async fn recover_replacement_job(
        &self,
        job: &ClaimedMediaJobRow,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<(), MediaJobRuntimeError> {
        let source_root = PathBuf::from(&job.source_root);
        let job_key = replacement_job_key(job.media_job_public_id, job.claim_generation);
        let terminal_committed = self
            .store
            .list_unpublished_terminal_events()
            .await?
            .iter()
            .any(|event| {
                event.media_job_public_id == job.media_job_public_id
                    && event.claim_generation == job.claim_generation
            });
        let replacement_backend = Arc::clone(&self.replacement_committer);
        let recovery_root = source_root.clone();
        let recovery = tokio::task::spawn_blocking(move || {
            replacement_backend.recover_job(&recovery_root, &job_key, terminal_committed)
        })
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        match recovery {
            Ok(recovered) => {
                if let Some(mut transaction) = recovered {
                    self.finish_recovered_rollback(&mut transaction, &source_root, shutdown)
                        .await?;
                }
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
        job: &ClaimedMediaJobRow,
        check_index: i32,
        check_kind: &'static str,
        output_path: &str,
        verification_context: &DesiredVerificationContext<'_>,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<(), MediaJobRuntimeError> {
        let inspection = self
            .inspect_media(job, output_path.to_string(), shutdown)
            .await?;
        self.verify_graph_inspection(
            job,
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
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<(), MediaJobRuntimeError> {
        let inspection = self
            .inspect_media(job, job.source_path.clone(), shutdown)
            .await?;
        self.verify_graph_inspection(job, 20, "final_graph", &inspection, verification_context)
            .await?;
        let matched = sidecar_state_matches(
            &inspection,
            sidecar_publication.outputs,
            sidecar_publication.removals,
        );
        self.append_verification_check(
            job,
            &AppendMediaJobVerificationCheckInput {
                media_job_public_id: job.media_job_public_id,
                check_index: 21,
                check_kind: "final_sidecar_state",
                check_status: if matched { "passed" } else { "failed" },
                expected_value: Some("desired_sidecar_state"),
                actual_value: Some(if matched { "matched" } else { "mismatched" }),
                details_text: Some(&inspection.graph.source_path),
            },
        )
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
        job: &ClaimedMediaJobRow,
        check_index: i32,
        check_kind: &'static str,
        inspection: &MediaInspection,
        verification_context: &DesiredVerificationContext<'_>,
    ) -> Result<(), MediaJobRuntimeError> {
        let matched = media_graph_matches_desired(&inspection.graph, verification_context.desired);
        let check_status = if matched { "passed" } else { "failed" };
        let actual_value = if matched { "matched" } else { "mismatched" };
        self.append_verification_check(
            job,
            &AppendMediaJobVerificationCheckInput {
                media_job_public_id: job.media_job_public_id,
                check_index,
                check_kind,
                check_status,
                expected_value: Some("desired_graph"),
                actual_value: Some(actual_value),
                details_text: Some(&inspection.graph.source_path),
            },
        )
        .await?;
        if matched {
            self.verify_video_constraints_inspection(
                job,
                video_constraint_check_index(check_kind, check_index),
                video_constraint_check_kind(check_kind),
                inspection,
                verification_context.desired,
                verification_context.desired_target,
            )
            .await?;
            self.verify_audio_constraints_inspection(
                job,
                audio_constraint_check_index(check_kind, check_index),
                audio_constraint_check_kind(check_kind),
                inspection,
                verification_context.desired,
                verification_context.desired_target,
            )
            .await?;
            self.verify_container_metadata_inspection(
                job,
                container_metadata_check_index(check_kind, check_index),
                container_metadata_check_kind(check_kind),
                inspection,
                verification_context.expected_container_metadata,
            )
            .await?;
            self.verify_chapter_timeline_inspection(
                job,
                chapter_timeline_check_index(check_kind, check_index),
                chapter_timeline_check_kind(check_kind),
                inspection,
                verification_context.expected_chapters,
            )
            .await
        } else {
            self.telemetry.inc_media_job_failure("verification");
            self.publish_event(Event::MediaJobVerificationFailed {
                media_job_public_id: job.media_job_public_id,
                check_kind: check_kind.to_string(),
                error_code: "media_job_output_graph_mismatch".to_string(),
            });
            Err(MediaJobRuntimeError::Verification(
                "media_job_output_graph_mismatch",
            ))
        }
    }

    async fn verify_container_metadata_inspection(
        &self,
        job: &ClaimedMediaJobRow,
        check_index: i32,
        check_kind: &'static str,
        inspection: &MediaInspection,
        expected_metadata: &[MetadataEntry],
    ) -> Result<(), MediaJobRuntimeError> {
        let verification = container_metadata_matches_inspection(inspection, expected_metadata);
        self.complete_verification_check(
            job,
            check_index,
            check_kind,
            VerificationCheckOutcome {
                matched: verification.matched,
                expected: verification.expected.as_str(),
                actual: verification.actual.as_str(),
                details: verification.details.as_deref(),
                error_code: "media_job_output_container_metadata_mismatch",
            },
        )
        .await
    }

    async fn verify_video_constraints_inspection(
        &self,
        job: &ClaimedMediaJobRow,
        check_index: i32,
        check_kind: &'static str,
        inspection: &MediaInspection,
        desired: &DesiredGraph,
        desired_target: Option<&DesiredTargetSnapshot>,
    ) -> Result<(), MediaJobRuntimeError> {
        let verification = video_constraints_match_inspection(inspection, desired, desired_target);
        self.complete_verification_check(
            job,
            check_index,
            check_kind,
            VerificationCheckOutcome {
                matched: verification.matched,
                expected: verification.expected.as_str(),
                actual: verification.actual.as_str(),
                details: verification.details.as_deref(),
                error_code: "media_job_output_video_constraints_mismatch",
            },
        )
        .await
    }

    async fn verify_audio_constraints_inspection(
        &self,
        job: &ClaimedMediaJobRow,
        check_index: i32,
        check_kind: &'static str,
        inspection: &MediaInspection,
        desired: &DesiredGraph,
        desired_target: Option<&DesiredTargetSnapshot>,
    ) -> Result<(), MediaJobRuntimeError> {
        let verification = self
            .audio_constraints_match_inspection(inspection, desired, desired_target)
            .await?;
        self.complete_verification_check(
            job,
            check_index,
            check_kind,
            VerificationCheckOutcome {
                matched: verification.matched,
                expected: verification.expected.as_str(),
                actual: verification.actual.as_str(),
                details: verification.details.as_deref(),
                error_code: "media_job_output_audio_constraints_mismatch",
            },
        )
        .await
    }

    async fn verify_chapter_timeline_inspection(
        &self,
        job: &ClaimedMediaJobRow,
        check_index: i32,
        check_kind: &'static str,
        inspection: &MediaInspection,
        expected_chapters: &[ChapterInspection],
    ) -> Result<(), MediaJobRuntimeError> {
        let verification = chapter_timeline_matches_inspection(inspection, expected_chapters);
        self.complete_verification_check(
            job,
            check_index,
            check_kind,
            VerificationCheckOutcome {
                matched: verification.matched,
                expected: verification.expected.as_str(),
                actual: verification.actual.as_str(),
                details: verification.details.as_deref(),
                error_code: "media_job_output_chapter_timeline_mismatch",
            },
        )
        .await
    }

    async fn complete_verification_check(
        &self,
        job: &ClaimedMediaJobRow,
        check_index: i32,
        check_kind: &'static str,
        outcome: VerificationCheckOutcome<'_>,
    ) -> Result<(), MediaJobRuntimeError> {
        self.append_verification_check(
            job,
            &AppendMediaJobVerificationCheckInput {
                media_job_public_id: job.media_job_public_id,
                check_index,
                check_kind,
                check_status: if outcome.matched { "passed" } else { "failed" },
                expected_value: Some(outcome.expected),
                actual_value: Some(outcome.actual),
                details_text: outcome.details,
            },
        )
        .await?;
        if outcome.matched {
            Ok(())
        } else {
            self.telemetry.inc_media_job_failure("verification");
            self.publish_event(Event::MediaJobVerificationFailed {
                media_job_public_id: job.media_job_public_id,
                check_kind: check_kind.to_string(),
                error_code: outcome.error_code.to_string(),
            });
            Err(MediaJobRuntimeError::Verification(outcome.error_code))
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
        job: &ClaimedMediaJobRow,
        report: &VerificationReport,
    ) -> Result<(), MediaJobRuntimeError> {
        for (offset, check) in report.checks.iter().enumerate() {
            let check_index = i32::try_from(offset)
                .map_err(index_error("verification check index"))?
                .checked_add(11)
                .ok_or(MediaJobRuntimeError::Verification(
                    "media_job_verification_check_index_overflow",
                ))?;
            self.append_verification_check(
                job,
                &AppendMediaJobVerificationCheckInput {
                    media_job_public_id: job.media_job_public_id,
                    check_index,
                    check_kind: check.kind,
                    check_status: if check.passed { "passed" } else { "failed" },
                    expected_value: Some(&check.expected),
                    actual_value: Some(&check.actual),
                    details_text: check.details.as_deref(),
                },
            )
            .await?;
        }
        if report.passed() {
            return Ok(());
        }
        self.telemetry.inc_media_job_failure("verification");
        self.publish_event(Event::MediaJobVerificationFailed {
            media_job_public_id: job.media_job_public_id,
            check_kind: "candidate_safety".to_string(),
            error_code: "media_job_candidate_safety_verification_failed".to_string(),
        });
        Err(MediaJobRuntimeError::Verification(
            "media_job_candidate_safety_verification_failed",
        ))
    }

    async fn inspect_media(
        &self,
        job: &ClaimedMediaJobRow,
        source_path: String,
        shutdown: Option<&RuntimeShutdownReceiver>,
    ) -> Result<MediaInspection, MediaJobRuntimeError> {
        self.ensure_not_cancelled(job).await?;
        let inspector = Arc::clone(&self.inspector);
        let signal = Arc::new(CancellationSignal::default());
        let inspection_signal = Arc::clone(&signal);
        let monitor_store = self.store.clone();
        let media_job_public_id = job.media_job_public_id;
        let observed_cancel_generation = job.cancel_generation;
        let (stop_tx, stop_rx) = watch::channel(false);
        let monitor = tokio::spawn(monitor_job_control(
            monitor_store,
            media_job_public_id,
            job.claim_generation,
            observed_cancel_generation,
            Arc::clone(&signal),
            stop_rx,
            shutdown.cloned(),
        ));
        let source_path = PathBuf::from(source_path);
        let inspection = tokio::task::spawn_blocking(move || {
            inspector.inspect_with_cancellation(&source_path, inspection_signal.as_ref())
        })
        .await;
        drop(stop_tx);
        let monitor = monitor
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        let inspection =
            inspection.map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
        if signal.interrupted.load(Ordering::Acquire) && monitor.is_ok() {
            match &inspection {
                Ok(_) => return Err(MediaJobRuntimeError::Interrupted),
                Err(InspectError::Cancelled { secondary_evidence })
                    if secondary_evidence.is_empty() =>
                {
                    return Err(MediaJobRuntimeError::Interrupted);
                }
                Err(_) => {}
            }
        }
        resolve_monitored_inspection_outcome(inspection, monitor)
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
        job: &ClaimedMediaJobRow,
        report: &JobPreflightReport,
        planning_outcome: Option<&PlanningOutcome>,
    ) -> Result<(), MediaJobRuntimeError> {
        for (index, operation) in report.planned.operations.iter().enumerate() {
            let evidence = operation_audit_evidence(report, index)?;
            let args = [
                evidence.fields[0].as_deref(),
                evidence.fields[1].as_deref(),
                evidence.fields[2].as_deref(),
                evidence.fields[3].as_deref(),
                evidence.fields[4].as_deref(),
            ];
            let operation_kind = operation_kind_code(operation.kind);
            self.store
                .append_job_operation(
                    job.claim_generation,
                    &AppendMediaJobOperationInput {
                        media_job_public_id: job.media_job_public_id,
                        operation_index: usize_to_i32(index, "operation_index")?,
                        operation_kind,
                        stream_id: stream_id_to_i32(operation)?,
                        command_bin: &evidence.command_bin,
                        args,
                    },
                )
                .await?;
            self.telemetry
                .inc_media_job_operation(operation_kind, "planned");
        }

        if let Some(outcome) = planning_outcome {
            self.persist_planning_outcome(job, outcome).await?;
        } else {
            for (index, explanation) in report.summary.explanations.iter().enumerate() {
                self.store
                    .append_job_plan_reason(
                        job.claim_generation,
                        &AppendMediaJobPlanReasonInput {
                            media_job_public_id: job.media_job_public_id,
                            reason_index: usize_to_i32(index, "reason_index")?,
                            candidate_index: Some(0),
                            selected: true,
                            reason_code: "selected_operation",
                            reason_text: &explanation.message,
                        },
                    )
                    .await?;
            }
        }
        Ok(())
    }

    async fn persist_planning_outcome(
        &self,
        job: &ClaimedMediaJobRow,
        outcome: &PlanningOutcome,
    ) -> Result<(), MediaJobRuntimeError> {
        let selected = &outcome.explanation.selected_plan;
        let selected_reason = format!("id={};total_cost={}", selected.id, selected.total_cost);
        self.store
            .append_job_plan_reason(
                job.claim_generation,
                &AppendMediaJobPlanReasonInput {
                    media_job_public_id: job.media_job_public_id,
                    reason_index: 0,
                    candidate_index: Some(0),
                    selected: true,
                    reason_code: "selected_least_cost_candidate",
                    reason_text: &selected_reason,
                },
            )
            .await?;
        let mut reason_index = 1_usize;
        for operation in &selected.operations {
            let reason_text = format!(
                "kind={};source_stream_id={};output_stream_id={};reason={}",
                operation_kind_code(operation.kind),
                optional_stream_id_code(operation.source_stream_id),
                optional_stream_id_code(operation.output_stream_id),
                operation.reason
            );
            self.store
                .append_job_plan_reason(
                    job.claim_generation,
                    &AppendMediaJobPlanReasonInput {
                        media_job_public_id: job.media_job_public_id,
                        reason_index: usize_to_i32(reason_index, "reason_index")?,
                        candidate_index: Some(0),
                        selected: true,
                        reason_code: "selected_operation",
                        reason_text: &reason_text,
                    },
                )
                .await?;
            reason_index = reason_index.saturating_add(1);
        }
        for (candidate_index, rejected) in outcome.selection.rejected.iter().enumerate() {
            let candidate_index = candidate_index.saturating_add(1);
            let rejected_reason = format!(
                "id={};total_cost={}",
                rejected.candidate.id, rejected.total_cost
            );
            self.store
                .append_job_plan_reason(
                    job.claim_generation,
                    &AppendMediaJobPlanReasonInput {
                        media_job_public_id: job.media_job_public_id,
                        reason_index: usize_to_i32(reason_index, "reason_index")?,
                        candidate_index: Some(usize_to_i32(candidate_index, "candidate_index")?),
                        selected: false,
                        reason_code: candidate_rejection_reason_code(rejected.reason),
                        reason_text: &rejected_reason,
                    },
                )
                .await?;
            reason_index = reason_index.saturating_add(1);
            for operation in &rejected.candidate.operations {
                let rationale = operation_rationale_code(operation);
                self.store
                    .append_job_plan_reason(
                        job.claim_generation,
                        &AppendMediaJobPlanReasonInput {
                            media_job_public_id: job.media_job_public_id,
                            reason_index: usize_to_i32(reason_index, "reason_index")?,
                            candidate_index: Some(usize_to_i32(
                                candidate_index,
                                "candidate_index",
                            )?),
                            selected: false,
                            reason_code: "rejected_operation",
                            reason_text: &rationale,
                        },
                    )
                    .await?;
                reason_index = reason_index.saturating_add(1);
            }
        }
        Ok(())
    }

    async fn persist_preflight_audits(
        &self,
        job: &ClaimedMediaJobRow,
        evaluation: &JobPreflightEvaluation,
    ) -> Result<(), MediaJobRuntimeError> {
        let observations = self
            .store
            .list_job_compact_audits(job.media_job_public_id)
            .await?;
        if observations.len() >= 1025 {
            return Err(MediaJobRuntimeError::IndexTooLarge(
                "media_job_preflight_audit_limit",
            ));
        }
        let next_index = observations
            .iter()
            .filter(|row| row.is_current && row.attempt_number == job.attempt_number)
            .map(|row| row.audit_index)
            .max()
            .map_or(Ok(0), |index| {
                index
                    .checked_add(1)
                    .ok_or(MediaJobRuntimeError::IndexTooLarge(
                        "media_job_preflight_audit_index_overflow",
                    ))
            })?;
        for fact in preflight_compact_audit_facts(evaluation) {
            let audit_index = next_index.checked_add(fact.audit_index).ok_or(
                MediaJobRuntimeError::IndexTooLarge("media_job_preflight_audit_index_overflow"),
            )?;
            self.store
                .append_job_compact_audit(
                    job.claim_generation,
                    &AppendMediaJobCompactAuditInput {
                        media_job_public_id: job.media_job_public_id,
                        audit_index,
                        fact_kind: fact.fact_kind,
                        fact_text: &fact.fact_text,
                    },
                )
                .await?;
        }
        Ok(())
    }

    async fn append_phase(
        &self,
        job: &ClaimedMediaJobRow,
        phase_index: i32,
        phase_name: &str,
        phase_status: &str,
        details_text: Option<&str>,
    ) -> Result<(), DataError> {
        let result = self
            .store
            .append_job_phase(
                job.media_job_public_id,
                job.claim_generation,
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
        job: &ClaimedMediaJobRow,
        input: &AppendMediaJobVerificationCheckInput<'_>,
    ) -> Result<(), DataError> {
        let result = self
            .store
            .append_job_verification_check(job.claim_generation, input)
            .await;
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
            .poll_job_control(
                job.media_job_public_id,
                job.claim_generation,
                job.cancel_generation,
            )
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
    ) -> Result<bool, MediaJobRuntimeError> {
        self.store
            .complete_job(
                job.media_job_public_id,
                job.claim_generation,
                job.cancel_generation,
            )
            .await
            .map_err(MediaJobRuntimeError::from)
    }

    async fn persist_cancellation(
        &self,
        job: &ClaimedMediaJobRow,
    ) -> Result<(), MediaJobRuntimeError> {
        self.append_phase(
            job,
            CANCELLATION_PHASE_INDEX,
            "runtime_cancellation",
            "cancelled",
            Some("media_job_cancelled_by_operator"),
        )
        .await?;
        self.append_verification_check(
            job,
            &AppendMediaJobVerificationCheckInput {
                media_job_public_id: job.media_job_public_id,
                check_index: CANCELLATION_CHECK_INDEX,
                check_kind: "cancellation",
                check_status: "skipped",
                expected_value: Some("continue"),
                actual_value: Some("operator_cancelled"),
                details_text: Some("worker acknowledged durable cancellation generation"),
            },
        )
        .await?;
        let acknowledged_generation = self
            .store
            .acknowledge_job_cancel(
                job.media_job_public_id,
                job.claim_generation,
                job.cancel_generation,
            )
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
                job,
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
            .append_verification_check(
                job,
                &AppendMediaJobVerificationCheckInput {
                    media_job_public_id: job.media_job_public_id,
                    check_index: FAILURE_CHECK_INDEX,
                    check_kind: "runtime_failure",
                    check_status: "failed",
                    expected_value: Some("success"),
                    actual_value: Some(code),
                    details_text: Some(code),
                },
            )
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
            .mark_job_status(
                job.media_job_public_id,
                job.claim_generation,
                "failed",
                Some(detail),
            )
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

fn validate_claimed_source_fingerprint(
    job: &ClaimedMediaJobRow,
    observed: Option<MediaAggregateFingerprint>,
) -> Result<MediaAggregateFingerprint, MediaJobRuntimeError> {
    let observed = observed.ok_or(MediaJobRuntimeError::SourceFingerprint(
        "media_job_source_fingerprint_unavailable",
    ))?;
    if observed.identity == job.source_identity
        && observed.size_bytes == job.source_size_bytes
        && observed.modified_ns == job.source_modified_ns
        && observed.changed_ns == job.source_changed_ns
        && observed.sha256 == job.source_sha256
    {
        Ok(observed)
    } else {
        Err(MediaJobRuntimeError::SourceFingerprint(
            "media_job_source_fingerprint_mismatch",
        ))
    }
}

async fn monitor_job_control(
    store: MediaStore,
    media_job_public_id: Uuid,
    claim_generation: i64,
    observed_cancel_generation: i64,
    signal: Arc<CancellationSignal>,
    mut stop: watch::Receiver<bool>,
    shutdown: Option<RuntimeShutdownReceiver>,
) -> Result<bool, DataError> {
    let mut ticker = interval(CONTROL_POLL_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut shutdown = shutdown;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let control = store
                    .poll_job_control(
                        media_job_public_id,
                        claim_generation,
                        observed_cancel_generation,
                    )
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
            () = monitor_runtime_shutdown(&mut shutdown), if shutdown.is_some() => {
                signal.interrupt();
                return Ok(true);
            }
        }
    }
}

async fn monitor_runtime_shutdown(shutdown: &mut Option<RuntimeShutdownReceiver>) {
    if let Some(receiver) = shutdown.as_mut() {
        runtime_shutdown::changed(receiver).await;
    }
}

fn resolve_inspection_outcome(
    inspection: Result<MediaInspection, InspectError>,
    cancellation_requested: bool,
) -> Result<MediaInspection, MediaJobRuntimeError> {
    match inspection {
        Err(InspectError::Cancelled { secondary_evidence }) if secondary_evidence.is_empty() => {
            Err(MediaJobRuntimeError::Cancelled)
        }
        Err(error) => Err(MediaJobRuntimeError::Inspect(error.to_string())),
        Ok(_) if cancellation_requested => Err(MediaJobRuntimeError::Inspect(
            "media inspection completed after cancellation was requested".to_string(),
        )),
        Ok(inspection) => Ok(inspection),
    }
}

fn resolve_monitored_inspection_outcome(
    inspection: Result<MediaInspection, InspectError>,
    monitor: Result<bool, DataError>,
) -> Result<MediaInspection, MediaJobRuntimeError> {
    match monitor {
        Ok(cancellation_requested) => {
            resolve_inspection_outcome(inspection, cancellation_requested)
        }
        Err(control) => match inspection {
            Ok(_) => Err(MediaJobRuntimeError::Data(control)),
            Err(inspection) => Err(MediaJobRuntimeError::InspectionControl {
                control,
                inspection: Box::new(inspection),
            }),
        },
    }
}

#[derive(Debug, Error)]
enum MediaJobRuntimeError {
    #[error("media job runtime data error: {0}")]
    Data(#[from] DataError),
    #[error("media job runtime cancelled by operator")]
    Cancelled,
    #[error("media job runtime interrupted by shutdown")]
    Interrupted,
    #[error("media job deferred until scratch capacity is available")]
    ScratchCapacityDeferred,
    #[error("media job runtime join error: {0}")]
    Join(String),
    #[error("media job runtime inspect error: {0}")]
    Inspect(String),
    #[error("media inspection control failed: {control}; inspection also failed: {inspection}")]
    InspectionControl {
        #[source]
        control: DataError,
        inspection: Box<InspectError>,
    },
    #[error("media job runtime source metadata failed for {path}: {source}")]
    SourceMetadata {
        path: String,
        source: std::io::Error,
    },
    #[error("media job runtime source artifact byte count overflowed")]
    SourceSizeOverflow,
    #[error("media job runtime source fingerprint error: {0}")]
    Fingerprint(#[from] FingerprintError),
    #[error("media job runtime source fingerprint validation failed: {0}")]
    SourceFingerprint(&'static str),
    #[error("media job runtime execute error: {0}")]
    Execute(ExecuteSequenceError),
    #[error("media job runtime execute step error: {0}")]
    ExecuteStep(ExecuteStepError),
    #[error("media job runtime replacement error: {0}")]
    Replacement(#[from] ReplacementError),
    #[error("media job runtime replacement rollback error: {0}")]
    ReplacementRollback(ReplacementError),
    #[error("media job runtime workspace error: {0}")]
    Workspace(#[from] ManagedWorkspaceError),
    #[error("media job runtime checkpoint I/O failed: {0}")]
    CheckpointIo(std::io::Error),
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
    #[error("media job runtime terminal outbox event kind is invalid: {0}")]
    InvalidTerminalEventKind(String),
    #[error("media job runtime terminal event publication failed: {0}")]
    EventPublish(String),
}

impl MediaJobRuntimeError {
    const fn code(&self) -> &'static str {
        match self {
            Self::Data(_) | Self::InspectionControl { .. } => "media_job_runtime_storage_failed",
            Self::Cancelled => "media_job_cancelled_by_operator",
            Self::Interrupted => "media_job_shutdown_interrupted",
            Self::ScratchCapacityDeferred => "media_job_scratch_capacity_deferred",
            Self::Join(_) => "media_job_runtime_join_failed",
            Self::Inspect(_) => "media_job_runtime_inspect_failed",
            Self::SourceMetadata { .. } => "media_job_runtime_source_metadata_failed",
            Self::SourceSizeOverflow => "media_job_runtime_source_size_overflow",
            Self::Fingerprint(_) => "media_job_runtime_source_fingerprint_failed",
            Self::Execute(_) | Self::ExecuteStep(_) => "media_job_runtime_execute_failed",
            Self::Replacement(_) => "media_job_runtime_replacement_failed",
            Self::ReplacementRollback(_) => "media_job_runtime_replacement_rollback_failed",
            Self::Workspace(_) => "media_job_runtime_workspace_failed",
            Self::CheckpointIo(_) => "media_job_runtime_checkpoint_io_failed",
            Self::Capacity(_) => "media_job_runtime_capacity_probe_failed",
            Self::SourceFingerprint(code)
            | Self::InvalidPath(code)
            | Self::InvalidDesiredGraph(code)
            | Self::Verification(code)
            | Self::InvalidCapability(code)
            | Self::IndexTooLarge(code) => code,
            Self::InvalidRecoveryJobKey(_) => "media_job_runtime_recovery_job_key_invalid",
            Self::InvalidTerminalEventKind(_) => "media_job_runtime_terminal_event_invalid",
            Self::EventPublish(_) => "media_job_runtime_terminal_event_publish_failed",
        }
    }

    const fn category(&self) -> &'static str {
        match self {
            Self::Data(_) | Self::InspectionControl { .. } => "storage",
            Self::Cancelled => "cancellation",
            Self::Interrupted => "interruption",
            Self::ScratchCapacityDeferred => "capacity",
            Self::Join(_) => "join",
            Self::Inspect(_)
            | Self::SourceMetadata { .. }
            | Self::SourceSizeOverflow
            | Self::Fingerprint(_) => "inspection",
            Self::Execute(_) | Self::ExecuteStep(_) => "execution",
            Self::Replacement(_)
            | Self::ReplacementRollback(_)
            | Self::InvalidRecoveryJobKey(_)
            | Self::InvalidTerminalEventKind(_) => "replacement",
            Self::EventPublish(_) => "event",
            Self::Workspace(_) | Self::CheckpointIo(_) => "workspace",
            Self::Capacity(_) => "disk_reserve",
            Self::InvalidPath(_) | Self::InvalidDesiredGraph(_) | Self::IndexTooLarge(_) => {
                "planning"
            }
            Self::Verification(_) | Self::SourceFingerprint(_) => "verification",
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

fn replacement_job_key(media_job_public_id: Uuid, claim_generation: i64) -> String {
    format!("{media_job_public_id}-g{claim_generation}")
}

fn parse_replacement_job_key(job_key: &str) -> Result<(Uuid, i64), MediaJobRuntimeError> {
    let Some((media_job_public_id, claim_generation)) = job_key.rsplit_once("-g") else {
        return Err(MediaJobRuntimeError::InvalidRecoveryJobKey(
            job_key.to_string(),
        ));
    };
    let media_job_public_id = Uuid::parse_str(media_job_public_id)
        .map_err(|_| MediaJobRuntimeError::InvalidRecoveryJobKey(job_key.to_string()))?;
    let claim_generation = claim_generation
        .parse::<i64>()
        .map_err(|_| MediaJobRuntimeError::InvalidRecoveryJobKey(job_key.to_string()))?;
    if claim_generation <= 0 {
        return Err(MediaJobRuntimeError::InvalidRecoveryJobKey(
            job_key.to_string(),
        ));
    }
    Ok((media_job_public_id, claim_generation))
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
    if streams.is_empty() {
        return Err(MediaJobRuntimeError::InvalidDesiredGraph(
            "media_job_desired_target_snapshot_empty",
        ));
    }
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
    let language = row
        .language_code
        .as_deref()
        .map(LanguageToken::parse)
        .transpose()
        .map_err(|_| {
            MediaJobRuntimeError::InvalidDesiredGraph("media_job_desired_target_language_invalid")
        })?;
    Ok(TargetStream {
        stream_key: row.stream_key,
        kind,
        role,
        language,
        source_binding_key: None,
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

fn identity_stream_bindings(streams: &[MediaStream]) -> Vec<DesiredStreamBinding> {
    streams
        .iter()
        .map(|stream| DesiredStreamBinding {
            output_stream_id: stream.stream_id,
            source_stream_id: Some(stream.stream_id),
        })
        .collect()
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
            max_bitrate_bps: max_bitrate_constraint(target_stream.video_bitrate_bps)?,
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
            max_bitrate_bps: max_bitrate_constraint(target_stream.video_bitrate_bps)
                .map_err(|_| video_target_constraint_unmatched(target_stream))?,
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
    if let Some(expected) = constraint.max_bitrate_bps
        && !stream
            .bit_rate
            .is_some_and(|average| bitrate_at_or_below_max(expected, average, stream.max_bit_rate))
    {
        let expected_text = format!("max:{}", expected.get());
        let actual_text = stream.bit_rate.map_or_else(
            || "missing".to_string(),
            |average| {
                stream.max_bit_rate.map_or_else(
                    || format!("average:{average},peak:missing"),
                    |peak| format!("average:{average},peak:{peak}"),
                )
            },
        );
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

fn max_bitrate_constraint(
    value: Option<u32>,
) -> Result<Option<MaxBitrateBps>, MediaJobRuntimeError> {
    value
        .map(|value| {
            MaxBitrateBps::new(value).ok_or(MediaJobRuntimeError::InvalidDesiredGraph(
                "media_job_desired_target_video_bitrate_invalid",
            ))
        })
        .transpose()
}

fn target_uses_embedded_outputs_only(target: &DesiredTarget) -> bool {
    target.streams.iter().all(|stream| {
        !matches!(
            stream.subtitle_placement,
            Some(SubtitlePlacement::Sidecar | SubtitlePlacement::Both)
        )
    })
}

fn bitrate_at_or_below_max(maximum: MaxBitrateBps, average: u64, peak: Option<u64>) -> bool {
    let maximum = u64::from(maximum.get());
    average <= maximum && peak.is_none_or(|peak| peak <= maximum)
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

fn compile_runtime_preflight(
    input: RuntimePreflightBuildInput,
) -> Result<RuntimePreflightEvaluation, MediaJobRuntimeError> {
    let expected_container_metadata = input.inspection.container.metadata.clone();
    let expected_chapters = input.inspection.chapters.clone();
    let source_file_bytes = source_artifact_bytes(&input.source_path, &input.inspection.sidecars)?;
    let source_inspection = input.inspection;
    let source_graph = &source_inspection.graph;
    let planning_outcome = input
        .desired_target
        .as_ref()
        .filter(|snapshot| target_uses_embedded_outputs_only(&snapshot.target))
        .map(|snapshot| {
            compile_and_plan_with_policy(
                source_graph,
                &input.output_path,
                &snapshot.target,
                snapshot.unmatched_stream_policy,
                &PlanningConstraints::all_supported(),
                &input.operation_costs,
            )
            .map_err(|_| {
                MediaJobRuntimeError::InvalidDesiredGraph(
                    "media_job_production_planning_pipeline_failed",
                )
            })
        })
        .transpose()?;
    let compiled = match input.desired_target.as_ref() {
        Some(snapshot) => compile_desired_target_with_sidecars_at(
            source_graph,
            &input.output_path,
            &input.source_path,
            &snapshot.target,
            snapshot.unmatched_stream_policy,
            &sidecar_inputs(&source_inspection.sidecars)?,
        )
        .map_err(|_| {
            MediaJobRuntimeError::InvalidDesiredGraph("media_job_desired_target_compile_failed")
        })?,
        None => CompiledDesiredTarget {
            graph: DesiredGraph {
                output_path: input.output_path.clone(),
                container_format: None,
                stream_bindings: identity_stream_bindings(&source_graph.streams),
                streams: source_graph.streams.clone(),
            },
            sidecar_embeddings: Vec::new(),
            sidecar_outputs: Vec::new(),
            sidecar_removals: Vec::new(),
        },
    };
    let video_policy = video_policy_from_target_snapshot(
        input.base_video_policy,
        input.desired_target.as_ref(),
        &compiled.graph,
    )?;
    let free_bytes = input
        .capacity_probe
        .available_bytes(&input.workspace_root_path)
        .map_err(MediaJobRuntimeError::Capacity)?;
    let preflight_input = build_preflight_input(
        PreflightBuildTemplate {
            source_path: &input.source_path,
            output_path: &input.output_path,
            desired: &compiled.graph,
            source_file_bytes,
            capabilities: &input.capabilities,
            workspace_policy: &input.workspace_policy,
            free_bytes,
        },
        PreflightPolicyInput {
            backup_root: None,
            quarantine_root: Some(&input.diagnostics_root),
            video_policy,
        },
    );
    let evaluation = planning_outcome.as_ref().map_or_else(
        || {
            evaluate_preflight_from_compiled_target(
                &source_inspection,
                &compiled,
                preflight_input.as_borrowed(),
            )
        },
        |outcome| {
            evaluate_preflight_from_planning_outcome(
                source_graph,
                &compiled,
                outcome,
                preflight_input.as_borrowed(),
            )
        },
    );
    policy::ensure_preflight_allowed(&input.operation_costs, &evaluation)?;
    Ok(RuntimePreflightEvaluation {
        evaluation,
        desired: compiled.graph,
        desired_target: input.desired_target,
        expected_container_metadata,
        expected_chapters,
        planning_outcome,
    })
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
            .is_some_and(|actual| audio_bitrate_within_tolerance(u64::from(expected), actual))
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

fn audio_bitrate_within_tolerance(expected: u64, actual: u64) -> bool {
    const BITRATE_TOLERANCE_PERCENT: u128 = 5;
    let expected = u128::from(expected);
    let actual = u128::from(actual);
    let delta = expected.abs_diff(actual);
    delta.saturating_mul(100) <= expected.saturating_mul(BITRATE_TOLERANCE_PERCENT)
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

fn container_metadata_check_kind(graph_check_kind: &'static str) -> &'static str {
    match graph_check_kind {
        "source_graph" => "source_container_metadata",
        "candidate_graph" => "candidate_container_metadata",
        "final_graph" => "final_container_metadata",
        _ => "container_metadata",
    }
}

fn container_metadata_check_index(graph_check_kind: &'static str, fallback: i32) -> i32 {
    match graph_check_kind {
        "source_graph" => 5,
        "candidate_graph" => 19,
        "final_graph" => 26,
        _ => fallback,
    }
}

const fn target_stream_has_audio_constraints(stream: &TargetStream) -> bool {
    stream.audio_bitrate_bps.is_some()
        || stream.audio_sample_rate_hz.is_some()
        || stream.audio_loudness_profile.is_some()
        || stream.audio_dynamic_range.is_some()
}

struct InspectionVerification {
    matched: bool,
    expected: String,
    actual: String,
    details: Option<String>,
}

fn container_metadata_matches_inspection(
    inspection: &MediaInspection,
    expected_metadata: &[MetadataEntry],
) -> InspectionVerification {
    if expected_metadata.is_empty() {
        return InspectionVerification {
            matched: true,
            expected: "source_container_metadata".to_string(),
            actual: "not_present".to_string(),
            details: None,
        };
    }
    let expected = normalized_metadata_entries(expected_metadata);
    let actual = normalized_metadata_entries(&inspection.container.metadata);
    let matched = expected
        .iter()
        .all(|entry| actual.iter().any(|actual_entry| actual_entry == entry));
    InspectionVerification {
        matched,
        expected: format!("{} source_container_metadata_entries", expected.len()),
        actual: if matched {
            "matched".to_string()
        } else {
            format!("{} output_container_metadata_entries", actual.len())
        },
        details: (!matched).then(|| {
            format!(
                "container metadata mismatch for {}; expected {} source entries but found {} output entries",
                inspection.graph.source_path,
                expected.len(),
                actual.len()
            )
        }),
    }
}

fn chapter_timeline_matches_inspection(
    inspection: &MediaInspection,
    expected_chapters: &[ChapterInspection],
) -> InspectionVerification {
    if expected_chapters.is_empty() {
        return InspectionVerification {
            matched: true,
            expected: "source_chapters".to_string(),
            actual: "not_present".to_string(),
            details: None,
        };
    }
    let expected = normalized_chapter_timeline(expected_chapters);
    let actual = normalized_chapter_timeline(&inspection.chapters);
    let matched = expected == actual;
    InspectionVerification {
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
    workspace: &WorkspacePaths,
) -> Result<String, MediaJobRuntimeError> {
    let Some(file_name) = Path::new(final_output_path)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
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
    value.and_then(|item| normalize_audio_channel_layout(item).map(str::to_string))
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

const fn candidate_rejection_reason_code(reason: CandidateRejectionReason) -> &'static str {
    match reason {
        CandidateRejectionReason::InvalidOperationShape => "invalid_operation_shape",
        CandidateRejectionReason::DominatedByLowerCost => "dominated_by_lower_cost",
        CandidateRejectionReason::DeterministicTieBreak => "deterministic_tie_break",
        CandidateRejectionReason::UnsupportedOperation => "unsupported_operation",
        CandidateRejectionReason::UnsafeOrUnverifiable => "unsafe_or_unverifiable",
    }
}

fn optional_stream_id_code(stream_id: Option<u32>) -> String {
    stream_id.map_or_else(|| "none".to_string(), |value| value.to_string())
}

fn operation_rationale_code(operation: &PlannedOperation) -> String {
    format!(
        "{}:{}:{}",
        operation_kind_code(operation.kind),
        optional_stream_id_code(operation.stream_id),
        optional_stream_id_code(operation.output_stream_id)
    )
}

fn planned_job_is_noop(report: &JobPreflightReport) -> bool {
    matches!(
        report.planned.operations.as_slice(),
        [PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
            output_stream_id: None,
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

struct OperationAuditEvidence {
    command_bin: String,
    fields: [Option<String>; 5],
}

const MAX_AUDIT_FIELD_CHARS: usize = 512;

fn operation_audit_evidence(
    report: &JobPreflightReport,
    operation_index: usize,
) -> Result<OperationAuditEvidence, MediaJobRuntimeError> {
    let operation = report.planned.operations.get(operation_index).ok_or(
        MediaJobRuntimeError::InvalidDesiredGraph("media_job_execution_operation_index_invalid"),
    )?;
    let audit = report
        .step_audits
        .iter()
        .find(|audit| audit.operation_indices.contains(&operation_index));
    let Some(audit) = audit else {
        if operation.kind != OperationKind::NoOp {
            return Err(MediaJobRuntimeError::InvalidDesiredGraph(
                "media_job_execution_operation_mapping_missing",
            ));
        }
        return Ok(OperationAuditEvidence {
            command_bin: "logical".to_string(),
            fields: [
                Some("step_id=logical-noop".to_string()),
                Some(format!("operation_ids={operation_index}")),
                Some(format!(
                    "argv_sha256={}",
                    command_argv_digest("logical-noop", &[])
                )),
                Some("kind=no_op".to_string()),
                Some("argv_count=0".to_string()),
            ],
        });
    };
    let step =
        report
            .steps
            .get(audit.step_index)
            .ok_or(MediaJobRuntimeError::InvalidDesiredGraph(
                "media_job_execution_step_mapping_invalid",
            ))?;
    let operation_ids = bounded_audit_text(
        &audit
            .operation_indices
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(","),
    );
    let (command_bin, digest, critical_flags, argument_count) = match step {
        ExecutionStep::Command { bin, argv } => (
            redacted_command_bin(bin),
            command_argv_digest(bin, argv),
            redacted_critical_flags(argv),
            argv.len(),
        ),
        _ => (
            "filesystem".to_string(),
            filesystem_step_digest(step),
            format!("kind={}", filesystem_step_kind(step)),
            0,
        ),
    };
    Ok(OperationAuditEvidence {
        command_bin,
        fields: [
            Some(format!("step_id={}", audit.step_id)),
            Some(format!("operation_ids={operation_ids}")),
            Some(format!("argv_sha256={digest}")),
            Some(critical_flags),
            Some(format!("argv_count={argument_count}")),
        ],
    })
}

fn redacted_command_bin(bin: &str) -> String {
    Path::new(bin)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("command")
        .to_string()
}

fn command_argv_digest(bin: &str, argv: &[String]) -> String {
    let mut digest = Sha256::new();
    append_digest_field(&mut digest, bin.as_bytes());
    for argument in argv {
        append_digest_field(&mut digest, argument.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn append_digest_field(digest: &mut Sha256, field: &[u8]) {
    digest.update(field.len().to_le_bytes());
    digest.update(field);
}

fn redacted_critical_flags(argv: &[String]) -> String {
    let mut fields = Vec::new();
    let mut index = 0;
    while index < argv.len() && fields.len() < 32 {
        let flag = &argv[index];
        if flag.starts_with('-') {
            let normalized = flag.to_ascii_lowercase();
            if ["pass", "password", "secret", "token", "key"]
                .iter()
                .any(|marker| normalized.contains(marker))
            {
                fields.push(format!("{flag}=[redacted]"));
            } else if flag == "-i" {
                fields.push("-i=[path]".to_string());
            } else if critical_flag_value_is_safe(flag)
                && argv
                    .get(index + 1)
                    .is_some_and(|value| !value.starts_with('-'))
            {
                fields.push(format!("{flag}={}", argv[index + 1]));
            } else {
                fields.push(flag.clone());
            }
        }
        index += 1;
    }
    bounded_audit_text(&format!("flags={}", fields.join(",")))
}

fn critical_flag_value_is_safe(flag: &str) -> bool {
    ["-map", "-f", "-ac", "-ar", "-map_chapters", "-map_metadata"].contains(&flag)
        || [
            "-c:",
            "-profile:",
            "-level:",
            "-b:",
            "-maxrate:",
            "-bufsize:",
            "-disposition:",
            "-color_primaries:",
            "-color_trc:",
            "-colorspace:",
        ]
        .iter()
        .any(|prefix| flag.starts_with(prefix))
}

fn bounded_audit_text(value: &str) -> String {
    const TRUNCATED_MARKER: &str = "[truncated]";
    if value.chars().count() <= MAX_AUDIT_FIELD_CHARS {
        return value.to_string();
    }
    let bounded = value
        .chars()
        .take(MAX_AUDIT_FIELD_CHARS - TRUNCATED_MARKER.len())
        .collect::<String>();
    format!("{bounded}{TRUNCATED_MARKER}")
}

fn filesystem_step_digest(step: &ExecutionStep) -> String {
    let mut digest = Sha256::new();
    append_digest_field(&mut digest, filesystem_step_kind(step).as_bytes());
    format!("{:x}", digest.finalize())
}

const fn filesystem_step_kind(step: &ExecutionStep) -> &'static str {
    match step {
        ExecutionStep::CopySidecarSubtitle { .. } => "copy_sidecar",
        ExecutionStep::BackupSource { .. } => "backup_source",
        ExecutionStep::VerifyOutput { .. } => "verify_output",
        ExecutionStep::QuarantineFailedOutput { .. } => "quarantine_output",
        ExecutionStep::AtomicReplace { .. } => "atomic_replace",
        ExecutionStep::Command { .. } => "command",
    }
}
