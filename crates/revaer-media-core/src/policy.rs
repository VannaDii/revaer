//! Validated policy values shared by configuration and policy compilation.
//!
//! No defaults, persistence, filesystem access or execution authority live here.
//! The operation table is one family of the complete ADR 518 aggregate, not a
//! complete effective policy or permission to execute a plan.

use crate::plan::OperationKind;
use thiserror::Error;

const OPERATION_COUNT: usize = 13;
const MAX_COST_WEIGHT: u32 = 1_000_000;

/// One explicitly authored operation permission and cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationCost {
    /// Closed operation kind; unknown names must fail at the decoding boundary.
    pub kind: OperationKind,
    /// Nonnegative weight, bounded by the persisted policy contract.
    pub cost_weight: u32,
    /// False forbids the operation, independently of its cost.
    pub enabled: bool,
}

/// Invalid operation-cost family, without submitted values in diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum OperationCostError {
    /// The family does not contain each of the thirteen kinds exactly once.
    #[error("operation_cost_incomplete")]
    Incomplete,
    /// A weight exceeds the persisted policy bound.
    #[error("operation_cost_out_of_bounds")]
    OutOfBounds,
}

/// Complete operation-cost family in ADR 518's canonical operation order.
///
/// Construct from explicit rows; there is no default or missing-kind fallback.
/// Disabled rows are retained, including their weights, for policy identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCosts {
    rows: [OperationCost; OPERATION_COUNT],
}

impl OperationCosts {
    /// Validate all thirteen kinds and canonicalize their order.
    ///
    /// # Errors
    /// Rejects missing, extra or duplicate kinds and weights over 1,000,000.
    pub fn new(input: &[OperationCost]) -> Result<Self, OperationCostError> {
        let mut rows: [OperationCost; OPERATION_COUNT] = input
            .try_into()
            .map_err(|_| OperationCostError::Incomplete)?;
        rows.sort_unstable_by_key(|row| operation_index(row.kind));
        for (index, row) in rows.iter().enumerate() {
            if operation_index(row.kind) != index {
                return Err(OperationCostError::Incomplete);
            }
            if row.cost_weight > MAX_COST_WEIGHT {
                return Err(OperationCostError::OutOfBounds);
            }
        }
        Ok(Self { rows })
    }

    /// Read the explicit permission and weight for a known operation.
    #[must_use]
    pub const fn get(&self, kind: OperationKind) -> OperationCost {
        self.rows[operation_index(kind)]
    }

    /// Read all rows in the canonical policy-identity order.
    #[must_use]
    pub const fn rows(&self) -> &[OperationCost; OPERATION_COUNT] {
        &self.rows
    }
}

const fn operation_index(kind: OperationKind) -> usize {
    match kind {
        OperationKind::NoOp => 0,
        OperationKind::Remux => 1,
        OperationKind::MetadataRewrite => 2,
        OperationKind::DispositionRewrite => 3,
        OperationKind::LabelRewrite => 4,
        OperationKind::StreamReorder => 5,
        OperationKind::EmbedSubtitle => 6,
        OperationKind::ExtractSubtitle => 7,
        OperationKind::CopySidecarSubtitle => 8,
        OperationKind::RemoveSidecarSubtitle => 9,
        OperationKind::SubtitleTranscode => 10,
        OperationKind::AudioTranscode => 11,
        OperationKind::VideoTranscode => 12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{
        CandidatePlan, CandidateRejectionReason, PlannedOperation, prune_with_policy,
        select_candidate,
    };

    fn candidate(id: &str, kind: OperationKind) -> CandidatePlan {
        CandidatePlan {
            id: id.to_owned(),
            operations: vec![PlannedOperation {
                kind,
                stream_id: None,
                output_stream_id: None,
            }],
        }
    }

    #[test]
    fn authored_costs_control_selection_and_recorded_totals()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut rows = explicit_rows();
        rows[1].cost_weight = 1;
        rows[2].cost_weight = 100;
        let costs = OperationCosts::new(&rows)?;
        let selection = select_candidate(prune_with_policy(
            vec![
                candidate("metadata", OperationKind::MetadataRewrite),
                candidate("remux", OperationKind::Remux),
            ],
            &costs,
            |_| Ok(()),
        )?)?;
        assert_eq!(selection.selected.id, "remux");
        assert_eq!(selection.selected_cost, 1);
        assert_eq!(selection.rejected[0].total_cost, 100);
        Ok(())
    }

    #[test]
    fn disabled_zero_cost_operation_never_wins() -> Result<(), Box<dyn std::error::Error>> {
        let mut rows = explicit_rows();
        rows[1].cost_weight = 0;
        rows[1].enabled = false;
        let costs = OperationCosts::new(&rows)?;
        let selection = select_candidate(prune_with_policy(
            vec![
                candidate("remux", OperationKind::Remux),
                candidate("metadata", OperationKind::MetadataRewrite),
            ],
            &costs,
            |_| Ok(()),
        )?)?;
        assert_eq!(selection.selected.id, "metadata");
        assert_eq!(selection.selected_cost, 7);
        assert_eq!(
            selection.rejected[0].reason,
            CandidateRejectionReason::UnsupportedOperation
        );
        assert_eq!(selection.rejected[0].total_cost, 0);
        Ok(())
    }

    #[test]
    fn policy_permission_does_not_override_safety_rejection()
    -> Result<(), Box<dyn std::error::Error>> {
        let costs = OperationCosts::new(&explicit_rows())?;
        let result = prune_with_policy(
            vec![candidate("remux", OperationKind::Remux)],
            &costs,
            |_| Err(CandidateRejectionReason::UnsafeOrUnverifiable),
        );
        let Err(crate::plan::PlanGenerationError::NoValidCandidate { rejected }) = result else {
            return Err("unsafe candidate was not rejected".into());
        };
        assert_eq!(
            rejected[0].reason,
            CandidateRejectionReason::UnsafeOrUnverifiable
        );
        Ok(())
    }

    fn explicit_rows() -> [OperationCost; OPERATION_COUNT] {
        [
            OperationKind::NoOp,
            OperationKind::Remux,
            OperationKind::MetadataRewrite,
            OperationKind::DispositionRewrite,
            OperationKind::LabelRewrite,
            OperationKind::StreamReorder,
            OperationKind::EmbedSubtitle,
            OperationKind::ExtractSubtitle,
            OperationKind::CopySidecarSubtitle,
            OperationKind::RemoveSidecarSubtitle,
            OperationKind::SubtitleTranscode,
            OperationKind::AudioTranscode,
            OperationKind::VideoTranscode,
        ]
        .map(|kind| OperationCost {
            kind,
            cost_weight: 7,
            enabled: true,
        })
    }

    #[test]
    fn canonicalizes_input_without_changing_explicit_values() -> Result<(), OperationCostError> {
        let expected = explicit_rows();
        let mut input = expected;
        input.reverse();
        let costs = OperationCosts::new(&input)?;
        assert_eq!(costs.rows(), &expected);
        for row in expected {
            assert_eq!(costs.get(row.kind), row);
        }
        Ok(())
    }

    #[test]
    fn rejects_missing_extra_and_each_duplicate_kind() {
        let rows = explicit_rows();
        assert_eq!(
            OperationCosts::new(&[]),
            Err(OperationCostError::Incomplete)
        );
        assert_eq!(
            OperationCosts::new(&rows[..12]),
            Err(OperationCostError::Incomplete)
        );
        let mut extra = rows.to_vec();
        extra.push(rows[0]);
        assert_eq!(
            OperationCosts::new(&extra),
            Err(OperationCostError::Incomplete)
        );
        for index in 0..OPERATION_COUNT {
            let mut duplicate = rows;
            duplicate[index] = rows[(index + 1) % OPERATION_COUNT];
            assert_eq!(
                OperationCosts::new(&duplicate),
                Err(OperationCostError::Incomplete)
            );
        }
    }

    #[test]
    fn disabled_rows_retain_zero_and_maximum_weights() -> Result<(), OperationCostError> {
        for enabled in [false, true] {
            for cost_weight in [0, MAX_COST_WEIGHT] {
                let rows = explicit_rows().map(|row| OperationCost {
                    cost_weight,
                    enabled,
                    ..row
                });
                let costs = OperationCosts::new(&rows)?;
                assert_eq!(costs.rows(), &rows);
            }
        }
        Ok(())
    }

    #[test]
    fn rejects_excess_weight_even_when_operation_is_disabled() {
        for index in 0..OPERATION_COUNT {
            let mut rows = explicit_rows();
            rows[index].cost_weight = MAX_COST_WEIGHT + 1;
            rows[index].enabled = false;
            assert_eq!(
                OperationCosts::new(&rows),
                Err(OperationCostError::OutOfBounds)
            );
        }
    }
}
