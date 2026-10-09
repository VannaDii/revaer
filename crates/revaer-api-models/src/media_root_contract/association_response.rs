//! Complete path-free representation of one immutable association head.

use serde::{Deserialize, Deserializer, Serialize, de};
use uuid::Uuid;

use super::{DiscoveryAssociationRequest, ProfileLifecycle, ProfileRootResolution, RootInputError};

/// Complete association representation; `ETag` is supplied by the HTTP header.
#[derive(Clone, Serialize)]
pub struct DiscoveryAssociationResponseFields {
    /// Complete unchanged logical request fields.
    #[serde(flatten)]
    pub request: DiscoveryAssociationRequest,
    /// Stable parent identity.
    pub media_discovery_association_public_id: Uuid,
    /// Latest immutable version, fenced by `ETag`.
    pub latest_version: i32,
    /// Current operational version, absent for a draft or archive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_version: Option<i32>,
    /// Latest version's immutable lifecycle.
    pub lifecycle_state: ProfileLifecycle,
    /// Logical source resolution state.
    pub resolution_state: ProfileRootResolution,
    /// Current binding readiness, not execution permission.
    pub binding_ready: bool,
    /// Closed reason when binding readiness is absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_reason: Option<String>,
    /// Current destructive readiness, independent of discovery intent.
    pub destructive_ready: bool,
    /// Closed reason when destructive readiness is absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_reason: Option<String>,
    /// UTC RFC 3339 parent creation time.
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(remote = "super::DiscoveryModes")]
struct ModesWire {
    manual_enabled: bool,
    watcher_enabled: bool,
    schedule_enabled: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    association_key: String,
    media_profile_public_id: Uuid,
    profile_version: i32,
    source_root_key: String,
    root_relative_path: String,
    #[serde(flatten, with = "ModesWire")]
    modes: super::DiscoveryModes,
    media_discovery_association_public_id: Uuid,
    latest_version: i32,
    #[serde(default, deserialize_with = "present")]
    active_version: Option<i32>,
    lifecycle_state: ProfileLifecycle,
    resolution_state: ProfileRootResolution,
    binding_ready: bool,
    #[serde(default, deserialize_with = "present")]
    binding_reason: Option<String>,
    destructive_ready: bool,
    #[serde(default, deserialize_with = "present")]
    destructive_reason: Option<String>,
    created_at: String,
}

fn present<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

impl<'de> Deserialize<'de> for DiscoveryAssociationResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire: Wire = super::object::deserialize(deserializer)
            .map_err(|_| de::Error::custom(RootInputError))?;
        let request = DiscoveryAssociationRequest::new(
            &wire.association_key,
            wire.media_profile_public_id,
            wire.profile_version,
            &wire.source_root_key,
            &wire.root_relative_path,
            wire.modes,
        )
        .map_err(de::Error::custom)?;
        Self::new(DiscoveryAssociationResponseFields {
            request,
            media_discovery_association_public_id: wire.media_discovery_association_public_id,
            latest_version: wire.latest_version,
            active_version: wire.active_version,
            lifecycle_state: wire.lifecycle_state,
            resolution_state: wire.resolution_state,
            binding_ready: wire.binding_ready,
            binding_reason: wire.binding_reason,
            destructive_ready: wire.destructive_ready,
            destructive_reason: wire.destructive_reason,
            created_at: wire.created_at,
        })
        .map_err(de::Error::custom)
    }
}

/// Validated complete persisted association representation.
#[derive(Clone, Serialize)]
#[serde(transparent)]
pub struct DiscoveryAssociationResponse(DiscoveryAssociationResponseFields);

impl DiscoveryAssociationResponse {
    /// Validate persisted head and closed readiness coherence before publication.
    ///
    /// # Errors
    /// Rejects invalid heads, time encoding, lifecycle or readiness reasons.
    pub fn new(fields: DiscoveryAssociationResponseFields) -> Result<Self, RootInputError> {
        if fields.latest_version <= 0
            || fields
                .active_version
                .is_some_and(|v| v <= 0 || v > fields.latest_version)
            || fields.binding_ready == fields.binding_reason.is_some()
            || fields.destructive_ready == fields.destructive_reason.is_some()
            || (fields.destructive_ready && !fields.binding_ready)
            || (fields.binding_ready
                && (fields.lifecycle_state != ProfileLifecycle::Active
                    || fields.resolution_state != ProfileRootResolution::Resolved
                    || fields.active_version != Some(fields.latest_version)))
            || (fields.lifecycle_state == ProfileLifecycle::Archived
                && fields.active_version.is_some())
            || (fields.lifecycle_state != ProfileLifecycle::Active
                && (fields.request.modes().manual_enabled
                    || fields.request.modes().watcher_enabled
                    || fields.request.modes().schedule_enabled))
            || (!fields.created_at.ends_with('Z') && !fields.created_at.ends_with("+00:00"))
            || chrono::DateTime::parse_from_rfc3339(&fields.created_at).is_err()
            || fields
                .binding_reason
                .iter()
                .chain(fields.destructive_reason.iter())
                .any(|reason| {
                    !matches!(
                        reason.as_str(),
                        "media_root_binding_incomplete" | "media_root_durability_unproven"
                    )
                })
        {
            return Err(RootInputError);
        }
        Ok(Self(fields))
    }

    /// Inspect the complete validated persisted fields.
    #[must_use]
    pub const fn fields(&self) -> &DiscoveryAssociationResponseFields {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media_root_contract::DiscoveryModes;

    fn fields() -> Result<DiscoveryAssociationResponseFields, RootInputError> {
        Ok(DiscoveryAssociationResponseFields {
            request: DiscoveryAssociationRequest::new(
                "library",
                Uuid::from_u128(1),
                1,
                "source",
                "",
                DiscoveryModes {
                    manual_enabled: true,
                    watcher_enabled: false,
                    schedule_enabled: false,
                },
            )?,
            media_discovery_association_public_id: Uuid::from_u128(2),
            latest_version: 1,
            active_version: Some(1),
            lifecycle_state: ProfileLifecycle::Active,
            resolution_state: ProfileRootResolution::Resolved,
            binding_ready: true,
            binding_reason: None,
            destructive_ready: false,
            destructive_reason: Some("media_root_durability_unproven".into()),
            created_at: "2026-10-02T00:00:00Z".into(),
        })
    }

    #[test]
    fn decoding_revalidates_complete_rows_and_page_order() -> Result<(), Box<dyn std::error::Error>>
    {
        use crate::media_root_contract::{
            AssociationCollectionCursor, DiscoveryAssociationPageResponse,
        };
        let value = serde_json::to_value(DiscoveryAssociationResponse::new(fields()?)?)?;
        let decoded: DiscoveryAssociationResponse = serde_json::from_value(value.clone())?;
        assert_eq!(serde_json::to_value(decoded)?, value);
        for (field, invalid) in [
            ("manual_enabled", serde_json::Value::Null),
            ("active_version", serde_json::Value::Null),
            ("active_version", serde_json::json!(2)),
            (
                "binding_reason",
                serde_json::json!("/private/never-display"),
            ),
            ("source_root", serde_json::json!("/private/root")),
        ] {
            let mut bad = value.clone();
            bad[field] = invalid;
            assert!(serde_json::from_value::<DiscoveryAssociationResponse>(bad).is_err());
        }
        let serialized = serde_json::to_string(&value)?;
        let duplicate = format!(
            "{{\"manual_enabled\":false,{}",
            serialized.strip_prefix('{').ok_or("association object")?
        );
        assert!(serde_json::from_str::<DiscoveryAssociationResponse>(&duplicate).is_err());
        let cursor = AssociationCollectionCursor::new("library", Uuid::from_u128(2))?.encode()?;
        let page = serde_json::json!({"associations": [value], "next_cursor": cursor});
        let parsed: DiscoveryAssociationPageResponse = serde_json::from_value(page)?;
        assert_eq!(parsed.into_parts().0.len(), 1);
        assert!(
            serde_json::from_value::<DiscoveryAssociationPageResponse>(
                serde_json::json!({"associations": [value.clone(), value]})
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn association_representation_preserves_explicit_scope_and_closed_readiness()
    -> Result<(), Box<dyn std::error::Error>> {
        let response = DiscoveryAssociationResponse::new(fields()?)?;
        let body = serde_json::to_value(response)?;
        assert_eq!(body["root_relative_path"], "");
        assert_eq!(body["source_root_key"], "source");
        assert_eq!(body["manual_enabled"], true);
        assert_eq!(body["active_version"], 1);
        assert_eq!(body["binding_ready"], true);
        assert_eq!(body["destructive_ready"], false);
        assert_eq!(body.as_object().ok_or("response object")?.len(), 17);
        for change in 0..6 {
            let mut invalid = fields()?;
            match change {
                0 => invalid.active_version = Some(2),
                1 => invalid.binding_reason = Some("/private/never-return".into()),
                2 => invalid.destructive_ready = true,
                3 => invalid.resolution_state = ProfileRootResolution::Unmapped,
                4 => invalid.lifecycle_state = ProfileLifecycle::Archived,
                _ => invalid.created_at = "2026-10-02T01:00:00+01:00".into(),
            }
            assert!(DiscoveryAssociationResponse::new(invalid).is_err());
        }
        Ok(())
    }
}
