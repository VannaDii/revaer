//! Plan explanation models.

use crate::plan::{OperationKind, PlannedOperation};

/// Human-readable explanation record for a selected operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    /// Deterministic message suitable for audit trails.
    pub message: String,
}

/// Create a concise deterministic explanation set.
#[must_use]
pub fn explain_plan(operations: &[PlannedOperation]) -> Vec<Explanation> {
    operations
        .iter()
        .map(|item| Explanation {
            message: format!(
                "selected operation: {} stream_id={}",
                operation_kind_code(item.kind),
                stream_id_code(item.stream_id)
            ),
        })
        .collect()
}

fn operation_kind_code(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Remux => "remux",
        OperationKind::AudioTranscode => "audio_transcode",
        OperationKind::VideoTranscode => "video_transcode",
    }
}

fn stream_id_code(stream_id: Option<u32>) -> String {
    match stream_id {
        Some(value) => value.to_string(),
        None => "none".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::explain_plan;
    use crate::plan::{OperationKind, PlannedOperation};

    #[test]
    fn produce_explanation_rows() {
        let explanations = explain_plan(&[PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
        }]);

        assert_eq!(explanations.len(), 1);
        assert!(explanations[0].message.contains("selected operation"));
    }
}
