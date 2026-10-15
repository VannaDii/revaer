//! Deterministic workspace capacity policy evaluation.

use super::{WorkspaceCapacityReport, WorkspaceError, WorkspacePolicy, WorkspaceRejectionReason};

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
