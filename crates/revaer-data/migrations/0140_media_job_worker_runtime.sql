ALTER TABLE media_job
ADD COLUMN IF NOT EXISTS heartbeat_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS ix_media_job_worker_queue
    ON media_job (status, queued_at ASC, media_job_id ASC)
    WHERE status = 'queued'::media_job_status;

CREATE OR REPLACE FUNCTION media_job_worker_claim_next_v1()
RETURNS TABLE (
    media_job_public_id UUID,
    media_profile_public_id UUID,
    source_path TEXT,
    output_path TEXT,
    dry_run BOOLEAN,
    source_root TEXT,
    output_root TEXT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT mj.media_job_id
          FROM media_job mj
          JOIN media_profile mp ON mp.media_profile_id = mj.media_profile_id
         WHERE mj.status = 'queued'::media_job_status
           AND mp.deleted_at IS NULL
         ORDER BY mj.queued_at ASC, mj.media_job_id ASC
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ),
    updated AS (
        UPDATE media_job mj
           SET status = 'running'::media_job_status,
               started_at = COALESCE(mj.started_at, now()),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL
          FROM claimed
         WHERE mj.media_job_id = claimed.media_job_id
         RETURNING mj.media_job_id,
                   mj.media_job_public_id,
                   mj.media_profile_id,
                   mj.source_path,
                   mj.output_path,
                   mj.dry_run
    )
    SELECT updated.media_job_public_id,
           mp.media_profile_public_id,
           updated.source_path,
           updated.output_path,
           updated.dry_run,
           mp.source_root,
           mp.output_root
      FROM updated
      JOIN media_profile mp ON mp.media_profile_id = updated.media_profile_id
     WHERE mp.deleted_at IS NULL;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_worker_heartbeat_v1(
    media_job_public_id_input UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    rows_updated INT;
BEGIN
    UPDATE media_job
       SET heartbeat_at = now()
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN ('running'::media_job_status, 'verifying'::media_job_status);

    GET DIAGNOSTICS rows_updated = ROW_COUNT;
    IF rows_updated = 0 THEN
        RAISE EXCEPTION 'media job not running'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_worker_heartbeat_invalid_status';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_worker_mark_status_v1(
    media_job_public_id_input UUID,
    status_input media_job_status,
    last_error_input TEXT DEFAULT NULL
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    current_status media_job_status;
BEGIN
    UPDATE media_job
       SET status = status_input,
           heartbeat_at = CASE
               WHEN status_input IN ('running'::media_job_status, 'verifying'::media_job_status)
                   THEN now()
               ELSE heartbeat_at
           END,
           completed_at = CASE
               WHEN status_input IN (
                   'completed'::media_job_status,
                   'failed'::media_job_status,
                   'cancelled'::media_job_status
               )
                   THEN now()
               ELSE completed_at
           END,
           last_error = NULLIF(btrim(COALESCE(last_error_input, '')), '')
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN ('running'::media_job_status, 'verifying'::media_job_status)
       AND status_input IN (
           'running'::media_job_status,
           'verifying'::media_job_status,
           'completed'::media_job_status,
           'failed'::media_job_status,
           'cancelled'::media_job_status
       )
     RETURNING status INTO current_status;

    IF current_status IS NULL THEN
        RAISE EXCEPTION 'media job worker status transition invalid'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_worker_status_invalid';
    END IF;
END;
$$;
