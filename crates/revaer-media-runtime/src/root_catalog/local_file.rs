use std::fs::File;
use std::io::Read;

use rustix::fs::{self, Access, AtFlags, FileType, Mode, OFlags, Stat};
use rustix::io::Errno;

use super::parse::{
    MAX_ROOT_CATALOG_DOCUMENT_BYTES, MAX_ROOT_CATALOG_PATH_BYTES, parse_root_catalog_v1,
};
use super::source::{
    RootCatalogFileEvidence, RootCatalogLoad, RootCatalogSource, RootCatalogSourceError,
    RootCatalogSourceTrust, RootCatalogTrustViolation,
};

/// Fixed release-package root-catalog location from ADR 550.
pub const PACKAGED_ROOT_CATALOG_PATH: &str = "/etc/revaer/media-root-catalog.json";

const UNTRUSTED_WRITE_BITS: u32 = 0o022;
const READ_CHUNK_BYTES: usize = 64 * 1024;

// Native mode fields have different integer widths across Unix ABIs.
fn mode_bits(raw_mode: impl Into<u32>) -> u32 {
    raw_mode.into()
}

#[derive(Debug, Clone, Copy)]
enum LocalTrustPolicy {
    Packaged {
        service_uid: u32,
    },
    NativeOverride {
        operator_uid: u32,
        file_owner_uid: u32,
    },
}

impl LocalTrustPolicy {
    const fn source_trust(self) -> RootCatalogSourceTrust {
        match self {
            Self::Packaged { .. } => RootCatalogSourceTrust::Packaged,
            Self::NativeOverride { .. } => RootCatalogSourceTrust::NativeOverride,
        }
    }

    const fn expected_file_owner(self) -> u32 {
        match self {
            Self::Packaged { .. } => 0,
            Self::NativeOverride { file_owner_uid, .. } => file_owner_uid,
        }
    }

    const fn directory_owner_is_trusted(self, owner_uid: u32) -> bool {
        match self {
            Self::Packaged { .. } => owner_uid == 0,
            Self::NativeOverride { operator_uid, .. } => {
                owner_uid == 0 || owner_uid == operator_uid
            }
        }
    }
}

/// Startup-only local file implementation of [`RootCatalogSource`].
#[derive(Debug, Clone)]
pub struct TrustedLocalRootCatalogSource {
    location: Box<str>,
    trust_policy: LocalTrustPolicy,
}

impl TrustedLocalRootCatalogSource {
    /// Construct the fixed packaged source for an injected non-root service UID.
    #[must_use]
    pub fn packaged(service_uid: u32) -> Self {
        Self {
            location: PACKAGED_ROOT_CATALOG_PATH.into(),
            trust_policy: LocalTrustPolicy::Packaged { service_uid },
        }
    }

    /// Construct a native operator-owned location override.
    ///
    /// The location is retained byte-for-byte and is never canonicalized.
    ///
    /// # Errors
    ///
    /// Returns [`RootCatalogSourceError::SourceUntrusted`] when the location is
    /// not absolute UTF-8 within 4,096 bytes or contains NUL.
    pub fn native_override(
        location: &str,
        operator_uid: u32,
    ) -> Result<Self, RootCatalogSourceError> {
        validate_location_bound(location)?;
        Ok(Self {
            location: location.into(),
            trust_policy: LocalTrustPolicy::NativeOverride {
                operator_uid,
                file_owner_uid: operator_uid,
            },
        })
    }

    pub(super) fn load_with_observer<F>(
        &self,
        mut observer: F,
    ) -> Result<RootCatalogLoad, RootCatalogSourceError>
    where
        F: FnMut(LoadPhase),
    {
        validate_policy_identity(self.trust_policy)?;
        let Some(mut opened) = open_source(&self.location, self.trust_policy)? else {
            return Ok(RootCatalogLoad::missing());
        };
        observer(LoadPhase::Opened);

        let document = read_bounded(&mut opened.file, &mut observer)?;
        observer(LoadPhase::ReadComplete);
        revalidate_source(&self.location, self.trust_policy, &opened)?;
        if document.len() > MAX_ROOT_CATALOG_DOCUMENT_BYTES {
            return Err(RootCatalogSourceError::BoundExceeded {
                maximum_bytes: MAX_ROOT_CATALOG_DOCUMENT_BYTES,
            });
        }

        let catalog = parse_root_catalog_v1(&document)?;
        observer(LoadPhase::Parsed);
        revalidate_source(&self.location, self.trust_policy, &opened)?;
        let file_evidence = RootCatalogFileEvidence {
            trust: self.trust_policy.source_trust(),
            owner_uid: opened.file_stat.st_uid,
            mode: mode_bits(opened.file_stat.st_mode),
            document_bytes: document.len(),
        };
        Ok(RootCatalogLoad::loaded(
            catalog,
            file_evidence,
            RetainedSource {
                location: self.location.clone(),
                policy: self.trust_policy,
                opened,
            },
        ))
    }

    #[cfg(test)]
    pub(super) fn native_with_file_owner_for_test(
        location: &str,
        operator_uid: u32,
        file_owner_uid: u32,
    ) -> Self {
        Self {
            location: location.into(),
            trust_policy: LocalTrustPolicy::NativeOverride {
                operator_uid,
                file_owner_uid,
            },
        }
    }
}

impl RootCatalogSource for TrustedLocalRootCatalogSource {
    fn load(&self) -> Result<RootCatalogLoad, RootCatalogSourceError> {
        if trusted_local_source_target_supported() {
            self.load_with_observer(|_phase| {})
        } else {
            Err(RootCatalogSourceError::PlatformUnsupported)
        }
    }
}

const fn trusted_local_source_target_supported() -> bool {
    cfg!(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LoadPhase {
    Opened,
    ReadProgress,
    ReadComplete,
    Parsed,
}

struct OpenedSource {
    file: File,
    directories: Vec<File>,
    directory_stats: Vec<Stat>,
    file_stat: Stat,
}

pub(super) struct RetainedSource {
    location: Box<str>,
    policy: LocalTrustPolicy,
    opened: OpenedSource,
}

impl std::fmt::Debug for RetainedSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RetainedSource")
    }
}

impl RetainedSource {
    pub(super) fn revalidate(&self) -> Result<(), RootCatalogSourceError> {
        validate_policy_identity(self.policy)?;
        revalidate_source(&self.location, self.policy, &self.opened)
    }
}

fn validate_location_bound(location: &str) -> Result<(), RootCatalogSourceError> {
    let valid = location.starts_with('/')
        && location.len() <= MAX_ROOT_CATALOG_PATH_BYTES
        && !location.as_bytes().contains(&0);
    if valid {
        Ok(())
    } else {
        Err(untrusted(RootCatalogTrustViolation::InvalidLocation))
    }
}

fn location_components(location: &str) -> Result<Vec<&str>, RootCatalogSourceError> {
    validate_location_bound(location)?;
    let components: Vec<_> = location[1..].split('/').collect();
    let valid = !components.is_empty()
        && components
            .iter()
            .all(|component| !component.is_empty() && *component != "." && *component != "..");
    if valid {
        Ok(components)
    } else {
        Err(untrusted(RootCatalogTrustViolation::NonCanonicalLocation))
    }
}

fn validate_policy_identity(policy: LocalTrustPolicy) -> Result<(), RootCatalogSourceError> {
    let effective_uid = rustix::process::geteuid().as_raw();
    match policy {
        LocalTrustPolicy::Packaged { service_uid: 0 } => {
            Err(untrusted(RootCatalogTrustViolation::PackagedServiceIsRoot))
        }
        LocalTrustPolicy::Packaged { service_uid } if service_uid != effective_uid => {
            Err(untrusted(RootCatalogTrustViolation::PolicyIdentityMismatch))
        }
        LocalTrustPolicy::NativeOverride { operator_uid, .. } if operator_uid != effective_uid => {
            Err(untrusted(RootCatalogTrustViolation::PolicyIdentityMismatch))
        }
        LocalTrustPolicy::Packaged { .. } | LocalTrustPolicy::NativeOverride { .. } => Ok(()),
    }
}

fn open_source(
    location: &str,
    policy: LocalTrustPolicy,
) -> Result<Option<OpenedSource>, RootCatalogSourceError> {
    let components = location_components(location)?;
    let Some((file_name, directory_components)) = components.split_last() else {
        return Err(untrusted(RootCatalogTrustViolation::NonCanonicalLocation));
    };
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut directory = fs::open("/", directory_flags, Mode::empty())
        .map_err(|source| filesystem("open root directory", source))?;
    let mut directory_stats = Vec::with_capacity(directory_components.len() + 1);
    let mut directories = Vec::with_capacity(directory_components.len() + 1);
    let root_stat =
        fs::fstat(&directory).map_err(|source| filesystem("inspect root directory", source))?;
    validate_directory(&directory, &root_stat, policy)?;
    directory_stats.push(root_stat);

    for component in directory_components {
        let next = match fs::openat(&directory, *component, directory_flags, Mode::empty()) {
            Ok(next) => next,
            Err(Errno::NOENT) => return Ok(None),
            Err(source) => return Err(filesystem("open source directory", source)),
        };
        let stat =
            fs::fstat(&next).map_err(|source| filesystem("inspect source directory", source))?;
        validate_directory(&next, &stat, policy)?;
        directory_stats.push(stat);
        directories.push(File::from(directory));
        directory = next;
    }

    let path_stat = match fs::statat(&directory, *file_name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) => stat,
        Err(Errno::NOENT) => return Ok(None),
        Err(source) => return Err(filesystem("inspect source entry", source)),
    };
    if FileType::from_raw_mode(path_stat.st_mode) != FileType::RegularFile {
        return Err(untrusted(RootCatalogTrustViolation::UnsafeFileType));
    }

    let file_flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file_descriptor = fs::openat(&directory, *file_name, file_flags, Mode::empty())
        .map_err(|source| filesystem("open source file", source))?;
    let file_stat =
        fs::fstat(&file_descriptor).map_err(|source| filesystem("inspect source file", source))?;
    validate_file(&file_stat, policy)?;
    if !stable_file_metadata(&path_stat, &file_stat) {
        return Err(untrusted(RootCatalogTrustViolation::SourceChanged));
    }
    validate_packaged_write_protection(&directory, file_name, policy)?;
    directories.push(File::from(directory));

    Ok(Some(OpenedSource {
        file: File::from(file_descriptor),
        directories,
        directory_stats,
        file_stat,
    }))
}

fn validate_directory(
    directory: &impl std::os::fd::AsFd,
    stat: &Stat,
    policy: LocalTrustPolicy,
) -> Result<(), RootCatalogSourceError> {
    let mode = mode_bits(stat.st_mode);
    let valid = FileType::from_raw_mode(stat.st_mode) == FileType::Directory
        && policy.directory_owner_is_trusted(stat.st_uid)
        && mode & UNTRUSTED_WRITE_BITS == 0;
    if !valid {
        return Err(untrusted(RootCatalogTrustViolation::UnsafeDirectory));
    }
    validate_packaged_directory_write_protection(directory, policy)
}

fn validate_packaged_directory_write_protection(
    directory: &impl std::os::fd::AsFd,
    policy: LocalTrustPolicy,
) -> Result<(), RootCatalogSourceError> {
    if !matches!(policy, LocalTrustPolicy::Packaged { .. }) {
        return Ok(());
    }
    match fs::accessat(directory, ".", Access::WRITE_OK, AtFlags::EACCESS) {
        Ok(()) => Err(untrusted(RootCatalogTrustViolation::WritableByService)),
        Err(Errno::ACCESS | Errno::PERM | Errno::ROFS) => Ok(()),
        Err(source) => Err(filesystem(
            "verify packaged directory write protection",
            source,
        )),
    }
}

fn validate_file(stat: &Stat, policy: LocalTrustPolicy) -> Result<(), RootCatalogSourceError> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(untrusted(RootCatalogTrustViolation::UnsafeFileType));
    }
    if stat.st_nlink != 1 {
        return Err(untrusted(RootCatalogTrustViolation::MultipleHardLinks));
    }
    if stat.st_uid != policy.expected_file_owner() {
        return Err(untrusted(RootCatalogTrustViolation::OwnerMismatch));
    }
    if mode_bits(stat.st_mode) & UNTRUSTED_WRITE_BITS != 0 {
        return Err(untrusted(RootCatalogTrustViolation::UntrustedWritableMode));
    }
    if stat.st_size < 0 {
        return Err(untrusted(RootCatalogTrustViolation::UnsafeFileType));
    }
    Ok(())
}

fn validate_packaged_write_protection(
    directory: &impl std::os::fd::AsFd,
    file_name: &str,
    policy: LocalTrustPolicy,
) -> Result<(), RootCatalogSourceError> {
    if !matches!(policy, LocalTrustPolicy::Packaged { .. }) {
        return Ok(());
    }
    let write_flags = OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    match fs::openat(directory, file_name, write_flags, Mode::empty()) {
        Ok(_writable) => Err(untrusted(RootCatalogTrustViolation::WritableByService)),
        Err(Errno::ACCESS | Errno::PERM | Errno::ROFS) => Ok(()),
        Err(source) => Err(filesystem(
            "verify packaged source write protection",
            source,
        )),
    }
}

fn read_bounded<F>(file: &mut File, observer: &mut F) -> Result<Vec<u8>, RootCatalogSourceError>
where
    F: FnMut(LoadPhase),
{
    let read_limit = MAX_ROOT_CATALOG_DOCUMENT_BYTES + 1;
    let mut document = Vec::with_capacity(READ_CHUNK_BYTES);
    let mut buffer = vec![0_u8; READ_CHUNK_BYTES];
    let mut observed_progress = false;
    while document.len() < read_limit {
        let remaining = read_limit - document.len();
        let request_bytes = remaining.min(buffer.len());
        let read_bytes = file.read(&mut buffer[..request_bytes]).map_err(|source| {
            RootCatalogSourceError::Filesystem {
                operation: "read source file",
                source,
            }
        })?;
        if read_bytes == 0 {
            break;
        }
        document.extend_from_slice(&buffer[..read_bytes]);
        if !observed_progress {
            observer(LoadPhase::ReadProgress);
            observed_progress = true;
        }
    }
    Ok(document)
}

fn revalidate_source(
    location: &str,
    policy: LocalTrustPolicy,
    opened: &OpenedSource,
) -> Result<(), RootCatalogSourceError> {
    for (directory, before) in opened.directories.iter().zip(&opened.directory_stats) {
        let after = fs::fstat(directory)
            .map_err(|source| filesystem("reinspect source directory", source))?;
        validate_directory(directory, &after, policy)?;
        if !stable_directory_metadata(before, &after) {
            return Err(untrusted(RootCatalogTrustViolation::SourceChanged));
        }
    }
    let current_stat =
        fs::fstat(&opened.file).map_err(|source| filesystem("reinspect source file", source))?;
    validate_file(&current_stat, policy)?;
    if !stable_file_metadata(&opened.file_stat, &current_stat) {
        return Err(untrusted(RootCatalogTrustViolation::SourceChanged));
    }

    let Some(reopened) = open_source(location, policy)? else {
        return Err(untrusted(RootCatalogTrustViolation::SourceChanged));
    };
    if opened.directory_stats.len() != reopened.directory_stats.len()
        || !opened
            .directory_stats
            .iter()
            .zip(&reopened.directory_stats)
            .all(|(before, after)| stable_directory_metadata(before, after))
        || !stable_file_metadata(&opened.file_stat, &reopened.file_stat)
    {
        return Err(untrusted(RootCatalogTrustViolation::SourceChanged));
    }
    Ok(())
}

const fn stable_directory_metadata(before: &Stat, after: &Stat) -> bool {
    before.st_dev == after.st_dev
        && before.st_ino == after.st_ino
        && before.st_mode == after.st_mode
        && before.st_uid == after.st_uid
        && before.st_gid == after.st_gid
}

const fn stable_file_metadata(before: &Stat, after: &Stat) -> bool {
    before.st_dev == after.st_dev
        && before.st_ino == after.st_ino
        && before.st_mode == after.st_mode
        && before.st_nlink == after.st_nlink
        && before.st_uid == after.st_uid
        && before.st_gid == after.st_gid
        && before.st_size == after.st_size
        && before.st_mtime == after.st_mtime
        && before.st_mtime_nsec == after.st_mtime_nsec
        && before.st_ctime == after.st_ctime
        && before.st_ctime_nsec == after.st_ctime_nsec
}

const fn untrusted(violation: RootCatalogTrustViolation) -> RootCatalogSourceError {
    RootCatalogSourceError::SourceUntrusted { violation }
}

fn filesystem(operation: &'static str, source: Errno) -> RootCatalogSourceError {
    RootCatalogSourceError::Filesystem {
        operation,
        source: source.into(),
    }
}
