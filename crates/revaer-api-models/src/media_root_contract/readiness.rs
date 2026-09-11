//! The accepted readiness wire shape, without root paths or authority.

use std::{error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeStruct};

mod object;
mod state;

pub use state::{RootAttestationFailure, RootCatalogReadinessState, RootSourceFailure};

/// A root role in ADR 557's fixed source-to-quarantine order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootKind {
    /// Descriptor-bound input media.
    Source,
    /// Verified replacement output.
    Output,
    /// Attempt-owned temporary work.
    Workspace,
    /// Retained original media when required by policy.
    Backup,
    /// Isolated failed output when required by policy.
    Quarantine,
}

impl<'de> Deserialize<'de> for RootKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer
            .deserialize_str(RootKindVisitor)
            .map_err(|_| de::Error::custom(RootReadinessError))
    }
}

struct RootKindVisitor;

impl de::Visitor<'_> for RootKindVisitor {
    type Value = RootKind;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a root-kind string")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<RootKind, E> {
        match value {
            "source" => Ok(RootKind::Source),
            "output" => Ok(RootKind::Output),
            "workspace" => Ok(RootKind::Workspace),
            "backup" => Ok(RootKind::Backup),
            "quarantine" => Ok(RootKind::Quarantine),
            _ => Err(E::custom(RootReadinessError)),
        }
    }
}

const ROOT_KIND_ORDER: [RootKind; 5] = [
    RootKind::Source,
    RootKind::Output,
    RootKind::Workspace,
    RootKind::Backup,
    RootKind::Quarantine,
];

/// Counts returned for one root kind, never individual slot identities.
///
/// [`RootCatalogReadinessResponse::new`] validates bounds, count relationships
/// and ordering before this value can become part of a readiness response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RootKindReadiness {
    /// The root kind whose normalized allowed-kind rows were counted.
    pub kind: RootKind,
    /// Slots attested in the current catalog generation, at most 256.
    pub attested_slot_count: u16,
    /// Attested slots with the capabilities and writer evidence for this kind.
    pub binding_ready_slot_count: u16,
    /// Binding-ready slots also satisfying destructive-use requirements.
    pub destructive_ready_slot_count: u16,
}

impl RootKindReadiness {
    fn validate(self, expected: RootKind, has_generation: bool) -> Result<(), RootReadinessError> {
        if self.kind != expected
            || self.attested_slot_count > 256
            || self.binding_ready_slot_count > self.attested_slot_count
            || self.destructive_ready_slot_count > self.binding_ready_slot_count
            || (!has_generation && self.attested_slot_count != 0)
        {
            return Err(RootReadinessError);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KindReadinessWire {
    kind: RootKind,
    attested_slot_count: u16,
    binding_ready_slot_count: u16,
    destructive_ready_slot_count: u16,
}

impl<'de> Deserialize<'de> for RootKindReadiness {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire: KindReadinessWire =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootReadinessError))?;
        let row = Self {
            kind: wire.kind,
            attested_slot_count: wire.attested_slot_count,
            binding_ready_slot_count: wire.binding_ready_slot_count,
            destructive_ready_slot_count: wire.destructive_ready_slot_count,
        };
        row.validate(row.kind, true).map_err(de::Error::custom)?;
        Ok(row)
    }
}

/// Malformed readiness evidence, with no caller values in its diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootReadinessError;

impl fmt::Display for RootReadinessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("media root readiness is invalid")
    }
}

impl Error for RootReadinessError {}

/// Validated, path-free version 1 root-catalog readiness representation.
///
/// Construction and JSON decoding enforce the accepted source/attestation
/// state combinations, canonical positive generation string, and exactly five
/// ordered, bounded count rows. Unknown fields are rejected at both levels.
/// Optional fields are omitted on serialization; JSON null means absence only
/// where the corresponding coherent state permits absence.
///
/// This type validates representation, not filesystem or database evidence.
/// It does not install a route, establish authentication, grant write authority,
/// or claim that the reported probes actually ran. A provider must obtain those
/// results from ADR 557's consistent stored-procedure snapshot.
///
/// ```
/// use revaer_api_models::media_root_contract::{
///     RootCatalogReadinessResponse, RootCatalogReadinessState, RootKind,
///     RootKindReadiness, RootSourceFailure,
/// };
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let kinds = [RootKind::Source, RootKind::Output, RootKind::Workspace,
///     RootKind::Backup, RootKind::Quarantine].map(|kind| RootKindReadiness {
///     kind, attested_slot_count: 0, binding_ready_slot_count: 0,
///     destructive_ready_slot_count: 0,
/// });
/// let response = RootCatalogReadinessResponse::new(
///     RootCatalogReadinessState::SourceUnavailable(RootSourceFailure::Missing),
///     kinds,
/// )?;
/// assert_eq!(response.kinds().len(), 5);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootCatalogReadinessResponse {
    state: RootCatalogReadinessState,
    kinds: [RootKindReadiness; 5],
}

impl RootCatalogReadinessResponse {
    /// Validate a complete source/attestation state and ordered count snapshot.
    ///
    /// # Errors
    /// Rejects generations above `PostgreSQL`'s positive `bigint` range, rows not
    /// in the five-kind order, counts above 256, destructive counts exceeding
    /// binding counts, binding counts exceeding attested counts, and nonzero
    /// counts when no active generation exists.
    pub fn new(
        state: RootCatalogReadinessState,
        kinds: [RootKindReadiness; 5],
    ) -> Result<Self, RootReadinessError> {
        state.validate()?;
        let has_generation = matches!(state, RootCatalogReadinessState::Ready { .. });
        for (row, expected) in kinds.iter().zip(ROOT_KIND_ORDER) {
            row.validate(expected, has_generation)?;
        }
        Ok(Self { state, kinds })
    }

    /// Return the coherent reported state, without interpreting it as authority.
    #[must_use]
    pub const fn state(&self) -> RootCatalogReadinessState {
        self.state
    }

    /// Return all five rows in source, output, workspace, backup, quarantine order.
    #[must_use]
    pub const fn kinds(&self) -> &[RootKindReadiness; 5] {
        &self.kinds
    }
}

impl Serialize for RootCatalogReadinessResponse {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // State owns mutually exclusive reason/generation fields; counts carry
        // no slot identifiers and are immutable after validation.
        let mut object = serializer.serialize_struct("RootCatalogReadinessResponse", 5)?;
        object.serialize_field("format_version", &1_u8)?;
        match self.state {
            RootCatalogReadinessState::SourceUnavailable(reason) => {
                object.serialize_field("source_state", reason.source_state())?;
                object.serialize_field("source_reason", reason.reason_code())?;
                object.serialize_field("attestation_state", "not_evaluated")?;
            }
            RootCatalogReadinessState::AttestationInvalid(reason) => {
                object.serialize_field("source_state", "ready")?;
                object.serialize_field("attestation_state", "invalid")?;
                object.serialize_field("attestation_reason", reason.reason_code())?;
            }
            RootCatalogReadinessState::Ready { generation } => {
                object.serialize_field("source_state", "ready")?;
                object.serialize_field("attestation_state", "ready")?;
                object.serialize_field("generation", &generation.to_string())?;
            }
        }
        object.serialize_field("kinds", &self.kinds)?;
        object.end()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessWire {
    format_version: u8,
    source_state: String,
    source_reason: Option<String>,
    attestation_state: String,
    attestation_reason: Option<String>,
    generation: Option<String>,
    kinds: [RootKindReadiness; 5],
}

impl<'de> Deserialize<'de> for RootCatalogReadinessResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire: ReadinessWire =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootReadinessError))?;
        if wire.format_version != 1 {
            return Err(de::Error::custom(RootReadinessError));
        }
        let state = RootCatalogReadinessState::from_wire(
            &wire.source_state,
            wire.source_reason.as_deref(),
            &wire.attestation_state,
            wire.attestation_reason.as_deref(),
            wire.generation.as_deref(),
        )
        .map_err(de::Error::custom)?;
        Self::new(state, wire.kinds).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod constructor_tests;

#[cfg(test)]
mod decoder_tests;
