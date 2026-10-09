//! Plan explanation models.

use crate::plan::{CandidateRejectionReason, OperationKind, PlanSelection, PlannedOperation};
use serde::{Deserialize, Serialize};

/// Human-readable explanation record for a selected operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Explanation {
    /// Deterministic message suitable for audit trails.
    pub message: String,
}

/// Structured explanation for one selected operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedOperationExplanation {
    /// Selected operation kind.
    pub kind: OperationKind,
    /// Source-container stream identity when the operation consumes one.
    pub source_stream_id: Option<u32>,
    /// Independent desired-output stream identity when the operation produces one.
    pub output_stream_id: Option<u32>,
    /// Deterministic human-readable selection rationale.
    pub reason: String,
}

/// Explanation for a selected plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedPlanExplanation {
    /// Stable selected plan identifier.
    pub id: String,
    /// Sum of operation costs for the selected plan.
    pub total_cost: u32,
    /// Deterministic structured explanation for every selected operation.
    pub operations: Vec<SelectedOperationExplanation>,
}

/// Explanation for a rejected plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedPlanExplanation {
    /// Stable rejected plan identifier.
    pub id: String,
    /// Sum of operation costs for the rejected plan.
    pub total_cost: u32,
    /// Stable planner rejection reason.
    pub reason: CandidateRejectionReason,
}

/// Explanation for plan selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
                "selected operation: {} source_stream_id={} output_stream_id={}",
                operation_kind_code(item.kind),
                stream_id_code(item.stream_id),
                stream_id_code(item.output_stream_id)
            ),
        })
        .collect()
}

/// Explain selected and rejected candidate plans with deterministic costs.
#[must_use]
pub fn explain_plan_selection(selection: &PlanSelection) -> PlanSelectionExplanation {
    let rejected_plans = selection
        .rejected
        .iter()
        .map(|rejected| RejectedPlanExplanation {
            id: rejected.candidate.id.clone(),
            total_cost: rejected.total_cost,
            reason: rejected.reason,
        })
        .collect();

    PlanSelectionExplanation {
        selected_plan: SelectedPlanExplanation {
            id: selection.selected.id.clone(),
            total_cost: selection.selected_cost,
            operations: selection
                .selected
                .operations
                .iter()
                .map(|operation| SelectedOperationExplanation {
                    kind: operation.kind,
                    source_stream_id: operation.stream_id,
                    output_stream_id: operation.output_stream_id,
                    reason: selected_operation_reason(operation),
                })
                .collect(),
        },
        rejected_plans,
    }
}

const fn operation_kind_code(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::NoOp => "no_op",
        OperationKind::Remux => "remux",
        OperationKind::MetadataRewrite => "metadata_rewrite",
        OperationKind::DispositionRewrite => "disposition_rewrite",
        OperationKind::LabelRewrite => "label_rewrite",
        OperationKind::StreamReorder => "stream_reorder",
        OperationKind::EmbedSubtitle => "embed_subtitle",
        OperationKind::ExtractSubtitle => "extract_subtitle",
        OperationKind::CopySidecarSubtitle => "copy_sidecar_subtitle",
        OperationKind::RemoveSidecarSubtitle => "remove_sidecar_subtitle",
        OperationKind::SubtitleTranscode => "subtitle_transcode",
        OperationKind::AudioTranscode => "audio_transcode",
        OperationKind::VideoTranscode => "video_transcode",
    }
}

fn selected_operation_reason(operation: &PlannedOperation) -> String {
    match operation.kind {
        OperationKind::NoOp => "source already satisfies desired output".to_string(),
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
        OperationKind::EmbedSubtitle => "existing sidecar must be embedded".to_string(),
        OperationKind::ExtractSubtitle => "embedded subtitle must be extracted".to_string(),
        OperationKind::CopySidecarSubtitle => {
            "existing sidecar must be materialized in managed output".to_string()
        }
        OperationKind::RemoveSidecarSubtitle => {
            "existing sidecar must be removed after verified replacement".to_string()
        }
        OperationKind::SubtitleTranscode => {
            "subtitle codec mismatch requires supported text or copy conversion".to_string()
        }
    }
}

fn stream_id_code(stream_id: Option<u32>) -> String {
    stream_id.map_or_else(|| "none".to_string(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{explain_plan, explain_plan_selection};
    use crate::plan::{
        CandidatePlan, CandidateRejectionReason, OperationKind, PlanSelection, PlannedOperation,
        RejectedCandidatePlan,
    };

    #[test]
    fn produce_explanation_rows() {
        let explanations = explain_plan(&[PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
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
                    output_stream_id: Some(0),
                },
                PlannedOperation {
                    kind: OperationKind::Remux,
                    stream_id: None,
                    output_stream_id: None,
                },
            ],
        };

        let explanation = explain_plan_selection(&PlanSelection {
            selected,
            selected_cost: 25,
            rejected: Vec::new(),
        });

        assert_eq!(explanation.selected_plan.id, "audio-remux");
        assert_eq!(explanation.selected_plan.total_cost, 25);
        assert_eq!(
            explanation.selected_plan.operations[0].source_stream_id,
            Some(2)
        );
        assert_eq!(
            explanation.selected_plan.operations[0].output_stream_id,
            Some(0)
        );
        assert_eq!(
            explanation.selected_plan.operations[0].reason,
            "audio codec mismatch requires audio_transcode stream_id=2"
        );
        assert_eq!(
            explanation.selected_plan.operations[1].reason,
            "container rewrite preserves selected streams"
        );
    }

    #[test]
    fn explain_plan_selection_includes_rejected_plan_costs_and_reasons() {
        let selected = CandidatePlan {
            id: "remux".to_string(),
            operations: vec![PlannedOperation {
                kind: OperationKind::Remux,
                stream_id: None,
                output_stream_id: None,
            }],
        };
        let rejected = CandidatePlan {
            id: "full-transcode".to_string(),
            operations: vec![PlannedOperation {
                kind: OperationKind::VideoTranscode,
                stream_id: Some(0),
                output_stream_id: Some(0),
            }],
        };

        let explanation = explain_plan_selection(&PlanSelection {
            selected,
            selected_cost: 5,
            rejected: vec![RejectedCandidatePlan {
                candidate: rejected,
                total_cost: 1_000,
                reason: CandidateRejectionReason::InvalidOperationShape,
            }],
        });

        assert_eq!(explanation.rejected_plans.len(), 1);
        assert_eq!(explanation.rejected_plans[0].id, "full-transcode");
        assert_eq!(explanation.rejected_plans[0].total_cost, 1000);
        assert_eq!(
            explanation.rejected_plans[0].reason,
            CandidateRejectionReason::InvalidOperationShape
        );
    }
}
