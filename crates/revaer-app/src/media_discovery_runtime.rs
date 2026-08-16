//! In-process media discovery runtime.
//!
//! # Design
//! - Scans scheduled media profile source roots on a bounded interval.
//! - Consumes recursive native filesystem events for watcher-enabled profiles.
//! - Reuses discovery preview path derivation before queueing jobs.
//! - Atomically persists stable source fingerprints with job creation for restart-safe de-duplication.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use revaer_api::app::media::MediaDiscoveryPreviewResponse;
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

use crate::media::{build_discovery_previews, ensure_execution_capability_snapshot};
use crate::media_discovery_fingerprint::{
    fingerprint_media_aggregate, owner_for_changed_path, revalidate_media_aggregate,
};
use crate::media_discovery_scan::{ScanBudget, ScanCursor, ScanError, scan_media_source_paths};
use crate::media_discovery_watcher::{
    MediaWatchEvent, MediaWatchEventBuffer, MediaWatcher, MediaWatcherError, NotifyMediaWatcher,
};

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
    overflowed_profiles: BTreeSet<Uuid>,
    scan_cursors: BTreeMap<Uuid, ScanCursor>,
    cancelled: Arc<AtomicBool>,
}

pub(crate) struct MediaDiscoveryTask {
    handle: JoinHandle<()>,
    cancelled: Arc<AtomicBool>,
}

impl MediaDiscoveryTask {
    pub(crate) fn abort(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.handle.abort();
    }

    pub(crate) async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.handle.await
    }
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
            overflowed_profiles: BTreeSet::new(),
            scan_cursors: BTreeMap::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawn the discovery loop.
    pub(crate) fn spawn(self) -> MediaDiscoveryTask {
        let cancelled = self.cancelled.clone();
        let task_cancelled = self.cancelled.clone();
        let handle = tokio::spawn(async move {
            let _guard = CancelOnDrop(task_cancelled);
            self.run_loop().await;
        });
        MediaDiscoveryTask { handle, cancelled }
    }

    async fn run_loop(mut self) {
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
        if !profile.dry_run_only {
            let latest = self.store.latest_capability().await?;
            if let Err(error) = ensure_execution_capability_snapshot(latest.as_ref()) {
                warn!(
                    media_profile_public_id = %profile.media_profile_public_id,
                    error = %error,
                    "media discovery skipped profile without ready execution capability"
                );
                self.telemetry
                    .inc_media_discovery_candidate(origin, "capability_not_ready");
                return Ok(());
            }
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
            let expected_sha256 = fingerprint.sha256.clone();
            let valid = tokio::task::spawn_blocking(move || {
                revalidate_media_aggregate(&validation_path, &validation_root, &expected_sha256)
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
                    output_path: &output_path,
                    source_size_bytes: fingerprint.size_bytes,
                    source_modified_ns: fingerprint.modified_ns,
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
        let Some(profile) = self.watcher_profiles.get(&event.media_profile_public_id) else {
            return;
        };
        let Some(profile_path) = rebase_watch_event_path(&event.path, &profile.source_root) else {
            return;
        };
        let Some(owner) = owner_for_changed_path(&profile_path, Path::new(&profile.source_root))
        else {
            return;
        };
        let key = (event.media_profile_public_id, owner);
        if self.pending_watch_events.len() >= WATCH_EVENT_CAPACITY
            && !self.pending_watch_events.contains_key(&key)
        {
            self.overflowed_profiles
                .insert(event.media_profile_public_id);
            return;
        }
        self.pending_watch_events
            .insert(key, Instant::now() + WATCH_DEBOUNCE_INTERVAL);
    }

    async fn flush_watch_events(&mut self) -> Result<(), MediaDiscoveryRuntimeError> {
        let drained = self.watch_events.drain()?;
        self.overflowed_profiles.extend(drained.overflowed_profiles);
        for event in drained.events {
            self.record_watch_event(&event);
        }
        let now = Instant::now();
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
            self.queue_source_paths(&profile, vec![path_text], "watcher")
                .await?;
        }
        let overflowed = std::mem::take(&mut self.overflowed_profiles);
        for profile_id in overflowed {
            self.telemetry
                .inc_media_discovery_candidate("watcher", "overflow");
            if let Some(profile) = self.watcher_profiles.get(&profile_id).cloned() {
                self.discover_profile(profile, "watcher").await?;
            }
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
    use chrono::Utc;
    use revaer_api::app::media::MediaDiscoveryPreviewResponse;
    use revaer_data::media::profiles::MediaProfileRow;
    use revaer_data::media::profiles::{UpsertMediaProfileInput, upsert_media_profile};
    use revaer_runtime::media::MediaStore;
    use revaer_telemetry::Metrics;
    use revaer_test_support::postgres::start_postgres;
    use std::cell::Cell;
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, Instant};
    use uuid::Uuid;

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

    #[test]
    fn is_media_file_accepts_expected_extensions() {
        assert!(is_media_file(Path::new("/media/movie.mkv")));
        assert!(is_media_file(Path::new("/media/movie.MP4")));
        assert!(is_media_file(Path::new("/media/movie.webm")));
        assert!(is_media_file(Path::new("/media/movie.ts")));
        assert!(is_media_file(Path::new("/media/movie.m4a")));
        assert!(!is_media_file(Path::new("/media/movie.srt")));
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
    async fn legacy_watcher_profile_requires_verified_root_identity() -> anyhow::Result<()> {
        let Ok(postgres) = start_postgres() else {
            return Ok(());
        };
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(postgres.connection_string())
            .await?;
        let mut migrator = sqlx::migrate!("../revaer-data/migrations");
        migrator.set_ignore_missing(true);
        migrator.run(&pool).await?;
        let store = MediaStore::new(pool);
        let temp = tempfile::tempdir()?;
        let source_root = temp.path().join("source");
        let output_root = temp.path().join("output");
        fs::create_dir_all(&source_root)?;
        fs::create_dir_all(&output_root)?;
        let source_root_text = source_root
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("source root is not Unicode"))?;
        let output_root_text = output_root
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("output root is not Unicode"))?;
        let result = upsert_media_profile(
            store.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: super::SYSTEM_USER_PUBLIC_ID,
                profile_key: "watcher-only-runtime",
                source_root: source_root_text,
                output_root: output_root_text,
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: true,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await;
        let Err(error) = result else {
            return Err(anyhow::anyhow!(
                "legacy profile creation must reject watcher automation"
            ));
        };
        assert_eq!(
            error.database_detail(),
            Some("media_profile_filesystem_identity_required")
        );
        Ok(())
    }
}
