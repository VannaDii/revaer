//! Filesystem event adapter for media discovery.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use revaer_data::media::profiles::MediaProfileRow;
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

/// One filesystem event associated with a configured media profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaWatchEvent {
    pub(crate) media_profile_public_id: Uuid,
    pub(crate) path: PathBuf,
}

/// Fixed-capacity, key-coalescing boundary between native callbacks and the runtime.
pub(crate) struct MediaWatchEventBuffer {
    capacity: usize,
    state: Mutex<MediaWatchEventBufferState>,
}

#[derive(Default)]
struct MediaWatchEventBufferState {
    events: BTreeMap<(Uuid, PathBuf), MediaWatchEvent>,
    overflowed_profiles: BTreeSet<Uuid>,
}

pub(crate) struct DrainedMediaWatchEvents {
    pub(crate) events: Vec<MediaWatchEvent>,
    pub(crate) overflowed_profiles: BTreeSet<Uuid>,
}

impl MediaWatchEventBuffer {
    #[must_use]
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            state: Mutex::new(MediaWatchEventBufferState::default()),
        }
    }

    fn record(&self, event: MediaWatchEvent) -> Result<(), MediaWatcherError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| MediaWatcherError::EventBuffer(error.to_string()))?;
        let key = (event.media_profile_public_id, event.path.clone());
        if state.events.contains_key(&key) || state.events.len() < self.capacity {
            state.events.insert(key, event);
        } else {
            state
                .overflowed_profiles
                .insert(event.media_profile_public_id);
        }
        drop(state);
        Ok(())
    }

    pub(crate) fn drain(&self) -> Result<DrainedMediaWatchEvents, MediaWatcherError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| MediaWatcherError::EventBuffer(error.to_string()))?;
        Ok(DrainedMediaWatchEvents {
            events: std::mem::take(&mut state.events).into_values().collect(),
            overflowed_profiles: std::mem::take(&mut state.overflowed_profiles),
        })
    }
}

/// Injectable filesystem watcher boundary.
pub(crate) trait MediaWatcher: Send + Sync {
    fn synchronize(&mut self, profiles: &[MediaProfileRow]) -> Vec<MediaWatcherError>;
}

struct WatchRegistration {
    source_root: PathBuf,
    _watcher: RecommendedWatcher,
}

/// Native watcher selected by `notify` for the current operating system.
pub(crate) struct NotifyMediaWatcher {
    events: Arc<MediaWatchEventBuffer>,
    registrations: BTreeMap<Uuid, WatchRegistration>,
}

impl NotifyMediaWatcher {
    #[must_use]
    pub(crate) const fn new(events: Arc<MediaWatchEventBuffer>) -> Self {
        Self {
            events,
            registrations: BTreeMap::new(),
        }
    }

    fn register(&mut self, profile: &MediaProfileRow) -> Result<(), MediaWatcherError> {
        let source_root = PathBuf::from(&profile.source_root);
        if !source_root.is_dir() {
            return Err(MediaWatcherError::SourceRootUnavailable(source_root));
        }

        let profile_id = profile.media_profile_public_id;
        let sender = self.events.clone();
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<Event>| match result {
                Ok(event) if event_can_change_media(event.kind) => {
                    for path in event.paths {
                        if let Err(error) = sender.record(MediaWatchEvent {
                            media_profile_public_id: profile_id,
                            path,
                        }) {
                            warn!(error = %error, "media watcher event buffer failed");
                        }
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    warn!(
                        media_profile_public_id = %profile_id,
                        error = %error,
                        "media filesystem watcher event failed"
                    );
                }
            })
            .map_err(|source| MediaWatcherError::Create {
                path: source_root.clone(),
                source,
            })?;
        watcher
            .watch(Path::new(&source_root), RecursiveMode::Recursive)
            .map_err(|source| MediaWatcherError::Watch {
                path: source_root.clone(),
                source,
            })?;
        self.registrations.insert(
            profile_id,
            WatchRegistration {
                source_root,
                _watcher: watcher,
            },
        );
        Ok(())
    }
}

impl MediaWatcher for NotifyMediaWatcher {
    fn synchronize(&mut self, profiles: &[MediaProfileRow]) -> Vec<MediaWatcherError> {
        let desired = profiles
            .iter()
            .filter(|profile| profile.watcher_enabled)
            .map(|profile| {
                (
                    profile.media_profile_public_id,
                    PathBuf::from(&profile.source_root),
                )
            })
            .collect::<BTreeMap<_, _>>();
        self.registrations.retain(|profile_id, registration| {
            desired
                .get(profile_id)
                .is_some_and(|root| root == &registration.source_root)
        });

        let mut errors = Vec::new();
        for profile in profiles.iter().filter(|profile| profile.watcher_enabled) {
            if !self
                .registrations
                .contains_key(&profile.media_profile_public_id)
                && let Err(error) = self.register(profile)
            {
                errors.push(error);
            }
        }
        errors
    }
}

const fn event_can_change_media(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any | EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

/// Filesystem watcher setup failure.
#[derive(Debug, Error)]
pub(crate) enum MediaWatcherError {
    #[error("media watcher source root is unavailable: {0}")]
    SourceRootUnavailable(PathBuf),
    #[error("media watcher creation failed for {path}: {source}")]
    Create {
        path: PathBuf,
        source: notify::Error,
    },
    #[error("media watcher registration failed for {path}: {source}")]
    Watch {
        path: PathBuf,
        source: notify::Error,
    },
    #[error("media watcher event buffer failed: {0}")]
    EventBuffer(String),
}

#[cfg(test)]
mod tests {
    use super::{
        MediaWatchEvent, MediaWatchEventBuffer, MediaWatcher, NotifyMediaWatcher,
        event_can_change_media,
    };
    use chrono::Utc;
    use notify::EventKind;
    use notify::event::{CreateKind, ModifyKind, RemoveKind};
    use revaer_data::media::profiles::MediaProfileRow;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::{sleep, timeout};
    use uuid::Uuid;

    #[test]
    fn watcher_accepts_media_change_events() {
        assert!(event_can_change_media(EventKind::Create(CreateKind::File)));
        assert!(event_can_change_media(EventKind::Modify(ModifyKind::Any)));
        assert!(event_can_change_media(EventKind::Remove(RemoveKind::File)));
        assert!(event_can_change_media(EventKind::Any));
        assert!(!event_can_change_media(EventKind::Other));
    }

    #[tokio::test]
    async fn native_watcher_reports_recursive_media_file_changes() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let nested = root.path().join("nested");
        fs::create_dir_all(&nested)?;
        let profile_id = Uuid::new_v4();
        let profile = MediaProfileRow {
            media_profile_public_id: profile_id,
            profile_key: "watch-test".to_string(),
            source_root: root.path().to_string_lossy().into_owned(),
            output_root: root.path().join("output").to_string_lossy().into_owned(),
            dry_run_only: true,
            retention_days: 30,
            compatibility_target_key: None,
            policy_key: "safe_dry_run".to_string(),
            watcher_enabled: true,
            schedule_enabled: false,
            schedule_interval_minutes: None,
            desired_target_key: None,
            desired_target_version: None,
            updated_at: Utc::now(),
        };
        let events = Arc::new(MediaWatchEventBuffer::new(32));
        let mut watcher = NotifyMediaWatcher::new(events.clone());
        let errors = watcher.synchronize(&[profile]);
        if let Some(error) = errors.into_iter().next() {
            return Err(error.into());
        }
        sleep(Duration::from_secs(1)).await;

        let media_path = nested.join("movie.webm");
        fs::write(&media_path, b"first")?;
        sleep(Duration::from_millis(100)).await;
        fs::write(&media_path, b"second")?;
        let canonical_media_path = media_path.canonicalize()?;
        let observed = timeout(Duration::from_secs(10), async {
            loop {
                let drained = events.drain().ok()?;
                if let Some(event) = drained.events.into_iter().find(|event| {
                    event.path.canonicalize().ok().is_some_and(|path| {
                        path == canonical_media_path || canonical_media_path.starts_with(path)
                    })
                }) {
                    return Some(event);
                }
                sleep(Duration::from_millis(25)).await;
            }
        })
        .await?
        .ok_or_else(|| anyhow::anyhow!("native watcher event channel closed"))?;

        assert_eq!(observed.media_profile_public_id, profile_id);
        assert!(
            observed
                .path
                .canonicalize()?
                .starts_with(root.path().canonicalize()?)
        );
        Ok(())
    }

    #[test]
    fn event_buffer_coalesces_and_bounds_a_paused_consumer() -> anyhow::Result<()> {
        let profile_id = Uuid::new_v4();
        let events = MediaWatchEventBuffer::new(4);
        for index in 0..1_024 {
            events.record(MediaWatchEvent {
                media_profile_public_id: profile_id,
                path: PathBuf::from(format!("movie-{}.mkv", index % 8)),
            })?;
        }
        let drained = events.drain()?;
        assert_eq!(drained.events.len(), 4);
        assert_eq!(drained.overflowed_profiles, BTreeSet::from([profile_id]));
        Ok(())
    }
}
