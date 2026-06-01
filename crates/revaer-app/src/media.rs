//! Media service wiring for API facade.

use async_trait::async_trait;
use revaer_api::app::media::{
    MediaCapabilityReadinessResponse as AppMediaCapabilityReadinessResponse,
    MediaCapabilityRecordParams, MediaCapabilityRefreshParams,
    MediaCapabilitySnapshotResponse as AppMediaCapabilitySnapshotResponse, MediaFacade,
    MediaJobCreateParams, MediaJobOperationAppendParams, MediaJobOperationResponse,
    MediaJobPhaseAppendParams, MediaJobPlanReasonAppendParams, MediaJobPlanReasonResponse,
    MediaJobResponse, MediaJobVerificationCheckAppendParams, MediaJobVerificationCheckResponse,
    MediaJobViolationAppendParams, MediaJobViolationResponse, MediaProfilePatchParams,
    MediaProfileResponse, MediaProfileUpsertParams, MediaServiceError, MediaServiceErrorKind,
    MediaYamlApplyResult, MediaYamlProfile, MediaYamlValidationResult,
};
use revaer_data::DataError;
use revaer_data::media::capabilities::CapabilitySnapshotRow;
use revaer_data::media::capabilities::RecordCapabilitySnapshotInput;
use revaer_data::media::jobs::{AppendMediaJobVerificationCheckInput, CreateMediaJobInput};
use revaer_data::media::profiles::{
    UpdateMediaProfileInput, UpsertMediaProfileInput, upsert_media_profile_with_executor,
};
use revaer_media_core::compile::{MediaProfile, validate_profiles};
use revaer_media_runtime::capabilities::{CapabilityDetectError, CapabilityDetector};
use revaer_runtime::media::MediaStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use uuid::Uuid;

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
}

#[async_trait]
impl MediaFacade for MediaService {
    async fn media_profile_upsert(
        &self,
        params: MediaProfileUpsertParams<'_>,
    ) -> Result<Uuid, MediaServiceError> {
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

    async fn media_job_create(
        &self,
        params: MediaJobCreateParams<'_>,
    ) -> Result<Uuid, MediaServiceError> {
        if !params.dry_run {
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

    async fn media_capability_record(
        &self,
        params: MediaCapabilityRecordParams<'_>,
    ) -> Result<i64, MediaServiceError> {
        self.store
            .record_capability(&RecordCapabilitySnapshotInput {
                actor_public_id: params.actor_user_public_id,
                ffmpeg_version: params.ffmpeg_version,
                ffprobe_version: params.ffprobe_version,
                codec_name: params.codec_name,
                encode_supported: params.encode_supported,
                decode_supported: params.decode_supported,
            })
            .await
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
        if !snapshot.is_valid() {
            return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_capability_refresh_invalid"));
        }

        let mut seen = BTreeSet::new();
        let mut last_snapshot_id = None;
        for codec in &snapshot.codecs {
            let normalized = codec.trim().to_ascii_lowercase();
            if normalized.is_empty() || !seen.insert(normalized.clone()) {
                continue;
            }
            let support = snapshot.codec_capability(&normalized);
            let snapshot_id = self
                .store
                .record_capability(&RecordCapabilitySnapshotInput {
                    actor_public_id: params.actor_user_public_id,
                    ffmpeg_version: &snapshot.ffmpeg_version,
                    ffprobe_version: &snapshot.ffprobe_version,
                    codec_name: &normalized,
                    encode_supported: support.encode_supported,
                    decode_supported: support.decode_supported,
                })
                .await
                .map_err(|err| map_data_error(&err))?;
            last_snapshot_id = Some(snapshot_id);
        }

        last_snapshot_id.ok_or_else(|| {
            MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_capability_refresh_invalid")
        })
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
                    ffmpeg_version: row.ffmpeg_version,
                    ffprobe_version: row.ffprobe_version,
                    codec_name: row.codec_name,
                    encode_supported: row.encode_supported,
                    decode_supported: row.decode_supported,
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
                    || item.codec_name.trim().is_empty() =>
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

    async fn media_yaml_export(&self) -> Result<String, MediaServiceError> {
        let profiles = self
            .media_profile_list()
            .await?
            .into_iter()
            .map(|profile| MediaYamlProfile {
                profile_key: profile.profile_key,
                source_root: profile.source_root,
                output_root: profile.output_root,
                dry_run_only: profile.dry_run_only,
                retention_days: profile.retention_days,
                compatibility_target_key: profile.compatibility_target_key,
                policy_key: profile.policy_key,
                watcher_enabled: profile.watcher_enabled,
                schedule_enabled: profile.schedule_enabled,
                schedule_interval_minutes: profile.schedule_interval_minutes,
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
            let profile_id = upsert_media_profile_with_executor(
                &mut *transaction,
                &UpsertMediaProfileInput {
                    actor_public_id: actor_user_public_id,
                    profile_key: &profile.profile_key,
                    source_root: &profile.source_root,
                    output_root: &profile.output_root,
                    dry_run_only: true,
                    retention_days: profile.retention_days,
                    compatibility_target_key: profile.compatibility_target_key.as_deref(),
                    policy_key: &profile.policy_key,
                    watcher_enabled: profile.watcher_enabled,
                    schedule_enabled: profile.schedule_enabled,
                    schedule_interval_minutes: profile.schedule_interval_minutes,
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
            source_root: profile.source_root.clone(),
            output_root: profile.output_root.clone(),
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

fn ensure_execution_capability_snapshot(
    snapshot: Option<&CapabilitySnapshotRow>,
) -> Result<(), MediaServiceError> {
    let Some(snapshot) = snapshot else {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_capability_snapshot_missing"));
    };

    let valid = !snapshot.ffmpeg_version.trim().is_empty()
        && !snapshot.ffprobe_version.trim().is_empty()
        && !snapshot.codec_name.trim().is_empty();
    if !valid {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_capability_snapshot_invalid"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MediaService, ensure_execution_capability_snapshot, map_data_error, map_detect_error,
        parse_yaml_bundle, validate_yaml_bundle,
    };
    use revaer_api::app::media::MediaServiceErrorKind;
    use revaer_api::app::media::{
        MediaCapabilityRefreshParams, MediaFacade, MediaJobCreateParams,
        MediaJobOperationAppendParams, MediaJobPhaseAppendParams, MediaJobPlanReasonAppendParams,
        MediaJobVerificationCheckAppendParams, MediaJobViolationAppendParams,
        MediaProfileUpsertParams,
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
                codecs: vec!["h264".to_string(), "h264".to_string(), "  ".to_string()],
                codec_support: vec![CodecCapability {
                    name: "h264".to_string(),
                    encode_supported: false,
                    decode_supported: true,
                }],
                encoders: Vec::new(),
            },
        })
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
            ffmpeg_version: String::new(),
            ffprobe_version: "7.0".to_string(),
            codec_name: "h264".to_string(),
            encode_supported: true,
            decode_supported: true,
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
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codec_name: "h264".to_string(),
            encode_supported: true,
            decode_supported: true,
            observed_at: chrono::Utc::now(),
        };
        assert!(ensure_execution_capability_snapshot(Some(&row)).is_ok());
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

        let job_id = service
            .media_job_create(MediaJobCreateParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/app-media/video.mkv",
                output_path: Some("/output/app-media/video.mkv"),
                dry_run: true,
            })
            .await?;
        append_plan_phase_and_operation(&service, job_id).await?;
        append_job_violation(&service, job_id).await?;
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
        assert_job_cancel_retry(&service, job_id).await?;

        assert_capability_refresh_uses_detected_support(&service, actor_user_public_id).await?;

        let yaml = service.media_yaml_export().await?;
        let validation = service.media_yaml_validate(&yaml).await?;
        assert!(validation.valid);
        let applied = service
            .media_yaml_apply(actor_user_public_id, &yaml)
            .await?;
        assert!(applied.forced_dry_run);

        Ok(())
    }

    async fn upsert_app_media_profile(
        service: &MediaService,
        actor_user_public_id: Uuid,
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
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
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
        assert!(!latest.encode_supported);
        assert!(latest.decode_supported);
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
