//! Bounded association registrations and coalesced rescan hints.

use std::collections::BTreeMap;
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::Path;
use std::time::{Duration, Instant};

use revaer_data::media::associations::{AssociationRow, read_association};
use revaer_data::media::rescan::{RescanFence, request_rescan};
use revaer_runtime::media::MediaStore;
use uuid::Uuid;

use super::NativeDiscoveryError;
use crate::media::source::{AssociationSource, SourceGeneration};
use crate::media_discovery_scan::ScanError;
use crate::media_discovery_watcher::{DrainedMediaWatchEvents, MediaWatcher};

const MAX_WATCHED_ASSOCIATIONS: usize = 128;
const QUIET_INTERVAL: Duration = Duration::from_secs(1);
const MAX_RESIDENCE: Duration = Duration::from_secs(5);

#[derive(Default)]
pub(super) struct NativeWatches {
    registrations: BTreeMap<Uuid, Registration>,
}

struct Registration {
    row: AssociationRow,
    directory: OwnedFd,
    seen: bool,
    pending: Option<Pending>,
}

struct Pending {
    first: Instant,
    last: Instant,
    overflow: bool,
    urgent: bool,
}

impl Pending {
    fn due(&self, now: Instant) -> bool {
        self.overflow
            || self.urgent
            || now.duration_since(self.last) >= QUIET_INTERVAL
            || now.duration_since(self.first) >= MAX_RESIDENCE
    }
}

impl NativeWatches {
    pub(super) fn has_pending(&self, id: Uuid) -> bool {
        self.registrations
            .get(&id)
            .is_some_and(|registration| registration.pending.is_some())
    }

    pub(super) fn observe(
        &mut self,
        row: &AssociationRow,
        source: &dyn AssociationSource,
        watcher: &mut dyn MediaWatcher,
    ) -> Result<(), NativeDiscoveryError> {
        let id = row.media_discovery_association_public_id;
        if !row.binding_ready
            || row.active_version != Some(row.latest_version)
            || !row.modes.watcher_enabled
        {
            self.registrations.remove(&id);
            return self.synchronize(watcher);
        }
        if !self.registrations.contains_key(&id)
            && self.registrations.len() >= MAX_WATCHED_ASSOCIATIONS
        {
            return Err(NativeDiscoveryError::WatchRegistration(
                "watched association capacity exceeded".to_owned(),
            ));
        }
        let directory = source
            .watch_directory(&row.source_root_key, Path::new(&row.root_relative_path))
            .map_err(ScanError::from)?;
        let identity = rustix::fs::fstat(&directory)
            .map_err(|error| NativeDiscoveryError::WatchRegistration(error.to_string()))?;
        let retained = self
            .registrations
            .get(&id)
            .map(|registration| {
                rustix::fs::fstat(&registration.directory).map(|old| {
                    registration.row == *row
                        && old.st_dev == identity.st_dev
                        && old.st_ino == identity.st_ino
                })
            })
            .transpose()
            .map_err(|error| NativeDiscoveryError::WatchRegistration(error.to_string()))?;
        if retained == Some(true) {
            if let Some(registration) = self.registrations.get_mut(&id) {
                registration.seen = true;
            }
        } else {
            self.registrations.insert(
                id,
                Registration {
                    row: row.clone(),
                    directory,
                    seen: true,
                    pending: Some(Pending {
                        first: Instant::now(),
                        last: Instant::now(),
                        overflow: false,
                        urgent: true,
                    }),
                },
            );
        }
        self.synchronize(watcher)
    }

    pub(super) fn finish_page_cycle(
        &mut self,
        watcher: &mut dyn MediaWatcher,
    ) -> Result<(), NativeDiscoveryError> {
        self.registrations
            .retain(|_, registration| registration.seen);
        for registration in self.registrations.values_mut() {
            registration.seen = false;
        }
        self.synchronize(watcher)
    }

    fn synchronize(&mut self, watcher: &mut dyn MediaWatcher) -> Result<(), NativeDiscoveryError> {
        // Keep descriptors alive while notify watches these kernel-resolved hints.
        // Event paths are never used as authority for candidate reads/admission.
        let roots = self
            .registrations
            .iter()
            .map(|(id, registration)| {
                (
                    *id,
                    format!("/proc/self/fd/{}/.", registration.directory.as_raw_fd()).into(),
                )
            })
            .collect();
        let errors = watcher.synchronize(&roots);
        if errors.is_empty() {
            return Ok(());
        }
        for registration in self.registrations.values_mut() {
            Self::record(registration, Instant::now(), false, true);
        }
        Err(NativeDiscoveryError::WatchRegistration(
            errors
                .into_iter()
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        ))
    }

    const fn record(registration: &mut Registration, now: Instant, overflow: bool, urgent: bool) {
        match registration.pending.as_mut() {
            Some(pending) => {
                pending.last = now;
                pending.overflow |= overflow;
                pending.urgent |= urgent;
            }
            None => {
                registration.pending = Some(Pending {
                    first: now,
                    last: now,
                    overflow,
                    urgent,
                });
            }
        }
    }

    pub(super) async fn flush(
        &mut self,
        store: &MediaStore,
        generation: SourceGeneration,
        drained: DrainedMediaWatchEvents,
        watcher: &mut dyn MediaWatcher,
    ) -> Result<(), NativeDiscoveryError> {
        let now = Instant::now();
        for event in drained.events {
            if let Some(registration) = self.registrations.get_mut(&event.registration_public_id) {
                Self::record(registration, now, false, false);
            }
        }
        for id in drained.overflowed_profiles {
            if let Some(registration) = self.registrations.get_mut(&id) {
                Self::record(registration, now, true, true);
            }
        }
        for id in drained.uncertain_profiles {
            if let Some(registration) = self.registrations.get_mut(&id) {
                Self::record(registration, now, false, true);
            }
        }
        let due = self
            .registrations
            .iter()
            .filter(|(_, registration)| {
                registration
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.due(now))
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        let mut failures = Vec::new();
        for id in due {
            if let Err(error) = self.publish(store, generation, id).await {
                // Retain this request for retry, and still visit other due bindings.
                failures.push(error.to_string());
            }
        }
        if let Err(error) = self.synchronize(watcher) {
            failures.push(error.to_string());
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(NativeDiscoveryError::WatchRegistration(failures.join("; ")))
        }
    }

    async fn publish(
        &mut self,
        store: &MediaStore,
        generation: SourceGeneration,
        id: Uuid,
    ) -> Result<(), NativeDiscoveryError> {
        let Some(registration) = self.registrations.get(&id) else {
            return Ok(());
        };
        if read_association(store.pool(), id).await?.as_ref() != Some(&registration.row) {
            self.registrations.remove(&id);
            return Ok(());
        }
        let Some(pending) = registration.pending.as_ref() else {
            return Ok(());
        };
        request_rescan(
            store.pool(),
            &RescanFence {
                association: id,
                version: registration.row.latest_version,
                generation: generation.number,
                generation_sha256: generation.sha256,
                trigger: "watcher",
            },
            if pending.overflow {
                "overflow"
            } else {
                "watcher_uncertain"
            },
        )
        .await?;
        if let Some(registration) = self.registrations.get_mut(&id) {
            registration.pending = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_RESIDENCE, Pending, QUIET_INTERVAL};
    use std::time::{Duration, Instant};

    #[test]
    fn watcher_debounce_has_a_quiet_window_and_a_fixed_residence_bound() {
        let first = Instant::now();
        let mut pending = Pending {
            first,
            last: first,
            overflow: false,
            urgent: false,
        };
        assert!(!pending.due(first + Duration::from_millis(900)));
        assert!(pending.due(first + QUIET_INTERVAL));
        pending.last = first + Duration::from_millis(4900);
        assert!(!pending.due(first + Duration::from_millis(4950)));
        assert!(pending.due(first + MAX_RESIDENCE));
        pending.overflow = true;
        assert!(pending.due(pending.last));
    }
}
