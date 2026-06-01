//! Plan explanation models.

use crate::plan::{CandidatePlan, OperationKind, PlannedOperation, candidate_plan_cost};

/// Human-readable explanation record for a selected operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    /// Deterministic message suitable for audit trails.
    pub message: String,
}

/// Explanation for a selected plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedPlanExplanation {
    /// Stable selected plan identifier.
    pub id: String,
    /// Sum of operation costs for the selected plan.
    pub total_cost: u32,
    /// Deterministic reasons for each selected operation.
    pub reasons: Vec<String>,
}

/// Explanation for a rejected plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedPlanExplanation {
    /// Stable rejected plan identifier.
    pub id: String,
    /// Sum of operation costs for the rejected plan.
    pub total_cost: u32,
    /// Deterministic rejection reason.
    pub reason: String,
}

/// Explanation for plan selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanSelectionExplanation {
    /// Selected plan explanation.
    pub selected_plan: SelectedPlanExplanation,
    /// Rejected plan explanations.
    pub rejected_plans: Vec<RejectedPlanExplanation>,
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

/// Explain selected and rejected candidate plans with deterministic costs.
#[must_use]
pub fn explain_plan_selection(
    selected: &CandidatePlan,
    rejected: &[CandidatePlan],
) -> PlanSelectionExplanation {
    let selected_cost = candidate_plan_cost(selected);
    let rejected_plans = rejected
        .iter()
        .map(|plan| {
            let rejected_cost = candidate_plan_cost(plan);
            RejectedPlanExplanation {
                id: plan.id.clone(),
                total_cost: rejected_cost,
                reason: rejection_reason(selected_cost, rejected_cost).to_string(),
            }
        })
        .collect();

    PlanSelectionExplanation {
        selected_plan: SelectedPlanExplanation {
            id: selected.id.clone(),
            total_cost: selected_cost,
            reasons: selected
                .operations
                .iter()
                .map(selected_operation_reason)
                .collect(),
        },
        rejected_plans,
    }
}

const fn operation_kind_code(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Remux => "remux",
        OperationKind::MetadataRewrite => "metadata_rewrite",
        OperationKind::DispositionRewrite => "disposition_rewrite",
        OperationKind::LabelRewrite => "label_rewrite",
        OperationKind::StreamReorder => "stream_reorder",
        OperationKind::AudioTranscode => "audio_transcode",
        OperationKind::VideoTranscode => "video_transcode",
    }
}

fn selected_operation_reason(operation: &PlannedOperation) -> String {
    match operation.kind {
        OperationKind::AudioTranscode => format!(
            "audio codec mismatch requires {} stream_id={}",
            operation_kind_code(operation.kind),
            stream_id_code(operation.stream_id)
        ),
        OperationKind::VideoTranscode => format!(
            "video codec mismatch requires {} stream_id={}",
            operation_kind_code(operation.kind),
            stream_id_code(operation.stream_id)
        ),
        OperationKind::Remux => "container rewrite preserves selected streams".to_string(),
        OperationKind::MetadataRewrite => "metadata differs from desired output".to_string(),
        OperationKind::DispositionRewrite => "stream disposition differs from policy".to_string(),
        OperationKind::LabelRewrite => "stream label differs from policy".to_string(),
        OperationKind::StreamReorder => {
            "stream order differs from deterministic ranking".to_string()
        }
    }
}

const fn rejection_reason(selected_cost: u32, rejected_cost: u32) -> &'static str {
    if rejected_cost > selected_cost {
        "higher cost with no compliance benefit"
    } else if rejected_cost == selected_cost {
        "equivalent cost without deterministic tie-break win"
    } else {
        "lower cost candidate rejected by upstream safety validation"
    }
}

fn stream_id_code(stream_id: Option<u32>) -> String {
    stream_id.map_or_else(|| "none".to_string(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{explain_plan, explain_plan_selection};
    use crate::plan::{CandidatePlan, OperationKind, PlannedOperation};

    #[test]
    fn produce_explanation_rows() {
        let explanations = explain_plan(&[PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
        }]);

        assert_eq!(explanations.len(), 1);
        assert!(explanations[0].message.contains("selected operation"));
    }

    #[test]
    fn explain_plan_selection_includes_selected_cost_and_operation_reasons() {
        let selected = CandidatePlan {
            id: "audio-remux".to_string(),
            operations: vec![
                PlannedOperation {
                    kind: OperationKind::AudioTranscode,
                    stream_id: Some(2),
                },
                PlannedOperation {
                    kind: OperationKind::Remux,
                    stream_id: None,
                },
            ],
        };

        let explanation = explain_plan_selection(&selected, &[]);

        assert_eq!(explanation.selected_plan.id, "audio-remux");
        assert_eq!(explanation.selected_plan.total_cost, 25);
        assert_eq!(
            explanation.selected_plan.reasons,
            vec![
                "audio codec mismatch requires audio_transcode stream_id=2",
                "container rewrite preserves selected streams",
            ]
        );
    }

    #[test]
    fn explain_plan_selection_includes_rejected_plan_costs_and_reasons() {
        let selected = CandidatePlan {
            id: "remux".to_string(),
            operations: vec![PlannedOperation {
                kind: OperationKind::Remux,
                stream_id: None,
            }],
        };
        let rejected = CandidatePlan {
            id: "full-transcode".to_string(),
            operations: vec![PlannedOperation {
                kind: OperationKind::VideoTranscode,
                stream_id: Some(0),
            }],
        };

        let explanation = explain_plan_selection(&selected, &[rejected]);

        assert_eq!(explanation.rejected_plans.len(), 1);
        assert_eq!(explanation.rejected_plans[0].id, "full-transcode");
        assert_eq!(explanation.rejected_plans[0].total_cost, 1000);
        assert_eq!(
            explanation.rejected_plans[0].reason,
            "higher cost with no compliance benefit"
        );
    }
}
