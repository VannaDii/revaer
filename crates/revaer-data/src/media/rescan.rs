//! Bounded normalized rescan requests and captured-sequence acknowledgements.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Result, try_op};

/// Current immutable-version request and one of its seven closed reasons.
#[derive(sqlx::FromRow)]
pub struct RescanStateRow {
    /// Exact association version owning the request.
    pub association_version: i32,
    /// Requested high-water mark; reading it does not acknowledge it.
    pub requested_sequence: i64,
    /// Last completely observed request high-water mark.
    pub satisfied_sequence: i64,
    /// Closed persisted reason.
    pub reason_code: String,
    /// Latest request carrying this reason; acknowledgements never erase it.
    pub last_requested_sequence: i64,
}

/// Read the latest association's bounded, coalesced reasons in one snapshot.
///
/// # Errors
/// Propagates storage/decoding failures. Empty means no current request.
pub async fn read_rescan_state(pool: &PgPool, id: Uuid) -> Result<Vec<RescanStateRow>> {
    sqlx::query_as("SELECT * FROM media_discovery_rescan_get_v1($1)")
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(try_op("read coalesced discovery rescan request"))
}

/// Exact association and retained catalog used by one native scan.
pub struct RescanFence<'a> {
    /// Stable logical association identity.
    pub association: Uuid,
    /// Immutable active association version.
    pub version: i32,
    /// Retained catalog generation, never inferred from a logical root name.
    pub generation: i64,
    /// Retained catalog's complete generation digest.
    pub generation_sha256: [u8; 32],
    /// Server-selected watcher or schedule mode; the procedure rechecks it.
    pub trigger: &'a str,
}

/// Coalesce one uncertainty/restart request under the exact current fences.
///
/// # Errors
/// Rejects stale, inactive, unready or disabled bindings and invalid reasons.
pub async fn request_rescan(pool: &PgPool, fence: &RescanFence<'_>, reason: &str) -> Result<i64> {
    sqlx::query_scalar("SELECT media_discovery_rescan_request_v1($1,$2,$3,$4,$5,$6)")
        .bind(fence.association)
        .bind(fence.version)
        .bind(fence.generation)
        .bind(fence.generation_sha256.as_slice())
        .bind(fence.trigger)
        .bind(reason)
        .fetch_one(pool)
        .await
        .map_err(try_op("request fenced native discovery rescan"))
}

/// Acknowledge only the high-water mark captured before a complete clean scan.
///
/// # Errors
/// Rejects changed fences and missing, invalid or future request sequences.
pub async fn satisfy_rescan(pool: &PgPool, fence: &RescanFence<'_>, sequence: i64) -> Result<()> {
    sqlx::query("SELECT media_discovery_rescan_satisfy_v1($1,$2,$3,$4,$5,$6)")
        .bind(fence.association)
        .bind(fence.version)
        .bind(fence.generation)
        .bind(fence.generation_sha256.as_slice())
        .bind(fence.trigger)
        .bind(sequence)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(try_op("acknowledge captured native discovery rescan"))
}

/// Presence observed during a captured scan, without acknowledging completion.
///
/// # Errors
/// Rejects stale authority, acknowledged sequences and invalid/beyond-cap paths.
pub async fn observe_rescan_paths(
    pool: &PgPool,
    fence: &RescanFence<'_>,
    sequence: i64,
    paths: &[String],
) -> Result<()> {
    sqlx::query("SELECT media_discovery_rescan_observe_paths_v1($1,$2,$3,$4,$5,$6,$7)")
        .bind(fence.association)
        .bind(fence.version)
        .bind(fence.generation)
        .bind(fence.generation_sha256.as_slice())
        .bind(fence.trigger)
        .bind(sequence)
        .bind(paths)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(try_op("observe captured native discovery source presence"))
}

/// Diagnostic absence evidence; it never authorizes source or job deletion.
#[derive(sqlx::FromRow)]
pub struct SourceDiagnosticRow {
    /// Relative source name in the immutable association namespace.
    pub source_path: String,
    /// Last actual present observation, including unchanged fingerprints.
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    /// Zero, one tentative observation, or two qualifying clean observations.
    pub absence_observations: i16,
    /// True only after two clean observations at least one second apart.
    pub diagnostic_absent: bool,
}

/// Read latest-association source diagnostics without obtaining deletion authority.
///
/// # Errors
/// Propagates storage/decoding failures; `None` means the path has no recorded fingerprint.
pub async fn read_source_diagnostics(
    pool: &PgPool,
    id: Uuid,
    path: &str,
) -> Result<Option<SourceDiagnosticRow>> {
    sqlx::query_as("SELECT * FROM media_discovery_source_diagnostics_get_v1($1,$2)")
        .bind(id)
        .bind(path)
        .fetch_optional(pool)
        .await
        .map_err(try_op("read source absence diagnostics"))
}
