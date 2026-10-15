//! Descriptor-derived identity values; never a deployment or capability grant.

use std::fmt;

/// Snapshot of a revalidated retained root and its matching mount record.
/// No readiness, durability, writer ownership or probe success is inferred.
#[derive(PartialEq, Eq)]
pub struct RootDirectoryObservation {
    pub(super) canonical_path: String,
    pub(super) filesystem_device: u64,
    pub(super) filesystem_inode: u64,
    pub(super) mount_id: u64,
    pub(super) filesystem_type: String,
    pub(super) owner_uid: u32,
    pub(super) owner_gid: u32,
    pub(super) mode_bits: u32,
}

impl fmt::Debug for RootDirectoryObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RootDirectoryObservation")
    }
}

impl RootDirectoryObservation {
    /// Exact protected no-symlink path; sensitive, not suitable for general logs.
    #[must_use]
    pub fn canonical_path(&self) -> &str {
        &self.canonical_path
    }
    /// Unsigned descriptor device identity.
    #[must_use]
    pub const fn filesystem_device(&self) -> u64 {
        self.filesystem_device
    }
    /// Unsigned descriptor inode identity.
    #[must_use]
    pub const fn filesystem_inode(&self) -> u64 {
        self.filesystem_inode
    }
    /// Descriptor-derived Linux mount ID matched against the supplied snapshot.
    #[must_use]
    pub const fn mount_id(&self) -> u64 {
        self.mount_id
    }
    /// Filesystem name from the matching mount record, not a support assertion.
    #[must_use]
    pub fn filesystem_type(&self) -> &str {
        &self.filesystem_type
    }
    /// Observed unsigned owner identity.
    #[must_use]
    pub const fn owner_uid(&self) -> u32 {
        self.owner_uid
    }
    /// Observed unsigned group identity.
    #[must_use]
    pub const fn owner_gid(&self) -> u32 {
        self.owner_gid
    }
    /// Permission and special bits only, excluding the directory type bits.
    #[must_use]
    pub const fn mode_bits(&self) -> u32 {
        self.mode_bits
    }
}
