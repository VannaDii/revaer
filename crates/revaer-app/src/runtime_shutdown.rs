//! Shared monotonic authority for cooperative background runtime shutdown.

use std::time::Duration;

use tokio::sync::watch;
use tokio::time::{Instant, sleep, sleep_until};

const APPLICATION_SHUTDOWN_BUDGET: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
struct ShutdownAuthority {
    origin: Instant,
    deadline: Instant,
}

/// Sender side for a cooperative background runtime shutdown signal.
pub(crate) struct RuntimeShutdownSender {
    state: watch::Sender<Option<ShutdownAuthority>>,
}

/// Receiver side for a cooperative background runtime shutdown signal.
#[derive(Clone)]
pub(crate) struct RuntimeShutdownReceiver {
    state: watch::Receiver<Option<ShutdownAuthority>>,
}

impl Drop for RuntimeShutdownSender {
    fn drop(&mut self) {
        // Owner loss also stops receivers, with one origin before the channel closes.
        // Receiver presence is informational; the authority is retained either way.
        let _ = request(self);
    }
}

/// Create a shutdown channel whose initial state is running.
#[must_use]
pub(crate) fn channel() -> (RuntimeShutdownSender, RuntimeShutdownReceiver) {
    let (state, receiver) = watch::channel(None);
    (
        RuntimeShutdownSender { state },
        RuntimeShutdownReceiver { state: receiver },
    )
}

/// Request cooperative shutdown.
///
/// Returns `true` when at least one runtime receiver was still alive.
#[must_use]
pub(crate) fn request(sender: &RuntimeShutdownSender) -> bool {
    let origin = Instant::now();
    request_until(sender, origin, origin + APPLICATION_SHUTDOWN_BUDGET)
}

/// Request shutdown using an observed origin and an absolute upper bound.
///
/// The first origin is immutable. Further requests can only shorten the shared
/// deadline, never exceed the application budget, or revive expired authority.
/// Returns whether receivers were present when the request began.
#[must_use]
pub(crate) fn request_until(
    sender: &RuntimeShutdownSender,
    origin: Instant,
    deadline: Instant,
) -> bool {
    let receivers_present = sender.state.receiver_count() > 0;
    sender.state.send_if_modified(|state| {
        if let Some(authority) = state {
            let shortened = deadline
                .min(authority.origin + APPLICATION_SHUTDOWN_BUDGET)
                .min(authority.deadline);
            if shortened == authority.deadline {
                return false;
            }
            authority.deadline = shortened;
        } else {
            *state = Some(ShutdownAuthority {
                origin,
                deadline: deadline.min(origin + APPLICATION_SHUTDOWN_BUDGET),
            });
        }
        true
    });
    receivers_present
}

/// Wait for the shared absolute deadline, observing shortening while pending.
///
/// The borrowed owner keeps the channel live. Channel loss is nevertheless
/// returned as an error, never classified as successful deadline observation.
pub(crate) async fn deadline_elapsed(
    sender: &RuntimeShutdownSender,
) -> Result<(), watch::error::RecvError> {
    let mut receiver = sender.state.subscribe();
    loop {
        let authority = *receiver.borrow_and_update();
        if let Some(authority) = authority {
            tokio::select! {
                () = sleep_until(authority.deadline) => return Ok(()),
                result = receiver.changed() => result?,
            }
        } else {
            receiver.changed().await?;
        }
    }
}

/// Return whether shutdown has already been requested.
#[must_use]
pub(crate) fn requested(receiver: &RuntimeShutdownReceiver) -> bool {
    receiver.state.borrow().is_some()
}

/// Wait until shutdown is requested or every sender is dropped.
pub(crate) async fn changed(receiver: &mut RuntimeShutdownReceiver) {
    loop {
        if requested(receiver) {
            return;
        }
        if receiver.state.changed().await.is_err() {
            return;
        }
    }
}

/// Sleep for a duration unless shutdown is requested first.
///
/// Returns `true` when shutdown interrupted the sleep.
pub(crate) async fn sleep_or_requested(
    duration: Duration,
    receiver: &mut RuntimeShutdownReceiver,
) -> bool {
    tokio::select! {
        () = sleep(duration) => false,
        () = changed(receiver) => true,
    }
}

#[cfg(test)]
mod tests;
