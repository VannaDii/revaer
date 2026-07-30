//! Media service wiring for API facade.

use async_trait::async_trait;
use revaer_api::app::media::{
    MediaCapabilityReadinessResponse as AppMediaCapabilityReadinessResponse,
    MediaCapabilityRefreshParams,
    MediaCapabilitySnapshotResponse as AppMediaCapabilitySnapshotResponse,
    MediaCompatibilityTargetResponse as AppMediaCompatibilityTargetResponse,
    MediaCompatibilityTargetUpsertParams, MediaDesiredTargetCreateParams,
    MediaDesiredTargetResponse as AppMediaDesiredTargetResponse, MediaDesiredTargetStreamParams,
    MediaDiscoveryAutomationRunParams, MediaDiscoveryPreviewParams, MediaDiscoveryPreviewResponse,
    MediaDiscoveryQueuedJobResponse, MediaDiscoveryRunParams, MediaDiscoveryRunResponse,
    MediaDiscoverySkippedItemResponse, MediaFacade, MediaJobArtifactAppendParams,
    MediaJobArtifactResponse, MediaJobCompactAuditAppendParams, MediaJobCompactAuditResponse,
    MediaJobCreateParams, MediaJobOperationAppendParams, MediaJobOperationResponse,
    MediaJobPhaseResponse, MediaJobPlanReasonAppendParams, MediaJobPlanReasonResponse,
    MediaJobResponse, MediaJobRetentionResponse as AppMediaJobRetentionResponse,
    MediaJobRetentionUpdateParams, MediaJobVerificationCheckAppendParams,
    MediaJobVerificationCheckResponse, MediaJobViolationAppendParams, MediaJobViolationResponse,
    MediaPolicyResponse as AppMediaPolicyResponse, MediaPolicyUpsertParams,
    MediaProfileDesiredTargetParams, MediaProfilePatchParams,
    MediaProfileReadinessResponse as AppMediaProfileReadinessResponse, MediaProfileResponse,
    MediaProfileUpsertParams, MediaServiceError, MediaServiceErrorKind, MediaYamlApplyResult,
    MediaYamlBundle, MediaYamlCompatibilityTarget, MediaYamlDesiredTarget, MediaYamlIssue,
    MediaYamlMetadata, MediaYamlPolicy, MediaYamlProfile, MediaYamlValidationResult,
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
    AppendMediaDesiredTargetStreamInput, CreateMediaDesiredTargetInput,
    MediaCompatibilityTargetRow, MediaDesiredTargetStreamRow, MediaPolicyProfileRow,
    UpdateMediaJobRetentionPolicyInput, UpsertMediaCompatibilityTargetInput,
    UpsertMediaPolicyProfileInput, append_media_desired_target_stream_with_executor,
    create_media_desired_target_with_executor, list_media_desired_target_streams,
    list_media_desired_targets, set_media_profile_desired_target,
    set_media_profile_desired_target_with_executor,
    upsert_media_compatibility_target_with_executor, upsert_media_policy_profile_with_executor,
};
use revaer_data::media::imports::{
    MediaImportTransaction, MediaProfileImportDraftRow, UpsertMediaProfileImportDraftInput,
    delete_media_profile_import_draft_with_executor, list_media_profile_import_drafts,
    upsert_media_profile_import_draft_with_executor,
};
use revaer_data::media::jobs::{
    AppendMediaJobArtifactInput, AppendMediaJobCompactAuditInput,
    AppendMediaJobVerificationCheckInput, EnqueueDiscoveredMediaJobInput,
};
use revaer_data::media::profiles::{
    MediaProfileRow, UpdateMediaProfileInput, UpsertMediaProfileInput,
    upsert_media_profile_with_executor,
};
use revaer_media_core::compile::{MediaProfile, validate_profiles};
use revaer_media_core::model::StreamKind;
use revaer_media_core::normalize::{
    audio_channel_count_for_layout, normalize_audio_channel_layout,
};
use revaer_media_runtime::capabilities::{
    CapabilityDetectError, CapabilityDetector, CapabilitySnapshot, CodecCapability,
};
use revaer_media_runtime::execute::{
    BuildArgsError, VideoTranscodeIntent, VideoTranscodePolicy,
    validate_container_muxer_capability, validate_declared_stream_codec_capability,
};
use revaer_runtime::media::MediaStore;
use revaer_telemetry::Metrics;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

use crate::media_source_fingerprint::{MediaSourceFingerprintError, fingerprint_media_file};

const REPLACE_CONFIRMATION_PHRASE: &str = "replace";

#[derive(Debug, Clone, Copy)]
enum DiscoveryRunMode {
    Manual,
    Schedule,
    Watcher,
}

impl DiscoveryRunMode {
    const fn metric_source(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Schedule => "schedule",
            Self::Watcher => "watcher",
        }
    }
}

/// Production media facade backed by `revaer-runtime` media store.
#[derive(Clone)]
pub(crate) struct MediaService {
    store: MediaStore,
    detector: Arc<dyn CapabilityDetector>,
    telemetry: Metrics,
}

impl MediaService {
    /// Construct media service from runtime media store.
    #[must_use]
    pub(crate) fn new(
        store: MediaStore,
        detector: Arc<dyn CapabilityDetector>,
        telemetry: Metrics,
    ) -> Self {
        Self {
            store,
            detector,
            telemetry,
        }
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

        ensure_discovery_mode_enabled(mode, profile.schedule_enabled, profile.watcher_enabled)?;

        if !profile.dry_run_only {
            self.ensure_profile_ready_for_execution(&profile).await?;
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
            &profile.source_root,
            profile.dry_run_only,
            previews,
            mode.metric_source(),
        )
        .await
    }

    async fn queue_discovery_previews(
        &self,
        actor_user_public_id: Uuid,
        media_profile_public_id: Uuid,
        source_root: &str,
        dry_run: bool,
        previews: Vec<MediaDiscoveryPreviewResponse>,
        source: &'static str,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        let mut queued_jobs = Vec::new();
        let mut skipped = Vec::new();

        for preview in previews {
            match (preview.accepted, preview.output_path) {
                (true, Some(output_path)) => {
                    let fingerprint =
                        fingerprint_source_candidate(&preview.source_path, source_root).await?;
                    let Some(fingerprint) = fingerprint else {
                        self.telemetry
                            .inc_media_discovery_candidate(source, "unstable");
                        skipped.push(MediaDiscoverySkippedItemResponse {
                            source_path: preview.source_path,
                            reason: Some("media_discovery_source_unstable".to_string()),
                        });
                        continue;
                    };
                    let create_result = self
                        .store
                        .enqueue_discovered_job(&EnqueueDiscoveredMediaJobInput {
                            actor_public_id: actor_user_public_id,
                            media_profile_public_id,
                            source_path: &preview.source_path,
                            output_path: Some(output_path.as_str()),
                            source_size_bytes: fingerprint.size_bytes,
                            source_modified_ns: fingerprint.modified_ns,
                            source_sha256: &fingerprint.sha256,
                        })
                        .await;
                    let media_job_public_id = match create_result {
                        Ok(Some(media_job_public_id)) => media_job_public_id,
                        Ok(None) => {
                            self.telemetry
                                .inc_media_discovery_candidate(source, "deduplicated");
                            skipped.push(MediaDiscoverySkippedItemResponse {
                                source_path: preview.source_path,
                                reason: Some("media_discovery_source_unchanged".to_string()),
                            });
                            continue;
                        }
                        Err(err) => {
                            self.telemetry
                                .inc_media_discovery_candidate(source, "queue_failed");
                            return Err(map_data_error(&err));
                        }
                    };
                    self.telemetry
                        .inc_media_discovery_candidate(source, "queued");
                    self.telemetry.inc_media_job_queued(source, dry_run);
                    queued_jobs.push(MediaDiscoveryQueuedJobResponse {
                        media_job_public_id,
                        source_path: preview.source_path,
                        output_path,
                        dry_run,
                    });
                }
                (_, output_path) => {
                    self.telemetry
                        .inc_media_discovery_candidate(source, "skipped");
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

    async fn refresh_capability_snapshot(
        &self,
        params: MediaCapabilityRefreshParams,
    ) -> Result<i64, MediaServiceError> {
        let snapshot = self.detect_capability_snapshot().await?;
        self.persist_capability_snapshot(params.actor_user_public_id, &snapshot)
            .await
    }

    async fn detect_capability_snapshot(&self) -> Result<CapabilitySnapshot, MediaServiceError> {
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
        Ok(snapshot)
    }

    async fn persist_capability_snapshot(
        &self,
        actor_user_public_id: Uuid,
        snapshot: &CapabilitySnapshot,
    ) -> Result<i64, MediaServiceError> {
        let snapshot_run_public_id = Uuid::new_v4();
        let mut transaction = self.store.pool().begin().await.map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_capability_refresh_transaction_start_failed")
        })?;
        start_capability_snapshot_run_with_executor(
            &mut *transaction,
            actor_user_public_id,
            snapshot_run_public_id,
        )
        .await
        .map_err(|err| map_data_error(&err))?;

        let mut last_snapshot_id = None;
        for codec in capability_codec_inputs(snapshot) {
            let snapshot_id = record_capability_snapshot_with_executor(
                &mut *transaction,
                &RecordCapabilitySnapshotInput {
                    actor_public_id: actor_user_public_id,
                    snapshot_run_public_id: Some(snapshot_run_public_id),
                    ffmpeg_version: &snapshot.ffmpeg_version,
                    ffprobe_version: &snapshot.ffprobe_version,
                    codec_name: &codec.name,
                    encode_supported: codec.encode_supported,
                    decode_supported: codec.decode_supported,
                },
            )
            .await
            .map_err(|err| map_data_error(&err))?;
            last_snapshot_id = Some(snapshot_id);
        }
        for encoder_name in normalized_unique_names(&snapshot.encoders) {
            record_capability_encoder_with_executor(
                &mut *transaction,
                &RecordCapabilityEncoderInput {
                    actor_public_id: actor_user_public_id,
                    snapshot_run_public_id,
                    encoder_name: &encoder_name,
                },
            )
            .await
            .map_err(|err| map_data_error(&err))?;
        }
        for feature in capability_feature_inputs(snapshot) {
            record_capability_feature_with_executor(
                &mut *transaction,
                &RecordCapabilityFeatureInput {
                    actor_public_id: actor_user_public_id,
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

    fn record_capability_refresh_metric(&self, result: &Result<i64, MediaServiceError>) {
        match result {
            Ok(_) => self.telemetry.inc_media_capability_refresh("success"),
            Err(error) => self
                .telemetry
                .inc_media_capability_refresh(error.code().unwrap_or("failed")),
        }
    }

    async fn ensure_profile_ready_for_execution(
        &self,
        profile: &MediaProfileRow,
    ) -> Result<(), MediaServiceError> {
        let latest = self
            .store
            .latest_capability()
            .await
            .map_err(|err| map_data_error(&err))?;
        if let Some(code) = self
            .profile_execution_readiness_failure_code(profile, latest.as_ref())
            .await?
        {
            return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid).with_code(&code));
        }
        Ok(())
    }

    async fn profile_execution_readiness_failure_code(
        &self,
        profile: &MediaProfileRow,
        latest: Option<&CapabilitySnapshotRow>,
    ) -> Result<Option<String>, MediaServiceError> {
        if let Some(code) = capability_snapshot_readiness_code(latest) {
            return Ok(Some(code.to_string()));
        }
        let Some(snapshot) = latest else {
            return Ok(None);
        };

        if profile
            .compatibility_target_key
            .as_deref()
            .and_then(trim_nonempty)
            .is_some()
        {
            let compatibility_targets = self
                .store
                .list_compatibility_targets()
                .await
                .map_err(|err| map_data_error(&err))?;
            if let Err(error) = ensure_profile_compatibility_target_readiness(
                profile,
                snapshot,
                &compatibility_targets,
            ) {
                return Ok(Some(
                    error
                        .code()
                        .unwrap_or("media_profile_readiness_failed")
                        .to_string(),
                ));
            }
        }

        if profile
            .desired_target_key
            .as_deref()
            .and_then(trim_nonempty)
            .is_some()
        {
            let desired_targets = self.media_desired_target_list().await?;
            let policies = self
                .store
                .list_policy_profiles()
                .await
                .map_err(|err| map_data_error(&err))?;
            if let Err(error) = ensure_profile_desired_target_readiness(
                profile,
                snapshot,
                &desired_targets,
                &policies,
            ) {
                return Ok(Some(
                    error
                        .code()
                        .unwrap_or("media_profile_readiness_failed")
                        .to_string(),
                ));
            }
        }

        Ok(None)
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
            .map(|rows| rows.into_iter().map(media_profile_response).collect())
            .map_err(|err| map_data_error(&err))
    }

    async fn media_profile_readiness(
        &self,
        media_profile_public_id: Uuid,
    ) -> Result<Option<AppMediaProfileReadinessResponse>, MediaServiceError> {
        let Some(profile) = self
            .store
            .get_profile(media_profile_public_id)
            .await
            .map_err(|err| map_data_error(&err))?
        else {
            return Ok(None);
        };
        let latest = self
            .store
            .latest_capability()
            .await
            .map_err(|err| map_data_error(&err))?;
        let reason = self
            .profile_execution_readiness_failure_code(&profile, latest.as_ref())
            .await?;

        Ok(Some(AppMediaProfileReadinessResponse {
            ready: reason.is_none(),
            reason,
            profile: shared_media_profile_response(profile),
            snapshot: latest.map(media_capability_snapshot_response),
        }))
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
                        audio_channels: row.audio_channels,
                        audio_channel_layout: row.audio_channel_layout,
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
                audio_channels: params.audio_channels,
                audio_channel_layout: params.audio_channel_layout,
                subtitle_policy: params.subtitle_policy,
            })
            .await
            .map(|row| AppMediaCompatibilityTargetResponse {
                compatibility_target_key: row.compatibility_target_key,
                version: row.version,
                display_name: row.display_name,
                video_codec: row.video_codec,
                audio_codec: row.audio_codec,
                audio_channels: row.audio_channels,
                audio_channel_layout: row.audio_channel_layout,
                subtitle_policy: row.subtitle_policy,
            })
            .map_err(|err| map_data_error(&err))
    }

    async fn media_desired_target_list(
        &self,
    ) -> Result<Vec<AppMediaDesiredTargetResponse>, MediaServiceError> {
        let targets = list_media_desired_targets(self.store.pool())
            .await
            .map_err(|err| map_data_error(&err))?;
        let mut responses = Vec::with_capacity(targets.len());
        for target in targets {
            let streams = list_media_desired_target_streams(
                self.store.pool(),
                target.media_desired_target_profile_public_id,
            )
            .await
            .map_err(|err| map_data_error(&err))?;
            responses.push(AppMediaDesiredTargetResponse {
                media_desired_target_profile_public_id: target
                    .media_desired_target_profile_public_id,
                target_key: target.target_key,
                version: target.version,
                display_name: target.display_name,
                container_format: target.container_format,
                streams: streams.into_iter().map(map_desired_target_stream).collect(),
            });
        }
        Ok(responses)
    }

    async fn media_desired_target_create(
        &self,
        params: MediaDesiredTargetCreateParams,
    ) -> Result<AppMediaDesiredTargetResponse, MediaServiceError> {
        if params.streams.is_empty() {
            return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_desired_target_streams_required"));
        }
        let mut transaction = self
            .store
            .pool()
            .begin()
            .await
            .map_err(|err| map_data_error(&DataError::from(err)))?;
        let target_id = create_media_desired_target_with_executor(
            &mut *transaction,
            CreateMediaDesiredTargetInput {
                actor_public_id: params.actor_user_public_id,
                target_key: &params.target_key,
                version: params.version,
                display_name: &params.display_name,
                container_format: &params.container_format,
            },
        )
        .await
        .map_err(|err| map_data_error(&err))?;
        for stream in &params.streams {
            append_media_desired_target_stream_with_executor(
                &mut *transaction,
                AppendMediaDesiredTargetStreamInput {
                    media_desired_target_profile_public_id: target_id,
                    stream_key: &stream.stream_key,
                    stream_kind: &stream.stream_kind,
                    semantic_role: stream.semantic_role.as_deref(),
                    language_code: stream.language_code.as_deref(),
                    optional: stream.optional,
                    sort_order: stream.sort_order,
                    codec: &stream.codec,
                    channel_count: stream.channel_count,
                    channel_layout: stream.channel_layout.as_deref(),
                    audio_bitrate_bps: stream.audio_bitrate_bps,
                    audio_sample_rate_hz: stream.audio_sample_rate_hz,
                    audio_loudness_profile: stream.audio_loudness_profile.as_deref(),
                    audio_dynamic_range: stream.audio_dynamic_range.as_deref(),
                    video_profile: stream.video_profile.as_deref(),
                    video_level: stream.video_level.as_deref(),
                    video_bitrate_bps: stream.video_bitrate_bps,
                    color_primaries: stream.color_primaries.as_deref(),
                    color_transfer: stream.color_transfer.as_deref(),
                    color_space: stream.color_space.as_deref(),
                    hdr_format: stream.hdr_format.as_deref(),
                    title: stream.title.as_deref(),
                    default_disposition: stream.default_disposition,
                    forced_disposition: stream.forced_disposition,
                    subtitle_placement: stream.subtitle_placement.as_deref(),
                    image_subtitle_action: stream.image_subtitle_action.as_deref(),
                },
            )
            .await
            .map_err(|err| map_data_error(&err))?;
        }
        transaction
            .commit()
            .await
            .map_err(|err| map_data_error(&DataError::from(err)))?;
        Ok(AppMediaDesiredTargetResponse {
            media_desired_target_profile_public_id: target_id,
            target_key: params.target_key,
            version: params.version,
            display_name: params.display_name,
            container_format: params.container_format,
            streams: params.streams,
        })
    }

    async fn media_profile_desired_target_set(
        &self,
        params: MediaProfileDesiredTargetParams,
    ) -> Result<Uuid, MediaServiceError> {
        set_media_profile_desired_target(
            self.store.pool(),
            params.actor_user_public_id,
            params.media_profile_public_id,
            params.target_key.as_deref(),
            params.version,
        )
        .await
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
                        verification_strictness: row.verification_strictness,
                        verification_duration_tolerance_millis: row
                            .verification_duration_tolerance_millis,
                        verification_mux_validation: row
                            .verification_mux_validation
                            .enabled()
                            .into(),
                        verification_decode_all_streams: row
                            .verification_decode_all_streams
                            .enabled()
                            .into(),
                        verification_keyframe_seek: row.verification_keyframe_seek.enabled().into(),
                        verification_playback_probe: row
                            .verification_playback_probe
                            .enabled()
                            .into(),
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
                verification_strictness: params.verification_strictness,
                verification_duration_tolerance_millis: params
                    .verification_duration_tolerance_millis,
                verification_mux_validation: params.verification_mux_validation.enabled().into(),
                verification_decode_all_streams: params
                    .verification_decode_all_streams
                    .enabled()
                    .into(),
                verification_keyframe_seek: params.verification_keyframe_seek.enabled().into(),
                verification_playback_probe: params.verification_playback_probe.enabled().into(),
            })
            .await
            .map(|row| AppMediaPolicyResponse {
                policy_key: row.policy_key,
                version: row.version,
                display_name: row.display_name,
                video_intent: row.video_intent,
                verification_strictness: row.verification_strictness,
                verification_duration_tolerance_millis: row.verification_duration_tolerance_millis,
                verification_mux_validation: row.verification_mux_validation.enabled().into(),
                verification_decode_all_streams: row
                    .verification_decode_all_streams
                    .enabled()
                    .into(),
                verification_keyframe_seek: row.verification_keyframe_seek.enabled().into(),
                verification_playback_probe: row.verification_playback_probe.enabled().into(),
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
            completed_enabled: retention.completed_enabled,
            completed_mode: retention.completed_mode,
            completed_limit: retention.completed_limit,
            failed_diagnostic_enabled: retention.failed_diagnostic_enabled,
            failed_diagnostic_mode: retention.failed_diagnostic_mode,
            failed_diagnostic_limit: retention.failed_diagnostic_limit,
        })
    }

    async fn media_job_retention_update(
        &self,
        params: MediaJobRetentionUpdateParams,
    ) -> Result<AppMediaJobRetentionResponse, MediaServiceError> {
        self.store
            .update_job_retention_policy(UpdateMediaJobRetentionPolicyInput {
                actor_public_id: params.actor_user_public_id,
                completed_enabled: params.completed_enabled,
                completed_mode: params.completed_mode.to_string(),
                completed_limit: params.completed_limit,
                failed_diagnostic_enabled: params.failed_diagnostic_enabled,
                failed_diagnostic_mode: params.failed_diagnostic_mode.to_string(),
                failed_diagnostic_limit: params.failed_diagnostic_limit,
            })
            .await
            .map(|row| AppMediaJobRetentionResponse {
                completed_enabled: row.completed_enabled,
                completed_mode: row.completed_mode,
                completed_limit: row.completed_limit,
                failed_diagnostic_enabled: row.failed_diagnostic_enabled,
                failed_diagnostic_mode: row.failed_diagnostic_mode,
                failed_diagnostic_limit: row.failed_diagnostic_limit,
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

            self.ensure_profile_ready_for_execution(&profile).await?;
        }

        let fingerprint = fingerprint_source_candidate(params.source_path, &profile.source_root)
            .await?
            .ok_or_else(|| {
                MediaServiceError::new(MediaServiceErrorKind::Invalid)
                    .with_code("media_discovery_source_unstable")
            })?;
        let create_result = self
            .store
            .enqueue_discovered_job(&EnqueueDiscoveredMediaJobInput {
                actor_public_id: params.actor_user_public_id,
                media_profile_public_id: params.media_profile_public_id,
                source_path: params.source_path,
                output_path: params.output_path,
                source_size_bytes: fingerprint.size_bytes,
                source_modified_ns: fingerprint.modified_ns,
                source_sha256: &fingerprint.sha256,
            })
            .await;
        match create_result {
            Ok(Some(media_job_public_id)) => {
                self.telemetry
                    .inc_media_job_queued("direct", params.dry_run);
                Ok(media_job_public_id)
            }
            Ok(None) => Err(MediaServiceError::new(MediaServiceErrorKind::Conflict)
                .with_code("media_discovery_source_unchanged")),
            Err(err) => {
                self.telemetry.inc_media_job_failure("queue");
                Err(map_data_error(&err))
            }
        }
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
        params: MediaDiscoveryAutomationRunParams<'_>,
    ) -> Result<MediaDiscoveryRunResponse, MediaServiceError> {
        self.run_discovery_for_profile(
            params.actor_user_public_id,
            params.media_profile_public_id,
            params.source_paths,
            DiscoveryRunMode::Watcher,
        )
        .await
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

    async fn media_job_phase_list(
        &self,
        media_job_public_id: Uuid,
    ) -> Result<Vec<MediaJobPhaseResponse>, MediaServiceError> {
        self.store
            .list_job_phases(media_job_public_id)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| MediaJobPhaseResponse {
                        phase_index: row.phase_index,
                        phase_name: row.phase_name,
                        phase_status: row.phase_status,
                        details_text: row.details_text,
                        created_at: row.created_at,
                    })
                    .collect()
            })
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
        let result = self.refresh_capability_snapshot(params).await;
        self.record_capability_refresh_metric(&result);
        result
    }

    async fn media_capability_latest(
        &self,
    ) -> Result<Option<AppMediaCapabilitySnapshotResponse>, MediaServiceError> {
        self.store
            .latest_capability()
            .await
            .map(|row_opt| row_opt.map(media_capability_snapshot_response))
            .map_err(|err| map_data_error(&err))
    }

    async fn media_capability_readiness(
        &self,
    ) -> Result<AppMediaCapabilityReadinessResponse, MediaServiceError> {
        let snapshot = self.media_capability_latest().await?;
        let reason = match snapshot.as_ref() {
            None => Some("media_capability_snapshot_missing".to_string()),
            Some(item) if media_capability_snapshot_response_invalid(item) => {
                Some("media_capability_snapshot_invalid".to_string())
            }
            Some(_) => None,
        };

        Ok(AppMediaCapabilityReadinessResponse {
            ready: reason.is_none(),
            reason,
            snapshot,
        })
    }

    async fn media_yaml_export(
        &self,
        include_local_paths: bool,
    ) -> Result<String, MediaServiceError> {
        let payload = build_media_yaml_bundle(self, include_local_paths).await?;
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
        let existing_compatibility_targets = self.media_compatibility_target_list().await?;
        let existing_desired_targets = self.media_desired_target_list().await?;
        let existing_policies = self.media_policy_list().await?;
        let issues = validate_yaml_bundle(
            &parsed,
            &existing_compatibility_targets,
            &existing_desired_targets,
            &existing_policies,
        );
        Ok(MediaYamlValidationResult {
            version: parsed.format_version.to_string(),
            valid: !issues.iter().any(|issue| issue.blocking),
            issues,
            bundle: parsed,
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
        let existing_profiles = self.media_profile_list().await?;
        let existing_desired_targets = self.media_desired_target_list().await?;
        let mut transaction = self.store.pool().begin().await.map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_yaml_apply_transaction_start_failed")
        })?;
        let bundle = validation.bundle;
        import_yaml_catalogs(
            &mut transaction,
            actor_user_public_id,
            &bundle,
            &existing_desired_targets,
        )
        .await?;
        let result = import_yaml_profiles(
            &mut transaction,
            actor_user_public_id,
            bundle.profiles,
            &existing_profiles,
        )
        .await?;
        transaction.commit().await.map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_yaml_apply_transaction_commit_failed")
        })?;

        Ok(result)
    }
}

async fn build_media_yaml_bundle(
    service: &MediaService,
    include_local_paths: bool,
) -> Result<MediaYamlBundle, MediaServiceError> {
    let active_profiles = service.media_profile_list().await?;
    let drafts = list_media_profile_import_drafts(service.store.pool())
        .await
        .map_err(|err| map_data_error(&err))?;
    let compatibility_targets = service
        .media_compatibility_target_list()
        .await?
        .into_iter()
        .map(media_yaml_compatibility_target)
        .collect();
    let targets = service
        .media_desired_target_list()
        .await?
        .into_iter()
        .map(media_yaml_desired_target)
        .collect();
    let policies = service
        .media_policy_list()
        .await?
        .into_iter()
        .map(media_yaml_policy)
        .collect();
    Ok(MediaYamlBundle {
        format_version: 1,
        kind: "revaer.media.profile_bundle".to_string(),
        metadata: MediaYamlMetadata {
            name: "Revaer media configuration".to_string(),
            description: Some(
                "Portable media profiles and their complete referenced catalogs".to_string(),
            ),
        },
        compatibility_targets,
        targets,
        policies,
        profiles: export_yaml_profiles(active_profiles, drafts, include_local_paths),
    })
}

fn export_yaml_profiles(
    active_profiles: Vec<MediaProfileResponse>,
    drafts: Vec<MediaProfileImportDraftRow>,
    include_local_paths: bool,
) -> Vec<MediaYamlProfile> {
    let mut profiles = active_profiles
        .into_iter()
        .map(|profile| media_yaml_active_profile(profile, include_local_paths))
        .collect::<Vec<_>>();
    profiles.extend(
        drafts
            .into_iter()
            .map(|draft| media_yaml_draft_profile(draft, include_local_paths)),
    );
    profiles.sort_by(|left, right| {
        left.profile_key
            .to_ascii_lowercase()
            .cmp(&right.profile_key.to_ascii_lowercase())
    });
    profiles
}

fn media_yaml_active_profile(
    profile: MediaProfileResponse,
    include_local_paths: bool,
) -> MediaYamlProfile {
    MediaYamlProfile {
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
        desired_target_key: profile.desired_target_key,
        desired_target_version: profile.desired_target_version,
        policy_key: profile.policy_key,
        watcher_enabled: false,
        schedule_enabled: false,
        schedule_interval_minutes: None,
    }
}

fn media_yaml_draft_profile(
    draft: MediaProfileImportDraftRow,
    include_local_paths: bool,
) -> MediaYamlProfile {
    MediaYamlProfile {
        profile_key: draft.profile_key.clone(),
        source_root: export_draft_root(
            include_local_paths,
            &draft.profile_key,
            "source",
            draft.source_root,
            draft.source_root_resolved,
        ),
        output_root: export_draft_root(
            include_local_paths,
            &draft.profile_key,
            "output",
            draft.output_root,
            draft.output_root_resolved,
        ),
        dry_run_only: true,
        retention_days: draft.retention_days,
        compatibility_target_key: draft.compatibility_target_key,
        desired_target_key: draft.desired_target_key,
        desired_target_version: draft.desired_target_version,
        policy_key: draft.policy_key,
        watcher_enabled: false,
        schedule_enabled: false,
        schedule_interval_minutes: None,
    }
}

fn export_draft_root(
    include_local_paths: bool,
    profile_key: &str,
    root_kind: &str,
    root: String,
    resolved: bool,
) -> String {
    if resolved {
        export_root(include_local_paths, profile_key, root_kind, &root)
    } else {
        root
    }
}

async fn import_yaml_catalogs(
    transaction: &mut MediaImportTransaction<'_>,
    actor_user_public_id: Uuid,
    bundle: &MediaYamlBundle,
    existing_desired_targets: &[AppMediaDesiredTargetResponse],
) -> Result<(), MediaServiceError> {
    import_yaml_compatibility_targets(
        transaction,
        actor_user_public_id,
        &bundle.compatibility_targets,
    )
    .await?;
    import_yaml_policies(transaction, actor_user_public_id, &bundle.policies).await?;
    import_yaml_desired_targets(
        transaction,
        actor_user_public_id,
        &bundle.targets,
        existing_desired_targets,
    )
    .await
}

async fn import_yaml_compatibility_targets(
    transaction: &mut MediaImportTransaction<'_>,
    actor_user_public_id: Uuid,
    targets: &[MediaYamlCompatibilityTarget],
) -> Result<(), MediaServiceError> {
    for target in targets {
        upsert_media_compatibility_target_with_executor(
            &mut **transaction,
            UpsertMediaCompatibilityTargetInput {
                actor_public_id: actor_user_public_id,
                compatibility_target_key: &target.compatibility_target_key,
                version: target.version,
                display_name: &target.display_name,
                video_codec: &target.video_codec,
                audio_codec: &target.audio_codec,
                audio_channels: target.audio_channels,
                audio_channel_layout: target.audio_channel_layout.as_deref(),
                subtitle_policy: &target.subtitle_policy,
            },
        )
        .await
        .map_err(|err| map_data_error(&err))?;
    }
    Ok(())
}

async fn import_yaml_policies(
    transaction: &mut MediaImportTransaction<'_>,
    actor_user_public_id: Uuid,
    policies: &[MediaYamlPolicy],
) -> Result<(), MediaServiceError> {
    for policy in policies {
        upsert_media_policy_profile_with_executor(
            &mut **transaction,
            UpsertMediaPolicyProfileInput {
                actor_public_id: actor_user_public_id,
                policy_key: &policy.policy_key,
                version: policy.version,
                display_name: &policy.display_name,
                video_intent: &policy.video_intent,
                verification_strictness: &policy.verification_strictness,
                verification_duration_tolerance_millis: policy
                    .verification_duration_tolerance_millis,
                verification_mux_validation: policy.verification_mux_validation.enabled().into(),
                verification_decode_all_streams: policy
                    .verification_decode_all_streams
                    .enabled()
                    .into(),
                verification_keyframe_seek: policy.verification_keyframe_seek.enabled().into(),
                verification_playback_probe: policy.verification_playback_probe.enabled().into(),
            },
        )
        .await
        .map_err(|err| map_data_error(&err))?;
    }
    Ok(())
}

async fn import_yaml_desired_targets(
    transaction: &mut MediaImportTransaction<'_>,
    actor_user_public_id: Uuid,
    targets: &[MediaYamlDesiredTarget],
    existing: &[AppMediaDesiredTargetResponse],
) -> Result<(), MediaServiceError> {
    for target in targets {
        if existing.iter().any(|item| {
            item.target_key
                .eq_ignore_ascii_case(target.target_key.trim())
                && item.version == target.version
        }) {
            continue;
        }
        let target_id = create_media_desired_target_with_executor(
            &mut **transaction,
            CreateMediaDesiredTargetInput {
                actor_public_id: actor_user_public_id,
                target_key: &target.target_key,
                version: target.version,
                display_name: &target.display_name,
                container_format: &target.container_format,
            },
        )
        .await
        .map_err(|err| map_data_error(&err))?;
        import_yaml_desired_streams(transaction, target_id, &target.streams).await?;
    }
    Ok(())
}

async fn import_yaml_desired_streams(
    transaction: &mut MediaImportTransaction<'_>,
    target_id: Uuid,
    streams: &[MediaDesiredTargetStreamParams],
) -> Result<(), MediaServiceError> {
    for stream in streams {
        append_media_desired_target_stream_with_executor(
            &mut **transaction,
            AppendMediaDesiredTargetStreamInput {
                media_desired_target_profile_public_id: target_id,
                stream_key: &stream.stream_key,
                stream_kind: &stream.stream_kind,
                semantic_role: stream.semantic_role.as_deref(),
                language_code: stream.language_code.as_deref(),
                optional: stream.optional,
                sort_order: stream.sort_order,
                codec: &stream.codec,
                channel_count: stream.channel_count,
                channel_layout: stream.channel_layout.as_deref(),
                audio_bitrate_bps: stream.audio_bitrate_bps,
                audio_sample_rate_hz: stream.audio_sample_rate_hz,
                audio_loudness_profile: stream.audio_loudness_profile.as_deref(),
                audio_dynamic_range: stream.audio_dynamic_range.as_deref(),
                video_profile: stream.video_profile.as_deref(),
                video_level: stream.video_level.as_deref(),
                video_bitrate_bps: stream.video_bitrate_bps,
                color_primaries: stream.color_primaries.as_deref(),
                color_transfer: stream.color_transfer.as_deref(),
                color_space: stream.color_space.as_deref(),
                hdr_format: stream.hdr_format.as_deref(),
                title: stream.title.as_deref(),
                default_disposition: stream.default_disposition,
                forced_disposition: stream.forced_disposition,
                subtitle_placement: stream.subtitle_placement.as_deref(),
                image_subtitle_action: stream.image_subtitle_action.as_deref(),
            },
        )
        .await
        .map_err(|err| map_data_error(&err))?;
    }
    Ok(())
}

#[derive(Debug)]
struct ResolvedYamlProfile {
    profile: MediaYamlProfile,
    source_root: Option<String>,
    output_root: Option<String>,
}

async fn import_yaml_profiles(
    transaction: &mut MediaImportTransaction<'_>,
    actor_user_public_id: Uuid,
    profiles: Vec<MediaYamlProfile>,
    existing_profiles: &[MediaProfileResponse],
) -> Result<MediaYamlApplyResult, MediaServiceError> {
    let mut media_profile_public_ids = Vec::with_capacity(profiles.len());
    let mut media_profile_import_draft_public_ids = Vec::new();
    for profile in profiles {
        let resolved = resolve_yaml_profile(profile, existing_profiles);
        if resolved.source_root.is_none() || resolved.output_root.is_none() {
            media_profile_import_draft_public_ids.push(
                persist_yaml_profile_draft(transaction, actor_user_public_id, &resolved).await?,
            );
        } else {
            media_profile_public_ids
                .push(persist_yaml_profile(transaction, actor_user_public_id, &resolved).await?);
        }
    }
    Ok(MediaYamlApplyResult {
        forced_dry_run: true,
        media_profile_public_ids,
        media_profile_import_draft_public_ids,
    })
}

fn resolve_yaml_profile(
    profile: MediaYamlProfile,
    existing_profiles: &[MediaProfileResponse],
) -> ResolvedYamlProfile {
    let existing = existing_profiles
        .iter()
        .find(|item| item.profile_key.eq_ignore_ascii_case(&profile.profile_key));
    let source_root =
        resolve_yaml_root(&profile.source_root, existing.map(|item| &item.source_root));
    let output_root =
        resolve_yaml_root(&profile.output_root, existing.map(|item| &item.output_root));
    ResolvedYamlProfile {
        profile,
        source_root,
        output_root,
    }
}

fn resolve_yaml_root(root: &str, existing: Option<&String>) -> Option<String> {
    if is_unresolved_export_root(root) {
        existing.cloned()
    } else {
        Some(root.to_string())
    }
}

async fn persist_yaml_profile_draft(
    transaction: &mut MediaImportTransaction<'_>,
    actor_user_public_id: Uuid,
    resolved: &ResolvedYamlProfile,
) -> Result<Uuid, MediaServiceError> {
    let profile = &resolved.profile;
    upsert_media_profile_import_draft_with_executor(
        &mut **transaction,
        UpsertMediaProfileImportDraftInput {
            actor_public_id: actor_user_public_id,
            profile_key: &profile.profile_key,
            source_root: &profile.source_root,
            output_root: &profile.output_root,
            source_root_resolved: !is_unresolved_export_root(&profile.source_root),
            output_root_resolved: !is_unresolved_export_root(&profile.output_root),
            retention_days: profile.retention_days,
            compatibility_target_key: profile.compatibility_target_key.as_deref(),
            desired_target_key: profile.desired_target_key.as_deref(),
            desired_target_version: profile.desired_target_version,
            policy_key: &profile.policy_key,
        },
    )
    .await
    .map_err(|err| map_data_error(&err))
}

async fn persist_yaml_profile(
    transaction: &mut MediaImportTransaction<'_>,
    actor_user_public_id: Uuid,
    resolved: &ResolvedYamlProfile,
) -> Result<Uuid, MediaServiceError> {
    let profile = &resolved.profile;
    let profile_id = upsert_media_profile_with_executor(
        &mut **transaction,
        &UpsertMediaProfileInput {
            actor_public_id: actor_user_public_id,
            profile_key: &profile.profile_key,
            source_root: required_resolved_root(resolved.source_root.as_deref(), "source")?,
            output_root: required_resolved_root(resolved.output_root.as_deref(), "output")?,
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
    pin_imported_profile_target(transaction, actor_user_public_id, profile_id, profile).await?;
    delete_media_profile_import_draft_with_executor(&mut **transaction, &profile.profile_key)
        .await
        .map_err(|err| map_data_error(&err))?;
    Ok(profile_id)
}

fn required_resolved_root<'a>(
    root: Option<&'a str>,
    root_kind: &str,
) -> Result<&'a str, MediaServiceError> {
    root.ok_or_else(|| {
        MediaServiceError::new(MediaServiceErrorKind::Invalid).with_code(if root_kind == "source" {
            "media_yaml_source_root_unresolved"
        } else {
            "media_yaml_output_root_unresolved"
        })
    })
}

async fn pin_imported_profile_target(
    transaction: &mut MediaImportTransaction<'_>,
    actor_user_public_id: Uuid,
    profile_id: Uuid,
    profile: &MediaYamlProfile,
) -> Result<(), MediaServiceError> {
    if profile.desired_target_key.is_none() {
        return Ok(());
    }
    set_media_profile_desired_target_with_executor(
        &mut **transaction,
        actor_user_public_id,
        profile_id,
        profile.desired_target_key.as_deref(),
        profile.desired_target_version,
    )
    .await
    .map_err(|err| map_data_error(&err))?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct CapabilityFeatureInput<'a> {
    family: &'a str,
    name: &'a str,
    supported: bool,
    detail: Option<&'a str>,
}

#[derive(Debug, Clone)]
struct CapabilityCodecInput {
    name: String,
    encode_supported: bool,
    decode_supported: bool,
}

fn capability_codec_inputs(snapshot: &CapabilitySnapshot) -> Vec<CapabilityCodecInput> {
    let mut seen = BTreeSet::new();
    let mut codecs = Vec::new();
    for codec in &snapshot.codecs {
        let normalized = codec.trim().to_ascii_lowercase();
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        let support = snapshot.codec_capability(&normalized);
        codecs.push(CapabilityCodecInput {
            name: normalized,
            encode_supported: support.encode_supported,
            decode_supported: support.decode_supported,
        });
    }
    codecs
}

fn normalized_unique_names(names: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for name in names {
        let candidate = name.trim().to_ascii_lowercase();
        if candidate.is_empty() || !seen.insert(candidate.clone()) {
            continue;
        }
        normalized.push(candidate);
    }
    normalized
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
    features.push(CapabilityFeatureInput {
        family: "ffmpeg_license",
        name: snapshot.ffmpeg_license_mode.as_str(),
        supported: true,
        detail: None,
    });
    features.push(CapabilityFeatureInput {
        family: "ffmpeg_build_flag",
        name: "--enable-gpl",
        supported: snapshot.ffmpeg_enable_gpl,
        detail: None,
    });
    features.push(CapabilityFeatureInput {
        family: "ffmpeg_build_flag",
        name: "--enable-version3",
        supported: snapshot.ffmpeg_enable_version3,
        detail: None,
    });
    features.push(CapabilityFeatureInput {
        family: "ffmpeg_build_flag",
        name: "--enable-nonfree",
        supported: snapshot.ffmpeg_enable_nonfree,
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

fn feature_supported(features: &[CapabilityFeatureRow], family: &str, name: &str) -> bool {
    features.iter().any(|feature| {
        feature.supported
            && feature.feature_family.eq_ignore_ascii_case(family)
            && feature.feature_name.eq_ignore_ascii_case(name)
    })
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

fn validate_yaml_bundle(
    bundle: &MediaYamlBundle,
    existing_compatibility_targets: &[AppMediaCompatibilityTargetResponse],
    existing_desired_targets: &[AppMediaDesiredTargetResponse],
    existing_policies: &[AppMediaPolicyResponse],
) -> Vec<MediaYamlIssue> {
    let mut issues = Vec::new();
    validate_yaml_header(&mut issues, bundle);
    validate_yaml_profiles(&mut issues, &bundle.profiles);
    validate_yaml_catalogs(
        &mut issues,
        bundle,
        existing_compatibility_targets,
        existing_desired_targets,
        existing_policies,
    );
    issues.sort_by(|left, right| {
        left.pointer
            .cmp(&right.pointer)
            .then_with(|| left.code.cmp(&right.code))
    });
    issues.dedup();
    issues
}

fn validate_yaml_header(issues: &mut Vec<MediaYamlIssue>, bundle: &MediaYamlBundle) {
    if bundle.format_version != 1 {
        push_yaml_issue(
            issues,
            "media_yaml_version_unsupported",
            "/format_version",
            true,
        );
    }
    if bundle.kind != "revaer.media.profile_bundle" {
        push_yaml_issue(issues, "media_yaml_kind_unsupported", "/kind", true);
    }
    if bundle.metadata.name.trim().is_empty() {
        push_yaml_issue(
            issues,
            "media_yaml_metadata_name_missing",
            "/metadata/name",
            true,
        );
    }
}

fn validate_yaml_profiles(issues: &mut Vec<MediaYamlIssue>, yaml_profiles: &[MediaYamlProfile]) {
    let mut profiles = Vec::with_capacity(yaml_profiles.len());
    let mut profile_keys = BTreeSet::new();
    for (index, profile) in yaml_profiles.iter().enumerate() {
        validate_yaml_profile(issues, &mut profile_keys, &mut profiles, index, profile);
    }

    if let Err(err) = validate_profiles(&profiles) {
        push_yaml_issue(
            issues,
            yaml_profile_validation_code(&err),
            "/profiles",
            true,
        );
    }
}

fn validate_yaml_profile(
    issues: &mut Vec<MediaYamlIssue>,
    profile_keys: &mut BTreeSet<String>,
    profiles: &mut Vec<MediaProfile>,
    index: usize,
    profile: &MediaYamlProfile,
) {
    let pointer = format!("/profiles/{index}");
    if !profile_keys.insert(profile.profile_key.trim().to_ascii_lowercase()) {
        push_yaml_issue(
            issues,
            "media_yaml_profile_key_duplicate",
            &format!("{pointer}/profile_key"),
            true,
        );
    }
    if !(1..=3650).contains(&profile.retention_days) {
        push_yaml_issue(
            issues,
            "media_yaml_profile_retention_days_out_of_bounds",
            &format!("{pointer}/retention_days"),
            true,
        );
    }
    for (root_kind, root) in [
        ("source_root", &profile.source_root),
        ("output_root", &profile.output_root),
    ] {
        if is_unresolved_export_root(root) {
            push_yaml_issue(
                issues,
                "media_yaml_local_path_mapping_required",
                &format!("{pointer}/{root_kind}"),
                false,
            );
        }
    }
    if profile.desired_target_key.is_some() != profile.desired_target_version.is_some() {
        push_yaml_issue(
            issues,
            "media_yaml_desired_target_reference_incomplete",
            &format!("{pointer}/desired_target_key"),
            true,
        );
    }
    if !is_unresolved_export_root(&profile.source_root)
        && !is_unresolved_export_root(&profile.output_root)
    {
        profiles.push(MediaProfile {
            key: profile.profile_key.clone(),
            source_root: profile.source_root.clone(),
            output_root: profile.output_root.clone(),
            dry_run_only: true,
        });
    }
}

const fn yaml_profile_validation_code(
    error: &revaer_media_core::compile::ValidationError,
) -> &'static str {
    match error {
        revaer_media_core::compile::ValidationError::OverlappingRoots
        | revaer_media_core::compile::ValidationError::OverlappingProfileRoots => {
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
    }
}

fn validate_yaml_catalogs(
    issues: &mut Vec<MediaYamlIssue>,
    bundle: &MediaYamlBundle,
    existing_compatibility_targets: &[AppMediaCompatibilityTargetResponse],
    existing_desired_targets: &[AppMediaDesiredTargetResponse],
    existing_policies: &[AppMediaPolicyResponse],
) {
    let compatibility_keys = bundle
        .compatibility_targets
        .iter()
        .map(|target| normalize_catalog_key(&target.compatibility_target_key))
        .chain(
            existing_compatibility_targets
                .iter()
                .map(|target| normalize_catalog_key(&target.compatibility_target_key)),
        )
        .collect::<BTreeSet<_>>();
    let policy_keys = bundle
        .policies
        .iter()
        .map(|policy| policy.policy_key.trim().to_ascii_lowercase())
        .chain(
            existing_policies
                .iter()
                .map(|policy| policy.policy_key.trim().to_ascii_lowercase()),
        )
        .collect::<BTreeSet<_>>();
    let desired_keys = bundle
        .targets
        .iter()
        .map(|target| {
            (
                target.target_key.trim().to_ascii_lowercase(),
                target.version,
            )
        })
        .chain(existing_desired_targets.iter().map(|target| {
            (
                target.target_key.trim().to_ascii_lowercase(),
                target.version,
            )
        }))
        .collect::<BTreeSet<_>>();

    validate_yaml_catalog_rows(
        issues,
        bundle,
        existing_compatibility_targets,
        existing_desired_targets,
        existing_policies,
    );
    for (index, profile) in bundle.profiles.iter().enumerate() {
        let pointer = format!("/profiles/{index}");
        if profile
            .compatibility_target_key
            .as_deref()
            .and_then(trim_nonempty)
            .is_some_and(|target| !compatibility_keys.contains(&normalize_catalog_key(target)))
        {
            push_yaml_issue(
                issues,
                "media_yaml_compatibility_target_not_found",
                &format!("{pointer}/compatibility_target_key"),
                true,
            );
        }
        if !policy_keys.contains(&profile.policy_key.trim().to_ascii_lowercase()) {
            push_yaml_issue(
                issues,
                "media_yaml_policy_profile_not_found",
                &format!("{pointer}/policy_key"),
                true,
            );
        }
        if let (Some(key), Some(version)) = (
            profile.desired_target_key.as_deref(),
            profile.desired_target_version,
        ) && !desired_keys.contains(&(key.trim().to_ascii_lowercase(), version))
        {
            push_yaml_issue(
                issues,
                "media_yaml_desired_target_not_found",
                &format!("{pointer}/desired_target_key"),
                true,
            );
        }
    }
}

fn validate_yaml_catalog_rows(
    issues: &mut Vec<MediaYamlIssue>,
    bundle: &MediaYamlBundle,
    existing_compatibility_targets: &[AppMediaCompatibilityTargetResponse],
    existing_desired_targets: &[AppMediaDesiredTargetResponse],
    existing_policies: &[AppMediaPolicyResponse],
) {
    validate_unique_catalog_keys(
        issues,
        "/compatibility_targets",
        bundle.compatibility_targets.iter().map(|row| {
            (
                normalize_catalog_key(&row.compatibility_target_key),
                row.version,
            )
        }),
    );
    validate_unique_catalog_keys(
        issues,
        "/policies",
        bundle
            .policies
            .iter()
            .map(|row| (row.policy_key.trim().to_ascii_lowercase(), row.version)),
    );
    validate_unique_catalog_keys(
        issues,
        "/targets",
        bundle
            .targets
            .iter()
            .map(|row| (row.target_key.trim().to_ascii_lowercase(), row.version)),
    );
    validate_yaml_compatibility_rows(
        issues,
        &bundle.compatibility_targets,
        existing_compatibility_targets,
    );
    validate_yaml_policy_rows(issues, &bundle.policies, existing_policies);
    validate_yaml_desired_rows(issues, &bundle.targets, existing_desired_targets);
}

fn validate_yaml_compatibility_rows(
    issues: &mut Vec<MediaYamlIssue>,
    targets: &[MediaYamlCompatibilityTarget],
    existing_targets: &[AppMediaCompatibilityTargetResponse],
) {
    for (index, target) in targets.iter().enumerate() {
        if target.compatibility_target_key.trim().is_empty()
            || target.display_name.trim().is_empty()
            || target.video_codec.trim().is_empty()
            || target.audio_codec.trim().is_empty()
            || target.subtitle_policy.trim().is_empty()
            || !matches!(
                target.subtitle_policy.trim().to_ascii_lowercase().as_str(),
                "selected" | "all" | "none"
            )
            || target.version <= 0
            || target.audio_channels.is_some_and(|channels| channels <= 0)
            || audio_layout_contract_invalid(
                target.audio_channel_layout.as_deref(),
                target.audio_channels,
            )
        {
            push_yaml_issue(
                issues,
                "media_yaml_compatibility_target_invalid",
                &format!("/compatibility_targets/{index}"),
                true,
            );
        }
        if let Some(existing) = existing_targets.iter().find(|existing| {
            normalize_catalog_key(&existing.compatibility_target_key)
                == normalize_catalog_key(&target.compatibility_target_key)
                && existing.version == target.version
        }) && !compatibility_target_matches_yaml(existing, target)
        {
            push_yaml_issue(
                issues,
                "media_yaml_compatibility_target_conflict",
                &format!("/compatibility_targets/{index}"),
                true,
            );
        }
    }
}

fn validate_yaml_policy_rows(
    issues: &mut Vec<MediaYamlIssue>,
    policies: &[MediaYamlPolicy],
    existing_policies: &[AppMediaPolicyResponse],
) {
    for (index, policy) in policies.iter().enumerate() {
        let strictness = policy.verification_strictness.trim().to_ascii_lowercase();
        let strict_checks = policy.verification_mux_validation.enabled()
            && policy.verification_decode_all_streams.enabled()
            && policy.verification_keyframe_seek.enabled()
            && policy.verification_playback_probe.enabled();
        let fast_checks = !policy.verification_decode_all_streams.enabled()
            && !policy.verification_keyframe_seek.enabled()
            && !policy.verification_playback_probe.enabled();
        if policy.policy_key.trim().is_empty()
            || policy.display_name.trim().is_empty()
            || policy.video_intent.trim().is_empty()
            || !matches!(
                policy.video_intent.trim().to_ascii_lowercase().as_str(),
                "general" | "anime" | "archival"
            )
            || policy.version <= 0
            || !matches!(strictness.as_str(), "strict" | "balanced" | "fast")
            || !(0..=60_000).contains(&policy.verification_duration_tolerance_millis)
            || (strictness == "strict" && !strict_checks)
            || (strictness == "fast" && !fast_checks)
        {
            push_yaml_issue(
                issues,
                "media_yaml_policy_invalid",
                &format!("/policies/{index}"),
                true,
            );
        }
        if let Some(existing) = existing_policies.iter().find(|existing| {
            existing
                .policy_key
                .eq_ignore_ascii_case(policy.policy_key.trim())
                && existing.version == policy.version
        }) && !policy_matches_yaml(existing, policy)
        {
            push_yaml_issue(
                issues,
                "media_yaml_policy_conflict",
                &format!("/policies/{index}"),
                true,
            );
        }
    }
}

fn validate_yaml_desired_rows(
    issues: &mut Vec<MediaYamlIssue>,
    targets: &[MediaYamlDesiredTarget],
    existing_targets: &[AppMediaDesiredTargetResponse],
) {
    for (index, target) in targets.iter().enumerate() {
        let pointer = format!("/targets/{index}");
        if yaml_desired_target_shape_invalid(target) {
            push_yaml_issue(issues, "media_yaml_desired_target_invalid", &pointer, true);
        }
        let mut stream_keys = BTreeSet::new();
        let mut stream_orders = BTreeSet::new();
        for (stream_index, stream) in target.streams.iter().enumerate() {
            let stream_pointer = format!("{pointer}/streams/{stream_index}");
            if yaml_desired_stream_invalid(stream, &mut stream_keys, &mut stream_orders) {
                push_yaml_issue(
                    issues,
                    "media_yaml_desired_target_stream_invalid",
                    &stream_pointer,
                    true,
                );
            }
        }
        if let Some(existing) = existing_targets.iter().find(|existing| {
            existing
                .target_key
                .eq_ignore_ascii_case(target.target_key.trim())
                && existing.version == target.version
        }) && !desired_target_matches_yaml(existing, target)
        {
            push_yaml_issue(issues, "media_yaml_desired_target_conflict", &pointer, true);
        }
    }
}

fn yaml_desired_target_shape_invalid(target: &MediaYamlDesiredTarget) -> bool {
    target.target_key.trim().is_empty()
        || target.display_name.trim().is_empty()
        || target.container_format.trim().is_empty()
        || target.version <= 0
        || target.streams.is_empty()
}

fn yaml_desired_stream_invalid(
    stream: &MediaDesiredTargetStreamParams,
    stream_keys: &mut BTreeSet<String>,
    stream_orders: &mut BTreeSet<i32>,
) -> bool {
    let stream_kind = stream.stream_kind.trim().to_ascii_lowercase();
    yaml_stream_identity_invalid(stream, &stream_kind, stream_keys, stream_orders)
        || yaml_audio_constraints_invalid(stream, &stream_kind)
        || yaml_subtitle_constraints_invalid(stream, &stream_kind)
}

fn yaml_stream_identity_invalid(
    stream: &MediaDesiredTargetStreamParams,
    stream_kind: &str,
    stream_keys: &mut BTreeSet<String>,
    stream_orders: &mut BTreeSet<i32>,
) -> bool {
    stream.stream_key.trim().is_empty()
        || stream.codec.trim().is_empty()
        || stream.sort_order < 0
        || !matches!(stream_kind, "video" | "audio" | "subtitle")
        || !stream_keys.insert(stream.stream_key.trim().to_ascii_lowercase())
        || !stream_orders.insert(stream.sort_order)
}

fn yaml_audio_constraints_invalid(
    stream: &MediaDesiredTargetStreamParams,
    stream_kind: &str,
) -> bool {
    if stream_kind == "audio" {
        return stream.channel_count.is_some_and(|count| count <= 0)
            || audio_layout_contract_invalid(
                stream.channel_layout.as_deref(),
                stream.channel_count,
            )
            || stream.audio_bitrate_bps.is_some_and(|bitrate| bitrate <= 0)
            || stream
                .audio_sample_rate_hz
                .is_some_and(|sample_rate| sample_rate <= 0)
            || stream
                .audio_loudness_profile
                .as_deref()
                .is_some_and(|profile| !profile.trim().eq_ignore_ascii_case("dialog-normalized"))
            || stream.audio_dynamic_range.as_deref().is_some_and(|range| {
                !["preserve", "speech"].contains(&range.trim().to_ascii_lowercase().as_str())
            });
    }

    stream.channel_count.is_some()
        || stream.channel_layout.is_some()
        || stream.audio_bitrate_bps.is_some()
        || stream.audio_sample_rate_hz.is_some()
        || stream.audio_loudness_profile.is_some()
        || stream.audio_dynamic_range.is_some()
}

fn audio_layout_contract_invalid(layout: Option<&str>, channels: Option<i32>) -> bool {
    let Some(layout) = layout else {
        return false;
    };
    let Some(canonical_layout) = normalize_audio_channel_layout(layout) else {
        return true;
    };
    let Some(layout_channels) = audio_channel_count_for_layout(canonical_layout) else {
        return true;
    };
    let Ok(layout_channels) = i32::try_from(layout_channels) else {
        return true;
    };
    channels.is_some_and(|channel_count| channel_count != layout_channels)
}

fn yaml_subtitle_constraints_invalid(
    stream: &MediaDesiredTargetStreamParams,
    stream_kind: &str,
) -> bool {
    stream_kind != "subtitle" && stream.forced_disposition
}

fn validate_unique_catalog_keys(
    issues: &mut Vec<MediaYamlIssue>,
    pointer: &str,
    keys: impl Iterator<Item = (String, i32)>,
) {
    let mut seen = BTreeSet::new();
    for (index, key) in keys.enumerate() {
        if key.0.is_empty() || key.1 <= 0 {
            push_yaml_issue(
                issues,
                "media_yaml_catalog_key_invalid",
                &format!("{pointer}/{index}"),
                true,
            );
        } else if !seen.insert(key) {
            push_yaml_issue(
                issues,
                "media_yaml_catalog_key_duplicate",
                &format!("{pointer}/{index}"),
                true,
            );
        }
    }
}

fn push_yaml_issue(issues: &mut Vec<MediaYamlIssue>, code: &str, pointer: &str, blocking: bool) {
    issues.push(MediaYamlIssue {
        code: code.to_string(),
        pointer: pointer.to_string(),
        blocking,
    });
}

fn media_yaml_compatibility_target(
    target: AppMediaCompatibilityTargetResponse,
) -> MediaYamlCompatibilityTarget {
    MediaYamlCompatibilityTarget {
        compatibility_target_key: target.compatibility_target_key,
        version: target.version,
        display_name: target.display_name,
        video_codec: target.video_codec,
        audio_codec: target.audio_codec,
        audio_channels: target.audio_channels,
        audio_channel_layout: target.audio_channel_layout,
        subtitle_policy: target.subtitle_policy,
    }
}

fn media_yaml_desired_target(target: AppMediaDesiredTargetResponse) -> MediaYamlDesiredTarget {
    MediaYamlDesiredTarget {
        target_key: target.target_key,
        version: target.version,
        display_name: target.display_name,
        container_format: target.container_format,
        streams: target.streams,
    }
}

fn media_yaml_policy(policy: AppMediaPolicyResponse) -> MediaYamlPolicy {
    MediaYamlPolicy {
        policy_key: policy.policy_key,
        version: policy.version,
        display_name: policy.display_name,
        video_intent: policy.video_intent,
        verification_strictness: policy.verification_strictness,
        verification_duration_tolerance_millis: policy.verification_duration_tolerance_millis,
        verification_mux_validation: policy.verification_mux_validation,
        verification_decode_all_streams: policy.verification_decode_all_streams,
        verification_keyframe_seek: policy.verification_keyframe_seek,
        verification_playback_probe: policy.verification_playback_probe,
    }
}

fn desired_target_matches_yaml(
    existing: &AppMediaDesiredTargetResponse,
    imported: &MediaYamlDesiredTarget,
) -> bool {
    existing.display_name == imported.display_name
        && existing
            .container_format
            .eq_ignore_ascii_case(&imported.container_format)
        && existing.streams == imported.streams
}

fn compatibility_target_matches_yaml(
    existing: &AppMediaCompatibilityTargetResponse,
    imported: &MediaYamlCompatibilityTarget,
) -> bool {
    existing.display_name == imported.display_name
        && existing
            .video_codec
            .eq_ignore_ascii_case(&imported.video_codec)
        && existing
            .audio_codec
            .eq_ignore_ascii_case(&imported.audio_codec)
        && existing.audio_channels == imported.audio_channels
        && existing.audio_channel_layout == imported.audio_channel_layout
        && existing
            .subtitle_policy
            .eq_ignore_ascii_case(&imported.subtitle_policy)
}

fn policy_matches_yaml(existing: &AppMediaPolicyResponse, imported: &MediaYamlPolicy) -> bool {
    existing.display_name == imported.display_name
        && existing
            .video_intent
            .eq_ignore_ascii_case(&imported.video_intent)
        && existing
            .verification_strictness
            .eq_ignore_ascii_case(&imported.verification_strictness)
        && existing.verification_duration_tolerance_millis
            == imported.verification_duration_tolerance_millis
        && existing.verification_mux_validation == imported.verification_mux_validation
        && existing.verification_decode_all_streams == imported.verification_decode_all_streams
        && existing.verification_keyframe_seek == imported.verification_keyframe_seek
        && existing.verification_playback_probe == imported.verification_playback_probe
}

fn normalize_catalog_key(key: &str) -> String {
    key.trim().replace('_', "-").to_ascii_lowercase()
}

fn trim_nonempty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn map_desired_target_stream(row: MediaDesiredTargetStreamRow) -> MediaDesiredTargetStreamParams {
    MediaDesiredTargetStreamParams {
        stream_key: row.stream_key,
        stream_kind: row.stream_kind,
        semantic_role: row.semantic_role,
        language_code: row.language_code,
        optional: row.optional,
        sort_order: row.sort_order,
        codec: row.codec,
        channel_count: row.channel_count,
        channel_layout: row.channel_layout,
        audio_bitrate_bps: row.audio_bitrate_bps,
        audio_sample_rate_hz: row.audio_sample_rate_hz,
        audio_loudness_profile: row.audio_loudness_profile,
        audio_dynamic_range: row.audio_dynamic_range,
        video_profile: row.video_profile,
        video_level: row.video_level,
        video_bitrate_bps: row.video_bitrate_bps,
        color_primaries: row.color_primaries,
        color_transfer: row.color_transfer,
        color_space: row.color_space,
        hdr_format: row.hdr_format,
        title: row.title,
        default_disposition: row.default_disposition,
        forced_disposition: row.forced_disposition,
        subtitle_placement: row.subtitle_placement,
        image_subtitle_action: row.image_subtitle_action,
    }
}

async fn fingerprint_source_candidate(
    source_path: &str,
    source_root: &str,
) -> Result<Option<crate::media_source_fingerprint::MediaSourceFingerprint>, MediaServiceError> {
    let source_path = PathBuf::from(source_path);
    let source_root = PathBuf::from(source_root);
    tokio::task::spawn_blocking(move || fingerprint_media_file(&source_path, &source_root))
        .await
        .map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Storage)
                .with_code("media_discovery_fingerprint_join_failed")
        })?
        .map_err(|error| map_fingerprint_error(&error))
}

fn map_fingerprint_error(error: &MediaSourceFingerprintError) -> MediaServiceError {
    MediaServiceError::new(MediaServiceErrorKind::Invalid).with_code(match error {
        MediaSourceFingerprintError::Io { .. } => "media_discovery_fingerprint_io",
        MediaSourceFingerprintError::TimeBeforeEpoch => "media_discovery_fingerprint_timestamp",
        MediaSourceFingerprintError::ValueTooLarge(_) => "media_discovery_fingerprint_overflow",
    })
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
        Some(
            "media_profile_roots_overlap"
            | "media_profile_discovery_root_overlap"
            | "media_compatibility_target_not_found"
            | "media_policy_profile_not_found"
            | "media_desired_target_streams_required"
            | "media_job_source_fingerprint_required",
        ) => MediaServiceErrorKind::Invalid,
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
    let Some(code) = capability_snapshot_readiness_code(snapshot) else {
        return Ok(());
    };

    Err(MediaServiceError::new(MediaServiceErrorKind::Invalid).with_code(code))
}

fn capability_snapshot_readiness_code(
    snapshot: Option<&CapabilitySnapshotRow>,
) -> Option<&'static str> {
    let Some(snapshot) = snapshot else {
        return Some("media_capability_snapshot_missing");
    };

    if media_capability_snapshot_row_invalid(snapshot) {
        return Some("media_capability_snapshot_invalid");
    }

    None
}

fn media_capability_snapshot_row_invalid(snapshot: &CapabilitySnapshotRow) -> bool {
    let utilities = feature_names(&snapshot.features, "utility", true);
    snapshot.ffmpeg_version.trim().is_empty()
        || snapshot.ffprobe_version.trim().is_empty()
        || snapshot.codecs.is_empty()
        || snapshot.encoders.is_empty()
        || !snapshot_has_supported_feature(snapshot, "decoder")
        || !snapshot_has_supported_feature(snapshot, "muxer")
        || !snapshot_has_supported_feature(snapshot, "demuxer")
        || !snapshot_has_supported_feature(snapshot, "filesystem")
        || !["ffmpeg", "ffprobe", "ffplay"]
            .iter()
            .all(|required| utilities.iter().any(|actual| actual == required))
        || !snapshot_has_supported_feature(snapshot, "license")
        || snapshot
            .codecs
            .iter()
            .any(|codec| codec.codec_name.trim().is_empty())
}

fn media_capability_snapshot_response_invalid(
    snapshot: &AppMediaCapabilitySnapshotResponse,
) -> bool {
    snapshot.ffmpeg_version.trim().is_empty()
        || snapshot.ffprobe_version.trim().is_empty()
        || snapshot.codecs.is_empty()
        || snapshot.encoders.is_empty()
        || snapshot.decoders.is_empty()
        || snapshot.muxers.is_empty()
        || snapshot.demuxers.is_empty()
        || snapshot.filesystem_utilities.is_empty()
        || snapshot.utility_capabilities.is_empty()
        || !["ffmpeg", "ffprobe", "ffplay"].iter().all(|required| {
            snapshot
                .utility_capabilities
                .iter()
                .any(|actual| actual == required)
        })
        || snapshot.license_mode.trim().is_empty()
        || snapshot
            .codecs
            .iter()
            .any(|codec| codec.codec_name.trim().is_empty())
}

fn media_profile_response(row: MediaProfileRow) -> MediaProfileResponse {
    MediaProfileResponse {
        media_profile_public_id: row.media_profile_public_id,
        profile_key: row.profile_key,
        source_root: row.source_root,
        output_root: row.output_root,
        dry_run_only: row.dry_run_only,
        retention_days: row.retention_days,
        compatibility_target_key: row.compatibility_target_key,
        desired_target_key: row.desired_target_key,
        desired_target_version: row.desired_target_version,
        policy_key: row.policy_key,
        watcher_enabled: row.watcher_enabled,
        schedule_enabled: row.schedule_enabled,
        schedule_interval_minutes: row.schedule_interval_minutes,
        updated_at: row.updated_at,
    }
}

fn shared_media_profile_response(row: MediaProfileRow) -> revaer_api::models::MediaProfileResponse {
    revaer_api::models::MediaProfileResponse {
        media_profile_public_id: row.media_profile_public_id,
        profile_key: row.profile_key,
        source_root: row.source_root,
        output_root: row.output_root,
        dry_run_only: row.dry_run_only,
        retention_days: row.retention_days,
        compatibility_target_key: row.compatibility_target_key,
        desired_target_key: row.desired_target_key,
        desired_target_version: row.desired_target_version,
        policy_key: row.policy_key,
        watcher_enabled: row.watcher_enabled,
        schedule_enabled: row.schedule_enabled,
        schedule_interval_minutes: row.schedule_interval_minutes,
        updated_at: row.updated_at,
    }
}

fn media_capability_snapshot_response(
    row: CapabilitySnapshotRow,
) -> AppMediaCapabilitySnapshotResponse {
    AppMediaCapabilitySnapshotResponse {
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
        ffmpeg_license_mode: feature_names(&row.features, "ffmpeg_license", true)
            .into_iter()
            .next()
            .unwrap_or_else(|| {
                feature_names(&row.features, "license", true)
                    .into_iter()
                    .next()
                    .unwrap_or_default()
            }),
        ffmpeg_enable_gpl: feature_supported(&row.features, "ffmpeg_build_flag", "--enable-gpl"),
        ffmpeg_enable_version3: feature_supported(
            &row.features,
            "ffmpeg_build_flag",
            "--enable-version3",
        ),
        ffmpeg_enable_nonfree: feature_supported(
            &row.features,
            "ffmpeg_build_flag",
            "--enable-nonfree",
        ),
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
    }
}

pub(crate) fn ensure_profile_compatibility_target_readiness(
    profile: &MediaProfileRow,
    snapshot: &CapabilitySnapshotRow,
    compatibility_targets: &[MediaCompatibilityTargetRow],
) -> Result<(), MediaServiceError> {
    let Some(target_key) = profile
        .compatibility_target_key
        .as_deref()
        .and_then(trim_nonempty)
    else {
        return Ok(());
    };
    let normalized_target_key = normalize_catalog_key(target_key);
    let target = compatibility_targets
        .iter()
        .filter(|target| {
            normalize_catalog_key(&target.compatibility_target_key) == normalized_target_key
        })
        .max_by_key(|target| target.version)
        .ok_or_else(|| {
            MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_profile_compatibility_target_not_found")
        })?;

    for codec in [&target.video_codec, &target.audio_codec] {
        if !snapshot_supports_encoding(snapshot, codec) {
            return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_profile_compatibility_target_unsupported"));
        }
    }
    Ok(())
}

fn ensure_profile_desired_target_readiness(
    profile: &MediaProfileRow,
    snapshot: &CapabilitySnapshotRow,
    desired_targets: &[AppMediaDesiredTargetResponse],
    policies: &[MediaPolicyProfileRow],
) -> Result<(), MediaServiceError> {
    let Some(target_key) = profile
        .desired_target_key
        .as_deref()
        .and_then(trim_nonempty)
    else {
        return Ok(());
    };
    let version = profile.desired_target_version.ok_or_else(|| {
        MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_profile_desired_target_reference_incomplete")
    })?;
    let normalized_target_key = normalize_catalog_key(target_key);
    let target = desired_targets
        .iter()
        .find(|target| {
            normalize_catalog_key(&target.target_key) == normalized_target_key
                && target.version == version
        })
        .ok_or_else(|| {
            MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_profile_desired_target_not_found")
        })?;
    let capabilities = runtime_capability_snapshot(snapshot);
    validate_container_muxer_capability(&target.container_format, &capabilities)
        .map_err(|error| map_desired_target_capability_error(&error))?;
    let policy = video_policy_for_profile(profile, policies)?;
    for stream in &target.streams {
        validate_declared_stream_codec_capability(
            desired_target_stream_kind(&stream.stream_kind)?,
            &stream.codec,
            &capabilities,
            &policy,
        )
        .map_err(|error| map_desired_target_capability_error(&error))?;
    }
    Ok(())
}

fn runtime_capability_snapshot(snapshot: &CapabilitySnapshotRow) -> CapabilitySnapshot {
    CapabilitySnapshot {
        ffmpeg_version: snapshot.ffmpeg_version.clone(),
        ffprobe_version: snapshot.ffprobe_version.clone(),
        codecs: snapshot
            .codecs
            .iter()
            .map(|codec| codec.codec_name.clone())
            .collect(),
        codec_support: snapshot
            .codecs
            .iter()
            .map(|codec| CodecCapability {
                name: codec.codec_name.clone(),
                encode_supported: codec.encode_supported,
                decode_supported: codec.decode_supported,
            })
            .collect(),
        encoders: snapshot.encoders.clone(),
        decoders: feature_names(&snapshot.features, "decoder", true),
        muxers: feature_names(&snapshot.features, "muxer", true),
        demuxers: feature_names(&snapshot.features, "demuxer", true),
        hardware_accelerators: feature_names(&snapshot.features, "hardware", true),
        subtitle_support: feature_names(&snapshot.features, "subtitle", true),
        filesystem_utilities: feature_names(&snapshot.features, "filesystem", true),
        utility_capabilities: feature_names(&snapshot.features, "utility", true),
        license_mode: feature_names(&snapshot.features, "license", true)
            .into_iter()
            .next()
            .unwrap_or_default(),
        ffmpeg_license_mode: feature_names(&snapshot.features, "ffmpeg_license", true)
            .into_iter()
            .next()
            .unwrap_or_default(),
        ffmpeg_enable_gpl: feature_supported(
            &snapshot.features,
            "ffmpeg_build_flag",
            "--enable-gpl",
        ),
        ffmpeg_enable_version3: feature_supported(
            &snapshot.features,
            "ffmpeg_build_flag",
            "--enable-version3",
        ),
        ffmpeg_enable_nonfree: feature_supported(
            &snapshot.features,
            "ffmpeg_build_flag",
            "--enable-nonfree",
        ),
        compliance_links: feature_names(&snapshot.features, "compliance", true),
        absent_capabilities: feature_names(&snapshot.features, "absent", false),
    }
}

fn video_policy_for_profile(
    profile: &MediaProfileRow,
    policies: &[MediaPolicyProfileRow],
) -> Result<VideoTranscodePolicy, MediaServiceError> {
    let normalized_policy_key = normalize_catalog_key(&profile.policy_key);
    let policy = policies
        .iter()
        .filter(|policy| normalize_catalog_key(&policy.policy_key) == normalized_policy_key)
        .max_by_key(|policy| policy.version)
        .ok_or_else(|| {
            MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_profile_policy_profile_not_found")
        })?;
    let intent = match policy.video_intent.trim().to_ascii_lowercase().as_str() {
        "general" => VideoTranscodeIntent::General,
        "anime" => VideoTranscodeIntent::Anime,
        "audiobook" => VideoTranscodeIntent::Audiobook,
        "archival" => VideoTranscodeIntent::Archival,
        _ => {
            return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_profile_policy_profile_invalid"));
        }
    };
    Ok(VideoTranscodePolicy {
        intent,
        ..VideoTranscodePolicy::default()
    })
}

fn desired_target_stream_kind(stream_kind: &str) -> Result<StreamKind, MediaServiceError> {
    match stream_kind.trim().to_ascii_lowercase().as_str() {
        "video" => Ok(StreamKind::Video),
        "audio" => Ok(StreamKind::Audio),
        "subtitle" => Ok(StreamKind::Subtitle),
        _ => Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_profile_desired_target_stream_unsupported")),
    }
}

fn map_desired_target_capability_error(error: &BuildArgsError) -> MediaServiceError {
    match error {
        BuildArgsError::UnsupportedMuxer(_) => {
            MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_profile_desired_target_muxer_unsupported")
        }
        BuildArgsError::UnsupportedCodec(_) => {
            MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_profile_desired_target_encoder_unsupported")
        }
        BuildArgsError::UnsupportedDesiredStreamKind { .. } => {
            MediaServiceError::new(MediaServiceErrorKind::Invalid)
                .with_code("media_profile_desired_target_stream_unsupported")
        }
        _ => MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_profile_desired_target_unsupported"),
    }
}

fn snapshot_supports_encoding(snapshot: &CapabilitySnapshotRow, codec_name: &str) -> bool {
    let normalized = codec_name.trim();
    normalized.eq_ignore_ascii_case("copy")
        || snapshot.codecs.iter().any(|codec| {
            codec.codec_name.eq_ignore_ascii_case(normalized) && codec.encode_supported
        })
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
    watcher_enabled: bool,
) -> Result<(), MediaServiceError> {
    match mode {
        DiscoveryRunMode::Manual => Ok(()),
        DiscoveryRunMode::Schedule if schedule_enabled => Ok(()),
        DiscoveryRunMode::Schedule => Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_discovery_schedule_disabled")),
        DiscoveryRunMode::Watcher if watcher_enabled => Ok(()),
        DiscoveryRunMode::Watcher => Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_discovery_watcher_disabled")),
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
        DiscoveryRunMode, MediaService, ensure_discovery_mode_enabled,
        ensure_execution_capability_snapshot, ensure_profile_compatibility_target_readiness,
        ensure_profile_desired_target_readiness, map_data_error, map_detect_error,
        parse_yaml_bundle, path_is_within_root, validate_yaml_bundle,
    };
    use revaer_api::app::media::MediaServiceErrorKind;
    use revaer_api::app::media::{
        MediaCapabilityRefreshParams, MediaCompatibilityTargetUpsertParams,
        MediaDesiredTargetCreateParams, MediaDesiredTargetStreamParams,
        MediaDiscoveryAutomationRunParams, MediaDiscoveryPreviewParams, MediaDiscoveryRunParams,
        MediaFacade, MediaJobArtifactAppendParams, MediaJobCompactAuditAppendParams,
        MediaJobCreateParams, MediaJobOperationAppendParams, MediaJobPlanReasonAppendParams,
        MediaJobRetentionUpdateParams, MediaJobVerificationCheckAppendParams,
        MediaJobViolationAppendParams, MediaPolicyUpsertParams, MediaProfileDesiredTargetParams,
        MediaProfileUpsertParams,
    };
    use revaer_data::DataError;
    use revaer_data::indexers::app_users::{app_user_create, app_user_verify_email};
    use revaer_data::media::capabilities::{
        CapabilityCodecRow, CapabilityFeatureRow, CapabilitySnapshotRow,
    };
    use revaer_data::media::configuration::{MediaCompatibilityTargetRow, MediaPolicyProfileRow};
    use revaer_data::media::imports::list_media_profile_import_drafts;
    use revaer_data::media::profiles::MediaProfileRow;
    use revaer_media_runtime::capabilities::CapabilityDetectError;
    use revaer_media_runtime::capabilities::CapabilityDetector;
    use revaer_media_runtime::capabilities::{CapabilitySnapshot, CodecCapability};
    use revaer_runtime::media::MediaStore;
    use revaer_telemetry::Metrics;
    use revaer_test_support::postgres::{TestDatabase, start_postgres};
    use sqlx::postgres::PgPoolOptions;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;
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

    struct TestMediaService {
        service: MediaService,
        _postgres: TestDatabase,
    }

    struct TestMediaSource {
        _temp_dir: TempDir,
        source_root: String,
        output_root: String,
        source_path: String,
        output_path: String,
    }

    impl std::ops::Deref for TestMediaService {
        type Target = MediaService;

        fn deref(&self) -> &Self::Target {
            &self.service
        }
    }

    async fn setup_media_service(
        detector: Arc<dyn CapabilityDetector>,
    ) -> anyhow::Result<Option<(TestMediaService, Uuid)>> {
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
            TestMediaService {
                service: MediaService::new(store, detector, Metrics::new()?),
                _postgres: postgres,
            },
            actor_user_public_id,
        )))
    }

    fn static_detector() -> Arc<dyn CapabilityDetector> {
        Arc::new(StaticDetector {
            snapshot: CapabilitySnapshot {
                ffmpeg_version: "7.1".to_string(),
                ffprobe_version: "7.1".to_string(),
                codecs: vec!["h264".to_string(), "hevc".to_string()],
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
                utility_capabilities: vec![
                    "ffmpeg".to_string(),
                    "ffprobe".to_string(),
                    "ffplay".to_string(),
                ],
                license_mode: "gpl".to_string(),
                ffmpeg_license_mode: "gpl".to_string(),
                ffmpeg_enable_gpl: true,
                ffmpeg_enable_version3: true,
                ffmpeg_enable_nonfree: false,
                compliance_links: vec!["/app/compliance/SOURCE-OFFER.txt".to_string()],
                absent_capabilities: vec!["--enable-nonfree".to_string()],
            },
        })
    }

    fn valid_capability_features() -> Vec<CapabilityFeatureRow> {
        [
            ("decoder", "hevc", true),
            ("muxer", "matroska", true),
            ("demuxer", "matroska", true),
            ("filesystem", "atomic_rename", true),
            ("utility", "ffmpeg", true),
            ("utility", "ffprobe", true),
            ("utility", "ffplay", true),
            ("license", "gpl", true),
            ("ffmpeg_license", "gpl", true),
            ("ffmpeg_build_flag", "--enable-gpl", true),
            ("ffmpeg_build_flag", "--enable-version3", true),
            ("ffmpeg_build_flag", "--enable-nonfree", false),
        ]
        .into_iter()
        .map(
            |(feature_family, feature_name, supported)| CapabilityFeatureRow {
                feature_family: feature_family.to_string(),
                feature_name: feature_name.to_string(),
                supported,
                detail_text: None,
            },
        )
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
    fn profile_compatibility_readiness_rejects_missing_encode_support() {
        let profile = media_profile_with_compatibility_target("hevc-aac");
        let snapshot = capability_snapshot_with_codecs(&[
            capability_codec("hevc", true, true),
            capability_codec("aac", false, true),
        ]);
        let targets = vec![compatibility_target("hevc-aac", 1, "hevc", "aac")];

        let result = ensure_profile_compatibility_target_readiness(&profile, &snapshot, &targets);

        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_profile_compatibility_target_unsupported".to_string())
        );
    }

    #[test]
    fn profile_compatibility_readiness_accepts_supported_codecs() {
        let profile = media_profile_with_compatibility_target("hevc-aac");
        let snapshot = capability_snapshot_with_codecs(&[
            capability_codec("hevc", true, true),
            capability_codec("aac", true, true),
        ]);
        let targets = vec![compatibility_target("hevc-aac", 1, "hevc", "aac")];

        assert!(
            ensure_profile_compatibility_target_readiness(&profile, &snapshot, &targets).is_ok()
        );
    }

    #[test]
    fn profile_compatibility_readiness_rejects_missing_target() {
        let profile = media_profile_with_compatibility_target("hevc-aac");
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("hevc", true, true)]);

        let result = ensure_profile_compatibility_target_readiness(&profile, &snapshot, &[]);

        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_profile_compatibility_target_not_found".to_string())
        );
    }

    #[test]
    fn profile_desired_target_readiness_rejects_missing_muxer() {
        let profile = media_profile_with_desired_target("living-room-output", 1);
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("hevc", true, true)]);
        let target = desired_target_response("mp4", vec![desired_video_stream()]);
        let policies = vec![policy_profile("safe_dry_run", "general")];

        let result =
            ensure_profile_desired_target_readiness(&profile, &snapshot, &[target], &policies);

        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_profile_desired_target_muxer_unsupported".to_string())
        );
    }

    #[test]
    fn profile_desired_target_readiness_rejects_missing_encoder() {
        let profile = media_profile_with_desired_target("living-room-output", 1);
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("opus", true, true)]);
        let target = desired_target_response("matroska", vec![desired_audio_stream()]);
        let policies = vec![policy_profile("safe_dry_run", "general")];

        let result =
            ensure_profile_desired_target_readiness(&profile, &snapshot, &[target], &policies);

        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_profile_desired_target_encoder_unsupported".to_string())
        );
    }

    #[test]
    fn profile_desired_target_readiness_accepts_unpinned_profile() {
        let profile = media_profile_with_compatibility_target("hevc-aac");
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("hevc", true, true)]);

        assert!(ensure_profile_desired_target_readiness(&profile, &snapshot, &[], &[]).is_ok());
    }

    #[test]
    fn profile_desired_target_readiness_rejects_incomplete_reference() {
        let mut profile = media_profile_with_desired_target("living-room-output", 1);
        profile.desired_target_version = None;
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("hevc", true, true)]);
        let policies = vec![policy_profile("safe_dry_run", "general")];

        let result = ensure_profile_desired_target_readiness(&profile, &snapshot, &[], &policies);

        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_profile_desired_target_reference_incomplete".to_string())
        );
    }

    #[test]
    fn profile_desired_target_readiness_rejects_missing_target() {
        let profile = media_profile_with_desired_target("living-room-output", 1);
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("hevc", true, true)]);
        let policies = vec![policy_profile("safe_dry_run", "general")];

        let result = ensure_profile_desired_target_readiness(&profile, &snapshot, &[], &policies);

        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_profile_desired_target_not_found".to_string())
        );
    }

    #[test]
    fn profile_desired_target_readiness_rejects_missing_policy() {
        let profile = media_profile_with_desired_target("living-room-output", 1);
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("hevc", true, true)]);
        let target = desired_target_response("matroska", vec![desired_video_stream()]);

        let result = ensure_profile_desired_target_readiness(&profile, &snapshot, &[target], &[]);

        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_profile_policy_profile_not_found".to_string())
        );
    }

    #[test]
    fn profile_desired_target_readiness_rejects_invalid_policy_intent() {
        let profile = media_profile_with_desired_target("living-room-output", 1);
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("hevc", true, true)]);
        let target = desired_target_response("matroska", vec![desired_video_stream()]);
        let policies = vec![policy_profile("safe_dry_run", "invalid")];

        let result =
            ensure_profile_desired_target_readiness(&profile, &snapshot, &[target], &policies);

        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_profile_policy_profile_invalid".to_string())
        );
    }

    #[test]
    fn profile_desired_target_readiness_rejects_unsupported_stream_kind() {
        let profile = media_profile_with_desired_target("living-room-output", 1);
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("hevc", true, true)]);
        let mut stream = desired_video_stream();
        stream.stream_kind = "data".to_string();
        let target = desired_target_response("matroska", vec![stream]);
        let policies = vec![policy_profile("safe_dry_run", "general")];

        let result =
            ensure_profile_desired_target_readiness(&profile, &snapshot, &[target], &policies);

        assert_eq!(
            result.err().and_then(|err| err.code().map(str::to_owned)),
            Some("media_profile_desired_target_stream_unsupported".to_string())
        );
    }

    #[test]
    fn profile_desired_target_readiness_accepts_supported_contract() {
        let profile = media_profile_with_desired_target("living-room-output", 1);
        let snapshot = capability_snapshot_with_codecs(&[capability_codec("hevc", true, true)]);
        let target = desired_target_response("matroska", vec![desired_video_stream()]);
        let policies = vec![policy_profile("safe_dry_run", "general")];

        assert!(
            ensure_profile_desired_target_readiness(&profile, &snapshot, &[target], &policies)
                .is_ok()
        );
    }

    fn capability_snapshot_with_codecs(codecs: &[CapabilityCodecRow]) -> CapabilitySnapshotRow {
        CapabilitySnapshotRow {
            media_capability_snapshot_id: 1,
            snapshot_run_public_id: Uuid::new_v4(),
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: codecs.to_vec(),
            encoders: vec!["libx265".to_string()],
            features: valid_capability_features(),
            observed_at: chrono::Utc::now(),
        }
    }

    fn capability_codec(
        codec_name: &str,
        encode_supported: bool,
        decode_supported: bool,
    ) -> CapabilityCodecRow {
        CapabilityCodecRow {
            codec_name: codec_name.to_string(),
            encode_supported,
            decode_supported,
        }
    }

    fn media_profile_with_compatibility_target(target_key: &str) -> MediaProfileRow {
        MediaProfileRow {
            media_profile_public_id: Uuid::new_v4(),
            profile_key: "app-media".to_string(),
            source_root: "/input/app-media".to_string(),
            output_root: "/output/app-media".to_string(),
            dry_run_only: false,
            retention_days: 30,
            compatibility_target_key: Some(target_key.to_string()),
            policy_key: "safe_dry_run".to_string(),
            watcher_enabled: false,
            schedule_enabled: false,
            schedule_interval_minutes: None,
            desired_target_key: None,
            desired_target_version: None,
            updated_at: chrono::Utc::now(),
        }
    }

    fn media_profile_with_desired_target(target_key: &str, version: i32) -> MediaProfileRow {
        MediaProfileRow {
            compatibility_target_key: None,
            desired_target_key: Some(target_key.to_string()),
            desired_target_version: Some(version),
            ..media_profile_with_compatibility_target("unused")
        }
    }

    fn compatibility_target(
        compatibility_target_key: &str,
        version: i32,
        video_codec: &str,
        audio_codec: &str,
    ) -> MediaCompatibilityTargetRow {
        MediaCompatibilityTargetRow {
            compatibility_target_key: compatibility_target_key.to_string(),
            version,
            display_name: "Compatibility target".to_string(),
            video_codec: video_codec.to_string(),
            audio_codec: audio_codec.to_string(),
            audio_channels: Some(2),
            audio_channel_layout: Some("stereo".to_string()),
            subtitle_policy: "selected".to_string(),
        }
    }

    fn desired_target_response(
        container_format: &str,
        streams: Vec<MediaDesiredTargetStreamParams>,
    ) -> super::AppMediaDesiredTargetResponse {
        super::AppMediaDesiredTargetResponse {
            media_desired_target_profile_public_id: Uuid::new_v4(),
            target_key: "living-room-output".to_string(),
            version: 1,
            display_name: "Living room output".to_string(),
            container_format: container_format.to_string(),
            streams,
        }
    }

    fn policy_profile(policy_key: &str, video_intent: &str) -> MediaPolicyProfileRow {
        MediaPolicyProfileRow {
            policy_key: policy_key.to_string(),
            version: 1,
            display_name: "Policy".to_string(),
            video_intent: video_intent.to_string(),
            verification_strictness: "balanced".to_string(),
            verification_duration_tolerance_millis: 250,
            verification_mux_validation: true.into(),
            verification_decode_all_streams: true.into(),
            verification_keyframe_seek: true.into(),
            verification_playback_probe: true.into(),
        }
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
    async fn media_desired_target_create_rejects_empty_stream_contract() -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };

        let result = service
            .media_desired_target_create(MediaDesiredTargetCreateParams {
                actor_user_public_id,
                target_key: "empty-target".to_string(),
                version: 1,
                display_name: "Empty target".to_string(),
                container_format: "matroska".to_string(),
                streams: Vec::new(),
            })
            .await;

        let err = result.expect_err("empty desired target should fail before persistence");
        assert_eq!(err.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(err.code(), Some("media_desired_target_streams_required"));
        assert!(
            service
                .media_desired_target_list()
                .await?
                .iter()
                .all(|target| target.target_key != "empty-target")
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_job_create_rejects_unsupported_profile_compatibility_target()
    -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        assert_capability_refresh_uses_detected_support(&service, actor_user_public_id).await?;
        service
            .media_compatibility_target_upsert(MediaCompatibilityTargetUpsertParams {
                actor_user_public_id,
                compatibility_target_key: "hevc-aac",
                version: 1,
                display_name: "HEVC AAC",
                video_codec: "hevc",
                audio_codec: "aac",
                audio_channels: Some(2),
                audio_channel_layout: Some("stereo"),
                subtitle_policy: "selected",
            })
            .await?;
        let media_source = create_test_media_source("video.mkv")?;
        let profile_id = service
            .media_profile_upsert(MediaProfileUpsertParams {
                actor_user_public_id,
                profile_key: "profile-readiness",
                source_root: &media_source.source_root,
                output_root: &media_source.output_root,
                dry_run_only: false,
                retention_days: 30,
                compatibility_target_key: Some("hevc-aac"),
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            })
            .await?;

        let error = service
            .media_job_create(MediaJobCreateParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_path: &media_source.source_path,
                output_path: Some(&media_source.output_path),
                dry_run: false,
                replace_confirmation: Some("replace"),
            })
            .await
            .expect_err("profile compatibility target should fail before queueing");

        assert_eq!(error.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(
            error.code(),
            Some("media_profile_compatibility_target_unsupported")
        );
        assert!(
            service
                .media_job_list(profile_id, Some("queued"))
                .await?
                .is_empty()
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_profile_readiness_returns_none_for_missing_profile() -> anyhow::Result<()> {
        let Some((service, _actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };

        let readiness = service.media_profile_readiness(Uuid::new_v4()).await?;

        assert!(readiness.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn media_profile_readiness_reports_missing_capability_snapshot() -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        let profile_id = service
            .media_profile_upsert(MediaProfileUpsertParams {
                actor_user_public_id,
                profile_key: "profile-readiness-missing-snapshot",
                source_root: "/input/profile-readiness-missing-snapshot",
                output_root: "/output/profile-readiness-missing-snapshot",
                dry_run_only: false,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            })
            .await?;

        let readiness = service
            .media_profile_readiness(profile_id)
            .await?
            .expect("profile readiness should exist");

        assert!(!readiness.ready);
        assert_eq!(
            readiness.reason.as_deref(),
            Some("media_capability_snapshot_missing")
        );
        assert_eq!(readiness.profile.media_profile_public_id, profile_id);
        assert!(readiness.snapshot.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn media_profile_readiness_reports_unsupported_compatibility_target() -> anyhow::Result<()>
    {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        assert_capability_refresh_uses_detected_support(&service, actor_user_public_id).await?;
        service
            .media_compatibility_target_upsert(MediaCompatibilityTargetUpsertParams {
                actor_user_public_id,
                compatibility_target_key: "hevc-aac",
                version: 1,
                display_name: "HEVC AAC",
                video_codec: "hevc",
                audio_codec: "aac",
                audio_channels: Some(2),
                audio_channel_layout: Some("stereo"),
                subtitle_policy: "selected",
            })
            .await?;
        let profile_id = service
            .media_profile_upsert(MediaProfileUpsertParams {
                actor_user_public_id,
                profile_key: "profile-readiness-api",
                source_root: "/input/profile-readiness-api",
                output_root: "/output/profile-readiness-api",
                dry_run_only: false,
                retention_days: 30,
                compatibility_target_key: Some("hevc-aac"),
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            })
            .await?;

        let readiness = service
            .media_profile_readiness(profile_id)
            .await?
            .expect("profile readiness should exist");

        assert!(!readiness.ready);
        assert_eq!(
            readiness.reason.as_deref(),
            Some("media_profile_compatibility_target_unsupported")
        );
        assert_eq!(readiness.profile.media_profile_public_id, profile_id);
        assert!(readiness.snapshot.is_some());
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
        let media_source = create_test_media_source("show/episode.mkv")?;
        let profile_id = upsert_app_media_profile_with_roots(
            &service,
            actor_user_public_id,
            &media_source.source_root,
            &media_source.output_root,
            false,
            false,
        )
        .await?;
        let source_paths = vec![
            media_source.source_path.clone(),
            format!("{}-other/show/episode.mkv", media_source.source_root),
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
            media_source.source_path
        );
        assert_eq!(
            response.queued_jobs[0].output_path,
            media_source.output_path
        );
        assert!(response.queued_jobs[0].dry_run);
        assert_eq!(
            response.skipped[0].reason.as_deref(),
            Some("media_discovery_source_path_outside_profile_root")
        );
        let jobs = service.media_job_list(profile_id, Some("queued")).await?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].source_path, response.queued_jobs[0].source_path);
        let repeated = service
            .media_discovery_run(MediaDiscoveryRunParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_paths: std::slice::from_ref(&media_source.source_path),
            })
            .await?;
        assert!(repeated.queued_jobs.is_empty());
        assert_eq!(repeated.skipped.len(), 1);
        assert_eq!(
            repeated.skipped[0].reason.as_deref(),
            Some("media_discovery_source_unchanged")
        );
        let rendered = service.telemetry.render()?;
        assert!(rendered_has_metric_labels(
            &rendered,
            "media_discovery_candidates_total",
            &[("source", "manual"), ("outcome", "queued")]
        ));
        assert!(rendered_has_metric_labels(
            &rendered,
            "media_discovery_candidates_total",
            &[("source", "manual"), ("outcome", "skipped")]
        ));
        assert!(rendered_has_metric_labels(
            &rendered,
            "media_jobs_queued_total",
            &[("source", "manual"), ("dry_run", "true")]
        ));
        Ok(())
    }

    fn rendered_has_metric_labels(
        rendered: &str,
        metric_name: &str,
        labels: &[(&str, &str)],
    ) -> bool {
        rendered.lines().any(|line| {
            line.starts_with(metric_name)
                && labels
                    .iter()
                    .all(|(name, value)| line.contains(&format!("{name}=\"{value}\"")))
        })
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

        let enabled_profile_id = {
            let media_source = create_test_media_source("show/schedule.mkv")?;
            let enabled_profile_id = upsert_app_media_profile_with_roots(
                &service,
                actor_user_public_id,
                &media_source.source_root,
                &media_source.output_root,
                false,
                true,
            )
            .await?;
            let enabled_source_paths = vec![media_source.source_path.clone()];
            let response = service
                .media_discovery_schedule_run(MediaDiscoveryAutomationRunParams {
                    actor_user_public_id,
                    media_profile_public_id: enabled_profile_id,
                    source_paths: &enabled_source_paths,
                })
                .await?;

            assert_eq!(response.queued_jobs.len(), 1);
            assert!(response.skipped.is_empty());
            assert_eq!(
                response.queued_jobs[0].output_path,
                media_source.output_path
            );
            enabled_profile_id
        };
        let jobs = service
            .media_job_list(enabled_profile_id, Some("queued"))
            .await?;
        assert_eq!(jobs.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn media_discovery_watcher_run_requires_enabled_watcher_and_queues_paths()
    -> anyhow::Result<()> {
        let Some((service, actor_user_public_id)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        let disabled_profile_id = upsert_app_media_profile(&service, actor_user_public_id).await?;
        let source_paths = vec!["/input/app-media/show/episode.mkv".to_string()];

        let disabled_result = service
            .media_discovery_watcher_run(MediaDiscoveryAutomationRunParams {
                actor_user_public_id,
                media_profile_public_id: disabled_profile_id,
                source_paths: &source_paths,
            })
            .await;
        let Err(disabled_error) = disabled_result else {
            return Err(anyhow::anyhow!("disabled watcher should reject"));
        };
        assert_eq!(disabled_error.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(
            disabled_error.code(),
            Some("media_discovery_watcher_disabled")
        );

        let enabled_profile_id = {
            let media_source = create_test_media_source("show/watcher.mkv")?;
            let enabled_profile_id = upsert_app_media_profile_with_roots(
                &service,
                actor_user_public_id,
                &media_source.source_root,
                &media_source.output_root,
                true,
                false,
            )
            .await?;
            let enabled_source_paths = vec![media_source.source_path.clone()];
            let response = service
                .media_discovery_watcher_run(MediaDiscoveryAutomationRunParams {
                    actor_user_public_id,
                    media_profile_public_id: enabled_profile_id,
                    source_paths: &enabled_source_paths,
                })
                .await?;

            assert_eq!(response.queued_jobs.len(), 1);
            assert!(response.skipped.is_empty());
            assert_eq!(
                response.queued_jobs[0].output_path,
                media_source.output_path
            );
            enabled_profile_id
        };
        let jobs = service
            .media_job_list(enabled_profile_id, Some("queued"))
            .await?;
        assert_eq!(jobs.len(), 1);
        Ok(())
    }

    #[test]
    fn discovery_mode_guard_accepts_enabled_watcher_only() -> anyhow::Result<()> {
        ensure_discovery_mode_enabled(DiscoveryRunMode::Watcher, false, true)?;
        let disabled = ensure_discovery_mode_enabled(DiscoveryRunMode::Watcher, true, false)
            .expect_err("disabled watcher should reject automation run");
        assert_eq!(disabled.kind(), MediaServiceErrorKind::Invalid);
        assert_eq!(disabled.code(), Some("media_discovery_watcher_disabled"));
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
            "format_version: 2\nkind: revaer.media.profile_bundle\nmetadata:\n  name: Invalid\nprofiles:\n  - profile_key: tv\n    source_root: /data\n    output_root: /data\n    dry_run_only: false\n    retention_days: 0\n",
        )
        .expect("bundle");
        let issues = validate_yaml_bundle(&bundle, &[], &[], &[]);
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "media_yaml_version_unsupported")
        );
        assert!(
            issues
                .iter()
                .any(|issue| { issue.code == "media_yaml_profile_retention_days_out_of_bounds" })
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "media_yaml_profile_roots_overlap")
        );
    }

    #[test]
    fn validate_yaml_bundle_rejects_invalid_audio_layout_contracts() {
        let bundle = parse_yaml_bundle(
            "format_version: 1\nkind: revaer.media.profile_bundle\nmetadata:\n  name: Invalid audio layouts\ncompatibility_targets:\n  - compatibility_target_key: bad-compat\n    version: 1\n    display_name: Bad compat\n    video_codec: hevc\n    audio_codec: opus\n    audio_channels: 2\n    audio_channel_layout: 5.1(side)\n    subtitle_policy: selected\ntargets:\n  - target_key: bad-target\n    version: 1\n    display_name: Bad target\n    container_format: matroska\n    streams:\n      - stream_key: audio-main\n        stream_kind: audio\n        semantic_role: primary\n        language_code: eng\n        optional: false\n        sort_order: 0\n        codec: opus\n        channel_count: 2\n        channel_layout: ambisonic\n        default_disposition: true\n        forced_disposition: false\n",
        )
        .expect("bundle");
        let issues = validate_yaml_bundle(&bundle, &[], &[], &[]);

        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "media_yaml_compatibility_target_invalid")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "media_yaml_desired_target_stream_invalid")
        );
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

        let media_source = create_test_media_source("video.mkv")?;
        let profile_id = upsert_app_media_profile_with_roots(
            &service,
            actor_user_public_id,
            &media_source.source_root,
            &media_source.output_root,
            false,
            false,
        )
        .await?;
        assert_profile_is_listed(&service, profile_id).await?;
        assert_capability_refresh_uses_detected_support(&service, actor_user_public_id).await?;

        let job_id =
            create_app_media_job(&service, actor_user_public_id, profile_id, &media_source).await?;
        assert_job_records_round_trip(&service, profile_id, job_id).await?;
        assert_job_cancel_retry(&service, job_id).await?;

        assert_yaml_round_trip(&service, actor_user_public_id).await?;

        Ok(())
    }

    async fn assert_yaml_round_trip(
        service: &MediaService,
        actor_user_public_id: Uuid,
    ) -> anyhow::Result<()> {
        let yaml = service.media_yaml_export(false).await?;
        let validation = service.media_yaml_validate(&yaml).await?;
        assert!(validation.valid);
        let invalid_yaml = [
            "format_version: 1",
            "kind: revaer.media.profile_bundle",
            "metadata:",
            "  name: Invalid catalog references",
            "profiles:",
            "  - profile_key: invalid-catalog",
            "    source_root: /input/app-media-invalid",
            "    output_root: /output/app-media-invalid",
            "    dry_run_only: true",
            "    retention_days: 30",
            "    compatibility_target_key: missing-target",
            "    policy_key: missing-policy",
        ]
        .join("\n");
        let invalid_validation = service.media_yaml_validate(&invalid_yaml).await?;
        assert!(!invalid_validation.valid);
        assert!(
            invalid_validation
                .issues
                .iter()
                .any(|issue| issue.code == "media_yaml_compatibility_target_not_found")
        );
        assert!(
            invalid_validation
                .issues
                .iter()
                .any(|issue| issue.code == "media_yaml_policy_profile_not_found")
        );
        let invalid_apply = service
            .media_yaml_apply(actor_user_public_id, &invalid_yaml)
            .await;
        assert_eq!(
            invalid_apply
                .expect_err("invalid YAML catalog refs should fail apply")
                .code(),
            Some("media_yaml_validation_failed")
        );
        let mut portable_bundle = parse_yaml_bundle(&yaml)?;
        {
            let portable_profile = portable_bundle
                .profiles
                .first_mut()
                .ok_or_else(|| anyhow::anyhow!("exported profile missing"))?;
            portable_profile.profile_key = "portable-copy".to_string();
            portable_profile.source_root = "${revaer.source_root:portable-copy}".to_string();
            portable_profile.output_root = "${revaer.output_root:portable-copy}".to_string();
        }
        let portable_yaml = serde_yaml::to_string(&portable_bundle)?;
        let portable_apply = service
            .media_yaml_apply(actor_user_public_id, &portable_yaml)
            .await?;
        assert!(portable_apply.forced_dry_run);
        assert!(portable_apply.media_profile_public_ids.is_empty());
        assert_eq!(
            portable_apply.media_profile_import_draft_public_ids.len(),
            1
        );
        let drafts = list_media_profile_import_drafts(service.store.pool()).await?;
        assert_eq!(drafts.len(), 1);
        assert!(!drafts[0].source_root_resolved);
        assert!(!drafts[0].output_root_resolved);
        let exported_with_draft = service.media_yaml_export(false).await?;
        assert!(exported_with_draft.contains("profile_key: portable-copy"));

        let portable_profile = portable_bundle
            .profiles
            .first_mut()
            .ok_or_else(|| anyhow::anyhow!("portable profile missing"))?;
        portable_profile.source_root = "/input/portable-copy".to_string();
        portable_profile.output_root = "/output/portable-copy".to_string();
        let mapped_yaml = serde_yaml::to_string(&portable_bundle)?;
        let mapped = service
            .media_yaml_apply(actor_user_public_id, &mapped_yaml)
            .await?;
        assert_eq!(mapped.media_profile_public_ids.len(), 1);
        assert!(mapped.media_profile_import_draft_public_ids.is_empty());
        assert!(
            list_media_profile_import_drafts(service.store.pool())
                .await?
                .is_empty()
        );

        let local_yaml = service.media_yaml_export(true).await?;
        let local_validation = service.media_yaml_validate(&local_yaml).await?;
        assert!(local_validation.valid);
        let applied = service
            .media_yaml_apply(actor_user_public_id, &local_yaml)
            .await?;
        assert!(applied.forced_dry_run);
        assert_eq!(applied.media_profile_public_ids.len(), 2);
        assert!(
            list_media_profile_import_drafts(service.store.pool())
                .await?
                .is_empty()
        );

        Ok(())
    }

    #[tokio::test]
    async fn portable_yaml_moves_complete_configuration_between_isolated_instances()
    -> anyhow::Result<()> {
        let Some((source, source_actor)) = setup_media_service(static_detector()).await? else {
            return Ok(());
        };
        let portable_yaml = configure_portable_source(&source, source_actor).await?;
        let Some((destination, destination_actor)) = setup_media_service(static_detector()).await?
        else {
            return Ok(());
        };
        assert_portable_import(&destination, destination_actor, &portable_yaml).await
    }

    async fn configure_portable_source(
        source: &MediaService,
        source_actor: Uuid,
    ) -> anyhow::Result<String> {
        source
            .media_compatibility_target_upsert(MediaCompatibilityTargetUpsertParams {
                actor_user_public_id: source_actor,
                compatibility_target_key: "portable-client",
                version: 3,
                display_name: "Portable client",
                video_codec: "hevc",
                audio_codec: "opus",
                audio_channels: Some(2),
                audio_channel_layout: Some("stereo"),
                subtitle_policy: "all",
            })
            .await?;
        source
            .media_policy_upsert(MediaPolicyUpsertParams {
                actor_user_public_id: source_actor,
                policy_key: "portable-strict",
                version: 2,
                display_name: "Portable strict",
                video_intent: "archival",
                verification_strictness: "strict",
                verification_duration_tolerance_millis: 100,
                verification_mux_validation: true.into(),
                verification_decode_all_streams: true.into(),
                verification_keyframe_seek: true.into(),
                verification_playback_probe: true.into(),
            })
            .await?;
        source
            .media_desired_target_create(MediaDesiredTargetCreateParams {
                actor_user_public_id: source_actor,
                target_key: "portable-ordered-target".to_string(),
                version: 4,
                display_name: "Portable ordered target".to_string(),
                container_format: "matroska".to_string(),
                streams: vec![
                    desired_video_stream(),
                    desired_audio_stream(),
                    desired_subtitle_stream(),
                ],
            })
            .await?;
        let profile_id = source
            .media_profile_upsert(MediaProfileUpsertParams {
                actor_user_public_id: source_actor,
                profile_key: "portable-library",
                source_root: "/source/portable-library",
                output_root: "/output/portable-library",
                dry_run_only: false,
                retention_days: 45,
                compatibility_target_key: Some("portable-client"),
                policy_key: "portable-strict",
                watcher_enabled: true,
                schedule_enabled: true,
                schedule_interval_minutes: Some(30),
            })
            .await?;
        source
            .media_profile_desired_target_set(MediaProfileDesiredTargetParams {
                actor_user_public_id: source_actor,
                media_profile_public_id: profile_id,
                target_key: Some("portable-ordered-target".to_string()),
                version: Some(4),
            })
            .await?;

        source.media_yaml_export(false).await.map_err(Into::into)
    }

    async fn assert_portable_import(
        destination: &MediaService,
        destination_actor: Uuid,
        portable_yaml: &str,
    ) -> anyhow::Result<()> {
        assert!(
            !destination
                .media_compatibility_target_list()
                .await?
                .iter()
                .any(|target| target.compatibility_target_key == "portable-client")
        );
        let validation = destination.media_yaml_validate(portable_yaml).await?;
        assert!(validation.valid);
        assert!(validation.issues.iter().any(|issue| {
            issue.code == "media_yaml_local_path_mapping_required" && !issue.blocking
        }));
        let drafted = destination
            .media_yaml_apply(destination_actor, portable_yaml)
            .await?;
        assert!(drafted.media_profile_public_ids.is_empty());
        assert_eq!(drafted.media_profile_import_draft_public_ids.len(), 1);

        let mut mapped_bundle = parse_yaml_bundle(portable_yaml)?;
        let mapped_profile = mapped_bundle
            .profiles
            .iter_mut()
            .find(|profile| profile.profile_key == "portable-library")
            .ok_or_else(|| anyhow::anyhow!("portable profile missing"))?;
        mapped_profile.source_root = "/mapped/source/portable-library".to_string();
        mapped_profile.output_root = "/mapped/output/portable-library".to_string();
        let mapped_yaml = serde_yaml::to_string(&mapped_bundle)?;
        let applied = destination
            .media_yaml_apply(destination_actor, &mapped_yaml)
            .await?;
        assert_eq!(applied.media_profile_public_ids.len(), 1);
        assert!(applied.media_profile_import_draft_public_ids.is_empty());

        let target = destination
            .media_desired_target_list()
            .await?
            .into_iter()
            .find(|target| target.target_key == "portable-ordered-target")
            .ok_or_else(|| anyhow::anyhow!("imported target missing"))?;
        assert_eq!(target.version, 4);
        assert_eq!(
            target
                .streams
                .iter()
                .map(|stream| stream.stream_key.as_str())
                .collect::<Vec<_>>(),
            vec!["video-main", "audio-main", "subtitle-forced"]
        );
        let imported_profile = destination
            .media_profile_list()
            .await?
            .into_iter()
            .find(|profile| profile.profile_key == "portable-library")
            .ok_or_else(|| anyhow::anyhow!("imported profile missing"))?;
        assert!(imported_profile.dry_run_only);
        assert!(!imported_profile.watcher_enabled);
        assert!(!imported_profile.schedule_enabled);
        assert_eq!(
            imported_profile.desired_target_key.as_deref(),
            Some("portable-ordered-target")
        );
        assert_eq!(imported_profile.desired_target_version, Some(4));
        Ok(())
    }

    async fn assert_desired_target_round_trip(
        service: &MediaService,
        actor_user_public_id: Uuid,
    ) -> anyhow::Result<()> {
        let desired = service
            .media_desired_target_create(MediaDesiredTargetCreateParams {
                actor_user_public_id,
                target_key: "living-room-output".to_string(),
                version: 1,
                display_name: "Living room output".to_string(),
                container_format: "matroska".to_string(),
                streams: vec![desired_video_stream(), desired_audio_stream()],
            })
            .await?;
        assert_eq!(desired.target_key, "living-room-output");
        assert_eq!(desired.streams.len(), 2);
        let listed = service.media_desired_target_list().await?;
        assert!(listed.iter().any(|target| {
            target.media_desired_target_profile_public_id
                == desired.media_desired_target_profile_public_id
                && target.streams[1].channel_count == Some(2)
        }));
        let profile_id = upsert_app_media_profile(service, actor_user_public_id).await?;
        service
            .media_profile_desired_target_set(MediaProfileDesiredTargetParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                target_key: Some("living-room-output".to_string()),
                version: Some(1),
            })
            .await?;
        let profile = service
            .media_profile_list()
            .await?
            .into_iter()
            .find(|profile| profile.media_profile_public_id == profile_id)
            .ok_or_else(|| anyhow::anyhow!("pinned media profile missing"))?;
        assert_eq!(
            profile.desired_target_key.as_deref(),
            Some("living-room-output")
        );
        assert_eq!(profile.desired_target_version, Some(1));
        Ok(())
    }

    fn desired_video_stream() -> MediaDesiredTargetStreamParams {
        MediaDesiredTargetStreamParams {
            stream_key: "video-main".to_string(),
            stream_kind: "video".to_string(),
            semantic_role: None,
            language_code: None,
            optional: false,
            sort_order: 0,
            codec: "hevc".to_string(),
            channel_count: None,
            channel_layout: None,
            audio_bitrate_bps: None,
            audio_sample_rate_hz: None,
            audio_loudness_profile: None,
            audio_dynamic_range: None,
            video_profile: Some("main10".to_string()),
            video_level: Some("5.1".to_string()),
            video_bitrate_bps: Some(8_000_000),
            color_primaries: Some("bt2020".to_string()),
            color_transfer: Some("smpte2084".to_string()),
            color_space: Some("bt2020nc".to_string()),
            hdr_format: Some("hdr10".to_string()),
            title: None,
            default_disposition: true,
            forced_disposition: false,
            subtitle_placement: None,
            image_subtitle_action: None,
        }
    }

    fn desired_audio_stream() -> MediaDesiredTargetStreamParams {
        MediaDesiredTargetStreamParams {
            stream_key: "audio-main".to_string(),
            stream_kind: "audio".to_string(),
            semantic_role: Some("primary".to_string()),
            language_code: Some("eng".to_string()),
            optional: false,
            sort_order: 1,
            codec: "opus".to_string(),
            channel_count: Some(2),
            channel_layout: Some("stereo".to_string()),
            audio_bitrate_bps: Some(160_000),
            audio_sample_rate_hz: Some(48_000),
            audio_loudness_profile: Some("dialog-normalized".to_string()),
            audio_dynamic_range: Some("speech".to_string()),
            video_profile: None,
            video_level: None,
            video_bitrate_bps: None,
            color_primaries: None,
            color_transfer: None,
            color_space: None,
            hdr_format: None,
            title: Some("English".to_string()),
            default_disposition: true,
            forced_disposition: false,
            subtitle_placement: None,
            image_subtitle_action: None,
        }
    }

    fn desired_subtitle_stream() -> MediaDesiredTargetStreamParams {
        MediaDesiredTargetStreamParams {
            stream_key: "subtitle-forced".to_string(),
            stream_kind: "subtitle".to_string(),
            semantic_role: Some("forced".to_string()),
            language_code: Some("eng".to_string()),
            optional: true,
            sort_order: 2,
            codec: "webvtt".to_string(),
            channel_count: None,
            channel_layout: None,
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
            title: Some("English forced".to_string()),
            default_disposition: false,
            forced_disposition: true,
            subtitle_placement: Some("embedded".to_string()),
            image_subtitle_action: Some("fail".to_string()),
        }
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
                audio_channels: Some(2),
                audio_channel_layout: Some("stereo"),
                subtitle_policy: "selected",
            })
            .await?;
        assert_eq!(target.compatibility_target_key, "plex-living-room");
        assert_eq!(target.version, 2);
        assert_eq!(target.audio_channels, Some(2));
        assert_eq!(target.audio_channel_layout.as_deref(), Some("stereo"));
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
        assert_desired_target_round_trip(&service, actor_user_public_id).await?;

        let policy = service
            .media_policy_upsert(MediaPolicyUpsertParams {
                actor_user_public_id,
                policy_key: "living-room",
                version: 3,
                display_name: "Living room",
                video_intent: "general",
                verification_strictness: "strict",
                verification_duration_tolerance_millis: 100,
                verification_mux_validation: true.into(),
                verification_decode_all_streams: true.into(),
                verification_keyframe_seek: true.into(),
                verification_playback_probe: true.into(),
            })
            .await?;
        assert_eq!(policy.policy_key, "living-room");
        assert_eq!(policy.version, 3);
        assert_eq!(policy.video_intent, "general");
        assert_eq!(policy.verification_strictness, "strict");
        assert!(policy.verification_playback_probe.enabled());
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
                completed_enabled: true,
                completed_mode: "count",
                completed_limit: 45,
                failed_diagnostic_enabled: true,
                failed_diagnostic_mode: "age",
                failed_diagnostic_limit: 90,
            })
            .await?;
        assert!(retention.completed_enabled);
        assert_eq!(retention.completed_mode, "count");
        assert_eq!(retention.completed_limit, 45);
        assert_eq!(retention.failed_diagnostic_limit, 90);
        let refreshed_retention = service.media_job_retention().await?;
        assert!(refreshed_retention.completed_enabled);
        assert_eq!(refreshed_retention.completed_mode, "count");
        assert_eq!(refreshed_retention.completed_limit, 45);
        assert_eq!(refreshed_retention.failed_diagnostic_limit, 90);
        Ok(())
    }

    async fn create_app_media_job(
        service: &MediaService,
        actor_user_public_id: Uuid,
        profile_id: Uuid,
        media_source: &TestMediaSource,
    ) -> anyhow::Result<Uuid> {
        service
            .media_job_create(MediaJobCreateParams {
                actor_user_public_id,
                media_profile_public_id: profile_id,
                source_path: &media_source.source_path,
                output_path: Some(&media_source.output_path),
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
        let phases = service.media_job_phase_list(job_id).await?;
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].phase_index, 0);
        assert_eq!(phases[0].phase_name, "plan");
        assert_eq!(phases[0].phase_status, "queued");
        assert_eq!(phases[0].details_text.as_deref(), Some("ok"));
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

    async fn upsert_app_media_profile_with_roots(
        service: &MediaService,
        actor_user_public_id: Uuid,
        source_root: &str,
        output_root: &str,
        watcher_enabled: bool,
        schedule_enabled: bool,
    ) -> anyhow::Result<Uuid> {
        let profile_key = format!("app-media-{}", Uuid::new_v4());
        service
            .media_profile_upsert(MediaProfileUpsertParams {
                actor_user_public_id,
                profile_key: &profile_key,
                source_root,
                output_root,
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

    fn create_test_media_source(relative_path: &str) -> anyhow::Result<TestMediaSource> {
        let temp_dir = tempfile::tempdir()?;
        let source_root_path = temp_dir.path().join("source");
        let output_root_path = temp_dir.path().join("output");
        let source_path = source_root_path.join(relative_path);
        let output_path = output_root_path.join(relative_path);
        if let Some(parent) = source_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&output_root_path)?;
        fs::write(&source_path, b"stable media test bytes")?;
        Ok(TestMediaSource {
            _temp_dir: temp_dir,
            source_root: source_root_path.to_string_lossy().into_owned(),
            output_root: output_root_path.to_string_lossy().into_owned(),
            source_path: source_path.to_string_lossy().into_owned(),
            output_path: output_path.to_string_lossy().into_owned(),
        })
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
            .store
            .append_job_phase(job_id, 0, "plan", "queued", Some("ok"))
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
