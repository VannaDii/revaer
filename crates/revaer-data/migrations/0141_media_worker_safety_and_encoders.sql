CREATE TABLE IF NOT EXISTS media_capability_snapshot_encoder (
    media_capability_snapshot_encoder_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    snapshot_run_public_id UUID NOT NULL,
    encoder_name TEXT NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    observed_by_user_id BIGINT NOT NULL REFERENCES app_user(user_id),
    CONSTRAINT media_capability_snapshot_encoder_name_nonempty CHECK (btrim(encoder_name) <> '')
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_capability_snapshot_encoder_run_name
    ON media_capability_snapshot_encoder (snapshot_run_public_id, lower(encoder_name));

CREATE INDEX IF NOT EXISTS ix_media_capability_snapshot_encoder_run_observed
    ON media_capability_snapshot_encoder (snapshot_run_public_id, observed_at DESC, media_capability_snapshot_encoder_id DESC);

CREATE OR REPLACE FUNCTION media_capability_snapshot_encoder_record_v1(
    actor_public_id_input UUID,
    snapshot_run_public_id_input UUID,
    encoder_name_input TEXT
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    encoder_id_out BIGINT;
BEGIN
    SELECT user_id
      INTO actor_id
      FROM app_user
     WHERE user_public_id = actor_public_id_input;

    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'actor not found'
            USING ERRCODE = 'P0001', DETAIL = 'app_user_not_found';
    END IF;

    IF snapshot_run_public_id_input IS NULL THEN
        RAISE EXCEPTION 'snapshot run id required'
            USING ERRCODE = 'P0001', DETAIL = 'media_capability_snapshot_run_required';
    END IF;

    INSERT INTO media_capability_snapshot_encoder (
        snapshot_run_public_id,
        encoder_name,
        observed_by_user_id
    )
    VALUES (
        snapshot_run_public_id_input,
        btrim(encoder_name_input),
        actor_id
    )
    ON CONFLICT (snapshot_run_public_id, lower(encoder_name))
    DO UPDATE SET
        observed_at = EXCLUDED.observed_at,
        observed_by_user_id = EXCLUDED.observed_by_user_id
    RETURNING media_capability_snapshot_encoder_id
    INTO encoder_id_out;

    RETURN encoder_id_out;
END;
$$;

CREATE OR REPLACE FUNCTION media_capability_snapshot_encoder_list_v1(
    snapshot_run_public_id_input UUID
)
RETURNS TABLE (
    encoder_name TEXT,
    observed_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT mcse.encoder_name,
           mcse.observed_at
      FROM media_capability_snapshot_encoder mcse
     WHERE mcse.snapshot_run_public_id = snapshot_run_public_id_input
     ORDER BY lower(mcse.encoder_name), mcse.media_capability_snapshot_encoder_id;
$$;

CREATE OR REPLACE FUNCTION media_job_cancel_v1(
    media_job_public_id_input UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    UPDATE media_job
       SET status = 'cancelled'::media_job_status,
           completed_at = now()
     WHERE media_job_public_id = media_job_public_id_input
       AND status = 'queued'::media_job_status;

    IF NOT FOUND THEN
        IF EXISTS (
            SELECT 1
              FROM media_job
             WHERE media_job_public_id = media_job_public_id_input
        ) THEN
            RAISE EXCEPTION 'job cancel blocked by status'
                USING ERRCODE = 'P0001', DETAIL = 'media_job_cancel_invalid_status';
        END IF;
        RAISE EXCEPTION 'job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
    END IF;
END;
$$;

DROP FUNCTION IF EXISTS media_job_worker_claim_next_v1();

CREATE OR REPLACE FUNCTION media_job_worker_claim_next_v1()
RETURNS TABLE (
    media_job_public_id UUID,
    media_profile_public_id UUID,
    source_path TEXT,
    output_path TEXT,
    dry_run BOOLEAN,
    source_root TEXT,
    output_root TEXT,
    compatibility_target_key TEXT,
    policy_key TEXT
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
           mp.output_root,
           mp.compatibility_target_key,
           mp.policy_key
      FROM updated
      JOIN media_profile mp ON mp.media_profile_id = updated.media_profile_id
     WHERE mp.deleted_at IS NULL;
END;
$$;
