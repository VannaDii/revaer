//! Filesystem event adapter for media discovery.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

/// One filesystem event keyed by its caller-owned registration identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaWatchEvent {
    pub(crate) registration_public_id: Uuid,
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
    uncertain_profiles: BTreeSet<Uuid>,
}

pub(crate) struct DrainedMediaWatchEvents {
    pub(crate) events: Vec<MediaWatchEvent>,
    pub(crate) overflowed_profiles: BTreeSet<Uuid>,
    pub(crate) uncertain_profiles: BTreeSet<Uuid>,
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
        // Retained DISC watch limits apply before cloning a callback path.
        let path_bytes = event.path.as_os_str().len();
        if path_bytes > 2 * 1024 * 1024 {
            state
                .overflowed_profiles
                .insert(event.registration_public_id);
            return Ok(());
        }
        let key = (event.registration_public_id, event.path.clone());
        let per_root = state
            .events
            .values()
            .filter(|pending| pending.registration_public_id == event.registration_public_id)
            .fold((0_usize, 0_usize), |(count, bytes), pending| {
                (count + 1, bytes + pending.path.as_os_str().len())
            });
        let total_bytes: usize = state
            .events
            .values()
            .map(|pending| pending.path.as_os_str().len())
            .sum();
        if state.events.contains_key(&key)
            || (state.events.len() < self.capacity
                && per_root.0 < 256
                && per_root.1 + path_bytes <= 2 * 1024 * 1024
                && total_bytes + path_bytes <= 8 * 1024 * 1024)
        {
            state.events.insert(key, event);
        } else {
            state
                .overflowed_profiles
                .insert(event.registration_public_id);
        }
        drop(state);
        Ok(())
    }

    fn record_uncertainty(&self, id: Uuid) -> Result<(), MediaWatcherError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| MediaWatcherError::EventBuffer(error.to_string()))?;
        state.uncertain_profiles.insert(id);
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
            uncertain_profiles: std::mem::take(&mut state.uncertain_profiles),
        })
    }
}

/// Injectable filesystem watcher boundary; callers resolve authority and roots.
pub(crate) trait MediaWatcher: Send + Sync {
    fn synchronize(&mut self, roots: &BTreeMap<Uuid, PathBuf>) -> Vec<MediaWatcherError>;
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

    fn register(&mut self, profile_id: Uuid, source_root: &Path) -> Result<(), MediaWatcherError> {
        let source_root = source_root.to_path_buf();
        if !source_root.is_dir() {
            return Err(MediaWatcherError::SourceRootUnavailable(source_root));
        }

        let sender = self.events.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                forward_watch_result(profile_id, &sender, result);
            },
            Config::default().with_follow_symlinks(false),
        )
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

pub(crate) fn forward_watch_result(
    profile_id: Uuid,
    events: &MediaWatchEventBuffer,
    result: notify::Result<Event>,
) {
    match result {
        Ok(event)
            if event.need_rescan()
                || (event_can_change_media(event.kind) && event.paths.is_empty()) =>
        {
            record_watch_uncertainty(profile_id, events);
        }
        Ok(event) if event_can_change_media(event.kind) => {
            for path in event.paths {
                if let Err(error) = events.record(MediaWatchEvent {
                    registration_public_id: profile_id,
                    path,
                }) {
                    warn!(error = %error, "media watcher event buffer failed");
                }
            }
        }
        Ok(_) => {}
        Err(error) => {
            record_watch_uncertainty(profile_id, events);
            warn!(
                registration_public_id = %profile_id,
                error = %error,
                "media filesystem watcher event failed"
            );
        }
    }
}

fn record_watch_uncertainty(id: Uuid, events: &MediaWatchEventBuffer) {
    if let Err(error) = events.record_uncertainty(id) {
        warn!(error = %error, "media watcher uncertainty buffer failed");
    }
}

impl MediaWatcher for NotifyMediaWatcher {
    fn synchronize(&mut self, roots: &BTreeMap<Uuid, PathBuf>) -> Vec<MediaWatcherError> {
        self.registrations.retain(|profile_id, registration| {
            roots
                .get(profile_id)
                .is_some_and(|root| root == &registration.source_root)
        });

        let mut errors = Vec::new();
        for (profile_id, root) in roots {
            if !self.registrations.contains_key(profile_id)
                && let Err(error) = self.register(*profile_id, root)
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
