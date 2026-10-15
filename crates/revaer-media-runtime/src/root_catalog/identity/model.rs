use thiserror::Error;

use super::super::RootCatalogSlot;

/// Caller-supplied normalized fields, not descriptor or deployment proof.
///
/// There is deliberately no constructor that labels these claims attested. The
/// encoder checks their shape and catalog coherence only. A resolver must still
/// independently prove and retain descriptors, locks, ancestry, mount aliases,
/// deployment evidence, and capability probes before reconciliation or use.
/// Database ids and timestamps cannot be supplied and never affect identity.
#[derive(Debug, Clone, Copy)]
pub struct RootSlotIdentityClaims<'a> {
    /// Exact parsed declaration; must equal the injected source's slot by key.
    pub declaration: &'a RootCatalogSlot,
    /// Claimed descriptor-resolved path; encoding does not resolve this path.
    pub canonical_path: &'a str,
    /// Unsigned device identity, encoded as exactly eight big-endian bytes.
    pub filesystem_device: u64,
    /// Unsigned inode identity, encoded as exactly eight big-endian bytes.
    pub filesystem_inode: u64,
    /// Claimed Linux mount id, restricted to the nonnegative SQL bigint range.
    pub mount_id: u64,
    /// Claimed filesystem type, one through 64 UTF-8 bytes without SQL NUL.
    pub filesystem_type: &'a str,
    /// Exact seven probe booleans in ADR 557 column order: read, write,
    /// create-new, fsync, rename, delete, capacity. No reserved bit is accepted.
    pub capabilities: [bool; 7],
    /// Unsigned owner UID, including the complete u32 range.
    pub owner_uid: u32,
    /// Unsigned owner GID, including the complete u32 range.
    pub owner_gid: u32,
    /// Permission and special mode bits only, zero through 4095.
    pub mode_bits: u32,
}

/// Exact slot frame and digest of validated claims, never proof of a root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootSlotIdentityEncoding {
    pub(super) logical_key: Box<str>,
    pub(super) frame: Box<[u8]>,
    pub(super) sha256: [u8; 32],
}

impl RootSlotIdentityEncoding {
    /// Return the decoded logical key used in aggregate ordering.
    #[must_use]
    pub fn logical_key(&self) -> &str {
        &self.logical_key
    }

    /// Return the complete ADR 557 slot frame, including its domain and version.
    /// This contains sensitive paths; it is not suitable for general logs.
    #[must_use]
    pub fn frame(&self) -> &[u8] {
        &self.frame
    }

    /// Return SHA-256 of the complete slot frame.
    #[must_use]
    pub const fn root_identity_sha256(&self) -> [u8; 32] {
        self.sha256
    }
}

/// Catalog-matched identity bytes, not an attestation, activation, or fence.
///
/// This value grants no readiness or write authority. The generation digest is
/// semantic content identity, not the positive monotonic database generation
/// occurrence. Returning to identical claims returns identical digest bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootCatalogIdentityEncoding {
    pub(super) source_sha256: [u8; 32],
    pub(super) slots: Box<[RootSlotIdentityEncoding]>,
    pub(super) attestation_sha256: [u8; 32],
    pub(super) generation_sha256: [u8; 32],
}

impl RootCatalogIdentityEncoding {
    /// Return the unchanged ADR 550 semantic source digest.
    #[must_use]
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }

    /// Return every slot encoding in decoded logical-key byte order.
    #[must_use]
    pub fn slots(&self) -> &[RootSlotIdentityEncoding] {
        &self.slots
    }

    /// Return SHA-256 of the complete aggregate attestation frame.
    #[must_use]
    pub const fn attestation_sha256(&self) -> [u8; 32] {
        self.attestation_sha256
    }

    /// Return SHA-256 of the source-plus-attestation generation frame.
    #[must_use]
    pub const fn generation_sha256(&self) -> [u8; 32] {
        self.generation_sha256
    }
}

/// Bounded encoding failure without paths, keys, or filesystem identifiers.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum RootIdentityEncodingError {
    /// A missing source is not a loaded zero-slot document.
    #[error("root identity encoding requires a loaded catalog source")]
    SourceNotLoaded,
    /// More than the accepted maximum 256 claims were supplied.
    #[error("root identity slot bound exceeded")]
    SlotBoundExceeded,
    /// A logical key was supplied twice.
    #[error("root identity contains a duplicate slot key")]
    DuplicateSlot,
    /// A supplied key does not occur in the source.
    #[error("root identity contains a slot absent from the source")]
    UnexpectedSlot,
    /// A source slot has no supplied claims.
    #[error("root identity is missing a source slot")]
    MissingSlot,
    /// A supplied declaration differs from the active injected catalog.
    #[error("root identity declaration differs from the source")]
    DeclarationMismatch,
    /// A claimed canonical path violates the normalized scalar contract.
    #[error("root identity canonical path is invalid")]
    InvalidCanonicalPath,
    /// A mount id is not representable by the normalized SQL column.
    #[error("root identity mount id exceeds SQL bigint")]
    InvalidMountId,
    /// A filesystem type is empty, too long, or contains SQL NUL.
    #[error("root identity filesystem type is invalid")]
    InvalidFilesystemType,
    /// A mode contains bits beyond the accepted 12-bit mode field.
    #[error("root identity mode bits exceed the accepted range")]
    InvalidModeBits,
    /// A kind lacks its required read or write capability claims.
    #[error("root identity capabilities do not satisfy the declared kinds")]
    CapabilityMismatch,
    /// A writing kind lacks an exclusive-writer declaration.
    #[error("root identity writing kind lacks exclusive writer control")]
    WriterControlMismatch,
    /// Claimed paths overlap or two slots claim the same device/inode pair.
    #[error("root identity claims overlap")]
    Overlap,
}
