use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::Interest;
use tracing::{Event, Metadata, Subscriber};

use super::{BaselineReadError, BaselineReadReason};

#[derive(Clone, Default)]
struct EventCounter(Arc<AtomicUsize>);

impl Subscriber for EventCounter {
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        // The workspace runs other tests at this callsite without this local
        // subscriber. Re-evaluate the thread-local dispatch for every event.
        Interest::sometimes()
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, _: &Event<'_>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

#[test]
fn baseline_read_leaves_cancelled_query_diagnostics_to_the_lifecycle() {
    let events = EventCounter::default();
    tracing::subscriber::with_default(events.clone(), || {
        let error = BaselineReadError::from_database(Some("57014"), None);
        assert_eq!(error.reason(), BaselineReadReason::StatementFailed);
        assert_eq!(error.sqlstate(), Some("57014"));
        assert_eq!(events.0.load(Ordering::SeqCst), 0);

        let error = BaselineReadError::from_database(Some("42P01"), None);
        assert_eq!(error.reason(), BaselineReadReason::BaselineShapeInvalid);
        assert_eq!(events.0.load(Ordering::SeqCst), 1);
    });
}
