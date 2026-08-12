//! Runtime media persistence facade wrapping media stored-procedure access.
//!
//! # Design
//! - Keep API and workers insulated from the `revaer-data` module layout.
//! - Expose strongly typed methods for media profiles, jobs, and capabilities.

use revaer_data::DataResult;
use revaer_data::media::capabilities::{
    CapabilitySnapshotRow, RecordCapabilityEncoderInput, RecordCapabilityFeatureInput,
    RecordCapabilitySnapshotInput, latest_capability_snapshot, record_capability_encoder,
    record_capability_feature, record_capability_snapshot,
};
use revaer_data::media::configuration::{
    MediaCompatibilityTargetRow, MediaJobRetentionPolicyRow, MediaPolicyProfileRow,
    UpdateMediaJobRetentionPolicyInput, UpsertMediaCompatibilityTargetInput,
    UpsertMediaPolicyProfileInput, get_media_job_retention_policy,
    list_media_compatibility_targets, list_media_policy_profiles,
    update_media_job_retention_policy, upsert_media_compatibility_target,
    upsert_media_policy_profile,
};
use revaer_data::media::jobs::{
    AppendMediaJobArtifactInput, AppendMediaJobCompactAuditInput,
    AppendMediaJobVerificationCheckInput, ClaimedMediaJobRow, CreateMediaJobInput,
    EnqueueDiscoveredMediaJobInput, MediaJobArtifactRow, MediaJobCompactAuditRow,
    MediaJobControlRow, MediaJobDesiredTargetStreamRow, MediaJobOperationRow,
    MediaJobPlanReasonRow, MediaJobRetentionRunRow, MediaJobRow, MediaJobVerificationCheckRow,
    MediaJobViolationRow, MediaRecentJobRow, append_media_job_artifact,
    append_media_job_compact_audit, append_media_job_operation, append_media_job_phase,
    append_media_job_plan_reason, append_media_job_verification_check, append_media_job_violation,
    cancel_media_job, create_media_job, enqueue_discovered_media_job, get_media_job,
    list_media_job_artifacts, list_media_job_compact_audits, list_media_job_desired_target_streams,
    list_media_job_operations, list_media_job_plan_reasons, list_media_job_verification_checks,
    list_media_job_violations, list_media_jobs, list_recent_media_jobs, mark_media_job_completed,
    media_job_worker_acknowledge_cancel, media_job_worker_claim_next, media_job_worker_complete,
    media_job_worker_heartbeat, media_job_worker_mark_status, media_job_worker_poll_control,
    retry_media_job, run_media_job_retention,
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

    /// Read a bounded keyset page of recent jobs with diagnostic counts.
    ///
    /// # Errors
    ///
    /// Returns an error when the read model rejects the request or persistence fails.
    pub async fn list_recent_jobs(
        &self,
        limit: i32,
        cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
        media_profile_public_id: Option<Uuid>,
    ) -> DataResult<Vec<MediaRecentJobRow>> {
        list_recent_media_jobs(&self.pool, limit, cursor, media_profile_public_id).await
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

    /// Atomically enqueue a changed file discovered by a watcher or scheduled scan.
    ///
    /// # Errors
    ///
    /// Returns an error when the durable fingerprint claim or job creation fails.
    pub async fn enqueue_discovered_job(
        &self,
        input: &EnqueueDiscoveredMediaJobInput<'_>,
    ) -> DataResult<Option<Uuid>> {
        enqueue_discovered_media_job(&self.pool, input).await
    }

    /// List active compatibility targets.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_compatibility_targets(&self) -> DataResult<Vec<MediaCompatibilityTargetRow>> {
        list_media_compatibility_targets(&self.pool).await
    }

    /// Create or replace a compatibility target version.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn upsert_compatibility_target(
        &self,
        input: UpsertMediaCompatibilityTargetInput<'_>,
    ) -> DataResult<MediaCompatibilityTargetRow> {
        upsert_media_compatibility_target(&self.pool, input).await
    }

    /// List active media policy profiles.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_policy_profiles(&self) -> DataResult<Vec<MediaPolicyProfileRow>> {
        list_media_policy_profiles(&self.pool).await
    }

    /// Create or replace a media policy profile version.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn upsert_policy_profile(
        &self,
        input: UpsertMediaPolicyProfileInput<'_>,
    ) -> DataResult<MediaPolicyProfileRow> {
        upsert_media_policy_profile(&self.pool, input).await
    }

    /// Read active media job retention policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn get_job_retention_policy(&self) -> DataResult<Option<MediaJobRetentionPolicyRow>> {
        get_media_job_retention_policy(&self.pool).await
    }

    /// Update active media job retention policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn update_job_retention_policy(
        &self,
        input: UpdateMediaJobRetentionPolicyInput,
    ) -> DataResult<MediaJobRetentionPolicyRow> {
        update_media_job_retention_policy(&self.pool, input).await
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

    /// Append a planner explanation reason for a media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn append_job_plan_reason(
        &self,
        media_job_public_id: Uuid,
        reason_index: i32,
        candidate_index: Option<i32>,
        selected: bool,
        reason_code: &str,
        reason_text: &str,
    ) -> DataResult<()> {
        append_media_job_plan_reason(
            &self.pool,
            media_job_public_id,
            reason_index,
            candidate_index,
            selected,
            reason_code,
            reason_text,
        )
        .await
    }

    /// Append a verification check for a media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn append_job_verification_check(
        &self,
        input: &AppendMediaJobVerificationCheckInput<'_>,
    ) -> DataResult<()> {
        append_media_job_verification_check(&self.pool, input).await
    }

    /// Append an artifact reference for a media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn append_job_artifact(
        &self,
        input: &AppendMediaJobArtifactInput<'_>,
    ) -> DataResult<()> {
        append_media_job_artifact(&self.pool, input).await
    }

    /// Append a compact audit fact for a media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn append_job_compact_audit(
        &self,
        input: &AppendMediaJobCompactAuditInput<'_>,
    ) -> DataResult<()> {
        append_media_job_compact_audit(&self.pool, input).await
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

    /// List persisted planner explanation reasons for one media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_job_plan_reasons(
        &self,
        media_job_public_id: Uuid,
    ) -> DataResult<Vec<MediaJobPlanReasonRow>> {
        list_media_job_plan_reasons(&self.pool, media_job_public_id).await
    }

    /// List persisted verification checks for one media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_job_verification_checks(
        &self,
        media_job_public_id: Uuid,
    ) -> DataResult<Vec<MediaJobVerificationCheckRow>> {
        list_media_job_verification_checks(&self.pool, media_job_public_id).await
    }

    /// List persisted artifact references for one media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_job_artifacts(
        &self,
        media_job_public_id: Uuid,
    ) -> DataResult<Vec<MediaJobArtifactRow>> {
        list_media_job_artifacts(&self.pool, media_job_public_id).await
    }

    /// List persisted compact audit facts for one media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_job_compact_audits(
        &self,
        media_job_public_id: Uuid,
    ) -> DataResult<Vec<MediaJobCompactAuditRow>> {
        list_media_job_compact_audits(&self.pool, media_job_public_id).await
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
        cancel_media_job(&self.pool, media_job_public_id)
            .await
            .map(|_| ())
    }

    /// Retry one media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn retry_job(&self, media_job_public_id: Uuid) -> DataResult<()> {
        retry_media_job(&self.pool, media_job_public_id).await
    }

    /// Mark one queued/running/verifying media job completed.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn mark_job_completed(&self, media_job_public_id: Uuid) -> DataResult<()> {
        mark_media_job_completed(&self.pool, media_job_public_id).await
    }

    /// Claim the next queued media job for worker processing.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn claim_next_job(&self) -> DataResult<Option<ClaimedMediaJobRow>> {
        media_job_worker_claim_next(&self.pool).await
    }

    /// List the immutable desired-target stream snapshot for one claimed job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn list_job_desired_target_streams(
        &self,
        media_job_public_id: Uuid,
    ) -> DataResult<Vec<MediaJobDesiredTargetStreamRow>> {
        list_media_job_desired_target_streams(&self.pool, media_job_public_id).await
    }

    /// Refresh worker heartbeat for a claimed media job.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn heartbeat_job(&self, media_job_public_id: Uuid) -> DataResult<()> {
        media_job_worker_heartbeat(&self.pool, media_job_public_id).await
    }

    /// Refresh heartbeat and read cancellation state for a claimed job.
    ///
    /// # Errors
    ///
    /// Returns an error when the job is no longer worker-owned or execution fails.
    pub async fn poll_job_control(
        &self,
        media_job_public_id: Uuid,
        observed_cancel_generation: i64,
    ) -> DataResult<MediaJobControlRow> {
        media_job_worker_poll_control(&self.pool, media_job_public_id, observed_cancel_generation)
            .await
    }

    /// Acknowledge a pending cancellation and mark the claimed job cancelled.
    ///
    /// # Errors
    ///
    /// Returns an error when no cancellation is pending or execution fails.
    pub async fn acknowledge_job_cancel(
        &self,
        media_job_public_id: Uuid,
        observed_cancel_generation: i64,
    ) -> DataResult<i64> {
        media_job_worker_acknowledge_cancel(
            &self.pool,
            media_job_public_id,
            observed_cancel_generation,
        )
        .await
    }

    /// Atomically complete a job or acknowledge a cancellation that won the terminal race.
    ///
    /// Returns `true` when cancellation won and `false` when completion won.
    ///
    /// # Errors
    ///
    /// Returns an error when the job is no longer worker-owned or execution fails.
    pub async fn complete_job(
        &self,
        media_job_public_id: Uuid,
        observed_cancel_generation: i64,
    ) -> DataResult<bool> {
        media_job_worker_complete(&self.pool, media_job_public_id, observed_cancel_generation).await
    }

    /// Mark a claimed media job with a worker status.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn mark_job_status(
        &self,
        media_job_public_id: Uuid,
        status_text: &str,
        last_error: Option<&str>,
    ) -> DataResult<()> {
        media_job_worker_mark_status(&self.pool, media_job_public_id, status_text, last_error).await
    }

    /// Run the active completed-job and failed-diagnostic retention policies.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn run_job_retention(
        &self,
        as_of: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    ) -> DataResult<MediaJobRetentionRunRow> {
        run_media_job_retention(&self.pool, as_of).await
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

    /// Record one concrete encoder for a capability snapshot run.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn record_capability_encoder(
        &self,
        input: &RecordCapabilityEncoderInput<'_>,
    ) -> DataResult<i64> {
        record_capability_encoder(&self.pool, input).await
    }

    /// Record one feature row for a capability snapshot run.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying stored-procedure call fails.
    pub async fn record_capability_feature(
        &self,
        input: &RecordCapabilityFeatureInput<'_>,
    ) -> DataResult<i64> {
        record_capability_feature(&self.pool, input).await
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
    use revaer_data::media::capabilities::{
        RecordCapabilityEncoderInput, RecordCapabilityFeatureInput, RecordCapabilitySnapshotInput,
        complete_capability_snapshot_run_with_executor, record_capability_snapshot_with_executor,
        start_capability_snapshot_run_with_executor,
    };
    use revaer_data::media::configuration::{
        MediaVerificationToggle, UpdateMediaJobRetentionPolicyInput,
        UpsertMediaCompatibilityTargetInput, UpsertMediaPolicyProfileInput,
    };
    use revaer_data::media::jobs::{
        AppendMediaJobArtifactInput, AppendMediaJobCompactAuditInput,
        AppendMediaJobVerificationCheckInput, CreateMediaJobInput, EnqueueDiscoveredMediaJobInput,
    };
    use revaer_data::media::profiles::{UpdateMediaProfileInput, UpsertMediaProfileInput};
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

    async fn append_and_assert_verification_check(
        store: &MediaStore,
        job_id: Uuid,
    ) -> anyhow::Result<()> {
        store
            .append_job_verification_check(&AppendMediaJobVerificationCheckInput {
                media_job_public_id: job_id,
                check_index: 0,
                check_kind: "duration",
                check_status: "passed",
                expected_value: Some("3600.0"),
                actual_value: Some("3599.9"),
                details_text: Some("within tolerance"),
            })
            .await?;
        let verification_checks = store.list_job_verification_checks(job_id).await?;
        assert_eq!(verification_checks.len(), 1);
        assert_eq!(verification_checks[0].check_kind, "duration");
        assert_eq!(verification_checks[0].check_status, "passed");
        Ok(())
    }

    async fn append_and_assert_artifact_and_audit(
        store: &MediaStore,
        job_id: Uuid,
    ) -> anyhow::Result<()> {
        store
            .append_job_artifact(&AppendMediaJobArtifactInput {
                media_job_public_id: job_id,
                artifact_index: 0,
                artifact_kind: "ffprobe_json",
                artifact_path: "jobs/abc/ffprobe.json",
                size_bytes: Some(2048),
                content_type: Some("application/json"),
            })
            .await?;
        let artifacts = store.list_job_artifacts(job_id).await?;
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].artifact_kind, "ffprobe_json");
        assert_eq!(artifacts[0].artifact_path, "jobs/abc/ffprobe.json");

        store
            .append_job_compact_audit(&AppendMediaJobCompactAuditInput {
                media_job_public_id: job_id,
                audit_index: 0,
                fact_kind: "replacement",
                fact_text: "source preserved before replace",
            })
            .await?;
        let audits = store.list_job_compact_audits(job_id).await?;
        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].fact_kind, "replacement");
        assert_eq!(audits[0].fact_text, "source preserved before replace");
        Ok(())
    }

    async fn assert_verification_check_errors(store: &MediaStore, job_id: Uuid) {
        assert!(
            store
                .append_job_verification_check(&AppendMediaJobVerificationCheckInput {
                    media_job_public_id: job_id,
                    check_index: 0,
                    check_kind: "duration",
                    check_status: "passed",
                    expected_value: Some("3600.0"),
                    actual_value: Some("3599.9"),
                    details_text: Some("within tolerance"),
                })
                .await
                .is_err()
        );
        assert!(store.list_job_verification_checks(job_id).await.is_err());
    }

    async fn assert_artifact_and_audit_errors(store: &MediaStore, job_id: Uuid) {
        assert!(
            store
                .append_job_artifact(&AppendMediaJobArtifactInput {
                    media_job_public_id: job_id,
                    artifact_index: 0,
                    artifact_kind: "ffprobe_json",
                    artifact_path: "jobs/abc/ffprobe.json",
                    size_bytes: Some(2048),
                    content_type: Some("application/json"),
                })
                .await
                .is_err()
        );
        assert!(store.list_job_artifacts(job_id).await.is_err());
        assert!(
            store
                .append_job_compact_audit(&AppendMediaJobCompactAuditInput {
                    media_job_public_id: job_id,
                    audit_index: 0,
                    fact_kind: "replacement",
                    fact_text: "source preserved before replace",
                })
                .await
                .is_err()
        );
        assert!(store.list_job_compact_audits(job_id).await.is_err());
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
        store
            .append_job_plan_reason(
                job_id,
                0,
                Some(0),
                true,
                "least_cost_selected",
                "Selected the least-cost compliant candidate.",
            )
            .await?;
        append_and_assert_verification_check(&store, job_id).await?;
        append_and_assert_artifact_and_audit(&store, job_id).await?;

        let jobs = store.list_jobs(profile_id, Some("queued")).await?;
        assert!(jobs.iter().any(|job| job.media_job_public_id == job_id));
        let operations = store.list_job_operations(job_id).await?;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].operation_kind, "remux");
        let violations = store.list_job_violations(job_id).await?;
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violation_kind, "video_codec_mismatch");
        let plan_reasons = store.list_job_plan_reasons(job_id).await?;
        assert_eq!(plan_reasons.len(), 1);
        assert_eq!(plan_reasons[0].reason_code, "least_cost_selected");

        let snapshot_id = record_completed_capability(&store, actor).await?;
        assert!(snapshot_id > 0);
        let latest = store.latest_capability().await?;
        assert!(latest.is_some());

        Ok(())
    }

    async fn record_completed_capability(store: &MediaStore, actor: Uuid) -> anyhow::Result<i64> {
        let snapshot_run_public_id = Uuid::new_v4();
        let mut transaction = store.pool().begin().await?;
        start_capability_snapshot_run_with_executor(
            &mut *transaction,
            actor,
            snapshot_run_public_id,
        )
        .await?;
        let snapshot_id = record_capability_snapshot_with_executor(
            &mut *transaction,
            &RecordCapabilitySnapshotInput {
                actor_public_id: actor,
                snapshot_run_public_id: Some(snapshot_run_public_id),
                ffmpeg_version: "7.0",
                ffprobe_version: "7.0",
                codec_name: "h264",
                encode_supported: true,
                decode_supported: true,
            },
        )
        .await?;
        complete_capability_snapshot_run_with_executor(&mut *transaction, snapshot_run_public_id)
            .await?;
        transaction.commit().await?;
        Ok(snapshot_id)
    }

    #[tokio::test]
    async fn media_store_cleans_up_expired_completed_jobs() -> anyhow::Result<()> {
        let Some((postgres, store)) = test_store().await? else {
            return Ok(());
        };
        let _keep_db_alive = postgres;
        let actor = system_actor(store.pool()).await?;

        let profile_id = store
            .upsert_profile(&UpsertMediaProfileInput {
                actor_public_id: actor,
                profile_key: "retained-runtime",
                source_root: "/input/runtime",
                output_root: "/output/runtime",
                dry_run_only: true,
                retention_days: 1,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            })
            .await?;
        let job_id = store
            .create_job(&CreateMediaJobInput {
                actor_public_id: actor,
                media_profile_public_id: profile_id,
                source_path: "/input/runtime/finished.mkv",
                output_path: Some("/output/runtime/finished.mkv"),
                dry_run: true,
            })
            .await?;

        store.mark_job_completed(job_id).await?;
        store
            .update_job_retention_policy(UpdateMediaJobRetentionPolicyInput {
                actor_public_id: actor,
                completed_enabled: true,
                completed_mode: "age".to_string(),
                completed_limit: 1,
                failed_diagnostic_enabled: false,
                failed_diagnostic_mode: "age".to_string(),
                failed_diagnostic_limit: 30,
            })
            .await?;
        let outcome = store
            .run_job_retention(chrono::Utc::now() + chrono::Duration::days(2))
            .await?;

        assert_eq!(outcome.completed_jobs_deleted, 1);
        assert!(store.get_job(job_id).await?.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn media_store_cleans_up_failed_terminal_diagnostics() -> anyhow::Result<()> {
        let Some((postgres, store)) = test_store().await? else {
            return Ok(());
        };
        let _keep_db_alive = postgres;
        let actor = system_actor(store.pool()).await?;
        let profile_id = store
            .upsert_profile(&UpsertMediaProfileInput {
                actor_public_id: actor,
                profile_key: "diagnostic-runtime",
                source_root: "/input/diagnostic-runtime",
                output_root: "/output/diagnostic-runtime",
                dry_run_only: true,
                retention_days: 3650,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            })
            .await?;
        let job_id = store
            .create_job(&CreateMediaJobInput {
                actor_public_id: actor,
                media_profile_public_id: profile_id,
                source_path: "/input/diagnostic-runtime/cancelled.mkv",
                output_path: Some("/output/diagnostic-runtime/cancelled.mkv"),
                dry_run: true,
            })
            .await?;

        store
            .append_job_violation(job_id, 0, "video_codec_mismatch", "high", Some(0))
            .await?;
        store
            .append_job_plan_reason(
                job_id,
                0,
                Some(0),
                true,
                "least_cost_selected",
                "Selected candidate.",
            )
            .await?;
        store
            .append_job_verification_check(&AppendMediaJobVerificationCheckInput {
                media_job_public_id: job_id,
                check_index: 0,
                check_kind: "duration",
                check_status: "passed",
                expected_value: Some("3600.0"),
                actual_value: Some("3599.9"),
                details_text: Some("within tolerance"),
            })
            .await?;
        store
            .append_job_artifact(&AppendMediaJobArtifactInput {
                media_job_public_id: job_id,
                artifact_index: 0,
                artifact_kind: "ffprobe_json",
                artifact_path: "jobs/runtime/ffprobe.json",
                size_bytes: Some(2048),
                content_type: Some("application/json"),
            })
            .await?;
        store
            .append_job_compact_audit(&AppendMediaJobCompactAuditInput {
                media_job_public_id: job_id,
                audit_index: 0,
                fact_kind: "replacement",
                fact_text: "source preserved before replace",
            })
            .await?;
        store.cancel_job(job_id).await?;

        let outcome = store
            .run_job_retention(chrono::Utc::now() + chrono::Duration::days(31))
            .await?;

        assert_eq!(outcome.failed_jobs_pruned, 1);
        assert_eq!(outcome.failed_detail_rows_deleted, 4);
        assert!(store.get_job(job_id).await?.is_some());
        assert!(store.list_job_violations(job_id).await?.is_empty());
        assert!(store.list_job_plan_reasons(job_id).await?.is_empty());
        assert!(store.list_job_verification_checks(job_id).await?.is_empty());
        assert!(store.list_job_artifacts(job_id).await?.is_empty());
        assert_eq!(store.list_job_compact_audits(job_id).await?.len(), 1);
        Ok(())
    }

    async fn assert_profile_and_catalog_errors(
        store: &MediaStore,
        actor_id: Uuid,
        profile_id: Uuid,
    ) {
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
        assert!(
            store
                .update_profile(&UpdateMediaProfileInput {
                    actor_public_id: actor_id,
                    media_profile_public_id: profile_id,
                    source_root: Some("/input/movies-renamed"),
                    output_root: Some("/output/movies-renamed"),
                    dry_run_only: Some(true),
                    retention_days: Some(45),
                    compatibility_target_key: Some("chromecast"),
                    policy_key: Some("balanced"),
                    watcher_enabled: Some(false),
                    schedule_enabled: Some(false),
                    schedule_interval_minutes: None,
                })
                .await
                .is_err()
        );
        assert!(store.list_profiles().await.is_err());
        assert!(store.get_profile(profile_id).await.is_err());
        assert!(
            store
                .enqueue_discovered_job(&EnqueueDiscoveredMediaJobInput {
                    actor_public_id: actor_id,
                    media_profile_public_id: profile_id,
                    source_path: "/input/movies/file.mkv",
                    output_path: "/output/movies/file.mkv",
                    source_size_bytes: 2048,
                    source_modified_ns: 123_456_789,
                    source_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                })
                .await
                .is_err()
        );
        assert!(store.list_compatibility_targets().await.is_err());
        assert!(
            store
                .upsert_compatibility_target(UpsertMediaCompatibilityTargetInput {
                    actor_public_id: actor_id,
                    compatibility_target_key: "chromecast",
                    version: 1,
                    display_name: "Chromecast",
                    video_codec: "h264",
                    audio_codec: "aac",
                    audio_channels: Some(2),
                    audio_channel_layout: Some("stereo"),
                    subtitle_policy: "text",
                })
                .await
                .is_err()
        );
        assert!(store.list_policy_profiles().await.is_err());
        assert!(
            store
                .upsert_policy_profile(UpsertMediaPolicyProfileInput {
                    actor_public_id: actor_id,
                    policy_key: "balanced",
                    version: 1,
                    display_name: "Balanced",
                    video_intent: "compatibility",
                    verification_strictness: "balanced",
                    verification_duration_tolerance_millis: 500,
                    verification_mux_validation: MediaVerificationToggle::from(true),
                    verification_decode_all_streams: MediaVerificationToggle::from(true),
                    verification_keyframe_seek: MediaVerificationToggle::from(false),
                    verification_playback_probe: MediaVerificationToggle::from(false),
                })
                .await
                .is_err()
        );
        assert!(store.get_job_retention_policy().await.is_err());
    }

    async fn assert_retention_policy_errors(store: &MediaStore, actor_id: Uuid) {
        assert!(
            store
                .update_job_retention_policy(UpdateMediaJobRetentionPolicyInput {
                    actor_public_id: actor_id,
                    completed_enabled: true,
                    completed_mode: "age".to_string(),
                    completed_limit: 30,
                    failed_diagnostic_enabled: true,
                    failed_diagnostic_mode: "age".to_string(),
                    failed_diagnostic_limit: 7,
                })
                .await
                .is_err()
        );
        assert!(store.run_job_retention(chrono::Utc::now()).await.is_err());
    }

    async fn assert_job_and_worker_errors(store: &MediaStore, actor_id: Uuid, profile_id: Uuid) {
        let job_id = Uuid::new_v4();

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
        assert!(
            store
                .append_job_plan_reason(
                    job_id,
                    0,
                    Some(0),
                    true,
                    "least_cost_selected",
                    "Selected candidate.",
                )
                .await
                .is_err()
        );
        assert!(store.list_job_plan_reasons(job_id).await.is_err());
        assert_verification_check_errors(store, job_id).await;
        assert_artifact_and_audit_errors(store, job_id).await;
        assert!(store.cancel_job(job_id).await.is_err());
        assert!(store.retry_job(job_id).await.is_err());
        assert!(store.mark_job_completed(job_id).await.is_err());
        assert!(store.claim_next_job().await.is_err());
        assert!(store.list_job_desired_target_streams(job_id).await.is_err());
        assert!(store.heartbeat_job(job_id).await.is_err());
        assert!(store.poll_job_control(job_id, 0).await.is_err());
        assert!(store.acknowledge_job_cancel(job_id, 0).await.is_err());
        assert!(store.complete_job(job_id, 0).await.is_err());
        assert!(
            store
                .mark_job_status(job_id, "running", Some("transcoding"))
                .await
                .is_err()
        );
    }

    async fn assert_capability_errors(store: &MediaStore, actor_id: Uuid) {
        assert!(
            store
                .record_capability(&RecordCapabilitySnapshotInput {
                    actor_public_id: actor_id,
                    snapshot_run_public_id: Some(Uuid::new_v4()),
                    ffmpeg_version: "7.1",
                    ffprobe_version: "7.1",
                    codec_name: "h264",
                    encode_supported: true,
                    decode_supported: true,
                })
                .await
                .is_err()
        );
        assert!(
            store
                .record_capability_encoder(&RecordCapabilityEncoderInput {
                    actor_public_id: actor_id,
                    snapshot_run_public_id: Uuid::new_v4(),
                    encoder_name: "libx264",
                })
                .await
                .is_err()
        );
        assert!(
            store
                .record_capability_feature(&RecordCapabilityFeatureInput {
                    actor_public_id: actor_id,
                    snapshot_run_public_id: Uuid::new_v4(),
                    feature_family: "muxer",
                    feature_name: "matroska",
                    supported: true,
                    detail_text: None,
                })
                .await
                .is_err()
        );
        assert!(store.latest_capability().await.is_err());
    }

    #[tokio::test]
    async fn media_store_methods_surface_query_errors_without_database() {
        let store = MediaStore::new(closed_pool().await);
        let actor_id = Uuid::new_v4();
        let profile_id = Uuid::new_v4();

        assert_profile_and_catalog_errors(&store, actor_id, profile_id).await;
        assert_retention_policy_errors(&store, actor_id).await;
        assert_job_and_worker_errors(&store, actor_id, profile_id).await;
        assert_capability_errors(&store, actor_id).await;
    }

    #[tokio::test]
    async fn media_store_cleanup_methods_surface_query_errors_without_database() {
        let store = MediaStore::new(closed_pool().await);
        let job_id = Uuid::new_v4();

        assert!(store.mark_job_completed(job_id).await.is_err());
        assert!(store.run_job_retention(chrono::Utc::now()).await.is_err());
    }
}
