ALTER TABLE media_policy_profile
    ADD COLUMN verification_strictness TEXT NOT NULL DEFAULT 'balanced',
    ADD COLUMN verification_duration_tolerance_millis BIGINT NOT NULL DEFAULT 1000,
    ADD COLUMN verification_mux_validation BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN verification_decode_all_streams BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN verification_keyframe_seek BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN verification_playback_probe BOOLEAN NOT NULL DEFAULT FALSE,
    ADD CONSTRAINT media_policy_verification_strictness_known CHECK (
        verification_strictness IN ('strict', 'balanced', 'fast')
    ),
    ADD CONSTRAINT media_policy_verification_duration_tolerance_bounded CHECK (
        verification_duration_tolerance_millis BETWEEN 0 AND 60000
    ),
    ADD CONSTRAINT media_policy_verification_strict_checks CHECK (
        verification_strictness <> 'strict'
        OR (
            verification_mux_validation
            AND verification_decode_all_streams
            AND verification_keyframe_seek
            AND verification_playback_probe
        )
    ),
    ADD CONSTRAINT media_policy_verification_fast_checks CHECK (
        verification_strictness <> 'fast'
        OR (
            NOT verification_decode_all_streams
            AND NOT verification_keyframe_seek
            AND NOT verification_playback_probe
        )
    );

UPDATE media_policy_profile
   SET verification_strictness = CASE
           WHEN lower(policy_key) IN ('safe_dry_run', 'archival') THEN 'strict'
           ELSE 'balanced'
       END,
       verification_duration_tolerance_millis = CASE
           WHEN lower(policy_key) IN ('safe_dry_run', 'archival') THEN 100
           ELSE 1000
       END,
       verification_mux_validation = TRUE,
       verification_decode_all_streams = TRUE,
       verification_keyframe_seek = TRUE,
       verification_playback_probe = lower(policy_key) IN ('safe_dry_run', 'archival');

ALTER TABLE media_job
    ADD COLUMN intent_verification_strictness TEXT NOT NULL DEFAULT 'balanced',
    ADD COLUMN intent_verification_duration_tolerance_millis BIGINT NOT NULL DEFAULT 1000,
    ADD COLUMN intent_verification_mux_validation BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN intent_verification_decode_all_streams BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN intent_verification_keyframe_seek BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN intent_verification_playback_probe BOOLEAN NOT NULL DEFAULT FALSE,
    ADD CONSTRAINT media_job_verification_strictness_known CHECK (
        intent_verification_strictness IN ('strict', 'balanced', 'fast')
    ),
    ADD CONSTRAINT media_job_verification_duration_tolerance_bounded CHECK (
        intent_verification_duration_tolerance_millis BETWEEN 0 AND 60000
    ),
    ADD CONSTRAINT media_job_verification_strict_checks CHECK (
        intent_verification_strictness <> 'strict'
        OR (
            intent_verification_mux_validation
            AND intent_verification_decode_all_streams
            AND intent_verification_keyframe_seek
            AND intent_verification_playback_probe
        )
    ),
    ADD CONSTRAINT media_job_verification_fast_checks CHECK (
        intent_verification_strictness <> 'fast'
        OR (
            NOT intent_verification_decode_all_streams
            AND NOT intent_verification_keyframe_seek
            AND NOT intent_verification_playback_probe
        )
    );

UPDATE media_job job
   SET intent_verification_strictness = policy.verification_strictness,
       intent_verification_duration_tolerance_millis = policy.verification_duration_tolerance_millis,
       intent_verification_mux_validation = policy.verification_mux_validation,
       intent_verification_decode_all_streams = policy.verification_decode_all_streams,
       intent_verification_keyframe_seek = policy.verification_keyframe_seek,
       intent_verification_playback_probe = policy.verification_playback_probe
  FROM media_policy_profile policy
 WHERE policy.media_policy_profile_id = job.intent_policy_profile_id;

DROP FUNCTION media_policy_profile_list_v1();

CREATE FUNCTION media_policy_profile_list_v1()
RETURNS TABLE (
    policy_key TEXT,
    version INT,
    display_name TEXT,
    video_intent TEXT,
    verification_strictness TEXT,
    verification_duration_tolerance_millis BIGINT,
    verification_mux_validation BOOLEAN,
    verification_decode_all_streams BOOLEAN,
    verification_keyframe_seek BOOLEAN,
    verification_playback_probe BOOLEAN
)
LANGUAGE sql
STABLE
AS $$
    SELECT profile.policy_key,
           profile.version,
           profile.display_name,
           profile.video_intent,
           profile.verification_strictness,
           profile.verification_duration_tolerance_millis,
           profile.verification_mux_validation,
           profile.verification_decode_all_streams,
           profile.verification_keyframe_seek,
           profile.verification_playback_probe
      FROM media_policy_profile AS profile
     WHERE profile.enabled
     ORDER BY lower(profile.policy_key), profile.version DESC;
$$;

DROP FUNCTION media_policy_profile_upsert_v1(UUID, TEXT, INT, TEXT, TEXT);

CREATE FUNCTION media_policy_profile_upsert_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    display_name_input TEXT,
    video_intent_input TEXT,
    verification_strictness_input TEXT,
    verification_duration_tolerance_millis_input BIGINT,
    verification_mux_validation_input BOOLEAN,
    verification_decode_all_streams_input BOOLEAN,
    verification_keyframe_seek_input BOOLEAN,
    verification_playback_probe_input BOOLEAN
)
RETURNS TABLE (
    policy_key TEXT,
    version INT,
    display_name TEXT,
    video_intent TEXT,
    verification_strictness TEXT,
    verification_duration_tolerance_millis BIGINT,
    verification_mux_validation BOOLEAN,
    verification_decode_all_streams BOOLEAN,
    verification_keyframe_seek BOOLEAN,
    verification_playback_probe BOOLEAN
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
#variable_conflict use_column
DECLARE
    actor_id BIGINT;
    version_value INT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    version_value := COALESCE(version_input, 1);

    RETURN QUERY
    INSERT INTO media_policy_profile (
        policy_key,
        version,
        display_name,
        video_intent,
        verification_strictness,
        verification_duration_tolerance_millis,
        verification_mux_validation,
        verification_decode_all_streams,
        verification_keyframe_seek,
        verification_playback_probe,
        enabled,
        updated_at
    )
    VALUES (
        btrim(policy_key_input),
        version_value,
        btrim(display_name_input),
        lower(btrim(video_intent_input)),
        lower(btrim(verification_strictness_input)),
        verification_duration_tolerance_millis_input,
        verification_mux_validation_input,
        verification_decode_all_streams_input,
        verification_keyframe_seek_input,
        verification_playback_probe_input,
        TRUE,
        now()
    )
    ON CONFLICT (lower(policy_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_intent = EXCLUDED.video_intent,
        verification_strictness = EXCLUDED.verification_strictness,
        verification_duration_tolerance_millis = EXCLUDED.verification_duration_tolerance_millis,
        verification_mux_validation = EXCLUDED.verification_mux_validation,
        verification_decode_all_streams = EXCLUDED.verification_decode_all_streams,
        verification_keyframe_seek = EXCLUDED.verification_keyframe_seek,
        verification_playback_probe = EXCLUDED.verification_playback_probe,
        enabled = TRUE,
        updated_at = now()
    RETURNING
        media_policy_profile.policy_key,
        media_policy_profile.version,
        media_policy_profile.display_name,
        media_policy_profile.video_intent,
        media_policy_profile.verification_strictness,
        media_policy_profile.verification_duration_tolerance_millis,
        media_policy_profile.verification_mux_validation,
        media_policy_profile.verification_decode_all_streams,
        media_policy_profile.verification_keyframe_seek,
        media_policy_profile.verification_playback_probe;
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
            optional, sort_order, codec, channel_count, channel_layout, title,
            default_disposition, forced_disposition
        )
        SELECT media_job_id_out, stream.stream_key, stream.stream_kind,
               stream.semantic_role, stream.language_code, stream.optional,
               stream.sort_order, stream.codec, audio.channel_count,
               audio.channel_layout, stream.title, stream.default_disposition,
               stream.forced_disposition
          FROM media_desired_target_stream stream
          LEFT JOIN media_desired_target_audio_stream audio
            ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
         WHERE stream.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
         ORDER BY stream.sort_order;
    END IF;

    RETURN media_job_public_id_out;
END;
$$;

DROP FUNCTION media_job_worker_claim_next_v2();

CREATE FUNCTION media_job_worker_claim_next_v2()
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
           updated.intent_desired_container_format, updated.intent_unmatched_stream_policy,
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
