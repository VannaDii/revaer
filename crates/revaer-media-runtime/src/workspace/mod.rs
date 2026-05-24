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
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceError, WorkspacePolicy};

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
}
