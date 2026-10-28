//! Owned manual-search execution, using the normal configuration and stored procedures.

use std::sync::Arc;
use std::time::{Duration, Instant};

use revaer_config::ConfigService;
use revaer_data::DataError;
use revaer_data::indexers::manual_search::{
    self as store, ManualSearchCompletion, ManualSearchEngine, ManualSearchOutcome,
    ManualSearchRun, ManualSearchWriteOutcome,
};
use revaer_data::indexers::search_results::SearchResultIngestInput;
use tokio::task::JoinHandle;

use crate::runtime_shutdown::{self, RuntimeShutdownReceiver};

pub(crate) mod torznab;

/// Concrete client construction belongs to bootstrap; runtime receives the factory.
pub(crate) type ClientFactory = fn(&ManualSearchRun) -> Result<reqwest::Client, torznab::Failure>;

pub(crate) struct ManualSearchRuntime {
    config: Arc<ConfigService>,
    client_factory: ClientFactory,
}

impl ManualSearchRuntime {
    pub(crate) fn new(config: Arc<ConfigService>, client_factory: ClientFactory) -> Self {
        Self {
            config,
            client_factory,
        }
    }

    pub(crate) fn spawn(
        self,
        mut shutdown: RuntimeShutdownReceiver,
    ) -> JoinHandle<Result<(), DataError>> {
        tokio::spawn(async move {
            // A single in-process dispatcher stays below both configured concurrency ceilings.
            // Claims and shared token buckets still fence multiple application processes.
            while !runtime_shutdown::requested(&shutdown) {
                if let Some(run) = store::claim_next(self.config.pool()).await? {
                    self.execute(run, &mut shutdown).await?;
                } else if runtime_shutdown::sleep_or_requested(
                    Duration::from_millis(100),
                    &mut shutdown,
                )
                .await
                {
                    break;
                }
            }
            Ok(())
        })
    }

    async fn execute(
        &self,
        run: ManualSearchRun,
        shutdown: &mut RuntimeShutdownReceiver,
    ) -> Result<(), DataError> {
        let started = Instant::now();
        let plan = if run
            .connection
            .as_ref()
            .is_ok_and(|connection| connection.engine == ManualSearchEngine::Cardigann)
        {
            revaer_data::indexers::cardigann::load(self.config.pool(), &run).await?
        } else {
            None
        };
        let fetched = if runtime_shutdown::requested(shutdown) {
            None
        } else {
            match run
                .connection
                .as_ref()
                .map_err(|error| torznab::Failure::configuration(error.action()))
                .and_then(|_| (self.client_factory)(&run))
            {
                Err(failure) => Some(Err(failure)),
                Ok(client) => {
                    let fetching = async {
                        match run
                            .connection
                            .as_ref()
                            .map_err(|error| torznab::Failure::configuration(error.action()))?
                            .engine
                        {
                            ManualSearchEngine::Torznab => torznab::fetch(&client, &run).await,
                            ManualSearchEngine::Cardigann => {
                                let plan = plan.as_ref().ok_or(torznab::Failure::configuration(
                                    crate::cardigann::REIMPORT_REQUIRED,
                                ))?;
                                crate::cardigann::fetch(&client, &run, plan).await
                            }
                        }
                    };
                    tokio::pin!(fetching);
                    loop {
                        tokio::select! {
                            response = &mut fetching => break Some(response),
                            () = runtime_shutdown::changed(shutdown) => break None,
                            () = tokio::time::sleep(Duration::from_millis(100)) => {
                                // Await each database operation; cancellation only drops HTTP work.
                                if !store::is_active(self.config.pool(), &run).await? { break None; }
                            }
                        }
                    }
                }
            }
        };
        let mut completion = ManualSearchCompletion {
            diagnostic: None,
            outcome: ManualSearchOutcome::Canceled,
            http_status: None,
            latency_ms: i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX),
            result_count: 0,
            retry_after_seconds: None,
        };
        match fetched {
            None => {}
            Some(Err(failure)) => {
                completion.diagnostic = failure.diagnostic;
                completion.outcome = ManualSearchOutcome::Failure(failure.class);
                completion.http_status = failure.http_status;
                completion.retry_after_seconds = failure.retry_after_seconds;
            }
            Some(Ok(releases)) => {
                completion.outcome = ManualSearchOutcome::Success;
                completion.http_status = Some(200);
                completion.result_count = i32::try_from(releases.len()).unwrap_or(i32::MAX);
                for release in &releases {
                    if runtime_shutdown::requested(shutdown) {
                        completion.outcome = ManualSearchOutcome::Canceled;
                        break;
                    }
                    match self.ingest(&run, release).await {
                        Ok(ManualSearchWriteOutcome::Applied) => {}
                        Ok(ManualSearchWriteOutcome::OwnershipLost) => {
                            completion.outcome = ManualSearchOutcome::Canceled;
                            break;
                        }
                        Err(error)
                            if matches!(&error, DataError::QueryFailed { .. })
                                && error.database_code().as_deref() == Some("22001") =>
                        {
                            // The authoritative schema rejected an upstream field's length.
                            // The failed statement has rolled back; settle through normal retry.
                            completion.outcome = ManualSearchOutcome::Failure("parse_error");
                            break;
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        }
        completion.latency_ms = i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX);
        // Bootstrap awaits this owner; no abort or select may interrupt its commit/rollback.
        match store::complete(self.config.pool(), &run, completion).await? {
            ManualSearchWriteOutcome::Applied | ManualSearchWriteOutcome::OwnershipLost => Ok(()),
        }
    }

    async fn ingest(
        &self,
        run: &ManualSearchRun,
        release: &torznab::Release,
    ) -> Result<ManualSearchWriteOutcome, DataError> {
        let category = release.tracker_category.map(|value| [Some(value)]);
        let keys = ["tracker_category".to_string()];
        let types = ["int".to_string()];
        let empty_text = [None::<String>];
        let empty_bigint = [None::<i64>];
        let empty_bool = [None::<bool>];
        let empty_uuid = [None::<uuid::Uuid>];
        store::ingest(
            self.config.pool(),
            run,
            &SearchResultIngestInput {
                search_request_public_id: run.search_request_public_id,
                indexer_instance_public_id: run.indexer_instance_public_id,
                source_guid: release.guid.as_deref(),
                details_url: release.details_url.as_deref(),
                download_url: release.download_url.as_deref(),
                magnet_uri: release.magnet.as_deref(),
                title_raw: &release.title,
                size_bytes: Some(release.size),
                infohash_v1: release.infohash.as_deref(),
                infohash_v2: None,
                magnet_hash: None,
                seeders: release.seeders,
                leechers: release.leechers,
                published_at: release.published_at,
                uploader: None,
                observed_at: chrono::Utc::now(),
                attr_keys: category.as_ref().map(|_| keys.as_slice()),
                attr_types: category.as_ref().map(|_| types.as_slice()),
                attr_value_text: category.as_ref().map(|_| empty_text.as_slice()),
                attr_value_int: category.as_ref().map(<[Option<i32>; 1]>::as_slice),
                attr_value_bigint: category.as_ref().map(|_| empty_bigint.as_slice()),
                attr_value_numeric: category.as_ref().map(|_| empty_text.as_slice()),
                attr_value_bool: category.as_ref().map(|_| empty_bool.as_slice()),
                attr_value_uuid: category.as_ref().map(|_| empty_uuid.as_slice()),
            },
            release.privacy,
        )
        .await
    }
}

#[cfg(test)]
mod tests;
