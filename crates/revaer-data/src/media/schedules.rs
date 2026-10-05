//! Native association-version cadence reads and conditional creation.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Result, try_op};

/// Complete persisted cadence, with no filesystem authority or inferred defaults.
#[derive(sqlx::FromRow)]
pub struct ScheduleConfigurationRow {
    /// Stable association identity.
    pub media_discovery_association_public_id: Uuid,
    /// Exact association version owning the state.
    pub association_version: i32,
    /// Explicit operator-selected quantity.
    pub interval_quantity: i32,
    /// Closed minute/hour unit.
    pub interval_unit: String,
    /// Durable due progression anchor.
    pub anchor_due_at: DateTime<Utc>,
    /// Exact persisted revision for conditional editing.
    pub updated_at: DateTime<Utc>,
}

/// Read configuration for the latest association version.
///
/// # Errors
/// Propagates storage and decoding failures. `None` means no configured cadence.
pub async fn read_schedule_configuration(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<ScheduleConfigurationRow>> {
    sqlx::query_as("SELECT * FROM media_discovery_schedule_configuration_get_v1($1)")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(try_op("read native schedule configuration"))
}

/// Create cadence atomically, checking the exact active association under a lock.
///
/// # Errors
/// Rejects stale versions, unavailable associations, invalid cadence and duplicates.
pub async fn create_schedule_configuration(
    pool: &PgPool,
    actor: Uuid,
    id: Uuid,
    version: i32,
    quantity: i32,
    unit: &str,
) -> Result<ScheduleConfigurationRow> {
    sqlx::query_as("SELECT * FROM media_discovery_schedule_configuration_create_v1($1,$2,$3,$4,$5)")
        .bind(actor)
        .bind(id)
        .bind(version)
        .bind(quantity)
        .bind(unit)
        .fetch_one(pool)
        .await
        .map_err(try_op("create native schedule configuration"))
}

/// Complete replacement input with an exact prior state revision.
pub struct ScheduleReplacementInput<'a> {
    /// Authenticated writer identity.
    pub actor: Uuid,
    /// Logical association identity.
    pub id: Uuid,
    /// Exact active association version.
    pub version: i32,
    /// Explicit cadence quantity.
    pub quantity: i32,
    /// Closed cadence unit token.
    pub unit: &'a str,
    /// Expected persisted state revision.
    pub updated_at: DateTime<Utc>,
}

/// Replace explicitly selected cadence only when its persisted revision matches.
///
/// # Errors
/// Rejects stale revisions, stale association heads and invalid quantities.
pub async fn replace_schedule_configuration(
    pool: &PgPool,
    request: &ScheduleReplacementInput<'_>,
) -> Result<ScheduleConfigurationRow> {
    sqlx::query_as(
        "SELECT * FROM media_discovery_schedule_configuration_replace_v1($1,$2,$3,$4,$5,$6)",
    )
    .bind(request.actor)
    .bind(request.id)
    .bind(request.version)
    .bind(request.quantity)
    .bind(request.unit)
    .bind(request.updated_at)
    .fetch_one(pool)
    .await
    .map_err(try_op("replace native schedule configuration"))
}

/// Observe overdue intervals once and publish one coalesced rescan transactionally.
///
/// # Errors
/// Rejects stale authority or disabled schedule mode and propagates storage failures.
/// `None` means no configured cadence or no due interval.
pub async fn observe_schedule_due(
    pool: &PgPool,
    fence: &super::rescan::RescanFence<'_>,
) -> Result<Option<i64>> {
    sqlx::query_scalar("SELECT media_discovery_schedule_observe_due_v1($1,$2,$3,$4,$5)")
        .bind(fence.association)
        .bind(fence.version)
        .bind(fence.generation)
        .bind(fence.generation_sha256.as_slice())
        .bind(fence.trigger)
        .fetch_one(pool)
        .await
        .map_err(try_op("observe fenced native schedule due intervals"))
}
