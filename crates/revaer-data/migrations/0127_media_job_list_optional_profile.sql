CREATE OR REPLACE FUNCTION media_job_list_v1(
    media_profile_public_id_input UUID DEFAULT NULL,
    status_input media_job_status DEFAULT NULL
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
    JOIN media_profile mp ON mp.media_profile_id = mj.media_profile_id
    WHERE mp.deleted_at IS NULL
      AND (
          media_profile_public_id_input IS NULL
          OR mp.media_profile_public_id = media_profile_public_id_input
      )
      AND (status_input IS NULL OR mj.status = status_input)
    ORDER BY mj.queued_at DESC;
$$;
