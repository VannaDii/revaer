//! Deterministic and bounded stale-workspace cleanup.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::filesystem::{normalize_workspace_root, trusted_directory_exists};
use super::{
    ManagedWorkspaceError, WorkspaceCleanupFailure, WorkspaceCleanupReport,
    WorkspaceRetentionPolicy,
};

/// Remove inactive workspace directories older than `max_age`.
///
/// Files directly under the root are ignored. Active job keys are preserved
/// regardless of age.
///
/// # Errors
///
/// Returns [`ManagedWorkspaceError`] when the root is invalid or directory
/// traversal/removal fails.
pub fn cleanup_stale_workspaces(
    root_path: impl AsRef<Path>,
    active_job_keys: &[String],
    now: SystemTime,
    max_age: Duration,
) -> Result<Vec<PathBuf>, ManagedWorkspaceError> {
    let root_path = normalize_workspace_root(root_path.as_ref())?;
    if !trusted_directory_exists(&root_path, "workspace.cleanup_root_exists")? {
        return Ok(Vec::new());
    }

    let active = active_job_keys
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut removed = Vec::new();
    for entry in fs::read_dir(&root_path).map_err(|source| ManagedWorkspaceError::Io {
        operation: "workspace.cleanup_read_root",
        path: root_path.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| ManagedWorkspaceError::Io {
            operation: "workspace.cleanup_read_entry",
            path: root_path.clone(),
            source,
        })?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|source| ManagedWorkspaceError::Io {
            operation: "workspace.cleanup_metadata",
            path: path.clone(),
            source,
        })?;
        if !metadata.is_dir() {
            continue;
        }
        let Some(job_key) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !is_revaer_workspace_key(job_key) {
            continue;
        }
        if active.contains(job_key) {
            continue;
        }
        let modified = metadata
            .modified()
            .map_err(|source| ManagedWorkspaceError::Io {
                operation: "workspace.cleanup_modified",
                path: path.clone(),
                source,
            })?;
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age >= max_age {
            remove_workspace_idempotently(&path).map_err(|source| ManagedWorkspaceError::Io {
                operation: "workspace.cleanup_remove",
                path: path.clone(),
                source,
            })?;
            removed.push(path);
        }
    }
    removed.sort();
    Ok(removed)
}

/// Run one partial-failure-tolerant, bounded workspace janitor tick.
///
/// Active keys are always preserved. Inactive full workspaces and diagnostics-only workspaces use
/// distinct persisted retention windows. Failures are reported per entry so a later tick can retry
/// them while unrelated expired workspaces continue to be reclaimed.
///
/// # Errors
///
/// Returns [`ManagedWorkspaceError`] when the root is invalid or cannot be opened. A zero entry
/// budget is rejected as an invalid cleanup policy.
pub fn cleanup_stale_workspaces_bounded(
    root_path: impl AsRef<Path>,
    active_job_keys: &[String],
    now: SystemTime,
    policy: WorkspaceRetentionPolicy,
) -> Result<WorkspaceCleanupReport, ManagedWorkspaceError> {
    let root_path = normalize_workspace_root(root_path.as_ref())?;
    if policy.max_entries_per_tick == 0 {
        return Err(ManagedWorkspaceError::InvalidCleanupPolicy);
    }
    if !trusted_directory_exists(&root_path, "workspace.cleanup_root_exists")? {
        return Ok(WorkspaceCleanupReport {
            examined_entries: 0,
            removed: Vec::new(),
            failures: Vec::new(),
            limit_reached: false,
        });
    }
    let active = active_job_keys
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut report = WorkspaceCleanupReport {
        examined_entries: 0,
        removed: Vec::new(),
        failures: Vec::new(),
        limit_reached: false,
    };
    let entries = fs::read_dir(&root_path).map_err(|source| ManagedWorkspaceError::Io {
        operation: "workspace.cleanup_read_root",
        path: root_path.clone(),
        source,
    })?;
    for entry in entries {
        if report.examined_entries >= policy.max_entries_per_tick {
            report.limit_reached = true;
            break;
        }
        report.examined_entries = report.examined_entries.saturating_add(1);
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report.failures.push(WorkspaceCleanupFailure {
                    path: root_path.clone(),
                    operation: "workspace.cleanup_read_entry",
                    error_kind: error.kind(),
                });
                continue;
            }
        };
        cleanup_bounded_entry(&entry, &active, now, policy, &mut report);
    }
    report.removed.sort();
    report
        .failures
        .sort_by(|left, right| left.path.cmp(&right.path));
    Ok(report)
}

fn cleanup_bounded_entry(
    entry: &fs::DirEntry,
    active: &BTreeSet<&str>,
    now: SystemTime,
    policy: WorkspaceRetentionPolicy,
    report: &mut WorkspaceCleanupReport,
) {
    let path = entry.path();
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_dir() => metadata,
        Ok(_) => return,
        Err(error) => {
            report.failures.push(WorkspaceCleanupFailure {
                path,
                operation: "workspace.cleanup_metadata",
                error_kind: error.kind(),
            });
            return;
        }
    };
    let Some(job_key) = path.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if !is_revaer_workspace_key(job_key) {
        return;
    }
    if active.contains(job_key) {
        return;
    }
    let modified = match metadata.modified() {
        Ok(modified) => modified,
        Err(error) => {
            report.failures.push(WorkspaceCleanupFailure {
                path,
                operation: "workspace.cleanup_modified",
                error_kind: error.kind(),
            });
            return;
        }
    };
    let Ok(age) = now.duration_since(modified) else {
        return;
    };
    let retention = if is_diagnostics_only_workspace(&path) {
        policy.diagnostics_max_age
    } else {
        policy.workspace_max_age
    };
    if age < retention {
        return;
    }
    if let Err(error) = remove_workspace_idempotently(&path) {
        report.failures.push(WorkspaceCleanupFailure {
            path,
            operation: "workspace.cleanup_remove",
            error_kind: error.kind(),
        });
        return;
    }
    report.removed.push(path);
}

fn remove_workspace_idempotently(path: &Path) -> Result<(), std::io::Error> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn is_revaer_workspace_key(value: &str) -> bool {
    let Some((job_id, attempt_and_generation)) = value.split_once("-attempt-") else {
        return false;
    };
    let Some((attempt, generation)) = attempt_and_generation.split_once("-claim-") else {
        return false;
    };
    is_canonical_uuid(job_id)
        && attempt.parse::<u32>().is_ok_and(|number| number > 0)
        && generation.parse::<u64>().is_ok()
}

fn is_canonical_uuid(value: &str) -> bool {
    value.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    }) && value.len() == 36
}

fn is_diagnostics_only_workspace(path: &Path) -> bool {
    let diagnostics = fs::symlink_metadata(path.join("diagnostics"))
        .is_ok_and(|metadata| metadata.file_type().is_dir());
    let input_exists = fs::symlink_metadata(path.join("input")).is_ok();
    let output_exists = fs::symlink_metadata(path.join("output")).is_ok();
    diagnostics && !input_exists && !output_exists
}

#[cfg(test)]
mod tests {
    use super::{is_revaer_workspace_key, remove_workspace_idempotently};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cleanup_identity_is_strict_and_absent_paths_succeed() -> anyhow::Result<()> {
        assert!(is_revaer_workspace_key(
            "00000000-0000-0000-0000-000000000001-attempt-1-claim-0"
        ));
        assert!(!is_revaer_workspace_key("unknown-directory"));
        assert!(!is_revaer_workspace_key(
            "00000000-0000-0000-0000-000000000001-attempt-0-claim-1"
        ));

        let root = std::env::temp_dir().join(format!(
            "revaer-absent-cleanup-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&root)?;
        let absent = root.join("already-absent");
        assert!(remove_workspace_idempotently(&absent).is_ok());
        fs::remove_dir(root)?;
        Ok(())
    }
}
