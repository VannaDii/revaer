//! Live workspace usage and free-space supervision.

use super::WorkspacePolicy;
use crate::execute::{ExecutionControl, ExecutionLimitBreach};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MAX_WORKSPACE_ENTRIES_PER_SAMPLE: usize = 100_000;

/// One atomic live workspace-capacity observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceBudgetSample {
    /// Cumulative logical bytes under the job workspace.
    pub workspace_bytes: u64,
    /// Bytes currently available to an unprivileged process on the workspace filesystem.
    pub free_bytes: u64,
}

/// Injected live workspace-capacity probe.
pub trait WorkspaceBudgetProbe: Send + Sync {
    /// Sample cumulative workspace bytes and filesystem free space.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when either value cannot be established completely.
    fn sample(&self, workspace_root: &Path) -> Result<WorkspaceBudgetSample, io::Error>;
}

/// Filesystem-backed live workspace-capacity probe.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemWorkspaceBudgetProbe;

impl WorkspaceBudgetProbe for SystemWorkspaceBudgetProbe {
    fn sample(&self, workspace_root: &Path) -> Result<WorkspaceBudgetSample, io::Error> {
        let mut examined_entries = 0_usize;
        let workspace_bytes = logical_workspace_bytes(workspace_root, &mut examined_entries)?;
        let filesystem = rustix::fs::statvfs(workspace_root).map_err(rustix_error)?;
        Ok(WorkspaceBudgetSample {
            workspace_bytes,
            free_bytes: filesystem.f_bavail.saturating_mul(filesystem.f_frsize),
        })
    }
}

/// Execution control that combines operator cancellation with live workspace limits.
pub struct WorkspaceBudgetControl<'a> {
    workspace_root: PathBuf,
    policy: &'a WorkspacePolicy,
    cancellation: &'a dyn ExecutionControl,
    probe: &'a dyn WorkspaceBudgetProbe,
}

impl<'a> WorkspaceBudgetControl<'a> {
    /// Build an injected live workspace control.
    #[must_use]
    pub fn new(
        workspace_root: &Path,
        policy: &'a WorkspacePolicy,
        cancellation: &'a dyn ExecutionControl,
        probe: &'a dyn WorkspaceBudgetProbe,
    ) -> Self {
        Self {
            workspace_root: workspace_root.to_path_buf(),
            policy,
            cancellation,
            probe,
        }
    }
}

impl ExecutionControl for WorkspaceBudgetControl<'_> {
    fn cancellation_requested(&self) -> bool {
        self.cancellation.cancellation_requested()
    }

    fn limit_breach(&self) -> Option<ExecutionLimitBreach> {
        match self.probe.sample(&self.workspace_root) {
            Ok(sample) if sample.workspace_bytes > self.policy.max_bytes => {
                Some(ExecutionLimitBreach::WorkspaceBytesExceeded)
            }
            Ok(sample) if sample.free_bytes < self.policy.reserve_bytes => {
                Some(ExecutionLimitBreach::WorkspaceReserveLost)
            }
            Ok(_) => self.cancellation.limit_breach(),
            Err(error) => Some(ExecutionLimitBreach::WorkspaceProbeFailed(error.kind())),
        }
    }
}

fn logical_workspace_bytes(path: &Path, examined_entries: &mut usize) -> io::Result<u64> {
    *examined_entries = examined_entries.saturating_add(1);
    if *examined_entries > MAX_WORKSPACE_ENTRIES_PER_SAMPLE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "workspace entry limit exceeded",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "workspace contains a symlink",
        ));
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "workspace contains an unsupported filesystem entry",
        ));
    }
    let mut bytes = 0_u64;
    for entry in fs::read_dir(path)? {
        bytes = bytes.saturating_add(logical_workspace_bytes(&entry?.path(), examined_entries)?);
    }
    Ok(bytes)
}

fn rustix_error(error: rustix::io::Errno) -> io::Error {
    io::Error::from_raw_os_error(error.raw_os_error())
}

#[cfg(test)]
mod tests {
    use super::{
        SystemWorkspaceBudgetProbe, WorkspaceBudgetControl, WorkspaceBudgetProbe,
        WorkspaceBudgetSample,
    };
    use crate::execute::{ExecutionControl, ExecutionLimitBreach};
    use crate::workspace::WorkspacePolicy;
    use std::fs::{self, OpenOptions};
    use std::io;
    use std::path::Path;

    struct NeverCancel;

    impl ExecutionControl for NeverCancel {
        fn cancellation_requested(&self) -> bool {
            false
        }
    }

    struct FixedProbe(Result<WorkspaceBudgetSample, io::ErrorKind>);

    impl WorkspaceBudgetProbe for FixedProbe {
        fn sample(&self, _workspace_root: &Path) -> Result<WorkspaceBudgetSample, io::Error> {
            self.0.map_err(|kind| io::Error::new(kind, "probe failure"))
        }
    }

    #[test]
    fn sparse_files_count_against_logical_workspace_budget() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let sparse = root.path().join("sparse-output.bin");
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&sparse)?;
        file.set_len(64 * 1024 * 1024)?;

        let sample = SystemWorkspaceBudgetProbe.sample(root.path())?;

        assert_eq!(sample.workspace_bytes, 64 * 1024 * 1024);
        fs::remove_file(sparse)?;
        Ok(())
    }

    #[test]
    fn control_reports_bytes_reserve_and_probe_failures() {
        let policy = WorkspacePolicy {
            max_bytes: 1_000,
            reserve_bytes: 500,
        };
        let root = Path::new("/unused-in-injected-probe");
        let bytes_probe = FixedProbe(Ok(WorkspaceBudgetSample {
            workspace_bytes: 1_001,
            free_bytes: 10_000,
        }));
        let reserve_probe = FixedProbe(Ok(WorkspaceBudgetSample {
            workspace_bytes: 100,
            free_bytes: 499,
        }));
        let failed_probe = FixedProbe(Err(io::ErrorKind::PermissionDenied));

        assert_eq!(
            WorkspaceBudgetControl::new(root, &policy, &NeverCancel, &bytes_probe).limit_breach(),
            Some(ExecutionLimitBreach::WorkspaceBytesExceeded)
        );
        assert_eq!(
            WorkspaceBudgetControl::new(root, &policy, &NeverCancel, &reserve_probe).limit_breach(),
            Some(ExecutionLimitBreach::WorkspaceReserveLost)
        );
        assert_eq!(
            WorkspaceBudgetControl::new(root, &policy, &NeverCancel, &failed_probe).limit_breach(),
            Some(ExecutionLimitBreach::WorkspaceProbeFailed(
                io::ErrorKind::PermissionDenied
            ))
        );
    }
}
