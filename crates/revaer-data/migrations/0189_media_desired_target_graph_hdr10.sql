CREATE FUNCTION media_desired_target_graph_page_v2(limit_input INT)
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
          SELECT * FROM media_desired_target_stream_list_v7(
              target.media_desired_target_profile_public_id
          ) LIMIT 1025
      ) stream ON TRUE
     ORDER BY lower(target.target_key), target.version DESC, stream.sort_order, stream.stream_key;
END;
$$;
