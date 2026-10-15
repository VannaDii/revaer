use super::watch_config_updates;
use crate::bootstrap::apply_config_snapshot;
use crate::bootstrap::shutdown_tests::support::{TEST_BOUND, capture, poll_pending};
use crate::bootstrap::tests::libtorrent_tests::sample_snapshot;
use crate::engine_config::EngineRuntimePlan;
use crate::orchestrator::{EngineConfigurator, TorrentOrchestrator};
use crate::runtime_shutdown;
use async_trait::async_trait;
use revaer_events::{Event, EventBus};
use revaer_fsops::FsOpsService;
use revaer_telemetry::Metrics;
use revaer_torrent_core::{
    AddTorrent, RemoveTorrent, TorrentEngine, TorrentError, TorrentRateLimit, TorrentResult,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use tokio::time::timeout;
use tokio_stream::StreamExt;
use tracing::Level;
use uuid::Uuid;

struct ControlledEngine {
    apply: Mutex<oneshot::Receiver<TorrentResult<()>>>,
    limits: Mutex<oneshot::Receiver<TorrentResult<()>>>,
    apply_calls: AtomicUsize,
    limits_calls: AtomicUsize,
}

#[async_trait]
impl TorrentEngine for ControlledEngine {
    async fn add_torrent(&self, _request: AddTorrent) -> TorrentResult<()> {
        Err(TorrentError::Unsupported {
            operation: "test_add",
        })
    }

    async fn remove_torrent(&self, _id: Uuid, _options: RemoveTorrent) -> TorrentResult<()> {
        Err(TorrentError::Unsupported {
            operation: "test_remove",
        })
    }

    async fn update_limits(&self, id: Option<Uuid>, limits: TorrentRateLimit) -> TorrentResult<()> {
        assert_eq!(id, None);
        assert_eq!(limits.download_bps, Some(1_500_000));
        assert_eq!(limits.upload_bps, Some(750_000));
        self.limits_calls.fetch_add(1, Ordering::SeqCst);
        (&mut *self.limits.lock().await)
            .await
            .map_err(|_| TorrentError::Unsupported {
                operation: "test_limits_reply_lost",
            })?
    }
}

#[async_trait]
impl EngineConfigurator for ControlledEngine {
    async fn apply_engine_plan(&self, _plan: &EngineRuntimePlan) -> TorrentResult<()> {
        self.apply_calls.fetch_add(1, Ordering::SeqCst);
        (&mut *self.apply.lock().await)
            .await
            .map_err(|_| TorrentError::Unsupported {
                operation: "test_apply_reply_lost",
            })?
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Completion {
    Success,
    ApplyFailure,
    LimitsFailure,
}

fn completion_result(failed: bool, operation: &'static str) -> TorrentResult<()> {
    if failed {
        Err(TorrentError::Unsupported { operation })
    } else {
        Ok(())
    }
}

async fn exercise_drain(completion: Completion, drain_during_limits: bool) -> anyhow::Result<()> {
    let (shutdown, receiver) = runtime_shutdown::channel();
    let (apply, apply_reply) = oneshot::channel();
    let (limits, limits_reply) = oneshot::channel();
    let engine = Arc::new(ControlledEngine {
        apply: Mutex::new(apply_reply),
        limits: Mutex::new(limits_reply),
        apply_calls: AtomicUsize::new(0),
        limits_calls: AtomicUsize::new(0),
    });
    let events = EventBus::new();
    let metrics = Metrics::new()?;
    let mut snapshot = sample_snapshot();
    let scratch = tempfile::tempdir()?;
    snapshot.engine_profile.resume_dir = scratch.path().join("resume").display().to_string();
    snapshot.engine_profile.download_root = scratch.path().join("downloads").display().to_string();
    snapshot.fs_policy.library_root = scratch.path().join("library").display().to_string();
    let orchestrator = TorrentOrchestrator::new(
        Arc::clone(&engine),
        FsOpsService::new(events.clone(), metrics.clone()),
        events.clone(),
        snapshot.fs_policy.clone(),
        snapshot.engine_profile.clone(),
        None,
        None,
    );
    snapshot.engine_profile.max_download_bps = Some(1_500_000);
    snapshot.engine_profile.max_upload_bps = Some(750_000);
    let mut stream = events.subscribe(None);
    let reads = AtomicUsize::new(0);
    let mut degraded = false;
    let logs = capture(async {
        let mut watcher = Box::pin(watch_config_updates(
            async || {
                reads.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(snapshot.clone())
            },
            async |snapshot| {
                apply_config_snapshot(
                    snapshot,
                    &orchestrator,
                    &events,
                    &metrics,
                    &mut degraded,
                    Duration::from_secs(2),
                )
                .await;
            },
            receiver,
        ));
        poll_pending(&mut watcher).await;
        assert_eq!(engine.apply_calls.load(Ordering::SeqCst), 1);
        assert_eq!(engine.limits_calls.load(Ordering::SeqCst), 0);
        poll_pending(stream.next()).await;
        if !drain_during_limits {
            assert!(runtime_shutdown::request(&shutdown));
            poll_pending(&mut watcher).await;
        }
        assert!(
            apply
                .send(completion_result(
                    completion == Completion::ApplyFailure,
                    "controlled_apply_failure"
                ))
                .is_ok()
        );
        if completion != Completion::ApplyFailure {
            poll_pending(&mut watcher).await;
            assert_eq!(engine.limits_calls.load(Ordering::SeqCst), 1);
            poll_pending(stream.next()).await;
            if drain_during_limits {
                assert!(runtime_shutdown::request(&shutdown));
            }
            poll_pending(&mut watcher).await;
            assert!(
                limits
                    .send(completion_result(
                        completion == Completion::LimitsFailure,
                        "controlled_limits_failure"
                    ))
                    .is_ok()
            );
        }
        assert_eq!(watcher.await, Ok(()));
    })
    .await?;
    assert_eq!(
        reads.load(Ordering::SeqCst),
        1,
        "drain must prevent a second snapshot"
    );
    assert_eq!(degraded, completion != Completion::Success);
    verify_events(&mut stream, completion, &logs).await?;
    scratch.close()?;
    Ok(())
}

async fn verify_events(
    stream: &mut revaer_events::EventStream,
    completion: Completion,
    logs: &[crate::bootstrap::shutdown_tests::support::CapturedEvent],
) -> anyhow::Result<()> {
    let settings = timeout(TEST_BOUND, stream.next())
        .await?
        .ok_or_else(|| anyhow::anyhow!("missing watcher outcome"))??;
    let Event::SettingsChanged { description } = settings.event else {
        anyhow::bail!("watcher did not report a settings outcome");
    };
    verify_outcome(completion, &description, logs);
    if completion != Completion::Success {
        let health = timeout(TEST_BOUND, stream.next())
            .await?
            .ok_or_else(|| anyhow::anyhow!("missing degraded health"))??;
        assert!(matches!(health.event, Event::HealthChanged { degraded }
            if degraded == ["config_watcher"]));
    }
    poll_pending(stream.next()).await;
    Ok(())
}

fn verify_outcome(
    completion: Completion,
    description: &str,
    logs: &[crate::bootstrap::shutdown_tests::support::CapturedEvent],
) {
    let succeeded = "applied configuration update from watcher";
    let failed = "failed to apply engine profile update from watcher";
    if completion == Completion::Success {
        assert!(description.starts_with("watcher revision 3 applied in "));
        assert!(
            logs.iter()
                .any(|event| event.matches_message(Level::INFO, succeeded))
        );
        assert!(
            !logs
                .iter()
                .any(|event| event.matches_message(Level::WARN, failed))
        );
    } else {
        // AppError's existing Display is intentionally generic; bootstrap does
        // not expose its typed torrent source in this event description.
        assert_eq!(
            description,
            "failed to apply watcher revision 3: torrent operation failed"
        );
        assert!(
            logs.iter()
                .any(|event| event.matches_message(Level::WARN, failed))
        );
        assert!(
            !logs
                .iter()
                .any(|event| event.matches_message(Level::INFO, succeeded))
        );
    }
}

#[tokio::test]
async fn drain_during_apply_preserves_completed_apply_and_limits_ack() -> anyhow::Result<()> {
    exercise_drain(Completion::Success, false).await
}

#[tokio::test]
async fn drain_during_limits_keeps_the_reply_owned_until_completion() -> anyhow::Result<()> {
    exercise_drain(Completion::Success, true).await
}

#[tokio::test]
async fn drain_retains_the_completed_apply_failure_without_success_ack() -> anyhow::Result<()> {
    exercise_drain(Completion::ApplyFailure, false).await
}

#[tokio::test]
async fn drain_retains_the_completed_limits_failure_without_success_ack() -> anyhow::Result<()> {
    exercise_drain(Completion::LimitsFailure, true).await
}
