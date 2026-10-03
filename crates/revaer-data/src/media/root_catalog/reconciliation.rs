//! Transaction-owned bootstrap reconciliation of independently attested roots.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::{Result, try_op};

/// Expected complete catalog identity, independently checked by `PostgreSQL`.
pub struct RootCatalogGenerationInput {
    /// Approved source document version.
    pub source_format_version: i16,
    /// Canonical source identity, not a hash of raw JSON.
    pub source_sha256: [u8; 32],
    /// Complete ordered attestation identity.
    pub attestation_sha256: [u8; 32],
    /// Source-plus-attestation identity.
    pub generation_sha256: [u8; 32],
    /// Complete slot count, constrained to zero through 256 by the procedure.
    pub slot_count: i16,
}

/// One normalized attestation; callers must already hold its actual proof.
/// This input deliberately has no Debug implementation because it carries paths.
pub struct RootCatalogSlotInput<'a> {
    /// Exact immutable logical key.
    pub logical_key: &'a str,
    /// Trusted declaration path, never an HTTP-supplied path.
    pub requested_path: &'a str,
    /// Descriptor-verified canonical path.
    pub canonical_path: &'a str,
    /// Unsigned device identity in big-endian bytes.
    pub filesystem_device: [u8; 8],
    /// Unsigned inode identity in big-endian bytes.
    pub filesystem_inode: [u8; 8],
    /// Nonnegative observed Linux mount ID.
    pub mount_id: i64,
    /// Observed filesystem type.
    pub filesystem_type: &'a str,
    /// Read/write/create/fsync/rename/delete/capacity, bound as seven scalar columns.
    pub capabilities: [bool; 7],
    /// Proven closed durability class.
    pub durability_class: &'a str,
    /// Proven closed deployment evidence class.
    pub durability_evidence: &'a str,
    /// Proven closed writer-control class.
    pub sole_writer_class: &'a str,
    /// Proven closed writer-control evidence.
    pub sole_writer_evidence: &'a str,
    /// Descriptor owner, widened losslessly for `PostgreSQL`.
    pub owner_uid: u32,
    /// Descriptor group, widened losslessly for `PostgreSQL`.
    pub owner_gid: u32,
    /// Permission/special bits, constrained by the procedure.
    pub mode_bits: i32,
    /// Exact slot identity, rechecked against rows and kinds during activation.
    pub root_identity_sha256: [u8; 32],
}

#[derive(sqlx::FromRow)]
struct Candidate {
    media_root_catalog_generation_public_id: Uuid,
    attestation_generation: i64,
    already_current: bool,
}

/// Committed generation metadata; not filesystem authority or a job admission.
#[derive(sqlx::FromRow)]
pub struct ActivatedRootCatalog {
    /// Public generation occurrence identity.
    pub media_root_catalog_generation_public_id: Uuid,
    /// Monotonic occurrence fence, not a semantic digest.
    pub attestation_generation: i64,
    /// Verified source identity.
    pub source_sha256: Vec<u8>,
    /// Verified complete generation identity.
    pub generation_sha256: Vec<u8>,
    /// Exact activated slot count.
    pub slot_count: i16,
    /// Original activation time, unchanged by idempotent revalidation.
    pub activated_at: DateTime<Utc>,
}

/// Owns one serializable transaction from begin through final activation.
///
/// Only injected bootstrap wiring may use this adapter. Keep process/root locks
/// alive throughout and revalidate descriptors immediately before activation.
/// Dropping an unfinished adapter invokes `SQLx` transaction rollback; explicit
/// rollback is available when its completion must be observed before recovery.
pub struct RootCatalogReconciliation {
    transaction: Transaction<'static, Postgres>,
    candidate: Candidate,
}

impl RootCatalogReconciliation {
    /// Begin a serialized candidate, or identify the exactly current generation.
    ///
    /// # Errors
    /// Propagates connection, transaction, contract and locking failures.
    pub async fn begin(pool: &PgPool, input: &RootCatalogGenerationInput) -> Result<Self> {
        let mut transaction = pool
            .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
            .await
            .map_err(try_op("begin root catalog reconciliation"))?;
        let candidate = sqlx::query_as(
            "SELECT * FROM media_root_catalog_reconcile_begin_v1($1, $2, $3, $4, $5)",
        )
        .bind(input.source_format_version)
        .bind(input.source_sha256.as_slice())
        .bind(input.attestation_sha256.as_slice())
        .bind(input.generation_sha256.as_slice())
        .bind(input.slot_count)
        .fetch_one(&mut *transaction)
        .await
        .map_err(try_op("stage root catalog generation"))?;
        Ok(Self {
            transaction,
            candidate,
        })
    }

    /// Whether the caller must skip appends and only revalidate/activate.
    #[must_use]
    pub const fn already_current(&self) -> bool {
        self.candidate.already_current
    }

    /// Exact generation fence for this transaction, including idempotent reuse.
    #[must_use]
    pub const fn generation(&self) -> i64 {
        self.candidate.attestation_generation
    }

    /// Append the next slot in logical-key order, returning its stable public ID.
    ///
    /// # Errors
    /// Rejects idempotent/current writes, unordered, duplicate or invalid rows.
    pub async fn append_slot(&mut self, input: &RootCatalogSlotInput<'_>) -> Result<Uuid> {
        sqlx::query_scalar("SELECT media_root_catalog_reconcile_slot_v1($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)")
            .bind(self.candidate.media_root_catalog_generation_public_id)
            .bind(input.logical_key).bind(input.requested_path).bind(input.canonical_path)
            .bind(input.filesystem_device.as_slice()).bind(input.filesystem_inode.as_slice())
            .bind(input.mount_id).bind(input.filesystem_type)
            .bind(input.capabilities[0]).bind(input.capabilities[1]).bind(input.capabilities[2])
            .bind(input.capabilities[3]).bind(input.capabilities[4]).bind(input.capabilities[5])
            .bind(input.capabilities[6]).bind(input.durability_class).bind(input.durability_evidence)
            .bind(input.sole_writer_class).bind(input.sole_writer_evidence)
            .bind(i64::from(input.owner_uid)).bind(i64::from(input.owner_gid)).bind(input.mode_bits)
            .bind(input.root_identity_sha256.as_slice()).fetch_one(&mut *self.transaction).await
            .map_err(try_op("append root catalog slot"))
    }

    /// Append one approved kind in ordinal order to a slot in this candidate.
    ///
    /// # Errors
    /// Rejects unknown, duplicate, unordered and foreign-slot kinds.
    pub async fn append_kind(&mut self, slot: Uuid, kind: &str) -> Result<()> {
        sqlx::query("SELECT media_root_catalog_reconcile_slot_kind_v1($1, $2, $3)")
            .bind(self.candidate.media_root_catalog_generation_public_id)
            .bind(slot)
            .bind(kind)
            .execute(&mut *self.transaction)
            .await
            .map_err(try_op("append root catalog kind"))?;
        Ok(())
    }

    /// Activate and commit after the caller's final descriptor revalidation.
    /// Metadata is returned only after a successful commit, including deferred checks.
    ///
    /// # Errors
    /// Propagates incomplete proof, identity, serialization and commit failures.
    pub async fn activate(mut self) -> Result<ActivatedRootCatalog> {
        let activated =
            sqlx::query_as("SELECT * FROM media_root_catalog_reconcile_activate_v1($1)")
                .bind(self.candidate.media_root_catalog_generation_public_id)
                .fetch_one(&mut *self.transaction)
                .await
                .map_err(try_op("activate root catalog"))?;
        self.transaction
            .commit()
            .await
            .map_err(try_op("commit root catalog reconciliation"))?;
        Ok(activated)
    }

    /// Finish rollback before attempting a separate fail-closed state mutation.
    ///
    /// # Errors
    /// Propagates rollback failure; it is never treated as successful cleanup.
    pub async fn rollback(self) -> Result<()> {
        self.transaction
            .rollback()
            .await
            .map_err(try_op("rollback root catalog reconciliation"))
    }
}
