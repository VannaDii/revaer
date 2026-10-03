//! Exact immutable roots read under current worker claim ownership.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Result, try_op};

/// One claimed destructive attempt whose immutable source owns its journal.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReplacementRecoveryRow {
    /// Monotonic pagination cursor; includes historical attempts.
    pub attempt_id: i64,
    /// Owning job identity.
    pub media_job_public_id: Uuid,
    /// Exact attempt journal generation.
    pub claim_generation: i64,
    /// Sealed source root, never a current profile lookup.
    pub source_root: String,
    /// Durable completion, including already published outbox entries.
    pub terminal_committed: bool,
}

/// Read at most 64 immutable recovery candidates after an attempt cursor.
///
/// # Errors
/// Rejects invalid cursors, stale catalog generations and invalid root seals.
pub async fn list_replacement_recovery(
    pool: &PgPool,
    after_attempt_id: i64,
) -> Result<Vec<ReplacementRecoveryRow>> {
    let rows = sqlx::query_as::<_, ReplacementRecoveryRow>(
        "SELECT * FROM media_job_replacement_recovery_list_v1(after_attempt_id_input => $1)",
    )
    .bind(after_attempt_id)
    .fetch_all(pool)
    .await
    .map_err(try_op("media job immutable replacement recovery read"))?;
    if !recovery_page_valid(&rows, after_attempt_id) {
        return Err(
            try_op("media job immutable replacement recovery transport")(sqlx::Error::Protocol(
                "media_job_root_snapshot_invalid".into(),
            )),
        );
    }
    Ok(rows)
}

fn recovery_page_valid(rows: &[ReplacementRecoveryRow], after_attempt_id: i64) -> bool {
    after_attempt_id >= 0
        && rows.len() <= 64
        && rows.iter().all(|row| {
            row.attempt_id > after_attempt_id
                && row.claim_generation > 0
                && !row.source_root.is_empty()
        })
        && rows
            .windows(2)
            .all(|pair| pair[0].attempt_id < pair[1].attempt_id)
}

/// Complete ordered root evidence. Nullable values express `not_required` only.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct JobRootSnapshotRow {
    /// Owning immutable job.
    pub media_job_id: i64,
    /// Canonical root-kind ordinal.
    pub media_root_kind_id: i16,
    /// Bound or explicitly not required.
    pub binding_state: String,
    /// Exact catalog identity.
    pub media_root_catalog_generation_public_id: Option<Uuid>,
    /// Exact numeric generation.
    pub attestation_generation: Option<i64>,
    /// Catalog source digest.
    pub source_sha256: Option<Vec<u8>>,
    /// Catalog generation digest.
    pub generation_sha256: Option<Vec<u8>>,
    /// Exact slot identity.
    pub media_root_catalog_slot_public_id: Option<Uuid>,
    /// Captured logical key, never a rebinding request.
    pub logical_key: Option<String>,
    /// Captured canonical root path.
    pub canonical_path: Option<String>,
    /// Canonical unsigned device bytes.
    pub filesystem_device: Option<Vec<u8>>,
    /// Canonical unsigned inode bytes.
    pub filesystem_inode: Option<Vec<u8>>,
    /// Captured mount identity.
    pub mount_id: Option<i64>,
    /// Captured filesystem type.
    pub filesystem_type: Option<String>,
    /// Captured capability bits.
    pub capability_mask: Option<i16>,
    /// Captured persistence class.
    pub durability_class: Option<String>,
    /// Captured persistence evidence.
    pub durability_evidence: Option<String>,
    /// Captured deployment writer class.
    pub sole_writer_class: Option<String>,
    /// Captured deployment writer evidence.
    pub sole_writer_evidence: Option<String>,
    /// Captured association prefix.
    pub root_relative_prefix: Option<String>,
    /// Captured root identity digest.
    pub root_identity_sha256: Option<Vec<u8>>,
}

/// Read the approved five-row family without resolving current profile keys.
///
/// # Errors
/// Rejects stale claim/catalog fences, invalid database seals or row transport.
pub async fn read_job_roots(
    pool: &PgPool,
    job: Uuid,
    attempt: i32,
    generation: i64,
) -> Result<Vec<JobRootSnapshotRow>> {
    let rows = sqlx::query_as::<_, JobRootSnapshotRow>(
        "SELECT * FROM media_job_policy_snapshot_root_list_v1(job_public_id_input => $1, attempt_number_input => $2, claim_generation_input => $3)",
    )
    .bind(job).bind(attempt).bind(generation)
    .fetch_all(pool).await.map_err(try_op("media job immutable root read"))?;
    if !ordered_complete(&rows) {
        return Err(try_op("media job immutable root transport")(
            sqlx::Error::Protocol("media_job_root_snapshot_invalid".into()),
        ));
    }
    Ok(rows)
}

fn ordered_complete(rows: &[JobRootSnapshotRow]) -> bool {
    rows.len() == 5
        && rows.iter().enumerate().all(|(index, row)| {
            usize::try_from(row.media_root_kind_id).ok() == Some(index + 1)
                && rows
                    .first()
                    .is_some_and(|first| first.media_job_id == row.media_job_id)
        })
}

#[cfg(test)]
mod tests {
    use super::{
        JobRootSnapshotRow, ReplacementRecoveryRow, ordered_complete, recovery_page_valid,
    };

    #[test]
    fn recovery_pages_require_bounded_forward_progress_and_complete_keys() {
        let row = |attempt_id| ReplacementRecoveryRow {
            attempt_id,
            media_job_public_id: uuid::Uuid::from_u128(1),
            claim_generation: 1,
            source_root: "/source".into(),
            terminal_committed: false,
        };
        assert!(recovery_page_valid(&[], 0));
        assert!(recovery_page_valid(&[row(1), row(2)], 0));
        assert!(!recovery_page_valid(&[], -1));
        assert!(!recovery_page_valid(&[row(1)], 1));
        assert!(!recovery_page_valid(&[row(2), row(1)], 0));
        assert!(!recovery_page_valid(&[row(1), row(1)], 0));
        assert!(recovery_page_valid(
            &(1..=64).map(row).collect::<Vec<_>>(),
            0
        ));
        assert!(!recovery_page_valid(
            &(1..=65).map(row).collect::<Vec<_>>(),
            0
        ));
        let mut invalid = row(1);
        invalid.claim_generation = 0;
        assert!(!recovery_page_valid(&[invalid.clone()], 0));
        invalid.claim_generation = 1;
        invalid.source_root.clear();
        assert!(!recovery_page_valid(&[invalid], 0));
    }

    fn row(kind: i16) -> JobRootSnapshotRow {
        JobRootSnapshotRow {
            media_job_id: 1,
            media_root_kind_id: kind,
            binding_state: "not_required".into(),
            media_root_catalog_generation_public_id: None,
            attestation_generation: None,
            source_sha256: None,
            generation_sha256: None,
            media_root_catalog_slot_public_id: None,
            logical_key: None,
            canonical_path: None,
            filesystem_device: None,
            filesystem_inode: None,
            mount_id: None,
            filesystem_type: None,
            capability_mask: None,
            durability_class: None,
            durability_evidence: None,
            sole_writer_class: None,
            sole_writer_evidence: None,
            root_relative_prefix: None,
            root_identity_sha256: None,
        }
    }

    #[test]
    fn transport_requires_five_ordered_rows_from_one_job() {
        let rows = (1..=5).map(row).collect::<Vec<_>>();
        assert!(ordered_complete(&rows));
        assert!(!ordered_complete(&rows[..4]));
        let mut extra = rows.clone();
        extra.push(row(6));
        assert!(!ordered_complete(&extra));
        let mut reordered = rows.clone();
        reordered.swap(0, 1);
        assert!(!ordered_complete(&reordered));
        let mut duplicate = rows.clone();
        duplicate[4].media_root_kind_id = 4;
        assert!(!ordered_complete(&duplicate));
        let mut mixed = rows;
        mixed[4].media_job_id = 2;
        assert!(!ordered_complete(&mixed));
    }
}
