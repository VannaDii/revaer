//! Bounded HTTP observation of committed search results; owns no attempt or transaction.

use std::time::Duration;

use tokio::time::{Instant, sleep_until, timeout_at};
use uuid::Uuid;

use crate::app::indexers::SearchRequestServiceError;
use crate::app::state::ApiState;
use crate::http::handlers::indexers::SYSTEM_ACTOR_PUBLIC_ID;
use crate::models::SearchPageSummaryResponse;

pub(super) async fn sealed_pages(
    state: &ApiState,
    request: Uuid,
    wanted_items: usize,
) -> Result<Vec<SearchPageSummaryResponse>, SearchRequestServiceError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut available = Vec::new();
    loop {
        let observation = async {
            let terminal = state
                .indexers
                .search_request_is_terminal(SYSTEM_ACTOR_PUBLIC_ID, request)
                .await?;
            let pages = state
                .indexers
                .search_page_list(SYSTEM_ACTOR_PUBLIC_ID, request)
                .await?;
            Ok::<_, SearchRequestServiceError>((pages.pages, terminal))
        };
        match timeout_at(deadline, observation).await {
            Ok(Ok((mut pages, terminal))) => {
                pages.retain(|page| page.sealed_at.is_some());
                let count = pages.iter().fold(0_usize, |total, page| {
                    total.saturating_add(usize::try_from(page.item_count).unwrap_or(0))
                });
                available = pages;
                if terminal || count >= wanted_items {
                    return Ok(available);
                }
            }
            Ok(Err(error)) => return Err(error),
            // Dropping these read-only observations cannot cancel or complete a run.
            Err(_) => return Ok(available),
        }
        sleep_until(deadline.min(Instant::now() + Duration::from_millis(100))).await;
        if Instant::now() >= deadline {
            return Ok(available);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::indexers::SearchRequestServiceErrorKind;
    use crate::http::handlers::indexers::test_support::{RecordingIndexers, indexer_test_state};
    use crate::models::{SearchPageListResponse, SearchRequestExplainabilityResponse};
    use std::sync::Arc;

    #[tokio::test]
    async fn torznab_deadline_leaves_pending_search_unfinished() -> anyhow::Result<()> {
        let indexers = RecordingIndexers::default();
        indexers.set_search_terminal_result(Ok(false));
        let state = indexer_test_state(Arc::new(indexers))?;
        let request = Uuid::new_v4();
        let started = Instant::now();
        let pages = sealed_pages(&state, request, 10).await?;
        anyhow::ensure!(started.elapsed() >= Duration::from_secs(5));
        anyhow::ensure!(pages.is_empty());
        anyhow::ensure!(
            !state
                .indexers
                .search_request_is_terminal(SYSTEM_ACTOR_PUBLIC_ID, request)
                .await?
        );
        Ok(())
    }

    #[tokio::test]
    async fn torznab_window_returns_only_sealed_pages_before_terminal_state() -> anyhow::Result<()>
    {
        let indexers = RecordingIndexers::default();
        indexers.set_search_terminal_result(Ok(false));
        indexers.set_search_page_list_response(SearchPageListResponse {
            pages: vec![
                SearchPageSummaryResponse {
                    page_number: 1,
                    item_count: 20,
                    sealed_at: Some(chrono::Utc::now()),
                },
                SearchPageSummaryResponse {
                    page_number: 2,
                    item_count: 5,
                    sealed_at: None,
                },
            ],
            explainability: SearchRequestExplainabilityResponse {
                zero_runnable_indexers: false,
                skipped_canceled_indexers: 0,
                skipped_failed_indexers: 0,
                blocked_results: 0,
                blocked_rule_public_ids: Vec::new(),
                rate_limited_indexers: 0,
                retrying_indexers: 0,
            },
        });
        let state = indexer_test_state(Arc::new(indexers))?;
        let request = Uuid::new_v4();
        let pages = sealed_pages(&state, request, 10).await?;
        anyhow::ensure!(pages.len() == 1 && pages[0].page_number == 1 && pages[0].item_count == 20);
        anyhow::ensure!(
            !state
                .indexers
                .search_request_is_terminal(SYSTEM_ACTOR_PUBLIC_ID, request)
                .await?
        );
        Ok(())
    }

    #[tokio::test]
    async fn torznab_wait_propagates_storage_failure() -> anyhow::Result<()> {
        let indexers = RecordingIndexers::default();
        indexers.set_search_terminal_result(Err(SearchRequestServiceError::new(
            SearchRequestServiceErrorKind::Storage,
        )));
        let state = indexer_test_state(Arc::new(indexers))?;
        let result = sealed_pages(&state, Uuid::new_v4(), 10).await;
        anyhow::ensure!(
            matches!(result, Err(error) if error.kind() == SearchRequestServiceErrorKind::Storage)
        );
        Ok(())
    }
}
