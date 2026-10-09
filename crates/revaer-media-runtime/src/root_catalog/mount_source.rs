//! Injectable fresh mount namespace observation for Linux bootstrap/admission.

use thiserror::Error;

use super::{RootMountError, RootMountTopology};

/// Supplies a new snapshot of the service process's mount namespace.
pub trait RootMountSource: Send + Sync {
    /// Observe the current namespace; implementations must not return cached proof.
    ///
    /// # Errors
    /// Propagates unavailable or malformed observations without their contents.
    fn snapshot(&self) -> Result<RootMountTopology, RootMountReadError>;
}

/// Reads the calling Linux process's kernel mountinfo file on every invocation.
/// Bootstrap constructs this collaborator; domain callers receive it by injection.
#[derive(Debug, Default)]
pub struct ProcRootMountSource;

impl RootMountSource for ProcRootMountSource {
    fn snapshot(&self) -> Result<RootMountTopology, RootMountReadError> {
        let input = std::fs::read_to_string("/proc/self/mountinfo")
            .map_err(RootMountReadError::Filesystem)?;
        RootMountTopology::parse(&input).map_err(RootMountReadError::Invalid)
    }
}

/// Path-free observation failure; no mount record is included in diagnostics.
#[derive(Debug, Error)]
pub enum RootMountReadError {
    /// Reading the process namespace failed.
    #[error("root mount namespace observation failed")]
    Filesystem(#[source] std::io::Error),
    /// The observed kernel record layout failed validation.
    #[error(transparent)]
    Invalid(#[from] RootMountError),
}
