ALTER TABLE media_job_retention_policy
    ADD COLUMN workspace_retention_hours INT NOT NULL DEFAULT 24,
    ADD COLUMN diagnostic_workspace_retention_hours INT NOT NULL DEFAULT 720,
    ADD COLUMN workspace_cleanup_batch_size INT NOT NULL DEFAULT 128,
    ADD CONSTRAINT media_job_retention_policy_workspace_hours_bounds CHECK (
        workspace_retention_hours BETWEEN 1 AND 87600
    ),
    ADD CONSTRAINT media_job_retention_policy_diagnostic_workspace_hours_bounds CHECK (
        diagnostic_workspace_retention_hours BETWEEN 1 AND 87600
    ),
    ADD CONSTRAINT media_job_retention_policy_workspace_batch_bounds CHECK (
        workspace_cleanup_batch_size BETWEEN 1 AND 4096
    );

CREATE OR REPLACE FUNCTION media_workspace_retention_snapshot_v1()
RETURNS TABLE (
    media_job_public_id UUID,
    workspace_retention_seconds BIGINT,
    diagnostic_workspace_retention_seconds BIGINT,
    max_entries_per_tick INT
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    WITH policy AS MATERIALIZED (
        SELECT
            retention.workspace_retention_hours::BIGINT * 3600
                AS workspace_retention_seconds,
            retention.diagnostic_workspace_retention_hours::BIGINT * 3600
                AS diagnostic_workspace_retention_seconds,
            retention.workspace_cleanup_batch_size AS max_entries_per_tick
        FROM media_job_retention_policy retention
        WHERE lower(retention.policy_key) = media_retention_policy_default_v1()
          AND retention.enabled
        ORDER BY retention.updated_at DESC, retention.media_job_retention_policy_id DESC
        LIMIT 1
    ), active_jobs AS MATERIALIZED (
        SELECT job.media_job_public_id
        FROM media_job job
        WHERE job.status IN (
            media_job_status_queued_v1(),
            media_job_status_running_v1(),
            media_job_status_verifying_v1()
        )
    )
    SELECT
        active_jobs.media_job_public_id,
        policy.workspace_retention_seconds,
        policy.diagnostic_workspace_retention_seconds,
        policy.max_entries_per_tick
    FROM policy
    LEFT JOIN active_jobs ON TRUE
    ORDER BY active_jobs.media_job_public_id NULLS FIRST;
$$;
