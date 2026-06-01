//! Command argument builders.

use crate::capabilities::CapabilitySnapshot;
use revaer_media_core::plan::{OperationKind, PlannedOperation};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

/// Build error for command arguments.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BuildArgsError {
    /// Stream id required but absent.
    #[error("stream id required for transcode operation")]
    MissingStreamId,
    /// Required codec is not supported by runtime capabilities.
    #[error("required codec is not supported: {0}")]
    UnsupportedCodec(&'static str),
    /// No operations were provided for execution planning.
    #[error("at least one operation is required")]
    EmptyOperations,
}

/// Filesystem execution error for non-command execution steps.
#[derive(Debug, Error)]
pub enum ExecuteStepError {
    /// Command steps require an injected command runner.
    #[error("command steps require an injected command runner")]
    CommandStepUnsupported,
    /// Verified output path does not exist.
    #[error("verified output is missing: {0}")]
    OutputMissing(PathBuf),
    /// Verified output path is empty.
    #[error("verified output is empty: {0}")]
    OutputEmpty(PathBuf),
    /// Command exited unsuccessfully.
    #[error("command {bin} exited unsuccessfully with status {status_code:?}")]
    CommandFailed {
        /// Binary that exited unsuccessfully.
        bin: String,
        /// Process exit status code when available.
        status_code: Option<i32>,
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
}

/// Process-backed command runner for execution workers.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessCommandRunner;

impl CommandRunner for ProcessCommandRunner {
    fn run(&self, bin: &str, argv: &[String]) -> Result<(), ExecuteStepError> {
        let status =
            Command::new(bin)
                .args(argv)
                .status()
                .map_err(|source| ExecuteStepError::Io {
                    operation: "execution.command_spawn",
                    path: PathBuf::from(bin),
                    source,
                })?;
        if status.success() {
            return Ok(());
        }
        Err(ExecuteStepError::CommandFailed {
            bin: bin.to_string(),
            status_code: status.code(),
        })
    }
}

const DEFAULT_VIDEO_ENCODER: &str = "libx265";
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

/// Video transcode policy applied to execution argv construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoTranscodePolicy {
    /// Intent-specific safeguards.
    pub intent: VideoTranscodeIntent,
    /// HDR/color policy.
    pub hdr_color: HdrColorPolicy,
}

impl Default for VideoTranscodePolicy {
    fn default() -> Self {
        Self {
            intent: VideoTranscodeIntent::General,
            hdr_color: HdrColorPolicy::PreserveSource,
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
    vec![
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
        output_path.to_string(),
    ]
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
        VideoTranscodePolicy::default(),
    )
}

fn build_ffmpeg_argv_with_video_policy(
    input_path: &str,
    output_path: &str,
    operation: &PlannedOperation,
    video_encoder: &str,
    policy: VideoTranscodePolicy,
) -> Result<Vec<String>, BuildArgsError> {
    let mut args = vec![
        "-nostdin".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string(),
    ];

    match operation.kind {
        OperationKind::Remux => {
            args.push("-map".to_string());
            args.push("0".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
        }
        OperationKind::MetadataRewrite => {
            args.push("-map".to_string());
            args.push("0".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
            args.push("-map_metadata".to_string());
            args.push("-1".to_string());
        }
        OperationKind::DispositionRewrite => {
            let stream_id = operation.stream_id.ok_or(BuildArgsError::MissingStreamId)?;
            args.push("-map".to_string());
            args.push("0".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
            args.push(format!("-disposition:{stream_id}"));
            args.push("0".to_string());
        }
        OperationKind::LabelRewrite => {
            let stream_id = operation.stream_id.ok_or(BuildArgsError::MissingStreamId)?;
            args.push("-map".to_string());
            args.push("0".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
            args.push(format!("-metadata:s:{stream_id}"));
            args.push("title=".to_string());
        }
        OperationKind::StreamReorder => {
            args.push("-map".to_string());
            args.push("0:v?".to_string());
            args.push("-map".to_string());
            args.push("0:a?".to_string());
            args.push("-map".to_string());
            args.push("0:s?".to_string());
            args.push("-map".to_string());
            args.push("0:t?".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
        }
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
    }

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
    if operations.is_empty() {
        return Err(BuildArgsError::EmptyOperations);
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
    build_execution_steps_with_replacement_video_policy(
        source_path,
        output_path,
        operations,
        capabilities,
        backup_path,
        quarantine_path,
        VideoTranscodePolicy::default(),
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
    video_policy: VideoTranscodePolicy,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
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
        VideoTranscodePolicy::default(),
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
    policy: VideoTranscodePolicy,
) -> Result<Vec<ExecutionStep>, BuildArgsError> {
    if operations.is_empty() {
        return Err(BuildArgsError::EmptyOperations);
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
        ExecutionStep::AtomicReplace {
            source_path,
            output_path,
        } => move_file_to_path(output_path, source_path, "execution.atomic_replace"),
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
    match step {
        ExecutionStep::Command { bin, argv } => command_runner.run(bin, argv),
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
    for (index, step) in steps.iter().enumerate() {
        if is_recovery_step(step) {
            continue;
        }
        if let Err(failed) = execute_step(step, command_runner) {
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

/// Verify generated output and atomically replace the source path.
///
/// # Errors
///
/// Returns [`ExecuteStepError`] when verification fails or the atomic move fails.
pub fn execute_verified_atomic_replace(
    source_path: &str,
    output_path: &str,
) -> Result<(), ExecuteStepError> {
    verify_output_file(output_path)?;
    move_file_to_path(output_path, source_path, "execution.atomic_replace")
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
        OperationKind::Remux
        | OperationKind::MetadataRewrite
        | OperationKind::DispositionRewrite
        | OperationKind::LabelRewrite
        | OperationKind::StreamReorder => Ok(()),
    }
}

fn capabilities_has_encoder(capabilities: &CapabilitySnapshot, required: &str) -> bool {
    capabilities
        .encoders
        .iter()
        .any(|encoder| encoder.trim().eq_ignore_ascii_case(required))
}

fn select_video_encoder_for_policy(
    capabilities: &CapabilitySnapshot,
    policy: VideoTranscodePolicy,
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
        BuildArgsError, CommandRunner, ExecutionStep, HdrColorPolicy, ProcessCommandRunner,
        VideoTranscodeIntent, VideoTranscodePolicy, build_execution_steps,
        build_execution_steps_with_capabilities, build_execution_steps_with_replacement,
        build_execution_steps_with_replacement_policy, build_execution_steps_with_video_policy,
        build_extract_subtitle_argv, build_ffmpeg_argv, build_sidecar_embed_argv,
        execute_filesystem_step, execute_step, execute_step_sequence,
        execute_verified_atomic_replace,
    };
    use crate::capabilities::CapabilitySnapshot;
    use revaer_media_core::plan::{OperationKind, PlannedOperation};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};

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

    #[test]
    fn transcode_requires_stream_id() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: None,
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
        };
        let args_result = build_ffmpeg_argv("/in.mkv", "/out.mkv", &op);
        assert!(args_result.is_ok());
        let args = args_result.ok().unwrap_or_default();
        assert!(args.iter().any(|item| item == "copy"));
    }

    #[test]
    fn metadata_rewrite_strips_container_metadata() {
        let op = PlannedOperation {
            kind: OperationKind::MetadataRewrite,
            stream_id: None,
        };
        let args_result = build_ffmpeg_argv("/in.mkv", "/out.mkv", &op);
        assert!(args_result.is_ok());
        let Ok(args) = args_result else {
            return;
        };
        assert!(args.windows(2).any(|pair| pair == ["-map", "0"]));
        assert!(args.windows(2).any(|pair| pair == ["-c", "copy"]));
        assert!(args.windows(2).any(|pair| pair == ["-map_metadata", "-1"]));
    }

    #[test]
    fn disposition_rewrite_requires_stream_id() {
        let op = PlannedOperation {
            kind: OperationKind::DispositionRewrite,
            stream_id: None,
        };
        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::MissingStreamId)
        );
    }

    #[test]
    fn label_rewrite_requires_stream_id() {
        let op = PlannedOperation {
            kind: OperationKind::LabelRewrite,
            stream_id: None,
        };
        assert_eq!(
            build_ffmpeg_argv("/in.mkv", "/out.mkv", &op),
            Err(BuildArgsError::MissingStreamId)
        );
    }

    #[test]
    fn stream_reorder_maps_ordered_families() {
        let op = PlannedOperation {
            kind: OperationKind::StreamReorder,
            stream_id: None,
        };
        let args_result = build_ffmpeg_argv("/in.mkv", "/out.mkv", &op);
        assert!(args_result.is_ok());
        let Ok(args) = args_result else {
            return;
        };
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:v?"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:a?"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:s?"]));
        assert!(args.windows(2).any(|pair| pair == ["-map", "0:t?"]));
    }

    #[test]
    fn transcode_targets_only_selected_input_stream() {
        let op = PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(3),
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
    fn execution_steps_include_verify_checkpoint() {
        let op = PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
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
    fn capability_checked_execution_rejects_missing_required_codec() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
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
            },
            PlannedOperation {
                kind: OperationKind::AudioTranscode,
                stream_id: Some(1),
            },
            PlannedOperation {
                kind: OperationKind::VideoTranscode,
                stream_id: Some(0),
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
                "/out.mkv",
            ]
        );
    }

    #[test]
    fn replacement_steps_include_optional_backup_verify_and_atomic_replace() {
        let operations = [PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
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
                "/work/movie.with-subs.mkv"
            ]
        );
    }

    #[test]
    fn verified_atomic_replace_rejects_empty_output_and_preserves_source()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_execution_root()?;
        let source_path = root.join("library/movie.mkv");
        let output_path = root.join("workspace/output/movie.mkv");
        fs::create_dir_all(
            source_path
                .parent()
                .ok_or_else(|| std::io::Error::other("source parent missing"))?,
        )?;
        fs::create_dir_all(
            output_path
                .parent()
                .ok_or_else(|| std::io::Error::other("output parent missing"))?,
        )?;
        fs::write(&source_path, b"original")?;
        fs::write(&output_path, b"")?;

        let result = execute_verified_atomic_replace(
            &source_path.to_string_lossy(),
            &output_path.to_string_lossy(),
        );

        assert!(result.is_err());
        assert_eq!(fs::read(&source_path)?, b"original");
        assert!(output_path.exists());
        fs::remove_dir_all(root)?;
        Ok(())
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
    fn process_command_runner_reports_nonzero_status() {
        let result = ProcessCommandRunner.run("/usr/bin/false", &[]);

        assert!(matches!(
            result,
            Err(super::ExecuteStepError::CommandFailed {
                bin,
                status_code: Some(1)
            }) if bin == "/usr/bin/false"
        ));
    }

    #[test]
    fn execute_step_sequence_skips_quarantine_recovery_on_success()
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

        execute_step_sequence(&steps, &RecordingRunner::default())?;

        assert_eq!(fs::read_to_string(source)?, "rewritten");
        assert!(!output.exists());
        assert!(!quarantine.exists());
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
    fn capability_checked_execution_accepts_explicit_encoder_support() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
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
            VideoTranscodePolicy::default(),
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
        };

        assert_eq!(
            build_execution_steps_with_video_policy(
                "/in.mkv",
                "/out.mkv",
                &[op],
                &capabilities,
                policy,
            ),
            Err(BuildArgsError::UnsupportedCodec("libx265"))
        );
    }

    #[test]
    fn hdr10_preservation_policy_adds_color_metadata() {
        let op = PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(0),
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
        };

        let result = build_execution_steps_with_video_policy(
            "/in.mkv",
            "/out.mkv",
            &[op],
            &capabilities,
            policy,
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
        };

        let result = build_execution_steps_with_video_policy(
            "/in.mkv",
            "/out.mkv",
            &[op],
            &capabilities,
            policy,
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
            },
            PlannedOperation {
                kind: OperationKind::AudioTranscode,
                stream_id: Some(1),
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
}
