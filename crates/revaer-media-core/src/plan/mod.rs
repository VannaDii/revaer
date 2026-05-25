//! Deterministic planning primitives.

use crate::diff::GraphDiff;
use crate::model::StreamKind;

/// Planned operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    /// Container-level remux only.
    Remux,
    /// Audio stream transcode.
    AudioTranscode,
    /// Video stream transcode.
    VideoTranscode,
}

/// Planned operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedOperation {
    /// Operation type.
    pub kind: OperationKind,
    /// Stream id if stream-scoped.
    pub stream_id: Option<u32>,
}

/// Generate a deterministic operation plan from a diff.
#[must_use]
pub fn generate_plan(diff: &GraphDiff) -> Vec<PlannedOperation> {
    let mut operations = Vec::new();

    if diff.removed_streams.is_empty() && diff.recoded_streams.is_empty() {
        operations.push(PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
        });
        return operations;
    }

    for stream in &diff.recoded_streams {
        let (kind, stream_id) = match stream.kind {
            StreamKind::Audio => (OperationKind::AudioTranscode, Some(stream.stream_id)),
            StreamKind::Video => (OperationKind::VideoTranscode, Some(stream.stream_id)),
            StreamKind::Subtitle | StreamKind::Attachment | StreamKind::Chapter => {
                (OperationKind::Remux, None)
            }
        };
        operations.push(PlannedOperation { kind, stream_id });
    }

    if !diff.removed_streams.is_empty() {
        operations.push(PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
        });
    }

    operations
}

#[cfg(test)]
mod tests {
    use super::{OperationKind, generate_plan};
    use crate::diff::{GraphDiff, RecodedStream};
    use crate::model::StreamKind;

    #[test]
    fn no_diff_yields_remux() {
        let operations = generate_plan(&GraphDiff {
            removed_streams: Vec::new(),
            recoded_streams: Vec::new(),
        });

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::Remux);
    }

    #[test]
    fn recoded_audio_stream_yields_audio_transcode() {
        let operations = generate_plan(&GraphDiff {
            removed_streams: Vec::new(),
            recoded_streams: vec![RecodedStream {
                stream_id: 2,
                kind: StreamKind::Audio,
            }],
        });

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::AudioTranscode);
        assert_eq!(operations[0].stream_id, Some(2));
    }

    #[test]
    fn recoded_video_stream_yields_video_transcode() {
        let operations = generate_plan(&GraphDiff {
            removed_streams: Vec::new(),
            recoded_streams: vec![RecodedStream {
                stream_id: 1,
                kind: StreamKind::Video,
            }],
        });

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::VideoTranscode);
        assert_eq!(operations[0].stream_id, Some(1));
    }

    #[test]
    fn removed_streams_yield_remux_operation() {
        let operations = generate_plan(&GraphDiff {
            removed_streams: vec![5],
            recoded_streams: Vec::new(),
        });

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::Remux);
        assert_eq!(operations[0].stream_id, None);
    }
}
