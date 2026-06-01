//! Stored-procedure access for media job lifecycle.

use crate::error::{Result, try_op};
use sqlx::PgPool;
use uuid::Uuid;

const MEDIA_JOB_CREATE_V1: &str = "SELECT media_job_create_v1(actor_public_id_input => $1, media_profile_public_id_input => $2, source_path_input => $3, output_path_input => $4, dry_run_input => $5)";
const MEDIA_JOB_PHASE_APPEND_V1: &str = "SELECT media_job_phase_append_v1(media_job_public_id_input => $1, phase_index_input => $2, phase_name_input => $3, phase_status_input => $4, details_text_input => $5)";
const MEDIA_JOB_OPERATION_APPEND_V1: &str = "SELECT media_job_operation_append_v1(media_job_public_id_input => $1, operation_index_input => $2, operation_kind_input => $3, stream_id_input => $4, command_bin_input => $5, arg_1_input => $6, arg_2_input => $7, arg_3_input => $8, arg_4_input => $9, arg_5_input => $10)";
const MEDIA_JOB_OPERATION_LIST_V1: &str = "SELECT operation_index, operation_kind, stream_id, command_bin, arg_1, arg_2, arg_3, arg_4, arg_5, created_at FROM media_job_operation_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_VIOLATION_APPEND_V1: &str = "SELECT media_job_violation_append_v1(media_job_public_id_input => $1, violation_index_input => $2, violation_kind_input => $3, severity_input => $4, stream_id_input => $5)";
const MEDIA_JOB_VIOLATION_LIST_V1: &str = "SELECT violation_index, violation_kind, severity, stream_id, created_at FROM media_job_violation_list_v1(media_job_public_id_input => $1)";
const MEDIA_JOB_PLAN_REASON_APPEND_V1: &str = "SELECT media_job_plan_reason_append_v1(media_job_public_id_input => $1, reason_index_input => $2, candidate_index_input => $3, selected_input => $4, reason_code_input => $5, reason_text_input => $6)";
const MEDIA_JOB_PLAN_REASON_LIST_V1: &str = "SELECT reason_index, candidate_index, selected, reason_code, reason_text, created_at FROM media_job_plan_reason_list_v1(media_job_public_id_input => $1)";
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

/// Media job compliance violation row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobViolationRow {
    /// Violation ordering index.
    pub violation_index: i32,
    /// Violation kind.
    pub violation_kind: String,
    /// Violation severity.
    pub severity: String,
    /// Optional stream id for stream-scoped violations.
    pub stream_id: Option<i32>,
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Media job plan explanation row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobPlanReasonRow {
    /// Reason ordering index.
    pub reason_index: i32,
    /// Optional candidate index for rejected/selected candidates.
    pub candidate_index: Option<i32>,
    /// Whether this reason describes the selected plan.
    pub selected: bool,
    /// Stable reason code.
    pub reason_code: String,
    /// Human-readable reason text.
    pub reason_text: String,
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

/// Append or update a media job compliance violation row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_violation(
    pool: &PgPool,
    media_job_public_id: Uuid,
    violation_index: i32,
    violation_kind: &str,
    severity: &str,
    stream_id: Option<i32>,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_VIOLATION_APPEND_V1)
        .bind(media_job_public_id)
        .bind(violation_index)
        .bind(violation_kind)
        .bind(severity)
        .bind(stream_id)
        .execute(pool)
        .await
        .map_err(try_op("media job violation append"))?;
    Ok(())
}

/// Append or update a media job plan-reason row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn append_media_job_plan_reason(
    pool: &PgPool,
    media_job_public_id: Uuid,
    reason_index: i32,
    candidate_index: Option<i32>,
    selected: bool,
    reason_code: &str,
    reason_text: &str,
) -> Result<()> {
    sqlx::query(MEDIA_JOB_PLAN_REASON_APPEND_V1)
        .bind(media_job_public_id)
        .bind(reason_index)
        .bind(candidate_index)
        .bind(selected)
        .bind(reason_code)
        .bind(reason_text)
        .execute(pool)
        .await
        .map_err(try_op("media job plan reason append"))?;
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

/// List media job compliance violations for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_violations(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobViolationRow>> {
    sqlx::query_as::<_, MediaJobViolationRow>(MEDIA_JOB_VIOLATION_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job violation list"))
}

/// List media job plan reasons for one job.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_job_plan_reasons(
    pool: &PgPool,
    media_job_public_id: Uuid,
) -> Result<Vec<MediaJobPlanReasonRow>> {
    sqlx::query_as::<_, MediaJobPlanReasonRow>(MEDIA_JOB_PLAN_REASON_LIST_V1)
        .bind(media_job_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media job plan reason list"))
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
        CreateMediaJobInput, append_media_job_operation, append_media_job_phase,
        append_media_job_plan_reason, append_media_job_violation, create_media_job, get_media_job,
        list_media_job_operations, list_media_job_plan_reasons, list_media_job_violations,
        list_media_jobs,
    };
    use crate::media::profiles::{UpsertMediaProfileInput, upsert_media_profile};
    use crate::media::schema_tests::setup_media_db;
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };
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

    async fn append_and_assert_plan_reason(pool: &PgPool, job_id: Uuid) -> anyhow::Result<()> {
        append_media_job_plan_reason(
            pool,
            job_id,
            0,
            Some(0),
            true,
            "least_cost_selected",
            "Selected the least-cost compliant candidate.",
        )
        .await?;
        let plan_reasons = list_media_job_plan_reasons(pool, job_id).await?;
        assert_eq!(plan_reasons.len(), 1);
        assert_eq!(plan_reasons[0].reason_index, 0);
        assert_eq!(plan_reasons[0].candidate_index, Some(0));
        assert!(plan_reasons[0].selected);
        assert_eq!(plan_reasons[0].reason_code, "least_cost_selected");
        assert_eq!(
            plan_reasons[0].reason_text,
            "Selected the least-cost compliant candidate."
        );
        Ok(())
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
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
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

        append_media_job_violation(
            db.pool(),
            job_id,
            0,
            "video_codec_mismatch",
            "high",
            Some(0),
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

        let violations = list_media_job_violations(db.pool(), job_id).await?;
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_index, 0);
        assert_eq!(violations[0].violation_kind, "video_codec_mismatch");
        assert_eq!(violations[0].severity, "high");
        assert_eq!(violations[0].stream_id, Some(0));

        append_and_assert_plan_reason(db.pool(), job_id).await?;
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

        let append_violation =
            append_media_job_violation(&pool, job_id, 0, "codec_mismatch", "high", Some(0)).await;
        assert!(append_violation.is_err());

        let violations = list_media_job_violations(&pool, job_id).await;
        assert!(violations.is_err());

        let append_reason = append_media_job_plan_reason(
            &pool,
            job_id,
            0,
            Some(0),
            true,
            "least_cost_selected",
            "Selected candidate.",
        )
        .await;
        assert!(append_reason.is_err());

        let reasons = list_media_job_plan_reasons(&pool, job_id).await;
        assert!(reasons.is_err());
    }
}
