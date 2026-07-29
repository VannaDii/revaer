CREATE OR REPLACE FUNCTION media_job_worker_complete_finalized_v1(
    media_job_public_id_input UUID
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    late_cancel_acknowledged BOOLEAN;
BEGIN
    WITH finalizable_job AS (
        SELECT
            media_job_id,
            cancel_generation > cancel_acknowledged_generation AS late_cancel_pending
        FROM media_job
        WHERE media_job_public_id = media_job_public_id_input
          AND status IN (
              media_job_status_verifying_v1(),
              media_job_status_completed_v1()
          )
        FOR UPDATE
    ),
    completed AS (
        UPDATE media_job job
           SET status = media_job_status_completed_v1(),
               cancel_acknowledged_generation = job.cancel_generation,
               completed_at = COALESCE(job.completed_at, now()),
               last_error = NULL
          FROM finalizable_job
         WHERE job.media_job_id = finalizable_job.media_job_id
        RETURNING finalizable_job.late_cancel_pending
    )
    SELECT completed.late_cancel_pending INTO late_cancel_acknowledged
    FROM completed;

    IF late_cancel_acknowledged IS NULL THEN
        RAISE EXCEPTION 'media job finalized worker completion invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_status_invalid';
    END IF;

    RETURN late_cancel_acknowledged;
END;
$$;
