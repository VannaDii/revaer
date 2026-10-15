use super::*;
use std::future::{Future, poll_fn};
use std::task::Poll;

const TEST_BOUND: Duration = Duration::from_secs(5);

fn authority(sender: &RuntimeShutdownSender) -> anyhow::Result<ShutdownAuthority> {
    (*sender.state.borrow()).ok_or_else(|| anyhow::anyhow!("shutdown authority missing"))
}

async fn assert_pending(future: impl Future) {
    tokio::pin!(future);
    poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
}

#[test]
fn default_request_latches_one_origin_and_the_unchanged_budget() -> anyhow::Result<()> {
    let (sender, receiver) = channel();
    assert!(!requested(&receiver));
    assert!(request(&sender));
    let first = authority(&sender)?;
    assert_eq!(first.deadline - first.origin, APPLICATION_SHUTDOWN_BUDGET);
    assert!(request(&sender));
    let repeated = authority(&sender)?;
    assert_eq!(repeated.origin, first.origin);
    assert_eq!(repeated.deadline, first.deadline);
    assert!(requested(&receiver));
    Ok(())
}

#[test]
fn requests_only_shorten_and_cannot_revive_expired_authority() -> anyhow::Result<()> {
    let (sender, receiver) = channel();
    let origin = Instant::now();
    let limit = origin + APPLICATION_SHUTDOWN_BUDGET;
    assert!(request_until(&sender, origin, limit + TEST_BOUND));
    assert_eq!(authority(&sender)?.deadline, limit);
    assert!(request_until(&sender, limit, limit + TEST_BOUND));
    assert_eq!(authority(&sender)?.origin, origin);
    assert_eq!(authority(&sender)?.deadline, limit);

    let shorter = origin + TEST_BOUND;
    assert!(request_until(&sender, limit, shorter));
    assert_eq!(authority(&sender)?.origin, origin);
    assert_eq!(authority(&sender)?.deadline, shorter);
    assert!(request_until(&sender, limit, origin));
    assert!(request_until(&sender, limit, limit + TEST_BOUND));
    assert_eq!(authority(&sender)?.origin, origin);
    assert_eq!(authority(&sender)?.deadline, origin);
    assert!(requested(&receiver));
    Ok(())
}

#[test]
fn duplicate_requests_coalesce_and_clones_observe_shortening() -> anyhow::Result<()> {
    let (sender, receiver) = channel();
    let mut observer = receiver.clone();
    assert!(request(&sender));
    let first = authority(&sender)?;
    observer.state.borrow_and_update();
    assert!(request(&sender));
    assert!(!observer.state.has_changed()?);
    assert!(request_until(&sender, first.origin, first.origin));
    assert!(observer.state.has_changed()?);
    let snapshot =
        (*observer.state.borrow()).ok_or_else(|| anyhow::anyhow!("observer missed authority"))?;
    assert_eq!(snapshot.origin, first.origin);
    assert_eq!(snapshot.deadline, first.origin);
    assert!(requested(&receiver));
    Ok(())
}

#[test]
fn authority_is_retained_without_receivers() -> anyhow::Result<()> {
    let (sender, receiver) = channel();
    drop(receiver);
    assert!(!request(&sender));
    let first = authority(&sender)?;
    let receiver = RuntimeShutdownReceiver {
        state: sender.state.subscribe(),
    };
    assert!(requested(&receiver));
    assert!(request(&sender));
    assert_eq!(authority(&sender)?.origin, first.origin);
    assert_eq!(authority(&sender)?.deadline, first.deadline);
    Ok(())
}

#[test]
fn concurrent_requests_preserve_the_origin_and_earliest_deadline() -> anyhow::Result<()> {
    let (sender, _receiver) = channel();
    let origin = Instant::now();
    assert!(request_until(
        &sender,
        origin,
        origin + APPLICATION_SHUTDOWN_BUDGET
    ));
    std::thread::scope(|scope| -> anyhow::Result<()> {
        let workers: Vec<_> = (0..8)
            .map(|offset| {
                let sender = &sender;
                scope.spawn(move || {
                    let deadline = origin + Duration::from_millis(offset);
                    assert!(request_until(sender, deadline, deadline));
                })
            })
            .collect();
        for worker in workers {
            worker
                .join()
                .map_err(|_| anyhow::anyhow!("shutdown requester panicked"))?;
        }
        Ok(())
    })?;
    assert_eq!(authority(&sender)?.origin, origin);
    assert_eq!(authority(&sender)?.deadline, origin);
    Ok(())
}

#[tokio::test]
async fn pending_deadline_wait_observes_shortening() -> anyhow::Result<()> {
    let (sender, _receiver) = channel();
    assert!(request(&sender));
    let original = authority(&sender)?;
    let wait = deadline_elapsed(&sender);
    tokio::pin!(wait);
    assert_pending(&mut wait).await;
    assert!(request_until(&sender, Instant::now(), original.origin));
    tokio::time::timeout(TEST_BOUND, wait).await??;
    assert_eq!(authority(&sender)?.origin, original.origin);
    assert_eq!(authority(&sender)?.deadline, original.origin);
    Ok(())
}

#[tokio::test]
async fn wait_started_before_request_uses_the_same_authority() -> anyhow::Result<()> {
    let (sender, _receiver) = channel();
    let wait = deadline_elapsed(&sender);
    tokio::pin!(wait);
    assert_pending(&mut wait).await;
    let origin = Instant::now();
    assert!(request_until(&sender, origin, origin));
    tokio::time::timeout(TEST_BOUND, wait).await??;
    assert_eq!(authority(&sender)?.origin, origin);
    assert_eq!(authority(&sender)?.deadline, origin);
    Ok(())
}

#[tokio::test]
async fn owner_loss_publishes_one_origin_to_every_receiver() -> anyhow::Result<()> {
    let (sender, mut receiver) = channel();
    let mut other = receiver.clone();
    drop(sender);
    let first = (*receiver.state.borrow())
        .ok_or_else(|| anyhow::anyhow!("owner loss did not latch shutdown"))?;
    let second = (*other.state.borrow())
        .ok_or_else(|| anyhow::anyhow!("clone missed owner-loss authority"))?;
    assert_eq!(first.origin, second.origin);
    assert_eq!(first.deadline, second.deadline);
    assert_eq!(first.deadline - first.origin, APPLICATION_SHUTDOWN_BUDGET);
    assert!(requested(&receiver));
    tokio::time::timeout(TEST_BOUND, changed(&mut receiver)).await?;
    assert!(sleep_or_requested(TEST_BOUND, &mut other).await);
    Ok(())
}

#[test]
fn owner_loss_cannot_extend_existing_authority() -> anyhow::Result<()> {
    let (sender, receiver) = channel();
    let origin = Instant::now();
    assert!(request_until(&sender, origin, origin));
    drop(sender);
    let retained = (*receiver.state.borrow())
        .ok_or_else(|| anyhow::anyhow!("owner loss discarded shutdown authority"))?;
    assert_eq!(retained.origin, origin);
    assert_eq!(retained.deadline, origin);
    Ok(())
}

#[tokio::test]
async fn cooperative_waits_preserve_running_and_stopped_behavior() -> anyhow::Result<()> {
    let (sender, mut receiver) = channel();
    assert!(!sleep_or_requested(Duration::ZERO, &mut receiver).await);
    let stop = changed(&mut receiver);
    tokio::pin!(stop);
    assert_pending(&mut stop).await;
    assert!(request(&sender));
    tokio::time::timeout(TEST_BOUND, stop).await?;
    Ok(())
}
