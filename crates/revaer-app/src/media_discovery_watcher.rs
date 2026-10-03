//! Filesystem event adapter for media discovery.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
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
        let key = (event.registration_public_id, event.path.clone());
        if state.events.contains_key(&key) || state.events.len() < self.capacity {
            state.events.insert(key, event);
        } else {
            state
                .overflowed_profiles
                .insert(event.registration_public_id);
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
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            forward_watch_result(profile_id, &sender, result);
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

fn forward_watch_result(
    profile_id: Uuid,
    events: &MediaWatchEventBuffer,
    result: notify::Result<Event>,
) {
    match result {
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
            warn!(
                registration_public_id = %profile_id,
                error = %error,
                "media filesystem watcher event failed"
            );
        }
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

#[cfg(test)]
mod tests {
    use super::{
        MediaWatchEvent, MediaWatchEventBuffer, MediaWatcher, NotifyMediaWatcher,
        event_can_change_media, forward_watch_result,
    };
    use notify::event::{CreateKind, ModifyKind, RemoveKind};
    use notify::{Event, EventKind};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn watcher_accepts_media_change_events() {
        assert!(event_can_change_media(EventKind::Create(CreateKind::File)));
        assert!(event_can_change_media(EventKind::Modify(ModifyKind::Any)));
        assert!(event_can_change_media(EventKind::Remove(RemoveKind::File)));
        assert!(event_can_change_media(EventKind::Any));
        assert!(!event_can_change_media(EventKind::Other));
    }

    #[test]
    fn native_watcher_registers_recursive_source_root() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let profile_id = Uuid::new_v4();
        let events = Arc::new(MediaWatchEventBuffer::new(32));
        let mut watcher = NotifyMediaWatcher::new(events);
        let roots = BTreeMap::from([(profile_id, root.path().to_path_buf())]);
        let errors = watcher.synchronize(&roots);
        if let Some(error) = errors.into_iter().next() {
            return Err(error.into());
        }
        assert!(watcher.registrations.contains_key(&profile_id));
        assert!(watcher.synchronize(&BTreeMap::new()).is_empty());
        assert!(watcher.registrations.is_empty());
        Ok(())
    }

    #[test]
    fn native_watcher_delivers_nested_file_creation() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let nested = root.path().join("nested");
        std::fs::create_dir(&nested)?;
        let id = Uuid::new_v4();
        let events = Arc::new(MediaWatchEventBuffer::new(32));
        let mut watcher = NotifyMediaWatcher::new(Arc::clone(&events));
        let errors = watcher.synchronize(&BTreeMap::from([(id, root.path().to_path_buf())]));
        if let Some(error) = errors.into_iter().next() {
            return Err(error.into());
        }
        let file = nested.join("native-event.txt");
        std::fs::write(&file, b"owned event fixture")?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let batch = events.drain()?;
            if batch.events.iter().any(|event| {
                event.registration_public_id == id
                    && event.path.ends_with("nested/native-event.txt")
            }) {
                return Ok(());
            }
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "native recursive event was not delivered"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn watcher_callback_forwards_recursive_media_file_changes() -> anyhow::Result<()> {
        let profile_id = Uuid::new_v4();
        let media_path = PathBuf::from("nested/movie.webm");
        let events = MediaWatchEventBuffer::new(4);
        let event = Event::new(EventKind::Create(CreateKind::File)).add_path(media_path.clone());

        forward_watch_result(profile_id, &events, Ok(event));

        let drained = events.drain()?;
        assert_eq!(
            drained.events,
            vec![MediaWatchEvent {
                registration_public_id: profile_id,
                path: media_path,
            }]
        );
        Ok(())
    }

    #[test]
    fn event_buffer_coalesces_and_bounds_a_paused_consumer() -> anyhow::Result<()> {
        let profile_id = Uuid::new_v4();
        let events = MediaWatchEventBuffer::new(4);
        for index in 0..1_024 {
            events.record(MediaWatchEvent {
                registration_public_id: profile_id,
                path: PathBuf::from(format!("movie-{}.mkv", index % 8)),
            })?;
        }
        let drained = events.drain()?;
        assert_eq!(drained.events.len(), 4);
        assert_eq!(drained.overflowed_profiles, BTreeSet::from([profile_id]));
        Ok(())
    }
}
