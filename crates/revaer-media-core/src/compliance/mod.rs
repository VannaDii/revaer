//! Diff-based media compliance scoring.

use crate::diff::GraphDiff;
use crate::model::StreamKind;

/// Compliance report status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Diff has no compliance violations.
    Compliant,
    /// Diff has at least one compliance violation.
    NonCompliant,
    /// Required media transformation is unsupported.
    Unsupported,
    /// Dry-run produced a plan without execution.
    DryRunPlanned,
    /// Semantic validation failed.
    FailedValidation,
    /// Execution failed.
    FailedExecution,
    /// Post-execution verification failed.
    FailedVerification,
}

/// Compliance violation severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Low severity.
    Low,
    /// Medium severity.
    Medium,
    /// High severity.
    High,
}

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
    /// Violation severity.
    pub severity: Severity,
    /// Source stream id when the violation is stream-scoped.
    pub stream_id: Option<u32>,
}

/// Diff-based compliance report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Compliance status.
    pub status: Status,
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
            severity: Severity::Medium,
            stream_id: Some(*stream_id),
        });
        penalty += 10;
    }

    for stream in &diff.recoded_streams {
        let (kind, severity, stream_penalty) = violation_for_stream_kind(stream.kind);
        violations.push(Violation {
            kind,
            severity,
            stream_id: Some(stream.stream_id),
        });
        penalty += stream_penalty;
    }

    Report {
        status: status_for_violations(&violations),
        score: score_after_penalty(penalty),
        violations,
    }
}

/// Create an empty report for a non-diff terminal status.
#[must_use]
pub const fn report_for_status(status: Status) -> Report {
    Report {
        status,
        score: score_for_status(status),
        violations: Vec::new(),
    }
}

const fn violation_for_stream_kind(kind: StreamKind) -> (ViolationKind, Severity, u16) {
    match kind {
        StreamKind::Audio => (ViolationKind::AudioCodecMismatch, Severity::High, 20),
        StreamKind::Video => (ViolationKind::VideoCodecMismatch, Severity::High, 30),
        StreamKind::Subtitle | StreamKind::Attachment | StreamKind::Chapter => {
            (ViolationKind::StreamCodecMismatch, Severity::Low, 10)
        }
    }
}

const fn status_for_violations(violations: &[Violation]) -> Status {
    if violations.is_empty() {
        Status::Compliant
    } else {
        Status::NonCompliant
    }
}

const fn score_for_status(status: Status) -> u8 {
    match status {
        Status::Compliant | Status::DryRunPlanned => 100,
        Status::NonCompliant
        | Status::Unsupported
        | Status::FailedValidation
        | Status::FailedExecution
        | Status::FailedVerification => 0,
    }
}

fn score_after_penalty(penalty: u16) -> u8 {
    u8::try_from(penalty).map_or(0, |deduction| 100_u8.saturating_sub(deduction))
}
