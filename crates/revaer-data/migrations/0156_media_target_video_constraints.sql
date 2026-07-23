ALTER TABLE media_desired_target_stream
    ADD COLUMN video_profile TEXT,
    ADD COLUMN video_level TEXT,
    ADD COLUMN video_bitrate_bps INT,
    ADD COLUMN color_primaries TEXT,
    ADD COLUMN color_transfer TEXT,
    ADD COLUMN color_space TEXT,
    ADD COLUMN hdr_format TEXT,
    ADD CONSTRAINT media_desired_target_stream_video_shape CHECK (
        (
            stream_kind = 'video'
            AND (video_bitrate_bps IS NULL OR video_bitrate_bps > 0)
        )
        OR
        (
            stream_kind <> 'video'
            AND video_profile IS NULL
            AND video_level IS NULL
            AND video_bitrate_bps IS NULL
            AND color_primaries IS NULL
            AND color_transfer IS NULL
            AND color_space IS NULL
            AND hdr_format IS NULL
        )
    );

ALTER TABLE media_job_desired_target_stream
    ADD COLUMN video_profile TEXT,
    ADD COLUMN video_level TEXT,
    ADD COLUMN video_bitrate_bps INT,
    ADD COLUMN color_primaries TEXT,
    ADD COLUMN color_transfer TEXT,
    ADD COLUMN color_space TEXT,
    ADD COLUMN hdr_format TEXT,
    ADD CONSTRAINT media_job_desired_target_stream_video_shape CHECK (
        (
            stream_kind = 'video'
            AND (video_bitrate_bps IS NULL OR video_bitrate_bps > 0)
        )
        OR
        (
            stream_kind <> 'video'
            AND video_profile IS NULL
            AND video_level IS NULL
            AND video_bitrate_bps IS NULL
            AND color_primaries IS NULL
            AND color_transfer IS NULL
            AND color_space IS NULL
            AND hdr_format IS NULL
        )
    );

CREATE FUNCTION media_desired_target_stream_append_v3(
    media_desired_target_profile_public_id_input UUID,
    stream_key_input TEXT,
    stream_kind_input TEXT,
    semantic_role_input TEXT,
    language_code_input TEXT,
    optional_input BOOLEAN,
    sort_order_input INT,
    codec_input TEXT,
    channel_count_input INT,
    channel_layout_input TEXT,
    video_profile_input TEXT,
    video_level_input TEXT,
    video_bitrate_bps_input INT,
    color_primaries_input TEXT,
    color_transfer_input TEXT,
    color_space_input TEXT,
    hdr_format_input TEXT,
    title_input TEXT,
    default_disposition_input BOOLEAN,
    forced_disposition_input BOOLEAN,
    subtitle_placement_input TEXT,
    image_subtitle_action_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    target_id BIGINT;
    target_stream_id BIGINT;
    stream_kind_value TEXT;
    semantic_role_value TEXT;
    subtitle_placement_value TEXT;
    image_subtitle_action_value TEXT;
    video_profile_value TEXT;
    video_level_value TEXT;
    color_primaries_value TEXT;
    color_transfer_value TEXT;
    color_space_value TEXT;
    hdr_format_value TEXT;
BEGIN
    SELECT media_desired_target_profile_id
      INTO target_id
      FROM media_desired_target_profile
     WHERE media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND enabled
     FOR UPDATE;

    IF target_id IS NULL THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    IF EXISTS (
        SELECT 1 FROM media_profile WHERE desired_target_profile_id = target_id
        UNION ALL
        SELECT 1 FROM media_job WHERE intent_desired_target_profile_id = target_id
    ) THEN
        RAISE EXCEPTION 'desired target version is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_immutable';
    END IF;

    stream_kind_value := lower(btrim(stream_kind_input));
    semantic_role_value := NULLIF(lower(btrim(semantic_role_input)), '');
    subtitle_placement_value := NULLIF(lower(btrim(subtitle_placement_input)), '');
    image_subtitle_action_value := NULLIF(lower(btrim(image_subtitle_action_input)), '');
    video_profile_value := NULLIF(lower(btrim(video_profile_input)), '');
    video_level_value := NULLIF(lower(btrim(video_level_input)), '');
    color_primaries_value := NULLIF(lower(btrim(color_primaries_input)), '');
    color_transfer_value := NULLIF(lower(btrim(color_transfer_input)), '');
    color_space_value := NULLIF(lower(btrim(color_space_input)), '');
    hdr_format_value := NULLIF(lower(btrim(hdr_format_input)), '');

    IF stream_kind_value = 'subtitle' THEN
        IF subtitle_placement_value IS NULL
            OR subtitle_placement_value NOT IN ('embedded', 'sidecar', 'both', 'none')
            OR image_subtitle_action_value IS NULL
            OR image_subtitle_action_value NOT IN ('preserve', 'remove', 'fail') THEN
            RAISE EXCEPTION 'subtitle target shape is incomplete'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_subtitle_shape_invalid';
        END IF;
        IF semantic_role_value = 'descriptive_audio' THEN
            RAISE EXCEPTION 'subtitle target semantic role is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_subtitle_shape_invalid';
        END IF;
    ELSIF subtitle_placement_value IS NOT NULL OR image_subtitle_action_value IS NOT NULL THEN
        RAISE EXCEPTION 'subtitle shape assigned to non-subtitle target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_subtitle_shape_invalid';
    END IF;

    IF stream_kind_value = 'video' THEN
        IF video_bitrate_bps_input IS NOT NULL AND video_bitrate_bps_input <= 0 THEN
            RAISE EXCEPTION 'video bitrate must be positive'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
    ELSIF video_profile_value IS NOT NULL
        OR video_level_value IS NOT NULL
        OR video_bitrate_bps_input IS NOT NULL
        OR color_primaries_value IS NOT NULL
        OR color_transfer_value IS NOT NULL
        OR color_space_value IS NOT NULL
        OR hdr_format_value IS NOT NULL THEN
        RAISE EXCEPTION 'video shape assigned to non-video target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
    END IF;

    INSERT INTO media_desired_target_stream (
        media_desired_target_profile_id,
        stream_key,
        stream_kind,
        semantic_role,
        language_code,
        optional,
        sort_order,
        codec,
        title,
        default_disposition,
        forced_disposition,
        subtitle_placement,
        image_subtitle_action,
        video_profile,
        video_level,
        video_bitrate_bps,
        color_primaries,
        color_transfer,
        color_space,
        hdr_format
    )
    VALUES (
        target_id,
        btrim(stream_key_input),
        stream_kind_value,
        semantic_role_value,
        NULLIF(lower(btrim(language_code_input)), ''),
        COALESCE(optional_input, FALSE),
        sort_order_input,
        lower(btrim(codec_input)),
        NULLIF(btrim(title_input), ''),
        COALESCE(default_disposition_input, FALSE),
        COALESCE(forced_disposition_input, FALSE),
        subtitle_placement_value,
        image_subtitle_action_value,
        video_profile_value,
        video_level_value,
        video_bitrate_bps_input,
        color_primaries_value,
        color_transfer_value,
        color_space_value,
        hdr_format_value
    )
    RETURNING media_desired_target_stream_id INTO target_stream_id;

    IF stream_kind_value = 'audio' THEN
        INSERT INTO media_desired_target_audio_stream (
            media_desired_target_stream_id,
            channel_count,
            channel_layout
        )
        VALUES (
            target_stream_id,
            channel_count_input,
            NULLIF(lower(btrim(channel_layout_input)), '')
        );
    ELSIF channel_count_input IS NOT NULL OR NULLIF(btrim(channel_layout_input), '') IS NOT NULL THEN
        RAISE EXCEPTION 'audio shape assigned to non-audio target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION media_desired_target_stream_append_v2(
    media_desired_target_profile_public_id_input UUID,
    stream_key_input TEXT,
    stream_kind_input TEXT,
    semantic_role_input TEXT,
    language_code_input TEXT,
    optional_input BOOLEAN,
    sort_order_input INT,
    codec_input TEXT,
    channel_count_input INT,
    channel_layout_input TEXT,
    title_input TEXT,
    default_disposition_input BOOLEAN,
    forced_disposition_input BOOLEAN,
    subtitle_placement_input TEXT,
    image_subtitle_action_input TEXT
)
RETURNS VOID
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT media_desired_target_stream_append_v3(
        media_desired_target_profile_public_id_input,
        stream_key_input,
        stream_kind_input,
        semantic_role_input,
        language_code_input,
        optional_input,
        sort_order_input,
        codec_input,
        channel_count_input,
        channel_layout_input,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        title_input,
        default_disposition_input,
        forced_disposition_input,
        subtitle_placement_input,
        image_subtitle_action_input
    );
$$;

CREATE FUNCTION media_desired_target_stream_list_v3(
    media_desired_target_profile_public_id_input UUID
)
RETURNS TABLE (
    stream_key TEXT,
    stream_kind TEXT,
    semantic_role TEXT,
    language_code TEXT,
    optional BOOLEAN,
    sort_order INT,
    codec TEXT,
    channel_count INT,
    channel_layout TEXT,
    video_profile TEXT,
    video_level TEXT,
    video_bitrate_bps INT,
    color_primaries TEXT,
    color_transfer TEXT,
    color_space TEXT,
    hdr_format TEXT,
    title TEXT,
    default_disposition BOOLEAN,
    forced_disposition BOOLEAN,
    subtitle_placement TEXT,
    image_subtitle_action TEXT
)
LANGUAGE sql
STABLE
AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           audio.channel_count,
           audio.channel_layout,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_desired_target_profile target
      JOIN media_desired_target_stream stream
        ON stream.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_audio_stream audio
        ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY stream.sort_order;
$$;

CREATE FUNCTION media_job_desired_target_stream_list_v3(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    stream_key TEXT,
    stream_kind TEXT,
    semantic_role TEXT,
    language_code TEXT,
    optional BOOLEAN,
    sort_order INT,
    codec TEXT,
    channel_count INT,
    channel_layout TEXT,
    video_profile TEXT,
    video_level TEXT,
    video_bitrate_bps INT,
    color_primaries TEXT,
    color_transfer TEXT,
    color_space TEXT,
    hdr_format TEXT,
    title TEXT,
    default_disposition BOOLEAN,
    forced_disposition BOOLEAN,
    subtitle_placement TEXT,
    image_subtitle_action TEXT
)
LANGUAGE sql
STABLE
AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           stream.channel_count,
           stream.channel_layout,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_job job
      JOIN media_job_desired_target_stream stream ON stream.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY stream.sort_order;
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
    compatibility_row media_compatibility_target%ROWTYPE;
    desired_target_row media_desired_target_profile%ROWTYPE;
    desired_container_format TEXT;
    policy_row media_policy_profile%ROWTYPE;
    output_path_value TEXT;
    media_job_id_out BIGINT;
    media_job_public_id_out UUID;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    SELECT * INTO profile_row
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
        SELECT * INTO compatibility_row
          FROM media_compatibility_target
         WHERE lower(compatibility_target_key) = lower(replace(profile_row.compatibility_target_key, '_', '-'))
           AND enabled
         ORDER BY version DESC, media_compatibility_target_id DESC
         LIMIT 1;
        IF compatibility_row.media_compatibility_target_id IS NULL THEN
            RAISE EXCEPTION 'compatibility target not found'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_not_found';
        END IF;
    END IF;

    IF profile_row.desired_target_profile_id IS NOT NULL THEN
        SELECT target.*
          INTO desired_target_row
          FROM media_desired_target_profile target
         WHERE target.media_desired_target_profile_id = profile_row.desired_target_profile_id
           AND target.enabled;
        IF desired_target_row.media_desired_target_profile_id IS NULL THEN
            RAISE EXCEPTION 'desired target not found'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
        END IF;
        SELECT container.container_format
          INTO desired_container_format
          FROM media_desired_target_container container
         WHERE container.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id;
    END IF;

    SELECT * INTO policy_row
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
        media_profile_id, source_path, output_path, dry_run,
        intent_source_root, intent_output_root,
        intent_compatibility_target_key, intent_policy_key,
        intent_compatibility_target_id, intent_compatibility_target_version,
        intent_target_video_codec, intent_target_audio_codec,
        intent_target_audio_channels, intent_target_audio_channel_layout,
        intent_target_subtitle_policy,
        intent_policy_profile_id, intent_policy_version, intent_policy_video_intent,
        intent_desired_target_profile_id, intent_desired_target_key,
        intent_desired_target_version, intent_desired_container_format,
        intent_unmatched_stream_policy,
        intent_verification_strictness,
        intent_verification_duration_tolerance_millis,
        intent_verification_mux_validation,
        intent_verification_decode_all_streams,
        intent_verification_keyframe_seek,
        intent_verification_playback_probe,
        created_by_user_id
    )
    VALUES (
        profile_row.media_profile_id, btrim(source_path_input), output_path_value,
        COALESCE(dry_run_input, TRUE), profile_row.source_root, profile_row.output_root,
        profile_row.compatibility_target_key, profile_row.policy_key,
        compatibility_row.media_compatibility_target_id, compatibility_row.version,
        compatibility_row.video_codec, compatibility_row.audio_codec,
        compatibility_row.audio_channels, compatibility_row.audio_channel_layout,
        compatibility_row.subtitle_policy,
        policy_row.media_policy_profile_id, policy_row.version, policy_row.video_intent,
        desired_target_row.media_desired_target_profile_id, desired_target_row.target_key,
        desired_target_row.version, desired_container_format,
        policy_row.unmatched_stream_policy,
        policy_row.verification_strictness,
        policy_row.verification_duration_tolerance_millis,
        policy_row.verification_mux_validation,
        policy_row.verification_decode_all_streams,
        policy_row.verification_keyframe_seek,
        policy_row.verification_playback_probe,
        actor_id
    )
    RETURNING media_job_id, media_job_public_id
    INTO media_job_id_out, media_job_public_id_out;

    IF desired_target_row.media_desired_target_profile_id IS NOT NULL THEN
        INSERT INTO media_job_desired_target_stream (
            media_job_id, stream_key, stream_kind, semantic_role, language_code,
            optional, sort_order, codec, channel_count, channel_layout,
            video_profile, video_level, video_bitrate_bps, color_primaries,
            color_transfer, color_space, hdr_format, title, default_disposition,
            forced_disposition, subtitle_placement, image_subtitle_action
        )
        SELECT media_job_id_out, stream.stream_key, stream.stream_kind,
               stream.semantic_role, stream.language_code, stream.optional,
               stream.sort_order, stream.codec, audio.channel_count,
               audio.channel_layout, stream.video_profile, stream.video_level,
               stream.video_bitrate_bps, stream.color_primaries,
               stream.color_transfer, stream.color_space, stream.hdr_format,
               stream.title, stream.default_disposition,
               stream.forced_disposition, stream.subtitle_placement,
               stream.image_subtitle_action
          FROM media_desired_target_stream stream
          LEFT JOIN media_desired_target_audio_stream audio
            ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
         WHERE stream.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
         ORDER BY stream.sort_order;
    END IF;

    RETURN media_job_public_id_out;
END;
$$;
