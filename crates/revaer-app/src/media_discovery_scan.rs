//! Deterministic, bounded filesystem traversal for automatic media discovery.

use std::collections::{BTreeMap, VecDeque};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use thiserror::Error;

use crate::media_discovery_runtime::is_media_file;

pub(crate) const SCAN_MAX_DEPTH: usize = 32;
pub(crate) const SCAN_MAX_ENTRIES: usize = 16_384;
pub(crate) const SCAN_MAX_FILES: usize = 1_024;
pub(crate) const SCAN_MAX_BYTES: u64 = 64 * 1024 * 1024 * 1024;
pub(crate) const SCAN_MAX_ELAPSED: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub(crate) struct ScanCursor {
    pending: VecDeque<DirectoryCursor>,
}

#[derive(Clone)]
struct DirectoryCursor {
    path: PathBuf,
    depth: usize,
    after: Option<OsString>,
}

pub(crate) struct ScanBudget {
    pub(crate) depth: usize,
    pub(crate) entries: usize,
    pub(crate) files: usize,
    pub(crate) bytes: u64,
    pub(crate) elapsed: Duration,
}

impl Default for ScanBudget {
    fn default() -> Self {
        Self {
            depth: SCAN_MAX_DEPTH,
            entries: SCAN_MAX_ENTRIES,
            files: SCAN_MAX_FILES,
            bytes: SCAN_MAX_BYTES,
            elapsed: SCAN_MAX_ELAPSED,
        }
    }
}

pub(crate) struct ScanBatch {
    pub(crate) paths: Vec<String>,
    pub(crate) cursor: Option<ScanCursor>,
    pub(crate) limit: Option<ScanLimit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanLimit {
    Entries,
    Files,
    Bytes,
    Depth,
    Elapsed,
    Cancelled,
}

#[derive(Debug, Error)]
pub(crate) enum ScanError {
    #[error("media discovery scan io error for {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("media discovery scan path is not unicode: {0}")]
    NonUnicodePath(PathBuf),
}

pub(crate) fn scan_media_source_paths(
    root: &Path,
    budget: &ScanBudget,
    cursor: Option<ScanCursor>,
    cancelled: impl Fn() -> bool,
) -> Result<ScanBatch, ScanError> {
    let started = Instant::now();
    let mut pending = initial_pending(root, cursor);
    let mut paths = Vec::new();
    let mut entries_seen = 0_usize;
    let mut bytes_selected = 0_u64;

    while let Some(mut directory) = pending.pop_front() {
        let remaining = budget.entries.saturating_sub(entries_seen);
        if remaining == 0 {
            pending.push_front(directory);
            return Ok(finish(paths, pending, ScanLimit::Entries));
        }
        let mut entries = BTreeMap::new();
        let mut has_more = false;
        for entry in fs::read_dir(&directory.path).map_err(|source| ScanError::Io {
            path: directory.path.clone(),
            source,
        })? {
            if cancelled() {
                pending.push_front(directory);
                return Ok(finish(paths, pending, ScanLimit::Cancelled));
            }
            if started.elapsed() >= budget.elapsed {
                pending.push_front(directory);
                return Ok(finish(paths, pending, ScanLimit::Elapsed));
            }
            let entry = entry.map_err(|source| ScanError::Io {
                path: directory.path.clone(),
                source,
            })?;
            let name = entry.file_name();
            if directory.after.as_ref().is_some_and(|after| name <= *after) {
                continue;
            }
            entries.insert(name, entry.path());
            if entries.len() > remaining {
                entries.pop_last();
                has_more = true;
            }
        }
        for (name, entry_path) in entries {
            entries_seen += 1;
            let file_type = fs::symlink_metadata(&entry_path)
                .map_err(|source| ScanError::Io {
                    path: entry_path.clone(),
                    source,
                })?
                .file_type();
            if file_type.is_dir() {
                if directory.depth >= budget.depth {
                    directory.after = Some(name);
                    pending.push_front(directory);
                    return Ok(finish(paths, pending, ScanLimit::Depth));
                }
                pending.push_back(DirectoryCursor {
                    path: entry_path,
                    depth: directory.depth + 1,
                    after: None,
                });
            } else if file_type.is_file() && is_media_file(&entry_path) {
                let size = fs::metadata(&entry_path)
                    .map_err(|source| ScanError::Io {
                        path: entry_path.clone(),
                        source,
                    })?
                    .len();
                if paths.len() >= budget.files {
                    pending.push_front(directory);
                    return Ok(finish(paths, pending, ScanLimit::Files));
                }
                if size > budget.bytes {
                    directory.after = Some(name);
                    pending.push_front(directory);
                    return Ok(finish(paths, pending, ScanLimit::Bytes));
                }
                if bytes_selected.saturating_add(size) > budget.bytes {
                    pending.push_front(directory);
                    return Ok(finish(paths, pending, ScanLimit::Bytes));
                }
                bytes_selected += size;
                paths.push(
                    entry_path
                        .to_str()
                        .map(str::to_string)
                        .ok_or_else(|| ScanError::NonUnicodePath(entry_path.clone()))?,
                );
            }
            directory.after = Some(name);
        }
        if has_more {
            pending.push_front(directory);
        }
    }
    paths.sort();
    Ok(ScanBatch {
        paths,
        cursor: None,
        limit: None,
    })
}

fn initial_pending(root: &Path, cursor: Option<ScanCursor>) -> VecDeque<DirectoryCursor> {
    cursor.map_or_else(
        || {
            VecDeque::from([DirectoryCursor {
                path: root.into(),
                depth: 0,
                after: None,
            }])
        },
        |value| value.pending,
    )
}

const fn finish(
    paths: Vec<String>,
    pending: VecDeque<DirectoryCursor>,
    limit: ScanLimit,
) -> ScanBatch {
    ScanBatch {
        paths,
        cursor: Some(ScanCursor { pending }),
        limit: Some(limit),
    }
}

#[cfg(test)]
mod tests {
    use super::{ScanBudget, ScanLimit, scan_media_source_paths};
    use std::fs;
    use std::time::Duration;

    #[test]
    fn scan_enforces_each_budget_and_resumes_without_duplicates() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        fs::create_dir_all(temp.path().join("a/b"))?;
        for index in 0..5 {
            fs::write(temp.path().join(format!("movie-{index}.mkv")), b"1234")?;
        }
        fs::write(temp.path().join("a/b/deep.mkv"), b"1234")?;

        let mut budget = ScanBudget {
            depth: 8,
            entries: 64,
            files: 2,
            bytes: 64,
            elapsed: Duration::from_secs(1),
        };
        let first = scan_media_source_paths(temp.path(), &budget, None, || false)?;
        assert_eq!(first.limit, Some(ScanLimit::Files));
        let second = scan_media_source_paths(temp.path(), &budget, first.cursor, || false)?;
        assert!(first.paths.iter().all(|path| !second.paths.contains(path)));

        budget.files = 64;
        budget.entries = 1;
        assert_eq!(
            scan_media_source_paths(temp.path(), &budget, None, || false)?.limit,
            Some(ScanLimit::Entries)
        );
        budget.entries = 64;
        budget.bytes = 3;
        let bytes = scan_media_source_paths(temp.path(), &budget, None, || false)?;
        assert!(bytes.paths.is_empty());
        assert_eq!(bytes.limit, Some(ScanLimit::Bytes));
        budget.bytes = 64;
        budget.depth = 0;
        assert_eq!(
            scan_media_source_paths(temp.path(), &budget, None, || false)?.limit,
            Some(ScanLimit::Depth)
        );
        budget.depth = 8;
        budget.elapsed = Duration::ZERO;
        assert_eq!(
            scan_media_source_paths(temp.path(), &budget, None, || false)?.limit,
            Some(ScanLimit::Elapsed)
        );
        budget.elapsed = Duration::from_secs(1);
        assert_eq!(
            scan_media_source_paths(temp.path(), &budget, None, || true)?.limit,
            Some(ScanLimit::Cancelled)
        );
        Ok(())
    }
}
