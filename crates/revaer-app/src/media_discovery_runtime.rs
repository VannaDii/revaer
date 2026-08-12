//! In-process media discovery runtime.
//!
//! # Design
//! - Scans scheduled media profile source roots on a bounded interval.
//! - Consumes recursive native filesystem events for watcher-enabled profiles.
//! - Reuses discovery preview path derivation before queueing jobs.
//! - Atomically persists stable source fingerprints with job creation for restart-safe de-duplication.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use revaer_api::app::media::{MediaDiscoveryPreviewResponse, MediaServiceError};
use revaer_data::DataError;
use revaer_data::media::jobs::EnqueueDiscoveredMediaJobInput;
use revaer_data::media::profiles::MediaProfileRow;
use revaer_runtime::media::MediaStore;
use revaer_telemetry::Metrics;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tracing::warn;
use uuid::Uuid;

use crate::media::{
    build_discovery_previews, ensure_execution_capability_snapshot,
    ensure_profile_compatibility_target_readiness, ensure_profile_desired_target_readiness,
    load_media_desired_targets,
};
use crate::media_discovery_fingerprint::{
    fingerprint_media_aggregate, owner_for_changed_path, revalidate_media_aggregate,
};
use crate::media_discovery_scan::{ScanBudget, ScanCursor, ScanError, scan_media_source_paths};
use crate::media_discovery_watcher::{
    MediaWatchEvent, MediaWatchEventBuffer, MediaWatcher, MediaWatcherError, NotifyMediaWatcher,
};
use crate::runtime_shutdown::{self, RuntimeShutdownReceiver};

const DEFAULT_DISCOVERY_TICK_INTERVAL: Duration = Duration::from_mins(1);
const WATCH_DEBOUNCE_INTERVAL: Duration = Duration::from_secs(1);
const WATCH_DEBOUNCE_TICK_INTERVAL: Duration = Duration::from_millis(100);
const WATCH_EVENT_CAPACITY: usize = 1_024;
const SYSTEM_USER_PUBLIC_ID: Uuid = Uuid::from_u128(0);
const MEDIA_FILE_EXTENSIONS: &[&str] = &[
    "3g2", "3gp", "aac", "ac3", "asf", "avi", "divx", "eac3", "flac", "flv", "m2ts", "m2v", "m4a",
    "m4v", "mka", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "mts", "ogg", "ogm", "ogv", "opus",
    "ts", "vob", "wav", "webm", "wma", "wmv",
];

/// Background runtime that discovers media files for enabled profiles.
pub(crate) struct MediaDiscoveryRuntime {
    store: MediaStore,
    telemetry: Metrics,
    tick_interval: Duration,
    last_scheduled: Mutex<BTreeMap<Uuid, Instant>>,
    watcher: Box<dyn MediaWatcher>,
    watch_events: Arc<MediaWatchEventBuffer>,
    watcher_profiles: BTreeMap<Uuid, MediaProfileRow>,
    pending_watch_events: BTreeMap<(Uuid, PathBuf), Instant>,
    pending_watch_rescans: BTreeMap<Uuid, Instant>,
    scan_cursors: BTreeMap<Uuid, ScanCursor>,
    cancelled: Arc<AtomicBool>,
}

impl MediaDiscoveryRuntime {
    /// Construct a production media discovery runtime.
    #[must_use]
    pub(crate) fn new(store: MediaStore, telemetry: Metrics) -> Self {
        Self::with_tick_interval(store, telemetry, DEFAULT_DISCOVERY_TICK_INTERVAL)
    }

    fn with_tick_interval(store: MediaStore, telemetry: Metrics, tick_interval: Duration) -> Self {
        let watch_events = Arc::new(MediaWatchEventBuffer::new(WATCH_EVENT_CAPACITY));
        Self {
            store,
            telemetry,
            tick_interval,
            last_scheduled: Mutex::new(BTreeMap::new()),
            watcher: Box::new(NotifyMediaWatcher::new(watch_events.clone())),
            watch_events,
            watcher_profiles: BTreeMap::new(),
            pending_watch_events: BTreeMap::new(),
            pending_watch_rescans: BTreeMap::new(),
            scan_cursors: BTreeMap::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawn the discovery loop.
    pub(crate) fn spawn(self, shutdown: RuntimeShutdownReceiver) -> JoinHandle<()> {
        let task_cancelled = self.cancelled.clone();
        tokio::spawn(async move {
            let _guard = CancelOnDrop(task_cancelled);
            self.run_loop(shutdown).await;
        })
    }

    async fn run_loop(mut self, mut shutdown: RuntimeShutdownReceiver) {
        if runtime_shutdown::requested(&shutdown) {
            return;
        }
        let mut ticker = interval(self.tick_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut debounce_ticker = interval(WATCH_DEBOUNCE_TICK_INTERVAL);
        debounce_ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(error) = self.run_tick().await {
                        warn!(error = %error, "media discovery runtime tick failed");
                    }
                }
                _ = debounce_ticker.tick() => {
                    if let Err(error) = self.flush_watch_events().await {
                        warn!(error = %error, "media watcher debounce flush failed");
                    }
                }
                () = runtime_shutdown::changed(&mut shutdown) => {
                    return;
                }
            }
        }
    }

    async fn run_tick(&mut self) -> Result<(), MediaDiscoveryRuntimeError> {
        let profiles = self.store.list_profiles().await?;
        for error in self.watcher.synchronize(&profiles) {
            warn!(error = %error, "media watcher synchronization failed");
            self.telemetry
                .inc_media_discovery_candidate("watcher", "setup_failed");
        }
        self.watcher_profiles = profiles
            .iter()
            .filter(|profile| profile.watcher_enabled)
            .cloned()
            .map(|profile| (profile.media_profile_public_id, profile))
            .collect();
        let now = Instant::now();
        for profile in profiles {
            if self.profile_due(&profile, now)? {
                self.discover_profile(profile, "schedule").await?;
            }
        }
        Ok(())
    }

    fn profile_due(
        &self,
        profile: &MediaProfileRow,
        now: Instant,
    ) -> Result<bool, MediaDiscoveryRuntimeError> {
        if !profile.schedule_enabled {
            return Ok(false);
        }

        let Some(interval_minutes) = profile.schedule_interval_minutes else {
            return Err(MediaDiscoveryRuntimeError::InvalidInterval);
        };
        if interval_minutes <= 0 {
            return Err(MediaDiscoveryRuntimeError::InvalidInterval);
        }
        let interval_seconds = u64::try_from(interval_minutes)
            .map_err(|_| MediaDiscoveryRuntimeError::InvalidInterval)?
            * 60;
        let schedule_interval = Duration::from_secs(interval_seconds);
        let mut last_scheduled = self
            .last_scheduled
            .lock()
            .map_err(|error| MediaDiscoveryRuntimeError::Lock(error.to_string()))?;
        if last_scheduled
            .get(&profile.media_profile_public_id)
            .is_some_and(|last| now.duration_since(*last) < schedule_interval)
        {
            return Ok(false);
        }
        last_scheduled.insert(profile.media_profile_public_id, now);
        drop(last_scheduled);
        Ok(true)
    }

    async fn discover_profile(
        &mut self,
        profile: MediaProfileRow,
        origin: &'static str,
    ) -> Result<(), MediaDiscoveryRuntimeError> {
        if !self.profile_ready_for_execution(&profile, origin).await? {
            return Ok(());
        }

        let profile_id = profile.media_profile_public_id;
        let root = PathBuf::from(&profile.source_root);
        let cursor = self.scan_cursors.remove(&profile_id);
        let cancelled = self.cancelled.clone();
        let batch = tokio::task::spawn_blocking(move || {
            scan_media_source_paths(&root, &ScanBudget::default(), cursor, || {
                cancelled.load(Ordering::Relaxed)
            })
        })
        .await
        .map_err(|error| MediaDiscoveryRuntimeError::Join(error.to_string()))??;
        if let Some(limit) = batch.limit {
            self.telemetry
                .inc_media_discovery_candidate(origin, scan_limit_label(limit));
        }
        if let Some(cursor) = batch.cursor {
            self.scan_cursors.insert(profile_id, cursor);
        }
        self.queue_source_paths(&profile, batch.paths, origin).await
    }

    async fn profile_ready_for_execution(
        &self,
        profile: &MediaProfileRow,
        origin: &'static str,
    ) -> Result<bool, MediaDiscoveryRuntimeError> {
        if profile.dry_run_only {
            return Ok(true);
        }

        let latest = self.store.latest_capability().await?;
        let Some(snapshot) = latest.as_ref() else {
            self.record_not_ready(profile, origin, "media_capability_snapshot_missing");
            return Ok(false);
        };
        if let Err(error) = ensure_execution_capability_snapshot(Some(snapshot)) {
            self.record_not_ready(profile, origin, error.code().unwrap_or("media_not_ready"));
            return Ok(false);
        }
        if profile
            .compatibility_target_key
            .as_deref()
            .is_some_and(|target_key| !target_key.trim().is_empty())
        {
            let targets = self.store.list_compatibility_targets().await?;
            if let Err(error) =
                ensure_profile_compatibility_target_readiness(profile, snapshot, &targets)
            {
                self.record_not_ready(profile, origin, error.code().unwrap_or("media_not_ready"));
                return Ok(false);
            }
        }
        if profile
            .desired_target_key
            .as_deref()
            .is_some_and(|target_key| !target_key.trim().is_empty())
        {
            let desired_targets = load_media_desired_targets(&self.store).await?;
            let policies = self.store.list_policy_profiles().await?;
            if let Err(error) = ensure_profile_desired_target_readiness(
                profile,
                snapshot,
                &desired_targets,
                &policies,
            ) {
                self.record_not_ready(profile, origin, error.code().unwrap_or("media_not_ready"));
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn record_not_ready(&self, profile: &MediaProfileRow, origin: &'static str, reason: &str) {
        warn!(
            media_profile_public_id = %profile.media_profile_public_id,
            reason,
            "media discovery skipped profile without ready execution capability"
        );
        self.telemetry
            .inc_media_discovery_candidate(origin, "capability_not_ready");
    }

    async fn queue_source_paths(
        &self,
        profile: &MediaProfileRow,
        source_paths: Vec<String>,
        origin: &'static str,
    ) -> Result<(), MediaDiscoveryRuntimeError> {
        let previews = build_discovery_previews(
            &source_paths,
            &profile.source_root,
            &profile.output_root,
            profile.dry_run_only,
        );
        self.queue_previews(profile, previews, origin).await
    }

    async fn queue_previews(
        &self,
        profile: &MediaProfileRow,
        previews: Vec<MediaDiscoveryPreviewResponse>,
        origin: &'static str,
    ) -> Result<(), MediaDiscoveryRuntimeError> {
        let mut accepted = Vec::new();
        for preview in previews {
            if !preview.accepted {
                self.telemetry
                    .inc_media_discovery_candidate(origin, "skipped");
                continue;
            }
            let Some(output_path) = preview.output_path.clone() else {
                self.telemetry
                    .inc_media_discovery_candidate(origin, "skipped");
                continue;
            };
            accepted.push((preview, output_path));
        }
        let (unique, duplicates, unstable) = canonicalize_candidates(accepted);
        for _ in 0..duplicates {
            self.telemetry
                .inc_media_discovery_candidate(origin, "deduplicated");
        }
        for _ in 0..unstable {
            self.telemetry
                .inc_media_discovery_candidate(origin, "unstable");
        }
        for (_, (preview, output_path)) in unique {
            let source_path = PathBuf::from(&preview.source_path);
            let source_root = PathBuf::from(&profile.source_root);
            let fingerprint = tokio::task::spawn_blocking(move || {
                fingerprint_media_aggregate(&source_path, &source_root)
            })
            .await
            .map_err(|error| MediaDiscoveryRuntimeError::Join(error.to_string()))??;
            let Some(fingerprint) = fingerprint else {
                self.telemetry
                    .inc_media_discovery_candidate(origin, "unstable");
                continue;
            };
            let validation_path = PathBuf::from(&preview.source_path);
            let validation_root = PathBuf::from(&profile.source_root);
            let expected_fingerprint = fingerprint.clone();
            let valid = tokio::task::spawn_blocking(move || {
                revalidate_media_aggregate(
                    &validation_path,
                    &validation_root,
                    &expected_fingerprint,
                )
            })
            .await
            .map_err(|error| MediaDiscoveryRuntimeError::Join(error.to_string()))??;
            if !valid {
                self.telemetry
                    .inc_media_discovery_candidate(origin, "unstable");
                continue;
            }
            let create_result = self
                .store
                .enqueue_discovered_job(&EnqueueDiscoveredMediaJobInput {
                    actor_public_id: SYSTEM_USER_PUBLIC_ID,
                    media_profile_public_id: profile.media_profile_public_id,
                    source_path: &preview.source_path,
                    output_path: Some(output_path.as_str()),
                    source_identity: &fingerprint.identity,
                    source_size_bytes: fingerprint.size_bytes,
                    source_modified_ns: fingerprint.modified_ns,
                    source_changed_ns: fingerprint.changed_ns,
                    source_sha256: &fingerprint.sha256,
                })
                .await;
            match create_result {
                Ok(Some(_)) => {
                    self.telemetry
                        .inc_media_discovery_candidate(origin, "queued");
                    self.telemetry
                        .inc_media_job_queued(origin, profile.dry_run_only);
                }
                Ok(None) => {
                    self.telemetry
                        .inc_media_discovery_candidate(origin, "deduplicated");
                }
                Err(error) => {
                    self.telemetry
                        .inc_media_discovery_candidate(origin, "queue_failed");
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }

    fn record_watch_event(&mut self, event: &MediaWatchEvent) {
        match event {
            MediaWatchEvent::Path {
                media_profile_public_id,
                path,
            } => self.record_watch_path(*media_profile_public_id, path),
            MediaWatchEvent::Rescan {
                media_profile_public_id,
            } => self.record_watch_rescan(*media_profile_public_id),
        }
    }

    fn record_watch_path(&mut self, profile_id: Uuid, event_path: &Path) {
        if self.pending_watch_rescans.contains_key(&profile_id) {
            return;
        }
        let Some(profile) = self.watcher_profiles.get(&profile_id) else {
            return;
        };
        let Some(profile_path) = rebase_watch_event_path(event_path, &profile.source_root) else {
            return;
        };
        if profile_path.is_dir() {
            self.record_watch_rescan(profile_id);
            return;
        }
        let Some(owner) = owner_for_changed_path(&profile_path, Path::new(&profile.source_root))
        else {
            return;
        };
        let key = (profile_id, owner);
        if self.pending_watch_events.len() >= WATCH_EVENT_CAPACITY
            && !self.pending_watch_events.contains_key(&key)
        {
            self.record_watch_rescan(profile_id);
            return;
        }
        self.pending_watch_events
            .insert(key, Instant::now() + WATCH_DEBOUNCE_INTERVAL);
    }

    fn record_watch_rescan(&mut self, profile_id: Uuid) {
        if !self.watcher_profiles.contains_key(&profile_id) {
            return;
        }
        self.pending_watch_events
            .retain(|(pending_profile_id, _), _| *pending_profile_id != profile_id);
        self.pending_watch_rescans
            .entry(profile_id)
            .or_insert_with(|| Instant::now() + WATCH_DEBOUNCE_INTERVAL);
    }

    async fn flush_watch_events(&mut self) -> Result<(), MediaDiscoveryRuntimeError> {
        let drained = self.watch_events.drain()?;
        if drained.overflowed {
            self.telemetry
                .inc_media_discovery_candidate("watcher", "overflow");
            let profile_ids = self.watcher_profiles.keys().copied().collect::<Vec<_>>();
            for profile_id in profile_ids {
                self.record_watch_rescan(profile_id);
            }
        }
        for event in drained.events {
            self.record_watch_event(&event);
        }
        let now = Instant::now();
        let due_rescans = self
            .pending_watch_rescans
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(profile_id, _)| *profile_id)
            .collect::<Vec<_>>();
        for profile_id in due_rescans {
            self.pending_watch_rescans.remove(&profile_id);
            if let Some(profile) = self.watcher_profiles.get(&profile_id).cloned() {
                self.discover_profile(profile, "watcher").await?;
            }
        }
        let due = self
            .pending_watch_events
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for (profile_id, path) in due {
            self.pending_watch_events
                .remove(&(profile_id, path.clone()));
            let Some(profile) = self.watcher_profiles.get(&profile_id).cloned() else {
                continue;
            };
            let Some(path_text) = path.to_str().map(str::to_string) else {
                self.telemetry
                    .inc_media_discovery_candidate("watcher", "skipped");
                continue;
            };
            if !self
                .profile_ready_for_execution(&profile, "watcher")
                .await?
            {
                continue;
            }
            self.queue_source_paths(&profile, vec![path_text], "watcher")
                .await?;
        }
        Ok(())
    }
}

fn canonicalize_candidates(
    candidates: Vec<(MediaDiscoveryPreviewResponse, String)>,
) -> (
    BTreeMap<PathBuf, (MediaDiscoveryPreviewResponse, String)>,
    usize,
    usize,
) {
    let mut unique = BTreeMap::new();
    let mut duplicates = 0;
    let mut unstable = 0;
    for candidate in candidates {
        match Path::new(&candidate.0.source_path).canonicalize() {
            Ok(path) => {
                if unique.insert(path, candidate).is_some() {
                    duplicates += 1;
                }
            }
            Err(_) => unstable += 1,
        }
    }
    (unique, duplicates, unstable)
}

struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn rebase_watch_event_path(event_path: &Path, source_root: &str) -> Option<PathBuf> {
    let configured_root = Path::new(source_root);
    let canonical_root = configured_root.canonicalize().ok()?;
    let canonical_event = event_path.canonicalize().ok().or_else(|| {
        let parent = event_path.parent()?.canonicalize().ok()?;
        Some(parent.join(event_path.file_name()?))
    })?;
    let relative = canonical_event.strip_prefix(canonical_root).ok()?;
    Some(configured_root.join(relative))
}

const fn scan_limit_label(limit: crate::media_discovery_scan::ScanLimit) -> &'static str {
    use crate::media_discovery_scan::ScanLimit;
    match limit {
        ScanLimit::Entries => "entry_budget",
        ScanLimit::Files => "file_budget",
        ScanLimit::Bytes => "byte_budget",
        ScanLimit::Depth => "depth_budget",
        ScanLimit::Elapsed => "time_budget",
        ScanLimit::Cancelled => "cancelled",
    }
}

#[derive(Debug, Error)]
enum MediaDiscoveryRuntimeError {
    #[error("media discovery runtime data error: {0}")]
    Data(#[from] DataError),
    #[error("media discovery runtime media service error: {0}")]
    MediaService(#[from] MediaServiceError),
    #[error(transparent)]
    Scan(#[from] ScanError),
    #[error(transparent)]
    Fingerprint(#[from] crate::media_discovery_fingerprint::FingerprintError),
    #[error(transparent)]
    Watcher(#[from] MediaWatcherError),
    #[error("media discovery runtime lock failed: {0}")]
    Lock(String),
    #[error("media discovery runtime task join failed: {0}")]
    Join(String),
    #[error("media discovery runtime schedule interval invalid")]
    InvalidInterval,
}

pub(crate) fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            MEDIA_FILE_EXTENSIONS
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

#[cfg(test)]
mod tests {
    use super::{MediaDiscoveryRuntime, canonicalize_candidates, is_media_file};
    use crate::media_discovery_watcher::MediaWatchEvent;
    use crate::runtime_shutdown;
    use chrono::Utc;
    use revaer_api::app::media::MediaDiscoveryPreviewResponse;
    use revaer_data::DataError;
    use revaer_data::media::capabilities::{
        RecordCapabilityEncoderInput, RecordCapabilityFeatureInput, RecordCapabilitySnapshotInput,
        complete_capability_snapshot_run_with_executor, record_capability_encoder,
        record_capability_feature, record_capability_snapshot,
        start_capability_snapshot_run_with_executor,
    };
    use revaer_data::media::configuration::{
        AppendMediaDesiredTargetStreamInput, CreateMediaDesiredTargetInput,
        UpsertMediaCompatibilityTargetInput, append_media_desired_target_stream,
        create_media_desired_target, set_media_profile_desired_target,
        upsert_media_compatibility_target,
    };
    use revaer_data::media::jobs::list_media_jobs;
    use revaer_data::media::profiles::{
        CreateVerifiedMediaProfileInput, MediaProfileRow, UpdateMediaProfileInput,
        create_verified_media_profile, update_media_profile,
    };
    use revaer_data::media::{MediaRootIdentityResolver, StdMediaRootIdentityResolver};
    use revaer_runtime::media::MediaStore;
    use revaer_telemetry::Metrics;
    use revaer_test_support::postgres::start_postgres;
    use std::cell::Cell;
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, Instant};
    use tokio::time::{sleep, timeout};
    use uuid::Uuid;

    const WATCHER_UNAVAILABLE_DESIRED_TARGET_KEY: &str = "watcher-unavailable-desired-target";

    fn closed_media_store() -> MediaStore {
        let options = sqlx::postgres::PgConnectOptions::new()
            .host("127.0.0.1")
            .port(9)
            .username("revaer")
            .database("revaer");
        MediaStore::new(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy_with(options),
        )
    }

    fn media_profile(
        watcher_enabled: bool,
        schedule_enabled: bool,
        schedule_interval_minutes: Option<i32>,
    ) -> MediaProfileRow {
        MediaProfileRow {
            media_profile_public_id: Uuid::new_v4(),
            profile_key: "movies-main".to_string(),
            source_root: "/input/movies".to_string(),
            output_root: "/output/movies".to_string(),
            dry_run_only: true,
            retention_days: 30,
            compatibility_target_key: None,
            desired_target_key: None,
            desired_target_version: None,
            policy_key: "safe_dry_run".to_string(),
            watcher_enabled,
            schedule_enabled,
            schedule_interval_minutes,
            updated_at: Utc::now(),
        }
    }

    async fn create_watcher_profile(
        store: &MediaStore,
        profile_key: &str,
        source_root: &Path,
        output_root: &Path,
        compatibility_target_key: Option<&str>,
    ) -> anyhow::Result<Uuid> {
        let resolver = StdMediaRootIdentityResolver;
        let source = resolver.resolve(source_root)?;
        let output = resolver.resolve(output_root)?;
        Ok(create_verified_media_profile(
            store.pool(),
            &CreateVerifiedMediaProfileInput {
                actor_public_id: super::SYSTEM_USER_PUBLIC_ID,
                profile_key,
                source_requested_path: path_text(source.requested_path())?,
                source_canonical_path: path_text(source.canonical_path())?,
                source_filesystem_device: i64::try_from(source.filesystem_device())?,
                source_filesystem_inode: i64::try_from(source.filesystem_inode())?,
                output_requested_path: path_text(output.requested_path())?,
                output_canonical_path: path_text(output.canonical_path())?,
                output_filesystem_device: i64::try_from(output.filesystem_device())?,
                output_filesystem_inode: i64::try_from(output.filesystem_inode())?,
                retention_days: 30,
                compatibility_target_key,
                policy_key: "safe_dry_run",
                watcher_enabled: true,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?)
    }

    fn path_text(path: &Path) -> anyhow::Result<&str> {
        path.to_str()
            .ok_or_else(|| anyhow::anyhow!("media root is not Unicode: {}", path.display()))
    }

    #[test]
    fn is_media_file_accepts_expected_extensions() {
        assert!(is_media_file(Path::new("/media/movie.mkv")));
        assert!(is_media_file(Path::new("/media/movie.MP4")));
        assert!(is_media_file(Path::new("/media/movie.webm")));
        assert!(is_media_file(Path::new("/media/movie.ts")));
        assert!(is_media_file(Path::new("/media/movie.m4a")));
        assert!(!is_media_file(Path::new("/media/movie.srt")));
    }

    #[tokio::test]
    async fn directory_watch_event_requests_a_bounded_profile_rescan() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let source_root = temp.path().join("source");
        let nested = source_root.join("nested");
        fs::create_dir_all(&nested)?;
        let mut profile = media_profile(true, false, None);
        profile.source_root = source_root.to_string_lossy().into_owned();
        let profile_id = profile.media_profile_public_id;
        let mut runtime = MediaDiscoveryRuntime::with_tick_interval(
            closed_media_store(),
            Metrics::new()?,
            Duration::from_secs(1),
        );
        runtime.watcher_profiles.insert(profile_id, profile);

        runtime.record_watch_event(&MediaWatchEvent::Path {
            media_profile_public_id: profile_id,
            path: nested.canonicalize()?,
        });

        assert!(runtime.pending_watch_events.is_empty());
        assert!(runtime.pending_watch_rescans.contains_key(&profile_id));
        Ok(())
    }

    #[tokio::test]
    async fn uncertain_events_coalesce_without_postponing_the_rescan() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let source_root = temp.path().join("source");
        fs::create_dir_all(&source_root)?;
        let media_path = source_root.join("movie.mkv");
        fs::write(&media_path, b"media")?;
        let mut profile = media_profile(true, false, None);
        profile.source_root = source_root.to_string_lossy().into_owned();
        let profile_id = profile.media_profile_public_id;
        let mut runtime = MediaDiscoveryRuntime::with_tick_interval(
            closed_media_store(),
            Metrics::new()?,
            Duration::from_secs(1),
        );
        runtime.watcher_profiles.insert(profile_id, profile);
        runtime.record_watch_event(&MediaWatchEvent::Path {
            media_profile_public_id: profile_id,
            path: media_path.canonicalize()?,
        });
        assert_eq!(runtime.pending_watch_events.len(), 1);

        let rescan = MediaWatchEvent::Rescan {
            media_profile_public_id: profile_id,
        };
        runtime.record_watch_event(&rescan);
        let first_deadline = runtime.pending_watch_rescans.get(&profile_id).copied();
        runtime.record_watch_event(&rescan);
        runtime.record_watch_event(&MediaWatchEvent::Path {
            media_profile_public_id: profile_id,
            path: media_path.canonicalize()?,
        });

        assert!(runtime.pending_watch_events.is_empty());
        assert_eq!(runtime.pending_watch_rescans.len(), 1);
        assert_eq!(
            runtime.pending_watch_rescans.get(&profile_id).copied(),
            first_deadline
        );
        Ok(())
    }

    #[test]
    fn every_trigger_deduplicates_aliases_before_instrumented_fingerprinting() -> anyhow::Result<()>
    {
        let temp = tempfile::tempdir()?;
        let media = temp.path().join("movie.mkv");
        fs::write(&media, b"media")?;
        for _origin in ["manual", "schedule", "watcher"] {
            let previews = (0..1_024)
                .map(|index| {
                    let source_path = if index % 2 == 0 {
                        media.clone()
                    } else {
                        temp.path().join(".").join("movie.mkv")
                    };
                    (
                        MediaDiscoveryPreviewResponse {
                            source_path: source_path.to_string_lossy().into_owned(),
                            output_path: Some("/output/movie.mkv".to_string()),
                            dry_run: true,
                            accepted: true,
                            reason: None,
                        },
                        "/output/movie.mkv".to_string(),
                    )
                })
                .collect();
            let (unique, duplicates, unstable) = canonicalize_candidates(previews);
            let fingerprint_reads = Cell::new(0);
            for path in unique.keys() {
                assert_eq!(path, &media.canonicalize()?);
                fingerprint_reads.set(fingerprint_reads.get() + 1);
            }
            assert_eq!(
                (fingerprint_reads.get(), duplicates, unstable),
                (1, 1_023, 0)
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn profile_due_ignores_watchers_and_skips_disabled_schedules() -> anyhow::Result<()> {
        let runtime = MediaDiscoveryRuntime::with_tick_interval(
            closed_media_store(),
            Metrics::new()?,
            Duration::from_secs(1),
        );
        let now = Instant::now();
        let watcher = media_profile(true, false, None);
        let disabled = media_profile(false, false, None);

        assert!(!runtime.profile_due(&watcher, now)?);
        assert!(!runtime.profile_due(&disabled, now)?);
        Ok(())
    }

    #[tokio::test]
    async fn profile_due_throttles_schedule_profiles_by_interval() -> anyhow::Result<()> {
        let runtime = MediaDiscoveryRuntime::with_tick_interval(
            closed_media_store(),
            Metrics::new()?,
            Duration::from_secs(1),
        );
        let now = Instant::now();
        let scheduled = media_profile(false, true, Some(2));

        assert!(runtime.profile_due(&scheduled, now)?);
        assert!(!runtime.profile_due(&scheduled, now + Duration::from_secs(119))?);
        assert!(runtime.profile_due(&scheduled, now + Duration::from_mins(2))?);

        Ok(())
    }

    #[tokio::test]
    async fn profile_due_rejects_enabled_schedule_without_positive_interval() -> anyhow::Result<()>
    {
        let runtime = MediaDiscoveryRuntime::with_tick_interval(
            closed_media_store(),
            Metrics::new()?,
            Duration::from_secs(1),
        );
        let now = Instant::now();

        for interval in [None, Some(0), Some(-1)] {
            let scheduled = media_profile(false, true, interval);

            assert!(runtime.profile_due(&scheduled, now).is_err());
        }

        Ok(())
    }

    #[tokio::test]
    async fn watcher_only_profile_enqueues_each_durable_file_version_exactly_once()
    -> anyhow::Result<()> {
        let Ok(postgres) = start_postgres() else {
            return Ok(());
        };
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(postgres.connection_string())
            .await?;
        let migrator = sqlx::migrate!("../revaer-data/init");
        migrator.run(&pool).await?;
        let store = MediaStore::new(pool);
        let temp = tempfile::tempdir()?;
        let source_root = temp.path().join("source");
        let output_root = temp.path().join("output");
        fs::create_dir_all(&source_root)?;
        fs::create_dir_all(&output_root)?;
        record_unsupported_hevc_aac_capability(store.pool()).await?;
        let profile_id = create_watcher_profile(
            &store,
            "watcher-only-runtime",
            &source_root,
            &output_root,
            None,
        )
        .await?;
        let runtime = MediaDiscoveryRuntime::with_tick_interval(
            store.clone(),
            Metrics::new()?,
            Duration::from_millis(100),
        );
        let (shutdown_tx, shutdown_rx) = runtime_shutdown::channel();
        let runtime_task = runtime.spawn(shutdown_rx);
        sleep(Duration::from_millis(500)).await;

        let source_path = source_root.join("movie.webm");
        fs::write(&source_path, b"first-version")?;
        wait_for_job_count(&store, profile_id, 1).await?;
        sleep(Duration::from_secs(2)).await;
        assert_eq!(
            list_media_jobs(store.pool(), Some(profile_id), None)
                .await?
                .len(),
            1
        );

        fs::write(&source_path, b"second-version")?;
        wait_for_job_count(&store, profile_id, 2).await?;
        sleep(Duration::from_secs(2)).await;
        let jobs = list_media_jobs(store.pool(), Some(profile_id), None).await?;
        assert_eq!(jobs.len(), 2);
        assert!(jobs.iter().all(|job| job.dry_run));

        assert!(runtime_shutdown::request(&shutdown_tx));
        timeout(Duration::from_secs(5), runtime_task).await??;
        Ok(())
    }

    #[tokio::test]
    async fn watcher_non_dry_run_profile_skips_queueing_without_profile_readiness()
    -> anyhow::Result<()> {
        let Ok(postgres) = start_postgres() else {
            return Ok(());
        };
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(postgres.connection_string())
            .await?;
        let migrator = sqlx::migrate!("../revaer-data/init");
        migrator.run(&pool).await?;
        let store = MediaStore::new(pool);
        let temp = tempfile::tempdir()?;
        let source_root = temp.path().join("source");
        let output_root = temp.path().join("output");
        fs::create_dir_all(&source_root)?;
        fs::create_dir_all(&output_root)?;
        record_unsupported_hevc_aac_capability(store.pool()).await?;
        upsert_media_compatibility_target(
            store.pool(),
            UpsertMediaCompatibilityTargetInput {
                actor_public_id: super::SYSTEM_USER_PUBLIC_ID,
                compatibility_target_key: "watcher-unavailable-codec-target",
                version: 1,
                display_name: "Watcher unavailable codec target",
                video_codec: "watcher-unavailable-video-codec",
                audio_codec: "watcher-unavailable-audio-codec",
                audio_channels: None,
                audio_channel_layout: None,
                subtitle_policy: "selected",
            },
        )
        .await?;
        let profile_id = create_watcher_profile(
            &store,
            "watcher-runtime-capability-gate",
            &source_root,
            &output_root,
            Some("watcher-unavailable-codec-target"),
        )
        .await?;
        update_media_profile(
            store.pool(),
            &UpdateMediaProfileInput {
                actor_public_id: super::SYSTEM_USER_PUBLIC_ID,
                media_profile_public_id: profile_id,
                source_root: None,
                output_root: None,
                dry_run_only: Some(false),
                retention_days: None,
                compatibility_target_key: None,
                policy_key: None,
                watcher_enabled: None,
                schedule_enabled: None,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        let runtime = MediaDiscoveryRuntime::with_tick_interval(
            store.clone(),
            Metrics::new()?,
            Duration::from_millis(100),
        );
        let (shutdown_tx, shutdown_rx) = runtime_shutdown::channel();
        let runtime_task = runtime.spawn(shutdown_rx);
        sleep(Duration::from_millis(500)).await;

        fs::write(source_root.join("movie.webm"), b"needs-capability")?;
        sleep(Duration::from_secs(2)).await;

        assert!(
            list_media_jobs(store.pool(), Some(profile_id), None)
                .await?
                .is_empty()
        );
        assert!(runtime_shutdown::request(&shutdown_tx));
        timeout(Duration::from_secs(5), runtime_task).await??;
        Ok(())
    }

    #[tokio::test]
    async fn watcher_non_dry_run_profile_skips_queueing_when_desired_target_not_ready()
    -> anyhow::Result<()> {
        let Ok(postgres) = start_postgres() else {
            return Ok(());
        };
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(postgres.connection_string())
            .await?;
        let migrator = sqlx::migrate!("../revaer-data/init");
        migrator.run(&pool).await?;
        let store = MediaStore::new(pool);
        let temp = tempfile::tempdir()?;
        let source_root = temp.path().join("source");
        let output_root = temp.path().join("output");
        fs::create_dir_all(&source_root)?;
        fs::create_dir_all(&output_root)?;
        record_unsupported_hevc_aac_capability(store.pool()).await?;
        create_unsupported_hevc_desired_target(store.pool()).await?;
        let profile_id = create_watcher_profile(
            &store,
            "watcher-runtime-desired-target-gate",
            &source_root,
            &output_root,
            None,
        )
        .await?;
        set_media_profile_desired_target(
            store.pool(),
            super::SYSTEM_USER_PUBLIC_ID,
            profile_id,
            Some(WATCHER_UNAVAILABLE_DESIRED_TARGET_KEY),
            Some(1),
        )
        .await?;
        update_media_profile(
            store.pool(),
            &UpdateMediaProfileInput {
                actor_public_id: super::SYSTEM_USER_PUBLIC_ID,
                media_profile_public_id: profile_id,
                source_root: None,
                output_root: None,
                dry_run_only: Some(false),
                retention_days: None,
                compatibility_target_key: None,
                policy_key: None,
                watcher_enabled: None,
                schedule_enabled: None,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        let runtime = MediaDiscoveryRuntime::with_tick_interval(
            store.clone(),
            Metrics::new()?,
            Duration::from_millis(100),
        );
        let (shutdown_tx, shutdown_rx) = runtime_shutdown::channel();
        let runtime_task = runtime.spawn(shutdown_rx);
        sleep(Duration::from_millis(500)).await;

        fs::write(source_root.join("movie.webm"), b"needs-desired-target")?;
        sleep(Duration::from_secs(2)).await;

        assert!(
            list_media_jobs(store.pool(), Some(profile_id), None)
                .await?
                .is_empty()
        );
        assert!(runtime_shutdown::request(&shutdown_tx));
        timeout(Duration::from_secs(5), runtime_task).await??;
        Ok(())
    }

    async fn create_unsupported_hevc_desired_target(pool: &sqlx::PgPool) -> anyhow::Result<()> {
        let target_id = create_media_desired_target(
            pool,
            CreateMediaDesiredTargetInput {
                actor_public_id: super::SYSTEM_USER_PUBLIC_ID,
                target_key: WATCHER_UNAVAILABLE_DESIRED_TARGET_KEY,
                version: 1,
                display_name: "Watcher unavailable desired target",
                container_format: "matroska",
                container_metadata_policy: "preserve",
                container_chapter_policy: "preserve",
                container_attachment_policy: "preserve",
            },
        )
        .await?;
        append_media_desired_target_stream(
            pool,
            AppendMediaDesiredTargetStreamInput {
                media_desired_target_profile_public_id: target_id,
                stream_key: "video-main",
                stream_kind: "video",
                semantic_role: None,
                language_code: None,
                optional: false,
                sort_order: 1,
                codec: "hevc",
                channel_count: None,
                channel_layout: None,
                audio_bitrate_bps: None,
                audio_sample_rate_hz: None,
                audio_loudness_profile: None,
                audio_dynamic_range: None,
                video_profile: None,
                video_level: None,
                video_bitrate_bps: None,
                video_width_px: None,
                video_height_px: None,
                video_pixel_format: None,
                video_bit_depth: None,
                video_average_frame_rate: None,
                color_range: None,
                color_primaries: None,
                color_transfer: None,
                color_space: None,
                hdr_format: None,
                hdr10_mastering_red_x: None,
                hdr10_mastering_red_y: None,
                hdr10_mastering_green_x: None,
                hdr10_mastering_green_y: None,
                hdr10_mastering_blue_x: None,
                hdr10_mastering_blue_y: None,
                hdr10_mastering_white_point_x: None,
                hdr10_mastering_white_point_y: None,
                hdr10_mastering_min_luminance: None,
                hdr10_mastering_max_luminance: None,
                hdr10_max_content_light_level: None,
                hdr10_max_frame_average_light_level: None,
                title: None,
                default_disposition: true,
                forced_disposition: false,
                subtitle_placement: None,
                image_subtitle_action: None,
            },
        )
        .await?;
        Ok(())
    }

    async fn record_unsupported_hevc_aac_capability(pool: &sqlx::PgPool) -> anyhow::Result<()> {
        let snapshot_run_public_id = Uuid::new_v4();
        start_capability_snapshot_run_with_executor(
            pool,
            super::SYSTEM_USER_PUBLIC_ID,
            snapshot_run_public_id,
        )
        .await?;
        for codec_name in ["h264", "mp3"] {
            record_capability_snapshot(
                pool,
                &RecordCapabilitySnapshotInput {
                    actor_public_id: super::SYSTEM_USER_PUBLIC_ID,
                    snapshot_run_public_id: Some(snapshot_run_public_id),
                    ffmpeg_version: "7.0",
                    ffprobe_version: "7.0",
                    codec_name,
                    encode_supported: true,
                    decode_supported: true,
                },
            )
            .await?;
        }
        record_capability_encoder(
            pool,
            &RecordCapabilityEncoderInput {
                actor_public_id: super::SYSTEM_USER_PUBLIC_ID,
                snapshot_run_public_id,
                encoder_name: "libx264",
            },
        )
        .await?;
        for (feature_family, feature_name) in [
            ("decoder", "h264"),
            ("muxer", "matroska"),
            ("demuxer", "matroska"),
            ("filesystem", "local"),
            ("utility", "ffmpeg"),
            ("utility", "ffprobe"),
            ("utility", "ffplay"),
            ("license", "gpl"),
        ] {
            record_capability_feature(
                pool,
                &RecordCapabilityFeatureInput {
                    actor_public_id: super::SYSTEM_USER_PUBLIC_ID,
                    snapshot_run_public_id,
                    feature_family,
                    feature_name,
                    supported: true,
                    detail_text: None,
                },
            )
            .await?;
        }
        complete_capability_snapshot_run_with_executor(pool, snapshot_run_public_id).await?;
        Ok(())
    }

    async fn wait_for_job_count(
        store: &MediaStore,
        profile_id: Uuid,
        expected: usize,
    ) -> anyhow::Result<()> {
        timeout(Duration::from_secs(10), async {
            loop {
                let jobs = list_media_jobs(store.pool(), Some(profile_id), None).await?;
                if jobs.len() == expected {
                    return Ok::<(), DataError>(());
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await??;
        Ok(())
    }
}
