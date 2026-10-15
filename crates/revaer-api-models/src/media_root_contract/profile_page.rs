//! Complete profile pages with collection-specific, canonical continuation keys.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, de};
use uuid::Uuid;

use super::{ProfileVersionResponse, RootCatalogCursor, RootInputError};

const PREFIX: &str = "profiles_";

/// Opaque profile-only keyset cursor; it conveys no mutation authority.
#[derive(Clone, PartialEq, Eq)]
pub struct ProfileCollectionCursor(RootCatalogCursor);

impl ProfileCollectionCursor {
    /// Construct the continuation for a page's final normalized key and identity.
    ///
    /// # Errors
    /// Rejects keys outside the exact profile-key grammar.
    pub fn new(key: &str, id: Uuid) -> Result<Self, RootInputError> {
        RootCatalogCursor::new(key, id).map(Self)
    }

    /// Decode a bounded canonical URL-safe token for this collection only.
    ///
    /// # Errors
    /// Rejects other collections, malformed keys, padding and trailing bytes.
    pub fn decode(token: &str) -> Result<Self, RootInputError> {
        RootCatalogCursor::decode(token.strip_prefix(PREFIX).ok_or(RootInputError)?).map(Self)
    }

    /// Encode the exact canonical profile collection token.
    ///
    /// # Errors
    /// Propagates an invalid private encoding invariant.
    pub fn encode(&self) -> Result<String, RootInputError> {
        Ok(format!("{PREFIX}{}", self.0.encode()?))
    }

    /// Return the exact key for database membership and ordering checks.
    #[must_use]
    pub fn profile_key(&self) -> &str {
        self.0.logical_key()
    }

    /// Return the stable public identity for the same membership check.
    #[must_use]
    pub const fn profile_public_id(&self) -> Uuid {
        self.0.slot_public_id()
    }
}

impl fmt::Debug for ProfileCollectionCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProfileCollectionCursor")
    }
}

/// Complete validated path-free profile collection response.
#[derive(Debug, Serialize)]
pub struct ProfileVersionPageResponse {
    profiles: Vec<ProfileVersionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

impl ProfileVersionPageResponse {
    /// Validate bounds, strict key ordering and the exact final-row continuation.
    ///
    /// # Errors
    /// Rejects oversized, duplicate/unordered pages or an incoherent cursor.
    pub fn new(
        profiles: Vec<ProfileVersionResponse>,
        next_cursor: Option<String>,
    ) -> Result<Self, RootInputError> {
        if profiles.len() > 200
            || profiles.windows(2).any(|pair| {
                let left = pair[0].fields();
                let right = pair[1].fields();
                (
                    &left.profile.fields().profile_key,
                    left.media_profile_public_id,
                ) >= (
                    &right.profile.fields().profile_key,
                    right.media_profile_public_id,
                )
            })
        {
            return Err(RootInputError);
        }
        if let Some(token) = next_cursor.as_deref() {
            let cursor = ProfileCollectionCursor::decode(token)?;
            let last = profiles.last().ok_or(RootInputError)?.fields();
            if cursor.profile_key() != last.profile.fields().profile_key
                || cursor.profile_public_id() != last.media_profile_public_id
            {
                return Err(RootInputError);
            }
        }
        Ok(Self {
            profiles,
            next_cursor,
        })
    }

    /// Consume the validated page for conversion into feature-owned state.
    #[must_use]
    pub fn into_parts(self) -> (Vec<ProfileVersionResponse>, Option<String>) {
        (self.profiles, self.next_cursor)
    }
}

impl<'de> Deserialize<'de> for ProfileVersionPageResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire: ProfilePageWire = super::object::deserialize(deserializer)
            .map_err(|_| de::Error::custom(RootInputError))?;
        Self::new(wire.profiles, wire.next_cursor).map_err(de::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfilePageWire {
    profiles: Vec<ProfileVersionResponse>,
    next_cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trip_is_canonical_and_collection_specific() -> Result<(), RootInputError> {
        let cursor = ProfileCollectionCursor::new("movies", Uuid::nil())?;
        let token = cursor.encode()?;
        assert_eq!(ProfileCollectionCursor::decode(&token)?, cursor);
        assert!(RootCatalogCursor::decode(&token).is_err());
        assert!(ProfileCollectionCursor::decode(&cursor.0.encode()?).is_err());
        for invalid in [format!("{token}="), format!("{token}x"), "profiles_".into()] {
            assert!(ProfileCollectionCursor::decode(&invalid).is_err());
        }
        assert!(ProfileVersionPageResponse::new(Vec::new(), Some(token)).is_err());
        assert!(ProfileVersionPageResponse::new(Vec::new(), None).is_ok());
        Ok(())
    }
}
