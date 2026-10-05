//! Mode-fenced admission using retained roots and real aggregate fingerprints.

mod admission;

pub(crate) use admission::{AdmissionBudget, AdmissionControl};

use std::{
    collections::BTreeSet,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use revaer_api::{app::media::MediaAssociationRunTrigger, models::MediaDiscoveryPreviewRequest};
use revaer_data::media::association_jobs::{
    AssociationFingerprint, AssociationJobInput, enqueue_association_job,
};
use revaer_data::media::associations::{AssociationModes, AssociationRow};
use uuid::Uuid;

use super::{
    MediaDiscoveryPreviewResponse, MediaDiscoveryQueuedJobResponse, MediaDiscoveryRunResponse,
    MediaDiscoverySkippedItemResponse, MediaService, MediaServiceError, MediaServiceErrorKind,
    map_data_error,
};
use crate::media_discovery_fingerprint::{FingerprintError, MediaAggregateFingerprint};

pub(crate) struct AdmissionBatch {
    pub(crate) response: MediaDiscoveryRunResponse,
    pub(crate) pending: Vec<String>,
    pub(crate) aggregate_bytes: u64,
    pub(crate) metadata_elapsed: Duration,
}

#[derive(Clone, Copy)]
pub(crate) struct SourceGeneration {
    pub(crate) number: i64,
    pub(crate) sha256: [u8; 32],
}

pub(crate) trait AssociationSource: Send + Sync {
    fn generation(&self) -> SourceGeneration;
    fn watch_directory(
        &self,
        logical_key: &str,
        relative_prefix: &Path,
    ) -> Result<std::os::fd::OwnedFd, FingerprintError>;

    fn fingerprint(
        &self,
        logical_key: &str,
        relative_path: &Path,
        cancelled: &dyn Fn() -> bool,
        hash_elapsed: &mut Duration,
    ) -> Result<Option<MediaAggregateFingerprint>, FingerprintError>;
    fn scan(
        &self,
        logical_key: &str,
        relative_prefix: &Path,
        budget: &crate::media_discovery_scan::ScanBudget,
        cursor: Option<crate::media_discovery_scan::ScanCursor>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<crate::media_discovery_scan::ScanBatch, crate::media_discovery_scan::ScanError>;
}

pub(super) async fn run(
    service: &MediaService,
    actor: Uuid,
    request: &MediaDiscoveryPreviewRequest,
    trigger: MediaAssociationRunTrigger,
    mut control: AdmissionControl,
) -> Result<(Uuid, AdmissionBatch), MediaServiceError> {
    let started = Instant::now();
    request.validate().map_err(|_| {
        MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_configuration_invalid")
    })?;
    let row = ready_association(
        service,
        request.media_discovery_association_public_id,
        trigger,
    )
    .await?;
    let source = service.source.as_ref().ok_or_else(|| {
        MediaServiceError::new(MediaServiceErrorKind::Unavailable)
            .with_code("media_root_binding_incomplete")
    })?;
    let previews = super::associations::preview_scope(
        &row.root_relative_path,
        row.effective_dry_run,
        &request.source_paths,
    );
    let preparation_elapsed = started.elapsed();
    if let Some(budget) = control.budget.as_mut() {
        budget.validate_metadata(preparation_elapsed)?;
        budget.batch_metadata = budget.batch_metadata.saturating_sub(preparation_elapsed);
        budget.remaining_run_metadata = budget
            .remaining_run_metadata
            .saturating_sub(preparation_elapsed);
    }
    let mut response =
        admit_previews(service, actor, &row, previews, source, trigger, control).await?;
    response.metadata_elapsed = response
        .metadata_elapsed
        .saturating_add(preparation_elapsed);
    Ok((row.media_profile_public_id, response))
}

async fn admit_previews(
    service: &MediaService,
    actor: Uuid,
    row: &AssociationRow,
    previews: Vec<MediaDiscoveryPreviewResponse>,
    source: &Arc<dyn AssociationSource>,
    trigger: MediaAssociationRunTrigger,
    control: AdmissionControl,
) -> Result<AdmissionBatch, MediaServiceError> {
    let generation = source.generation();
    let metric_source = trigger.as_str();
    let mut queued_jobs = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = BTreeSet::new();
    let mut aggregate_bytes = 0_u64;
    let mut metadata_elapsed = Duration::ZERO;
    let mut pending = Vec::new();
    let mut previews = previews.into_iter();
    while let Some(preview) = previews.next() {
        ensure_running(control.cancelled.as_deref())?;
        if let Some(rejected) = reject_preview(service, metric_source, &preview, &mut seen) {
            skipped.push(rejected);
            continue;
        }
        if control
            .budget
            .is_some_and(|budget| !budget.metadata_available(metadata_elapsed))
        {
            pending.push(preview.source_path);
            pending.extend(previews.map(|item| item.source_path));
            break;
        }
        let (fingerprint, fingerprint_metadata) = fingerprint(
            source,
            &row.source_root_key,
            &preview.source_path,
            control.cancelled.clone(),
        )
        .await?;
        metadata_elapsed = metadata_elapsed.saturating_add(fingerprint_metadata);
        if let Some(budget) = control.budget {
            budget.validate_metadata(metadata_elapsed)?;
        }
        ensure_running(control.cancelled.as_deref())?;
        let Some(fingerprint) = fingerprint else {
            service
                .telemetry
                .inc_media_discovery_candidate(metric_source, "unstable");
            skipped.push(MediaDiscoverySkippedItemResponse {
                source_path: preview.source_path,
                reason: Some("media_discovery_source_unstable".into()),
            });
            continue;
        };
        let size = u64::try_from(fingerprint.size_bytes).map_err(|_| {
            MediaServiceError::new(MediaServiceErrorKind::Unavailable)
                .with_code("media_discovery_source_unstable")
        })?;
        if let Some(budget) = control.budget
            && !budget.admit(aggregate_bytes, size)?
        {
            pending.push(preview.source_path);
            pending.extend(previews.map(|item| item.source_path));
            break;
        }
        aggregate_bytes = aggregate_bytes.checked_add(size).ok_or_else(|| {
            MediaServiceError::new(MediaServiceErrorKind::Unavailable)
                .with_code("media_discovery_aggregate_limit")
        })?;
        let enqueue_started = Instant::now();
        let admitted = enqueue_candidate(
            service,
            actor,
            row,
            &preview.source_path,
            &fingerprint,
            trigger,
            generation,
        )
        .await?;
        metadata_elapsed = metadata_elapsed.saturating_add(enqueue_started.elapsed());
        if let Some(budget) = control.budget {
            budget.validate_metadata(metadata_elapsed)?;
        }
        if let Some(job) = admitted {
            queued_jobs.push(job);
        } else {
            skipped.push(MediaDiscoverySkippedItemResponse {
                source_path: preview.source_path,
                reason: Some("media_discovery_source_unchanged".into()),
            });
        }
    }
    Ok(AdmissionBatch {
        response: MediaDiscoveryRunResponse {
            queued_jobs,
            skipped,
        },
        pending,
        aggregate_bytes,
        metadata_elapsed,
    })
}

fn reject_preview(
    service: &MediaService,
    metric_source: &str,
    preview: &MediaDiscoveryPreviewResponse,
    seen: &mut BTreeSet<String>,
) -> Option<MediaDiscoverySkippedItemResponse> {
    if preview.accepted && seen.insert(preview.source_path.clone()) {
        return None;
    }
    let outcome = if preview.accepted {
        "deduplicated"
    } else {
        "skipped"
    };
    service
        .telemetry
        .inc_media_discovery_candidate(metric_source, outcome);
    Some(MediaDiscoverySkippedItemResponse {
        source_path: preview.source_path.clone(),
        reason: Some(
            preview
                .reason
                .clone()
                .unwrap_or_else(|| "media_discovery_source_path_duplicate".into()),
        ),
    })
}

async fn enqueue_candidate(
    service: &MediaService,
    actor: Uuid,
    row: &AssociationRow,
    path: &str,
    fingerprint: &MediaAggregateFingerprint,
    trigger: MediaAssociationRunTrigger,
    generation: SourceGeneration,
) -> Result<Option<MediaDiscoveryQueuedJobResponse>, MediaServiceError> {
    let admitted = enqueue_association_job(
        service.store.pool(),
        &AssociationJobInput {
            actor_public_id: actor,
            association_public_id: row.media_discovery_association_public_id,
            association_version: row.latest_version,
            relative_path: path,
            dry_run: row.effective_dry_run,
            trigger: trigger.as_str(),
            generation: generation.number,
            generation_sha256: generation.sha256,
            fingerprint: AssociationFingerprint {
                identity: &fingerprint.identity,
                size_bytes: fingerprint.size_bytes,
                modified_ns: fingerprint.modified_ns,
                changed_ns: fingerprint.changed_ns,
                sha256: &fingerprint.sha256,
            },
        },
    )
    .await
    .map_err(|error| {
        service
            .telemetry
            .inc_media_discovery_candidate(trigger.as_str(), "queue_failed");
        map_data_error(&error)
    })?;
    let Some(job) = admitted else {
        service
            .telemetry
            .inc_media_discovery_candidate(trigger.as_str(), "deduplicated");
        return Ok(None);
    };
    service
        .telemetry
        .inc_media_discovery_candidate(trigger.as_str(), "queued");
    service
        .telemetry
        .inc_media_job_queued(trigger.as_str(), job.dry_run);
    Ok(Some(MediaDiscoveryQueuedJobResponse {
        media_job_public_id: job.media_job_public_id,
        output_path: path.to_owned(),
        source_path: path.to_owned(),
        dry_run: job.dry_run,
    }))
}

async fn ready_association(
    service: &MediaService,
    id: Uuid,
    trigger: MediaAssociationRunTrigger,
) -> Result<AssociationRow, MediaServiceError> {
    let row = revaer_data::media::associations::read_association(service.store.pool(), id)
        .await
        .map_err(|error| map_data_error(&error))?
        .ok_or_else(|| {
            MediaServiceError::new(MediaServiceErrorKind::NotFound)
                .with_code("media_association_not_found")
        })?;
    ensure_mode_enabled(&row.modes, trigger)?;
    if !row.binding_ready
        || row.active_version != Some(row.latest_version)
        || (!row.effective_dry_run && !row.destructive_ready)
    {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Unavailable)
            .with_code("media_root_binding_incomplete"));
    }
    let capability = service
        .store
        .latest_capability()
        .await
        .map_err(|error| map_data_error(&error))?;
    if let Some(code) = super::capability_snapshot_readiness_code(capability.as_ref()) {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Unavailable).with_code(code));
    }
    Ok(row)
}

fn ensure_mode_enabled(
    modes: &AssociationModes,
    trigger: MediaAssociationRunTrigger,
) -> Result<(), MediaServiceError> {
    let (enabled, code) = match trigger {
        MediaAssociationRunTrigger::Manual => {
            (modes.manual_enabled, "media_discovery_manual_disabled")
        }
        MediaAssociationRunTrigger::Schedule => {
            (modes.schedule_enabled, "media_discovery_schedule_disabled")
        }
        MediaAssociationRunTrigger::Watcher => {
            (modes.watcher_enabled, "media_discovery_watcher_disabled")
        }
    };
    if enabled {
        Ok(())
    } else {
        Err(MediaServiceError::new(MediaServiceErrorKind::Invalid).with_code(code))
    }
}

async fn fingerprint(
    source: &Arc<dyn AssociationSource>,
    key: &str,
    path: &str,
    cancelled: Option<Arc<AtomicBool>>,
) -> Result<(Option<MediaAggregateFingerprint>, Duration), MediaServiceError> {
    let retained = Arc::clone(source);
    let key = key.to_owned();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let mut hash_elapsed = Duration::ZERO;
        let fingerprint = retained.fingerprint(
            &key,
            Path::new(&path),
            &|| {
                cancelled
                    .as_ref()
                    .is_some_and(|signal| signal.load(Ordering::Relaxed))
            },
            &mut hash_elapsed,
        );
        fingerprint.map(|value| (value, started.elapsed().saturating_sub(hash_elapsed)))
    })
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "media association fingerprint task failed");
        MediaServiceError::new(MediaServiceErrorKind::Storage)
    })?
    .map_err(|error| {
        if matches!(error, FingerprintError::Cancelled) {
            return interrupted();
        }
        tracing::error!(error = %error, "media association fingerprint failed");
        MediaServiceError::new(MediaServiceErrorKind::Unavailable)
            .with_code("media_discovery_source_unstable")
    })
}

fn interrupted() -> MediaServiceError {
    MediaServiceError::new(MediaServiceErrorKind::Unavailable)
        .with_code("media_discovery_shutdown_interrupted")
}

fn ensure_running(cancelled: Option<&AtomicBool>) -> Result<(), MediaServiceError> {
    if cancelled.is_some_and(|signal| signal.load(Ordering::Relaxed)) {
        Err(interrupted())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AssociationModes, MediaAssociationRunTrigger, ensure_mode_enabled};

    #[test]
    fn each_trigger_requires_its_own_mode() {
        for bits in 0_u8..8 {
            let modes = AssociationModes {
                manual_enabled: bits & 1 != 0,
                schedule_enabled: bits & 2 != 0,
                watcher_enabled: bits & 4 != 0,
            };
            for (trigger, enabled, code) in [
                (
                    MediaAssociationRunTrigger::Manual,
                    modes.manual_enabled,
                    "media_discovery_manual_disabled",
                ),
                (
                    MediaAssociationRunTrigger::Schedule,
                    modes.schedule_enabled,
                    "media_discovery_schedule_disabled",
                ),
                (
                    MediaAssociationRunTrigger::Watcher,
                    modes.watcher_enabled,
                    "media_discovery_watcher_disabled",
                ),
            ] {
                let result = ensure_mode_enabled(&modes, trigger);
                assert_eq!(result.is_ok(), enabled);
                if let Err(error) = result {
                    assert_eq!(error.code(), Some(code));
                }
                assert_eq!(
                    code,
                    format!("media_discovery_{}_disabled", trigger.as_str())
                );
            }
        }
    }
}
