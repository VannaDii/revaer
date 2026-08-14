//! Descriptor-bound identity helpers for regular media source files.

use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::os::fd::AsFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use rustix::fs::{Mode, OFlags, openat};

use crate::media_discovery_fingerprint::FingerprintError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) size_bytes: u64,
    pub(crate) modified_ns: i64,
    pub(crate) changed_ns: i64,
}

pub(crate) fn open_regular_at(
    directory: impl AsFd,
    name: &OsStr,
    display_path: &Path,
) -> Result<Option<File>, FingerprintError> {
    match openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(file) => {
            let file = File::from(file);
            match file_identity(&file, display_path) {
                Ok(_) => Ok(Some(file)),
                Err(FingerprintError::InvalidPath(_)) => Ok(None),
                Err(error) => Err(error),
            }
        }
        Err(rustix::io::Errno::NOENT | rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) => {
            Ok(None)
        }
        Err(source) => Err(FingerprintError::Io {
            path: display_path.to_path_buf(),
            source: io::Error::from(source),
        }),
    }
}

pub(crate) fn file_identity(file: &File, path: &Path) -> Result<FileIdentity, FingerprintError> {
    let metadata = file.metadata().map_err(|source| FingerprintError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(FingerprintError::InvalidPath(path.to_path_buf()));
    }
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        size_bytes: metadata.len(),
        modified_ns: timestamp_ns(metadata.mtime(), metadata.mtime_nsec(), "modified_ns")?,
        changed_ns: timestamp_ns(metadata.ctime(), metadata.ctime_nsec(), "changed_ns")?,
    })
}

fn timestamp_ns(
    seconds: i64,
    nanoseconds: i64,
    field: &'static str,
) -> Result<i64, FingerprintError> {
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(nanoseconds))
        .ok_or(FingerprintError::ValueTooLarge(field))
}
