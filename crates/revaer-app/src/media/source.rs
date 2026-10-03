//! Mode-fenced admission using retained roots and real aggregate fingerprints.

use std::{collections::BTreeSet, path::Path, sync::Arc};

use revaer_api::{app::media::MediaAssociationRunTrigger, models::MediaDiscoveryPreviewRequest};
use revaer_data::media::association_jobs::{
    AssociationFingerprint, AssociationJobInput, enqueue_association_job,
};
use revaer_data::media::associations::{AssociationModes, AssociationRow};
use uuid::Uuid;

use super::{
    MediaDiscoveryQueuedJobResponse, MediaDiscoveryRunResponse, MediaDiscoverySkippedItemResponse,
    MediaService, MediaServiceError, MediaServiceErrorKind, map_data_error,
};
use crate::media_discovery_fingerprint::{FingerprintError, MediaAggregateFingerprint};

#[derive(Clone, Copy)]
pub(crate) struct SourceGeneration {
    pub(crate) number: i64,
    pub(crate) sha256: [u8; 32],
}

pub(crate) trait AssociationSource: Send + Sync {
    fn generation(&self) -> SourceGeneration;
    fn fingerprint(
        &self,
        logical_key: &str,
        relative_path: &Path,
    ) -> Result<Option<MediaAggregateFingerprint>, FingerprintError>;
}

pub(super) async fn run(
    service: &MediaService,
    actor: Uuid,
    request: &MediaDiscoveryPreviewRequest,
    trigger: MediaAssociationRunTrigger,
) -> Result<(Uuid, MediaDiscoveryRunResponse), MediaServiceError> {
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
    let generation = source.generation();
    let previews = super::associations::preview_scope(
        &row.root_relative_path,
        row.effective_dry_run,
        &request.source_paths,
    );
    let mut queued_jobs = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = BTreeSet::new();
    for preview in previews {
        if !preview.accepted || !seen.insert(preview.source_path.clone()) {
            skipped.push(MediaDiscoverySkippedItemResponse {
                source_path: preview.source_path,
                reason: Some(
                    preview
                        .reason
                        .unwrap_or_else(|| "media_discovery_source_path_duplicate".into()),
                ),
            });
            continue;
        }
        let fingerprint = fingerprint(source, &row.source_root_key, &preview.source_path).await?;
        let Some(fingerprint) = fingerprint else {
            skipped.push(MediaDiscoverySkippedItemResponse {
                source_path: preview.source_path,
                reason: Some("media_discovery_source_unstable".into()),
            });
            continue;
        };
        let admitted = enqueue_association_job(
            service.store.pool(),
            &AssociationJobInput {
                actor_public_id: actor,
                association_public_id: request.media_discovery_association_public_id,
                association_version: row.latest_version,
                relative_path: &preview.source_path,
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
        .map_err(|error| map_data_error(&error))?;
        match admitted {
            Some(job) => queued_jobs.push(MediaDiscoveryQueuedJobResponse {
                media_job_public_id: job.media_job_public_id,
                output_path: preview.source_path.clone(),
                source_path: preview.source_path,
                dry_run: job.dry_run,
            }),
            None => skipped.push(MediaDiscoverySkippedItemResponse {
                source_path: preview.source_path,
                reason: Some("media_discovery_source_unchanged".into()),
            }),
        }
    }
    Ok((
        row.media_profile_public_id,
        MediaDiscoveryRunResponse {
            queued_jobs,
            skipped,
        },
    ))
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
) -> Result<Option<MediaAggregateFingerprint>, MediaServiceError> {
    let retained = Arc::clone(source);
    let key = key.to_owned();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || retained.fingerprint(&key, Path::new(&path)))
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "media association fingerprint task failed");
            MediaServiceError::new(MediaServiceErrorKind::Storage)
        })?
        .map_err(|error| {
            tracing::error!(error = %error, "media association fingerprint failed");
            MediaServiceError::new(MediaServiceErrorKind::Unavailable)
                .with_code("media_discovery_source_unstable")
        })
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
