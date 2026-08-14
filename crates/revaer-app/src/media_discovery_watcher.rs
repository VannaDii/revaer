//! Filesystem event adapter for media discovery.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{Config, Event, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use revaer_data::media::profiles::MediaProfileRow;
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

const WATCH_POLL_FALLBACK_INTERVAL: Duration = Duration::from_secs(2);
const MAX_PATHS_PER_WATCH_EVENT: usize = 256;

/// One filesystem change associated with a configured media profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MediaWatchEvent {
    Path {
        media_profile_public_id: Uuid,
        path: PathBuf,
    },
    Rescan {
        media_profile_public_id: Uuid,
    },
}

/// Fixed-capacity, key-coalescing boundary between native callbacks and the runtime.
pub(crate) struct MediaWatchEventBuffer {
    capacity: usize,
    state: Mutex<MediaWatchEventBufferState>,
}

#[derive(Default)]
struct MediaWatchEventBufferState {
    paths: BTreeMap<(Uuid, PathBuf), MediaWatchEvent>,
    rescans: BTreeSet<Uuid>,
    overflowed: bool,
}

pub(crate) struct DrainedMediaWatchEvents {
    pub(crate) events: Vec<MediaWatchEvent>,
    pub(crate) overflowed: bool,
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
        match event {
            MediaWatchEvent::Path {
                media_profile_public_id,
                path,
            } => {
                if state.rescans.contains(&media_profile_public_id) {
                    return Ok(());
                }
                let key = (media_profile_public_id, path.clone());
                if state.paths.contains_key(&key)
                    || state.paths.len() + state.rescans.len() < self.capacity
                {
                    state.paths.insert(
                        key,
                        MediaWatchEvent::Path {
                            media_profile_public_id,
                            path,
                        },
                    );
                } else {
                    state.overflowed = true;
                }
            }
            MediaWatchEvent::Rescan {
                media_profile_public_id,
            } => {
                state
                    .paths
                    .retain(|(profile_id, _), _| *profile_id != media_profile_public_id);
                if state.rescans.contains(&media_profile_public_id)
                    || state.paths.len() + state.rescans.len() < self.capacity
                {
                    state.rescans.insert(media_profile_public_id);
                } else {
                    state.overflowed = true;
                }
            }
        }
        drop(state);
        Ok(())
    }

    pub(crate) fn drain(&self) -> Result<DrainedMediaWatchEvents, MediaWatcherError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| MediaWatcherError::EventBuffer(error.to_string()))?;
        let mut events = std::mem::take(&mut state.rescans)
            .into_iter()
            .map(|media_profile_public_id| MediaWatchEvent::Rescan {
                media_profile_public_id,
            })
            .collect::<Vec<_>>();
        events.extend(std::mem::take(&mut state.paths).into_values());
        Ok(DrainedMediaWatchEvents {
            events,
            overflowed: std::mem::take(&mut state.overflowed),
        })
    }
}

/// Injectable filesystem watcher boundary.
pub(crate) trait MediaWatcher: Send + Sync {
    fn synchronize(&mut self, profiles: &[MediaProfileRow]) -> Vec<MediaWatcherError>;
}

struct WatchRegistration {
    source_root: PathBuf,
    backend: WatchBackend,
}

enum WatchBackend {
    Native {
        _watcher: RecommendedWatcher,
        degraded: Arc<AtomicBool>,
    },
    Poll {
        _watcher: PollWatcher,
    },
}

impl WatchBackend {
    fn native_degraded(&self) -> bool {
        match self {
            Self::Native { degraded, .. } => degraded.load(Ordering::Acquire),
            Self::Poll { .. } => false,
        }
    }
}

/// Native watcher selected by `notify` for the current operating system.
pub(crate) struct NotifyMediaWatcher {
    events: Arc<MediaWatchEventBuffer>,
    registrations: BTreeMap<Uuid, WatchRegistration>,
    #[cfg(test)]
    force_native_unavailable: bool,
}

impl NotifyMediaWatcher {
    #[must_use]
    pub(crate) const fn new(events: Arc<MediaWatchEventBuffer>) -> Self {
        Self {
            events,
            registrations: BTreeMap::new(),
            #[cfg(test)]
            force_native_unavailable: false,
        }
    }

    fn register(&mut self, profile: &MediaProfileRow) -> Result<(), MediaWatcherError> {
        let source_root = PathBuf::from(&profile.source_root);
        if !source_root.is_dir() {
            return Err(MediaWatcherError::SourceRootUnavailable(source_root));
        }

        let profile_id = profile.media_profile_public_id;
        #[cfg(test)]
        let native_result = if self.force_native_unavailable {
            None
        } else {
            Some(self.create_native_backend(profile_id, &source_root))
        };
        #[cfg(not(test))]
        let native_result = Some(self.create_native_backend(profile_id, &source_root));

        let backend = match native_result {
            Some(Ok(backend)) => backend,
            Some(Err(error)) => {
                warn!(
                    media_profile_public_id = %profile_id,
                    error = %error,
                    "native media watcher unavailable; polling fallback activated"
                );
                self.create_poll_backend(profile_id, &source_root)?
            }
            None => self.create_poll_backend(profile_id, &source_root)?,
        };
        self.registrations.insert(
            profile_id,
            WatchRegistration {
                source_root,
                backend,
            },
        );
        Ok(())
    }

    fn create_native_backend(
        &self,
        profile_id: Uuid,
        source_root: &Path,
    ) -> Result<WatchBackend, MediaWatcherError> {
        let events = self.events.clone();
        let degraded = Arc::new(AtomicBool::new(false));
        let callback_degraded = Arc::clone(&degraded);
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            forward_native_result(profile_id, &events, &callback_degraded, result);
        })
        .map_err(|source| MediaWatcherError::Create {
            backend: "native",
            path: source_root.to_path_buf(),
            source,
        })?;
        watcher
            .watch(source_root, RecursiveMode::Recursive)
            .map_err(|source| MediaWatcherError::Watch {
                backend: "native",
                path: source_root.to_path_buf(),
                source,
            })?;
        Ok(WatchBackend::Native {
            _watcher: watcher,
            degraded,
        })
    }

    fn create_poll_backend(
        &self,
        profile_id: Uuid,
        source_root: &Path,
    ) -> Result<WatchBackend, MediaWatcherError> {
        let events = self.events.clone();
        let error_reported = Arc::new(AtomicBool::new(false));
        let callback_error_reported = Arc::clone(&error_reported);
        let config = Config::default().with_poll_interval(WATCH_POLL_FALLBACK_INTERVAL);
        let mut watcher = PollWatcher::new(
            move |result: notify::Result<Event>| {
                forward_poll_result(profile_id, &events, &callback_error_reported, result);
            },
            config,
        )
        .map_err(|source| MediaWatcherError::Create {
            backend: "poll",
            path: source_root.to_path_buf(),
            source,
        })?;
        watcher
            .watch(source_root, RecursiveMode::Recursive)
            .map_err(|source| MediaWatcherError::Watch {
                backend: "poll",
                path: source_root.to_path_buf(),
                source,
            })?;
        Ok(WatchBackend::Poll { _watcher: watcher })
    }

    fn activate_poll_fallback(
        &mut self,
        profile: &MediaProfileRow,
    ) -> Result<(), MediaWatcherError> {
        let source_root = PathBuf::from(&profile.source_root);
        let profile_id = profile.media_profile_public_id;
        let backend = self.create_poll_backend(profile_id, &source_root)?;
        warn!(
            media_profile_public_id = %profile_id,
            "native media watcher degraded; polling fallback activated"
        );
        self.registrations.insert(
            profile_id,
            WatchRegistration {
                source_root,
                backend,
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
            let native_degraded = self
                .registrations
                .get(&profile.media_profile_public_id)
                .is_some_and(|registration| registration.backend.native_degraded());
            let result = if native_degraded {
                self.activate_poll_fallback(profile)
            } else if !self
                .registrations
                .contains_key(&profile.media_profile_public_id)
            {
                self.register(profile)
            } else {
                Ok(())
            };
            if let Err(error) = result {
                errors.push(error);
            }
        }
        errors
    }
}

fn event_disposition(event: &Event) -> EventDisposition {
    if event.need_rescan() {
        return EventDisposition::Rescan;
    }
    match event.kind {
        EventKind::Access(notify::event::AccessKind::Close(notify::event::AccessMode::Write))
        | EventKind::Create(_)
        | EventKind::Modify(_)
            if event.paths.is_empty() || event.paths.len() > MAX_PATHS_PER_WATCH_EVENT =>
        {
            EventDisposition::Rescan
        }
        EventKind::Access(notify::event::AccessKind::Close(notify::event::AccessMode::Write))
        | EventKind::Create(_)
        | EventKind::Modify(_) => EventDisposition::Paths,
        EventKind::Access(_) => EventDisposition::Ignore,
        EventKind::Any | EventKind::Other | EventKind::Remove(_) => EventDisposition::Rescan,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventDisposition {
    Ignore,
    Paths,
    Rescan,
}

fn forward_native_result(
    profile_id: Uuid,
    events: &MediaWatchEventBuffer,
    degraded: &AtomicBool,
    result: notify::Result<Event>,
) {
    match result {
        Ok(event) => forward_notify_event(profile_id, events, event),
        Err(error) => {
            if degraded.swap(true, Ordering::AcqRel) {
                return;
            }
            warn!(
                media_profile_public_id = %profile_id,
                error = %error,
                "native media filesystem watcher degraded"
            );
            send_rescan(profile_id, events);
        }
    }
}

fn forward_poll_result(
    profile_id: Uuid,
    events: &MediaWatchEventBuffer,
    error_reported: &AtomicBool,
    result: notify::Result<Event>,
) {
    match result {
        Ok(event) => {
            error_reported.store(false, Ordering::Release);
            forward_notify_event(profile_id, events, event);
        }
        Err(error) => {
            if error_reported.swap(true, Ordering::AcqRel) {
                return;
            }
            warn!(
                media_profile_public_id = %profile_id,
                error = %error,
                "polling media filesystem watcher event failed"
            );
            send_rescan(profile_id, events);
        }
    }
}

fn forward_notify_event(profile_id: Uuid, events: &MediaWatchEventBuffer, event: Event) {
    match event_disposition(&event) {
        EventDisposition::Ignore => {}
        EventDisposition::Rescan => send_rescan(profile_id, events),
        EventDisposition::Paths => {
            for path in event.paths {
                if let Err(error) = events.record(MediaWatchEvent::Path {
                    media_profile_public_id: profile_id,
                    path,
                }) {
                    warn!(error = %error, "media watcher event buffer failed");
                    return;
                }
            }
        }
    }
}

fn send_rescan(profile_id: Uuid, events: &MediaWatchEventBuffer) {
    if let Err(error) = events.record(MediaWatchEvent::Rescan {
        media_profile_public_id: profile_id,
    }) {
        warn!(error = %error, "media watcher rescan buffer failed");
    }
}

/// Filesystem watcher setup failure.
#[derive(Debug, Error)]
pub(crate) enum MediaWatcherError {
    #[error("media watcher source root is unavailable: {0}")]
    SourceRootUnavailable(PathBuf),
    #[error("{backend} media watcher creation failed for {path}: {source}")]
    Create {
        backend: &'static str,
        path: PathBuf,
        source: notify::Error,
    },
    #[error("{backend} media watcher registration failed for {path}: {source}")]
    Watch {
        backend: &'static str,
        path: PathBuf,
        source: notify::Error,
    },
    #[error("media watcher event buffer failed: {0}")]
    EventBuffer(String),
}

#[cfg(test)]
mod tests {
    use super::{
        EventDisposition, MAX_PATHS_PER_WATCH_EVENT, MediaWatchEvent, MediaWatchEventBuffer,
        MediaWatcher, NotifyMediaWatcher, WatchBackend, event_disposition, forward_native_result,
        forward_notify_event,
    };
    use chrono::Utc;
    use notify::event::{
        AccessKind, AccessMode, CreateKind, Flag, ModifyKind, RemoveKind, RenameMode,
    };
    use notify::{Event, EventKind};
    use revaer_data::media::profiles::MediaProfileRow;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use tokio::time::{sleep, timeout};
    use uuid::Uuid;

    fn media_profile(profile_id: Uuid, source_root: &std::path::Path) -> MediaProfileRow {
        MediaProfileRow {
            media_profile_public_id: profile_id,
            profile_key: "watch-test".to_string(),
            source_root: source_root.to_string_lossy().into_owned(),
            output_root: source_root.join("output").to_string_lossy().into_owned(),
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
        }
    }

    #[test]
    fn watcher_ignores_read_and_open_access_noise() {
        for kind in [
            EventKind::Access(AccessKind::Open(AccessMode::Read)),
            EventKind::Access(AccessKind::Open(AccessMode::Write)),
            EventKind::Access(AccessKind::Close(AccessMode::Read)),
            EventKind::Access(AccessKind::Any),
        ] {
            let mut event = Event::new(kind);
            for index in 0..=MAX_PATHS_PER_WATCH_EVENT {
                event.paths.push(PathBuf::from(format!("noise-{index}")));
            }
            assert_eq!(event_disposition(&event), EventDisposition::Ignore);
        }
    }

    #[test]
    fn watcher_bounds_precise_paths_and_rescans_uncertain_events() {
        let create =
            Event::new(EventKind::Create(CreateKind::File)).add_path(PathBuf::from("movie.mkv"));
        let modify =
            Event::new(EventKind::Modify(ModifyKind::Any)).add_path(PathBuf::from("movie.mkv"));
        let rename = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path(PathBuf::from("movie.partial"))
            .add_path(PathBuf::from("movie.mkv"));
        let write_close = Event::new(EventKind::Access(AccessKind::Close(AccessMode::Write)))
            .add_path(PathBuf::from("movie.mkv"));
        for event in [create, modify, rename, write_close] {
            assert_eq!(event_disposition(&event), EventDisposition::Paths);
        }

        for event in [
            Event::new(EventKind::Any),
            Event::new(EventKind::Other),
            Event::new(EventKind::Remove(RemoveKind::File)).add_path(PathBuf::from("movie.mkv")),
            Event::new(EventKind::Modify(ModifyKind::Any)),
            Event::new(EventKind::Modify(ModifyKind::Any)).set_flag(Flag::Rescan),
        ] {
            assert_eq!(event_disposition(&event), EventDisposition::Rescan);
        }

        let mut oversized = Event::new(EventKind::Modify(ModifyKind::Any));
        for index in 0..=MAX_PATHS_PER_WATCH_EVENT {
            oversized
                .paths
                .push(PathBuf::from(format!("movie-{index}.mkv")));
        }
        assert_eq!(event_disposition(&oversized), EventDisposition::Rescan);
    }

    #[test]
    fn uncertain_event_emits_one_profile_rescan() -> anyhow::Result<()> {
        let profile_id = Uuid::new_v4();
        let events = MediaWatchEventBuffer::new(4);
        let event = Event::new(EventKind::Other)
            .add_path(PathBuf::from("first.mkv"))
            .add_path(PathBuf::from("second.mkv"));

        forward_notify_event(profile_id, &events, event);

        let drained = events.drain()?;
        assert_eq!(
            drained.events,
            vec![MediaWatchEvent::Rescan {
                media_profile_public_id: profile_id
            }]
        );
        Ok(())
    }

    #[test]
    fn healthy_native_registration_does_not_activate_polling() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let profile_id = Uuid::new_v4();
        let profile = media_profile(profile_id, root.path());
        let events = Arc::new(MediaWatchEventBuffer::new(32));
        let mut watcher = NotifyMediaWatcher::new(events);

        let errors = watcher.synchronize(&[profile]);

        assert!(errors.is_empty());
        assert!(matches!(
            watcher
                .registrations
                .get(&profile_id)
                .map(|registration| &registration.backend),
            Some(WatchBackend::Native { .. })
        ));
        Ok(())
    }

    #[test]
    fn degraded_native_registration_activates_polling() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let profile_id = Uuid::new_v4();
        let profile = media_profile(profile_id, root.path());
        let events = Arc::new(MediaWatchEventBuffer::new(32));
        let mut watcher = NotifyMediaWatcher::new(events);
        assert!(
            watcher
                .synchronize(std::slice::from_ref(&profile))
                .is_empty()
        );
        let registration = watcher
            .registrations
            .get(&profile_id)
            .ok_or_else(|| anyhow::anyhow!("native registration missing"))?;
        let WatchBackend::Native { degraded, .. } = &registration.backend else {
            return Err(anyhow::anyhow!("expected native watcher registration"));
        };
        degraded.store(true, Ordering::Release);

        let errors = watcher.synchronize(&[profile]);

        assert!(errors.is_empty());
        assert!(matches!(
            watcher
                .registrations
                .get(&profile_id)
                .map(|registration| &registration.backend),
            Some(WatchBackend::Poll { .. })
        ));
        Ok(())
    }

    #[test]
    fn native_callback_forwards_precise_media_file_changes() -> anyhow::Result<()> {
        let profile_id = Uuid::new_v4();
        let events = Arc::new(MediaWatchEventBuffer::new(32));
        let degraded = std::sync::atomic::AtomicBool::new(false);
        let media_path = PathBuf::from("nested/movie.webm");
        let event = Event::new(EventKind::Modify(ModifyKind::Data(
            notify::event::DataChange::Content,
        )))
        .add_path(media_path.clone());

        forward_native_result(profile_id, &events, &degraded, Ok(event));

        let drained = events.drain()?;

        assert_eq!(
            drained.events,
            vec![MediaWatchEvent::Path {
                media_profile_public_id: profile_id,
                path: media_path,
            }]
        );
        assert!(!degraded.load(Ordering::Acquire));
        Ok(())
    }

    #[tokio::test]
    async fn polling_fallback_reports_recursive_media_file_changes() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let nested = root.path().join("nested");
        fs::create_dir_all(&nested)?;
        let profile_id = Uuid::new_v4();
        let profile = media_profile(profile_id, root.path());
        let events = Arc::new(MediaWatchEventBuffer::new(32));
        let mut watcher = NotifyMediaWatcher::new(events.clone());
        watcher.force_native_unavailable = true;
        let errors = watcher.synchronize(&[profile]);
        if let Some(error) = errors.into_iter().next() {
            return Err(error.into());
        }
        assert!(matches!(
            watcher
                .registrations
                .get(&profile_id)
                .map(|registration| &registration.backend),
            Some(WatchBackend::Poll { .. })
        ));
        sleep(Duration::from_millis(100)).await;

        let media_path = nested.join("movie.webm");
        fs::write(&media_path, b"first")?;
        let canonical_media_path = media_path.canonicalize()?;
        let observed = timeout(Duration::from_secs(5), async {
            loop {
                let drained = events.drain().ok()?;
                if let Some(event) = drained.events.into_iter().find(|event| match event {
                    MediaWatchEvent::Path { path, .. } => {
                        path.canonicalize().ok().as_ref() == Some(&canonical_media_path)
                    }
                    MediaWatchEvent::Rescan { .. } => false,
                }) {
                    return Some(event);
                }
                sleep(Duration::from_millis(25)).await;
            }
        })
        .await?
        .ok_or_else(|| anyhow::anyhow!("poll watcher event buffer closed"))?;

        assert!(matches!(
            observed,
            MediaWatchEvent::Path {
                media_profile_public_id,
                ..
            } if media_profile_public_id == profile_id
        ));
        Ok(())
    }

    #[test]
    fn event_buffer_coalesces_and_bounds_a_paused_consumer() -> anyhow::Result<()> {
        let profile_id = Uuid::new_v4();
        let events = MediaWatchEventBuffer::new(4);
        for index in 0..1_024 {
            events.record(MediaWatchEvent::Path {
                media_profile_public_id: profile_id,
                path: PathBuf::from(format!("movie-{}.mkv", index % 8)),
            })?;
        }
        let drained = events.drain()?;
        assert_eq!(drained.events.len(), 4);
        assert!(drained.overflowed);
        Ok(())
    }

    #[test]
    fn event_buffer_rescan_supersedes_profile_paths() -> anyhow::Result<()> {
        let profile_id = Uuid::new_v4();
        let events = MediaWatchEventBuffer::new(4);
        events.record(MediaWatchEvent::Path {
            media_profile_public_id: profile_id,
            path: PathBuf::from("movie.mkv"),
        })?;
        events.record(MediaWatchEvent::Rescan {
            media_profile_public_id: profile_id,
        })?;
        events.record(MediaWatchEvent::Path {
            media_profile_public_id: profile_id,
            path: PathBuf::from("other.mkv"),
        })?;

        let drained = events.drain()?;
        assert_eq!(
            drained.events,
            vec![MediaWatchEvent::Rescan {
                media_profile_public_id: profile_id
            }]
        );
        Ok(())
    }
}
