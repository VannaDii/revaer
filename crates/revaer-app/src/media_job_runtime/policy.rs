//! Validate immutable job policy inputs before inspecting or planning media.

use std::collections::BTreeSet;

use revaer_data::media::policy_snapshot::OperationCostSnapshotRow;
use revaer_media_core::policy::{OperationCost, OperationCosts};
use serde::Deserialize;

use super::MediaJobRuntimeError;

const fn invalid_snapshot() -> MediaJobRuntimeError {
    MediaJobRuntimeError::InvalidDesiredGraph("media_job_operation_cost_snapshot_invalid")
}

pub(super) fn ensure_preflight_allowed(
    costs: &OperationCosts,
    evaluation: &revaer_media_runtime::jobs::JobPreflightEvaluation,
) -> Result<(), MediaJobRuntimeError> {
    if let Some(report) = evaluation.as_ready() {
        ensure_operations_allowed(costs, &report.planned.operations)?;
    }
    Ok(())
}

fn ensure_operations_allowed(
    costs: &OperationCosts,
    operations: &[revaer_media_core::plan::PlannedOperation],
) -> Result<(), MediaJobRuntimeError> {
    if operations
        .iter()
        .any(|operation| !costs.get(operation.kind).enabled)
    {
        return Err(MediaJobRuntimeError::InvalidDesiredGraph(
            "media_job_operation_forbidden_by_policy",
        ));
    }
    Ok(())
}

pub(super) fn compile_costs(
    rows: Vec<OperationCostSnapshotRow>,
) -> Result<OperationCosts, MediaJobRuntimeError> {
    if rows.len() != 13 {
        return Err(invalid_snapshot());
    }
    let mut orders = BTreeSet::new();
    let mut costs = Vec::with_capacity(rows.len());
    for row in rows {
        if row.sort_order < 0 || !orders.insert(row.sort_order) {
            return Err(invalid_snapshot());
        }
        let kind = Deserialize::deserialize(serde::de::value::StrDeserializer::<
            serde::de::value::Error,
        >::new(&row.operation_kind))
        .map_err(|_| invalid_snapshot())?;
        costs.push(OperationCost {
            kind,
            cost_weight: u32::try_from(row.cost_weight).map_err(|_| invalid_snapshot())?,
            enabled: row.enabled,
        });
    }
    OperationCosts::new(&costs).map_err(|_| invalid_snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;
    use revaer_media_core::plan::OperationKind;

    fn rows() -> Vec<OperationCostSnapshotRow> {
        [
            "no_op",
            "remux",
            "metadata_rewrite",
            "disposition_rewrite",
            "label_rewrite",
            "stream_reorder",
            "embed_subtitle",
            "extract_subtitle",
            "copy_sidecar_subtitle",
            "remove_sidecar_subtitle",
            "subtitle_transcode",
            "audio_transcode",
            "video_transcode",
        ]
        .into_iter()
        .zip(0..13)
        .map(|(kind, order)| OperationCostSnapshotRow {
            operation_kind: kind.into(),
            cost_weight: 7,
            sort_order: order,
            enabled: true,
        })
        .collect()
    }

    #[test]
    fn preserves_explicit_disabled_zero_cost() -> Result<(), MediaJobRuntimeError> {
        let mut input = rows();
        input[1].cost_weight = 0;
        input[1].enabled = false;
        let costs = compile_costs(input)?;
        assert!(!costs.get(OperationKind::Remux).enabled);
        assert_eq!(costs.get(OperationKind::Remux).cost_weight, 0);
        Ok(())
    }

    #[test]
    fn final_permission_check_rejects_disabled_sidecar_operations()
    -> Result<(), MediaJobRuntimeError> {
        let mut input = rows();
        input[9].enabled = false;
        let costs = compile_costs(input)?;
        let operation = revaer_media_core::plan::PlannedOperation {
            kind: OperationKind::RemoveSidecarSubtitle,
            stream_id: None,
            output_stream_id: None,
        };
        assert!(ensure_operations_allowed(&costs, &[operation]).is_err());
        assert!(ensure_operations_allowed(&costs, &[]).is_ok());
        Ok(())
    }

    #[test]
    fn rejects_incomplete_unknown_duplicate_and_invalid_values() {
        assert!(compile_costs(Vec::new()).is_err());
        for mutation in 0..7 {
            let mut input = rows();
            match mutation {
                0 => {
                    input.pop();
                }
                1 => input.push(input[0].clone()),
                2 => input[0].operation_kind = "unknown".into(),
                3 => input[0].operation_kind = "remux".into(),
                4 => input[0].sort_order = 1,
                5 => input[0].cost_weight = -1,
                _ => input[0].cost_weight = 1_000_001,
            }
            assert!(compile_costs(input).is_err());
        }
    }
}
