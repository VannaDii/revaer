//! Runtime media persistence facade wrapping media stored-procedure access.
//!
//! # Design
//! - Keep API and workers insulated from the `revaer-data` module layout.
//! - Expose strongly typed methods for media profiles, jobs, and capabilities.

use revaer_data::DataResult;
use revaer_data::media::capabilities::{
    CapabilitySnapshotRow, RecordCapabilitySnapshotInput, latest_capability_snapshot,
    record_capability_snapshot,
};
use revaer_data::media::jobs::{
    CreateMediaJobInput, MediaJobOperationRow, MediaJobRow, MediaJobViolationRow,
    append_media_job_operation, append_media_job_phase, append_media_job_violation,
    cancel_media_job, create_media_job, get_media_job, list_media_job_operations,
    list_media_job_violations, list_media_jobs, retry_media_job,
};
use revaer_data::media::profiles::{
    MediaProfileRow, UpdateMediaProfileInput, UpsertMediaProfileInput, get_media_profile,
    list_media_profiles, update_media_profile, upsert_media_profile,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Runtime media store facade.
#[derive(Clone)]
pub struct MediaStore {
    pool: PgPool,
}

impl MediaStore {
    /// Construct a media store facade from a connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Access underlying connection pool.
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Upsert a media profile.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn upsert_profile(&self, input: &UpsertMediaProfileInput<'_>) -> DataResult<Uuid> {
        upsert_media_profile(&self.pool, input).await
    }

    /// Patch a media profile.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn update_profile(&self, input: &UpdateMediaProfileInput<'_>) -> DataResult<Uuid> {
        update_media_profile(&self.pool, input).await
    }

    /// List active media profiles.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_profiles(&self) -> DataResult<Vec<MediaProfileRow>> {
        list_media_profiles(&self.pool).await
    }

    /// Get one media profile by public id.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn get_profile(
        &self,
        media_profile_public_id: Uuid,
    ) -> DataResult<Option<MediaProfileRow>> {
        get_media_profile(&self.pool, media_profile_public_id).await
    }

    /// Create a media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn create_job(&self, input: &CreateMediaJobInput<'_>) -> DataResult<Uuid> {
        create_media_job(&self.pool, input).await
    }

    /// Append a media job phase.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn append_job_phase(
        &self,
        media_job_public_id: Uuid,
        phase_index: i32,
        phase_name: &str,
        phase_status_text: &str,
        details_text: Option<&str>,
    ) -> DataResult<()> {
        append_media_job_phase(
            &self.pool,
            media_job_public_id,
            phase_index,
            phase_name,
            phase_status_text,
            details_text,
        )
        .await
    }

    /// Append a deterministic execution operation for a media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn append_job_operation(
        &self,
        media_job_public_id: Uuid,
        operation_index: i32,
        operation_kind: &str,
        stream_id: Option<i32>,
        command_bin: &str,
        args: [Option<&str>; 5],
    ) -> DataResult<()> {
        append_media_job_operation(
            &self.pool,
            media_job_public_id,
            operation_index,
            operation_kind,
            stream_id,
            command_bin,
            args,
        )
        .await
    }

    /// Append a compliance violation for a media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn append_job_violation(
        &self,
        media_job_public_id: Uuid,
        violation_index: i32,
        violation_kind: &str,
        severity: &str,
        stream_id: Option<i32>,
    ) -> DataResult<()> {
        append_media_job_violation(
            &self.pool,
            media_job_public_id,
            violation_index,
            violation_kind,
            severity,
            stream_id,
        )
        .await
    }

    /// List media jobs for a profile.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_jobs(
        &self,
        media_profile_public_id: Uuid,
        status_text: Option<&str>,
    ) -> DataResult<Vec<MediaJobRow>> {
        list_media_jobs(&self.pool, media_profile_public_id, status_text).await
    }

    /// List persisted execution operations for one media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_job_operations(
        &self,
        media_job_public_id: Uuid,
    ) -> DataResult<Vec<MediaJobOperationRow>> {
        list_media_job_operations(&self.pool, media_job_public_id).await
    }

    /// List persisted compliance violations for one media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_job_violations(
        &self,
        media_job_public_id: Uuid,
    ) -> DataResult<Vec<MediaJobViolationRow>> {
        list_media_job_violations(&self.pool, media_job_public_id).await
    }

    /// Load one media job by public id.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn get_job(&self, media_job_public_id: Uuid) -> DataResult<Option<MediaJobRow>> {
        get_media_job(&self.pool, media_job_public_id).await
    }

    /// Cancel one media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn cancel_job(&self, media_job_public_id: Uuid) -> DataResult<()> {
        cancel_media_job(&self.pool, media_job_public_id).await
    }

    /// Retry one media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn retry_job(&self, media_job_public_id: Uuid) -> DataResult<()> {
        retry_media_job(&self.pool, media_job_public_id).await
    }

    /// Record one capability snapshot row.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn record_capability(
        &self,
        input: &RecordCapabilitySnapshotInput<'_>,
    ) -> DataResult<i64> {
        record_capability_snapshot(&self.pool, input).await
    }

    /// Load latest capability snapshot, if present.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying query fails.
    pub async fn latest_capability(&self) -> DataResult<Option<CapabilitySnapshotRow>> {
        latest_capability_snapshot(&self.pool).await
    }
}

#[cfg(test)]
mod tests {
    use super::MediaStore;
    use revaer_data::indexers::app_users::{app_user_create, app_user_verify_email};
    use revaer_data::media::capabilities::RecordCapabilitySnapshotInput;
    use revaer_data::media::jobs::CreateMediaJobInput;
    use revaer_data::media::profiles::UpsertMediaProfileInput;
    use revaer_test_support::postgres::TestDatabase;
    use revaer_test_support::postgres::start_postgres;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use std::time::Duration;
    use tokio::time::sleep;
    use uuid::Uuid;

    fn is_transient_postgres_startup_error(message: &str) -> bool {
        message.contains("database system is in recovery mode")
            || message.contains("database system is starting up")
            || message.contains("not yet accepting connections")
    }

    fn has_transient_postgres_startup_error_text(message: &str) -> bool {
        is_transient_postgres_startup_error(message)
            || message.contains("failed to create database")
    }

    async fn test_store() -> anyhow::Result<Option<(TestDatabase, MediaStore)>> {
        let postgres = match start_postgres() {
            Ok(db) => db,
            Err(err) => {
                let message = err.to_string();
                if message.contains("docker daemon is not available")
                    || message.contains("docker command not found")
                    || message.contains("could not map host port")
                    || message.contains("test database url is required")
                    || has_transient_postgres_startup_error_text(&format!("{err:#}"))
                {
                    eprintln!("skipping media store test: {err}");
                    return Ok(None);
                }
                return Err(err);
            }
        };

        let mut pool = None;
        for _ in 0..30 {
            match PgPoolOptions::new()
                .max_connections(5)
                .connect(postgres.connection_string())
                .await
            {
                Ok(connected_pool) => {
                    pool = Some(connected_pool);
                    break;
                }
                Err(err) if has_transient_postgres_startup_error_text(&format!("{err:#}")) => {
                    sleep(Duration::from_secs(1)).await;
                }
                Err(err) => return Err(err.into()),
            }
        }
        let Some(pool) = pool else {
            eprintln!("skipping media store test: transient Postgres startup recovery timeout");
            return Ok(None);
        };

        let mut migrator = sqlx::migrate!("../revaer-data/migrations");
        migrator.set_ignore_missing(true);
        let mut migrated = false;
        for _ in 0..30 {
            match migrator.run(&pool).await {
                Ok(()) => {
                    migrated = true;
                    break;
                }
                Err(err) if has_transient_postgres_startup_error_text(&format!("{err:#}")) => {
                    sleep(Duration::from_secs(1)).await;
                }
                Err(err) => return Err(err.into()),
            }
        }
        if !migrated {
            eprintln!("skipping media store test: transient Postgres migration recovery timeout");
            return Ok(None);
        }

        Ok(Some((postgres, MediaStore::new(pool))))
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

    async fn system_actor(pool: &sqlx::PgPool) -> anyhow::Result<Uuid> {
        let email = format!("media-runtime-{}@example.invalid", Uuid::new_v4());
        let user_public_id = app_user_create(pool, &email, "Media Runtime").await?;
        app_user_verify_email(pool, user_public_id).await?;
        Ok(user_public_id)
    }

    #[tokio::test]
    async fn media_store_round_trips_profiles_jobs_and_capabilities() -> anyhow::Result<()> {
        let Some((postgres, store)) = test_store().await? else {
            return Ok(());
        };
        let _keep_db_alive = postgres;
        let actor = system_actor(store.pool()).await?;

        let profile_id = store
            .upsert_profile(&UpsertMediaProfileInput {
                actor_public_id: actor,
                profile_key: "tv-runtime",
                source_root: "/input/tv",
                output_root: "/output/tv",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            })
            .await?;

        let profiles = store.list_profiles().await?;
        assert!(
            profiles
                .iter()
                .any(|profile| profile.media_profile_public_id == profile_id)
        );

        let job_id = store
            .create_job(&CreateMediaJobInput {
                actor_public_id: actor,
                media_profile_public_id: profile_id,
                source_path: "/input/tv/show.mkv",
                output_path: Some("/output/tv/show.mkv"),
                dry_run: true,
            })
            .await?;

        store
            .append_job_phase(job_id, 0, "planning", "queued", Some("scheduled"))
            .await?;
        store
            .append_job_operation(
                job_id,
                0,
                "remux",
                None,
                "ffmpeg",
                [
                    Some("-i"),
                    Some("/input/tv/show.mkv"),
                    Some("-c"),
                    Some("copy"),
                    None,
                ],
            )
            .await?;
        store
            .append_job_violation(job_id, 0, "video_codec_mismatch", "high", Some(0))
            .await?;

        let jobs = store.list_jobs(profile_id, Some("queued")).await?;
        assert!(jobs.iter().any(|job| job.media_job_public_id == job_id));
        let operations = store.list_job_operations(job_id).await?;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].operation_kind, "remux");
        let violations = store.list_job_violations(job_id).await?;
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_kind, "video_codec_mismatch");

        let snapshot_id = store
            .record_capability(&RecordCapabilitySnapshotInput {
                actor_public_id: actor,
                ffmpeg_version: "7.0",
                ffprobe_version: "7.0",
                codec_name: "h264",
                encode_supported: true,
                decode_supported: true,
            })
            .await?;
        assert!(snapshot_id > 0);
        let latest = store.latest_capability().await?;
        assert!(latest.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn media_store_methods_surface_query_errors_without_database() {
        let store = MediaStore::new(closed_pool().await);
        let actor_id = Uuid::new_v4();
        let profile_id = Uuid::new_v4();
        let job_id = Uuid::new_v4();

        assert!(
            store
                .upsert_profile(&UpsertMediaProfileInput {
                    actor_public_id: actor_id,
                    profile_key: "movies-main",
                    source_root: "/input/movies",
                    output_root: "/output/movies",
                    dry_run_only: true,
                    retention_days: 30,
                    compatibility_target_key: None,
                    policy_key: "safe_dry_run",
                    watcher_enabled: false,
                    schedule_enabled: false,
                    schedule_interval_minutes: None,
                })
                .await
                .is_err()
        );
        assert!(store.list_profiles().await.is_err());
        assert!(store.get_profile(profile_id).await.is_err());
        assert!(
            store
                .create_job(&CreateMediaJobInput {
                    actor_public_id: actor_id,
                    media_profile_public_id: profile_id,
                    source_path: "/input/movies/file.mkv",
                    output_path: None,
                    dry_run: true,
                })
                .await
                .is_err()
        );
        assert!(
            store
                .append_job_phase(job_id, 0, "plan", "queued", None)
                .await
                .is_err()
        );
        assert!(
            store
                .append_job_operation(job_id, 0, "remux", None, "ffmpeg", [None; 5])
                .await
                .is_err()
        );
        assert!(store.list_jobs(profile_id, Some("queued")).await.is_err());
        assert!(store.get_job(job_id).await.is_err());
        assert!(store.list_job_operations(job_id).await.is_err());
        assert!(
            store
                .append_job_violation(job_id, 0, "codec_mismatch", "high", Some(0))
                .await
                .is_err()
        );
        assert!(store.list_job_violations(job_id).await.is_err());
        assert!(store.cancel_job(job_id).await.is_err());
        assert!(store.retry_job(job_id).await.is_err());

        assert!(
            store
                .record_capability(&RecordCapabilitySnapshotInput {
                    actor_public_id: actor_id,
                    ffmpeg_version: "7.1",
                    ffprobe_version: "7.1",
                    codec_name: "h264",
                    encode_supported: true,
                    decode_supported: true,
                })
                .await
                .is_err()
        );
        assert!(store.latest_capability().await.is_err());
    }
}
