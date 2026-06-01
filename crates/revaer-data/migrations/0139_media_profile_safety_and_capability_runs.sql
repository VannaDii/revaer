ALTER TABLE media_capability_snapshot
    ADD COLUMN snapshot_run_public_id UUID;

UPDATE media_capability_snapshot
   SET snapshot_run_public_id = gen_random_uuid()
 WHERE snapshot_run_public_id IS NULL;

ALTER TABLE media_capability_snapshot
    ALTER COLUMN snapshot_run_public_id SET NOT NULL;

CREATE INDEX ix_media_capability_snapshot_run_observed
    ON media_capability_snapshot (snapshot_run_public_id, observed_at DESC, media_capability_snapshot_id DESC);

CREATE OR REPLACE FUNCTION media_profile_upsert_v2(
    actor_public_id_input UUID,
    profile_key_input TEXT,
    source_root_input TEXT,
    output_root_input TEXT,
    dry_run_only_input BOOLEAN,
    retention_days_input INT,
    compatibility_target_key_input TEXT,
    policy_key_input TEXT,
    watcher_enabled_input BOOLEAN,
    schedule_enabled_input BOOLEAN,
    schedule_interval_minutes_input INT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    profile_public_id_out UUID;
    compatibility_target_key_value TEXT;
    policy_key_value TEXT;
    schedule_enabled_value BOOLEAN;
BEGIN
    SELECT user_id
      INTO actor_id
      FROM app_user
     WHERE user_public_id = actor_public_id_input;

    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'actor not found'
            USING ERRCODE = 'P0001', DETAIL = 'app_user_not_found';
    END IF;

    compatibility_target_key_value := NULLIF(btrim(compatibility_target_key_input), '');
    policy_key_value := COALESCE(NULLIF(btrim(policy_key_input), ''), 'safe_dry_run');
    schedule_enabled_value := COALESCE(schedule_enabled_input, FALSE);

    IF schedule_enabled_value AND schedule_interval_minutes_input IS NULL THEN
        RAISE EXCEPTION 'schedule interval required'
            USING ERRCODE = 'P0001', DETAIL = 'media_profile_schedule_interval_required';
    END IF;

    IF lower(btrim(source_root_input)) = lower(btrim(output_root_input))
       OR lower(btrim(source_root_input)) LIKE lower(btrim(output_root_input)) || '/%'
       OR lower(btrim(output_root_input)) LIKE lower(btrim(source_root_input)) || '/%' THEN
        RAISE EXCEPTION 'profile roots overlap'
            USING ERRCODE = 'P0001', DETAIL = 'media_profile_roots_overlap';
    END IF;

    INSERT INTO media_profile (
        profile_key,
        source_root,
        output_root,
        dry_run_only,
        retention_days,
        compatibility_target_key,
        policy_key,
        watcher_enabled,
        schedule_enabled,
        schedule_interval_minutes,
        created_by_user_id
    )
    VALUES (
        btrim(profile_key_input),
        btrim(source_root_input),
        btrim(output_root_input),
        TRUE,
        COALESCE(retention_days_input, 30),
        compatibility_target_key_value,
        policy_key_value,
        COALESCE(watcher_enabled_input, FALSE),
        schedule_enabled_value,
        schedule_interval_minutes_input,
        actor_id
    )
    ON CONFLICT ((lower(profile_key)))
    WHERE deleted_at IS NULL
    DO UPDATE SET
        source_root = EXCLUDED.source_root,
        output_root = EXCLUDED.output_root,
        retention_days = EXCLUDED.retention_days,
        compatibility_target_key = EXCLUDED.compatibility_target_key,
        policy_key = EXCLUDED.policy_key,
        watcher_enabled = EXCLUDED.watcher_enabled,
        schedule_enabled = EXCLUDED.schedule_enabled,
        schedule_interval_minutes = EXCLUDED.schedule_interval_minutes,
        updated_at = now()
    RETURNING media_profile_public_id
    INTO profile_public_id_out;

    RETURN profile_public_id_out;
END;
$$;

CREATE OR REPLACE FUNCTION media_capability_snapshot_record_v2(
    actor_public_id_input UUID,
    snapshot_run_public_id_input UUID,
    ffmpeg_version_input TEXT,
    ffprobe_version_input TEXT,
    codec_name_input TEXT,
    encode_supported_input BOOLEAN,
    decode_supported_input BOOLEAN
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    snapshot_id_out BIGINT;
BEGIN
    SELECT user_id
      INTO actor_id
      FROM app_user
     WHERE user_public_id = actor_public_id_input;

    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'actor not found'
            USING ERRCODE = 'P0001', DETAIL = 'app_user_not_found';
    END IF;

    INSERT INTO media_capability_snapshot (
        snapshot_run_public_id,
        ffmpeg_version,
        ffprobe_version,
        codec_name,
        encode_supported,
        decode_supported,
        observed_by_user_id
    )
    VALUES (
        COALESCE(snapshot_run_public_id_input, gen_random_uuid()),
        btrim(ffmpeg_version_input),
        btrim(ffprobe_version_input),
        btrim(codec_name_input),
        COALESCE(encode_supported_input, FALSE),
        COALESCE(decode_supported_input, TRUE),
        actor_id
    )
    RETURNING media_capability_snapshot_id
    INTO snapshot_id_out;

    RETURN snapshot_id_out;
END;
$$;

CREATE OR REPLACE FUNCTION media_capability_snapshot_latest_v2()
RETURNS TABLE (
    media_capability_snapshot_id BIGINT,
    snapshot_run_public_id UUID,
    ffmpeg_version TEXT,
    ffprobe_version TEXT,
    codec_name TEXT,
    encode_supported BOOLEAN,
    decode_supported BOOLEAN,
    observed_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    WITH latest_run AS (
        SELECT mcs.snapshot_run_public_id
        FROM media_capability_snapshot mcs
        GROUP BY mcs.snapshot_run_public_id
        ORDER BY max(mcs.observed_at) DESC, max(mcs.media_capability_snapshot_id) DESC
        LIMIT 1
    )
    SELECT
        mcs.media_capability_snapshot_id,
        mcs.snapshot_run_public_id,
        mcs.ffmpeg_version,
        mcs.ffprobe_version,
        mcs.codec_name,
        mcs.encode_supported,
        mcs.decode_supported,
        mcs.observed_at
    FROM media_capability_snapshot mcs
    JOIN latest_run lr ON lr.snapshot_run_public_id = mcs.snapshot_run_public_id
    ORDER BY lower(mcs.codec_name), mcs.media_capability_snapshot_id;
$$;
