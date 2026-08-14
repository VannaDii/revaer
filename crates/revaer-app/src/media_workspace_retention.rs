//! Persisted, bounded media-workspace retention service.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use revaer_data::DataError;
use revaer_data::media::jobs::MediaWorkspaceRetentionSnapshotRow;
use revaer_media_runtime::workspace::{
    ManagedWorkspaceError, WorkspaceCleanupReport, WorkspaceRetentionPolicy,
    cleanup_stale_workspaces_bounded,
};
use revaer_runtime::media::MediaStore;
use revaer_telemetry::Metrics;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceRetentionSnapshot {
    active_job_keys: Vec<String>,
    policy: WorkspaceRetentionPolicy,
}

#[async_trait]
trait WorkspaceRetentionRepository: Send + Sync {
    async fn load_snapshot(&self) -> Result<WorkspaceRetentionSnapshot, WorkspaceRetentionError>;
}

#[async_trait]
impl WorkspaceRetentionRepository for MediaStore {
    async fn load_snapshot(&self) -> Result<WorkspaceRetentionSnapshot, WorkspaceRetentionError> {
        snapshot_from_rows(self.load_workspace_retention_snapshot().await?)
    }
}

trait WorkspaceCleaner: Send + Sync {
    fn cleanup(
        &self,
        root: &Path,
        active_job_keys: &[String],
        now: SystemTime,
        policy: WorkspaceRetentionPolicy,
    ) -> Result<WorkspaceCleanupReport, ManagedWorkspaceError>;
}

struct SystemWorkspaceCleaner;

impl WorkspaceCleaner for SystemWorkspaceCleaner {
    fn cleanup(
        &self,
        root: &Path,
        active_job_keys: &[String],
        now: SystemTime,
        policy: WorkspaceRetentionPolicy,
    ) -> Result<WorkspaceCleanupReport, ManagedWorkspaceError> {
        cleanup_stale_workspaces_bounded(root, active_job_keys, now, policy)
    }
}

trait WorkspaceRetentionClock: Send + Sync {
    fn now(&self) -> SystemTime;
}

struct SystemWorkspaceRetentionClock;

impl WorkspaceRetentionClock for SystemWorkspaceRetentionClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[async_trait]
pub(crate) trait WorkspaceRetentionRunner: Send + Sync {
    async fn run_once(&self) -> Result<WorkspaceCleanupReport, WorkspaceRetentionError>;
}

/// Production service that combines persisted active-job state with bounded filesystem cleanup.
pub(crate) struct MediaWorkspaceRetentionService {
    repository: Arc<dyn WorkspaceRetentionRepository>,
    cleaner: Arc<dyn WorkspaceCleaner>,
    clock: Arc<dyn WorkspaceRetentionClock>,
    telemetry: Metrics,
    workspace_root: PathBuf,
}

impl MediaWorkspaceRetentionService {
    /// Construct the production workspace-retention service.
    #[must_use]
    pub(crate) fn new(store: MediaStore, telemetry: Metrics, workspace_root: PathBuf) -> Self {
        Self::with_dependencies(
            Arc::new(store),
            Arc::new(SystemWorkspaceCleaner),
            Arc::new(SystemWorkspaceRetentionClock),
            telemetry,
            workspace_root,
        )
    }

    fn with_dependencies(
        repository: Arc<dyn WorkspaceRetentionRepository>,
        cleaner: Arc<dyn WorkspaceCleaner>,
        clock: Arc<dyn WorkspaceRetentionClock>,
        telemetry: Metrics,
        workspace_root: PathBuf,
    ) -> Self {
        Self {
            repository,
            cleaner,
            clock,
            telemetry,
            workspace_root,
        }
    }
}

#[async_trait]
impl WorkspaceRetentionRunner for MediaWorkspaceRetentionService {
    async fn run_once(&self) -> Result<WorkspaceCleanupReport, WorkspaceRetentionError> {
        let snapshot = self.repository.load_snapshot().await?;
        let cleaner = Arc::clone(&self.cleaner);
        let workspace_root = self.workspace_root.clone();
        let now = self.clock.now();
        let cleanup_result = tokio::task::spawn_blocking(move || {
            cleaner.cleanup(
                &workspace_root,
                &snapshot.active_job_keys,
                now,
                snapshot.policy,
            )
        })
        .await
        .map_err(|error| WorkspaceRetentionError::Join(error.to_string()))?;
        let report = match cleanup_result {
            Ok(report) => report,
            Err(error) => {
                self.telemetry.inc_media_workspace_cleanup("failed");
                return Err(error.into());
            }
        };

        let outcome = if report.failures.is_empty() {
            "success"
        } else {
            "partial_failure"
        };
        self.telemetry.inc_media_workspace_cleanup(outcome);
        self.telemetry.add_media_retention_rows(
            "workspace_directories",
            u64::try_from(report.removed.len()).map_err(|_| {
                WorkspaceRetentionError::InvalidPolicy("workspace_removed_count_overflow")
            })?,
        );
        info!(
            examined_entries = report.examined_entries,
            removed_entries = report.removed.len(),
            failed_entries = report.failures.len(),
            limit_reached = report.limit_reached,
            "media workspace retention tick completed"
        );
        Ok(report)
    }
}

fn snapshot_from_rows(
    rows: Vec<MediaWorkspaceRetentionSnapshotRow>,
) -> Result<WorkspaceRetentionSnapshot, WorkspaceRetentionError> {
    let first = rows.first().ok_or(WorkspaceRetentionError::MissingPolicy)?;
    if rows.iter().any(|row| {
        row.workspace_retention_seconds != first.workspace_retention_seconds
            || row.diagnostic_workspace_retention_seconds
                != first.diagnostic_workspace_retention_seconds
            || row.max_entries_per_tick != first.max_entries_per_tick
    }) {
        return Err(WorkspaceRetentionError::InconsistentPolicy);
    }

    let workspace_seconds = u64::try_from(first.workspace_retention_seconds)
        .map_err(|_| WorkspaceRetentionError::InvalidPolicy("workspace_retention_seconds"))?;
    let diagnostics_seconds =
        u64::try_from(first.diagnostic_workspace_retention_seconds).map_err(|_| {
            WorkspaceRetentionError::InvalidPolicy("diagnostic_workspace_retention_seconds")
        })?;
    let max_entries_per_tick = usize::try_from(first.max_entries_per_tick)
        .map_err(|_| WorkspaceRetentionError::InvalidPolicy("max_entries_per_tick"))?;
    if workspace_seconds == 0 || diagnostics_seconds == 0 || max_entries_per_tick == 0 {
        return Err(WorkspaceRetentionError::InvalidPolicy(
            "workspace_retention_policy_zero_value",
        ));
    }

    Ok(WorkspaceRetentionSnapshot {
        active_job_keys: rows
            .into_iter()
            .filter_map(|row| row.media_job_public_id.map(|id| id.to_string()))
            .collect(),
        policy: WorkspaceRetentionPolicy {
            workspace_max_age: Duration::from_secs(workspace_seconds),
            diagnostics_max_age: Duration::from_secs(diagnostics_seconds),
            max_entries_per_tick,
        },
    })
}

#[derive(Debug, Error)]
pub(crate) enum WorkspaceRetentionError {
    #[error("workspace retention persistence failed")]
    Data(#[from] DataError),
    #[error("workspace retention policy is missing")]
    MissingPolicy,
    #[error("workspace retention policy rows are inconsistent")]
    InconsistentPolicy,
    #[error("workspace retention policy is invalid: {0}")]
    InvalidPolicy(&'static str),
    #[error("workspace retention filesystem task failed to join: {0}")]
    Join(String),
    #[error("workspace retention filesystem cleanup failed")]
    Filesystem(#[from] ManagedWorkspaceError),
}

#[cfg(test)]
mod tests;
