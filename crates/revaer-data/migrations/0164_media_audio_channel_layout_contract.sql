CREATE OR REPLACE FUNCTION media_audio_channel_layout_count_v1(
    channel_layout_input TEXT
)
RETURNS INT
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT CASE lower(btrim(channel_layout_input))
        WHEN 'mono' THEN 1
        WHEN '1c' THEN 1
        WHEN 'stereo' THEN 2
        WHEN '2c' THEN 2
        WHEN '2.1' THEN 3
        WHEN '3.0' THEN 3
        WHEN '3.0(back)' THEN 3
        WHEN '4.0' THEN 4
        WHEN 'quad' THEN 4
        WHEN 'quad(side)' THEN 4
        WHEN '3.1' THEN 4
        WHEN '5.0' THEN 5
        WHEN '5.0(side)' THEN 5
        WHEN '4.1' THEN 5
        WHEN '5.1' THEN 6
        WHEN '5.1(side)' THEN 6
        WHEN '6.1' THEN 7
        WHEN '6.1(back)' THEN 7
        WHEN '7.1' THEN 8
        WHEN '7.1(wide)' THEN 8
        WHEN '7.1(wide-side)' THEN 8
        ELSE NULL
    END
$$;

ALTER TABLE media_compatibility_target
    ADD CONSTRAINT media_compatibility_target_audio_layout_known CHECK (
        audio_channel_layout IS NULL
        OR media_audio_channel_layout_count_v1(audio_channel_layout) IS NOT NULL
    ),
    ADD CONSTRAINT media_compatibility_target_audio_layout_count_matches CHECK (
        audio_channel_layout IS NULL
        OR audio_channels IS NULL
        OR media_audio_channel_layout_count_v1(audio_channel_layout) = audio_channels
    );

ALTER TABLE media_job
    ADD CONSTRAINT media_job_intent_target_audio_layout_known CHECK (
        intent_target_audio_channel_layout IS NULL
        OR media_audio_channel_layout_count_v1(intent_target_audio_channel_layout) IS NOT NULL
    ),
    ADD CONSTRAINT media_job_intent_target_audio_layout_count_matches CHECK (
        intent_target_audio_channel_layout IS NULL
        OR intent_target_audio_channels IS NULL
        OR media_audio_channel_layout_count_v1(intent_target_audio_channel_layout)
            = intent_target_audio_channels
    );

ALTER TABLE media_desired_target_audio_stream
    ADD CONSTRAINT media_desired_target_audio_layout_known CHECK (
        channel_layout IS NULL
        OR media_audio_channel_layout_count_v1(channel_layout) IS NOT NULL
    ),
    ADD CONSTRAINT media_desired_target_audio_layout_count_matches CHECK (
        channel_layout IS NULL
        OR channel_count IS NULL
        OR media_audio_channel_layout_count_v1(channel_layout) = channel_count
    );

ALTER TABLE media_job_desired_target_stream
    ADD CONSTRAINT media_job_desired_target_stream_audio_layout_known CHECK (
        channel_layout IS NULL
        OR media_audio_channel_layout_count_v1(channel_layout) IS NOT NULL
    ),
    ADD CONSTRAINT media_job_desired_target_stream_audio_layout_count_matches CHECK (
        channel_layout IS NULL
        OR channel_count IS NULL
        OR media_audio_channel_layout_count_v1(channel_layout) = channel_count
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
    audio_channel_layout_value TEXT;
    audio_layout_channel_count INT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    version_value := COALESCE(version_input, 1);
    audio_channel_layout_value := NULLIF(lower(btrim(audio_channel_layout_input)), '');
    audio_layout_channel_count := media_audio_channel_layout_count_v1(audio_channel_layout_value);

    IF audio_channel_layout_value IS NOT NULL AND audio_layout_channel_count IS NULL THEN
        RAISE EXCEPTION 'audio channel layout is not supported'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_audio_shape_invalid';
    END IF;
    IF audio_channels_input IS NOT NULL
        AND audio_layout_channel_count IS NOT NULL
        AND audio_channels_input <> audio_layout_channel_count THEN
        RAISE EXCEPTION 'audio channel count does not match layout'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_audio_shape_invalid';
    END IF;

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
        audio_channel_layout_value,
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

CREATE OR REPLACE FUNCTION media_desired_target_stream_append_v5(
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
    audio_bitrate_bps_input INT,
    audio_sample_rate_hz_input INT,
    audio_loudness_profile_input TEXT,
    audio_dynamic_range_input TEXT,
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
    codec_value TEXT;
    semantic_role_value TEXT;
    subtitle_placement_value TEXT;
    image_subtitle_action_value TEXT;
    channel_layout_value TEXT;
    audio_layout_channel_count INT;
    audio_loudness_profile_value TEXT;
    audio_dynamic_range_value TEXT;
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
    codec_value := lower(btrim(codec_input));
    semantic_role_value := NULLIF(lower(btrim(semantic_role_input)), '');
    subtitle_placement_value := NULLIF(lower(btrim(subtitle_placement_input)), '');
    image_subtitle_action_value := NULLIF(lower(btrim(image_subtitle_action_input)), '');
    channel_layout_value := NULLIF(lower(btrim(channel_layout_input)), '');
    audio_layout_channel_count := media_audio_channel_layout_count_v1(channel_layout_value);
    audio_loudness_profile_value := NULLIF(lower(btrim(audio_loudness_profile_input)), '');
    audio_dynamic_range_value := NULLIF(lower(btrim(audio_dynamic_range_input)), '');
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
        IF hdr_format_value IS NOT NULL AND hdr_format_value NOT IN ('hdr10') THEN
            RAISE EXCEPTION 'HDR format is not supported'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF NOT media_video_level_known_v1(codec_value, video_level_value) THEN
            RAISE EXCEPTION 'video level is not supported for codec'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF NOT media_video_color_value_known_v1('color_primaries', color_primaries_value)
            OR NOT media_video_color_value_known_v1('color_transfer', color_transfer_value)
            OR NOT media_video_color_value_known_v1('color_space', color_space_value) THEN
            RAISE EXCEPTION 'video color value is not supported'
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

    IF stream_kind_value = 'audio' THEN
        IF channel_count_input IS NOT NULL AND channel_count_input <= 0 THEN
            RAISE EXCEPTION 'audio channel count must be positive'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF channel_layout_value IS NOT NULL AND audio_layout_channel_count IS NULL THEN
            RAISE EXCEPTION 'audio channel layout is not supported'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF channel_count_input IS NOT NULL
            AND audio_layout_channel_count IS NOT NULL
            AND channel_count_input <> audio_layout_channel_count THEN
            RAISE EXCEPTION 'audio channel count does not match layout'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF audio_bitrate_bps_input IS NOT NULL AND audio_bitrate_bps_input <= 0 THEN
            RAISE EXCEPTION 'audio bitrate must be positive'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF audio_sample_rate_hz_input IS NOT NULL AND audio_sample_rate_hz_input <= 0 THEN
            RAISE EXCEPTION 'audio sample rate must be positive'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF audio_loudness_profile_value IS NOT NULL
            AND audio_loudness_profile_value NOT IN ('dialog-normalized') THEN
            RAISE EXCEPTION 'audio loudness profile is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF audio_dynamic_range_value IS NOT NULL
            AND audio_dynamic_range_value NOT IN ('preserve', 'speech') THEN
            RAISE EXCEPTION 'audio dynamic range is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
    ELSIF channel_count_input IS NOT NULL
        OR channel_layout_value IS NOT NULL
        OR audio_bitrate_bps_input IS NOT NULL
        OR audio_sample_rate_hz_input IS NOT NULL
        OR audio_loudness_profile_value IS NOT NULL
        OR audio_dynamic_range_value IS NOT NULL THEN
        RAISE EXCEPTION 'audio shape assigned to non-audio target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
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
        codec_value,
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
            channel_layout,
            audio_bitrate_bps,
            audio_sample_rate_hz,
            audio_loudness_profile,
            audio_dynamic_range
        )
        VALUES (
            target_stream_id,
            channel_count_input,
            channel_layout_value,
            audio_bitrate_bps_input,
            audio_sample_rate_hz_input,
            audio_loudness_profile_value,
            audio_dynamic_range_value
        );
    END IF;
END;
$$;
