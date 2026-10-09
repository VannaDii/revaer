//! Validate one persisted profile snapshot before exposing its public contract.

use revaer_api::models::media_root_contract::{
    ProfileCollectionCursor, ProfileLifecycle, ProfileRootBinding, ProfileRootResolution,
    ProfileVersionPageResponse, ProfileVersionRequest, ProfileVersionRequestFields,
    ProfileVersionResponse, ProfileVersionResponseFields, RootInputError, RootKind,
};
use revaer_data::media::profile_versions::ProfileVersionRow;

pub(super) fn write_input(
    actor_public_id: uuid::Uuid,
    request: &ProfileVersionRequest,
) -> revaer_data::media::profile_versions::CreateProfileVersionInput<'_> {
    let fields = request.fields();
    revaer_data::media::profile_versions::CreateProfileVersionInput {
        actor_public_id,
        profile_key: &fields.profile_key,
        display_name: &fields.display_name,
        description: &fields.description,
        enabled: fields.enabled,
        dry_run_only: fields.dry_run_only,
        desired_target_key: &fields.desired_target_key,
        desired_target_version: fields.desired_target_version,
        policy_key: &fields.policy_key,
        policy_version: fields.policy_version,
        output_root_key: &fields.output_root_key,
        workspace_root_key: &fields.workspace_root_key,
        backup_root_key: fields.backup_root_key.as_deref(),
        quarantine_root_key: fields.quarantine_root_key.as_deref(),
    }
}

pub(super) fn page(
    rows: &[ProfileVersionRow],
    limit: u16,
) -> Result<ProfileVersionPageResponse, RootInputError> {
    let limit = usize::from(
        revaer_api::models::media_root_contract::validate_root_catalog_limit(Some(limit))?,
    );
    if rows.len() > (limit + 1) * 4
        || rows.windows(2).any(|pair| {
            pair[0].media_profile_public_id != pair[1].media_profile_public_id
                && (&pair[0].profile_key, pair[0].media_profile_public_id)
                    >= (&pair[1].profile_key, pair[1].media_profile_public_id)
        })
    {
        return Err(RootInputError);
    }
    let mut profiles = Vec::new();
    let mut remaining = rows;
    while let Some(first) = remaining.first() {
        let count = remaining
            .iter()
            .take_while(|row| row.media_profile_public_id == first.media_profile_public_id)
            .count();
        let (group, rest) = remaining.split_at(count);
        profiles.push(response(group)?.ok_or(RootInputError)?);
        remaining = rest;
        if profiles.len() > limit + 1 {
            return Err(RootInputError);
        }
    }
    let next_cursor = if profiles.len() > limit {
        profiles.truncate(limit);
        let last = profiles.last().ok_or(RootInputError)?.fields();
        Some(
            ProfileCollectionCursor::new(
                &last.profile.fields().profile_key,
                last.media_profile_public_id,
            )?
            .encode()?,
        )
    } else {
        None
    };
    ProfileVersionPageResponse::new(profiles, next_cursor)
}

pub(super) fn response(
    rows: &[ProfileVersionRow],
) -> Result<Option<ProfileVersionResponse>, RootInputError> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let key = |kind: &str| {
        rows.iter()
            .find(|row| row.root_kind == kind)
            .map(|row| row.logical_key.clone())
    };
    let profile = ProfileVersionRequest::new(ProfileVersionRequestFields {
        profile_key: first.profile_key.clone(),
        display_name: first.display_name.clone(),
        description: first.description.clone(),
        enabled: first.enabled,
        dry_run_only: first.dry_run_only,
        desired_target_key: first.desired_target_key.clone(),
        desired_target_version: first.desired_target_version,
        policy_key: first.policy_key.clone(),
        policy_version: first.policy_version,
        output_root_key: key("output").ok_or(RootInputError)?,
        workspace_root_key: key("workspace").ok_or(RootInputError)?,
        backup_root_key: key("backup"),
        quarantine_root_key: key("quarantine"),
    })?;
    let root_bindings = rows
        .iter()
        .map(|row| {
            if !same_profile(first, row) {
                return Err(RootInputError);
            }
            Ok(ProfileRootBinding {
                kind: match row.root_kind.as_str() {
                    "output" => RootKind::Output,
                    "workspace" => RootKind::Workspace,
                    "backup" => RootKind::Backup,
                    "quarantine" => RootKind::Quarantine,
                    _ => return Err(RootInputError),
                },
                logical_key: row.logical_key.clone(),
                resolution_state: match row.resolution_state.as_str() {
                    "resolved" => ProfileRootResolution::Resolved,
                    "unmapped" => ProfileRootResolution::Unmapped,
                    "kind_forbidden" => ProfileRootResolution::KindForbidden,
                    _ => return Err(RootInputError),
                },
                binding_ready: row.readiness.binding_ready,
                binding_reason: row.readiness.binding_reason.clone(),
                destructive_ready: row.readiness.destructive_ready,
                destructive_reason: row.readiness.destructive_reason.clone(),
            })
        })
        .collect::<Result<_, _>>()?;
    ProfileVersionResponse::new(ProfileVersionResponseFields {
        profile,
        media_profile_public_id: first.media_profile_public_id,
        latest_version: first.latest_version,
        active_version: first.active_version,
        lifecycle_state: match first.lifecycle_state.as_str() {
            "draft" => ProfileLifecycle::Draft,
            "active" => ProfileLifecycle::Active,
            "archived" => ProfileLifecycle::Archived,
            _ => return Err(RootInputError),
        },
        root_bindings,
        created_at: first.created_at.to_rfc3339(),
        updated_at: first.updated_at.to_rfc3339(),
    })
    .map(Some)
}

fn same_profile(left: &ProfileVersionRow, right: &ProfileVersionRow) -> bool {
    left.media_profile_public_id == right.media_profile_public_id
        && left.profile_key == right.profile_key
        && left.display_name == right.display_name
        && left.description == right.description
        && left.enabled == right.enabled
        && left.dry_run_only == right.dry_run_only
        && left.desired_target_key == right.desired_target_key
        && left.desired_target_version == right.desired_target_version
        && left.policy_key == right.policy_key
        && left.policy_version == right.policy_version
        && left.latest_version == right.latest_version
        && left.active_version == right.active_version
        && left.lifecycle_state == right.lifecycle_state
        && left.created_at == right.created_at
        && left.updated_at == right.updated_at
}
