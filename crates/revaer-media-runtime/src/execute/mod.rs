//! Command argument builders.

use crate::capabilities::CapabilitySnapshot;
use revaer_media_core::plan::{OperationKind, PlannedOperation};
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

const DEFAULT_VIDEO_ENCODER: &str = "libx265";
const VIDEO_ENCODER_FALLBACKS: &[&str] = &[
    "libx265",
    "hevc_nvenc",
    "hevc_qsv",
    "hevc_vaapi",
    "hevc",
    "h265",
];
const VIDEO_TRANSCODE_PRESET: &str = "medium";
const VIDEO_TRANSCODE_CRF: &str = "22";

/// Deterministic execution step for runtime orchestration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStep {
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

fn build_ffmpeg_argv_with_video_encoder(
    input_path: &str,
    output_path: &str,
    operation: &PlannedOperation,
    video_encoder: &str,
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
            args.push("-preset".to_string());
            args.push(VIDEO_TRANSCODE_PRESET.to_string());
            args.push("-crf".to_string());
            args.push(VIDEO_TRANSCODE_CRF.to_string());
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
    let mut steps = Vec::new();
    if let Some(path) = backup_path {
        steps.push(ExecutionStep::BackupSource {
            source_path: source_path.to_string(),
            backup_path: path.to_string(),
        });
    }
    steps.extend(build_execution_steps_with_capabilities(
        source_path,
        output_path,
        operations,
        capabilities,
    )?);
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
    if operations.is_empty() {
        return Err(BuildArgsError::EmptyOperations);
    }

    let selected_video_encoder = select_video_encoder(capabilities);
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
        let argv = build_ffmpeg_argv_with_video_encoder(
            &stage_input,
            &stage_output,
            operation,
            video_encoder,
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

fn select_video_encoder(capabilities: &CapabilitySnapshot) -> Option<&'static str> {
    VIDEO_ENCODER_FALLBACKS
        .iter()
        .copied()
        .find(|candidate| capabilities_has_encoder(capabilities, candidate))
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

#[cfg(test)]
mod tests {
    use super::{
        BuildArgsError, ExecutionStep, build_execution_steps,
        build_execution_steps_with_capabilities, build_execution_steps_with_replacement,
        build_ffmpeg_argv,
    };
    use crate::capabilities::CapabilitySnapshot;
    use revaer_media_core::plan::{OperationKind, PlannedOperation};

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
