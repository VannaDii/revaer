//! Scheduled recent-feed polling; each claimed write is awaited through settlement.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use revaer_config::ConfigService;
use revaer_data::DataError;
use revaer_data::indexers::manual_search::{
    ManualSearchConfigurationError, ManualSearchConnection, ManualSearchEngine, ManualSearchRouting,
};
use revaer_data::indexers::{cardigann, executor, normalization, rss_execution};

use crate::manual_search::torznab::{Failure, fetch_text};
use crate::rss_feed;
use crate::runtime_shutdown::{self, RuntimeShutdownReceiver};

pub(crate) type ClientFactory = fn(
    &Result<ManualSearchConnection, ManualSearchConfigurationError>,
    i32,
) -> Result<reqwest::Client, Failure>;

pub(crate) struct RssPollRuntime {
    config: Arc<ConfigService>,
    client_factory: ClientFactory,
}

impl RssPollRuntime {
    pub(crate) const fn new(config: Arc<ConfigService>, client_factory: ClientFactory) -> Self {
        Self {
            config,
            client_factory,
        }
    }

    pub(crate) async fn run_batch(
        &self,
        shutdown: &mut RuntimeShutdownReceiver,
    ) -> Result<(), DataError> {
        // Claim individually so shutdown never abandons an unstarted batch of leases.
        for _ in 0..25 {
            if runtime_shutdown::requested(shutdown) {
                break;
            }
            let Some(claim) = executor::rss_poll_claim(self.config.pool(), 1).await?.pop() else {
                break;
            };
            self.execute(&claim, shutdown).await?;
        }
        rss_execution::complete_job(self.config.pool()).await
    }

    async fn fetch(
        &self,
        context: &rss_execution::RssExecutionContext,
        claim: &executor::RssPollClaimRow,
        plan: Option<&cardigann::CardigannPlan>,
        shutdown: &mut RuntimeShutdownReceiver,
    ) -> Result<rss_feed::Listing, Failure> {
        let connection = context.connection.as_ref().map_err(|error| match error {
            ManualSearchConfigurationError::ReimportCardigann => {
                Failure::configuration(crate::cardigann::REIMPORT_REQUIRED)
            }
            ManualSearchConfigurationError::CredentialUnavailable => {
                Failure::classified("auth_error")
            }
            ManualSearchConfigurationError::MissingEndpoint
            | ManualSearchConfigurationError::UnsupportedEngine
            | ManualSearchConfigurationError::InvalidRoute => Failure::classified("unknown"),
        })?;
        let client = (self.client_factory)(&context.connection, claim.connect_timeout_ms)?;
        let request = async {
            match connection.engine {
                ManualSearchEngine::Torznab => {
                    let api_key = crate::manual_search::torznab::api_key(connection)?;
                    let url = rss_feed::request_url(&connection.endpoint, api_key)?;
                    let text = fetch_text(&client, url, claim.read_timeout_ms).await?;
                    let items = rss_feed::parse(&text, api_key)?;
                    Ok(rss_feed::Listing {
                        parsed_count: items.len(),
                        items,
                    })
                }
                ManualSearchEngine::Cardigann => {
                    let plan =
                        plan.ok_or(Failure::configuration(crate::cardigann::REIMPORT_REQUIRED))?;
                    let releases =
                        crate::cardigann::fetch_recent(&client, claim.read_timeout_ms, plan)
                            .await?;
                    Ok(rss_feed::html_listing(releases))
                }
            }
        };
        tokio::select! {
            result = request => result,
            () = runtime_shutdown::changed(shutdown) => Err(Failure::classified("timeout")),
        }
    }

    async fn execute(
        &self,
        claim: &executor::RssPollClaimRow,
        shutdown: &mut RuntimeShutdownReceiver,
    ) -> Result<(), DataError> {
        let started_at = Utc::now();
        let started = Instant::now();
        let context = rss_execution::resolve(
            self.config.pool(),
            claim.rss_subscription_id,
            claim.correlation_id,
        )
        .await?;
        let mut items = Vec::new();
        let mut input = executor::RssPollApplyInput {
            rss_subscription_id: claim.rss_subscription_id,
            correlation_id: claim.correlation_id,
            retry_seq: claim.retry_seq,
            started_at,
            finished_at: started_at,
            outcome: "failure",
            error_class: Some("timeout"),
            http_status: None,
            latency_ms: Some(0),
            parse_ok: false,
            result_count: None,
            via_mitigation: "none",
            rate_limit_denied_scope: None,
            cf_detected: false,
            cf_retryable: false,
            item_guid: None,
            infohash_v1: None,
            infohash_v2: None,
            magnet_hash: None,
        };
        if let Some(context) = &context {
            if let Ok(connection) = &context.connection
                && matches!(connection.routing, ManualSearchRouting::Proxy { .. })
            {
                input.via_mitigation = "proxy";
            }
            if let Some(scope) = context.rate_limit_denied_scope.as_deref() {
                input.error_class = Some("rate_limited");
                input.rate_limit_denied_scope = Some(scope);
            } else if !runtime_shutdown::requested(shutdown) {
                let plan = if context
                    .connection
                    .as_ref()
                    .is_ok_and(|connection| connection.engine == ManualSearchEngine::Cardigann)
                {
                    cardigann::load_rss(self.config.pool(), claim).await?
                } else {
                    None
                };
                let fetched = self.fetch(context, claim, plan.as_ref(), shutdown).await;
                match fetched {
                    Ok(parsed) => {
                        input.result_count =
                            Some(i32::try_from(parsed.parsed_count).unwrap_or(i32::MAX));
                        items = parsed.items;
                        input.outcome = "success";
                        input.error_class = None;
                        input.http_status = Some(200);
                        input.parse_ok = true;
                    }
                    Err(error) => {
                        if let Some(diagnostic) = error.diagnostic {
                            tracing::warn!(indexer_instance_public_id = %claim.indexer_instance_public_id, diagnostic, "RSS configuration requires operator action");
                        }
                        input.error_class = Some(error.class);
                        input.http_status = error.http_status;
                    }
                }
            }
        }
        let mut guids = Vec::with_capacity(items.len());
        let mut v1 = Vec::with_capacity(items.len());
        let mut v2 = Vec::with_capacity(items.len());
        let mut magnets = Vec::with_capacity(items.len());
        for item in items {
            let hash = normalization::derive_magnet_hash(
                self.config.pool(),
                (!item.infohash_v2.is_empty()).then_some(item.infohash_v2.as_str()),
                (!item.infohash_v1.is_empty()).then_some(item.infohash_v1.as_str()),
                item.magnet.as_deref(),
            )
            .await?;
            guids.push(item.guid);
            v1.push(item.infohash_v1);
            v2.push(item.infohash_v2);
            magnets.push(hash.unwrap_or_default());
        }
        input.item_guid = Some(&guids);
        input.infohash_v1 = Some(&v1);
        input.infohash_v2 = Some(&v2);
        input.magnet_hash = Some(&magnets);
        input.finished_at = Utc::now();
        input.latency_ms = Some(i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX));
        // No abort/select around the write owner. The procedure fences stale claims
        // and preserves operator changes before applying the existing ledger/policy.
        executor::rss_poll_finish(self.config.pool(), &input).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
