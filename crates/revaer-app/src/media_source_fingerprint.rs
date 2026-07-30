use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) struct MediaSourceFingerprint {
    pub(crate) size_bytes: i64,
    pub(crate) modified_ns: i64,
    pub(crate) sha256: String,
}

#[derive(Debug, Error)]
pub(crate) enum MediaSourceFingerprintError {
    #[error("media source fingerprint io error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("media source fingerprint timestamp predates the Unix epoch")]
    TimeBeforeEpoch,
    #[error("media source fingerprint value is too large: {0}")]
    ValueTooLarge(&'static str),
}

pub(crate) fn fingerprint_media_file(
    path: &Path,
    source_root: &Path,
) -> Result<Option<MediaSourceFingerprint>, MediaSourceFingerprintError> {
    let canonical_root =
        source_root
            .canonicalize()
            .map_err(|source| MediaSourceFingerprintError::Io {
                path: source_root.to_path_buf(),
                source,
            })?;
    let canonical_path = match path.canonicalize() {
        Ok(value) => value,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(MediaSourceFingerprintError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if !canonical_path.starts_with(&canonical_root) {
        return Ok(None);
    }
    let before =
        fs::metadata(&canonical_path).map_err(|source| MediaSourceFingerprintError::Io {
            path: canonical_path.clone(),
            source,
        })?;
    if !before.is_file() {
        return Ok(None);
    }
    let before_modified = before
        .modified()
        .map_err(|source| MediaSourceFingerprintError::Io {
            path: canonical_path.clone(),
            source,
        })?;
    let mut file =
        File::open(&canonical_path).map_err(|source| MediaSourceFingerprintError::Io {
            path: canonical_path.clone(),
            source,
        })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| MediaSourceFingerprintError::Io {
                path: canonical_path.clone(),
                source,
            })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let after =
        fs::metadata(&canonical_path).map_err(|source| MediaSourceFingerprintError::Io {
            path: canonical_path.clone(),
            source,
        })?;
    let after_modified = after
        .modified()
        .map_err(|source| MediaSourceFingerprintError::Io {
            path: canonical_path.clone(),
            source,
        })?;
    if before.len() != after.len() || before_modified != after_modified {
        return Ok(None);
    }
    let size_bytes = i64::try_from(after.len())
        .map_err(|_| MediaSourceFingerprintError::ValueTooLarge("size_bytes"))?;
    let modified_ns = system_time_ns(after_modified)?;
    Ok(Some(MediaSourceFingerprint {
        size_bytes,
        modified_ns,
        sha256: format!("{:x}", hasher.finalize()),
    }))
}

fn system_time_ns(value: SystemTime) -> Result<i64, MediaSourceFingerprintError> {
    let duration = value
        .duration_since(UNIX_EPOCH)
        .map_err(|_| MediaSourceFingerprintError::TimeBeforeEpoch)?;
    i64::try_from(duration.as_nanos())
        .map_err(|_| MediaSourceFingerprintError::ValueTooLarge("modified_ns"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{MediaSourceFingerprintError, fingerprint_media_file};

    #[test]
    fn fingerprint_media_file_returns_stable_identity_for_regular_file() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("movie.mkv");
        fs::write(&path, b"media")?;

        let fingerprint = fingerprint_media_file(&path, temp.path())?
            .ok_or_else(|| anyhow::anyhow!("stable media file did not produce a fingerprint"))?;

        assert_eq!(fingerprint.size_bytes, 5);
        assert!(fingerprint.modified_ns > 0);
        assert_eq!(
            fingerprint.sha256,
            "721c9525ade2ea8903d343ef25cf68b9bf4ab0aad56bb7b01fbe48d09bc7fcf4"
        );
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
        assert!(fingerprint_media_file(&directory, temp.path())?.is_none());
        assert!(fingerprint_media_file(&outside_file, temp.path())?.is_none());
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
