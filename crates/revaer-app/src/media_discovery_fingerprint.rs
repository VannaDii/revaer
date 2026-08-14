//! Stable aggregate identity and workspace capture for media plus owned subtitle sidecars.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Component, Path, PathBuf};

use rustix::fs::{Dir, Mode, OFlags, open, openat};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::media_discovery_runtime::is_media_file;
use crate::media_source_fingerprint::{file_identity, open_regular_at};

const HASH_BUFFER_BYTES: usize = 64 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 4_096;
const SIDECAR_EXTENSIONS: &[&str] = &["ass", "idx", "srt", "ssa", "sub", "sup", "vtt"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaAggregateFingerprint {
    pub(crate) identity: String,
    pub(crate) size_bytes: i64,
    pub(crate) modified_ns: i64,
    pub(crate) changed_ns: i64,
    pub(crate) sha256: String,
}

pub(crate) struct CapturedMediaAggregate {
    pub(crate) fingerprint: MediaAggregateFingerprint,
    pub(crate) source_path: PathBuf,
}

#[derive(Debug, Error)]
pub(crate) enum FingerprintError {
    #[error("media discovery fingerprint io error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("media discovery fingerprint path is not a normal descendant of its root: {0}")]
    InvalidPath(PathBuf),
    #[error("media discovery fingerprint source set exceeds {0} directory entries")]
    DirectoryEntryLimitExceeded(usize),
    #[error("media discovery fingerprint value is too large: {0}")]
    ValueTooLarge(&'static str),
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
    let sidecar_stem = path.file_stem()?.to_str()?;
    let mut matches = fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| is_media_file(candidate))
        .filter_map(|candidate| {
            let stem = candidate.file_stem()?.to_str()?;
            sidecar_matches_media_stem(sidecar_stem, stem).then_some((stem.len(), candidate))
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
    aggregate_media(media_path, root, None).map(|value| value.map(|item| item.fingerprint))
}

pub(crate) fn capture_media_aggregate(
    media_path: &Path,
    root: &Path,
    destination: &Path,
) -> Result<Option<CapturedMediaAggregate>, FingerprintError> {
    aggregate_media(media_path, root, Some(destination))
}

pub(crate) fn revalidate_media_aggregate(
    media_path: &Path,
    root: &Path,
    expected: &MediaAggregateFingerprint,
) -> Result<bool, FingerprintError> {
    Ok(fingerprint_media_aggregate(media_path, root)?.as_ref() == Some(expected))
}

fn aggregate_media(
    media_path: &Path,
    root: &Path,
    destination: Option<&Path>,
) -> Result<Option<CapturedMediaAggregate>, FingerprintError> {
    aggregate_media_with_observer(media_path, root, destination, || {})
}

fn aggregate_media_with_observer(
    media_path: &Path,
    root: &Path,
    destination: Option<&Path>,
    mut after_member_set: impl FnMut(),
) -> Result<Option<CapturedMediaAggregate>, FingerprintError> {
    let opened = open_media_parent(media_path, root)?;
    let Some(member_names) = owned_member_names(&opened.directory, &opened.source_name)? else {
        return Ok(None);
    };
    after_member_set();

    let mut aggregate = Sha256::new();
    let mut total_size = 0_u64;
    let mut latest_modified_ns = 0_i64;
    let mut source_identity = None;
    for name in &member_names {
        let display_path = media_path.parent().unwrap_or(root).join(name);
        let Some(mut file) = open_regular_at(&opened.directory, name, &display_path)? else {
            return Ok(None);
        };
        let before = file_identity(&file, &display_path)?;
        let relative = opened.relative_parent.join(name);
        aggregate.update(relative.as_os_str().as_bytes());
        let mut output = destination
            .map(|directory| create_snapshot_file(directory, name))
            .transpose()?;
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
            if let Some(writer) = output.as_mut() {
                writer
                    .write_all(&buffer[..read])
                    .map_err(|source| FingerprintError::Io {
                        path: destination.unwrap_or(root).join(name),
                        source,
                    })?;
            }
        }
        if let Some(writer) = output.as_mut() {
            writer.sync_all().map_err(|source| FingerprintError::Io {
                path: destination.unwrap_or(root).join(name),
                source,
            })?;
        }
        let after = file_identity(&file, &display_path)?;
        if before != after {
            return Ok(None);
        }
        let Some(current) = open_regular_at(&opened.directory, name, &display_path)? else {
            return Ok(None);
        };
        if file_identity(&current, &display_path)? != after {
            return Ok(None);
        }
        total_size = total_size
            .checked_add(after.size_bytes)
            .ok_or(FingerprintError::ValueTooLarge("size_bytes"))?;
        latest_modified_ns = latest_modified_ns.max(after.modified_ns);
        if name == &opened.source_name {
            source_identity = Some(after);
        }
    }
    if owned_member_names(&opened.directory, &opened.source_name)?.as_ref() != Some(&member_names) {
        return Ok(None);
    }
    let source_identity =
        source_identity.ok_or_else(|| FingerprintError::InvalidPath(media_path.to_path_buf()))?;
    let source_path = destination.map_or_else(
        || media_path.to_path_buf(),
        |directory| directory.join(&opened.source_name),
    );
    Ok(Some(CapturedMediaAggregate {
        fingerprint: MediaAggregateFingerprint {
            identity: format!(
                "{:016x}:{:016x}",
                source_identity.device, source_identity.inode
            ),
            size_bytes: i64::try_from(total_size)
                .map_err(|_| FingerprintError::ValueTooLarge("size_bytes"))?,
            modified_ns: latest_modified_ns,
            changed_ns: source_identity.changed_ns,
            sha256: format!("{:x}", aggregate.finalize()),
        },
        source_path,
    }))
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
    let mut directory = open(
        root,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|source| FingerprintError::Io {
        path: root.to_path_buf(),
        source: io::Error::from(source),
    })?;
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

fn owned_member_names(
    directory: &OwnedFd,
    source_name: &OsStr,
) -> Result<Option<Vec<OsString>>, FingerprintError> {
    let names = directory_names(directory)?;
    if !names.iter().any(|name| name == source_name) {
        return Ok(None);
    }
    let mut media_names = Vec::new();
    for name in &names {
        if is_media_file(Path::new(name)) {
            let Some(file) = open_regular_at(directory, name, Path::new(name))? else {
                return Ok(None);
            };
            drop(file);
            media_names.push(name.clone());
        }
    }
    if !media_names.iter().any(|name| name == source_name) {
        return Ok(None);
    }
    let mut members = vec![source_name.to_os_string()];
    for name in names {
        if is_sidecar(Path::new(&name))
            && sidecar_owner(&name, &media_names).as_deref() == Some(source_name)
        {
            let Some(file) = open_regular_at(directory, &name, Path::new(&name))? else {
                return Ok(None);
            };
            drop(file);
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
        let name = entry.file_name().to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        if names.len() == MAX_DIRECTORY_ENTRIES {
            return Err(FingerprintError::DirectoryEntryLimitExceeded(
                MAX_DIRECTORY_ENTRIES,
            ));
        }
        names.push(OsString::from_vec(name.to_vec()));
    }
    names.sort();
    Ok(names)
}

fn create_snapshot_file(directory: &Path, name: &OsStr) -> Result<File, FingerprintError> {
    let path = directory.join(name);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| FingerprintError::Io { path, source })
}

fn sidecar_owner(sidecar_name: &OsStr, media_names: &[OsString]) -> Option<OsString> {
    let sidecar_stem = Path::new(sidecar_name).file_stem()?.to_str()?;
    let mut matches = media_names
        .iter()
        .filter_map(|name| {
            let stem = Path::new(name).file_stem()?.to_str()?;
            sidecar_matches_media_stem(sidecar_stem, stem).then_some((stem.len(), name))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(right.1)));
    let longest = matches.first()?.0;
    let mut owners = matches.into_iter().filter(|item| item.0 == longest);
    let owner = owners.next()?.1;
    if owners.next().is_some() {
        return None;
    }
    Some(owner.clone())
}

fn sidecar_matches_media_stem(sidecar_stem: &str, media_stem: &str) -> bool {
    sidecar_stem
        .strip_prefix(media_stem)
        .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with('.'))
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
    use std::fs;

    use super::{
        aggregate_media_with_observer, capture_media_aggregate, fingerprint_media_aggregate,
        owner_for_changed_path, revalidate_media_aggregate,
    };

    #[test]
    fn captures_regular_source_and_sorted_sidecars_into_workspace() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let source_root = temp.path().join("source");
        let workspace = temp.path().join("workspace");
        fs::create_dir(&source_root)?;
        fs::create_dir(&workspace)?;
        let media = source_root.join("movie.mkv");
        fs::write(&media, b"media")?;
        fs::write(source_root.join("movie.fr.srt"), b"fr")?;
        fs::write(source_root.join("movie.en.srt"), b"en")?;

        let captured = capture_media_aggregate(&media, &source_root, &workspace)?
            .ok_or_else(|| anyhow::anyhow!("missing capture"))?;

        assert_eq!(captured.source_path, workspace.join("movie.mkv"));
        assert_eq!(fs::read(captured.source_path)?, b"media");
        assert_eq!(fs::read(workspace.join("movie.en.srt"))?, b"en");
        assert_eq!(fs::read(workspace.join("movie.fr.srt"))?, b"fr");
        assert!(revalidate_media_aggregate(
            &media,
            &source_root,
            &captured.fingerprint
        )?);
        Ok(())
    }

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

    #[test]
    fn capture_rejects_create_and_delete_after_member_enumeration() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let media = temp.path().join("movie.mkv");
        let sidecar = temp.path().join("movie.en.srt");
        fs::write(&media, b"media")?;
        let created = aggregate_media_with_observer(&media, temp.path(), None, || {
            fs::write(&sidecar, b"created").expect("test sidecar create should succeed");
        })?;
        assert!(created.is_none());

        let deleted = aggregate_media_with_observer(&media, temp.path(), None, || {
            fs::remove_file(&sidecar).expect("test sidecar delete should succeed");
        })?;
        assert!(deleted.is_none());
        Ok(())
    }

    #[test]
    fn aggregate_revalidation_rejects_symlink_and_rename_replacement() -> anyhow::Result<()> {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir()?;
        let source_root = temp.path().join("source");
        fs::create_dir(&source_root)?;
        let media = source_root.join("movie.mkv");
        let moved = source_root.join("movie.original.mkv");
        fs::write(&media, b"media")?;
        let expected = fingerprint_media_aggregate(&media, &source_root)?
            .ok_or_else(|| anyhow::anyhow!("missing aggregate"))?;
        fs::rename(&media, &moved)?;
        fs::write(&media, b"replacement")?;
        assert!(!revalidate_media_aggregate(
            &media,
            &source_root,
            &expected
        )?);

        fs::remove_file(&media)?;
        symlink(&moved, &media)?;
        assert!(fingerprint_media_aggregate(&media, &source_root)?.is_none());
        Ok(())
    }

    #[test]
    fn aggregate_rejects_symlinked_shared_root_and_ancestor() -> anyhow::Result<()> {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir()?;
        let real_root = temp.path().join("real");
        let linked_root = temp.path().join("linked");
        fs::create_dir(&real_root)?;
        fs::write(real_root.join("movie.mkv"), b"media")?;
        symlink(&real_root, &linked_root)?;
        assert!(fingerprint_media_aggregate(&linked_root.join("movie.mkv"), &linked_root).is_err());

        let nested = real_root.join("nested");
        let outside = temp.path().join("outside");
        fs::create_dir(&outside)?;
        fs::write(outside.join("movie.mkv"), b"outside")?;
        symlink(&outside, &nested)?;
        assert!(fingerprint_media_aggregate(&nested.join("movie.mkv"), &real_root).is_err());
        Ok(())
    }
}
