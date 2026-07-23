//! Managed workspace creation and terminal cleanup.

use std::fs;
use std::fs::File;
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustix::fs::{CWD, Mode, OFlags, mkdirat, openat};
use rustix::process::geteuid;

use super::model::{WorkspaceDirectoryIdentity, WorkspaceTrust};
use super::{
    ManagedWorkspace, ManagedWorkspaceError, TerminalWorkspaceCleanupPolicy,
    TerminalWorkspaceState, WorkspacePaths,
};

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;

/// Project deterministic workspace paths without touching the filesystem.
///
/// # Errors
///
/// Returns [`ManagedWorkspaceError`] when the root or job key is invalid.
pub fn project_managed_workspace(
    root_path: impl AsRef<Path>,
    job_key: &str,
) -> Result<WorkspacePaths, ManagedWorkspaceError> {
    let root_path = normalize_workspace_root(root_path.as_ref())?;
    let job_key = normalize_workspace_job_key(job_key)?;
    let job_path = root_path.join(&job_key);
    Ok(WorkspacePaths {
        job_key,
        root_path,
        input_path: job_path.join("input"),
        output_path: job_path.join("output"),
        diagnostics_path: job_path.join("diagnostics"),
        job_path,
    })
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
    let paths = project_managed_workspace(root_path, job_key)?;
    ensure_private_root(&paths.root_path)?;
    let root = open_directory_at(
        CWD,
        &paths.root_path,
        &paths.root_path,
        "workspace.open_root",
    )?;
    let root_identity = private_directory_identity(&paths.root_path, &root)?;
    create_private_directory_at(
        &root,
        &paths.job_key,
        &paths.job_path,
        "workspace.create_job",
    )?;
    let job = open_directory_at(&root, &paths.job_key, &paths.job_path, "workspace.open_job")?;
    let job_identity = private_directory_identity(&paths.job_path, &job)?;
    create_private_directory_at(&job, "input", &paths.input_path, "workspace.create_input")?;
    create_private_directory_at(
        &job,
        "output",
        &paths.output_path,
        "workspace.create_output",
    )?;
    create_private_directory_at(
        &job,
        "diagnostics",
        &paths.diagnostics_path,
        "workspace.create_diagnostics",
    )?;
    let input = open_directory_at(&job, "input", &paths.input_path, "workspace.open_input")?;
    let output = open_directory_at(&job, "output", &paths.output_path, "workspace.open_output")?;
    let diagnostics = open_directory_at(
        &job,
        "diagnostics",
        &paths.diagnostics_path,
        "workspace.open_diagnostics",
    )?;
    let workspace = ManagedWorkspace {
        trust: Arc::new(WorkspaceTrust {
            root: Arc::new(root),
            root_identity,
            job_identity,
            input_identity: private_directory_identity(&paths.input_path, &input)?,
            output_identity: private_directory_identity(&paths.output_path, &output)?,
            diagnostics_identity: private_directory_identity(
                &paths.diagnostics_path,
                &diagnostics,
            )?,
        }),
        paths,
    };
    validate_managed_workspace(&workspace)?;
    Ok(workspace)
}

/// Remove one managed per-job workspace if it exists.
///
/// # Errors
///
/// Returns [`ManagedWorkspaceError::Io`] when existence checks or removal fail.
pub fn teardown_managed_workspace(
    workspace: &ManagedWorkspace,
) -> Result<(), ManagedWorkspaceError> {
    validate_managed_workspace(workspace)?;
    remove_dir_if_exists(&workspace.paths.job_path, "workspace.teardown_remove")
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
            validate_managed_workspace(workspace)?;
            remove_dir_if_exists(
                &workspace.paths.input_path,
                "workspace.terminal_remove_input",
            )?;
            remove_dir_if_exists(
                &workspace.paths.output_path,
                "workspace.terminal_remove_output",
            )?;
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
    if !path.is_absolute() {
        return Err(ManagedWorkspaceError::UnsafePath(path.to_path_buf()));
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

fn ensure_private_root(path: &Path) -> Result<(), ManagedWorkspaceError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_private_directory(path, &metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Err(source) = fs::DirBuilder::new()
                .mode(PRIVATE_DIRECTORY_MODE)
                .create(path)
            {
                return Err(ManagedWorkspaceError::Io {
                    operation: "workspace.create_root",
                    path: path.to_path_buf(),
                    source,
                });
            }
            let metadata =
                fs::symlink_metadata(path).map_err(|source| ManagedWorkspaceError::Io {
                    operation: "workspace.create_root",
                    path: path.to_path_buf(),
                    source,
                })?;
            validate_private_directory(path, &metadata)
        }
        Err(source) => Err(ManagedWorkspaceError::Io {
            operation: "workspace.inspect_root",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn create_private_directory_at(
    parent: &File,
    name: &str,
    path: &Path,
    operation: &'static str,
) -> Result<(), ManagedWorkspaceError> {
    mkdirat(parent, name, Mode::RWXU).map_err(|source| {
        if source == rustix::io::Errno::EXIST {
            ManagedWorkspaceError::UnsafePath(path.to_path_buf())
        } else {
            ManagedWorkspaceError::Io {
                operation,
                path: path.to_path_buf(),
                source: io::Error::from_raw_os_error(source.raw_os_error()),
            }
        }
    })
}

fn open_directory_at(
    parent: impl std::os::fd::AsFd,
    name: impl rustix::path::Arg,
    path: &Path,
    operation: &'static str,
) -> Result<File, ManagedWorkspaceError> {
    openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(|source| ManagedWorkspaceError::Io {
        operation,
        path: path.to_path_buf(),
        source: io::Error::from_raw_os_error(source.raw_os_error()),
    })
}

fn private_directory_identity(
    path: &Path,
    directory: &File,
) -> Result<WorkspaceDirectoryIdentity, ManagedWorkspaceError> {
    let metadata = directory
        .metadata()
        .map_err(|source| ManagedWorkspaceError::Io {
            operation: "workspace.inspect_handle",
            path: path.to_path_buf(),
            source,
        })?;
    validate_private_directory(path, &metadata)?;
    Ok(directory_identity(&metadata))
}

fn directory_identity(metadata: &fs::Metadata) -> WorkspaceDirectoryIdentity {
    WorkspaceDirectoryIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        owner: metadata.uid(),
        mode: metadata.mode() & 0o777,
    }
}

fn validate_private_directory(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), ManagedWorkspaceError> {
    validate_directory(path, metadata)?;
    if metadata.uid() != geteuid().as_raw() || metadata.mode() & 0o777 != PRIVATE_DIRECTORY_MODE {
        return Err(ManagedWorkspaceError::UnsafeDirectoryPolicy(
            path.to_path_buf(),
        ));
    }
    Ok(())
}

fn validate_identity(
    path: &Path,
    expected: WorkspaceDirectoryIdentity,
) -> Result<(), ManagedWorkspaceError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| ManagedWorkspaceError::Io {
        operation: "workspace.validate_identity",
        path: path.to_path_buf(),
        source,
    })?;
    validate_private_directory(path, &metadata)?;
    if directory_identity(&metadata) != expected {
        return Err(ManagedWorkspaceError::IdentityChanged(path.to_path_buf()));
    }
    Ok(())
}

pub(super) fn validate_managed_workspace(
    workspace: &ManagedWorkspace,
) -> Result<(), ManagedWorkspaceError> {
    let root_handle_identity =
        private_directory_identity(&workspace.paths.root_path, workspace.trust.root.as_ref())?;
    if root_handle_identity != workspace.trust.root_identity {
        return Err(ManagedWorkspaceError::IdentityChanged(
            workspace.paths.root_path.clone(),
        ));
    }
    validate_identity(&workspace.paths.root_path, workspace.trust.root_identity)?;
    validate_identity(&workspace.paths.job_path, workspace.trust.job_identity)?;
    validate_identity(&workspace.paths.input_path, workspace.trust.input_identity)?;
    validate_identity(
        &workspace.paths.output_path,
        workspace.trust.output_identity,
    )?;
    validate_identity(
        &workspace.paths.diagnostics_path,
        workspace.trust.diagnostics_identity,
    )
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
