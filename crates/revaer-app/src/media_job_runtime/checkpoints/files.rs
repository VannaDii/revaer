//! Integrity checks and removal of exact job-owned execution outputs.

use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

use revaer_data::media::step_checkpoints::StepCheckpoint;
use revaer_media_runtime::workspace::ManagedWorkspace;
use rustix::fs::{AtFlags, Mode, OFlags, open, unlinkat};
use sha2::{Digest, Sha256};

use super::super::{CancellationSignal, ExecutionControl, MediaJobRuntimeError};

const fn failure(source: io::Error) -> MediaJobRuntimeError {
    MediaJobRuntimeError::CheckpointIo(source)
}

fn parent_directory(
    path: &Path,
    workspace: &ManagedWorkspace,
) -> Result<File, MediaJobRuntimeError> {
    workspace.validate()?;
    let relative = path
        .strip_prefix(&workspace.job_path)
        .map_err(|_| MediaJobRuntimeError::InvalidPath("media_checkpoint_output_unowned"))?;
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(MediaJobRuntimeError::InvalidPath(
            "media_checkpoint_output_unowned",
        ));
    }
    let parent = path.parent().ok_or(MediaJobRuntimeError::InvalidPath(
        "media_checkpoint_parent_missing",
    ))?;
    let expected_parent = fs::canonicalize(&workspace.job_path)
        .map_err(failure)?
        .join(relative.parent().ok_or(MediaJobRuntimeError::InvalidPath(
            "media_checkpoint_parent_missing",
        ))?);
    if fs::canonicalize(parent).map_err(failure)? != expected_parent {
        return Err(MediaJobRuntimeError::InvalidPath(
            "media_checkpoint_parent_changed",
        ));
    }
    let directory = open(
        parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| failure(error.into()))?;
    Ok(File::from(directory))
}

pub(super) fn discard(
    path: &Path,
    workspace: &ManagedWorkspace,
) -> Result<(), MediaJobRuntimeError> {
    let directory = parent_directory(path, workspace)?;
    let name = path.file_name().ok_or(MediaJobRuntimeError::InvalidPath(
        "media_checkpoint_output_missing",
    ))?;
    match unlinkat(&directory, name, AtFlags::empty()) {
        Ok(()) => directory.sync_all().map_err(failure)?,
        Err(rustix::io::Errno::NOENT) => {}
        Err(error) => return Err(failure(error.into())),
    }
    workspace.validate()?;
    Ok(())
}

pub(super) fn inspect(
    path: &Path,
    workspace: &ManagedWorkspace,
    signal: &CancellationSignal,
    synchronize: bool,
) -> Result<Option<StepCheckpoint>, MediaJobRuntimeError> {
    let directory = parent_directory(path, workspace)?;
    let fd = match open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        // Missing or symlink outputs cannot be completed regular files.
        Err(rustix::io::Errno::NOENT | rustix::io::Errno::LOOP) => return Ok(None),
        Err(error) => return Err(failure(error.into())),
    };
    let mut file = File::from(fd);
    let before = file.metadata().map_err(failure)?;
    if !before.is_file() {
        return Err(MediaJobRuntimeError::InvalidPath(
            "media_checkpoint_output_not_regular",
        ));
    }
    let digest = digest_file(&mut file, signal)?;
    let after = file.metadata().map_err(failure)?;
    let current = fs::symlink_metadata(path).map_err(failure)?;
    if identity(&before) != identity(&after) || identity(&after) != identity(&current) {
        return Err(MediaJobRuntimeError::InvalidPath(
            "media_checkpoint_output_changed",
        ));
    }
    if synchronize {
        file.sync_all().map_err(failure)?;
        directory.sync_all().map_err(failure)?;
    }
    workspace.validate()?;
    Ok(Some(StepCheckpoint {
        step_signature: Vec::new(),
        output_path: path
            .to_str()
            .ok_or(MediaJobRuntimeError::InvalidPath(
                "media_checkpoint_path_not_utf8",
            ))?
            .to_owned(),
        size_bytes: i64::try_from(after.len())
            .map_err(|_| MediaJobRuntimeError::SourceSizeOverflow)?,
        output_sha256: digest,
    }))
}

fn identity(metadata: &fs::Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

fn digest_file(
    file: &mut File,
    signal: &CancellationSignal,
) -> Result<Vec<u8>, MediaJobRuntimeError> {
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    loop {
        if signal.cancellation_requested() {
            return Err(MediaJobRuntimeError::Cancelled);
        }
        let count = file.read(&mut buffer).map_err(failure)?;
        if count == 0 {
            return Ok(digest.finalize().to_vec());
        }
        digest.update(&buffer[..count]);
    }
}
