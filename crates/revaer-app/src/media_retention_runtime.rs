//! In-process media retention janitor.
//!
//! The runtime injects persistence and time so policy execution is deterministic in tests while
//! production wiring remains a thin owner of concrete dependencies.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use revaer_data::DataError;
use revaer_data::media::jobs::MediaJobRetentionRunRow;
use revaer_runtime::media::MediaStore;
use revaer_telemetry::Metrics;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};

use crate::media_workspace_retention::{WorkspaceRetentionError, WorkspaceRetentionRunner};

const DEFAULT_RETENTION_TICK_INTERVAL: Duration = Duration::from_hours(1);

#[async_trait]
trait MediaRetentionRepository: Send + Sync {
    async fn run_retention(
        &self,
        as_of: DateTime<Utc>,
    ) -> Result<MediaJobRetentionRunRow, DataError>;
}

#[async_trait]
impl MediaRetentionRepository for MediaStore {
    async fn run_retention(
        &self,
        as_of: DateTime<Utc>,
    ) -> Result<MediaJobRetentionRunRow, DataError> {
        self.run_job_retention(as_of).await
    }
}

trait MediaRetentionClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

struct SystemMediaRetentionClock;

impl MediaRetentionClock for SystemMediaRetentionClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Background janitor that applies the persisted media retention policy.
pub(crate) struct MediaRetentionRuntime {
    repository: Arc<dyn MediaRetentionRepository>,
    clock: Arc<dyn MediaRetentionClock>,
    workspace_retention: Arc<dyn WorkspaceRetentionRunner>,
    telemetry: Metrics,
    tick_interval: Duration,
}

impl MediaRetentionRuntime {
    /// Construct the production janitor with injected persistence and telemetry.
    #[must_use]
    pub(crate) fn new(
        store: MediaStore,
        workspace_retention: Arc<dyn WorkspaceRetentionRunner>,
        telemetry: Metrics,
    ) -> Self {
        Self::with_dependencies(
            Arc::new(store),
            Arc::new(SystemMediaRetentionClock),
            workspace_retention,
            telemetry,
            DEFAULT_RETENTION_TICK_INTERVAL,
        )
    }

    fn with_dependencies(
        repository: Arc<dyn MediaRetentionRepository>,
        clock: Arc<dyn MediaRetentionClock>,
        workspace_retention: Arc<dyn WorkspaceRetentionRunner>,
        telemetry: Metrics,
        tick_interval: Duration,
    ) -> Self {
        Self {
            repository,
            clock,
            workspace_retention,
            telemetry,
            tick_interval,
        }
    }

    /// Spawn the janitor loop.
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
                self.telemetry.inc_media_retention_run("failed");
                warn!(error = %error, "media retention janitor tick failed");
            }
        }
    }

    async fn run_tick(
        &self,
    ) -> Result<Option<MediaJobRetentionRunRow>, MediaRetentionRuntimeError> {
        let workspace_report = self.workspace_retention.run_once().await?;
        if !workspace_report.failures.is_empty() {
            self.telemetry.inc_media_retention_run("deferred");
            for failure in workspace_report.failures {
                warn!(
                    path = %failure.path.display(),
                    operation = failure.operation,
                    error_kind = ?failure.error_kind,
                    "media retention deferred database pruning after workspace cleanup failure"
                );
            }
            return Ok(None);
        }

        let outcome = self.repository.run_retention(self.clock.now()).await?;
        let completed = nonnegative_count(outcome.completed_jobs_deleted)?;
        let failed = nonnegative_count(outcome.failed_jobs_pruned)?;
        let details = nonnegative_count(outcome.failed_detail_rows_deleted)?;

        self.telemetry.inc_media_retention_run("success");
        self.telemetry
            .add_media_retention_rows("completed_jobs", completed);
        self.telemetry
            .add_media_retention_rows("failed_jobs", failed);
        self.telemetry
            .add_media_retention_rows("failed_detail_rows", details);
        info!(
            completed_jobs_deleted = outcome.completed_jobs_deleted,
            failed_jobs_pruned = outcome.failed_jobs_pruned,
            failed_detail_rows_deleted = outcome.failed_detail_rows_deleted,
            "media retention janitor tick completed"
        );
        Ok(Some(outcome))
    }
}

fn nonnegative_count(value: i32) -> Result<u64, MediaRetentionRuntimeError> {
    u64::try_from(value).map_err(|_| MediaRetentionRuntimeError::NegativeCount(value))
}

#[derive(Debug, Error)]
enum MediaRetentionRuntimeError {
    #[error("media retention persistence failed")]
    Data(#[from] DataError),
    #[error("media retention returned a negative affected-row count: {0}")]
    NegativeCount(i32),
    #[error("media workspace retention failed")]
    Workspace(#[from] WorkspaceRetentionError),
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use revaer_media_runtime::workspace::{WorkspaceCleanupFailure, WorkspaceCleanupReport};
    use tokio::time::sleep;

    struct FixedClock(DateTime<Utc>);

    impl MediaRetentionClock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    struct StubRepository {
        expected: Result<MediaJobRetentionRunRow, ()>,
        observed: Mutex<Vec<DateTime<Utc>>>,
    }

    #[async_trait]
    impl MediaRetentionRepository for StubRepository {
        async fn run_retention(
            &self,
            as_of: DateTime<Utc>,
        ) -> Result<MediaJobRetentionRunRow, DataError> {
            self.observed
                .lock()
                .map_err(|_| DataError::from(sqlx::Error::Protocol("retention test lock".into())))?
                .push(as_of);
            self.expected
                .map_err(|()| DataError::from(sqlx::Error::RowNotFound))
        }
    }

    struct StubWorkspaceRetention {
        reports: Mutex<VecDeque<WorkspaceCleanupReport>>,
        runs: AtomicUsize,
    }

    #[async_trait]
    impl WorkspaceRetentionRunner for StubWorkspaceRetention {
        async fn run_once(&self) -> Result<WorkspaceCleanupReport, WorkspaceRetentionError> {
            self.runs.fetch_add(1, Ordering::Relaxed);
            self.reports
                .lock()
                .map_err(|_| WorkspaceRetentionError::InvalidPolicy("retention_test_lock"))?
                .pop_front()
                .ok_or(WorkspaceRetentionError::InvalidPolicy(
                    "retention_test_report_missing",
                ))
        }
    }

    fn successful_workspace_retention() -> Arc<StubWorkspaceRetention> {
        Arc::new(StubWorkspaceRetention {
            reports: Mutex::new(VecDeque::from([WorkspaceCleanupReport {
                examined_entries: 0,
                removed: Vec::new(),
                failures: Vec::new(),
                limit_reached: false,
            }])),
            runs: AtomicUsize::new(0),
        })
    }

    fn fixed_time() -> anyhow::Result<DateTime<Utc>> {
        "2026-07-20T12:00:00Z".parse().map_err(Into::into)
    }

    async fn wait_for_observations(
        repository: &StubRepository,
        expected_count: usize,
    ) -> anyhow::Result<()> {
        for _ in 0..100 {
            let count = repository
                .observed
                .lock()
                .map_err(|error| anyhow::anyhow!("retention test lock failed: {error}"))?
                .len();
            if count >= expected_count {
                return Ok(());
            }
            sleep(Duration::from_millis(10)).await;
        }
        Err(anyhow::anyhow!(
            "retention runtime did not record {expected_count} observations"
        ))
    }

    #[tokio::test]
    async fn tick_uses_injected_time_and_records_bounded_metrics() -> anyhow::Result<()> {
        let now = fixed_time()?;
        let repository = Arc::new(StubRepository {
            expected: Ok(MediaJobRetentionRunRow {
                completed_jobs_deleted: 2,
                failed_jobs_pruned: 3,
                failed_detail_rows_deleted: 8,
            }),
            observed: Mutex::new(Vec::new()),
        });
        let runtime = MediaRetentionRuntime::with_dependencies(
            repository.clone(),
            Arc::new(FixedClock(now)),
            successful_workspace_retention(),
            Metrics::new()?,
            Duration::from_secs(1),
        );

        let outcome = runtime
            .run_tick()
            .await?
            .ok_or_else(|| anyhow::anyhow!("database retention was unexpectedly deferred"))?;

        assert_eq!(outcome.completed_jobs_deleted, 2);
        assert_eq!(
            repository
                .observed
                .lock()
                .map_err(|error| { anyhow::anyhow!("retention test lock failed: {error}") })?
                .as_slice(),
            &[now]
        );
        let rendered = runtime.telemetry.render()?;
        assert!(rendered.contains("media_retention_runs_total{outcome=\"success\"} 1"));
        assert!(rendered.contains("media_retention_rows_total{category=\"failed_detail_rows\"} 8"));
        Ok(())
    }

    #[tokio::test]
    async fn tick_propagates_repository_failure_without_success_metric() -> anyhow::Result<()> {
        let runtime = MediaRetentionRuntime::with_dependencies(
            Arc::new(StubRepository {
                expected: Err(()),
                observed: Mutex::new(Vec::new()),
            }),
            Arc::new(FixedClock(fixed_time()?)),
            successful_workspace_retention(),
            Metrics::new()?,
            Duration::from_secs(1),
        );

        assert!(matches!(
            runtime.run_tick().await,
            Err(MediaRetentionRuntimeError::Data(_))
        ));
        assert!(
            !runtime
                .telemetry
                .render()?
                .contains("media_retention_runs_total")
        );
        Ok(())
    }

    #[tokio::test]
    async fn spawned_runtime_runs_immediately_after_restart() -> anyhow::Result<()> {
        let repository = Arc::new(StubRepository {
            expected: Ok(MediaJobRetentionRunRow {
                completed_jobs_deleted: 0,
                failed_jobs_pruned: 0,
                failed_detail_rows_deleted: 0,
            }),
            observed: Mutex::new(Vec::new()),
        });
        let telemetry = Metrics::new()?;
        let workspace_retention = Arc::new(StubWorkspaceRetention {
            reports: Mutex::new(VecDeque::from([
                WorkspaceCleanupReport {
                    examined_entries: 0,
                    removed: Vec::new(),
                    failures: Vec::new(),
                    limit_reached: false,
                },
                WorkspaceCleanupReport {
                    examined_entries: 0,
                    removed: Vec::new(),
                    failures: Vec::new(),
                    limit_reached: false,
                },
            ])),
            runs: AtomicUsize::new(0),
        });

        let first = MediaRetentionRuntime::with_dependencies(
            repository.clone(),
            Arc::new(FixedClock(fixed_time()?)),
            workspace_retention.clone(),
            telemetry.clone(),
            Duration::from_mins(1),
        )
        .spawn();
        wait_for_observations(&repository, 1).await?;
        first.abort();

        let restarted = MediaRetentionRuntime::with_dependencies(
            repository.clone(),
            Arc::new(FixedClock(fixed_time()?)),
            workspace_retention.clone(),
            telemetry,
            Duration::from_mins(1),
        )
        .spawn();
        wait_for_observations(&repository, 2).await?;
        restarted.abort();
        assert_eq!(workspace_retention.runs.load(Ordering::Relaxed), 2);
        Ok(())
    }

    #[tokio::test]
    async fn partial_workspace_failure_defers_database_pruning_until_retry() -> anyhow::Result<()> {
        let repository = Arc::new(StubRepository {
            expected: Ok(MediaJobRetentionRunRow {
                completed_jobs_deleted: 1,
                failed_jobs_pruned: 0,
                failed_detail_rows_deleted: 0,
            }),
            observed: Mutex::new(Vec::new()),
        });
        let workspace_retention = Arc::new(StubWorkspaceRetention {
            reports: Mutex::new(VecDeque::from([
                WorkspaceCleanupReport {
                    examined_entries: 2,
                    removed: vec![PathBuf::from("removed-job")],
                    failures: vec![WorkspaceCleanupFailure {
                        path: PathBuf::from("retry-job"),
                        operation: "workspace.cleanup_remove",
                        error_kind: io::ErrorKind::PermissionDenied,
                    }],
                    limit_reached: false,
                },
                WorkspaceCleanupReport {
                    examined_entries: 1,
                    removed: vec![PathBuf::from("retry-job")],
                    failures: Vec::new(),
                    limit_reached: false,
                },
            ])),
            runs: AtomicUsize::new(0),
        });
        let runtime = MediaRetentionRuntime::with_dependencies(
            repository.clone(),
            Arc::new(FixedClock(fixed_time()?)),
            workspace_retention,
            Metrics::new()?,
            Duration::from_secs(1),
        );

        assert!(runtime.run_tick().await?.is_none());
        assert!(
            repository
                .observed
                .lock()
                .map_err(|error| anyhow::anyhow!("retention test lock failed: {error}"))?
                .is_empty()
        );
        assert!(runtime.run_tick().await?.is_some());
        assert_eq!(
            repository
                .observed
                .lock()
                .map_err(|error| anyhow::anyhow!("retention test lock failed: {error}"))?
                .len(),
            1
        );
        assert!(
            runtime
                .telemetry
                .render()?
                .contains("media_retention_runs_total{outcome=\"deferred\"} 1")
        );
        Ok(())
    }

    #[test]
    fn negative_database_counts_are_rejected() {
        assert!(matches!(
            nonnegative_count(-1),
            Err(MediaRetentionRuntimeError::NegativeCount(-1))
        ));
    }
}
