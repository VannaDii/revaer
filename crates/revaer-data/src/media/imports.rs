//! Stored-procedure access for portable media configuration import drafts.

use crate::error::{Result, try_op};
use chrono::{DateTime, Utc};
use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

/// Caller-owned transaction used for one atomic media configuration import.
pub type MediaImportTransaction<'a> = sqlx::Transaction<'a, Postgres>;

const MEDIA_PROFILE_IMPORT_DRAFT_UPSERT_V1: &str = "SELECT media_profile_import_draft_upsert_v1(actor_public_id_input => $1, profile_key_input => $2, source_root_input => $3, output_root_input => $4, source_root_resolved_input => $5, output_root_resolved_input => $6, retention_days_input => $7, compatibility_target_key_input => $8, desired_target_key_input => $9, desired_target_version_input => $10, policy_key_input => $11)";
const MEDIA_PROFILE_IMPORT_DRAFT_DELETE_V1: &str =
    "SELECT media_profile_import_draft_delete_v1(profile_key_input => $1)";
const MEDIA_PROFILE_IMPORT_DRAFT_LIST_V1: &str = "SELECT media_profile_import_draft_public_id, profile_key, source_root, output_root, source_root_resolved, output_root_resolved, retention_days, compatibility_target_key, desired_target_key, desired_target_version, policy_key, updated_at FROM media_profile_import_draft_list_v1()";

/// Input for a disabled profile draft with unresolved local paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpsertMediaProfileImportDraftInput<'a> {
    /// Actor performing the import.
    pub actor_public_id: Uuid,
    /// Stable profile key.
    pub profile_key: &'a str,
    /// Source-root path or portable token.
    pub source_root: &'a str,
    /// Output-root path or portable token.
    pub output_root: &'a str,
    /// Whether the source root is mapped locally.
    pub source_root_resolved: bool,
    /// Whether the output root is mapped locally.
    pub output_root_resolved: bool,
    /// Retention in days.
    pub retention_days: i32,
    /// Optional compatibility target key.
    pub compatibility_target_key: Option<&'a str>,
    /// Optional immutable desired-target key.
    pub desired_target_key: Option<&'a str>,
    /// Optional immutable desired-target version.
    pub desired_target_version: Option<i32>,
    /// Policy catalog key.
    pub policy_key: &'a str,
}

/// Persisted disabled profile-import draft.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaProfileImportDraftRow {
    /// Draft public id.
    pub media_profile_import_draft_public_id: Uuid,
    /// Stable profile key.
    pub profile_key: String,
    /// Source-root path or portable token.
    pub source_root: String,
    /// Output-root path or portable token.
    pub output_root: String,
    /// Whether the source root is mapped locally.
    pub source_root_resolved: bool,
    /// Whether the output root is mapped locally.
    pub output_root_resolved: bool,
    /// Retention in days.
    pub retention_days: i32,
    /// Optional compatibility target key.
    pub compatibility_target_key: Option<String>,
    /// Optional immutable desired-target key.
    pub desired_target_key: Option<String>,
    /// Optional immutable desired-target version.
    pub desired_target_version: Option<i32>,
    /// Policy catalog key.
    pub policy_key: String,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Create or replace one disabled import draft in a caller-owned transaction.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn upsert_media_profile_import_draft_with_executor<'e, E>(
    executor: E,
    input: UpsertMediaProfileImportDraftInput<'_>,
) -> Result<Uuid>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, Uuid>(MEDIA_PROFILE_IMPORT_DRAFT_UPSERT_V1)
        .bind(input.actor_public_id)
        .bind(input.profile_key)
        .bind(input.source_root)
        .bind(input.output_root)
        .bind(input.source_root_resolved)
        .bind(input.output_root_resolved)
        .bind(input.retention_days)
        .bind(input.compatibility_target_key)
        .bind(input.desired_target_key)
        .bind(input.desired_target_version)
        .bind(input.policy_key)
        .fetch_one(executor)
        .await
        .map_err(try_op("media profile import draft upsert"))
}

/// Delete a resolved import draft in a caller-owned transaction.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn delete_media_profile_import_draft_with_executor<'e, E>(
    executor: E,
    profile_key: &str,
) -> Result<bool>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, bool>(MEDIA_PROFILE_IMPORT_DRAFT_DELETE_V1)
        .bind(profile_key)
        .fetch_one(executor)
        .await
        .map_err(try_op("media profile import draft delete"))
}

/// List disabled import drafts awaiting local path mapping.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_profile_import_drafts(
    pool: &PgPool,
) -> Result<Vec<MediaProfileImportDraftRow>> {
    sqlx::query_as::<_, MediaProfileImportDraftRow>(MEDIA_PROFILE_IMPORT_DRAFT_LIST_V1)
        .fetch_all(pool)
        .await
        .map_err(try_op("media profile import draft list"))
}

#[cfg(test)]
mod tests {
    use super::{
        UpsertMediaProfileImportDraftInput, delete_media_profile_import_draft_with_executor,
        list_media_profile_import_drafts, upsert_media_profile_import_draft_with_executor,
    };
    use crate::media::schema_tests::setup_media_db;

    #[tokio::test]
    async fn unresolved_import_draft_round_trips_and_deletes_transactionally() -> anyhow::Result<()>
    {
        let db = setup_media_db("media YAML import draft round trip").await?;
        let mut transaction = db.pool().begin().await?;
        let draft_id = upsert_media_profile_import_draft_with_executor(
            &mut *transaction,
            UpsertMediaProfileImportDraftInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "portable-library",
                source_root: "${revaer.source_root:portable-library}",
                output_root: "/mapped/output",
                source_root_resolved: false,
                output_root_resolved: true,
                retention_days: 30,
                compatibility_target_key: Some("web"),
                desired_target_key: None,
                desired_target_version: None,
                policy_key: "safe_dry_run",
            },
        )
        .await?;
        transaction.commit().await?;

        let drafts = list_media_profile_import_drafts(db.pool()).await?;
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].media_profile_import_draft_public_id, draft_id);
        assert!(!drafts[0].source_root_resolved);
        assert!(drafts[0].output_root_resolved);

        let mut transaction = db.pool().begin().await?;
        assert!(
            delete_media_profile_import_draft_with_executor(&mut *transaction, "portable-library",)
                .await?
        );
        transaction.commit().await?;
        assert!(
            list_media_profile_import_drafts(db.pool())
                .await?
                .is_empty()
        );
        Ok(())
    }
}
