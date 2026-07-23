//! Workspace policy models.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

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

/// Managed per-job workspace directory layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedWorkspace {
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

/// Create a managed per-job workspace directory tree under the configured root.
///
/// # Errors
///
/// Returns [`ManagedWorkspaceError`] when the root/job key is invalid or when
/// directory creation fails.
pub fn create_managed_workspace(
    root_path: impl AsRef<Path>,
    job_key: &str,
) -> Result<ManagedWorkspace, ManagedWorkspaceError> {
    let root_path = normalize_workspace_root(root_path.as_ref())?;
    let job_key = normalize_workspace_job_key(job_key)?;
    let job_path = root_path.join(&job_key);
    let input_path = job_path.join("input");
    let output_path = job_path.join("output");
    let diagnostics_path = job_path.join("diagnostics");

    create_dir_all(&root_path, "workspace.create_root")?;
    create_dir_all(&input_path, "workspace.create_input")?;
    create_dir_all(&output_path, "workspace.create_output")?;
    create_dir_all(&diagnostics_path, "workspace.create_diagnostics")?;

    Ok(ManagedWorkspace {
        job_key,
        root_path,
        job_path,
        input_path,
        output_path,
        diagnostics_path,
    })
}

/// Remove one managed per-job workspace if it exists.
///
/// # Errors
///
/// Returns [`ManagedWorkspaceError::Io`] when existence checks or removal fail.
pub fn teardown_managed_workspace(
    workspace: &ManagedWorkspace,
) -> Result<(), ManagedWorkspaceError> {
    if try_exists(&workspace.job_path, "workspace.teardown_exists")? {
        fs::remove_dir_all(&workspace.job_path).map_err(|source| ManagedWorkspaceError::Io {
            operation: "workspace.teardown_remove",
            path: workspace.job_path.clone(),
            source,
        })?;
    }
    Ok(())
}

/// Clean a managed workspace after a terminal job state.
///
/// Completed jobs always remove the whole job workspace. Failed and cancelled
/// jobs remove the whole workspace by default, but can keep bounded diagnostics
/// while deleting transient input/output directories.
///
/// # Errors
///
/// Returns [`ManagedWorkspaceError::Io`] when existence checks or removal fail.
pub fn cleanup_terminal_workspace(
    workspace: &ManagedWorkspace,
    state: TerminalWorkspaceState,
    policy: TerminalWorkspaceCleanupPolicy,
) -> Result<(), ManagedWorkspaceError> {
    match state {
        TerminalWorkspaceState::Completed => teardown_managed_workspace(workspace),
        TerminalWorkspaceState::Failed | TerminalWorkspaceState::Cancelled
            if policy.retain_diagnostics =>
        {
            remove_dir_if_exists(&workspace.input_path, "workspace.terminal_remove_input")?;
            remove_dir_if_exists(&workspace.output_path, "workspace.terminal_remove_output")?;
            Ok(())
        }
        TerminalWorkspaceState::Failed | TerminalWorkspaceState::Cancelled => {
            teardown_managed_workspace(workspace)
        }
    }
}

/// Remove inactive workspace directories older than `max_age`.
///
/// Files directly under the root are ignored. Active job keys are preserved
/// regardless of age.
///
/// # Errors
///
/// Returns [`ManagedWorkspaceError`] when the root is invalid or directory
/// traversal/removal fails.
pub fn cleanup_stale_workspaces(
    root_path: impl AsRef<Path>,
    active_job_keys: &[String],
    now: SystemTime,
    max_age: Duration,
) -> Result<Vec<PathBuf>, ManagedWorkspaceError> {
    let root_path = normalize_workspace_root(root_path.as_ref())?;
    if !try_exists(&root_path, "workspace.cleanup_root_exists")? {
        return Ok(Vec::new());
    }

    let active = active_job_keys
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut removed = Vec::new();
    for entry in fs::read_dir(&root_path).map_err(|source| ManagedWorkspaceError::Io {
        operation: "workspace.cleanup_read_root",
        path: root_path.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| ManagedWorkspaceError::Io {
            operation: "workspace.cleanup_read_entry",
            path: root_path.clone(),
            source,
        })?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|source| ManagedWorkspaceError::Io {
                operation: "workspace.cleanup_metadata",
                path: path.clone(),
                source,
            })?;
        if !metadata.is_dir() {
            continue;
        }
        let Some(job_key) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if active.contains(job_key) {
            continue;
        }
        let modified = metadata
            .modified()
            .map_err(|source| ManagedWorkspaceError::Io {
                operation: "workspace.cleanup_modified",
                path: path.clone(),
                source,
            })?;
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age >= max_age {
            fs::remove_dir_all(&path).map_err(|source| ManagedWorkspaceError::Io {
                operation: "workspace.cleanup_remove",
                path: path.clone(),
                source,
            })?;
            removed.push(path);
        }
    }
    removed.sort();
    Ok(removed)
}

fn normalize_workspace_root(path: &Path) -> Result<PathBuf, ManagedWorkspaceError> {
    if path.as_os_str().is_empty() {
        return Err(ManagedWorkspaceError::EmptyRoot);
    }
    Ok(path.to_path_buf())
}

fn normalize_workspace_job_key(job_key: &str) -> Result<String, ManagedWorkspaceError> {
    let trimmed = job_key.trim();
    if trimmed.is_empty() {
        return Err(ManagedWorkspaceError::EmptyJobKey);
    }
    if !trimmed
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ManagedWorkspaceError::InvalidJobKey);
    }
    Ok(trimmed.to_string())
}

fn create_dir_all(path: &Path, operation: &'static str) -> Result<(), ManagedWorkspaceError> {
    fs::create_dir_all(path).map_err(|source| ManagedWorkspaceError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn remove_dir_if_exists(path: &Path, operation: &'static str) -> Result<(), ManagedWorkspaceError> {
    if try_exists(path, operation)? {
        fs::remove_dir_all(path).map_err(|source| ManagedWorkspaceError::Io {
            operation,
            path: path.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

fn try_exists(path: &Path, operation: &'static str) -> Result<bool, ManagedWorkspaceError> {
    path.try_exists()
        .map_err(|source| ManagedWorkspaceError::Io {
            operation,
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::{
        ManagedWorkspaceError, TerminalWorkspaceCleanupPolicy, TerminalWorkspaceState,
        WorkspaceError, WorkspacePolicy, WorkspaceRejectionReason, cleanup_stale_workspaces,
        cleanup_terminal_workspace, create_managed_workspace, teardown_managed_workspace,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, SystemTime};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "revaer-media-runtime-workspace-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

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
    fn policy_validation_rejects_reserve_above_max() {
        let policy = WorkspacePolicy {
            max_bytes: 512,
            reserve_bytes: 1024,
        };

        assert_eq!(policy.validate(), Err(WorkspaceError::InvalidPolicy));
        assert_eq!(
            policy.ensure_capacity(2048, 256),
            Err(WorkspaceError::InvalidPolicy)
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

    #[test]
    fn managed_workspace_creation_and_teardown_uses_job_scoped_dirs()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_workspace_root()?;
        let workspace = create_managed_workspace(&root, "job-1")?;

        assert_eq!(workspace.job_key, "job-1");
        assert!(workspace.job_path.exists());
        assert!(workspace.input_path.exists());
        assert!(workspace.output_path.exists());
        assert!(workspace.diagnostics_path.exists());

        teardown_managed_workspace(&workspace)?;
        assert!(!workspace.job_path.exists());
        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn managed_workspace_rejects_empty_root() {
        assert!(matches!(
            create_managed_workspace("", "job-1"),
            Err(ManagedWorkspaceError::EmptyRoot)
        ));
    }

    #[test]
    fn managed_workspace_rejects_empty_and_path_like_job_keys()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_workspace_root()?;

        assert!(matches!(
            create_managed_workspace(&root, "  "),
            Err(ManagedWorkspaceError::EmptyJobKey)
        ));
        assert!(matches!(
            create_managed_workspace(&root, "bad/key"),
            Err(ManagedWorkspaceError::InvalidJobKey)
        ));
        assert!(matches!(
            create_managed_workspace(&root, ".."),
            Err(ManagedWorkspaceError::InvalidJobKey)
        ));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn stale_workspace_janitor_removes_only_inactive_directories()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_workspace_root()?;
        let active = root.join("active-job");
        let stale = root.join("stale-job");
        fs::create_dir_all(&active)?;
        fs::create_dir_all(&stale)?;
        fs::write(root.join("not-a-workspace"), b"skip")?;

        let removed = cleanup_stale_workspaces(
            &root,
            &["active-job".to_string()],
            SystemTime::now(),
            Duration::ZERO,
        )?;

        assert_eq!(removed, vec![stale]);
        assert!(active.exists());
        assert!(!root.join("stale-job").exists());
        assert!(root.join("not-a-workspace").exists());
        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn stale_workspace_cleanup_missing_root_returns_empty() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = temp_workspace_root()?;
        fs::remove_dir_all(&root)?;

        let removed = cleanup_stale_workspaces(&root, &[], SystemTime::now(), Duration::ZERO)?;

        assert!(removed.is_empty());
        Ok(())
    }

    #[test]
    fn terminal_workspace_cleanup_completed_removes_job_directory()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_workspace_root()?;
        let workspace = create_managed_workspace(&root, "job-completed")?;
        fs::write(workspace.output_path.join("movie.mkv"), b"output")?;

        cleanup_terminal_workspace(
            &workspace,
            TerminalWorkspaceState::Completed,
            TerminalWorkspaceCleanupPolicy {
                retain_diagnostics: true,
            },
        )?;

        assert!(!workspace.job_path.exists());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn terminal_workspace_cleanup_failed_can_retain_diagnostics_only()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_workspace_root()?;
        let workspace = create_managed_workspace(&root, "job-failed")?;
        fs::write(workspace.input_path.join("movie.mkv"), b"input")?;
        fs::write(workspace.output_path.join("movie.tmp"), b"output")?;
        fs::write(
            workspace.diagnostics_path.join("ffprobe.json"),
            b"diagnostic",
        )?;

        cleanup_terminal_workspace(
            &workspace,
            TerminalWorkspaceState::Failed,
            TerminalWorkspaceCleanupPolicy {
                retain_diagnostics: true,
            },
        )?;

        assert!(workspace.job_path.exists());
        assert!(!workspace.input_path.exists());
        assert!(!workspace.output_path.exists());
        assert!(workspace.diagnostics_path.join("ffprobe.json").exists());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn terminal_workspace_cleanup_cancelled_removes_transients_by_default()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_workspace_root()?;
        let workspace = create_managed_workspace(&root, "job-cancelled")?;
        fs::write(workspace.output_path.join("movie.tmp"), b"output")?;
        fs::write(workspace.diagnostics_path.join("run.log"), b"diagnostic")?;

        cleanup_terminal_workspace(
            &workspace,
            TerminalWorkspaceState::Cancelled,
            TerminalWorkspaceCleanupPolicy {
                retain_diagnostics: false,
            },
        )?;

        assert!(!workspace.job_path.exists());
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
