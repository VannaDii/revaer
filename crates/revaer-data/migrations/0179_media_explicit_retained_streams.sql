ALTER TABLE media_desired_target_stream
    DROP CONSTRAINT media_desired_target_stream_kind_known,
    ADD CONSTRAINT media_desired_target_stream_kind_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle', 'attachment', 'data')
    ),
    ADD CONSTRAINT media_desired_target_stream_retained_exact_passthrough CHECK (
        stream_kind NOT IN ('attachment', 'data')
        OR (
            title IS NULL
            AND NOT default_disposition
            AND NOT forced_disposition
            AND subtitle_placement IS NULL
            AND image_subtitle_action IS NULL
            AND video_profile IS NULL
            AND video_level IS NULL
            AND video_bitrate_bps IS NULL
            AND color_primaries IS NULL
            AND color_transfer IS NULL
            AND color_space IS NULL
            AND hdr_format IS NULL
            AND hdr10_mastering_red_x IS NULL
            AND hdr10_mastering_red_y IS NULL
            AND hdr10_mastering_green_x IS NULL
            AND hdr10_mastering_green_y IS NULL
            AND hdr10_mastering_blue_x IS NULL
            AND hdr10_mastering_blue_y IS NULL
            AND hdr10_mastering_white_x IS NULL
            AND hdr10_mastering_white_y IS NULL
            AND hdr10_mastering_min_luminance IS NULL
            AND hdr10_mastering_max_luminance IS NULL
            AND hdr10_max_content_light_level IS NULL
            AND hdr10_max_frame_average_light_level IS NULL
        )
    );

ALTER TABLE media_job_desired_target_stream
    DROP CONSTRAINT media_job_desired_target_stream_kind_known,
    ADD CONSTRAINT media_job_desired_target_stream_kind_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle', 'attachment', 'data')
    ),
    ADD CONSTRAINT media_job_desired_target_stream_retained_exact_passthrough CHECK (
        stream_kind NOT IN ('attachment', 'data')
        OR (
            channel_count IS NULL
            AND channel_layout IS NULL
            AND audio_bitrate_bps IS NULL
            AND audio_sample_rate_hz IS NULL
            AND audio_loudness_profile IS NULL
            AND audio_dynamic_range IS NULL
            AND video_profile IS NULL
            AND video_level IS NULL
            AND video_bitrate_bps IS NULL
            AND color_primaries IS NULL
            AND color_transfer IS NULL
            AND color_space IS NULL
            AND hdr_format IS NULL
            AND hdr10_mastering_red_x IS NULL
            AND hdr10_mastering_red_y IS NULL
            AND hdr10_mastering_green_x IS NULL
            AND hdr10_mastering_green_y IS NULL
            AND hdr10_mastering_blue_x IS NULL
            AND hdr10_mastering_blue_y IS NULL
            AND hdr10_mastering_white_x IS NULL
            AND hdr10_mastering_white_y IS NULL
            AND hdr10_mastering_min_luminance IS NULL
            AND hdr10_mastering_max_luminance IS NULL
            AND hdr10_max_content_light_level IS NULL
            AND hdr10_max_frame_average_light_level IS NULL
            AND title IS NULL
            AND NOT default_disposition
            AND NOT forced_disposition
            AND subtitle_placement IS NULL
            AND image_subtitle_action IS NULL
        )
    );

CREATE OR REPLACE FUNCTION media_desired_target_stream_append_v6(
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
    hdr10_mastering_red_x_input TEXT,
    hdr10_mastering_red_y_input TEXT,
    hdr10_mastering_green_x_input TEXT,
    hdr10_mastering_green_y_input TEXT,
    hdr10_mastering_blue_x_input TEXT,
    hdr10_mastering_blue_y_input TEXT,
    hdr10_mastering_white_x_input TEXT,
    hdr10_mastering_white_y_input TEXT,
    hdr10_mastering_min_luminance_input TEXT,
    hdr10_mastering_max_luminance_input TEXT,
    hdr10_max_content_light_level_input TEXT,
    hdr10_max_frame_average_light_level_input TEXT,
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
    hdr10_mastering_red_x_value TEXT;
    hdr10_mastering_red_y_value TEXT;
    hdr10_mastering_green_x_value TEXT;
    hdr10_mastering_green_y_value TEXT;
    hdr10_mastering_blue_x_value TEXT;
    hdr10_mastering_blue_y_value TEXT;
    hdr10_mastering_white_x_value TEXT;
    hdr10_mastering_white_y_value TEXT;
    hdr10_mastering_min_luminance_value TEXT;
    hdr10_mastering_max_luminance_value TEXT;
    hdr10_max_content_light_level_value TEXT;
    hdr10_max_frame_average_light_level_value TEXT;
    hdr10_value_count INT;
    title_value TEXT;
    default_disposition_value BOOLEAN;
    forced_disposition_value BOOLEAN;
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
    hdr10_mastering_red_x_value := NULLIF(btrim(hdr10_mastering_red_x_input), '');
    hdr10_mastering_red_y_value := NULLIF(btrim(hdr10_mastering_red_y_input), '');
    hdr10_mastering_green_x_value := NULLIF(btrim(hdr10_mastering_green_x_input), '');
    hdr10_mastering_green_y_value := NULLIF(btrim(hdr10_mastering_green_y_input), '');
    hdr10_mastering_blue_x_value := NULLIF(btrim(hdr10_mastering_blue_x_input), '');
    hdr10_mastering_blue_y_value := NULLIF(btrim(hdr10_mastering_blue_y_input), '');
    hdr10_mastering_white_x_value := NULLIF(btrim(hdr10_mastering_white_x_input), '');
    hdr10_mastering_white_y_value := NULLIF(btrim(hdr10_mastering_white_y_input), '');
    hdr10_mastering_min_luminance_value := NULLIF(btrim(hdr10_mastering_min_luminance_input), '');
    hdr10_mastering_max_luminance_value := NULLIF(btrim(hdr10_mastering_max_luminance_input), '');
    hdr10_max_content_light_level_value := NULLIF(btrim(hdr10_max_content_light_level_input), '');
    hdr10_max_frame_average_light_level_value := NULLIF(btrim(hdr10_max_frame_average_light_level_input), '');
    hdr10_value_count := num_nonnulls(
        hdr10_mastering_red_x_value,
        hdr10_mastering_red_y_value,
        hdr10_mastering_green_x_value,
        hdr10_mastering_green_y_value,
        hdr10_mastering_blue_x_value,
        hdr10_mastering_blue_y_value,
        hdr10_mastering_white_x_value,
        hdr10_mastering_white_y_value,
        hdr10_mastering_min_luminance_value,
        hdr10_mastering_max_luminance_value,
        hdr10_max_content_light_level_value,
        hdr10_max_frame_average_light_level_value
    );
    title_value := NULLIF(btrim(title_input), '');
    default_disposition_value := COALESCE(default_disposition_input, FALSE);
    forced_disposition_value := COALESCE(forced_disposition_input, FALSE);

    IF stream_kind_value NOT IN ('video', 'audio', 'subtitle', 'attachment', 'data') THEN
        RAISE EXCEPTION 'desired target stream kind is unsupported'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_stream_kind_unsupported';
    END IF;

    IF stream_kind_value IN ('attachment', 'data')
        AND (
            channel_count_input IS NOT NULL
            OR channel_layout_value IS NOT NULL
            OR audio_bitrate_bps_input IS NOT NULL
            OR audio_sample_rate_hz_input IS NOT NULL
            OR audio_loudness_profile_value IS NOT NULL
            OR audio_dynamic_range_value IS NOT NULL
            OR video_profile_value IS NOT NULL
            OR video_level_value IS NOT NULL
            OR video_bitrate_bps_input IS NOT NULL
            OR color_primaries_value IS NOT NULL
            OR color_transfer_value IS NOT NULL
            OR color_space_value IS NOT NULL
            OR hdr_format_value IS NOT NULL
            OR hdr10_value_count > 0
            OR title_value IS NOT NULL
            OR default_disposition_value
            OR forced_disposition_value
            OR subtitle_placement_value IS NOT NULL
            OR image_subtitle_action_value IS NOT NULL
        ) THEN
        RAISE EXCEPTION 'retained stream row can only select exact passthrough'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_retained_stream_shape_invalid';
    END IF;

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
        IF hdr10_value_count NOT IN (0, 12)
            OR (
                hdr10_value_count = 12
                AND (
                    hdr_format_value IS DISTINCT FROM 'hdr10'
                    OR NOT media_hdr10_metadata_valid_v1(
                        hdr10_mastering_red_x_value,
                        hdr10_mastering_red_y_value,
                        hdr10_mastering_green_x_value,
                        hdr10_mastering_green_y_value,
                        hdr10_mastering_blue_x_value,
                        hdr10_mastering_blue_y_value,
                        hdr10_mastering_white_x_value,
                        hdr10_mastering_white_y_value,
                        hdr10_mastering_min_luminance_value,
                        hdr10_mastering_max_luminance_value,
                        hdr10_max_content_light_level_value,
                        hdr10_max_frame_average_light_level_value
                    )
                )
            ) THEN
            RAISE EXCEPTION 'HDR10 metadata shape is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
    ELSIF video_profile_value IS NOT NULL
        OR video_level_value IS NOT NULL
        OR video_bitrate_bps_input IS NOT NULL
        OR color_primaries_value IS NOT NULL
        OR color_transfer_value IS NOT NULL
        OR color_space_value IS NOT NULL
        OR hdr_format_value IS NOT NULL
        OR hdr10_value_count > 0 THEN
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
        hdr_format,
        hdr10_mastering_red_x,
        hdr10_mastering_red_y,
        hdr10_mastering_green_x,
        hdr10_mastering_green_y,
        hdr10_mastering_blue_x,
        hdr10_mastering_blue_y,
        hdr10_mastering_white_x,
        hdr10_mastering_white_y,
        hdr10_mastering_min_luminance,
        hdr10_mastering_max_luminance,
        hdr10_max_content_light_level,
        hdr10_max_frame_average_light_level
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
        title_value,
        default_disposition_value,
        forced_disposition_value,
        subtitle_placement_value,
        image_subtitle_action_value,
        video_profile_value,
        video_level_value,
        video_bitrate_bps_input,
        color_primaries_value,
        color_transfer_value,
        color_space_value,
        hdr_format_value,
        hdr10_mastering_red_x_value,
        hdr10_mastering_red_y_value,
        hdr10_mastering_green_x_value,
        hdr10_mastering_green_y_value,
        hdr10_mastering_blue_x_value,
        hdr10_mastering_blue_y_value,
        hdr10_mastering_white_x_value,
        hdr10_mastering_white_y_value,
        hdr10_mastering_min_luminance_value,
        hdr10_mastering_max_luminance_value,
        hdr10_max_content_light_level_value,
        hdr10_max_frame_average_light_level_value
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
