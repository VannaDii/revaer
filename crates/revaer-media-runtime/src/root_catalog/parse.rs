use std::collections::BTreeSet;

use serde::Deserialize;
use thiserror::Error;

use super::model::{
    DurabilityClass, DurabilityEvidence, RootCatalog, RootCatalogSlot, RootKind, SoleWriterClass,
    SoleWriterEvidence, ValidatedRootCatalogSlot,
};

/// Maximum accepted raw JSON document size.
pub const MAX_ROOT_CATALOG_DOCUMENT_BYTES: usize = 8_388_608;
/// Maximum accepted number of catalog slots.
pub const MAX_ROOT_CATALOG_SLOTS: usize = 256;
/// Maximum decoded byte length of a normalized root key.
pub const MAX_ROOT_CATALOG_KEY_BYTES: usize = 64;
/// Maximum decoded byte length of a catalog or override path.
pub const MAX_ROOT_CATALOG_PATH_BYTES: usize = 4_096;

/// Exact version-1 root-catalog validation failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RootCatalogParseError {
    /// The raw JSON document exceeded its reviewed byte bound.
    #[error("root catalog document exceeds {maximum_bytes} bytes")]
    DocumentBoundExceeded {
        /// Reviewed maximum raw document size.
        maximum_bytes: usize,
    },
    /// JSON syntax, shape, field uniqueness, field set, or enum value was invalid.
    #[error("root catalog JSON shape is invalid")]
    MalformedDocument,
    /// The document did not select exact format version 1.
    #[error("unsupported root catalog format version {actual}")]
    UnsupportedVersion {
        /// Parsed integer format version.
        actual: u32,
    },
    /// The catalog exceeded the reviewed slot bound.
    #[error("root catalog contains more than {maximum_slots} slots")]
    SlotBoundExceeded {
        /// Reviewed maximum slot count.
        maximum_slots: usize,
    },
    /// A key was empty, too long, or contained bytes outside the closed grammar.
    #[error("root catalog slot {slot_index} has an invalid key")]
    InvalidKey {
        /// Zero-based slot position in the source document.
        slot_index: usize,
    },
    /// Two slots declared the same normalized key.
    #[error("root catalog slot {slot_index} duplicates an earlier key")]
    DuplicateKey {
        /// Zero-based duplicate slot position in the source document.
        slot_index: usize,
    },
    /// A slot declared no root kinds.
    #[error("root catalog slot {slot_index} has no allowed kinds")]
    EmptyKindSet {
        /// Zero-based slot position in the source document.
        slot_index: usize,
    },
    /// A slot repeated a root kind.
    #[error("root catalog slot {slot_index} repeats an allowed kind")]
    DuplicateKind {
        /// Zero-based slot position in the source document.
        slot_index: usize,
    },
    /// A path was not absolute UTF-8 within the decoded bound or contained NUL.
    #[error("root catalog slot {slot_index} has an invalid path")]
    InvalidPath {
        /// Zero-based slot position in the source document.
        slot_index: usize,
    },
    /// A durability class and evidence pair was outside the accepted matrix.
    #[error("root catalog slot {slot_index} has an invalid durability mapping")]
    InvalidDurabilityMapping {
        /// Zero-based slot position in the source document.
        slot_index: usize,
    },
    /// A sole-writer class and evidence pair was outside the accepted matrix.
    #[error("root catalog slot {slot_index} has an invalid sole-writer mapping")]
    InvalidSoleWriterMapping {
        /// Zero-based slot position in the source document.
        slot_index: usize,
    },
    /// A combined source/output slot lacked persistent and exclusive declarations.
    #[error("root catalog slot {slot_index} has an invalid source/output declaration")]
    InvalidSourceOutputDeclaration {
        /// Zero-based slot position in the source document.
        slot_index: usize,
    },
}

impl RootCatalogParseError {
    /// Return the stable ADR 550 source-loading reason code.
    #[must_use]
    pub const fn reason_code(&self) -> &'static str {
        match self {
            Self::DocumentBoundExceeded { .. } => "media_root_catalog_bound_exceeded",
            Self::MalformedDocument
            | Self::UnsupportedVersion { .. }
            | Self::SlotBoundExceeded { .. }
            | Self::InvalidKey { .. }
            | Self::DuplicateKey { .. }
            | Self::EmptyKindSet { .. }
            | Self::DuplicateKind { .. }
            | Self::InvalidPath { .. }
            | Self::InvalidDurabilityMapping { .. }
            | Self::InvalidSoleWriterMapping { .. }
            | Self::InvalidSourceOutputDeclaration { .. } => "media_root_catalog_format_invalid",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCatalog {
    format_version: u32,
    slots: Vec<RawSlot>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSlot {
    key: String,
    allowed_kinds: Vec<RawRootKind>,
    path: String,
    durability_class: RawDurabilityClass,
    durability_evidence: RawDurabilityEvidence,
    sole_writer_class: RawSoleWriterClass,
    sole_writer_evidence: RawSoleWriterEvidence,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RawRootKind {
    Source,
    Output,
    Workspace,
    Backup,
    Quarantine,
}

impl From<RawRootKind> for RootKind {
    fn from(value: RawRootKind) -> Self {
        match value {
            RawRootKind::Source => Self::Source,
            RawRootKind::Output => Self::Output,
            RawRootKind::Workspace => Self::Workspace,
            RawRootKind::Backup => Self::Backup,
            RawRootKind::Quarantine => Self::Quarantine,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum RawDurabilityClass {
    Disposable,
    RestartPersistent,
}

impl From<RawDurabilityClass> for DurabilityClass {
    fn from(value: RawDurabilityClass) -> Self {
        match value {
            RawDurabilityClass::Disposable => Self::Disposable,
            RawDurabilityClass::RestartPersistent => Self::RestartPersistent,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum RawDurabilityEvidence {
    None,
    LinuxDedicatedMount,
    KubernetesPersistentVolumeClaim,
}

impl From<RawDurabilityEvidence> for DurabilityEvidence {
    fn from(value: RawDurabilityEvidence) -> Self {
        match value {
            RawDurabilityEvidence::None => Self::None,
            RawDurabilityEvidence::LinuxDedicatedMount => Self::LinuxDedicatedMount,
            RawDurabilityEvidence::KubernetesPersistentVolumeClaim => {
                Self::KubernetesPersistentVolumeClaim
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum RawSoleWriterClass {
    Uncontrolled,
    RevaerExclusive,
}

impl From<RawSoleWriterClass> for SoleWriterClass {
    fn from(value: RawSoleWriterClass) -> Self {
        match value {
            RawSoleWriterClass::Uncontrolled => Self::Uncontrolled,
            RawSoleWriterClass::RevaerExclusive => Self::RevaerExclusive,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum RawSoleWriterEvidence {
    None,
    LinuxDedicatedService,
    KubernetesReadWriteOncePod,
}

impl From<RawSoleWriterEvidence> for SoleWriterEvidence {
    fn from(value: RawSoleWriterEvidence) -> Self {
        match value {
            RawSoleWriterEvidence::None => Self::None,
            RawSoleWriterEvidence::LinuxDedicatedService => Self::LinuxDedicatedService,
            RawSoleWriterEvidence::KubernetesReadWriteOncePod => Self::KubernetesReadWriteOncePod,
        }
    }
}

/// Parse and fully validate one exact version-1 root-catalog document.
///
/// Parsing preserves exact decoded key and path bytes, normalizes only slot and
/// kind ordering for semantic identity, and grants no filesystem readiness.
///
/// # Errors
///
/// Returns [`RootCatalogParseError`] for every malformed value, duplicate,
/// unknown field or enum, invalid class/evidence mapping, or reviewed bound
/// violation.
pub fn parse_root_catalog_v1(document: &[u8]) -> Result<RootCatalog, RootCatalogParseError> {
    if document.len() > MAX_ROOT_CATALOG_DOCUMENT_BYTES {
        return Err(RootCatalogParseError::DocumentBoundExceeded {
            maximum_bytes: MAX_ROOT_CATALOG_DOCUMENT_BYTES,
        });
    }

    let raw: RawCatalog = serde_json::from_slice(document)
        .map_err(|_error| RootCatalogParseError::MalformedDocument)?;
    if raw.format_version != 1 {
        return Err(RootCatalogParseError::UnsupportedVersion {
            actual: raw.format_version,
        });
    }
    if raw.slots.len() > MAX_ROOT_CATALOG_SLOTS {
        return Err(RootCatalogParseError::SlotBoundExceeded {
            maximum_slots: MAX_ROOT_CATALOG_SLOTS,
        });
    }

    let slot_count = u32::try_from(raw.slots.len()).map_err(|_error| {
        RootCatalogParseError::SlotBoundExceeded {
            maximum_slots: MAX_ROOT_CATALOG_SLOTS,
        }
    })?;
    let mut keys = BTreeSet::new();
    let mut slots = Vec::with_capacity(raw.slots.len());
    for (slot_index, slot) in raw.slots.into_iter().enumerate() {
        slots.push(validate_slot(slot, slot_index, &mut keys)?);
    }

    Ok(RootCatalog::from_validated_slots(slots, slot_count))
}

fn validate_slot(
    raw: RawSlot,
    slot_index: usize,
    keys: &mut BTreeSet<String>,
) -> Result<RootCatalogSlot, RootCatalogParseError> {
    let key_byte_len = validate_key(&raw.key, slot_index)?;
    if !keys.insert(raw.key.clone()) {
        return Err(RootCatalogParseError::DuplicateKey { slot_index });
    }
    let allowed_kinds = validate_kinds(&raw.allowed_kinds, slot_index)?;
    let path_byte_len = validate_path(&raw.path, slot_index)?;
    let durability_class = raw.durability_class.into();
    let durability_evidence = raw.durability_evidence.into();
    validate_durability(durability_class, durability_evidence, slot_index)?;
    let sole_writer_class = raw.sole_writer_class.into();
    let sole_writer_evidence = raw.sole_writer_evidence.into();
    validate_sole_writer(sole_writer_class, sole_writer_evidence, slot_index)?;
    validate_source_output(
        &allowed_kinds,
        durability_class,
        sole_writer_class,
        slot_index,
    )?;

    Ok(RootCatalogSlot::validated(ValidatedRootCatalogSlot {
        key: raw.key,
        key_byte_len,
        allowed_kinds,
        path: raw.path,
        path_byte_len,
        durability_class,
        durability_evidence,
        sole_writer_class,
        sole_writer_evidence,
    }))
}

fn validate_key(key: &str, slot_index: usize) -> Result<u32, RootCatalogParseError> {
    let valid = !key.is_empty()
        && key.len() <= MAX_ROOT_CATALOG_KEY_BYTES
        && key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !valid {
        return Err(RootCatalogParseError::InvalidKey { slot_index });
    }
    u32::try_from(key.len()).map_err(|_error| RootCatalogParseError::InvalidKey { slot_index })
}

fn validate_kinds(
    raw_kinds: &[RawRootKind],
    slot_index: usize,
) -> Result<Vec<RootKind>, RootCatalogParseError> {
    if raw_kinds.is_empty() {
        return Err(RootCatalogParseError::EmptyKindSet { slot_index });
    }
    let mut mask = 0_u8;
    for raw_kind in raw_kinds {
        let kind = RootKind::from(*raw_kind);
        if mask & kind.mask() != 0 {
            return Err(RootCatalogParseError::DuplicateKind { slot_index });
        }
        mask |= kind.mask();
    }

    Ok(RootKind::ORDERED
        .into_iter()
        .filter(|kind| mask & kind.mask() != 0)
        .collect())
}

fn validate_path(path: &str, slot_index: usize) -> Result<u32, RootCatalogParseError> {
    let valid = path.starts_with('/')
        && path.len() <= MAX_ROOT_CATALOG_PATH_BYTES
        && !path.as_bytes().contains(&0);
    if !valid {
        return Err(RootCatalogParseError::InvalidPath { slot_index });
    }
    u32::try_from(path.len()).map_err(|_error| RootCatalogParseError::InvalidPath { slot_index })
}

const fn validate_durability(
    class: DurabilityClass,
    evidence: DurabilityEvidence,
    slot_index: usize,
) -> Result<(), RootCatalogParseError> {
    let valid = matches!(
        (class, evidence),
        (DurabilityClass::Disposable, DurabilityEvidence::None)
            | (
                DurabilityClass::RestartPersistent,
                DurabilityEvidence::LinuxDedicatedMount
                    | DurabilityEvidence::KubernetesPersistentVolumeClaim
            )
    );
    if valid {
        Ok(())
    } else {
        Err(RootCatalogParseError::InvalidDurabilityMapping { slot_index })
    }
}

const fn validate_sole_writer(
    class: SoleWriterClass,
    evidence: SoleWriterEvidence,
    slot_index: usize,
) -> Result<(), RootCatalogParseError> {
    let valid = matches!(
        (class, evidence),
        (SoleWriterClass::Uncontrolled, SoleWriterEvidence::None)
            | (
                SoleWriterClass::RevaerExclusive,
                SoleWriterEvidence::LinuxDedicatedService
                    | SoleWriterEvidence::KubernetesReadWriteOncePod
            )
    );
    if valid {
        Ok(())
    } else {
        Err(RootCatalogParseError::InvalidSoleWriterMapping { slot_index })
    }
}

fn validate_source_output(
    kinds: &[RootKind],
    durability: DurabilityClass,
    sole_writer: SoleWriterClass,
    slot_index: usize,
) -> Result<(), RootCatalogParseError> {
    let combines_source_output =
        kinds.contains(&RootKind::Source) && kinds.contains(&RootKind::Output);
    let valid = !combines_source_output
        || (durability == DurabilityClass::RestartPersistent
            && sole_writer == SoleWriterClass::RevaerExclusive);
    if valid {
        Ok(())
    } else {
        Err(RootCatalogParseError::InvalidSourceOutputDeclaration { slot_index })
    }
}
