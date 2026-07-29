CREATE OR REPLACE FUNCTION media_job_phase_list_v1(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    phase_index INT,
    phase_name TEXT,
    phase_status media_job_status,
    details_text TEXT,
    created_at TIMESTAMPTZ
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT
        mjp.phase_index,
        mjp.phase_name,
        mjp.phase_status,
        mjp.details_text,
        mjp.created_at
    FROM media_job_phase mjp
    JOIN media_job mj ON mj.media_job_id = mjp.media_job_id
    WHERE mj.media_job_public_id = media_job_public_id_input
    ORDER BY mjp.phase_index ASC;
$$;
