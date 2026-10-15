//! Reported per-kind readiness; no capability probing or authority calculation.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, de};

use super::{RootCatalogError, RootKind, object};

/// One reported allowed kind. Slot construction revalidates these public fields.
/// Ready values require absent reasons; false values require closed reason codes.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct RootCatalogAllowedKind {
    /// Root role in the fixed source-to-quarantine order.
    pub kind: RootKind,
    /// Reported binding eligibility, not authorization to use the root.
    pub binding_ready: bool,
    /// Bounded reason for unavailable binding; omitted when ready.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_reason: Option<String>,
    /// Reported destructive eligibility, a subset of binding readiness.
    pub destructive_ready: bool,
    /// Bounded reason for unavailable destructive use; omitted when ready.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_reason: Option<String>,
}

impl RootCatalogAllowedKind {
    pub(super) fn validate(&self) -> Result<(), RootCatalogError> {
        if self.destructive_ready && !self.binding_ready {
            return Err(RootCatalogError);
        }
        validate_reason(self.binding_ready, self.binding_reason.as_deref())?;
        validate_reason(self.destructive_ready, self.destructive_reason.as_deref())
    }
}

impl fmt::Debug for RootCatalogAllowedKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RootCatalogAllowedKind")
    }
}

fn validate_reason(ready: bool, reason: Option<&str>) -> Result<(), RootCatalogError> {
    match (ready, reason) {
        (true, None)
        | (
            false,
            Some(
                "media_configuration_root_unmapped"
                | "media_root_slot_unknown"
                | "media_root_kind_forbidden"
                | "media_root_binding_incomplete"
                | "media_root_attestation_stale"
                | "media_root_attestation_invalid"
                | "media_root_overlap"
                | "media_root_unsafe_ancestry"
                | "media_root_durability_unproven"
                | "media_root_writer_control_unproven"
                | "media_root_identity_mismatch",
            ),
        ) => Ok(()),
        _ => Err(RootCatalogError),
    }
}

pub(super) const fn ordinal(kind: RootKind) -> u8 {
    match kind {
        RootKind::Source => 1,
        RootKind::Output => 2,
        RootKind::Workspace => 3,
        RootKind::Backup => 4,
        RootKind::Quarantine => 5,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AllowedKindWire {
    kind: RootKind,
    binding_ready: bool,
    binding_reason: Option<String>,
    destructive_ready: bool,
    destructive_reason: Option<String>,
}

impl<'de> Deserialize<'de> for RootCatalogAllowedKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire: AllowedKindWire =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootCatalogError))?;
        let row = Self {
            kind: wire.kind,
            binding_ready: wire.binding_ready,
            binding_reason: wire.binding_reason,
            destructive_ready: wire.destructive_ready,
            destructive_reason: wire.destructive_reason,
        };
        row.validate().map_err(de::Error::custom)?;
        Ok(row)
    }
}
