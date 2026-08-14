//! Stored-procedure access for media profile management.

use crate::error::{Result, try_op};
use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

const MEDIA_PROFILE_UPSERT_V2: &str = "SELECT media_profile_upsert_v2(actor_public_id_input => $1, profile_key_input => $2, source_root_input => $3, output_root_input => $4, dry_run_only_input => $5, retention_days_input => $6, compatibility_target_key_input => $7, policy_key_input => $8, watcher_enabled_input => $9, schedule_enabled_input => $10, schedule_interval_minutes_input => $11)";
const MEDIA_PROFILE_CREATE_V3: &str = "SELECT media_profile_create_v3(actor_public_id_input => $1, profile_key_input => $2, source_requested_path_input => $3, source_canonical_path_input => $4, source_filesystem_device_input => $5, source_filesystem_inode_input => $6, output_requested_path_input => $7, output_canonical_path_input => $8, output_filesystem_device_input => $9, output_filesystem_inode_input => $10, retention_days_input => $11, compatibility_target_key_input => $12, policy_key_input => $13, watcher_enabled_input => $14, schedule_enabled_input => $15, schedule_interval_minutes_input => $16)";
const MEDIA_PROFILE_UPDATE_V1: &str = "SELECT media_profile_update_v1(actor_public_id_input => $1, media_profile_public_id_input => $2, source_root_input => $3, output_root_input => $4, dry_run_only_input => $5, retention_days_input => $6, compatibility_target_key_input => $7, policy_key_input => $8, watcher_enabled_input => $9, schedule_enabled_input => $10, schedule_interval_minutes_input => $11)";
const MEDIA_PROFILE_UPDATE_VERIFIED_V1: &str = "SELECT media_profile_update_verified_v1(actor_public_id_input => $1, media_profile_public_id_input => $2, source_requested_path_input => $3, source_canonical_path_input => $4, source_filesystem_device_input => $5, source_filesystem_inode_input => $6, output_requested_path_input => $7, output_canonical_path_input => $8, output_filesystem_device_input => $9, output_filesystem_inode_input => $10, dry_run_only_input => $11, retention_days_input => $12, compatibility_target_key_input => $13, policy_key_input => $14, watcher_enabled_input => $15, schedule_enabled_input => $16, schedule_interval_minutes_input => $17)";
const MEDIA_PROFILE_LIST_V3: &str = "SELECT media_profile_public_id, profile_key, source_root, output_root, dry_run_only, retention_days, compatibility_target_key, policy_key, watcher_enabled, schedule_enabled, schedule_interval_minutes, desired_target_key, desired_target_version, updated_at FROM media_profile_list_v3()";
const MEDIA_PROFILE_GET_V3: &str = "SELECT media_profile_public_id, profile_key, source_root, output_root, dry_run_only, retention_days, compatibility_target_key, policy_key, watcher_enabled, schedule_enabled, schedule_interval_minutes, desired_target_key, desired_target_version, updated_at FROM media_profile_get_v3(media_profile_public_id_input => $1)";

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
    /// Optional compatibility target key.
    pub compatibility_target_key: Option<&'a str>,
    /// Operational policy key.
    pub policy_key: &'a str,
    /// Whether filesystem watching is enabled.
    pub watcher_enabled: bool,
    /// Whether scheduled discovery is enabled.
    pub schedule_enabled: bool,
    /// Scheduled discovery interval in minutes.
    pub schedule_interval_minutes: Option<i32>,
}

/// Verified filesystem identity payload for creating a media profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateVerifiedMediaProfileInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Unique profile key.
    pub profile_key: &'a str,
    /// Operator-supplied source root.
    pub source_requested_path: &'a str,
    /// Canonical source root.
    pub source_canonical_path: &'a str,
    /// Source filesystem device identity.
    pub source_filesystem_device: i64,
    /// Source filesystem inode identity.
    pub source_filesystem_inode: i64,
    /// Operator-supplied output root.
    pub output_requested_path: &'a str,
    /// Canonical output root.
    pub output_canonical_path: &'a str,
    /// Output filesystem device identity.
    pub output_filesystem_device: i64,
    /// Output filesystem inode identity.
    pub output_filesystem_inode: i64,
    /// Retention duration in days.
    pub retention_days: i32,
    /// Optional compatibility target key.
    pub compatibility_target_key: Option<&'a str>,
    /// Operational policy key.
    pub policy_key: &'a str,
    /// Whether filesystem watching is enabled.
    pub watcher_enabled: bool,
    /// Whether scheduled discovery is enabled.
    pub schedule_enabled: bool,
    /// Scheduled discovery interval in minutes.
    pub schedule_interval_minutes: Option<i32>,
}

/// Create a media profile with verified source and output root identities.
///
/// # Errors
///
/// Returns an error when identity validation or stored-procedure execution fails.
pub async fn create_verified_media_profile(
    pool: &PgPool,
    input: &CreateVerifiedMediaProfileInput<'_>,
) -> Result<Uuid> {
    create_verified_media_profile_with_executor(pool, input).await
}

/// Create a verified media profile using a caller-provided SQL executor.
///
/// # Errors
///
/// Returns an error when identity validation or stored-procedure execution fails.
pub async fn create_verified_media_profile_with_executor<'e, E>(
    executor: E,
    input: &CreateVerifiedMediaProfileInput<'_>,
) -> Result<Uuid>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, Uuid>(MEDIA_PROFILE_CREATE_V3)
        .bind(input.actor_public_id)
        .bind(input.profile_key)
        .bind(input.source_requested_path)
        .bind(input.source_canonical_path)
        .bind(input.source_filesystem_device)
        .bind(input.source_filesystem_inode)
        .bind(input.output_requested_path)
        .bind(input.output_canonical_path)
        .bind(input.output_filesystem_device)
        .bind(input.output_filesystem_inode)
        .bind(input.retention_days)
        .bind(input.compatibility_target_key)
        .bind(input.policy_key)
        .bind(input.watcher_enabled)
        .bind(input.schedule_enabled)
        .bind(input.schedule_interval_minutes)
        .fetch_one(executor)
        .await
        .map_err(try_op("verified media profile create"))
}

/// Input payload for media profile patching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMediaProfileInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Profile public id.
    pub media_profile_public_id: Uuid,
    /// Source root path override.
    pub source_root: Option<&'a str>,
    /// Output root path override.
    pub output_root: Option<&'a str>,
    /// Dry-run-only policy override.
    pub dry_run_only: Option<bool>,
    /// Retention days override.
    pub retention_days: Option<i32>,
    /// Compatibility target key override.
    pub compatibility_target_key: Option<&'a str>,
    /// Operational policy key override.
    pub policy_key: Option<&'a str>,
    /// Filesystem watcher override.
    pub watcher_enabled: Option<bool>,
    /// Scheduled discovery enablement override.
    pub schedule_enabled: Option<bool>,
    /// Scheduled discovery interval override.
    pub schedule_interval_minutes: Option<i32>,
}

/// Verified filesystem identity payload for patching a media profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateVerifiedMediaProfileInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Profile public id.
    pub media_profile_public_id: Uuid,
    /// Operator-supplied source root.
    pub source_requested_path: &'a str,
    /// Canonical source root.
    pub source_canonical_path: &'a str,
    /// Source filesystem device identity.
    pub source_filesystem_device: i64,
    /// Source filesystem inode identity.
    pub source_filesystem_inode: i64,
    /// Operator-supplied output root.
    pub output_requested_path: &'a str,
    /// Canonical output root.
    pub output_canonical_path: &'a str,
    /// Output filesystem device identity.
    pub output_filesystem_device: i64,
    /// Output filesystem inode identity.
    pub output_filesystem_inode: i64,
    /// Dry-run-only policy override.
    pub dry_run_only: Option<bool>,
    /// Retention days override.
    pub retention_days: Option<i32>,
    /// Compatibility target key override.
    pub compatibility_target_key: Option<&'a str>,
    /// Operational policy key override.
    pub policy_key: Option<&'a str>,
    /// Filesystem watcher override.
    pub watcher_enabled: Option<bool>,
    /// Scheduled discovery enablement override.
    pub schedule_enabled: Option<bool>,
    /// Scheduled discovery interval override.
    pub schedule_interval_minutes: Option<i32>,
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
    /// Optional compatibility target key.
    pub compatibility_target_key: Option<String>,
    /// Operational policy key.
    pub policy_key: String,
    /// Whether filesystem watching is enabled.
    pub watcher_enabled: bool,
    /// Whether scheduled discovery is enabled.
    pub schedule_enabled: bool,
    /// Scheduled discovery interval in minutes.
    pub schedule_interval_minutes: Option<i32>,
    /// Optional pinned desired-target key.
    pub desired_target_key: Option<String>,
    /// Optional pinned desired-target version.
    pub desired_target_version: Option<i32>,
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
    upsert_media_profile_with_executor(pool, input).await
}

/// Create or update a media profile using a caller-provided SQL executor.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn upsert_media_profile_with_executor<'e, E>(
    executor: E,
    input: &UpsertMediaProfileInput<'_>,
) -> Result<Uuid>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, Uuid>(MEDIA_PROFILE_UPSERT_V2)
        .bind(input.actor_public_id)
        .bind(input.profile_key)
        .bind(input.source_root)
        .bind(input.output_root)
        .bind(input.dry_run_only)
        .bind(input.retention_days)
        .bind(input.compatibility_target_key)
        .bind(input.policy_key)
        .bind(input.watcher_enabled)
        .bind(input.schedule_enabled)
        .bind(input.schedule_interval_minutes)
        .fetch_one(executor)
        .await
        .map_err(try_op("media profile upsert"))
}

/// Patch a media profile by public id.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn update_media_profile(
    pool: &PgPool,
    input: &UpdateMediaProfileInput<'_>,
) -> Result<Uuid> {
    update_media_profile_with_executor(pool, input).await
}

/// Patch a media profile using a caller-provided SQL executor.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn update_media_profile_with_executor<'e, E>(
    executor: E,
    input: &UpdateMediaProfileInput<'_>,
) -> Result<Uuid>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, Uuid>(MEDIA_PROFILE_UPDATE_V1)
        .bind(input.actor_public_id)
        .bind(input.media_profile_public_id)
        .bind(input.source_root)
        .bind(input.output_root)
        .bind(input.dry_run_only)
        .bind(input.retention_days)
        .bind(input.compatibility_target_key)
        .bind(input.policy_key)
        .bind(input.watcher_enabled)
        .bind(input.schedule_enabled)
        .bind(input.schedule_interval_minutes)
        .fetch_one(executor)
        .await
        .map_err(try_op("media profile update"))
}

/// Patch a media profile while atomically replacing verified root identities.
///
/// # Errors
///
/// Returns an error when identity validation or stored-procedure execution fails.
pub async fn update_verified_media_profile(
    pool: &PgPool,
    input: &UpdateVerifiedMediaProfileInput<'_>,
) -> Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(MEDIA_PROFILE_UPDATE_VERIFIED_V1)
        .bind(input.actor_public_id)
        .bind(input.media_profile_public_id)
        .bind(input.source_requested_path)
        .bind(input.source_canonical_path)
        .bind(input.source_filesystem_device)
        .bind(input.source_filesystem_inode)
        .bind(input.output_requested_path)
        .bind(input.output_canonical_path)
        .bind(input.output_filesystem_device)
        .bind(input.output_filesystem_inode)
        .bind(input.dry_run_only)
        .bind(input.retention_days)
        .bind(input.compatibility_target_key)
        .bind(input.policy_key)
        .bind(input.watcher_enabled)
        .bind(input.schedule_enabled)
        .bind(input.schedule_interval_minutes)
        .fetch_one(pool)
        .await
        .map_err(try_op("verified media profile update"))
}

/// List active media profiles.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_profiles(pool: &PgPool) -> Result<Vec<MediaProfileRow>> {
    sqlx::query_as::<_, MediaProfileRow>(MEDIA_PROFILE_LIST_V3)
        .fetch_all(pool)
        .await
        .map_err(try_op("media profile list"))
}

/// Get one media profile by public id.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn get_media_profile(
    pool: &PgPool,
    media_profile_public_id: Uuid,
) -> Result<Option<MediaProfileRow>> {
    sqlx::query_as::<_, MediaProfileRow>(MEDIA_PROFILE_GET_V3)
        .bind(media_profile_public_id)
        .fetch_optional(pool)
        .await
        .map_err(try_op("media profile get"))
}

#[cfg(test)]
mod tests {
    use super::{
        CreateVerifiedMediaProfileInput, UpdateMediaProfileInput, UpsertMediaProfileInput,
        create_verified_media_profile, get_media_profile, list_media_profiles,
        update_media_profile, upsert_media_profile, upsert_media_profile_with_executor,
    };
    use crate::DataError;
    use crate::media::schema_tests::{MediaTestDb, setup_media_db};
    use crate::media::{
        MediaRootIdentity, MediaRootIdentityResolver, StdMediaRootIdentityResolver,
    };
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use std::path::Path;
    use uuid::Uuid;

    struct TestDirectory(std::path::PathBuf);

    impl TestDirectory {
        fn new() -> anyhow::Result<Self> {
            let path =
                std::env::temp_dir().join(format!("revaer-media-profile-roots-{}", Uuid::new_v4()));
            std::fs::create_dir_all(&path)?;
            Ok(Self(path))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_dir_all(&self.0) {
                eprintln!("failed to remove media profile root fixture: {error}");
            }
        }
    }

    fn closed_pool_options() -> PgConnectOptions {
        PgConnectOptions::new()
            .host("127.0.0.1")
            .port(9)
            .username("revaer")
            .password(
                &['r', 'e', 'v', 'a', 'e', 'r']
                    .into_iter()
                    .collect::<String>(),
            )
            .database("revaer")
    }

    async fn closed_pool() -> sqlx::PgPool {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy_with(closed_pool_options());
        pool.close().await;
        pool
    }

    fn profile_input<'a>(
        actor_public_id: Uuid,
        profile_key: &'a str,
        source_root: &'a str,
        output_root: &'a str,
    ) -> UpsertMediaProfileInput<'a> {
        UpsertMediaProfileInput {
            actor_public_id,
            profile_key,
            source_root,
            output_root,
            dry_run_only: true,
            retention_days: 30,
            compatibility_target_key: None,
            policy_key: "safe_dry_run",
            watcher_enabled: false,
            schedule_enabled: false,
            schedule_interval_minutes: None,
        }
    }

    async fn upsert_profile(
        db: &MediaTestDb,
        profile_key: &str,
        source_root: &str,
        output_root: &str,
    ) -> anyhow::Result<Uuid> {
        Ok(upsert_media_profile(
            db.pool(),
            &profile_input(
                db.system_user_public_id,
                profile_key,
                source_root,
                output_root,
            ),
        )
        .await?)
    }

    fn path_text(path: &Path) -> anyhow::Result<&str> {
        path.to_str()
            .ok_or_else(|| anyhow::anyhow!("test path is not valid UTF-8: {}", path.display()))
    }

    fn identity_number(value: u64) -> anyhow::Result<i64> {
        Ok(i64::try_from(value)?)
    }

    fn verified_profile_input<'a>(
        db: &MediaTestDb,
        profile_key: &'a str,
        source: &'a MediaRootIdentity,
        output: &'a MediaRootIdentity,
    ) -> anyhow::Result<CreateVerifiedMediaProfileInput<'a>> {
        Ok(CreateVerifiedMediaProfileInput {
            actor_public_id: db.system_user_public_id,
            profile_key,
            source_requested_path: path_text(source.requested_path())?,
            source_canonical_path: path_text(source.canonical_path())?,
            source_filesystem_device: identity_number(source.filesystem_device())?,
            source_filesystem_inode: identity_number(source.filesystem_inode())?,
            output_requested_path: path_text(output.requested_path())?,
            output_canonical_path: path_text(output.canonical_path())?,
            output_filesystem_device: identity_number(output.filesystem_device())?,
            output_filesystem_inode: identity_number(output.filesystem_inode())?,
            retention_days: 30,
            compatibility_target_key: None,
            policy_key: "safe_dry_run",
            watcher_enabled: false,
            schedule_enabled: false,
            schedule_interval_minutes: None,
        })
    }

    async fn create_verified_profile(
        db: &MediaTestDb,
        profile_key: &str,
        source: &MediaRootIdentity,
        output: &MediaRootIdentity,
    ) -> anyhow::Result<Uuid> {
        Ok(create_verified_media_profile(
            db.pool(),
            &verified_profile_input(db, profile_key, source, output)?,
        )
        .await?)
    }

    async fn add_verified_source_root(
        db: &MediaTestDb,
        profile_public_id: Uuid,
        root: &MediaRootIdentity,
        media_type: &str,
    ) -> anyhow::Result<crate::DataResult<Uuid>> {
        let result =
            sqlx::query_scalar("SELECT media_profile_root_add_v1($1,$2,$3,$4,$5,$6,$7,$8,$9)")
                .bind(profile_public_id)
                .bind("source")
                .bind(path_text(root.requested_path())?)
                .bind(path_text(root.canonical_path())?)
                .bind(identity_number(root.filesystem_device())?)
                .bind(identity_number(root.filesystem_inode())?)
                .bind(media_type)
                .bind(1_i32)
                .bind(true)
                .fetch_one(db.pool())
                .await
                .map_err(|source| DataError::QueryFailed {
                    operation: "test media profile root add",
                    source,
                });
        Ok(result)
    }

    fn assert_discovery_root_overlap(result: crate::DataResult<Uuid>) -> anyhow::Result<()> {
        let Err(err) = result else {
            return Err(anyhow::anyhow!(
                "expected discovery root overlap validation error"
            ));
        };
        assert_eq!(
            err.database_detail(),
            Some("media_profile_discovery_root_overlap")
        );
        Ok(())
    }

    fn assert_root_identity_overlap(result: crate::DataResult<Uuid>) -> anyhow::Result<()> {
        let Err(err) = result else {
            return Err(anyhow::anyhow!(
                "expected filesystem root identity overlap validation error"
            ));
        };
        assert_eq!(
            err.database_detail(),
            Some("media_profile_root_identity_overlap")
        );
        Ok(())
    }

    #[tokio::test]
    async fn upsert_and_list_media_profile() -> anyhow::Result<()> {
        let db = match setup_media_db("upsert_and_list_media_profile").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
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
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;

        let rows = list_media_profiles(db.pool()).await?;
        assert!(
            rows.iter()
                .any(|item| item.media_profile_public_id == profile_id)
        );
        let profile = get_media_profile(db.pool(), profile_id).await?;
        assert!(profile.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn profile_upsert_enforces_media_key_boundary() -> anyhow::Result<()> {
        let db = match setup_media_db("profile_upsert_enforces_media_key_boundary").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        };
        let exact = format!("a{}z", "b".repeat(126));
        let exact_id = upsert_profile(&db, &exact, "/input/exact-key", "/output/exact-key").await?;
        assert_ne!(exact_id, Uuid::nil());

        let oversized = format!("a{}z", "b".repeat(127));
        let result = upsert_profile(
            &db,
            &oversized,
            "/input/oversized-key",
            "/output/oversized-key",
        )
        .await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn upsert_rejects_automation_without_verified_roots() -> anyhow::Result<()> {
        let db = match setup_media_db("upsert_rejects_automation_without_verified_roots").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
            }
        };
        let error = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-create-dry-run",
                source_root: "/input/tv-create",
                output_root: "/output/tv-create",
                dry_run_only: false,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: true,
                schedule_enabled: true,
                schedule_interval_minutes: Some(60),
            },
        )
        .await
        .expect_err("unverified automation must fail closed");

        assert_eq!(
            error.database_detail(),
            Some("media_profile_filesystem_identity_required")
        );
        Ok(())
    }

    #[tokio::test]
    async fn reject_overlapping_roots() -> anyhow::Result<()> {
        let db = match setup_media_db("reject_overlapping_roots").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
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
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await;

        let err = result.expect_err("expected overlap validation error");
        assert!(matches!(err, DataError::QueryFailed { .. }));
        assert_eq!(err.database_detail(), Some("media_profile_roots_overlap"));

        let nested_with_trailing_slash = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-overlap-nested",
                source_root: "/input/tv/",
                output_root: "/input/tv/out",
                dry_run_only: false,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await;
        let err = nested_with_trailing_slash
            .expect_err("expected trailing-slash ancestor roots to be rejected");
        assert_eq!(err.database_detail(), Some("media_profile_roots_overlap"));

        let sibling_prefix = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-sibling-prefix",
                source_root: "/input/tv",
                output_root: "/input/tv2",
                dry_run_only: false,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        assert_ne!(sibling_prefix, uuid::Uuid::nil());

        Ok(())
    }

    #[tokio::test]
    async fn reject_discovery_roots_claimed_by_another_profile() -> anyhow::Result<()> {
        let db = match setup_media_db("reject_discovery_roots_claimed_by_another_profile").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
            }
        };

        let temp = TestDirectory::new()?;
        let owner_source_path = temp.0.join("library/movies");
        let owner_output_path = temp.0.join("work/movies");
        let duplicate_output_path = temp.0.join("work/movies-duplicate");
        let nested_source_path = owner_source_path.join("hd");
        let nested_output_path = temp.0.join("work/movies-hd");
        let sibling_source_path = temp.0.join("library/movies-archive");
        let sibling_output_path = temp.0.join("work/movies-archive");
        let bonus_source_path = owner_source_path.join("bonus");
        for path in [
            &owner_source_path,
            &owner_output_path,
            &duplicate_output_path,
            &nested_source_path,
            &nested_output_path,
            &sibling_source_path,
            &sibling_output_path,
            &bonus_source_path,
        ] {
            std::fs::create_dir_all(path)?;
        }
        let resolver = StdMediaRootIdentityResolver;
        let owner_source = resolver.resolve(&owner_source_path)?;
        let owner_output = resolver.resolve(&owner_output_path)?;
        let existing_profile_id = create_verified_profile(
            &db,
            "movies-existing-discovery-root",
            &owner_source,
            &owner_output,
        )
        .await?;

        let duplicate_output = resolver.resolve(&duplicate_output_path)?;
        let duplicate_source_root = create_verified_media_profile(
            db.pool(),
            &verified_profile_input(
                &db,
                "movies-duplicate-discovery-root",
                &owner_source,
                &duplicate_output,
            )?,
        )
        .await;
        assert_root_identity_overlap(duplicate_source_root)?;

        let nested_source = resolver.resolve(&nested_source_path)?;
        let nested_output = resolver.resolve(&nested_output_path)?;
        let nested_source_root = create_verified_media_profile(
            db.pool(),
            &verified_profile_input(
                &db,
                "movies-nested-discovery-root",
                &nested_source,
                &nested_output,
            )?,
        )
        .await;
        assert_root_identity_overlap(nested_source_root)?;

        let sibling_source = resolver.resolve(&sibling_source_path)?;
        let sibling_output = resolver.resolve(&sibling_output_path)?;
        let sibling_source_root = create_verified_profile(
            &db,
            "movies-sibling-discovery-root",
            &sibling_source,
            &sibling_output,
        )
        .await?;
        assert_ne!(sibling_source_root, uuid::Uuid::nil());

        let bonus_source = resolver.resolve(&bonus_source_path)?;
        let update_to_conflicting_source_root =
            add_verified_source_root(&db, sibling_source_root, &bonus_source, "bonus").await?;
        assert_root_identity_overlap(update_to_conflicting_source_root)?;

        let self_update = update_media_profile(
            db.pool(),
            &UpdateMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: existing_profile_id,
                source_root: None,
                output_root: None,
                dry_run_only: None,
                retention_days: Some(31),
                compatibility_target_key: None,
                policy_key: None,
                watcher_enabled: None,
                schedule_enabled: None,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        assert_eq!(self_update, existing_profile_id);

        Ok(())
    }

    #[tokio::test]
    async fn reject_any_root_claimed_by_another_profile() -> anyhow::Result<()> {
        let db = match setup_media_db("reject_any_root_claimed_by_another_profile").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        };

        upsert_profile(
            &db,
            "movies-all-root-owner",
            "/incoming/movies",
            "/library/movies",
        )
        .await?;

        let source_overlaps_existing_output = upsert_media_profile(
            db.pool(),
            &profile_input(
                db.system_user_public_id,
                "tv-source-overlaps-output",
                "/library/movies/tv",
                "/library/tv",
            ),
        )
        .await;
        assert_discovery_root_overlap(source_overlaps_existing_output)?;

        let output_overlaps_existing_source = upsert_media_profile(
            db.pool(),
            &profile_input(
                db.system_user_public_id,
                "tv-output-overlaps-source",
                "/incoming/tv",
                "/incoming/movies/encoded",
            ),
        )
        .await;
        assert_discovery_root_overlap(output_overlaps_existing_source)?;

        let sibling_roots = upsert_profile(
            &db,
            "movies-all-root-sibling",
            "/incoming/movies-archive",
            "/library/movies-archive",
        )
        .await?;
        assert_ne!(sibling_roots, uuid::Uuid::nil());

        Ok(())
    }

    #[tokio::test]
    async fn concurrent_upserts_cannot_claim_overlapping_roots() -> anyhow::Result<()> {
        let db = match setup_media_db("concurrent_upserts_cannot_claim_overlapping_roots").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => return Err(err),
        };

        let first = profile_input(
            db.system_user_public_id,
            "concurrent-root-first",
            "/concurrent/media",
            "/concurrent/output-first",
        );
        let second = profile_input(
            db.system_user_public_id,
            "concurrent-root-second",
            "/concurrent/media/tv",
            "/concurrent/output-second",
        );
        let (first_result, second_result) = tokio::join!(
            upsert_media_profile(db.pool(), &first),
            upsert_media_profile(db.pool(), &second)
        );

        assert_eq!(
            usize::from(first_result.is_ok()) + usize::from(second_result.is_ok()),
            1
        );
        let rejected = if first_result.is_err() {
            first_result
        } else {
            second_result
        };
        assert_discovery_root_overlap(rejected)?;
        Ok(())
    }

    #[test]
    fn migration_guards_cross_profile_discovery_root_overlap() -> anyhow::Result<()> {
        let init_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("init")
            .join("0001_init.sql");
        let migration_body = std::fs::read_to_string(init_path)?;

        assert!(
            migration_body.contains("media_profile_discovery_root_overlap"),
            "media profile migrations must reject discovery roots claimed by another active profile"
        );
        assert!(
            migration_body.contains(
                "pg_advisory_xact_lock(hashtextextended('media_profile_all_root_overlap_v1', 0))"
            ),
            "media profile overlap validation must serialize all profile writes"
        );
        assert!(
            migration_body.contains(
                "pg_advisory_xact_lock(hashtextextended('media_profile_root_identity_v1', 0))"
            ),
            "media profile identity validation must serialize all root write paths"
        );
        assert!(
            migration_body.contains("media_profile_all_root_overlap_trigger"),
            "media profile roots must be validated for every insert and update"
        );
        Ok(())
    }

    #[tokio::test]
    async fn upsert_rejects_unknown_catalog_refs() -> anyhow::Result<()> {
        let db = match setup_media_db("upsert_rejects_unknown_catalog_refs").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
            }
        };

        let missing_target = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-missing-target",
                source_root: "/input/tv-missing-target",
                output_root: "/output/tv-missing-target",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: Some("missing-target"),
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await;
        let err = missing_target.expect_err("expected missing target to be rejected");
        assert_eq!(
            err.database_detail(),
            Some("media_compatibility_target_not_found")
        );

        let missing_policy = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-missing-policy",
                source_root: "/input/tv-missing-policy",
                output_root: "/output/tv-missing-policy",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "missing-policy",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await;
        let err = missing_policy.expect_err("expected missing policy to be rejected");
        assert_eq!(
            err.database_detail(),
            Some("media_policy_profile_not_found")
        );

        Ok(())
    }

    #[tokio::test]
    async fn update_rejects_unknown_catalog_refs() -> anyhow::Result<()> {
        let db = match setup_media_db("update_rejects_unknown_catalog_refs").await {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
            }
        };
        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-update-catalog",
                source_root: "/input/tv-update-catalog",
                output_root: "/output/tv-update-catalog",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;

        let missing_target = update_media_profile(
            db.pool(),
            &UpdateMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_root: None,
                output_root: None,
                dry_run_only: None,
                retention_days: None,
                compatibility_target_key: Some("missing-target"),
                policy_key: None,
                watcher_enabled: None,
                schedule_enabled: None,
                schedule_interval_minutes: None,
            },
        )
        .await;
        let err = missing_target.expect_err("expected missing target to be rejected");
        assert_eq!(
            err.database_detail(),
            Some("media_compatibility_target_not_found")
        );

        let missing_policy = update_media_profile(
            db.pool(),
            &UpdateMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_root: None,
                output_root: None,
                dry_run_only: None,
                retention_days: None,
                compatibility_target_key: None,
                policy_key: Some("missing-policy"),
                watcher_enabled: None,
                schedule_enabled: None,
                schedule_interval_minutes: None,
            },
        )
        .await;
        let err = missing_policy.expect_err("expected missing policy to be rejected");
        assert_eq!(
            err.database_detail(),
            Some("media_policy_profile_not_found")
        );

        Ok(())
    }

    #[tokio::test]
    async fn upsert_profile_with_executor_accepts_transaction_executor() -> anyhow::Result<()> {
        let db = match setup_media_db("upsert_profile_with_executor_accepts_transaction_executor")
            .await
        {
            Ok(Some(db)) => db,
            Ok(None) => return Ok(()),
            Err(err) => {
                return Err(err);
            }
        };

        let mut transaction = db.pool().begin().await?;
        let profile_id = upsert_media_profile_with_executor(
            &mut *transaction,
            &UpsertMediaProfileInput {
                actor_public_id: db.system_user_public_id,
                profile_key: "tv-tx-executor",
                source_root: "/input/tv",
                output_root: "/output/tv",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        transaction.commit().await?;

        let rows = list_media_profiles(db.pool()).await?;
        assert!(
            rows.iter()
                .any(|item| item.media_profile_public_id == profile_id)
        );
        Ok(())
    }

    #[tokio::test]
    async fn media_profile_queries_surface_query_errors_without_database() {
        let pool = closed_pool().await;
        let profile_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();
        let input = UpsertMediaProfileInput {
            actor_public_id: actor_id,
            profile_key: "movies-main",
            source_root: "/input/movies",
            output_root: "/output/movies",
            dry_run_only: true,
            retention_days: 14,
            compatibility_target_key: None,
            policy_key: "safe_dry_run",
            watcher_enabled: false,
            schedule_enabled: false,
            schedule_interval_minutes: None,
        };

        let upsert = upsert_media_profile(&pool, &input).await;
        assert!(upsert.is_err());

        let list = list_media_profiles(&pool).await;
        assert!(list.is_err());

        let get = get_media_profile(&pool, profile_id).await;
        assert!(get.is_err());

        let upsert_with_executor = upsert_media_profile_with_executor(&pool, &input).await;
        assert!(upsert_with_executor.is_err());
    }
}
