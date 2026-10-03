//! Production composition of deterministic target compilation and plan selection.

use crate::diff::diff_graphs;
use crate::explain::{PlanSelectionExplanation, explain_plan_selection};
use crate::model::{DesiredGraph, MediaGraph};
use crate::plan::{
    CandidateRejectionReason, OperationKind, PlanGenerationError, PlanSelection,
    generate_candidates, prune_invalid_and_dominated_with, select_candidate,
};
use crate::target::{
    DesiredTarget, TargetCompileError, UnmatchedStreamPolicy, compile_desired_target,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

/// Runtime operation capabilities applied before candidate ranking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningConstraints {
    /// Operation kinds the injected runtime can execute for this job.
    pub supported_operations: BTreeSet<OperationKind>,
}

impl PlanningConstraints {
    /// Build constraints that support every operation currently modeled by the core planner.
    #[must_use]
    pub fn all_supported() -> Self {
        Self {
            supported_operations: [
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
            .into_iter()
            .collect(),
        }
    }
}

/// Persistable output of the core planning pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningOutcome {
    /// Concrete desired graph produced from the immutable target.
    pub desired_graph: DesiredGraph,
    /// Selected plan plus every rejected candidate and stable rejection reason.
    pub selection: PlanSelection,
    /// Structured human-readable explanation retaining source and output identities.
    pub explanation: PlanSelectionExplanation,
}

/// Core planning pipeline failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PlanningPipelineError {
    /// Desired-target compilation failed closed.
    #[error(transparent)]
    Target(#[from] TargetCompileError),
    /// Candidate generation or selection failed closed.
    #[error(transparent)]
    Plan(#[from] PlanGenerationError),
}

/// Compile, compare, generate, prune, and select one production planning outcome.
///
/// # Errors
///
/// Returns [`PlanningPipelineError`] when target compilation or least-cost candidate selection
/// fails. The function performs no I/O; callers persist the complete returned outcome before
/// execution.
pub fn compile_and_plan(
    source: &MediaGraph,
    output_path: &str,
    target: &DesiredTarget,
    unmatched_policy: UnmatchedStreamPolicy,
    constraints: &PlanningConstraints,
) -> Result<PlanningOutcome, PlanningPipelineError> {
    compile_and_plan_using(
        source,
        output_path,
        target,
        unmatched_policy,
        constraints,
        |candidates, validate| prune_invalid_and_dominated_with(candidates, validate),
    )
}

/// Compile and select a plan using the job's complete captured operation policy.
///
/// No current-policy or hard-coded cost fallback is used by this entry point.
///
/// # Errors
/// Rejects invalid targets, unsupported or disabled operations, unsafe candidates,
/// and overflowing costs.
pub fn compile_and_plan_with_policy(
    source: &MediaGraph,
    output_path: &str,
    target: &DesiredTarget,
    unmatched_policy: UnmatchedStreamPolicy,
    constraints: &PlanningConstraints,
    costs: &crate::policy::OperationCosts,
) -> Result<PlanningOutcome, PlanningPipelineError> {
    compile_and_plan_using(
        source,
        output_path,
        target,
        unmatched_policy,
        constraints,
        |candidates, validate| crate::plan::prune_with_policy(candidates, costs, validate),
    )
}

type CandidateValidator<'a> =
    dyn Fn(&crate::plan::CandidatePlan) -> Result<(), CandidateRejectionReason> + 'a;

fn compile_and_plan_using<F>(
    source: &MediaGraph,
    output_path: &str,
    target: &DesiredTarget,
    unmatched_policy: UnmatchedStreamPolicy,
    constraints: &PlanningConstraints,
    prune: F,
) -> Result<PlanningOutcome, PlanningPipelineError>
where
    F: FnOnce(
        Vec<crate::plan::CandidatePlan>,
        &CandidateValidator<'_>,
    ) -> Result<crate::plan::PrunedCandidates, PlanGenerationError>,
{
    let desired_graph = compile_desired_target(source, output_path, target, unmatched_policy)?;
    let generated = generate_candidates(&diff_graphs(source, &desired_graph))?;
    let validate = |candidate: &crate::plan::CandidatePlan| {
        if candidate
            .operations
            .iter()
            .any(|operation| !constraints.supported_operations.contains(&operation.kind))
        {
            return Err(CandidateRejectionReason::UnsupportedOperation);
        }
        crate::verify::verify_plan_against_graphs(source, &desired_graph, &candidate.operations)
            .map_err(|_| CandidateRejectionReason::UnsafeOrUnverifiable)
    };
    let pruned = prune(generated, &validate)?;
    let selection = select_candidate(pruned)?;
    let explanation = explain_plan_selection(&selection);
    Ok(PlanningOutcome {
        desired_graph,
        selection,
        explanation,
    })
}

#[cfg(test)]
mod tests {
    use super::{PlanningConstraints, PlanningPipelineError, compile_and_plan};
    use crate::classify::SemanticRole;
    use crate::model::{MediaGraph, MediaStream, StreamKind};
    use crate::plan::{CandidateRejectionReason, OperationKind, PlanGenerationError};
    use crate::target::{DesiredTarget, LanguageToken, TargetStream, UnmatchedStreamPolicy};

    #[test]
    fn production_pipeline_selects_remux_and_retains_serializable_rejection()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = MediaGraph {
            source_path: "/library/movie.mp4".to_string(),
            container_formats: vec!["mov".to_string()],
            streams: vec![MediaStream {
                stream_id: 7,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: Some("eng".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["default".to_string()],
            }],
        };
        let target = DesiredTarget {
            target_key: "matroska-remux".to_string(),
            version: 1,
            container: "matroska".to_string(),
            streams: vec![TargetStream {
                stream_key: "main-audio".to_string(),
                kind: StreamKind::Audio,
                role: Some(SemanticRole::Primary),
                language: Some(LanguageToken::parse("eng")?),
                source_binding_key: None,
                optional: false,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                audio_bitrate_bps: None,
                audio_sample_rate_hz: None,
                audio_loudness_profile: None,
                audio_dynamic_range: None,
                video_profile: None,
                video_level: None,
                video_bitrate_bps: None,
                color_primaries: None,
                color_transfer: None,
                color_space: None,
                hdr_format: None,
                title: Some("Main".to_string()),
                dispositions: vec!["default".to_string()],
                subtitle_placement: None,
                image_subtitle_action: None,
            }],
        };

        let outcome = compile_and_plan(
            &source,
            "/workspace/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Reject,
            &PlanningConstraints::all_supported(),
        )?;

        assert_eq!(outcome.selection.selected.id, "remux-only");
        assert_eq!(
            outcome.selection.selected.operations[0].kind,
            OperationKind::Remux
        );
        assert_eq!(outcome.selection.rejected.len(), 1);
        assert_eq!(
            outcome.selection.rejected[0].reason,
            CandidateRejectionReason::DominatedByLowerCost
        );
        let persisted = serde_yaml::to_string(&outcome)?;
        assert!(persisted.contains("dominated_by_lower_cost"));
        assert!(persisted.contains("output_stream_id: 0"));
        let rows: Vec<_> = PlanningConstraints::all_supported()
            .supported_operations
            .into_iter()
            .map(|kind| crate::policy::OperationCost {
                kind,
                cost_weight: crate::plan::operation_cost(kind) * 3,
                enabled: true,
            })
            .collect();
        let costs = crate::policy::OperationCosts::new(&rows)?;
        let explicit = super::compile_and_plan_with_policy(
            &source,
            "/workspace/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Reject,
            &PlanningConstraints::all_supported(),
            &costs,
        )?;
        assert_eq!(explicit.selection.selected, outcome.selection.selected);
        assert_eq!(explicit.selection.selected_cost, 15);
        assert_eq!(explicit.explanation.selected_plan.total_cost, 15);
        Ok(())
    }

    #[test]
    fn unsupported_candidates_fail_with_complete_rejection_evidence()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = MediaGraph {
            source_path: "/library/audio.mp4".to_string(),
            container_formats: vec!["mov".to_string()],
            streams: vec![MediaStream {
                stream_id: 2,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: Some("eng".to_string()),
                title: None,
                dispositions: vec!["default".to_string()],
            }],
        };
        let target = DesiredTarget {
            target_key: "capability-fallback".to_string(),
            version: 1,
            container: "matroska".to_string(),
            streams: vec![TargetStream {
                stream_key: "main-audio".to_string(),
                kind: StreamKind::Audio,
                role: Some(SemanticRole::Primary),
                language: Some(LanguageToken::parse("eng")?),
                source_binding_key: None,
                optional: false,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                audio_bitrate_bps: None,
                audio_sample_rate_hz: None,
                audio_loudness_profile: None,
                audio_dynamic_range: None,
                video_profile: None,
                video_level: None,
                video_bitrate_bps: None,
                color_primaries: None,
                color_transfer: None,
                color_space: None,
                hdr_format: None,
                title: None,
                dispositions: vec!["default".to_string()],
                subtitle_placement: None,
                image_subtitle_action: None,
            }],
        };
        let constraints = PlanningConstraints {
            supported_operations: std::iter::once(OperationKind::AudioTranscode).collect(),
        };

        let result = compile_and_plan(
            &source,
            "/workspace/audio.mkv",
            &target,
            UnmatchedStreamPolicy::Reject,
            &constraints,
        );

        let Err(PlanningPipelineError::Plan(PlanGenerationError::NoValidCandidate { rejected })) =
            result
        else {
            return Err(std::io::Error::other(
                "unsupported operation set did not fail with candidate evidence",
            )
            .into());
        };
        assert_eq!(rejected.len(), 2);
        assert!(
            rejected
                .iter()
                .all(|candidate| candidate.reason == CandidateRejectionReason::UnsupportedOperation)
        );
        Ok(())
    }
}
