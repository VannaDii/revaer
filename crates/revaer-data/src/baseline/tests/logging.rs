use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

use super::{BaselineReadError, BaselineReadReason};

#[derive(Clone, Default)]
struct EventCounter(Arc<AtomicUsize>);

impl Subscriber for EventCounter {
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
