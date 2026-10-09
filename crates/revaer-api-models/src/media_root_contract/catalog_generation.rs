//! Generation metadata, excluding the non-HTTP aggregate attestation digest.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, de};
use uuid::Uuid;

use super::{RootCatalogError, catalog_scalar, object};

/// Explicit generation inputs; validation occurs in [`RootCatalogGeneration::new`].
/// No omitted metadata is synthesized from legacy values.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct RootCatalogGenerationFields {
    /// Public identity of this generation occurrence.
    pub media_root_catalog_generation_public_id: Uuid,
    /// Canonical positive decimal within `PostgreSQL`'s signed bigint range.
    pub generation: String,
    /// Exactly 64 lowercase hexadecimal source-digest characters.
    pub source_sha256: String,
    /// Exactly 64 lowercase hexadecimal generation-digest characters.
    pub generation_sha256: String,
    /// Complete generation size, zero through 256 (not this page's size).
    pub slot_count: u16,
    /// RFC 3339 UTC first activation timestamp.
    pub activated_at: String,
    /// RFC 3339 UTC reconciliation timestamp.
    pub reconciled_at: String,
}

impl fmt::Debug for RootCatalogGenerationFields {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RootCatalogGenerationFields")
    }
}

/// Immutable, validated version 1 generation metadata with exact wire names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RootCatalogGeneration {
    fields: RootCatalogGenerationFields,
}

impl RootCatalogGeneration {
    /// Validate explicit metadata without establishing generation authority.
    ///
    /// # Errors
    /// Rejects noncanonical or out-of-range decimals, digests, counts or UTC times.
    pub fn new(fields: RootCatalogGenerationFields) -> Result<Self, RootCatalogError> {
        catalog_scalar::decimal(&fields.generation, true)?;
        catalog_scalar::hex(&fields.source_sha256, 64)?;
        catalog_scalar::hex(&fields.generation_sha256, 64)?;
        catalog_scalar::timestamp(&fields.activated_at)?;
        catalog_scalar::timestamp(&fields.reconciled_at)?;
        if fields.slot_count > 256 {
            return Err(RootCatalogError);
        }
        Ok(Self { fields })
    }

    /// Return validated metadata under the exact HTTP column names.
    #[must_use]
    pub const fn fields(&self) -> &RootCatalogGenerationFields {
        &self.fields
    }
}

impl<'de> Deserialize<'de> for RootCatalogGeneration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input: GenerationInput =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootCatalogError))?;
        Self::new(input.0).map_err(de::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct GenerationInput(#[serde(with = "GenerationWire")] RootCatalogGenerationFields);

#[derive(Deserialize)]
#[serde(remote = "RootCatalogGenerationFields", deny_unknown_fields)]
struct GenerationWire {
    media_root_catalog_generation_public_id: Uuid,
    generation: String,
    source_sha256: String,
    generation_sha256: String,
    slot_count: u16,
    activated_at: String,
    reconciled_at: String,
}
