//! API application state, health tracking, and helpers.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use revaer_config::ApiKeyRateLimit;
use revaer_events::{Event as CoreEvent, EventBus, TorrentState};
use revaer_telemetry::Metrics;
use revaer_torrent_core::TorrentStatus;
use serde_json::Value;
use systemstat::{Platform, System};
use tracing::{error, warn};
use uuid::Uuid;

use crate::TorrentHandles;
use crate::app::indexers::IndexerFacade;
use crate::app::media::MediaFacade;
#[cfg(test)]
use crate::app::media::noop_media;
use crate::config::ConfigFacade;
use crate::http::rate_limit::{RateLimitError, RateLimitSnapshot, RateLimiter};
use crate::http::torrents::TorrentMetadata;
use crate::models::DashboardResponse;

pub(crate) struct ApiState {
    pub(crate) config: Arc<dyn ConfigFacade>,
    pub(crate) indexers: Arc<dyn IndexerFacade>,
    pub(crate) media: Arc<dyn MediaFacade>,
    pub(crate) setup_token_ttl: Duration,
    pub(crate) telemetry: Metrics,
    pub(crate) openapi_document: Arc<Value>,
    pub(crate) events: EventBus,
    health_status: Mutex<Vec<String>>,
    rate_limiters: Mutex<HashMap<String, RateLimiter>>,
    torrent_metadata: Mutex<HashMap<Uuid, TorrentMetadata>>,
    pub(crate) torrent: Option<TorrentHandles>,
    dashboard_disk_usage: fn(&Path) -> std::io::Result<(u32, u32)>,
    #[cfg(feature = "compat-qb")]
    compat_sessions: Mutex<HashMap<String, CompatSession>>,
}

#[cfg(feature = "compat-qb")]
#[derive(Clone)]
pub(crate) struct CompatSession {
    pub(crate) expires_at: Instant,
}

#[cfg(feature = "compat-qb")]
pub(crate) const COMPAT_SESSION_TTL: Duration = Duration::from_mins(30);
const DASHBOARD_TORRENTS_COMPONENT: &str = "dashboard_torrents";
const DASHBOARD_DISK_COMPONENT: &str = "dashboard_disk";

impl ApiState {
    #[cfg(test)]
    pub(crate) fn new(
        config: Arc<dyn ConfigFacade>,
        indexers: Arc<dyn IndexerFacade>,
        telemetry: Metrics,
        openapi_document: Arc<Value>,
        events: EventBus,
        torrent: Option<TorrentHandles>,
    ) -> Self {
        Self::new_with_media(
            config,
            indexers,
            noop_media(),
            telemetry,
            openapi_document,
            events,
            torrent,
        )
    }

    pub(crate) fn new_with_media(
        config: Arc<dyn ConfigFacade>,
        indexers: Arc<dyn IndexerFacade>,
        media: Arc<dyn MediaFacade>,
        telemetry: Metrics,
        openapi_document: Arc<Value>,
        events: EventBus,
        torrent: Option<TorrentHandles>,
    ) -> Self {
        Self {
            config,
            indexers,
            media,
            setup_token_ttl: Duration::from_mins(15),
            telemetry,
            openapi_document,
            events,
            health_status: Mutex::new(Vec::new()),
            rate_limiters: Mutex::new(HashMap::new()),
            torrent_metadata: Mutex::new(HashMap::new()),
            torrent,
            dashboard_disk_usage: dashboard_disk_usage_gb,
            #[cfg(feature = "compat-qb")]
            compat_sessions: Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_dashboard_disk_usage(
        mut self,
        dashboard_disk_usage: fn(&Path) -> std::io::Result<(u32, u32)>,
    ) -> Self {
        self.dashboard_disk_usage = dashboard_disk_usage;
        self
    }

    pub(crate) fn add_degraded_component(&self, component: &str) -> bool {
        let mut guard = Self::lock_guard(&self.health_status, "health_status");
        if guard.iter().any(|entry| entry == component) {
            return false;
        }
        guard.push(component.to_string());
        guard.sort();
        guard.dedup();
        let snapshot = guard.clone();
        drop(guard);
        self.publish_event(CoreEvent::HealthChanged { degraded: snapshot });
        true
    }

    pub(crate) fn remove_degraded_component(&self, component: &str) -> bool {
        let mut guard = Self::lock_guard(&self.health_status, "health_status");
        let previous = guard.len();
        guard.retain(|entry| entry != component);
        if guard.len() == previous {
            return false;
        }
        let snapshot = guard.clone();
        drop(guard);
        self.publish_event(CoreEvent::HealthChanged { degraded: snapshot });
        true
    }

    pub(crate) fn publish_event(&self, event: CoreEvent) {
        if let Err(error) = self.events.publish(event) {
            warn!(
                event_id = error.event_id(),
                event_kind = error.event_kind(),
                error = %error,
                "failed to publish event"
            );
        }
    }

    pub(crate) fn record_torrent_metrics(&self, statuses: &[TorrentStatus]) {
        let active = i64::try_from(statuses.len()).unwrap_or(i64::MAX);
        let queued = i64::try_from(
            statuses
                .iter()
                .filter(|status| matches!(status.state, TorrentState::Queued))
                .count(),
        )
        .unwrap_or(i64::MAX);
        self.telemetry.set_active_torrents(active);
        self.telemetry.set_queue_depth(queued);
    }

    pub(crate) async fn update_torrent_metrics(&self) {
        if let Some(handles) = &self.torrent {
            match handles.inspector().list().await {
                Ok(statuses) => {
                    self.record_torrent_metrics(&statuses);
                }
                Err(err) => {
                    warn!(error = %err, "failed to refresh torrent metrics");
                }
            }
        } else {
            self.record_torrent_metrics(&[]);
        }
    }

    pub(crate) async fn dashboard_snapshot(&self, library_root: &Path) -> DashboardResponse {
        let statuses = self.dashboard_statuses().await;
        let (download_bps, upload_bps, active, paused, completed) =
            aggregate_dashboard_counts(&statuses);
        let (disk_total_gb, disk_used_gb) = match (self.dashboard_disk_usage)(library_root) {
            Ok(snapshot) => {
                self.remove_degraded_component(DASHBOARD_DISK_COMPONENT);
                snapshot
            }
            Err(err) => {
                self.add_degraded_component(DASHBOARD_DISK_COMPONENT);
                warn!(
                    error = %err,
                    library_root = %library_root.display(),
                    "failed to read dashboard disk usage"
                );
                (0, 0)
            }
        };

        DashboardResponse {
            download_bps,
            upload_bps,
            active,
            paused,
            completed,
            disk_total_gb,
            disk_used_gb,
        }
    }

    async fn dashboard_statuses(&self) -> Vec<TorrentStatus> {
        if let Some(handles) = &self.torrent {
            match handles.inspector().list().await {
                Ok(statuses) => {
                    self.remove_degraded_component(DASHBOARD_TORRENTS_COMPONENT);
                    statuses
                }
                Err(err) => {
                    self.add_degraded_component(DASHBOARD_TORRENTS_COMPONENT);
                    warn!(error = %err, "failed to list torrents for dashboard snapshot");
                    Vec::new()
                }
            }
        } else {
            self.remove_degraded_component(DASHBOARD_TORRENTS_COMPONENT);
            Vec::new()
        }
    }

    pub(crate) fn current_health_degraded(&self) -> Vec<String> {
        Self::lock_guard(&self.health_status, "health_status").clone()
    }

    pub(crate) fn enforce_rate_limit(
        &self,
        key_id: &str,
        limit: Option<&ApiKeyRateLimit>,
    ) -> Result<Option<RateLimitSnapshot>, RateLimitError> {
        limit.map_or_else(
            || {
                if self.add_degraded_component("api_rate_limit_guard") {
                    self.telemetry.inc_guardrail_violation();
                    warn!("api key guard rail triggered: missing or unlimited rate limit");
                }
                Ok(None)
            },
            |limit| {
                self.remove_degraded_component("api_rate_limit_guard");
                let mut guard = Self::lock_guard(&self.rate_limiters, "rate_limiters");
                let limiter = guard
                    .entry(key_id.to_string())
                    .or_insert_with(|| RateLimiter::new(limit.clone()));
                let now = Instant::now();
                let status = limiter.evaluate(limit, now);
                drop(guard);
                if status.allowed {
                    Ok(Some(RateLimitSnapshot {
                        limit: limit.burst,
                        remaining: status.remaining,
                    }))
                } else {
                    self.telemetry.inc_rate_limit_throttled();
                    warn!(api_key = %key_id, "API key rate limit exceeded");
                    Err(RateLimitError {
                        limit: limit.burst,
                        retry_after: status.retry_after,
                    })
                }
            },
        )
    }

    pub(crate) fn set_metadata(&self, id: Uuid, metadata: TorrentMetadata) {
        let mut guard = Self::lock_guard(&self.torrent_metadata, "torrent_metadata");
        guard.insert(id, metadata);
    }

    pub(crate) fn update_metadata(&self, id: &Uuid, update: impl FnOnce(&mut TorrentMetadata)) {
        update(
            Self::lock_guard(&self.torrent_metadata, "torrent_metadata")
                .entry(*id)
                .or_default(),
        );
    }

    pub(crate) fn get_metadata(&self, id: &Uuid) -> TorrentMetadata {
        Self::lock_guard(&self.torrent_metadata, "torrent_metadata")
            .get(id)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn remove_metadata(&self, id: &Uuid) {
        let mut guard = Self::lock_guard(&self.torrent_metadata, "torrent_metadata");
        guard.remove(id);
    }

    #[cfg(feature = "compat-qb")]
    pub(crate) fn issue_qb_session(&self) -> String {
        let session_id = uuid::Uuid::new_v4().simple().to_string();
        let mut guard = Self::lock_guard(&self.compat_sessions, "compat_sessions");
        guard.insert(
            session_id.clone(),
            CompatSession {
                expires_at: Instant::now() + COMPAT_SESSION_TTL,
            },
        );
        session_id
    }

    #[cfg(feature = "compat-qb")]
    pub(crate) fn validate_qb_session(&self, session_id: &str) -> bool {
        let mut guard = Self::lock_guard(&self.compat_sessions, "compat_sessions");
        if let Some(session) = guard.get(session_id)
            && session.expires_at > Instant::now()
        {
            return true;
        }
        guard.remove(session_id);
        false
    }

    #[cfg(feature = "compat-qb")]
    pub(crate) fn revoke_qb_session(&self, session_id: &str) {
        let mut guard = Self::lock_guard(&self.compat_sessions, "compat_sessions");
        guard.remove(session_id);
    }

    fn lock_guard<'a, T>(mutex: &'a Mutex<T>, name: &'a str) -> MutexGuard<'a, T> {
        match mutex.lock() {
            Ok(guard) => guard,
            Err(err) => {
                error!(mutex = name, error = ?err, "mutex poisoned");
                err.into_inner()
            }
        }
    }
}

fn aggregate_dashboard_counts(statuses: &[TorrentStatus]) -> (u64, u64, u32, u32, u32) {
    let mut download_bps = 0_u64;
    let mut upload_bps = 0_u64;
    let mut active = 0_u32;
    let mut paused = 0_u32;
    let mut completed = 0_u32;

    for status in statuses {
        download_bps = download_bps.saturating_add(status.rates.download_bps);
        upload_bps = upload_bps.saturating_add(status.rates.upload_bps);
        match status.state {
            TorrentState::FetchingMetadata | TorrentState::Downloading | TorrentState::Seeding => {
                active = active.saturating_add(1);
            }
            TorrentState::Queued | TorrentState::Stopped => {
                paused = paused.saturating_add(1);
            }
            TorrentState::Completed => {
                completed = completed.saturating_add(1);
            }
            TorrentState::Failed { .. } => {}
        }
    }

    (download_bps, upload_bps, active, paused, completed)
}

fn dashboard_disk_usage_gb(path: &Path) -> std::io::Result<(u32, u32)> {
    let mount = System::new().mount_at(path)?;
    let total_bytes = mount.total.as_u64();
    let used_bytes = total_bytes.saturating_sub(mount.avail.as_u64());
    Ok((
        bytes_to_whole_gb(total_bytes),
        bytes_to_whole_gb(used_bytes),
    ))
}

fn bytes_to_whole_gb(bytes: u64) -> u32 {
    let gigabytes = bytes / 1_000_000_000;
    u32::try_from(gigabytes).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod state_tests;
