//! Deterministic planning primitives.

use crate::diff::GraphDiff;
use crate::model::StreamKind;

/// Planned operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    /// Container-level remux only.
    Remux,
    /// Container-level metadata rewrite.
    MetadataRewrite,
    /// Stream-level disposition rewrite.
    DispositionRewrite,
    /// Stream-level label rewrite.
    LabelRewrite,
    /// Container stream ordering rewrite.
    StreamReorder,
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

/// Candidate operation plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePlan {
    /// Stable plan identifier.
    pub id: String,
    /// Operations contained in the candidate.
    pub operations: Vec<PlannedOperation>,
}

/// Return the deterministic planning cost for an operation kind.
#[must_use]
pub const fn operation_cost(kind: OperationKind) -> u32 {
    match kind {
        OperationKind::MetadataRewrite
        | OperationKind::DispositionRewrite
        | OperationKind::LabelRewrite => 1,
        OperationKind::StreamReorder => 2,
        OperationKind::Remux => 5,
        OperationKind::AudioTranscode => 20,
        OperationKind::VideoTranscode => 1000,
    }
}

/// Return the deterministic total cost for a candidate plan.
#[must_use]
pub fn candidate_plan_cost(candidate: &CandidatePlan) -> u32 {
    candidate
        .operations
        .iter()
        .map(|operation| operation_cost(operation.kind))
        .sum()
}

/// Select the least expensive candidate plan, breaking ties by stable id.
#[must_use]
pub fn select_least_cost_plan(candidates: &[CandidatePlan]) -> Option<&CandidatePlan> {
    candidates.iter().min_by(|left, right| {
        candidate_plan_cost(left)
            .cmp(&candidate_plan_cost(right))
            .then_with(|| left.id.cmp(&right.id))
    })
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
    use super::{
        CandidatePlan, OperationKind, PlannedOperation, candidate_plan_cost, generate_plan,
        operation_cost, select_least_cost_plan,
    };
    use crate::diff::{GraphDiff, RecodedStream};
    use crate::model::StreamKind;

    #[test]
    fn operation_cost_uses_documented_planner_defaults() {
        assert_eq!(operation_cost(OperationKind::MetadataRewrite), 1);
        assert_eq!(operation_cost(OperationKind::DispositionRewrite), 1);
        assert_eq!(operation_cost(OperationKind::LabelRewrite), 1);
        assert_eq!(operation_cost(OperationKind::StreamReorder), 2);
        assert_eq!(operation_cost(OperationKind::Remux), 5);
        assert_eq!(operation_cost(OperationKind::AudioTranscode), 20);
        assert_eq!(operation_cost(OperationKind::VideoTranscode), 1000);
    }

    #[test]
    fn candidate_plan_cost_sums_operation_costs() {
        let candidate = CandidatePlan {
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

        assert_eq!(candidate_plan_cost(&candidate), 25);
    }

    #[test]
    fn select_least_cost_plan_prefers_lower_cost_candidate() {
        let candidates = vec![
            CandidatePlan {
                id: "full-transcode".to_string(),
                operations: vec![PlannedOperation {
                    kind: OperationKind::VideoTranscode,
                    stream_id: Some(0),
                }],
            },
            CandidatePlan {
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
            },
        ];

        let selected = select_least_cost_plan(&candidates);

        assert_eq!(
            selected.map(|candidate| candidate.id.as_str()),
            Some("audio-remux")
        );
    }

    #[test]
    fn select_least_cost_plan_breaks_equal_cost_ties_by_id() {
        let candidates = vec![
            CandidatePlan {
                id: "z-remux".to_string(),
                operations: vec![PlannedOperation {
                    kind: OperationKind::Remux,
                    stream_id: None,
                }],
            },
            CandidatePlan {
                id: "a-remux".to_string(),
                operations: vec![PlannedOperation {
                    kind: OperationKind::Remux,
                    stream_id: None,
                }],
            },
        ];

        let selected = select_least_cost_plan(&candidates);

        assert_eq!(
            selected.map(|candidate| candidate.id.as_str()),
            Some("a-remux")
        );
    }

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
