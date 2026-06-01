//! Pure helpers for the media UI feature.

use crate::features::media::state::MediaJobDiagnostics;
use uuid::Uuid;

pub(crate) fn media_jobs_path(media_profile_public_id: Option<Uuid>) -> String {
    media_profile_public_id.map_or_else(
        || "/v1/media/jobs".to_string(),
        |profile_id| format!("/v1/media/jobs?media_profile_public_id={profile_id}"),
    )
}

pub(crate) fn media_job_operations_path(media_job_public_id: Uuid) -> String {
    format!("/v1/media/jobs/{media_job_public_id}/operations")
}

pub(crate) fn media_job_violations_path(media_job_public_id: Uuid) -> String {
    format!("/v1/media/jobs/{media_job_public_id}/violations")
}

pub(crate) fn media_job_plan_reasons_path(media_job_public_id: Uuid) -> String {
    format!("/v1/media/jobs/{media_job_public_id}/plan-reasons")
}

pub(crate) fn media_job_verification_checks_path(media_job_public_id: Uuid) -> String {
    format!("/v1/media/jobs/{media_job_public_id}/verification-checks")
}

pub(crate) fn media_job_artifacts_path(media_job_public_id: Uuid) -> String {
    format!("/v1/media/jobs/{media_job_public_id}/artifacts")
}

pub(crate) fn media_job_compact_audits_path(media_job_public_id: Uuid) -> String {
    format!("/v1/media/jobs/{media_job_public_id}/compact-audits")
}

pub(crate) fn summarize_media_job_diagnostics(diagnostics: &MediaJobDiagnostics) -> String {
    let selected_reason = diagnostics
        .plan_reasons
        .iter()
        .find(|row| row.selected)
        .map_or("none", |row| row.reason_code.as_str());
    format!(
        "operations={} violations={} reasons={} checks={} artifacts={} audits={} selected_reason={selected_reason}",
        diagnostics.operations.len(),
        diagnostics.violations.len(),
        diagnostics.plan_reasons.len(),
        diagnostics.verification_checks.len(),
        diagnostics.artifacts.len(),
        diagnostics.compact_audits.len()
    )
}

#[cfg(test)]
mod tests {
    use super::{
        media_job_artifacts_path, media_job_compact_audits_path, media_job_operations_path,
        media_job_plan_reasons_path, media_job_verification_checks_path, media_job_violations_path,
        media_jobs_path, summarize_media_job_diagnostics,
    };
    use crate::features::media::state::MediaJobDiagnostics;
    use crate::models::{
        MediaJobArtifactResponse, MediaJobCompactAuditResponse, MediaJobOperationResponse,
        MediaJobPlanReasonResponse, MediaJobVerificationCheckResponse, MediaJobViolationResponse,
    };
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn media_jobs_path_includes_profile_filter() {
        let profile_id = Uuid::from_u128(1);

        assert_eq!(
            media_jobs_path(Some(profile_id)),
            format!("/v1/media/jobs?media_profile_public_id={profile_id}")
        );
    }

    #[test]
    fn media_jobs_path_without_profile_uses_collection_route() {
        assert_eq!(media_jobs_path(None), "/v1/media/jobs");
    }

    #[test]
    fn media_job_diagnostic_paths_use_nested_job_routes() {
        let job_id = Uuid::from_u128(2);

        assert_eq!(
            media_job_operations_path(job_id),
            format!("/v1/media/jobs/{job_id}/operations")
        );
        assert_eq!(
            media_job_violations_path(job_id),
            format!("/v1/media/jobs/{job_id}/violations")
        );
        assert_eq!(
            media_job_plan_reasons_path(job_id),
            format!("/v1/media/jobs/{job_id}/plan-reasons")
        );
        assert_eq!(
            media_job_verification_checks_path(job_id),
            format!("/v1/media/jobs/{job_id}/verification-checks")
        );
        assert_eq!(
            media_job_artifacts_path(job_id),
            format!("/v1/media/jobs/{job_id}/artifacts")
        );
        assert_eq!(
            media_job_compact_audits_path(job_id),
            format!("/v1/media/jobs/{job_id}/compact-audits")
        );
    }

    #[test]
    fn summarize_media_job_diagnostics_counts_rows_and_selected_reasons() {
        let created_at = Utc::now();
        let diagnostics = MediaJobDiagnostics {
            operations: vec![MediaJobOperationResponse {
                operation_index: 0,
                operation_kind: "remux".to_string(),
                stream_id: Some(1),
                command_bin: "ffmpeg".to_string(),
                arg_1: Some("-i".to_string()),
                arg_2: None,
                arg_3: None,
                arg_4: None,
                arg_5: None,
                created_at,
            }],
            violations: vec![MediaJobViolationResponse {
                violation_index: 0,
                violation_kind: "audio_codec".to_string(),
                severity: "medium".to_string(),
                stream_id: Some(2),
                created_at,
            }],
            plan_reasons: vec![
                MediaJobPlanReasonResponse {
                    reason_index: 0,
                    candidate_index: Some(0),
                    selected: false,
                    reason_code: "too_expensive".to_string(),
                    reason_text: "Rejected higher cost candidate.".to_string(),
                    created_at,
                },
                MediaJobPlanReasonResponse {
                    reason_index: 1,
                    candidate_index: Some(1),
                    selected: true,
                    reason_code: "least_cost".to_string(),
                    reason_text: "Selected least cost candidate.".to_string(),
                    created_at,
                },
            ],
            verification_checks: vec![MediaJobVerificationCheckResponse {
                check_index: 0,
                check_kind: "duration".to_string(),
                check_status: "passed".to_string(),
                expected_value: Some("3600.0".to_string()),
                actual_value: Some("3599.9".to_string()),
                details_text: Some("within tolerance".to_string()),
                created_at,
            }],
            artifacts: vec![MediaJobArtifactResponse {
                artifact_index: 0,
                artifact_kind: "ffprobe_json".to_string(),
                artifact_path: "jobs/abc/ffprobe.json".to_string(),
                size_bytes: Some(2048),
                content_type: Some("application/json".to_string()),
                created_at,
            }],
            compact_audits: vec![MediaJobCompactAuditResponse {
                audit_index: 0,
                fact_kind: "replacement".to_string(),
                fact_text: "source preserved before replace".to_string(),
                created_at,
            }],
        };

        let summary = summarize_media_job_diagnostics(&diagnostics);

        assert_eq!(
            summary,
            "operations=1 violations=1 reasons=2 checks=1 artifacts=1 audits=1 selected_reason=least_cost"
        );
    }
}
