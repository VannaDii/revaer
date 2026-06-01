//! Diff-based media compliance scoring.

use crate::diff::GraphDiff;
use crate::model::StreamKind;

/// Normalized compliance violation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// Source stream is absent from the desired output graph.
    RemovedStream,
    /// Audio stream codec differs from the desired output graph.
    AudioCodecMismatch,
    /// Video stream codec differs from the desired output graph.
    VideoCodecMismatch,
    /// Non-audio/video stream codec differs from the desired output graph.
    StreamCodecMismatch,
}

/// Normalized compliance violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Violation {
    /// Violation kind.
    pub kind: ViolationKind,
    /// Source stream id when the violation is stream-scoped.
    pub stream_id: Option<u32>,
}

/// Diff-based compliance report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Deterministic score from 0 to 100.
    pub score: u8,
    /// Violations contributing to the score.
    pub violations: Vec<Violation>,
}

/// Score graph compliance from a source-to-desired diff.
#[must_use]
pub fn score_diff(diff: &GraphDiff) -> Report {
    let mut violations = Vec::new();
    let mut penalty = 0_u16;

    for stream_id in &diff.removed_streams {
        violations.push(Violation {
            kind: ViolationKind::RemovedStream,
            stream_id: Some(*stream_id),
        });
        penalty += 10;
    }

    for stream in &diff.recoded_streams {
        let (kind, stream_penalty) = violation_for_stream_kind(stream.kind);
        violations.push(Violation {
            kind,
            stream_id: Some(stream.stream_id),
        });
        penalty += stream_penalty;
    }

    Report {
        score: score_after_penalty(penalty),
        violations,
    }
}

const fn violation_for_stream_kind(kind: StreamKind) -> (ViolationKind, u16) {
    match kind {
        StreamKind::Audio => (ViolationKind::AudioCodecMismatch, 20),
        StreamKind::Video => (ViolationKind::VideoCodecMismatch, 30),
        StreamKind::Subtitle | StreamKind::Attachment | StreamKind::Chapter => {
            (ViolationKind::StreamCodecMismatch, 10)
        }
    }
}

fn score_after_penalty(penalty: u16) -> u8 {
    u8::try_from(penalty).map_or(0, |deduction| 100_u8.saturating_sub(deduction))
}
