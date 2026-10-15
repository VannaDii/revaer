//! Single-process native association scanning with an in-memory ancestor cursor.

mod watches;

use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use revaer_api::{app::media::MediaAssociationRunTrigger, models::MediaDiscoveryPreviewRequest};
use revaer_data::media::associations::{AssociationRow, read_association, read_association_page};
use revaer_data::media::{
    rescan::{
        RescanFence, observe_rescan_paths, read_rescan_state, request_rescan, satisfy_rescan,
    },
    schedules::{observe_schedule_due, read_schedule_configuration},
};
use thiserror::Error;
use uuid::Uuid;

use super::MediaService;
use super::source::{AdmissionBudget, AdmissionControl, SourceGeneration};
use crate::media_discovery_scan::{ScanBatch, ScanBudget, ScanCursor, ScanError, ScanLimit};
use crate::media_discovery_watcher::{MediaWatchEventBuffer, MediaWatcher};
use watches::NativeWatches;

pub(crate) struct NativeDiscovery {
    service: Arc<MediaService>,
    page_cursor: Option<(String, Uuid)>,
    scan: Option<AssociationScan>,
    uncertain: Option<AssociationScan>,
    reconcile: bool,
    watches: NativeWatches,
}

struct AssociationScan {
    row: AssociationRow,
    cursor: Option<ScanCursor>,
    trigger: MediaAssociationRunTrigger,
    generation: SourceGeneration,
    captured_sequence: i64,
    pending: Option<ScanBatch>,
    aggregate_bytes: u64,
    metadata_elapsed: std::time::Duration,
}

impl AssociationScan {
    async fn next_batch(
        &mut self,
        source: Arc<dyn super::source::AssociationSource>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(ScanBatch, std::time::Duration), NativeDiscoveryError> {
        let key = self.row.source_root_key.clone();
        let prefix = self.row.root_relative_path.clone();
        let cursor = self.cursor.take();
        let scan_cancelled = Arc::clone(&cancelled);
        if let Some(pending) = self.pending.take() {
            Ok((pending, std::time::Duration::ZERO))
        } else {
            tokio::task::spawn_blocking(move || {
                let started = std::time::Instant::now();
                let batch = source.scan(
                    &key,
                    Path::new(&prefix),
                    &ScanBudget {
                        // Retained DISC-1 defaults; admission's smaller hard cap
                        // still applies if its request contract is narrowed.
                        files: 64.min(MediaDiscoveryPreviewRequest::MAX_SOURCE_PATHS),
                        entries: 256,
                        bytes: 1024 * 1024 * 1024 * 1024,
                        elapsed: std::time::Duration::from_secs(2),
                        depth: 32,
                        #[cfg(any(target_os = "linux", test))]
                        run: crate::media_discovery_scan::SCAN_RUN_LIMITS,
                    },
                    cursor,
                    &|| scan_cancelled.load(Ordering::Relaxed),
                )?;
                Ok::<_, ScanError>((batch, started.elapsed()))
            })
            .await
            .map_err(|error| NativeDiscoveryError::Join(error.to_string()))
            .and_then(|batch| batch.map_err(NativeDiscoveryError::from))
        }
    }

    const fn fence(&self) -> RescanFence<'_> {
        RescanFence {
            association: self.row.media_discovery_association_public_id,
            version: self.row.latest_version,
            generation: self.generation.number,
            generation_sha256: self.generation.sha256,
            trigger: self.trigger.as_str(),
        }
    }
}

impl NativeDiscovery {
    pub(crate) fn new(service: Arc<MediaService>) -> Self {
        Self {
            service,
            page_cursor: None,
            scan: None,
            uncertain: None,
            reconcile: true,
            watches: NativeWatches::default(),
        }
    }

    pub(crate) async fn tick(
        &mut self,
        cancelled: Arc<AtomicBool>,
        events: &MediaWatchEventBuffer,
        watcher: &mut dyn MediaWatcher,
    ) -> Result<(), NativeDiscoveryError> {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(());
        }
        let Some(source) = self.service.source.as_ref().map(Arc::clone) else {
            return Ok(());
        };
        self.flush_watch_events(events, watcher).await?;
        self.retry_uncertainty(source.generation()).await?;
        if self.scan.is_none() {
            self.scan = self.next_scan(source.generation(), watcher).await?;
        }
        let Some(mut scan) = self.scan.take() else {
            return Ok(());
        };
        let metadata_started = std::time::Instant::now();
        let current = read_association(
            self.service.store.pool(),
            scan.row.media_discovery_association_public_id,
        )
        .await?;
        if current.as_ref() != Some(&scan.row) {
            return Ok(());
        }
        let binding_elapsed = metadata_started.elapsed();
        let batch = scan.next_batch(source, cancelled.clone()).await;
        let (mut batch, traversal_elapsed) = match batch {
            Ok(batch) => batch,
            Err(error) => return Err(self.reset_uncertain_scan(&scan, error).await),
        };
        let traversal_elapsed = traversal_elapsed.saturating_add(binding_elapsed);
        scan.metadata_elapsed = scan.metadata_elapsed.saturating_add(traversal_elapsed);
        if scan.metadata_elapsed > std::time::Duration::from_hours(6) {
            return Err(self
                .reset_uncertain_scan(
                    &scan,
                    ScanError::Inventory(
                        crate::media_discovery_fingerprint::FingerprintError::ResourceLimit(
                            "scan metadata",
                        ),
                    )
                    .into(),
                )
                .await);
        }
        if cancelled.load(Ordering::Relaxed) || batch.limit == Some(ScanLimit::Cancelled) {
            return Ok(());
        }
        if batch.limit == Some(ScanLimit::Depth) {
            let error = ScanError::Inventory(
                crate::media_discovery_fingerprint::FingerprintError::ResourceLimit("scan depth"),
            )
            .into();
            return Err(self.reset_uncertain_scan(&scan, error).await);
        }
        if let Err(error) = self
            .admit_batch(
                &mut scan,
                &mut batch,
                Arc::clone(&cancelled),
                traversal_elapsed,
            )
            .await
        {
            if cancelled.load(Ordering::Relaxed) {
                return Ok(());
            }
            return Err(self.reset_uncertain_scan(&scan, error).await);
        }
        if !batch.paths.is_empty() {
            scan.pending = Some(batch);
            self.scan = Some(scan);
        } else if let Some(cursor) = batch.cursor {
            scan.cursor = Some(cursor);
            self.scan = Some(scan);
        } else {
            self.finish_scan(&scan, events, watcher).await?;
        }
        Ok(())
    }

    fn remember_uncertainty(&mut self, scan: &AssociationScan) {
        self.uncertain = Some(AssociationScan {
            row: scan.row.clone(),
            cursor: None,
            trigger: scan.trigger,
            generation: scan.generation,
            captured_sequence: scan.captured_sequence,
            pending: None,
            aggregate_bytes: 0,
            metadata_elapsed: std::time::Duration::ZERO,
        });
    }

    async fn retry_uncertainty(
        &mut self,
        generation: SourceGeneration,
    ) -> Result<(), NativeDiscoveryError> {
        let Some(scan) = self.uncertain.as_ref() else {
            return Ok(());
        };
        let current = read_association(
            self.service.store.pool(),
            scan.row.media_discovery_association_public_id,
        )
        .await?;
        if current.as_ref() == Some(&scan.row)
            && generation.number == scan.generation.number
            && generation.sha256 == scan.generation.sha256
        {
            request_rescan(
                self.service.store.pool(),
                &scan.fence(),
                "directory_changed",
            )
            .await?;
        }
        self.uncertain = None;
        Ok(())
    }

    async fn finish_scan(
        &mut self,
        scan: &AssociationScan,
        events: &MediaWatchEventBuffer,
        watcher: &mut dyn MediaWatcher,
    ) -> Result<(), NativeDiscoveryError> {
        // Process notifications received during traversal/admission before
        // deciding that this was a clean census.
        self.flush_watch_events(events, watcher).await?;
        if self
            .watches
            .has_pending(scan.row.media_discovery_association_public_id)
        {
            self.remember_uncertainty(scan);
            request_rescan(
                self.service.store.pool(),
                &scan.fence(),
                "directory_changed",
            )
            .await?;
            self.uncertain = None;
        } else {
            satisfy_rescan(
                self.service.store.pool(),
                &scan.fence(),
                scan.captured_sequence,
            )
            .await?;
        }
        Ok(())
    }

    async fn reset_uncertain_scan(
        &mut self,
        scan: &AssociationScan,
        error: NativeDiscoveryError,
    ) -> NativeDiscoveryError {
        self.remember_uncertainty(scan);
        match request_rescan(
            self.service.store.pool(),
            &scan.fence(),
            "directory_changed",
        )
        .await
        {
            Ok(_) => {
                self.uncertain = None;
                error
            }
            Err(reset_error) => NativeDiscoveryError::WatchRegistration(format!(
                "{error}; cannot preserve scan uncertainty: {reset_error}"
            )),
        }
    }

    async fn admit_batch(
        &self,
        scan: &mut AssociationScan,
        batch: &mut ScanBatch,
        cancelled: Arc<AtomicBool>,
        traversal_elapsed: std::time::Duration,
    ) -> Result<(), NativeDiscoveryError> {
        if !batch.paths.is_empty() {
            let (_, admission) = Box::pin(super::source::run(
                &self.service,
                Uuid::from_u128(0),
                &MediaDiscoveryPreviewRequest {
                    media_discovery_association_public_id: scan
                        .row
                        .media_discovery_association_public_id,
                    source_paths: std::mem::take(&mut batch.paths),
                },
                scan.trigger,
                AdmissionControl {
                    cancelled: Some(cancelled),
                    budget: Some(AdmissionBudget {
                        batch_bytes: 1040 * 1024 * 1024 * 1024,
                        remaining_run_bytes: 16_640 * 1024 * 1024 * 1024 - scan.aggregate_bytes,
                        batch_metadata: std::time::Duration::from_secs(2)
                            .saturating_sub(traversal_elapsed),
                        remaining_run_metadata: std::time::Duration::from_hours(6)
                            .saturating_sub(scan.metadata_elapsed),
                    }),
                },
            ))
            .await
            .map_err(|error| NativeDiscoveryError::Admission(error.to_string()))?;
            scan.aggregate_bytes += admission.aggregate_bytes;
            scan.metadata_elapsed = scan
                .metadata_elapsed
                .saturating_add(admission.metadata_elapsed);
            batch.paths = admission.pending;
            let observed_paths = admission
                .response
                .queued_jobs
                .iter()
                .map(|item| item.source_path.clone())
                .chain(
                    admission
                        .response
                        .skipped
                        .iter()
                        .map(|item| item.source_path.clone()),
                )
                .collect::<Vec<_>>();
            if admission
                .response
                .skipped
                .iter()
                .any(|item| item.reason.as_deref() != Some("media_discovery_source_unchanged"))
            {
                return Err(ScanError::from(
                    crate::media_discovery_fingerprint::FingerprintError::DirectoryChanged,
                )
                .into());
            }
            if !observed_paths.is_empty() {
                observe_rescan_paths(
                    self.service.store.pool(),
                    &scan.fence(),
                    scan.captured_sequence,
                    &observed_paths,
                )
                .await?;
            }
        }
        Ok(())
    }

    pub(crate) async fn flush_watch_events(
        &mut self,
        events: &MediaWatchEventBuffer,
        watcher: &mut dyn MediaWatcher,
    ) -> Result<(), NativeDiscoveryError> {
        let drained = events.drain()?;
        let Some(source) = self.service.source.as_ref() else {
            return Ok(());
        };
        self.watches
            .flush(&self.service.store, source.generation(), drained, watcher)
            .await
    }

    async fn next_scan(
        &mut self,
        generation: SourceGeneration,
        watcher: &mut dyn MediaWatcher,
    ) -> Result<Option<AssociationScan>, NativeDiscoveryError> {
        let key = self.page_cursor.as_ref().map(|(key, _)| key.as_str());
        let id = self.page_cursor.as_ref().map(|(_, id)| *id);
        let page = read_association_page(self.service.store.pool(), 1, key, id).await?;
        let Some(row) = page.into_iter().next() else {
            self.page_cursor = None;
            self.reconcile = false;
            self.watches.finish_page_cycle(watcher)?;
            return Ok(None);
        };
        let next_cursor = (
            row.association_key.clone(),
            row.media_discovery_association_public_id,
        );
        if let Some(source) = self.service.source.as_ref() {
            self.watches.observe(&row, source.as_ref(), watcher)?;
        }
        let scan = self.scan_for(row, generation).await?;
        self.page_cursor = Some(next_cursor);
        Ok(scan)
    }

    async fn scan_for(
        &self,
        row: AssociationRow,
        generation: SourceGeneration,
    ) -> Result<Option<AssociationScan>, NativeDiscoveryError> {
        if !row.binding_ready
            || row.active_version != Some(row.latest_version)
            || (!row.modes.watcher_enabled && !row.modes.schedule_enabled)
        {
            return Ok(None);
        }
        if row.modes.schedule_enabled {
            observe_schedule_due(
                self.service.store.pool(),
                &RescanFence {
                    association: row.media_discovery_association_public_id,
                    version: row.latest_version,
                    generation: generation.number,
                    generation_sha256: generation.sha256,
                    trigger: MediaAssociationRunTrigger::Schedule.as_str(),
                },
            )
            .await?;
        }
        let mut trigger = if row.modes.watcher_enabled {
            MediaAssociationRunTrigger::Watcher
        } else {
            if read_schedule_configuration(
                self.service.store.pool(),
                row.media_discovery_association_public_id,
            )
            .await?
            .is_none()
            {
                return Ok(None);
            }
            MediaAssociationRunTrigger::Schedule
        };
        let fence = RescanFence {
            association: row.media_discovery_association_public_id,
            version: row.latest_version,
            generation: generation.number,
            generation_sha256: generation.sha256,
            trigger: trigger.as_str(),
        };
        if self.reconcile {
            request_rescan(self.service.store.pool(), &fence, "restart_reconcile").await?;
        }
        let requests = read_rescan_state(
            self.service.store.pool(),
            row.media_discovery_association_public_id,
        )
        .await?;
        if row.modes.schedule_enabled
            && requests.iter().any(|request| {
                request.reason_code == "schedule"
                    && request.last_requested_sequence > request.satisfied_sequence
            })
        {
            trigger = MediaAssociationRunTrigger::Schedule;
        }
        let Some(request) = requests.into_iter().next() else {
            return Ok(None);
        };
        if request.association_version != row.latest_version {
            return Ok(None);
        }
        if request.satisfied_sequence >= request.requested_sequence {
            return Ok(None);
        }
        Ok(Some(AssociationScan {
            row,
            cursor: None,
            trigger,
            generation,
            captured_sequence: request.requested_sequence,
            pending: None,
            aggregate_bytes: 0,
            metadata_elapsed: std::time::Duration::ZERO,
        }))
    }
}

#[derive(Debug, Error)]
pub(crate) enum NativeDiscoveryError {
    #[error(transparent)]
    Storage(#[from] revaer_data::DataError),
    #[error(transparent)]
    Scan(#[from] ScanError),
    #[error("native discovery task failed: {0}")]
    Join(String),
    #[error(transparent)]
    Watcher(#[from] crate::media_discovery_watcher::MediaWatcherError),
    #[error("native watcher registration or rescan failed: {0}")]
    WatchRegistration(String),
    #[error("native discovery admission failed: {0}")]
    Admission(String),
}
