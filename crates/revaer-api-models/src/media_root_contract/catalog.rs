//! Complete administrative catalog page, not filesystem or readiness authority.

use std::{collections::HashSet, error::Error, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeStruct};

use super::{
    RootCatalogCursor, RootCatalogGeneration, RootCatalogReadinessState, RootCatalogSlot,
    catalog_scalar, catalog_state, object,
};

/// Invalid catalog representation, without rejected input or filesystem details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootCatalogError;

impl fmt::Display for RootCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("media root catalog is invalid")
    }
}

impl Error for RootCatalogError {}

/// Validated version 1 `GET /v1/media/root-catalog` page.
///
/// Only this administrative response includes root paths and filesystem identity.
/// Consumers must enforce authentication, no-store handling and path-free logs.
/// Construction and decoding validate scalars, state coherence, page order and
/// uniqueness. They do not prove filesystem identity, database snapshot consistency,
/// cursor membership, authentication or readiness, and do not enable legacy roots.
/// Optional fields serialize as omitted, never as invented ready defaults.
///
/// ```
/// use revaer_api_models::media_root_contract::{
///     RootCatalogPageResponse, RootCatalogReadinessState, RootSourceFailure,
/// };
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let page = RootCatalogPageResponse::new(
///     RootCatalogReadinessState::SourceUnavailable(RootSourceFailure::Missing),
///     None, Vec::new(), None,
/// )?;
/// assert!(page.generation().is_none());
/// assert!(page.slots().is_empty());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootCatalogPageResponse {
    state: RootCatalogReadinessState,
    generation: Option<RootCatalogGeneration>,
    slots: Vec<RootCatalogSlot>,
    next_cursor: Option<String>,
}

impl RootCatalogPageResponse {
    /// Validate a complete page with a canonical encoded continuation cursor.
    ///
    /// # Errors
    /// Rejects state/metadata disagreement, more than 200 rows, rows exceeding
    /// generation size, duplicate keys/identities/paths/digests, unordered rows,
    /// or a cursor that does not exactly identify this nonfinal page's last row.
    pub fn new(
        state: RootCatalogReadinessState,
        generation: Option<RootCatalogGeneration>,
        slots: Vec<RootCatalogSlot>,
        next_cursor: Option<String>,
    ) -> Result<Self, RootCatalogError> {
        let slot_count = match (state, generation.as_ref()) {
            (
                RootCatalogReadinessState::Ready {
                    generation: expected,
                },
                Some(metadata),
            ) => {
                if catalog_scalar::decimal(&metadata.fields().generation, true)? != expected.get() {
                    return Err(RootCatalogError);
                }
                usize::from(metadata.fields().slot_count)
            }
            (
                RootCatalogReadinessState::SourceUnavailable(_)
                | RootCatalogReadinessState::AttestationInvalid(_),
                None,
            ) => 0,
            _ => return Err(RootCatalogError),
        };
        if slots.len() > 200 || slots.len() > slot_count {
            return Err(RootCatalogError);
        }
        validate_slots(&slots)?;
        if let Some(token) = next_cursor.as_deref() {
            let cursor = RootCatalogCursor::decode(token).map_err(|_| RootCatalogError)?;
            let last = slots.last().ok_or(RootCatalogError)?;
            if slots.len() >= slot_count
                || cursor.logical_key() != last.logical_key()
                || cursor.slot_public_id() != last.fields().media_root_catalog_slot_public_id
            {
                return Err(RootCatalogError);
            }
        }
        Ok(Self {
            state,
            generation,
            slots,
            next_cursor,
        })
    }

    /// Return the coherent reported source/attestation state.
    #[must_use]
    pub const fn state(&self) -> RootCatalogReadinessState {
        self.state
    }

    /// Return active generation metadata, absent when either stage is unavailable.
    #[must_use]
    pub const fn generation(&self) -> Option<&RootCatalogGeneration> {
        self.generation.as_ref()
    }

    /// Return this page's slots in logical-key/public-identity order.
    #[must_use]
    pub fn slots(&self) -> &[RootCatalogSlot] {
        &self.slots
    }

    /// Return the exact validated encoded cursor, absent on a final or empty page.
    #[must_use]
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}

fn validate_slots(slots: &[RootCatalogSlot]) -> Result<(), RootCatalogError> {
    let mut ids = HashSet::new();
    let mut paths = HashSet::new();
    let mut identities = HashSet::new();
    let mut digests = HashSet::new();
    for slot in slots {
        let fields = slot.fields();
        if !ids.insert(fields.media_root_catalog_slot_public_id)
            || !paths.insert(&fields.canonical_path)
            || !identities.insert((&fields.filesystem_device, &fields.filesystem_inode))
            || !digests.insert(&fields.root_identity_sha256)
        {
            return Err(RootCatalogError);
        }
    }
    // Keys are globally unique, so strict key order also satisfies tuple order.
    if slots
        .windows(2)
        .any(|pair| pair[0].logical_key() >= pair[1].logical_key())
    {
        return Err(RootCatalogError);
    }
    Ok(())
}

impl Serialize for RootCatalogPageResponse {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let length = 4
            + usize::from(self.generation.is_some())
            + usize::from(self.next_cursor.is_some())
            + usize::from(!matches!(
                self.state,
                RootCatalogReadinessState::Ready { .. }
            ));
        let mut object = serializer.serialize_struct("RootCatalogPageResponse", length)?;
        object.serialize_field("format_version", &1_u8)?;
        match self.state {
            RootCatalogReadinessState::SourceUnavailable(reason) => {
                object.serialize_field("source_state", catalog_state::source_name(reason))?;
                object.serialize_field("source_reason", reason.reason_code())?;
                object.serialize_field("attestation_state", "not_evaluated")?;
            }
            RootCatalogReadinessState::AttestationInvalid(reason) => {
                object.serialize_field("source_state", "ready")?;
                object.serialize_field("attestation_state", "invalid")?;
                object.serialize_field("attestation_reason", reason.reason_code())?;
            }
            RootCatalogReadinessState::Ready { .. } => {
                object.serialize_field("source_state", "ready")?;
                object.serialize_field("attestation_state", "ready")?;
            }
        }
        if let Some(generation) = &self.generation {
            object.serialize_field("generation", generation)?;
        }
        object.serialize_field("slots", &self.slots)?;
        if let Some(cursor) = &self.next_cursor {
            object.serialize_field("next_cursor", cursor)?;
        }
        object.end()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageWire {
    format_version: u8,
    source_state: String,
    source_reason: Option<String>,
    attestation_state: String,
    attestation_reason: Option<String>,
    generation: Option<RootCatalogGeneration>,
    slots: Vec<RootCatalogSlot>,
    next_cursor: Option<String>,
}

impl<'de> Deserialize<'de> for RootCatalogPageResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire: PageWire =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootCatalogError))?;
        if wire.format_version != 1 {
            return Err(de::Error::custom(RootCatalogError));
        }
        let generation = wire
            .generation
            .as_ref()
            .map(|metadata| catalog_scalar::decimal(&metadata.fields().generation, true))
            .transpose()
            .map_err(de::Error::custom)?;
        let state = catalog_state::decode(
            &wire.source_state,
            wire.source_reason.as_deref(),
            &wire.attestation_state,
            wire.attestation_reason.as_deref(),
            generation,
        )
        .map_err(de::Error::custom)?;
        Self::new(state, wire.generation, wire.slots, wire.next_cursor).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
