//! Per-batch aggregate accounting without a persistent debit ledger.

use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

use super::{MediaServiceError, MediaServiceErrorKind};

#[derive(Default)]
pub(crate) struct AdmissionControl {
    pub(crate) cancelled: Option<Arc<AtomicBool>>,
    pub(crate) budget: Option<AdmissionBudget>,
}

#[derive(Clone, Copy)]
pub(crate) struct AdmissionBudget {
    pub(crate) batch_bytes: u64,
    pub(crate) remaining_run_bytes: u64,
    pub(crate) batch_metadata: Duration,
    pub(crate) remaining_run_metadata: Duration,
}

impl AdmissionBudget {
    pub(super) fn metadata_available(self, elapsed: Duration) -> bool {
        elapsed < self.batch_metadata
    }

    pub(super) fn validate_metadata(self, elapsed: Duration) -> Result<(), MediaServiceError> {
        if elapsed > self.remaining_run_metadata {
            return Err(MediaServiceError::new(MediaServiceErrorKind::Unavailable)
                .with_code("media_discovery_metadata_limit"));
        }
        Ok(())
    }

    pub(super) fn admit(self, used: u64, size: u64) -> Result<bool, MediaServiceError> {
        let total = used.checked_add(size).ok_or_else(aggregate_limit)?;
        if total > self.remaining_run_bytes || size > self.batch_bytes {
            return Err(aggregate_limit());
        }
        // A fitting candidate is retained for the next batch, never skipped.
        Ok(total <= self.batch_bytes)
    }
}

fn aggregate_limit() -> MediaServiceError {
    MediaServiceError::new(MediaServiceErrorKind::Unavailable)
        .with_code("media_discovery_aggregate_limit")
}

#[cfg(test)]
mod tests {
    use super::{AdmissionBudget, Duration};

    #[test]
    fn aggregate_batch_yields_fitting_candidate_and_accepts_exact_limit()
    -> Result<(), Box<dyn std::error::Error>> {
        let budget = AdmissionBudget {
            batch_bytes: 1040,
            remaining_run_bytes: 16640,
            batch_metadata: Duration::from_secs(2),
            remaining_run_metadata: Duration::from_hours(6),
        };
        assert!(budget.admit(780, 260)?);
        assert!(!budget.admit(1040, 260)?);
        assert!(budget.admit(0, 260)?);
        Ok(())
    }

    #[test]
    fn metadata_quantum_yields_and_run_exhaustion_is_an_error()
    -> Result<(), Box<dyn std::error::Error>> {
        let budget = AdmissionBudget {
            batch_bytes: 1040,
            remaining_run_bytes: 16640,
            batch_metadata: Duration::from_secs(2),
            remaining_run_metadata: Duration::from_secs(6),
        };
        assert!(budget.metadata_available(Duration::from_secs(1)));
        assert!(!budget.metadata_available(Duration::from_secs(2)));
        budget.validate_metadata(Duration::from_secs(6))?;
        assert!(budget.validate_metadata(Duration::from_secs(7)).is_err());
        Ok(())
    }

    #[test]
    fn aggregate_run_exhaustion_and_oversize_are_errors() -> Result<(), Box<dyn std::error::Error>>
    {
        let budget = AdmissionBudget {
            batch_bytes: 1040,
            remaining_run_bytes: 260,
            batch_metadata: Duration::from_secs(2),
            remaining_run_metadata: Duration::from_hours(6),
        };
        assert!(budget.admit(0, 260)?);
        assert!(budget.admit(260, 1).is_err());
        assert!(budget.admit(0, 1041).is_err());
        assert!(budget.admit(u64::MAX, 1).is_err());
        Ok(())
    }
}
