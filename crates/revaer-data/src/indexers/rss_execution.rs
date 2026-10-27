//! Private execution context for scheduled RSS, sharing search connection resolution.

use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::connection::ConnectionRow;
use super::manual_search::{ManualSearchConfigurationError, ManualSearchConnection};
use crate::error::{Result, try_op};

#[derive(FromRow)]
struct ContextRow {
    rate_limit_denied_scope: Option<String>,
    #[sqlx(flatten)]
    connection: ConnectionRow,
}

/// Resolved context, including a budget denial that forbids outbound HTTP.
pub struct RssExecutionContext {
    /// Scope whose shared token bucket denied this poll.
    pub rate_limit_denied_scope: Option<String>,
    /// Credential-safe configuration result; deliberately has no Debug representation.
    pub connection: std::result::Result<ManualSearchConnection, ManualSearchConfigurationError>,
}

/// Resolve one still-owned poll and charge existing outbound budgets atomically.
///
/// # Errors
/// Propagates database failures; absence means ownership or eligibility was withdrawn.
pub async fn resolve(
    pool: &PgPool,
    subscription: i64,
    correlation: Uuid,
) -> Result<Option<RssExecutionContext>> {
    let row: Option<ContextRow> = try_op("resolve RSS execution", || {
        sqlx::query_as("SELECT * FROM rss_poll_context_v1(rss_subscription_id_input => $1, correlation_id_input => $2)")
            .bind(subscription).bind(correlation).fetch_optional(pool)
    }).await?;
    Ok(row.map(|row| RssExecutionContext {
        rate_limit_denied_scope: row.rate_limit_denied_scope,
        connection: row.connection.resolve(),
    }))
}

/// Advance the deployment schedule after its bounded batch has settled.
///
/// # Errors
/// Propagates database errors; the deployment lease recovers an interrupted batch.
pub async fn complete_job(pool: &PgPool) -> Result<()> {
    let _: bool = try_op("complete RSS job", || {
        sqlx::query_scalar("SELECT job_schedule_mark_completed(job_key_input => 'rss_poll')")
            .fetch_one(pool)
    })
    .await?;
    Ok(())
}
