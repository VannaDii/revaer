CREATE TABLE IF NOT EXISTS media_capability_snapshot_feature (
    media_capability_snapshot_feature_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    snapshot_run_public_id UUID NOT NULL,
    feature_family TEXT NOT NULL,
    feature_name TEXT NOT NULL,
    supported BOOLEAN NOT NULL DEFAULT TRUE,
    detail_text TEXT,
    observed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    observed_by_user_id BIGINT NOT NULL REFERENCES app_user(user_id),
    CONSTRAINT media_capability_snapshot_feature_family_nonempty CHECK (btrim(feature_family) <> ''),
    CONSTRAINT media_capability_snapshot_feature_name_nonempty CHECK (btrim(feature_name) <> '')
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_capability_snapshot_feature_run_family_name
    ON media_capability_snapshot_feature (
        snapshot_run_public_id,
        lower(feature_family),
        lower(feature_name)
    );

CREATE INDEX IF NOT EXISTS ix_media_capability_snapshot_feature_run_observed
    ON media_capability_snapshot_feature (
        snapshot_run_public_id,
        observed_at DESC,
        media_capability_snapshot_feature_id DESC
    );

CREATE OR REPLACE FUNCTION media_capability_snapshot_feature_record_v1(
    actor_public_id_input UUID,
    snapshot_run_public_id_input UUID,
    feature_family_input TEXT,
    feature_name_input TEXT,
    supported_input BOOLEAN,
    detail_text_input TEXT
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    feature_id_out BIGINT;
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

    INSERT INTO media_capability_snapshot_feature (
        snapshot_run_public_id,
        feature_family,
        feature_name,
        supported,
        detail_text,
        observed_by_user_id
    )
    VALUES (
        snapshot_run_public_id_input,
        lower(btrim(feature_family_input)),
        lower(btrim(feature_name_input)),
        COALESCE(supported_input, TRUE),
        NULLIF(btrim(COALESCE(detail_text_input, '')), ''),
        actor_id
    )
    ON CONFLICT (
        snapshot_run_public_id,
        lower(feature_family),
        lower(feature_name)
    )
    DO UPDATE SET
        supported = EXCLUDED.supported,
        detail_text = EXCLUDED.detail_text,
        observed_at = EXCLUDED.observed_at,
        observed_by_user_id = EXCLUDED.observed_by_user_id
    RETURNING media_capability_snapshot_feature_id
    INTO feature_id_out;

    RETURN feature_id_out;
END;
$$;

CREATE OR REPLACE FUNCTION media_capability_snapshot_feature_list_v1(
    snapshot_run_public_id_input UUID
)
RETURNS TABLE (
    feature_family TEXT,
    feature_name TEXT,
    supported BOOLEAN,
    detail_text TEXT,
    observed_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT mcsf.feature_family,
           mcsf.feature_name,
           mcsf.supported,
           mcsf.detail_text,
           mcsf.observed_at
      FROM media_capability_snapshot_feature mcsf
     WHERE mcsf.snapshot_run_public_id = snapshot_run_public_id_input
     ORDER BY lower(mcsf.feature_family),
              lower(mcsf.feature_name),
              mcsf.media_capability_snapshot_feature_id;
$$;

ALTER TABLE media_job
    ADD COLUMN IF NOT EXISTS intent_source_root TEXT,
    ADD COLUMN IF NOT EXISTS intent_output_root TEXT,
    ADD COLUMN IF NOT EXISTS intent_compatibility_target_key TEXT,
    ADD COLUMN IF NOT EXISTS intent_policy_key TEXT;

UPDATE media_job mj
   SET intent_source_root = COALESCE(mj.intent_source_root, mp.source_root),
       intent_output_root = COALESCE(mj.intent_output_root, mp.output_root),
       intent_compatibility_target_key = COALESCE(
           mj.intent_compatibility_target_key,
           mp.compatibility_target_key
       ),
       intent_policy_key = COALESCE(mj.intent_policy_key, mp.policy_key, 'safe_dry_run')
  FROM media_profile mp
 WHERE mp.media_profile_id = mj.media_profile_id;

ALTER TABLE media_job
    ALTER COLUMN intent_source_root SET NOT NULL,
    ALTER COLUMN intent_output_root SET NOT NULL,
    ALTER COLUMN intent_policy_key SET NOT NULL;

ALTER TABLE media_job
    ADD CONSTRAINT media_job_intent_roots_nonempty CHECK (
        btrim(intent_source_root) <> '' AND btrim(intent_output_root) <> ''
    ),
    ADD CONSTRAINT media_job_intent_policy_key_nonempty CHECK (
        btrim(intent_policy_key) <> ''
    ),
    ADD CONSTRAINT media_job_intent_compatibility_target_nonempty CHECK (
        intent_compatibility_target_key IS NULL
        OR btrim(intent_compatibility_target_key) <> ''
    );

CREATE OR REPLACE FUNCTION media_job_create_v1(
    actor_public_id_input UUID,
    media_profile_public_id_input UUID,
    source_path_input TEXT,
    output_path_input TEXT,
    dry_run_input BOOLEAN
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    profile_row media_profile%ROWTYPE;
    media_job_public_id_out UUID;
BEGIN
    SELECT user_id
      INTO actor_id
      FROM app_user
     WHERE user_public_id = actor_public_id_input;

    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'actor not found'
            USING ERRCODE = 'P0001', DETAIL = 'app_user_not_found';
    END IF;

    SELECT *
      INTO profile_row
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_row.media_profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_profile_not_found';
    END IF;

    INSERT INTO media_job (
        media_profile_id,
        source_path,
        output_path,
        dry_run,
        intent_source_root,
        intent_output_root,
        intent_compatibility_target_key,
        intent_policy_key,
        created_by_user_id
    )
    VALUES (
        profile_row.media_profile_id,
        btrim(source_path_input),
        NULLIF(btrim(output_path_input), ''),
        COALESCE(dry_run_input, TRUE),
        profile_row.source_root,
        profile_row.output_root,
        profile_row.compatibility_target_key,
        profile_row.policy_key,
        actor_id
    )
    RETURNING media_job_public_id
    INTO media_job_public_id_out;

    RETURN media_job_public_id_out;
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
         WHERE mj.status = 'queued'::media_job_status
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
         RETURNING mj.media_job_public_id,
                   mj.media_profile_id,
                   mj.source_path,
                   mj.output_path,
                   mj.dry_run,
                   mj.intent_source_root,
                   mj.intent_output_root,
                   mj.intent_compatibility_target_key,
                   mj.intent_policy_key
    )
    SELECT updated.media_job_public_id,
           mp.media_profile_public_id,
           updated.source_path,
           updated.output_path,
           updated.dry_run,
           updated.intent_source_root,
           updated.intent_output_root,
           updated.intent_compatibility_target_key,
           updated.intent_policy_key
      FROM updated
      JOIN media_profile mp ON mp.media_profile_id = updated.media_profile_id;
END;
$$;
