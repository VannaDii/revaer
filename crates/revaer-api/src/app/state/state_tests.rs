use super::*;
use crate::app::indexers::test_indexers;
use crate::http::handlers::indexers::test_support::StubConfig;
use anyhow::Result;
use async_trait::async_trait;
use revaer_torrent_core::{
    AddTorrent, FileSelectionUpdate, PeerSnapshot, RemoveTorrent, TorrentRateLimit, TorrentResult,
    TorrentWorkflow,
};
use serde_json::json;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

#[derive(Default)]
struct RecordingWorkflow {
    statuses: RwLock<Vec<TorrentStatus>>,
    peers: RwLock<HashMap<Uuid, Vec<PeerSnapshot>>>,
}

impl RecordingWorkflow {
    fn with_status(status: TorrentStatus) -> Arc<Self> {
        Self {
            statuses: RwLock::new(vec![status]),
            peers: RwLock::new(HashMap::new()),
        }
        .into()
    }
}

#[async_trait]
impl TorrentWorkflow for RecordingWorkflow {
    async fn add_torrent(&self, _: AddTorrent) -> TorrentResult<()> {
        Ok(())
    }

    async fn remove_torrent(&self, _: Uuid, _: RemoveTorrent) -> TorrentResult<()> {
        Ok(())
    }

    async fn pause_torrent(&self, _: Uuid) -> TorrentResult<()> {
        Ok(())
    }

    async fn resume_torrent(&self, _: Uuid) -> TorrentResult<()> {
        Ok(())
    }

    async fn set_sequential(&self, _: Uuid, _: bool) -> TorrentResult<()> {
        Ok(())
    }

    async fn update_limits(&self, _: Option<Uuid>, _: TorrentRateLimit) -> TorrentResult<()> {
        Ok(())
    }

    async fn update_selection(&self, _: Uuid, _: FileSelectionUpdate) -> TorrentResult<()> {
        Ok(())
    }

    async fn update_trackers(
        &self,
        _: Uuid,
        _: revaer_torrent_core::model::TorrentTrackersUpdate,
    ) -> TorrentResult<()> {
        Ok(())
    }

    async fn update_web_seeds(
        &self,
        _: Uuid,
        _: revaer_torrent_core::model::TorrentWebSeedsUpdate,
    ) -> TorrentResult<()> {
        Ok(())
    }

    async fn reannounce(&self, _: Uuid) -> TorrentResult<()> {
        Ok(())
    }

    async fn recheck(&self, _: Uuid) -> TorrentResult<()> {
        Ok(())
    }

    async fn set_piece_deadline(
        &self,
        _: Uuid,
        _: revaer_torrent_core::model::PieceDeadline,
    ) -> TorrentResult<()> {
        Ok(())
    }
}

#[async_trait]
impl revaer_torrent_core::TorrentInspector for RecordingWorkflow {
    async fn list(&self) -> TorrentResult<Vec<TorrentStatus>> {
        Ok(self.statuses.read().await.clone())
    }

    async fn get(&self, _: Uuid) -> TorrentResult<Option<TorrentStatus>> {
        Ok(None)
    }

    async fn peers(&self, id: Uuid) -> TorrentResult<Vec<PeerSnapshot>> {
        Ok(self
            .peers
            .read()
            .await
            .get(&id)
            .cloned()
            .unwrap_or_default())
    }
}

#[test]
fn add_and_remove_degraded_components_emit_events() -> Result<()> {
    let events = EventBus::with_capacity(4);
    let metrics = Metrics::new()?;
    let state = ApiState::new(
        Arc::new(StubConfig),
        test_indexers(),
        metrics,
        Arc::new(json!({})),
        events.clone(),
        None,
    );
    let runtime = Runtime::new()?;
    let mut stream = events.subscribe(None);

    assert!(state.add_degraded_component("db"));
    assert!(!state.add_degraded_component("db"));

    let envelope = runtime
        .block_on(async { stream.next().await })
        .ok_or_else(|| anyhow::anyhow!("health event missing"))??;
    assert!(matches!(envelope.event, CoreEvent::HealthChanged { .. }));
    assert!(state.remove_degraded_component("db"));
    Ok(())
}

#[tokio::test]
async fn update_torrent_metrics_handles_stub_handles() -> Result<()> {
    let status = TorrentStatus {
        id: Uuid::new_v4(),
        name: Some("demo".into()),
        state: TorrentState::Completed,
        progress: revaer_torrent_core::TorrentProgress::default(),
        rates: revaer_torrent_core::TorrentRates::default(),
        files: None,
        library_path: None,
        download_dir: None,
        comment: None,
        source: None,
        private: None,
        sequential: false,
        added_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
        last_updated: chrono::Utc::now(),
    };
    let workflow = RecordingWorkflow::with_status(status);
    let handles = TorrentHandles::new(workflow.clone(), workflow);
    let state = ApiState::new(
        Arc::new(StubConfig),
        test_indexers(),
        Metrics::new()?,
        Arc::new(json!({})),
        EventBus::with_capacity(4),
        Some(handles),
    );

    state.update_torrent_metrics().await;
    Ok(())
}

#[test]
fn update_metadata_inserts_defaults() -> Result<()> {
    let id = Uuid::new_v4();
    let state = ApiState::new(
        Arc::new(StubConfig),
        test_indexers(),
        Metrics::new()?,
        Arc::new(json!({})),
        EventBus::with_capacity(4),
        None,
    );

    state.update_metadata(&id, |metadata| metadata.tags.push("tag-a".into()));
    let metadata = state.get_metadata(&id);
    assert_eq!(metadata.tags, vec!["tag-a".to_string()]);
    assert!(metadata.selection.priorities.is_empty());
    Ok(())
}

#[test]
fn metadata_can_be_set_and_removed() -> Result<()> {
    let id = Uuid::new_v4();
    let state = ApiState::new(
        Arc::new(StubConfig),
        test_indexers(),
        Metrics::new()?,
        Arc::new(json!({})),
        EventBus::with_capacity(4),
        None,
    );

    let mut metadata = TorrentMetadata::default();
    metadata.tags.push("alpha".to_string());
    state.set_metadata(id, metadata);
    assert_eq!(state.get_metadata(&id).tags, vec!["alpha".to_string()]);

    state.remove_metadata(&id);
    assert!(state.get_metadata(&id).tags.is_empty());
    Ok(())
}

#[test]
fn rate_limit_guard_tracks_missing_limit_and_active_limit() -> Result<()> {
    let state = ApiState::new(
        Arc::new(StubConfig),
        test_indexers(),
        Metrics::new()?,
        Arc::new(json!({})),
        EventBus::with_capacity(4),
        None,
    );

    let missing = state.enforce_rate_limit("demo", None)?;
    assert!(missing.is_none());
    assert_eq!(
        state.current_health_degraded(),
        vec!["api_rate_limit_guard".to_string()]
    );

    let limit = ApiKeyRateLimit {
        burst: 2,
        replenish_period: Duration::from_mins(1),
    };
    let snapshot = state
        .enforce_rate_limit("demo", Some(&limit))?
        .ok_or_else(|| anyhow::anyhow!("expected snapshot"))?;
    assert_eq!(snapshot.limit, 2);
    assert!(snapshot.remaining <= 1);
    assert!(state.current_health_degraded().is_empty());
    Ok(())
}

#[test]
fn rate_limit_enforcement_rejects_when_burst_exhausted() -> Result<()> {
    let state = ApiState::new(
        Arc::new(StubConfig),
        test_indexers(),
        Metrics::new()?,
        Arc::new(json!({})),
        EventBus::with_capacity(4),
        None,
    );
    let limit = ApiKeyRateLimit {
        burst: 1,
        replenish_period: Duration::from_mins(1),
    };
    assert!(state.enforce_rate_limit("demo", Some(&limit))?.is_some());
    let Err(err) = state.enforce_rate_limit("demo", Some(&limit)) else {
        return Err(anyhow::anyhow!("second request should be rate limited"));
    };
    assert_eq!(err.limit, 1);
    assert!(err.retry_after.as_secs() <= 60);
    Ok(())
}
