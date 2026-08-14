//! Command argument builders.

use crate::capabilities::CapabilitySnapshot;
use revaer_media_core::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};
use revaer_media_core::normalize::{
    normalize_audio_channel_layout, normalize_container_format,
    normalize_container_metadata_policy, normalize_subtitle_codec,
};
use revaer_media_core::plan::{OperationKind, PlannedOperation};
use revaer_media_core::target::{DesiredSidecarOutput, SidecarEmbedding, SidecarOutputSource};
use revaer_media_core::verify::{verify_plan, verify_unique_stream_ids};
use std::fs;
use std::io::{self, Read};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

const MAX_EXECUTION_DETAIL_CHARS: usize = 2_048;
const EXECUTION_TRUNCATION_MARKER: &str = "...[truncated]";
const DEFAULT_PROCESS_COMMAND_TIMEOUT: Duration = Duration::from_hours(12);

/// Build error for command arguments.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BuildArgsError {
    /// Stream id required but absent.
    #[error("stream id required for transcode operation")]
    MissingStreamId,
    /// Required codec is not supported by runtime capabilities.
    #[error("required codec is not supported: {0}")]
    UnsupportedCodec(&'static str),
    /// Desired output muxer is not supported by runtime capabilities.
    #[error("required muxer is not supported: {0}")]
    UnsupportedMuxer(String),
    /// Desired container metadata policy is not supported by runtime command construction.
    #[error("required container metadata policy is not supported: {0}")]
    UnsupportedContainerMetadataPolicy(String),
    /// Arbitrary metadata rewrite is not implemented as a verified desired-state contract.
    #[error("metadata rewrite is not supported until desired metadata verification is implemented")]
    UnsupportedMetadataRewrite,
    /// Stream metadata, disposition, and ordering rewrites require the complete desired graph.
    #[error("stream rewrite requires complete desired graph context")]
    ContextlessStreamRewrite,
    /// No operations were provided for execution planning.
    #[error("at least one operation is required")]
    EmptyOperations,
    /// Operation list violates the planned operation contract.
    #[error("operation list is invalid: {0}")]
    InvalidOperations(&'static str),
    /// No-op operations do not map to command arguments.
    #[error("no-op operation does not require command arguments")]
    NoOpCommand,
    /// Desired graph references a stream that is absent from the inspected source.
    #[error("desired graph references missing source stream: {0}")]
    DesiredStreamMissing(u32),
    /// Desired graph stream identity resolves to a source stream with a different kind.
    #[error("desired graph stream kind does not match source stream: {0}")]
    DesiredStreamKindMismatch(u32),
    /// Desired graph contains a retained inspection stream kind without a mutating contract.
    #[error("desired graph stream kind is not supported for command materialization: {stream_id}")]
    UnsupportedDesiredStreamKind {
        /// Desired stream identity.
        stream_id: u32,
    },
    /// Source graph stream ids are ambiguous.
    #[error("source graph contains duplicate stream ids")]
    DuplicateSourceStreamIds,
    /// Desired graph stream ids are ambiguous.
    #[error("desired graph contains duplicate stream ids")]
    DuplicateDesiredStreamIds,
    /// Artifact-aware operations require the complete desired-target execution builder.
    #[error("subtitle artifact operation requires complete desired-target execution context")]
    SubtitleArtifactContextRequired,
    /// Paired sidecar source and output companion paths are inconsistent.
    #[error("paired sidecar companion paths are inconsistent")]
    SidecarCompanionMismatch,
}

/// Filesystem execution error for non-command execution steps.
#[derive(Debug, Error)]
pub enum ExecuteStepError {
    /// Cooperative cancellation stopped command execution.
    #[error("command execution cancelled")]
    Cancelled,
    /// A live execution resource limit was breached.
    #[error("execution resource limit breached: {0}")]
    LimitBreached(ExecutionLimitBreach),
    /// Command steps require an injected command runner.
    #[error("command steps require an injected command runner")]
    CommandStepUnsupported,
    /// Atomic replacement requires the durable source-filesystem transaction boundary.
    #[error("atomic replacement requires the managed replacement committer")]
    ManagedReplacementRequired,
    /// Verified output path does not exist.
    #[error("verified output is missing: {0}")]
    OutputMissing(PathBuf),
    /// Verified output path is empty.
    #[error("verified output is empty: {0}")]
    OutputEmpty(PathBuf),
    /// Command exited unsuccessfully.
    #[error("command {bin} exited unsuccessfully with status {status_code:?}: {stderr}")]
    CommandFailed {
        /// Binary that exited unsuccessfully.
        bin: String,
        /// Process exit status code when available.
        status_code: Option<i32>,
        /// Bounded stderr detail captured from the failed command.
        stderr: String,
    },
    /// Command exceeded the configured wall-clock execution deadline.
    #[error("command {bin} exceeded execution timeout after {timeout:?}")]
    CommandTimedOut {
        /// Binary that exceeded the deadline.
        bin: String,
        /// Configured wall-clock timeout.
        timeout: Duration,
    },
    /// Filesystem operation failed.
    #[error("filesystem operation {operation} failed for {path}: {source}")]
    Io {
        /// Operation being performed.
        operation: &'static str,
        /// Path being accessed.
        path: PathBuf,
        /// Source I/O error.
        source: io::Error,
    },
}

/// Synchronous cancellation signal observed by process-backed command execution.
pub trait ExecutionControl {
    /// Return whether the active operation must stop.
    fn cancellation_requested(&self) -> bool;

    /// Return a live resource breach that requires immediate child termination.
    fn limit_breach(&self) -> Option<ExecutionLimitBreach> {
        None
    }
}

/// Machine-readable live execution limit breach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ExecutionLimitBreach {
    /// Cumulative logical workspace bytes exceeded the configured maximum.
    #[error("workspace byte budget exceeded")]
    WorkspaceBytesExceeded,
    /// Current free bytes fell below the configured reserve.
    #[error("workspace free-space reserve lost")]
    WorkspaceReserveLost,
    /// The injected live usage probe failed, so execution stopped fail-closed.
    #[error("workspace usage probe failed: {0:?}")]
    WorkspaceProbeFailed(io::ErrorKind),
}

#[derive(Debug, Default, Clone, Copy)]
struct NeverCancel;

impl ExecutionControl for NeverCancel {
    fn cancellation_requested(&self) -> bool {
        false
    }
}

/// Execution sequence failure with optional recovery failure details.
#[derive(Debug, Error)]
#[error("execution step {failed_step_index} failed: {failed}")]
pub struct ExecuteSequenceError {
    /// Index of the step that failed.
    pub failed_step_index: usize,
    /// Primary execution failure.
    pub failed: ExecuteStepError,
    /// Recovery failure when a recovery step was attempted and failed.
    pub recovery: Option<ExecuteStepError>,
}

/// Injected command executor for process-backed execution steps.
pub trait CommandRunner {
    /// Run a binary with prebuilt argv values.
    ///
    /// # Errors
    ///
    /// Returns [`ExecuteStepError`] when command execution fails.
    fn run(&self, bin: &str, argv: &[String]) -> Result<(), ExecuteStepError>;

    /// Run a binary while observing cooperative cancellation.
    ///
    /// The default implementation provides boundary checks for injected runners that complete
    /// synchronously. Process-backed runners override this method to terminate an active child.
    ///
    /// # Errors
    ///
    /// Returns [`ExecuteStepError`] when execution fails or cancellation is requested.
    fn run_controlled(
        &self,
        bin: &str,
        argv: &[String],
        control: &dyn ExecutionControl,
    ) -> Result<(), ExecuteStepError> {
        if let Some(breach) = control.limit_breach() {
            return Err(ExecuteStepError::LimitBreached(breach));
        }
        if control.cancellation_requested() {
            return Err(ExecuteStepError::Cancelled);
        }
        self.run(bin, argv)?;
        if let Some(breach) = control.limit_breach() {
            return Err(ExecuteStepError::LimitBreached(breach));
        }
        if control.cancellation_requested() {
            return Err(ExecuteStepError::Cancelled);
        }
        Ok(())
    }
}

/// Process-backed command runner for execution workers.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessCommandRunner;

impl CommandRunner for ProcessCommandRunner {
    fn run(&self, bin: &str, argv: &[String]) -> Result<(), ExecuteStepError> {
        self.run_controlled(bin, argv, &NeverCancel)
    }

    fn run_controlled(
        &self,
        bin: &str,
        argv: &[String],
        control: &dyn ExecutionControl,
    ) -> Result<(), ExecuteStepError> {
        self.run_controlled_with_timeout(bin, argv, control, DEFAULT_PROCESS_COMMAND_TIMEOUT)
    }
}

impl ProcessCommandRunner {
    /// Run a binary while observing cooperative cancellation and a wall-clock timeout.
    ///
    /// # Errors
    ///
    /// Returns [`ExecuteStepError`] when execution fails, cancellation is requested, or the
    /// command exceeds `timeout`.
    pub fn run_controlled_with_timeout(
        &self,
        bin: &str,
        argv: &[String],
        control: &dyn ExecutionControl,
        timeout: Duration,
    ) -> Result<(), ExecuteStepError> {
        if control.cancellation_requested() {
            return Err(ExecuteStepError::Cancelled);
        }
        let mut child = Command::new(bin)
            .args(argv)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| ExecuteStepError::Io {
                operation: "execution.command_spawn",
                path: PathBuf::from(bin),
                source,
            })?;
        let Some(stderr) = child.stderr.take() else {
            terminate_command(&mut child, bin)?;
            return Err(ExecuteStepError::Io {
                operation: "execution.command_stderr_pipe",
                path: PathBuf::from(bin),
                source: io::Error::other("stderr pipe unavailable"),
            });
        };
        let stderr_reader = thread::spawn(move || read_bounded_stderr(stderr));
        let started_at = Instant::now();

        loop {
            if let Some(breach) = control.limit_breach() {
                terminate_command(&mut child, bin)?;
                let _stderr = join_stderr_reader(stderr_reader, bin)?;
                return Err(ExecuteStepError::LimitBreached(breach));
            }
            if control.cancellation_requested() {
                terminate_command(&mut child, bin)?;
                let _stderr = join_stderr_reader(stderr_reader, bin)?;
                return Err(ExecuteStepError::Cancelled);
            }
            if started_at.elapsed() >= timeout {
                terminate_command(&mut child, bin)?;
                let _stderr = join_stderr_reader(stderr_reader, bin)?;
                return Err(ExecuteStepError::CommandTimedOut {
                    bin: bin.to_string(),
                    timeout,
                });
            }
            if let Some(status) = child.try_wait().map_err(|source| ExecuteStepError::Io {
                operation: "execution.command_wait",
                path: PathBuf::from(bin),
                source,
            })? {
                let stderr = join_stderr_reader(stderr_reader, bin)?;
                return if status.success() {
                    Ok(())
                } else {
                    Err(ExecuteStepError::CommandFailed {
                        bin: bin.to_string(),
                        status_code: status.code(),
                        stderr,
                    })
                };
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn terminate_command(child: &mut Child, bin: &str) -> Result<(), ExecuteStepError> {
    match child.kill() {
        Ok(()) => {}
        Err(source) if source.kind() == io::ErrorKind::InvalidInput => {}
        Err(source) => {
            return Err(ExecuteStepError::Io {
                operation: "execution.command_kill",
                path: PathBuf::from(bin),
                source,
            });
        }
    }
    child.wait().map_err(|source| ExecuteStepError::Io {
        operation: "execution.command_reap",
        path: PathBuf::from(bin),
        source,
    })?;
    Ok(())
}

fn join_stderr_reader(
    stderr_reader: thread::JoinHandle<io::Result<String>>,
    bin: &str,
) -> Result<String, ExecuteStepError> {
    stderr_reader
        .join()
        .map_err(|_| ExecuteStepError::Io {
            operation: "execution.command_stderr_join",
            path: PathBuf::from(bin),
            source: io::Error::other("stderr reader thread panicked"),
        })?
        .map_err(|source| ExecuteStepError::Io {
            operation: "execution.command_stderr_read",
            path: PathBuf::from(bin),
            source,
        })
}

fn read_bounded_stderr(mut stderr: ChildStderr) -> io::Result<String> {
    let mut bytes = Vec::with_capacity(MAX_EXECUTION_DETAIL_CHARS);
    let mut scratch = [0_u8; 1024];
    let mut truncated = false;
    loop {
        let read = stderr.read(&mut scratch)?;
        if read == 0 {
            break;
        }
        let remaining = MAX_EXECUTION_DETAIL_CHARS.saturating_sub(bytes.len());
        if remaining == 0 {
            truncated = true;
            continue;
        }
        let retained = read.min(remaining);
        bytes.extend_from_slice(&scratch[..retained]);
        if retained < read {
            truncated = true;
        }
    }
    let mut detail = String::from_utf8_lossy(&bytes).trim().to_string();
    if detail.chars().count() > MAX_EXECUTION_DETAIL_CHARS {
        detail = detail.chars().take(MAX_EXECUTION_DETAIL_CHARS).collect();
        truncated = true;
    }
    if truncated {
        detail.push_str(EXECUTION_TRUNCATION_MARKER);
    }
    Ok(detail)
}
const DEFAULT_VIDEO_ENCODER: &str = "libx265";
const VIDEO_AVERAGE_TARGET_PERCENT: u64 = 95;
const VIDEO_VBV_SECONDS: u64 = 2;
const VIDEO_ENCODER_FALLBACKS: &[&str] = &[
    "hevc_nvenc",
    "hevc_qsv",
    "hevc_vaapi",
    "libx265",
    "hevc",
    "h265",
];
const SOFTWARE_VIDEO_ENCODER_FALLBACKS: &[&str] = &["libx265", "hevc", "h265"];
const VIDEO_TRANSCODE_PRESET: &str = "medium";
const VIDEO_TRANSCODE_CRF: &str = "22";

/// Video transcode intent family for encoder safety decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoTranscodeIntent {
    /// General media transcode behavior.
    General,
    /// Anime-focused safeguards that avoid naive hardware encodes.
    Anime,
    /// Audiobook intent; video handling remains conservative.
    Audiobook,
    /// Archival intent; prefer conservative preservation behavior.
    Archival,
}

/// HDR and color handling policy for video transcode argv construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdrColorPolicy {
    /// Do not add explicit color conversion flags.
    PreserveSource,
    /// Preserve HDR10 signaling in the generated output.
    PreserveHdr10,
    /// Convert HDR signaling to SDR BT.709 output.
    ToneMapToSdr,
}

/// Per-stream video constraints selected by an immutable desired target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxBitrateBps(NonZeroU32);

impl MaxBitrateBps {
    /// Construct a positive maximum bitrate.
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        match NonZeroU32::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Return the maximum bitrate in bits per second.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

/// Per-stream video constraints selected by an immutable desired target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoStreamConstraints {
    /// Desired stream id after target compilation.
    pub stream_id: u32,
    /// Desired encoder profile, such as `main` or `main10`.
    pub profile: Option<String>,
    /// Desired encoder level in the toolchain's accepted representation.
    pub level: Option<String>,
    /// Maximum permitted bitrate in bits per second.
    pub max_bitrate_bps: Option<MaxBitrateBps>,
    /// Desired color primaries.
    pub color_primaries: Option<String>,
    /// Desired transfer characteristic.
    pub color_transfer: Option<String>,
    /// Desired color space.
    pub color_space: Option<String>,
    /// Desired HDR format label.
    pub hdr_format: Option<String>,
}

/// Per-stream audio constraints selected by an immutable desired target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStreamConstraints {
    /// Desired stream id after target compilation.
    pub stream_id: u32,
    /// Desired average bitrate in bits per second.
    pub bitrate_bps: Option<u32>,
    /// Desired sample rate in hertz.
    pub sample_rate_hz: Option<u32>,
    /// Desired audio loudness processing profile.
    pub loudness_profile: Option<String>,
    /// Desired audio dynamic-range behavior.
    pub dynamic_range: Option<String>,
}

/// Video transcode policy applied to execution argv construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoTranscodePolicy {
    /// Intent-specific safeguards.
    pub intent: VideoTranscodeIntent,
    /// HDR/color policy.
    pub hdr_color: HdrColorPolicy,
    /// Per-stream constraints carried from immutable desired-target snapshots.
    pub stream_constraints: Vec<VideoStreamConstraints>,
    /// Per-audio-stream constraints carried from immutable desired-target snapshots.
    pub audio_stream_constraints: Vec<AudioStreamConstraints>,
}

impl Default for VideoTranscodePolicy {
    fn default() -> Self {
        Self {
            intent: VideoTranscodeIntent::General,
            hdr_color: HdrColorPolicy::PreserveSource,
            stream_constraints: Vec::new(),
            audio_stream_constraints: Vec::new(),
        }
    }
}

/// Deterministic execution step for runtime orchestration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStep {
    /// Copy an existing sidecar subtitle into a managed output location.
    CopySidecarSubtitle {
        /// Existing sidecar subtitle path.
        source_path: String,
        /// Managed copied sidecar output path.
        output_path: String,
    },
    /// Backup source media before mutation.
    BackupSource {
        /// Original source path.
        source_path: String,
        /// Destination backup path.
        backup_path: String,
    },
    /// Command invocation and argv.
    Command {
        /// Binary to invoke.
        bin: String,
        /// Positional argument vector.
        argv: Vec<String>,
    },
    /// Output verification checkpoint.
    VerifyOutput {
        /// Path to verify.
        output_path: String,
    },
    /// Conditional failure-recovery quarantine for output that fails verification.
    QuarantineFailedOutput {
        /// Failed generated output path.
        output_path: String,
        /// Managed quarantine path.
        quarantine_path: String,
    },
    /// Atomically replace original source with verified output.
    AtomicReplace {
        /// Original source path.
        source_path: String,
        /// Verified output path.
        output_path: String,
    },
}

/// Stable audit mapping emitted for one compiled execution step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStepAudit {
    /// Stable identifier derived from the compiled step order and kind.
    pub step_id: String,
    /// Zero-based compiled step index.
    pub step_index: usize,
    /// Logical operation indices satisfied by this step.
    pub operation_indices: Vec<usize>,
}

/// Emit a stable mapping from compiled steps to the logical operations they satisfy.
#[must_use]
pub fn compile_execution_step_audits(
    operations: &[PlannedOperation],
    steps: &[ExecutionStep],
) -> Vec<ExecutionStepAudit> {
    let mut audits = steps
        .iter()
        .enumerate()
        .map(|(step_index, step)| ExecutionStepAudit {
            step_id: format!("step-{step_index:04}-{}", execution_step_kind(step)),
            step_index,
            operation_indices: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut assigned = vec![false; operations.len()];
    let primary_command = steps
        .iter()
        .position(|step| matches!(step, ExecutionStep::Command { .. }));

    if let Some(step_index) = primary_command {
        for (operation_index, operation) in operations.iter().enumerate() {
            if !matches!(
                operation.kind,
                OperationKind::CopySidecarSubtitle
                    | OperationKind::ExtractSubtitle
                    | OperationKind::RemoveSidecarSubtitle
            ) {
                audits[step_index].operation_indices.push(operation_index);
                assigned[operation_index] = true;
            }
        }
    }

    for (step_index, step) in steps.iter().enumerate() {
        let operation_kind = match step {
            ExecutionStep::CopySidecarSubtitle { .. } => Some(OperationKind::CopySidecarSubtitle),
            ExecutionStep::Command { .. } if Some(step_index) != primary_command => {
                Some(OperationKind::ExtractSubtitle)
            }
            ExecutionStep::AtomicReplace { .. } => Some(OperationKind::RemoveSidecarSubtitle),
            _ => None,
        };
        let Some(operation_kind) = operation_kind else {
            continue;
        };
        for (operation_index, operation) in operations.iter().enumerate() {
            if !assigned[operation_index] && operation.kind == operation_kind {
                audits[step_index].operation_indices.push(operation_index);
                assigned[operation_index] = true;
                if operation_kind != OperationKind::RemoveSidecarSubtitle {
                    break;
                }
            }
        }
    }

    if let Some(fallback_step) = primary_command.or_else(|| {
        steps
            .iter()
            .position(|step| matches!(step, ExecutionStep::AtomicReplace { .. }))
    }) {
        for (operation_index, was_assigned) in assigned.into_iter().enumerate() {
            if !was_assigned {
                audits[fallback_step]
                    .operation_indices
                    .push(operation_index);
            }
        }
    }
    audits
}

const fn execution_step_kind(step: &ExecutionStep) -> &'static str {
    match step {
        ExecutionStep::Command { .. } => "command",
        ExecutionStep::CopySidecarSubtitle { .. } => "copy-sidecar",
        ExecutionStep::BackupSource { .. } => "backup",
        ExecutionStep::VerifyOutput { .. } => "verify",
        ExecutionStep::QuarantineFailedOutput { .. } => "quarantine",
        ExecutionStep::AtomicReplace { .. } => "replace",
    }
}

/// Build ffmpeg-compatible argv vector without shell-string construction.
///
/// # Errors
///
/// Returns [`BuildArgsError::MissingStreamId`] for transcode operations missing stream identity.
pub fn build_ffmpeg_argv(
    input_path: &str,
    output_path: &str,
    operation: &PlannedOperation,
) -> Result<Vec<String>, BuildArgsError> {
    build_ffmpeg_argv_with_video_encoder(input_path, output_path, operation, DEFAULT_VIDEO_ENCODER)
}

/// Build ffmpeg-compatible argv for extracting an existing embedded subtitle stream.
#[must_use]
pub fn build_extract_subtitle_argv(
    input_path: &str,
    output_path: &str,
    stream_id: u32,
) -> Vec<String> {
    vec![
        "-nostdin".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string(),
        "-map".to_string(),
        format!("0:{stream_id}"),
        "-c".to_string(),
        "copy".to_string(),
        output_path.to_string(),
    ]
}

/// Build ffmpeg-compatible argv for embedding an existing sidecar subtitle.
#[must_use]
pub fn build_sidecar_embed_argv(
    input_path: &str,
    sidecar_path: &str,
    output_path: &str,
) -> Vec<String> {
    let mut args = vec![
        "-nostdin".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string(),
        "-i".to_string(),
        sidecar_path.to_string(),
        "-map".to_string(),
        "0".to_string(),
        "-map".to_string(),
        "1:0".to_string(),
        "-c".to_string(),
        "copy".to_string(),
        "-c:s".to_string(),
        "copy".to_string(),
    ];
    append_default_input_preservation_args(&mut args);
    args.push(output_path.to_string());
    args
}

fn append_default_input_preservation_args(args: &mut Vec<String>) {
    args.push("-map_metadata".to_string());
    args.push("0".to_string());
    append_chapter_preservation_args(args);
}

fn append_input_preservation_args(
    args: &mut Vec<String>,
    container_metadata_policy: Option<&str>,
) -> Result<(), BuildArgsError> {
    append_container_metadata_args(args, container_metadata_policy)?;
    append_chapter_preservation_args(args);
    Ok(())
}

fn append_container_metadata_args(
    args: &mut Vec<String>,
    policy: Option<&str>,
) -> Result<(), BuildArgsError> {
    let normalized = match policy {
        Some(value) => normalize_container_metadata_policy(value)
            .ok_or_else(|| BuildArgsError::UnsupportedContainerMetadataPolicy(value.to_string()))?,
        None => "preserve",
    };
    args.push("-map_metadata".to_string());
    args.push(match normalized {
        "preserve" => "0".to_string(),
        "strip" => "-1".to_string(),
        _ => {
            return Err(BuildArgsError::UnsupportedContainerMetadataPolicy(
                policy.unwrap_or_default().to_string(),
            ));
        }
    });
    Ok(())
}

fn append_chapter_preservation_args(args: &mut Vec<String>) {
    args.push("-map_chapters".to_string());
    args.push("0".to_string());
}

fn build_ffmpeg_argv_with_video_encoder(
    input_path: &str,
    output_path: &str,
    operation: &PlannedOperation,
    video_encoder: &str,
) -> Result<Vec<String>, BuildArgsError> {
    build_ffmpeg_argv_with_video_policy(
        input_path,
        output_path,
        operation,
        video_encoder,
        &VideoTranscodePolicy::default(),
    )
}

fn build_ffmpeg_argv_with_video_policy(
    input_path: &str,
    output_path: &str,
    operation: &PlannedOperation,
    video_encoder: &str,
    policy: &VideoTranscodePolicy,
) -> Result<Vec<String>, BuildArgsError> {
    validate_operations(std::slice::from_ref(operation))?;

    let mut args = vec![
        "-nostdin".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string(),
    ];

    match operation.kind {
        OperationKind::NoOp => return Err(BuildArgsError::NoOpCommand),
        OperationKind::Remux => {
            args.push("-map".to_string());
            args.push("0".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
        }
        OperationKind::MetadataRewrite => return Err(BuildArgsError::UnsupportedMetadataRewrite),
        OperationKind::DispositionRewrite
        | OperationKind::LabelRewrite
        | OperationKind::StreamReorder => return Err(BuildArgsError::ContextlessStreamRewrite),
        OperationKind::AudioTranscode => {
            let stream_id = operation.stream_id.ok_or(BuildArgsError::MissingStreamId)?;
            args.push("-map".to_string());
            args.push("0".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
            args.push(format!("-c:{stream_id}"));
            args.push("aac".to_string());
        }
        OperationKind::VideoTranscode => {
            let stream_id = operation.stream_id.ok_or(BuildArgsError::MissingStreamId)?;
            args.push("-map".to_string());
            args.push("0".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
            args.push(format!("-c:{stream_id}"));
            args.push(video_encoder.to_string());
            append_video_quality_args(&mut args, video_encoder);
            append_hdr_color_args(&mut args, policy.hdr_color);
        }
        OperationKind::EmbedSubtitle
        | OperationKind::ExtractSubtitle
        | OperationKind::CopySidecarSubtitle
        | OperationKind::RemoveSidecarSubtitle
        | OperationKind::SubtitleTranscode => {
            return Err(BuildArgsError::SubtitleArtifactContextRequired);
        }
    }

    append_input_preservation_args(&mut args, None)?;
    args.push(output_path.to_string());
    Ok(args)
}

/// Build deterministic execution steps from planned operations.
///
/// # Errors
///
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_execution_steps(
    input_path: &str,
    output_path: &str,
    operations: &[PlannedOperation],
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    validate_operations(operations)?;
    if operations_are_noop(operations) {
        return Ok(vec![ExecutionStep::VerifyOutput {
            output_path: input_path.to_string(),
        }]);
    }
    let mut steps = Vec::with_capacity(operations.len() + 1);
    for (index, operation) in operations.iter().enumerate() {
        let stage_input = if index == 0 {
            input_path.to_string()
        } else {
            build_intermediate_output_path(output_path, index - 1)
        };
        let stage_output = if index + 1 == operations.len() {
            output_path.to_string()
        } else {
            build_intermediate_output_path(output_path, index)
        };
        let argv = build_ffmpeg_argv(&stage_input, &stage_output, operation)?;
        steps.push(ExecutionStep::Command {
            bin: "ffmpeg".to_string(),
            argv,
        });
    }
    steps.push(ExecutionStep::VerifyOutput {
        output_path: output_path.to_string(),
    });
    Ok(steps)
}

/// Build deterministic execution steps including optional backup and atomic replacement.
///
/// # Errors
///
/// Returns [`BuildArgsError::UnsupportedCodec`] when a required transcode codec is unavailable.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_execution_steps_with_replacement(
    source_path: &str,
    output_path: &str,
    operations: &[PlannedOperation],
    capabilities: &CapabilitySnapshot,
    backup_path: Option<&str>,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    build_execution_steps_with_replacement_policy(
        source_path,
        output_path,
        operations,
        capabilities,
        backup_path,
        None,
    )
}

/// Build deterministic execution steps with optional backup, quarantine, and replacement.
///
/// # Errors
///
/// Returns [`BuildArgsError::UnsupportedCodec`] when a required transcode codec is unavailable.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_execution_steps_with_replacement_policy(
    source_path: &str,
    output_path: &str,
    operations: &[PlannedOperation],
    capabilities: &CapabilitySnapshot,
    backup_path: Option<&str>,
    quarantine_path: Option<&str>,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    let video_policy = VideoTranscodePolicy::default();
    build_execution_steps_with_replacement_video_policy(
        source_path,
        output_path,
        operations,
        capabilities,
        backup_path,
        quarantine_path,
        &video_policy,
    )
}

/// Build deterministic execution steps with optional backup, quarantine, replacement, and video policy.
///
/// # Errors
///
/// Returns [`BuildArgsError::UnsupportedCodec`] when a required transcode codec is unavailable.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_execution_steps_with_replacement_video_policy(
    source_path: &str,
    output_path: &str,
    operations: &[PlannedOperation],
    capabilities: &CapabilitySnapshot,
    backup_path: Option<&str>,
    quarantine_path: Option<&str>,
    video_policy: &VideoTranscodePolicy,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    if operations_are_noop(operations) {
        return build_execution_steps_with_video_policy(
            source_path,
            output_path,
            operations,
            capabilities,
            video_policy,
        );
    }

    let mut steps = Vec::new();
    if let Some(path) = backup_path {
        steps.push(ExecutionStep::BackupSource {
            source_path: source_path.to_string(),
            backup_path: path.to_string(),
        });
    }
    steps.extend(build_execution_steps_with_video_policy(
        source_path,
        output_path,
        operations,
        capabilities,
        video_policy,
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

/// Build deterministic execution steps and validate required transcode codecs against capabilities.
///
/// # Errors
///
/// Returns [`BuildArgsError::UnsupportedCodec`] when a required transcode codec is unavailable.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_execution_steps_with_capabilities(
    input_path: &str,
    output_path: &str,
    operations: &[PlannedOperation],
    capabilities: &CapabilitySnapshot,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    build_execution_steps_with_video_policy(
        input_path,
        output_path,
        operations,
        capabilities,
        &VideoTranscodePolicy::default(),
    )
}

/// Build deterministic execution steps with video transcode policy and capability validation.
///
/// # Errors
///
/// Returns [`BuildArgsError::UnsupportedCodec`] when required transcode codec support is missing.
/// Returns [`BuildArgsError::MissingStreamId`] when operation metadata is incomplete.
pub fn build_execution_steps_with_video_policy(
    input_path: &str,
    output_path: &str,
    operations: &[PlannedOperation],
    capabilities: &CapabilitySnapshot,
    policy: &VideoTranscodePolicy,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    validate_operations(operations)?;
    if operations_are_noop(operations) {
        return Ok(vec![ExecutionStep::VerifyOutput {
            output_path: input_path.to_string(),
        }]);
    }

    let selected_video_encoder = select_video_encoder_for_policy(capabilities, policy);
    validate_operation_capabilities(operations, capabilities, selected_video_encoder)?;

    let video_encoder = selected_video_encoder.unwrap_or(DEFAULT_VIDEO_ENCODER);
    let mut steps = Vec::with_capacity(operations.len() + 1);
    for (index, operation) in operations.iter().enumerate() {
        let stage_input = if index == 0 {
            input_path.to_string()
        } else {
            build_intermediate_output_path(output_path, index - 1)
        };
        let stage_output = if index + 1 == operations.len() {
            output_path.to_string()
        } else {
            build_intermediate_output_path(output_path, index)
        };
        let argv = build_ffmpeg_argv_with_video_policy(
            &stage_input,
            &stage_output,
            operation,
            video_encoder,
            policy,
        )?;
        steps.push(ExecutionStep::Command {
            bin: "ffmpeg".to_string(),
            argv,
        });
    }
    steps.push(ExecutionStep::VerifyOutput {
        output_path: output_path.to_string(),
    });
    Ok(steps)
}

/// Shared desired-graph command construction inputs.
#[derive(Debug, Clone)]
pub struct DesiredGraphBuildContext<'a> {
    /// Inspected primary input path.
    pub input_path: &'a str,
    /// Managed primary output candidate path.
    pub output_path: &'a str,
    /// Inspected primary source graph.
    pub source: &'a MediaGraph,
    /// Complete desired primary graph.
    pub desired: &'a DesiredGraph,
    /// Verified deterministic operation list.
    pub operations: &'a [PlannedOperation],
    /// Optional runtime capability snapshot.
    pub capabilities: Option<&'a CapabilitySnapshot>,
    /// Video transcode policy.
    pub policy: VideoTranscodePolicy,
}

/// Subtitle artifact bindings associated with a desired graph.
#[derive(Debug, Clone, Copy)]
pub struct SubtitleArtifactPlan<'a> {
    /// External sidecars selected as embedded inputs.
    pub embeddings: &'a [SidecarEmbedding],
    /// Managed sidecar candidates to materialize.
    pub outputs: &'a [DesiredSidecarOutput],
    /// Existing sidecar paths removed by the replacement transaction.
    pub removals: &'a [String],
}

/// Build `FFmpeg` argv that materializes an explicit desired stream graph.
///
/// # Errors
///
/// Returns [`BuildArgsError`] when desired streams are missing from the source graph or required
/// codecs are not supported by the supplied capabilities.
pub fn build_desired_graph_ffmpeg_argv(
    input_path: &str,
    output_path: &str,
    source: &MediaGraph,
    desired: &DesiredGraph,
    operations: &[PlannedOperation],
    capabilities: Option<&CapabilitySnapshot>,
    policy: VideoTranscodePolicy,
) -> Result<Vec<String>, BuildArgsError> {
    build_desired_graph_ffmpeg_argv_with_sidecars(
        DesiredGraphBuildContext {
            input_path,
            output_path,
            source,
            desired,
            operations,
            capabilities,
            policy,
        },
        &[],
    )
}

/// Build `FFmpeg` argv for a desired graph with external subtitle inputs.
///
/// # Errors
///
/// Returns [`BuildArgsError`] when stream bindings, codecs, or capabilities are invalid.
pub fn build_desired_graph_ffmpeg_argv_with_sidecars(
    context: DesiredGraphBuildContext<'_>,
    sidecar_embeddings: &[SidecarEmbedding],
) -> Result<Vec<String>, BuildArgsError> {
    let DesiredGraphBuildContext {
        input_path,
        output_path,
        source,
        desired,
        operations,
        capabilities,
        policy,
    } = context;
    validate_operations(operations)?;
    if operations_are_noop(operations) {
        return Err(BuildArgsError::NoOpCommand);
    }
    verify_desired_graph_stream_ids(source, desired, sidecar_embeddings)?;
    validate_materialized_stream_kinds(source, desired)?;

    let selected_video_encoder = capabilities.and_then(|snapshot| {
        validate_operation_capabilities(
            operations,
            snapshot,
            select_video_encoder_for_policy(snapshot, &policy),
        )
        .ok()
        .and_then(|()| select_video_encoder_for_policy(snapshot, &policy))
    });
    if let Some(snapshot) = capabilities {
        validate_operation_capabilities(operations, snapshot, selected_video_encoder)?;
        validate_muxer_capability(desired, snapshot)?;
    }
    let video_encoder = selected_video_encoder.unwrap_or(DEFAULT_VIDEO_ENCODER);

    let mut args = vec![
        "-nostdin".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string(),
    ];
    for binding in sidecar_embeddings {
        args.push("-i".to_string());
        args.push(binding.path.clone());
    }

    for stream in &desired.streams {
        args.push("-map".to_string());
        args.push(desired_stream_map(
            source,
            desired,
            stream,
            sidecar_embeddings,
        )?);
    }

    append_desired_stream_args(
        &mut args,
        source,
        desired,
        sidecar_embeddings,
        capabilities,
        video_encoder,
        &policy,
    )?;

    if let Some(container_format) = desired
        .container_format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
    {
        args.push("-f".to_string());
        args.push(normalize_container_format(container_format));
    }

    append_input_preservation_args(&mut args, desired.container_metadata_policy.as_deref())?;
    args.push(output_path.to_string());
    Ok(args)
}

fn append_desired_stream_args(
    args: &mut Vec<String>,
    source: &MediaGraph,
    desired: &DesiredGraph,
    sidecar_embeddings: &[SidecarEmbedding],
    capabilities: Option<&CapabilitySnapshot>,
    video_encoder: &str,
    policy: &VideoTranscodePolicy,
) -> Result<(), BuildArgsError> {
    let mut audio_output_index = 0;
    for (output_index, stream) in desired.streams.iter().enumerate() {
        let constraints = audio_constraints_for_stream(policy, stream);
        let output_codec = output_codec_for_desired_with_audio_policy(
            source,
            desired,
            stream,
            sidecar_embeddings,
            video_encoder,
            constraints,
        )?;
        if let Some(snapshot) = capabilities {
            validate_output_codec_capability(snapshot, &output_codec)?;
        }
        args.push(format!("-c:{output_index}"));
        args.push(output_codec.clone());
        if stream.kind == StreamKind::Video && output_codec != "copy" {
            let constraints = video_constraints_for_stream(policy, stream);
            append_video_quality_args(args, &output_codec);
            append_video_constraint_args(args, output_index, constraints);
            append_hdr_color_args(args, policy.hdr_color);
        }
        if stream.kind == StreamKind::Audio && output_codec != "copy" {
            append_audio_shape_args(args, output_index, audio_output_index, stream, constraints);
        }
        append_stream_metadata_args(args, output_index, stream);
        if stream.kind == StreamKind::Audio {
            audio_output_index += 1;
        }
    }
    Ok(())
}

fn validate_muxer_capability(
    desired: &DesiredGraph,
    capabilities: &CapabilitySnapshot,
) -> Result<(), BuildArgsError> {
    let Some(container_format) = desired
        .container_format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
    else {
        return Ok(());
    };
    validate_container_muxer_capability(container_format, capabilities)
}

/// Validate that a declared output container can be muxed by the supplied capability snapshot.
///
/// # Errors
///
/// Returns [`BuildArgsError::UnsupportedMuxer`] when the normalized container has no matching
/// muxer in the snapshot.
pub fn validate_container_muxer_capability(
    container_format: &str,
    capabilities: &CapabilitySnapshot,
) -> Result<(), BuildArgsError> {
    let container_format = normalize_container_format(container_format);
    if container_format.is_empty() {
        return Ok(());
    }
    if capabilities
        .muxers
        .iter()
        .map(|muxer| normalize_container_format(muxer))
        .any(|muxer| muxer == container_format)
    {
        Ok(())
    } else {
        Err(BuildArgsError::UnsupportedMuxer(container_format))
    }
}

/// Validate source-independent encoder support for a declared desired-target stream.
///
/// This intentionally ignores source-copy shortcuts because profile readiness has no concrete
/// source graph. Runtime preflight can still choose `copy` for a specific source later.
///
/// # Errors
///
/// Returns [`BuildArgsError::UnsupportedCodec`] when the stream kind or desired codec cannot be
/// materialized by the supplied capability snapshot and video policy.
pub fn validate_declared_stream_codec_capability(
    stream_kind: StreamKind,
    codec: &str,
    capabilities: &CapabilitySnapshot,
    policy: &VideoTranscodePolicy,
) -> Result<(), BuildArgsError> {
    let codec = codec.trim().to_ascii_lowercase();
    if codec == "copy" {
        return Ok(());
    }
    let selected_video_encoder =
        select_video_encoder_for_policy(capabilities, policy).unwrap_or(DEFAULT_VIDEO_ENCODER);
    let output_codec = match stream_kind {
        StreamKind::Video => video_encoder_for_codec(&codec, selected_video_encoder)?,
        StreamKind::Audio => audio_encoder_for_codec(&codec)?,
        StreamKind::Subtitle => subtitle_encoder_for_codec(&codec)?,
        StreamKind::Attachment | StreamKind::Chapter | StreamKind::Data => {
            return Err(BuildArgsError::UnsupportedCodec("stream"));
        }
    };
    validate_output_codec_capability(capabilities, &output_codec)
}

/// Build execution steps for materializing an explicit desired stream graph.
///
/// # Errors
///
/// Returns [`BuildArgsError`] when desired stream mapping or codec validation fails.
pub fn build_desired_graph_execution_steps(
    input_path: &str,
    output_path: &str,
    source: &MediaGraph,
    desired: &DesiredGraph,
    operations: &[PlannedOperation],
    capabilities: Option<&CapabilitySnapshot>,
    policy: VideoTranscodePolicy,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    build_desired_graph_execution_steps_with_sidecars(
        DesiredGraphBuildContext {
            input_path,
            output_path,
            source,
            desired,
            operations,
            capabilities,
            policy,
        },
        SubtitleArtifactPlan {
            embeddings: &[],
            outputs: &[],
            removals: &[],
        },
    )
}

/// Build execution steps for the primary graph and managed subtitle artifacts.
///
/// # Errors
///
/// Returns [`BuildArgsError`] when bindings, capabilities, artifact paths, or transaction
/// requirements are invalid.
pub fn build_desired_graph_execution_steps_with_sidecars(
    context: DesiredGraphBuildContext<'_>,
    artifacts: SubtitleArtifactPlan<'_>,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    let DesiredGraphBuildContext {
        input_path,
        output_path,
        source,
        desired,
        operations,
        capabilities,
        policy,
    } = context;
    let SubtitleArtifactPlan {
        embeddings: sidecar_embeddings,
        outputs: sidecar_outputs,
        removals: _,
    } = artifacts;
    validate_operations(operations)?;
    verify_desired_graph_stream_ids(source, desired, sidecar_embeddings)?;
    if operations_are_noop(operations) {
        return Ok(vec![ExecutionStep::VerifyOutput {
            output_path: input_path.to_string(),
        }]);
    }

    let argv = build_desired_graph_ffmpeg_argv_with_sidecars(
        DesiredGraphBuildContext {
            input_path,
            output_path,
            source,
            desired,
            operations,
            capabilities,
            policy,
        },
        sidecar_embeddings,
    )?;
    let mut steps = vec![
        ExecutionStep::Command {
            bin: "ffmpeg".to_string(),
            argv,
        },
        ExecutionStep::VerifyOutput {
            output_path: output_path.to_string(),
        },
    ];
    append_sidecar_output_steps(
        &mut steps,
        input_path,
        source,
        sidecar_outputs,
        capabilities,
    )?;
    Ok(steps)
}

fn validate_operations(operations: &[PlannedOperation]) -> Result<(), BuildArgsError> {
    if operations.is_empty() {
        return Err(BuildArgsError::EmptyOperations);
    }
    verify_plan(operations).map_err(operation_plan_error)
}

fn operation_plan_error(error: &'static str) -> BuildArgsError {
    match error {
        "plan must contain at least one operation" => BuildArgsError::EmptyOperations,
        "stream-scoped operation is missing stream id"
        | "stream-scoped operation is missing source or output stream id" => {
            BuildArgsError::MissingStreamId
        }
        other => BuildArgsError::InvalidOperations(other),
    }
}

fn source_stream_for_desired<'a>(
    source: &'a MediaGraph,
    desired_graph: &DesiredGraph,
    desired: &MediaStream,
) -> Result<&'a MediaStream, BuildArgsError> {
    let source_stream_id = if desired_graph.stream_bindings.is_empty() {
        desired.stream_id
    } else {
        desired_graph
            .stream_bindings
            .iter()
            .find(|binding| binding.output_stream_id == desired.stream_id)
            .and_then(|binding| binding.source_stream_id)
            .ok_or(BuildArgsError::DesiredStreamMissing(desired.stream_id))?
    };
    let source_stream = source
        .streams
        .iter()
        .find(|stream| stream.stream_id == source_stream_id)
        .ok_or(BuildArgsError::DesiredStreamMissing(desired.stream_id))?;
    if source_stream.kind != desired.kind {
        return Err(BuildArgsError::DesiredStreamKindMismatch(desired.stream_id));
    }
    Ok(source_stream)
}

fn desired_stream_map(
    source: &MediaGraph,
    desired_graph: &DesiredGraph,
    desired: &MediaStream,
    embeddings: &[SidecarEmbedding],
) -> Result<String, BuildArgsError> {
    match source_stream_for_desired(source, desired_graph, desired) {
        Ok(source_stream) => return Ok(format!("0:{}", source_stream.stream_id)),
        Err(BuildArgsError::DesiredStreamMissing(_)) => {}
        Err(error) => return Err(error),
    }
    embeddings
        .iter()
        .position(|binding| binding.output_stream == *desired)
        .map(|index| format!("{}:0", index + 1))
        .ok_or(BuildArgsError::DesiredStreamMissing(desired.stream_id))
}

fn output_codec_for_desired(
    source: &MediaGraph,
    desired_graph: &DesiredGraph,
    desired: &MediaStream,
    embeddings: &[SidecarEmbedding],
    video_encoder: &str,
) -> Result<String, BuildArgsError> {
    match source_stream_for_desired(source, desired_graph, desired) {
        Ok(source_stream) => {
            return output_codec_for_stream(source_stream, desired, video_encoder);
        }
        Err(BuildArgsError::DesiredStreamMissing(_)) => {}
        Err(error) => return Err(error),
    }
    let binding = embeddings
        .iter()
        .find(|binding| binding.output_stream == *desired)
        .ok_or(BuildArgsError::DesiredStreamMissing(desired.stream_id))?;
    if desired.kind != StreamKind::Subtitle {
        return Err(BuildArgsError::DesiredStreamKindMismatch(desired.stream_id));
    }
    if subtitle_codecs_equivalent(&binding.source_codec, &desired.codec) {
        Ok("copy".to_string())
    } else {
        subtitle_encoder_for_codec(&desired.codec.trim().to_ascii_lowercase())
    }
}

fn verify_desired_graph_stream_ids(
    source: &MediaGraph,
    desired: &DesiredGraph,
    embeddings: &[SidecarEmbedding],
) -> Result<(), BuildArgsError> {
    verify_unique_stream_ids(&source.streams)
        .map_err(|_| BuildArgsError::DuplicateSourceStreamIds)?;
    verify_unique_stream_ids(&desired.streams)
        .map_err(|_| BuildArgsError::DuplicateDesiredStreamIds)?;
    let mut binding_ids = std::collections::BTreeSet::new();
    for binding in embeddings {
        if !binding_ids.insert(binding.output_stream.stream_id)
            || source
                .streams
                .iter()
                .any(|stream| stream.stream_id == binding.output_stream.stream_id)
            || !desired
                .streams
                .iter()
                .any(|stream| stream == &binding.output_stream)
        {
            return Err(BuildArgsError::DuplicateDesiredStreamIds);
        }
    }
    for stream in &desired.streams {
        desired_stream_map(source, desired, stream, embeddings)?;
    }
    Ok(())
}

fn validate_materialized_stream_kinds(
    source: &MediaGraph,
    desired: &DesiredGraph,
) -> Result<(), BuildArgsError> {
    for stream in &desired.streams {
        match stream.kind {
            StreamKind::Attachment | StreamKind::Data => {
                validate_exact_passthrough(source, desired, stream)?;
            }
            StreamKind::Chapter => {
                return Err(BuildArgsError::UnsupportedDesiredStreamKind {
                    stream_id: stream.stream_id,
                });
            }
            StreamKind::Audio | StreamKind::Video | StreamKind::Subtitle => {}
        }
    }
    Ok(())
}

fn validate_exact_passthrough(
    source: &MediaGraph,
    desired_graph: &DesiredGraph,
    desired: &MediaStream,
) -> Result<(), BuildArgsError> {
    let source_stream = source_stream_for_desired(source, desired_graph, desired)?;
    if source_stream == desired {
        Ok(())
    } else {
        Err(BuildArgsError::UnsupportedDesiredStreamKind {
            stream_id: desired.stream_id,
        })
    }
}

fn append_sidecar_output_steps(
    steps: &mut Vec<ExecutionStep>,
    input_path: &str,
    source: &MediaGraph,
    outputs: &[DesiredSidecarOutput],
    capabilities: Option<&CapabilitySnapshot>,
) -> Result<(), BuildArgsError> {
    for output in outputs {
        append_sidecar_materialization_step(steps, input_path, source, output, capabilities)?;
        steps.push(ExecutionStep::VerifyOutput {
            output_path: output.path.clone(),
        });
        if let Some(companion) = &output.companion_path {
            steps.push(ExecutionStep::VerifyOutput {
                output_path: companion.clone(),
            });
        }
    }
    Ok(())
}

fn append_sidecar_materialization_step(
    steps: &mut Vec<ExecutionStep>,
    input_path: &str,
    source: &MediaGraph,
    output: &DesiredSidecarOutput,
    capabilities: Option<&CapabilitySnapshot>,
) -> Result<(), BuildArgsError> {
    match &output.source {
        SidecarOutputSource::EmbeddedStream { stream_id } => {
            let source_stream = source
                .streams
                .iter()
                .find(|stream| stream.stream_id == *stream_id)
                .ok_or(BuildArgsError::DesiredStreamMissing(*stream_id))?;
            if source_stream.kind != StreamKind::Subtitle {
                return Err(BuildArgsError::DesiredStreamKindMismatch(*stream_id));
            }
            let codec = subtitle_output_codec(&source_stream.codec, &output.codec)?;
            validate_sidecar_output_capability(capabilities, &codec)?;
            steps.push(ExecutionStep::Command {
                bin: "ffmpeg".to_string(),
                argv: build_subtitle_materialization_argv(
                    input_path,
                    &output.path,
                    &format!("0:{stream_id}"),
                    &codec,
                ),
            });
        }
        SidecarOutputSource::ExistingSidecar {
            path,
            companion_path,
            codec,
        } => append_existing_sidecar_materialization(
            steps,
            output,
            path,
            companion_path.as_ref(),
            codec,
            capabilities,
        )?,
    }
    Ok(())
}

fn append_existing_sidecar_materialization(
    steps: &mut Vec<ExecutionStep>,
    output: &DesiredSidecarOutput,
    path: &str,
    companion_path: Option<&String>,
    codec: &str,
    capabilities: Option<&CapabilitySnapshot>,
) -> Result<(), BuildArgsError> {
    let output_codec = subtitle_output_codec(codec, &output.codec)?;
    if output_codec == "copy" {
        steps.push(ExecutionStep::CopySidecarSubtitle {
            source_path: path.to_string(),
            output_path: output.path.clone(),
        });
        return append_companion_copy(steps, companion_path, output.companion_path.as_ref());
    }
    if companion_path.is_some() || output.companion_path.is_some() {
        return Err(BuildArgsError::SidecarCompanionMismatch);
    }
    validate_sidecar_output_capability(capabilities, &output_codec)?;
    steps.push(ExecutionStep::Command {
        bin: "ffmpeg".to_string(),
        argv: build_subtitle_materialization_argv(path, &output.path, "0:0", &output_codec),
    });
    Ok(())
}

fn append_companion_copy(
    steps: &mut Vec<ExecutionStep>,
    source: Option<&String>,
    output: Option<&String>,
) -> Result<(), BuildArgsError> {
    match (source, output) {
        (Some(source_path), Some(output_path)) => {
            steps.push(ExecutionStep::CopySidecarSubtitle {
                source_path: source_path.clone(),
                output_path: output_path.clone(),
            });
            Ok(())
        }
        (None, None) => Ok(()),
        _ => Err(BuildArgsError::SidecarCompanionMismatch),
    }
}

fn subtitle_output_codec(source: &str, desired: &str) -> Result<String, BuildArgsError> {
    if subtitle_codecs_equivalent(source, desired) {
        Ok("copy".to_string())
    } else {
        subtitle_encoder_for_codec(&desired.trim().to_ascii_lowercase())
    }
}

fn validate_sidecar_output_capability(
    capabilities: Option<&CapabilitySnapshot>,
    codec: &str,
) -> Result<(), BuildArgsError> {
    capabilities.map_or(Ok(()), |snapshot| {
        validate_output_codec_capability(snapshot, codec)
    })
}

fn build_subtitle_materialization_argv(
    input_path: &str,
    output_path: &str,
    stream_map: &str,
    codec: &str,
) -> Vec<String> {
    vec![
        "-nostdin".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string(),
        "-map".to_string(),
        stream_map.to_string(),
        "-c:s".to_string(),
        codec.to_string(),
        output_path.to_string(),
    ]
}

fn output_codec_for_stream(
    source: &MediaStream,
    desired: &MediaStream,
    video_encoder: &str,
) -> Result<String, BuildArgsError> {
    if source
        .codec
        .trim()
        .eq_ignore_ascii_case(desired.codec.trim())
        && !audio_shape_differs(source, desired)
    {
        return Ok("copy".to_string());
    }

    let codec = desired.codec.trim().to_ascii_lowercase();
    match desired.kind {
        StreamKind::Video => video_encoder_for_codec(&codec, video_encoder),
        StreamKind::Audio => audio_encoder_for_codec(&codec),
        StreamKind::Subtitle => subtitle_encoder_for_codec(&codec),
        StreamKind::Attachment | StreamKind::Chapter | StreamKind::Data => Ok("copy".to_string()),
    }
}

fn output_codec_for_desired_with_audio_policy(
    source: &MediaGraph,
    desired_graph: &DesiredGraph,
    desired: &MediaStream,
    sidecar_embeddings: &[SidecarEmbedding],
    video_encoder: &str,
    audio_constraints: Option<&AudioStreamConstraints>,
) -> Result<String, BuildArgsError> {
    let output_codec = output_codec_for_desired(
        source,
        desired_graph,
        desired,
        sidecar_embeddings,
        video_encoder,
    )?;
    if output_codec == "copy" && audio_constraints_require_filter(audio_constraints) {
        return audio_encoder_for_codec(desired.codec.trim());
    }
    Ok(output_codec)
}

fn audio_shape_differs(source: &MediaStream, desired: &MediaStream) -> bool {
    if source.kind != StreamKind::Audio || desired.kind != StreamKind::Audio {
        return false;
    }
    let channels_differ = desired
        .channels
        .is_some_and(|desired_channels| source.channels != Some(desired_channels));
    let layout_differ = normalized_channel_layout(desired.channel_layout.as_deref()).is_some_and(
        |desired_layout| {
            normalized_channel_layout(source.channel_layout.as_deref()) != Some(desired_layout)
        },
    );
    channels_differ || layout_differ
}

fn append_audio_shape_args(
    args: &mut Vec<String>,
    output_index: usize,
    audio_output_index: usize,
    stream: &MediaStream,
    constraints: Option<&AudioStreamConstraints>,
) {
    if let Some(channel_layout) = normalized_channel_layout(stream.channel_layout.as_deref()) {
        args.push(format!("-channel_layout:a:{audio_output_index}"));
        args.push(channel_layout);
    } else if let Some(channels) = stream.channels {
        args.push(format!("-ac:{output_index}"));
        args.push(channels.to_string());
    }
    if let Some(bitrate_bps) = constraints.and_then(|constraint| constraint.bitrate_bps) {
        args.push(format!("-b:a:{audio_output_index}"));
        args.push(bitrate_bps.to_string());
    }
    if let Some(sample_rate_hz) = constraints.and_then(|constraint| constraint.sample_rate_hz) {
        args.push(format!("-ar:{audio_output_index}"));
        args.push(sample_rate_hz.to_string());
    }
    if let Some(filter) = audio_filtergraph_for_constraints(constraints) {
        args.push(format!("-filter:a:{audio_output_index}"));
        args.push(filter);
    }
}

fn audio_constraints_for_stream<'a>(
    policy: &'a VideoTranscodePolicy,
    stream: &MediaStream,
) -> Option<&'a AudioStreamConstraints> {
    policy
        .audio_stream_constraints
        .iter()
        .find(|constraint| constraint.stream_id == stream.stream_id)
}

fn audio_constraints_require_filter(constraints: Option<&AudioStreamConstraints>) -> bool {
    constraints.is_some_and(|constraint| {
        constraint
            .loudness_profile
            .as_deref()
            .is_some_and(|profile| profile.trim().eq_ignore_ascii_case("dialog-normalized"))
            || constraint
                .dynamic_range
                .as_deref()
                .is_some_and(|behavior| behavior.trim().eq_ignore_ascii_case("speech"))
    })
}

fn audio_filtergraph_for_constraints(
    constraints: Option<&AudioStreamConstraints>,
) -> Option<String> {
    let constraints = constraints?;
    let mut filters = Vec::new();
    if constraints
        .loudness_profile
        .as_deref()
        .is_some_and(|profile| profile.trim().eq_ignore_ascii_case("dialog-normalized"))
    {
        filters.push("loudnorm=I=-16:TP=-1.5:LRA=11");
    }
    if constraints
        .dynamic_range
        .as_deref()
        .is_some_and(|behavior| behavior.trim().eq_ignore_ascii_case("speech"))
    {
        filters.push("acompressor=threshold=-18dB:ratio=2:attack=20:release=250");
    }
    (!filters.is_empty()).then(|| filters.join(","))
}

fn normalized_channel_layout(value: Option<&str>) -> Option<String> {
    value.and_then(|item| normalize_audio_channel_layout(item).map(str::to_string))
}

fn validate_output_codec_capability(
    capabilities: &CapabilitySnapshot,
    output_codec: &str,
) -> Result<(), BuildArgsError> {
    if output_codec == "copy" || capabilities_has_encoder(capabilities, output_codec) {
        return Ok(());
    }
    Err(BuildArgsError::UnsupportedCodec(unsupported_encoder_label(
        output_codec,
    )))
}

fn unsupported_encoder_label(output_codec: &str) -> &'static str {
    match output_codec {
        "aac" => "aac",
        "ac3" => "ac3",
        "eac3" => "eac3",
        "mpeg4" => "mpeg4",
        "srt" => "srt",
        "webvtt" => "webvtt",
        "ass" => "ass",
        "ssa" => "ssa",
        "microdvd" => "microdvd",
        "mov_text" => "mov_text",
        "libopus" => "libopus",
        "libx264" => "libx264",
        "libx265" => "libx265",
        "libvpx" => "libvpx",
        "libvpx-vp9" => "libvpx-vp9",
        "libaom-av1" => "libaom-av1",
        "libmp3lame" => "libmp3lame",
        "libvorbis" => "libvorbis",
        "hevc_nvenc" => "hevc_nvenc",
        "hevc_qsv" => "hevc_qsv",
        "hevc_vaapi" => "hevc_vaapi",
        _ => "encoder",
    }
}

fn video_encoder_for_codec(
    codec: &str,
    selected_hevc_encoder: &str,
) -> Result<String, BuildArgsError> {
    match codec {
        "hevc" | "h265" => Ok(selected_hevc_encoder.to_string()),
        "h264" => Ok("libx264".to_string()),
        "mpeg4" => Ok("mpeg4".to_string()),
        "vp8" => Ok("libvpx".to_string()),
        "vp9" => Ok("libvpx-vp9".to_string()),
        "av1" => Ok("libaom-av1".to_string()),
        _ => Err(BuildArgsError::UnsupportedCodec("video")),
    }
}

fn audio_encoder_for_codec(codec: &str) -> Result<String, BuildArgsError> {
    match codec {
        "aac" => Ok("aac".to_string()),
        "opus" => Ok("libopus".to_string()),
        "ac3" => Ok("ac3".to_string()),
        "eac3" => Ok("eac3".to_string()),
        "mp3" => Ok("libmp3lame".to_string()),
        "vorbis" => Ok("libvorbis".to_string()),
        _ => Err(BuildArgsError::UnsupportedCodec("audio")),
    }
}

fn subtitle_encoder_for_codec(codec: &str) -> Result<String, BuildArgsError> {
    match codec {
        "srt" | "subrip" => Ok("srt".to_string()),
        "webvtt" | "vtt" => Ok("webvtt".to_string()),
        "ass" => Ok("ass".to_string()),
        "ssa" => Ok("ssa".to_string()),
        "microdvd" => Ok("microdvd".to_string()),
        "mov_text" => Ok("mov_text".to_string()),
        _ => Err(BuildArgsError::UnsupportedCodec("subtitle")),
    }
}

fn subtitle_codecs_equivalent(left: &str, right: &str) -> bool {
    normalize_subtitle_codec(left) == normalize_subtitle_codec(right)
}

fn append_stream_metadata_args(args: &mut Vec<String>, output_index: usize, stream: &MediaStream) {
    if matches!(stream.kind, StreamKind::Attachment | StreamKind::Data) {
        return;
    }

    if let Some(language) = stream
        .language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.push(format!("-metadata:s:{output_index}"));
        args.push(format!("language={language}"));
    }
    if let Some(title) = stream
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.push(format!("-metadata:s:{output_index}"));
        args.push(format!("title={title}"));
    }

    args.push(format!("-disposition:{output_index}"));
    if stream.dispositions.is_empty() {
        args.push("0".to_string());
    } else {
        args.push(stream.dispositions.join("+"));
    }
}

/// Execute a non-command filesystem step.
///
/// Command invocation remains the responsibility of an injected command runner.
///
/// # Errors
///
/// Returns [`ExecuteStepError`] when the step is a command, verification fails, or filesystem
/// operations fail.
pub fn execute_filesystem_step(step: &ExecutionStep) -> Result<(), ExecuteStepError> {
    match step {
        ExecutionStep::CopySidecarSubtitle {
            source_path,
            output_path,
        } => copy_file_to_path(source_path, output_path, "execution.copy_sidecar_subtitle"),
        ExecutionStep::BackupSource {
            source_path,
            backup_path,
        } => copy_file_to_path(source_path, backup_path, "execution.backup_source"),
        ExecutionStep::VerifyOutput { output_path } => verify_output_file(output_path),
        ExecutionStep::QuarantineFailedOutput {
            output_path,
            quarantine_path,
        } => move_file_to_path(
            output_path,
            quarantine_path,
            "execution.quarantine_failed_output",
        ),
        ExecutionStep::AtomicReplace { .. } => Err(ExecuteStepError::ManagedReplacementRequired),
        ExecutionStep::Command { .. } => Err(ExecuteStepError::CommandStepUnsupported),
    }
}

/// Execute a planned step with an injected command runner.
///
/// # Errors
///
/// Returns [`ExecuteStepError`] when command execution, verification, or filesystem operations
/// fail.
pub fn execute_step(
    step: &ExecutionStep,
    command_runner: &dyn CommandRunner,
) -> Result<(), ExecuteStepError> {
    execute_step_controlled(step, command_runner, &NeverCancel)
}

/// Execute a planned step while observing cooperative cancellation.
///
/// # Errors
///
/// Returns [`ExecuteStepError`] when execution, verification, filesystem work, or cancellation
/// fails.
pub fn execute_step_controlled(
    step: &ExecutionStep,
    command_runner: &dyn CommandRunner,
    control: &dyn ExecutionControl,
) -> Result<(), ExecuteStepError> {
    if let Some(breach) = control.limit_breach() {
        return Err(ExecuteStepError::LimitBreached(breach));
    }
    if control.cancellation_requested() {
        return Err(ExecuteStepError::Cancelled);
    }
    match step {
        ExecutionStep::Command { bin, argv } => command_runner.run_controlled(bin, argv, control),
        _ => execute_filesystem_step(step),
    }
}

/// Execute planned steps in order, using quarantine steps only as recovery actions.
///
/// # Errors
///
/// Returns [`ExecuteSequenceError`] when a step fails. If a quarantine recovery step is present,
/// the returned error also includes any recovery failure.
pub fn execute_step_sequence(
    steps: &[ExecutionStep],
    command_runner: &dyn CommandRunner,
) -> Result<(), ExecuteSequenceError> {
    execute_step_sequence_controlled(steps, command_runner, &NeverCancel)
}

/// Execute planned steps while observing cooperative cancellation.
///
/// # Errors
///
/// Returns [`ExecuteSequenceError`] when a step fails or cancellation is requested.
pub fn execute_step_sequence_controlled(
    steps: &[ExecutionStep],
    command_runner: &dyn CommandRunner,
    control: &dyn ExecutionControl,
) -> Result<(), ExecuteSequenceError> {
    for (index, step) in steps.iter().enumerate() {
        if is_recovery_step(step) {
            continue;
        }
        if let Err(failed) = execute_step_controlled(step, command_runner, control) {
            let recovery = steps
                .iter()
                .find(|candidate| is_recovery_step(candidate))
                .and_then(|candidate| execute_filesystem_step(candidate).err());
            return Err(ExecuteSequenceError {
                failed_step_index: index,
                failed,
                recovery,
            });
        }
    }
    Ok(())
}

fn validate_operation_capabilities(
    operations: &[PlannedOperation],
    capabilities: &CapabilitySnapshot,
    selected_video_encoder: Option<&'static str>,
) -> Result<(), BuildArgsError> {
    for operation in operations {
        validate_operation_capability(operation, capabilities, selected_video_encoder)?;
    }
    Ok(())
}

fn validate_operation_capability(
    operation: &PlannedOperation,
    capabilities: &CapabilitySnapshot,
    selected_video_encoder: Option<&'static str>,
) -> Result<(), BuildArgsError> {
    match operation.kind {
        OperationKind::AudioTranscode => {
            if capabilities_has_encoder(capabilities, "aac") {
                Ok(())
            } else {
                Err(BuildArgsError::UnsupportedCodec("aac"))
            }
        }
        OperationKind::VideoTranscode => selected_video_encoder.map_or_else(
            || Err(BuildArgsError::UnsupportedCodec(DEFAULT_VIDEO_ENCODER)),
            |_| Ok(()),
        ),
        OperationKind::NoOp
        | OperationKind::Remux
        | OperationKind::DispositionRewrite
        | OperationKind::LabelRewrite
        | OperationKind::MetadataRewrite
        | OperationKind::StreamReorder
        | OperationKind::EmbedSubtitle
        | OperationKind::ExtractSubtitle
        | OperationKind::CopySidecarSubtitle
        | OperationKind::RemoveSidecarSubtitle
        | OperationKind::SubtitleTranscode => Ok(()),
    }
}

fn operations_are_noop(operations: &[PlannedOperation]) -> bool {
    matches!(
        operations,
        [PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
            output_stream_id: None,
        }]
    )
}

fn capabilities_has_encoder(capabilities: &CapabilitySnapshot, required: &str) -> bool {
    capabilities
        .encoders
        .iter()
        .any(|encoder| encoder.trim().eq_ignore_ascii_case(required))
}

fn select_video_encoder_for_policy(
    capabilities: &CapabilitySnapshot,
    policy: &VideoTranscodePolicy,
) -> Option<&'static str> {
    let candidates = match policy.intent {
        VideoTranscodeIntent::Anime | VideoTranscodeIntent::Archival => {
            SOFTWARE_VIDEO_ENCODER_FALLBACKS
        }
        VideoTranscodeIntent::General | VideoTranscodeIntent::Audiobook => VIDEO_ENCODER_FALLBACKS,
    };
    candidates
        .iter()
        .copied()
        .find(|candidate| capabilities_has_encoder(capabilities, candidate))
}

fn append_video_quality_args(args: &mut Vec<String>, video_encoder: &str) {
    args.push("-preset".to_string());
    args.push(VIDEO_TRANSCODE_PRESET.to_string());
    match video_encoder {
        "hevc_nvenc" => {
            args.push("-b:v".to_string());
            args.push("0".to_string());
            args.push("-cq".to_string());
            args.push(VIDEO_TRANSCODE_CRF.to_string());
        }
        "hevc_qsv" => {
            args.push("-global_quality".to_string());
            args.push(VIDEO_TRANSCODE_CRF.to_string());
        }
        "hevc_vaapi" => {
            args.push("-qp".to_string());
            args.push(VIDEO_TRANSCODE_CRF.to_string());
        }
        _ => {
            args.push("-crf".to_string());
            args.push(VIDEO_TRANSCODE_CRF.to_string());
        }
    }
}

fn video_constraints_for_stream<'a>(
    policy: &'a VideoTranscodePolicy,
    stream: &MediaStream,
) -> Option<&'a VideoStreamConstraints> {
    policy
        .stream_constraints
        .iter()
        .find(|constraints| constraints.stream_id == stream.stream_id)
}

fn append_video_constraint_args(
    args: &mut Vec<String>,
    output_index: usize,
    constraints: Option<&VideoStreamConstraints>,
) {
    let Some(constraints) = constraints else {
        return;
    };
    append_optional_stream_arg(
        args,
        "profile",
        output_index,
        constraints.profile.as_deref(),
    );
    append_optional_stream_arg(args, "level", output_index, constraints.level.as_deref());
    if let Some(max_bitrate_bps) = constraints.max_bitrate_bps {
        let max_bitrate_bps = u64::from(max_bitrate_bps.get());
        let average_target = max_bitrate_bps.saturating_mul(VIDEO_AVERAGE_TARGET_PERCENT) / 100;
        let vbv_buffer = max_bitrate_bps.saturating_mul(VIDEO_VBV_SECONDS);
        args.push(format!("-b:{output_index}"));
        args.push(average_target.to_string());
        args.push(format!("-maxrate:{output_index}"));
        args.push(max_bitrate_bps.to_string());
        args.push(format!("-bufsize:{output_index}"));
        args.push(vbv_buffer.to_string());
    }
    let (color_primaries, color_transfer, color_space) =
        normalized_video_color_constraints(constraints);
    append_optional_stream_arg(
        args,
        "color_primaries",
        output_index,
        color_primaries.as_deref(),
    );
    append_optional_stream_arg(args, "color_trc", output_index, color_transfer.as_deref());
    append_optional_stream_arg(args, "colorspace", output_index, color_space.as_deref());
}

fn append_optional_stream_arg(
    args: &mut Vec<String>,
    key: &str,
    output_index: usize,
    value: Option<&str>,
) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    args.push(format!("-{key}:{output_index}"));
    args.push(value.to_string());
}

fn normalized_video_color_constraints(
    constraints: &VideoStreamConstraints,
) -> (Option<String>, Option<String>, Option<String>) {
    let hdr_format = constraints
        .hdr_format
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if hdr_format.eq_ignore_ascii_case("hdr10") {
        return (
            constraints
                .color_primaries
                .clone()
                .or_else(|| Some("bt2020".to_string())),
            constraints
                .color_transfer
                .clone()
                .or_else(|| Some("smpte2084".to_string())),
            constraints
                .color_space
                .clone()
                .or_else(|| Some("bt2020nc".to_string())),
        );
    }
    (
        constraints.color_primaries.clone(),
        constraints.color_transfer.clone(),
        constraints.color_space.clone(),
    )
}

fn append_hdr_color_args(args: &mut Vec<String>, policy: HdrColorPolicy) {
    match policy {
        HdrColorPolicy::PreserveSource => {}
        HdrColorPolicy::PreserveHdr10 => {
            args.push("-color_primaries".to_string());
            args.push("bt2020".to_string());
            args.push("-color_trc".to_string());
            args.push("smpte2084".to_string());
            args.push("-colorspace".to_string());
            args.push("bt2020nc".to_string());
        }
        HdrColorPolicy::ToneMapToSdr => {
            args.push("-vf".to_string());
            args.push(
                "zscale=t=linear:npl=100,tonemap=hable,zscale=t=bt709:m=bt709:r=tv,format=yuv420p"
                    .to_string(),
            );
            args.push("-color_primaries".to_string());
            args.push("bt709".to_string());
            args.push("-color_trc".to_string());
            args.push("bt709".to_string());
            args.push("-colorspace".to_string());
            args.push("bt709".to_string());
            args.push("-color_range".to_string());
            args.push("tv".to_string());
        }
    }
}

fn build_intermediate_output_path(output_path: &str, step_index: usize) -> String {
    let filename_start = output_path
        .rfind('/')
        .map_or(0_usize, |index| index.saturating_add(1));
    let filename = &output_path[filename_start..];
    let Some(dot_index) = filename.rfind('.') else {
        return format!("{output_path}.stage{step_index}.tmp");
    };
    let extension = &filename[dot_index + 1..];
    if extension.is_empty() {
        return format!("{output_path}.stage{step_index}.tmp");
    }
    let stem_path = &output_path[..filename_start + dot_index];
    format!("{stem_path}.stage{step_index}.tmp.{extension}")
}

fn verify_output_file(output_path: &str) -> Result<(), ExecuteStepError> {
    let path = Path::new(output_path);
    let metadata = fs::metadata(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            ExecuteStepError::OutputMissing(path.to_path_buf())
        } else {
            ExecuteStepError::Io {
                operation: "execution.verify_output_metadata",
                path: path.to_path_buf(),
                source,
            }
        }
    })?;
    if metadata.len() == 0 {
        return Err(ExecuteStepError::OutputEmpty(path.to_path_buf()));
    }
    Ok(())
}

fn copy_file_to_path(
    source_path: &str,
    destination_path: &str,
    operation: &'static str,
) -> Result<(), ExecuteStepError> {
    let source = Path::new(source_path);
    let destination = Path::new(destination_path);
    ensure_parent_dir(destination, operation)?;
    fs::copy(source, destination)
        .map(|_| ())
        .map_err(|source| ExecuteStepError::Io {
            operation,
            path: destination.to_path_buf(),
            source,
        })
}

fn move_file_to_path(
    source_path: &str,
    destination_path: &str,
    operation: &'static str,
) -> Result<(), ExecuteStepError> {
    let source = Path::new(source_path);
    let destination = Path::new(destination_path);
    ensure_parent_dir(destination, operation)?;
    fs::rename(source, destination).map_err(|source| ExecuteStepError::Io {
        operation,
        path: destination.to_path_buf(),
        source,
    })
}

fn ensure_parent_dir(path: &Path, operation: &'static str) -> Result<(), ExecuteStepError> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|source| ExecuteStepError::Io {
        operation,
        path: parent.to_path_buf(),
        source,
    })
}

const fn is_recovery_step(step: &ExecutionStep) -> bool {
    matches!(step, ExecutionStep::QuarantineFailedOutput { .. })
}

#[cfg(test)]
mod tests {
    use super::{
        AudioStreamConstraints, BuildArgsError, CommandRunner, DesiredGraphBuildContext,
        EXECUTION_TRUNCATION_MARKER, ExecutionControl, ExecutionStep, HdrColorPolicy,
        MAX_EXECUTION_DETAIL_CHARS, MaxBitrateBps, ProcessCommandRunner, SubtitleArtifactPlan,
        VideoStreamConstraints, VideoTranscodeIntent, VideoTranscodePolicy,
        append_video_constraint_args, append_video_quality_args,
        build_desired_graph_execution_steps, build_desired_graph_execution_steps_with_sidecars,
        build_desired_graph_ffmpeg_argv, build_desired_graph_ffmpeg_argv_with_sidecars,
        build_execution_steps, build_execution_steps_with_capabilities,
        build_execution_steps_with_replacement, build_execution_steps_with_replacement_policy,
        build_execution_steps_with_video_policy, build_extract_subtitle_argv, build_ffmpeg_argv,
        build_sidecar_embed_argv, execute_filesystem_step, execute_step, execute_step_sequence,
        validate_container_muxer_capability, validate_declared_stream_codec_capability,
    };
    use crate::capabilities::CapabilitySnapshot;
    use revaer_media_core::model::{
        DesiredGraph, DesiredStreamBinding, MediaGraph, MediaStream, StreamKind,
    };
    use revaer_media_core::plan::{OperationKind, PlannedOperation};
    use revaer_media_core::target::{DesiredSidecarOutput, SidecarEmbedding, SidecarOutputSource};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_execution_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "revaer-media-runtime-execute-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    #[derive(Default)]
    struct RecordingRunner {
        calls: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl super::CommandRunner for RecordingRunner {
        fn run(&self, bin: &str, argv: &[String]) -> Result<(), super::ExecuteStepError> {
            self.calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push((bin.to_string(), argv.to_vec()));
            Ok(())
        }
    }

    struct AtomicExecutionControl {
        requested: Arc<AtomicBool>,
    }

    impl ExecutionControl for AtomicExecutionControl {
        fn cancellation_requested(&self) -> bool {
            self.requested.load(Ordering::Acquire)
        }
    }

    struct ReserveLossControl {
        polls: AtomicU64,
    }

    impl ExecutionControl for ReserveLossControl {
        fn cancellation_requested(&self) -> bool {
            false
        }

        fn limit_breach(&self) -> Option<super::ExecutionLimitBreach> {
            (self.polls.fetch_add(1, Ordering::AcqRel) >= 1)
                .then_some(super::ExecutionLimitBreach::WorkspaceReserveLost)
        }
    }

    #[test]
    fn transcode_requires_stream_id() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: None,
            output_stream_id: Some(0),
        };
        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::MissingStreamId)
        );
    }

    #[test]
    fn remux_uses_copy_codec() {
        let op = PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        };
        let args_result = build_ffmpeg_argv("/in.mkv", "/out.mkv", &op);
        assert!(args_result.is_ok());
        let args = args_result.ok().unwrap_or_default();
        assert!(args.iter().any(|item| item == "copy"));
        assert!(args.windows(2).any(|pair| pair == ["-map_metadata", "0"]));
        assert!(args.windows(2).any(|pair| pair == ["-map_chapters", "0"]));
    }

    #[test]
    fn remux_rejects_ignored_stream_target() {
        let op = PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: Some(1),
            output_stream_id: None,
        };

        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::InvalidOperations(
                "non-stream-scoped operation must not target a stream"
            ))
        );
    }

    #[test]
    fn metadata_rewrite_fails_closed_without_desired_metadata_contract() {
        let op = PlannedOperation {
            kind: OperationKind::MetadataRewrite,
            stream_id: None,
            output_stream_id: None,
        };
        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::UnsupportedMetadataRewrite)
        );
    }

    #[test]
    fn disposition_rewrite_requires_stream_id() {
        let op = PlannedOperation {
            kind: OperationKind::DispositionRewrite,
            stream_id: None,
            output_stream_id: Some(0),
        };
        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::MissingStreamId)
        );
    }

    #[test]
    fn disposition_rewrite_without_desired_graph_fails_closed() {
        let op = PlannedOperation {
            kind: OperationKind::DispositionRewrite,
            stream_id: Some(1),
            output_stream_id: Some(1),
        };
        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::ContextlessStreamRewrite)
        );
    }

    #[test]
    fn label_rewrite_requires_stream_id() {
        let op = PlannedOperation {
            kind: OperationKind::LabelRewrite,
            stream_id: None,
            output_stream_id: Some(0),
        };
        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::MissingStreamId)
        );
    }

    #[test]
    fn label_rewrite_without_desired_graph_fails_closed() {
        let op = PlannedOperation {
            kind: OperationKind::LabelRewrite,
            stream_id: Some(1),
            output_stream_id: Some(1),
        };
        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::ContextlessStreamRewrite)
        );
    }

    #[test]
    fn stream_reorder_without_desired_graph_fails_closed() {
        let op = PlannedOperation {
            kind: OperationKind::StreamReorder,
            stream_id: None,
            output_stream_id: None,
        };
        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::ContextlessStreamRewrite)
        );
    }

    #[test]
    fn transcode_targets_only_selected_input_stream() {
        let op = PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(3),
            output_stream_id: Some(3),
        };
        let args_result = build_ffmpeg_argv("/in.mkv", "/out.mkv", &op);
        assert!(args_result.is_ok());
        let Ok(args) = args_result else {
            return;
        };
        assert!(args.windows(2).any(|pair| pair == ["-map", "0"]));
        assert!(args.windows(2).any(|pair| pair == ["-c", "copy"]));
        assert!(args.windows(2).any(|pair| pair == ["-c:3", "aac"]));
    }

    #[test]
    fn declared_muxer_capability_accepts_normalized_supported_container() {
        let capabilities = CapabilitySnapshot {
            muxers: vec!["matroska".to_string()],
            ..CapabilitySnapshot::default()
        };

        assert!(validate_container_muxer_capability("Matroska", &capabilities).is_ok());
    }

    #[test]
    fn declared_muxer_capability_rejects_missing_container() {
        let capabilities = CapabilitySnapshot {
            muxers: vec!["matroska".to_string()],
            ..CapabilitySnapshot::default()
        };

        assert_eq!(
            validate_container_muxer_capability("mp4", &capabilities),
            Err(BuildArgsError::UnsupportedMuxer("mp4".to_string()))
        );
    }

    #[test]
    fn declared_stream_capability_uses_policy_encoder_fallbacks() {
        let capabilities = CapabilitySnapshot {
            encoders: vec!["hevc_nvenc".to_string()],
            ..CapabilitySnapshot::default()
        };
        let policy = VideoTranscodePolicy {
            intent: VideoTranscodeIntent::Archival,
            ..VideoTranscodePolicy::default()
        };

        assert_eq!(
            validate_declared_stream_codec_capability(
                StreamKind::Video,
                "hevc",
                &capabilities,
                &policy,
            ),
            Err(BuildArgsError::UnsupportedCodec("libx265"))
        );
    }

    #[test]
    fn execution_steps_include_verify_checkpoint() {
        let op = PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        };
        let steps_result = build_execution_steps("/in.mkv", "/out.mkv", &[op]);
        assert!(steps_result.is_ok());
        let Ok(steps) = steps_result else {
            return;
        };
        assert_eq!(steps.len(), 2);
        assert!(matches!(steps[0], ExecutionStep::Command { .. }));
        assert!(matches!(steps[1], ExecutionStep::VerifyOutput { .. }));
    }

    #[test]
    fn execution_steps_reject_container_operation_targeting_stream() {
        let op = PlannedOperation {
            kind: OperationKind::StreamReorder,
            stream_id: Some(2),
            output_stream_id: None,
        };

        assert_eq!(
            build_execution_steps("/in.mkv", "/out.mkv", &[op]),
            Err(BuildArgsError::InvalidOperations(
                "non-stream-scoped operation must not target a stream"
            ))
        );
    }

    #[test]
    fn capability_checked_execution_rejects_missing_required_codec() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
            codec_support: Vec::new(),
            encoders: Vec::new(),
            ..CapabilitySnapshot::default()
        };
        assert_eq!(
            build_execution_steps_with_capabilities("/in.mkv", "/out.mkv", &[op], &capabilities),
            Err(BuildArgsError::UnsupportedCodec("libx265"))
        );
    }

    #[test]
    fn capability_checked_execution_accepts_supported_audio_encoder() {
        let op = PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["aac".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["aac".to_string()],
            ..CapabilitySnapshot::default()
        };
        let steps =
            build_execution_steps_with_capabilities("/in.mkv", "/out.mkv", &[op], &capabilities);
        assert!(steps.is_ok());
    }

    #[test]
    fn video_transcode_applies_quality_guard_flags() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let args_result = build_ffmpeg_argv("/in.mkv", "/out.mkv", &op);
        assert!(args_result.is_ok());
        let Ok(args) = args_result else {
            return;
        };
        assert!(args.windows(2).any(|pair| pair == ["-preset", "medium"]));
        assert!(args.windows(2).any(|pair| pair == ["-crf", "22"]));
    }

    #[test]
    fn capability_checked_execution_accepts_trimmed_case_insensitive_encoder_names() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["  LIBX265  ".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["  LIBX265  ".to_string()],
            ..CapabilitySnapshot::default()
        };
        let steps =
            build_execution_steps_with_capabilities("/in.mkv", "/out.mkv", &[op], &capabilities);
        assert!(steps.is_ok());
    }

    #[test]
    fn multi_operation_execution_is_composed_deterministically() {
        let operations = [
            PlannedOperation {
                kind: OperationKind::Remux,
                stream_id: None,
                output_stream_id: None,
            },
            PlannedOperation {
                kind: OperationKind::AudioTranscode,
                stream_id: Some(1),
                output_stream_id: Some(1),
            },
            PlannedOperation {
                kind: OperationKind::VideoTranscode,
                stream_id: Some(0),
                output_stream_id: Some(0),
            },
        ];
        let result = build_execution_steps("/in.mkv", "/out.mkv", &operations);
        assert!(result.is_ok());
        let Ok(steps) = result else {
            return;
        };
        assert_eq!(steps.len(), 4);
        let ExecutionStep::Command { argv: first, .. } = &steps[0] else {
            panic!("expected first command step");
        };
        let ExecutionStep::Command { argv: second, .. } = &steps[1] else {
            panic!("expected second command step");
        };
        let ExecutionStep::Command { argv: third, .. } = &steps[2] else {
            panic!("expected third command step");
        };
        assert_eq!(
            first,
            &vec![
                "-nostdin",
                "-y",
                "-i",
                "/in.mkv",
                "-map",
                "0",
                "-c",
                "copy",
                "-map_metadata",
                "0",
                "-map_chapters",
                "0",
                "/out.stage0.tmp.mkv",
            ]
        );
        assert_eq!(
            second,
            &vec![
                "-nostdin",
                "-y",
                "-i",
                "/out.stage0.tmp.mkv",
                "-map",
                "0",
                "-c",
                "copy",
                "-c:1",
                "aac",
                "-map_metadata",
                "0",
                "-map_chapters",
                "0",
                "/out.stage1.tmp.mkv",
            ]
        );
        assert_eq!(
            third,
            &vec![
                "-nostdin",
                "-y",
                "-i",
                "/out.stage1.tmp.mkv",
                "-map",
                "0",
                "-c",
                "copy",
                "-c:0",
                "libx265",
                "-preset",
                "medium",
                "-crf",
                "22",
                "-map_metadata",
                "0",
                "-map_chapters",
                "0",
                "/out.mkv",
            ]
        );
    }

    #[test]
    fn replacement_steps_include_optional_backup_verify_and_atomic_replace() {
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }];
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["aac".to_string(), "libx265".to_string()],
            codec_support: Vec::new(),
            encoders: Vec::new(),
            ..CapabilitySnapshot::default()
        };
        let result = build_execution_steps_with_replacement(
            "/input/movie.mkv",
            "/workspace/output/movie.mkv",
            &operations,
            &capabilities,
            Some("/backup/movie.mkv"),
        );
        assert!(result.is_ok());
        let Ok(steps) = result else {
            return;
        };
        assert!(matches!(steps[0], ExecutionStep::BackupSource { .. }));
        assert!(
            steps
                .iter()
                .any(|step| matches!(step, ExecutionStep::VerifyOutput { .. }))
        );
        assert!(matches!(
            steps.last(),
            Some(ExecutionStep::AtomicReplace { .. })
        ));
    }

    #[test]
    fn replacement_policy_steps_include_conditional_quarantine_before_replace() {
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }];
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
            codec_support: Vec::new(),
            encoders: Vec::new(),
            ..CapabilitySnapshot::default()
        };
        let result = build_execution_steps_with_replacement_policy(
            "/input/movie.mkv",
            "/workspace/output/movie.mkv",
            &operations,
            &capabilities,
            Some("/backup/movie.mkv"),
            Some("/quarantine/movie.mkv"),
        );
        assert!(result.is_ok());
        let Ok(steps) = result else {
            return;
        };
        assert!(steps.iter().any(|step| {
            matches!(
                step,
                ExecutionStep::QuarantineFailedOutput {
                    output_path,
                    quarantine_path,
                } if output_path == "/workspace/output/movie.mkv"
                    && quarantine_path == "/quarantine/movie.mkv"
            )
        }));
        assert!(matches!(
            steps.last(),
            Some(ExecutionStep::AtomicReplace { .. })
        ));
    }

    #[test]
    fn filesystem_backup_step_copies_source_to_backup_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_execution_root()?;
        let source_path = root.join("source/movie.mkv");
        let backup_path = root.join("backup/movie.mkv");
        fs::create_dir_all(
            source_path
                .parent()
                .ok_or_else(|| std::io::Error::other("source parent missing"))?,
        )?;
        fs::write(&source_path, b"source-bytes")?;

        execute_filesystem_step(&ExecutionStep::BackupSource {
            source_path: source_path.to_string_lossy().into_owned(),
            backup_path: backup_path.to_string_lossy().into_owned(),
        })?;

        assert_eq!(fs::read(&backup_path)?, b"source-bytes");
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn filesystem_quarantine_step_moves_failed_output_to_quarantine()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_execution_root()?;
        let output_path = root.join("workspace/output/movie.mkv");
        let quarantine_path = root.join("quarantine/movie.mkv");
        fs::create_dir_all(
            output_path
                .parent()
                .ok_or_else(|| std::io::Error::other("output parent missing"))?,
        )?;
        fs::write(&output_path, b"bad-output")?;

        execute_filesystem_step(&ExecutionStep::QuarantineFailedOutput {
            output_path: output_path.to_string_lossy().into_owned(),
            quarantine_path: quarantine_path.to_string_lossy().into_owned(),
        })?;

        assert!(!output_path.exists());
        assert_eq!(fs::read(&quarantine_path)?, b"bad-output");
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn filesystem_sidecar_copy_step_copies_existing_sidecar_to_output()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_execution_root()?;
        let source = root.join("movie.en.srt");
        let output = root.join("workspace").join("movie.en.srt");
        fs::write(&source, "subtitle body")?;

        execute_filesystem_step(&ExecutionStep::CopySidecarSubtitle {
            source_path: source.to_string_lossy().into_owned(),
            output_path: output.to_string_lossy().into_owned(),
        })?;

        assert_eq!(fs::read_to_string(output)?, "subtitle body");
        Ok(())
    }

    #[test]
    fn extract_subtitle_argv_maps_requested_embedded_subtitle_stream() {
        let args = build_extract_subtitle_argv("/input/movie.mkv", "/work/movie.en.srt", 4);

        assert_eq!(
            args,
            vec![
                "-nostdin",
                "-y",
                "-i",
                "/input/movie.mkv",
                "-map",
                "0:4",
                "-c",
                "copy",
                "/work/movie.en.srt"
            ]
        );
    }

    #[test]
    fn embed_sidecar_argv_maps_media_and_sidecar_inputs() {
        let args = build_sidecar_embed_argv(
            "/input/movie.mkv",
            "/input/movie.en.srt",
            "/work/movie.with-subs.mkv",
        );

        assert_eq!(
            args,
            vec![
                "-nostdin",
                "-y",
                "-i",
                "/input/movie.mkv",
                "-i",
                "/input/movie.en.srt",
                "-map",
                "0",
                "-map",
                "1:0",
                "-c",
                "copy",
                "-c:s",
                "copy",
                "-map_metadata",
                "0",
                "-map_chapters",
                "0",
                "/work/movie.with-subs.mkv"
            ]
        );
    }

    #[test]
    fn execute_step_dispatches_command_argv_to_injected_runner()
    -> Result<(), Box<dyn std::error::Error>> {
        let runner = RecordingRunner::default();
        let step = ExecutionStep::Command {
            bin: "ffmpeg".to_string(),
            argv: vec![
                "-nostdin".to_string(),
                "-i".to_string(),
                "/input/movie.mkv".to_string(),
                "/output/movie.mkv".to_string(),
            ],
        };

        execute_step(&step, &runner)?;

        let calls = runner
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "ffmpeg");
        assert_eq!(
            calls[0].1,
            vec!["-nostdin", "-i", "/input/movie.mkv", "/output/movie.mkv"]
        );
        Ok(())
    }

    #[test]
    fn process_command_runner_reports_nonzero_status_with_bounded_stderr() {
        let result = ProcessCommandRunner.run(
            "/bin/sh",
            &[
                "-c".to_string(),
                "printf failure-detail >&2; exit 7".to_string(),
            ],
        );

        assert!(matches!(
            result,
            Err(super::ExecuteStepError::CommandFailed {
                bin,
                status_code: Some(7),
                stderr
            }) if bin == "/bin/sh" && stderr == "failure-detail"
        ));
    }

    #[test]
    fn process_command_runner_truncates_noisy_stderr() {
        let result = ProcessCommandRunner.run(
            "/bin/sh",
            &[
                "-c".to_string(),
                "i=0; while [ \"$i\" -lt 3000 ]; do printf x >&2; i=$((i + 1)); done; exit 9"
                    .to_string(),
            ],
        );

        let Err(super::ExecuteStepError::CommandFailed { stderr, .. }) = result else {
            panic!("expected command failure with bounded stderr");
        };
        assert!(stderr.ends_with(EXECUTION_TRUNCATION_MARKER));
        assert!(stderr.len() <= MAX_EXECUTION_DETAIL_CHARS + EXECUTION_TRUNCATION_MARKER.len());
    }

    #[test]
    fn process_command_runner_terminates_active_child_on_cancellation() {
        let requested = Arc::new(AtomicBool::new(false));
        let request_from_thread = Arc::clone(&requested);
        let request_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            request_from_thread.store(true, Ordering::Release);
        });
        let control = AtomicExecutionControl { requested };
        let started = Instant::now();

        let result =
            ProcessCommandRunner.run_controlled("/bin/sleep", &["30".to_string()], &control);
        let join_result = request_thread.join();

        assert!(join_result.is_ok());
        assert!(matches!(result, Err(super::ExecuteStepError::Cancelled)));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn mid_run_reserve_loss_terminates_and_quarantines_output()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_execution_root()?;
        let output = root.join("output.bin");
        let quarantine = root.join("quarantine/output.bin");
        fs::write(&output, b"partial-output")?;
        let steps = [
            ExecutionStep::Command {
                bin: "/bin/sleep".to_string(),
                argv: vec!["30".to_string()],
            },
            ExecutionStep::QuarantineFailedOutput {
                output_path: output.to_string_lossy().into_owned(),
                quarantine_path: quarantine.to_string_lossy().into_owned(),
            },
        ];
        let control = ReserveLossControl {
            polls: AtomicU64::new(0),
        };

        let result =
            super::execute_step_sequence_controlled(&steps, &ProcessCommandRunner, &control);

        assert!(matches!(
            result,
            Err(super::ExecuteSequenceError {
                failed: super::ExecuteStepError::LimitBreached(
                    super::ExecutionLimitBreach::WorkspaceReserveLost
                ),
                recovery: None,
                ..
            })
        ));
        assert!(!output.exists());
        assert_eq!(fs::read(&quarantine)?, b"partial-output");
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn process_command_runner_terminates_active_child_on_timeout() {
        let control = AtomicExecutionControl {
            requested: Arc::new(AtomicBool::new(false)),
        };
        let started = Instant::now();

        let result = ProcessCommandRunner.run_controlled_with_timeout(
            "/bin/sleep",
            &["30".to_string()],
            &control,
            Duration::from_millis(150),
        );

        assert!(matches!(
            result,
            Err(super::ExecuteStepError::CommandTimedOut {
                bin,
                timeout
            }) if bin == "/bin/sleep" && timeout == Duration::from_millis(150)
        ));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn execute_step_sequence_rejects_unmanaged_atomic_replacement()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_execution_root()?;
        let source = root.join("movie.mkv");
        let output = root.join("workspace").join("movie.mkv");
        let quarantine = root.join("quarantine").join("movie.mkv");
        fs::create_dir_all(output.parent().unwrap_or(root.as_path()))?;
        fs::write(&source, "original")?;
        fs::write(&output, "rewritten")?;
        let steps = vec![
            ExecutionStep::VerifyOutput {
                output_path: output.to_string_lossy().into_owned(),
            },
            ExecutionStep::QuarantineFailedOutput {
                output_path: output.to_string_lossy().into_owned(),
                quarantine_path: quarantine.to_string_lossy().into_owned(),
            },
            ExecutionStep::AtomicReplace {
                source_path: source.to_string_lossy().into_owned(),
                output_path: output.to_string_lossy().into_owned(),
            },
        ];

        let result = execute_step_sequence(&steps, &RecordingRunner::default());

        assert!(matches!(
            result,
            Err(super::ExecuteSequenceError {
                failed_step_index: 2,
                failed: super::ExecuteStepError::ManagedReplacementRequired,
                recovery: None
            })
        ));
        assert_eq!(fs::read_to_string(source)?, "original");
        assert!(!output.exists());
        assert_eq!(fs::read_to_string(quarantine)?, "rewritten");
        Ok(())
    }

    #[test]
    fn execute_step_sequence_quarantines_failed_output_on_error()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_execution_root()?;
        let source = root.join("movie.mkv");
        let output = root.join("workspace").join("movie.mkv");
        let quarantine = root.join("quarantine").join("movie.mkv");
        fs::create_dir_all(output.parent().unwrap_or(root.as_path()))?;
        fs::write(&source, "original")?;
        fs::write(&output, "")?;
        let steps = vec![
            ExecutionStep::VerifyOutput {
                output_path: output.to_string_lossy().into_owned(),
            },
            ExecutionStep::QuarantineFailedOutput {
                output_path: output.to_string_lossy().into_owned(),
                quarantine_path: quarantine.to_string_lossy().into_owned(),
            },
            ExecutionStep::AtomicReplace {
                source_path: source.to_string_lossy().into_owned(),
                output_path: output.to_string_lossy().into_owned(),
            },
        ];

        let result = execute_step_sequence(&steps, &RecordingRunner::default());

        assert!(matches!(
            result,
            Err(super::ExecuteSequenceError {
                failed_step_index: 0,
                failed: super::ExecuteStepError::OutputEmpty(_),
                recovery: None
            })
        ));
        assert_eq!(fs::read_to_string(source)?, "original");
        assert!(!output.exists());
        assert!(quarantine.exists());
        Ok(())
    }

    #[test]
    fn empty_operation_list_is_rejected() {
        let result = build_execution_steps("/in.mkv", "/out.mkv", &[]);
        assert_eq!(result, Err(BuildArgsError::EmptyOperations));
    }

    #[test]
    fn capability_checked_execution_rejects_missing_audio_codec() {
        let op = PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
            codec_support: Vec::new(),
            encoders: Vec::new(),
            ..CapabilitySnapshot::default()
        };
        assert_eq!(
            build_execution_steps_with_capabilities("/in.mkv", "/out.mkv", &[op], &capabilities),
            Err(BuildArgsError::UnsupportedCodec("aac"))
        );
    }

    #[test]
    fn desired_graph_rejects_missing_concrete_audio_encoder() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "opus".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["aac".to_string(), "opus".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["aac".to_string()],
            ..CapabilitySnapshot::default()
        };
        let operations = [PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        }];

        assert_eq!(
            build_desired_graph_execution_steps(
                "/in.mkv",
                "/out.mkv",
                &source,
                &desired,
                &operations,
                Some(&capabilities),
                VideoTranscodePolicy::default(),
            ),
            Err(BuildArgsError::UnsupportedCodec("libopus"))
        );
    }

    #[test]
    fn desired_graph_uses_canonical_muxer_before_extension_preserving_output_path() {
        let source_stream = MediaStream {
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
            source_path: "/in.mp4".to_string(),
            container_formats: vec!["mp4".to_string()],
            streams: vec![source_stream.clone()],
        };
        let desired = DesiredGraph {
            output_path: "/workspace/in.mp4".to_string(),
            container_format: Some("mkv".to_string()),
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![source_stream],
        };
        let capabilities = CapabilitySnapshot {
            muxers: vec!["matroska".to_string()],
            ..CapabilitySnapshot::default()
        };
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }];

        let argv_result = build_desired_graph_ffmpeg_argv(
            "/in.mp4",
            "/workspace/in.mp4",
            &source,
            &desired,
            &operations,
            Some(&capabilities),
            VideoTranscodePolicy::default(),
        );

        assert!(argv_result.is_ok());
        let Ok(argv) = argv_result else {
            return;
        };
        assert!(argv.windows(2).any(|pair| pair == ["-map_metadata", "0"]));
        assert!(argv.windows(2).any(|pair| pair == ["-map_chapters", "0"]));
        assert_eq!(
            argv.get(argv.len().saturating_sub(7)..),
            Some(
                [
                    "-f",
                    "matroska",
                    "-map_metadata",
                    "0",
                    "-map_chapters",
                    "0",
                    "/workspace/in.mp4",
                ]
                .map(String::from)
                .as_slice()
            )
        );
    }

    #[test]
    fn desired_graph_strip_container_metadata_emits_metadata_removal_args() {
        let source_stream = MediaStream {
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
            source_path: "/in.mkv".to_string(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![source_stream.clone()],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: Some("strip".to_string()),
            streams: vec![source_stream],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::MetadataRewrite,
            stream_id: None,
            output_stream_id: None,
        }];

        let argv = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        )
        .unwrap_or_default();

        assert!(argv.windows(2).any(|pair| pair == ["-map_metadata", "-1"]));
        assert!(argv.windows(2).any(|pair| pair == ["-map_chapters", "0"]));
    }

    #[test]
    fn desired_graph_rejects_unknown_container_metadata_policy() {
        let source_stream = MediaStream {
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
            source_path: "/in.mkv".to_string(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![source_stream.clone()],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: Some("rewrite".to_string()),
            streams: vec![source_stream],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::MetadataRewrite,
            stream_id: None,
            output_stream_id: None,
        }];

        assert_eq!(
            build_desired_graph_ffmpeg_argv(
                "/in.mkv",
                "/out.mkv",
                &source,
                &desired,
                &operations,
                None,
                VideoTranscodePolicy::default(),
            ),
            Err(BuildArgsError::UnsupportedContainerMetadataPolicy(
                "rewrite".to_string()
            ))
        );
    }

    #[test]
    fn desired_graph_metadata_rewrite_operations_emit_target_stream_tags() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: Some("jpn".to_string()),
                title: Some("Source Main".to_string()),
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 1,
                source_stream_id: Some(1),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: Some("eng".to_string()),
                title: Some("English Main".to_string()),
                dispositions: vec!["default".to_string(), "forced".to_string()],
            }],
        };
        let operations = [
            PlannedOperation {
                kind: OperationKind::LabelRewrite,
                stream_id: Some(1),
                output_stream_id: Some(1),
            },
            PlannedOperation {
                kind: OperationKind::DispositionRewrite,
                stream_id: Some(1),
                output_stream_id: Some(1),
            },
        ];

        let argv_result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert!(argv_result.is_ok());
        let Ok(argv) = argv_result else {
            return;
        };
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["-metadata:s:0", "language=eng"])
        );
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["-metadata:s:0", "title=English Main"])
        );
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["-disposition:0", "default+forced"])
        );
    }

    #[test]
    fn desired_graph_rejects_unsupported_muxer() {
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
            source_path: "/in.mkv".to_string(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream.clone()],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: Some("mp4".to_string()),
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![stream],
        };
        let capabilities = CapabilitySnapshot {
            muxers: vec!["matroska".to_string()],
            ..CapabilitySnapshot::default()
        };
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }];

        assert_eq!(
            build_desired_graph_ffmpeg_argv(
                "/in.mkv",
                "/out.mkv",
                &source,
                &desired,
                &operations,
                Some(&capabilities),
                VideoTranscodePolicy::default(),
            ),
            Err(BuildArgsError::UnsupportedMuxer("mp4".to_string()))
        );
    }

    #[test]
    fn desired_graph_allows_exact_attachment_passthrough() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
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
                    stream_id: 2,
                    kind: StreamKind::Attachment,
                    codec: "ttf".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: Some("font".to_string()),
                    dispositions: Vec::new(),
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            stream_bindings: vec![
                DesiredStreamBinding {
                    output_stream_id: 0,
                    source_stream_id: Some(0),
                },
                DesiredStreamBinding {
                    output_stream_id: 2,
                    source_stream_id: Some(2),
                },
            ],
            container_metadata_policy: None,
            streams: source.streams.clone(),
        };
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }];

        let argv_result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );
        assert!(
            argv_result.is_ok(),
            "exact attachment passthrough should build: {argv_result:?}"
        );
        let Ok(argv) = argv_result else {
            return;
        };

        assert!(argv.windows(2).any(|pair| pair == ["-map", "0:2"]));
        assert!(argv.windows(2).any(|pair| pair == ["-c:1", "copy"]));
        assert!(!argv.iter().any(|value| value == "-metadata:s:1"));
        assert!(!argv.iter().any(|value| value == "-disposition:1"));
    }

    #[test]
    fn desired_graph_allows_exact_data_passthrough() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
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
                    stream_id: 3,
                    kind: StreamKind::Data,
                    codec: "bin_data".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("timecode".to_string()),
                    dispositions: vec!["default".to_string()],
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            stream_bindings: vec![
                DesiredStreamBinding {
                    output_stream_id: 0,
                    source_stream_id: Some(0),
                },
                DesiredStreamBinding {
                    output_stream_id: 3,
                    source_stream_id: Some(3),
                },
            ],
            container_metadata_policy: None,
            streams: source.streams.clone(),
        };
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }];

        let argv_result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );
        assert!(
            argv_result.is_ok(),
            "exact data passthrough should build: {argv_result:?}"
        );
        let Ok(argv) = argv_result else {
            return;
        };

        assert!(argv.windows(2).any(|pair| pair == ["-map", "0:3"]));
        assert!(argv.windows(2).any(|pair| pair == ["-c:1", "copy"]));
        assert!(!argv.iter().any(|value| value == "-metadata:s:1"));
        assert!(!argv.iter().any(|value| value == "-disposition:1"));
    }

    #[test]
    fn desired_graph_rejects_attachment_mutation_for_command_materialization() {
        let attachment = MediaStream {
            stream_id: 2,
            kind: StreamKind::Attachment,
            codec: "ttf".to_string(),
            channels: None,
            channel_layout: None,
            language: None,
            title: Some("font".to_string()),
            dispositions: Vec::new(),
        };
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![attachment.clone()],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 2,
                source_stream_id: Some(2),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                title: Some("renamed".to_string()),
                ..attachment
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }];

        assert_eq!(
            build_desired_graph_ffmpeg_argv(
                "/in.mkv",
                "/out.mkv",
                &source,
                &desired,
                &operations,
                None,
                VideoTranscodePolicy::default(),
            ),
            Err(BuildArgsError::UnsupportedDesiredStreamKind { stream_id: 2 })
        );
    }

    #[test]
    fn desired_graph_rejects_data_mutation_for_command_materialization() {
        let data_stream = MediaStream {
            stream_id: 3,
            kind: StreamKind::Data,
            codec: "bin_data".to_string(),
            channels: None,
            channel_layout: None,
            language: Some("eng".to_string()),
            title: Some("timecode".to_string()),
            dispositions: Vec::new(),
        };
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![data_stream.clone()],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 3,
                source_stream_id: Some(3),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                title: Some("renamed".to_string()),
                ..data_stream
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }];

        assert_eq!(
            build_desired_graph_ffmpeg_argv(
                "/in.mkv",
                "/out.mkv",
                &source,
                &desired,
                &operations,
                None,
                VideoTranscodePolicy::default(),
            ),
            Err(BuildArgsError::UnsupportedDesiredStreamKind { stream_id: 3 })
        );
    }

    #[test]
    fn desired_graph_rejects_chapter_streams_for_command_materialization() {
        let stream = MediaStream {
            stream_id: 2,
            kind: StreamKind::Chapter,
            codec: "bin_data".to_string(),
            channels: None,
            channel_layout: None,
            language: None,
            title: None,
            dispositions: Vec::new(),
        };
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![stream.clone()],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 2,
                source_stream_id: Some(2),
            }],
            container_metadata_policy: None,
            streams: vec![stream],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }];

        assert_eq!(
            build_desired_graph_ffmpeg_argv(
                "/in.mkv",
                "/out.mkv",
                &source,
                &desired,
                &operations,
                None,
                VideoTranscodePolicy::default(),
            ),
            Err(BuildArgsError::UnsupportedDesiredStreamKind { stream_id: 2 })
        );
    }

    #[test]
    fn desired_graph_rejects_duplicate_desired_stream_ids() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![
                DesiredStreamBinding {
                    output_stream_id: 0,
                    source_stream_id: Some(0),
                },
                DesiredStreamBinding {
                    output_stream_id: 0,
                    source_stream_id: Some(0),
                },
            ],
            container_metadata_policy: None,
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Audio,
                    codec: "opus".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        }];

        let result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(
            result.err().map(|err| err.to_string()),
            Some("desired graph contains duplicate stream ids".to_string())
        );
    }

    #[test]
    fn desired_graph_rejects_duplicate_source_stream_ids() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
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
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        }];

        let result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(
            result.err().map(|err| err.to_string()),
            Some("source graph contains duplicate stream ids".to_string())
        );
    }

    #[test]
    fn desired_graph_rejects_source_stream_kind_mismatch() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        }];

        let result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(result, Err(BuildArgsError::DesiredStreamKindMismatch(0)));
    }

    #[test]
    fn desired_graph_rejects_transcode_operation_missing_stream_id() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "opus".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: None,
            output_stream_id: Some(0),
        }];

        let result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(result, Err(BuildArgsError::MissingStreamId));
    }

    #[test]
    fn desired_graph_rejects_noop_mixed_with_mutating_operation() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "opus".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [
            PlannedOperation {
                kind: OperationKind::NoOp,
                stream_id: None,
                output_stream_id: None,
            },
            PlannedOperation {
                kind: OperationKind::AudioTranscode,
                stream_id: Some(0),
                output_stream_id: Some(0),
            },
        ];

        let result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(
            result,
            Err(BuildArgsError::InvalidOperations(
                "no-op operation must not be combined with mutating operations"
            ))
        );
    }

    #[test]
    fn desired_graph_noop_rejects_duplicate_desired_stream_ids() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![
                DesiredStreamBinding {
                    output_stream_id: 0,
                    source_stream_id: Some(0),
                },
                DesiredStreamBinding {
                    output_stream_id: 0,
                    source_stream_id: Some(0),
                },
            ],
            container_metadata_policy: None,
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
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
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
            output_stream_id: None,
        }];

        let result = build_desired_graph_execution_steps(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(
            result.err().map(|err| err.to_string()),
            Some("desired graph contains duplicate stream ids".to_string())
        );
    }

    #[test]
    fn desired_graph_noop_rejects_duplicate_source_stream_ids() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
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
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
            output_stream_id: None,
        }];

        let result = build_desired_graph_execution_steps(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(
            result.err().map(|err| err.to_string()),
            Some("source graph contains duplicate stream ids".to_string())
        );
    }

    #[test]
    fn desired_graph_noop_rejects_missing_source_stream() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 9,
                source_stream_id: Some(9),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 9,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
            output_stream_id: None,
        }];

        let result = build_desired_graph_execution_steps(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(result, Err(BuildArgsError::DesiredStreamMissing(9)));
    }

    #[test]
    fn desired_graph_noop_rejects_source_stream_kind_mismatch() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
            output_stream_id: None,
        }];

        let result = build_desired_graph_execution_steps(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(result, Err(BuildArgsError::DesiredStreamKindMismatch(0)));
    }

    #[test]
    fn desired_graph_execution_steps_reject_noop_targeting_stream() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: Some(0),
            output_stream_id: None,
        }];

        let result = build_desired_graph_execution_steps(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );

        assert_eq!(
            result,
            Err(BuildArgsError::InvalidOperations(
                "no-op operation must not target a stream"
            ))
        );
    }

    #[test]
    fn desired_graph_audio_channel_shape_uses_audio_encoder_and_shape_args() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(6),
                channel_layout: Some("5.1(side)".to_string()),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        }];

        let argv_result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            VideoTranscodePolicy::default(),
        );
        assert!(argv_result.is_ok());
        let Ok(argv) = argv_result else {
            return;
        };

        assert!(argv.windows(2).any(|pair| pair == ["-c:0", "aac"]));
        assert!(!argv.iter().any(|argument| argument == "-ac:0"));
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["-channel_layout:a:0", "stereo"])
        );
    }

    #[test]
    fn desired_graph_audio_policy_filters_force_audio_encoder() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = [PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        }];
        let policy = VideoTranscodePolicy {
            audio_stream_constraints: vec![AudioStreamConstraints {
                stream_id: 0,
                bitrate_bps: Some(160_000),
                sample_rate_hz: Some(48_000),
                loudness_profile: Some("dialog-normalized".to_string()),
                dynamic_range: Some("speech".to_string()),
            }],
            ..VideoTranscodePolicy::default()
        };

        let argv_result = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            None,
            policy,
        );
        assert!(argv_result.is_ok());
        let Ok(argv) = argv_result else {
            return;
        };

        assert!(argv.windows(2).any(|pair| pair == ["-c:0", "aac"]));
        assert!(argv.windows(2).any(|pair| pair == ["-b:a:0", "160000"]));
        assert!(argv.windows(2).any(|pair| pair == ["-ar:0", "48000"]));
        assert!(argv.windows(2).any(|pair| {
            pair == [
                "-filter:a:0",
                "loudnorm=I=-16:TP=-1.5:LRA=11,acompressor=threshold=-18dB:ratio=2:attack=20:release=250",
            ]
        }));
    }

    #[test]
    fn desired_graph_rejects_missing_concrete_video_encoder() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
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
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(0),
            }],
            container_metadata_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "av1".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string(), "av1".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["libx265".to_string()],
            ..CapabilitySnapshot::default()
        };
        let operations = [PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        }];

        assert_eq!(
            build_desired_graph_execution_steps(
                "/in.mkv",
                "/out.mkv",
                &source,
                &desired,
                &operations,
                Some(&capabilities),
                VideoTranscodePolicy::default(),
            ),
            Err(BuildArgsError::UnsupportedCodec("libaom-av1"))
        );
    }

    #[test]
    fn desired_graph_video_constraints_apply_to_transcoded_output_stream() {
        let source = MediaGraph {
            source_path: "/in.mkv".to_string(),
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
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/out.mkv".to_string(),
            container_format: None,
            stream_bindings: vec![
                DesiredStreamBinding {
                    output_stream_id: 0,
                    source_stream_id: Some(0),
                },
                DesiredStreamBinding {
                    output_stream_id: 1,
                    source_stream_id: Some(1),
                },
            ],
            container_metadata_policy: None,
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "hevc".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                source.streams[1].clone(),
            ],
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["libx265".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["libx265".to_string()],
            ..CapabilitySnapshot::default()
        };
        let operations = [PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        }];
        let policy = VideoTranscodePolicy {
            stream_constraints: vec![VideoStreamConstraints {
                stream_id: 0,
                profile: Some("main10".to_string()),
                level: Some("5.1".to_string()),
                max_bitrate_bps: MaxBitrateBps::new(8_000_000),
                color_primaries: Some("bt2020".to_string()),
                color_transfer: Some("smpte2084".to_string()),
                color_space: Some("bt2020nc".to_string()),
                hdr_format: Some("hdr10".to_string()),
            }],
            ..VideoTranscodePolicy::default()
        };

        let argv = build_desired_graph_ffmpeg_argv(
            "/in.mkv",
            "/out.mkv",
            &source,
            &desired,
            &operations,
            Some(&capabilities),
            policy,
        )
        .expect("desired graph argv");

        let has_arg = |name: &str, value: &str| argv.windows(2).any(|pair| pair == [name, value]);
        assert!(has_arg("-profile:0", "main10"));
        assert!(has_arg("-level:0", "5.1"));
        assert!(has_arg("-b:0", "7600000"));
        assert!(has_arg("-maxrate:0", "8000000"));
        assert!(has_arg("-bufsize:0", "16000000"));
        assert!(has_arg("-color_primaries:0", "bt2020"));
        assert!(has_arg("-color_trc:0", "smpte2084"));
        assert!(has_arg("-colorspace:0", "bt2020nc"));
    }

    #[test]
    fn software_and_hardware_encoders_receive_peak_and_vbv_controls() {
        let constraints = VideoStreamConstraints {
            stream_id: 0,
            profile: None,
            level: None,
            max_bitrate_bps: MaxBitrateBps::new(8_000_000),
            color_primaries: None,
            color_transfer: None,
            color_space: None,
            hdr_format: None,
        };
        for encoder in ["libx265", "hevc_nvenc", "hevc_qsv", "hevc_vaapi"] {
            let mut argv = Vec::new();
            append_video_quality_args(&mut argv, encoder);
            append_video_constraint_args(&mut argv, 0, Some(&constraints));
            let has_arg =
                |name: &str, value: &str| argv.windows(2).any(|pair| pair == [name, value]);
            assert!(has_arg("-b:0", "7600000"), "encoder={encoder}");
            assert!(has_arg("-maxrate:0", "8000000"), "encoder={encoder}");
            assert!(has_arg("-bufsize:0", "16000000"), "encoder={encoder}");
        }
    }

    #[test]
    fn capability_checked_execution_accepts_explicit_encoder_support() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["libx265".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["libx265".to_string()],
            ..CapabilitySnapshot::default()
        };
        let result =
            build_execution_steps_with_capabilities("/in.mkv", "/out.mkv", &[op], &capabilities);
        assert!(result.is_ok());
    }

    #[test]
    fn capability_checked_execution_rejects_codec_without_encoder_support() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["hevc".to_string()],
            codec_support: Vec::new(),
            encoders: Vec::new(),
            ..CapabilitySnapshot::default()
        };
        let result =
            build_execution_steps_with_capabilities("/in.mkv", "/out.mkv", &[op], &capabilities);
        assert_eq!(result, Err(BuildArgsError::UnsupportedCodec("libx265")));
    }

    #[test]
    fn capability_checked_execution_prefers_hardware_encoder_fallback() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["hevc_nvenc".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["hevc_nvenc".to_string()],
            ..CapabilitySnapshot::default()
        };
        let result =
            build_execution_steps_with_capabilities("/in.mkv", "/out.mkv", &[op], &capabilities);
        assert!(result.is_ok());
        let Ok(steps) = result else {
            return;
        };
        let Some(ExecutionStep::Command { argv, .. }) = steps.first() else {
            return;
        };
        assert!(argv.iter().any(|item| item == "hevc_nvenc"));
    }

    #[test]
    fn video_policy_prefers_hardware_encoder_for_general_intent() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["libx265".to_string(), "hevc_nvenc".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["libx265".to_string(), "hevc_nvenc".to_string()],
            ..CapabilitySnapshot::default()
        };

        let result = build_execution_steps_with_video_policy(
            "/in.mkv",
            "/out.mkv",
            &[op],
            &capabilities,
            &VideoTranscodePolicy::default(),
        );

        assert!(result.is_ok());
        let Ok(steps) = result else {
            return;
        };
        let Some(ExecutionStep::Command { argv, .. }) = steps.first() else {
            return;
        };
        assert!(argv.iter().any(|item| item == "hevc_nvenc"));
        assert!(argv.windows(2).any(|pair| pair == ["-cq", "22"]));
        assert!(!argv.windows(2).any(|pair| pair == ["-crf", "22"]));
    }

    #[test]
    fn anime_video_policy_rejects_hardware_only_encoder() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["hevc_nvenc".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["hevc_nvenc".to_string()],
            ..CapabilitySnapshot::default()
        };
        let policy = VideoTranscodePolicy {
            intent: VideoTranscodeIntent::Anime,
            hdr_color: HdrColorPolicy::PreserveSource,
            ..VideoTranscodePolicy::default()
        };

        assert_eq!(
            build_execution_steps_with_video_policy(
                "/in.mkv",
                "/out.mkv",
                &[op],
                &capabilities,
                &policy,
            ),
            Err(BuildArgsError::UnsupportedCodec("libx265"))
        );
    }

    #[test]
    fn hdr10_preservation_policy_adds_color_metadata() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["libx265".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["libx265".to_string()],
            ..CapabilitySnapshot::default()
        };
        let policy = VideoTranscodePolicy {
            intent: VideoTranscodeIntent::General,
            hdr_color: HdrColorPolicy::PreserveHdr10,
            ..VideoTranscodePolicy::default()
        };

        let result = build_execution_steps_with_video_policy(
            "/in.mkv",
            "/out.mkv",
            &[op],
            &capabilities,
            &policy,
        );

        assert!(result.is_ok());
        let Ok(steps) = result else {
            return;
        };
        let Some(ExecutionStep::Command { argv, .. }) = steps.first() else {
            return;
        };
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["-color_primaries", "bt2020"])
        );
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["-color_trc", "smpte2084"])
        );
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["-colorspace", "bt2020nc"])
        );
    }

    #[test]
    fn hdr_to_sdr_policy_adds_tonemap_filter_and_sdr_range() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
            output_stream_id: Some(0),
        };
        let capabilities = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["libx265".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["libx265".to_string()],
            ..CapabilitySnapshot::default()
        };
        let policy = VideoTranscodePolicy {
            intent: VideoTranscodeIntent::General,
            hdr_color: HdrColorPolicy::ToneMapToSdr,
            ..VideoTranscodePolicy::default()
        };

        let result = build_execution_steps_with_video_policy(
            "/in.mkv",
            "/out.mkv",
            &[op],
            &capabilities,
            &policy,
        );

        assert!(result.is_ok());
        let Ok(steps) = result else {
            return;
        };
        let Some(ExecutionStep::Command { argv, .. }) = steps.first() else {
            return;
        };
        assert!(
            argv.windows(2)
                .any(|pair| pair[0] == "-vf" && pair[1].contains("tonemap=hable"))
        );
        assert!(argv.windows(2).any(|pair| pair == ["-color_range", "tv"]));
    }

    #[test]
    fn intermediate_outputs_preserve_requested_extension() {
        let operations = [
            PlannedOperation {
                kind: OperationKind::Remux,
                stream_id: None,
                output_stream_id: None,
            },
            PlannedOperation {
                kind: OperationKind::AudioTranscode,
                stream_id: Some(1),
                output_stream_id: Some(1),
            },
        ];
        let result = build_execution_steps("/in.mkv", "/out.mp4", &operations);
        assert!(result.is_ok());
        let Ok(steps) = result else {
            return;
        };
        let ExecutionStep::Command { argv: first, .. } = &steps[0] else {
            return;
        };
        assert_eq!(
            first.last().map(String::as_str),
            Some("/out.stage0.tmp.mp4")
        );
    }

    #[test]
    fn sidecar_aware_argv_binds_external_subtitle_stream() {
        let video = MediaStream {
            stream_id: 0,
            kind: StreamKind::Video,
            codec: "h264".to_string(),
            channels: None,
            channel_layout: None,
            language: None,
            title: None,
            dispositions: Vec::new(),
        };
        let subtitle = MediaStream {
            stream_id: 1,
            kind: StreamKind::Subtitle,
            codec: "srt".to_string(),
            channels: None,
            channel_layout: None,
            language: Some("eng".to_string()),
            title: None,
            dispositions: vec!["forced".to_string()],
        };
        let source = MediaGraph {
            source_path: "/library/movie.mkv".to_string(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![video.clone()],
        };
        let desired = DesiredGraph {
            output_path: "/workspace/movie.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            stream_bindings: vec![
                DesiredStreamBinding {
                    output_stream_id: 0,
                    source_stream_id: Some(0),
                },
                DesiredStreamBinding {
                    output_stream_id: 1,
                    source_stream_id: None,
                },
            ],
            container_metadata_policy: None,
            streams: vec![video, subtitle.clone()],
        };
        let embeddings = [SidecarEmbedding {
            path: "/library/movie.eng.forced.srt".to_string(),
            companion_path: None,
            output_stream: subtitle,
            source_codec: "subrip".to_string(),
        }];
        let operations = [PlannedOperation {
            kind: OperationKind::EmbedSubtitle,
            stream_id: None,
            output_stream_id: Some(1),
        }];

        let argv = build_desired_graph_ffmpeg_argv_with_sidecars(
            DesiredGraphBuildContext {
                input_path: "/library/movie.mkv",
                output_path: "/workspace/movie.mkv",
                source: &source,
                desired: &desired,
                operations: &operations,
                capabilities: None,
                policy: VideoTranscodePolicy::default(),
            },
            &embeddings,
        );
        assert!(argv.is_ok());
        let Ok(argv) = argv else {
            return;
        };
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["-i", "/library/movie.eng.forced.srt"])
        );
        assert!(argv.windows(2).any(|pair| pair == ["-map", "1:0"]));
        assert!(argv.windows(2).any(|pair| pair == ["-c:1", "copy"]));
        assert!(argv.windows(2).any(|pair| pair == ["-map_metadata", "0"]));
        assert!(argv.windows(2).any(|pair| pair == ["-map_chapters", "0"]));
    }

    #[test]
    fn sidecar_aware_steps_materialize_existing_sidecar_output() {
        let source = MediaGraph {
            source_path: "/library/movie.mkv".to_string(),
            container_formats: vec!["matroska".to_string()],
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
        let outputs = [DesiredSidecarOutput {
            path: "/workspace/movie.eng.forced.srt".to_string(),
            companion_path: None,
            destination_path: "/library/movie.eng.forced.srt".to_string(),
            destination_companion_path: None,
            source: SidecarOutputSource::ExistingSidecar {
                path: "/incoming/movie.eng.forced.srt".to_string(),
                companion_path: None,
                codec: "subrip".to_string(),
            },
            codec: "srt".to_string(),
        }];
        let output_operations = [PlannedOperation {
            kind: OperationKind::CopySidecarSubtitle,
            stream_id: None,
            output_stream_id: None,
        }];
        let steps = build_desired_graph_execution_steps_with_sidecars(
            DesiredGraphBuildContext {
                input_path: "/library/movie.mkv",
                output_path: "/workspace/movie.mkv",
                source: &source,
                desired: &DesiredGraph {
                    output_path: "/workspace/movie.mkv".to_string(),
                    container_format: Some("matroska".to_string()),
                    stream_bindings: vec![DesiredStreamBinding {
                        output_stream_id: 0,
                        source_stream_id: Some(0),
                    }],
                    container_metadata_policy: None,
                    streams: source.streams.clone(),
                },
                operations: &output_operations,
                capabilities: None,
                policy: VideoTranscodePolicy::default(),
            },
            SubtitleArtifactPlan {
                embeddings: &[],
                outputs: &outputs,
                removals: &[],
            },
        );
        assert!(steps.is_ok());
        let Ok(steps) = steps else {
            return;
        };
        assert!(steps.iter().any(|step| matches!(
            step,
            ExecutionStep::CopySidecarSubtitle { source_path, output_path }
                if source_path == "/incoming/movie.eng.forced.srt"
                    && output_path == "/workspace/movie.eng.forced.srt"
        )));
    }

    #[test]
    fn execution_step_audits_map_fused_and_non_command_operations() {
        let operations = vec![
            PlannedOperation {
                kind: OperationKind::VideoTranscode,
                stream_id: Some(0),
                output_stream_id: Some(0),
            },
            PlannedOperation {
                kind: OperationKind::MetadataRewrite,
                stream_id: None,
                output_stream_id: None,
            },
            PlannedOperation {
                kind: OperationKind::CopySidecarSubtitle,
                stream_id: None,
                output_stream_id: None,
            },
            PlannedOperation {
                kind: OperationKind::RemoveSidecarSubtitle,
                stream_id: None,
                output_stream_id: None,
            },
        ];
        let steps = vec![
            ExecutionStep::Command {
                bin: "ffmpeg".to_string(),
                argv: vec!["-i".to_string(), "source.mkv".to_string()],
            },
            ExecutionStep::CopySidecarSubtitle {
                source_path: "source.srt".to_string(),
                output_path: "output.srt".to_string(),
            },
            ExecutionStep::AtomicReplace {
                source_path: "source.mkv".to_string(),
                output_path: "output.mkv".to_string(),
            },
        ];

        let audits = super::compile_execution_step_audits(&operations, &steps);
        assert_eq!(audits[0].step_id, "step-0000-command");
        assert_eq!(audits[0].operation_indices, vec![0, 1]);
        assert_eq!(audits[1].operation_indices, vec![2]);
        assert_eq!(audits[2].operation_indices, vec![3]);
    }
}
