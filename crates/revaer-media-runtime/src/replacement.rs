//! Durable source-filesystem replacement transactions.

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

const MANAGED_ROOT_NAME: &str = ".revaer-media-runtime";
const REPLACEMENTS_DIR_NAME: &str = "replacements";
const MANIFEST_FILE_NAME: &str = "replacement.json";
const MANIFEST_TEMP_FILE_NAME: &str = "replacement.json.tmp";
const STAGED_FILE_NAME: &str = "candidate.stage";
const RECOVERY_FILE_NAME: &str = "original.recovery";

/// Input needed to prepare one durable replacement transaction.
#[derive(Debug, Clone, Copy)]
pub struct ReplacementRequest<'a> {
    /// Stable job identity used for deterministic recovery.
    pub job_key: &'a str,
    /// Configured profile source root and replacement transaction anchor.
    pub source_root: &'a Path,
    /// Existing source media path to replace.
    pub source_path: &'a Path,
    /// Verified candidate produced in the transient execution workspace.
    pub candidate_path: &'a Path,
}

/// One auxiliary artifact mutation committed with the primary media replacement.
#[derive(Debug, Clone, Copy)]
pub struct ReplacementArtifactRequest<'a> {
    /// Final source-root-relative destination path.
    pub destination_path: &'a Path,
    /// Verified candidate to publish, or `None` to remove the existing destination.
    pub candidate_path: Option<&'a Path>,
}

/// Primary media replacement plus every source-adjacent artifact mutation.
#[derive(Debug, Clone, Copy)]
pub struct ReplacementBundleRequest<'a> {
    /// Stable job identity used for deterministic recovery.
    pub job_key: &'a str,
    /// Configured source root and transaction anchor.
    pub source_root: &'a Path,
    /// Existing source media path to replace.
    pub source_path: &'a Path,
    /// Verified primary media candidate.
    pub candidate_path: &'a Path,
    /// Auxiliary creates, replacements, and removals in deterministic order.
    pub artifacts: &'a [ReplacementArtifactRequest<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedEntry {
    destination: PathBuf,
    staged: Option<PathBuf>,
    recovery: Option<PathBuf>,
    existed: bool,
}

/// Prepared replacement whose stage and recovery copy are durable on the source filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedReplacement {
    transaction_dir: PathBuf,
    source: PathBuf,
    staged: PathBuf,
    recovery: PathBuf,
    entries: Vec<PreparedEntry>,
}

impl PreparedReplacement {
    /// Return the source-filesystem path that will be atomically renamed into place.
    #[must_use]
    pub fn staged_path(&self) -> &Path {
        &self.staged
    }

    /// Return the durable original retained for rollback.
    #[must_use]
    pub fn recovery_path(&self) -> &Path {
        &self.recovery
    }
}

/// Committed replacement retained until post-rename verification succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedReplacement {
    prepared: PreparedReplacement,
}

/// Startup action applied to an interrupted replacement transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementRecoveryAction {
    /// A durable original was restored over an uncertain source path.
    RolledBack,
    /// A transaction that had not committed was removed.
    DiscardedPrepared,
    /// A verified transaction had already removed its recovery copy and only needed cleanup.
    Finalized,
}

/// Result of recovering one deterministic replacement transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredReplacement {
    /// Stable job identity recorded in the transaction manifest.
    pub job_key: String,
    /// Source path named by the transaction.
    pub source_path: PathBuf,
    /// Conservative action taken during recovery.
    pub action: ReplacementRecoveryAction,
}

/// Durable replacement failure.
#[derive(Debug, Error)]
pub enum ReplacementError {
    /// Job identity could escape or alias the deterministic transaction directory.
    #[error("replacement job key is invalid")]
    InvalidJobKey,
    /// Source root is not an existing configured directory.
    #[error("replacement source root is invalid: {0}")]
    InvalidSourceRoot(PathBuf),
    /// Source path does not resolve inside the configured source root.
    #[error("replacement source path is outside its configured root: {0}")]
    SourceOutsideRoot(PathBuf),
    /// A previous transaction with the same job identity needs recovery first.
    #[error("replacement transaction already exists: {0}")]
    TransactionExists(PathBuf),
    /// Recovery state needed for rollback is absent.
    #[error("replacement recovery copy is missing: {0}")]
    RecoveryMissing(PathBuf),
    /// A bundle contains duplicate destination paths.
    #[error("replacement bundle contains duplicate destination: {0}")]
    DuplicateDestination(PathBuf),
    /// The injected committer does not implement auxiliary artifact transactions.
    #[error("replacement committer does not support artifact bundles")]
    ArtifactBundleUnsupported,
    /// A requested removal destination does not exist as a regular file.
    #[error("replacement removal destination is missing: {0}")]
    RemovalDestinationMissing(PathBuf),
    /// Transaction manifest is invalid or inconsistent with its directory.
    #[error("replacement manifest is invalid: {0}")]
    InvalidManifest(PathBuf),
    /// Manifest serialization failed.
    #[error("replacement manifest serialization failed: {0}")]
    Manifest(#[from] serde_json::Error),
    /// Preparation failed and its managed transaction directory could not be removed.
    #[error("replacement preparation failed: {primary}; cleanup also failed: {cleanup}")]
    PreparationCleanup {
        /// Original preparation failure.
        primary: Box<Self>,
        /// Cleanup failure that left managed transaction state behind.
        cleanup: Box<Self>,
    },
    /// Filesystem operation failed.
    #[error("replacement operation {operation} failed for {path}: {source}")]
    Io {
        /// Stable operation name.
        operation: &'static str,
        /// Path being operated on.
        path: PathBuf,
        /// Source I/O error.
        source: io::Error,
    },
}

/// Injected durable replacement transaction boundary.
pub trait ReplacementCommitter {
    /// Copy a verified candidate and the current original into a managed directory on the source
    /// filesystem, preserving source permissions and syncing both files before returning.
    ///
    /// # Errors
    ///
    /// Returns [`ReplacementError`] for invalid paths, conflicting transactions, or I/O failure.
    fn prepare(
        &self,
        request: ReplacementRequest<'_>,
    ) -> Result<PreparedReplacement, ReplacementError>;

    /// Prepare a primary replacement and auxiliary artifact mutations as one recovery unit.
    ///
    /// # Errors
    ///
    /// Returns [`ReplacementError`] when any path, candidate, or durable staging operation fails.
    fn prepare_bundle(
        &self,
        request: ReplacementBundleRequest<'_>,
    ) -> Result<PreparedReplacement, ReplacementError> {
        if request.artifacts.is_empty() {
            self.prepare(ReplacementRequest {
                job_key: request.job_key,
                source_root: request.source_root,
                source_path: request.source_path,
                candidate_path: request.candidate_path,
            })
        } else {
            Err(ReplacementError::ArtifactBundleUnsupported)
        }
    }

    /// Atomically rename the source-filesystem stage over the source and sync the source directory.
    ///
    /// # Errors
    ///
    /// Returns [`ReplacementError`] when the durable commit cannot complete.
    fn commit(
        &self,
        prepared: PreparedReplacement,
    ) -> Result<CommittedReplacement, ReplacementError>;

    /// Remove a prepared but uncommitted transaction without modifying the source.
    ///
    /// # Errors
    ///
    /// Returns [`ReplacementError`] when durable transaction cleanup fails.
    fn discard_prepared(&self, prepared: PreparedReplacement) -> Result<(), ReplacementError>;

    /// Restore the durable original over an uncertain committed result and remove transaction data.
    ///
    /// # Errors
    ///
    /// Returns [`ReplacementError`] when rollback or durable cleanup fails.
    fn rollback(&self, committed: CommittedReplacement) -> Result<(), ReplacementError>;

    /// Remove the recovery copy only after post-rename verification succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`ReplacementError`] when durable cleanup fails.
    fn finalize(&self, committed: CommittedReplacement) -> Result<(), ReplacementError>;

    /// Recover every interrupted transaction under one configured source root.
    ///
    /// # Errors
    ///
    /// Returns [`ReplacementError`] when manifests cannot be read or conservative recovery fails.
    fn recover(&self, source_root: &Path) -> Result<Vec<RecoveredReplacement>, ReplacementError>;
}

/// Standard-library implementation of durable replacement transactions.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemReplacementCommitter;

impl ReplacementCommitter for SystemReplacementCommitter {
    fn prepare(
        &self,
        request: ReplacementRequest<'_>,
    ) -> Result<PreparedReplacement, ReplacementError> {
        prepare_bundle_transaction(ReplacementBundleRequest {
            job_key: request.job_key,
            source_root: request.source_root,
            source_path: request.source_path,
            candidate_path: request.candidate_path,
            artifacts: &[],
        })
    }

    fn prepare_bundle(
        &self,
        request: ReplacementBundleRequest<'_>,
    ) -> Result<PreparedReplacement, ReplacementError> {
        prepare_bundle_transaction(request)
    }

    fn commit(
        &self,
        prepared: PreparedReplacement,
    ) -> Result<CommittedReplacement, ReplacementError> {
        commit_entries(&prepared)?;
        Ok(CommittedReplacement { prepared })
    }

    fn discard_prepared(&self, prepared: PreparedReplacement) -> Result<(), ReplacementError> {
        remove_transaction(&prepared.transaction_dir)
    }

    fn rollback(&self, committed: CommittedReplacement) -> Result<(), ReplacementError> {
        let prepared = committed.prepared;
        rollback_entries(&prepared.entries)?;
        remove_transaction(&prepared.transaction_dir)
    }

    fn finalize(&self, committed: CommittedReplacement) -> Result<(), ReplacementError> {
        let prepared = committed.prepared;
        write_manifest(
            &prepared.transaction_dir,
            &ReplacementManifest {
                job_key: transaction_job_key(&prepared.transaction_dir)?,
                source_path: prepared.source.to_string_lossy().into_owned(),
                phase: ReplacementPhase::Verified,
                entries: manifest_entries(&prepared.entries),
                committed_entries: prepared.entries.len(),
            },
        )?;
        remove_recovery_files(&prepared.entries)?;
        sync_directory(
            &prepared.transaction_dir,
            "replacement.finalize_transaction_sync",
        )?;
        remove_transaction(&prepared.transaction_dir)
    }

    fn recover(&self, source_root: &Path) -> Result<Vec<RecoveredReplacement>, ReplacementError> {
        let source_root = canonical_source_root(source_root)?;
        let root = replacement_root(&source_root);
        if !root.exists() {
            return Ok(Vec::new());
        }
        let entries = read_dir(&root, "replacement.recovery_list")?;
        let mut recovered = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| ReplacementError::Io {
                operation: "replacement.recovery_entry",
                path: root.clone(),
                source,
            })?;
            recovered.push(recover_transaction(&source_root, &entry.path())?);
        }
        sync_directory(&root, "replacement.recovery_root_sync")?;
        Ok(recovered)
    }
}

fn prepare_bundle_transaction(
    request: ReplacementBundleRequest<'_>,
) -> Result<PreparedReplacement, ReplacementError> {
    validate_job_key(request.job_key)?;
    let source_root = canonical_source_root(request.source_root)?;
    let source = resolve_existing_destination(
        &source_root,
        request.source_path,
        "replacement.source_canonicalize",
    )?;
    let candidate = canonical_file(request.candidate_path, "replacement.candidate_canonicalize")?;
    let replacement_root = replacement_root(&source_root);
    create_dir_all(&replacement_root, "replacement.root_create")?;
    let transaction_dir = replacement_root.join(request.job_key);
    create_transaction_dir(&transaction_dir)?;

    let preparation = (|| {
        write_manifest(
            &transaction_dir,
            &ReplacementManifest {
                job_key: request.job_key.to_string(),
                source_path: source.to_string_lossy().into_owned(),
                phase: ReplacementPhase::Prepared,
                entries: Vec::new(),
                committed_entries: 0,
            },
        )?;
        let mut destinations = std::collections::BTreeSet::new();
        destinations.insert(source.clone());
        let mut entries = vec![prepare_entry(
            &transaction_dir,
            0,
            source.clone(),
            Some(&candidate),
        )?];
        for (offset, artifact) in request.artifacts.iter().enumerate() {
            let destination = resolve_bundle_destination(&source_root, artifact.destination_path)?;
            if !destinations.insert(destination.clone()) {
                return Err(ReplacementError::DuplicateDestination(destination));
            }
            let candidate = artifact
                .candidate_path
                .map(|path| canonical_file(path, "replacement.artifact_candidate_canonicalize"))
                .transpose()?;
            if candidate.is_none() && !destination.is_file() {
                return Err(ReplacementError::RemovalDestinationMissing(destination));
            }
            entries.push(prepare_entry(
                &transaction_dir,
                offset + 1,
                destination,
                candidate.as_ref(),
            )?);
        }
        let prepared = PreparedReplacement {
            staged: transaction_dir.join(STAGED_FILE_NAME),
            recovery: transaction_dir.join(RECOVERY_FILE_NAME),
            transaction_dir: transaction_dir.clone(),
            source,
            entries,
        };
        write_manifest(
            &prepared.transaction_dir,
            &ReplacementManifest {
                job_key: request.job_key.to_string(),
                source_path: prepared.source.to_string_lossy().into_owned(),
                phase: ReplacementPhase::Prepared,
                entries: manifest_entries(&prepared.entries),
                committed_entries: 0,
            },
        )?;
        sync_directory(&prepared.transaction_dir, "replacement.transaction_sync")?;
        Ok(prepared)
    })();
    preparation.map_err(|primary| match remove_transaction(&transaction_dir) {
        Ok(()) => primary,
        Err(cleanup) => ReplacementError::PreparationCleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        },
    })
}

fn prepare_entry(
    transaction_dir: &Path,
    index: usize,
    destination: PathBuf,
    candidate: Option<&PathBuf>,
) -> Result<PreparedEntry, ReplacementError> {
    let existed = destination.is_file();
    let permissions = if existed {
        Some(metadata(&destination, "replacement.destination_metadata")?.permissions())
    } else {
        None
    };
    let staged = candidate
        .map(|candidate_path| {
            let path = transaction_entry_path(transaction_dir, index, "candidate.stage");
            copy_new_file(candidate_path, &path, "replacement.stage_candidate")?;
            if let Some(value) = permissions.clone() {
                set_permissions(&path, value, "replacement.stage_permissions")?;
            }
            sync_file(&path, "replacement.stage_sync")?;
            Ok::<PathBuf, ReplacementError>(path)
        })
        .transpose()?;
    let recovery = if existed {
        let path = transaction_entry_path(transaction_dir, index, "original.recovery");
        copy_new_file(&destination, &path, "replacement.copy_recovery")?;
        if let Some(value) = permissions {
            set_permissions(&path, value, "replacement.recovery_permissions")?;
        }
        sync_file(&path, "replacement.recovery_sync")?;
        Some(path)
    } else {
        None
    };
    Ok(PreparedEntry {
        destination,
        staged,
        recovery,
        existed,
    })
}

fn transaction_entry_path(transaction_dir: &Path, index: usize, suffix: &str) -> PathBuf {
    if index == 0 {
        return transaction_dir.join(match suffix {
            "candidate.stage" => STAGED_FILE_NAME,
            _ => RECOVERY_FILE_NAME,
        });
    }
    transaction_dir.join(format!("entry-{index}.{suffix}"))
}

fn resolve_existing_destination(
    source_root: &Path,
    path: &Path,
    operation: &'static str,
) -> Result<PathBuf, ReplacementError> {
    let destination = canonical_file(path, operation)?;
    if destination.starts_with(source_root) {
        Ok(destination)
    } else {
        Err(ReplacementError::SourceOutsideRoot(destination))
    }
}

fn resolve_bundle_destination(
    source_root: &Path,
    path: &Path,
) -> Result<PathBuf, ReplacementError> {
    if path.exists() {
        return resolve_existing_destination(
            source_root,
            path,
            "replacement.artifact_destination_canonicalize",
        );
    }
    let file_name = path.file_name().ok_or_else(|| ReplacementError::Io {
        operation: "replacement.artifact_destination_resolve",
        path: path.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name"),
    })?;
    let parent = path.parent().ok_or_else(|| ReplacementError::Io {
        operation: "replacement.artifact_destination_resolve",
        path: path.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"),
    })?;
    let canonical_parent = canonicalize(parent, "replacement.artifact_parent_canonicalize")?;
    let destination = canonical_parent.join(file_name);
    if destination.starts_with(source_root) {
        Ok(destination)
    } else {
        Err(ReplacementError::SourceOutsideRoot(destination))
    }
}

fn commit_entries(prepared: &PreparedReplacement) -> Result<(), ReplacementError> {
    for (index, entry) in prepared.entries.iter().enumerate() {
        match &entry.staged {
            Some(staged) => rename(staged, &entry.destination, "replacement.commit_rename")?,
            None => remove_file(&entry.destination, "replacement.commit_remove")?,
        }
        if entry.destination.is_file() {
            sync_file(&entry.destination, "replacement.committed_file_sync")?;
        }
        sync_parent(&entry.destination, "replacement.destination_parent_sync")?;
        write_manifest(
            &prepared.transaction_dir,
            &ReplacementManifest {
                job_key: transaction_job_key(&prepared.transaction_dir)?,
                source_path: prepared.source.to_string_lossy().into_owned(),
                phase: ReplacementPhase::Committed,
                entries: manifest_entries(&prepared.entries),
                committed_entries: index + 1,
            },
        )?;
    }
    Ok(())
}

fn rollback_entries(entries: &[PreparedEntry]) -> Result<(), ReplacementError> {
    for entry in entries.iter().rev() {
        if let Some(recovery) = &entry.recovery {
            if !recovery.is_file() {
                return Err(ReplacementError::RecoveryMissing(recovery.clone()));
            }
            rename(recovery, &entry.destination, "replacement.rollback_rename")?;
            sync_file(&entry.destination, "replacement.rollback_file_sync")?;
            sync_parent(&entry.destination, "replacement.rollback_parent_sync")?;
        } else if !entry.existed
            && entry.staged.as_ref().is_some_and(|path| !path.exists())
            && entry.destination.exists()
        {
            remove_file(&entry.destination, "replacement.rollback_remove_created")?;
            sync_parent(&entry.destination, "replacement.rollback_parent_sync")?;
        }
    }
    Ok(())
}

fn remove_recovery_files(entries: &[PreparedEntry]) -> Result<(), ReplacementError> {
    for recovery in entries.iter().filter_map(|entry| entry.recovery.as_deref()) {
        if recovery.exists() {
            remove_file(recovery, "replacement.finalize_remove_recovery")?;
        }
    }
    Ok(())
}

fn recover_transaction(
    source_root: &Path,
    transaction_path: &Path,
) -> Result<RecoveredReplacement, ReplacementError> {
    if !transaction_path.is_dir() {
        return Err(ReplacementError::InvalidManifest(
            transaction_path.to_path_buf(),
        ));
    }
    let manifest = read_manifest(transaction_path)?;
    if manifest.job_key != transaction_job_key(transaction_path)? {
        return Err(ReplacementError::InvalidManifest(
            transaction_path.to_path_buf(),
        ));
    }
    validate_replacement_manifest(transaction_path, &manifest)?;
    let source_path = PathBuf::from(&manifest.source_path);
    if path_has_relative_components(&source_path) {
        return Err(ReplacementError::InvalidManifest(
            transaction_path.to_path_buf(),
        ));
    }
    if !source_path.starts_with(source_root) {
        return Err(ReplacementError::SourceOutsideRoot(source_path));
    }
    let action = recover_transaction_state(source_root, transaction_path, &source_path, &manifest)?;
    remove_transaction(transaction_path)?;
    Ok(RecoveredReplacement {
        job_key: manifest.job_key,
        source_path,
        action,
    })
}

fn recover_transaction_state(
    source_root: &Path,
    transaction_path: &Path,
    source_path: &Path,
    manifest: &ReplacementManifest,
) -> Result<ReplacementRecoveryAction, ReplacementError> {
    if !manifest.entries.is_empty() {
        let entries =
            prepared_entries_from_manifest(source_root, transaction_path, &manifest.entries)?;
        if manifest.phase == ReplacementPhase::Verified {
            remove_recovery_files(&entries)?;
            sync_directory(
                transaction_path,
                "replacement.recovery_verified_transaction_sync",
            )?;
            return Ok(ReplacementRecoveryAction::Finalized);
        }
        rollback_entries(&entries)?;
        return Ok(if manifest.committed_entries == 0 {
            ReplacementRecoveryAction::DiscardedPrepared
        } else {
            ReplacementRecoveryAction::RolledBack
        });
    }
    let recovery_path = transaction_path.join(RECOVERY_FILE_NAME);
    if manifest.phase == ReplacementPhase::Verified {
        if recovery_path.is_file() {
            remove_file(
                &recovery_path,
                "replacement.recovery_remove_verified_original",
            )?;
            sync_directory(
                transaction_path,
                "replacement.recovery_verified_transaction_sync",
            )?;
        }
        return Ok(ReplacementRecoveryAction::Finalized);
    }
    if recovery_path.is_file() {
        rename(&recovery_path, source_path, "replacement.recovery_rollback")?;
        sync_file(source_path, "replacement.recovery_file_sync")?;
        sync_parent(source_path, "replacement.recovery_parent_sync")?;
        return Ok(ReplacementRecoveryAction::RolledBack);
    }
    if manifest.phase == ReplacementPhase::Committed {
        return Ok(ReplacementRecoveryAction::Finalized);
    }
    Ok(ReplacementRecoveryAction::DiscardedPrepared)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReplacementPhase {
    Prepared,
    Committed,
    Verified,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReplacementManifest {
    job_key: String,
    source_path: String,
    phase: ReplacementPhase,
    #[serde(default)]
    entries: Vec<ReplacementManifestEntry>,
    #[serde(default)]
    committed_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplacementManifestEntry {
    destination_path: String,
    staged_file_name: Option<String>,
    recovery_file_name: Option<String>,
    existed: bool,
}

fn validate_replacement_manifest(
    transaction_path: &Path,
    manifest: &ReplacementManifest,
) -> Result<(), ReplacementError> {
    validate_job_key(&manifest.job_key)?;
    if manifest.committed_entries > manifest.entries.len() {
        return Err(ReplacementError::InvalidManifest(
            transaction_path.to_path_buf(),
        ));
    }
    if manifest.entries.is_empty() {
        return Ok(());
    }
    match manifest.phase {
        ReplacementPhase::Prepared if manifest.committed_entries != 0 => {
            return Err(ReplacementError::InvalidManifest(
                transaction_path.to_path_buf(),
            ));
        }
        ReplacementPhase::Committed if manifest.committed_entries == 0 => {
            return Err(ReplacementError::InvalidManifest(
                transaction_path.to_path_buf(),
            ));
        }
        ReplacementPhase::Verified if manifest.committed_entries != manifest.entries.len() => {
            return Err(ReplacementError::InvalidManifest(
                transaction_path.to_path_buf(),
            ));
        }
        ReplacementPhase::Prepared | ReplacementPhase::Committed | ReplacementPhase::Verified => {}
    }
    let mut destinations = std::collections::BTreeSet::new();
    for entry in &manifest.entries {
        validate_manifest_entry_shape(transaction_path, entry)?;
        if !destinations.insert(&entry.destination_path) {
            return Err(ReplacementError::InvalidManifest(
                transaction_path.to_path_buf(),
            ));
        }
    }
    Ok(())
}

fn validate_manifest_entry_shape(
    transaction_path: &Path,
    entry: &ReplacementManifestEntry,
) -> Result<(), ReplacementError> {
    let has_stage = entry.staged_file_name.is_some();
    let has_recovery = entry.recovery_file_name.is_some();
    let valid = match (entry.existed, has_stage, has_recovery) {
        (true, true | false, true) | (false, true, false) => true,
        (true | false, _, _) => false,
    };
    if valid {
        Ok(())
    } else {
        Err(ReplacementError::InvalidManifest(
            transaction_path.to_path_buf(),
        ))
    }
}

fn manifest_entries(entries: &[PreparedEntry]) -> Vec<ReplacementManifestEntry> {
    entries
        .iter()
        .map(|entry| ReplacementManifestEntry {
            destination_path: entry.destination.to_string_lossy().into_owned(),
            staged_file_name: entry.staged.as_deref().and_then(file_name_string),
            recovery_file_name: entry.recovery.as_deref().and_then(file_name_string),
            existed: entry.existed,
        })
        .collect()
}

fn file_name_string(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
}

fn prepared_entries_from_manifest(
    source_root: &Path,
    transaction_path: &Path,
    entries: &[ReplacementManifestEntry],
) -> Result<Vec<PreparedEntry>, ReplacementError> {
    entries
        .iter()
        .map(|entry| {
            let destination = PathBuf::from(&entry.destination_path);
            if path_has_relative_components(&destination) {
                return Err(ReplacementError::InvalidManifest(
                    transaction_path.to_path_buf(),
                ));
            }
            if !destination.starts_with(source_root) {
                return Err(ReplacementError::SourceOutsideRoot(destination));
            }
            let staged = manifest_entry_path(transaction_path, entry.staged_file_name.as_deref())?;
            let recovery =
                manifest_entry_path(transaction_path, entry.recovery_file_name.as_deref())?;
            Ok(PreparedEntry {
                destination,
                staged,
                recovery,
                existed: entry.existed,
            })
        })
        .collect()
}

fn path_has_relative_components(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            std::path::Component::CurDir | std::path::Component::ParentDir
        )
    })
}

fn manifest_entry_path(
    transaction_path: &Path,
    file_name: Option<&str>,
) -> Result<Option<PathBuf>, ReplacementError> {
    let Some(file_name) = file_name else {
        return Ok(None);
    };
    if !matches!(
        Path::new(file_name)
            .components()
            .collect::<Vec<_>>()
            .as_slice(),
        [std::path::Component::Normal(_)]
    ) {
        return Err(ReplacementError::InvalidManifest(
            transaction_path.to_path_buf(),
        ));
    }
    Ok(Some(transaction_path.join(file_name)))
}

fn validate_job_key(job_key: &str) -> Result<(), ReplacementError> {
    if job_key.is_empty()
        || !job_key
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || value == b'-')
    {
        return Err(ReplacementError::InvalidJobKey);
    }
    Ok(())
}

fn canonical_source_root(path: &Path) -> Result<PathBuf, ReplacementError> {
    let canonical = canonicalize(path, "replacement.source_root_canonicalize")?;
    if !canonical.is_dir() {
        return Err(ReplacementError::InvalidSourceRoot(canonical));
    }
    Ok(canonical)
}

fn canonical_file(path: &Path, operation: &'static str) -> Result<PathBuf, ReplacementError> {
    let canonical = canonicalize(path, operation)?;
    if !canonical.is_file() {
        return Err(ReplacementError::Io {
            operation,
            path: canonical,
            source: io::Error::new(io::ErrorKind::InvalidInput, "path is not a regular file"),
        });
    }
    Ok(canonical)
}

fn replacement_root(source_root: &Path) -> PathBuf {
    source_root
        .join(MANAGED_ROOT_NAME)
        .join(REPLACEMENTS_DIR_NAME)
}

fn create_transaction_dir(path: &Path) -> Result<(), ReplacementError> {
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            Err(ReplacementError::TransactionExists(path.to_path_buf()))
        }
        Err(source) => Err(ReplacementError::Io {
            operation: "replacement.transaction_create",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn write_manifest(path: &Path, manifest: &ReplacementManifest) -> Result<(), ReplacementError> {
    let temp_path = path.join(MANIFEST_TEMP_FILE_NAME);
    let manifest_path = path.join(MANIFEST_FILE_NAME);
    let file = File::create(&temp_path).map_err(|source| ReplacementError::Io {
        operation: "replacement.manifest_create",
        path: temp_path.clone(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, manifest)?;
    writer.flush().map_err(|source| ReplacementError::Io {
        operation: "replacement.manifest_flush",
        path: temp_path.clone(),
        source,
    })?;
    writer
        .into_inner()
        .map_err(|error| ReplacementError::Io {
            operation: "replacement.manifest_unwrap",
            path: temp_path.clone(),
            source: error.into_error(),
        })?
        .sync_all()
        .map_err(|source| ReplacementError::Io {
            operation: "replacement.manifest_sync",
            path: temp_path.clone(),
            source,
        })?;
    rename(&temp_path, &manifest_path, "replacement.manifest_publish")?;
    sync_directory(path, "replacement.manifest_parent_sync")
}

fn read_manifest(path: &Path) -> Result<ReplacementManifest, ReplacementError> {
    let manifest_path = path.join(MANIFEST_FILE_NAME);
    let file = File::open(&manifest_path).map_err(|source| ReplacementError::Io {
        operation: "replacement.manifest_open",
        path: manifest_path.clone(),
        source,
    })?;
    serde_json::from_reader(BufReader::new(file)).map_err(ReplacementError::Manifest)
}

fn transaction_job_key(path: &Path) -> Result<String, ReplacementError> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .ok_or_else(|| ReplacementError::InvalidManifest(path.to_path_buf()))
}

fn copy_new_file(
    source: &Path,
    destination: &Path,
    operation: &'static str,
) -> Result<(), ReplacementError> {
    let mut input = File::open(source).map_err(|source_error| ReplacementError::Io {
        operation,
        path: source.to_path_buf(),
        source: source_error,
    })?;
    let output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)
        .map_err(|source_error| ReplacementError::Io {
            operation,
            path: destination.to_path_buf(),
            source: source_error,
        })?;
    let mut writer = BufWriter::new(output);
    io::copy(&mut input, &mut writer).map_err(|source_error| ReplacementError::Io {
        operation,
        path: destination.to_path_buf(),
        source: source_error,
    })?;
    writer.flush().map_err(|source_error| ReplacementError::Io {
        operation,
        path: destination.to_path_buf(),
        source: source_error,
    })
}

fn remove_transaction(path: &Path) -> Result<(), ReplacementError> {
    fs::remove_dir_all(path).map_err(|source| ReplacementError::Io {
        operation: "replacement.transaction_remove",
        path: path.to_path_buf(),
        source,
    })?;
    if let Some(parent) = path.parent() {
        sync_directory(parent, "replacement.transaction_parent_sync")?;
    }
    Ok(())
}

fn canonicalize(path: &Path, operation: &'static str) -> Result<PathBuf, ReplacementError> {
    fs::canonicalize(path).map_err(|source| ReplacementError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn create_dir_all(path: &Path, operation: &'static str) -> Result<(), ReplacementError> {
    fs::create_dir_all(path).map_err(|source| ReplacementError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn read_dir(path: &Path, operation: &'static str) -> Result<fs::ReadDir, ReplacementError> {
    fs::read_dir(path).map_err(|source| ReplacementError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn metadata(path: &Path, operation: &'static str) -> Result<fs::Metadata, ReplacementError> {
    fs::metadata(path).map_err(|source| ReplacementError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn set_permissions(
    path: &Path,
    permissions: fs::Permissions,
    operation: &'static str,
) -> Result<(), ReplacementError> {
    fs::set_permissions(path, permissions).map_err(|source| ReplacementError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn rename(
    source: &Path,
    destination: &Path,
    operation: &'static str,
) -> Result<(), ReplacementError> {
    fs::rename(source, destination).map_err(|source_error| ReplacementError::Io {
        operation,
        path: destination.to_path_buf(),
        source: source_error,
    })
}

fn remove_file(path: &Path, operation: &'static str) -> Result<(), ReplacementError> {
    fs::remove_file(path).map_err(|source| ReplacementError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn sync_file(path: &Path, operation: &'static str) -> Result<(), ReplacementError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| ReplacementError::Io {
            operation,
            path: path.to_path_buf(),
            source,
        })
}

fn sync_parent(path: &Path, operation: &'static str) -> Result<(), ReplacementError> {
    let parent = path.parent().ok_or_else(|| ReplacementError::Io {
        operation,
        path: path.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory"),
    })?;
    sync_directory(parent, operation)
}

fn sync_directory(path: &Path, operation: &'static str) -> Result<(), ReplacementError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| ReplacementError::Io {
            operation,
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::{
        ReplacementArtifactRequest, ReplacementBundleRequest, ReplacementCommitter,
        ReplacementError, ReplacementManifest, ReplacementPhase, ReplacementRecoveryAction,
        ReplacementRequest, SystemReplacementCommitter, manifest_entries, transaction_job_key,
        write_manifest,
    };
    use std::fs;

    #[test]
    fn replacement_stages_on_source_root_and_finalizes_after_commit() -> anyhow::Result<()> {
        let source_fs = tempfile::tempdir()?;
        let execution_fs = tempfile::tempdir()?;
        let source = source_fs.path().join("library/movie.mkv");
        let candidate = execution_fs.path().join("workspace/movie.mkv");
        fs::create_dir_all(
            source
                .parent()
                .ok_or_else(|| anyhow::anyhow!("source parent"))?,
        )?;
        fs::create_dir_all(
            candidate
                .parent()
                .ok_or_else(|| anyhow::anyhow!("candidate parent"))?,
        )?;
        fs::write(&source, b"original")?;
        fs::write(&candidate, b"verified")?;

        let committer = SystemReplacementCommitter;
        let prepared = committer.prepare(ReplacementRequest {
            job_key: "a1b2-c3d4",
            source_root: source_fs.path(),
            source_path: &source,
            candidate_path: &candidate,
        })?;
        let canonical_source_root = fs::canonicalize(source_fs.path())?;
        let canonical_execution_root = fs::canonicalize(execution_fs.path())?;
        assert!(prepared.staged_path().starts_with(canonical_source_root));
        assert!(!prepared.staged_path().starts_with(canonical_execution_root));
        assert_eq!(fs::read(prepared.recovery_path())?, b"original");
        let active = committer.commit(prepared)?;
        assert_eq!(fs::read(&source)?, b"verified");
        committer.finalize(active)?;
        assert_eq!(fs::read(&source)?, b"verified");
        Ok(())
    }

    #[test]
    fn replacement_rolls_back_post_commit_failure() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let candidate_root = tempfile::tempdir()?;
        let source = root.path().join("movie.mkv");
        let candidate = candidate_root.path().join("movie.mkv");
        fs::write(&source, b"original")?;
        fs::write(&candidate, b"invalid-result")?;
        let committer = SystemReplacementCommitter;
        let prepared = committer.prepare(ReplacementRequest {
            job_key: "rollback-job",
            source_root: root.path(),
            source_path: &source,
            candidate_path: &candidate,
        })?;
        let active = committer.commit(prepared)?;
        committer.rollback(active)?;
        assert_eq!(fs::read(&source)?, b"original");
        Ok(())
    }

    #[test]
    fn bundle_commit_and_rollback_cover_sidecar_create_replace_and_remove() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let candidates = tempfile::tempdir()?;
        let source = root.path().join("movie.mkv");
        let existing = root.path().join("movie.eng.srt");
        let created = root.path().join("movie.fra.srt");
        let removed = root.path().join("movie.forced.srt");
        let media_candidate = candidates.path().join("movie.mkv");
        let existing_candidate = candidates.path().join("movie.eng.srt");
        let created_candidate = candidates.path().join("movie.fra.srt");
        fs::write(&source, b"original-media")?;
        fs::write(&existing, b"old-english")?;
        fs::write(&removed, b"old-forced")?;
        fs::write(&media_candidate, b"new-media")?;
        fs::write(&existing_candidate, b"new-english")?;
        fs::write(&created_candidate, b"new-french")?;
        let artifacts = [
            ReplacementArtifactRequest {
                destination_path: &existing,
                candidate_path: Some(&existing_candidate),
            },
            ReplacementArtifactRequest {
                destination_path: &created,
                candidate_path: Some(&created_candidate),
            },
            ReplacementArtifactRequest {
                destination_path: &removed,
                candidate_path: None,
            },
        ];
        let committer = SystemReplacementCommitter;
        let prepared = committer.prepare_bundle(ReplacementBundleRequest {
            job_key: "sidecar-bundle",
            source_root: root.path(),
            source_path: &source,
            candidate_path: &media_candidate,
            artifacts: &artifacts,
        })?;
        let active = committer.commit(prepared)?;
        assert_eq!(fs::read(&source)?, b"new-media");
        assert_eq!(fs::read(&existing)?, b"new-english");
        assert_eq!(fs::read(&created)?, b"new-french");
        assert!(!removed.exists());

        committer.rollback(active)?;
        assert_eq!(fs::read(&source)?, b"original-media");
        assert_eq!(fs::read(&existing)?, b"old-english");
        assert!(!created.exists());
        assert_eq!(fs::read(&removed)?, b"old-forced");
        Ok(())
    }

    #[test]
    fn startup_recovery_rolls_back_committed_sidecar_bundle() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let candidates = tempfile::tempdir()?;
        let source = root.path().join("movie.mkv");
        let created = root.path().join("movie.eng.srt");
        let media_candidate = candidates.path().join("movie.mkv");
        let sidecar_candidate = candidates.path().join("movie.eng.srt");
        fs::write(&source, b"original-media")?;
        fs::write(&media_candidate, b"new-media")?;
        fs::write(&sidecar_candidate, b"new-sidecar")?;
        let artifacts = [ReplacementArtifactRequest {
            destination_path: &created,
            candidate_path: Some(&sidecar_candidate),
        }];
        let committer = SystemReplacementCommitter;
        let prepared = committer.prepare_bundle(ReplacementBundleRequest {
            job_key: "recover-sidecar-bundle",
            source_root: root.path(),
            source_path: &source,
            candidate_path: &media_candidate,
            artifacts: &artifacts,
        })?;
        let _committed = committer.commit(prepared)?;

        let recovered = committer.recover(root.path())?;

        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].action, ReplacementRecoveryAction::RolledBack);
        assert_eq!(fs::read(&source)?, b"original-media");
        assert!(!created.exists());
        Ok(())
    }

    #[test]
    fn prepared_replacement_can_be_discarded_without_modifying_source() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let candidate_root = tempfile::tempdir()?;
        let source = root.path().join("movie.mkv");
        let candidate = candidate_root.path().join("movie.mkv");
        fs::write(&source, b"original")?;
        fs::write(&candidate, b"cancelled-candidate")?;
        let committer = SystemReplacementCommitter;
        let prepared = committer.prepare(ReplacementRequest {
            job_key: "cancel-before-commit",
            source_root: root.path(),
            source_path: &source,
            candidate_path: &candidate,
        })?;

        committer.discard_prepared(prepared)?;

        assert_eq!(fs::read(&source)?, b"original");
        assert!(committer.recover(root.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn startup_recovery_restores_uncertain_committed_source() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let candidate_root = tempfile::tempdir()?;
        let source = root.path().join("movie.mkv");
        let candidate = candidate_root.path().join("movie.mkv");
        fs::write(&source, b"original")?;
        fs::write(&candidate, b"uncertain")?;
        let committer = SystemReplacementCommitter;
        let prepared = committer.prepare(ReplacementRequest {
            job_key: "interrupted-job",
            source_root: root.path(),
            source_path: &source,
            candidate_path: &candidate,
        })?;
        let _committed = committer.commit(prepared)?;

        let recovered = committer.recover(root.path())?;
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].action, ReplacementRecoveryAction::RolledBack);
        assert_eq!(recovered[0].job_key, "interrupted-job");
        assert_eq!(fs::read(&source)?, b"original");
        Ok(())
    }

    #[test]
    fn startup_recovery_preserves_verified_committed_source() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let candidate_root = tempfile::tempdir()?;
        let source = root.path().join("movie.mkv");
        let candidate = candidate_root.path().join("movie.mkv");
        fs::write(&source, b"original")?;
        fs::write(&candidate, b"verified")?;
        let committer = SystemReplacementCommitter;
        let prepared = committer.prepare(ReplacementRequest {
            job_key: "verified-job",
            source_root: root.path(),
            source_path: &source,
            candidate_path: &candidate,
        })?;
        let active = committer.commit(prepared)?;
        write_manifest(
            &active.prepared.transaction_dir,
            &ReplacementManifest {
                job_key: transaction_job_key(&active.prepared.transaction_dir)?,
                source_path: active.prepared.source.to_string_lossy().into_owned(),
                phase: ReplacementPhase::Verified,
                entries: manifest_entries(&active.prepared.entries),
                committed_entries: active.prepared.entries.len(),
            },
        )?;
        drop(active);

        let recovered = committer.recover(root.path())?;

        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].action, ReplacementRecoveryAction::Finalized);
        assert_eq!(fs::read(&source)?, b"verified");
        Ok(())
    }

    #[test]
    fn startup_recovery_rejects_manifest_without_required_recovery() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let candidate_root = tempfile::tempdir()?;
        let source = root.path().join("movie.mkv");
        let candidate = candidate_root.path().join("movie.mkv");
        fs::write(&source, b"original")?;
        fs::write(&candidate, b"candidate")?;
        let committer = SystemReplacementCommitter;
        let prepared = committer.prepare(ReplacementRequest {
            job_key: "bad-recovery-manifest",
            source_root: root.path(),
            source_path: &source,
            candidate_path: &candidate,
        })?;
        let transaction_dir = prepared.transaction_dir.clone();
        let mut entries = manifest_entries(&prepared.entries);
        entries[0].recovery_file_name = None;
        write_manifest(
            &transaction_dir,
            &ReplacementManifest {
                job_key: transaction_job_key(&transaction_dir)?,
                source_path: prepared.source.to_string_lossy().into_owned(),
                phase: ReplacementPhase::Committed,
                entries,
                committed_entries: 1,
            },
        )?;

        let recovery = committer.recover(root.path());

        assert!(matches!(
            recovery,
            Err(ReplacementError::InvalidManifest(path)) if path == transaction_dir
        ));
        assert_eq!(fs::read(&source)?, b"original");
        Ok(())
    }

    #[test]
    fn startup_recovery_rejects_manifest_destination_with_parent_component() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let candidate_root = tempfile::tempdir()?;
        let library = root.path().join("library");
        fs::create_dir_all(&library)?;
        let source = library.join("movie.mkv");
        let candidate = candidate_root.path().join("movie.mkv");
        fs::write(&source, b"original")?;
        fs::write(&candidate, b"candidate")?;
        let committer = SystemReplacementCommitter;
        let prepared = committer.prepare(ReplacementRequest {
            job_key: "bad-path-manifest",
            source_root: root.path(),
            source_path: &source,
            candidate_path: &candidate,
        })?;
        let transaction_dir = prepared.transaction_dir.clone();
        let mut entries = manifest_entries(&prepared.entries);
        entries[0].destination_path = root
            .path()
            .join("library")
            .join("..")
            .join("movie.mkv")
            .to_string_lossy()
            .into_owned();
        write_manifest(
            &transaction_dir,
            &ReplacementManifest {
                job_key: transaction_job_key(&transaction_dir)?,
                source_path: prepared.source.to_string_lossy().into_owned(),
                phase: ReplacementPhase::Committed,
                entries,
                committed_entries: 1,
            },
        )?;

        let recovery = committer.recover(root.path());

        assert!(matches!(
            recovery,
            Err(ReplacementError::InvalidManifest(path)) if path == transaction_dir
        ));
        assert_eq!(fs::read(&source)?, b"original");
        Ok(())
    }
}
