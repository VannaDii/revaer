//! Stored-procedure access for media configuration catalogs.

use crate::error::{Result, try_op};
use sqlx::PgPool;
use uuid::Uuid;

const MEDIA_COMPATIBILITY_TARGET_LIST_V1: &str = "SELECT compatibility_target_key, version, display_name, video_codec, audio_codec, subtitle_policy FROM media_compatibility_target_list_v1()";
const MEDIA_COMPATIBILITY_TARGET_UPSERT_V1: &str = "SELECT compatibility_target_key, version, display_name, video_codec, audio_codec, subtitle_policy FROM media_compatibility_target_upsert_v1($1, $2, $3, $4, $5, $6, $7)";
const MEDIA_POLICY_PROFILE_LIST_V1: &str =
    "SELECT policy_key, version, display_name, video_intent FROM media_policy_profile_list_v1()";
const MEDIA_POLICY_PROFILE_UPSERT_V1: &str = "SELECT policy_key, version, display_name, video_intent FROM media_policy_profile_upsert_v1($1, $2, $3, $4, $5)";
const MEDIA_JOB_RETENTION_POLICY_GET_V1: &str = "SELECT completed_retention_days, failed_diagnostic_retention_days FROM media_job_retention_policy_get_v1()";
const MEDIA_JOB_RETENTION_POLICY_UPDATE_V1: &str = "SELECT completed_retention_days, failed_diagnostic_retention_days FROM media_job_retention_policy_update_v1($1, $2, $3)";

/// Compatibility target upsert payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpsertMediaCompatibilityTargetInput<'a> {
    /// Actor performing the write.
    pub actor_public_id: Uuid,
    /// Stable target key.
    pub compatibility_target_key: &'a str,
    /// Version to create or replace.
    pub version: i32,
    /// Operator display label.
    pub display_name: &'a str,
    /// Desired video codec.
    pub video_codec: &'a str,
    /// Desired audio codec.
    pub audio_codec: &'a str,
    /// Subtitle retention policy.
    pub subtitle_policy: &'a str,
}

/// Policy profile upsert payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpsertMediaPolicyProfileInput<'a> {
    /// Actor performing the write.
    pub actor_public_id: Uuid,
    /// Stable policy key.
    pub policy_key: &'a str,
    /// Version to create or replace.
    pub version: i32,
    /// Operator display label.
    pub display_name: &'a str,
    /// Worker video intent.
    pub video_intent: &'a str,
}

/// Job retention update payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateMediaJobRetentionPolicyInput {
    /// Actor performing the write.
    pub actor_public_id: Uuid,
    /// Completed-job retention window.
    pub completed_retention_days: i32,
    /// Failed-terminal diagnostic retention window.
    pub failed_diagnostic_retention_days: i32,
}

/// Versioned compatibility target row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaCompatibilityTargetRow {
    /// Stable target key.
    pub compatibility_target_key: String,
    /// Target version.
    pub version: i32,
    /// Display label.
    pub display_name: String,
    /// Desired video codec.
    pub video_codec: String,
    /// Desired audio codec.
    pub audio_codec: String,
    /// Subtitle retention policy.
    pub subtitle_policy: String,
}

/// Versioned policy profile row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaPolicyProfileRow {
    /// Stable policy key.
    pub policy_key: String,
    /// Policy version.
    pub version: i32,
    /// Display label.
    pub display_name: String,
    /// Video transcode intent.
    pub video_intent: String,
}

/// Job retention policy row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobRetentionPolicyRow {
    /// Completed-job retention window.
    pub completed_retention_days: i32,
    /// Failed-terminal diagnostic retention window.
    pub failed_diagnostic_retention_days: i32,
}

/// List enabled compatibility target versions.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_compatibility_targets(
    pool: &PgPool,
) -> Result<Vec<MediaCompatibilityTargetRow>> {
    sqlx::query_as::<_, MediaCompatibilityTargetRow>(MEDIA_COMPATIBILITY_TARGET_LIST_V1)
        .fetch_all(pool)
        .await
        .map_err(try_op("media compatibility target list"))
}

/// Create or replace a compatibility target version.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn upsert_media_compatibility_target(
    pool: &PgPool,
    input: UpsertMediaCompatibilityTargetInput<'_>,
) -> Result<MediaCompatibilityTargetRow> {
    sqlx::query_as::<_, MediaCompatibilityTargetRow>(MEDIA_COMPATIBILITY_TARGET_UPSERT_V1)
        .bind(input.actor_public_id)
        .bind(input.compatibility_target_key)
        .bind(input.version)
        .bind(input.display_name)
        .bind(input.video_codec)
        .bind(input.audio_codec)
        .bind(input.subtitle_policy)
        .fetch_one(pool)
        .await
        .map_err(try_op("media compatibility target upsert"))
}

/// List enabled policy profile versions.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_policy_profiles(pool: &PgPool) -> Result<Vec<MediaPolicyProfileRow>> {
    sqlx::query_as::<_, MediaPolicyProfileRow>(MEDIA_POLICY_PROFILE_LIST_V1)
        .fetch_all(pool)
        .await
        .map_err(try_op("media policy profile list"))
}

/// Create or replace a policy profile version.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn upsert_media_policy_profile(
    pool: &PgPool,
    input: UpsertMediaPolicyProfileInput<'_>,
) -> Result<MediaPolicyProfileRow> {
    sqlx::query_as::<_, MediaPolicyProfileRow>(MEDIA_POLICY_PROFILE_UPSERT_V1)
        .bind(input.actor_public_id)
        .bind(input.policy_key)
        .bind(input.version)
        .bind(input.display_name)
        .bind(input.video_intent)
        .fetch_one(pool)
        .await
        .map_err(try_op("media policy profile upsert"))
}

/// Read the active job retention policy.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn get_media_job_retention_policy(
    pool: &PgPool,
) -> Result<Option<MediaJobRetentionPolicyRow>> {
    sqlx::query_as::<_, MediaJobRetentionPolicyRow>(MEDIA_JOB_RETENTION_POLICY_GET_V1)
        .fetch_optional(pool)
        .await
        .map_err(try_op("media job retention policy get"))
}

/// Update the active job retention policy.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn update_media_job_retention_policy(
    pool: &PgPool,
    input: UpdateMediaJobRetentionPolicyInput,
) -> Result<MediaJobRetentionPolicyRow> {
    sqlx::query_as::<_, MediaJobRetentionPolicyRow>(MEDIA_JOB_RETENTION_POLICY_UPDATE_V1)
        .bind(input.actor_public_id)
        .bind(input.completed_retention_days)
        .bind(input.failed_diagnostic_retention_days)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job retention policy update"))
}

#[cfg(test)]
mod tests {
    use super::{
        UpdateMediaJobRetentionPolicyInput, UpsertMediaCompatibilityTargetInput,
        UpsertMediaPolicyProfileInput, get_media_job_retention_policy,
        list_media_compatibility_targets, list_media_policy_profiles,
        update_media_job_retention_policy, upsert_media_compatibility_target,
        upsert_media_policy_profile,
    };
    use crate::config::factory_reset;
    use crate::media::schema_tests::setup_media_db;

    #[tokio::test]
    async fn media_configuration_catalogs_are_stored_procedure_backed() -> anyhow::Result<()> {
        let Some(db) = setup_media_db("media_configuration_catalogs").await? else {
            return Ok(());
        };

        let targets = list_media_compatibility_targets(db.pool()).await?;
        assert!(targets.iter().any(|target| {
            target.compatibility_target_key == "plex-apple-tv"
                && target.version == 1
                && target.video_codec == "hevc"
                && target.audio_codec == "aac"
                && target.subtitle_policy == "selected"
        }));

        let policies = list_media_policy_profiles(db.pool()).await?;
        assert!(policies.iter().any(|policy| {
            policy.policy_key == "anime" && policy.version == 1 && policy.video_intent == "anime"
        }));

        let retention = get_media_job_retention_policy(db.pool()).await?;
        let Some(retention) = retention else {
            return Err(anyhow::anyhow!("retention policy should be seeded"));
        };
        assert_eq!(retention.completed_retention_days, 30);
        assert_eq!(retention.failed_diagnostic_retention_days, 30);

        let actor = db.system_user_public_id;
        let target = upsert_media_compatibility_target(
            db.pool(),
            UpsertMediaCompatibilityTargetInput {
                actor_public_id: actor,
                compatibility_target_key: "plex-living-room",
                version: 2,
                display_name: "Plex living room",
                video_codec: "av1",
                audio_codec: "opus",
                subtitle_policy: "all",
            },
        )
        .await?;
        assert_eq!(target.compatibility_target_key, "plex-living-room");
        assert_eq!(target.version, 2);
        assert_eq!(target.video_codec, "av1");

        let policy = upsert_media_policy_profile(
            db.pool(),
            UpsertMediaPolicyProfileInput {
                actor_public_id: actor,
                policy_key: "archive-quality",
                version: 3,
                display_name: "Archive quality",
                video_intent: "archival",
            },
        )
        .await?;
        assert_eq!(policy.policy_key, "archive-quality");
        assert_eq!(policy.version, 3);
        assert_eq!(policy.video_intent, "archival");

        let retention = update_media_job_retention_policy(
            db.pool(),
            UpdateMediaJobRetentionPolicyInput {
                actor_public_id: actor,
                completed_retention_days: 90,
                failed_diagnostic_retention_days: 180,
            },
        )
        .await?;
        assert_eq!(retention.completed_retention_days, 90);
        assert_eq!(retention.failed_diagnostic_retention_days, 180);

        factory_reset(db.pool()).await?;
        let reset_targets = list_media_compatibility_targets(db.pool()).await?;
        assert!(reset_targets.iter().any(|target| {
            target.compatibility_target_key == "hevc-aac"
                && target.version == 1
                && target.video_codec == "hevc"
                && target.audio_codec == "aac"
        }));
        let reset_policies = list_media_policy_profiles(db.pool()).await?;
        assert!(
            reset_policies
                .iter()
                .any(|policy| policy.policy_key == "safe_dry_run" && policy.version == 1)
        );
        let Some(reset_retention) = get_media_job_retention_policy(db.pool()).await? else {
            return Err(anyhow::anyhow!(
                "retention policy should be restored by factory reset"
            ));
        };
        assert_eq!(reset_retention.completed_retention_days, 30);
        assert_eq!(reset_retention.failed_diagnostic_retention_days, 30);
        Ok(())
    }
}
