//! Completed execution-output evidence for one current, resumable attempt.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Result, try_op};

/// Durable output evidence; callers revalidate the file before reuse.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct StepCheckpoint {
    /// Digest of the compiled execution prefix.
    pub step_signature: Vec<u8>,
    /// Exact managed output path.
    pub output_path: String,
    /// Completed byte length.
    pub size_bytes: i64,
    /// Completed file content digest.
    pub output_sha256: Vec<u8>,
}

/// Read a completed output, or expected absence, under a current active attempt.
///
/// # Errors
/// Returns stale-claim and database errors.
pub async fn get_step_checkpoint(
    pool: &PgPool,
    job_id: Uuid,
    generation: i64,
    step_index: i32,
) -> Result<Option<StepCheckpoint>> {
    sqlx::query_as::<_, StepCheckpoint>(
        "SELECT * FROM media_job_step_checkpoint_get_v1(media_job_public_id_input => $1, claim_generation_input => $2, step_index_input => $3)",
    )
    .bind(job_id)
    .bind(generation)
    .bind(step_index)
    .fetch_optional(pool)
    .await
    .map_err(try_op("read media execution checkpoint"))
}

/// Persist evidence only after a writer exited and its output was synchronized.
///
/// # Errors
/// Returns stale-claim, malformed-evidence and database errors.
pub async fn write_step_checkpoint(
    pool: &PgPool,
    job_id: Uuid,
    generation: i64,
    step_index: i32,
    checkpoint: &StepCheckpoint,
) -> Result<()> {
    sqlx::query(
        "SELECT media_job_step_checkpoint_write_v1(media_job_public_id_input => $1, claim_generation_input => $2, step_index_input => $3, step_signature_input => $4, output_path_input => $5, size_bytes_input => $6, output_sha256_input => $7)",
    )
    .bind(job_id)
    .bind(generation)
    .bind(step_index)
    .bind(&checkpoint.step_signature)
    .bind(&checkpoint.output_path)
    .bind(checkpoint.size_bytes)
    .bind(&checkpoint.output_sha256)
    .execute(pool)
    .await
    .map_err(try_op("write media execution checkpoint"))?;
    Ok(())
}
