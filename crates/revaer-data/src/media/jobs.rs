//! Stored-procedure access for media job lifecycle.

use crate::error::{Result, try_op};
use sqlx::PgPool;
use uuid::Uuid;

const MEDIA_JOB_CREATE_V1: &str = "SELECT media_job_create_v1(actor_public_id_input => $1, media_profile_public_id_input => $2, source_path_input => $3, output_path_input => $4, dry_run_input => $5)";
const MEDIA_JOB_PHASE_APPEND_V1: &str = "SELECT media_job_phase_append_v1(media_job_public_id_input => $1, phase_index_input => $2, phase_name_input => $3, phase_status_input => $4, details_text_input => $5)";
const MEDIA_JOB_OPERATION_APPEND_V1: &str = "SELECT media_job_operation_append_v1(media_job_public_id_input => $1, operation_index_input => $2, operation_kind_input => $3, stream_id_input => $4, command_bin_input => $5, arg_1_input => $6, arg_2_input => $7, arg_3_input => $8, arg_4_input => $9, arg_5_input => $10)";
const MEDIA_JOB_OPERATION_LIST_V1: &str = "SELECT operation_index, operation_kind, stream_id, command_bin, arg_1, arg_2, arg_3, arg_4, arg_5, created_at FROM media_job_operation_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_LIST_V1: &str = "SELECT media_job_public_id, source_path, output_path, status::text AS status_text, dry_run, queued_at, started_at, completed_at, last_error FROM media_job_list_v1(media_profile_public_id_input => $1, status_input => $2::media_job_status)";
const MEDIA_JOB_GET_V1: &str = "SELECT media_job_public_id, source_path, output_path, status::text AS status_text, dry_run, queued_at, started_at, completed_at, last_error FROM media_job_get_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_CANCEL_V1: &str = "SELECT media_job_cancel_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_RETRY_V1: &str = "SELECT media_job_retry_v1(media_job_public_id_input => $1)";

/// Create media job payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMediaJobInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Owning media profile public id.
    pub media_profile_public_id: Uuid,
    /// Source media path.
    pub source_path: &'a str,
    /// Optional output path.
    pub output_path: Option<&'a str>,
    /// Dry-run execution flag.
    pub dry_run: bool,
}

/// Media job listing row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobRow {
    /// Job public id.
    pub media_job_public_id: Uuid,
    /// Source path.
    pub source_path: String,
    /// Output path.
    pub output_path: Option<String>,
    /// Status text.
    pub status_text: String,
    /// Dry-run job flag.
    pub dry_run: bool,
    /// Queue timestamp.
    pub queued_at: chrono::DateTime<chrono::Utc>,
    /// Start timestamp.
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Completion timestamp.
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last error text.
    pub last_error: Option<String>,
}

/// Media job operation row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobOperationRow {
    /// Operation ordering index.
    pub operation_index: i32,
    /// Operation kind.
    pub operation_kind: String,
    /// Optional stream id for stream-scoped operations.
    pub stream_id: Option<i32>,
    /// Command binary.
    pub command_bin: String,
    /// Optional argument 1.
    pub arg_1: Option<String>,
    /// Optional argument 2.
    pub arg_2: Option<String>,
    /// Optional argument 3.
    pub arg_3: Option<String>,
    /// Optional argument 4.
    pub arg_4: Option<String>,
    /// Optional argument 5.
    pub arg_5: Option<String>,
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create media job row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn create_media_job(pool: &PgPool, input: &CreateMediaJobInput<'_>) -> Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(MEDIA_JOB_CREATE_V1)
        .bind(input.actor_public_id)
        .bind(input.media_profile_public_id)
        .bind(input.source_path)
        .bind(input.output_path.unwrap_or_default())
        .bind(input.dry_run)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job create"))
}

/// Append or update a media job phase row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_phase(
    pool: &PgPool,
    media_job_public_id: Uuid,
    phase_index: i32,
    phase_name: &str,
    phase_status_text: &str,
    details_text: Option<&str>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_PHASE_APPEND_V1)
        .bind(media_job_public_id)
        .bind(phase_index)
        .bind(phase_name)
        .bind(phase_status_text)
        .bind(details_text.unwrap_or_default())
        .execute(pool)
        .await
        .map_err(try_op("media job phase append"))?;

    Ok(())
}

/// Append or update a media job operation row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_operation(
    pool: &PgPool,
    media_job_public_id: Uuid,
    operation_index: i32,
    operation_kind: &str,
    stream_id: Option<i32>,
    command_bin: &str,
    args: [Option<&str>; 5],
) -> Result<()> {
    sqlx::query(MEDIA_JOB_OPERATION_APPEND_V1)
        .bind(media_job_public_id)
        .bind(operation_index)
        .bind(operation_kind)
        .bind(stream_id)
        .bind(command_bin)
        .bind(args[0].unwrap_or_default())
        .bind(args[1].unwrap_or_default())
        .bind(args[2].unwrap_or_default())
        .bind(args[3].unwrap_or_default())
        .bind(args[4].unwrap_or_default())
        .execute(pool)
        .await
        .map_err(try_op("media job operation append"))?;
    Ok(())
}

/// List media jobs for profile and optional status.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_jobs(
    pool: &PgPool,
    media_profile_public_id: Uuid,
    status_text: Option<&str>,
) -> Result<Vec<MediaJobRow>> {
    sqlx::query_as::<_, MediaJobRow>(MEDIA_JOB_LIST_V1)
        .bind(media_profile_public_id)
        .bind(status_text)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job list"))
}

/// List media job operations for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_operations(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobOperationRow>> {
    sqlx::query_as::<_, MediaJobOperationRow>(MEDIA_JOB_OPERATION_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job operation list"))
}

/// Get one media job by public id.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn get_media_job(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Option<MediaJobRow>> {
    sqlx::query_as::<_, MediaJobRow>(MEDIA_JOB_GET_V1)
        .bind(media_job_public_id)
        .fetch_optional(pool)
        .await
        .map_err(try_op("media job get"))
}

/// Cancel one queued/running/verifying media job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn cancel_media_job(pool: &PgPool, media_job_public_id: Uuid) -> Result<()> {
    sqlx::query(MEDIA_JOB_CANCEL_V1)
        .bind(media_job_public_id)
        .execute(pool)
        .await
        .map_err(try_op("media job cancel"))?;
    Ok(())
}

/// Retry one failed/cancelled media job by requeueing it.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn retry_media_job(pool: &PgPool, media_job_public_id: Uuid) -> Result<()> {
    sqlx::query(MEDIA_JOB_RETRY_V1)
        .bind(media_job_public_id)
        .execute(pool)
        .await
        .map_err(try_op("media job retry"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CreateMediaJobInput, append_media_job_operation, append_media_job_phase, create_media_job,
        get_media_job, list_media_job_operations, list_media_jobs,
    };
    use crate::media::profiles::{UpsertMediaProfileInput, upsert_media_profile};
    use crate::media::schema_tests::setup_media_db;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use uuid::Uuid;

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

    #[tokio::test]
    async fn create_and_list_media_job() -> anyhow::Result<()> {
        let db = match setup_media_db("create_and_list_media_job").await {
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
                profile_key: "tv-jobs",
                source_root: "/input/tv",
                output_root: "/output/tv",
                dry_run_only: true,
                retention_days: 30,
            },
        )
        .await?;

        let job_id = create_media_job(
            db.pool(),
            &CreateMediaJobInput {
                actor_public_id: db.system_user_public_id,
                media_profile_public_id: profile_id,
                source_path: "/input/tv/show.mkv",
                output_path: Some("/output/tv/show.mkv"),
                dry_run: true,
            },
        )
        .await?;

        append_media_job_phase(
            db.pool(),
            job_id,
            0,
            "planning",
            "queued",
            Some("scheduled"),
        )
        .await?;

        append_media_job_operation(
            db.pool(),
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

        let rows = list_media_jobs(db.pool(), profile_id, Some("queued")).await?;
        assert!(rows.iter().any(|item| item.media_job_public_id == job_id));

        let job = get_media_job(db.pool(), job_id).await?;
        assert!(job.is_some());
        let Some(job) = job else {
            return Ok(());
        };
        assert_eq!(job.media_job_public_id, job_id);

        let operations = list_media_job_operations(db.pool(), job_id).await?;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].operation_index, 0);
        assert_eq!(operations[0].operation_kind, "remux");
        assert_eq!(operations[0].command_bin, "ffmpeg");
        Ok(())
    }

    #[tokio::test]
    async fn media_job_queries_surface_query_errors_without_database() {
        let pool = closed_pool().await;
        let profile_id = Uuid::new_v4();
        let job_id = Uuid::new_v4();
        let actor_id = Uuid::new_v4();

        let create = create_media_job(
            &pool,
            &CreateMediaJobInput {
                actor_public_id: actor_id,
                media_profile_public_id: profile_id,
                source_path: "/input/movie.mkv",
                output_path: None,
                dry_run: true,
            },
        )
        .await;
        assert!(create.is_err());

        let append = append_media_job_phase(&pool, job_id, 0, "plan", "queued", None).await;
        assert!(append.is_err());

        let list = list_media_jobs(&pool, profile_id, Some("queued")).await;
        assert!(list.is_err());

        let get = get_media_job(&pool, job_id).await;
        assert!(get.is_err());

        let operations = list_media_job_operations(&pool, job_id).await;
        assert!(operations.is_err());
    }
}
