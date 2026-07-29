CREATE INDEX IF NOT EXISTS ix_media_job_active_heartbeat
    ON media_job (status, heartbeat_at, started_at, media_job_id)
    WHERE status IN (media_job_status_running_v1(), media_job_status_verifying_v1());

CREATE OR REPLACE FUNCTION media_job_worker_recover_stale_v1(
    stale_after_seconds_input INT
)
RETURNS TABLE (
    media_job_public_id UUID,
    status media_job_status,
    last_error TEXT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    IF stale_after_seconds_input IS NULL OR stale_after_seconds_input < 0 THEN
        RAISE EXCEPTION 'stale worker recovery interval invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_stale_after_invalid';
    END IF;

    RETURN QUERY
    UPDATE media_job job
       SET status = CASE
               WHEN job.cancel_generation > job.cancel_acknowledged_generation
                   THEN media_job_status_cancelled_v1()
               ELSE media_job_status_failed_v1()
           END,
           cancel_acknowledged_generation = CASE
               WHEN job.cancel_generation > job.cancel_acknowledged_generation
                   THEN job.cancel_generation
               ELSE job.cancel_acknowledged_generation
           END,
           completed_at = now(),
           last_error = CASE
               WHEN job.cancel_generation > job.cancel_acknowledged_generation
                   THEN NULL
               ELSE 'media_job_worker_heartbeat_stale'
           END
     WHERE job.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND COALESCE(job.heartbeat_at, job.started_at, job.queued_at)
           <= now() - make_interval(secs => stale_after_seconds_input)
    RETURNING job.media_job_public_id,
              job.status,
              job.last_error;
END;
$$;
