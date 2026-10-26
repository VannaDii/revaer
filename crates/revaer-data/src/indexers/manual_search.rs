//! Stored-procedure boundary for owned, live manual-search attempts.
//!
//! Claims retain the creation procedure's eligibility and policy snapshot. A
//! per-attempt token fences subsequent writes; the outbound correlation remains
//! stable across retries. Plain credentials never implement diagnostic output.

use sqlx::{FromRow, PgPool};
use std::future::Future;
use std::time::Duration;
use uuid::Uuid;

use super::search_results::SearchResultIngestInput;
use crate::error::{Result, try_op};

const CLAIM: &str = r"
    SELECT * FROM manual_search_claim_next_v1()
";

const ACTIVE: &str = r"
    SELECT manual_search_active_v1(
        search_request_public_id_input => $1,
        indexer_instance_public_id_input => $2,
        claim_token_input => $3
    )
";

const COMPLETE: &str = r"
    SELECT manual_search_finish_v1(
        search_request_public_id_input => $1,
        indexer_instance_public_id_input => $2,
        claim_token_input => $3,
        error_class_input => $4::error_class,
        canceled_input => $5,
        http_status_input => $6,
        latency_ms_input => $7,
        result_count_input => $8,
        retry_after_seconds_input => $9
    )
";

const INGEST: &str = r"
    SELECT manual_search_ingest_v1(
        search_request_public_id_input => $1,
        indexer_instance_public_id_input => $2,
        source_guid_input => $3,
        details_url_input => $4,
        download_url_input => $5,
        magnet_uri_input => $6,
        title_raw_input => $7,
        size_bytes_input => $8,
        infohash_v1_input => $9,
        infohash_v2_input => $10,
        magnet_hash_input => $11,
        seeders_input => $12,
        leechers_input => $13,
        published_at_input => $14,
        uploader_input => $15,
        observed_at_input => $16,
        attr_keys_input => $17::observation_attr_key[],
        attr_types_input => $18::attr_value_type[],
        attr_value_text_input => $19,
        attr_value_int_input => $20,
        attr_value_bigint_input => $21,
        attr_value_numeric_input => $22::numeric[],
        attr_value_bool_input => $23,
        attr_value_uuid_input => $24,
        claim_token_input => $25,
        details_private_input => $26,
        download_private_input => $27,
        magnet_private_input => $28
    )
";

/// Expected ownership outcomes; database errors are returned separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualSearchWriteOutcome {
    /// This owner applied the operation.
    Applied,
    /// Cancellation, completion or another attempt superseded this owner.
    OwnershipLost,
}

/// Classified attempt outcome without upstream text or credential-bearing URLs.
#[derive(Debug, Clone, Copy)]
pub enum ManualSearchOutcome {
    /// A recognized response, including a valid empty feed.
    Success,
    /// An existing database error-class label.
    Failure(&'static str),
    /// Cooperative shutdown interrupted the owned execution.
    Canceled,
}

/// Bounded outbound measurements passed to the existing attempt ledger.
#[derive(Debug, Clone, Copy)]
pub struct ManualSearchCompletion {
    /// Classified outcome; raw upstream messages are deliberately not accepted.
    pub outcome: ManualSearchOutcome,
    /// HTTP status when one was received.
    pub http_status: Option<i32>,
    /// Measured elapsed milliseconds.
    pub latency_ms: i32,
    /// Parsed result count on success.
    pub result_count: i32,
    /// Optional upstream retry delay, interpreted by the existing backoff procedure.
    pub retry_after_seconds: Option<i32>,
}

use super::connection::ConnectionRow;
pub use super::connection::{
    ManualSearchConfigurationError, ManualSearchConnection, ManualSearchProxy, ManualSearchRouting,
};

/// One atomically owned search attempt, consumed by the production executor.
pub struct ManualSearchRun {
    /// Existing request identity.
    pub search_request_public_id: Uuid,
    /// Selected source identity.
    pub indexer_instance_public_id: Uuid,
    /// Unique attempt ownership; never substitute the stable correlation ID.
    pub claim_token: Uuid,
    /// Stable outbound correlation for this logical fetch across retries.
    pub correlation_id: Uuid,
    /// Existing bounded retry sequence, excluding rate-limit deferrals.
    pub retry_seq: i16,
    /// Query normalized by the creation procedure.
    pub query_text: String,
    /// Persisted Torznab mode; absence retains the ordinary manual-search mode.
    pub torznab_mode: Option<String>,
    /// Normalized identifier type and value, when the request supplies one.
    pub identifier: Option<(String, String)>,
    /// Optional TV season, including specials at season zero.
    pub season_number: Option<i32>,
    /// Optional TV episode; the creation procedure validates its season anchor.
    pub episode_number: Option<i32>,
    /// Effective Torznab categories after profile and policy filtering.
    pub category_ids: Vec<i32>,
    /// Configured connection deadline.
    pub connect_timeout_ms: i32,
    /// Configured response deadline.
    pub read_timeout_ms: i32,
    /// Optional explicit route for outbound attempt logging.
    pub routing_policy_public_id: Option<Uuid>,
    /// Configuration errors remain explicit and are completed without network I/O.
    pub connection: std::result::Result<ManualSearchConnection, ManualSearchConfigurationError>,
}

#[derive(FromRow)]
struct ClaimRow {
    search_request_public_id: Uuid,
    indexer_instance_public_id: Uuid,
    claim_token: Uuid,
    correlation_id: Uuid,
    retry_seq: i16,
    query_text: String,
    torznab_mode: Option<String>,
    identifier_type: Option<String>,
    identifier_value: Option<String>,
    season_number: Option<i32>,
    episode_number: Option<i32>,
    category_ids: Vec<i32>,
    connect_timeout_ms: i32,
    read_timeout_ms: i32,
    routing_policy_public_id: Option<Uuid>,
    #[sqlx(flatten)]
    connection: ConnectionRow,
}

impl ClaimRow {
    fn into_run(self) -> ManualSearchRun {
        let connection = self.connection.resolve();
        ManualSearchRun {
            search_request_public_id: self.search_request_public_id,
            indexer_instance_public_id: self.indexer_instance_public_id,
            claim_token: self.claim_token,
            correlation_id: self.correlation_id,
            retry_seq: self.retry_seq,
            query_text: self.query_text,
            torznab_mode: self.torznab_mode,
            identifier: self.identifier_type.zip(self.identifier_value),
            season_number: self.season_number,
            episode_number: self.episode_number,
            category_ids: self.category_ids,
            connect_timeout_ms: self.connect_timeout_ms,
            read_timeout_ms: self.read_timeout_ms,
            routing_policy_public_id: self.routing_policy_public_id,
            connection,
        }
    }
}

/// Claim one due run; `None` means no executable attempt is currently available.
///
/// # Errors
/// Returns database errors without exposing connection configuration or secrets.
pub async fn claim_next(pool: &PgPool) -> Result<Option<ManualSearchRun>> {
    try_op("manual search claim", || async {
        claim_query(|| sqlx::query_as::<_, ClaimRow>(CLAIM).fetch_optional(pool))
            .await
            .map(|row| row.map(ClaimRow::into_run))
    })
    .await
}

async fn claim_query<F, Fut>(mut query: F) -> sqlx::Result<Option<ClaimRow>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = sqlx::Result<Option<ClaimRow>>>,
{
    let mut attempt = 1_u32;
    loop {
        match query().await {
            Err(error)
                if attempt < 3
                    && matches!(
                        error
                            .as_database_error()
                            .and_then(sqlx::error::DatabaseError::code)
                            .as_deref(),
                        Some("40P01" | "40001")
                    ) =>
            {
                // PostgreSQL rolled back the complete claim statement. Retrying
                // cannot retain a token, consume a budget or duplicate an attempt.
                tracing::warn!(
                    attempt,
                    "manual search claim rolled back by contention; retrying"
                );
                tokio::time::sleep(Duration::from_millis(100 * u64::from(attempt))).await;
                attempt += 1;
            }
            result => return result,
        }
    }
}

/// Observe whether this attempt still owns the live request.
///
/// # Errors
/// Returns database errors; cancellation is the expected `false` outcome.
pub async fn is_active(pool: &PgPool, run: &ManualSearchRun) -> Result<bool> {
    try_op("manual search ownership", || {
        sqlx::query_scalar(ACTIVE)
            .bind(run.search_request_public_id)
            .bind(run.indexer_instance_public_id)
            .bind(run.claim_token)
            .fetch_one(pool)
    })
    .await
}

/// Settle an owned attempt atomically with its outbound log and existing backoff.
///
/// # Errors
/// Returns database errors without exposing credentials. Stale ownership is explicit.
pub async fn complete(
    pool: &PgPool,
    run: &ManualSearchRun,
    completion: ManualSearchCompletion,
) -> Result<ManualSearchWriteOutcome> {
    let error_class = match completion.outcome {
        ManualSearchOutcome::Success => None,
        ManualSearchOutcome::Failure(class) => Some(class),
        ManualSearchOutcome::Canceled => Some("unknown"),
    };
    let applied: bool = try_op("manual search completion", || {
        sqlx::query_scalar(COMPLETE)
            .bind(run.search_request_public_id)
            .bind(run.indexer_instance_public_id)
            .bind(run.claim_token)
            .bind(error_class)
            .bind(matches!(completion.outcome, ManualSearchOutcome::Canceled))
            .bind(completion.http_status)
            .bind(completion.latency_ms)
            .bind(completion.result_count)
            .bind(completion.retry_after_seconds)
            .fetch_one(pool)
    })
    .await?;
    Ok(if applied {
        ManualSearchWriteOutcome::Applied
    } else {
        ManualSearchWriteOutcome::OwnershipLost
    })
}

/// Visibility of upstream URLs; private links remain available for acquisition.
#[derive(Clone, Copy, Default)]
pub struct ManualSearchLinkPrivacy {
    /// The details URL contains authentication material.
    pub details: bool,
    /// The download URL contains authentication material.
    pub download: bool,
    /// The magnet URI contains authentication material.
    pub magnet: bool,
}

/// Ingest one observation only while its attempt still owns the request.
///
/// # Errors
/// Returns database errors. Cancellation and stale ownership perform no writes.
pub async fn ingest(
    pool: &PgPool,
    run: &ManualSearchRun,
    input: &SearchResultIngestInput<'_>,
    privacy: ManualSearchLinkPrivacy,
) -> Result<ManualSearchWriteOutcome> {
    let applied: bool = try_op("owned manual search ingest", || {
        sqlx::query_scalar(INGEST)
            .bind(input.search_request_public_id)
            .bind(input.indexer_instance_public_id)
            .bind(input.source_guid)
            .bind(input.details_url)
            .bind(input.download_url)
            .bind(input.magnet_uri)
            .bind(input.title_raw)
            .bind(input.size_bytes)
            .bind(input.infohash_v1)
            .bind(input.infohash_v2)
            .bind(input.magnet_hash)
            .bind(input.seeders)
            .bind(input.leechers)
            .bind(input.published_at)
            .bind(input.uploader)
            .bind(input.observed_at)
            .bind(input.attr_keys)
            .bind(input.attr_types)
            .bind(input.attr_value_text)
            .bind(input.attr_value_int)
            .bind(input.attr_value_bigint)
            .bind(input.attr_value_numeric)
            .bind(input.attr_value_bool)
            .bind(input.attr_value_uuid)
            .bind(run.claim_token)
            .bind(privacy.details)
            .bind(privacy.download)
            .bind(privacy.magnet)
            .fetch_one(pool)
    })
    .await?;
    Ok(if applied {
        ManualSearchWriteOutcome::Applied
    } else {
        ManualSearchWriteOutcome::OwnershipLost
    })
}

#[cfg(test)]
mod tests;
