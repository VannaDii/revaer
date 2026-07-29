//! Cooperative background runtime shutdown helpers.

use std::time::Duration;

use tokio::sync::watch;
use tokio::time::sleep;

/// Sender side for a cooperative background runtime shutdown signal.
pub(crate) type RuntimeShutdownSender = watch::Sender<bool>;

/// Receiver side for a cooperative background runtime shutdown signal.
pub(crate) type RuntimeShutdownReceiver = watch::Receiver<bool>;

/// Create a shutdown channel whose initial state is running.
#[must_use]
pub(crate) fn channel() -> (RuntimeShutdownSender, RuntimeShutdownReceiver) {
    watch::channel(false)
}

/// Request cooperative shutdown.
///
/// Returns `true` when at least one runtime receiver was still alive.
#[must_use]
pub(crate) fn request(sender: &RuntimeShutdownSender) -> bool {
    sender.send(true).is_ok()
}

/// Return whether shutdown has already been requested.
#[must_use]
pub(crate) fn requested(receiver: &RuntimeShutdownReceiver) -> bool {
    *receiver.borrow()
}

/// Wait until shutdown is requested or every sender is dropped.
pub(crate) async fn changed(receiver: &mut RuntimeShutdownReceiver) {
    loop {
        if *receiver.borrow() {
            return;
        }
        if receiver.changed().await.is_err() {
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
