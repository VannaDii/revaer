//! Managed workspace creation and terminal cleanup.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{
    ManagedWorkspace, ManagedWorkspaceError, TerminalWorkspaceCleanupPolicy, TerminalWorkspaceState,
};

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

    ensure_directory(&root_path, true, "workspace.create_root")?;
    ensure_directory(&job_path, false, "workspace.create_job")?;
    ensure_directory(&input_path, false, "workspace.create_input")?;
    ensure_directory(&output_path, false, "workspace.create_output")?;
    ensure_directory(&diagnostics_path, false, "workspace.create_diagnostics")?;

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
    remove_dir_if_exists(&workspace.job_path, "workspace.teardown_remove")
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
pub(super) fn normalize_workspace_root(path: &Path) -> Result<PathBuf, ManagedWorkspaceError> {
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

fn ensure_directory(
    path: &Path,
    recursive: bool,
    operation: &'static str,
) -> Result<(), ManagedWorkspaceError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_directory(path, &metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let creation = if recursive {
                fs::create_dir_all(path)
            } else {
                fs::create_dir(path)
            };
            if let Err(source) = creation
                && source.kind() != io::ErrorKind::AlreadyExists
            {
                return Err(ManagedWorkspaceError::Io {
                    operation,
                    path: path.to_path_buf(),
                    source,
                });
            }
            let metadata =
                fs::symlink_metadata(path).map_err(|source| ManagedWorkspaceError::Io {
                    operation,
                    path: path.to_path_buf(),
                    source,
                })?;
            validate_directory(path, &metadata)
        }
        Err(source) => Err(ManagedWorkspaceError::Io {
            operation,
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn validate_directory(path: &Path, metadata: &fs::Metadata) -> Result<(), ManagedWorkspaceError> {
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err(ManagedWorkspaceError::UnsafePath(path.to_path_buf()))
    }
}

fn remove_dir_if_exists(path: &Path, operation: &'static str) -> Result<(), ManagedWorkspaceError> {
    if trusted_directory_exists(path, operation)? {
        fs::remove_dir_all(path).map_err(|source| ManagedWorkspaceError::Io {
            operation,
            path: path.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

pub(super) fn trusted_directory_exists(
    path: &Path,
    operation: &'static str,
) -> Result<bool, ManagedWorkspaceError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_directory(path, &metadata).map(|()| true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(ManagedWorkspaceError::Io {
            operation,
            path: path.to_path_buf(),
            source,
        }),
    }
}
