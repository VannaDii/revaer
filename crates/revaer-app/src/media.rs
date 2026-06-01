//! Media service wiring for API facade.

use async_trait::async_trait;
use revaer_api::app::media::{
    MediaCapabilityReadinessResponse as AppMediaCapabilityReadinessResponse,
    MediaCapabilityRefreshParams,
    MediaCapabilitySnapshotResponse as AppMediaCapabilitySnapshotResponse,
    MediaCompatibilityTargetResponse as AppMediaCompatibilityTargetResponse,
    MediaCompatibilityTargetUpsertParams, MediaDiscoveryAutomationRunParams,
    MediaDiscoveryPreviewParams, MediaDiscoveryPreviewResponse, MediaDiscoveryQueuedJobResponse,
    MediaDiscoveryRunParams, MediaDiscoveryRunResponse, MediaDiscoverySkippedItemResponse,
    MediaFacade, MediaJobArtifactAppendParams, MediaJobArtifactResponse,
    MediaJobCompactAuditAppendParams, MediaJobCompactAuditResponse, MediaJobCreateParams,
    MediaJobOperationAppendParams, MediaJobOperationResponse, MediaJobPhaseAppendParams,
    MediaJobPlanReasonAppendParams, MediaJobPlanReasonResponse, MediaJobResponse,
    MediaJobRetentionResponse as AppMediaJobRetentionResponse, MediaJobRetentionUpdateParams,
    MediaJobVerificationCheckAppendParams, MediaJobVerificationCheckResponse,
    MediaJobViolationAppendParams, MediaJobViolationResponse,
    MediaPolicyResponse as AppMediaPolicyResponse, MediaPolicyUpsertParams,
    MediaProfilePatchParams, MediaProfileResponse, MediaProfileUpsertParams, MediaServiceError,
    MediaServiceErrorKind, MediaYamlApplyResult, MediaYamlProfile, MediaYamlValidationResult,
};
use revaer_data::DataError;
use revaer_data::media::capabilities::{
    CapabilityFeatureRow, CapabilitySnapshotRow, RecordCapabilityEncoderInput,
    RecordCapabilityFeatureInput, RecordCapabilitySnapshotInput,
    complete_capability_snapshot_run_with_executor, record_capability_encoder_with_executor,
    record_capability_feature_with_executor, record_capability_snapshot_with_executor,
    start_capability_snapshot_run_with_executor,
};
use revaer_data::media::configuration::{
    UpdateMediaJobRetentionPolicyInput, UpsertMediaCompatibilityTargetInput,
    UpsertMediaPolicyProfileInput,
};
use revaer_data::media::jobs::{
    AppendMediaJobArtifactInput, AppendMediaJobCompactAuditInput,
    AppendMediaJobVerificationCheckInput, CreateMediaJobInput,
};
use revaer_data::media::profiles::{
    UpdateMediaProfileInput, UpsertMediaProfileInput, upsert_media_profile_with_executor,
};
use revaer_media_core::compile::{MediaProfile, validate_profiles};
use revaer_media_runtime::capabilities::{CapabilityDetectError, CapabilityDetector};
use revaer_runtime::media::MediaStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

const REPLACE_CONFIRMATION_PHRASE: &str = "replace";

#[derive(Debug, Clone, Copy)]
enum DiscoveryRunMode {
    Manual,
    Schedule,
}

/// Production media facade backed by `revaer-runtime` media store.
#[derive(Clone)]
pub(crate) struct MediaService {
    store: MediaStore,
    detector: Arc<dyn CapabilityDetector>,
}

impl MediaService {
    /// Construct media service from runtime media store.
    #[must_use]
    pub(crate) fn new(store: MediaStore, detector: Arc<dyn CapabilityDetector>) -> Self {
        Self { store, detector }
    }

    async fn run_discovery_for_profile(
        &self,
        actor_user_public_id: Uuid,
        media_profile_public_id: Uuid,
        source_paths: &[String],
        mode: DiscoveryRunMode,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        let profile = self
            .store
            .get_profile(media_profile_public_id)
            .await
            .map_err(|err| map_data_error(&err))?
            .ok_or_else(|| {
                MediaServiceError::new(MediaServiceErrorKind::NotFound)
                    .with_code("media_profile_not_found")
            })?;

        ensure_discovery_mode_enabled(mode, profile.schedule_enabled)?;

        if !profile.dry_run_only {
            let latest = self
                .store
                .latest_capability()
                .await
                .map_err(|err| map_data_error(&err))?;
            ensure_execution_capability_snapshot(latest.as_ref())?;
        }

        let previews = build_discovery_previews(
            source_paths,
            &profile.source_root,
            &profile.output_root,
            profile.dry_run_only,
        );
        self.queue_discovery_previews(
            actor_user_public_id,
            media_profile_public_id,
            profile.dry_run_only,
            previews,
        )
        .await
    }

    async fn queue_discovery_previews(
        &self,
        actor_user_public_id: Uuid,
        media_profile_public_id: Uuid,
        dry_run: bool,
        previews: Vec<MediaDiscoveryPreviewResponse>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        let mut queued_jobs = Vec::new();
        let mut skipped = Vec::new();

        for preview in previews {
            match (preview.accepted, preview.output_path) {
                (true, Some(output_path)) => {
                    let media_job_public_id = self
                        .store
                        .create_job(&CreateMediaJobInput {
                            actor_public_id: actor_user_public_id,
                            media_profile_public_id,
                            source_path: &preview.source_path,
                            output_path: Some(output_path.as_str()),
                            dry_run,
                        })
                        .await
                        .map_err(|err| map_data_error(&err))?;
                    queued_jobs.push(MediaDiscoveryQueuedJobResponse {
                        media_job_public_id,
                        source_path: preview.source_path,
                        output_path,
                        dry_run,
                    });
                }
                (_, output_path) => {
                    skipped.push(MediaDiscoverySkippedItemResponse {
                        source_path: preview.source_path,
                        reason: preview.reason.or_else(|| {
                            output_path.map(|_| "media_discovery_source_path_rejected".to_string())
                        }),
                    });
                }
            }
        }

        Ok(MediaDiscoveryRunResponse {
            queued_jobs,
            skipped,
        })
    }
}

#[async_trait]
impl MediaFacade for MediaService {
    async fn media_profile_upsert(
        &self,
        params: MediaProfileUpsertParams<'_>,
    ) -> Result<Uuid, MediaServiceError> {
        ensure_watcher_disabled(params.watcher_enabled)?;
        self.store
            .upsert_profile(&UpsertMediaProfileInput {
                actor_public_id: params.actor_user_public_id,
                profile_key: params.profile_key,
                source_root: params.source_root,
                output_root: params.output_root,
                dry_run_only: params.dry_run_only,
                retention_days: params.retention_days,
                compatibility_target_key: params.compatibility_target_key,
                policy_key: params.policy_key,
                watcher_enabled: params.watcher_enabled,
                schedule_enabled: params.schedule_enabled,
                schedule_interval_minutes: params.schedule_interval_minutes,
            })
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_profile_patch(
        &self,
        params: MediaProfilePatchParams<'_>,
    ) -> Result<Uuid, MediaServiceError> {
        if let Some(watcher_enabled) = params.watcher_enabled {
            ensure_watcher_disabled(watcher_enabled)?;
        }
        self.store
            .update_profile(&UpdateMediaProfileInput {
                actor_public_id: params.actor_user_public_id,
                media_profile_public_id: params.media_profile_public_id,
                source_root: params.source_root,
                output_root: params.output_root,
                dry_run_only: params.dry_run_only,
                retention_days: params.retention_days,
                compatibility_target_key: params.compatibility_target_key,
                policy_key: params.policy_key,
                watcher_enabled: params.watcher_enabled,
                schedule_enabled: params.schedule_enabled,
                schedule_interval_minutes: params.schedule_interval_minutes,
            })
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_profile_list(&self) -> Result<Vec<MediaProfileResponse>, MediaServiceError> {
        self.store
            .list_profiles()
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| MediaProfileResponse {
                        media_profile_public_id: row.media_profile_public_id,
                        profile_key: row.profile_key,
                        source_root: row.source_root,
                        output_root: row.output_root,
                        dry_run_only: row.dry_run_only,
                        retention_days: row.retention_days,
                        compatibility_target_key: row.compatibility_target_key,
                        policy_key: row.policy_key,
                        watcher_enabled: row.watcher_enabled,
                        schedule_enabled: row.schedule_enabled,
                        schedule_interval_minutes: row.schedule_interval_minutes,
                        updated_at: row.updated_at,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_compatibility_target_list(
        &self,
    ) -> Result<Vec<AppMediaCompatibilityTargetResponse>, MediaServiceError> {
        self.store
            .list_compatibility_targets()
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| AppMediaCompatibilityTargetResponse {
                        compatibility_target_key: row.compatibility_target_key,
                        version: row.version,
                        display_name: row.display_name,
                        video_codec: row.video_codec,
                        audio_codec: row.audio_codec,
                        subtitle_policy: row.subtitle_policy,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_compatibility_target_upsert(
        &self,
        params: MediaCompatibilityTargetUpsertParams<'_>,
    ) -> Result<AppMediaCompatibilityTargetResponse, MediaServiceError> {
        self.store
            .upsert_compatibility_target(UpsertMediaCompatibilityTargetInput {
                actor_public_id: params.actor_user_public_id,
                compatibility_target_key: params.compatibility_target_key,
                version: params.version,
                display_name: params.display_name,
                video_codec: params.video_codec,
                audio_codec: params.audio_codec,
                subtitle_policy: params.subtitle_policy,
            })
            .await
            .map(|row| AppMediaCompatibilityTargetResponse {
                compatibility_target_key: row.compatibility_target_key,
                version: row.version,
                display_name: row.display_name,
                video_codec: row.video_codec,
                audio_codec: row.audio_codec,
                subtitle_policy: row.subtitle_policy,
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_policy_list(&self) -> Result<Vec<AppMediaPolicyResponse>, MediaServiceError> {
        self.store
            .list_policy_profiles()
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| AppMediaPolicyResponse {
                        policy_key: row.policy_key,
                        version: row.version,
                        display_name: row.display_name,
                        video_intent: row.video_intent,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_policy_upsert(
        &self,
        params: MediaPolicyUpsertParams<'_>,
    ) -> Result<AppMediaPolicyResponse, MediaServiceError> {
        self.store
            .upsert_policy_profile(UpsertMediaPolicyProfileInput {
                actor_public_id: params.actor_user_public_id,
                policy_key: params.policy_key,
                version: params.version,
                display_name: params.display_name,
                video_intent: params.video_intent,
            })
            .await
            .map(|row| AppMediaPolicyResponse {
                policy_key: row.policy_key,
                version: row.version,
                display_name: row.display_name,
                video_intent: row.video_intent,
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_retention(&self) -> Result<AppMediaJobRetentionResponse, MediaServiceError> {
        let retention = self
            .store
            .get_job_retention_policy()
            .await
            .map_err(|err| map_data_error(&err))?
            .ok_or_else(|| {
                MediaServiceError::new(MediaServiceErrorKind::Storage)
                    .with_code("media_job_retention_policy_missing")
            })?;
        Ok(AppMediaJobRetentionResponse {
            completed_retention_days: retention.completed_retention_days,
            failed_diagnostic_retention_days: retention.failed_diagnostic_retention_days,
        })
    }

    async fn media_job_retention_update(
        &self,
        params: MediaJobRetentionUpdateParams,
    ) -> Result<AppMediaJobRetentionResponse, MediaServiceError> {
        self.store
            .update_job_retention_policy(UpdateMediaJobRetentionPolicyInput {
                actor_public_id: params.actor_user_public_id,
                completed_retention_days: params.completed_retention_days,
                failed_diagnostic_retention_days: params.failed_diagnostic_retention_days,
            })
            .await
            .map(|row| AppMediaJobRetentionResponse {
                completed_retention_days: row.completed_retention_days,
                failed_diagnostic_retention_days: row.failed_diagnostic_retention_days,
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_create(
        &self,
        params: MediaJobCreateParams<'_>,
    ) -> Result<Uuid, MediaServiceError> {
        let profile = self
            .store
            .get_profile(params.media_profile_public_id)
            .await
            .map_err(|err| map_data_error(&err))?
            .ok_or_else(|| {
                MediaServiceError::new(MediaServiceErrorKind::NotFound)
                    .with_code("media_profile_not_found")
            })?;
        ensure_media_job_paths_within_profile(
            params.source_path,
            params.output_path,
            &profile.source_root,
            &profile.output_root,
        )?;

        if !params.dry_run {
            if profile.dry_run_only
                && params.replace_confirmation != Some(REPLACE_CONFIRMATION_PHRASE)
            {
                return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
                    .with_code("media_job_replace_confirmation_required"));
            }

            let latest = self
                .store
                .latest_capability()
                .await
                .map_err(|err| map_data_error(&err))?;
            ensure_execution_capability_snapshot(latest.as_ref())?;
        }

        self.store
            .create_job(&CreateMediaJobInput {
                actor_public_id: params.actor_user_public_id,
                media_profile_public_id: params.media_profile_public_id,
                source_path: params.source_path,
                output_path: params.output_path,
                dry_run: params.dry_run,
            })
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_discovery_preview(
        &self,
        params: MediaDiscoveryPreviewParams<'_>,
    ) -> Result<Vec<MediaDiscoveryPreviewResponse>, MediaServiceError> {
        let profile = self
            .store
            .get_profile(params.media_profile_public_id)
            .await
            .map_err(|err| map_data_error(&err))?
            .ok_or_else(|| {
                MediaServiceError::new(MediaServiceErrorKind::NotFound)
                    .with_code("media_profile_not_found")
            })?;

        Ok(build_discovery_previews(
            params.source_paths,
            &profile.source_root,
            &profile.output_root,
            profile.dry_run_only,
        ))
    }

    async fn media_discovery_run(
        &self,
        params: MediaDiscoveryRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        self.run_discovery_for_profile(
            params.actor_user_public_id,
            params.media_profile_public_id,
            params.source_paths,
            DiscoveryRunMode::Manual,
        )
        .await
    }

    async fn media_discovery_schedule_run(
        &self,
        params: MediaDiscoveryAutomationRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        self.run_discovery_for_profile(
            params.actor_user_public_id,
            params.media_profile_public_id,
            params.source_paths,
            DiscoveryRunMode::Schedule,
        )
        .await
    }

    async fn media_discovery_watcher_run(
        &self,
        _params: MediaDiscoveryAutomationRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_discovery_watcher_unavailable"))
    }

    async fn media_job_list(
        &self,
        media_profile_public_id: Uuid,
        status: Option<&str>,
    ) -> Result<Vec<MediaJobResponse>, MediaServiceError> {
        self.store
            .list_jobs(media_profile_public_id, status)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| MediaJobResponse {
                        media_job_public_id: row.media_job_public_id,
                        source_path: row.source_path,
                        output_path: row.output_path,
                        status: row.status_text,
                        dry_run: row.dry_run,
                        queued_at: row.queued_at,
                        started_at: row.started_at,
                        completed_at: row.completed_at,
                        last_error: row.last_error,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_get(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Option<MediaJobResponse>, MediaServiceError> {
        self.store
            .get_job(media_job_public_id)
            .await
            .map(|row_opt| {
                row_opt.map(|row| MediaJobResponse {
                    media_job_public_id: row.media_job_public_id,
                    source_path: row.source_path,
                    output_path: row.output_path,
                    status: row.status_text,
                    dry_run: row.dry_run,
                    queued_at: row.queued_at,
                    started_at: row.started_at,
                    completed_at: row.completed_at,
                    last_error: row.last_error,
                })
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_cancel(&self, media_job_public_id: Uuid) -> Result<(), MediaServiceError> {
        self.store
            .cancel_job(media_job_public_id)
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_retry(&self, media_job_public_id: Uuid) -> Result<(), MediaServiceError> {
        self.store
            .retry_job(media_job_public_id)
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_phase_append(
        &self,
        params: MediaJobPhaseAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        self.store
            .append_job_phase(
                params.media_job_public_id,
                params.phase_index,
                params.phase_name,
                params.phase_status,
                params.details_text,
            )
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_operation_append(
        &self,
        params: MediaJobOperationAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        self.store
            .append_job_operation(
                params.media_job_public_id,
                params.operation_index,
                params.operation_kind,
                params.stream_id,
                params.command_bin,
                params.args,
            )
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_operation_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobOperationResponse>, MediaServiceError> {
        self.store
            .list_job_operations(media_job_public_id)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| MediaJobOperationResponse {
                        operation_index: row.operation_index,
                        operation_kind: row.operation_kind,
                        stream_id: row.stream_id,
                        command_bin: row.command_bin,
                        arg_1: row.arg_1,
                        arg_2: row.arg_2,
                        arg_3: row.arg_3,
                        arg_4: row.arg_4,
                        arg_5: row.arg_5,
                        created_at: row.created_at,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_violation_append(
        &self,
        params: MediaJobViolationAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        self.store
            .append_job_violation(
                params.media_job_public_id,
                params.violation_index,
                params.violation_kind,
                params.severity,
                params.stream_id,
            )
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_violation_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobViolationResponse>, MediaServiceError> {
        self.store
            .list_job_violations(media_job_public_id)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| MediaJobViolationResponse {
                        violation_index: row.violation_index,
                        violation_kind: row.violation_kind,
                        severity: row.severity,
                        stream_id: row.stream_id,
                        created_at: row.created_at,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_plan_reason_append(
        &self,
        params: MediaJobPlanReasonAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        self.store
            .append_job_plan_reason(
                params.media_job_public_id,
                params.reason_index,
                params.candidate_index,
                params.selected,
                params.reason_code,
                params.reason_text,
            )
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_plan_reason_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobPlanReasonResponse>, MediaServiceError> {
        self.store
            .list_job_plan_reasons(media_job_public_id)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| MediaJobPlanReasonResponse {
                        reason_index: row.reason_index,
                        candidate_index: row.candidate_index,
                        selected: row.selected,
                        reason_code: row.reason_code,
                        reason_text: row.reason_text,
                        created_at: row.created_at,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_verification_check_append(
        &self,
        params: MediaJobVerificationCheckAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        self.store
            .append_job_verification_check(&AppendMediaJobVerificationCheckInput {
                media_job_public_id: params.media_job_public_id,
                check_index: params.check_index,
                check_kind: params.check_kind,
                check_status: params.check_status,
                expected_value: params.expected_value,
                actual_value: params.actual_value,
                details_text: params.details_text,
            })
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_verification_check_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobVerificationCheckResponse>, MediaServiceError> {
        self.store
            .list_job_verification_checks(media_job_public_id)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| MediaJobVerificationCheckResponse {
                        check_index: row.check_index,
                        check_kind: row.check_kind,
                        check_status: row.check_status,
                        expected_value: row.expected_value,
                        actual_value: row.actual_value,
                        details_text: row.details_text,
                        created_at: row.created_at,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_artifact_append(
        &self,
        params: MediaJobArtifactAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        self.store
            .append_job_artifact(&AppendMediaJobArtifactInput {
                media_job_public_id: params.media_job_public_id,
                artifact_index: params.artifact_index,
                artifact_kind: params.artifact_kind,
                artifact_path: params.artifact_path,
                size_bytes: params.size_bytes,
                content_type: params.content_type,
            })
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_artifact_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobArtifactResponse>, MediaServiceError> {
        self.store
            .list_job_artifacts(media_job_public_id)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| MediaJobArtifactResponse {
                        artifact_index: row.artifact_index,
                        artifact_kind: row.artifact_kind,
                        artifact_path: row.artifact_path,
                        size_bytes: row.size_bytes,
                        content_type: row.content_type,
                        created_at: row.created_at,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_compact_audit_append(
        &self,
        params: MediaJobCompactAuditAppendParams<'_>,
    ) -> Result<(), MediaServiceError> {
        self.store
            .append_job_compact_audit(&AppendMediaJobCompactAuditInput {
                media_job_public_id: params.media_job_public_id,
                audit_index: params.audit_index,
                fact_kind: params.fact_kind,
                fact_text: params.fact_text,
            })
            .await
            .map_err(|err| map_data_error(&err))
    }

    async fn media_job_compact_audit_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobCompactAuditResponse>, MediaServiceError> {
        self.store
            .list_job_compact_audits(media_job_public_id)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| MediaJobCompactAuditResponse {
                        audit_index: row.audit_index,
                        fact_kind: row.fact_kind,
                        fact_text: row.fact_text,
                        created_at: row.created_at,
                    })
                    .collect()
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_capability_refresh(
        &self,
        params: MediaCapabilityRefreshParams,
    ) -> Result<i64, MediaServiceError> {
        let detector = self.detector.clone();
        let snapshot = tokio::task::spawn_blocking(move || detector.detect())
            .await
            .map_err(|_| {
                MediaServiceError::new(MediaServiceErrorKind::Storage)
                    .with_code("media_capability_refresh_join_failed")
            })?
            .map_err(|error| map_detect_error(&error))?;
        if !snapshot.is_valid()
            || snapshot
                .encoders
                .iter()
                .all(|encoder| encoder.trim().is_empty())
        {
            return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_capability_refresh_invalid"));
        }

        let snapshot_run_public_id = Uuid::new_v4();
        let mut transaction = self.store.pool().begin().await.map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_capability_refresh_transaction_start_failed")
        })?;
        start_capability_snapshot_run_with_executor(
            &mut *transaction,
            params.actor_user_public_id,
            snapshot_run_public_id,
        )
        .await
        .map_err(|err| map_data_error(&err))?;

        let mut seen = BTreeSet::new();
        let mut last_snapshot_id = None;
        for codec in &snapshot.codecs {
            let normalized = codec.trim().to_ascii_lowercase();
            if normalized.is_empty() || !seen.insert(normalized.clone()) {
                continue;
            }
            let support = snapshot.codec_capability(&normalized);
            let snapshot_id = record_capability_snapshot_with_executor(
                &mut *transaction,
                &RecordCapabilitySnapshotInput {
                    actor_public_id: params.actor_user_public_id,
                    snapshot_run_public_id: Some(snapshot_run_public_id),
                    ffmpeg_version: &snapshot.ffmpeg_version,
                    ffprobe_version: &snapshot.ffprobe_version,
                    codec_name: &normalized,
                    encode_supported: support.encode_supported,
                    decode_supported: support.decode_supported,
                },
            )
            .await
            .map_err(|err| map_data_error(&err))?;
            last_snapshot_id = Some(snapshot_id);
        }
        let mut seen_encoders = BTreeSet::new();
        for encoder in &snapshot.encoders {
            let normalized = encoder.trim().to_ascii_lowercase();
            if normalized.is_empty() || !seen_encoders.insert(normalized.clone()) {
                continue;
            }
            record_capability_encoder_with_executor(
                &mut *transaction,
                &RecordCapabilityEncoderInput {
                    actor_public_id: params.actor_user_public_id,
                    snapshot_run_public_id,
                    encoder_name: &normalized,
                },
            )
            .await
            .map_err(|err| map_data_error(&err))?;
        }
        for feature in capability_feature_inputs(&snapshot) {
            record_capability_feature_with_executor(
                &mut *transaction,
                &RecordCapabilityFeatureInput {
                    actor_public_id: params.actor_user_public_id,
                    snapshot_run_public_id,
                    feature_family: feature.family,
                    feature_name: feature.name,
                    supported: feature.supported,
                    detail_text: feature.detail,
                },
            )
            .await
            .map_err(|err| map_data_error(&err))?;
        }

        let snapshot_id = last_snapshot_id.ok_or_else(|| {
            MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_capability_refresh_invalid")
        })?;
        complete_capability_snapshot_run_with_executor(&mut *transaction, snapshot_run_public_id)
            .await
            .map_err(|err| map_data_error(&err))?;
        transaction.commit().await.map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_capability_refresh_transaction_commit_failed")
        })?;
        Ok(snapshot_id)
    }

    async fn media_capability_latest(
        &self,
    ) -> Result<Option<AppMediaCapabilitySnapshotResponse>, MediaServiceError> {
        self.store
            .latest_capability()
            .await
            .map(|row_opt| {
                row_opt.map(|row| AppMediaCapabilitySnapshotResponse {
                    media_capability_snapshot_id: row.media_capability_snapshot_id,
                    snapshot_run_public_id: row.snapshot_run_public_id,
                    ffmpeg_version: row.ffmpeg_version,
                    ffprobe_version: row.ffprobe_version,
                    codecs: row
                        .codecs
                        .into_iter()
                        .map(
                            |codec| revaer_api::app::media::MediaCapabilityCodecResponse {
                                codec_name: codec.codec_name,
                                encode_supported: codec.encode_supported,
                                decode_supported: codec.decode_supported,
                            },
                        )
                        .collect(),
                    encoders: row.encoders,
                    decoders: feature_names(&row.features, "decoder", true),
                    muxers: feature_names(&row.features, "muxer", true),
                    demuxers: feature_names(&row.features, "demuxer", true),
                    subtitle_support: feature_names(&row.features, "subtitle", true),
                    hardware_accelerators: feature_names(&row.features, "hardware", true),
                    filesystem_utilities: feature_names(&row.features, "filesystem", true),
                    utility_capabilities: feature_names(&row.features, "utility", true),
                    license_mode: feature_names(&row.features, "license", true)
                        .into_iter()
                        .next()
                        .unwrap_or_default(),
                    compliance_links: feature_names(&row.features, "compliance", true),
                    absent_capabilities: feature_names(&row.features, "absent", false),
                    features: row
                        .features
                        .into_iter()
                        .map(
                            |feature| revaer_api::app::media::MediaCapabilityFeatureResponse {
                                feature_family: feature.feature_family,
                                feature_name: feature.feature_name,
                                supported: feature.supported,
                                detail_text: feature.detail_text,
                            },
                        )
                        .collect(),
                    observed_at: row.observed_at,
                })
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_capability_readiness(
        &self,
    ) -> Result<AppMediaCapabilityReadinessResponse, MediaServiceError> {
        let snapshot = self.media_capability_latest().await?;
        let (ready, reason) = match snapshot.as_ref() {
            None => (false, Some("media_capability_snapshot_missing".to_string())),
            Some(item)
                if item.ffmpeg_version.trim().is_empty()
                    || item.ffprobe_version.trim().is_empty()
                    || item.codecs.is_empty()
                    || item.encoders.is_empty()
                    || item.decoders.is_empty()
                    || item.muxers.is_empty()
                    || item.demuxers.is_empty()
                    || item.filesystem_utilities.is_empty()
                    || item.utility_capabilities.is_empty()
                    || item.license_mode.trim().is_empty()
                    || item
                        .codecs
                        .iter()
                        .any(|codec| codec.codec_name.trim().is_empty()) =>
            {
                (false, Some("media_capability_snapshot_invalid".to_string()))
            }
            Some(_) => (true, None),
        };

        Ok(AppMediaCapabilityReadinessResponse {
            ready,
            reason,
            snapshot,
        })
    }

    async fn media_yaml_export(
        &self,
        include_local_paths: bool,
    ) -> Result<String, MediaServiceError> {
        let profiles = self
            .media_profile_list()
            .await?
            .into_iter()
            .map(|profile| MediaYamlProfile {
                source_root: export_root(
                    include_local_paths,
                    &profile.profile_key,
                    "source",
                    &profile.source_root,
                ),
                output_root: export_root(
                    include_local_paths,
                    &profile.profile_key,
                    "output",
                    &profile.output_root,
                ),
                profile_key: profile.profile_key,
                dry_run_only: true,
                retention_days: profile.retention_days,
                compatibility_target_key: profile.compatibility_target_key,
                policy_key: profile.policy_key,
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            })
            .collect();

        let payload = MediaYamlBundle {
            version: "revaer.media.v1".to_string(),
            profiles,
        };

        serde_yaml::to_string(&payload).map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_yaml_serialize_failed")
        })
    }

    async fn media_yaml_validate(
        &self,
        yaml_payload: &str,
    ) -> Result<MediaYamlValidationResult, MediaServiceError> {
        let parsed = parse_yaml_bundle(yaml_payload)?;
        let issues = validate_yaml_bundle(&parsed);
        Ok(MediaYamlValidationResult {
            version: parsed.version,
            valid: issues.is_empty(),
            issues,
            profiles: parsed.profiles,
        })
    }

    async fn media_yaml_apply(
        &self,
        actor_user_public_id: Uuid,
        yaml_payload: &str,
    ) -> Result<MediaYamlApplyResult, MediaServiceError> {
        let validation = self.media_yaml_validate(yaml_payload).await?;
        if !validation.valid {
            return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_yaml_validation_failed"));
        }
        let mut transaction = self.store.pool().begin().await.map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_yaml_apply_transaction_start_failed")
        })?;

        let mut media_profile_public_ids = Vec::with_capacity(validation.profiles.len());
        for profile in validation.profiles {
            let source_root =
                importable_yaml_root(&profile.profile_key, "source", &profile.source_root);
            let output_root =
                importable_yaml_root(&profile.profile_key, "output", &profile.output_root);
            let profile_id = upsert_media_profile_with_executor(
                &mut *transaction,
                &UpsertMediaProfileInput {
                    actor_public_id: actor_user_public_id,
                    profile_key: &profile.profile_key,
                    source_root: &source_root,
                    output_root: &output_root,
                    dry_run_only: true,
                    retention_days: profile.retention_days,
                    compatibility_target_key: profile.compatibility_target_key.as_deref(),
                    policy_key: &profile.policy_key,
                    watcher_enabled: false,
                    schedule_enabled: false,
                    schedule_interval_minutes: None,
                },
            )
            .await
            .map_err(|err| map_data_error(&err))?;
            media_profile_public_ids.push(profile_id);
        }
        transaction.commit().await.map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_yaml_apply_transaction_commit_failed")
        })?;

        Ok(MediaYamlApplyResult {
            forced_dry_run: true,
            media_profile_public_ids,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MediaYamlBundle {
    version: String,
    profiles: Vec<MediaYamlProfile>,
}

#[derive(Debug, Clone, Copy)]
struct CapabilityFeatureInput<'a> {
    family: &'a str,
    name: &'a str,
    supported: bool,
    detail: Option<&'a str>,
}

fn capability_feature_inputs(
    snapshot: &revaer_media_runtime::capabilities::CapabilitySnapshot,
) -> Vec<CapabilityFeatureInput<'_>> {
    let mut features = Vec::new();
    push_feature_names(&mut features, "encoder", &snapshot.encoders, true, None);
    push_feature_names(&mut features, "decoder", &snapshot.decoders, true, None);
    push_feature_names(&mut features, "muxer", &snapshot.muxers, true, None);
    push_feature_names(&mut features, "demuxer", &snapshot.demuxers, true, None);
    push_feature_names(
        &mut features,
        "hardware",
        &snapshot.hardware_accelerators,
        true,
        None,
    );
    push_feature_names(
        &mut features,
        "subtitle",
        &snapshot.subtitle_support,
        true,
        None,
    );
    push_feature_names(
        &mut features,
        "filesystem",
        &snapshot.filesystem_utilities,
        true,
        None,
    );
    push_feature_names(
        &mut features,
        "utility",
        &snapshot.utility_capabilities,
        true,
        None,
    );
    features.push(CapabilityFeatureInput {
        family: "license",
        name: snapshot.license_mode.as_str(),
        supported: true,
        detail: None,
    });
    push_feature_names(
        &mut features,
        "compliance",
        &snapshot.compliance_links,
        true,
        None,
    );
    push_feature_names(
        &mut features,
        "absent",
        &snapshot.absent_capabilities,
        false,
        Some("excluded from runtime"),
    );
    features
}

fn push_feature_names<'a>(
    features: &mut Vec<CapabilityFeatureInput<'a>>,
    family: &'a str,
    names: &'a [String],
    supported: bool,
    detail: Option<&'a str>,
) {
    for name in names {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            continue;
        }
        features.push(CapabilityFeatureInput {
            family,
            name: trimmed,
            supported,
            detail,
        });
    }
}

fn feature_names(features: &[CapabilityFeatureRow], family: &str, supported: bool) -> Vec<String> {
    let mut names = BTreeSet::new();
    for feature in features {
        if feature.supported == supported && feature.feature_family.eq_ignore_ascii_case(family) {
            names.insert(feature.feature_name.clone());
        }
    }
    names.into_iter().collect()
}

fn parse_yaml_bundle(yaml_payload: &str) -> Result<MediaYamlBundle, MediaServiceError> {
    let trimmed = yaml_payload.trim();
    if trimmed.is_empty() {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_yaml_payload_missing"));
    }

    serde_yaml::from_str::<MediaYamlBundle>(trimmed).map_err(|_| {
        MediaServiceError::new(MediaServiceErrorKind::Invalid).with_code("media_yaml_invalid")
    })
}

fn export_root(
    include_local_paths: bool,
    profile_key: &str,
    root_kind: &str,
    root: &str,
) -> String {
    if include_local_paths {
        root.to_string()
    } else {
        format!(
            "${{revaer.{root_kind}_root:{}}}",
            profile_key.trim().to_ascii_lowercase()
        )
    }
}

fn is_unresolved_export_root(root: &str) -> bool {
    let trimmed = root.trim();
    trimmed.starts_with("${revaer.") && trimmed.ends_with('}')
}

fn importable_yaml_root(profile_key: &str, root_kind: &str, root: &str) -> String {
    if !is_unresolved_export_root(root) {
        return root.to_string();
    }

    format!(
        "/var/lib/revaer/media-import-drafts/{}/{}",
        portable_profile_slug(profile_key),
        root_kind
    )
}

fn portable_profile_slug(profile_key: &str) -> String {
    let mut slug = String::new();
    for character in profile_key.trim().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if character == '-' || character == '_' {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "profile".to_string()
    } else {
        trimmed.to_string()
    }
}

fn validate_yaml_bundle(bundle: &MediaYamlBundle) -> Vec<String> {
    let mut issues = Vec::new();
    if bundle.version != "revaer.media.v1" {
        issues.push("media_yaml_version_unsupported".to_string());
    }
    if bundle.profiles.is_empty() {
        issues.push("media_yaml_profiles_missing".to_string());
    }

    let mut profiles = Vec::with_capacity(bundle.profiles.len());
    for profile in &bundle.profiles {
        if !(1..=3650).contains(&profile.retention_days) {
            issues.push("media_yaml_profile_retention_days_out_of_bounds".to_string());
        }
        profiles.push(MediaProfile {
            key: profile.profile_key.clone(),
            source_root: importable_yaml_root(&profile.profile_key, "source", &profile.source_root),
            output_root: importable_yaml_root(&profile.profile_key, "output", &profile.output_root),
            dry_run_only: true,
        });
    }

    if let Err(err) = validate_profiles(&profiles) {
        let code = match err {
            revaer_media_core::compile::ValidationError::OverlappingRoots => {
                "media_yaml_profile_roots_overlap"
            }
            revaer_media_core::compile::ValidationError::EmptyProfileKey => {
                "media_yaml_profile_key_missing"
            }
            revaer_media_core::compile::ValidationError::EmptySourceRoot
            | revaer_media_core::compile::ValidationError::EmptyOutputRoot => {
                "media_yaml_profile_root_missing"
            }
            revaer_media_core::compile::ValidationError::DuplicateProfileKey => {
                "media_yaml_profile_key_duplicate"
            }
            revaer_media_core::compile::ValidationError::OverlappingProfileRoots => {
                "media_yaml_profile_source_roots_overlap"
            }
        };
        issues.push(code.to_string());
    }

    issues
}

fn map_data_error(error: &DataError) -> MediaServiceError {
    let sqlstate = error.database_code();
    let detail = error.database_detail().map(ToOwned::to_owned);

    let detail_kind = match detail.as_deref() {
        Some("app_user_not_found" | "media_profile_not_found" | "media_job_not_found") => {
            MediaServiceErrorKind::NotFound
        }
        Some("media_job_cancel_invalid_status" | "media_job_retry_invalid_status") => {
            MediaServiceErrorKind::Conflict
        }
        Some("media_profile_roots_overlap") => MediaServiceErrorKind::Invalid,
        _ => MediaServiceErrorKind::Storage,
    };
    let kind = if sqlstate.as_deref() == Some("22P02") {
        MediaServiceErrorKind::Invalid
    } else {
        detail_kind
    };

    let mut service_error = MediaServiceError::new(kind);
    if let Some(code) = detail {
        service_error = service_error.with_code(code);
    }
    if let Some(sqlstate) = sqlstate {
        service_error = service_error.with_sqlstate(sqlstate);
    }

    service_error
}

fn map_detect_error(error: &CapabilityDetectError) -> MediaServiceError {
    match error {
        CapabilityDetectError::Unavailable => {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_capability_refresh_unavailable")
        }
        CapabilityDetectError::CommandFailed(_) | CapabilityDetectError::OutputMalformed(_) => {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_capability_refresh_failed")
        }
    }
}

pub(crate) fn ensure_execution_capability_snapshot(
    snapshot: Option<&CapabilitySnapshotRow>,
) -> Result<(), MediaServiceError> {
    let Some(snapshot) = snapshot else {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_capability_snapshot_missing"));
    };

    let valid = !snapshot.ffmpeg_version.trim().is_empty()
        && !snapshot.ffprobe_version.trim().is_empty()
        && !snapshot.codecs.is_empty()
        && !snapshot.encoders.is_empty()
        && snapshot_has_supported_feature(snapshot, "decoder")
        && snapshot_has_supported_feature(snapshot, "muxer")
        && snapshot_has_supported_feature(snapshot, "demuxer")
        && snapshot_has_supported_feature(snapshot, "filesystem")
        && snapshot_has_supported_feature(snapshot, "utility")
        && snapshot_has_supported_feature(snapshot, "license")
        && snapshot
            .codecs
            .iter()
            .all(|codec| !codec.codec_name.trim().is_empty());
    if !valid {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_capability_snapshot_invalid"));
    }

    Ok(())
}

fn snapshot_has_supported_feature(snapshot: &CapabilitySnapshotRow, family: &str) -> bool {
    snapshot.features.iter().any(|feature| {
        feature.supported
            && feature.feature_family.eq_ignore_ascii_case(family)
            && !feature.feature_name.trim().is_empty()
    })
}

fn ensure_media_job_paths_within_profile(
    source_path: &str,
    output_path: Option<&str>,
    source_root: &str,
    output_root: &str,
) -> Result<(), MediaServiceError> {
    if !path_is_within_root(source_path, source_root) {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_job_source_path_outside_profile_root"));
    }

    if output_path.is_some_and(|path| !path_is_within_root(path, output_root)) {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_job_output_path_outside_profile_root"));
    }

    Ok(())
}

fn ensure_discovery_mode_enabled(
    mode: DiscoveryRunMode,
    schedule_enabled: bool,
) -> Result<(), MediaServiceError> {
    match mode {
        DiscoveryRunMode::Manual => Ok(()),
        DiscoveryRunMode::Schedule if schedule_enabled => Ok(()),
        DiscoveryRunMode::Schedule => Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_discovery_schedule_disabled")),
    }
}

fn ensure_watcher_disabled(watcher_enabled: bool) -> Result<(), MediaServiceError> {
    if watcher_enabled {
        Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_discovery_watcher_unavailable"))
    } else {
        Ok(())
    }
}

fn path_is_within_root(path: &str, root: &str) -> bool {
    let Some(normalized_path) = clean_absolute_path(path) else {
        return false;
    };
    let Some(normalized_root) = clean_absolute_path(root) else {
        return false;
    };
    normalized_path == normalized_root || normalized_path.starts_with(normalized_root)
}

fn normalize_path_text(path: &str) -> String {
    if let Some(normalized) = clean_absolute_path(path)
        && let Some(text) = normalized.to_str()
    {
        return text.to_string();
    }
    path.trim().trim_end_matches('/').to_string()
}

fn clean_absolute_path(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let input = Path::new(trimmed);
    if !input.is_absolute() {
        return None;
    }

    let mut normalized = PathBuf::new();
    for component in input.components() {
        match component {
            Component::RootDir => normalized = PathBuf::from("/"),
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

fn derive_profile_output_path(
    source_path: &str,
    source_root: &str,
    output_root: &str,
) -> Option<String> {
    let normalized_source_path = clean_absolute_path(source_path)?;
    let normalized_source_root = clean_absolute_path(source_root)?;
    let normalized_output_root = clean_absolute_path(output_root)?;
    if normalized_source_path != normalized_source_root
        && !normalized_source_path.starts_with(&normalized_source_root)
    {
        return None;
    }
    if normalized_source_path == normalized_source_root {
        return normalized_output_root.to_str().map(str::to_string);
    }

    let suffix = normalized_source_path
        .strip_prefix(&normalized_source_root)
        .ok()?;
    normalized_output_root
        .join(suffix)
        .to_str()
        .map(str::to_string)
}

pub(crate) fn build_discovery_previews(
    source_paths: &[String],
    source_root: &str,
    output_root: &str,
    dry_run: bool,
) -> Vec<MediaDiscoveryPreviewResponse> {
    source_paths
        .iter()
        .map(|source_path| {
            let normalized_source_path = normalize_path_text(source_path);
            let output_path =
                derive_profile_output_path(&normalized_source_path, source_root, output_root);
            let accepted = output_path.is_some();
            MediaDiscoveryPreviewResponse {
                source_path: normalized_source_path,
                output_path,
                dry_run,
                accepted,
                reason: if accepted {
                    None
                } else {
                    Some("media_discovery_source_path_outside_profile_root".to_string())
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        MediaService, ensure_execution_capability_snapshot, map_data_error, map_detect_error,
        parse_yaml_bundle, path_is_within_root, validate_yaml_bundle,
    };
    use revaer_api::app::media::MediaServiceErrorKind;
    use revaer_api::app::media::{
        MediaCapabilityRefreshParams, MediaCompatibilityTargetUpsertParams,
        MediaDiscoveryAutomationRunParams, MediaDiscoveryPreviewParams, MediaDiscoveryRunParams,
        MediaFacade, MediaJobArtifactAppendParams, MediaJobCompactAuditAppendParams,
        MediaJobCreateParams, MediaJobOperationAppendParams, MediaJobPhaseAppendParams,
        MediaJobPlanReasonAppendParams, MediaJobRetentionUpdateParams,
        MediaJobVerificationCheckAppendParams, MediaJobViolationAppendParams,
        MediaPolicyUpsertParams, MediaProfileUpsertParams,
    };
    use revaer_data::DataError;
    use revaer_data::indexers::app_users::{app_user_create, app_user_verify_email};
    use revaer_data::media::capabilities::CapabilitySnapshotRow;
    use revaer_media_runtime::capabilities::CapabilityDetectError;
    use revaer_media_runtime::capabilities::CapabilityDetector;
    use revaer_media_runtime::capabilities::{CapabilitySnapshot, CodecCapability};
    use revaer_runtime::media::MediaStore;
    use revaer_test_support::postgres::start_postgres;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;
    use uuid::Uuid;

    #[derive(Clone)]
    struct StaticDetector {
        snapshot: CapabilitySnapshot,
    }

    impl CapabilityDetector for StaticDetector {
        fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError> {
            Ok(self.snapshot.clone())
        }
    }

    #[derive(Clone)]
    struct PanicDetector;

    impl CapabilityDetector for PanicDetector {
        fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError> {
            panic!("detector panic");
        }
    }

    async fn setup_media_service(
        detector: Arc<dyn CapabilityDetector>,
    ) -> anyhow::Result<Option<(MediaService, Uuid)>> {
        let Ok(postgres) = start_postgres() else {
            return Ok(None);
        };
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(postgres.connection_string())
            .await?;
        let mut migrator = sqlx::migrate!("../revaer-data/migrations");
        migrator.set_ignore_missing(true);
        migrator.run(&pool).await?;

        let store = MediaStore::new(pool);
        let email = format!("media-app-{}@example.invalid", Uuid::new_v4());
        let actor_user_public_id = app_user_create(store.pool(), &email, "Media App").await?;
        app_user_verify_email(store.pool(), actor_user_public_id).await?;
        Ok(Some((
            MediaService::new(store, detector),
            actor_user_public_id,
        )))
    }

    fn static_detector() -> Arc<dyn CapabilityDetector> {
        Arc::new(StaticDetector {
            snapshot: CapabilitySnapshot {
                ffmpeg_version: "7.1".to_string(),
                ffprobe_version: "7.1".to_string(),
                codecs: vec![
                    "h264".to_string(),
                    "hevc".to_string(),
                    "h264".to_string(),
                    "  ".to_string(),
                ],
                codec_support: vec![
                    CodecCapability {
                        name: "h264".to_string(),
                        encode_supported: false,
                        decode_supported: true,
                    },
                    CodecCapability {
                        name: "hevc".to_string(),
                        encode_supported: true,
                        decode_supported: true,
                    },
                ],
                encoders: vec!["libx265".to_string()],
                decoders: vec!["hevc".to_string()],
                muxers: vec!["matroska".to_string()],
                demuxers: vec!["matroska".to_string()],
                hardware_accelerators: Vec::new(),
                subtitle_support: vec!["subrip".to_string()],
                filesystem_utilities: vec!["atomic_rename".to_string()],
                utility_capabilities: vec!["ffmpeg".to_string(), "ffprobe".to_string()],
                license_mode: "gpl".to_string(),
                compliance_links: vec!["/app/compliance/SOURCE-OFFER.txt".to_string()],
                absent_capabilities: vec!["--enable-nonfree".to_string()],
            },
        })
    }

    fn valid_capability_features() -> Vec<revaer_data::media::capabilities::CapabilityFeatureRow> {
        [
            ("decoder", "hevc", true),
            ("muxer", "matroska", true),
            ("demuxer", "matroska", true),
            ("filesystem", "atomic_rename", true),
            ("utility", "ffmpeg", true),
            ("license", "gpl", true),
        ]
        .into_iter()
        .map(|(feature_family, feature_name, supported)| {
            revaer_data::media::capabilities::CapabilityFeatureRow {
                feature_family: feature_family.to_string(),
                feature_name: feature_name.to_string(),
                supported,
                detail_text: None,
            }
        })
        .collect()
    }

    #[test]
    fn reject_missing_capability_snapshot() {
        let result = ensure_execution_capability_snapshot(None);
        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_capability_snapshot_missing".to_string())
        );
    }

    #[test]
    fn reject_invalid_capability_snapshot() {
        let row = CapabilitySnapshotRow {
            media_capability_snapshot_id: 1,
            snapshot_run_public_id: Uuid::new_v4(),
            ffmpeg_version: String::new(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec![revaer_data::media::capabilities::CapabilityCodecRow {
                codec_name: "h264".to_string(),
                encode_supported: true,
                decode_supported: true,
            }],
            encoders: vec!["libx265".to_string()],
            features: Vec::new(),
            observed_at: chrono::Utc::now(),
        };
        let result = ensure_execution_capability_snapshot(Some(&row));
        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_capability_snapshot_invalid".to_string())
        );
    }

    #[test]
    fn accept_valid_capability_snapshot() {
        let row = CapabilitySnapshotRow {
            media_capability_snapshot_id: 1,
            snapshot_run_public_id: Uuid::new_v4(),
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec![revaer_data::media::capabilities::CapabilityCodecRow {
                codec_name: "h264".to_string(),
                encode_supported: true,
                decode_supported: true,
            }],
            encoders: vec!["libx265".to_string()],
            features: valid_capability_features(),
            observed_at: chrono::Utc::now(),
        };
        assert!(ensure_execution_capability_snapshot(Some(&row)).is_ok());
    }

    #[test]
    fn path_is_within_root_rejects_sibling_prefixes() {
        assert!(path_is_within_root(
            "/input/app-media/video.mkv",
            "/input/app-media"
        ));
        assert!(!path_is_within_root(
            "/input/app-media-other/video.mkv",
            "/input/app-media"
        ));
    }

    #[test]
    fn path_is_within_root_rejects_traversal_and_relative_paths() {
        assert!(!path_is_within_root(
            "/media/source/../outside/movie.mkv",
            "/media/source"
        ));
        assert!(!path_is_within_root(
            "/media/source/movie.mkv",
            "/media/source/../outside"
        ));
        assert!(!path_is_within_root(
            "media/source/movie.mkv",
            "/media/source"
        ));
    }

    #[tokio::test]
    async fn media_job_create_requires_replace_confirmation_for_dry_run_profile_override()
    -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        let profile_id = upsert_app_media_profile(&service, actor_user_public_id).await?;

        let result = service
            .media_job_create(MediaJobCreateParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/app-media/video.mkv",
                output_path: Some("/output/app-media/video.mkv"),
                dry_run: false,
                replace_confirmation: None,
            })
            .await;
        let Err(err) = result else {
            panic!("expected dry-run override without confirmation to be rejected");
        };

        assert_eq!(err.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(err.code(), Some("media_job_replace_confirmation_required"));
        Ok(())
    }

    #[tokio::test]
    async fn media_job_create_rejects_paths_outside_profile_roots() -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        let profile_id = upsert_app_media_profile(&service, actor_user_public_id).await?;

        let source_result = service
            .media_job_create(MediaJobCreateParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/other/video.mkv",
                output_path: Some("/output/app-media/video.mkv"),
                dry_run: true,
                replace_confirmation: None,
            })
            .await;
        let source_error = source_result.expect_err("outside source root should be rejected");
        assert_eq!(source_error.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(
            source_error.code(),
            Some("media_job_source_path_outside_profile_root")
        );

        let output_result = service
            .media_job_create(MediaJobCreateParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/app-media/video.mkv",
                output_path: Some("/output/other/video.mkv"),
                dry_run: true,
                replace_confirmation: None,
            })
            .await;
        let output_error = output_result.expect_err("outside output root should be rejected");
        assert_eq!(output_error.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(
            output_error.code(),
            Some("media_job_output_path_outside_profile_root")
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_job_create_rejects_traversal_outside_profile_roots() -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        let profile_id = upsert_app_media_profile(&service, actor_user_public_id).await?;

        let source_result = service
            .media_job_create(MediaJobCreateParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/app-media/../outside/video.mkv",
                output_path: Some("/output/app-media/video.mkv"),
                dry_run: true,
                replace_confirmation: None,
            })
            .await;
        let source_error = source_result.expect_err("source traversal should be rejected");
        assert_eq!(
            source_error.code(),
            Some("media_job_source_path_outside_profile_root")
        );

        let output_result = service
            .media_job_create(MediaJobCreateParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/app-media/video.mkv",
                output_path: Some("/output/app-media/../outside/video.mkv"),
                dry_run: true,
                replace_confirmation: None,
            })
            .await;
        let output_error = output_result.expect_err("output traversal should be rejected");
        assert_eq!(
            output_error.code(),
            Some("media_job_output_path_outside_profile_root")
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_discovery_preview_marks_profile_paths_and_rejects_outside_sources()
    -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        let profile_id = upsert_app_media_profile(&service, actor_user_public_id).await?;
        let source_paths = vec![
            "/input/app-media/show/episode.mkv".to_string(),
            "/input/app-media-other/show/episode.mkv".to_string(),
            "/input/app-media/../outside/episode.mkv".to_string(),
        ];

        let previews = service
            .media_discovery_preview(MediaDiscoveryPreviewParams {
                media_profile_public_id: profile_id,
                source_paths: &source_paths,
            })
            .await?;

        assert_eq!(previews.len(), 3);
        assert!(previews[0].accepted);
        assert!(previews[0].dry_run);
        assert_eq!(
            previews[0].output_path.as_deref(),
            Some("/output/app-media/show/episode.mkv")
        );
        assert!(!previews[1].accepted);
        assert_eq!(
            previews[1].reason.as_deref(),
            Some("media_discovery_source_path_outside_profile_root")
        );
        assert!(!previews[2].accepted);
        assert_eq!(
            previews[2].reason.as_deref(),
            Some("media_discovery_source_path_outside_profile_root")
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_discovery_run_queues_accepted_profile_paths_and_skips_rejected_sources()
    -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        let profile_id = upsert_app_media_profile(&service, actor_user_public_id).await?;
        let source_paths = vec![
            "/input/app-media/show/episode.mkv".to_string(),
            "/input/app-media-other/show/episode.mkv".to_string(),
        ];

        let response = service
            .media_discovery_run(MediaDiscoveryRunParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_paths: &source_paths,
            })
            .await?;

        assert_eq!(response.queued_jobs.len(), 1);
        assert_eq!(response.skipped.len(), 1);
        assert_eq!(
            response.queued_jobs[0].source_path,
            "/input/app-media/show/episode.mkv"
        );
        assert_eq!(
            response.queued_jobs[0].output_path,
            "/output/app-media/show/episode.mkv"
        );
        assert!(response.queued_jobs[0].dry_run);
        assert_eq!(
            response.skipped[0].reason.as_deref(),
            Some("media_discovery_source_path_outside_profile_root")
        );
        let jobs = service.media_job_list(profile_id, Some("queued")).await?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].source_path, response.queued_jobs[0].source_path);
        Ok(())
    }

    #[tokio::test]
    async fn media_discovery_schedule_run_requires_enabled_schedule_and_queues_paths()
    -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        let disabled_profile_id = upsert_app_media_profile(&service, actor_user_public_id).await?;
        let source_paths = vec!["/input/app-media/show/episode.mkv".to_string()];

        let disabled_result = service
            .media_discovery_schedule_run(MediaDiscoveryAutomationRunParams {
                actor_user_public_id,
                media_profile_public_id: disabled_profile_id,
                source_paths: &source_paths,
            })
            .await;
        let Err(disabled_error) = disabled_result else {
            return Err(anyhow::anyhow!("disabled schedule should reject"));
        };
        assert_eq!(disabled_error.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(
            disabled_error.code(),
            Some("media_discovery_schedule_disabled")
        );

        let enabled_profile_id =
            upsert_app_media_profile_with_discovery(&service, actor_user_public_id, false, true)
                .await?;
        let response = service
            .media_discovery_schedule_run(MediaDiscoveryAutomationRunParams {
                actor_user_public_id,
                media_profile_public_id: enabled_profile_id,
                source_paths: &source_paths,
            })
            .await?;

        assert_eq!(response.queued_jobs.len(), 1);
        assert!(response.skipped.is_empty());
        assert_eq!(
            response.queued_jobs[0].output_path,
            "/output/app-media/show/episode.mkv"
        );
        let jobs = service
            .media_job_list(enabled_profile_id, Some("queued"))
            .await?;
        assert_eq!(jobs.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn media_discovery_watcher_run_is_unavailable() -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        let profile_id = upsert_app_media_profile(&service, actor_user_public_id).await?;
        let source_paths = vec!["/input/app-media/show/episode.mkv".to_string()];

        let result = service
            .media_discovery_watcher_run(MediaDiscoveryAutomationRunParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_paths: &source_paths,
            })
            .await;
        let Err(error) = result else {
            return Err(anyhow::anyhow!("watcher discovery should be unavailable"));
        };
        assert_eq!(error.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(error.code(), Some("media_discovery_watcher_unavailable"));
        Ok(())
    }

    #[test]
    fn parse_yaml_bundle_rejects_empty_and_invalid_payloads() {
        let empty = parse_yaml_bundle(" \n\t ");
        assert_eq!(
            empty.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_yaml_payload_missing".to_string())
        );

        let invalid = parse_yaml_bundle("profiles: [");
        assert_eq!(
            invalid.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_yaml_invalid".to_string())
        );
    }

    #[test]
    fn validate_yaml_bundle_reports_version_shape_and_profile_errors() {
        let bundle = parse_yaml_bundle(
            "version: revaer.media.v2\nprofiles:\n  - profile_key: tv\n    source_root: /data\n    output_root: /data\n    dry_run_only: false\n    retention_days: 0\n",
        )
        .expect("bundle");
        let issues = validate_yaml_bundle(&bundle);
        assert!(issues.contains(&"media_yaml_version_unsupported".to_string()));
        assert!(issues.contains(&"media_yaml_profile_retention_days_out_of_bounds".to_string()));
        assert!(issues.contains(&"media_yaml_profile_roots_overlap".to_string()));
    }

    #[test]
    fn map_data_error_projects_expected_kind_and_codes() {
        let not_found = DataError::JobFailed {
            operation: "job",
            job_key: "job",
            error_code: Some("P0001".to_string()),
            error_detail: Some("media_job_not_found".to_string()),
        };
        let mapped = map_data_error(&not_found);
        assert_eq!(mapped.kind(), MediaServiceErrorKind::NotFound);
        assert_eq!(mapped.code(), Some("media_job_not_found"));
        assert_eq!(mapped.sqlstate(), Some("P0001"));

        let conflict = DataError::JobFailed {
            operation: "job",
            job_key: "job",
            error_code: None,
            error_detail: Some("media_job_retry_invalid_status".to_string()),
        };
        assert_eq!(
            map_data_error(&conflict).kind(),
            MediaServiceErrorKind::Conflict
        );

        let invalid = DataError::JobFailed {
            operation: "job",
            job_key: "job",
            error_code: None,
            error_detail: Some("media_profile_roots_overlap".to_string()),
        };
        assert_eq!(
            map_data_error(&invalid).kind(),
            MediaServiceErrorKind::Invalid
        );

        let invalid_cast = DataError::JobFailed {
            operation: "job",
            job_key: "job",
            error_code: Some("22P02".to_string()),
            error_detail: None,
        };
        let mapped_invalid_cast = map_data_error(&invalid_cast);
        assert_eq!(mapped_invalid_cast.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(mapped_invalid_cast.sqlstate(), Some("22P02"));

        let invalid_cast_overrides_conflict_detail = DataError::JobFailed {
            operation: "job",
            job_key: "job",
            error_code: Some("22P02".to_string()),
            error_detail: Some("media_job_retry_invalid_status".to_string()),
        };
        assert_eq!(
            map_data_error(&invalid_cast_overrides_conflict_detail).kind(),
            MediaServiceErrorKind::Invalid
        );
    }

    #[test]
    fn map_detect_error_projects_expected_codes() {
        assert_eq!(
            map_detect_error(&CapabilityDetectError::Unavailable).code(),
            Some("media_capability_refresh_unavailable")
        );
        assert_eq!(
            map_detect_error(&CapabilityDetectError::CommandFailed("x".to_string())).code(),
            Some("media_capability_refresh_failed")
        );
        assert_eq!(
            map_detect_error(&CapabilityDetectError::OutputMalformed("x".to_string())).code(),
            Some("media_capability_refresh_failed")
        );
    }

    #[tokio::test]
    async fn media_service_round_trips_profile_job_yaml_and_capability_paths() -> anyhow::Result<()>
    {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };

        let profile_id = upsert_app_media_profile(&service, actor_user_public_id).await?;
        assert_profile_is_listed(&service, profile_id).await?;

        let job_id = create_app_media_job(&service, actor_user_public_id, profile_id).await?;
        assert_job_records_round_trip(&service, profile_id, job_id).await?;
        assert_job_cancel_retry(&service, job_id).await?;

        assert_capability_refresh_uses_detected_support(&service, actor_user_public_id).await?;

        let yaml = service.media_yaml_export(false).await?;
        let validation = service.media_yaml_validate(&yaml).await?;
        assert!(validation.valid);
        let portable_apply = service
            .media_yaml_apply(actor_user_public_id, &yaml)
            .await?;
        assert!(portable_apply.forced_dry_run);
        let profiles = service.media_profile_list().await?;
        assert!(profiles.iter().any(|profile| {
            profile
                .source_root
                .starts_with("/var/lib/revaer/media-import-drafts/")
                && profile.dry_run_only
                && !profile.watcher_enabled
                && !profile.schedule_enabled
        }));

        let local_yaml = service.media_yaml_export(true).await?;
        let local_validation = service.media_yaml_validate(&local_yaml).await?;
        assert!(local_validation.valid);
        let applied = service
            .media_yaml_apply(actor_user_public_id, &local_yaml)
            .await?;
        assert!(applied.forced_dry_run);

        Ok(())
    }

    #[tokio::test]
    async fn media_service_round_trips_configuration_catalogs_and_retention() -> anyhow::Result<()>
    {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };

        let target = service
            .media_compatibility_target_upsert(MediaCompatibilityTargetUpsertParams {
                actor_user_public_id,
                compatibility_target_key: "plex-living-room",
                version: 2,
                display_name: "Plex living room",
                video_codec: "hevc",
                audio_codec: "aac",
                subtitle_policy: "selected",
            })
            .await?;
        assert_eq!(target.compatibility_target_key, "plex-living-room");
        assert_eq!(target.version, 2);
        assert_eq!(target.subtitle_policy, "selected");
        assert!(
            service
                .media_compatibility_target_list()
                .await?
                .iter()
                .any(
                    |target| target.compatibility_target_key == "plex-living-room"
                        && target.version == 2
                )
        );

        let policy = service
            .media_policy_upsert(MediaPolicyUpsertParams {
                actor_user_public_id,
                policy_key: "living-room",
                version: 3,
                display_name: "Living room",
                video_intent: "general",
            })
            .await?;
        assert_eq!(policy.policy_key, "living-room");
        assert_eq!(policy.version, 3);
        assert_eq!(policy.video_intent, "general");
        assert!(
            service
                .media_policy_list()
                .await?
                .iter()
                .any(|policy| policy.policy_key == "living-room" && policy.version == 3)
        );

        let retention = service
            .media_job_retention_update(MediaJobRetentionUpdateParams {
                actor_user_public_id,
                completed_retention_days: 45,
                failed_diagnostic_retention_days: 90,
            })
            .await?;
        assert_eq!(retention.completed_retention_days, 45);
        assert_eq!(retention.failed_diagnostic_retention_days, 90);
        let refreshed_retention = service.media_job_retention().await?;
        assert_eq!(refreshed_retention.completed_retention_days, 45);
        assert_eq!(refreshed_retention.failed_diagnostic_retention_days, 90);
        Ok(())
    }

    async fn create_app_media_job(
        service: &MediaService,
        actor_user_public_id: Uuid,
        profile_id: Uuid,
    ) -> anyhow::Result<Uuid> {
        service
            .media_job_create(MediaJobCreateParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/app-media/video.mkv",
                output_path: Some("/output/app-media/video.mkv"),
                dry_run: true,
                replace_confirmation: None,
            })
            .await
            .map_err(Into::into)
    }

    async fn assert_job_records_round_trip(
        service: &MediaService,
        profile_id: Uuid,
        job_id: Uuid,
    ) -> anyhow::Result<()> {
        append_plan_phase_and_operation(service, job_id).await?;
        append_job_violation(service, job_id).await?;
        assert!(service.media_job_get(job_id).await?.is_some());
        assert!(
            !service
                .media_job_list(profile_id, Some("queued"))
                .await?
                .is_empty()
        );
        assert_eq!(service.media_job_operation_list(job_id).await?.len(), 1);
        let violations = service.media_job_violation_list(job_id).await?;
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_kind, "video_codec_mismatch");
        append_job_planning_records(service, job_id).await?;
        append_job_artifact_and_audit(service, job_id).await
    }

    async fn append_job_planning_records(
        service: &MediaService,
        job_id: Uuid,
    ) -> anyhow::Result<()> {
        service
            .media_job_plan_reason_append(MediaJobPlanReasonAppendParams {
                media_job_public_id: job_id,
                reason_index: 0,
                candidate_index: Some(0),
                selected: true,
                reason_code: "least_cost_selected",
                reason_text: "Selected the least-cost compliant candidate.",
            })
            .await?;
        let plan_reasons = service.media_job_plan_reason_list(job_id).await?;
        assert_eq!(plan_reasons.len(), 1);
        assert_eq!(plan_reasons[0].reason_code, "least_cost_selected");
        service
            .media_job_verification_check_append(MediaJobVerificationCheckAppendParams {
                media_job_public_id: job_id,
                check_index: 0,
                check_kind: "duration",
                check_status: "passed",
                expected_value: Some("3600.0"),
                actual_value: Some("3599.9"),
                details_text: Some("within tolerance"),
            })
            .await?;
        let verification_checks = service.media_job_verification_check_list(job_id).await?;
        assert_eq!(verification_checks.len(), 1);
        assert_eq!(verification_checks[0].check_kind, "duration");
        assert_eq!(verification_checks[0].check_status, "passed");
        Ok(())
    }

    async fn append_job_artifact_and_audit(
        service: &MediaService,
        job_id: Uuid,
    ) -> anyhow::Result<()> {
        service
            .media_job_artifact_append(MediaJobArtifactAppendParams {
                media_job_public_id: job_id,
                artifact_index: 0,
                artifact_kind: "ffprobe_json",
                artifact_path: "jobs/abc/ffprobe.json",
                size_bytes: Some(2048),
                content_type: Some("application/json"),
            })
            .await?;
        let artifacts = service.media_job_artifact_list(job_id).await?;
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].artifact_kind, "ffprobe_json");
        assert_eq!(artifacts[0].artifact_path, "jobs/abc/ffprobe.json");
        service
            .media_job_compact_audit_append(MediaJobCompactAuditAppendParams {
                media_job_public_id: job_id,
                audit_index: 0,
                fact_kind: "replacement",
                fact_text: "source preserved before replace",
            })
            .await?;
        let audits = service.media_job_compact_audit_list(job_id).await?;
        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].fact_kind, "replacement");
        assert_eq!(audits[0].fact_text, "source preserved before replace");
        Ok(())
    }

    async fn upsert_app_media_profile(
        service: &MediaService,
        actor_user_public_id: Uuid,
    ) -> anyhow::Result<Uuid> {
        upsert_app_media_profile_with_discovery(service, actor_user_public_id, false, false).await
    }

    async fn upsert_app_media_profile_with_discovery(
        service: &MediaService,
        actor_user_public_id: Uuid,
        watcher_enabled: bool,
        schedule_enabled: bool,
    ) -> anyhow::Result<Uuid> {
        service
            .media_profile_upsert(MediaProfileUpsertParams {
                actor_user_public_id,
                profile_key: "app-media",
                source_root: "/input/app-media",
                output_root: "/output/app-media",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled,
                schedule_enabled,
                schedule_interval_minutes: schedule_enabled.then_some(60),
            })
            .await
            .map_err(Into::into)
    }

    async fn assert_profile_is_listed(
        service: &MediaService,
        profile_id: Uuid,
    ) -> anyhow::Result<()> {
        let profiles = service.media_profile_list().await?;
        assert!(
            profiles
                .iter()
                .any(|profile| profile.media_profile_public_id == profile_id)
        );
        Ok(())
    }

    async fn append_plan_phase_and_operation(
        service: &MediaService,
        job_id: Uuid,
    ) -> anyhow::Result<()> {
        service
            .media_job_phase_append(MediaJobPhaseAppendParams {
                media_job_public_id: job_id,
                phase_index: 0,
                phase_name: "plan",
                phase_status: "queued",
                details_text: Some("ok"),
            })
            .await?;
        service
            .media_job_operation_append(MediaJobOperationAppendParams {
                media_job_public_id: job_id,
                operation_index: 0,
                operation_kind: "remux",
                stream_id: None,
                command_bin: "ffmpeg",
                args: [Some("-i"), Some("in.mkv"), Some("-c"), Some("copy"), None],
            })
            .await?;
        Ok(())
    }

    async fn append_job_violation(service: &MediaService, job_id: Uuid) -> anyhow::Result<()> {
        service
            .media_job_violation_append(MediaJobViolationAppendParams {
                media_job_public_id: job_id,
                violation_index: 0,
                violation_kind: "video_codec_mismatch",
                severity: "high",
                stream_id: Some(0),
            })
            .await?;
        Ok(())
    }

    async fn assert_job_cancel_retry(service: &MediaService, job_id: Uuid) -> anyhow::Result<()> {
        service.media_job_cancel(job_id).await?;
        let cancelled_job = service
            .media_job_get(job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("cancelled media job missing"))?;
        assert_eq!(cancelled_job.status, "cancelled");
        service.media_job_retry(job_id).await?;
        let retried_job = service
            .media_job_get(job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("retried media job missing"))?;
        assert_eq!(retried_job.status, "queued");
        Ok(())
    }

    async fn assert_capability_refresh_uses_detected_support(
        service: &MediaService,
        actor_user_public_id: Uuid,
    ) -> anyhow::Result<()> {
        let cap_id = service
            .media_capability_refresh(MediaCapabilityRefreshParams {
                actor_user_public_id,
            })
            .await?;
        assert!(cap_id > 0);
        let latest = service.media_capability_latest().await?;
        assert!(latest.is_some());
        let Some(latest) = latest else {
            return Ok(());
        };
        assert_eq!(latest.codecs.len(), 2);
        assert!(latest.codecs.iter().any(|codec| codec.codec_name == "h264"
            && !codec.encode_supported
            && codec.decode_supported));
        assert!(latest.codecs.iter().any(|codec| codec.codec_name == "hevc"
            && codec.encode_supported
            && codec.decode_supported));
        assert_eq!(latest.encoders, vec!["libx265".to_string()]);
        assert!(service.media_capability_readiness().await?.ready);
        Ok(())
    }

    #[tokio::test]
    async fn media_capability_refresh_reports_join_failure_when_detector_panics()
    -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) =
            setup_media_service(Arc::new(PanicDetector)).await?
        else {
            return Ok(());
        };

        let result = service
            .media_capability_refresh(MediaCapabilityRefreshParams {
                actor_user_public_id,
            })
            .await;
        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_capability_refresh_join_failed".to_string())
        );

        Ok(())
    }
}
