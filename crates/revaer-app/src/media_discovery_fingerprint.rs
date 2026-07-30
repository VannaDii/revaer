//! Aggregate media identity over a container and its owned subtitle sidecars.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::media_discovery_runtime::is_media_file;
use crate::media_source_fingerprint::{MediaSourceFingerprintError, fingerprint_media_file};

const SIDECAR_EXTENSIONS: &[&str] = &["ass", "idx", "srt", "ssa", "sub", "sup", "vtt"];

pub(crate) struct MediaAggregateFingerprint {
    pub(crate) identity: String,
    pub(crate) size_bytes: i64,
    pub(crate) modified_ns: i64,
    pub(crate) changed_ns: i64,
    pub(crate) sha256: String,
}

#[derive(Debug, Error)]
pub(crate) enum FingerprintError {
    #[error("media discovery fingerprint io error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("media discovery fingerprint timestamp predates the Unix epoch")]
    TimeBeforeEpoch,
    #[error("media discovery fingerprint value is too large: {0}")]
    ValueTooLarge(&'static str),
    #[error(transparent)]
    Source(#[from] MediaSourceFingerprintError),
}

pub(crate) fn owner_for_changed_path(path: &Path, root: &Path) -> Option<PathBuf> {
    if is_media_file(path) {
        return Some(path.to_path_buf());
    }
    if !is_sidecar(path) {
        return None;
    }
    let parent = path.parent()?;
    let sidecar_stem = path.file_stem()?.to_str()?;
    let mut matches = fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| is_media_file(candidate))
        .filter_map(|candidate| {
            let stem = candidate.file_stem()?.to_str()?;
            let stem_len = stem.len();
            sidecar_stem
                .strip_prefix(stem)
                .filter(|suffix| suffix.is_empty() || suffix.starts_with('.'))
                .map(|_| (stem_len, candidate))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let longest = matches.first()?.0;
    let mut owners = matches.into_iter().filter(|item| item.0 == longest);
    let owner = owners.next()?.1;
    if owners.next().is_some() {
        return None;
    }
    owner
        .strip_prefix(root)
        .ok()
        .map(|relative| root.join(relative))
}

pub(crate) fn fingerprint_media_aggregate(
    media_path: &Path,
    root: &Path,
) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
    let Some(source_fingerprint) = fingerprint_media_file(media_path, root)? else {
        return Ok(None);
    };
    let canonical_root = canonical(root)?;
    let canonical_media = match media_path.canonicalize() {
        Ok(path) if path.starts_with(&canonical_root) => path,
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(FingerprintError::Io {
                path: media_path.into(),
                source,
            });
        }
    };
    if !fs::symlink_metadata(media_path)
        .map_err(|source| FingerprintError::Io {
            path: media_path.into(),
            source,
        })?
        .file_type()
        .is_file()
    {
        return Ok(None);
    }
    let mut members = vec![canonical_media.clone()];
    if let Some(parent) = canonical_media.parent() {
        for entry in fs::read_dir(parent).map_err(|source| FingerprintError::Io {
            path: parent.into(),
            source,
        })? {
            let path = entry
                .map_err(|source| FingerprintError::Io {
                    path: parent.into(),
                    source,
                })?
                .path();
            if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_file())
                && is_sidecar(&path)
                && owner_for_changed_path(&path, &canonical_root).as_deref()
                    == Some(canonical_media.as_path())
            {
                members.push(path);
            }
        }
    }
    members.sort();
    let mut aggregate = Sha256::new();
    let mut total_size = 0_u64;
    let mut latest_modified = UNIX_EPOCH;
    for member in members {
        let before = metadata(&member)?;
        let modified = before.modified().map_err(|source| FingerprintError::Io {
            path: member.clone(),
            source,
        })?;
        let relative = member
            .strip_prefix(&canonical_root)
            .unwrap_or(member.as_path());
        aggregate.update(relative.as_os_str().as_encoded_bytes());
        let mut file = File::open(&member).map_err(|source| FingerprintError::Io {
            path: member.clone(),
            source,
        })?;
        let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|source| FingerprintError::Io {
                    path: member.clone(),
                    source,
                })?;
            if read == 0 {
                break;
            }
            aggregate.update(&buffer[..read]);
        }
        let after = metadata(&member)?;
        if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
            return Ok(None);
        }
        total_size = total_size.saturating_add(after.len());
        latest_modified = latest_modified.max(modified);
    }
    Ok(Some(MediaAggregateFingerprint {
        identity: source_fingerprint.identity,
        size_bytes: i64::try_from(total_size)
            .map_err(|_| FingerprintError::ValueTooLarge("size"))?,
        modified_ns: i64::try_from(
            latest_modified
                .duration_since(UNIX_EPOCH)
                .map_err(|_| FingerprintError::TimeBeforeEpoch)?
                .as_nanos(),
        )
        .map_err(|_| FingerprintError::ValueTooLarge("modified_ns"))?,
        changed_ns: source_fingerprint.changed_ns,
        sha256: format!("{:x}", aggregate.finalize()),
    }))
}

pub(crate) fn revalidate_media_aggregate(
    media_path: &Path,
    root: &Path,
    expected_sha256: &str,
) -> Result<bool, FingerprintError> {
    Ok(fingerprint_media_aggregate(media_path, root)?
        .is_some_and(|fingerprint| fingerprint.sha256 == expected_sha256))
}

fn metadata(path: &Path) -> Result<fs::Metadata, FingerprintError> {
    fs::metadata(path).map_err(|source| FingerprintError::Io {
        path: path.into(),
        source,
    })
}

fn canonical(path: &Path) -> Result<PathBuf, FingerprintError> {
    path.canonicalize().map_err(|source| FingerprintError::Io {
        path: path.into(),
        source,
    })
}

fn is_sidecar(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            SIDECAR_EXTENSIONS
                .iter()
                .any(|extension| value.eq_ignore_ascii_case(extension))
        })
}

#[cfg(test)]
mod tests {
    use super::{fingerprint_media_aggregate, owner_for_changed_path, revalidate_media_aggregate};
    use std::fs;

    #[test]
    fn sidecars_change_identity_and_vobsub_pairs_share_one_owner() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let media = temp.path().join("movie.mkv");
        let idx = temp.path().join("movie.idx");
        let sub = temp.path().join("movie.sub");
        fs::write(&media, b"media")?;
        fs::write(&idx, b"index")?;
        fs::write(&sub, b"bitmap")?;
        assert_eq!(
            owner_for_changed_path(&idx, temp.path()),
            Some(media.clone())
        );
        assert_eq!(
            owner_for_changed_path(&sub, temp.path()),
            Some(media.clone())
        );
        let first = fingerprint_media_aggregate(&media, temp.path())?
            .ok_or_else(|| anyhow::anyhow!("missing fingerprint"))?;
        fs::write(&sub, b"changed")?;
        let changed = fingerprint_media_aggregate(&media, temp.path())?
            .ok_or_else(|| anyhow::anyhow!("missing fingerprint"))?;
        assert_ne!(first.sha256, changed.sha256);
        fs::remove_file(&idx)?;
        let deleted = fingerprint_media_aggregate(&media, temp.path())?
            .ok_or_else(|| anyhow::anyhow!("missing fingerprint"))?;
        assert_ne!(changed.sha256, deleted.sha256);
        fs::write(temp.path().join("movie.mp4"), b"other")?;
        assert_eq!(owner_for_changed_path(&sub, temp.path()), None);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn aggregate_revalidation_rejects_symlink_rename_create_and_delete_swaps() -> anyhow::Result<()>
    {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir()?;
        let media = temp.path().join("movie.mkv");
        let sidecar = temp.path().join("movie.srt");
        fs::write(&media, b"media")?;
        fs::write(&sidecar, b"subtitle")?;
        let expected = fingerprint_media_aggregate(&media, temp.path())?
            .ok_or_else(|| anyhow::anyhow!("missing aggregate"))?
            .sha256;
        assert!(revalidate_media_aggregate(&media, temp.path(), &expected)?);

        fs::rename(&sidecar, temp.path().join("movie.en.srt"))?;
        assert!(!revalidate_media_aggregate(&media, temp.path(), &expected)?);
        fs::write(&sidecar, b"replacement")?;
        assert!(!revalidate_media_aggregate(&media, temp.path(), &expected)?);
        fs::remove_file(&sidecar)?;
        fs::remove_file(temp.path().join("movie.en.srt"))?;
        assert!(!revalidate_media_aggregate(&media, temp.path(), &expected)?);

        let outside = tempfile::NamedTempFile::new()?;
        symlink(outside.path(), &sidecar)?;
        assert!(!revalidate_media_aggregate(&media, temp.path(), &expected)?);
        Ok(())
    }
}
