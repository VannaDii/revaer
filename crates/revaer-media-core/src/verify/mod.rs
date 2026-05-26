//! Output verification helpers.

use crate::plan::{OperationKind, PlannedOperation};
use crate::{model::MediaGraph, model::StreamKind};

/// Verify plan has at least one operation and no unknown state.
///
/// # Errors
///
/// Returns an error string when the operation list is empty or when a
/// stream-scoped operation is missing its stream id.
pub fn verify_plan(operations: &[PlannedOperation]) -> Result<(), &'static str> {
    if operations.is_empty() {
        return Err("plan must contain at least one operation");
    }

    if operations.iter().any(|item| {
        (item.kind == OperationKind::AudioTranscode
            || item.kind == OperationKind::VideoTranscode
            || item.kind == OperationKind::DispositionRewrite
            || item.kind == OperationKind::LabelRewrite)
            && item.stream_id.is_none()
    }) {
        return Err("stream-scoped operation is missing stream id");
    }

    Ok(())
}

/// Verify stream-scoped operations target existing streams with compatible kinds.
///
/// # Errors
///
/// Returns an error string when the plan contains stream-scoped operations that
/// reference missing stream ids or mismatch transcode stream kinds.
pub fn verify_plan_against_source(
    source: &MediaGraph,
    operations: &[PlannedOperation],
) -> Result<(), &'static str> {
    verify_plan(operations)?;

    for operation in operations {
        let Some(stream_id) = operation.stream_id else {
            continue;
        };
        let Some(stream) = source
            .streams
            .iter()
            .find(|item| item.stream_id == stream_id)
        else {
            return Err("stream-scoped operation references unknown stream id");
        };
        match operation.kind {
            OperationKind::AudioTranscode if stream.kind != StreamKind::Audio => {
                return Err("audio transcode operation must target an audio stream");
            }
            OperationKind::VideoTranscode if stream.kind != StreamKind::Video => {
                return Err("video transcode operation must target a video stream");
            }
            OperationKind::Remux
            | OperationKind::MetadataRewrite
            | OperationKind::DispositionRewrite
            | OperationKind::LabelRewrite
            | OperationKind::StreamReorder
            | OperationKind::AudioTranscode
            | OperationKind::VideoTranscode => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{verify_plan, verify_plan_against_source};
    use crate::model::{MediaGraph, MediaStream, StreamKind};
    use crate::plan::{OperationKind, PlannedOperation};

    #[test]
    fn reject_invalid_transcode_operation() {
        let operations = vec![PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: None,
        }];
        assert!(verify_plan(&operations).is_err());
    }

    #[test]
    fn reject_unknown_stream_id_for_stream_scoped_operation() {
        let source = MediaGraph {
            source_path: "/tmp/source.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = vec![PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(9),
        }];
        assert!(verify_plan_against_source(&source, &operations).is_err());
    }

    #[test]
    fn reject_video_transcode_targeting_non_video_stream() {
        let source = MediaGraph {
            source_path: "/tmp/source.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = vec![PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(1),
        }];
        assert!(verify_plan_against_source(&source, &operations).is_err());
    }

    #[test]
    fn accept_matching_transcode_stream_kind() {
        let source = MediaGraph {
            source_path: "/tmp/source.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 2,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let operations = vec![PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(2),
        }];
        assert!(verify_plan_against_source(&source, &operations).is_ok());
    }
}
