CREATE OR REPLACE FUNCTION media_job_normalized_absolute_path_v1(path_input TEXT)
RETURNS TEXT
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    trimmed_value TEXT;
    component_value TEXT;
    normalized_value TEXT := '';
BEGIN
    trimmed_value := btrim(path_input);

    IF trimmed_value IS NULL
       OR trimmed_value = ''
       OR left(trimmed_value, 1) <> '/' THEN
        RETURN NULL;
    END IF;

    FOREACH component_value IN ARRAY regexp_split_to_array(trimmed_value, '/+')
    LOOP
        IF component_value = ''
           OR component_value = '.' THEN
            CONTINUE;
        END IF;

        IF component_value = '..' THEN
            RETURN NULL;
        END IF;

        normalized_value := normalized_value || '/' || component_value;
    END LOOP;

    IF normalized_value = '' THEN
        RETURN '/';
    END IF;

    RETURN normalized_value;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_validate_path_within_root_v1(
    path_input TEXT,
    root_input TEXT,
    detail_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    normalized_path_value TEXT;
    normalized_root_value TEXT;
BEGIN
    normalized_path_value := media_job_normalized_absolute_path_v1(path_input);
    normalized_root_value := media_job_normalized_absolute_path_v1(root_input);

    IF normalized_path_value IS NULL
       OR normalized_root_value IS NULL
       OR NOT (
           normalized_path_value = normalized_root_value
           OR normalized_root_value = '/'
           OR left(normalized_path_value, length(normalized_root_value) + 1) = normalized_root_value || '/'
       ) THEN
        RAISE EXCEPTION 'media job path outside profile root'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = detail_input;
    END IF;
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
    output_path_value TEXT;
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

    output_path_value := NULLIF(btrim(output_path_input), '');

    PERFORM media_job_validate_path_within_root_v1(
        source_path_input,
        profile_row.source_root,
        'media_job_source_path_outside_profile_root'
    );

    IF output_path_value IS NOT NULL THEN
        PERFORM media_job_validate_path_within_root_v1(
            output_path_value,
            profile_row.output_root,
            'media_job_output_path_outside_profile_root'
        );
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
        output_path_value,
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
