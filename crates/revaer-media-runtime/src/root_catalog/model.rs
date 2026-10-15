use sha2::{Digest, Sha256};

const CATALOG_DIGEST_DOMAIN: &[u8] = b"revaer-media-root-catalog";

/// Exact media-root role that a catalog slot may serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RootKind {
    /// Read-only admitted source media.
    Source,
    /// Final transcoding output.
    Output,
    /// Attempt-scoped managed workspace.
    Workspace,
    /// Replacement backup material.
    Backup,
    /// Quarantined material retained for operator review.
    Quarantine,
}

impl RootKind {
    pub(super) const ORDERED: [Self; 5] = [
        Self::Source,
        Self::Output,
        Self::Workspace,
        Self::Backup,
        Self::Quarantine,
    ];

    pub(super) const fn mask(self) -> u8 {
        match self {
            Self::Source => 1 << 0,
            Self::Output => 1 << 1,
            Self::Workspace => 1 << 2,
            Self::Backup => 1 << 3,
            Self::Quarantine => 1 << 4,
        }
    }
}

/// Declared persistence behavior for one root slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityClass {
    /// No survival guarantee is declared.
    Disposable,
    /// Ordinary process and container or pod replacement survival is declared.
    RestartPersistent,
}

impl DurabilityClass {
    pub(super) const fn canonical_byte(self) -> u8 {
        match self {
            Self::Disposable => 0,
            Self::RestartPersistent => 1,
        }
    }
}

/// Closed deployment evidence declaration for root durability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityEvidence {
    /// No durability evidence is declared.
    None,
    /// A dedicated native Linux mount is declared.
    LinuxDedicatedMount,
    /// A Kubernetes persistent volume claim is declared.
    KubernetesPersistentVolumeClaim,
}

impl DurabilityEvidence {
    pub(super) const fn canonical_byte(self) -> u8 {
        match self {
            Self::None => 0,
            Self::LinuxDedicatedMount => 1,
            Self::KubernetesPersistentVolumeClaim => 2,
        }
    }
}

/// Declared write-ownership behavior for one root slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoleWriterClass {
    /// Exclusive writer ownership is not declared.
    Uncontrolled,
    /// One Revaer deployment is declared as the exclusive writer.
    RevaerExclusive,
}

impl SoleWriterClass {
    pub(super) const fn canonical_byte(self) -> u8 {
        match self {
            Self::Uncontrolled => 0,
            Self::RevaerExclusive => 1,
        }
    }
}

/// Closed deployment evidence declaration for sole-writer control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoleWriterEvidence {
    /// No sole-writer evidence is declared.
    None,
    /// A dedicated native Linux service is declared.
    LinuxDedicatedService,
    /// A single Kubernetes pod with `ReadWriteOncePod` storage is declared.
    KubernetesReadWriteOncePod,
}

impl SoleWriterEvidence {
    pub(super) const fn canonical_byte(self) -> u8 {
        match self {
            Self::None => 0,
            Self::LinuxDedicatedService => 1,
            Self::KubernetesReadWriteOncePod => 2,
        }
    }
}

/// One validated version-1 logical root declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootCatalogSlot {
    key: Box<str>,
    key_byte_len: u32,
    allowed_kinds: Box<[RootKind]>,
    path: Box<str>,
    path_byte_len: u32,
    durability_class: DurabilityClass,
    durability_evidence: DurabilityEvidence,
    sole_writer_class: SoleWriterClass,
    sole_writer_evidence: SoleWriterEvidence,
}

pub(super) struct ValidatedRootCatalogSlot {
    pub(super) key: String,
    pub(super) key_byte_len: u32,
    pub(super) allowed_kinds: Vec<RootKind>,
    pub(super) path: String,
    pub(super) path_byte_len: u32,
    pub(super) durability_class: DurabilityClass,
    pub(super) durability_evidence: DurabilityEvidence,
    pub(super) sole_writer_class: SoleWriterClass,
    pub(super) sole_writer_evidence: SoleWriterEvidence,
}

impl RootCatalogSlot {
    pub(super) fn validated(fields: ValidatedRootCatalogSlot) -> Self {
        Self {
            key: fields.key.into_boxed_str(),
            key_byte_len: fields.key_byte_len,
            allowed_kinds: fields.allowed_kinds.into_boxed_slice(),
            path: fields.path.into_boxed_str(),
            path_byte_len: fields.path_byte_len,
            durability_class: fields.durability_class,
            durability_evidence: fields.durability_evidence,
            sole_writer_class: fields.sole_writer_class,
            sole_writer_evidence: fields.sole_writer_evidence,
        }
    }

    /// Return the normalized logical slot key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Return root kinds in the fixed canonical order.
    #[must_use]
    pub fn allowed_kinds(&self) -> &[RootKind] {
        &self.allowed_kinds
    }

    /// Return the exact decoded absolute UTF-8 path bytes as a string.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Return the declared durability class.
    #[must_use]
    pub const fn durability_class(&self) -> DurabilityClass {
        self.durability_class
    }

    /// Return the declared durability evidence class.
    #[must_use]
    pub const fn durability_evidence(&self) -> DurabilityEvidence {
        self.durability_evidence
    }

    /// Return the declared sole-writer class.
    #[must_use]
    pub const fn sole_writer_class(&self) -> SoleWriterClass {
        self.sole_writer_class
    }

    /// Return the declared sole-writer evidence class.
    #[must_use]
    pub const fn sole_writer_evidence(&self) -> SoleWriterEvidence {
        self.sole_writer_evidence
    }

    fn kind_mask(&self) -> u8 {
        self.allowed_kinds
            .iter()
            .fold(0, |mask, kind| mask | kind.mask())
    }
}

/// Fully validated version-1 media-root catalog and semantic identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootCatalog {
    slots: Box<[RootCatalogSlot]>,
    semantic_sha256: [u8; 32],
}

impl RootCatalog {
    pub(super) fn from_validated_slots(mut slots: Vec<RootCatalogSlot>, slot_count: u32) -> Self {
        slots.sort_unstable_by(|left, right| left.key.cmp(&right.key));
        let semantic_sha256 = canonical_digest(&slots, slot_count);
        Self {
            slots: slots.into_boxed_slice(),
            semantic_sha256,
        }
    }

    /// Return the exact format version represented by this catalog.
    #[must_use]
    pub const fn format_version(&self) -> u32 {
        1
    }

    /// Return validated slots sorted by normalized key.
    #[must_use]
    pub fn slots(&self) -> &[RootCatalogSlot] {
        &self.slots
    }

    /// Return the canonical semantic SHA-256 digest bytes.
    #[must_use]
    pub const fn semantic_sha256(&self) -> [u8; 32] {
        self.semantic_sha256
    }

    /// Return whether the catalog contains no configured root slots.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub(super) fn empty() -> Self {
        Self::from_validated_slots(Vec::new(), 0)
    }
}

fn canonical_digest(slots: &[RootCatalogSlot], slot_count: u32) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(CATALOG_DIGEST_DOMAIN);
    digest.update([0]);
    digest.update(1_u32.to_be_bytes());
    digest.update(slot_count.to_be_bytes());

    for slot in slots {
        digest.update(slot.key_byte_len.to_be_bytes());
        digest.update(slot.key.as_bytes());
        digest.update(slot.path_byte_len.to_be_bytes());
        digest.update(slot.path.as_bytes());
        digest.update([slot.kind_mask()]);
        digest.update([slot.durability_class.canonical_byte()]);
        digest.update([slot.durability_evidence.canonical_byte()]);
        digest.update([slot.sole_writer_class.canonical_byte()]);
        digest.update([slot.sole_writer_evidence.canonical_byte()]);
    }

    digest.finalize().into()
}
