ALTER TABLE media_desired_target_audio_stream
    ADD COLUMN audio_loudness_profile TEXT,
    ADD COLUMN audio_dynamic_range TEXT,
    ADD CONSTRAINT media_desired_target_audio_loudness_profile_known CHECK (
        audio_loudness_profile IS NULL OR audio_loudness_profile IN ('dialog-normalized')
    ),
    ADD CONSTRAINT media_desired_target_audio_dynamic_range_known CHECK (
        audio_dynamic_range IS NULL OR audio_dynamic_range IN ('preserve', 'speech')
    );

ALTER TABLE media_job_desired_target_stream
    ADD COLUMN audio_loudness_profile TEXT,
    ADD COLUMN audio_dynamic_range TEXT,
    DROP CONSTRAINT media_job_desired_target_stream_audio_constraints,
    ADD CONSTRAINT media_job_desired_target_stream_audio_constraints CHECK (
        (
            stream_kind = 'audio'
            AND (audio_bitrate_bps IS NULL OR audio_bitrate_bps > 0)
            AND (audio_sample_rate_hz IS NULL OR audio_sample_rate_hz > 0)
            AND (
                audio_loudness_profile IS NULL
                OR audio_loudness_profile IN ('dialog-normalized')
            )
            AND (
                audio_dynamic_range IS NULL
                OR audio_dynamic_range IN ('preserve', 'speech')
            )
        )
        OR (
            stream_kind <> 'audio'
            AND audio_bitrate_bps IS NULL
            AND audio_sample_rate_hz IS NULL
            AND audio_loudness_profile IS NULL
            AND audio_dynamic_range IS NULL
        )
    );

CREATE FUNCTION media_desired_target_stream_append_v5(
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
    semantic_role_value TEXT;
    subtitle_placement_value TEXT;
    image_subtitle_action_value TEXT;
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
    semantic_role_value := NULLIF(lower(btrim(semantic_role_input)), '');
    subtitle_placement_value := NULLIF(lower(btrim(subtitle_placement_input)), '');
    image_subtitle_action_value := NULLIF(lower(btrim(image_subtitle_action_input)), '');
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
    ELSIF audio_bitrate_bps_input IS NOT NULL
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
            channel_layout,
            audio_bitrate_bps,
            audio_sample_rate_hz,
            audio_loudness_profile,
            audio_dynamic_range
        )
        VALUES (
            target_stream_id,
            channel_count_input,
            NULLIF(lower(btrim(channel_layout_input)), ''),
            audio_bitrate_bps_input,
            audio_sample_rate_hz_input,
            audio_loudness_profile_value,
            audio_dynamic_range_value
        );
    ELSIF channel_count_input IS NOT NULL OR NULLIF(btrim(channel_layout_input), '') IS NOT NULL THEN
        RAISE EXCEPTION 'audio shape assigned to non-audio target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION media_desired_target_stream_append_v4(
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
    SELECT media_desired_target_stream_append_v5(
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
        NULL,
        NULL,
        video_profile_input,
        video_level_input,
        video_bitrate_bps_input,
        color_primaries_input,
        color_transfer_input,
        color_space_input,
        hdr_format_input,
        title_input,
        default_disposition_input,
        forced_disposition_input,
        subtitle_placement_input,
        image_subtitle_action_input
    );
$$;

CREATE FUNCTION media_desired_target_stream_list_v5(
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

CREATE OR REPLACE FUNCTION media_job_desired_target_audio_constraints_snapshot_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    IF NEW.stream_kind = 'audio' THEN
        SELECT audio.audio_bitrate_bps,
               audio.audio_sample_rate_hz,
               audio.audio_loudness_profile,
               audio.audio_dynamic_range
          INTO NEW.audio_bitrate_bps,
               NEW.audio_sample_rate_hz,
               NEW.audio_loudness_profile,
               NEW.audio_dynamic_range
          FROM media_job job
          JOIN media_desired_target_stream target_stream
            ON target_stream.media_desired_target_profile_id = job.intent_desired_target_profile_id
           AND lower(target_stream.stream_key) = lower(NEW.stream_key)
          JOIN media_desired_target_audio_stream audio
            ON audio.media_desired_target_stream_id = target_stream.media_desired_target_stream_id
         WHERE job.media_job_id = NEW.media_job_id;
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION media_job_desired_target_stream_list_v5(
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
