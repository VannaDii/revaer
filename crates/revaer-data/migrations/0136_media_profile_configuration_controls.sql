ALTER TABLE media_profile
    ADD COLUMN compatibility_target_key TEXT,
    ADD COLUMN policy_key TEXT NOT NULL DEFAULT 'safe_dry_run',
    ADD COLUMN watcher_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN schedule_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN schedule_interval_minutes INT;

ALTER TABLE media_profile
    ADD CONSTRAINT media_profile_compatibility_target_nonempty CHECK (
        compatibility_target_key IS NULL OR btrim(compatibility_target_key) <> ''
    ),
    ADD CONSTRAINT media_profile_policy_key_nonempty CHECK (btrim(policy_key) <> ''),
    ADD CONSTRAINT media_profile_schedule_interval_bounds CHECK (
        schedule_interval_minutes IS NULL OR schedule_interval_minutes BETWEEN 1 AND 525600
    ),
    ADD CONSTRAINT media_profile_schedule_requires_interval CHECK (
        schedule_enabled = FALSE OR schedule_interval_minutes IS NOT NULL
    );

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
        COALESCE(dry_run_only_input, TRUE),
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
        dry_run_only = EXCLUDED.dry_run_only,
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

CREATE OR REPLACE FUNCTION media_profile_update_v1(
    actor_public_id_input UUID,
    media_profile_public_id_input UUID,
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
    profile_id BIGINT;
    source_root_value TEXT;
    output_root_value TEXT;
    compatibility_target_key_value TEXT;
    policy_key_value TEXT;
    schedule_enabled_value BOOLEAN;
    schedule_interval_minutes_value INT;
BEGIN
    SELECT user_id
      INTO actor_id
      FROM app_user
     WHERE user_public_id = actor_public_id_input;

    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'actor not found'
            USING ERRCODE = 'P0001', DETAIL = 'app_user_not_found';
    END IF;

    SELECT
        media_profile_id,
        COALESCE(NULLIF(btrim(source_root_input), ''), source_root),
        COALESCE(NULLIF(btrim(output_root_input), ''), output_root),
        CASE
            WHEN compatibility_target_key_input IS NULL THEN compatibility_target_key
            ELSE NULLIF(btrim(compatibility_target_key_input), '')
        END,
        COALESCE(NULLIF(btrim(policy_key_input), ''), policy_key),
        COALESCE(schedule_enabled_input, schedule_enabled),
        CASE
            WHEN schedule_interval_minutes_input IS NULL THEN schedule_interval_minutes
            ELSE schedule_interval_minutes_input
        END
      INTO
        profile_id,
        source_root_value,
        output_root_value,
        compatibility_target_key_value,
        policy_key_value,
        schedule_enabled_value,
        schedule_interval_minutes_value
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_profile_not_found';
    END IF;

    IF schedule_enabled_value AND schedule_interval_minutes_value IS NULL THEN
        RAISE EXCEPTION 'schedule interval required'
            USING ERRCODE = 'P0001', DETAIL = 'media_profile_schedule_interval_required';
    END IF;

    IF lower(source_root_value) = lower(output_root_value)
       OR lower(source_root_value) LIKE lower(output_root_value) || '/%'
       OR lower(output_root_value) LIKE lower(source_root_value) || '/%' THEN
        RAISE EXCEPTION 'profile roots overlap'
            USING ERRCODE = 'P0001', DETAIL = 'media_profile_roots_overlap';
    END IF;

    UPDATE media_profile
       SET source_root = source_root_value,
           output_root = output_root_value,
           dry_run_only = COALESCE(dry_run_only_input, dry_run_only),
           retention_days = COALESCE(retention_days_input, retention_days),
           compatibility_target_key = compatibility_target_key_value,
           policy_key = policy_key_value,
           watcher_enabled = COALESCE(watcher_enabled_input, watcher_enabled),
           schedule_enabled = schedule_enabled_value,
           schedule_interval_minutes = schedule_interval_minutes_value,
           updated_at = now()
     WHERE media_profile_id = profile_id;

    RETURN media_profile_public_id_input;
END;
$$;

CREATE OR REPLACE FUNCTION media_profile_list_v2()
RETURNS TABLE (
    media_profile_public_id UUID,
    profile_key TEXT,
    source_root TEXT,
    output_root TEXT,
    dry_run_only BOOLEAN,
    retention_days INT,
    compatibility_target_key TEXT,
    policy_key TEXT,
    watcher_enabled BOOLEAN,
    schedule_enabled BOOLEAN,
    schedule_interval_minutes INT,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        mp.media_profile_public_id,
        mp.profile_key,
        mp.source_root,
        mp.output_root,
        mp.dry_run_only,
        mp.retention_days,
        mp.compatibility_target_key,
        mp.policy_key,
        mp.watcher_enabled,
        mp.schedule_enabled,
        mp.schedule_interval_minutes,
        mp.updated_at
    FROM media_profile mp
    WHERE mp.deleted_at IS NULL
    ORDER BY lower(mp.profile_key);
$$;

CREATE OR REPLACE FUNCTION media_profile_get_v2(media_profile_public_id_input UUID)
RETURNS TABLE (
    media_profile_public_id UUID,
    profile_key TEXT,
    source_root TEXT,
    output_root TEXT,
    dry_run_only BOOLEAN,
    retention_days INT,
    compatibility_target_key TEXT,
    policy_key TEXT,
    watcher_enabled BOOLEAN,
    schedule_enabled BOOLEAN,
    schedule_interval_minutes INT,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        mp.media_profile_public_id,
        mp.profile_key,
        mp.source_root,
        mp.output_root,
        mp.dry_run_only,
        mp.retention_days,
        mp.compatibility_target_key,
        mp.policy_key,
        mp.watcher_enabled,
        mp.schedule_enabled,
        mp.schedule_interval_minutes,
        mp.updated_at
    FROM media_profile mp
    WHERE mp.media_profile_public_id = media_profile_public_id_input
      AND mp.deleted_at IS NULL;
$$;
