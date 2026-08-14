ALTER TABLE media_compatibility_target
    ADD COLUMN IF NOT EXISTS audio_channels INT,
    ADD COLUMN IF NOT EXISTS audio_channel_layout TEXT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conname = 'media_compatibility_target_audio_channels_positive'
    ) THEN
        ALTER TABLE media_compatibility_target
            ADD CONSTRAINT media_compatibility_target_audio_channels_positive CHECK (
                audio_channels IS NULL OR audio_channels > 0
            );
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conname = 'media_compatibility_target_audio_layout_nonempty'
    ) THEN
        ALTER TABLE media_compatibility_target
            ADD CONSTRAINT media_compatibility_target_audio_layout_nonempty CHECK (
                audio_channel_layout IS NULL
                OR NULLIF(btrim(audio_channel_layout), '') IS NOT NULL
            );
    END IF;
END;
$$;

ALTER TABLE media_job
    ADD COLUMN IF NOT EXISTS intent_target_audio_channels INT,
    ADD COLUMN IF NOT EXISTS intent_target_audio_channel_layout TEXT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conname = 'media_job_intent_target_audio_channels_positive'
    ) THEN
        ALTER TABLE media_job
            ADD CONSTRAINT media_job_intent_target_audio_channels_positive CHECK (
                intent_target_audio_channels IS NULL OR intent_target_audio_channels > 0
            );
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conname = 'media_job_intent_target_audio_layout_nonempty'
    ) THEN
        ALTER TABLE media_job
            ADD CONSTRAINT media_job_intent_target_audio_layout_nonempty CHECK (
                intent_target_audio_channel_layout IS NULL
                OR NULLIF(btrim(intent_target_audio_channel_layout), '') IS NOT NULL
            );
    END IF;
END;
$$;

DROP FUNCTION IF EXISTS media_compatibility_target_list_v1();

CREATE OR REPLACE FUNCTION media_compatibility_target_list_v1()
RETURNS TABLE (
    compatibility_target_key TEXT,
    version INT,
    display_name TEXT,
    video_codec TEXT,
    audio_codec TEXT,
    audio_channels INT,
    audio_channel_layout TEXT,
    subtitle_policy TEXT
)
LANGUAGE sql
STABLE
AS $$
    SELECT target.compatibility_target_key,
           target.version,
           target.display_name,
           target.video_codec,
           target.audio_codec,
           target.audio_channels,
           target.audio_channel_layout,
           target.subtitle_policy
      FROM media_compatibility_target AS target
     WHERE target.enabled
     ORDER BY lower(target.compatibility_target_key), target.version DESC;
$$;

DROP FUNCTION IF EXISTS media_compatibility_target_upsert_v1(
    UUID,
    TEXT,
    INT,
    TEXT,
    TEXT,
    TEXT,
    TEXT
);

CREATE OR REPLACE FUNCTION media_compatibility_target_upsert_v1(
    actor_public_id_input UUID,
    compatibility_target_key_input TEXT,
    version_input INT,
    display_name_input TEXT,
    video_codec_input TEXT,
    audio_codec_input TEXT,
    audio_channels_input INT,
    audio_channel_layout_input TEXT,
    subtitle_policy_input TEXT
)
RETURNS TABLE (
    compatibility_target_key TEXT,
    version INT,
    display_name TEXT,
    video_codec TEXT,
    audio_codec TEXT,
    audio_channels INT,
    audio_channel_layout TEXT,
    subtitle_policy TEXT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
#variable_conflict use_column
DECLARE
    actor_id BIGINT;
    version_value INT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    version_value := COALESCE(version_input, 1);

    RETURN QUERY
    INSERT INTO media_compatibility_target (
        compatibility_target_key,
        version,
        display_name,
        video_codec,
        audio_codec,
        audio_channels,
        audio_channel_layout,
        subtitle_policy,
        enabled,
        updated_at
    )
    VALUES (
        btrim(compatibility_target_key_input),
        version_value,
        btrim(display_name_input),
        lower(btrim(video_codec_input)),
        lower(btrim(audio_codec_input)),
        audio_channels_input,
        NULLIF(lower(btrim(audio_channel_layout_input)), ''),
        lower(btrim(subtitle_policy_input)),
        TRUE,
        now()
    )
    ON CONFLICT (lower(compatibility_target_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_codec = EXCLUDED.video_codec,
        audio_codec = EXCLUDED.audio_codec,
        audio_channels = EXCLUDED.audio_channels,
        audio_channel_layout = EXCLUDED.audio_channel_layout,
        subtitle_policy = EXCLUDED.subtitle_policy,
        enabled = TRUE,
        updated_at = now()
    RETURNING
        media_compatibility_target.compatibility_target_key,
        media_compatibility_target.version,
        media_compatibility_target.display_name,
        media_compatibility_target.video_codec,
        media_compatibility_target.audio_codec,
        media_compatibility_target.audio_channels,
        media_compatibility_target.audio_channel_layout,
        media_compatibility_target.subtitle_policy;
END;
$$;

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
    target_row media_compatibility_target%ROWTYPE;
    policy_row media_policy_profile%ROWTYPE;
    media_job_public_id_out UUID;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    SELECT *
      INTO profile_row
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_row.media_profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    IF profile_row.compatibility_target_key IS NOT NULL THEN
        SELECT *
          INTO target_row
          FROM media_compatibility_target
         WHERE lower(compatibility_target_key) = lower(replace(profile_row.compatibility_target_key, '_', '-'))
           AND enabled
         ORDER BY version DESC, media_compatibility_target_id DESC
         LIMIT 1;

        IF target_row.media_compatibility_target_id IS NULL THEN
            RAISE EXCEPTION 'compatibility target not found'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_not_found';
        END IF;
    END IF;

    SELECT *
      INTO policy_row
      FROM media_policy_profile
     WHERE lower(policy_key) = lower(COALESCE(profile_row.policy_key, 'safe_dry_run'))
       AND enabled
     ORDER BY version DESC, media_policy_profile_id DESC
     LIMIT 1;

    IF policy_row.media_policy_profile_id IS NULL THEN
        RAISE EXCEPTION 'policy profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_profile_not_found';
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
        intent_compatibility_target_id,
        intent_compatibility_target_version,
        intent_target_video_codec,
        intent_target_audio_codec,
        intent_target_audio_channels,
        intent_target_audio_channel_layout,
        intent_target_subtitle_policy,
        intent_policy_profile_id,
        intent_policy_version,
        intent_policy_video_intent,
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
        target_row.media_compatibility_target_id,
        target_row.version,
        target_row.video_codec,
        target_row.audio_codec,
        target_row.audio_channels,
        target_row.audio_channel_layout,
        target_row.subtitle_policy,
        policy_row.media_policy_profile_id,
        policy_row.version,
        policy_row.video_intent,
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
    policy_key TEXT,
    target_video_codec TEXT,
    target_audio_codec TEXT,
    target_audio_channels INT,
    target_audio_channel_layout TEXT,
    target_subtitle_policy TEXT,
    policy_video_intent TEXT
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
         WHERE mj.status = media_job_status_queued_v1()
         ORDER BY mj.queued_at ASC, mj.media_job_id ASC
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ),
    updated AS (
        UPDATE media_job mj
           SET status = media_job_status_running_v1(),
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
                   mj.intent_policy_key,
                   mj.intent_target_video_codec,
                   mj.intent_target_audio_codec,
                   mj.intent_target_audio_channels,
                   mj.intent_target_audio_channel_layout,
                   mj.intent_target_subtitle_policy,
                   mj.intent_policy_video_intent
    )
    SELECT updated.media_job_public_id,
           mp.media_profile_public_id,
           updated.source_path,
           updated.output_path,
           updated.dry_run,
           updated.intent_source_root,
           updated.intent_output_root,
           updated.intent_compatibility_target_key,
           updated.intent_policy_key,
           updated.intent_target_video_codec,
           updated.intent_target_audio_codec,
           updated.intent_target_audio_channels,
           updated.intent_target_audio_channel_layout,
           updated.intent_target_subtitle_policy,
           updated.intent_policy_video_intent
      FROM updated
      JOIN media_profile mp ON mp.media_profile_id = updated.media_profile_id;
END;
$$;
