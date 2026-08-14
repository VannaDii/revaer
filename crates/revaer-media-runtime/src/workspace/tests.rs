use super::{
    ManagedWorkspaceError, TerminalWorkspaceCleanupPolicy, TerminalWorkspaceState, WorkspaceError,
    WorkspacePolicy, WorkspaceRejectionReason, WorkspaceRetentionPolicy, cleanup_stale_workspaces,
    cleanup_stale_workspaces_bounded, cleanup_terminal_workspace, create_managed_workspace,
    teardown_managed_workspace,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "revaer-media-runtime-workspace-{}-{counter}",
        std::process::id()
    ));
    fs::create_dir_all(&root)?;
    Ok(root)
}

#[test]
fn reserve_check_rejects_low_free_space() {
    let policy = WorkspacePolicy {
        max_bytes: 1024,
        reserve_bytes: 512,
    };
    assert_eq!(
        policy.ensure_reserve(256),
        Err(WorkspaceError::InsufficientReserve)
    );
}

#[test]
fn policy_validation_rejects_reserve_above_max() {
    let policy = WorkspacePolicy {
        max_bytes: 512,
        reserve_bytes: 1024,
    };

    assert_eq!(policy.validate(), Err(WorkspaceError::InvalidPolicy));
    assert_eq!(
        policy.ensure_capacity(2048, 256),
        Err(WorkspaceError::InvalidPolicy)
    );
}

#[test]
fn capacity_check_rejects_excess_demand() {
    let policy = WorkspacePolicy {
        max_bytes: 10_000,
        reserve_bytes: 4_000,
    };
    assert_eq!(
        policy.ensure_capacity(8_000, 4_100),
        Err(WorkspaceError::InsufficientCapacity)
    );
}

#[test]
fn capacity_check_accepts_fit_above_reserve() {
    let policy = WorkspacePolicy {
        max_bytes: 10_000,
        reserve_bytes: 4_000,
    };
    assert!(policy.ensure_capacity(8_000, 4_000).is_ok());
}

#[test]
fn capacity_check_rejects_when_required_exceeds_workspace_max() {
    let policy = WorkspacePolicy {
        max_bytes: 6_000,
        reserve_bytes: 1_000,
    };
    assert_eq!(
        policy.ensure_capacity(20_000, 6_001),
        Err(WorkspaceError::ExceedsMaxWorkspace)
    );
}

#[test]
fn evaluate_capacity_reports_acceptance_and_budget() {
    let policy = WorkspacePolicy {
        max_bytes: 10_000,
        reserve_bytes: 2_000,
    };
    let report = policy.evaluate_capacity(9_000, 5_000);
    assert!(report.accepted);
    assert_eq!(report.reason, None);
    assert_eq!(report.available_after_reserve_bytes, 7_000);
    assert_eq!(report.required_workspace_bytes, 5_000);
}

#[test]
fn evaluate_capacity_reports_rejection_reason() {
    let policy = WorkspacePolicy {
        max_bytes: 10_000,
        reserve_bytes: 2_000,
    };
    let report = policy.evaluate_capacity(3_000, 9_000);
    assert!(!report.accepted);
    assert_eq!(
        report.reason,
        Some(WorkspaceRejectionReason::InsufficientCapacity)
    );
    assert_eq!(report.available_after_reserve_bytes, 1_000);
}

#[test]
fn evaluate_capacity_classifies_each_precondition_failure() {
    let invalid = WorkspacePolicy {
        max_bytes: 1_000,
        reserve_bytes: 2_000,
    }
    .evaluate_capacity(1_500, 500);
    assert_eq!(
        invalid.reason,
        Some(WorkspaceRejectionReason::InvalidPolicy)
    );
    assert_eq!(invalid.available_after_reserve_bytes, 0);

    let insufficient_reserve = WorkspacePolicy {
        max_bytes: 2_000,
        reserve_bytes: 1_000,
    }
    .evaluate_capacity(999, 1);
    assert_eq!(
        insufficient_reserve.reason,
        Some(WorkspaceRejectionReason::InsufficientReserve)
    );
    assert_eq!(insufficient_reserve.available_after_reserve_bytes, 0);

    let exceeds_max = WorkspacePolicy {
        max_bytes: 2_000,
        reserve_bytes: 1_000,
    }
    .evaluate_capacity(4_000, 2_001);
    assert_eq!(
        exceeds_max.reason,
        Some(WorkspaceRejectionReason::ExceedsMaxWorkspace)
    );
    assert_eq!(exceeds_max.available_after_reserve_bytes, 3_000);
}

#[test]
fn managed_workspace_creation_and_teardown_uses_job_scoped_dirs()
-> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    let workspace = create_managed_workspace(&root, "job-1")?;

    assert_eq!(workspace.job_key, "job-1");
    assert!(workspace.job_path.exists());
    assert!(workspace.input_path.exists());
    assert!(workspace.output_path.exists());
    assert!(workspace.diagnostics_path.exists());

    teardown_managed_workspace(&workspace)?;
    assert!(!workspace.job_path.exists());
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn managed_workspace_rejects_empty_root() {
    assert!(matches!(
        create_managed_workspace("", "job-1"),
        Err(ManagedWorkspaceError::EmptyRoot)
    ));
}

#[test]
fn managed_workspace_rejects_empty_and_path_like_job_keys() -> Result<(), Box<dyn std::error::Error>>
{
    let root = temp_workspace_root()?;

    assert!(matches!(
        create_managed_workspace(&root, "  "),
        Err(ManagedWorkspaceError::EmptyJobKey)
    ));
    assert!(matches!(
        create_managed_workspace(&root, "bad/key"),
        Err(ManagedWorkspaceError::InvalidJobKey)
    ));
    assert!(matches!(
        create_managed_workspace(&root, ".."),
        Err(ManagedWorkspaceError::InvalidJobKey)
    ));

    fs::remove_dir_all(root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn managed_workspace_rejects_preplaced_job_symlink() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;

    let root = temp_workspace_root()?;
    let outside = temp_workspace_root()?;
    let job_path = root.join("job-link");
    symlink(&outside, &job_path)?;

    let result = create_managed_workspace(&root, "job-link");
    assert!(matches!(result, Err(ManagedWorkspaceError::UnsafePath(path)) if path == job_path));
    assert!(!outside.join("input").exists());

    fs::remove_file(job_path)?;
    fs::remove_dir_all(root)?;
    fs::remove_dir_all(outside)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn managed_workspace_rejects_preplaced_child_symlink() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;

    let root = temp_workspace_root()?;
    let outside = temp_workspace_root()?;
    let job_path = root.join("job-child-link");
    fs::create_dir(&job_path)?;
    let input_path = job_path.join("input");
    symlink(&outside, &input_path)?;

    let result = create_managed_workspace(&root, "job-child-link");
    assert!(matches!(result, Err(ManagedWorkspaceError::UnsafePath(path)) if path == input_path));

    fs::remove_file(input_path)?;
    fs::remove_dir_all(root)?;
    fs::remove_dir_all(outside)?;
    Ok(())
}

#[test]
fn stale_workspace_janitor_removes_only_inactive_directories()
-> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    let active = root.join("active-job");
    let stale = root.join("stale-job");
    fs::create_dir_all(&active)?;
    fs::create_dir_all(&stale)?;
    fs::write(root.join("not-a-workspace"), b"skip")?;

    let removed = cleanup_stale_workspaces(
        &root,
        &["active-job".to_string()],
        SystemTime::now(),
        Duration::ZERO,
    )?;

    assert_eq!(removed, vec![stale]);
    assert!(active.exists());
    assert!(!root.join("stale-job").exists());
    assert!(root.join("not-a-workspace").exists());
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn stale_workspace_cleanup_missing_root_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    fs::remove_dir_all(&root)?;

    let removed = cleanup_stale_workspaces(&root, &[], SystemTime::now(), Duration::ZERO)?;

    assert!(removed.is_empty());
    Ok(())
}

#[test]
fn bounded_janitor_rejects_zero_budget_and_handles_missing_root()
-> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    fs::remove_dir_all(&root)?;

    let invalid = cleanup_stale_workspaces_bounded(
        &root,
        &[],
        SystemTime::now(),
        WorkspaceRetentionPolicy {
            workspace_max_age: Duration::ZERO,
            diagnostics_max_age: Duration::ZERO,
            max_entries_per_tick: 0,
        },
    );
    assert!(matches!(
        invalid,
        Err(ManagedWorkspaceError::InvalidCleanupPolicy)
    ));

    let report = cleanup_stale_workspaces_bounded(
        &root,
        &[],
        SystemTime::now(),
        WorkspaceRetentionPolicy {
            workspace_max_age: Duration::ZERO,
            diagnostics_max_age: Duration::ZERO,
            max_entries_per_tick: 1,
        },
    )?;
    assert_eq!(report.examined_entries, 0);
    assert!(report.removed.is_empty());
    assert!(report.failures.is_empty());
    assert!(!report.limit_reached);
    Ok(())
}

#[cfg(unix)]
#[test]
fn managed_workspace_and_janitors_report_inaccessible_parent_paths() {
    let inaccessible = PathBuf::from("/dev/null/revaer-workspace");

    assert!(matches!(
        create_managed_workspace(&inaccessible, "job"),
        Err(ManagedWorkspaceError::Io {
            operation: "workspace.create_root",
            path,
            ..
        }) if path == inaccessible
    ));
    assert!(matches!(
        cleanup_stale_workspaces(&inaccessible, &[], SystemTime::now(), Duration::ZERO),
        Err(ManagedWorkspaceError::Io {
            operation: "workspace.cleanup_root_exists",
            path,
            ..
        }) if path == inaccessible
    ));
    assert!(matches!(
        cleanup_stale_workspaces_bounded(
            &inaccessible,
            &[],
            SystemTime::now(),
            WorkspaceRetentionPolicy {
                workspace_max_age: Duration::ZERO,
                diagnostics_max_age: Duration::ZERO,
                max_entries_per_tick: 1,
            },
        ),
        Err(ManagedWorkspaceError::Io {
            operation: "workspace.cleanup_root_exists",
            path,
            ..
        }) if path == inaccessible
    ));
}

#[test]
fn janitors_preserve_future_dated_workspace_entries() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    let future = root.join("future-job");
    fs::create_dir(&future)?;

    let removed = cleanup_stale_workspaces(&root, &[], SystemTime::UNIX_EPOCH, Duration::ZERO)?;
    assert!(removed.is_empty());

    let report = cleanup_stale_workspaces_bounded(
        &root,
        &[],
        SystemTime::UNIX_EPOCH,
        WorkspaceRetentionPolicy {
            workspace_max_age: Duration::ZERO,
            diagnostics_max_age: Duration::ZERO,
            max_entries_per_tick: 4,
        },
    )?;
    assert_eq!(report.examined_entries, 1);
    assert!(report.removed.is_empty());
    assert!(report.failures.is_empty());
    assert!(!report.limit_reached);
    assert!(future.exists());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn janitors_ignore_non_utf8_workspace_entries() -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let root = temp_workspace_root()?;
    let non_utf8 = root.join(OsString::from_vec(vec![0xff]));
    fs::create_dir(&non_utf8)?;

    let removed = cleanup_stale_workspaces(
        &root,
        &[],
        SystemTime::now() + Duration::from_secs(1),
        Duration::ZERO,
    )?;
    assert!(removed.is_empty());
    let report = cleanup_stale_workspaces_bounded(
        &root,
        &[],
        SystemTime::now() + Duration::from_secs(1),
        WorkspaceRetentionPolicy {
            workspace_max_age: Duration::ZERO,
            diagnostics_max_age: Duration::ZERO,
            max_entries_per_tick: 1,
        },
    )?;
    assert_eq!(report.examined_entries, 1);
    assert!(report.removed.is_empty());
    assert!(report.failures.is_empty());
    assert!(!report.limit_reached);
    assert!(non_utf8.exists());

    fs::remove_dir_all(root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn janitors_report_unreadable_root() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_workspace_root()?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o000))?;
    let legacy = cleanup_stale_workspaces(&root, &[], SystemTime::now(), Duration::ZERO);
    let bounded = cleanup_stale_workspaces_bounded(
        &root,
        &[],
        SystemTime::now(),
        WorkspaceRetentionPolicy {
            workspace_max_age: Duration::ZERO,
            diagnostics_max_age: Duration::ZERO,
            max_entries_per_tick: 1,
        },
    );
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;

    assert!(matches!(
        legacy,
        Err(ManagedWorkspaceError::Io {
            operation: "workspace.cleanup_read_root",
            path,
            ..
        }) if path == root
    ));
    assert!(matches!(
        bounded,
        Err(ManagedWorkspaceError::Io {
            operation: "workspace.cleanup_read_root",
            path,
            ..
        }) if path == root
    ));
    fs::remove_dir_all(root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn legacy_and_direct_cleanup_report_unreadable_workspace() -> Result<(), Box<dyn std::error::Error>>
{
    use std::os::unix::fs::PermissionsExt;

    let root = temp_workspace_root()?;
    let workspace = create_managed_workspace(&root, "blocked-job")?;
    fs::write(workspace.diagnostics_path.join("evidence"), b"bounded")?;
    fs::set_permissions(&workspace.job_path, fs::Permissions::from_mode(0o000))?;

    let legacy = cleanup_stale_workspaces(
        &root,
        &[],
        SystemTime::now() + Duration::from_secs(1),
        Duration::ZERO,
    );
    let direct = teardown_managed_workspace(&workspace);
    fs::set_permissions(&workspace.job_path, fs::Permissions::from_mode(0o700))?;

    assert!(matches!(
        legacy,
        Err(ManagedWorkspaceError::Io {
            operation: "workspace.cleanup_remove",
            path,
            ..
        }) if path == workspace.job_path
    ));
    assert!(matches!(
        direct,
        Err(ManagedWorkspaceError::Io {
            operation: "workspace.teardown_remove",
            path,
            ..
        }) if path == workspace.job_path
    ));
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn terminal_workspace_cleanup_completed_removes_job_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    let workspace = create_managed_workspace(&root, "job-completed")?;
    fs::write(workspace.output_path.join("movie.mkv"), b"output")?;

    cleanup_terminal_workspace(
        &workspace,
        TerminalWorkspaceState::Completed,
        TerminalWorkspaceCleanupPolicy {
            retain_diagnostics: true,
        },
    )?;

    assert!(!workspace.job_path.exists());
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn terminal_workspace_cleanup_failed_can_retain_diagnostics_only()
-> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    let workspace = create_managed_workspace(&root, "job-failed")?;
    fs::write(workspace.input_path.join("movie.mkv"), b"input")?;
    fs::write(workspace.output_path.join("movie.tmp"), b"output")?;
    fs::write(
        workspace.diagnostics_path.join("ffprobe.json"),
        b"diagnostic",
    )?;

    cleanup_terminal_workspace(
        &workspace,
        TerminalWorkspaceState::Failed,
        TerminalWorkspaceCleanupPolicy {
            retain_diagnostics: true,
        },
    )?;

    assert!(workspace.job_path.exists());
    assert!(!workspace.input_path.exists());
    assert!(!workspace.output_path.exists());
    assert!(workspace.diagnostics_path.join("ffprobe.json").exists());
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn terminal_workspace_cleanup_cancelled_removes_transients_by_default()
-> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    let workspace = create_managed_workspace(&root, "job-cancelled")?;
    fs::write(workspace.output_path.join("movie.tmp"), b"output")?;
    fs::write(workspace.diagnostics_path.join("run.log"), b"diagnostic")?;

    cleanup_terminal_workspace(
        &workspace,
        TerminalWorkspaceState::Cancelled,
        TerminalWorkspaceCleanupPolicy {
            retain_diagnostics: false,
        },
    )?;

    assert!(!workspace.job_path.exists());
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn bounded_janitor_preserves_active_then_expires_after_restart()
-> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    let active = create_managed_workspace(&root, "active-job")?;
    let created_at = SystemTime::now();
    let policy = WorkspaceRetentionPolicy {
        workspace_max_age: Duration::from_secs(10),
        diagnostics_max_age: Duration::from_secs(10),
        max_entries_per_tick: 16,
    };
    let first = cleanup_stale_workspaces_bounded(
        &root,
        std::slice::from_ref(&active.job_key),
        created_at + Duration::from_secs(20),
        policy,
    )?;
    assert!(first.removed.is_empty());
    assert!(active.job_path.exists());

    let restarted =
        cleanup_stale_workspaces_bounded(&root, &[], created_at + Duration::from_secs(20), policy)?;
    assert_eq!(restarted.removed, vec![active.job_path]);
    assert!(restarted.failures.is_empty());
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn diagnostics_only_workspace_uses_persisted_diagnostic_expiry()
-> Result<(), Box<dyn std::error::Error>> {
    let root = temp_workspace_root()?;
    let full = create_managed_workspace(&root, "full-job")?;
    let diagnostics = create_managed_workspace(&root, "failed-job")?;
    cleanup_terminal_workspace(
        &diagnostics,
        TerminalWorkspaceState::Failed,
        TerminalWorkspaceCleanupPolicy {
            retain_diagnostics: true,
        },
    )?;
    let created_at = SystemTime::now();
    let report = cleanup_stale_workspaces_bounded(
        &root,
        &[],
        created_at + Duration::from_secs(20),
        WorkspaceRetentionPolicy {
            workspace_max_age: Duration::from_mins(1),
            diagnostics_max_age: Duration::from_secs(10),
            max_entries_per_tick: 16,
        },
    )?;
    assert_eq!(report.removed, vec![diagnostics.job_path]);
    assert!(full.job_path.exists());
    fs::remove_dir_all(root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn bounded_janitor_continues_after_partial_removal_failure()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_workspace_root()?;
    let blocked = create_managed_workspace(&root, "blocked-job")?;
    let removable = create_managed_workspace(&root, "removable-job")?;
    fs::write(blocked.diagnostics_path.join("evidence"), b"bounded")?;
    fs::set_permissions(&blocked.job_path, fs::Permissions::from_mode(0o000))?;
    let report = cleanup_stale_workspaces_bounded(
        &root,
        &[],
        SystemTime::now() + Duration::from_secs(20),
        WorkspaceRetentionPolicy {
            workspace_max_age: Duration::from_secs(10),
            diagnostics_max_age: Duration::from_secs(10),
            max_entries_per_tick: 16,
        },
    )?;
    fs::set_permissions(&blocked.job_path, fs::Permissions::from_mode(0o700))?;
    assert!(report.removed.contains(&removable.job_path));
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.path == blocked.job_path)
    );
    fs::remove_dir_all(root)?;
    Ok(())
}
