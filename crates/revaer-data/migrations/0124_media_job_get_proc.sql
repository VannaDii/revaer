CREATE OR REPLACE FUNCTION media_job_get_v1(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    media_job_public_id UUID,
    source_path TEXT,
    output_path TEXT,
    status media_job_status,
    dry_run BOOLEAN,
    queued_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    last_error TEXT
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        mj.media_job_public_id,
        mj.source_path,
        mj.output_path,
        mj.status,
        mj.dry_run,
        mj.queued_at,
        mj.started_at,
        mj.completed_at,
        mj.last_error
    FROM media_job mj
    WHERE mj.media_job_public_id = media_job_public_id_input;
$$;
