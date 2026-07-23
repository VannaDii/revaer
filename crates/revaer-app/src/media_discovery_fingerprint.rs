//! Aggregate media identity over descriptor-bound media and subtitle sidecars.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use rustix::fs::{Dir, Mode, OFlags, open, openat};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::media_discovery_runtime::is_media_file;

const HASH_BUFFER_BYTES: usize = 64 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 4_096;
const MAX_LOGICAL_SIDECARS: usize = 64;
const MAX_SIDECAR_BYTES: u64 = 256 * 1024 * 1024;
const SIDECAR_EXTENSIONS: &[&str] = &["ass", "idx", "srt", "ssa", "sub", "sup", "vtt"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaAggregateFingerprint {
    pub(crate) size_bytes: i64,
    pub(crate) modified_ns: i64,
    pub(crate) sha256: String,
}

#[derive(Debug, Error)]
pub(crate) enum FingerprintError {
    #[error("media discovery fingerprint io error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("media discovery fingerprint path is not a regular descendant of its root: {0}")]
    InvalidPath(PathBuf),
    #[error("media discovery fingerprint exceeded the {0} resource limit")]
    ResourceLimit(&'static str),
    #[error("media discovery fingerprint value is too large: {0}")]
    ValueTooLarge(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    size: u64,
    modified_ns: i64,
    changed_ns: i64,
}

struct OpenedMediaParent {
    directory: OwnedFd,
    source_name: OsString,
    relative_parent: PathBuf,
}

pub(crate) fn owner_for_changed_path(path: &Path, root: &Path) -> Option<PathBuf> {
    if is_media_file(path) {
        return Some(path.to_path_buf());
    }
    if !is_sidecar(path) {
        return None;
    }
    let parent = path.parent()?;
    let directory = open_directory(parent).ok()?;
    let names = directory_names(&directory).ok()?;
    let media_names = names
        .into_iter()
        .filter(|name| is_media_file(Path::new(name)))
        .collect::<Vec<_>>();
    let owner = sidecar_owner(path.file_name()?, &media_names)?;
    parent
        .join(owner)
        .strip_prefix(root)
        .ok()
        .map(|relative| root.join(relative))
}

pub(crate) fn fingerprint_media_aggregate(
    media_path: &Path,
    root: &Path,
) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
    let opened = open_media_parent(media_path, root)?;
    let Some(members) = owned_member_names(&opened.directory, &opened.source_name)? else {
        return Ok(None);
    };
    let mut aggregate = Sha256::new();
    let mut total_size = 0_u64;
    let mut latest_modified_ns = 0_i64;
    let mut observed_identities = Vec::with_capacity(members.len());
    for member in &members {
        let display_path = media_path.parent().unwrap_or(root).join(member);
        let Some(mut file) = open_regular_at(&opened.directory, member, &display_path)? else {
            return Ok(None);
        };
        let before = file_identity(&file, &display_path)?;
        let relative = opened.relative_parent.join(member);
        let relative = relative.as_os_str().as_bytes();
        aggregate.update(
            u64::try_from(relative.len())
                .map_err(|_| FingerprintError::ValueTooLarge("path"))?
                .to_le_bytes(),
        );
        aggregate.update(relative);
        aggregate.update(before.size.to_le_bytes());
        let mut buffer = vec![0_u8; HASH_BUFFER_BYTES].into_boxed_slice();
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|source| FingerprintError::Io {
                    path: display_path.clone(),
                    source,
                })?;
            if read == 0 {
                break;
            }
            aggregate.update(&buffer[..read]);
        }
        let after = file_identity(&file, &display_path)?;
        let current = open_regular_at(&opened.directory, member, &display_path)?;
        if before != after
            || current
                .as_ref()
                .map(|item| file_identity(item, &display_path))
                .transpose()?
                .as_ref()
                != Some(&after)
        {
            return Ok(None);
        }
        total_size = total_size
            .checked_add(after.size)
            .ok_or(FingerprintError::ValueTooLarge("size"))?;
        latest_modified_ns = latest_modified_ns.max(after.modified_ns);
        observed_identities.push((member, after));
    }
    if owned_member_names(&opened.directory, &opened.source_name)?.as_ref() != Some(&members) {
        return Ok(None);
    }
    for (member, expected) in observed_identities {
        let display_path = media_path.parent().unwrap_or(root).join(member);
        let Some(current) = open_regular_at(&opened.directory, member, &display_path)? else {
            return Ok(None);
        };
        if file_identity(&current, &display_path)? != expected {
            return Ok(None);
        }
    }
    Ok(Some(MediaAggregateFingerprint {
        size_bytes: i64::try_from(total_size)
            .map_err(|_| FingerprintError::ValueTooLarge("size"))?,
        modified_ns: latest_modified_ns,
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

fn open_media_parent(
    media_path: &Path,
    root: &Path,
) -> Result<OpenedMediaParent, FingerprintError> {
    if !root.is_absolute() || !media_path.is_absolute() {
        return Err(FingerprintError::InvalidPath(media_path.to_path_buf()));
    }
    let relative = media_path
        .strip_prefix(root)
        .map_err(|_| FingerprintError::InvalidPath(media_path.to_path_buf()))?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_os_string()),
            _ => Err(FingerprintError::InvalidPath(media_path.to_path_buf())),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let Some((source_name, directories)) = components.split_last() else {
        return Err(FingerprintError::InvalidPath(media_path.to_path_buf()));
    };
    let mut directory = open_directory(root)?;
    let mut relative_parent = PathBuf::new();
    for component in directories {
        directory = openat(
            &directory,
            component,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|source| FingerprintError::Io {
            path: root.join(&relative_parent).join(component),
            source: io::Error::from(source),
        })?;
        relative_parent.push(component);
    }
    Ok(OpenedMediaParent {
        directory,
        source_name: source_name.clone(),
        relative_parent,
    })
}

fn open_directory(path: &Path) -> Result<OwnedFd, FingerprintError> {
    open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|source| FingerprintError::Io {
        path: path.to_path_buf(),
        source: io::Error::from(source),
    })
}

fn owned_member_names(
    directory: &OwnedFd,
    source_name: &OsStr,
) -> Result<Option<Vec<OsString>>, FingerprintError> {
    let names = directory_names(directory)?;
    let media_names = names
        .iter()
        .filter(|name| is_media_file(Path::new(name)))
        .cloned()
        .collect::<Vec<_>>();
    if !media_names.iter().any(|name| name == source_name)
        || open_regular_at(directory, source_name, Path::new(source_name))?.is_none()
    {
        return Ok(None);
    }
    let mut members = vec![source_name.to_os_string()];
    let mut logical_sidecars = BTreeSet::new();
    let mut sidecar_bytes = 0_u64;
    for name in names {
        if is_sidecar(Path::new(&name))
            && sidecar_owner(&name, &media_names).as_deref() == Some(source_name)
        {
            let Some(file) = open_regular_at(directory, &name, Path::new(&name))? else {
                return Ok(None);
            };
            sidecar_bytes = sidecar_bytes
                .checked_add(file_identity(&file, Path::new(&name))?.size)
                .ok_or(FingerprintError::ResourceLimit("sidecar bytes"))?;
            if sidecar_bytes > MAX_SIDECAR_BYTES {
                return Err(FingerprintError::ResourceLimit("sidecar bytes"));
            }
            logical_sidecars.insert(logical_sidecar_key(&name)?);
            if logical_sidecars.len() > MAX_LOGICAL_SIDECARS {
                return Err(FingerprintError::ResourceLimit("logical sidecars"));
            }
            members.push(name);
        }
    }
    members.sort();
    Ok(Some(members))
}

fn directory_names(directory: &OwnedFd) -> Result<Vec<OsString>, FingerprintError> {
    let mut entries = Dir::read_from(directory).map_err(|source| FingerprintError::Io {
        path: PathBuf::from("<opened-media-directory>"),
        source: io::Error::from(source),
    })?;
    let mut names = Vec::new();
    while let Some(entry) = entries.read() {
        let entry = entry.map_err(|source| FingerprintError::Io {
            path: PathBuf::from("<opened-media-directory>"),
            source: io::Error::from(source),
        })?;
        let bytes = entry.file_name().to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        if names.len() == MAX_DIRECTORY_ENTRIES {
            return Err(FingerprintError::ResourceLimit("directory entries"));
        }
        if std::str::from_utf8(bytes).is_err() {
            return Err(FingerprintError::InvalidPath(
                OsString::from_vec(bytes.to_vec()).into(),
            ));
        }
        names.push(OsString::from_vec(bytes.to_vec()));
    }
    names.sort();
    Ok(names)
}

fn open_regular_at(
    directory: impl AsFd,
    name: &OsStr,
    display_path: &Path,
) -> Result<Option<File>, FingerprintError> {
    match openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    ) {
        Ok(file) => {
            let file = File::from(file);
            let metadata = file.metadata().map_err(|source| FingerprintError::Io {
                path: display_path.to_path_buf(),
                source,
            })?;
            Ok(metadata.is_file().then_some(file))
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

fn file_identity(file: &File, path: &Path) -> Result<FileIdentity, FingerprintError> {
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
        size: metadata.len(),
        modified_ns: timestamp_ns(metadata.mtime(), metadata.mtime_nsec())?,
        changed_ns: timestamp_ns(metadata.ctime(), metadata.ctime_nsec())?,
    })
}

fn timestamp_ns(seconds: i64, nanoseconds: i64) -> Result<i64, FingerprintError> {
    if seconds < 0 {
        return Err(FingerprintError::ValueTooLarge("modified_ns"));
    }
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(nanoseconds))
        .ok_or(FingerprintError::ValueTooLarge("modified_ns"))
}

fn sidecar_owner(sidecar_name: &OsStr, media_names: &[OsString]) -> Option<OsString> {
    let sidecar_stem = Path::new(sidecar_name).file_stem()?.to_str()?;
    let mut matches = media_names
        .iter()
        .filter_map(|name| {
            let stem = Path::new(name).file_stem()?.to_str()?;
            sidecar_stem
                .strip_prefix(stem)
                .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with('.'))
                .then_some((stem.len(), name))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(right.1)));
    let longest = matches.first()?.0;
    let mut owners = matches.into_iter().filter(|item| item.0 == longest);
    let owner = owners.next()?.1;
    owners.next().is_none().then(|| owner.clone())
}

fn logical_sidecar_key(name: &OsStr) -> Result<String, FingerprintError> {
    let path = Path::new(name);
    let text = name
        .to_str()
        .ok_or_else(|| FingerprintError::InvalidPath(path.to_path_buf()))?;
    let paired = path
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("idx") || extension.eq_ignore_ascii_case("sub")
        });
    Ok(if paired {
        path.file_stem()
            .and_then(OsStr::to_str)
            .ok_or_else(|| FingerprintError::InvalidPath(path.to_path_buf()))?
            .to_string()
    } else {
        text.to_string()
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
    use super::{
        FingerprintError, MAX_LOGICAL_SIDECARS, MAX_SIDECAR_BYTES, fingerprint_media_aggregate,
        owner_for_changed_path, revalidate_media_aggregate,
    };
    use std::fs::{self, File};

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

    #[test]
    fn aggregate_rejects_excess_sidecars_and_sidecar_bytes() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let media = temp.path().join("movie.mkv");
        fs::write(&media, b"media")?;
        for index in 0..=MAX_LOGICAL_SIDECARS {
            fs::write(temp.path().join(format!("movie.{index}.srt")), b"x")?;
        }
        assert!(matches!(
            fingerprint_media_aggregate(&media, temp.path()),
            Err(FingerprintError::ResourceLimit("logical sidecars"))
        ));
        for index in 0..=MAX_LOGICAL_SIDECARS {
            fs::remove_file(temp.path().join(format!("movie.{index}.srt")))?;
        }
        File::create(temp.path().join("movie.srt"))?.set_len(MAX_SIDECAR_BYTES + 1)?;
        assert!(matches!(
            fingerprint_media_aggregate(&media, temp.path()),
            Err(FingerprintError::ResourceLimit("sidecar bytes"))
        ));
        Ok(())
    }
}
