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
    #[test]
    fn watcher_uncertainty_is_not_lost_without_a_path() -> anyhow::Result<()> {
        let id = Uuid::new_v4();
        let events = MediaWatchEventBuffer::new(4);
        for result in [
            Ok(Event::new(EventKind::Other).set_flag(notify::event::Flag::Rescan)),
            Ok(Event::new(EventKind::Any)),
            Err(notify::Error::generic("owned backend uncertainty")),
        ] {
            forward_watch_result(id, &events, result);
            let drained = events.drain()?;
            assert!(drained.events.is_empty());
            assert_eq!(drained.uncertain_profiles, BTreeSet::from([id]));
            assert!(drained.overflowed_profiles.is_empty());
        }
        Ok(())
    }

    #[test]
    fn event_buffer_enforces_root_record_and_byte_limits() -> anyhow::Result<()> {
        let id = Uuid::new_v4();
        let events = MediaWatchEventBuffer::new(1024);
        for index in 0..257 {
            events.record(MediaWatchEvent {
                registration_public_id: id,
                path: PathBuf::from(format!("movie-{index}.mkv")),
            })?;
        }
        let drained = events.drain()?;
        assert_eq!(drained.events.len(), 256);
        assert_eq!(drained.overflowed_profiles, BTreeSet::from([id]));
        events.record(MediaWatchEvent {
            registration_public_id: id,
            path: PathBuf::from("x".repeat(2 * 1024 * 1024 + 1)),
        })?;
        let drained = events.drain()?;
        assert!(drained.events.is_empty());
        assert_eq!(drained.overflowed_profiles, BTreeSet::from([id]));
        Ok(())
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn retained_descriptor_watcher_detects_nested_changes_without_following_symlinks()
    -> anyhow::Result<()> {
        use std::os::fd::AsRawFd;
        let root = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        std::fs::create_dir(root.path().join("nested"))?;
        std::os::unix::fs::symlink(outside.path(), root.path().join("alias"))?;
        let directory = std::fs::File::open(root.path())?;
        let id = Uuid::new_v4();
        let events = Arc::new(MediaWatchEventBuffer::new(32));
        let mut watcher = NotifyMediaWatcher::new(Arc::clone(&events));
        let errors = watcher.synchronize(&BTreeMap::from([(
            id,
            format!("/proc/self/fd/{}/.", directory.as_raw_fd()).into(),
        )]));
        anyhow::ensure!(
            errors.is_empty(),
            "descriptor registration failed: {errors:?}"
        );
        std::fs::write(
            outside.path().join("outside.mkv"),
            b"outside watcher fixture",
        )?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(events.drain()?.events.is_empty());
        std::fs::write(
            root.path().join("nested/inside.mkv"),
            b"inside watcher fixture",
        )?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let batch = events.drain()?;
            if batch.events.iter().any(|event| {
                event.registration_public_id == id && event.path.ends_with("nested/inside.mkv")
            }) {
                break;
            }
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "descriptor watch missed nested creation"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(watcher.synchronize(&BTreeMap::new()).is_empty());
        drop(directory);
        Ok(())
    }
}
