//! Filesystem capacity probes.

use std::path::Path;

use crate::error::{FsOpsError, FsOpsResult};

/// Return available bytes for the filesystem containing `path`.
///
/// # Errors
///
/// Returns [`FsOpsError`] when the filesystem cannot be queried.
pub fn available_bytes(path: impl AsRef<Path>) -> FsOpsResult<u64> {
    let path = path.as_ref();
    let report = nix::sys::statvfs::statvfs(path).map_err(|source| FsOpsError::Nix {
        operation: "capacity.statvfs",
        path: path.to_path_buf(),
        source,
    })?;
    let available_bytes = u128::from(report.fragment_size())
        .checked_mul(u128::from(report.blocks_available()))
        .ok_or_else(|| FsOpsError::InvalidInput {
            field: "available_bytes",
            reason: "capacity_overflow",
            value: Some(path.display().to_string()),
        })?;
    u64::try_from(available_bytes).map_err(|_| FsOpsError::InvalidInput {
        field: "available_bytes",
        reason: "capacity_overflow",
        value: Some(path.display().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::available_bytes;

    use crate::error::FsOpsError;

    #[test]
    fn available_bytes_reports_capacity_for_existing_path() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let available = available_bytes(temp.path())?;
        assert!(available > 0);
        Ok(())
    }

    #[test]
    fn available_bytes_reports_missing_path() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let missing = temp.path().join("missing");
        let result = available_bytes(&missing);

        assert!(matches!(
            result,
            Err(FsOpsError::Nix {
                operation: "capacity.statvfs",
                path,
                ..
            }) if path == missing
        ));
        Ok(())
    }
}
