//! In-process media discovery runtime.
//!
//! # Design
//! - Scans scheduled media profile source roots on a bounded interval.
//! - Consumes recursive native filesystem events for watcher-enabled profiles.
//! - Reuses discovery preview path derivation before queueing jobs.
//! - Atomically persists stable source fingerprints with job creation for restart-safe de-duplication.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use revaer_api::app::media::MediaDiscoveryPreviewResponse;
use revaer_data::DataError;
use revaer_data::media::jobs::EnqueueDiscoveredMediaJobInput;
use revaer_data::media::profiles::MediaProfileRow;
use revaer_runtime::media::MediaStore;
use revaer_telemetry::Metrics;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tracing::warn;
use uuid::Uuid;

use crate::media::{build_discovery_previews, ensure_execution_capability_snapshot};
use crate::media_discovery_watcher::{MediaWatchEvent, MediaWatcher, NotifyMediaWatcher};
use crate::runtime_shutdown::{self, RuntimeShutdownReceiver};

const DEFAULT_DISCOVERY_TICK_INTERVAL: Duration = Duration::from_mins(1);
const WATCH_DEBOUNCE_INTERVAL: Duration = Duration::from_secs(1);
const WATCH_DEBOUNCE_TICK_INTERVAL: Duration = Duration::from_millis(100);
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
    watch_events: UnboundedReceiver<MediaWatchEvent>,
    watcher_profiles: BTreeMap<Uuid, MediaProfileRow>,
    pending_watch_events: BTreeMap<(Uuid, PathBuf), Instant>,
}

impl MediaDiscoveryRuntime {
    /// Construct a production media discovery runtime.
    #[must_use]
    pub(crate) fn new(store: MediaStore, telemetry: Metrics) -> Self {
        Self::with_tick_interval(store, telemetry, DEFAULT_DISCOVERY_TICK_INTERVAL)
    }

    fn with_tick_interval(store: MediaStore, telemetry: Metrics, tick_interval: Duration) -> Self {
        let (watch_events_tx, watch_events) = unbounded_channel();
        Self {
            store,
            telemetry,
            tick_interval,
            last_scheduled: Mutex::new(BTreeMap::new()),
            watcher: Box::new(NotifyMediaWatcher::new(watch_events_tx)),
            watch_events,
            watcher_profiles: BTreeMap::new(),
            pending_watch_events: BTreeMap::new(),
        }
    }

    /// Spawn the discovery loop.
    pub(crate) fn spawn(self, shutdown: RuntimeShutdownReceiver) -> JoinHandle<()> {
        tokio::spawn(async move {
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
                event = self.watch_events.recv() => {
                    if let Some(event) = event {
                        self.record_watch_event(&event);
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
                self.discover_profile(profile).await?;
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
        &self,
        profile: MediaProfileRow,
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
                    .inc_media_discovery_candidate("schedule", "capability_not_ready");
                return Ok(());
            }
        }

        let source_paths = discover_media_source_paths(&profile.source_root)?;
        self.queue_source_paths(&profile, source_paths, "schedule")
            .await
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
        for preview in previews {
            if !preview.accepted {
                self.telemetry
                    .inc_media_discovery_candidate(origin, "skipped");
                continue;
            }
            let Some(output_path) = preview.output_path.as_deref() else {
                self.telemetry
                    .inc_media_discovery_candidate(origin, "skipped");
                continue;
            };
            let source_path = PathBuf::from(&preview.source_path);
            let source_root = PathBuf::from(&profile.source_root);
            let fingerprint = tokio::task::spawn_blocking(move || {
                fingerprint_media_file(&source_path, &source_root)
            })
            .await
            .map_err(|error| MediaDiscoveryRuntimeError::Join(error.to_string()))??;
            let Some(fingerprint) = fingerprint else {
                self.telemetry
                    .inc_media_discovery_candidate(origin, "unstable");
                continue;
            };
            let create_result = self
                .store
                .enqueue_discovered_job(&EnqueueDiscoveredMediaJobInput {
                    actor_public_id: SYSTEM_USER_PUBLIC_ID,
                    media_profile_public_id: profile.media_profile_public_id,
                    source_path: &preview.source_path,
                    output_path,
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
        if !is_media_file(&profile_path) {
            return;
        }
        self.pending_watch_events.insert(
            (event.media_profile_public_id, profile_path),
            Instant::now() + WATCH_DEBOUNCE_INTERVAL,
        );
    }

    async fn flush_watch_events(&mut self) -> Result<(), MediaDiscoveryRuntimeError> {
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
        Ok(())
    }
}

fn rebase_watch_event_path(event_path: &Path, source_root: &str) -> Option<PathBuf> {
    let configured_root = Path::new(source_root);
    let canonical_root = configured_root.canonicalize().ok()?;
    let canonical_event = event_path.canonicalize().ok()?;
    let relative = canonical_event.strip_prefix(canonical_root).ok()?;
    Some(configured_root.join(relative))
}

#[derive(Debug, Error)]
enum MediaDiscoveryRuntimeError {
    #[error("media discovery runtime data error: {0}")]
    Data(#[from] DataError),
    #[error("media discovery runtime io error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("media discovery runtime path is not unicode: {0}")]
    NonUnicodePath(PathBuf),
    #[error("media discovery runtime lock failed: {0}")]
    Lock(String),
    #[error("media discovery runtime task join failed: {0}")]
    Join(String),
    #[error("media discovery runtime schedule interval invalid")]
    InvalidInterval,
    #[error("media discovery fingerprint timestamp predates the Unix epoch")]
    FingerprintTimeBeforeEpoch,
    #[error("media discovery fingerprint value is too large: {0}")]
    FingerprintValueTooLarge(&'static str),
}

struct MediaFileFingerprint {
    size_bytes: i64,
    modified_ns: i64,
    sha256: String,
}

fn fingerprint_media_file(
    path: &Path,
    source_root: &Path,
) -> Result<Option<MediaFileFingerprint>, MediaDiscoveryRuntimeError> {
    let canonical_root =
        source_root
            .canonicalize()
            .map_err(|source| MediaDiscoveryRuntimeError::Io {
                path: source_root.to_path_buf(),
                source,
            })?;
    let canonical_path = match path.canonicalize() {
        Ok(value) => value,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(MediaDiscoveryRuntimeError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if !canonical_path.starts_with(&canonical_root) {
        return Ok(None);
    }
    let before =
        fs::metadata(&canonical_path).map_err(|source| MediaDiscoveryRuntimeError::Io {
            path: canonical_path.clone(),
            source,
        })?;
    if !before.is_file() {
        return Ok(None);
    }
    let before_modified = before
        .modified()
        .map_err(|source| MediaDiscoveryRuntimeError::Io {
            path: canonical_path.clone(),
            source,
        })?;
    let mut file =
        File::open(&canonical_path).map_err(|source| MediaDiscoveryRuntimeError::Io {
            path: canonical_path.clone(),
            source,
        })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| MediaDiscoveryRuntimeError::Io {
                path: canonical_path.clone(),
                source,
            })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let after = fs::metadata(&canonical_path).map_err(|source| MediaDiscoveryRuntimeError::Io {
        path: canonical_path.clone(),
        source,
    })?;
    let after_modified = after
        .modified()
        .map_err(|source| MediaDiscoveryRuntimeError::Io {
            path: canonical_path.clone(),
            source,
        })?;
    if before.len() != after.len() || before_modified != after_modified {
        return Ok(None);
    }
    let size_bytes = i64::try_from(after.len())
        .map_err(|_| MediaDiscoveryRuntimeError::FingerprintValueTooLarge("size_bytes"))?;
    let modified_ns = system_time_ns(after_modified)?;
    Ok(Some(MediaFileFingerprint {
        size_bytes,
        modified_ns,
        sha256: format!("{:x}", hasher.finalize()),
    }))
}

fn system_time_ns(value: SystemTime) -> Result<i64, MediaDiscoveryRuntimeError> {
    let duration = value
        .duration_since(UNIX_EPOCH)
        .map_err(|_| MediaDiscoveryRuntimeError::FingerprintTimeBeforeEpoch)?;
    i64::try_from(duration.as_nanos())
        .map_err(|_| MediaDiscoveryRuntimeError::FingerprintValueTooLarge("modified_ns"))
}

fn discover_media_source_paths(
    source_root: &str,
) -> Result<Vec<String>, MediaDiscoveryRuntimeError> {
    let mut paths = Vec::new();
    visit_media_source_paths(Path::new(source_root), &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn visit_media_source_paths(
    path: &Path,
    paths: &mut Vec<String>,
) -> Result<(), MediaDiscoveryRuntimeError> {
    for entry in fs::read_dir(path).map_err(|source| MediaDiscoveryRuntimeError::Io {
        path: path.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| MediaDiscoveryRuntimeError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| MediaDiscoveryRuntimeError::Io {
                path: entry_path.clone(),
                source,
            })?;
        if file_type.is_dir() {
            visit_media_source_paths(&entry_path, paths)?;
        } else if file_type.is_file() && is_media_file(&entry_path) {
            let path_text = entry_path
                .to_str()
                .map(str::to_string)
                .ok_or_else(|| MediaDiscoveryRuntimeError::NonUnicodePath(entry_path.clone()))?;
            paths.push(path_text);
        }
    }
    Ok(())
}

fn is_media_file(path: &Path) -> bool {
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
    use super::{
        MediaDiscoveryRuntime, discover_media_source_paths, fingerprint_media_file, is_media_file,
        rebase_watch_event_path,
    };
    use crate::runtime_shutdown;
    use chrono::Utc;
    use revaer_data::DataError;
    use revaer_data::media::jobs::list_media_jobs;
    use revaer_data::media::profiles::MediaProfileRow;
    use revaer_data::media::profiles::{UpsertMediaProfileInput, upsert_media_profile};
    use revaer_runtime::media::MediaStore;
    use revaer_telemetry::Metrics;
    use revaer_test_support::postgres::start_postgres;
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, Instant};
    use tokio::time::{sleep, timeout};
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
    fn discover_media_source_paths_returns_supported_files_sorted() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        fs::create_dir_all(root.join("nested"))?;
        fs::write(root.join("b.txt"), b"ignored")?;
        fs::write(root.join("nested").join("c.MP4"), b"media")?;
        fs::write(root.join("a.mkv"), b"media")?;

        let root_text = root.to_string_lossy();
        let paths = discover_media_source_paths(root_text.as_ref())?;

        assert_eq!(
            paths
                .iter()
                .map(|path| path.strip_prefix(root.to_string_lossy().as_ref()))
                .collect::<Vec<_>>(),
            vec![Some("/a.mkv"), Some("/nested/c.MP4")]
        );
        Ok(())
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
    fn fingerprint_is_stable_and_detects_same_size_content_changes() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("movie.mkv");
        fs::write(&path, b"first")?;
        let first = fingerprint_media_file(&path, temp.path())?
            .ok_or_else(|| anyhow::anyhow!("stable test file did not produce a fingerprint"))?;
        let repeated = fingerprint_media_file(&path, temp.path())?.ok_or_else(|| {
            anyhow::anyhow!("stable test file did not produce a repeated fingerprint")
        })?;
        assert_eq!(first.size_bytes, repeated.size_bytes);
        assert_eq!(first.modified_ns, repeated.modified_ns);
        assert_eq!(first.sha256, repeated.sha256);

        fs::write(&path, b"other")?;
        let changed = fingerprint_media_file(&path, temp.path())?
            .ok_or_else(|| anyhow::anyhow!("changed test file did not produce a fingerprint"))?;
        assert_eq!(first.size_bytes, changed.size_bytes);
        assert_ne!(first.sha256, changed.sha256);
        assert_eq!(
            rebase_watch_event_path(
                &path.canonicalize()?,
                temp.path().to_string_lossy().as_ref()
            ),
            Some(path)
        );
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
        let profile_id = upsert_media_profile(
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
            list_media_jobs(store.pool(), profile_id, None).await?.len(),
            1
        );

        fs::write(&source_path, b"second-version")?;
        wait_for_job_count(&store, profile_id, 2).await?;
        sleep(Duration::from_secs(2)).await;
        let jobs = list_media_jobs(store.pool(), profile_id, None).await?;
        assert_eq!(jobs.len(), 2);
        assert!(jobs.iter().all(|job| job.dry_run));

        assert!(runtime_shutdown::request(&shutdown_tx));
        timeout(Duration::from_secs(5), runtime_task).await??;
        Ok(())
    }

    async fn wait_for_job_count(
        store: &MediaStore,
        profile_id: Uuid,
        expected: usize,
    ) -> anyhow::Result<()> {
        timeout(Duration::from_secs(10), async {
            loop {
                let jobs = list_media_jobs(store.pool(), profile_id, None).await?;
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
