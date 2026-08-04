//! Deterministic candidate generation, pruning, and least-cost selection.

use crate::diff::{BoundStream, GraphDiff};
use crate::model::StreamKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

/// Planned operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

/// Planned operation with explicit source and desired-output identities.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlannedOperation {
    /// Operation type.
    pub kind: OperationKind,
    /// Source-container stream id for source-scoped operations.
    pub stream_id: Option<u32>,
    /// Independent desired output stream id for output-scoped operations.
    pub output_stream_id: Option<u32>,
}

/// Candidate operation plan for one desired output graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidatePlan {
    /// Stable plan identifier.
    pub id: String,
    /// Operations contained in the candidate.
    pub operations: Vec<PlannedOperation>,
}

/// Stable reason a candidate was rejected before execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateRejectionReason {
    /// Candidate operation shape is not executable.
    InvalidOperationShape,
    /// A cheaper candidate produces the same desired graph.
    DominatedByLowerCost,
    /// An equal-cost candidate lost the deterministic id tie-break.
    DeterministicTieBreak,
    /// The active runtime capability set cannot execute an operation in this candidate.
    UnsupportedOperation,
    /// Candidate operations do not safely reconcile the source and desired graphs.
    UnsafeOrUnverifiable,
}

/// Persistable rejected candidate rationale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedCandidatePlan {
    /// Rejected candidate.
    pub candidate: CandidatePlan,
    /// Deterministic operation cost.
    pub total_cost: u32,
    /// Stable rejection reason.
    pub reason: CandidateRejectionReason,
}

/// Persistable result of production candidate selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSelection {
    /// Selected least-cost valid candidate.
    pub selected: CandidatePlan,
    /// Selected candidate operation cost.
    pub selected_cost: u32,
    /// Every generated candidate not selected, with deterministic rationale.
    pub rejected: Vec<RejectedCandidatePlan>,
}

/// Candidate generation failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PlanGenerationError {
    /// The desired graph references outputs that have no available source or external input.
    #[error("desired output stream is missing a source binding")]
    MissingDesiredStream,
    /// Candidate generation produced no valid executable plan.
    #[error("candidate generation produced no valid plan: {rejected:?}")]
    NoValidCandidate {
        /// Every candidate rejected before the terminal failure.
        rejected: Vec<RejectedCandidatePlan>,
    },
}

/// Intermediate candidate set after invalid and dominated plans are removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrunedCandidates {
    viable: Vec<CandidatePlan>,
    rejected: Vec<RejectedCandidatePlan>,
}

/// Return the documented deterministic planning cost for an operation kind.
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
        OperationKind::ExtractSubtitle => 3,
        OperationKind::EmbedSubtitle => 4,
        OperationKind::Remux => 5,
        OperationKind::AudioTranscode => 20,
        OperationKind::SubtitleTranscode => 80,
        OperationKind::VideoTranscode => 1_000,
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

/// Generate all production candidates for one graph diff.
///
/// # Errors
///
/// Returns [`PlanGenerationError::MissingDesiredStream`] when compilation left an unbound desired
/// output. The generated alternatives all target the same desired graph.
pub fn generate_candidates(diff: &GraphDiff) -> Result<Vec<CandidatePlan>, PlanGenerationError> {
    if !diff.missing_desired_streams.is_empty() {
        return Err(PlanGenerationError::MissingDesiredStream);
    }

    let minimal_operations = generate_minimal_operations(diff);
    let mut candidates = vec![CandidatePlan {
        id: minimal_candidate_id(&minimal_operations).to_string(),
        operations: minimal_operations.clone(),
    }];
    if diff.container_mismatch {
        let full_rewrite = full_rewrite_operations(&diff.bound_streams);
        if !full_rewrite.is_empty() && full_rewrite != minimal_operations {
            candidates.push(CandidatePlan {
                id: "full-transcode".to_string(),
                operations: full_rewrite,
            });
        }
    }
    Ok(candidates)
}

/// Reject invalid and same-output dominated candidates.
///
/// The input must contain alternatives for one desired graph. Cost and stable id order determine
/// the surviving candidate and persisted rejection rationale.
///
/// # Errors
///
/// Returns [`PlanGenerationError::NoValidCandidate`] when every candidate is malformed.
pub fn prune_invalid_and_dominated(
    candidates: Vec<CandidatePlan>,
) -> Result<PrunedCandidates, PlanGenerationError> {
    prune_invalid_and_dominated_with(candidates, |_| Ok(()))
}

/// Reject malformed, unsafe, unsupported, and same-output dominated candidates.
///
/// The validator receives only shape-valid candidates and returns a stable reason when the active
/// source, desired graph, policy, or runtime capabilities cannot execute one safely.
///
/// # Errors
///
/// Returns [`PlanGenerationError::NoValidCandidate`] with every rejection when no candidate is
/// executable.
pub fn prune_invalid_and_dominated_with<F>(
    candidates: Vec<CandidatePlan>,
    validate: F,
) -> Result<PrunedCandidates, PlanGenerationError>
where
    F: Fn(&CandidatePlan) -> Result<(), CandidateRejectionReason>,
{
    let mut valid = Vec::new();
    let mut rejected = Vec::new();
    for candidate in candidates {
        let rejection = if candidate_shape_is_valid(&candidate) {
            validate(&candidate).err()
        } else {
            Some(CandidateRejectionReason::InvalidOperationShape)
        };
        if let Some(reason) = rejection {
            rejected.push(RejectedCandidatePlan {
                total_cost: candidate_plan_cost(&candidate),
                candidate,
                reason,
            });
        } else {
            valid.push(candidate);
        }
    }
    valid.sort_by(|left, right| {
        candidate_plan_cost(left)
            .cmp(&candidate_plan_cost(right))
            .then_with(|| left.id.cmp(&right.id))
    });
    let Some(selected) = valid.first().cloned() else {
        rejected.sort_by(|left, right| left.candidate.id.cmp(&right.candidate.id));
        return Err(PlanGenerationError::NoValidCandidate { rejected });
    };
    let selected_cost = candidate_plan_cost(&selected);
    for candidate in valid.into_iter().skip(1) {
        let total_cost = candidate_plan_cost(&candidate);
        rejected.push(RejectedCandidatePlan {
            candidate,
            total_cost,
            reason: if total_cost == selected_cost {
                CandidateRejectionReason::DeterministicTieBreak
            } else {
                CandidateRejectionReason::DominatedByLowerCost
            },
        });
    }
    Ok(PrunedCandidates {
        viable: vec![selected],
        rejected,
    })
}

/// Select the least-cost remaining candidate and preserve all rejection evidence.
///
/// # Errors
///
/// Returns [`PlanGenerationError::NoValidCandidate`] when the pruned set is empty.
pub fn select_candidate(
    mut candidates: PrunedCandidates,
) -> Result<PlanSelection, PlanGenerationError> {
    let Some(selected) = candidates.viable.pop() else {
        return Err(PlanGenerationError::NoValidCandidate {
            rejected: candidates.rejected,
        });
    };
    let selected_cost = candidate_plan_cost(&selected);
    candidates.rejected.sort_by(|left, right| {
        left.total_cost
            .cmp(&right.total_cost)
            .then_with(|| left.candidate.id.cmp(&right.candidate.id))
    });
    Ok(PlanSelection {
        selected,
        selected_cost,
        rejected: candidates.rejected,
    })
}

/// Generate, prune, rank, and select a production plan with persistable rationale.
///
/// # Errors
///
/// Returns a [`PlanGenerationError`] when desired outputs are unbound or no valid plan exists.
pub fn generate_plan(diff: &GraphDiff) -> Result<PlanSelection, PlanGenerationError> {
    let generated = generate_candidates(diff)?;
    let pruned = prune_invalid_and_dominated(generated)?;
    select_candidate(pruned)
}

fn generate_minimal_operations(diff: &GraphDiff) -> Vec<PlannedOperation> {
    if !diff_has_changes(diff) {
        return vec![container_operation(OperationKind::NoOp)];
    }

    let mut operations = Vec::new();
    append_recode_operations(&mut operations, diff);
    append_audio_channel_operations(&mut operations, diff);
    append_stream_rewrite_operations(&mut operations, diff);
    append_remux_operations(&mut operations, diff);
    if diff.stream_order_changed {
        operations.push(container_operation(OperationKind::StreamReorder));
    }
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
        let kind = match stream.kind {
            StreamKind::Audio => OperationKind::AudioTranscode,
            StreamKind::Video => OperationKind::VideoTranscode,
            StreamKind::Subtitle => OperationKind::SubtitleTranscode,
            StreamKind::Attachment | StreamKind::Chapter | StreamKind::Data => OperationKind::Remux,
        };
        if kind == OperationKind::Remux {
            push_unique(operations, container_operation(kind));
        } else {
            push_unique(
                operations,
                stream_operation(kind, stream.stream_id, stream.output_stream_id),
            );
        }
    }
}

fn append_audio_channel_operations(operations: &mut Vec<PlannedOperation>, diff: &GraphDiff) {
    for output_stream_id in &diff.audio_channel_mismatched_streams {
        if let Some(source_stream_id) = source_for_output(diff, *output_stream_id) {
            push_unique(
                operations,
                stream_operation(
                    OperationKind::AudioTranscode,
                    source_stream_id,
                    *output_stream_id,
                ),
            );
        }
    }
}

fn append_stream_rewrite_operations(operations: &mut Vec<PlannedOperation>, diff: &GraphDiff) {
    append_rewrites(
        operations,
        diff,
        &diff.stream_metadata_mismatched_streams,
        OperationKind::LabelRewrite,
    );
    append_rewrites(
        operations,
        diff,
        &diff.disposition_mismatched_streams,
        OperationKind::DispositionRewrite,
    );
}

fn append_rewrites(
    operations: &mut Vec<PlannedOperation>,
    diff: &GraphDiff,
    output_stream_ids: &[u32],
    kind: OperationKind,
) {
    for output_stream_id in output_stream_ids {
        if stream_output_planned(operations, *output_stream_id) {
            continue;
        }
        if let Some(source_stream_id) = source_for_output(diff, *output_stream_id) {
            push_unique(
                operations,
                stream_operation(kind, source_stream_id, *output_stream_id),
            );
        }
    }
}

fn append_remux_operations(operations: &mut Vec<PlannedOperation>, diff: &GraphDiff) {
    if !diff.removed_streams.is_empty() || diff.container_mismatch {
        push_unique(operations, container_operation(OperationKind::Remux));
    }
}

fn full_rewrite_operations(bound_streams: &[BoundStream]) -> Vec<PlannedOperation> {
    let mut operations = Vec::new();
    for stream in bound_streams {
        let kind = match stream.kind {
            StreamKind::Video => Some(OperationKind::VideoTranscode),
            StreamKind::Audio => Some(OperationKind::AudioTranscode),
            StreamKind::Subtitle => Some(OperationKind::SubtitleTranscode),
            StreamKind::Attachment | StreamKind::Chapter | StreamKind::Data => None,
        };
        if let Some(kind) = kind {
            push_unique(
                &mut operations,
                stream_operation(kind, stream.source_stream_id, stream.output_stream_id),
            );
        }
    }
    if !operations.is_empty() {
        operations.push(container_operation(OperationKind::Remux));
    }
    operations
}

fn minimal_candidate_id(operations: &[PlannedOperation]) -> &'static str {
    if operations.len() == 1 && operations[0].kind == OperationKind::NoOp {
        "no-op"
    } else if operations.len() == 1 && operations[0].kind == OperationKind::Remux {
        "remux-only"
    } else {
        "minimal-transform"
    }
}

fn source_for_output(diff: &GraphDiff, output_stream_id: u32) -> Option<u32> {
    diff.stream_bindings
        .iter()
        .find(|binding| binding.output_stream_id == output_stream_id)
        .and_then(|binding| binding.source_stream_id)
        .or_else(|| Some(output_stream_id).filter(|_| diff.stream_bindings.is_empty()))
}

const fn container_operation(kind: OperationKind) -> PlannedOperation {
    PlannedOperation {
        kind,
        stream_id: None,
        output_stream_id: None,
    }
}

const fn stream_operation(
    kind: OperationKind,
    source_stream_id: u32,
    output_stream_id: u32,
) -> PlannedOperation {
    PlannedOperation {
        kind,
        stream_id: Some(source_stream_id),
        output_stream_id: Some(output_stream_id),
    }
}

fn push_unique(operations: &mut Vec<PlannedOperation>, operation: PlannedOperation) {
    if !operations.contains(&operation) {
        operations.push(operation);
    }
}

fn stream_output_planned(operations: &[PlannedOperation], output_stream_id: u32) -> bool {
    operations
        .iter()
        .any(|operation| operation.output_stream_id == Some(output_stream_id))
}

fn candidate_shape_is_valid(candidate: &CandidatePlan) -> bool {
    if candidate.id.trim().is_empty() || candidate.operations.is_empty() {
        return false;
    }
    let mut unique = BTreeSet::new();
    candidate
        .operations
        .iter()
        .all(|operation| unique.insert(operation.clone()) && operation_shape_is_valid(operation))
        && (candidate.operations.len() == 1
            || !candidate
                .operations
                .iter()
                .any(|operation| operation.kind == OperationKind::NoOp))
}

const fn operation_shape_is_valid(operation: &PlannedOperation) -> bool {
    match operation.kind {
        OperationKind::NoOp
        | OperationKind::Remux
        | OperationKind::MetadataRewrite
        | OperationKind::StreamReorder
        | OperationKind::CopySidecarSubtitle
        | OperationKind::RemoveSidecarSubtitle => {
            operation.stream_id.is_none() && operation.output_stream_id.is_none()
        }
        OperationKind::EmbedSubtitle => operation.output_stream_id.is_some(),
        OperationKind::DispositionRewrite
        | OperationKind::LabelRewrite
        | OperationKind::ExtractSubtitle
        | OperationKind::SubtitleTranscode
        | OperationKind::AudioTranscode
        | OperationKind::VideoTranscode => {
            operation.stream_id.is_some() && operation.output_stream_id.is_some()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CandidatePlan, CandidateRejectionReason, OperationKind, PlanGenerationError,
        PlannedOperation, RejectedCandidatePlan, candidate_plan_cost, generate_plan,
        operation_cost, prune_invalid_and_dominated, prune_invalid_and_dominated_with,
        select_candidate,
    };
    use crate::diff::{BoundStream, GraphDiff, RecodedStream};
    use crate::model::{DesiredStreamBinding, StreamKind};

    fn stream_operation(
        kind: OperationKind,
        source_stream_id: u32,
        output_stream_id: u32,
    ) -> PlannedOperation {
        PlannedOperation {
            kind,
            stream_id: Some(source_stream_id),
            output_stream_id: Some(output_stream_id),
        }
    }

    fn container_operation(kind: OperationKind) -> PlannedOperation {
        PlannedOperation {
            kind,
            stream_id: None,
            output_stream_id: None,
        }
    }

    #[test]
    fn operation_cost_uses_documented_planner_defaults() {
        assert_eq!(operation_cost(OperationKind::ExtractSubtitle), 3);
        assert_eq!(operation_cost(OperationKind::EmbedSubtitle), 4);
        assert_eq!(operation_cost(OperationKind::Remux), 5);
        assert_eq!(operation_cost(OperationKind::AudioTranscode), 20);
        assert_eq!(operation_cost(OperationKind::SubtitleTranscode), 80);
        assert_eq!(operation_cost(OperationKind::VideoTranscode), 1_000);
    }

    #[test]
    fn corrected_extraction_cost_changes_candidate_ranking() -> Result<(), PlanGenerationError> {
        let extract = CandidatePlan {
            id: "z-extract-and-label".to_string(),
            operations: vec![
                stream_operation(OperationKind::ExtractSubtitle, 3, 0),
                stream_operation(OperationKind::LabelRewrite, 3, 0),
            ],
        };
        let remux = CandidatePlan {
            id: "a-remux".to_string(),
            operations: vec![container_operation(OperationKind::Remux)],
        };

        let selection = select_candidate(prune_invalid_and_dominated(vec![remux, extract])?)?;

        assert_eq!(selection.selected.id, "z-extract-and-label");
        assert_eq!(selection.selected_cost, 4);
        Ok(())
    }

    #[test]
    fn production_pipeline_selects_remux_and_retains_transcode_rejection()
    -> Result<(), PlanGenerationError> {
        let selection = generate_plan(&GraphDiff {
            container_mismatch: true,
            stream_bindings: vec![DesiredStreamBinding {
                output_stream_id: 0,
                source_stream_id: Some(7),
            }],
            bound_streams: vec![BoundStream {
                output_stream_id: 0,
                source_stream_id: 7,
                kind: StreamKind::Video,
            }],
            ..GraphDiff::default()
        })?;

        assert_eq!(selection.selected.id, "remux-only");
        assert_eq!(selection.selected_cost, 5);
        assert_eq!(selection.rejected.len(), 1);
        assert_eq!(selection.rejected[0].candidate.id, "full-transcode");
        assert_eq!(
            selection.rejected[0].reason,
            CandidateRejectionReason::DominatedByLowerCost
        );
        Ok(())
    }

    #[test]
    fn fanout_plans_independent_outputs_from_one_source() -> Result<(), PlanGenerationError> {
        let selection = generate_plan(&GraphDiff {
            stream_bindings: vec![
                DesiredStreamBinding {
                    output_stream_id: 0,
                    source_stream_id: Some(9),
                },
                DesiredStreamBinding {
                    output_stream_id: 1,
                    source_stream_id: Some(9),
                },
            ],
            recoded_streams: vec![
                RecodedStream {
                    stream_id: 9,
                    output_stream_id: 0,
                    kind: StreamKind::Audio,
                },
                RecodedStream {
                    stream_id: 9,
                    output_stream_id: 1,
                    kind: StreamKind::Audio,
                },
            ],
            ..GraphDiff::default()
        })?;

        assert_eq!(selection.selected.operations.len(), 2);
        assert_eq!(selection.selected.operations[0].stream_id, Some(9));
        assert_eq!(selection.selected.operations[0].output_stream_id, Some(0));
        assert_eq!(selection.selected.operations[1].stream_id, Some(9));
        assert_eq!(selection.selected.operations[1].output_stream_id, Some(1));
        Ok(())
    }

    #[test]
    fn no_diff_selects_explicit_noop() -> Result<(), PlanGenerationError> {
        let selection = generate_plan(&GraphDiff::default())?;
        assert_eq!(
            selection.selected.operations,
            vec![container_operation(OperationKind::NoOp)]
        );
        Ok(())
    }

    #[test]
    fn missing_desired_stream_fails_closed() {
        assert_eq!(
            generate_plan(&GraphDiff {
                missing_desired_streams: vec![9],
                ..GraphDiff::default()
            }),
            Err(PlanGenerationError::MissingDesiredStream)
        );
    }

    #[test]
    fn candidate_cost_sums_documented_weights() {
        let candidate = CandidatePlan {
            id: "audio-remux".to_string(),
            operations: vec![
                stream_operation(OperationKind::AudioTranscode, 2, 0),
                container_operation(OperationKind::Remux),
            ],
        };
        assert_eq!(candidate_plan_cost(&candidate), 25);
    }

    #[test]
    fn invalid_candidate_is_retained_with_rationale() {
        let invalid = CandidatePlan {
            id: "invalid".to_string(),
            operations: vec![PlannedOperation {
                kind: OperationKind::VideoTranscode,
                stream_id: None,
                output_stream_id: Some(0),
            }],
        };
        assert_eq!(
            prune_invalid_and_dominated(vec![invalid.clone()]),
            Err(PlanGenerationError::NoValidCandidate {
                rejected: vec![RejectedCandidatePlan {
                    candidate: invalid,
                    total_cost: 1_000,
                    reason: CandidateRejectionReason::InvalidOperationShape,
                }],
            })
        );
    }

    #[test]
    fn unsupported_cheapest_candidate_falls_back_to_supported_candidate()
    -> Result<(), PlanGenerationError> {
        let cheap = CandidatePlan {
            id: "metadata-only".to_string(),
            operations: vec![container_operation(OperationKind::MetadataRewrite)],
        };
        let supported = CandidatePlan {
            id: "audio-transcode".to_string(),
            operations: vec![stream_operation(OperationKind::AudioTranscode, 2, 0)],
        };
        let pruned = prune_invalid_and_dominated_with(vec![cheap, supported], |candidate| {
            if candidate
                .operations
                .iter()
                .any(|operation| operation.kind == OperationKind::MetadataRewrite)
            {
                Err(CandidateRejectionReason::UnsupportedOperation)
            } else {
                Ok(())
            }
        })?;
        let selection = select_candidate(pruned)?;

        assert_eq!(selection.selected.id, "audio-transcode");
        assert_eq!(selection.rejected[0].candidate.id, "metadata-only");
        assert_eq!(
            selection.rejected[0].reason,
            CandidateRejectionReason::UnsupportedOperation
        );
        Ok(())
    }
}
