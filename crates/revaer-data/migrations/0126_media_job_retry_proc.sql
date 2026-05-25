CREATE OR REPLACE FUNCTION media_job_retry_v1(
    media_job_public_id_input UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    UPDATE media_job
       SET status = 'queued'::media_job_status,
           queued_at = now(),
           started_at = NULL,
           completed_at = NULL,
           last_error = NULL
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN ('failed'::media_job_status, 'cancelled'::media_job_status);

    IF NOT FOUND THEN
        IF EXISTS (
            SELECT 1
              FROM media_job
             WHERE media_job_public_id = media_job_public_id_input
        ) THEN
            RAISE EXCEPTION 'job retry blocked by status'
                USING ERRCODE = 'P0001', DETAIL = 'media_job_retry_invalid_status';
        END IF;
        RAISE EXCEPTION 'job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
    END IF;
END;
$$;
