//! Workspace lifecycle and capacity models.

use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

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

/// Projected per-job workspace paths. Projection performs no filesystem mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePaths {
    /// Stable job key used for the workspace directory name.
    pub job_key: String,
    /// Configured workspace root.
    pub root_path: PathBuf,
    /// Job-scoped workspace directory.
    pub job_path: PathBuf,
    /// Directory for inspected or staged input artifacts.
    pub input_path: PathBuf,
    /// Directory for generated output artifacts.
    pub output_path: PathBuf,
    /// Directory for bounded diagnostic artifacts.
    pub diagnostics_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorkspaceDirectoryIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) owner: u32,
    pub(super) mode: u32,
}

#[derive(Debug)]
pub(super) struct WorkspaceTrust {
    pub(super) root: Arc<File>,
    pub(super) root_identity: WorkspaceDirectoryIdentity,
    pub(super) job_identity: WorkspaceDirectoryIdentity,
    pub(super) input_identity: WorkspaceDirectoryIdentity,
    pub(super) output_identity: WorkspaceDirectoryIdentity,
    pub(super) diagnostics_identity: WorkspaceDirectoryIdentity,
}

/// Managed per-job workspace with retained trust anchors.
#[derive(Debug, Clone)]
pub struct ManagedWorkspace {
    /// Trusted path projection.
    pub paths: WorkspacePaths,
    pub(super) trust: Arc<WorkspaceTrust>,
}

impl ManagedWorkspace {
    /// Validate that every managed directory still has its original identity and policy.
    ///
    /// # Errors
    ///
    /// Returns [`ManagedWorkspaceError`] when a path was replaced or its policy changed.
    pub fn validate(&self) -> Result<(), ManagedWorkspaceError> {
        super::filesystem::validate_managed_workspace(self)
    }
}

impl std::ops::Deref for ManagedWorkspace {
    type Target = WorkspacePaths;

    fn deref(&self) -> &Self::Target {
        &self.paths
    }
}

/// Terminal state driving managed workspace cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalWorkspaceState {
    /// Job completed successfully.
    Completed,
    /// Job failed before completion.
    Failed,
    /// Job was cancelled before completion.
    Cancelled,
}

/// Policy for terminal workspace cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalWorkspaceCleanupPolicy {
    /// Keep diagnostics for failed/cancelled jobs while deleting transient inputs/outputs.
    pub retain_diagnostics: bool,
}

/// Persisted policy applied by the production workspace janitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceRetentionPolicy {
    /// Age after which an inactive full workspace can be removed.
    pub workspace_max_age: Duration,
    /// Age after which an inactive diagnostics-only workspace can be removed.
    pub diagnostics_max_age: Duration,
    /// Maximum root entries examined in one scheduled tick.
    pub max_entries_per_tick: usize,
}

/// One bounded filesystem cleanup failure retained for retry and observability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCleanupFailure {
    /// Job workspace path that could not be inspected or removed.
    pub path: PathBuf,
    /// Stable operation name.
    pub operation: &'static str,
    /// Stable operating-system error category.
    pub error_kind: io::ErrorKind,
}

/// Result of one bounded workspace janitor tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCleanupReport {
    /// Number of root entries examined.
    pub examined_entries: usize,
    /// Removed inactive workspace paths in deterministic order.
    pub removed: Vec<PathBuf>,
    /// Per-entry failures retained without aborting unrelated cleanup.
    pub failures: Vec<WorkspaceCleanupFailure>,
    /// Whether more root entries remained beyond the per-tick bound.
    pub limit_reached: bool,
}

/// Managed workspace filesystem error.
#[derive(Debug, Error)]
pub enum ManagedWorkspaceError {
    /// Workspace root path was empty.
    #[error("workspace root is required")]
    EmptyRoot,
    /// Workspace job key was empty.
    #[error("workspace job key is required")]
    EmptyJobKey,
    /// Workspace job key contained unsafe path characters.
    #[error("workspace job key is invalid")]
    InvalidJobKey,
    /// Cleanup policy had a zero per-tick entry bound.
    #[error("workspace cleanup policy is invalid")]
    InvalidCleanupPolicy,
    /// A managed path was a symlink or a non-directory object.
    #[error("workspace path is not a trusted directory: {0}")]
    UnsafePath(PathBuf),
    /// A managed directory was not owner-only or was owned by another user.
    #[error("workspace directory policy is unsafe: {0}")]
    UnsafeDirectoryPolicy(PathBuf),
    /// A managed directory was replaced after its trusted handle was opened.
    #[error("workspace directory identity changed: {0}")]
    IdentityChanged(PathBuf),
    /// Workspace filesystem operation failed.
    #[error("workspace filesystem operation {operation} failed for {path}: {source}")]
    Io {
        /// Operation being performed.
        operation: &'static str,
        /// Path being accessed.
        path: PathBuf,
        /// Source I/O error.
        source: io::Error,
    },
}
