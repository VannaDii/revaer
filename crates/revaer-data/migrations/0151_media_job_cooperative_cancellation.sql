ALTER TABLE media_job
    ADD COLUMN cancel_generation BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN cancel_acknowledged_generation BIGINT NOT NULL DEFAULT 0,
    ADD CONSTRAINT media_job_cancel_generation_nonnegative CHECK (cancel_generation >= 0),
    ADD CONSTRAINT media_job_cancel_acknowledged_nonnegative CHECK (
        cancel_acknowledged_generation >= 0
    ),
    ADD CONSTRAINT media_job_cancel_acknowledged_bounded CHECK (
        cancel_acknowledged_generation <= cancel_generation
    );

CREATE OR REPLACE FUNCTION media_job_cancel_v2(
    media_job_public_id_input UUID
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    requested_generation BIGINT;
BEGIN
    UPDATE media_job
       SET cancel_generation = cancel_generation + 1,
           cancel_acknowledged_generation = CASE
               WHEN status = media_job_status_queued_v1() THEN cancel_generation + 1
               ELSE cancel_acknowledged_generation
           END,
           status = CASE
               WHEN status = media_job_status_queued_v1() THEN media_job_status_cancelled_v1()
               ELSE status
           END,
           completed_at = CASE
               WHEN status = media_job_status_queued_v1() THEN now()
               ELSE completed_at
           END
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN (
           media_job_status_queued_v1(),
           media_job_status_running_v1(),
           media_job_status_verifying_v1()
       )
    RETURNING cancel_generation INTO requested_generation;

    IF requested_generation IS NULL THEN
        IF EXISTS (
            SELECT 1
              FROM media_job
             WHERE media_job_public_id = media_job_public_id_input
        ) THEN
            RAISE EXCEPTION 'job cancel blocked by status'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_cancel_invalid_status';
        END IF;
        RAISE EXCEPTION 'job not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_not_found';
    END IF;

    RETURN requested_generation;
END;
$$;

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
       SET status = media_job_status_queued_v1(),
           queued_at = now(),
           started_at = NULL,
           completed_at = NULL,
           last_error = NULL,
           cancel_acknowledged_generation = cancel_generation
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN (media_job_status_failed_v1(), media_job_status_cancelled_v1());

    IF NOT FOUND THEN
        IF EXISTS (
            SELECT 1
              FROM media_job
             WHERE media_job_public_id = media_job_public_id_input
        ) THEN
            RAISE EXCEPTION 'job retry blocked by status'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_retry_invalid_status';
        END IF;
        RAISE EXCEPTION 'job not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_not_found';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_worker_claim_next_v2()
RETURNS TABLE (
    media_job_public_id UUID,
    media_profile_public_id UUID,
    source_path TEXT,
    output_path TEXT,
    dry_run BOOLEAN,
    source_root TEXT,
    output_root TEXT,
    compatibility_target_key TEXT,
    policy_key TEXT,
    target_video_codec TEXT,
    target_audio_codec TEXT,
    target_audio_channels INT,
    target_audio_channel_layout TEXT,
    target_subtitle_policy TEXT,
    policy_video_intent TEXT,
    desired_target_key TEXT,
    desired_target_version INT,
    desired_container_format TEXT,
    unmatched_stream_policy TEXT,
    cancel_generation BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id
          FROM media_job job
         WHERE job.status = media_job_status_queued_v1()
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ),
    updated AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = COALESCE(job.started_at, now()),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM claimed
         WHERE job.media_job_id = claimed.media_job_id
        RETURNING job.*
    )
    SELECT updated.media_job_public_id, profile.media_profile_public_id,
           updated.source_path, updated.output_path, updated.dry_run,
           updated.intent_source_root, updated.intent_output_root,
           updated.intent_compatibility_target_key, updated.intent_policy_key,
           updated.intent_target_video_codec, updated.intent_target_audio_codec,
           updated.intent_target_audio_channels, updated.intent_target_audio_channel_layout,
           updated.intent_target_subtitle_policy, updated.intent_policy_video_intent,
           updated.intent_desired_target_key, updated.intent_desired_target_version,
           updated.intent_desired_container_format, updated.intent_unmatched_stream_policy,
           updated.cancel_generation
      FROM updated
      JOIN media_profile profile ON profile.media_profile_id = updated.media_profile_id;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_worker_poll_control_v1(
    media_job_public_id_input UUID,
    observed_cancel_generation_input BIGINT
)
RETURNS TABLE (
    cancel_requested BOOLEAN,
    cancel_generation BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    RETURN QUERY
    UPDATE media_job job
       SET heartbeat_at = now()
     WHERE job.media_job_public_id = media_job_public_id_input
       AND job.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
    RETURNING job.cancel_generation > observed_cancel_generation_input,
              job.cancel_generation;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'media job not running'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_poll_invalid_status';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_worker_acknowledge_cancel_v1(
    media_job_public_id_input UUID,
    observed_cancel_generation_input BIGINT
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    acknowledged_generation BIGINT;
BEGIN
    UPDATE media_job
       SET status = media_job_status_cancelled_v1(),
           cancel_acknowledged_generation = cancel_generation,
           completed_at = now(),
           last_error = NULL
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND cancel_generation > observed_cancel_generation_input
    RETURNING cancel_generation INTO acknowledged_generation;

    IF acknowledged_generation IS NULL THEN
        SELECT cancel_generation
          INTO acknowledged_generation
          FROM media_job
         WHERE media_job_public_id = media_job_public_id_input
           AND status = media_job_status_cancelled_v1()
           AND cancel_generation > observed_cancel_generation_input
           AND cancel_acknowledged_generation = cancel_generation;
    END IF;

    IF acknowledged_generation IS NULL THEN
        RAISE EXCEPTION 'media job has no pending cancellation'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_cancel_not_requested';
    END IF;

    RETURN acknowledged_generation;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_worker_complete_v1(
    media_job_public_id_input UUID,
    observed_cancel_generation_input BIGINT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    cancelled BOOLEAN;
BEGIN
    UPDATE media_job
       SET status = CASE
               WHEN cancel_generation > observed_cancel_generation_input
                   THEN media_job_status_cancelled_v1()
               ELSE media_job_status_completed_v1()
           END,
           cancel_acknowledged_generation = CASE
               WHEN cancel_generation > observed_cancel_generation_input
                   THEN cancel_generation
               ELSE cancel_acknowledged_generation
           END,
           completed_at = now(),
           last_error = NULL
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
    RETURNING status = media_job_status_cancelled_v1() INTO cancelled;

    IF cancelled IS NULL THEN
        RAISE EXCEPTION 'media job worker completion invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_status_invalid';
    END IF;

    RETURN cancelled;
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
               WHEN status_input IN (media_job_status_running_v1(), media_job_status_verifying_v1())
                   THEN now()
               ELSE heartbeat_at
           END,
           completed_at = CASE
               WHEN status_input IN (
                   media_job_status_completed_v1(),
                   media_job_status_failed_v1(),
                   media_job_status_cancelled_v1()
               )
                   THEN now()
               ELSE completed_at
           END,
           last_error = NULLIF(btrim(COALESCE(last_error_input, '')), '')
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND status_input IN (
           media_job_status_running_v1(),
           media_job_status_verifying_v1(),
           media_job_status_completed_v1(),
           media_job_status_failed_v1()
       )
       AND (
           status_input = media_job_status_failed_v1()
           OR status_input IN (media_job_status_running_v1(), media_job_status_verifying_v1())
           OR cancel_generation = cancel_acknowledged_generation
       )
     RETURNING status INTO current_status;

    IF current_status IS NULL THEN
        RAISE EXCEPTION 'media job worker status transition invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_status_invalid';
    END IF;
END;
$$;
