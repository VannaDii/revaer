//! Stored-procedure access for media profile management.

use crate::error::{Result, try_op};
use sqlx::PgPool;
use uuid::Uuid;

const MEDIA_PROFILE_UPSERT_V1: &str = "SELECT media_profile_upsert_v1($1, $2, $3, $4, $5, $6)";
const MEDIA_PROFILE_LIST_V1: &str = "SELECT media_profile_public_id, profile_key, source_root, output_root, dry_run_only, retention_days, updated_at FROM media_profile_list_v1()";

/// Input payload for media profile upsert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertMediaProfileInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Unique profile key.
    pub profile_key: &'a str,
    /// Source root path.
    pub source_root: &'a str,
    /// Output root path.
    pub output_root: &'a str,
    /// Dry-run-only policy.
    pub dry_run_only: bool,
    /// Retention days.
    pub retention_days: i32,
}

/// Row returned by profile listing.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaProfileRow {
    /// Profile id.
    pub media_profile_public_id: Uuid,
    /// Profile key.
    pub profile_key: String,
    /// Source path root.
    pub source_root: String,
    /// Output path root.
    pub output_root: String,
    /// Dry-run only flag.
    pub dry_run_only: bool,
    /// Retention days.
    pub retention_days: i32,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Create or update a media profile.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn upsert_media_profile(
    pool: &PgPool,
    input: &UpsertMediaProfileInput<'_>,
) -> Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(MEDIA_PROFILE_UPSERT_V1)
        .bind(input.actor_public_id)
        .bind(input.profile_key)
        .bind(input.source_root)
        .bind(input.output_root)
        .bind(input.dry_run_only)
        .bind(input.retention_days)
        .fetch_one(pool)
        .await
        .map_err(try_op("media profile upsert"))
}

/// List active media profiles.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_profiles(pool: &PgPool) -> Result<Vec<MediaProfileRow>> {
    sqlx::query_as::<_, MediaProfileRow>(MEDIA_PROFILE_LIST_V1)
        .fetch_all(pool)
        .await
        .map_err(try_op("media profile list"))
}

#[cfg(test)]
mod tests {
    use super::{UpsertMediaProfileInput, list_media_profiles, upsert_media_profile};
    use crate::DataError;
    use crate::media::schema_tests::setup_media_db;

    #[tokio::test]
    async fn upsert_and_list_media_profile() -> anyhow::Result<()> {
        let db = match setup_media_db("upsert_and_list_media_profile").await {
            Ok(db) => db,
            Err(err) => {
                eprintln!("skipping upsert_and_list_media_profile: {err}");
                return Ok(());
            }
        };
        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-main",
                source_root: "/input/tv",
                output_root: "/output/tv",
                dry_run_only: true,
                retention_days: 30,
            },
        )
        .await?;

        let rows = list_media_profiles(db.pool()).await?;
        assert!(
            rows.iter()
                .any(|item| item.media_profile_public_id == profile_id)
        );
        Ok(())
    }

    #[tokio::test]
    async fn reject_overlapping_roots() -> anyhow::Result<()> {
        let db = match setup_media_db("reject_overlapping_roots").await {
            Ok(db) => db,
            Err(err) => {
                eprintln!("skipping reject_overlapping_roots: {err}");
                return Ok(());
            }
        };
        let result = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-overlap",
                source_root: "/input/tv",
                output_root: "/input/tv",
                dry_run_only: false,
                retention_days: 30,
            },
        )
        .await;

        let err = result.expect_err("expected overlap validation error");
        assert!(matches!(err, DataError::QueryFailed { .. }));
        assert_eq!(err.database_detail(), Some("media_profile_roots_overlap"));

        Ok(())
    }
}
