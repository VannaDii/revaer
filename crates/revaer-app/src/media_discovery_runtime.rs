//! In-process media discovery runtime.
//!
//! # Design
//! - Scans scheduled media profile source roots on a bounded interval.
//! - Reuses discovery preview path derivation before queueing jobs.
//! - Keeps in-process de-duplication so one app run does not enqueue the same path repeatedly.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use revaer_api::app::media::MediaDiscoveryPreviewResponse;
use revaer_data::DataError;
use revaer_data::media::jobs::CreateMediaJobInput;
use revaer_data::media::profiles::MediaProfileRow;
use revaer_runtime::media::MediaStore;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tracing::warn;
use uuid::Uuid;

use crate::media::{build_discovery_previews, ensure_execution_capability_snapshot};

const DEFAULT_DISCOVERY_TICK_INTERVAL: Duration = Duration::from_mins(1);
const SYSTEM_USER_PUBLIC_ID: Uuid = Uuid::from_u128(0);
const MEDIA_FILE_EXTENSIONS: &[&str] = &["avi", "m4v", "mkv", "mov", "mp4"];

/// Background runtime that discovers media files for enabled profiles.
pub(crate) struct MediaDiscoveryRuntime {
    store: MediaStore,
    tick_interval: Duration,
    seen: Mutex<BTreeSet<(Uuid, String)>>,
    last_scheduled: Mutex<BTreeMap<Uuid, Instant>>,
}

impl MediaDiscoveryRuntime {
    /// Construct a production media discovery runtime.
    #[must_use]
    pub(crate) const fn new(store: MediaStore) -> Self {
        Self::with_tick_interval(store, DEFAULT_DISCOVERY_TICK_INTERVAL)
    }

    const fn with_tick_interval(store: MediaStore, tick_interval: Duration) -> Self {
        Self {
            store,
            tick_interval,
            seen: Mutex::new(BTreeSet::new()),
            last_scheduled: Mutex::new(BTreeMap::new()),
        }
    }

    /// Spawn the discovery loop.
    pub(crate) fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run_loop().await;
        })
    }

    async fn run_loop(self) {
        let mut ticker = interval(self.tick_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            if let Err(error) = self.run_tick().await {
                warn!(error = %error, "media discovery runtime tick failed");
            }
        }
    }

    async fn run_tick(&self) -> Result<(), MediaDiscoveryRuntimeError> {
        let profiles = self.store.list_profiles().await?;
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

        let interval_minutes = profile.schedule_interval_minutes.unwrap_or(1).max(1);
        let interval_seconds = u64::try_from(interval_minutes)
            .map_err(|_| MediaDiscoveryRuntimeError::InvalidInterval)?
            .saturating_mul(60);
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
                return Ok(());
            }
        }

        let source_paths = discover_media_source_paths(&profile.source_root)?;
        let previews = build_discovery_previews(
            &source_paths,
            &profile.source_root,
            &profile.output_root,
            profile.dry_run_only,
        );
        self.queue_previews(&profile, previews).await
    }

    async fn queue_previews(
        &self,
        profile: &MediaProfileRow,
        previews: Vec<MediaDiscoveryPreviewResponse>,
    ) -> Result<(), MediaDiscoveryRuntimeError> {
        for preview in previews {
            if !preview.accepted {
                continue;
            }
            let Some(output_path) = preview.output_path.as_deref() else {
                continue;
            };
            if self.has_seen(profile.media_profile_public_id, &preview.source_path)? {
                continue;
            }
            self.store
                .create_job(&CreateMediaJobInput {
                    actor_public_id: SYSTEM_USER_PUBLIC_ID,
                    media_profile_public_id: profile.media_profile_public_id,
                    source_path: &preview.source_path,
                    output_path: Some(output_path),
                    dry_run: profile.dry_run_only,
                })
                .await?;
            self.mark_seen(profile.media_profile_public_id, &preview.source_path)?;
        }
        Ok(())
    }

    fn has_seen(
        &self,
        media_profile_public_id: Uuid,
        source_path: &str,
    ) -> Result<bool, MediaDiscoveryRuntimeError> {
        let seen = self
            .seen
            .lock()
            .map_err(|error| MediaDiscoveryRuntimeError::Lock(error.to_string()))?;
        Ok(seen.contains(&(media_profile_public_id, source_path.to_string())))
    }

    fn mark_seen(
        &self,
        media_profile_public_id: Uuid,
        source_path: &str,
    ) -> Result<(), MediaDiscoveryRuntimeError> {
        let mut seen = self
            .seen
            .lock()
            .map_err(|error| MediaDiscoveryRuntimeError::Lock(error.to_string()))?;
        seen.insert((media_profile_public_id, source_path.to_string()));
        drop(seen);
        Ok(())
    }
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
    #[error("media discovery runtime schedule interval invalid")]
    InvalidInterval,
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
    use super::{MediaDiscoveryRuntime, discover_media_source_paths, is_media_file};
    use chrono::Utc;
    use revaer_data::media::profiles::MediaProfileRow;
    use revaer_runtime::media::MediaStore;
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
        assert!(!is_media_file(Path::new("/media/movie.srt")));
    }

    #[tokio::test]
    async fn profile_due_ignores_watchers_and_skips_disabled_schedules() -> anyhow::Result<()> {
        let runtime =
            MediaDiscoveryRuntime::with_tick_interval(closed_media_store(), Duration::from_secs(1));
        let now = Instant::now();
        let watcher = media_profile(true, false, None);
        let disabled = media_profile(false, false, None);

        assert!(!runtime.profile_due(&watcher, now)?);
        assert!(!runtime.profile_due(&disabled, now)?);
        Ok(())
    }

    #[tokio::test]
    async fn profile_due_throttles_schedule_profiles_by_interval() -> anyhow::Result<()> {
        let runtime =
            MediaDiscoveryRuntime::with_tick_interval(closed_media_store(), Duration::from_secs(1));
        let now = Instant::now();
        let scheduled = media_profile(false, true, Some(2));

        assert!(runtime.profile_due(&scheduled, now)?);
        assert!(!runtime.profile_due(&scheduled, now + Duration::from_secs(119))?);
        assert!(runtime.profile_due(&scheduled, now + Duration::from_mins(2))?);

        let default_interval = media_profile(false, true, None);
        assert!(runtime.profile_due(&default_interval, now)?);
        assert!(!runtime.profile_due(&default_interval, now + Duration::from_secs(59))?);
        assert!(runtime.profile_due(&default_interval, now + Duration::from_mins(1))?);
        Ok(())
    }
}
