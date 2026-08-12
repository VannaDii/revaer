ALTER TABLE media_desired_target_stream
    ADD COLUMN video_width_px INT,
    ADD COLUMN video_height_px INT,
    ADD COLUMN video_pixel_format TEXT,
    ADD COLUMN video_average_frame_rate TEXT;

ALTER TABLE media_job_desired_target_stream
    ADD COLUMN video_width_px INT,
    ADD COLUMN video_height_px INT,
    ADD COLUMN video_pixel_format TEXT,
    ADD COLUMN video_average_frame_rate TEXT;

CREATE FUNCTION media_video_pixel_format_valid_v1(value_input TEXT)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT value_input IS NULL OR btrim(value_input) ~ '^[a-z0-9_]+$'
$$;

CREATE FUNCTION media_video_frame_rate_reduce_v1(value_input TEXT)
RETURNS TEXT
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    value_text TEXT;
    numerator_text TEXT;
    denominator_text TEXT;
    numerator_value BIGINT;
    denominator_value BIGINT;
    left_value BIGINT;
    right_value BIGINT;
    remainder_value BIGINT;
BEGIN
    IF value_input IS NULL THEN
        RETURN NULL;
    END IF;
    value_text := btrim(value_input);
    IF value_text !~ '^[1-9][0-9]*(/[1-9][0-9]*)?$' THEN
        RETURN NULL;
    END IF;
    numerator_text := split_part(value_text, '/', 1);
    denominator_text := CASE
        WHEN strpos(value_text, '/') = 0 THEN '1'
        ELSE split_part(value_text, '/', 2)
    END;
    IF length(numerator_text) > 7 OR length(denominator_text) > 7 THEN
        RETURN NULL;
    END IF;
    numerator_value := numerator_text::BIGINT;
    denominator_value := denominator_text::BIGINT;
    IF numerator_value > 1000000
        OR denominator_value > 1000000
        OR numerator_value > 240 * denominator_value THEN
        RETURN NULL;
    END IF;
    left_value := numerator_value;
    right_value := denominator_value;
    WHILE right_value <> 0 LOOP
        remainder_value := left_value % right_value;
        left_value := right_value;
        right_value := remainder_value;
    END LOOP;
    RETURN (numerator_value / left_value)::TEXT || '/' || (denominator_value / left_value)::TEXT;
END;
$$;

CREATE FUNCTION media_video_frame_rate_valid_v1(value_input TEXT)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT value_input IS NULL OR media_video_frame_rate_reduce_v1(value_input) IS NOT NULL
$$;

ALTER TABLE media_desired_target_stream
    ADD CONSTRAINT media_desired_target_stream_video_technical_shape CHECK (
        (
            video_width_px IS NULL
            AND video_height_px IS NULL
            AND video_pixel_format IS NULL
            AND video_average_frame_rate IS NULL
        )
        OR (
            stream_kind = 'video'
            AND (
                (video_width_px IS NULL AND video_height_px IS NULL)
                OR (
                    video_width_px IS NOT NULL
                    AND video_height_px IS NOT NULL
                    AND video_width_px > 0
                    AND video_height_px > 0
                    AND video_width_px <= 16384
                    AND video_height_px <= 16384
                    AND video_width_px::BIGINT * video_height_px::BIGINT <= 134217728
                )
            )
            AND media_video_pixel_format_valid_v1(video_pixel_format)
            AND media_video_frame_rate_valid_v1(video_average_frame_rate)
        )
    );

ALTER TABLE media_job_desired_target_stream
    ADD CONSTRAINT media_job_desired_target_stream_video_technical_shape CHECK (
        (
            video_width_px IS NULL
            AND video_height_px IS NULL
            AND video_pixel_format IS NULL
            AND video_average_frame_rate IS NULL
        )
        OR (
            stream_kind = 'video'
            AND (
                (video_width_px IS NULL AND video_height_px IS NULL)
                OR (
                    video_width_px IS NOT NULL
                    AND video_height_px IS NOT NULL
                    AND video_width_px > 0
                    AND video_height_px > 0
                    AND video_width_px <= 16384
                    AND video_height_px <= 16384
                    AND video_width_px::BIGINT * video_height_px::BIGINT <= 134217728
                )
            )
            AND media_video_pixel_format_valid_v1(video_pixel_format)
            AND media_video_frame_rate_valid_v1(video_average_frame_rate)
        )
    );

ALTER FUNCTION media_desired_target_stream_append_v7
    RENAME TO media_desired_target_stream_append_v6;

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
    video_width_px_input INT,
    video_height_px_input INT,
    video_pixel_format_input TEXT,
    video_average_frame_rate_input TEXT,
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
    stream_kind_value TEXT;
    video_pixel_format_value TEXT;
    video_average_frame_rate_value TEXT;
BEGIN
    stream_kind_value := lower(btrim(stream_kind_input));
    video_pixel_format_value := NULLIF(btrim(video_pixel_format_input), '');
    video_average_frame_rate_value := CASE
        WHEN NULLIF(btrim(video_average_frame_rate_input), '') IS NULL THEN NULL
        ELSE media_video_frame_rate_reduce_v1(video_average_frame_rate_input)
    END;

    IF stream_kind_value = 'video' THEN
        IF video_bitrate_bps_input IS NOT NULL
            AND (video_bitrate_bps_input <= 0 OR video_bitrate_bps_input > 1000000000) THEN
            RAISE EXCEPTION 'video bitrate is outside the supported range'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF video_width_px_input IS NOT NULL
            AND (video_width_px_input <= 0 OR video_width_px_input > 16384) THEN
            RAISE EXCEPTION 'video width is outside the supported range'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF video_height_px_input IS NOT NULL
            AND (video_height_px_input <= 0 OR video_height_px_input > 16384) THEN
            RAISE EXCEPTION 'video height is outside the supported range'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF (video_width_px_input IS NULL) IS DISTINCT FROM (video_height_px_input IS NULL) THEN
            RAISE EXCEPTION 'video resolution requires width and height'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF video_width_px_input IS NOT NULL
            AND video_width_px_input::BIGINT * video_height_px_input::BIGINT > 134217728 THEN
            RAISE EXCEPTION 'video frame area exceeds the supported range'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF NOT media_video_pixel_format_valid_v1(video_pixel_format_value)
            OR (
                video_average_frame_rate_input IS NOT NULL
                AND video_average_frame_rate_value IS NULL
            ) THEN
            RAISE EXCEPTION 'video technical shape is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
    ELSIF video_width_px_input IS NOT NULL
        OR video_height_px_input IS NOT NULL
        OR video_pixel_format_value IS NOT NULL
        OR NULLIF(btrim(video_average_frame_rate_input), '') IS NOT NULL THEN
        RAISE EXCEPTION 'video shape assigned to non-video target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
    END IF;

    PERFORM media_desired_target_stream_append_v6(
        media_desired_target_profile_public_id_input => media_desired_target_profile_public_id_input,
        stream_key_input => stream_key_input,
        stream_kind_input => stream_kind_input,
        semantic_role_input => semantic_role_input,
        language_code_input => language_code_input,
        optional_input => optional_input,
        sort_order_input => sort_order_input,
        codec_input => codec_input,
        channel_count_input => channel_count_input,
        channel_layout_input => channel_layout_input,
        audio_bitrate_bps_input => audio_bitrate_bps_input,
        audio_sample_rate_hz_input => audio_sample_rate_hz_input,
        audio_loudness_profile_input => audio_loudness_profile_input,
        audio_dynamic_range_input => audio_dynamic_range_input,
        video_profile_input => video_profile_input,
        video_level_input => video_level_input,
        video_bitrate_bps_input => video_bitrate_bps_input,
        color_primaries_input => color_primaries_input,
        color_transfer_input => color_transfer_input,
        color_space_input => color_space_input,
        hdr_format_input => hdr_format_input,
        hdr10_mastering_red_x_input => hdr10_mastering_red_x_input,
        hdr10_mastering_red_y_input => hdr10_mastering_red_y_input,
        hdr10_mastering_green_x_input => hdr10_mastering_green_x_input,
        hdr10_mastering_green_y_input => hdr10_mastering_green_y_input,
        hdr10_mastering_blue_x_input => hdr10_mastering_blue_x_input,
        hdr10_mastering_blue_y_input => hdr10_mastering_blue_y_input,
        hdr10_mastering_white_point_x_input => hdr10_mastering_white_point_x_input,
        hdr10_mastering_white_point_y_input => hdr10_mastering_white_point_y_input,
        hdr10_mastering_min_luminance_input => hdr10_mastering_min_luminance_input,
        hdr10_mastering_max_luminance_input => hdr10_mastering_max_luminance_input,
        hdr10_max_content_light_level_input => hdr10_max_content_light_level_input,
        hdr10_max_frame_average_light_level_input => hdr10_max_frame_average_light_level_input,
        title_input => title_input,
        default_disposition_input => default_disposition_input,
        forced_disposition_input => forced_disposition_input,
        subtitle_placement_input => subtitle_placement_input,
        image_subtitle_action_input => image_subtitle_action_input
    );

    UPDATE media_desired_target_stream stream
       SET video_width_px = video_width_px_input,
           video_height_px = video_height_px_input,
           video_pixel_format = video_pixel_format_value,
           video_average_frame_rate = video_average_frame_rate_value
      FROM media_desired_target_profile target
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND stream.media_desired_target_profile_id = target.media_desired_target_profile_id
       AND lower(stream.stream_key) = lower(btrim(stream_key_input));
END;
$$;

ALTER TABLE media_desired_target_stream
    ADD CONSTRAINT media_desired_target_stream_video_bitrate_bound CHECK (
        video_bitrate_bps IS NULL OR video_bitrate_bps <= 1000000000
    );

ALTER TABLE media_job_desired_target_stream
    ADD CONSTRAINT media_job_desired_target_stream_video_bitrate_bound CHECK (
        video_bitrate_bps IS NULL OR video_bitrate_bps <= 1000000000
    );

DROP FUNCTION media_desired_target_stream_list_v7;

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
    video_width_px INT,
    video_height_px INT,
    video_pixel_format TEXT,
    video_average_frame_rate TEXT,
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
           stream.video_width_px,
           stream.video_height_px,
           stream.video_pixel_format,
           stream.video_average_frame_rate,
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

CREATE FUNCTION media_job_desired_target_video_technical_snapshot_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    IF NEW.stream_kind = 'video' THEN
        SELECT target_stream.video_width_px,
               target_stream.video_height_px,
               target_stream.video_pixel_format,
               target_stream.video_average_frame_rate
          INTO NEW.video_width_px,
               NEW.video_height_px,
               NEW.video_pixel_format,
               NEW.video_average_frame_rate
          FROM media_job job
          JOIN media_desired_target_stream target_stream
            ON target_stream.media_desired_target_profile_id = job.intent_desired_target_profile_id
           AND lower(target_stream.stream_key) = lower(NEW.stream_key)
         WHERE job.media_job_id = NEW.media_job_id;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER media_job_desired_target_video_technical_snapshot
BEFORE INSERT ON media_job_desired_target_stream
FOR EACH ROW
EXECUTE FUNCTION media_job_desired_target_video_technical_snapshot_v1();

DROP FUNCTION media_job_desired_target_stream_list_v7;

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
    video_width_px INT,
    video_height_px INT,
    video_pixel_format TEXT,
    video_average_frame_rate TEXT,
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
           stream.video_width_px,
           stream.video_height_px,
           stream.video_pixel_format,
           stream.video_average_frame_rate,
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

ALTER TABLE media_desired_target_stream
    ADD COLUMN video_bit_depth INT,
    ADD COLUMN color_range TEXT;

ALTER TABLE media_job_desired_target_stream
    ADD COLUMN video_bit_depth INT,
    ADD COLUMN color_range TEXT;

CREATE FUNCTION media_video_bit_depth_supported_v1(value_input INT)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT value_input IS NULL OR value_input IN (8, 10, 12, 16)
$$;

CREATE FUNCTION media_video_pixel_format_bit_depth_v1(value_input TEXT)
RETURNS INT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT CASE NULLIF(lower(btrim(value_input)), '')
        WHEN 'yuv420p' THEN 8
        WHEN 'yuv422p' THEN 8
        WHEN 'yuv444p' THEN 8
        WHEN 'yuv410p' THEN 8
        WHEN 'yuv411p' THEN 8
        WHEN 'yuvj420p' THEN 8
        WHEN 'yuvj422p' THEN 8
        WHEN 'yuvj444p' THEN 8
        WHEN 'nv12' THEN 8
        WHEN 'nv21' THEN 8
        WHEN 'rgb24' THEN 8
        WHEN 'bgr24' THEN 8
        WHEN 'rgba' THEN 8
        WHEN 'bgra' THEN 8
        WHEN 'argb' THEN 8
        WHEN 'abgr' THEN 8
        WHEN 'gray' THEN 8
        WHEN 'pal8' THEN 8
        WHEN 'yuv420p10le' THEN 10
        WHEN 'yuv420p10be' THEN 10
        WHEN 'yuv422p10le' THEN 10
        WHEN 'yuv422p10be' THEN 10
        WHEN 'yuv444p10le' THEN 10
        WHEN 'yuv444p10be' THEN 10
        WHEN 'gbrp10le' THEN 10
        WHEN 'gbrp10be' THEN 10
        WHEN 'gray10le' THEN 10
        WHEN 'gray10be' THEN 10
        WHEN 'p010le' THEN 10
        WHEN 'p010be' THEN 10
        WHEN 'yuv420p12le' THEN 12
        WHEN 'yuv420p12be' THEN 12
        WHEN 'yuv422p12le' THEN 12
        WHEN 'yuv422p12be' THEN 12
        WHEN 'yuv444p12le' THEN 12
        WHEN 'yuv444p12be' THEN 12
        WHEN 'gbrp12le' THEN 12
        WHEN 'gbrp12be' THEN 12
        WHEN 'gray12le' THEN 12
        WHEN 'gray12be' THEN 12
        WHEN 'p012le' THEN 12
        WHEN 'p012be' THEN 12
        WHEN 'yuv420p16le' THEN 16
        WHEN 'yuv420p16be' THEN 16
        WHEN 'yuv422p16le' THEN 16
        WHEN 'yuv422p16be' THEN 16
        WHEN 'yuv444p16le' THEN 16
        WHEN 'yuv444p16be' THEN 16
        WHEN 'gbrp16le' THEN 16
        WHEN 'gbrp16be' THEN 16
        WHEN 'gray16le' THEN 16
        WHEN 'gray16be' THEN 16
        WHEN 'p016le' THEN 16
        WHEN 'p016be' THEN 16
        WHEN 'rgba64le' THEN 16
        WHEN 'rgba64be' THEN 16
        WHEN 'bgra64le' THEN 16
        WHEN 'bgra64be' THEN 16
        ELSE NULL
    END
$$;

CREATE FUNCTION media_video_color_range_known_v1(value_input TEXT)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT NULLIF(lower(btrim(value_input)), '') IS NULL
        OR lower(btrim(value_input)) IN ('tv', 'pc')
$$;

ALTER TABLE media_desired_target_stream
    ADD CONSTRAINT media_desired_target_stream_video_bit_depth_color_range_shape CHECK (
        (
            video_bit_depth IS NULL
            AND color_range IS NULL
        )
        OR (
            stream_kind = 'video'
            AND media_video_bit_depth_supported_v1(video_bit_depth)
            AND (
                video_bit_depth IS NULL
                OR COALESCE(
                    media_video_pixel_format_bit_depth_v1(video_pixel_format) = video_bit_depth,
                    FALSE
                )
            )
            AND media_video_color_range_known_v1(color_range)
        )
    );

ALTER TABLE media_job_desired_target_stream
    ADD CONSTRAINT media_job_desired_target_stream_video_bit_depth_color_range_shape CHECK (
        (
            video_bit_depth IS NULL
            AND color_range IS NULL
        )
        OR (
            stream_kind = 'video'
            AND media_video_bit_depth_supported_v1(video_bit_depth)
            AND (
                video_bit_depth IS NULL
                OR COALESCE(
                    media_video_pixel_format_bit_depth_v1(video_pixel_format) = video_bit_depth,
                    FALSE
                )
            )
            AND media_video_color_range_known_v1(color_range)
        )
    );

CREATE FUNCTION media_desired_target_stream_append_v8(
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
    video_width_px_input INT,
    video_height_px_input INT,
    video_pixel_format_input TEXT,
    video_bit_depth_input INT,
    video_average_frame_rate_input TEXT,
    color_range_input TEXT,
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
    stream_kind_value TEXT;
    video_pixel_format_value TEXT;
    color_range_value TEXT;
BEGIN
    stream_kind_value := lower(btrim(stream_kind_input));
    video_pixel_format_value := NULLIF(btrim(video_pixel_format_input), '');
    color_range_value := NULLIF(lower(btrim(color_range_input)), '');

    IF stream_kind_value = 'video' THEN
        IF NOT media_video_bit_depth_supported_v1(video_bit_depth_input) THEN
            RAISE EXCEPTION 'video bit depth is not supported'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF video_bit_depth_input IS NOT NULL
            AND media_video_pixel_format_bit_depth_v1(video_pixel_format_value)
                IS DISTINCT FROM video_bit_depth_input THEN
            RAISE EXCEPTION 'video bit depth must match pixel format'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF NOT media_video_color_range_known_v1(color_range_value) THEN
            RAISE EXCEPTION 'video color range is not supported'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
    ELSIF video_bit_depth_input IS NOT NULL OR color_range_value IS NOT NULL THEN
        RAISE EXCEPTION 'video shape assigned to non-video target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
    END IF;

    PERFORM media_desired_target_stream_append_v7(
        media_desired_target_profile_public_id_input => media_desired_target_profile_public_id_input,
        stream_key_input => stream_key_input,
        stream_kind_input => stream_kind_input,
        semantic_role_input => semantic_role_input,
        language_code_input => language_code_input,
        optional_input => optional_input,
        sort_order_input => sort_order_input,
        codec_input => codec_input,
        channel_count_input => channel_count_input,
        channel_layout_input => channel_layout_input,
        audio_bitrate_bps_input => audio_bitrate_bps_input,
        audio_sample_rate_hz_input => audio_sample_rate_hz_input,
        audio_loudness_profile_input => audio_loudness_profile_input,
        audio_dynamic_range_input => audio_dynamic_range_input,
        video_profile_input => video_profile_input,
        video_level_input => video_level_input,
        video_bitrate_bps_input => video_bitrate_bps_input,
        video_width_px_input => video_width_px_input,
        video_height_px_input => video_height_px_input,
        video_pixel_format_input => video_pixel_format_input,
        video_average_frame_rate_input => video_average_frame_rate_input,
        color_primaries_input => color_primaries_input,
        color_transfer_input => color_transfer_input,
        color_space_input => color_space_input,
        hdr_format_input => hdr_format_input,
        hdr10_mastering_red_x_input => hdr10_mastering_red_x_input,
        hdr10_mastering_red_y_input => hdr10_mastering_red_y_input,
        hdr10_mastering_green_x_input => hdr10_mastering_green_x_input,
        hdr10_mastering_green_y_input => hdr10_mastering_green_y_input,
        hdr10_mastering_blue_x_input => hdr10_mastering_blue_x_input,
        hdr10_mastering_blue_y_input => hdr10_mastering_blue_y_input,
        hdr10_mastering_white_point_x_input => hdr10_mastering_white_point_x_input,
        hdr10_mastering_white_point_y_input => hdr10_mastering_white_point_y_input,
        hdr10_mastering_min_luminance_input => hdr10_mastering_min_luminance_input,
        hdr10_mastering_max_luminance_input => hdr10_mastering_max_luminance_input,
        hdr10_max_content_light_level_input => hdr10_max_content_light_level_input,
        hdr10_max_frame_average_light_level_input => hdr10_max_frame_average_light_level_input,
        title_input => title_input,
        default_disposition_input => default_disposition_input,
        forced_disposition_input => forced_disposition_input,
        subtitle_placement_input => subtitle_placement_input,
        image_subtitle_action_input => image_subtitle_action_input
    );

    UPDATE media_desired_target_stream stream
       SET video_bit_depth = video_bit_depth_input,
           color_range = color_range_value
      FROM media_desired_target_profile target
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND stream.media_desired_target_profile_id = target.media_desired_target_profile_id
       AND lower(stream.stream_key) = lower(btrim(stream_key_input));
END;
$$;

CREATE FUNCTION media_desired_target_stream_list_v8(
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
    video_width_px INT,
    video_height_px INT,
    video_pixel_format TEXT,
    video_bit_depth INT,
    video_average_frame_rate TEXT,
    color_range TEXT,
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
           stream.video_width_px,
           stream.video_height_px,
           stream.video_pixel_format,
           stream.video_bit_depth,
           stream.video_average_frame_rate,
           stream.color_range,
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

CREATE OR REPLACE FUNCTION media_job_desired_target_video_technical_snapshot_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    IF NEW.stream_kind = 'video' THEN
        SELECT target_stream.video_width_px,
               target_stream.video_height_px,
               target_stream.video_pixel_format,
               target_stream.video_bit_depth,
               target_stream.video_average_frame_rate,
               target_stream.color_range
          INTO NEW.video_width_px,
               NEW.video_height_px,
               NEW.video_pixel_format,
               NEW.video_bit_depth,
               NEW.video_average_frame_rate,
               NEW.color_range
          FROM media_job job
          JOIN media_desired_target_stream target_stream
            ON target_stream.media_desired_target_profile_id = job.intent_desired_target_profile_id
           AND lower(target_stream.stream_key) = lower(NEW.stream_key)
         WHERE job.media_job_id = NEW.media_job_id;
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION media_job_desired_target_stream_list_v8(
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
    video_width_px INT,
    video_height_px INT,
    video_pixel_format TEXT,
    video_bit_depth INT,
    video_average_frame_rate TEXT,
    color_range TEXT,
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
           stream.video_width_px,
           stream.video_height_px,
           stream.video_pixel_format,
           stream.video_bit_depth,
           stream.video_average_frame_rate,
           stream.color_range,
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

CREATE FUNCTION media_job_worker_claim_next_v7()
RETURNS TABLE (
    media_job_public_id UUID,
    media_profile_public_id UUID,
    source_path TEXT,
    output_path TEXT,
    dry_run BOOLEAN,
    source_root TEXT,
    output_root TEXT,
    source_identity TEXT,
    source_size_bytes BIGINT,
    source_modified_ns BIGINT,
    source_changed_ns BIGINT,
    source_sha256 TEXT,
    compatibility_target_key TEXT,
    compatibility_target_version INT,
    policy_key TEXT,
    target_video_codec TEXT,
    target_audio_codec TEXT,
    target_audio_channels INT,
    target_audio_channel_layout TEXT,
    target_subtitle_policy TEXT,
    policy_video_intent TEXT,
    desired_target_key TEXT,
    desired_target_version INT,
    desired_container_format TEXT,
    desired_container_metadata_policy TEXT,
    desired_container_chapter_policy TEXT,
    desired_container_attachment_policy TEXT,
    unmatched_stream_policy TEXT,
    unmatched_video_action TEXT,
    unmatched_audio_action TEXT,
    unmatched_subtitle_action TEXT,
    unmatched_attachment_action TEXT,
    unmatched_data_action TEXT,
    verification_strictness TEXT,
    verification_duration_tolerance_millis BIGINT,
    verification_mux_validation BOOLEAN,
    verification_decode_all_streams BOOLEAN,
    verification_keyframe_seek BOOLEAN,
    verification_playback_probe BOOLEAN,
    cancel_generation BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id
          FROM media_job job
         WHERE job.status = media_job_status_queued_v1()
           AND job.intent_source_identity IS NOT NULL
           AND job.intent_source_size_bytes IS NOT NULL
           AND job.intent_source_modified_ns IS NOT NULL
           AND job.intent_source_changed_ns IS NOT NULL
           AND job.intent_source_sha256 IS NOT NULL
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ),
    updated AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = COALESCE(job.started_at, now()),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM claimed
         WHERE job.media_job_id = claimed.media_job_id
        RETURNING job.*
    )
    SELECT updated.media_job_public_id, profile.media_profile_public_id,
           updated.source_path, updated.output_path, updated.dry_run,
           updated.intent_source_root, updated.intent_output_root,
           updated.intent_source_identity, updated.intent_source_size_bytes,
           updated.intent_source_modified_ns, updated.intent_source_changed_ns,
           updated.intent_source_sha256,
           updated.intent_compatibility_target_key,
           updated.intent_compatibility_target_version,
           updated.intent_policy_key,
           updated.intent_target_video_codec, updated.intent_target_audio_codec,
           updated.intent_target_audio_channels, updated.intent_target_audio_channel_layout,
           updated.intent_target_subtitle_policy, updated.intent_policy_video_intent,
           updated.intent_desired_target_key, updated.intent_desired_target_version,
           updated.intent_desired_container_format,
           updated.intent_desired_container_metadata_policy,
           updated.intent_desired_container_chapter_policy,
           updated.intent_desired_container_attachment_policy,
           updated.intent_unmatched_stream_policy,
           updated.intent_unmatched_video_action,
           updated.intent_unmatched_audio_action,
           updated.intent_unmatched_subtitle_action,
           updated.intent_unmatched_attachment_action,
           updated.intent_unmatched_data_action,
           updated.intent_verification_strictness,
           updated.intent_verification_duration_tolerance_millis,
           updated.intent_verification_mux_validation,
           updated.intent_verification_decode_all_streams,
           updated.intent_verification_keyframe_seek,
           updated.intent_verification_playback_probe,
           updated.cancel_generation
      FROM updated
      JOIN media_profile profile ON profile.media_profile_id = updated.media_profile_id;
END;
$$;

CREATE FUNCTION media_desired_target_graph_page_v3(limit_input INT)
RETURNS TABLE (
    media_desired_target_profile_public_id UUID,
    target_key TEXT,
    version INT,
    display_name TEXT,
    container_format TEXT,
    container_metadata_policy TEXT,
    container_chapter_policy TEXT,
    container_attachment_policy TEXT,
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
    video_width_px INT,
    video_height_px INT,
    video_pixel_format TEXT,
    video_bit_depth INT,
    video_average_frame_rate TEXT,
    color_range TEXT,
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
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF limit_input < 1 OR limit_input > 128 THEN
        RAISE EXCEPTION 'desired target page limit is outside 1..128'
            USING ERRCODE = media_app_error_code_v1(),
                  DETAIL = 'media_desired_target_page_limit_invalid';
    END IF;

    RETURN QUERY
    WITH target_page AS MATERIALIZED (
        SELECT * FROM media_desired_target_list_v4() LIMIT limit_input
    )
    SELECT target.*, stream.*
      FROM target_page target
      JOIN LATERAL (
          SELECT * FROM media_desired_target_stream_list_v8(
              target.media_desired_target_profile_public_id
          ) LIMIT 1025
      ) stream ON TRUE
     ORDER BY lower(target.target_key), target.version DESC, stream.sort_order, stream.stream_key;
END;
$$;
