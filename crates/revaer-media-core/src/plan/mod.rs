//! Deterministic planning primitives.

use crate::diff::GraphDiff;
use crate::model::StreamKind;

/// Planned operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    /// Source already satisfies desired output.
    NoOp,
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
    /// Embed an existing adjacent subtitle into the media output.
    EmbedSubtitle,
    /// Extract an embedded subtitle into a sidecar output.
    ExtractSubtitle,
    /// Copy or convert an existing sidecar into a managed sidecar output.
    CopySidecarSubtitle,
    /// Remove an existing sidecar after verified replacement.
    RemoveSidecarSubtitle,
    /// Subtitle stream transcode without OCR.
    SubtitleTranscode,
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
        OperationKind::NoOp => 0,
        OperationKind::MetadataRewrite
        | OperationKind::DispositionRewrite
        | OperationKind::LabelRewrite => 1,
        OperationKind::StreamReorder
        | OperationKind::CopySidecarSubtitle
        | OperationKind::RemoveSidecarSubtitle => 2,
        OperationKind::EmbedSubtitle | OperationKind::ExtractSubtitle => 4,
        OperationKind::SubtitleTranscode => 10,
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

    if !diff_has_changes(diff) {
        operations.push(PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
        });
        return operations;
    }

    append_recode_operations(&mut operations, diff);
    append_audio_channel_operations(&mut operations, diff);
    append_stream_rewrite_operations(&mut operations, diff);
    append_remux_operations(&mut operations, diff);
    append_stream_reorder_operation(&mut operations, diff);

    operations
}

const fn diff_has_changes(diff: &GraphDiff) -> bool {
    !diff.removed_streams.is_empty()
        || !diff.missing_desired_streams.is_empty()
        || !diff.stream_metadata_mismatched_streams.is_empty()
        || !diff.disposition_mismatched_streams.is_empty()
        || !diff.recoded_streams.is_empty()
        || !diff.audio_channel_mismatched_streams.is_empty()
        || diff.stream_order_changed
        || diff.container_mismatch
}

fn append_recode_operations(operations: &mut Vec<PlannedOperation>, diff: &GraphDiff) {
    for stream in &diff.recoded_streams {
        let (kind, stream_id) = match stream.kind {
            StreamKind::Audio => (OperationKind::AudioTranscode, Some(stream.stream_id)),
            StreamKind::Video => (OperationKind::VideoTranscode, Some(stream.stream_id)),
            StreamKind::Subtitle => (OperationKind::SubtitleTranscode, Some(stream.stream_id)),
            StreamKind::Attachment | StreamKind::Chapter | StreamKind::Data => {
                (OperationKind::Remux, None)
            }
        };
        operations.push(PlannedOperation { kind, stream_id });
    }
}

fn append_audio_channel_operations(operations: &mut Vec<PlannedOperation>, diff: &GraphDiff) {
    for stream_id in &diff.audio_channel_mismatched_streams {
        if !stream_operation_planned(operations, OperationKind::AudioTranscode, *stream_id) {
            operations.push(PlannedOperation {
                kind: OperationKind::AudioTranscode,
                stream_id: Some(*stream_id),
            });
        }
    }
}

fn append_stream_rewrite_operations(operations: &mut Vec<PlannedOperation>, diff: &GraphDiff) {
    for stream_id in &diff.stream_metadata_mismatched_streams {
        if !stream_planned(operations, *stream_id) {
            operations.push(PlannedOperation {
                kind: OperationKind::LabelRewrite,
                stream_id: Some(*stream_id),
            });
        }
    }

    for stream_id in &diff.disposition_mismatched_streams {
        if !stream_planned(operations, *stream_id) {
            operations.push(PlannedOperation {
                kind: OperationKind::DispositionRewrite,
                stream_id: Some(*stream_id),
            });
        }
    }
}

fn append_remux_operations(operations: &mut Vec<PlannedOperation>, diff: &GraphDiff) {
    if !diff.removed_streams.is_empty() {
        operations.push(PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
        });
    }

    if diff.container_mismatch
        && !operations
            .iter()
            .any(|operation| operation.kind == OperationKind::Remux)
    {
        operations.push(PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
        });
    }
}

fn append_stream_reorder_operation(operations: &mut Vec<PlannedOperation>, diff: &GraphDiff) {
    if diff.stream_order_changed {
        operations.push(PlannedOperation {
            kind: OperationKind::StreamReorder,
            stream_id: None,
        });
    }
}

fn stream_operation_planned(
    operations: &[PlannedOperation],
    kind: OperationKind,
    stream_id: u32,
) -> bool {
    operations
        .iter()
        .any(|operation| operation.kind == kind && operation.stream_id == Some(stream_id))
}

fn stream_planned(operations: &[PlannedOperation], stream_id: u32) -> bool {
    operations
        .iter()
        .any(|operation| operation.stream_id == Some(stream_id))
}

#[cfg(test)]
mod tests {
    use super::{
        CandidatePlan, OperationKind, PlannedOperation, candidate_plan_cost, generate_plan,
        operation_cost, select_least_cost_plan,
    };
    use crate::diff::{GraphDiff, RecodedStream, diff_graphs};
    use crate::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};

    #[test]
    fn operation_cost_uses_documented_planner_defaults() {
        assert_eq!(operation_cost(OperationKind::NoOp), 0);
        assert_eq!(operation_cost(OperationKind::MetadataRewrite), 1);
        assert_eq!(operation_cost(OperationKind::DispositionRewrite), 1);
        assert_eq!(operation_cost(OperationKind::LabelRewrite), 1);
        assert_eq!(operation_cost(OperationKind::StreamReorder), 2);
        assert_eq!(operation_cost(OperationKind::CopySidecarSubtitle), 2);
        assert_eq!(operation_cost(OperationKind::RemoveSidecarSubtitle), 2);
        assert_eq!(operation_cost(OperationKind::EmbedSubtitle), 4);
        assert_eq!(operation_cost(OperationKind::ExtractSubtitle), 4);
        assert_eq!(operation_cost(OperationKind::Remux), 5);
        assert_eq!(operation_cost(OperationKind::SubtitleTranscode), 10);
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
    fn no_diff_yields_noop() {
        let operations = generate_plan(&GraphDiff::default());

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::NoOp);
    }

    #[test]
    fn missing_desired_stream_does_not_yield_noop() {
        let operations = generate_plan(&GraphDiff {
            missing_desired_streams: vec![9],
            ..GraphDiff::default()
        });

        assert!(operations.is_empty());
    }

    #[test]
    fn container_mismatch_yields_remux() {
        let operations = generate_plan(&GraphDiff {
            container_mismatch: true,
            ..GraphDiff::default()
        });

        assert_eq!(
            operations,
            vec![PlannedOperation {
                kind: OperationKind::Remux,
                stream_id: None,
            }]
        );
    }

    #[test]
    fn recoded_audio_stream_yields_audio_transcode() {
        let operations = generate_plan(&GraphDiff {
            recoded_streams: vec![RecodedStream {
                stream_id: 2,
                kind: StreamKind::Audio,
            }],
            ..GraphDiff::default()
        });

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::AudioTranscode);
        assert_eq!(operations[0].stream_id, Some(2));
    }

    #[test]
    fn recoded_video_stream_yields_video_transcode() {
        let operations = generate_plan(&GraphDiff {
            recoded_streams: vec![RecodedStream {
                stream_id: 1,
                kind: StreamKind::Video,
            }],
            ..GraphDiff::default()
        });

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::VideoTranscode);
        assert_eq!(operations[0].stream_id, Some(1));
    }

    #[test]
    fn removed_streams_yield_remux_operation() {
        let operations = generate_plan(&GraphDiff {
            removed_streams: vec![5],
            ..GraphDiff::default()
        });

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::Remux);
        assert_eq!(operations[0].stream_id, None);
    }

    #[test]
    fn audio_channel_shape_mismatch_yields_audio_transcode() {
        let operations = generate_plan(&GraphDiff {
            audio_channel_mismatched_streams: vec![2],
            ..GraphDiff::default()
        });

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].kind, OperationKind::AudioTranscode);
        assert_eq!(operations[0].stream_id, Some(2));
    }

    #[test]
    fn stream_metadata_mismatch_yields_label_rewrite() {
        let operations = generate_plan(&GraphDiff {
            stream_metadata_mismatched_streams: vec![4],
            ..GraphDiff::default()
        });

        assert_eq!(
            operations,
            vec![PlannedOperation {
                kind: OperationKind::LabelRewrite,
                stream_id: Some(4),
            }]
        );
    }

    #[test]
    fn disposition_mismatch_yields_disposition_rewrite() {
        let operations = generate_plan(&GraphDiff {
            disposition_mismatched_streams: vec![3],
            ..GraphDiff::default()
        });

        assert_eq!(
            operations,
            vec![PlannedOperation {
                kind: OperationKind::DispositionRewrite,
                stream_id: Some(3),
            }]
        );
    }

    #[test]
    fn metadata_and_disposition_mismatches_deduplicate_existing_transcode() {
        let operations = generate_plan(&GraphDiff {
            stream_metadata_mismatched_streams: vec![2],
            disposition_mismatched_streams: vec![2],
            recoded_streams: vec![RecodedStream {
                stream_id: 2,
                kind: StreamKind::Audio,
            }],
            ..GraphDiff::default()
        });

        assert_eq!(
            operations,
            vec![PlannedOperation {
                kind: OperationKind::AudioTranscode,
                stream_id: Some(2),
            }]
        );
    }

    #[test]
    fn recoded_audio_with_channel_shape_mismatch_deduplicates_transcode() {
        let operations = generate_plan(&GraphDiff {
            recoded_streams: vec![RecodedStream {
                stream_id: 2,
                kind: StreamKind::Audio,
            }],
            audio_channel_mismatched_streams: vec![2],
            ..GraphDiff::default()
        });

        assert_eq!(
            operations,
            vec![PlannedOperation {
                kind: OperationKind::AudioTranscode,
                stream_id: Some(2),
            }]
        );
    }

    #[test]
    fn desired_stream_order_change_yields_stream_reorder() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![
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
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: vec!["default".to_string()],
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            streams: vec![
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: Some("Main".to_string()),
                    dispositions: vec!["default".to_string()],
                },
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
            ],
        };

        let operations = generate_plan(&diff_graphs(&source, &desired));

        assert_eq!(
            operations,
            vec![PlannedOperation {
                kind: OperationKind::StreamReorder,
                stream_id: None,
            }]
        );
    }
}
