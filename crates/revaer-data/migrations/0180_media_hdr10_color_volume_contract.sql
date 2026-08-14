ALTER TABLE media_desired_target_stream
    ADD COLUMN hdr10_mastering_red_x TEXT,
    ADD COLUMN hdr10_mastering_red_y TEXT,
    ADD COLUMN hdr10_mastering_green_x TEXT,
    ADD COLUMN hdr10_mastering_green_y TEXT,
    ADD COLUMN hdr10_mastering_blue_x TEXT,
    ADD COLUMN hdr10_mastering_blue_y TEXT,
    ADD COLUMN hdr10_mastering_white_point_x TEXT,
    ADD COLUMN hdr10_mastering_white_point_y TEXT,
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
    ADD COLUMN hdr10_mastering_white_point_x TEXT,
    ADD COLUMN hdr10_mastering_white_point_y TEXT,
    ADD COLUMN hdr10_mastering_min_luminance TEXT,
    ADD COLUMN hdr10_mastering_max_luminance TEXT,
    ADD COLUMN hdr10_max_content_light_level TEXT,
    ADD COLUMN hdr10_max_frame_average_light_level TEXT;

ALTER TABLE media_desired_target_stream
    DROP CONSTRAINT media_desired_target_stream_kind_known,
    ADD CONSTRAINT media_desired_target_stream_kind_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle', 'attachment', 'data')
    );

ALTER TABLE media_job_desired_target_stream
    DROP CONSTRAINT media_job_desired_target_stream_kind_known,
    ADD CONSTRAINT media_job_desired_target_stream_kind_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle', 'attachment', 'data')
    );

CREATE FUNCTION media_hdr10_value_v1(value_input TEXT)
RETURNS DOUBLE PRECISION
LANGUAGE plpgsql
IMMUTABLE
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    raw_value TEXT;
    parts TEXT[];
    numerator DOUBLE PRECISION;
    denominator DOUBLE PRECISION;
BEGIN
    raw_value := NULLIF(btrim(value_input), '');
    IF raw_value IS NULL THEN
        RETURN NULL;
    END IF;

    IF position('/' IN raw_value) > 0 THEN
        parts := string_to_array(raw_value, '/');
        IF array_length(parts, 1) IS DISTINCT FROM 2 THEN
            RETURN NULL;
        END IF;
        IF btrim(parts[1]) !~ '^[+-]?([0-9]+([.][0-9]+)?|[.][0-9]+)$'
            OR btrim(parts[2]) !~ '^[+-]?([0-9]+([.][0-9]+)?|[.][0-9]+)$' THEN
            RETURN NULL;
        END IF;
        numerator := btrim(parts[1])::DOUBLE PRECISION;
        denominator := btrim(parts[2])::DOUBLE PRECISION;
        IF denominator <= 0 THEN
            RETURN NULL;
        END IF;
        RETURN numerator / denominator;
    END IF;

    IF raw_value !~ '^[+-]?([0-9]+([.][0-9]+)?|[.][0-9]+)$' THEN
        RETURN NULL;
    END IF;
    RETURN raw_value::DOUBLE PRECISION;
END;
$$;

CREATE FUNCTION media_hdr10_color_volume_field_count_v1(
    hdr10_mastering_red_x_input TEXT,
    hdr10_mastering_red_y_input TEXT,
    hdr10_mastering_green_x_input TEXT,
    hdr10_mastering_green_y_input TEXT,
    hdr10_mastering_blue_x_input TEXT,
    hdr10_mastering_blue_y_input TEXT,
    hdr10_mastering_white_point_x_input TEXT,
    hdr10_mastering_white_point_y_input TEXT,
    hdr10_mastering_min_luminance_input TEXT,
    hdr10_mastering_max_luminance_input TEXT,
    hdr10_max_content_light_level_input TEXT,
    hdr10_max_frame_average_light_level_input TEXT
)
RETURNS INT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT
        CASE WHEN NULLIF(btrim(hdr10_mastering_red_x_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_red_y_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_green_x_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_green_y_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_blue_x_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_blue_y_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_white_point_x_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_white_point_y_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_min_luminance_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_max_luminance_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_max_content_light_level_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_max_frame_average_light_level_input), '') IS NULL THEN 0 ELSE 1 END;
$$;

CREATE FUNCTION media_hdr10_triangle_area_v1(
    first_x DOUBLE PRECISION,
    first_y DOUBLE PRECISION,
    second_x DOUBLE PRECISION,
    second_y DOUBLE PRECISION,
    third_x DOUBLE PRECISION,
    third_y DOUBLE PRECISION
)
RETURNS DOUBLE PRECISION
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT abs(
        ((second_x - first_x) * (third_y - first_y))
        - ((third_x - first_x) * (second_y - first_y))
    ) / 2.0;
$$;

CREATE FUNCTION media_hdr10_point_inside_triangle_v1(
    point_x DOUBLE PRECISION,
    point_y DOUBLE PRECISION,
    first_x DOUBLE PRECISION,
    first_y DOUBLE PRECISION,
    second_x DOUBLE PRECISION,
    second_y DOUBLE PRECISION,
    third_x DOUBLE PRECISION,
    third_y DOUBLE PRECISION
)
RETURNS BOOLEAN
LANGUAGE plpgsql
IMMUTABLE
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    triangle_area DOUBLE PRECISION;
    first_area DOUBLE PRECISION;
    second_area DOUBLE PRECISION;
    third_area DOUBLE PRECISION;
BEGIN
    triangle_area := media_hdr10_triangle_area_v1(
        first_x,
        first_y,
        second_x,
        second_y,
        third_x,
        third_y
    );
    first_area := media_hdr10_triangle_area_v1(
        point_x,
        point_y,
        second_x,
        second_y,
        third_x,
        third_y
    );
    second_area := media_hdr10_triangle_area_v1(
        first_x,
        first_y,
        point_x,
        point_y,
        third_x,
        third_y
    );
    third_area := media_hdr10_triangle_area_v1(
        first_x,
        first_y,
        second_x,
        second_y,
        point_x,
        point_y
    );

    RETURN triangle_area > 0.000000001
        AND abs(triangle_area - (first_area + second_area + third_area)) <= 0.000001;
END;
$$;

CREATE FUNCTION media_hdr10_color_volume_valid_v1(
    hdr10_mastering_red_x_input TEXT,
    hdr10_mastering_red_y_input TEXT,
    hdr10_mastering_green_x_input TEXT,
    hdr10_mastering_green_y_input TEXT,
    hdr10_mastering_blue_x_input TEXT,
    hdr10_mastering_blue_y_input TEXT,
    hdr10_mastering_white_point_x_input TEXT,
    hdr10_mastering_white_point_y_input TEXT,
    hdr10_mastering_min_luminance_input TEXT,
    hdr10_mastering_max_luminance_input TEXT,
    hdr10_max_content_light_level_input TEXT,
    hdr10_max_frame_average_light_level_input TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
IMMUTABLE
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    red_x DOUBLE PRECISION;
    red_y DOUBLE PRECISION;
    green_x DOUBLE PRECISION;
    green_y DOUBLE PRECISION;
    blue_x DOUBLE PRECISION;
    blue_y DOUBLE PRECISION;
    white_x DOUBLE PRECISION;
    white_y DOUBLE PRECISION;
    min_luminance DOUBLE PRECISION;
    max_luminance DOUBLE PRECISION;
    max_content_light_level DOUBLE PRECISION;
    max_frame_average_light_level DOUBLE PRECISION;
BEGIN
    IF media_hdr10_color_volume_field_count_v1(
        hdr10_mastering_red_x_input,
        hdr10_mastering_red_y_input,
        hdr10_mastering_green_x_input,
        hdr10_mastering_green_y_input,
        hdr10_mastering_blue_x_input,
        hdr10_mastering_blue_y_input,
        hdr10_mastering_white_point_x_input,
        hdr10_mastering_white_point_y_input,
        hdr10_mastering_min_luminance_input,
        hdr10_mastering_max_luminance_input,
        hdr10_max_content_light_level_input,
        hdr10_max_frame_average_light_level_input
    ) <> 12 THEN
        RETURN FALSE;
    END IF;

    red_x := media_hdr10_value_v1(hdr10_mastering_red_x_input);
    red_y := media_hdr10_value_v1(hdr10_mastering_red_y_input);
    green_x := media_hdr10_value_v1(hdr10_mastering_green_x_input);
    green_y := media_hdr10_value_v1(hdr10_mastering_green_y_input);
    blue_x := media_hdr10_value_v1(hdr10_mastering_blue_x_input);
    blue_y := media_hdr10_value_v1(hdr10_mastering_blue_y_input);
    white_x := media_hdr10_value_v1(hdr10_mastering_white_point_x_input);
    white_y := media_hdr10_value_v1(hdr10_mastering_white_point_y_input);
    min_luminance := media_hdr10_value_v1(hdr10_mastering_min_luminance_input);
    max_luminance := media_hdr10_value_v1(hdr10_mastering_max_luminance_input);
    max_content_light_level := media_hdr10_value_v1(hdr10_max_content_light_level_input);
    max_frame_average_light_level := media_hdr10_value_v1(
        hdr10_max_frame_average_light_level_input
    );

    IF red_x IS NULL OR red_y IS NULL
        OR green_x IS NULL OR green_y IS NULL
        OR blue_x IS NULL OR blue_y IS NULL
        OR white_x IS NULL OR white_y IS NULL
        OR min_luminance IS NULL OR max_luminance IS NULL
        OR max_content_light_level IS NULL
        OR max_frame_average_light_level IS NULL THEN
        RETURN FALSE;
    END IF;

    IF red_x <= 0.0 OR red_y <= 0.0 OR red_x + red_y > 1.0
        OR green_x <= 0.0 OR green_y <= 0.0 OR green_x + green_y > 1.0
        OR blue_x <= 0.0 OR blue_y <= 0.0 OR blue_x + blue_y > 1.0
        OR white_x <= 0.0 OR white_y <= 0.0 OR white_x + white_y > 1.0 THEN
        RETURN FALSE;
    END IF;

    IF media_hdr10_triangle_area_v1(red_x, red_y, green_x, green_y, blue_x, blue_y)
        <= 0.000000001 THEN
        RETURN FALSE;
    END IF;

    IF NOT media_hdr10_point_inside_triangle_v1(
        white_x,
        white_y,
        red_x,
        red_y,
        green_x,
        green_y,
        blue_x,
        blue_y
    ) THEN
        RETURN FALSE;
    END IF;

    RETURN min_luminance >= 0.0
        AND max_luminance > min_luminance
        AND max_content_light_level > 0.0
        AND max_frame_average_light_level > 0.0
        AND max_frame_average_light_level <= max_content_light_level;
END;
$$;

ALTER TABLE media_desired_target_stream
    ADD CONSTRAINT media_desired_target_stream_hdr10_color_volume_shape CHECK (
        (
            stream_kind = 'video'
            AND (video_bitrate_bps IS NULL OR video_bitrate_bps > 0)
            AND (
                media_hdr10_color_volume_field_count_v1(
                    hdr10_mastering_red_x,
                    hdr10_mastering_red_y,
                    hdr10_mastering_green_x,
                    hdr10_mastering_green_y,
                    hdr10_mastering_blue_x,
                    hdr10_mastering_blue_y,
                    hdr10_mastering_white_point_x,
                    hdr10_mastering_white_point_y,
                    hdr10_mastering_min_luminance,
                    hdr10_mastering_max_luminance,
                    hdr10_max_content_light_level,
                    hdr10_max_frame_average_light_level
                ) = 0
                OR COALESCE(
                    hdr_format = 'hdr10'
                    AND media_hdr10_color_volume_valid_v1(
                        hdr10_mastering_red_x,
                        hdr10_mastering_red_y,
                        hdr10_mastering_green_x,
                        hdr10_mastering_green_y,
                        hdr10_mastering_blue_x,
                        hdr10_mastering_blue_y,
                        hdr10_mastering_white_point_x,
                        hdr10_mastering_white_point_y,
                        hdr10_mastering_min_luminance,
                        hdr10_mastering_max_luminance,
                        hdr10_max_content_light_level,
                        hdr10_max_frame_average_light_level
                    ),
                    FALSE
                )
            )
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
            AND hdr10_mastering_red_x IS NULL
            AND hdr10_mastering_red_y IS NULL
            AND hdr10_mastering_green_x IS NULL
            AND hdr10_mastering_green_y IS NULL
            AND hdr10_mastering_blue_x IS NULL
            AND hdr10_mastering_blue_y IS NULL
            AND hdr10_mastering_white_point_x IS NULL
            AND hdr10_mastering_white_point_y IS NULL
            AND hdr10_mastering_min_luminance IS NULL
            AND hdr10_mastering_max_luminance IS NULL
            AND hdr10_max_content_light_level IS NULL
            AND hdr10_max_frame_average_light_level IS NULL
        )
    ),
    ADD CONSTRAINT media_desired_target_stream_retained_color_volume_passthrough CHECK (
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
            AND hdr10_mastering_white_point_x IS NULL
            AND hdr10_mastering_white_point_y IS NULL
            AND hdr10_mastering_min_luminance IS NULL
            AND hdr10_mastering_max_luminance IS NULL
            AND hdr10_max_content_light_level IS NULL
            AND hdr10_max_frame_average_light_level IS NULL
        )
    );

ALTER TABLE media_job_desired_target_stream
    ADD CONSTRAINT media_job_desired_target_stream_hdr10_color_volume_shape CHECK (
        (
            stream_kind = 'video'
            AND (video_bitrate_bps IS NULL OR video_bitrate_bps > 0)
            AND (
                media_hdr10_color_volume_field_count_v1(
                    hdr10_mastering_red_x,
                    hdr10_mastering_red_y,
                    hdr10_mastering_green_x,
                    hdr10_mastering_green_y,
                    hdr10_mastering_blue_x,
                    hdr10_mastering_blue_y,
                    hdr10_mastering_white_point_x,
                    hdr10_mastering_white_point_y,
                    hdr10_mastering_min_luminance,
                    hdr10_mastering_max_luminance,
                    hdr10_max_content_light_level,
                    hdr10_max_frame_average_light_level
                ) = 0
                OR COALESCE(
                    hdr_format = 'hdr10'
                    AND media_hdr10_color_volume_valid_v1(
                        hdr10_mastering_red_x,
                        hdr10_mastering_red_y,
                        hdr10_mastering_green_x,
                        hdr10_mastering_green_y,
                        hdr10_mastering_blue_x,
                        hdr10_mastering_blue_y,
                        hdr10_mastering_white_point_x,
                        hdr10_mastering_white_point_y,
                        hdr10_mastering_min_luminance,
                        hdr10_mastering_max_luminance,
                        hdr10_max_content_light_level,
                        hdr10_max_frame_average_light_level
                    ),
                    FALSE
                )
            )
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
            AND hdr10_mastering_red_x IS NULL
            AND hdr10_mastering_red_y IS NULL
            AND hdr10_mastering_green_x IS NULL
            AND hdr10_mastering_green_y IS NULL
            AND hdr10_mastering_blue_x IS NULL
            AND hdr10_mastering_blue_y IS NULL
            AND hdr10_mastering_white_point_x IS NULL
            AND hdr10_mastering_white_point_y IS NULL
            AND hdr10_mastering_min_luminance IS NULL
            AND hdr10_mastering_max_luminance IS NULL
            AND hdr10_max_content_light_level IS NULL
            AND hdr10_max_frame_average_light_level IS NULL
        )
    ),
    ADD CONSTRAINT media_job_desired_target_stream_retained_color_volume_passthrough CHECK (
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
            AND hdr10_mastering_white_point_x IS NULL
            AND hdr10_mastering_white_point_y IS NULL
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

CREATE FUNCTION media_desired_target_stream_append_v7(
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
    hdr10_mastering_white_point_x_input TEXT,
    hdr10_mastering_white_point_y_input TEXT,
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
    hdr10_mastering_white_point_x_value TEXT;
    hdr10_mastering_white_point_y_value TEXT;
    hdr10_mastering_min_luminance_value TEXT;
    hdr10_mastering_max_luminance_value TEXT;
    hdr10_max_content_light_level_value TEXT;
    hdr10_max_frame_average_light_level_value TEXT;
    hdr10_field_count INT;
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
    hdr10_mastering_white_point_x_value := NULLIF(
        btrim(hdr10_mastering_white_point_x_input),
        ''
    );
    hdr10_mastering_white_point_y_value := NULLIF(
        btrim(hdr10_mastering_white_point_y_input),
        ''
    );
    hdr10_mastering_min_luminance_value := NULLIF(
        btrim(hdr10_mastering_min_luminance_input),
        ''
    );
    hdr10_mastering_max_luminance_value := NULLIF(
        btrim(hdr10_mastering_max_luminance_input),
        ''
    );
    hdr10_max_content_light_level_value := NULLIF(
        btrim(hdr10_max_content_light_level_input),
        ''
    );
    hdr10_max_frame_average_light_level_value := NULLIF(
        btrim(hdr10_max_frame_average_light_level_input),
        ''
    );
    hdr10_field_count := media_hdr10_color_volume_field_count_v1(
        hdr10_mastering_red_x_value,
        hdr10_mastering_red_y_value,
        hdr10_mastering_green_x_value,
        hdr10_mastering_green_y_value,
        hdr10_mastering_blue_x_value,
        hdr10_mastering_blue_y_value,
        hdr10_mastering_white_point_x_value,
        hdr10_mastering_white_point_y_value,
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
            title_value IS NOT NULL
            OR default_disposition_value
            OR forced_disposition_value
            OR video_profile_value IS NOT NULL
            OR video_level_value IS NOT NULL
            OR video_bitrate_bps_input IS NOT NULL
            OR color_primaries_value IS NOT NULL
            OR color_transfer_value IS NOT NULL
            OR color_space_value IS NOT NULL
            OR hdr_format_value IS NOT NULL
            OR hdr10_field_count > 0
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
        IF hdr10_field_count > 0
            AND (
                hdr_format_value IS DISTINCT FROM 'hdr10'
                OR NOT media_hdr10_color_volume_valid_v1(
                    hdr10_mastering_red_x_value,
                    hdr10_mastering_red_y_value,
                    hdr10_mastering_green_x_value,
                    hdr10_mastering_green_y_value,
                    hdr10_mastering_blue_x_value,
                    hdr10_mastering_blue_y_value,
                    hdr10_mastering_white_point_x_value,
                    hdr10_mastering_white_point_y_value,
                    hdr10_mastering_min_luminance_value,
                    hdr10_mastering_max_luminance_value,
                    hdr10_max_content_light_level_value,
                    hdr10_max_frame_average_light_level_value
                )
            ) THEN
            RAISE EXCEPTION 'HDR10 color volume is invalid'
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
        OR hdr_format_value IS NOT NULL
        OR hdr10_field_count > 0 THEN
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
        hdr10_mastering_white_point_x,
        hdr10_mastering_white_point_y,
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
        hdr10_mastering_white_point_x_value,
        hdr10_mastering_white_point_y_value,
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
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT media_desired_target_stream_append_v7(
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
        audio_bitrate_bps_input,
        audio_sample_rate_hz_input,
        audio_loudness_profile_input,
        audio_dynamic_range_input,
        video_profile_input,
        video_level_input,
        video_bitrate_bps_input,
        color_primaries_input,
        color_transfer_input,
        color_space_input,
        hdr_format_input,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        title_input,
        default_disposition_input,
        forced_disposition_input,
        subtitle_placement_input,
        image_subtitle_action_input
    );
$$;

CREATE FUNCTION media_desired_target_stream_list_v7(
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
    hdr10_mastering_white_point_x TEXT,
    hdr10_mastering_white_point_y TEXT,
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
           stream.hdr10_mastering_white_point_x,
           stream.hdr10_mastering_white_point_y,
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

CREATE FUNCTION media_job_desired_target_stream_list_v7(
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
    hdr10_mastering_white_point_x TEXT,
    hdr10_mastering_white_point_y TEXT,
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
           stream.hdr10_mastering_white_point_x,
           stream.hdr10_mastering_white_point_y,
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
    desired_container_metadata_policy TEXT;
    desired_container_chapter_policy TEXT;
    desired_container_attachment_policy TEXT;
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

    IF NOT EXISTS (
        SELECT 1
          FROM media_discovery_source_fingerprint fingerprint
         WHERE fingerprint.media_profile_id = profile_row.media_profile_id
           AND fingerprint.source_path = btrim(source_path_input)
    ) THEN
        RAISE EXCEPTION 'media job source fingerprint required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_source_fingerprint_required';
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
        IF NOT EXISTS (
            SELECT 1
              FROM media_desired_target_stream stream
             WHERE stream.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
        ) THEN
            RAISE EXCEPTION 'desired target has no streams'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_streams_required';
        END IF;
        SELECT container.container_format,
               container.container_metadata_policy,
               container.container_chapter_policy,
               container.container_attachment_policy
          INTO desired_container_format,
               desired_container_metadata_policy,
               desired_container_chapter_policy,
               desired_container_attachment_policy
          FROM media_desired_target_container container
         WHERE container.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id;

        IF desired_container_metadata_policy = 'replace'
           AND NOT EXISTS (
               SELECT 1
                 FROM media_desired_target_container_metadata metadata
                WHERE metadata.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           ) THEN
            RAISE EXCEPTION 'replace metadata target has no metadata rows'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_required';
        END IF;

        IF desired_container_metadata_policy <> 'replace'
           AND EXISTS (
               SELECT 1
                 FROM media_desired_target_container_metadata metadata
                WHERE metadata.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           ) THEN
            RAISE EXCEPTION 'metadata rows require replace policy'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_policy_mismatch';
        END IF;

        IF desired_container_chapter_policy = 'replace'
           AND NOT EXISTS (
               SELECT 1
                 FROM media_desired_target_container_chapter chapter
                WHERE chapter.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           ) THEN
            RAISE EXCEPTION 'replace chapter target has no chapter rows'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapters_required';
        END IF;

        IF desired_container_chapter_policy <> 'replace'
           AND EXISTS (
               SELECT 1
                 FROM media_desired_target_container_chapter chapter
                WHERE chapter.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           ) THEN
            RAISE EXCEPTION 'chapter rows require replace policy'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_policy_mismatch';
        END IF;
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
        intent_desired_container_metadata_policy, intent_desired_container_chapter_policy,
        intent_desired_container_attachment_policy, intent_unmatched_stream_policy,
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
        desired_container_metadata_policy, desired_container_chapter_policy,
        desired_container_attachment_policy, policy_row.unmatched_stream_policy,
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
            audio_bitrate_bps, audio_sample_rate_hz, audio_loudness_profile,
            audio_dynamic_range,
            video_profile, video_level, video_bitrate_bps, color_primaries,
            color_transfer, color_space, hdr_format, hdr10_mastering_red_x,
            hdr10_mastering_red_y, hdr10_mastering_green_x,
            hdr10_mastering_green_y, hdr10_mastering_blue_x,
            hdr10_mastering_blue_y, hdr10_mastering_white_point_x,
            hdr10_mastering_white_point_y, hdr10_mastering_min_luminance,
            hdr10_mastering_max_luminance, hdr10_max_content_light_level,
            hdr10_max_frame_average_light_level, title, default_disposition,
            forced_disposition, subtitle_placement, image_subtitle_action
        )
        SELECT media_job_id_out, stream.stream_key, stream.stream_kind,
               stream.semantic_role, stream.language_code, stream.optional,
               stream.sort_order, stream.codec, audio.channel_count,
               audio.channel_layout, audio.audio_bitrate_bps,
               audio.audio_sample_rate_hz, audio.audio_loudness_profile,
               audio.audio_dynamic_range, stream.video_profile, stream.video_level,
               stream.video_bitrate_bps, stream.color_primaries,
               stream.color_transfer, stream.color_space, stream.hdr_format,
               stream.hdr10_mastering_red_x, stream.hdr10_mastering_red_y,
               stream.hdr10_mastering_green_x, stream.hdr10_mastering_green_y,
               stream.hdr10_mastering_blue_x, stream.hdr10_mastering_blue_y,
               stream.hdr10_mastering_white_point_x,
               stream.hdr10_mastering_white_point_y,
               stream.hdr10_mastering_min_luminance,
               stream.hdr10_mastering_max_luminance,
               stream.hdr10_max_content_light_level,
               stream.hdr10_max_frame_average_light_level,
               stream.title, stream.default_disposition,
               stream.forced_disposition, stream.subtitle_placement,
               stream.image_subtitle_action
          FROM media_desired_target_stream stream
          LEFT JOIN media_desired_target_audio_stream audio
            ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
         WHERE stream.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
         ORDER BY stream.sort_order;

        INSERT INTO media_job_desired_target_metadata (
            media_job_id,
            metadata_key,
            metadata_value
        )
        SELECT media_job_id_out,
               metadata.metadata_key,
               metadata.metadata_value
          FROM media_desired_target_container_metadata metadata
         WHERE metadata.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
         ORDER BY metadata.metadata_key;

        WITH copied_chapters AS (
            INSERT INTO media_job_desired_target_chapter (
                media_job_id,
                start_millis,
                end_millis,
                sort_order
            )
            SELECT media_job_id_out,
                   chapter.start_millis,
                   chapter.end_millis,
                   chapter.sort_order
              FROM media_desired_target_container_chapter chapter
             WHERE chapter.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
             ORDER BY chapter.start_millis
            RETURNING media_job_desired_target_chapter_id, start_millis
        )
        INSERT INTO media_job_desired_target_chapter_metadata (
            media_job_desired_target_chapter_id,
            metadata_key,
            metadata_value
        )
        SELECT copied.media_job_desired_target_chapter_id,
               metadata.metadata_key,
               metadata.metadata_value
          FROM copied_chapters copied
          JOIN media_desired_target_container_chapter chapter
            ON chapter.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           AND chapter.start_millis = copied.start_millis
          JOIN media_desired_target_container_chapter_metadata metadata
            ON metadata.media_desired_target_container_chapter_id = chapter.media_desired_target_container_chapter_id
         ORDER BY copied.start_millis, metadata.metadata_key;
    END IF;

    RETURN media_job_public_id_out;
END;
$$;
