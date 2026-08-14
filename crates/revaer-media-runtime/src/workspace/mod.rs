//! Managed media workspace capacity, lifecycle, and cleanup policy.

mod filesystem;
mod janitor;
mod model;
mod policy;

pub use filesystem::{
    cleanup_terminal_workspace, create_managed_workspace, teardown_managed_workspace,
};
pub use janitor::{cleanup_stale_workspaces, cleanup_stale_workspaces_bounded};
pub use model::{
    ManagedWorkspace, ManagedWorkspaceError, TerminalWorkspaceCleanupPolicy,
    TerminalWorkspaceState, WorkspaceCapacityReport, WorkspaceCleanupFailure,
    WorkspaceCleanupReport, WorkspaceError, WorkspacePolicy, WorkspaceRejectionReason,
    WorkspaceRetentionPolicy,
};

#[cfg(test)]
mod tests;
