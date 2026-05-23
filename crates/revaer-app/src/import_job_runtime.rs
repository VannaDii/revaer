//! In-process import-job runtime.
//!
//! # Design
//! - Polls running import jobs and seals them to terminal states.
//! - Uses stored-procedure wrappers only for database access.
//! - Keeps processing deterministic and panic-free while richer adapters land.

use std::sync::Arc;
use std::time::Duration;

use revaer_config::ConfigService;
use revaer_data::DataError;
use revaer_data::indexers::import_jobs::{
    ClaimedImportJobRow, ImportJobWorkerResultInput, import_job_worker_claim_next,
    import_job_worker_mark_terminal, import_job_worker_record_result,
};
use revaer_telemetry::Metrics;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};

const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(1);
const MAX_IDENTIFIER_LEN: usize = 256;
const API_NOT_CONFIGURED_DETAIL: &str = "prowlarr_api_runtime_not_configured";
const BACKUP_RESULT_DETAIL: &str = "backup snapshot staged; secret binding reconciliation required";

pub(crate) struct ImportJobRuntime {
    config: Arc<ConfigService>,
    telemetry: Metrics,
    tick_interval: Duration,
}

impl ImportJobRuntime {
    pub(crate) const fn new(config: Arc<ConfigService>, telemetry: Metrics) -> Self {
        Self {
            config,
            telemetry,
            tick_interval: DEFAULT_TICK_INTERVAL,
        }
    }

    pub(crate) fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run_loop().await;
        })
    }

    async fn run_loop(self) {
        let mut ticker = interval(self.tick_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            if let Err(error) = self.run_tick().await {
                warn!(error = %error, "import job runtime tick failed");
            }
        }
    }

    async fn run_tick(&self) -> Result<(), DataError> {
        let claimed = import_job_worker_claim_next(self.config.pool()).await?;
        if let Some(job) = claimed {
            self.process_job(job).await;
        }
        Ok(())
    }

    async fn process_job(&self, job: ClaimedImportJobRow) {
        let source = job.source.as_str();
        let result = match source {
            "prowlarr_backup" => self.process_backup_job(&job).await,
            "prowlarr_api" => self.process_api_job(&job).await,
            _ => {
                self.mark_failed(&job, Some("unsupported_import_source"))
                    .await
            }
        };

        match result {
            Ok(()) => {
                self.telemetry
                    .inc_indexer_job_outcome("import_runtime", "success");
                info!(import_job_public_id = %job.import_job_public_id, source, "import job runtime processed job");
            }
            Err(error) => {
                self.telemetry
                    .inc_indexer_job_outcome("import_runtime", "error");
                warn!(import_job_public_id = %job.import_job_public_id, source, error = %error, "import job runtime failed job processing");
                if let Err(mark_error) = self
                    .mark_failed(&job, Some("runtime_processing_failure"))
                    .await
                {
                    warn!(
                        import_job_public_id = %job.import_job_public_id,
                        error = %mark_error,
                        "import job runtime failed to mark job failed"
                    );
                }
            }
        }
    }

    async fn process_backup_job(&self, job: &ClaimedImportJobRow) -> Result<(), DataError> {
        let backup_ref = parse_config_value(job.config_detail.as_deref(), "backup_blob_ref")
            .unwrap_or("backup")
            .trim();
        let identifier = truncated_identifier(backup_ref);

        import_job_worker_record_result(
            self.config.pool(),
            &ImportJobWorkerResultInput {
                import_job_public_id: job.import_job_public_id,
                prowlarr_identifier: &identifier,
                status: "imported_needs_secret",
                detail: Some(BACKUP_RESULT_DETAIL),
                resolved_is_enabled: Some(false),
                resolved_priority: Some(50),
                missing_secret_fields: 1,
            },
        )
        .await?;

        import_job_worker_mark_terminal(
            self.config.pool(),
            job.import_job_public_id,
            "completed",
            None,
        )
        .await
    }

    async fn process_api_job(&self, job: &ClaimedImportJobRow) -> Result<(), DataError> {
        import_job_worker_record_result(
            self.config.pool(),
            &ImportJobWorkerResultInput {
                import_job_public_id: job.import_job_public_id,
                prowlarr_identifier: "prowlarr-api",
                status: "imported_test_failed",
                detail: Some(API_NOT_CONFIGURED_DETAIL),
                resolved_is_enabled: None,
                resolved_priority: None,
                missing_secret_fields: 0,
            },
        )
        .await?;

        self.mark_failed(job, Some(API_NOT_CONFIGURED_DETAIL)).await
    }

    async fn mark_failed(
        &self,
        job: &ClaimedImportJobRow,
        error_detail: Option<&str>,
    ) -> Result<(), DataError> {
        import_job_worker_mark_terminal(
            self.config.pool(),
            job.import_job_public_id,
            "failed",
            error_detail,
        )
        .await
    }
}

fn parse_config_value<'a>(config_detail: Option<&'a str>, key: &str) -> Option<&'a str> {
    let value = config_detail?;
    for part in value.split(';') {
        let (entry_key, entry_value) = part.split_once('=')?;
        if entry_key.trim() == key {
            let trimmed = entry_value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

fn truncated_identifier(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "backup".to_string();
    }
    trimmed.chars().take(MAX_IDENTIFIER_LEN).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_value_reads_key_pairs() {
        let config = Some("prowlarr_url=http://localhost:9696;secret_public_id=abc");
        assert_eq!(
            parse_config_value(config, "prowlarr_url"),
            Some("http://localhost:9696")
        );
        assert_eq!(parse_config_value(config, "missing"), None);
    }

    #[test]
    fn truncated_identifier_clamps_length_and_handles_empty() {
        assert_eq!(truncated_identifier("  "), "backup");
        let long = "a".repeat(MAX_IDENTIFIER_LEN + 100);
        assert_eq!(truncated_identifier(&long).len(), MAX_IDENTIFIER_LEN);
    }
}
