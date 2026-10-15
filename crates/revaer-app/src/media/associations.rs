//! Validate one association snapshot before exposing its path-free contract.

use revaer_api::models::media_root_contract::{
    DiscoveryAssociationRequest, DiscoveryAssociationResponse, DiscoveryAssociationResponseFields,
    DiscoveryModes, ProfileLifecycle, ProfileRootResolution, RootInputError,
};
use revaer_data::media::associations::AssociationRow;

use super::{MediaServiceError, MediaServiceErrorKind};

pub(super) fn response(
    row: AssociationRow,
) -> Result<DiscoveryAssociationResponse, MediaServiceError> {
    validated(row).map_err(|_| {
        tracing::error!(
            operation = "media_association_representation",
            "invalid persisted association representation"
        );
        MediaServiceError::new(MediaServiceErrorKind::Storage)
    })
}

fn validated(row: AssociationRow) -> Result<DiscoveryAssociationResponse, RootInputError> {
    let request = DiscoveryAssociationRequest::new(
        &row.association_key,
        row.media_profile_public_id,
        row.profile_version,
        &row.source_root_key,
        &row.root_relative_path,
        DiscoveryModes {
            manual_enabled: row.modes.manual_enabled,
            watcher_enabled: row.modes.watcher_enabled,
            schedule_enabled: row.modes.schedule_enabled,
        },
    )?;
    DiscoveryAssociationResponse::new(DiscoveryAssociationResponseFields {
        request,
        media_discovery_association_public_id: row.media_discovery_association_public_id,
        latest_version: row.latest_version,
        active_version: row.active_version,
        lifecycle_state: match row.lifecycle_state.as_str() {
            "active" => ProfileLifecycle::Active,
            "draft" => ProfileLifecycle::Draft,
            "archived" => ProfileLifecycle::Archived,
            _ => return Err(RootInputError),
        },
        resolution_state: match row.resolution_state.as_str() {
            "resolved" => ProfileRootResolution::Resolved,
            "unmapped" => ProfileRootResolution::Unmapped,
            "kind_forbidden" => ProfileRootResolution::KindForbidden,
            _ => return Err(RootInputError),
        },
        binding_ready: row.binding_ready,
        binding_reason: row.binding_reason,
        destructive_ready: row.destructive_ready,
        destructive_reason: row.destructive_reason,
        created_at: row.created_at.to_rfc3339(),
    })
}
pub(super) fn preview_scope(
    prefix: &str,
    dry_run: bool,
    paths: &[String],
) -> Vec<revaer_api::app::media::MediaDiscoveryPreviewResponse> {
    paths
        .iter()
        .map(|path| {
            let accepted = prefix.is_empty()
                || path == prefix
                || path
                    .strip_prefix(prefix)
                    .is_some_and(|suffix| suffix.starts_with('/'));
            revaer_api::app::media::MediaDiscoveryPreviewResponse {
                source_path: path.clone(),
                output_path: accepted.then(|| path.clone()),
                accepted,
                dry_run,
                reason: (!accepted)
                    .then(|| "media_discovery_source_path_outside_profile_root".into()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::preview_scope;

    #[test]
    fn preview_scope_keeps_relative_bytes_and_component_boundaries() {
        let paths = vec![
            "Movies/a.mkv".into(),
            "Movies-other/a.mkv".into(),
            "Other/a.mkv".into(),
        ];
        let scoped = preview_scope("Movies", true, &paths);
        assert!(scoped[0].accepted);
        assert!(scoped[0].dry_run);
        assert_eq!(scoped[0].output_path.as_deref(), Some("Movies/a.mkv"));
        assert!(!scoped[1].accepted);
        assert!(!scoped[2].accepted);
        assert_eq!(scoped[1].source_path, paths[1]);
        assert!(scoped[1].output_path.is_none());
        assert!(
            preview_scope("", false, &paths)
                .iter()
                .all(|item| item.accepted && !item.dry_run)
        );
    }
}
