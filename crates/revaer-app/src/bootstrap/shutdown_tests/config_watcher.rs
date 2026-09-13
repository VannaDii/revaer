use super::super::{config_watch_step, stop_config_watch_task};
use super::support::{
    CapturedEvent, CleanupWitness, TEST_BOUND, capture, pending_task, poll_pending, wait_finished,
};
use crate::runtime_shutdown;
use std::cell::Cell;
use std::future::pending;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{Instant, timeout};
use tracing::Level;

async fn watch_config_updates<T, E>(
    mut next: impl AsyncFnMut() -> Result<T, E>,
    mut apply: impl AsyncFnMut(T),
    mut shutdown: runtime_shutdown::RuntimeShutdownReceiver,
) -> Result<(), E> {
    loop {
        if config_watch_step(next(), |update| apply(update), &mut shutdown)
            .await?
            .is_break()
        {
            return Ok(());
        }
    }
}

#[tokio::test]
async fn idle_wait_is_cancelled_and_the_owned_watcher_joins() -> anyhow::Result<()> {
    let (shutdown, mut receiver) = runtime_shutdown::channel();
    let drops = Arc::new(AtomicUsize::new(0));
    let witness = CleanupWitness {
        drops: Arc::clone(&drops),
        panic_on_drop: false,
    };
    let (ready, mut started) = tokio::sync::oneshot::channel();
    let applied = Arc::new(AtomicUsize::new(0));
    let applications = Arc::clone(&applied);
    let mut task = tokio::spawn(async move {
        let _witness = witness;
        let result = config_watch_step(
            async {
                assert!(ready.send(()).is_ok());
                pending::<Result<(), ()>>().await
            },
            |()| async {
                applications.fetch_add(1, Ordering::SeqCst);
            },
            &mut receiver,
        )
        .await;
        assert_eq!(result, Ok(std::ops::ControlFlow::Break(())));
    });
    timeout(TEST_BOUND, &mut started).await??;
    assert!(runtime_shutdown::request(&shutdown));
    let events = capture(stop_config_watch_task(&mut task, &shutdown)).await?;
    assert!(
        events.is_empty(),
        "cooperative join must not warn: {events:?}"
    );
    assert!(task.is_finished());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(applied.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
async fn prelatched_drain_never_polls_a_ready_update() -> anyhow::Result<()> {
    let (shutdown, receiver) = runtime_shutdown::channel();
    assert!(runtime_shutdown::request(&shutdown));
    let reads = Cell::new(0);
    let applied = Cell::new(0);
    let result = timeout(
        TEST_BOUND,
        watch_config_updates(
            async || {
                reads.set(reads.get() + 1);
                Ok::<_, ()>(7)
            },
            async |_| applied.set(applied.get() + 1),
            receiver,
        ),
    )
    .await?;
    assert_eq!(result, Ok(()));
    assert_eq!(reads.get(), 0);
    assert_eq!(applied.get(), 0);
    Ok(())
}

#[tokio::test]
async fn drain_during_next_poll_does_not_admit_the_returned_update() -> anyhow::Result<()> {
    let (shutdown, receiver) = runtime_shutdown::channel();
    let reads = Cell::new(0);
    let applied = Cell::new(0);
    let result = timeout(
        TEST_BOUND,
        watch_config_updates(
            async || {
                reads.set(reads.get() + 1);
                assert!(runtime_shutdown::request(&shutdown));
                Ok::<_, ()>(7)
            },
            async |_| applied.set(applied.get() + 1),
            receiver,
        ),
    )
    .await?;
    assert_eq!(result, Ok(()));
    assert_eq!(reads.get(), 1);
    assert_eq!(applied.get(), 0);
    Ok(())
}

#[tokio::test]
async fn ready_update_loses_to_drain_after_idle_poll() -> anyhow::Result<()> {
    let (shutdown, receiver) = runtime_shutdown::channel();
    let (send, mut update) = tokio::sync::oneshot::channel();
    let applied = Cell::new(0);
    let mut watcher = Box::pin(watch_config_updates(
        async || (&mut update).await,
        async |()| applied.set(applied.get() + 1),
        receiver,
    ));
    poll_pending(&mut watcher).await;
    assert!(runtime_shutdown::request(&shutdown));
    assert!(send.send(()).is_ok());
    assert_eq!(timeout(TEST_BOUND, watcher).await?, Ok(()));
    assert_eq!(applied.get(), 0);
    Ok(())
}

#[tokio::test]
async fn admitted_apply_outcome_finishes_before_exit_without_another_read() -> anyhow::Result<()> {
    for outcome in [Ok(()), Err("completed apply failed")] {
        let (shutdown, receiver) = runtime_shutdown::channel();
        let (finish, mut finished) = tokio::sync::oneshot::channel();
        let reads = Cell::new(0);
        let applied = Cell::new(0);
        let mut observed = Vec::new();
        let mut watcher = Box::pin(watch_config_updates(
            async || {
                reads.set(reads.get() + 1);
                Ok::<_, ()>(7)
            },
            async |revision| {
                applied.set(applied.get() + 1);
                let result = (&mut finished).await;
                observed.push((revision, result));
            },
            receiver,
        ));
        poll_pending(&mut watcher).await;
        assert_eq!(applied.get(), 1);
        assert!(runtime_shutdown::request(&shutdown));
        poll_pending(&mut watcher).await;
        assert_eq!(reads.get(), 1);
        assert!(
            finish.send(outcome).is_ok(),
            "admitted reply must remain owned"
        );
        assert_eq!(timeout(TEST_BOUND, watcher).await?, Ok(()));
        assert_eq!(reads.get(), 1);
        assert_eq!(applied.get(), 1);
        assert_eq!(observed, [(7, Ok(outcome))]);
    }
    Ok(())
}

#[tokio::test]
async fn read_failure_remains_the_original_error_even_when_drain_latches() -> anyhow::Result<()> {
    for latch in [false, true] {
        let (shutdown, receiver) = runtime_shutdown::channel();
        let applied = Cell::new(0);
        let result = timeout(
            TEST_BOUND,
            watch_config_updates(
                async || {
                    if latch {
                        assert!(runtime_shutdown::request(&shutdown));
                    }
                    Err::<(), _>("original watcher failure")
                },
                async |()| applied.set(applied.get() + 1),
                receiver,
            ),
        )
        .await?;
        assert_eq!(result, Err("original watcher failure"));
        assert_eq!(applied.get(), 0);
    }
    Ok(())
}

#[tokio::test]
async fn watcher_wait_observes_shortening_without_replenishing_other_waits() -> anyhow::Result<()> {
    let (mut task, drops) = pending_task(false).await?;
    let (shutdown, _receiver) = runtime_shutdown::channel();
    let origin = Instant::now();
    assert!(runtime_shutdown::request_until(
        &shutdown,
        origin,
        origin + TEST_BOUND
    ));
    let events = capture(async {
        let stop = stop_config_watch_task(&mut task, &shutdown);
        tokio::pin!(stop);
        poll_pending(&mut stop).await;
        let cutoff = Instant::now();
        assert!(runtime_shutdown::request_until(&shutdown, cutoff, cutoff));
        stop.await;
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        assert!(runtime_shutdown::request(&shutdown));
        assert!(runtime_shutdown::deadline_elapsed(&shutdown).await.is_ok());
    })
    .await?;
    assert_eq!(
        events,
        [
            CapturedEvent::watcher_deadline(),
            CapturedEvent::watcher_warning(
                "config watcher task settlement unconfirmed after shutdown abort request"
            ),
        ]
    );
    assert!(events.iter().all(|event| !event.matches_message(
        Level::INFO,
        "runtime task cancelled after shutdown abort request"
    )));
    // The production bound did not wait again or claim this later cancellation.
    let result = timeout(TEST_BOUND, &mut task).await?;
    assert!(result.is_err_and(|error| error.is_cancelled()));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}

#[cfg(feature = "libtorrent")]
#[path = "config_apply.rs"]
mod config_apply;

#[tokio::test]
async fn expired_authority_aborts_without_unbounded_join_or_false_success() -> anyhow::Result<()> {
    for panic_on_drop in [false, true] {
        let (mut task, drops) = pending_task(panic_on_drop).await?;
        let shutdown = super::shutdown_after(std::time::Duration::ZERO);
        let events = capture(stop_config_watch_task(&mut task, &shutdown)).await?;
        assert_eq!(
            events,
            [
                CapturedEvent::watcher_deadline(),
                CapturedEvent::watcher_warning(
                    "config watcher task settlement unconfirmed after shutdown abort request"
                ),
            ]
        );
        let result = timeout(TEST_BOUND, &mut task).await?;
        assert!(result.is_err_and(|error| if panic_on_drop {
            error.is_panic()
        } else {
            error.is_cancelled()
        }));
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    Ok(())
}

#[tokio::test]
async fn genuine_finished_watcher_outcomes_keep_their_join_classification() -> anyhow::Result<()> {
    let shutdown = super::shutdown_after(TEST_BOUND);
    let mut success = tokio::spawn(async {});
    wait_finished(&success).await?;
    assert!(
        capture(stop_config_watch_task(&mut success, &shutdown))
            .await?
            .is_empty()
    );
    for panic_on_drop in [false, true] {
        let (mut task, drops) = pending_task(panic_on_drop).await?;
        let id = task.id();
        task.abort();
        wait_finished(&task).await?;
        let events = capture(stop_config_watch_task(&mut task, &shutdown)).await?;
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

#[tokio::test]
async fn external_abort_during_cooperative_watcher_join_remains_warn() -> anyhow::Result<()> {
    let (mut task, drops) = pending_task(false).await?;
    let id = task.id();
    let external = task.abort_handle();
    let shutdown = super::shutdown_after(TEST_BOUND);
    let events = capture(async {
        let stop = stop_config_watch_task(&mut task, &shutdown);
        tokio::pin!(stop);
        poll_pending(&mut stop).await;
        external.abort();
        stop.await;
    })
    .await?;
    assert_eq!(
        events,
        [CapturedEvent::join(
            Level::WARN,
            "config_watcher",
            id,
            false
        )]
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    Ok(())
}
