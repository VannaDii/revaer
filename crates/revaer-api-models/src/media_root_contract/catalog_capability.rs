//! Explicit reported probe results, serialized as the required JSON booleans.

use serde::{Deserialize, Deserializer, Serialize, de};

use super::RootCatalogError;

/// A reported capability probe result, not a filesystem access capability.
/// There is intentionally no default: every transport field must be supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RootCatalogCapability(bool);

impl RootCatalogCapability {
    /// Preserve an explicitly supplied probe result without deriving readiness.
    #[must_use]
    pub const fn new(value: bool) -> Self {
        Self(value)
    }

    /// Return the exact reported probe result.
    #[must_use]
    pub const fn get(self) -> bool {
        self.0
    }
}

impl<'de> Deserialize<'de> for RootCatalogCapability {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bool::deserialize(deserializer)
            .map(Self)
            .map_err(|_| de::Error::custom(RootCatalogError))
    }
}
