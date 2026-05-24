//! Deterministic planning primitives.

use crate::diff::GraphDiff;

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

    for stream_id in &diff.recoded_streams {
        operations.push(PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: Some(*stream_id),
        });
    }

    for stream_id in &diff.removed_streams {
        operations.push(PlannedOperation {
            kind: OperationKind::AudioTranscode,
            stream_id: Some(*stream_id),
        });
    }

    operations
}

#[cfg(test)]
mod tests {
    use super::{OperationKind, generate_plan};
    use crate::diff::GraphDiff;

    #[test]
    fn no_diff_yields_remux() {
        let operations = generate_plan(&GraphDiff {
            removed_streams: Vec::new(),
            recoded_streams: Vec::new(),
        });

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::Remux);
    }
}
