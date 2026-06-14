//! Output verification helpers.

use crate::model::{MediaGraph, MediaStream, StreamKind};
use crate::plan::{OperationKind, PlannedOperation};
use std::collections::BTreeSet;

/// Verify stream ids are unique within one graph.
///
/// # Errors
///
/// Returns an error string when two streams share the same stream id.
pub fn verify_unique_stream_ids(streams: &[MediaStream]) -> Result<(), &'static str> {
    let mut seen = BTreeSet::new();
    for stream in streams {
        if !seen.insert(stream.stream_id) {
            return Err("duplicate stream id");
        }
    }
    Ok(())
}

/// Verify plan has at least one operation and consistent stream scoping.
///
/// # Errors
///
/// Returns an error string when the operation list is empty or when operation
/// stream scope does not match the operation kind.
pub fn verify_plan(operations: &[PlannedOperation]) -> Result<(), &'static str> {
    if operations.is_empty() {
        return Err("plan must contain at least one operation");
    }

    if operations.iter().any(|item| {
        (item.kind == OperationKind::AudioTranscode
            || item.kind == OperationKind::VideoTranscode
            || item.kind == OperationKind::DispositionRewrite
            || item.kind == OperationKind::LabelRewrite
            || item.kind == OperationKind::EmbedSubtitle
            || item.kind == OperationKind::ExtractSubtitle
            || item.kind == OperationKind::SubtitleTranscode)
            && item.stream_id.is_none()
    }) {
        return Err("stream-scoped operation is missing stream id");
    }

    if operations
        .iter()
        .any(|item| item.kind == OperationKind::NoOp && item.stream_id.is_some())
    {
        return Err("no-op operation must not target a stream");
    }

    if operations.iter().any(|item| {
        (item.kind == OperationKind::Remux
            || item.kind == OperationKind::MetadataRewrite
            || item.kind == OperationKind::StreamReorder
            || item.kind == OperationKind::CopySidecarSubtitle
            || item.kind == OperationKind::RemoveSidecarSubtitle)
            && item.stream_id.is_some()
    }) {
        return Err("non-stream-scoped operation must not target a stream");
    }

    if operations.len() > 1
        && operations
            .iter()
            .any(|item| item.kind == OperationKind::NoOp)
    {
        return Err("no-op operation must not be combined with mutating operations");
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
        if operation.kind == OperationKind::EmbedSubtitle {
            continue;
        }
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
            OperationKind::ExtractSubtitle if stream.kind != StreamKind::Subtitle => {
                return Err("subtitle extract operation must target a subtitle stream");
            }
            OperationKind::SubtitleTranscode if stream.kind != StreamKind::Subtitle => {
                return Err("subtitle transcode operation must target a subtitle stream");
            }
            OperationKind::NoOp
            | OperationKind::Remux
            | OperationKind::MetadataRewrite
            | OperationKind::DispositionRewrite
            | OperationKind::LabelRewrite
            | OperationKind::StreamReorder
            | OperationKind::EmbedSubtitle
            | OperationKind::ExtractSubtitle
            | OperationKind::CopySidecarSubtitle
            | OperationKind::RemoveSidecarSubtitle
            | OperationKind::SubtitleTranscode
            | OperationKind::AudioTranscode
            | OperationKind::VideoTranscode => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{verify_plan, verify_plan_against_source, verify_unique_stream_ids};
    use crate::model::{MediaGraph, MediaStream, StreamKind};
    use crate::plan::{OperationKind, PlannedOperation};

    #[test]
    fn reject_duplicate_stream_ids() {
        let streams = vec![
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
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: Some("eng".to_string()),
                title: None,
                dispositions: Vec::new(),
            },
        ];

        assert_eq!(
            verify_unique_stream_ids(&streams),
            Err("duplicate stream id")
        );
    }

    #[test]
    fn accept_unique_stream_ids() {
        let streams = vec![
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
        ];

        assert_eq!(verify_unique_stream_ids(&streams), Ok(()));
    }

    #[test]
    fn reject_invalid_transcode_operation() {
        let operations = vec![PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: None,
        }];
        assert!(verify_plan(&operations).is_err());
    }

    #[test]
    fn reject_container_scoped_operation_targeting_stream() {
        let operations = vec![PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: Some(1),
        }];
        assert_eq!(
            verify_plan(&operations),
            Err("non-stream-scoped operation must not target a stream")
        );
    }

    #[test]
    fn reject_unknown_stream_id_for_stream_scoped_operation() {
        let source = MediaGraph {
            source_path: "/tmp/source.mkv".to_string(),
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
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
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
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 2,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                channels: None,
                channel_layout: None,
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
