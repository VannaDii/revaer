//! Aggregate media identity over descriptor-bound media and subtitle sidecars.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

#[cfg(any(target_os = "linux", test))]
use rustix::fs::{AtFlags, FileType, statat};
use rustix::fs::{Dir, Mode, OFlags, Stat, fstat, open, openat};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::media_discovery_runtime::is_media_file;

const HASH_BUFFER_BYTES: usize = 64 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 4_096;
const MAX_DIRECTORY_NAME_BYTES: usize = 1024 * 1024;
// Approved initial DISC limits; higher envelopes need an explicit policy version.
const MAX_LOGICAL_SIDECARS: usize = 32;
const MAX_PHYSICAL_MEMBERS: usize = 65;
const MAX_PRIMARY_BYTES: u64 = 256 * 1024 * 1024 * 1024;
const MAX_SIDECAR_FILE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_SIDECAR_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const SIDECAR_EXTENSIONS: &[&str] = &["ass", "idx", "srt", "ssa", "sub", "sup", "vtt"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaAggregateFingerprint {
    pub(crate) identity: String,
    pub(crate) size_bytes: i64,
    pub(crate) modified_ns: i64,
    pub(crate) changed_ns: i64,
    pub(crate) sha256: String,
}

#[cfg(any(target_os = "linux", test))]
pub(crate) struct MediaDirectoryEntry {
    pub(crate) name: OsString,
    pub(crate) kind: MediaDirectoryEntryKind,
}

#[cfg(any(target_os = "linux", test))]
pub(crate) enum MediaDirectoryEntryKind {
    Directory,
    File(u64),
    Other,
}

#[cfg(any(target_os = "linux", test))]
pub(crate) struct MediaDirectoryInventory {
    pub(crate) entries: Vec<MediaDirectoryEntry>,
    pub(crate) observation: Stat,
}

/// Inventory every raw name before selecting media; never follow child links.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn read_media_directory_at(
    directory: &OwnedFd,
    entry_limit: usize,
) -> Result<MediaDirectoryInventory, FingerprintError> {
    let before = directory_observation(directory)?;
    let names = enumerate_directory_names(
        directory,
        entry_limit.min(MAX_DIRECTORY_ENTRIES),
        MAX_DIRECTORY_NAME_BYTES,
    )?;
    let mut entries = Vec::with_capacity(names.len());
    for name in names {
        let stat = statat(directory, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(|source| {
            FingerprintError::Io {
                path: PathBuf::from(&name),
                source: io::Error::from(source),
            }
        })?;
        let kind = match FileType::from_raw_mode(stat.st_mode) {
            FileType::Directory => MediaDirectoryEntryKind::Directory,
            FileType::RegularFile => MediaDirectoryEntryKind::File(
                u64::try_from(stat.st_size)
                    .map_err(|_| FingerprintError::ValueTooLarge("directory member size"))?,
            ),
            _ => MediaDirectoryEntryKind::Other,
        };
        entries.push(MediaDirectoryEntry { name, kind });
    }
    validate_directory_observation(directory, &before)?;
    Ok(MediaDirectoryInventory {
        entries,
        observation: before,
    })
}

#[derive(Debug, Error)]
pub(crate) enum FingerprintError {
    #[error("media discovery fingerprint cancelled")]
    Cancelled,
    #[error(transparent)]
    Root(#[from] revaer_media_runtime::root_catalog::RootDirectoryError),
    #[error("media discovery fingerprint io error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("media discovery fingerprint path is not a regular descendant of its root: {0}")]
    InvalidPath(PathBuf),
    #[error("media discovery fingerprint exceeded the {0} resource limit")]
    ResourceLimit(&'static str),
    #[error("media discovery directory changed during enumeration")]
    DirectoryChanged,
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
    fingerprint_media_aggregate_cancellable(media_path, root, &|| false)
}

pub(crate) fn fingerprint_media_aggregate_cancellable(
    media_path: &Path,
    root: &Path,
    cancelled: &dyn Fn() -> bool,
) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
    let opened = open_media_parent(media_path, root)?;
    fingerprint_media_aggregate_at_cancellable(media_path, root, opened.directory, cancelled)
}

#[cfg(test)]
pub(crate) fn fingerprint_media_aggregate_measured(
    media_path: &Path,
    root: &Path,
    cancelled: &dyn Fn() -> bool,
    hash_elapsed: &mut Duration,
) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
    let opened = open_media_parent(media_path, root)?;
    fingerprint_media_aggregate_at_measured(
        media_path,
        root,
        opened.directory,
        cancelled,
        hash_elapsed,
    )
}

#[cfg(test)]
fn fingerprint_media_aggregate_at(
    media_path: &Path,
    root: &Path,
    parent: OwnedFd,
) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
    fingerprint_media_aggregate_at_cancellable(media_path, root, parent, &|| false)
}

pub(crate) fn fingerprint_media_aggregate_at_cancellable(
    media_path: &Path,
    root: &Path,
    parent: OwnedFd,
    cancelled: &dyn Fn() -> bool,
) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
    fingerprint_media_aggregate_at_measured(
        media_path,
        root,
        parent,
        cancelled,
        &mut Duration::default(),
    )
}

pub(crate) fn fingerprint_media_aggregate_at_measured(
    media_path: &Path,
    root: &Path,
    parent: OwnedFd,
    cancelled: &dyn Fn() -> bool,
    hash_elapsed: &mut Duration,
) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
    let components = media_components(media_path, root)?;
    let Some((source_name, directories)) = components.split_last() else {
        return Err(FingerprintError::InvalidPath(media_path.to_path_buf()));
    };
    let opened = OpenedMediaParent {
        directory: parent,
        source_name: source_name.clone(),
        relative_parent: directories.iter().collect(),
    };
    fingerprint_opened_aggregate(media_path, root, &opened, cancelled, hash_elapsed)
}

fn fingerprint_opened_aggregate(
    media_path: &Path,
    root: &Path,
    opened: &OpenedMediaParent,
    cancelled: &dyn Fn() -> bool,
    hash_elapsed: &mut Duration,
) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
    match fingerprint_stable_aggregate(media_path, root, opened, cancelled, hash_elapsed) {
        Err(FingerprintError::DirectoryChanged) => Ok(None),
        result => result,
    }
}

fn fingerprint_stable_aggregate(
    media_path: &Path,
    root: &Path,
    opened: &OpenedMediaParent,
    cancelled: &dyn Fn() -> bool,
    hash_elapsed: &mut Duration,
) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
    if cancelled() {
        return Err(FingerprintError::Cancelled);
    }
    let Some(members) = owned_member_names(&opened.directory, &opened.source_name)? else {
        return Ok(None);
    };
    let mut aggregate = Sha256::new();
    let mut total_size = 0_u64;
    let mut sidecar_size = 0_u64;
    let mut latest_modified_ns = 0_i64;
    let mut source_identity = None;
    let mut observed_identities = Vec::with_capacity(members.len());
    for member in &members {
        if cancelled() {
            return Err(FingerprintError::Cancelled);
        }
        let display_path = media_path.parent().unwrap_or(root).join(member);
        let Some(mut file) = open_regular_at(&opened.directory, member, &display_path)? else {
            return Ok(None);
        };
        let before = file_identity(&file, &display_path)?;
        validate_member_size(
            before.size,
            member == &opened.source_name,
            &mut sidecar_size,
        )?;
        let relative = opened.relative_parent.join(member);
        let relative = relative.as_os_str().as_bytes();
        aggregate.update(
            u64::try_from(relative.len())
                .map_err(|_| FingerprintError::ValueTooLarge("path"))?
                .to_le_bytes(),
        );
        aggregate.update(relative);
        aggregate.update(before.size.to_le_bytes());
        let hash_started = Instant::now();
        let hashed = hash_member(
            &mut file,
            &display_path,
            &mut aggregate,
            cancelled,
            before.size,
        );
        *hash_elapsed = hash_elapsed.saturating_add(hash_started.elapsed());
        hashed?;
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
        if member == &opened.source_name {
            source_identity = Some(after.clone());
        }
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
    let source_identity =
        source_identity.ok_or_else(|| FingerprintError::InvalidPath(media_path.to_path_buf()))?;
    Ok(Some(MediaAggregateFingerprint {
        identity: format!(
            "{:016x}:{:016x}",
            source_identity.device, source_identity.inode
        ),
        size_bytes: i64::try_from(total_size)
            .map_err(|_| FingerprintError::ValueTooLarge("size"))?,
        modified_ns: latest_modified_ns,
        changed_ns: source_identity.changed_ns,
        sha256: format!("{:x}", aggregate.finalize()),
    }))
}

fn hash_member(
    file: &mut impl Read,
    path: &Path,
    aggregate: &mut Sha256,
    cancelled: &dyn Fn() -> bool,
    expected_bytes: u64,
) -> Result<(), FingerprintError> {
    let mut observed_bytes = 0_u64;
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES].into_boxed_slice();
    let max_read =
        u64::try_from(buffer.len()).map_err(|_| FingerprintError::ValueTooLarge("read size"))?;
    loop {
        if cancelled() {
            return Err(FingerprintError::Cancelled);
        }
        if observed_bytes == expected_bytes {
            return Ok(());
        }
        let remaining = usize::try_from((expected_bytes - observed_bytes).min(max_read))
            .map_err(|_| FingerprintError::ValueTooLarge("read size"))?;
        let read = file
            .read(&mut buffer[..remaining])
            .map_err(|source| FingerprintError::Io {
                path: path.into(),
                source,
            })?;
        if read == 0 {
            return Err(FingerprintError::DirectoryChanged);
        }
        observed_bytes +=
            u64::try_from(read).map_err(|_| FingerprintError::ValueTooLarge("read size"))?;
        aggregate.update(&buffer[..read]);
    }
}

pub(crate) fn revalidate_media_aggregate(
    media_path: &Path,
    root: &Path,
    expected: &MediaAggregateFingerprint,
) -> Result<bool, FingerprintError> {
    Ok(fingerprint_media_aggregate(media_path, root)?.as_ref() == Some(expected))
}

fn open_media_parent(
    media_path: &Path,
    root: &Path,
) -> Result<OpenedMediaParent, FingerprintError> {
    let components = media_components(media_path, root)?;
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

fn media_components(media_path: &Path, root: &Path) -> Result<Vec<OsString>, FingerprintError> {
    if !root.is_absolute() || !media_path.is_absolute() {
        return Err(FingerprintError::InvalidPath(media_path.to_path_buf()));
    }
    let relative = media_path
        .strip_prefix(root)
        .map_err(|_| FingerprintError::InvalidPath(media_path.to_path_buf()))?;
    relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_os_string()),
            _ => Err(FingerprintError::InvalidPath(media_path.to_path_buf())),
        })
        .collect()
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
    if !media_names.iter().any(|name| name == source_name) {
        return Ok(None);
    }
    let Some(source) = open_regular_at(directory, source_name, Path::new(source_name))? else {
        return Ok(None);
    };
    validate_member_size(
        file_identity(&source, Path::new(source_name))?.size,
        true,
        &mut 0,
    )?;
    drop(source);
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
            validate_member_size(
                file_identity(&file, Path::new(&name))?.size,
                false,
                &mut sidecar_bytes,
            )?;
            logical_sidecars.insert(logical_sidecar_key(&name)?);
            if logical_sidecars.len() > MAX_LOGICAL_SIDECARS {
                return Err(FingerprintError::ResourceLimit("logical sidecars"));
            }
            if members.len() == MAX_PHYSICAL_MEMBERS {
                return Err(FingerprintError::ResourceLimit("physical members"));
            }
            members.push(name);
        }
    }
    members.sort();
    Ok(Some(members))
}

fn validate_member_size(
    size: u64,
    primary: bool,
    sidecar_bytes: &mut u64,
) -> Result<(), FingerprintError> {
    if primary {
        if size > MAX_PRIMARY_BYTES {
            return Err(FingerprintError::ResourceLimit("primary bytes"));
        }
        return Ok(());
    }
    if size > MAX_SIDECAR_FILE_BYTES {
        return Err(FingerprintError::ResourceLimit("sidecar file bytes"));
    }
    *sidecar_bytes = sidecar_bytes
        .checked_add(size)
        .ok_or(FingerprintError::ResourceLimit("sidecar bytes"))?;
    if *sidecar_bytes > MAX_SIDECAR_BYTES {
        return Err(FingerprintError::ResourceLimit("sidecar bytes"));
    }
    Ok(())
}

fn directory_names(directory: &OwnedFd) -> Result<Vec<OsString>, FingerprintError> {
    let before = directory_observation(directory)?;
    let names =
        enumerate_directory_names(directory, MAX_DIRECTORY_ENTRIES, MAX_DIRECTORY_NAME_BYTES)?;
    validate_directory_observation(directory, &before)?;
    Ok(names)
}

fn directory_observation(directory: &OwnedFd) -> Result<Stat, FingerprintError> {
    fstat(directory).map_err(|source| FingerprintError::Io {
        path: PathBuf::from("<opened-media-directory>"),
        source: io::Error::from(source),
    })
}

fn validate_directory_observation(
    directory: &OwnedFd,
    before: &Stat,
) -> Result<(), FingerprintError> {
    let after = directory_observation(directory)?;
    if !directory_observations_match(before, &after) {
        return Err(FingerprintError::DirectoryChanged);
    }
    Ok(())
}

pub(crate) const fn directory_observations_match(before: &Stat, after: &Stat) -> bool {
    before.st_dev == after.st_dev
        && before.st_ino == after.st_ino
        && before.st_mode == after.st_mode
        && before.st_size == after.st_size
        && before.st_mtime == after.st_mtime
        && before.st_mtime_nsec == after.st_mtime_nsec
        && before.st_ctime == after.st_ctime
        && before.st_ctime_nsec == after.st_ctime_nsec
}

fn enumerate_directory_names(
    directory: &OwnedFd,
    entry_limit: usize,
    name_byte_limit: usize,
) -> Result<Vec<OsString>, FingerprintError> {
    let mut entries = Dir::read_from(directory).map_err(|source| FingerprintError::Io {
        path: PathBuf::from("<opened-media-directory>"),
        source: io::Error::from(source),
    })?;
    let mut names = Vec::new();
    let mut name_bytes = 0_usize;
    while let Some(entry) = entries.read() {
        let entry = entry.map_err(|source| FingerprintError::Io {
            path: PathBuf::from("<opened-media-directory>"),
            source: io::Error::from(source),
        })?;
        let bytes = entry.file_name().to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        if names.len() == entry_limit {
            return Err(FingerprintError::ResourceLimit("directory entries"));
        }
        name_bytes = name_bytes
            .checked_add(bytes.len())
            .ok_or(FingerprintError::ResourceLimit("directory name bytes"))?;
        if name_bytes > name_byte_limit {
            return Err(FingerprintError::ResourceLimit("directory name bytes"));
        }
        if std::str::from_utf8(bytes).is_err() || bytes.contains(&b'\\') {
            return Err(FingerprintError::InvalidPath(
                OsString::from_vec(bytes.to_vec()).into(),
            ));
        }
        names.push(OsString::from_vec(bytes.to_vec()));
    }
    names.sort();
    if names.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(FingerprintError::DirectoryChanged);
    }
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
        FingerprintError, MAX_LOGICAL_SIDECARS, fingerprint_media_aggregate,
        owner_for_changed_path, revalidate_media_aggregate,
    };
    use std::fs::{self, File};

    #[test]
    fn directory_inventory_counts_all_raw_names_and_rejects_partial_results() -> anyhow::Result<()>
    {
        let temp = tempfile::tempdir()?;
        fs::write(temp.path().join("zz"), b"")?;
        fs::write(temp.path().join("aa"), b"")?;
        let directory = super::open_directory(temp.path())?;
        assert_eq!(
            super::enumerate_directory_names(&directory, 2, 4)?,
            [
                std::ffi::OsString::from("aa"),
                std::ffi::OsString::from("zz")
            ]
        );
        assert!(matches!(
            super::enumerate_directory_names(&directory, 1, 4),
            Err(FingerprintError::ResourceLimit("directory entries"))
        ));
        assert!(matches!(
            super::enumerate_directory_names(&directory, 2, 3),
            Err(FingerprintError::ResourceLimit("directory name bytes"))
        ));
        fs::write(temp.path().join("\u{e9}"), b"")?;
        assert_eq!(super::enumerate_directory_names(&directory, 3, 6)?.len(), 3);
        assert!(matches!(
            super::enumerate_directory_names(&directory, 3, 5),
            Err(FingerprintError::ResourceLimit("directory name bytes"))
        ));
        temp.close()?;
        Ok(())
    }

    #[test]
    fn directory_inventory_rejects_mutation_and_invalid_components() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let directory = super::open_directory(temp.path())?;
        let before = super::directory_observation(&directory)?;
        assert!(super::directory_names(&directory)?.is_empty());
        fs::write(temp.path().join("new-name"), b"")?;
        assert!(matches!(
            super::validate_directory_observation(&directory, &before),
            Err(FingerprintError::DirectoryChanged)
        ));
        fs::write(temp.path().join("invalid\\name"), b"")?;
        assert!(matches!(
            super::directory_names(&directory),
            Err(FingerprintError::InvalidPath(_))
        ));
        temp.close()?;
        Ok(())
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn directory_inventory_rejects_invalid_utf8() -> anyhow::Result<()> {
        use std::os::unix::ffi::OsStringExt;

        let temp = tempfile::tempdir()?;
        let directory = super::open_directory(temp.path())?;
        fs::write(
            temp.path().join(std::ffi::OsString::from_vec(vec![0xff])),
            b"",
        )?;
        assert!(matches!(
            super::directory_names(&directory),
            Err(FingerprintError::InvalidPath(_))
        ));
        temp.close()?;
        Ok(())
    }

    #[test]
    fn aggregate_at_retained_parent_does_not_reopen_a_replaced_path() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let directory = temp.path().join("Movies");
        fs::create_dir(&directory)?;
        let media = directory.join("movie.mkv");
        fs::write(&media, b"original media")?;
        fs::write(directory.join("movie.srt"), b"original subtitles")?;
        let expected = fingerprint_media_aggregate(&media, temp.path())?;
        let parent = super::open_directory(&directory)?;
        fs::rename(&directory, temp.path().join("moved"))?;
        fs::create_dir(&directory)?;
        fs::write(&media, b"replacement media")?;
        let retained = super::fingerprint_media_aggregate_at(&media, temp.path(), parent)?;
        assert_eq!(retained, expected);
        assert_ne!(fingerprint_media_aggregate(&media, temp.path())?, expected);
        fs::remove_file(temp.path().join("moved/movie.mkv"))?;
        let parent = super::open_directory(&temp.path().join("moved"))?;
        assert!(super::fingerprint_media_aggregate_at(&media, temp.path(), parent)?.is_none());
        temp.close()?;
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
            .ok_or_else(|| anyhow::anyhow!("missing aggregate"))?;
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
        for index in 0..4 {
            File::create(temp.path().join(format!("movie.{index}.srt")))?
                .set_len(super::MAX_SIDECAR_FILE_BYTES)?;
        }
        fs::write(temp.path().join("movie.4.srt"), b"x")?;
        assert!(matches!(
            fingerprint_media_aggregate(&media, temp.path()),
            Err(FingerprintError::ResourceLimit("sidecar bytes"))
        ));
        Ok(())
    }
    struct CountingFile<'a> {
        file: std::fs::File,
        read_bytes: &'a std::cell::Cell<usize>,
    }
    impl std::io::Read for CountingFile<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = self.file.read(buffer)?;
            self.read_bytes.set(self.read_bytes.get() + count);
            Ok(count)
        }
    }

    #[test]
    fn aggregate_read_cancellation_stops_physical_reads_and_preserves_source() -> anyhow::Result<()>
    {
        let root = tempfile::tempdir()?;
        let media = root.path().join("movie.mkv");
        let original = vec![0x35_u8; 1024 * 1024];
        fs::write(&media, &original)?;
        let read_bytes = std::cell::Cell::new(0_usize);
        let mut reader = CountingFile {
            file: std::fs::File::open(&media)?,
            read_bytes: &read_bytes,
        };
        let mut digest = sha2::Sha256::default();
        let result = super::hash_member(
            &mut reader,
            &media,
            &mut digest,
            &|| read_bytes.get() >= super::HASH_BUFFER_BYTES,
            u64::try_from(original.len())?,
        );
        assert!(matches!(result, Err(FingerprintError::Cancelled)));
        assert_eq!(read_bytes.get(), super::HASH_BUFFER_BYTES);
        assert_eq!(fs::read(&media)?, original);
        assert!(matches!(
            super::fingerprint_media_aggregate_cancellable(&media, root.path(), &|| true),
            Err(FingerprintError::Cancelled)
        ));
        assert_eq!(
            super::fingerprint_media_aggregate_cancellable(&media, root.path(), &|| false)?,
            fingerprint_media_aggregate(&media, root.path())?
        );
        Ok(())
    }
    #[test]
    fn aggregate_initial_size_limits_reject_sparse_oversize_before_reading() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let media = root.path().join("movie.mkv");
        File::create(&media)?.set_len(super::MAX_PRIMARY_BYTES + 1)?;
        assert!(matches!(
            fingerprint_media_aggregate(&media, root.path()),
            Err(FingerprintError::ResourceLimit("primary bytes"))
        ));
        fs::write(&media, b"source")?;
        let sidecar = root.path().join("movie.srt");
        File::create(&sidecar)?.set_len(super::MAX_SIDECAR_FILE_BYTES + 1)?;
        assert!(matches!(
            fingerprint_media_aggregate(&media, root.path()),
            Err(FingerprintError::ResourceLimit("sidecar file bytes"))
        ));
        let mut sidecars = 0;
        super::validate_member_size(super::MAX_PRIMARY_BYTES, true, &mut sidecars)?;
        for _ in 0..4 {
            super::validate_member_size(super::MAX_SIDECAR_FILE_BYTES, false, &mut sidecars)?;
        }
        assert_eq!(sidecars, super::MAX_SIDECAR_BYTES);
        assert!(matches!(
            super::validate_member_size(1, false, &mut sidecars),
            Err(FingerprintError::ResourceLimit("sidecar bytes"))
        ));
        Ok(())
    }

    #[test]
    fn aggregate_initial_member_limits_accept_pairs_and_reject_extra_case_variant()
    -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let media = root.path().join("movie.mkv");
        fs::write(&media, b"source")?;
        for index in 0..super::MAX_LOGICAL_SIDECARS {
            fs::write(root.path().join(format!("movie.{index}.idx")), b"index")?;
            fs::write(root.path().join(format!("movie.{index}.sub")), b"payload")?;
        }
        assert!(fingerprint_media_aggregate(&media, root.path())?.is_some());
        fs::write(root.path().join("movie.0.IDX"), b"extra")?;
        // This distinct physical case variant exists on Linux's case-sensitive filesystem.
        if fs::read_dir(root.path())?
            .collect::<Result<Vec<_>, _>>()?
            .len()
            > super::MAX_PHYSICAL_MEMBERS
        {
            assert!(matches!(
                fingerprint_media_aggregate(&media, root.path()),
                Err(FingerprintError::ResourceLimit("physical members"))
            ));
        }
        Ok(())
    }

    #[test]
    fn aggregate_hash_reads_only_observed_length_and_rejects_truncation() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let media = root.path().join("movie.mkv");
        fs::write(&media, b"source-grew")?;
        let read_bytes = std::cell::Cell::new(0);
        let mut reader = CountingFile {
            file: File::open(&media)?,
            read_bytes: &read_bytes,
        };
        super::hash_member(
            &mut reader,
            &media,
            &mut sha2::Sha256::default(),
            &|| false,
            6,
        )?;
        assert_eq!(read_bytes.get(), 6);
        let mut reader = File::open(&media)?;
        assert!(matches!(
            super::hash_member(
                &mut reader,
                &media,
                &mut sha2::Sha256::default(),
                &|| false,
                12
            ),
            Err(FingerprintError::DirectoryChanged)
        ));
        Ok(())
    }
}
