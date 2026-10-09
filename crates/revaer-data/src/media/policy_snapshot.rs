//! Bounded readers of immutable policy values captured for a job.

use crate::error::{Result, try_op};
use sqlx::PgPool;
use uuid::Uuid;

const LIST_OPERATION_COSTS: &str = "SELECT operation_kind, cost_weight, sort_order, enabled FROM media_job_operation_cost_snapshot_list_v1(media_job_public_id_input => $1)";

/// Explicit persisted operation-cost row, not yet a validated effective policy.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct OperationCostSnapshotRow {
    /// Exact snapshotted operation spelling.
    pub operation_kind: String,
    /// Explicit weight; the compiler rejects negative or excessive values.
    pub cost_weight: i32,
    /// Persisted ordering for duplicate-order validation.
    pub sort_order: i32,
    /// Explicit permission; disabled rows must not be discarded by the reader.
    pub enabled: bool,
}

/// Read only the named job's immutable rows, never the current policy.
///
/// Returns at most fourteen rows so callers can reject overflow beyond thirteen.
/// An absent job or empty snapshot returns an empty family; callers must reject
/// that family as incomplete, not substitute default costs.
///
/// # Errors
/// Returns database execution errors without substituting another policy.
pub async fn list_operation_costs(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<OperationCostSnapshotRow>> {
    sqlx::query_as::<_, OperationCostSnapshotRow>(LIST_OPERATION_COSTS)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job operation cost snapshot list"))
}
