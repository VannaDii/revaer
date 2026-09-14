//! Exact logical association inputs, without filesystem or admission authority.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeStruct};
use uuid::Uuid;

use super::{RootInputError, object, validate_root_association_prefix, validate_root_logical_key};

/// Explicitly requested discovery modes; none is inferred or silently enabled.
///
/// These flags are intent, not evidence of readiness or permission to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryModes {
    /// Permit explicitly requested discovery for this association.
    pub manual_enabled: bool,
    /// Request the approved, readiness-gated filesystem watcher workflow.
    pub watcher_enabled: bool,
    /// Request the approved, readiness-gated scheduled discovery workflow.
    pub schedule_enabled: bool,
}

/// Validated create/replace body for ADR 557's discovery-association contract.
///
/// All eight JSON fields are required, including the three booleans and the
/// relative prefix. An explicitly empty prefix selects the whole attested root;
/// missing or null is never converted to empty. Unknown and duplicate fields,
/// positional arrays, raw root paths and nonpositive profile versions fail.
/// Logical keys and relative path bytes are preserved without normalization.
///
/// The immutable body carries requested configuration only. Active profile
/// membership, catalog binding, overlap, conditional HTTP headers, decompressed
/// body limits and actual discovery readiness must be checked at their owning
/// boundaries. This model neither installs a route nor activates legacy writes.
///
/// ```
/// use revaer_api_models::media_root_contract::{
///     DiscoveryAssociationRequest, DiscoveryModes,
/// };
/// use uuid::Uuid;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let request = DiscoveryAssociationRequest::new(
///     "library", Uuid::from_u128(1), 1, "source", "",
///     DiscoveryModes {
///         manual_enabled: true, watcher_enabled: false, schedule_enabled: false,
///     },
/// )?;
/// assert_eq!(request.root_relative_path(), "");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct DiscoveryAssociationRequest {
    association_key: String,
    media_profile_public_id: Uuid,
    profile_version: i32,
    source_root_key: String,
    root_relative_path: String,
    modes: DiscoveryModes,
}

impl DiscoveryAssociationRequest {
    /// Validate exact logical keys, explicit prefix and a positive profile version.
    ///
    /// # Errors
    /// Rejects keys outside the accepted 1-64 byte grammar, an invalid relative
    /// prefix, or a version outside the positive `PostgreSQL` integer range.
    pub fn new(
        association_key: &str,
        media_profile_public_id: Uuid,
        profile_version: i32,
        source_root_key: &str,
        root_relative_path: &str,
        modes: DiscoveryModes,
    ) -> Result<Self, RootInputError> {
        validate_root_logical_key(association_key)?;
        validate_root_logical_key(source_root_key)?;
        validate_root_association_prefix(root_relative_path)?;
        if profile_version <= 0 {
            return Err(RootInputError);
        }
        Ok(Self {
            association_key: association_key.to_owned(),
            media_profile_public_id,
            profile_version,
            source_root_key: source_root_key.to_owned(),
            root_relative_path: root_relative_path.to_owned(),
            modes,
        })
    }

    /// Return the unchanged logical association key.
    #[must_use]
    pub fn association_key(&self) -> &str {
        &self.association_key
    }

    /// Return the requested profile identity, not a resolved active profile.
    #[must_use]
    pub const fn media_profile_public_id(&self) -> Uuid {
        self.media_profile_public_id
    }

    /// Return the exact requested positive profile version.
    #[must_use]
    pub const fn profile_version(&self) -> i32 {
        self.profile_version
    }

    /// Return the logical source key, never a reconstructed filesystem path.
    #[must_use]
    pub fn source_root_key(&self) -> &str {
        &self.source_root_key
    }

    /// Return the exact explicit prefix; empty means whole-root selection.
    #[must_use]
    pub fn root_relative_path(&self) -> &str {
        &self.root_relative_path
    }

    /// Return all requested modes without applying defaults or readiness policy.
    #[must_use]
    pub const fn modes(&self) -> DiscoveryModes {
        self.modes
    }
}

impl fmt::Debug for DiscoveryAssociationRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DiscoveryAssociationRequest")
            .finish_non_exhaustive()
    }
}

impl Serialize for DiscoveryAssociationRequest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serializer.serialize_struct("DiscoveryAssociationRequest", 8)?;
        object.serialize_field("association_key", &self.association_key)?;
        object.serialize_field("media_profile_public_id", &self.media_profile_public_id)?;
        object.serialize_field("profile_version", &self.profile_version)?;
        object.serialize_field("source_root_key", &self.source_root_key)?;
        object.serialize_field("root_relative_path", &self.root_relative_path)?;
        object.serialize_field("manual_enabled", &self.modes.manual_enabled)?;
        object.serialize_field("watcher_enabled", &self.modes.watcher_enabled)?;
        object.serialize_field("schedule_enabled", &self.modes.schedule_enabled)?;
        object.end()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AssociationWire {
    association_key: String,
    media_profile_public_id: Uuid,
    profile_version: i32,
    source_root_key: String,
    root_relative_path: String,
    manual_enabled: bool,
    watcher_enabled: bool,
    schedule_enabled: bool,
}

impl<'de> Deserialize<'de> for DiscoveryAssociationRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire: AssociationWire =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootInputError))?;
        Self::new(
            &wire.association_key,
            wire.media_profile_public_id,
            wire.profile_version,
            &wire.source_root_key,
            &wire.root_relative_path,
            DiscoveryModes {
                manual_enabled: wire.manual_enabled,
                watcher_enabled: wire.watcher_enabled,
                schedule_enabled: wire.schedule_enabled,
            },
        )
        .map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
