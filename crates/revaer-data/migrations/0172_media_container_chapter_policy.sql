ALTER TABLE media_desired_target_container
    ADD COLUMN container_chapter_policy TEXT NOT NULL DEFAULT 'preserve',
    ADD CONSTRAINT media_desired_target_container_chapter_policy_known CHECK (
        container_chapter_policy IN ('preserve', 'strip')
    );

ALTER TABLE media_job
    ADD COLUMN intent_desired_container_chapter_policy TEXT;

UPDATE media_job
   SET intent_desired_container_chapter_policy = 'preserve'
 WHERE intent_desired_target_profile_id IS NOT NULL
   AND intent_desired_container_chapter_policy IS NULL;

ALTER TABLE media_job
    DROP CONSTRAINT media_job_intent_desired_target_complete,
    ADD CONSTRAINT media_job_intent_desired_target_complete CHECK (
        (intent_desired_target_profile_id IS NULL
            AND intent_desired_target_key IS NULL
            AND intent_desired_target_version IS NULL
            AND intent_desired_container_format IS NULL
            AND intent_desired_container_metadata_policy IS NULL
            AND intent_desired_container_chapter_policy IS NULL)
        OR
        (intent_desired_target_profile_id IS NOT NULL
            AND btrim(intent_desired_target_key) <> ''
            AND intent_desired_target_version > 0
            AND btrim(intent_desired_container_format) <> ''
            AND intent_desired_container_metadata_policy IN ('preserve', 'strip')
            AND intent_desired_container_chapter_policy IN ('preserve', 'strip'))
    );

CREATE FUNCTION media_desired_target_create_v3(
    actor_public_id_input UUID,
    target_key_input TEXT,
    version_input INT,
    display_name_input TEXT,
    container_format_input TEXT,
    container_metadata_policy_input TEXT,
    container_chapter_policy_input TEXT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    target_id BIGINT;
    target_public_id UUID;
    chapter_policy_value TEXT;
BEGIN
    chapter_policy_value := lower(COALESCE(NULLIF(btrim(container_chapter_policy_input), ''), 'preserve'));

    IF chapter_policy_value NOT IN ('preserve', 'strip') THEN
        RAISE EXCEPTION 'invalid desired target'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_invalid';
    END IF;

    target_public_id := media_desired_target_create_v2(
        actor_public_id_input,
        target_key_input,
        version_input,
        display_name_input,
        container_format_input,
        container_metadata_policy_input
    );

    SELECT media_desired_target_profile_id
      INTO target_id
      FROM media_desired_target_profile
     WHERE media_desired_target_profile_public_id = target_public_id;

    IF target_id IS NULL THEN
        RAISE EXCEPTION 'desired target not found after create'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    UPDATE media_desired_target_container
       SET container_chapter_policy = chapter_policy_value
     WHERE media_desired_target_profile_id = target_id;

    RETURN target_public_id;
END;
$$;

CREATE FUNCTION media_desired_target_list_v3()
RETURNS TABLE (
    media_desired_target_profile_public_id UUID,
    target_key TEXT,
    version INT,
    display_name TEXT,
    container_format TEXT,
    container_metadata_policy TEXT,
    container_chapter_policy TEXT
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT target.media_desired_target_profile_public_id,
           target.target_key,
           target.version,
           target.display_name,
           container.container_format,
           container.container_metadata_policy,
           container.container_chapter_policy
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.enabled
     ORDER BY lower(target.target_key), target.version DESC;
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
               container.container_chapter_policy
          INTO desired_container_format,
               desired_container_metadata_policy,
               desired_container_chapter_policy
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
        intent_desired_container_metadata_policy, intent_desired_container_chapter_policy,
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
        desired_container_metadata_policy, desired_container_chapter_policy,
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

CREATE FUNCTION media_job_worker_claim_next_v4()
RETURNS TABLE (
    media_job_public_id UUID,
    media_profile_public_id UUID,
    source_path TEXT,
    output_path TEXT,
    dry_run BOOLEAN,
    source_root TEXT,
    output_root TEXT,
    compatibility_target_key TEXT,
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
    unmatched_stream_policy TEXT,
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
           updated.intent_compatibility_target_key, updated.intent_policy_key,
           updated.intent_target_video_codec, updated.intent_target_audio_codec,
           updated.intent_target_audio_channels, updated.intent_target_audio_channel_layout,
           updated.intent_target_subtitle_policy, updated.intent_policy_video_intent,
           updated.intent_desired_target_key, updated.intent_desired_target_version,
           updated.intent_desired_container_format,
           updated.intent_desired_container_metadata_policy,
           updated.intent_desired_container_chapter_policy,
           updated.intent_unmatched_stream_policy,
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
