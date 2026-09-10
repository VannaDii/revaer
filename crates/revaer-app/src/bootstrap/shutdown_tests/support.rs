use std::collections::BTreeMap;
use std::fmt::Debug;
use std::future::{Future, pending};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::task::Poll;
use std::time::Duration;

use tokio::task::{Id, JoinHandle};
use tracing::field::{Field, Visit};
use tracing::instrument::WithSubscriber;
use tracing::span::{Attributes, Record};
use tracing::subscriber::Interest;
use tracing::{Event, Level, Metadata, Subscriber};

pub(super) const TEST_BOUND: Duration = Duration::from_secs(5);
pub(super) const PANIC_MESSAGE: &str = "shutdown event regression panic";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CapturedEvent {
    level: Level,
    fields: BTreeMap<String, String>,
}

impl CapturedEvent {
    pub(super) fn join(level: Level, task: &str, id: Id, panicked: bool) -> Self {
        let message = if level == Level::INFO {
            "runtime task cancelled after shutdown abort request"
        } else {
            "runtime task join failed"
        };
        let error = if panicked {
            format!("task {id} panicked with message {PANIC_MESSAGE:?}")
        } else {
            format!("task {id} was cancelled")
        };
        Self {
            level,
            fields: BTreeMap::from([
                ("message".to_owned(), message.to_owned()),
                ("task".to_owned(), task.to_owned()),
                ("error".to_owned(), error),
            ]),
        }
    }

    pub(super) fn deadline(task: &str) -> Self {
        Self {
            level: Level::WARN,
            fields: BTreeMap::from([
                (
                    "message".to_owned(),
                    "runtime task aborted after graceful shutdown timeout".to_owned(),
                ),
                ("task".to_owned(), task.to_owned()),
            ]),
        }
    }
}

impl Visit for CapturedEvent {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_owned(), value.to_owned());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.fields
            .insert(field.name().to_owned(), format!("{value:?}"));
    }
}

struct EventRecorder(mpsc::Sender<CapturedEvent>);

impl Subscriber for EventRecorder {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.is_event() && metadata.target() == "revaer_app::bootstrap"
    }

    fn register_callsite(&self, _metadata: &'static Metadata<'static>) -> Interest {
        // Recheck the current scoped subscriber, even when another test installs telemetry.
        Interest::sometimes()
    }

    fn event(&self, event: &Event<'_>) {
        let mut captured = CapturedEvent {
            level: *event.metadata().level(),
            fields: BTreeMap::new(),
        };
        event.record(&mut captured);
        assert!(
            self.0.send(captured).is_ok(),
            "event receiver must remain live"
        );
    }

    // This subscriber selects events only; spans are never enabled.
    fn new_span(&self, _attributes: &Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

pub(super) async fn capture(
    future: impl Future<Output = ()>,
) -> anyhow::Result<Vec<CapturedEvent>> {
    let (sender, receiver) = mpsc::channel();
    tokio::time::timeout(TEST_BOUND, future.with_subscriber(EventRecorder(sender))).await?;
    let events = receiver.try_iter().collect();
    eprintln!("captured shutdown events: {events:#?}");
    Ok(events)
}

pub(super) struct CleanupWitness {
    pub(super) drops: Arc<AtomicUsize>,
    pub(super) panic_on_drop: bool,
}

impl Drop for CleanupWitness {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
        // Deliberately exercise Tokio's panic JoinError, including during an abort.
        assert!(!self.panic_on_drop, "{PANIC_MESSAGE}");
    }
}

pub(super) async fn pending_task(
    panic_on_drop: bool,
) -> anyhow::Result<(JoinHandle<()>, Arc<AtomicUsize>)> {
    let drops = Arc::new(AtomicUsize::new(0));
    let witness = CleanupWitness {
        drops: Arc::clone(&drops),
        panic_on_drop,
    };
    let (started, ready) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _witness = witness;
        assert!(started.send(()).is_ok(), "start receiver must remain live");
        pending::<()>().await;
    });
    tokio::time::timeout(TEST_BOUND, ready).await??;
    Ok((task, drops))
}

pub(super) async fn wait_finished<T>(task: &JoinHandle<T>) -> anyhow::Result<()> {
    tokio::time::timeout(TEST_BOUND, async {
        while !task.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

pub(super) async fn poll_pending(future: impl Future<Output = ()>) {
    tokio::pin!(future);
    std::future::poll_fn(|context| {
        assert!(
            future.as_mut().poll(context).is_pending(),
            "stop must remain pending before the controlled task terminates"
        );
        Poll::Ready(())
    })
    .await;
}
