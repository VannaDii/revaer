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

pub(crate) fn summarize_media_job_diagnostics(diagnostics: &MediaJobDiagnostics) -> String {
    let selected_reason = diagnostics
        .plan_reasons
        .iter()
        .find(|row| row.selected)
        .map_or("none", |row| row.reason_code.as_str());
    format!(
        "operations={} violations={} reasons={} selected_reason={selected_reason}",
        diagnostics.operations.len(),
        diagnostics.violations.len(),
        diagnostics.plan_reasons.len()
    )
}

#[cfg(test)]
mod tests {
    use super::{
        media_job_operations_path, media_job_plan_reasons_path, media_job_violations_path,
        media_jobs_path, summarize_media_job_diagnostics,
    };
    use crate::features::media::state::MediaJobDiagnostics;
    use crate::models::{
        MediaJobOperationResponse, MediaJobPlanReasonResponse, MediaJobViolationResponse,
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
        };

        let summary = summarize_media_job_diagnostics(&diagnostics);

        assert_eq!(
            summary,
            "operations=1 violations=1 reasons=2 selected_reason=least_cost"
        );
    }
}
