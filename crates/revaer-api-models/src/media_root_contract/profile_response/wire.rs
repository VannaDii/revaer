//! Strict flat response decoding; no flatten decoder or implicit field defaults.

use serde::{Deserialize, Deserializer, de};
use uuid::Uuid;

use super::{
    ProfileLifecycle, ProfileRootBinding, ProfileRootResolution, ProfileVersionResponse,
    ProfileVersionResponseFields,
};
use crate::media_root_contract::{
    ProfileVersionRequest, ProfileVersionRequestFields, RootInputError, RootKind, object,
};

#[derive(Deserialize)]
#[serde(transparent)]
struct BindingInput(#[serde(with = "BindingWire")] ProfileRootBinding);

#[derive(Deserialize)]
#[serde(remote = "ProfileRootBinding", deny_unknown_fields)]
struct BindingWire {
    kind: RootKind,
    logical_key: String,
    resolution_state: ProfileRootResolution,
    binding_ready: bool,
    binding_reason: Option<String>,
    destructive_ready: bool,
    destructive_reason: Option<String>,
}

impl<'de> Deserialize<'de> for ProfileRootBinding {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input: BindingInput =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootInputError))?;
        Ok(input.0)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    profile_key: String,
    display_name: String,
    description: String,
    enabled: bool,
    dry_run_only: bool,
    desired_target_key: String,
    desired_target_version: i32,
    policy_key: String,
    policy_version: i32,
    output_root_key: String,
    workspace_root_key: String,
    #[serde(default, deserialize_with = "present")]
    backup_root_key: Option<String>,
    #[serde(default, deserialize_with = "present")]
    quarantine_root_key: Option<String>,
    media_profile_public_id: Uuid,
    latest_version: i32,
    #[serde(default, deserialize_with = "present")]
    active_version: Option<i32>,
    lifecycle_state: ProfileLifecycle,
    root_bindings: Vec<ProfileRootBinding>,
    created_at: String,
    updated_at: String,
}

fn present<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

impl<'de> Deserialize<'de> for ProfileVersionResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire: Wire =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootInputError))?;
        let profile = ProfileVersionRequest::new(ProfileVersionRequestFields {
            profile_key: wire.profile_key,
            display_name: wire.display_name,
            description: wire.description,
            enabled: wire.enabled,
            dry_run_only: wire.dry_run_only,
            desired_target_key: wire.desired_target_key,
            desired_target_version: wire.desired_target_version,
            policy_key: wire.policy_key,
            policy_version: wire.policy_version,
            output_root_key: wire.output_root_key,
            workspace_root_key: wire.workspace_root_key,
            backup_root_key: wire.backup_root_key,
            quarantine_root_key: wire.quarantine_root_key,
        })
        .map_err(de::Error::custom)?;
        Self::new(ProfileVersionResponseFields {
            profile,
            media_profile_public_id: wire.media_profile_public_id,
            latest_version: wire.latest_version,
            active_version: wire.active_version,
            lifecycle_state: wire.lifecycle_state,
            root_bindings: wire.root_bindings,
            created_at: wire.created_at,
            updated_at: wire.updated_at,
        })
        .map_err(de::Error::custom)
    }
}
