//! Command argument builders.

use revaer_media_core::plan::{OperationKind, PlannedOperation};
use thiserror::Error;

/// Build error for command arguments.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BuildArgsError {
    /// Stream id required but absent.
    #[error("stream id required for transcode operation")]
    MissingStreamId,
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
    let mut args = vec![
        "-i".to_string(),
        input_path.to_string(),
        "-map".to_string(),
        "0".to_string(),
    ];

    match operation.kind {
        OperationKind::Remux => {
            args.push("-c".to_string());
            args.push("copy".to_string());
        }
        OperationKind::AudioTranscode => {
            let stream_id = operation.stream_id.ok_or(BuildArgsError::MissingStreamId)?;
            args.push("-c:a".to_string());
            args.push("aac".to_string());
            args.push("-metadata:s:a".to_string());
            args.push(format!("index={stream_id}"));
        }
        OperationKind::VideoTranscode => {
            let stream_id = operation.stream_id.ok_or(BuildArgsError::MissingStreamId)?;
            args.push("-c:v".to_string());
            args.push("libx265".to_string());
            args.push("-metadata:s:v".to_string());
            args.push(format!("index={stream_id}"));
        }
    }

    args.push(output_path.to_string());
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::{BuildArgsError, build_ffmpeg_argv};
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
}
