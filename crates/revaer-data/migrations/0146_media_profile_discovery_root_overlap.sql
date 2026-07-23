CREATE OR REPLACE FUNCTION media_profile_normalized_root_v1(root_input TEXT)
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT COALESCE(NULLIF(lower(regexp_replace(btrim(root_input), '/+$', '')), ''), '/')
$$;

CREATE OR REPLACE FUNCTION media_profile_validate_discovery_root_overlap_v1(
    media_profile_id_input BIGINT,
    source_root_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    source_root_compare_value TEXT;
BEGIN
    source_root_compare_value := media_profile_normalized_root_v1(source_root_input);

    IF EXISTS (
        SELECT 1
          FROM media_profile AS existing
         WHERE existing.deleted_at IS NULL
           AND (
               media_profile_id_input IS NULL
               OR existing.media_profile_id <> media_profile_id_input
           )
           AND (
               source_root_compare_value = media_profile_normalized_root_v1(existing.source_root)
               OR source_root_compare_value LIKE media_profile_normalized_root_v1(existing.source_root) || '/%'
               OR media_profile_normalized_root_v1(existing.source_root) LIKE source_root_compare_value || '/%'
           )
    ) THEN
        RAISE EXCEPTION 'profile discovery roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_discovery_root_overlap';
    END IF;
END;
$$;

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
    profile_id BIGINT;
    profile_public_id_out UUID;
    source_root_compare_value TEXT;
    output_root_compare_value TEXT;
    compatibility_target_key_value TEXT;
    policy_key_value TEXT;
    schedule_enabled_value BOOLEAN;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    LOCK TABLE media_profile IN SHARE ROW EXCLUSIVE MODE;

    SELECT media_profile_id
      INTO profile_id
      FROM media_profile
     WHERE lower(profile_key) = lower(btrim(profile_key_input))
       AND deleted_at IS NULL;

    compatibility_target_key_value := NULLIF(btrim(compatibility_target_key_input), '');
    policy_key_value := COALESCE(NULLIF(btrim(policy_key_input), ''), 'safe_dry_run');
    schedule_enabled_value := COALESCE(schedule_enabled_input, FALSE);
    source_root_compare_value := media_profile_normalized_root_v1(source_root_input);
    output_root_compare_value := media_profile_normalized_root_v1(output_root_input);

    PERFORM media_profile_validate_catalog_refs_v1(
        compatibility_target_key_value,
        policy_key_value
    );

    IF schedule_enabled_value AND schedule_interval_minutes_input IS NULL THEN
        RAISE EXCEPTION 'schedule interval required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_schedule_interval_required';
    END IF;

    IF source_root_compare_value = output_root_compare_value
       OR source_root_compare_value = '/'
       OR output_root_compare_value = '/'
       OR source_root_compare_value LIKE output_root_compare_value || '/%'
       OR output_root_compare_value LIKE source_root_compare_value || '/%' THEN
        RAISE EXCEPTION 'profile roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_roots_overlap';
    END IF;

    PERFORM media_profile_validate_discovery_root_overlap_v1(
        profile_id,
        source_root_compare_value
    );

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
    source_root_compare_value TEXT;
    output_root_compare_value TEXT;
    compatibility_target_key_value TEXT;
    policy_key_value TEXT;
    schedule_enabled_value BOOLEAN;
    schedule_interval_minutes_value INT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    LOCK TABLE media_profile IN SHARE ROW EXCLUSIVE MODE;

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
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    PERFORM media_profile_validate_catalog_refs_v1(
        compatibility_target_key_value,
        policy_key_value
    );
    source_root_compare_value := media_profile_normalized_root_v1(source_root_value);
    output_root_compare_value := media_profile_normalized_root_v1(output_root_value);

    IF schedule_enabled_value AND schedule_interval_minutes_value IS NULL THEN
        RAISE EXCEPTION 'schedule interval required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_schedule_interval_required';
    END IF;

    IF source_root_compare_value = output_root_compare_value
       OR source_root_compare_value = '/'
       OR output_root_compare_value = '/'
       OR source_root_compare_value LIKE output_root_compare_value || '/%'
       OR output_root_compare_value LIKE source_root_compare_value || '/%' THEN
        RAISE EXCEPTION 'profile roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_roots_overlap';
    END IF;

    PERFORM media_profile_validate_discovery_root_overlap_v1(
        profile_id,
        source_root_compare_value
    );

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
