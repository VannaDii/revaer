CREATE OR REPLACE FUNCTION media_subtitle_policy_selected_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'selected'
$$;

CREATE OR REPLACE FUNCTION media_subtitle_policy_all_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'all'
$$;

CREATE OR REPLACE FUNCTION media_subtitle_policy_none_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'none'
$$;

CREATE OR REPLACE FUNCTION media_policy_safe_dry_run_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'safe_dry_run'
$$;

CREATE OR REPLACE FUNCTION media_policy_general_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'general'
$$;

CREATE OR REPLACE FUNCTION media_policy_anime_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'anime'
$$;

CREATE OR REPLACE FUNCTION media_policy_archival_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'archival'
$$;

CREATE OR REPLACE FUNCTION media_retention_policy_default_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'default'
$$;

CREATE OR REPLACE FUNCTION media_capability_run_status_running_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'running'
$$;

CREATE OR REPLACE FUNCTION media_capability_run_status_completed_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'completed'
$$;

CREATE OR REPLACE FUNCTION media_capability_run_status_failed_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'failed'
$$;

CREATE TABLE IF NOT EXISTS media_compatibility_target (
    media_compatibility_target_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    compatibility_target_key TEXT NOT NULL,
    version INT NOT NULL DEFAULT 1,
    display_name TEXT NOT NULL,
    video_codec TEXT NOT NULL,
    audio_codec TEXT NOT NULL,
    subtitle_policy TEXT NOT NULL DEFAULT media_subtitle_policy_selected_v1(),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_compatibility_target_key_nonempty CHECK (btrim(compatibility_target_key) <> ''),
    CONSTRAINT media_compatibility_target_display_nonempty CHECK (btrim(display_name) <> ''),
    CONSTRAINT media_compatibility_target_codecs_nonempty CHECK (
        btrim(video_codec) <> '' AND btrim(audio_codec) <> ''
    ),
    CONSTRAINT media_compatibility_target_subtitle_policy_known CHECK (
        subtitle_policy IN (
            media_subtitle_policy_selected_v1(),
            media_subtitle_policy_all_v1(),
            media_subtitle_policy_none_v1()
        )
    ),
    CONSTRAINT media_compatibility_target_version_positive CHECK (version > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_compatibility_target_key_version
    ON media_compatibility_target (lower(compatibility_target_key), version);

CREATE TABLE IF NOT EXISTS media_policy_profile (
    media_policy_profile_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    policy_key TEXT NOT NULL,
    version INT NOT NULL DEFAULT 1,
    display_name TEXT NOT NULL,
    video_intent TEXT NOT NULL DEFAULT media_policy_general_v1(),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_policy_profile_key_nonempty CHECK (btrim(policy_key) <> ''),
    CONSTRAINT media_policy_profile_display_nonempty CHECK (btrim(display_name) <> ''),
    CONSTRAINT media_policy_profile_intent_known CHECK (
        video_intent IN (
            media_policy_general_v1(),
            media_policy_anime_v1(),
            media_policy_archival_v1()
        )
    ),
    CONSTRAINT media_policy_profile_version_positive CHECK (version > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_policy_profile_key_version
    ON media_policy_profile (lower(policy_key), version);

CREATE TABLE IF NOT EXISTS media_job_retention_policy (
    media_job_retention_policy_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    policy_key TEXT NOT NULL,
    completed_retention_days INT NOT NULL,
    failed_diagnostic_retention_days INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_retention_policy_key_nonempty CHECK (btrim(policy_key) <> ''),
    CONSTRAINT media_job_retention_policy_completed_bounds CHECK (
        completed_retention_days BETWEEN 1 AND 3650
    ),
    CONSTRAINT media_job_retention_policy_failed_bounds CHECK (
        failed_diagnostic_retention_days BETWEEN 1 AND 3650
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_job_retention_policy_key
    ON media_job_retention_policy (lower(policy_key));

CREATE OR REPLACE FUNCTION revaer_config.seed_media_configuration_defaults()
RETURNS VOID AS
$$
BEGIN
    INSERT INTO media_compatibility_target (
        compatibility_target_key,
        version,
        display_name,
        video_codec,
        audio_codec,
        subtitle_policy
    )
    VALUES
        ('hevc-aac', 1, 'HEVC/AAC', 'hevc', 'aac', media_subtitle_policy_selected_v1()),
        ('plex-apple-tv', 1, 'Plex Apple TV HEVC/AAC', 'hevc', 'aac', media_subtitle_policy_selected_v1()),
        ('plex-general-hevc-aac', 1, 'Plex General HEVC/AAC', 'hevc', 'aac', media_subtitle_policy_selected_v1())
    ON CONFLICT (lower(compatibility_target_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_codec = EXCLUDED.video_codec,
        audio_codec = EXCLUDED.audio_codec,
        subtitle_policy = EXCLUDED.subtitle_policy,
        enabled = TRUE,
        updated_at = now();

    INSERT INTO media_policy_profile (
        policy_key,
        version,
        display_name,
        video_intent
    )
    VALUES
        (media_policy_safe_dry_run_v1(), 1, 'Safe dry run', media_policy_general_v1()),
        (media_policy_general_v1(), 1, 'General media', media_policy_general_v1()),
        (media_policy_anime_v1(), 1, 'Anime', media_policy_anime_v1()),
        (media_policy_archival_v1(), 1, 'Archival', media_policy_archival_v1())
    ON CONFLICT (lower(policy_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_intent = EXCLUDED.video_intent,
        enabled = TRUE,
        updated_at = now();

    INSERT INTO media_job_retention_policy (
        policy_key,
        completed_retention_days,
        failed_diagnostic_retention_days
    )
    VALUES (media_retention_policy_default_v1(), 30, 30)
    ON CONFLICT (lower(policy_key)) DO UPDATE SET
        completed_retention_days = EXCLUDED.completed_retention_days,
        failed_diagnostic_retention_days = EXCLUDED.failed_diagnostic_retention_days,
        enabled = TRUE,
        updated_at = now();
END;
$$ LANGUAGE plpgsql;

SELECT revaer_config.seed_media_configuration_defaults();

ALTER TABLE media_job
    ADD COLUMN IF NOT EXISTS intent_compatibility_target_id BIGINT REFERENCES media_compatibility_target(media_compatibility_target_id),
    ADD COLUMN IF NOT EXISTS intent_compatibility_target_version INT,
    ADD COLUMN IF NOT EXISTS intent_target_video_codec TEXT,
    ADD COLUMN IF NOT EXISTS intent_target_audio_codec TEXT,
    ADD COLUMN IF NOT EXISTS intent_target_subtitle_policy TEXT,
    ADD COLUMN IF NOT EXISTS intent_policy_profile_id BIGINT REFERENCES media_policy_profile(media_policy_profile_id),
    ADD COLUMN IF NOT EXISTS intent_policy_version INT,
    ADD COLUMN IF NOT EXISTS intent_policy_video_intent TEXT;

WITH job_config AS (
    SELECT mj.media_job_id,
           target.media_compatibility_target_id,
           target.version AS target_version,
           target.video_codec,
           target.audio_codec,
           target.subtitle_policy,
           policy.media_policy_profile_id,
           policy.version AS policy_version,
           policy.video_intent
      FROM media_job mj
      JOIN media_profile mp
        ON mp.media_profile_id = mj.media_profile_id
      LEFT JOIN LATERAL (
          SELECT media_compatibility_target_id,
                 version,
                 video_codec,
                 audio_codec,
                 subtitle_policy
            FROM media_compatibility_target
           WHERE lower(compatibility_target_key) = lower(replace(COALESCE(
                     mj.intent_compatibility_target_key,
                     mp.compatibility_target_key,
                     ''
                 ), '_', '-'))
             AND enabled
           ORDER BY version DESC, media_compatibility_target_id DESC
           LIMIT 1
      ) target ON TRUE
      JOIN LATERAL (
          SELECT media_policy_profile_id,
                 version,
                 video_intent
            FROM media_policy_profile
           WHERE lower(policy_key) = lower(COALESCE(
                     mj.intent_policy_key,
                     mp.policy_key,
                     media_policy_safe_dry_run_v1()
                 ))
             AND enabled
           ORDER BY version DESC, media_policy_profile_id DESC
           LIMIT 1
      ) policy ON TRUE
)
UPDATE media_job mj
   SET intent_compatibility_target_id = COALESCE(
           mj.intent_compatibility_target_id,
           job_config.media_compatibility_target_id
       ),
       intent_compatibility_target_version = COALESCE(
           mj.intent_compatibility_target_version,
           job_config.target_version
       ),
       intent_target_video_codec = COALESCE(mj.intent_target_video_codec, job_config.video_codec),
       intent_target_audio_codec = COALESCE(mj.intent_target_audio_codec, job_config.audio_codec),
       intent_target_subtitle_policy = COALESCE(
           mj.intent_target_subtitle_policy,
           job_config.subtitle_policy
       ),
       intent_policy_profile_id = COALESCE(
           mj.intent_policy_profile_id,
           job_config.media_policy_profile_id
       ),
       intent_policy_version = COALESCE(mj.intent_policy_version, job_config.policy_version),
       intent_policy_video_intent = COALESCE(mj.intent_policy_video_intent, job_config.video_intent)
  FROM job_config
 WHERE job_config.media_job_id = mj.media_job_id;

ALTER TABLE media_job
    ADD CONSTRAINT media_job_intent_target_codecs_nonempty CHECK (
        (intent_compatibility_target_id IS NULL
            AND intent_target_video_codec IS NULL
            AND intent_target_audio_codec IS NULL
            AND intent_target_subtitle_policy IS NULL)
        OR (
            intent_compatibility_target_id IS NOT NULL
            AND intent_compatibility_target_version IS NOT NULL
            AND btrim(intent_target_video_codec) <> ''
            AND btrim(intent_target_audio_codec) <> ''
            AND intent_target_subtitle_policy IN (
                media_subtitle_policy_selected_v1(),
                media_subtitle_policy_all_v1(),
                media_subtitle_policy_none_v1()
            )
        )
    ),
    ADD CONSTRAINT media_job_intent_policy_nonempty CHECK (
        intent_policy_profile_id IS NULL
        OR (
            intent_policy_version IS NOT NULL
            AND intent_policy_video_intent IN (
                media_policy_general_v1(),
                media_policy_anime_v1(),
                media_policy_archival_v1()
            )
        )
    );

CREATE TABLE IF NOT EXISTS media_capability_snapshot_run (
    snapshot_run_public_id UUID PRIMARY KEY,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    observed_by_user_id BIGINT NOT NULL REFERENCES app_user(user_id),
    error_code TEXT,
    CONSTRAINT media_capability_snapshot_run_status_known CHECK (
        status IN (
            media_capability_run_status_running_v1(),
            media_capability_run_status_completed_v1(),
            media_capability_run_status_failed_v1()
        )
    ),
    CONSTRAINT media_capability_snapshot_run_error_nonempty CHECK (
        error_code IS NULL OR btrim(error_code) <> ''
    )
);

INSERT INTO media_capability_snapshot_run (
    snapshot_run_public_id,
    status,
    completed_at,
    observed_by_user_id
)
SELECT snapshot_run_public_id,
       media_capability_run_status_completed_v1(),
       max(observed_at),
       min(observed_by_user_id)
  FROM media_capability_snapshot
 WHERE snapshot_run_public_id IS NOT NULL
 GROUP BY snapshot_run_public_id
ON CONFLICT (snapshot_run_public_id) DO NOTHING;

CREATE OR REPLACE FUNCTION media_compatibility_target_list_v1()
RETURNS TABLE (
    compatibility_target_key TEXT,
    version INT,
    display_name TEXT,
    video_codec TEXT,
    audio_codec TEXT,
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
           target.subtitle_policy
      FROM media_compatibility_target AS target
     WHERE target.enabled
     ORDER BY lower(target.compatibility_target_key), target.version DESC;
$$;

CREATE OR REPLACE FUNCTION media_compatibility_target_upsert_v1(
    actor_public_id_input UUID,
    compatibility_target_key_input TEXT,
    version_input INT,
    display_name_input TEXT,
    video_codec_input TEXT,
    audio_codec_input TEXT,
    subtitle_policy_input TEXT
)
RETURNS TABLE (
    compatibility_target_key TEXT,
    version INT,
    display_name TEXT,
    video_codec TEXT,
    audio_codec TEXT,
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
        lower(btrim(subtitle_policy_input)),
        TRUE,
        now()
    )
    ON CONFLICT (lower(compatibility_target_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_codec = EXCLUDED.video_codec,
        audio_codec = EXCLUDED.audio_codec,
        subtitle_policy = EXCLUDED.subtitle_policy,
        enabled = TRUE,
        updated_at = now()
    RETURNING
        media_compatibility_target.compatibility_target_key,
        media_compatibility_target.version,
        media_compatibility_target.display_name,
        media_compatibility_target.video_codec,
        media_compatibility_target.audio_codec,
        media_compatibility_target.subtitle_policy;
END;
$$;

CREATE OR REPLACE FUNCTION media_policy_profile_list_v1()
RETURNS TABLE (
    policy_key TEXT,
    version INT,
    display_name TEXT,
    video_intent TEXT
)
LANGUAGE sql
STABLE
AS $$
    SELECT profile.policy_key,
           profile.version,
           profile.display_name,
           profile.video_intent
      FROM media_policy_profile AS profile
     WHERE profile.enabled
     ORDER BY lower(profile.policy_key), profile.version DESC;
$$;

CREATE OR REPLACE FUNCTION media_policy_profile_upsert_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    display_name_input TEXT,
    video_intent_input TEXT
)
RETURNS TABLE (
    policy_key TEXT,
    version INT,
    display_name TEXT,
    video_intent TEXT
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
    INSERT INTO media_policy_profile (
        policy_key,
        version,
        display_name,
        video_intent,
        enabled,
        updated_at
    )
    VALUES (
        btrim(policy_key_input),
        version_value,
        btrim(display_name_input),
        lower(btrim(video_intent_input)),
        TRUE,
        now()
    )
    ON CONFLICT (lower(policy_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_intent = EXCLUDED.video_intent,
        enabled = TRUE,
        updated_at = now()
    RETURNING
        media_policy_profile.policy_key,
        media_policy_profile.version,
        media_policy_profile.display_name,
        media_policy_profile.video_intent;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_retention_policy_get_v1()
RETURNS TABLE (
    completed_retention_days INT,
    failed_diagnostic_retention_days INT
)
LANGUAGE sql
STABLE
AS $$
    SELECT completed_retention_days,
           failed_diagnostic_retention_days
      FROM media_job_retention_policy
     WHERE lower(policy_key) = media_retention_policy_default_v1()
       AND enabled
     ORDER BY updated_at DESC, media_job_retention_policy_id DESC
     LIMIT 1;
$$;

CREATE OR REPLACE FUNCTION media_job_retention_policy_update_v1(
    actor_public_id_input UUID,
    completed_retention_days_input INT,
    failed_diagnostic_retention_days_input INT
)
RETURNS TABLE (
    completed_retention_days INT,
    failed_diagnostic_retention_days INT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    RETURN QUERY
    INSERT INTO media_job_retention_policy (
        policy_key,
        completed_retention_days,
        failed_diagnostic_retention_days,
        enabled,
        updated_at
    )
    VALUES (
        media_retention_policy_default_v1(),
        completed_retention_days_input,
        failed_diagnostic_retention_days_input,
        TRUE,
        now()
    )
    ON CONFLICT (lower(policy_key)) DO UPDATE SET
        completed_retention_days = EXCLUDED.completed_retention_days,
        failed_diagnostic_retention_days = EXCLUDED.failed_diagnostic_retention_days,
        enabled = TRUE,
        updated_at = now()
    RETURNING
        media_job_retention_policy.completed_retention_days,
        media_job_retention_policy.failed_diagnostic_retention_days;
END;
$$;

CREATE OR REPLACE FUNCTION media_capability_snapshot_run_start_v1(
    actor_public_id_input UUID,
    snapshot_run_public_id_input UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    INSERT INTO media_capability_snapshot_run (
        snapshot_run_public_id,
        status,
        observed_by_user_id
    )
    VALUES (
        snapshot_run_public_id_input,
        media_capability_run_status_running_v1(),
        actor_id
    );
END;
$$;

CREATE OR REPLACE FUNCTION media_capability_snapshot_run_complete_v1(
    snapshot_run_public_id_input UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    UPDATE media_capability_snapshot_run
       SET status = media_capability_run_status_completed_v1(),
           completed_at = now(),
           error_code = NULL
     WHERE snapshot_run_public_id = snapshot_run_public_id_input
       AND status = media_capability_run_status_running_v1();

    IF NOT FOUND THEN
        RAISE EXCEPTION 'capability snapshot run not running'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_capability_snapshot_run_not_running';
    END IF;
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
        SELECT run.snapshot_run_public_id
          FROM media_capability_snapshot_run run
         WHERE run.status = media_capability_run_status_completed_v1()
           AND run.completed_at IS NOT NULL
         ORDER BY run.completed_at DESC, run.started_at DESC
         LIMIT 1
    )
    SELECT mcs.media_capability_snapshot_id,
           mcs.snapshot_run_public_id,
           mcs.ffmpeg_version,
           mcs.ffprobe_version,
           mcs.codec_name,
           mcs.encode_supported,
           mcs.decode_supported,
           mcs.observed_at
      FROM media_capability_snapshot mcs
      JOIN latest_run lr ON lr.snapshot_run_public_id = mcs.snapshot_run_public_id
     ORDER BY lower(mcs.codec_name) ASC, mcs.media_capability_snapshot_id ASC;
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
     WHERE lower(policy_key) = lower(COALESCE(profile_row.policy_key, media_policy_safe_dry_run_v1()))
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
           updated.intent_target_subtitle_policy,
           updated.intent_policy_video_intent
      FROM updated
      JOIN media_profile mp ON mp.media_profile_id = updated.media_profile_id;
END;
$$;

DROP FUNCTION IF EXISTS revaer_config.factory_reset_without_media_defaults_v1();
ALTER FUNCTION revaer_config.factory_reset() RENAME TO factory_reset_without_media_defaults_v1;

CREATE OR REPLACE FUNCTION revaer_config.factory_reset()
RETURNS VOID AS
$$
BEGIN
    PERFORM revaer_config.factory_reset_without_media_defaults_v1();
    PERFORM revaer_config.seed_media_configuration_defaults();
END;
$$ LANGUAGE plpgsql;
