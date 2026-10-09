//! Snapshot-consistent, path-free reads of immutable profile versions.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Result, try_op};

const GET: &str = "SELECT media_profile_public_id, profile_key, display_name, description, enabled, dry_run_only, desired_target_key, desired_target_version, policy_key, policy_version, latest_version, active_version, lifecycle_state, created_at, updated_at, root_kind, logical_key, resolution_state, binding_ready, binding_reason, destructive_ready, destructive_reason FROM media_profile_version_get_v1(media_profile_public_id_input => $1)";
const PAGE: &str = "SELECT * FROM media_profile_version_page_v1(limit_input => $1, cursor_key_input => $2, cursor_id_input => $3)";

/// Read one bounded complete-profile page plus one lookahead parent.
///
/// # Errors
/// Propagates invalid bounds, cursor membership, privilege and decoding errors.
pub async fn read_profile_page(
    pool: &PgPool,
    limit: u16,
    cursor_key: Option<&str>,
    cursor_id: Option<Uuid>,
) -> Result<Vec<ProfileVersionRow>> {
    sqlx::query_as(PAGE)
        .bind(i32::from(limit))
        .bind(cursor_key)
        .bind(cursor_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("read profile version page"))
}
const CREATE: &str = "SELECT media_profile_version_create_v1(actor_public_id_input => $1, profile_key_input => $2, display_name_input => $3, description_input => $4, enabled_input => $5, dry_run_only_input => $6, desired_target_key_input => $7, desired_target_version_input => $8, policy_key_input => $9, policy_version_input => $10, output_root_key_input => $11, workspace_root_key_input => $12, backup_root_key_input => $13, quarantine_root_key_input => $14)";
const REPLACE: &str = "SELECT media_profile_version_replace_v1(actor_public_id_input => $1, profile_key_input => $2, display_name_input => $3, description_input => $4, enabled_input => $5, dry_run_only_input => $6, desired_target_key_input => $7, desired_target_version_input => $8, policy_key_input => $9, policy_version_input => $10, output_root_key_input => $11, workspace_root_key_input => $12, backup_root_key_input => $13, quarantine_root_key_input => $14, media_profile_public_id_input => $15, expected_version_input => $16)";

#[derive(Clone, Copy)]
enum ProfileWrite {
    Create,
    Replace { id: Uuid, expected_version: i32 },
}

/// Complete immutable version creation; paths and implicit versions are absent.
#[derive(Clone, PartialEq, Eq)]
pub struct CreateProfileVersionInput<'a> {
    /// Authenticated operation's actor identity.
    pub actor_public_id: Uuid,
    /// Exact profile key.
    pub profile_key: &'a str,
    /// Exact display name.
    pub display_name: &'a str,
    /// Exact description.
    pub description: &'a str,
    /// Operational enablement.
    pub enabled: bool,
    /// Profile-only dry-run restriction.
    pub dry_run_only: bool,
    /// Exact desired-target key.
    pub desired_target_key: &'a str,
    /// Explicit desired-target version.
    pub desired_target_version: i32,
    /// Exact policy key.
    pub policy_key: &'a str,
    /// Explicit policy version.
    pub policy_version: i32,
    /// Logical output root.
    pub output_root_key: &'a str,
    /// Logical workspace root.
    pub workspace_root_key: &'a str,
    /// Present exactly when policy enables backup.
    pub backup_root_key: Option<&'a str>,
    /// Present exactly when policy enables quarantine.
    pub quarantine_root_key: Option<&'a str>,
}

/// Atomically create version 1 and read its complete persisted representation.
///
/// Only definitive serialization/deadlock failures are retried, at most twice.
/// Other commit failures are propagated rather than risking duplicate writes.
///
/// # Errors
/// Propagates contract, privilege, transaction and decoding failures.
pub async fn create_profile_version(
    pool: &PgPool,
    input: &CreateProfileVersionInput<'_>,
) -> Result<Vec<ProfileVersionRow>> {
    write_profile_version(pool, input, ProfileWrite::Create).await
}

/// Append a complete immutable version only when the latest head still matches.
///
/// # Errors
/// Propagates stale heads, invalid references, transaction and decoding failures.
pub async fn replace_profile_version(
    pool: &PgPool,
    input: &CreateProfileVersionInput<'_>,
    id: Uuid,
    expected_version: i32,
) -> Result<Vec<ProfileVersionRow>> {
    write_profile_version(
        pool,
        input,
        ProfileWrite::Replace {
            id,
            expected_version,
        },
    )
    .await
}

async fn write_profile_version(
    pool: &PgPool,
    input: &CreateProfileVersionInput<'_>,
    operation: ProfileWrite,
) -> Result<Vec<ProfileVersionRow>> {
    let mut retries = 0;
    loop {
        let result = write_once(pool, input, operation).await;
        if let Err(error) = &result
            && retries < 2
            && matches!(error.database_code().as_deref(), Some("40001" | "40P01"))
        {
            retries += 1;
            continue;
        }
        return result;
    }
}

async fn write_once(
    pool: &PgPool,
    input: &CreateProfileVersionInput<'_>,
    operation: ProfileWrite,
) -> Result<Vec<ProfileVersionRow>> {
    let mut transaction = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await
        .map_err(try_op("begin media profile write"))?;
    let query = match operation {
        ProfileWrite::Create => CREATE,
        ProfileWrite::Replace { .. } => REPLACE,
    };
    let write = sqlx::query_scalar::<_, Uuid>(query)
        .bind(input.actor_public_id)
        .bind(input.profile_key)
        .bind(input.display_name)
        .bind(input.description)
        .bind(input.enabled)
        .bind(input.dry_run_only)
        .bind(input.desired_target_key)
        .bind(input.desired_target_version)
        .bind(input.policy_key)
        .bind(input.policy_version)
        .bind(input.output_root_key)
        .bind(input.workspace_root_key)
        .bind(input.backup_root_key)
        .bind(input.quarantine_root_key);
    let write = match operation {
        ProfileWrite::Create => write,
        ProfileWrite::Replace {
            id,
            expected_version,
        } => write.bind(id).bind(expected_version),
    }
    .fetch_one(&mut *transaction)
    .await;
    let rows = match write {
        Ok(id) => {
            sqlx::query_as(GET)
                .bind(id)
                .fetch_all(&mut *transaction)
                .await
        }
        Err(error) => Err(error),
    };
    match rows {
        Ok(rows) => {
            transaction
                .commit()
                .await
                .map_err(try_op("commit media profile write"))?;
            Ok(rows)
        }
        Err(error) => {
            transaction
                .rollback()
                .await
                .map_err(try_op("rollback media profile write"))?;
            Err(try_op("write media profile version")(error))
        }
    }
}

/// One binding row; repeated profile fields come from the same database snapshot.
#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct ProfileVersionRow {
    /// Stable public profile identity.
    pub media_profile_public_id: Uuid,
    /// Exact profile key.
    pub profile_key: String,
    /// Exact submitted display name, including whitespace.
    pub display_name: String,
    /// Exact submitted description.
    pub description: String,
    /// Operational enablement, independent of lifecycle.
    pub enabled: bool,
    /// Profile restriction; cannot override policy dry-run safety.
    pub dry_run_only: bool,
    /// Explicit selected desired-target key.
    pub desired_target_key: String,
    /// Explicit selected desired-target version.
    pub desired_target_version: i32,
    /// Explicit selected policy key.
    pub policy_key: String,
    /// Explicit selected policy version.
    pub policy_version: i32,
    /// Selected immutable version: latest by default, active for an active read.
    pub latest_version: i32,
    /// Active version, absent for an unactivated or archived profile.
    pub active_version: Option<i32>,
    /// Selected immutable version lifecycle.
    pub lifecycle_state: String,
    /// Parent creation time.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Parent last-write time.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Binding role, returned in output-through-quarantine order.
    pub root_kind: String,
    /// Persisted logical root key, never a filesystem path.
    pub logical_key: String,
    /// Immutable binding resolution.
    pub resolution_state: String,
    /// Reported readiness, separate from the profile's safety settings.
    #[sqlx(flatten)]
    pub readiness: ProfileBindingReadiness,
}

/// Current catalog readiness for one persisted binding, not execution authority.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct ProfileBindingReadiness {
    /// Current reported binding readiness; not execution authority.
    pub binding_ready: bool,
    /// Bounded reason, absent when binding-ready.
    pub binding_reason: Option<String>,
    /// Current reported destructive readiness; not execution authority.
    pub destructive_ready: bool,
    /// Bounded reason, absent when destructive-ready.
    pub destructive_reason: Option<String>,
}

impl std::fmt::Debug for ProfileVersionRow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ProfileVersionRow")
    }
}

/// Read the latest profile and all bindings in one statement.
///
/// Empty rows represent an absent or deleted profile; malformed persisted
/// aggregates must be rejected by the typed response constructor, not repaired.
///
/// # Errors
/// Propagates query and row-decoding failures without synthesizing profile data.
pub async fn read_profile_version(
    pool: &PgPool,
    media_profile_public_id: Uuid,
) -> Result<Vec<ProfileVersionRow>> {
    sqlx::query_as(GET)
        .bind(media_profile_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("read media profile version"))
}

/// One read-only snapshot of both heads and their current readiness evidence.
pub struct ProfileReadinessRows {
    /// Latest complete immutable version, including its bindings.
    pub latest: Vec<ProfileVersionRow>,
    /// Active version and bindings, empty only when the active head is absent.
    pub active: Vec<ProfileVersionRow>,
    /// Path-free catalog readiness by kind in the same snapshot.
    pub roots: Vec<super::root_catalog::RootCatalogReadinessRow>,
    /// Active associations across all versions of this logical profile.
    pub active_association_count: i64,
}

/// Read profile readiness without loading retired path-taking parent fields.
///
/// # Errors
/// Propagates privilege, transaction and row-decoding errors.
pub async fn read_profile_readiness(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<ProfileReadinessRows>> {
    let mut transaction = pool
        .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .await
        .map_err(try_op("begin profile readiness read"))?;
    let read: std::result::Result<_, sqlx::Error> = async {
        let latest: Vec<ProfileVersionRow> = sqlx::query_as(GET)
            .bind(id).fetch_all(&mut *transaction).await?;
        if latest.is_empty() {
            return Ok(None);
        }
        let active = sqlx::query_as("SELECT * FROM media_profile_version_get_v1(media_profile_public_id_input => $1, active_input => true)")
            .bind(id).fetch_all(&mut *transaction).await?;
        let roots = sqlx::query_as(super::root_catalog::READINESS)
            .fetch_all(&mut *transaction).await?;
        let active_association_count = sqlx::query_scalar("SELECT media_profile_active_association_count_v1(media_profile_public_id_input => $1)")
            .bind(id).fetch_one(&mut *transaction).await?;
        Ok(Some(ProfileReadinessRows { latest, active, roots, active_association_count }))
    }.await;
    match read {
        Ok(rows) => {
            transaction
                .commit()
                .await
                .map_err(try_op("commit profile readiness read"))?;
            Ok(rows)
        }
        Err(error) => {
            transaction
                .rollback()
                .await
                .map_err(try_op("rollback profile readiness read"))?;
            Err(try_op("read profile readiness")(error))
        }
    }
}
