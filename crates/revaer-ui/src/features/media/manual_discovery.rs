//! Bounded relative candidates and complete manual discovery confirmations.

use revaer_api_models::{
    MediaDiscoveryPreviewRequest, MediaDiscoveryPreviewResponse, MediaDiscoveryRunResponse,
};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AssociationChoice {
    pub id: Uuid,
    pub key: String,
    pub source_key: String,
    pub prefix: String,
    pub profile_version: i32,
    pub association_version: i32,
    pub configuration_ready: bool,
    pub manual_ready: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AssociationPage {
    pub choices: Vec<AssociationChoice>,
    pub next_cursor: Option<String>,
}

impl From<revaer_api_models::media_root_contract::DiscoveryAssociationPageResponse>
    for AssociationPage
{
    fn from(
        page: revaer_api_models::media_root_contract::DiscoveryAssociationPageResponse,
    ) -> Self {
        let (associations, next_cursor) = page.into_parts();
        let choices = associations
            .into_iter()
            .map(|association| {
                let fields = association.fields();
                AssociationChoice {
                    id: fields.media_discovery_association_public_id,
                    key: fields.request.association_key().into(),
                    source_key: fields.request.source_root_key().into(),
                    prefix: fields.request.root_relative_path().into(),
                    profile_version: fields.request.profile_version(),
                    association_version: fields.latest_version,
                    configuration_ready: fields.binding_ready
                        && fields.active_version == Some(fields.latest_version),
                    manual_ready: fields.request.modes().manual_enabled
                        && fields.binding_ready
                        && fields.active_version == Some(fields.latest_version),
                }
            })
            .collect();
        Self {
            choices,
            next_cursor,
        }
    }
}

pub(crate) fn request(
    association: Uuid,
    candidates: &str,
) -> Result<MediaDiscoveryPreviewRequest, &'static str> {
    let request = MediaDiscoveryPreviewRequest {
        media_discovery_association_public_id: association,
        source_paths: candidates.lines().map(str::to_owned).collect(),
    };
    request
        .validate()
        .map_err(|_| "Enter 1 to 128 nonempty root-relative paths, one per line.")?;
    Ok(request)
}

pub(crate) fn confirm_preview(
    request: &MediaDiscoveryPreviewRequest,
    response: &MediaDiscoveryPreviewResponse,
) -> Result<(), &'static str> {
    if response.previews.len() != request.source_paths.len()
        || response
            .previews
            .iter()
            .zip(&request.source_paths)
            .any(|(row, path)| {
                row.source_path != *path
                    || (row.accepted && row.output_path.as_deref() != Some(path.as_str()))
                    || (!row.accepted && row.output_path.is_some())
            })
    {
        return Err("Discovery preview returned inconsistent candidates. No jobs were queued.");
    }
    Ok(())
}

pub(crate) fn confirm_run(
    request: &MediaDiscoveryPreviewRequest,
    response: &MediaDiscoveryRunResponse,
) -> Result<(), &'static str> {
    let mut remaining = request.source_paths.clone();
    let mut ids = std::collections::BTreeSet::new();
    for row in &response.queued_jobs {
        if row.output_path != row.source_path
            || !ids.insert(row.media_job_public_id)
            || !consume(&mut remaining, &row.source_path)
        {
            return Err(
                "Discovery returned inconsistent jobs. Check recent jobs before resubmitting.",
            );
        }
    }
    for row in &response.skipped {
        if !consume(&mut remaining, &row.source_path) {
            return Err(
                "Discovery returned inconsistent skips. Check recent jobs before resubmitting.",
            );
        }
    }
    if !remaining.is_empty() {
        return Err("Discovery confirmation is incomplete. Check recent jobs before resubmitting.");
    }
    Ok(())
}

fn consume(remaining: &mut Vec<String>, path: &str) -> bool {
    remaining
        .iter()
        .position(|candidate| candidate == path)
        .is_some_and(|index| {
            remaining.remove(index);
            true
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use revaer_api_models::{
        MediaDiscoveryPreviewItemResponse, MediaDiscoveryQueuedJobResponse,
        MediaDiscoverySkippedItemResponse,
    };

    #[test]
    fn saved_choices_preserve_scope_and_withhold_unready_bindings()
    -> Result<(), Box<dyn std::error::Error>> {
        use revaer_api_models::media_root_contract::{
            DiscoveryAssociationPageResponse, DiscoveryAssociationRequest,
            DiscoveryAssociationResponse, DiscoveryAssociationResponseFields, DiscoveryModes,
            ProfileLifecycle, ProfileRootResolution,
        };
        let row = |key,
                   id,
                   ready|
         -> Result<
            DiscoveryAssociationResponse,
            revaer_api_models::media_root_contract::RootInputError,
        > {
            DiscoveryAssociationResponse::new(DiscoveryAssociationResponseFields {
                request: DiscoveryAssociationRequest::new(
                    key,
                    Uuid::from_u128(9),
                    3,
                    "source",
                    "Movies",
                    DiscoveryModes {
                        manual_enabled: true,
                        watcher_enabled: false,
                        schedule_enabled: false,
                    },
                )?,
                media_discovery_association_public_id: Uuid::from_u128(id),
                latest_version: 1,
                active_version: Some(1),
                lifecycle_state: ProfileLifecycle::Active,
                resolution_state: ProfileRootResolution::Resolved,
                binding_ready: ready,
                binding_reason: (!ready).then(|| "media_root_binding_incomplete".into()),
                destructive_ready: false,
                destructive_reason: Some(
                    if ready {
                        "media_root_durability_unproven"
                    } else {
                        "media_root_binding_incomplete"
                    }
                    .into(),
                ),
                created_at: "2026-10-02T00:00:00Z".into(),
            })
        };
        let page = AssociationPage::from(DiscoveryAssociationPageResponse::new(
            vec![row("library-a", 1, true)?, row("library-b", 2, false)?],
            None,
        )?);
        assert_eq!(page.choices.len(), 2);
        assert!(page.choices[0].manual_ready);
        assert!(!page.choices[1].manual_ready);
        assert_eq!(page.choices[0].prefix, "Movies");
        assert_eq!(page.choices[0].profile_version, 3);
        assert_eq!(page.choices[0].source_key, "source");
        assert_eq!(page.choices[0].id, Uuid::from_u128(1));
        assert!(page.next_cursor.is_none());
        Ok(())
    }

    #[test]
    fn candidates_are_relative_bounded_and_not_silently_trimmed() {
        let id = Uuid::from_u128(1);
        for invalid in ["", "/private/a", "../a", "a//b", "a\n\nb"] {
            assert!(request(id, invalid).is_err());
        }
        assert!(request(id, &vec!["a"; 129].join("\n")).is_err());
        let parsed = request(id, "Movies/a b.mkv\nMovies/c.mkv");
        assert!(parsed.is_ok_and(|value| value.source_paths == ["Movies/a b.mkv", "Movies/c.mkv"]));
    }

    #[test]
    fn preview_requires_exact_order_and_no_physical_output_paths() -> Result<(), &'static str> {
        let request = request(Uuid::from_u128(1), "a")?;
        let mut response = MediaDiscoveryPreviewResponse {
            previews: vec![MediaDiscoveryPreviewItemResponse {
                source_path: "a".into(),
                output_path: Some("a".into()),
                accepted: true,
                dry_run: true,
                reason: None,
            }],
        };
        assert!(confirm_preview(&request, &response).is_ok());
        response.previews[0].output_path = Some("/private/a".into());
        assert!(confirm_preview(&request, &response).is_err());
        response.previews.clear();
        assert!(confirm_preview(&request, &response).is_err());
        Ok(())
    }

    #[test]
    fn queue_confirmation_accounts_for_duplicates_and_rejects_extra_or_missing_rows()
    -> Result<(), &'static str> {
        let request = request(Uuid::from_u128(1), "a\na\nb")?;
        let mut response = MediaDiscoveryRunResponse {
            queued_jobs: vec![MediaDiscoveryQueuedJobResponse {
                media_job_public_id: Uuid::from_u128(2),
                source_path: "a".into(),
                output_path: "a".into(),
                dry_run: true,
            }],
            skipped: vec![
                MediaDiscoverySkippedItemResponse {
                    source_path: "a".into(),
                    reason: None,
                },
                MediaDiscoverySkippedItemResponse {
                    source_path: "b".into(),
                    reason: None,
                },
            ],
        };
        assert!(confirm_run(&request, &response).is_ok());
        response.skipped.pop();
        assert!(confirm_run(&request, &response).is_err());
        response.skipped.push(MediaDiscoverySkippedItemResponse {
            source_path: "/private/b".into(),
            reason: None,
        });
        assert!(confirm_run(&request, &response).is_err());
        response.queued_jobs.push(response.queued_jobs[0].clone());
        assert!(confirm_run(&request, &response).is_err());
        Ok(())
    }
}
