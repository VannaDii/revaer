//! Complete path-free profile responses under ADRs 557 and 590.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    ProfileVersionRequest, RootCatalogAllowedKind, RootInputError, RootKind, catalog_scalar,
};

mod wire;

/// Immutable version lifecycle, distinct from operational enablement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileLifecycle {
    /// Unresolved draft; an earlier active version may remain operational.
    Draft,
    /// Latest version is the active head, even when disabled.
    Active,
    /// Latest version archives the profile and clears its active head.
    Archived,
}

/// Persisted root resolution, not current filesystem readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileRootResolution {
    /// Bound to a catalog attestation; freshness is reported separately.
    Resolved,
    /// Logical key has not been mapped.
    Unmapped,
    /// Logical key does not permit this root role.
    KindForbidden,
}

/// A path-free reported binding; response construction validates coherence.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ProfileRootBinding {
    /// Output, workspace, backup or quarantine, in canonical ordinal order.
    pub kind: RootKind,
    /// Exact key matching the corresponding profile input.
    pub logical_key: String,
    /// Immutable resolution state of this version's binding.
    pub resolution_state: ProfileRootResolution,
    /// Current reported binding readiness, not execution authority.
    pub binding_ready: bool,
    /// Bounded reason, absent when binding-ready.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_reason: Option<String>,
    /// Current reported destructive readiness, not execution authority.
    pub destructive_ready: bool,
    /// Bounded reason, absent when destructive-ready.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_reason: Option<String>,
}

impl ProfileRootBinding {
    fn validate(&self) -> Result<(), RootInputError> {
        if self.resolution_state != ProfileRootResolution::Resolved && self.binding_ready {
            return Err(RootInputError);
        }
        RootCatalogAllowedKind {
            kind: self.kind,
            binding_ready: self.binding_ready,
            binding_reason: self.binding_reason.clone(),
            destructive_ready: self.destructive_ready,
            destructive_reason: self.destructive_reason.clone(),
        }
        .validate()
        .map_err(|_| RootInputError)
    }
}

impl fmt::Debug for ProfileRootBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProfileRootBinding")
    }
}

/// Complete response inputs, validated by [`ProfileVersionResponse::new`].
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ProfileVersionResponseFields {
    /// Complete submitted fields, serialized at the response's top level.
    #[serde(flatten)]
    pub profile: ProfileVersionRequest,
    /// Stable public profile identity.
    pub media_profile_public_id: Uuid,
    /// Latest immutable version fenced by the strong HTTP `ETag`.
    pub latest_version: i32,
    /// Operational head, omitted when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_version: Option<i32>,
    /// Latest version's lifecycle, independent of its enabled flag.
    pub lifecycle_state: ProfileLifecycle,
    /// Exact ordered bindings for required and supplied optional root keys.
    pub root_bindings: Vec<ProfileRootBinding>,
    /// UTC RFC 3339 creation time, preserved without normalization.
    pub created_at: String,
    /// UTC RFC 3339 update time, preserved without normalization.
    pub updated_at: String,
}

impl fmt::Debug for ProfileVersionResponseFields {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProfileVersionResponseFields")
    }
}

/// Validated complete profile representation without root filesystem identity.
///
/// Validation proves transport coherence only. Authentication, snapshot
/// consistency, `ETag` agreement and execution eligibility belong to callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ProfileVersionResponse(ProfileVersionResponseFields);

impl ProfileVersionResponse {
    /// Validate exact version heads, timestamps and ordered root bindings.
    ///
    /// # Errors
    /// Rejects invalid heads, lifecycle disagreement, non-UTC timestamps,
    /// missing/extra/reordered bindings, mismatched keys or incoherent readiness.
    pub fn new(fields: ProfileVersionResponseFields) -> Result<Self, RootInputError> {
        if fields.latest_version <= 0
            || fields
                .active_version
                .is_some_and(|version| version <= 0 || version > fields.latest_version)
        {
            return Err(RootInputError);
        }
        let valid_head = match fields.lifecycle_state {
            ProfileLifecycle::Active => fields.active_version == Some(fields.latest_version),
            ProfileLifecycle::Draft => fields.active_version != Some(fields.latest_version),
            ProfileLifecycle::Archived => fields.active_version.is_none(),
        };
        if !valid_head {
            return Err(RootInputError);
        }
        catalog_scalar::timestamp(&fields.created_at).map_err(|_| RootInputError)?;
        catalog_scalar::timestamp(&fields.updated_at).map_err(|_| RootInputError)?;
        let profile = fields.profile.fields();
        let expected = [
            (RootKind::Output, Some(profile.output_root_key.as_str())),
            (
                RootKind::Workspace,
                Some(profile.workspace_root_key.as_str()),
            ),
            (RootKind::Backup, profile.backup_root_key.as_deref()),
            (RootKind::Quarantine, profile.quarantine_root_key.as_deref()),
        ];
        let expected = expected
            .into_iter()
            .filter_map(|(kind, key)| key.map(|key| (kind, key)));
        let mut bindings = fields.root_bindings.iter();
        for (kind, key) in expected {
            let binding = bindings.next().ok_or(RootInputError)?;
            if binding.kind != kind || binding.logical_key != key {
                return Err(RootInputError);
            }
            binding.validate()?;
        }
        if bindings.next().is_some() {
            return Err(RootInputError);
        }
        Ok(Self(fields))
    }

    /// Borrow the exact validated representation without granting mutation.
    #[must_use]
    pub const fn fields(&self) -> &ProfileVersionResponseFields {
        &self.0
    }
}

#[cfg(test)]
mod tests;
