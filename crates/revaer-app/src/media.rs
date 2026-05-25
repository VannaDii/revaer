//! Media service wiring for API facade.

use async_trait::async_trait;
use revaer_api::app::media::{
    MediaCapabilityReadinessResponse as AppMediaCapabilityReadinessResponse,
    MediaCapabilityRecordParams, MediaCapabilityRefreshParams,
    MediaCapabilitySnapshotResponse as AppMediaCapabilitySnapshotResponse, MediaFacade,
    MediaJobCancelParams, MediaJobCreateParams, MediaJobPhaseAppendParams, MediaJobResponse,
    MediaProfileResponse, MediaProfileUpsertParams, MediaServiceError, MediaServiceErrorKind,
    MediaYamlApplyResult, MediaYamlProfile, MediaYamlValidationResult,
};
use revaer_data::DataError;
use revaer_data::media::capabilities::CapabilitySnapshotRow;
use revaer_data::media::capabilities::RecordCapabilitySnapshotInput;
use revaer_data::media::jobs::CreateMediaJobInput;
use revaer_data::media::profiles::{UpsertMediaProfileInput, upsert_media_profile_with_executor};
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

    async fn media_job_cancel(
        &self,
        params: MediaJobCancelParams,
    ) -> Result<(), MediaServiceError> {
        self.store
            .cancel_job(params.media_job_public_id)
            .await
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
        let snapshot = self
            .detector
            .detect()
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
            let snapshot_id = self
                .store
                .record_capability(&RecordCapabilitySnapshotInput {
                    actor_public_id: params.actor_user_public_id,
                    ffmpeg_version: &snapshot.ffmpeg_version,
                    ffprobe_version: &snapshot.ffprobe_version,
                    codec_name: &normalized,
                    encode_supported: true,
                    decode_supported: true,
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

    let kind = match detail.as_deref() {
        Some("app_user_not_found" | "media_profile_not_found" | "media_job_not_found") => {
            MediaServiceErrorKind::NotFound
        }
        Some("media_job_cancel_invalid_status") => MediaServiceErrorKind::Conflict,
        Some("media_profile_roots_overlap") => MediaServiceErrorKind::Invalid,
        _ => MediaServiceErrorKind::Storage,
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
    use super::ensure_execution_capability_snapshot;
    use revaer_data::media::capabilities::CapabilitySnapshotRow;

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
}
