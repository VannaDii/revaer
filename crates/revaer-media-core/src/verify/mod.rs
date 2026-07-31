//! Output verification helpers.

use crate::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};
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
        matches!(
            item.kind,
            OperationKind::AudioTranscode
                | OperationKind::VideoTranscode
                | OperationKind::DispositionRewrite
                | OperationKind::LabelRewrite
                | OperationKind::SubtitleTranscode
        ) && (item.stream_id.is_none() || item.output_stream_id.is_none())
    }) {
        return Err("stream-scoped operation is missing source or output stream id");
    }

    if operations
        .iter()
        .any(|item| item.kind == OperationKind::EmbedSubtitle && item.output_stream_id.is_none())
    {
        return Err("subtitle embed operation is missing output stream id");
    }

    if operations.iter().any(|item| {
        item.kind == OperationKind::ExtractSubtitle
            && (item.stream_id.is_none() || item.output_stream_id.is_some())
    }) {
        return Err("subtitle extract operation must target only a source stream");
    }

    if operations.iter().any(|item| {
        item.kind == OperationKind::NoOp
            && (item.stream_id.is_some() || item.output_stream_id.is_some())
    }) {
        return Err("no-op operation must not target a stream");
    }

    if operations.iter().any(|item| {
        (item.kind == OperationKind::Remux
            || item.kind == OperationKind::MetadataRewrite
            || item.kind == OperationKind::StreamReorder
            || item.kind == OperationKind::CopySidecarSubtitle
            || item.kind == OperationKind::RemoveSidecarSubtitle)
            && (item.stream_id.is_some() || item.output_stream_id.is_some())
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

/// Verify operation output identities and source bindings against compiled graphs.
///
/// # Errors
///
/// Returns an error string when an operation references an unknown desired output or disagrees
/// with the compiler-owned source binding.
pub fn verify_plan_against_graphs(
    source: &MediaGraph,
    desired: &DesiredGraph,
    operations: &[PlannedOperation],
) -> Result<(), &'static str> {
    verify_plan_against_source(source, operations)?;
    for operation in operations {
        let Some(output_stream_id) = operation.output_stream_id else {
            continue;
        };
        if !desired
            .streams
            .iter()
            .any(|stream| stream.stream_id == output_stream_id)
        {
            return Err("operation references unknown desired output stream id");
        }
        if let Some(source_stream_id) = operation.stream_id {
            let bound_source = desired
                .stream_bindings
                .iter()
                .find(|binding| binding.output_stream_id == output_stream_id)
                .and_then(|binding| binding.source_stream_id)
                .or_else(|| Some(output_stream_id).filter(|_| desired.stream_bindings.is_empty()));
            if bound_source != Some(source_stream_id) {
                return Err("operation source disagrees with desired output binding");
            }
        }
    }
    Ok(())
}

/// Verify a materialized output graph against compiled desired output identities and stream state.
///
/// # Errors
///
/// Returns an error string when container format, stream cardinality, identity, or normalized
/// stream state differs from the desired graph.
pub fn verify_output_graph(
    actual: &MediaGraph,
    desired: &DesiredGraph,
) -> Result<(), &'static str> {
    verify_unique_stream_ids(&actual.streams)?;
    if desired.container_format.as_deref().is_some_and(|format| {
        !actual
            .container_formats
            .iter()
            .any(|actual_format| actual_format.eq_ignore_ascii_case(format))
    }) {
        return Err("output container does not match desired container");
    }
    if actual.streams.len() != desired.streams.len() {
        return Err("output stream count does not match desired graph");
    }
    for desired_stream in &desired.streams {
        let Some(actual_stream) = actual
            .streams
            .iter()
            .find(|stream| stream.stream_id == desired_stream.stream_id)
        else {
            return Err("desired output stream is missing");
        };
        if actual_stream != desired_stream {
            return Err("output stream state does not match desired graph");
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
            output_stream_id: Some(0),
        }];
        assert!(verify_plan(&operations).is_err());
    }

    #[test]
    fn reject_container_scoped_operation_targeting_stream() {
        let operations = vec![PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: Some(1),
            output_stream_id: None,
        }];
        assert_eq!(
            verify_plan(&operations),
            Err("non-stream-scoped operation must not target a stream")
        );
    }

    #[test]
    fn accept_source_scoped_subtitle_extraction() {
        let operations = vec![PlannedOperation {
            kind: OperationKind::ExtractSubtitle,
            stream_id: Some(3),
            output_stream_id: None,
        }];

        assert_eq!(verify_plan(&operations), Ok(()));
    }

    #[test]
    fn reject_subtitle_extraction_with_container_output_identity() {
        let operations = vec![PlannedOperation {
            kind: OperationKind::ExtractSubtitle,
            stream_id: Some(3),
            output_stream_id: Some(4),
        }];

        assert_eq!(
            verify_plan(&operations),
            Err("subtitle extract operation must target only a source stream")
        );
    }

    #[test]
    fn reject_unknown_stream_id_for_stream_scoped_operation() {
        let source = MediaGraph {
            source_path: "/tmp/source.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
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
            output_stream_id: Some(0),
        }];
        assert!(verify_plan_against_source(&source, &operations).is_err());
    }

    #[test]
    fn reject_video_transcode_targeting_non_video_stream() {
        let source = MediaGraph {
            source_path: "/tmp/source.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
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
            output_stream_id: Some(0),
        }];
        assert!(verify_plan_against_source(&source, &operations).is_err());
    }

    #[test]
    fn accept_matching_transcode_stream_kind() {
        let source = MediaGraph {
            source_path: "/tmp/source.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
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
            output_stream_id: Some(0),
        }];
        assert!(verify_plan_against_source(&source, &operations).is_ok());
    }
}
