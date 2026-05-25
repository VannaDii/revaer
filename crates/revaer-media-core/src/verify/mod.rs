//! Output verification helpers.

use crate::plan::{OperationKind, PlannedOperation};

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

#[cfg(test)]
mod tests {
    use super::verify_plan;
    use crate::plan::{OperationKind, PlannedOperation};

    #[test]
    fn reject_invalid_transcode_operation() {
        let operations = vec![PlannedOperation {
            kind: OperationKind::VideoTranscode,
            stream_id: None,
        }];
        assert!(verify_plan(&operations).is_err());
    }
}
