//! Snapshot-consistent, informational readiness of immutable profile heads.

use revaer_api::models::{
    MediaProfileReadinessResponse,
    media_root_contract::{ProfileLifecycle, RootInputError},
};
use revaer_data::media::profile_versions::ProfileReadinessRows;

pub(super) fn response(
    rows: ProfileReadinessRows,
) -> Result<MediaProfileReadinessResponse, RootInputError> {
    let profile = super::profile_versions::response(&rows.latest)?.ok_or(RootInputError)?;
    let active = super::profile_versions::response(&rows.active)?;
    let latest = profile.fields();
    if latest.active_version != active.as_ref().map(|value| value.fields().latest_version)
        || active.as_ref().is_some_and(|value| {
            let fields = value.fields();
            fields.media_profile_public_id != latest.media_profile_public_id
                || fields.profile.fields().profile_key != latest.profile.fields().profile_key
                || fields.lifecycle_state != ProfileLifecycle::Active
        })
    {
        return Err(RootInputError);
    }
    let active_association_count =
        u16::try_from(rows.active_association_count).map_err(|_| RootInputError)?;
    if active_association_count > 128 {
        return Err(RootInputError);
    }
    let root_readiness = super::root_readiness::response(rows.roots).map_err(|_| RootInputError)?;
    let active_root_bindings = active
        .as_ref()
        .map(|value| value.fields().root_bindings.clone())
        .unwrap_or_default();
    let binding_reason = if active.is_none() {
        Some("media_root_binding_incomplete".to_owned())
    } else {
        active_root_bindings
            .iter()
            .find(|binding| !binding.binding_ready)
            .map(|binding| binding.binding_reason.clone().ok_or(RootInputError))
            .transpose()?
    };
    let destructive_reason = if let Some(reason) = &binding_reason {
        Some(reason.clone())
    } else {
        active_root_bindings
            .iter()
            .find(|binding| !binding.destructive_ready)
            .map(|binding| binding.destructive_reason.clone().ok_or(RootInputError))
            .transpose()?
    };
    Ok(MediaProfileReadinessResponse {
        active_profile: active.map(|value| value.fields().profile.clone()),
        profile,
        active_root_bindings,
        root_readiness,
        active_association_count,
        binding_ready: binding_reason.is_none(),
        binding_reason,
        destructive_ready: destructive_reason.is_none(),
        destructive_reason,
    })
}

#[cfg(test)]
mod tests;
