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
    use super::{ScanBudget, ScanError, ScanLimit, scan_media_source_paths};
    use std::cell::Cell;
    use std::fs;
    use std::io::ErrorKind;
    #[cfg(target_os = "linux")]
    use std::os::unix::ffi::OsStringExt;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use std::time::Duration;

    const fn test_budget() -> ScanBudget {
        ScanBudget {
            depth: 8,
            entries: 64,
            files: 64,
            bytes: 64,
            elapsed: Duration::from_secs(1),
        }
    }

    fn path_string(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    fn drain_scan(
        root: &Path,
        budget: &ScanBudget,
        maximum_batches: usize,
    ) -> anyhow::Result<(Vec<String>, Vec<ScanLimit>)> {
        let mut cursor = None;
        let mut paths = Vec::new();
        let mut limits = Vec::new();
        for _ in 0..maximum_batches {
            let batch = scan_media_source_paths(root, budget, cursor, || false)?;
            paths.extend(batch.paths);
            if let Some(limit) = batch.limit {
                limits.push(limit);
            }
            cursor = batch.cursor;
            if cursor.is_none() {
                break;
            }
        }
        assert!(
            cursor.is_none(),
            "scan did not finish within its batch bound"
        );
        paths.sort();
        Ok((paths, limits))
    }

    #[test]
    fn scan_returns_sorted_recursive_media_files() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let nested = temp.path().join("a");
        let nested_media = nested.join("deep.mp4");
        let root_media = temp.path().join("z-root.MKV");
        fs::create_dir(&nested)?;
        fs::write(&nested_media, b"nested")?;
        fs::write(&root_media, b"root")?;
        fs::write(temp.path().join("subtitle.srt"), b"subtitle")?;
        fs::write(temp.path().join("README"), b"notes")?;

        let batch = scan_media_source_paths(temp.path(), &ScanBudget::default(), None, || false)?;
        let mut expected = vec![path_string(&nested_media), path_string(&root_media)];
        expected.sort();

        assert_eq!(batch.paths, expected);
        assert!(batch.cursor.is_none());
        assert!(batch.limit.is_none());
        Ok(())
    }

    #[test]
    fn scan_resumes_entry_limited_batches_without_duplicates() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let nested = temp.path().join("c");
        let first_media = temp.path().join("a.mkv");
        let nested_media = nested.join("d.mp4");
        fs::create_dir(&nested)?;
        fs::write(&first_media, b"first")?;
        fs::write(temp.path().join("b.txt"), b"ignored")?;
        fs::write(&nested_media, b"nested")?;

        let mut budget = test_budget();
        budget.entries = 1;
        let (paths, limits) = drain_scan(temp.path(), &budget, 8)?;
        let mut expected = vec![path_string(&first_media), path_string(&nested_media)];
        expected.sort();

        assert_eq!(paths, expected);
        assert_eq!(
            limits,
            vec![ScanLimit::Entries, ScanLimit::Entries, ScanLimit::Entries]
        );
        Ok(())
    }

    #[test]
    fn scan_resumes_file_limited_batches_without_duplicates() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let media = ["a.mkv", "b.mp4", "c.webm"].map(|name| temp.path().join(name));
        for path in &media {
            fs::write(path, b"media")?;
        }

        let mut budget = test_budget();
        budget.files = 1;
        let (paths, limits) = drain_scan(temp.path(), &budget, 4)?;
        let expected = media
            .iter()
            .map(|path| path_string(path))
            .collect::<Vec<_>>();

        assert_eq!(paths, expected);
        assert_eq!(limits, vec![ScanLimit::Files, ScanLimit::Files]);
        Ok(())
    }

    #[test]
    fn scan_defers_file_that_exceeds_remaining_byte_budget() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let first_media = temp.path().join("a.mkv");
        let second_media = temp.path().join("b.mkv");
        fs::write(&first_media, b"1234")?;
        fs::write(&second_media, b"5678")?;

        let mut budget = test_budget();
        budget.bytes = 5;
        let first = scan_media_source_paths(temp.path(), &budget, None, || false)?;
        assert_eq!(first.paths, vec![path_string(&first_media)]);
        assert_eq!(first.limit, Some(ScanLimit::Bytes));
        assert!(first.cursor.is_some());

        let second = scan_media_source_paths(temp.path(), &budget, first.cursor, || false)?;
        assert_eq!(second.paths, vec![path_string(&second_media)]);
        assert!(second.cursor.is_none());
        assert!(second.limit.is_none());
        Ok(())
    }

    #[test]
    fn scan_skips_file_larger_than_total_byte_budget() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let oversized = temp.path().join("a-oversized.mkv");
        let selected = temp.path().join("b-selected.mkv");
        fs::write(&oversized, b"123456")?;
        fs::write(&selected, b"1234")?;

        let mut budget = test_budget();
        budget.bytes = 5;
        let first = scan_media_source_paths(temp.path(), &budget, None, || false)?;
        assert!(first.paths.is_empty());
        assert_eq!(first.limit, Some(ScanLimit::Bytes));

        let second = scan_media_source_paths(temp.path(), &budget, first.cursor, || false)?;
        assert_eq!(second.paths, vec![path_string(&selected)]);
        assert!(second.cursor.is_none());
        assert!(second.limit.is_none());
        Ok(())
    }

    #[test]
    fn scan_skips_directories_beyond_depth_and_resumes_siblings() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let nested = temp.path().join("a");
        let nested_media = nested.join("deep.mkv");
        let root_media = temp.path().join("z-root.mkv");
        fs::create_dir(&nested)?;
        fs::write(&nested_media, b"nested")?;
        fs::write(&root_media, b"root")?;

        let mut budget = test_budget();
        budget.depth = 0;
        let first = scan_media_source_paths(temp.path(), &budget, None, || false)?;
        assert!(first.paths.is_empty());
        assert_eq!(first.limit, Some(ScanLimit::Depth));

        let second = scan_media_source_paths(temp.path(), &budget, first.cursor, || false)?;
        assert_eq!(second.paths, vec![path_string(&root_media)]);
        assert!(!second.paths.contains(&path_string(&nested_media)));
        assert!(second.cursor.is_none());
        assert!(second.limit.is_none());
        Ok(())
    }

    #[test]
    fn scan_resumes_after_cancellation() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let media = temp.path().join("movie.mkv");
        fs::write(&media, b"media")?;
        let cancel_next_check = Cell::new(true);

        let first = scan_media_source_paths(temp.path(), &test_budget(), None, || {
            cancel_next_check.replace(false)
        })?;
        assert!(first.paths.is_empty());
        assert_eq!(first.limit, Some(ScanLimit::Cancelled));
        assert!(!cancel_next_check.get());

        let second = scan_media_source_paths(temp.path(), &test_budget(), first.cursor, || false)?;
        assert_eq!(second.paths, vec![path_string(&media)]);
        assert!(second.cursor.is_none());
        assert!(second.limit.is_none());
        Ok(())
    }

    #[test]
    fn scan_resumes_after_elapsed_limit() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let media = temp.path().join("movie.mkv");
        fs::write(&media, b"media")?;
        let mut budget = test_budget();
        budget.elapsed = Duration::ZERO;

        let first = scan_media_source_paths(temp.path(), &budget, None, || false)?;
        assert!(first.paths.is_empty());
        assert_eq!(first.limit, Some(ScanLimit::Elapsed));

        budget.elapsed = Duration::from_secs(1);
        let second = scan_media_source_paths(temp.path(), &budget, first.cursor, || false)?;
        assert_eq!(second.paths, vec![path_string(&media)]);
        assert!(second.cursor.is_none());
        assert!(second.limit.is_none());
        Ok(())
    }

    #[test]
    fn scan_reports_missing_root() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let missing = temp.path().join("missing");
        let result = scan_media_source_paths(&missing, &test_budget(), None, || false);

        assert!(result.as_ref().err().is_some_and(|error| {
            error
                .to_string()
                .starts_with("media discovery scan io error for ")
        }));
        assert!(matches!(
            result,
            Err(ScanError::Io { path, source })
                if path == missing && source.kind() == ErrorKind::NotFound
        ));
        Ok(())
    }

    #[test]
    fn scan_reports_entry_removed_during_traversal() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let media = temp.path().join("movie.mkv");
        fs::write(&media, b"media")?;
        let removal_attempted = Cell::new(false);
        let media_to_remove = media.clone();

        let result = scan_media_source_paths(temp.path(), &test_budget(), None, || {
            if !removal_attempted.replace(true) {
                assert!(
                    fs::remove_file(&media_to_remove).is_ok(),
                    "test setup must remove the enumerated entry"
                );
            }
            false
        });

        assert!(removal_attempted.get());
        assert!(matches!(
            result,
            Err(ScanError::Io { path, source })
                if path == media && source.kind() == ErrorKind::NotFound
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn scan_ignores_symbolic_links() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let real_directory = temp.path().join("real");
        let real_media = real_directory.join("movie.mkv");
        fs::create_dir(&real_directory)?;
        fs::write(&real_media, b"media")?;
        symlink(&real_media, temp.path().join("linked-movie.mkv"))?;
        symlink(&real_directory, temp.path().join("linked-directory"))?;

        let batch = scan_media_source_paths(temp.path(), &test_budget(), None, || false)?;

        assert_eq!(batch.paths, vec![path_string(&real_media)]);
        assert!(batch.cursor.is_none());
        assert!(batch.limit.is_none());
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn scan_reports_non_unicode_media_path() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let name = std::ffi::OsString::from_vec(b"movie-\xff.mkv".to_vec());
        let media = temp.path().join(name);
        fs::write(&media, b"media")?;

        let result = scan_media_source_paths(temp.path(), &test_budget(), None, || false);

        assert!(result.as_ref().err().is_some_and(|error| {
            error
                .to_string()
                .starts_with("media discovery scan path is not unicode: ")
        }));
        assert!(matches!(
            result,
            Err(ScanError::NonUnicodePath(path)) if path == media
        ));
        Ok(())
    }
}
