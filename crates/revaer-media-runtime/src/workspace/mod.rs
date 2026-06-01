//! Workspace policy models.

use thiserror::Error;

/// Workspace capacity and reserve policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePolicy {
    /// Maximum bytes permitted for temporary artifacts.
    pub max_bytes: u64,
    /// Minimum free bytes required to start a job.
    pub reserve_bytes: u64,
}

/// Workspace policy error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    /// Policy is inconsistent.
    #[error("workspace reserve must not exceed max bytes")]
    InvalidPolicy,
    /// Available bytes are insufficient.
    #[error("insufficient free disk for workspace reserve")]
    InsufficientReserve,
    /// Free bytes cannot hold required workspace bytes above reserve.
    #[error("insufficient free disk for estimated workspace demand")]
    InsufficientCapacity,
    /// Required workspace demand exceeds configured workspace max.
    #[error("required workspace demand exceeds configured workspace max")]
    ExceedsMaxWorkspace,
}

/// Deterministic workspace capacity evaluation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCapacityReport {
    /// Whether reserve and capacity constraints are satisfied.
    pub accepted: bool,
    /// Optional machine-readable rejection reason.
    pub reason: Option<WorkspaceRejectionReason>,
    /// Bytes available after reserve subtraction.
    pub available_after_reserve_bytes: u64,
    /// Requested temporary workspace bytes.
    pub required_workspace_bytes: u64,
}

/// Machine-readable workspace rejection reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRejectionReason {
    /// Policy is internally inconsistent.
    InvalidPolicy,
    /// Free bytes are below reserve floor.
    InsufficientReserve,
    /// Required bytes exceed configured workspace max.
    ExceedsMaxWorkspace,
    /// Required bytes do not fit above reserve.
    InsufficientCapacity,
}

impl WorkspacePolicy {
    /// Validate policy invariants.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError::InvalidPolicy`] when reserve exceeds max workspace bytes.
    pub const fn validate(&self) -> Result<(), WorkspaceError> {
        if self.reserve_bytes > self.max_bytes {
            return Err(WorkspaceError::InvalidPolicy);
        }
        Ok(())
    }

    /// Check whether free bytes satisfy reserve.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError::InvalidPolicy`] when policy values conflict.
    /// Returns [`WorkspaceError::InsufficientReserve`] when free bytes are below reserve.
    pub fn ensure_reserve(&self, free_bytes: u64) -> Result<(), WorkspaceError> {
        self.validate()?;
        if free_bytes < self.reserve_bytes {
            return Err(WorkspaceError::InsufficientReserve);
        }
        Ok(())
    }

    /// Check whether free bytes satisfy reserve plus required demand.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError::InvalidPolicy`] when policy values conflict.
    /// Returns [`WorkspaceError::InsufficientReserve`] when free bytes are below reserve.
    /// Returns [`WorkspaceError::InsufficientCapacity`] when demand cannot fit above reserve.
    /// Returns [`WorkspaceError::ExceedsMaxWorkspace`] when demand exceeds configured max.
    pub fn ensure_capacity(
        &self,
        free_bytes: u64,
        required_workspace_bytes: u64,
    ) -> Result<(), WorkspaceError> {
        self.ensure_reserve(free_bytes)?;
        if required_workspace_bytes > self.max_bytes {
            return Err(WorkspaceError::ExceedsMaxWorkspace);
        }
        let available_after_reserve = free_bytes - self.reserve_bytes;
        if required_workspace_bytes > available_after_reserve {
            return Err(WorkspaceError::InsufficientCapacity);
        }
        Ok(())
    }

    /// Evaluate capacity and return a structured deterministic report.
    #[must_use]
    pub const fn evaluate_capacity(
        &self,
        free_bytes: u64,
        required_workspace_bytes: u64,
    ) -> WorkspaceCapacityReport {
        if self.reserve_bytes > self.max_bytes {
            return WorkspaceCapacityReport {
                accepted: false,
                reason: Some(WorkspaceRejectionReason::InvalidPolicy),
                available_after_reserve_bytes: free_bytes.saturating_sub(self.reserve_bytes),
                required_workspace_bytes,
            };
        }

        if free_bytes < self.reserve_bytes {
            return WorkspaceCapacityReport {
                accepted: false,
                reason: Some(WorkspaceRejectionReason::InsufficientReserve),
                available_after_reserve_bytes: 0,
                required_workspace_bytes,
            };
        }

        if required_workspace_bytes > self.max_bytes {
            return WorkspaceCapacityReport {
                accepted: false,
                reason: Some(WorkspaceRejectionReason::ExceedsMaxWorkspace),
                available_after_reserve_bytes: free_bytes - self.reserve_bytes,
                required_workspace_bytes,
            };
        }

        let available_after_reserve_bytes = free_bytes - self.reserve_bytes;
        if required_workspace_bytes > available_after_reserve_bytes {
            return WorkspaceCapacityReport {
                accepted: false,
                reason: Some(WorkspaceRejectionReason::InsufficientCapacity),
                available_after_reserve_bytes,
                required_workspace_bytes,
            };
        }

        WorkspaceCapacityReport {
            accepted: true,
            reason: None,
            available_after_reserve_bytes,
            required_workspace_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceError, WorkspacePolicy, WorkspaceRejectionReason};

    #[test]
    fn reserve_check_rejects_low_free_space() {
        let policy = WorkspacePolicy {
            max_bytes: 1024,
            reserve_bytes: 512,
        };
        assert_eq!(
            policy.ensure_reserve(256),
            Err(WorkspaceError::InsufficientReserve)
        );
    }

    #[test]
    fn capacity_check_rejects_excess_demand() {
        let policy = WorkspacePolicy {
            max_bytes: 10_000,
            reserve_bytes: 4_000,
        };
        assert_eq!(
            policy.ensure_capacity(8_000, 4_100),
            Err(WorkspaceError::InsufficientCapacity)
        );
    }

    #[test]
    fn capacity_check_accepts_fit_above_reserve() {
        let policy = WorkspacePolicy {
            max_bytes: 10_000,
            reserve_bytes: 4_000,
        };
        assert!(policy.ensure_capacity(8_000, 4_000).is_ok());
    }

    #[test]
    fn capacity_check_rejects_when_required_exceeds_workspace_max() {
        let policy = WorkspacePolicy {
            max_bytes: 6_000,
            reserve_bytes: 1_000,
        };
        assert_eq!(
            policy.ensure_capacity(20_000, 6_001),
            Err(WorkspaceError::ExceedsMaxWorkspace)
        );
    }

    #[test]
    fn evaluate_capacity_reports_acceptance_and_budget() {
        let policy = WorkspacePolicy {
            max_bytes: 10_000,
            reserve_bytes: 2_000,
        };
        let report = policy.evaluate_capacity(9_000, 5_000);
        assert!(report.accepted);
        assert_eq!(report.reason, None);
        assert_eq!(report.available_after_reserve_bytes, 7_000);
        assert_eq!(report.required_workspace_bytes, 5_000);
    }

    #[test]
    fn evaluate_capacity_reports_rejection_reason() {
        let policy = WorkspacePolicy {
            max_bytes: 10_000,
            reserve_bytes: 2_000,
        };
        let report = policy.evaluate_capacity(3_000, 9_000);
        assert!(!report.accepted);
        assert_eq!(
            report.reason,
            Some(WorkspaceRejectionReason::InsufficientCapacity)
        );
        assert_eq!(report.available_after_reserve_bytes, 1_000);
    }
}
