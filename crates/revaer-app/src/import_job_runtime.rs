//! In-process import-job runtime.
//!
//! # Design
//! - Polls running import jobs and seals them to terminal states.
//! - Uses stored-procedure wrappers only for database access.
//! - Keeps processing deterministic and panic-free while richer adapters land.

use std::sync::Arc;
use std::time::Duration;

use revaer_api::app::indexers::IndexerFacade;
use revaer_api::models::IndexerBackupSnapshot;
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
use uuid::Uuid;

use crate::indexers::IndexerService;

const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(1);
const MAX_IDENTIFIER_LEN: usize = 256;
const API_NOT_CONFIGURED_DETAIL: &str = "prowlarr_api_runtime_not_configured";
const API_CONFIG_MISSING_DETAIL: &str = "prowlarr_api_runtime_config_missing";
const BACKUP_RESULT_DETAIL: &str = "backup snapshot staged; secret binding reconciliation required";
const BACKUP_INLINE_PREFIX: &str = "inline-json:";
const BACKUP_INLINE_INVALID_DETAIL: &str = "backup_snapshot_inline_json_invalid";
const BACKUP_RESTORE_FAILED_DETAIL: &str = "backup_snapshot_restore_failed";

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

        if let Some(snapshot) = parse_inline_backup_snapshot(backup_ref) {
            return self
                .apply_inline_backup_snapshot(job, &identifier, snapshot)
                .await;
        }
        if backup_ref.starts_with(BACKUP_INLINE_PREFIX) {
            import_job_worker_record_result(
                self.config.pool(),
                &ImportJobWorkerResultInput {
                    import_job_public_id: job.import_job_public_id,
                    prowlarr_identifier: &identifier,
                    status: "imported_test_failed",
                    detail: Some(BACKUP_INLINE_INVALID_DETAIL),
                    resolved_is_enabled: None,
                    resolved_priority: None,
                    missing_secret_fields: 0,
                },
            )
            .await?;
            return self
                .mark_failed(job, Some(BACKUP_INLINE_INVALID_DETAIL))
                .await;
        }

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

    async fn apply_inline_backup_snapshot(
        &self,
        job: &ClaimedImportJobRow,
        identifier: &str,
        snapshot: IndexerBackupSnapshot,
    ) -> Result<(), DataError> {
        let service = IndexerService::new(Arc::clone(&self.config), self.telemetry.clone());
        let restore = service.indexer_backup_restore(Uuid::nil(), &snapshot).await;
        let restore = match restore {
            Ok(value) => value,
            Err(error) => {
                import_job_worker_record_result(
                    self.config.pool(),
                    &ImportJobWorkerResultInput {
                        import_job_public_id: job.import_job_public_id,
                        prowlarr_identifier: identifier,
                        status: "imported_test_failed",
                        detail: Some(BACKUP_RESTORE_FAILED_DETAIL),
                        resolved_is_enabled: None,
                        resolved_priority: None,
                        missing_secret_fields: 0,
                    },
                )
                .await?;
                return self.mark_failed(job, error.code()).await;
            }
        };

        let missing_secret_fields =
            i32::try_from(restore.unresolved_secret_bindings.len()).unwrap_or(i32::MAX);
        let status = if missing_secret_fields == 0 {
            "imported_ready"
        } else {
            "imported_needs_secret"
        };

        import_job_worker_record_result(
            self.config.pool(),
            &ImportJobWorkerResultInput {
                import_job_public_id: job.import_job_public_id,
                prowlarr_identifier: identifier,
                status,
                detail: Some("backup snapshot applied from inline payload"),
                resolved_is_enabled: None,
                resolved_priority: None,
                missing_secret_fields,
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
        let prowlarr_url = parse_config_value(job.config_detail.as_deref(), "prowlarr_url");
        let secret_public_id = parse_config_value(job.config_detail.as_deref(), "secret_public_id");

        if prowlarr_url.is_none() || secret_public_id.is_none() {
            import_job_worker_record_result(
                self.config.pool(),
                &ImportJobWorkerResultInput {
                    import_job_public_id: job.import_job_public_id,
                    prowlarr_identifier: "prowlarr-api",
                    status: "imported_test_failed",
                    detail: Some(API_CONFIG_MISSING_DETAIL),
                    resolved_is_enabled: None,
                    resolved_priority: None,
                    missing_secret_fields: 0,
                },
            )
            .await?;

            return self.mark_failed(job, Some(API_CONFIG_MISSING_DETAIL)).await;
        }

        let identifier = prowlarr_identifier_from_url(prowlarr_url.unwrap_or("prowlarr-api"));
        import_job_worker_record_result(
            self.config.pool(),
            &ImportJobWorkerResultInput {
                import_job_public_id: job.import_job_public_id,
                prowlarr_identifier: &identifier,
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

fn prowlarr_identifier_from_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "prowlarr-api".to_string();
    }
    let without_scheme = trimmed
        .split_once("://")
        .map_or(trimmed, |(_, remainder)| remainder);
    let no_path = without_scheme
        .split_once('/')
        .map_or(without_scheme, |(value, _)| value);
    let no_query = no_path.split_once('?').map_or(no_path, |(value, _)| value);
    let host = no_query
        .split_once('#')
        .map_or(no_query, |(value, _)| value);
    let candidate = host.trim();
    if candidate.is_empty() {
        "prowlarr-api".to_string()
    } else {
        truncated_identifier(candidate)
    }
}

fn parse_inline_backup_snapshot(input: &str) -> Option<IndexerBackupSnapshot> {
    let payload = input.strip_prefix(BACKUP_INLINE_PREFIX)?;
    serde_json::from_str::<IndexerBackupSnapshot>(payload).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use revaer_data::indexers::import_jobs::{
        import_job_create, import_job_get_status, import_job_list_results,
        import_job_run_prowlarr_backup,
    };
    use revaer_test_support::postgres::start_postgres;

    type TestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    async fn running_backup_job(config: &ConfigService, backup_ref: &str) -> TestResult<Uuid> {
        let job_id = import_job_create(
            config.pool(),
            Uuid::nil(),
            "prowlarr_backup",
            Some(false),
            None,
            None,
        )
        .await?;
        import_job_run_prowlarr_backup(config.pool(), job_id, backup_ref).await?;
        Ok(job_id)
    }

    fn claimed_job(
        import_job_public_id: Uuid,
        source: &str,
        config_detail: Option<&str>,
    ) -> ClaimedImportJobRow {
        ClaimedImportJobRow {
            import_job_public_id,
            source: source.to_string(),
            is_dry_run: false,
            config_detail: config_detail.map(str::to_string),
        }
    }

    #[test]
    fn parse_config_value_reads_key_pairs() {
        let config = Some("prowlarr_url=http://localhost:9696;secret_public_id=abc");
        assert_eq!(
            parse_config_value(config, "prowlarr_url"),
            Some("http://localhost:9696")
        );
        assert_eq!(parse_config_value(config, "missing"), None);
        assert_eq!(parse_config_value(None, "prowlarr_url"), None);
        assert_eq!(parse_config_value(Some("malformed"), "prowlarr_url"), None);
        assert_eq!(
            parse_config_value(Some("prowlarr_url=  "), "prowlarr_url"),
            None
        );
    }

    #[test]
    fn truncated_identifier_clamps_length_and_handles_empty() {
        assert_eq!(truncated_identifier("  "), "backup");
        let long = "a".repeat(MAX_IDENTIFIER_LEN + 100);
        assert_eq!(truncated_identifier(&long).len(), MAX_IDENTIFIER_LEN);
    }

    #[test]
    fn prowlarr_identifier_from_url_prefers_host_segment() {
        assert_eq!(
            prowlarr_identifier_from_url("https://prowlarr.example.test:9696/api?x=1"),
            "prowlarr.example.test:9696"
        );
        assert_eq!(prowlarr_identifier_from_url(""), "prowlarr-api");
        assert_eq!(
            prowlarr_identifier_from_url("https://?query"),
            "prowlarr-api"
        );
        assert_eq!(
            prowlarr_identifier_from_url("host.example#fragment"),
            "host.example"
        );
    }

    #[test]
    fn parse_inline_backup_snapshot_reads_prefixed_json() {
        let payload = r#"{"version":"1","exported_at":"2026-01-01T00:00:00Z","tags":[],"rate_limit_policies":[],"routing_policies":[],"indexer_instances":[],"secrets":[]}"#;
        let prefixed = format!("{BACKUP_INLINE_PREFIX}{payload}");
        let snapshot =
            parse_inline_backup_snapshot(&prefixed).expect("inline payload should decode snapshot");
        assert_eq!(snapshot.version, "1");
        assert!(snapshot.tags.is_empty());
    }

    #[test]
    fn parse_inline_backup_snapshot_rejects_invalid_payload() {
        assert!(parse_inline_backup_snapshot("backup").is_none());
        assert!(parse_inline_backup_snapshot("inline-json:{bad}").is_none());
    }

    async fn assert_backup_job_paths(
        runtime: &ImportJobRuntime,
        config: &ConfigService,
    ) -> TestResult<()> {
        let backup_id = running_backup_job(config, "snapshot-ref").await?;
        runtime.run_tick().await?;
        let backup_status = import_job_get_status(config.pool(), backup_id).await?;
        assert_eq!(backup_status.status, "completed");
        let backup_results = import_job_list_results(config.pool(), backup_id).await?;
        assert_eq!(backup_results.len(), 1);
        assert_eq!(backup_results[0].status, "imported_needs_secret");
        assert_eq!(backup_results[0].resolved_is_enabled, Some(false));

        let default_backup_id = running_backup_job(config, "ignored").await?;
        runtime
            .process_job(claimed_job(default_backup_id, "prowlarr_backup", None))
            .await;
        let default_results = import_job_list_results(config.pool(), default_backup_id).await?;
        assert_eq!(default_results[0].prowlarr_identifier, "backup");

        let invalid_inline_id = running_backup_job(config, "inline-json:{bad}").await?;
        runtime.run_tick().await?;
        let invalid_inline_status = import_job_get_status(config.pool(), invalid_inline_id).await?;
        assert_eq!(invalid_inline_status.status, "failed");
        let invalid_inline_results =
            import_job_list_results(config.pool(), invalid_inline_id).await?;
        assert_eq!(
            invalid_inline_results[0].detail.as_deref(),
            Some(BACKUP_INLINE_INVALID_DETAIL)
        );

        let empty_snapshot = r#"inline-json:{"version":"1","exported_at":"2026-01-01T00:00:00Z","tags":[],"rate_limit_policies":[],"routing_policies":[],"indexer_instances":[],"secrets":[]}"#;
        let inline_success_id = running_backup_job(config, empty_snapshot).await?;
        runtime.run_tick().await?;
        let inline_success_status = import_job_get_status(config.pool(), inline_success_id).await?;
        assert_eq!(inline_success_status.status, "completed");
        let inline_success_results =
            import_job_list_results(config.pool(), inline_success_id).await?;
        assert_eq!(inline_success_results[0].status, "imported_ready");
        assert_eq!(inline_success_results[0].missing_secret_fields, 0);

        let failing_snapshot = r#"inline-json:{"version":"1","exported_at":"2026-01-01T00:00:00Z","tags":[],"rate_limit_policies":[],"routing_policies":[{"display_name":"missing-rate-policy","mode":"http_proxy","rate_limit_display_name":"absent","parameters":[]}],"indexer_instances":[],"secrets":[]}"#;
        let inline_failure_id = running_backup_job(config, failing_snapshot).await?;
        runtime.run_tick().await?;
        let inline_failure_status = import_job_get_status(config.pool(), inline_failure_id).await?;
        assert_eq!(inline_failure_status.status, "failed");
        let inline_failure_results =
            import_job_list_results(config.pool(), inline_failure_id).await?;
        assert_eq!(
            inline_failure_results[0].detail.as_deref(),
            Some(BACKUP_RESTORE_FAILED_DETAIL)
        );
        Ok(())
    }

    async fn assert_rejected_job_paths(
        runtime: &ImportJobRuntime,
        config: &ConfigService,
    ) -> TestResult<()> {
        let api_missing_id = running_backup_job(config, "ignored").await?;
        runtime
            .process_job(claimed_job(api_missing_id, "prowlarr_api", None))
            .await;
        let api_missing_results = import_job_list_results(config.pool(), api_missing_id).await?;
        assert_eq!(
            api_missing_results[0].detail.as_deref(),
            Some(API_CONFIG_MISSING_DETAIL)
        );

        let api_configured_id = running_backup_job(config, "ignored").await?;
        runtime
            .process_job(claimed_job(
                api_configured_id,
                "prowlarr_api",
                Some("prowlarr_url=https://prowlarr.example.test/api;secret_public_id=secret"),
            ))
            .await;
        let api_configured_results =
            import_job_list_results(config.pool(), api_configured_id).await?;
        assert_eq!(
            api_configured_results[0].prowlarr_identifier,
            "prowlarr.example.test"
        );
        assert_eq!(
            api_configured_results[0].detail.as_deref(),
            Some(API_NOT_CONFIGURED_DETAIL)
        );

        let unsupported_id = running_backup_job(config, "ignored").await?;
        runtime
            .process_job(claimed_job(unsupported_id, "unsupported", None))
            .await;
        let unsupported_status = import_job_get_status(config.pool(), unsupported_id).await?;
        assert_eq!(unsupported_status.status, "failed");
        assert!(
            import_job_list_results(config.pool(), unsupported_id)
                .await?
                .is_empty()
        );

        runtime
            .process_job(claimed_job(
                Uuid::new_v4(),
                "prowlarr_backup",
                Some("backup_blob_ref=missing-job"),
            ))
            .await;
        Ok(())
    }

    #[tokio::test]
    async fn runtime_processes_supported_and_rejected_import_jobs() -> TestResult<()> {
        let Ok(postgres) = start_postgres() else {
            return Ok(());
        };
        let config = Arc::new(ConfigService::new(postgres.connection_string().to_string()).await?);
        let telemetry = Metrics::new()?;
        let runtime = ImportJobRuntime::new(Arc::clone(&config), telemetry.clone());

        runtime.run_tick().await?;
        assert_backup_job_paths(&runtime, &config).await?;
        assert_rejected_job_paths(&runtime, &config).await?;

        config.pool().close().await;
        let handle = ImportJobRuntime::new(config, telemetry).spawn();
        tokio::time::sleep(Duration::from_millis(20)).await;
        handle.abort();
        assert!(handle.await.is_err());
        Ok(())
    }
}
