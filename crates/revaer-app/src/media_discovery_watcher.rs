//! Filesystem event adapter for media discovery.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::event::{AccessKind, AccessMode};
use notify::{Config, Event, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use revaer_data::media::profiles::MediaProfileRow;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;
use uuid::Uuid;

const WATCH_POLL_FALLBACK_INTERVAL: Duration = Duration::from_secs(2);

/// One filesystem event associated with a configured media profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaWatchEvent {
    pub(crate) media_profile_public_id: Uuid,
    pub(crate) path: PathBuf,
}

/// Injectable filesystem watcher boundary.
pub(crate) trait MediaWatcher: Send + Sync {
    fn synchronize(&mut self, profiles: &[MediaProfileRow]) -> Vec<MediaWatcherError>;
}

struct WatchRegistration {
    source_root: PathBuf,
    _native_watcher: Option<RecommendedWatcher>,
    _poll_watcher: PollWatcher,
}

/// Native watcher with a polling correctness backstop selected through `notify`.
pub(crate) struct NotifyMediaWatcher {
    events: UnboundedSender<MediaWatchEvent>,
    registrations: BTreeMap<Uuid, WatchRegistration>,
}

impl NotifyMediaWatcher {
    #[must_use]
    pub(crate) const fn new(events: UnboundedSender<MediaWatchEvent>) -> Self {
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
        let native_sender = self.events.clone();
        let native_watcher = match notify::recommended_watcher(
            move |result: notify::Result<Event>| {
                forward_notify_result(profile_id, &native_sender, result);
            },
        ) {
            Ok(mut watcher) => {
                if let Err(source) =
                    watcher.watch(Path::new(&source_root), RecursiveMode::Recursive)
                {
                    warn!(
                        media_profile_public_id = %profile_id,
                        path = %source_root.display(),
                        error = %source,
                        "media native filesystem watcher registration failed; poll fallback remains active"
                    );
                    None
                } else {
                    Some(watcher)
                }
            }
            Err(source) => {
                warn!(
                    media_profile_public_id = %profile_id,
                    path = %source_root.display(),
                    error = %source,
                    "media native filesystem watcher creation failed; poll fallback remains active"
                );
                None
            }
        };

        let poll_sender = self.events.clone();
        let poll_config = Config::default().with_poll_interval(WATCH_POLL_FALLBACK_INTERVAL);
        let mut poll_watcher = PollWatcher::new(
            move |result: notify::Result<Event>| {
                forward_notify_result(profile_id, &poll_sender, result);
            },
            poll_config,
        )
        .map_err(|source| MediaWatcherError::Create {
            path: source_root.clone(),
            source,
        })?;
        poll_watcher
            .watch(Path::new(&source_root), RecursiveMode::Recursive)
            .map_err(|source| MediaWatcherError::Watch {
                path: source_root.clone(),
                source,
            })?;
        self.registrations.insert(
            profile_id,
            WatchRegistration {
                source_root,
                _native_watcher: native_watcher,
                _poll_watcher: poll_watcher,
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
        EventKind::Any
            | EventKind::Other
            | EventKind::Create(_)
            | EventKind::Modify(_)
            | EventKind::Access(
                AccessKind::Any
                    | AccessKind::Other
                    | AccessKind::Open(AccessMode::Any | AccessMode::Other | AccessMode::Write)
                    | AccessKind::Close(AccessMode::Any | AccessMode::Other | AccessMode::Write),
            )
    )
}

fn forward_notify_result(
    profile_id: Uuid,
    sender: &UnboundedSender<MediaWatchEvent>,
    result: notify::Result<Event>,
) {
    match result {
        Ok(event) => forward_notify_event(profile_id, sender, event),
        Err(error) => {
            warn!(
                media_profile_public_id = %profile_id,
                error = %error,
                "media filesystem watcher event failed"
            );
        }
    }
}

fn forward_notify_event(profile_id: Uuid, sender: &UnboundedSender<MediaWatchEvent>, event: Event) {
    let Event { kind, paths, .. } = event;
    if !event_can_change_media(kind) {
        return;
    }

    for path in paths {
        if sender
            .send(MediaWatchEvent {
                media_profile_public_id: profile_id,
                path,
            })
            .is_err()
        {
            return;
        }
    }
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
}

#[cfg(test)]
mod tests {
    use super::{MediaWatcher, NotifyMediaWatcher, event_can_change_media};
    use chrono::Utc;
    use notify::EventKind;
    use notify::event::{AccessKind, AccessMode, CreateKind, ModifyKind, RemoveKind};
    use revaer_data::media::profiles::MediaProfileRow;
    use std::fs;
    use std::time::Duration;
    use tokio::sync::mpsc::unbounded_channel;
    use tokio::time::{sleep, timeout};
    use uuid::Uuid;

    #[test]
    fn watcher_accepts_media_change_events() {
        assert!(event_can_change_media(EventKind::Create(CreateKind::File)));
        assert!(event_can_change_media(EventKind::Modify(ModifyKind::Any)));
        assert!(event_can_change_media(EventKind::Access(
            AccessKind::Close(AccessMode::Write)
        )));
        assert!(event_can_change_media(EventKind::Access(AccessKind::Open(
            AccessMode::Write
        ))));
        assert!(event_can_change_media(EventKind::Access(AccessKind::Any)));
        assert!(event_can_change_media(EventKind::Any));
        assert!(event_can_change_media(EventKind::Other));
        assert!(!event_can_change_media(EventKind::Remove(RemoveKind::File)));
        assert!(!event_can_change_media(EventKind::Access(
            AccessKind::Close(AccessMode::Read)
        )));
        assert!(!event_can_change_media(EventKind::Access(
            AccessKind::Open(AccessMode::Read)
        )));
    }

    #[tokio::test]
    async fn watcher_reports_recursive_media_file_changes() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
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
        let (tx, mut rx) = unbounded_channel();
        let mut watcher = NotifyMediaWatcher::new(tx);
        let errors = watcher.synchronize(&[profile]);
        if let Some(error) = errors.into_iter().next() {
            return Err(error.into());
        }
        sleep(Duration::from_millis(250)).await;

        let nested = root.path().join("nested");
        fs::create_dir_all(&nested)?;
        let media_path = nested.join("movie.webm");
        fs::write(&media_path, b"first")?;
        let canonical_media_path = media_path.canonicalize()?;
        let observed = timeout(Duration::from_secs(5), async {
            loop {
                let event = rx.recv().await?;
                let Ok(observed_path) = event.path.canonicalize() else {
                    continue;
                };
                if observed_path == canonical_media_path
                    || (observed_path.is_dir() && canonical_media_path.starts_with(&observed_path))
                {
                    return Some(event);
                }
            }
        })
        .await?
        .ok_or_else(|| anyhow::anyhow!("native watcher event channel closed"))?;

        assert_eq!(observed.media_profile_public_id, profile_id);
        let observed_path = observed.path.canonicalize()?;
        assert!(
            observed_path == canonical_media_path
                || (observed_path.is_dir() && canonical_media_path.starts_with(observed_path))
        );
        Ok(())
    }
}
