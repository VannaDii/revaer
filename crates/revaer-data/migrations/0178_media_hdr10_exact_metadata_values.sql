ALTER TABLE media_desired_target_stream
    ADD COLUMN hdr10_mastering_red_x TEXT,
    ADD COLUMN hdr10_mastering_red_y TEXT,
    ADD COLUMN hdr10_mastering_green_x TEXT,
    ADD COLUMN hdr10_mastering_green_y TEXT,
    ADD COLUMN hdr10_mastering_blue_x TEXT,
    ADD COLUMN hdr10_mastering_blue_y TEXT,
    ADD COLUMN hdr10_mastering_white_x TEXT,
    ADD COLUMN hdr10_mastering_white_y TEXT,
    ADD COLUMN hdr10_mastering_min_luminance TEXT,
    ADD COLUMN hdr10_mastering_max_luminance TEXT,
    ADD COLUMN hdr10_max_content_light_level TEXT,
    ADD COLUMN hdr10_max_frame_average_light_level TEXT;

ALTER TABLE media_job_desired_target_stream
    ADD COLUMN hdr10_mastering_red_x TEXT,
    ADD COLUMN hdr10_mastering_red_y TEXT,
    ADD COLUMN hdr10_mastering_green_x TEXT,
    ADD COLUMN hdr10_mastering_green_y TEXT,
    ADD COLUMN hdr10_mastering_blue_x TEXT,
    ADD COLUMN hdr10_mastering_blue_y TEXT,
    ADD COLUMN hdr10_mastering_white_x TEXT,
    ADD COLUMN hdr10_mastering_white_y TEXT,
    ADD COLUMN hdr10_mastering_min_luminance TEXT,
    ADD COLUMN hdr10_mastering_max_luminance TEXT,
    ADD COLUMN hdr10_max_content_light_level TEXT,
    ADD COLUMN hdr10_max_frame_average_light_level TEXT;

CREATE FUNCTION media_hdr10_numeric_text_valid_v1(value_input TEXT)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT value_input IS NULL
        OR btrim(value_input) ~ '^((0|[1-9][0-9]*)(\.[0-9]+)?|(0|[1-9][0-9]*)/[1-9][0-9]*)$'
$$;

CREATE FUNCTION media_hdr10_scaled_value_v1(value_input TEXT, scale_input BIGINT)
RETURNS BIGINT
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    raw_value TEXT;
    numeric_value NUMERIC;
    scaled_value NUMERIC;
BEGIN
    IF value_input IS NULL OR scale_input IS NULL OR scale_input <= 0 THEN
        RETURN NULL;
    END IF;

    raw_value := btrim(value_input);
    IF NOT media_hdr10_numeric_text_valid_v1(raw_value) THEN
        RETURN NULL;
    END IF;

    IF raw_value LIKE '%/%' THEN
        numeric_value := split_part(raw_value, '/', 1)::NUMERIC
            / split_part(raw_value, '/', 2)::NUMERIC;
    ELSE
        numeric_value := raw_value::NUMERIC;
    END IF;

    scaled_value := numeric_value * scale_input;
    IF scaled_value <> trunc(scaled_value)
        OR scaled_value < 0
        OR scaled_value > 9223372036854775807::NUMERIC THEN
        RETURN NULL;
    END IF;

    RETURN scaled_value::BIGINT;
END;
$$;

CREATE FUNCTION media_hdr10_metadata_valid_v1(
    mastering_red_x_input TEXT,
    mastering_red_y_input TEXT,
    mastering_green_x_input TEXT,
    mastering_green_y_input TEXT,
    mastering_blue_x_input TEXT,
    mastering_blue_y_input TEXT,
    mastering_white_x_input TEXT,
    mastering_white_y_input TEXT,
    mastering_min_luminance_input TEXT,
    mastering_max_luminance_input TEXT,
    max_content_light_level_input TEXT,
    max_frame_average_light_level_input TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    red_x BIGINT;
    red_y BIGINT;
    green_x BIGINT;
    green_y BIGINT;
    blue_x BIGINT;
    blue_y BIGINT;
    white_x BIGINT;
    white_y BIGINT;
    min_luminance BIGINT;
    max_luminance BIGINT;
    max_content_light BIGINT;
    max_frame_average_light BIGINT;
    triangle_area BIGINT;
    red_green_white_area BIGINT;
    green_blue_white_area BIGINT;
    blue_red_white_area BIGINT;
BEGIN
    red_x := media_hdr10_scaled_value_v1(mastering_red_x_input, 50000);
    red_y := media_hdr10_scaled_value_v1(mastering_red_y_input, 50000);
    green_x := media_hdr10_scaled_value_v1(mastering_green_x_input, 50000);
    green_y := media_hdr10_scaled_value_v1(mastering_green_y_input, 50000);
    blue_x := media_hdr10_scaled_value_v1(mastering_blue_x_input, 50000);
    blue_y := media_hdr10_scaled_value_v1(mastering_blue_y_input, 50000);
    white_x := media_hdr10_scaled_value_v1(mastering_white_x_input, 50000);
    white_y := media_hdr10_scaled_value_v1(mastering_white_y_input, 50000);
    min_luminance := media_hdr10_scaled_value_v1(mastering_min_luminance_input, 10000);
    max_luminance := media_hdr10_scaled_value_v1(mastering_max_luminance_input, 10000);
    max_content_light := media_hdr10_scaled_value_v1(max_content_light_level_input, 1);
    max_frame_average_light := media_hdr10_scaled_value_v1(max_frame_average_light_level_input, 1);

    IF red_x IS NULL OR red_y IS NULL
        OR green_x IS NULL OR green_y IS NULL
        OR blue_x IS NULL OR blue_y IS NULL
        OR white_x IS NULL OR white_y IS NULL
        OR min_luminance IS NULL OR max_luminance IS NULL
        OR max_content_light IS NULL OR max_frame_average_light IS NULL THEN
        RETURN FALSE;
    END IF;

    IF red_x <= 0 OR red_y <= 0 OR red_x + red_y > 50000
        OR green_x <= 0 OR green_y <= 0 OR green_x + green_y > 50000
        OR blue_x <= 0 OR blue_y <= 0 OR blue_x + blue_y > 50000
        OR white_x <= 0 OR white_y <= 0 OR white_x + white_y > 50000 THEN
        RETURN FALSE;
    END IF;

    triangle_area := (green_x - red_x) * (blue_y - red_y)
        - (green_y - red_y) * (blue_x - red_x);
    red_green_white_area := (green_x - red_x) * (white_y - red_y)
        - (green_y - red_y) * (white_x - red_x);
    green_blue_white_area := (blue_x - green_x) * (white_y - green_y)
        - (blue_y - green_y) * (white_x - green_x);
    blue_red_white_area := (red_x - blue_x) * (white_y - blue_y)
        - (red_y - blue_y) * (white_x - blue_x);

    IF triangle_area = 0 THEN
        RETURN FALSE;
    END IF;
    IF triangle_area > 0
        AND (red_green_white_area < 0
            OR green_blue_white_area < 0
            OR blue_red_white_area < 0) THEN
        RETURN FALSE;
    END IF;
    IF triangle_area < 0
        AND (red_green_white_area > 0
            OR green_blue_white_area > 0
            OR blue_red_white_area > 0) THEN
        RETURN FALSE;
    END IF;

    RETURN max_luminance > min_luminance
        AND max_content_light > 0
        AND max_frame_average_light > 0
        AND max_frame_average_light <= max_content_light;
END;
$$;

ALTER TABLE media_desired_target_stream
    ADD CONSTRAINT media_desired_target_stream_hdr10_metadata_shape CHECK (
        num_nonnulls(
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
        ) = 0
        OR (
            stream_kind = 'video'
            AND hdr_format = 'hdr10'
            AND num_nonnulls(
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
            ) = 12
            AND media_hdr10_metadata_valid_v1(
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
        )
    );

ALTER TABLE media_job_desired_target_stream
    ADD CONSTRAINT media_job_desired_target_stream_hdr10_metadata_shape CHECK (
        num_nonnulls(
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
        ) = 0
        OR (
            stream_kind = 'video'
            AND hdr_format = 'hdr10'
            AND num_nonnulls(
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
            ) = 12
            AND media_hdr10_metadata_valid_v1(
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
        )
    );

CREATE FUNCTION media_desired_target_stream_append_v6(
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

CREATE FUNCTION media_desired_target_stream_list_v6(
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
    audio_bitrate_bps INT,
    audio_sample_rate_hz INT,
    audio_loudness_profile TEXT,
    audio_dynamic_range TEXT,
    video_profile TEXT,
    video_level TEXT,
    video_bitrate_bps INT,
    color_primaries TEXT,
    color_transfer TEXT,
    color_space TEXT,
    hdr_format TEXT,
    hdr10_mastering_red_x TEXT,
    hdr10_mastering_red_y TEXT,
    hdr10_mastering_green_x TEXT,
    hdr10_mastering_green_y TEXT,
    hdr10_mastering_blue_x TEXT,
    hdr10_mastering_blue_y TEXT,
    hdr10_mastering_white_x TEXT,
    hdr10_mastering_white_y TEXT,
    hdr10_mastering_min_luminance TEXT,
    hdr10_mastering_max_luminance TEXT,
    hdr10_max_content_light_level TEXT,
    hdr10_max_frame_average_light_level TEXT,
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
           audio.audio_bitrate_bps,
           audio.audio_sample_rate_hz,
           audio.audio_loudness_profile,
           audio.audio_dynamic_range,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.hdr10_mastering_red_x,
           stream.hdr10_mastering_red_y,
           stream.hdr10_mastering_green_x,
           stream.hdr10_mastering_green_y,
           stream.hdr10_mastering_blue_x,
           stream.hdr10_mastering_blue_y,
           stream.hdr10_mastering_white_x,
           stream.hdr10_mastering_white_y,
           stream.hdr10_mastering_min_luminance,
           stream.hdr10_mastering_max_luminance,
           stream.hdr10_max_content_light_level,
           stream.hdr10_max_frame_average_light_level,
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

CREATE OR REPLACE FUNCTION media_job_desired_target_hdr10_snapshot_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    IF NEW.stream_kind = 'video' THEN
        SELECT target_stream.hdr10_mastering_red_x,
               target_stream.hdr10_mastering_red_y,
               target_stream.hdr10_mastering_green_x,
               target_stream.hdr10_mastering_green_y,
               target_stream.hdr10_mastering_blue_x,
               target_stream.hdr10_mastering_blue_y,
               target_stream.hdr10_mastering_white_x,
               target_stream.hdr10_mastering_white_y,
               target_stream.hdr10_mastering_min_luminance,
               target_stream.hdr10_mastering_max_luminance,
               target_stream.hdr10_max_content_light_level,
               target_stream.hdr10_max_frame_average_light_level
          INTO NEW.hdr10_mastering_red_x,
               NEW.hdr10_mastering_red_y,
               NEW.hdr10_mastering_green_x,
               NEW.hdr10_mastering_green_y,
               NEW.hdr10_mastering_blue_x,
               NEW.hdr10_mastering_blue_y,
               NEW.hdr10_mastering_white_x,
               NEW.hdr10_mastering_white_y,
               NEW.hdr10_mastering_min_luminance,
               NEW.hdr10_mastering_max_luminance,
               NEW.hdr10_max_content_light_level,
               NEW.hdr10_max_frame_average_light_level
          FROM media_job job
          JOIN media_desired_target_stream target_stream
            ON target_stream.media_desired_target_profile_id = job.intent_desired_target_profile_id
           AND lower(target_stream.stream_key) = lower(NEW.stream_key)
         WHERE job.media_job_id = NEW.media_job_id;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER media_job_desired_target_hdr10_snapshot
BEFORE INSERT ON media_job_desired_target_stream
FOR EACH ROW
EXECUTE FUNCTION media_job_desired_target_hdr10_snapshot_v1();

CREATE FUNCTION media_job_desired_target_stream_list_v6(
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
    audio_bitrate_bps INT,
    audio_sample_rate_hz INT,
    audio_loudness_profile TEXT,
    audio_dynamic_range TEXT,
    video_profile TEXT,
    video_level TEXT,
    video_bitrate_bps INT,
    color_primaries TEXT,
    color_transfer TEXT,
    color_space TEXT,
    hdr_format TEXT,
    hdr10_mastering_red_x TEXT,
    hdr10_mastering_red_y TEXT,
    hdr10_mastering_green_x TEXT,
    hdr10_mastering_green_y TEXT,
    hdr10_mastering_blue_x TEXT,
    hdr10_mastering_blue_y TEXT,
    hdr10_mastering_white_x TEXT,
    hdr10_mastering_white_y TEXT,
    hdr10_mastering_min_luminance TEXT,
    hdr10_mastering_max_luminance TEXT,
    hdr10_max_content_light_level TEXT,
    hdr10_max_frame_average_light_level TEXT,
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
           stream.audio_bitrate_bps,
           stream.audio_sample_rate_hz,
           stream.audio_loudness_profile,
           stream.audio_dynamic_range,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.hdr10_mastering_red_x,
           stream.hdr10_mastering_red_y,
           stream.hdr10_mastering_green_x,
           stream.hdr10_mastering_green_y,
           stream.hdr10_mastering_blue_x,
           stream.hdr10_mastering_blue_y,
           stream.hdr10_mastering_white_x,
           stream.hdr10_mastering_white_y,
           stream.hdr10_mastering_min_luminance,
           stream.hdr10_mastering_max_luminance,
           stream.hdr10_max_content_light_level,
           stream.hdr10_max_frame_average_light_level,
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
