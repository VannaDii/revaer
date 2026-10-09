//! Descriptor-relative capability checks, not root or deployment attestation.

use std::{
    fs::File,
    io::{self, Read, Seek, Write},
    sync::atomic::{AtomicU64, Ordering},
};

use rustix::fs::{self, AtFlags, Mode, OFlags};
use thiserror::Error;

static NEXT_PROBE: AtomicU64 = AtomicU64::new(0);
const CONTENT: &[u8] = b"revaer root capability probe\n";

/// A capability failure, preserving cleanup failures without exposing root paths.
#[derive(Debug, Error)]
pub enum RootWriteProbeError {
    /// A required operation failed. Existing colliding entries are never removed.
    #[error("root capability probe failed during {operation}")]
    Operation {
        /// Closed operation label, never caller input.
        operation: &'static str,
        /// Original operating-system failure.
        #[source]
        source: io::Error,
    },
    /// Owned probe cleanup failed; the original failure is retained too.
    #[error("root capability probe cleanup failed")]
    Cleanup {
        /// Original probe failure, absent if only cleanup failed.
        original: Option<Box<Self>>,
        /// Every failed cleanup operation, in attempted order.
        failures: Vec<Self>,
    },
}

/// Exercise create-new, write/readback, file/directory sync, no-replace rename,
/// deletion and capacity inspection using only an already-open root descriptor.
///
/// The caller must first prove directory identity, protected ancestry and
/// deployment ownership, and hold the process-lifetime exclusive root lock.
/// This check alone grants neither root readiness nor persistence authority.
/// It creates only a private probe directory and never opens source media.
/// Every owned entry is removed on success or failure; cleanup failure is fatal.
///
/// # Errors
/// Returns a bounded failure for an unsupported capability, collision, probe
/// mismatch or cleanup error. Never recursively removes a failed probe tree.
pub(super) fn probe_root_writes(root: &File) -> Result<(), RootWriteProbeError> {
    let sequence = NEXT_PROBE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| {
            failure(
                "allocate probe identity",
                io::Error::other("probe identity exhausted"),
            )
        })?;
    let name = format!(".revaer-root-probe-{}-{sequence}", std::process::id());
    probe(root, &name, |_| Ok(()))
}

fn probe(
    root: &File,
    name: &str,
    mut observe: impl FnMut(&'static str) -> io::Result<()>,
) -> Result<(), RootWriteProbeError> {
    fs::mkdirat(root, name, Mode::RWXU)
        .map_err(|error| failure("create probe directory", error))?;
    let mut owned_file = None;
    let directory = fs::openat(
        root,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    );
    let result = match &directory {
        Ok(directory) => run_probe(directory, &mut owned_file, &mut observe),
        Err(error) => Err(failure("open probe directory", *error)),
    };
    let mut failures = Vec::new();
    if let (Ok(directory), Some(file)) = (&directory, owned_file)
        && let Err(error) = fs::unlinkat(directory, file, AtFlags::empty())
    {
        failures.push(failure("remove probe file", error));
    }
    if let Err(error) = fs::unlinkat(root, name, AtFlags::REMOVEDIR) {
        failures.push(failure("remove probe directory", error));
    }
    if let Err(error) = fs::fsync(root) {
        failures.push(failure("sync probe cleanup", error));
    }
    if failures.is_empty() {
        result
    } else {
        Err(RootWriteProbeError::Cleanup {
            original: result.err().map(Box::new),
            failures,
        })
    }
}

fn run_probe(
    directory: &rustix::fd::OwnedFd,
    owned_file: &mut Option<&'static str>,
    observe: &mut impl FnMut(&'static str) -> io::Result<()>,
) -> Result<(), RootWriteProbeError> {
    let descriptor = fs::openat(
        directory,
        "before",
        OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(|error| failure("create probe file", error))?;
    *owned_file = Some("before");
    let mut file = File::from(descriptor);
    file.write_all(CONTENT)
        .map_err(|error| failure("write probe file", error))?;
    file.sync_all()
        .map_err(|error| failure("sync probe file", error))?;
    file.rewind()
        .map_err(|error| failure("rewind probe file", error))?;
    let mut actual = [0_u8; CONTENT.len()];
    file.read_exact(&mut actual)
        .map_err(|error| failure("read probe file", error))?;
    if actual != CONTENT {
        return Err(failure(
            "verify probe file",
            io::Error::other("probe content mismatch"),
        ));
    }
    observe("written").map_err(|error| failure("observe written probe", error))?;
    fs::fsync(directory).map_err(|error| failure("sync probe directory", error))?;
    fs::renameat_with(
        directory,
        "before",
        directory,
        "after",
        fs::RenameFlags::NOREPLACE,
    )
    .map_err(|error| failure("rename probe file", error))?;
    *owned_file = Some("after");
    observe("renamed").map_err(|error| failure("observe renamed probe", error))?;
    fs::fsync(directory).map_err(|error| failure("sync probe rename", error))?;
    let capacity =
        fs::fstatvfs(directory).map_err(|error| failure("inspect probe capacity", error))?;
    if capacity.f_frsize == 0 || capacity.f_frsize.checked_mul(capacity.f_bavail).is_none() {
        return Err(failure(
            "validate probe capacity",
            io::Error::other("invalid capacity result"),
        ));
    }
    fs::unlinkat(directory, "after", AtFlags::empty())
        .map_err(|error| failure("delete probe file", error))?;
    *owned_file = None;
    fs::fsync(directory).map_err(|error| failure("sync probe deletion", error))?;
    Ok(())
}

fn failure(operation: &'static str, source: impl Into<io::Error>) -> RootWriteProbeError {
    RootWriteProbeError::Operation {
        operation,
        source: source.into(),
    }
}

#[cfg(test)]
mod tests;
