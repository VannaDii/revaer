//! Filesystem identity boundary for media discovery roots.

use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

/// Canonical identity captured for an existing media root directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaRootIdentity {
    requested_path: PathBuf,
    canonical_path: PathBuf,
    filesystem_device: u64,
    filesystem_inode: u64,
}

impl MediaRootIdentity {
    /// Returns the operator-supplied absolute path.
    #[must_use]
    pub fn requested_path(&self) -> &Path {
        &self.requested_path
    }

    /// Returns the fully resolved canonical path.
    #[must_use]
    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    /// Returns the filesystem device identifier.
    #[must_use]
    pub const fn filesystem_device(&self) -> u64 {
        self.filesystem_device
    }

    /// Returns the filesystem inode identifier.
    #[must_use]
    pub const fn filesystem_inode(&self) -> u64 {
        self.filesystem_inode
    }

    /// Returns true when a fresh observation identifies the same directory.
    #[must_use]
    pub const fn matches(&self, fresh: &Self) -> bool {
        self.filesystem_device == fresh.filesystem_device
            && self.filesystem_inode == fresh.filesystem_inode
    }
}

/// Failure while resolving a media root to stable filesystem identity.
#[derive(Debug)]
pub enum MediaRootIdentityError {
    /// The requested root is not absolute.
    PathNotAbsolute(PathBuf),
    /// The requested root could not be canonicalized or inspected.
    Io(std::io::Error),
    /// The resolved path does not identify a directory.
    NotDirectory(PathBuf),
    /// This platform does not expose the stable identity required by the contract.
    UnsupportedPlatform,
}

impl Display for MediaRootIdentityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathNotAbsolute(path) => {
                write!(formatter, "media root is not absolute: {}", path.display())
            }
            Self::Io(error) => write!(formatter, "media root filesystem operation failed: {error}"),
            Self::NotDirectory(path) => {
                write!(
                    formatter,
                    "media root is not a directory: {}",
                    path.display()
                )
            }
            Self::UnsupportedPlatform => formatter
                .write_str("media root identity requires filesystem device and inode support"),
        }
    }
}

impl std::error::Error for MediaRootIdentityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::PathNotAbsolute(_) | Self::NotDirectory(_) | Self::UnsupportedPlatform => None,
        }
    }
}

impl From<std::io::Error> for MediaRootIdentityError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Injected boundary used before persisting or revalidating a discovery root.
pub trait MediaRootIdentityResolver: Send + Sync {
    /// Resolves an absolute existing directory to canonical filesystem identity.
    ///
    /// # Errors
    /// Returns an error when the path is relative, missing, not a directory, or
    /// the platform cannot provide stable device and inode identifiers.
    fn resolve(&self, path: &Path) -> Result<MediaRootIdentity, MediaRootIdentityError>;
}

/// Standard-library filesystem identity resolver for Unix hosts.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdMediaRootIdentityResolver;

impl MediaRootIdentityResolver for StdMediaRootIdentityResolver {
    fn resolve(&self, path: &Path) -> Result<MediaRootIdentity, MediaRootIdentityError> {
        if !path.is_absolute() {
            return Err(MediaRootIdentityError::PathNotAbsolute(path.to_path_buf()));
        }

        let canonical_path = std::fs::canonicalize(path)?;
        let metadata = std::fs::metadata(&canonical_path)?;
        if !metadata.is_dir() {
            return Err(MediaRootIdentityError::NotDirectory(canonical_path));
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            Ok(MediaRootIdentity {
                requested_path: path.to_path_buf(),
                canonical_path,
                filesystem_device: metadata.dev(),
                filesystem_inode: metadata.ino(),
            })
        }

        #[cfg(not(unix))]
        {
            let _ = metadata;
            Err(MediaRootIdentityError::UnsupportedPlatform)
        }
    }
}
