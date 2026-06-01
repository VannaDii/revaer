CREATE OR REPLACE FUNCTION media_job_mark_completed_v1(
    media_job_public_id_input UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    affected_count INTEGER;
    current_status media_job_status;
BEGIN
    UPDATE media_job
       SET status = 'completed'::media_job_status,
           completed_at = now(),
           last_error = NULL
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN (
           'queued'::media_job_status,
           'running'::media_job_status,
           'verifying'::media_job_status
       );

    GET DIAGNOSTICS affected_count = ROW_COUNT;
    IF affected_count > 0 THEN
        RETURN;
    END IF;

    SELECT status INTO current_status
    FROM media_job
    WHERE media_job_public_id = media_job_public_id_input;

    IF current_status IS NULL THEN
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
    END IF;

    IF current_status = 'completed'::media_job_status THEN
        RETURN;
    END IF;

    RAISE EXCEPTION 'media job cannot be completed from current status'
        USING ERRCODE = 'P0001', DETAIL = 'media_job_complete_invalid_status';
END;
$$;

CREATE OR REPLACE FUNCTION media_job_cleanup_completed_v1(
    as_of_input TIMESTAMPTZ
)
RETURNS INTEGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    IF as_of_input IS NULL THEN
        RAISE EXCEPTION 'media cleanup timestamp is required'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_cleanup_as_of_required';
    END IF;

    WITH expired_jobs AS (
        SELECT mj.media_job_id
        FROM media_job mj
        JOIN media_profile mp ON mp.media_profile_id = mj.media_profile_id
        WHERE mj.status = 'completed'::media_job_status
          AND mj.completed_at IS NOT NULL
          AND mj.completed_at <= as_of_input - make_interval(days => mp.retention_days)
    ),
    deleted AS (
        DELETE FROM media_job mj
        USING expired_jobs ej
        WHERE mj.media_job_id = ej.media_job_id
        RETURNING mj.media_job_id
    )
    SELECT COUNT(*)::INTEGER INTO deleted_count
    FROM deleted;

    RETURN COALESCE(deleted_count, 0);
END;
$$;
