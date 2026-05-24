//! Runtime media persistence facade wrapping media stored-procedure access.
//!
//! # Design
//! - Keep API and workers insulated from the `revaer-data` module layout.
//! - Expose strongly typed methods for media profiles, jobs, and capabilities.

use revaer_data::DataResult;
use revaer_data::media::capabilities::{RecordCapabilitySnapshotInput, record_capability_snapshot};
use revaer_data::media::jobs::{
    CreateMediaJobInput, MediaJobRow, append_media_job_phase, create_media_job, list_media_jobs,
};
use revaer_data::media::profiles::{
    MediaProfileRow, UpsertMediaProfileInput, list_media_profiles, upsert_media_profile,
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

    /// List active media profiles.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_profiles(&self) -> DataResult<Vec<MediaProfileRow>> {
        list_media_profiles(&self.pool).await
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
}

#[cfg(test)]
mod tests {
    use super::MediaStore;
    use revaer_data::media::capabilities::RecordCapabilitySnapshotInput;
    use revaer_data::media::jobs::CreateMediaJobInput;
    use revaer_data::media::profiles::UpsertMediaProfileInput;
    use revaer_test_support::postgres::TestDatabase;
    use revaer_test_support::postgres::start_postgres;
    use sqlx::postgres::PgPoolOptions;
    use uuid::Uuid;

    async fn test_store() -> anyhow::Result<(TestDatabase, MediaStore)> {
        let postgres = match start_postgres() {
            Ok(db) => db,
            Err(err) => {
                eprintln!("skipping media store test: {err}");
                return Err(anyhow::anyhow!("media store test skipped"));
            }
        };

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(postgres.connection_string())
            .await?;

        let mut migrator = sqlx::migrate!("../revaer-data/migrations");
        migrator.set_ignore_missing(true);
        migrator.run(&pool).await?;

        Ok((postgres, MediaStore::new(pool)))
    }

    async fn system_actor(pool: &sqlx::PgPool) -> anyhow::Result<Uuid> {
        if let Some(existing) =
            sqlx::query_scalar::<_, Uuid>("SELECT user_public_id FROM app_user LIMIT 1")
                .fetch_optional(pool)
                .await
                .map_err(anyhow::Error::from)?
        {
            return Ok(existing);
        }

        let user_public_id = Uuid::new_v4();
        sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO app_user (
                user_public_id,
                email,
                email_normalized,
                display_name,
                role,
                is_email_verified
            ) VALUES ($1, $2, $3, $4, 'owner', TRUE)
            RETURNING user_public_id",
        )
        .bind(user_public_id)
        .bind("media-runtime@example.invalid")
        .bind("media-runtime@example.invalid")
        .bind("Media Runtime")
        .fetch_one(pool)
        .await
        .map_err(anyhow::Error::from)
    }

    #[tokio::test]
    async fn media_store_round_trips_profiles_jobs_and_capabilities() -> anyhow::Result<()> {
        let Ok((postgres, store)) = test_store().await else {
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

        let jobs = store.list_jobs(profile_id, Some("queued")).await?;
        assert!(jobs.iter().any(|job| job.media_job_public_id == job_id));

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

        Ok(())
    }
}
