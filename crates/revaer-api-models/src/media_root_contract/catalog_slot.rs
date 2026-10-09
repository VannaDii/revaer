//! Immutable catalog slot transport evidence, with path-free diagnostics.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, de};
use uuid::Uuid;

use super::{
    RootCatalogAllowedKind, RootCatalogCapability, RootCatalogError, RootKind, catalog_kind,
    catalog_scalar, object, validate_root_logical_key,
};

/// Explicit slot inputs, validated by [`RootCatalogSlot::new`].
/// All capabilities and evidence must be supplied; there are no ready defaults.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct RootCatalogSlotFields {
    /// Stable public slot identity.
    pub media_root_catalog_slot_public_id: Uuid,
    /// Exact lowercase logical key, one through 64 bytes.
    pub logical_key: String,
    /// Exact requested absolute, non-root, NUL-free path, at most 4096 bytes.
    pub requested_path: String,
    /// Reported canonical path, with the same scalar bounds as requested path.
    pub canonical_path: String,
    /// Device identity as exactly 16 lowercase hexadecimal characters.
    pub filesystem_device: String,
    /// Inode identity as exactly 16 lowercase hexadecimal characters.
    pub filesystem_inode: String,
    /// Canonical nonnegative decimal within `PostgreSQL`'s signed bigint range.
    pub mount_id: String,
    /// Filesystem type, one through 64 UTF-8 bytes.
    pub filesystem_type: String,
    /// Exact reported read probe result.
    pub read_capable: RootCatalogCapability,
    /// Exact reported write probe result.
    pub write_capable: RootCatalogCapability,
    /// Exact reported exclusive-create probe result.
    pub create_new_capable: RootCatalogCapability,
    /// Exact reported synchronization probe result.
    pub fsync_capable: RootCatalogCapability,
    /// Exact reported rename probe result.
    pub rename_capable: RootCatalogCapability,
    /// Exact reported delete probe result.
    pub delete_capable: RootCatalogCapability,
    /// Exact reported capacity probe result.
    pub capacity_probe_capable: RootCatalogCapability,
    /// Closed durability class, checked together with its evidence.
    pub durability_class: String,
    /// Closed durability evidence, checked together with its class.
    pub durability_evidence: String,
    /// Closed writer class, checked together with its evidence.
    pub sole_writer_class: String,
    /// Closed writer evidence, checked together with its class.
    pub sole_writer_evidence: String,
    /// Unsigned 32-bit owner identity, serialized as a JSON integer.
    pub owner_uid: u32,
    /// Unsigned 32-bit group identity, serialized as a JSON integer.
    pub owner_gid: u32,
    /// Exactly four octal digits, including special permission bits.
    pub mode_bits: String,
    /// RFC 3339 UTC validation timestamp.
    pub validated_at: String,
    /// Exactly 64 lowercase hexadecimal attestation-identity characters.
    pub root_identity_sha256: String,
    /// One through five distinct allowed kinds in root-kind ordinal order.
    pub allowed_kinds: Vec<RootCatalogAllowedKind>,
}

impl fmt::Debug for RootCatalogSlotFields {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RootCatalogSlotFields")
    }
}

/// Validated, immutable slot evidence. Debug output never contains paths.
/// Validation checks representation and internal consistency, not proof truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RootCatalogSlot {
    fields: RootCatalogSlotFields,
}

impl RootCatalogSlot {
    /// Validate explicit scalars, closed evidence pairs and ordered kind rows.
    ///
    /// # Errors
    /// Rejects malformed scalars, unknown/mismatched evidence, duplicate or
    /// unordered kinds, or readiness claims contradicted by supplied evidence.
    /// It does not resolve paths, access storage, or calculate readiness authority.
    pub fn new(fields: RootCatalogSlotFields) -> Result<Self, RootCatalogError> {
        validate_root_logical_key(&fields.logical_key).map_err(|_| RootCatalogError)?;
        catalog_scalar::path(&fields.requested_path)?;
        catalog_scalar::path(&fields.canonical_path)?;
        catalog_scalar::hex(&fields.filesystem_device, 16)?;
        catalog_scalar::hex(&fields.filesystem_inode, 16)?;
        catalog_scalar::decimal(&fields.mount_id, false)?;
        catalog_scalar::hex(&fields.root_identity_sha256, 64)?;
        catalog_scalar::timestamp(&fields.validated_at)?;
        if fields.filesystem_type.is_empty()
            || fields.filesystem_type.len() > 64
            || fields.filesystem_type.contains('\0')
            || fields.mode_bits.len() != 4
            || !fields
                .mode_bits
                .bytes()
                .all(|byte| (b'0'..=b'7').contains(&byte))
        {
            return Err(RootCatalogError);
        }
        validate_evidence(&fields)?;
        validate_kinds(&fields)?;
        Ok(Self { fields })
    }

    /// Return every immutable validated field using its HTTP column name.
    #[must_use]
    pub const fn fields(&self) -> &RootCatalogSlotFields {
        &self.fields
    }

    /// Return the exact logical key without normalization.
    #[must_use]
    pub fn logical_key(&self) -> &str {
        &self.fields.logical_key
    }

    /// Return the requested path for authenticated, nonpersistent administration.
    #[must_use]
    pub fn requested_path(&self) -> &str {
        &self.fields.requested_path
    }

    /// Return reported canonical evidence, not a filesystem access capability.
    #[must_use]
    pub fn canonical_path(&self) -> &str {
        &self.fields.canonical_path
    }

    /// Return reported readiness in the fixed root-kind order.
    #[must_use]
    pub fn allowed_kinds(&self) -> &[RootCatalogAllowedKind] {
        &self.fields.allowed_kinds
    }

    /// Return the reported read probe result.
    #[must_use]
    pub const fn read_capable(&self) -> bool {
        self.fields.read_capable.get()
    }

    /// Return the reported write probe result.
    #[must_use]
    pub const fn write_capable(&self) -> bool {
        self.fields.write_capable.get()
    }

    /// Return the reported exclusive-create probe result.
    #[must_use]
    pub const fn create_new_capable(&self) -> bool {
        self.fields.create_new_capable.get()
    }

    /// Return the reported synchronization probe result.
    #[must_use]
    pub const fn fsync_capable(&self) -> bool {
        self.fields.fsync_capable.get()
    }

    /// Return the reported rename probe result.
    #[must_use]
    pub const fn rename_capable(&self) -> bool {
        self.fields.rename_capable.get()
    }

    /// Return the reported delete probe result.
    #[must_use]
    pub const fn delete_capable(&self) -> bool {
        self.fields.delete_capable.get()
    }

    /// Return the reported capacity probe result.
    #[must_use]
    pub const fn capacity_probe_capable(&self) -> bool {
        self.fields.capacity_probe_capable.get()
    }
}

fn validate_evidence(fields: &RootCatalogSlotFields) -> Result<(), RootCatalogError> {
    let durability = matches!(
        (
            fields.durability_class.as_str(),
            fields.durability_evidence.as_str()
        ),
        ("disposable", "none")
            | (
                "restart_persistent",
                "linux_dedicated_mount" | "kubernetes_persistent_volume_claim"
            )
    );
    let writer = matches!(
        (
            fields.sole_writer_class.as_str(),
            fields.sole_writer_evidence.as_str()
        ),
        ("uncontrolled", "none")
            | (
                "revaer_exclusive",
                "linux_dedicated_service" | "kubernetes_read_write_once_pod"
            )
    );
    if !durability || !writer {
        return Err(RootCatalogError);
    }
    Ok(())
}

fn validate_kinds(fields: &RootCatalogSlotFields) -> Result<(), RootCatalogError> {
    if !(1..=5).contains(&fields.allowed_kinds.len())
        || fields
            .allowed_kinds
            .windows(2)
            .any(|pair| catalog_kind::ordinal(pair[0].kind) >= catalog_kind::ordinal(pair[1].kind))
    {
        return Err(RootCatalogError);
    }
    let write_evidence = fields.write_capable.get()
        && fields.create_new_capable.get()
        && fields.fsync_capable.get()
        && fields.rename_capable.get()
        && fields.delete_capable.get()
        && fields.capacity_probe_capable.get()
        && fields.sole_writer_class == "revaer_exclusive";
    let has_output = fields
        .allowed_kinds
        .iter()
        .any(|row| row.kind == RootKind::Output);
    for row in &fields.allowed_kinds {
        row.validate()?;
        let binding_evidence = if row.kind == RootKind::Source {
            fields.read_capable.get()
        } else {
            write_evidence
        };
        if (row.binding_ready && !binding_evidence)
            || (row.destructive_ready
                && (fields.durability_class != "restart_persistent"
                    || (row.kind == RootKind::Source && !(has_output && write_evidence))))
        {
            return Err(RootCatalogError);
        }
    }
    Ok(())
}

impl<'de> Deserialize<'de> for RootCatalogSlot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input: SlotInput =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootCatalogError))?;
        Self::new(input.0).map_err(de::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct SlotInput(#[serde(with = "SlotWire")] RootCatalogSlotFields);

#[derive(Deserialize)]
#[serde(remote = "RootCatalogSlotFields", deny_unknown_fields)]
struct SlotWire {
    media_root_catalog_slot_public_id: Uuid,
    logical_key: String,
    requested_path: String,
    canonical_path: String,
    filesystem_device: String,
    filesystem_inode: String,
    mount_id: String,
    filesystem_type: String,
    read_capable: RootCatalogCapability,
    write_capable: RootCatalogCapability,
    create_new_capable: RootCatalogCapability,
    fsync_capable: RootCatalogCapability,
    rename_capable: RootCatalogCapability,
    delete_capable: RootCatalogCapability,
    capacity_probe_capable: RootCatalogCapability,
    durability_class: String,
    durability_evidence: String,
    sole_writer_class: String,
    sole_writer_evidence: String,
    owner_uid: u32,
    owner_gid: u32,
    mode_bits: String,
    validated_at: String,
    root_identity_sha256: String,
    allowed_kinds: Vec<RootCatalogAllowedKind>,
}
