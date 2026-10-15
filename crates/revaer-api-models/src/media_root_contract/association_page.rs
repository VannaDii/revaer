//! Validated, collection-specific association continuation and complete pages.

use serde::{Deserialize, Deserializer, Serialize, de};
use uuid::Uuid;

use super::{DiscoveryAssociationResponse, RootCatalogCursor, RootInputError};

/// Association-only keyset continuation; never mutation authority.
#[derive(Clone, PartialEq, Eq)]
pub struct AssociationCollectionCursor(RootCatalogCursor);

impl AssociationCollectionCursor {
    /// Construct a continuation from the final association key and identity.
    ///
    /// # Errors
    /// Rejects keys outside the catalog logical-key grammar.
    pub fn new(key: &str, id: Uuid) -> Result<Self, RootInputError> {
        RootCatalogCursor::new(key, id).map(Self)
    }

    /// Decode an exact, canonical association-only token.
    ///
    /// # Errors
    /// Rejects foreign collections, malformed keys and noncanonical encoding.
    pub fn decode(token: &str) -> Result<Self, RootInputError> {
        RootCatalogCursor::decode(token.strip_prefix("associations_").ok_or(RootInputError)?)
            .map(Self)
    }

    /// Encode this collection's exact canonical continuation.
    ///
    /// # Errors
    /// Propagates invalid private encoding invariants.
    pub fn encode(&self) -> Result<String, RootInputError> {
        Ok(format!("associations_{}", self.0.encode()?))
    }

    /// Return the association's immutable logical key.
    #[must_use]
    pub fn association_key(&self) -> &str {
        self.0.logical_key()
    }

    /// Return the same association's public identity.
    #[must_use]
    pub const fn public_id(&self) -> Uuid {
        self.0.slot_public_id()
    }
}

/// Complete bounded association page in strict immutable-key order.
#[derive(Serialize)]
pub struct DiscoveryAssociationPageResponse {
    associations: Vec<DiscoveryAssociationResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

impl DiscoveryAssociationPageResponse {
    /// Validate page size, strict ordering and final-row continuation.
    ///
    /// # Errors
    /// Rejects oversized/unordered pages and incoherent or foreign cursors.
    pub fn new(
        associations: Vec<DiscoveryAssociationResponse>,
        next_cursor: Option<String>,
    ) -> Result<Self, RootInputError> {
        if associations.len() > 200
            || associations.windows(2).any(|pair| {
                let left = pair[0].fields();
                let right = pair[1].fields();
                (
                    left.request.association_key(),
                    left.media_discovery_association_public_id,
                ) >= (
                    right.request.association_key(),
                    right.media_discovery_association_public_id,
                )
            })
        {
            return Err(RootInputError);
        }
        if let Some(token) = next_cursor.as_deref() {
            let cursor = AssociationCollectionCursor::decode(token)?;
            let last = associations.last().ok_or(RootInputError)?.fields();
            if cursor.association_key() != last.request.association_key()
                || cursor.public_id() != last.media_discovery_association_public_id
            {
                return Err(RootInputError);
            }
        }
        Ok(Self {
            associations,
            next_cursor,
        })
    }

    /// Consume validated transport into feature-owned display state.
    #[must_use]
    pub fn into_parts(self) -> (Vec<DiscoveryAssociationResponse>, Option<String>) {
        (self.associations, self.next_cursor)
    }
}

impl<'de> Deserialize<'de> for DiscoveryAssociationPageResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            associations: Vec<DiscoveryAssociationResponse>,
            next_cursor: Option<String>,
        }
        let wire: Wire = super::object::deserialize(deserializer)
            .map_err(|_| de::Error::custom(RootInputError))?;
        Self::new(wire.associations, wire.next_cursor).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn association_cursor_is_canonical_and_collection_specific() -> Result<(), RootInputError> {
        let cursor = AssociationCollectionCursor::new("movies", Uuid::from_u128(1))?;
        let token = cursor.encode()?;
        let decoded = AssociationCollectionCursor::decode(&token)?;
        assert_eq!(decoded.association_key(), "movies");
        assert_eq!(decoded.public_id(), cursor.public_id());
        for invalid in [
            format!("{token}="),
            format!("{token}x"),
            "associations_".into(),
            super::super::ProfileCollectionCursor::new("movies", cursor.public_id())?.encode()?,
        ] {
            assert!(AssociationCollectionCursor::decode(&invalid).is_err());
        }
        assert!(DiscoveryAssociationPageResponse::new(Vec::new(), Some(token)).is_err());
        assert!(DiscoveryAssociationPageResponse::new(Vec::new(), None).is_ok());
        Ok(())
    }
}
