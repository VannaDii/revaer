//! Atomic immutable association creation and path-free stored-procedure reads.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Result, try_op};
use crate::transaction::{begin_transaction_with, finish_transaction, release_connection};

const CREATE: &str = "SELECT media_discovery_association_create_v1(actor_public_id_input => $1, association_key_input => $2, media_profile_public_id_input => $3, profile_version_input => $4, source_root_key_input => $5, root_relative_path_input => $6, manual_enabled_input => $7, watcher_enabled_input => $8, schedule_enabled_input => $9)";
const GET: &str = "SELECT * FROM media_discovery_association_get_v1(public_id_input => $1)";

/// Read a bounded page plus one lookahead using one database snapshot.
///
/// # Errors
/// Rejects invalid bounds, cursor membership and persisted representations.
pub async fn read_association_page(
    pool: &PgPool,
    limit: u16,
    key: Option<&str>,
    id: Option<Uuid>,
) -> Result<Vec<AssociationRow>> {
    try_op("read discovery association page", || { sqlx::query_as("SELECT * FROM media_discovery_association_page_v1(limit_input => $1, cursor_key_input => $2, cursor_id_input => $3)")
        .bind(i32::from(limit)).bind(key).bind(id).fetch_all(pool) }).await
}

/// Exact validated association fields; no inferred version, scope or mode.
pub struct CreateAssociationInput<'a> {
    /// Authenticated operation actor.
    pub actor_public_id: Uuid,
    /// Immutable logical association key.
    pub association_key: &'a str,
    /// Selected profile parent.
    pub media_profile_public_id: Uuid,
    /// Exact active profile version.
    pub profile_version: i32,
    /// Catalog source key, never a path.
    pub source_root_key: &'a str,
    /// Explicit relative prefix, including explicit whole-root empty value.
    pub root_relative_path: &'a str,
    /// Explicit manual mode.
    pub manual_enabled: bool,
    /// Requested watcher mode, subject to activation qualification.
    pub watcher_enabled: bool,
    /// Requested schedule mode, subject to activation qualification.
    pub schedule_enabled: bool,
}

/// Complete path-free latest association representation in one snapshot.
#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct AssociationRow {
    /// Stable public identity.
    pub media_discovery_association_public_id: Uuid,
    /// Immutable logical key.
    pub association_key: String,
    /// Latest immutable version.
    pub latest_version: i32,
    /// Current operational version, if any.
    pub active_version: Option<i32>,
    /// Exact profile parent reference.
    pub media_profile_public_id: Uuid,
    /// Exact profile version reference.
    pub profile_version: i32,
    /// Logical source key.
    pub source_root_key: String,
    /// Explicit relative prefix.
    pub root_relative_path: String,
    /// Explicit discovery intent, independently qualified from readiness.
    #[sqlx(flatten)]
    pub modes: AssociationModes,
    /// Closed immutable lifecycle state.
    pub lifecycle_state: String,
    /// Closed binding resolution state.
    pub resolution_state: String,
    /// Current binding readiness, not historical activation alone.
    pub binding_ready: bool,
    /// Bounded reason when binding is unavailable.
    pub binding_reason: Option<String>,
    /// Current destructive readiness, independent of manual intent.
    pub destructive_ready: bool,
    /// Bounded reason when destructive work is unavailable.
    pub destructive_reason: Option<String>,
    /// Parent creation time.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Combined referenced profile and output-policy dry-run restrictions.
    pub effective_dry_run: bool,
}

/// Persisted discovery intent without conflating it with execution readiness.
#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct AssociationModes {
    /// Manual discovery intent.
    pub manual_enabled: bool,
    /// Watcher discovery intent.
    pub watcher_enabled: bool,
    /// Scheduled discovery intent.
    pub schedule_enabled: bool,
}

/// Read the latest association using its public identity.
///
/// # Errors
/// Propagates storage and decoding failures; an unknown identity returns `None`.
pub async fn read_association(pool: &PgPool, id: Uuid) -> Result<Option<AssociationRow>> {
    try_op("read discovery association", || {
        sqlx::query_as(GET).bind(id).fetch_optional(pool)
    })
    .await
}

/// Create and read version 1 in one serializable transaction.
///
/// # Errors
/// Propagates contract, overlap, privilege and transaction failures. Only
/// definitive serialization/deadlock failures are retried, at most twice.
pub async fn create_association(
    pool: &PgPool,
    input: &CreateAssociationInput<'_>,
) -> Result<AssociationRow> {
    let pool = pool.clone();
    let actor_public_id = input.actor_public_id;
    let association_key = input.association_key.to_owned();
    let media_profile_public_id = input.media_profile_public_id;
    let profile_version = input.profile_version;
    let source_root_key = input.source_root_key.to_owned();
    let root_relative_path = input.root_relative_path.to_owned();
    let manual_enabled = input.manual_enabled;
    let watcher_enabled = input.watcher_enabled;
    let schedule_enabled = input.schedule_enabled;
    // The worker owns BEGIN and cleanup even when its caller stops waiting.
    tokio::spawn(async move {
        let input = CreateAssociationInput {
            actor_public_id,
            association_key: &association_key,
            media_profile_public_id,
            profile_version,
            source_root_key: &source_root_key,
            root_relative_path: &root_relative_path,
            manual_enabled,
            watcher_enabled,
            schedule_enabled,
        };

        let mut retries = 0;
        loop {
            let result = create_once(&pool, &input).await;
            if let Err(error) = &result
                && retries < 2
                && matches!(error.database_code().as_deref(), Some("40001" | "40P01"))
            {
                retries += 1;
                continue;
            }
            return result;
        }
    })
    .await
    .map_err(|source| crate::error::DataError::BoundaryFailed {
        operation: "create_association",
        source: Box::new(source),
    })?
}

async fn create_once(pool: &PgPool, input: &CreateAssociationInput<'_>) -> Result<AssociationRow> {
    let mut connection = try_op("transaction acquire", || pool.acquire()).await?;
    let completed = async {
        let mut transaction =
            begin_transaction_with(&mut connection, "BEGIN ISOLATION LEVEL SERIALIZABLE").await?;
        let row = try_op("create discovery association", || async {
            let write = sqlx::query_scalar::<_, Uuid>(CREATE)
                .bind(input.actor_public_id)
                .bind(input.association_key)
                .bind(input.media_profile_public_id)
                .bind(input.profile_version)
                .bind(input.source_root_key)
                .bind(input.root_relative_path)
                .bind(input.manual_enabled)
                .bind(input.watcher_enabled)
                .bind(input.schedule_enabled)
                .fetch_one(&mut *transaction)
                .await;
            match write {
                Ok(id) => {
                    sqlx::query_as(GET)
                        .bind(id)
                        .fetch_one(&mut *transaction)
                        .await
                }
                Err(error) => Err(error),
            }
        })
        .await;
        finish_transaction(transaction, row).await
    }
    .await;
    release_connection(connection, completed).await
}
