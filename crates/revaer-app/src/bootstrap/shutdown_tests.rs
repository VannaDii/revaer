use super::{
    MediaRuntimeTasks, log_runtime_task_join_error, stop_media_runtime_tasks, stop_runtime_task,
    stop_runtime_task_gracefully as stop_with_authority,
};
use crate::runtime_shutdown::{self, RuntimeShutdownSender};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::time::Instant;
use tracing::Level;

#[path = "shutdown_tests/support.rs"]
mod support;
use support::{
    CapturedEvent, CleanupWitness, PANIC_MESSAGE, TEST_BOUND, capture, pending_task, poll_pending,
    wait_finished,
};

fn shutdown_after(grace: Duration) -> RuntimeShutdownSender {
    let (shutdown, _receiver) = runtime_shutdown::channel();
    let origin = Instant::now();
    assert!(runtime_shutdown::request_until(
        &shutdown,
        origin,
        origin + grace
    ));
    shutdown
}

async fn stop_runtime_task_gracefully<T>(
    task: tokio::task::JoinHandle<T>,
    name: &'static str,
    grace: Duration,
) {
    let shutdown = shutdown_after(grace);
    stop_with_authority(task, name, &shutdown).await;
}

#[tokio::test]
async fn shortening_authority_wakes_an_existing_grace_wait() -> anyhow::Result<()> {
    let (task, drops) = pending_task(false).await?;
    let id = task.id();
    let shutdown = shutdown_after(TEST_BOUND);
    let events = capture(async {
        let stop = stop_with_authority(task, "media_job", &shutdown);
        tokio::pin!(stop);
        poll_pending(&mut stop).await;
        let now = Instant::now();
        assert!(runtime_shutdown::request_until(&shutdown, now, now));
        stop.await;
    })
    .await?;
    assert_eq!(
        events,
        [
            CapturedEvent::deadline("media_job"),
            CapturedEvent::join(Level::INFO, "media_job", id, false),
        ]
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn sequential_media_stops_do_not_replenish_expired_authority() -> anyhow::Result<()> {
    let (discovery, discovery_drops) = pending_task(false).await?;
    let (job, job_drops) = pending_task(false).await?;
    let (retention, retention_drops) = pending_task(false).await?;
    let expected: Vec<_> = [
        ("media_discovery", discovery.id()),
        ("media_job", job.id()),
        ("media_retention", retention.id()),
    ]
    .into_iter()
    .flat_map(|(name, id)| {
        [
            CapturedEvent::deadline(name),
            CapturedEvent::join(Level::INFO, name, id, false),
        ]
    })
    .collect();
    let shutdown = shutdown_after(Duration::ZERO);
    let tasks = MediaRuntimeTasks {
        discovery,
        job,
        retention,
    };
    let events = capture(stop_media_runtime_tasks(tasks, &shutdown)).await?;
    assert_eq!(events, expected);
    for drops in [discovery_drops, job_drops, retention_drops] {
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[test]
fn bootstrap_requests_shared_shutdown_before_waiting_for_any_runtime() -> anyhow::Result<()> {
    let source = include_str!("../bootstrap.rs");
    let scope = source
        .split_once("let serve_result = api.serve(addr).await;")
        .ok_or_else(|| anyhow::anyhow!("bootstrap serve boundary missing"))?
        .1;
    let request = scope
        .find("request_runtime_shutdown(&shutdown);")
        .ok_or_else(|| anyhow::anyhow!("bootstrap shutdown request missing"))?;
    for stop in [
        "stop_runtime_task(indexer_runtime_task",
        "stop_runtime_task(import_job_runtime_task",
        "stop_media_runtime_tasks(media_runtime_tasks, &shutdown)",
        "stop_runtime_task(fsops_worker",
        "stop_config_watch_task(&mut config_task, &shutdown)",
    ] {
        assert!(
            request
                < scope
                    .find(stop)
                    .ok_or_else(|| anyhow::anyhow!("bootstrap stop missing: {stop}"))?
        );
    }
    Ok(())
}

#[tokio::test]
async fn requested_abort_is_info_with_error_task_and_cleanup() -> anyhow::Result<()> {
    for name in ["indexer", "import_job", "fsops"] {
        let (task, drops) = pending_task(false).await?;
        let id = task.id();
        let events = capture(stop_runtime_task(task, name)).await?;
        assert_eq!(events, [CapturedEvent::join(Level::INFO, name, id, false)]);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[tokio::test]
async fn already_finished_success_emits_no_join_event() -> anyhow::Result<()> {
    let drops = Arc::new(AtomicUsize::new(0));
    for graceful in [false, true] {
        let witness = CleanupWitness {
            drops: Arc::clone(&drops),
            panic_on_drop: false,
        };
        let task = tokio::spawn(async move { drop(witness) });
        wait_finished(&task).await?;
        let events = capture(async {
            if graceful {
                stop_runtime_task_gracefully(task, "media_job", TEST_BOUND).await;
            } else {
                stop_runtime_task(task, "indexer").await;
            }
        })
        .await?;
        assert!(
            events.is_empty(),
            "successful join is not a failure: {events:?}"
        );
    }
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    Ok(())
}

#[tokio::test]
async fn already_finished_external_cancellation_remains_warn() -> anyhow::Result<()> {
    for graceful in [false, true] {
        let (task, drops) = pending_task(false).await?;
        let id = task.id();
        task.abort();
        wait_finished(&task).await?;
        let events = capture(async {
            if graceful {
                stop_runtime_task_gracefully(task, "media_job", TEST_BOUND).await;
            } else {
                stop_runtime_task(task, "media_job").await;
            }
        })
        .await?;
        assert_eq!(
            events,
            [CapturedEvent::join(Level::WARN, "media_job", id, false)]
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[tokio::test]
async fn already_finished_panic_remains_warn() -> anyhow::Result<()> {
    for graceful in [false, true] {
        let drops = Arc::new(AtomicUsize::new(0));
        let witness = CleanupWitness {
            drops: Arc::clone(&drops),
            panic_on_drop: false,
        };
        let task = tokio::spawn(async move {
            let _witness = witness;
            panic!("{PANIC_MESSAGE}");
        });
        let id = task.id();
        wait_finished(&task).await?;
        let events = capture(async {
            if graceful {
                stop_runtime_task_gracefully(task, "media_job", TEST_BOUND).await;
            } else {
                stop_runtime_task(task, "media_job").await;
            }
        })
        .await?;
        assert_eq!(
            events,
            [CapturedEvent::join(Level::WARN, "media_job", id, true)]
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[tokio::test]
async fn graceful_completion_emits_no_join_event() -> anyhow::Result<()> {
    let drops = Arc::new(AtomicUsize::new(0));
    let witness = CleanupWitness {
        drops: Arc::clone(&drops),
        panic_on_drop: false,
    };
    let (finish, finished) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let _witness = witness;
        assert!(finished.await.is_ok());
    });
    let events = capture(async {
        let stop = stop_runtime_task_gracefully(task, "media_discovery", TEST_BOUND);
        tokio::pin!(stop);
        poll_pending(&mut stop).await;
        assert!(finish.send(()).is_ok());
        stop.await;
    })
    .await?;
    assert!(
        events.is_empty(),
        "graceful success is not a failure: {events:?}"
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn external_cancellation_during_grace_remains_warn() -> anyhow::Result<()> {
    let (task, drops) = pending_task(false).await?;
    let id = task.id();
    let external_abort = task.abort_handle();
    let events = capture(async {
        let stop = stop_runtime_task_gracefully(task, "media_retention", TEST_BOUND);
        tokio::pin!(stop);
        poll_pending(&mut stop).await;
        external_abort.abort();
        stop.await;
    })
    .await?;
    assert_eq!(
        events,
        [CapturedEvent::join(
            Level::WARN,
            "media_retention",
            id,
            false
        )]
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn panic_during_grace_remains_warn() -> anyhow::Result<()> {
    let drops = Arc::new(AtomicUsize::new(0));
    let witness = CleanupWitness {
        drops: Arc::clone(&drops),
        panic_on_drop: false,
    };
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let _witness = witness;
        assert!(released.await.is_ok());
        panic!("{PANIC_MESSAGE}");
    });
    let id = task.id();
    let events = capture(async {
        let stop = stop_runtime_task_gracefully(task, "media_job", TEST_BOUND);
        tokio::pin!(stop);
        poll_pending(&mut stop).await;
        assert!(release.send(()).is_ok());
        stop.await;
    })
    .await?;
    assert_eq!(
        events,
        [CapturedEvent::join(Level::WARN, "media_job", id, true)]
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn grace_expiry_retains_warn_before_requested_abort_info() -> anyhow::Result<()> {
    for name in ["media_discovery", "media_job", "media_retention"] {
        let (task, drops) = pending_task(false).await?;
        let id = task.id();
        let events = capture(stop_runtime_task_gracefully(
            task,
            name,
            Duration::from_millis(1),
        ))
        .await?;
        assert_eq!(
            events,
            [
                CapturedEvent::deadline(name),
                CapturedEvent::join(Level::INFO, name, id, false),
            ]
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[tokio::test]
async fn panic_after_locally_requested_abort_is_not_downgraded() -> anyhow::Result<()> {
    let (task, drops) = pending_task(true).await?;
    let id = task.id();
    let events = capture(stop_runtime_task(task, "import_job")).await?;
    assert_eq!(
        events,
        [CapturedEvent::join(Level::WARN, "import_job", id, true)]
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn panic_after_grace_expiry_retains_both_warnings() -> anyhow::Result<()> {
    let (task, drops) = pending_task(true).await?;
    let id = task.id();
    let events = capture(stop_runtime_task_gracefully(
        task,
        "media_job",
        Duration::from_millis(1),
    ))
    .await?;
    assert_eq!(
        events,
        [
            CapturedEvent::deadline("media_job"),
            CapturedEvent::join(Level::WARN, "media_job", id, true),
        ]
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
async fn finished_config_join_retains_real_error_and_warning() -> anyhow::Result<()> {
    for panic_on_drop in [false, true] {
        let (task, drops) = pending_task(panic_on_drop).await?;
        let id = task.id();
        task.abort();
        wait_finished(&task).await?;
        let error = match task.await {
            Ok(()) => anyhow::bail!("cancelled task must produce a real JoinError"),
            Err(error) => error,
        };
        let events = capture(async {
            log_runtime_task_join_error(&error, "config_watcher", false);
        })
        .await?;
        assert_eq!(
            events,
            [CapturedEvent::join(
                Level::WARN,
                "config_watcher",
                id,
                panic_on_drop
            )]
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[test]
fn bootstrap_watcher_uses_the_shared_authority_and_owned_join() {
    let source = include_str!("../bootstrap.rs");
    assert!(source.contains("stop_config_watch_task(&mut config_task, &shutdown).await;"));
    assert!(!source.contains("config_task.abort();"));
}

#[path = "shutdown_tests/config_watcher.rs"]
mod config_watcher;
