//! Real service authorization and lifecycle for profile-bound public searches.

use super::super::*;
use super::{SYSTEM_USER_PUBLIC_ID, build_service, unique_name};
use revaer_data::indexers::manual_search::{
    self, ManualSearchCompletion, ManualSearchOutcome, ManualSearchWriteOutcome,
};

#[tokio::test]
async fn public_torznab_creation_uses_bound_profile_and_rejects_wrong_key() -> anyhow::Result<()> {
    let (service, database) = build_service().await?;
    let result = async {
        let actor = SYSTEM_USER_PUBLIC_ID;
        let included = service
            .indexer_instance_create(actor, "torznab", &unique_name("Included"), None, None, None)
            .await?;
        let _excluded = service
            .indexer_instance_create(actor, "torznab", &unique_name("Excluded"), None, None, None)
            .await?;
        let profile = service
            .search_profile_create(
                actor,
                &unique_name("Public profile"),
                None,
                Some(20),
                None,
                None,
            )
            .await?;
        service
            .search_profile_indexer_allow(actor, profile, &[included])
            .await?;
        let instance = service
            .torznab_instance_create(actor, profile, &unique_name("Public instance"))
            .await?;
        let created = service
            .torznab_search_request_create(
                instance.torznab_instance_public_id,
                &instance.api_key_plaintext,
                parameters(),
            )
            .await?;
        let request = created.search_request_public_id;
        anyhow::ensure!(!service.search_request_is_terminal(actor, request).await?);
        let run = manual_search::claim_next(service.config.pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected permitted work"))?;
        anyhow::ensure!(
            run.search_request_public_id == request && run.indexer_instance_public_id == included
        );
        anyhow::ensure!(
            manual_search::claim_next(service.config.pool())
                .await?
                .is_none()
        );
        service.search_request_cancel(actor, request).await?;
        anyhow::ensure!(service.search_request_is_terminal(actor, request).await?);
        anyhow::ensure!(
            manual_search::complete(
                service.config.pool(),
                &run,
                ManualSearchCompletion {
                    outcome: ManualSearchOutcome::Canceled,
                    http_status: None,
                    latency_ms: 0,
                    result_count: 0,
                    retry_after_seconds: None,
                }
            )
            .await?
                == ManualSearchWriteOutcome::Applied
        );
        let denied = service
            .torznab_search_request_create(
                instance.torznab_instance_public_id,
                &Uuid::new_v4().to_string(),
                parameters(),
            )
            .await;
        anyhow::ensure!(matches!(denied, Err(error) if error.code() == Some("api_key_invalid")));
        anyhow::ensure!(
            manual_search::claim_next(service.config.pool())
                .await?
                .is_none()
        );
        Ok::<(), anyhow::Error>(())
    }
    .await;
    service.config.pool().close().await;
    match (result, database.close()) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(operation), Err(cleanup)) => Err(anyhow::anyhow!(
            "Public search failed: {operation}; cleanup failed: {cleanup}"
        )),
    }
}

fn parameters() -> SearchRequestCreateParams<'static> {
    SearchRequestCreateParams {
        // These overrides must never authorize another actor/profile or policy.
        actor_user_public_id: Some(Uuid::new_v4()),
        query_text: "Bound public search",
        query_type: "free_text",
        torznab_mode: Some("generic"),
        requested_media_domain_key: None,
        page_size: Some(200),
        search_profile_public_id: Some(Uuid::new_v4()),
        request_policy_set_public_id: Some(Uuid::new_v4()),
        season_number: None,
        episode_number: None,
        identifier_types: None,
        identifier_values: None,
        torznab_cat_ids: None,
    }
}
