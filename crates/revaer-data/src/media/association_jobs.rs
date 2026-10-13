//! Serializable admission under exact association and retained-root fences.

use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use super::jobs::EnqueuedMediaJobRow;
use crate::error::{Result, try_op};
use crate::transaction::{begin_transaction_with, finish_transaction, release_connection};

const ENQUEUE: &str = "SELECT * FROM media_discovery_association_job_enqueue_v1(actor_public_id_input => $1, association_public_id_input => $2, source_relative_path_input => $3, dry_run_input => $4, source_identity_input => $5, source_size_bytes_input => $6, source_modified_ns_input => $7, source_changed_ns_input => $8, source_sha256_input => $9, association_version_input => $10, expected_generation_input => $11, expected_generation_sha256_input => $12, trigger_input => $13)";

/// Descriptor-observed aggregate identity, never request-supplied evidence.
pub struct AssociationFingerprint<'a> {
    /// Unsigned native source device/inode in canonical hexadecimal form.
    pub identity: &'a str,
    /// Complete media-and-sidecar aggregate size.
    pub size_bytes: i64,
    /// Latest aggregate modification time in nanoseconds.
    pub modified_ns: i64,
    /// Native source change time in nanoseconds.
    pub changed_ns: i64,
    /// Canonical aggregate digest.
    pub sha256: &'a str,
}

/// One candidate and the exact bootstrap/association identities used to read it.
pub struct AssociationJobInput<'a> {
    /// Authenticated operation actor.
    pub actor_public_id: Uuid,
    /// Stable association identity.
    pub association_public_id: Uuid,
    /// Exact ready version observed before fingerprinting.
    pub association_version: i32,
    /// Validated root-relative candidate, preserved byte-for-byte.
    pub relative_path: &'a str,
    /// Requested dry-run; the procedure also enforces immutable policy.
    pub dry_run: bool,
    /// Server-selected mode, rechecked against the locked immutable association.
    pub trigger: &'a str,
    /// Retained bootstrap's numeric generation, not a logical-key lookup.
    pub generation: i64,
    /// Retained bootstrap's complete generation digest.
    pub generation_sha256: [u8; 32],
    /// Real descriptor-owned fingerprint.
    pub fingerprint: AssociationFingerprint<'a>,
}

/// Admit one candidate atomically; an unchanged candidate returns `None`.
///
/// # Errors
/// Propagates stale fences, invalid input, privilege and transaction failures.
/// Only definitive serialization/deadlock failures are retried, at most twice,
/// with 50 ms and 100 ms waits so concurrent discovery can finish its write.
pub async fn enqueue_association_job(
    pool: &PgPool,
    input: &AssociationJobInput<'_>,
) -> Result<Option<EnqueuedMediaJobRow>> {
    let pool = pool.clone();
    let actor_public_id = input.actor_public_id;
    let association_public_id = input.association_public_id;
    let association_version = input.association_version;
    let relative_path = input.relative_path.to_owned();
    let dry_run = input.dry_run;
    let trigger = input.trigger.to_owned();
    let generation = input.generation;
    let generation_sha256 = input.generation_sha256;
    let fingerprint_identity = input.fingerprint.identity.to_owned();
    let fingerprint_size_bytes = input.fingerprint.size_bytes;
    let fingerprint_modified_ns = input.fingerprint.modified_ns;
    let fingerprint_changed_ns = input.fingerprint.changed_ns;
    let fingerprint_sha256 = input.fingerprint.sha256.to_owned();
    // The worker owns BEGIN and cleanup even when its caller stops waiting.
    tokio::spawn(async move {
        let input = AssociationJobInput {
            actor_public_id,
            association_public_id,
            association_version,
            relative_path: &relative_path,
            dry_run,
            trigger: &trigger,
            generation,
            generation_sha256,
            fingerprint: AssociationFingerprint {
                identity: &fingerprint_identity,
                size_bytes: fingerprint_size_bytes,
                modified_ns: fingerprint_modified_ns,
                changed_ns: fingerprint_changed_ns,
                sha256: &fingerprint_sha256,
            },
        };

        let mut retries = 0;
        loop {
            let result = enqueue_once(&pool, &input).await;
            if let Err(error) = &result
                && retries < 2
                && matches!(error.database_code().as_deref(), Some("40001" | "40P01"))
            {
                retries += 1;
                tokio::time::sleep(Duration::from_millis(50 * retries)).await;
                continue;
            }
            return result;
        }
    })
    .await
    .map_err(|source| crate::error::DataError::BoundaryFailed {
        operation: "enqueue_association_job",
        source: Box::new(source),
    })?
}

async fn enqueue_once(
    pool: &PgPool,
    input: &AssociationJobInput<'_>,
) -> Result<Option<EnqueuedMediaJobRow>> {
    let mut connection = try_op("transaction acquire", || pool.acquire()).await?;
    let completed = async {
        let mut transaction =
            begin_transaction_with(&mut connection, "BEGIN ISOLATION LEVEL SERIALIZABLE").await?;
        let result = try_op("association job admission", || async {
            sqlx::query_as::<_, EnqueuedMediaJobRow>(ENQUEUE)
                .bind(input.actor_public_id)
                .bind(input.association_public_id)
                .bind(input.relative_path)
                .bind(input.dry_run)
                .bind(input.fingerprint.identity)
                .bind(input.fingerprint.size_bytes)
                .bind(input.fingerprint.modified_ns)
                .bind(input.fingerprint.changed_ns)
                .bind(input.fingerprint.sha256)
                .bind(input.association_version)
                .bind(input.generation)
                .bind(input.generation_sha256.as_slice())
                .bind(input.trigger)
                .fetch_optional(&mut *transaction)
                .await
        })
        .await;
        finish_transaction(transaction, result).await
    }
    .await;
    release_connection(connection, completed).await
}
