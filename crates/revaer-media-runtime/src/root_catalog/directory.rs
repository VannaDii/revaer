//! Retained directory links and root locks. Mount/deployment proof is separate.

use std::{fmt, fs::File, io};

use rustix::fs::{self, AtFlags, FileType, FlockOperation, Mode, OFlags, Stat};
use thiserror::Error;

use super::{RootCatalogSlot, RootKind, RootWriteProbeError, SoleWriterClass, write_probe};

const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

/// Bounded opening/revalidation failure; diagnostics never contain root paths.
#[derive(Debug, Error)]
pub enum RootDirectoryError {
    /// Public attestation support is restricted to the approved Linux targets.
    #[error("root directory platform is unsupported")]
    PlatformUnsupported,
    /// The supplied service identity does not match the effective process UID.
    #[error("root directory service identity mismatch")]
    ServiceIdentityMismatch,
    /// The declaration requires normalization, traversal or an invalid path.
    #[error("root directory path is invalid")]
    InvalidPath,
    /// A descendant candidate parent disappeared while the declared root remained valid.
    #[error("source candidate parent is missing")]
    CandidateMissing,
    /// An ancestor is not a protected directory owned by root or the service.
    #[error("root directory ancestry is unsafe")]
    UnsafeAncestry,
    /// Descriptor identity or a retained parent-to-child link changed.
    #[error("root directory identity changed")]
    IdentityChanged,
    /// The kernel did not report a mount identity within the persistence bound.
    #[error("root directory mount identity is unavailable")]
    MountIdentityUnavailable,
    /// Distinct catalog slots share a root or contain one another.
    #[error("root directory overlaps another catalog slot")]
    Overlap,
    /// The source declaration does not authorize writing.
    #[error("root directory writing is not declared")]
    WriteNotDeclared,
    /// A required descriptor operation failed.
    #[error("root directory operation failed during {operation}")]
    Filesystem {
        /// Closed operation label, never caller input.
        operation: &'static str,
        /// Original operating-system error.
        #[source]
        source: io::Error,
    },
    /// A required capability or owned cleanup failed.
    #[error(transparent)]
    Probe(#[from] RootWriteProbeError),
    /// The mount namespace snapshot cannot establish distinct root locations.
    #[cfg(target_os = "linux")]
    #[error(transparent)]
    Mount(#[from] super::RootMountError),
    /// A fresh process mount namespace could not be observed.
    #[cfg(target_os = "linux")]
    #[error(transparent)]
    MountRead(#[from] super::RootMountReadError),
}

struct DirectoryLink {
    name: String,
    file: File,
    stat: Stat,
    #[cfg(target_os = "linux")]
    mount_id: u64,
}

/// A retained, no-symlink directory chain and any declared exclusive root lock.
///
/// This is not a complete root attestation: callers still must prove trusted
/// declaration provenance, deployment ownership, mount identity/aliases and
/// durability, and retain this value while any bound operation is admitted.
/// No clone or descriptor export can accidentally extend the public lock lifetime.
pub struct OpenedRootDirectory {
    links: Vec<DirectoryLink>,
    service_uid: u32,
    writing: bool,
}

impl fmt::Debug for OpenedRootDirectory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OpenedRootDirectory")
    }
}

impl OpenedRootDirectory {
    /// Open a trusted declaration without following or canonicalizing symlinks.
    /// All components must be protected; no implicit writable-ancestor exception
    /// is inferred. Writing kinds require exclusive declaration and a held lock.
    /// This operation does not create files or perform capability probes.
    ///
    /// # Errors
    /// Rejects unsupported hosts, identity mismatch, unsafe components, changed
    /// links and a competing root lock. No partially opened tree is retained.
    pub fn open(slot: &RootCatalogSlot, service_uid: u32) -> Result<Self, RootDirectoryError> {
        if !cfg!(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )) {
            return Err(RootDirectoryError::PlatformUnsupported);
        }
        Self::open_descriptors(slot, service_uid)
    }

    fn open_descriptors(
        slot: &RootCatalogSlot,
        service_uid: u32,
    ) -> Result<Self, RootDirectoryError> {
        if service_uid != rustix::process::geteuid().as_raw() {
            return Err(RootDirectoryError::ServiceIdentityMismatch);
        }
        let components = components(slot.path())?;
        let writing = slot
            .allowed_kinds()
            .iter()
            .any(|kind| *kind != RootKind::Source);
        if writing && slot.sole_writer_class() != SoleWriterClass::RevaerExclusive {
            return Err(RootDirectoryError::WriteNotDeclared);
        }
        let root = fs::open("/", DIRECTORY_FLAGS, Mode::empty())
            .map_err(|error| failure("open filesystem root", error))?;
        let root = link("/", File::from(root), service_uid)?;
        let mut opened = Self {
            links: vec![root],
            service_uid,
            writing,
        };
        for component in components {
            let next = fs::openat(
                &opened.last()?.file,
                component,
                DIRECTORY_FLAGS,
                Mode::empty(),
            )
            .map_err(|error| failure("open root component", error))?;
            opened
                .links
                .push(link(component, File::from(next), service_uid)?);
        }
        if slot.sole_writer_class() == SoleWriterClass::RevaerExclusive {
            fs::flock(
                &opened.last()?.file,
                FlockOperation::NonBlockingLockExclusive,
            )
            .map_err(|error| failure("acquire exclusive root lock", error))?;
        }
        opened.revalidate()?;
        Ok(opened)
    }

    /// Recheck every retained descriptor and parent-to-child directory entry.
    /// Linux mount identities are checked as well; deployment and durability
    /// evidence still require separate validation.
    ///
    /// # Errors
    /// Fails closed on renamed/replaced entries, changed ownership or permissions.
    pub fn revalidate(&self) -> Result<(), RootDirectoryError> {
        let first = self
            .links
            .first()
            .ok_or(RootDirectoryError::IdentityChanged)?;
        let current_root = fs::open("/", DIRECTORY_FLAGS, Mode::empty())
            .map_err(|error| failure("reopen filesystem root", error))?;
        let current_stat =
            fs::fstat(&current_root).map_err(|error| failure("inspect filesystem root", error))?;
        same_identity(&first.stat, &current_stat)?;
        #[cfg(target_os = "linux")]
        same_mount(
            first.mount_id,
            mount_id(&current_root, "", AtFlags::EMPTY_PATH)?,
        )?;
        for entry in &self.links {
            let stat = fs::fstat(&entry.file)
                .map_err(|error| failure("revalidate directory descriptor", error))?;
            validate(&stat, self.service_uid)?;
            same_identity(&entry.stat, &stat)?;
            #[cfg(target_os = "linux")]
            same_mount(
                entry.mount_id,
                mount_id(&entry.file, "", AtFlags::EMPTY_PATH)?,
            )?;
        }
        for pair in self.links.windows(2) {
            let stat = fs::statat(
                &pair[0].file,
                pair[1].name.as_str(),
                AtFlags::SYMLINK_NOFOLLOW,
            )
            .map_err(|error| failure("revalidate directory link", error))?;
            same_identity(&pair[1].stat, &stat)?;
            #[cfg(target_os = "linux")]
            same_mount(
                pair[1].mount_id,
                mount_id(&pair[0].file, &pair[1].name, AtFlags::SYMLINK_NOFOLLOW)?,
            )?;
        }
        Ok(())
    }

    /// Check root traversal and directory enumeration without creating entries.
    /// This does not prove that every descendant file is readable, or establish
    /// deployment ownership or durability.
    ///
    /// # Errors
    /// Rejects changed links and failed traversal or enumeration operations.
    pub fn probe_reads(&self) -> Result<(), RootDirectoryError> {
        self.revalidate()?;
        let traversed = fs::openat(&self.last()?.file, ".", DIRECTORY_FLAGS, Mode::empty())
            .map_err(|error| failure("probe root traversal", error))?;
        let mut entries = fs::Dir::read_from(&traversed)
            .map_err(|error| failure("open root enumeration", error))?;
        // One read exercises enumeration without walking an unbounded tree.
        if let Some(entry) = entries.next() {
            entry.map_err(|error| failure("probe root enumeration", error))?;
        }
        self.revalidate()
    }

    /// Open a candidate's parent beneath this retained root without symlinks.
    /// The returned read-only descriptor is a new open-file description, not
    /// a clone of the root's lock-bearing descriptor. Keep the catalog alive
    /// and repeat the generation fence while using it.
    ///
    /// # Errors
    /// Rejects non-component-safe relative candidates, changed root links and
    /// missing, symlinked or unreadable parent directories. Never creates files.
    pub fn open_read_parent(
        &self,
        relative_candidate: &std::path::Path,
    ) -> Result<std::os::fd::OwnedFd, RootDirectoryError> {
        let relative = relative_candidate
            .to_str()
            .ok_or(RootDirectoryError::InvalidPath)?;
        if relative.is_empty() || relative.len() > 4096 || relative.contains(['\\', '\0']) {
            return Err(RootDirectoryError::InvalidPath);
        }
        let components = relative.split('/').collect::<Vec<_>>();
        if components
            .iter()
            .any(|part| matches!(*part, "" | "." | ".."))
        {
            return Err(RootDirectoryError::InvalidPath);
        }
        self.revalidate()?;
        let mut parent = fs::openat(&self.last()?.file, ".", DIRECTORY_FLAGS, Mode::empty())
            .map_err(|error| failure("open retained root traversal", error))?;
        for component in &components[..components.len() - 1] {
            parent = match fs::openat(&parent, *component, DIRECTORY_FLAGS, Mode::empty()) {
                Ok(parent) => parent,
                Err(error) if error == rustix::io::Errno::NOENT => {
                    self.revalidate()?;
                    return Err(RootDirectoryError::CandidateMissing);
                }
                Err(error) => return Err(failure("open candidate parent", error)),
            };
        }
        self.revalidate()?;
        Ok(parent)
    }

    /// Exercise writable capabilities while retaining the exclusive root lock.
    /// The caller must first establish deployment ownership and mount evidence.
    ///
    /// # Errors
    /// Rejects source-only declarations and changed directory links before any
    /// write. Rechecks links after successful probes; preserves probe failures.
    pub fn probe_writes(&self) -> Result<(), RootDirectoryError> {
        if !self.writing {
            return Err(RootDirectoryError::WriteNotDeclared);
        }
        self.revalidate()?;
        write_probe::probe_root_writes(&self.last()?.file)?;
        self.revalidate()
    }

    #[cfg(target_os = "linux")]
    pub(super) fn probe_capabilities(&self) -> Result<[bool; 7], RootDirectoryError> {
        self.probe_reads()?;
        if self.writing {
            self.probe_writes()?;
        }
        Ok([
            true,
            self.writing,
            self.writing,
            self.writing,
            self.writing,
            self.writing,
            self.writing,
        ])
    }

    #[cfg(any(target_os = "linux", test))]
    pub(super) fn reject_observed_overlap(&self, other: &Self) -> Result<(), RootDirectoryError> {
        self.revalidate()?;
        other.revalidate()?;
        let first = &self.last()?.stat;
        let second = &other.last()?.stat;
        let contains = |links: &[DirectoryLink], root: &Stat| {
            links
                .iter()
                .any(|entry| entry.stat.st_dev == root.st_dev && entry.stat.st_ino == root.st_ino)
        };
        if contains(&self.links, second) || contains(&other.links, first) {
            return Err(RootDirectoryError::Overlap);
        }
        Ok(())
    }

    fn last(&self) -> Result<&DirectoryLink, RootDirectoryError> {
        self.links.last().ok_or(RootDirectoryError::IdentityChanged)
    }

    /// Read identity from retained descriptors and a matching mount snapshot.
    /// This performs no writes and does not establish deployment or durability.
    ///
    /// # Errors
    /// Rejects changed links and missing/mismatched mount evidence. Snapshot
    /// freshness remains the caller's responsibility at each admission boundary.
    #[cfg(target_os = "linux")]
    pub fn observe(
        &self,
        topology: &super::RootMountTopology,
    ) -> Result<super::RootDirectoryObservation, RootDirectoryError> {
        self.revalidate()?;
        let root = self.last()?;
        let path: std::path::PathBuf = self.links.iter().map(|link| link.name.as_str()).collect();
        let location = topology.resolve(
            root.mount_id,
            (fs::major(root.stat.st_dev), fs::minor(root.stat.st_dev)),
            &path,
        )?;
        let observed = super::RootDirectoryObservation {
            canonical_path: path.to_str().ok_or(RootDirectoryError::InvalidPath)?.into(),
            filesystem_device: root.stat.st_dev,
            filesystem_inode: root.stat.st_ino,
            mount_id: root.mount_id,
            filesystem_type: location.filesystem_type().into(),
            owner_uid: root.stat.st_uid,
            owner_gid: root.stat.st_gid,
            mode_bits: mode(root.stat.st_mode) & 0o7777,
        };
        self.revalidate()?;
        Ok(observed)
    }

    #[cfg(target_os = "linux")]
    pub(super) fn mount_regions(
        &self,
        topology: &super::RootMountTopology,
    ) -> Result<Vec<super::mounts::MountLocation>, RootDirectoryError> {
        self.revalidate()?;
        let root = self.last()?;
        let path: std::path::PathBuf = self.links.iter().map(|link| link.name.as_str()).collect();
        Ok(topology.regions(
            root.mount_id,
            (fs::major(root.stat.st_dev), fs::minor(root.stat.st_dev)),
            &path,
        )?)
    }
}

fn components(path: &str) -> Result<Vec<&str>, RootDirectoryError> {
    if path.len() > 4096 || path.contains('\0') {
        return Err(RootDirectoryError::InvalidPath);
    }
    let relative = path
        .strip_prefix('/')
        .ok_or(RootDirectoryError::InvalidPath)?;
    let parts: Vec<_> = relative.split('/').collect();
    if parts.iter().any(|part| matches!(*part, "" | "." | "..")) {
        return Err(RootDirectoryError::InvalidPath);
    }
    Ok(parts)
}

fn link(name: &str, file: File, service_uid: u32) -> Result<DirectoryLink, RootDirectoryError> {
    let stat = fs::fstat(&file).map_err(|error| failure("inspect directory descriptor", error))?;
    validate(&stat, service_uid)?;
    Ok(DirectoryLink {
        name: name.into(),
        #[cfg(target_os = "linux")]
        mount_id: mount_id(&file, "", AtFlags::EMPTY_PATH)?,
        file,
        stat,
    })
}

#[cfg(target_os = "linux")]
fn mount_id(
    directory: &impl std::os::fd::AsFd,
    name: &str,
    flags: AtFlags,
) -> Result<u64, RootDirectoryError> {
    let stat = fs::statx(directory, name, flags, fs::StatxFlags::MNT_ID)
        .map_err(|error| failure("read directory mount identity", error))?;
    validate_mount_id(
        stat.stx_mask & fs::StatxFlags::MNT_ID.bits() != 0,
        stat.stx_mnt_id,
    )
}

#[cfg(any(target_os = "linux", test))]
fn validate_mount_id(reported: bool, value: u64) -> Result<u64, RootDirectoryError> {
    if !reported || i64::try_from(value).is_err() {
        return Err(RootDirectoryError::MountIdentityUnavailable);
    }
    Ok(value)
}

#[cfg(any(target_os = "linux", test))]
const fn same_mount(expected: u64, actual: u64) -> Result<(), RootDirectoryError> {
    if expected != actual {
        return Err(RootDirectoryError::IdentityChanged);
    }
    Ok(())
}

fn mode(value: impl Into<u32>) -> u32 {
    value.into()
}

fn validate(stat: &Stat, service_uid: u32) -> Result<(), RootDirectoryError> {
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || (stat.st_uid != 0 && stat.st_uid != service_uid)
        || mode(stat.st_mode) & 0o022 != 0
    {
        return Err(RootDirectoryError::UnsafeAncestry);
    }
    Ok(())
}

const fn same_identity(expected: &Stat, actual: &Stat) -> Result<(), RootDirectoryError> {
    if expected.st_dev != actual.st_dev
        || expected.st_ino != actual.st_ino
        || expected.st_uid != actual.st_uid
        || expected.st_gid != actual.st_gid
        || expected.st_mode != actual.st_mode
    {
        return Err(RootDirectoryError::IdentityChanged);
    }
    Ok(())
}

fn failure(operation: &'static str, source: impl Into<io::Error>) -> RootDirectoryError {
    RootDirectoryError::Filesystem {
        operation,
        source: source.into(),
    }
}

#[cfg(test)]
mod tests;
