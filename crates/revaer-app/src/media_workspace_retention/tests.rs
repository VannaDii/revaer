use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use revaer_media_runtime::workspace::WorkspaceRetentionPolicy;

use super::*;

const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

struct StubRepository {
    snapshot: WorkspaceRetentionSnapshot,
}

#[async_trait]
impl WorkspaceRetentionRepository for StubRepository {
    async fn load_snapshot(&self) -> Result<WorkspaceRetentionSnapshot, WorkspaceRetentionError> {
        Ok(self.snapshot.clone())
    }
}

struct FixedClock(SystemTime);

impl WorkspaceRetentionClock for FixedClock {
    fn now(&self) -> SystemTime {
        self.0
    }
}

struct FailingCleaner;

impl WorkspaceCleaner for FailingCleaner {
    fn cleanup(
        &self,
        root: &Path,
        _active_job_keys: &[String],
        _now: SystemTime,
        _policy: WorkspaceRetentionPolicy,
    ) -> Result<WorkspaceCleanupReport, ManagedWorkspaceError> {
        Err(ManagedWorkspaceError::UnsafePath(root.to_path_buf()))
    }
}

fn private_root(parent: &Path) -> anyhow::Result<PathBuf> {
    let root = parent.join("workspaces");
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    Ok(root)
}

fn workspace_policy() -> WorkspaceRetentionPolicy {
    WorkspaceRetentionPolicy {
        workspace_max_age: Duration::from_hours(24),
        diagnostics_max_age: Duration::from_secs(30 * SECONDS_PER_DAY),
        max_entries_per_tick: 32,
    }
}

fn service(
    root: PathBuf,
    active_job_keys: Vec<String>,
    now: SystemTime,
) -> anyhow::Result<MediaWorkspaceRetentionService> {
    Ok(MediaWorkspaceRetentionService::with_dependencies(
        Arc::new(StubRepository {
            snapshot: WorkspaceRetentionSnapshot {
                active_job_keys,
                policy: workspace_policy(),
            },
        }),
        Arc::new(SystemWorkspaceCleaner),
        Arc::new(FixedClock(now)),
        Metrics::new()?,
        root,
    ))
}

#[tokio::test]
async fn startup_cleanup_removes_expired_inactive_workspace() -> anyhow::Result<()> {
    let parent = tempfile::tempdir()?;
    let root = private_root(parent.path())?;
    let stale = root.join("stale-job");
    fs::create_dir(&stale)?;
    let runtime = service(
        root,
        Vec::new(),
        SystemTime::now() + Duration::from_secs(2 * SECONDS_PER_DAY),
    )?;

    let report = runtime.run_once().await?;

    assert_eq!(report.removed, vec![stale.clone()]);
    assert!(!stale.exists());
    Ok(())
}

#[tokio::test]
async fn active_job_workspace_is_preserved_past_retention() -> anyhow::Result<()> {
    let parent = tempfile::tempdir()?;
    let root = private_root(parent.path())?;
    let active_key = "7b88867e-1382-4143-9c33-26d477019705";
    let active = root.join(active_key);
    fs::create_dir(&active)?;
    let runtime = service(
        root,
        vec![active_key.to_string()],
        SystemTime::now() + Duration::from_secs(365 * SECONDS_PER_DAY),
    )?;

    let report = runtime.run_once().await?;

    assert!(report.removed.is_empty());
    assert!(active.exists());
    Ok(())
}

#[tokio::test]
async fn failed_and_cancelled_diagnostics_expire_after_quarantine_window() -> anyhow::Result<()> {
    let parent = tempfile::tempdir()?;
    let root = private_root(parent.path())?;
    let failed = root.join("failed-job");
    let cancelled = root.join("cancelled-job");
    fs::create_dir_all(failed.join("diagnostics"))?;
    fs::create_dir_all(cancelled.join("diagnostics"))?;
    let runtime = service(
        root,
        Vec::new(),
        SystemTime::now() + Duration::from_secs(31 * SECONDS_PER_DAY),
    )?;

    let report = runtime.run_once().await?;

    assert_eq!(report.removed, vec![cancelled.clone(), failed.clone()]);
    assert!(!failed.exists());
    assert!(!cancelled.exists());
    Ok(())
}

#[test]
fn persisted_snapshot_requires_positive_consistent_bounds() {
    let rows = vec![MediaWorkspaceRetentionSnapshotRow {
        media_job_public_id: None,
        workspace_retention_seconds: 0,
        diagnostic_workspace_retention_seconds: 30,
        max_entries_per_tick: 1,
    }];

    assert!(matches!(
        snapshot_from_rows(rows),
        Err(WorkspaceRetentionError::InvalidPolicy(
            "workspace_retention_policy_zero_value"
        ))
    ));
}

#[tokio::test]
async fn root_cleanup_failure_is_observable_and_retryable() -> anyhow::Result<()> {
    let parent = tempfile::tempdir()?;
    let root = parent.path().join("failed-root");
    let runtime = MediaWorkspaceRetentionService::with_dependencies(
        Arc::new(StubRepository {
            snapshot: WorkspaceRetentionSnapshot {
                active_job_keys: Vec::new(),
                policy: workspace_policy(),
            },
        }),
        Arc::new(FailingCleaner),
        Arc::new(FixedClock(SystemTime::now())),
        Metrics::new()?,
        root,
    );

    assert!(matches!(
        runtime.run_once().await,
        Err(WorkspaceRetentionError::Filesystem(
            ManagedWorkspaceError::UnsafePath(_)
        ))
    ));
    assert!(
        runtime
            .telemetry
            .render()?
            .contains("media_workspace_cleanup_total{outcome=\"failed\"} 1")
    );
    Ok(())
}
