use std::fs::File;
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use rustix::fs::{Mode, OFlags, openat};
use sha2::{Digest, Sha256};
use thiserror::Error;

const HASH_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaSourceFingerprint {
    pub(crate) identity: String,
    pub(crate) size_bytes: i64,
    pub(crate) modified_ns: i64,
    pub(crate) changed_ns: i64,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileHandleIdentity {
    identity: String,
    size_bytes: i64,
    modified_ns: i64,
    changed_ns: i64,
}

#[derive(Debug, Error)]
pub(crate) enum MediaSourceFingerprintError {
    #[error("media source fingerprint io error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("media source fingerprint path is not a normal descendant of its root: {0}")]
    InvalidPath(PathBuf),
    #[error("media source fingerprint value is too large: {0}")]
    ValueTooLarge(&'static str),
}

pub(crate) fn fingerprint_media_file(
    path: &Path,
    source_root: &Path,
) -> Result<Option<MediaSourceFingerprint>, MediaSourceFingerprintError> {
    fingerprint_media_file_with_observer(path, source_root, || {})
}

fn fingerprint_media_file_with_observer(
    path: &Path,
    source_root: &Path,
    mut after_first_read: impl FnMut(),
) -> Result<Option<MediaSourceFingerprint>, MediaSourceFingerprintError> {
    let Some(mut file) = open_file_beneath_root(path, source_root)? else {
        return Ok(None);
    };
    let before = identity_from_handle(&file)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES].into_boxed_slice();
    let mut first_read = true;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| MediaSourceFingerprintError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        if first_read {
            first_read = false;
            after_first_read();
        }
    }
    let after = identity_from_handle(&file)?;
    if before != after {
        return Ok(None);
    }
    let Some(current_file) = open_file_beneath_root(path, source_root)? else {
        return Ok(None);
    };
    if identity_from_handle(&current_file)? != after {
        return Ok(None);
    }
    Ok(Some(MediaSourceFingerprint {
        identity: after.identity,
        size_bytes: after.size_bytes,
        modified_ns: after.modified_ns,
        changed_ns: after.changed_ns,
        sha256: format!("{:x}", hasher.finalize()),
    }))
}

#[cfg(test)]
fn fingerprint_matches(
    path: &Path,
    source_root: &Path,
    expected: &MediaSourceFingerprint,
) -> Result<bool, MediaSourceFingerprintError> {
    Ok(fingerprint_media_file(path, source_root)?.as_ref() == Some(expected))
}

fn open_file_beneath_root(
    path: &Path,
    source_root: &Path,
) -> Result<Option<File>, MediaSourceFingerprintError> {
    let canonical_root =
        source_root
            .canonicalize()
            .map_err(|source| MediaSourceFingerprintError::Io {
                path: source_root.to_path_buf(),
                source,
            })?;
    let relative = path
        .strip_prefix(source_root)
        .or_else(|_| path.strip_prefix(&canonical_root))
        .map_err(|_| MediaSourceFingerprintError::InvalidPath(path.to_path_buf()))?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err(MediaSourceFingerprintError::InvalidPath(path.to_path_buf())),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let Some((file_name, directories)) = components.split_last() else {
        return Err(MediaSourceFingerprintError::InvalidPath(path.to_path_buf()));
    };
    let root = File::open(&canonical_root).map_err(|source| MediaSourceFingerprintError::Io {
        path: canonical_root.clone(),
        source,
    })?;
    let mut directory: OwnedFd = openat(
        &root,
        ".",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|source| MediaSourceFingerprintError::Io {
        path: canonical_root.clone(),
        source: io::Error::from(source),
    })?;
    for component in directories {
        directory = match openat(
            &directory,
            Path::new(component),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(value) => value,
            Err(source)
                if source == rustix::io::Errno::NOENT
                    || source == rustix::io::Errno::LOOP
                    || source == rustix::io::Errno::NOTDIR =>
            {
                return Ok(None);
            }
            Err(source) => {
                return Err(MediaSourceFingerprintError::Io {
                    path: path.to_path_buf(),
                    source: io::Error::from(source),
                });
            }
        };
    }
    match openat(
        &directory,
        Path::new(file_name),
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(file) => Ok(Some(File::from(file))),
        Err(source) if source == rustix::io::Errno::NOENT || source == rustix::io::Errno::LOOP => {
            Ok(None)
        }
        Err(source) => Err(MediaSourceFingerprintError::Io {
            path: path.to_path_buf(),
            source: io::Error::from(source),
        }),
    }
}

fn identity_from_handle(file: &File) -> Result<FileHandleIdentity, MediaSourceFingerprintError> {
    let metadata = file
        .metadata()
        .map_err(|source| MediaSourceFingerprintError::Io {
            path: PathBuf::from("<opened-media-source>"),
            source,
        })?;
    if !metadata.is_file() {
        return Err(MediaSourceFingerprintError::InvalidPath(PathBuf::from(
            "<opened-media-source>",
        )));
    }
    Ok(FileHandleIdentity {
        identity: format!("{:016x}:{:016x}", metadata.dev(), metadata.ino()),
        size_bytes: i64::try_from(metadata.len())
            .map_err(|_| MediaSourceFingerprintError::ValueTooLarge("size_bytes"))?,
        modified_ns: timestamp_ns(metadata.mtime(), metadata.mtime_nsec(), "modified_ns")?,
        changed_ns: timestamp_ns(metadata.ctime(), metadata.ctime_nsec(), "changed_ns")?,
    })
}

fn timestamp_ns(
    seconds: i64,
    nanoseconds: i64,
    field: &'static str,
) -> Result<i64, MediaSourceFingerprintError> {
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(nanoseconds))
        .ok_or(MediaSourceFingerprintError::ValueTooLarge(field))
}

#[cfg(test)]
mod tests {
    use std::fs::{self, FileTimes};
    use std::io::{Seek, SeekFrom, Write};

    use super::{
        MediaSourceFingerprintError, fingerprint_matches, fingerprint_media_file,
        fingerprint_media_file_with_observer,
    };

    #[test]
    fn fingerprint_media_file_returns_stable_identity_for_regular_file() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("movie.mkv");
        fs::write(&path, b"media")?;

        let fingerprint = fingerprint_media_file(&path, temp.path())?
            .ok_or_else(|| anyhow::anyhow!("stable media file did not produce a fingerprint"))?;

        assert_eq!(fingerprint.size_bytes, 5);
        assert!(fingerprint.modified_ns > 0);
        assert!(fingerprint.changed_ns > 0);
        assert_eq!(fingerprint.identity.len(), 33);
        assert_eq!(
            fingerprint.sha256,
            "721c9525ade2ea8903d343ef25cf68b9bf4ab0aad56bb7b01fbe48d09bc7fcf4"
        );
        assert!(fingerprint_matches(&path, temp.path(), &fingerprint)?);
        Ok(())
    }

    #[test]
    fn fingerprint_rejects_rename_and_symlink_swap() -> anyhow::Result<()> {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir()?;
        let path = temp.path().join("movie.mkv");
        let moved = temp.path().join("moved.mkv");
        let replacement = temp.path().join("replacement.mkv");
        fs::write(&path, vec![b'a'; super::HASH_BUFFER_BYTES + 1])?;
        fs::write(&replacement, vec![b'b'; super::HASH_BUFFER_BYTES + 1])?;

        let fingerprint = fingerprint_media_file_with_observer(&path, temp.path(), || {
            fs::rename(&path, &moved).expect("test rename should succeed");
            symlink(&replacement, &path).expect("test symlink should succeed");
        })?;

        assert!(fingerprint.is_none());
        Ok(())
    }

    #[test]
    fn fingerprint_rejects_ancestor_directory_symlink_swap() -> anyhow::Result<()> {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir()?;
        let directory = temp.path().join("library");
        let moved = temp.path().join("library-moved");
        let replacement = temp.path().join("replacement");
        fs::create_dir(&directory)?;
        fs::create_dir(&replacement)?;
        let path = directory.join("movie.mkv");
        fs::write(&path, vec![b'a'; super::HASH_BUFFER_BYTES + 1])?;
        fs::write(
            replacement.join("movie.mkv"),
            vec![b'b'; super::HASH_BUFFER_BYTES + 1],
        )?;

        let fingerprint = fingerprint_media_file_with_observer(&path, temp.path(), || {
            fs::rename(&directory, &moved).expect("test directory rename should succeed");
            symlink(&replacement, &directory).expect("test ancestor symlink should succeed");
        })?;

        assert!(fingerprint.is_none());
        Ok(())
    }

    #[test]
    fn fingerprint_rejects_same_size_same_mtime_replacement() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("movie.mkv");
        let replacement = temp.path().join("replacement.mkv");
        fs::write(&path, vec![b'a'; super::HASH_BUFFER_BYTES + 1])?;
        fs::write(&replacement, vec![b'b'; super::HASH_BUFFER_BYTES + 1])?;
        let modified = fs::metadata(&path)?.modified()?;
        fs::File::options()
            .write(true)
            .open(&replacement)?
            .set_times(FileTimes::new().set_modified(modified))?;

        let fingerprint = fingerprint_media_file_with_observer(&path, temp.path(), || {
            fs::rename(&replacement, &path).expect("test replacement rename should succeed");
        })?;

        assert!(fingerprint.is_none());
        Ok(())
    }

    #[test]
    fn fingerprint_rejects_mutation_during_hash() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("movie.mkv");
        fs::write(&path, vec![b'a'; super::HASH_BUFFER_BYTES + 1])?;

        let fingerprint = fingerprint_media_file_with_observer(&path, temp.path(), || {
            let mut writer = fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .expect("test mutation open should succeed");
            writer
                .seek(SeekFrom::Start(super::HASH_BUFFER_BYTES as u64))
                .expect("test mutation seek should succeed");
            writer
                .write_all(b"b")
                .expect("test mutation write should succeed");
            writer
                .sync_all()
                .expect("test mutation sync should succeed");
        })?;

        assert!(fingerprint.is_none());
        Ok(())
    }

    #[test]
    fn fingerprint_snapshot_rejects_mutation_after_claim() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("movie.mkv");
        fs::write(&path, b"media-a")?;
        let claimed = fingerprint_media_file(&path, temp.path())?
            .ok_or_else(|| anyhow::anyhow!("source did not produce a claim fingerprint"))?;
        let modified = fs::metadata(&path)?.modified()?;
        let mut file = fs::OpenOptions::new().write(true).open(&path)?;
        file.write_all(b"media-b")?;
        file.set_times(FileTimes::new().set_modified(modified))?;

        assert!(!fingerprint_matches(&path, temp.path(), &claimed)?);
        Ok(())
    }

    #[test]
    fn fingerprint_media_file_returns_none_for_non_file_candidates() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let directory = temp.path().join("nested");
        let missing = temp.path().join("missing.mkv");
        let outside_file = outside.path().join("outside.mkv");
        fs::create_dir_all(&directory)?;
        fs::write(&outside_file, b"media")?;

        assert!(fingerprint_media_file(&missing, temp.path())?.is_none());
        assert!(fingerprint_media_file(&directory, temp.path()).is_err());
        assert!(fingerprint_media_file(&outside_file, temp.path()).is_err());
        Ok(())
    }

    #[test]
    fn fingerprint_media_file_reports_missing_source_root() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("movie.mkv");
        let missing_root = temp.path().join("missing-root");
        fs::write(&path, b"media")?;

        match fingerprint_media_file(&path, &missing_root) {
            Ok(Some(_)) => anyhow::bail!("missing source root produced a fingerprint"),
            Ok(None) => anyhow::bail!("missing source root was treated as an unstable source"),
            Err(MediaSourceFingerprintError::Io { path, source }) => {
                assert_eq!(path, missing_root);
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            Err(unexpected) => anyhow::bail!("unexpected fingerprint error: {unexpected}"),
        }
        Ok(())
    }
}
