ALTER TABLE media_policy_profile
    ADD COLUMN unmatched_video_action TEXT NOT NULL DEFAULT 'fail',
    ADD COLUMN unmatched_audio_action TEXT NOT NULL DEFAULT 'preserve',
    ADD COLUMN unmatched_subtitle_action TEXT NOT NULL DEFAULT 'preserve',
    ADD COLUMN unmatched_attachment_action TEXT NOT NULL DEFAULT 'preserve',
    ADD COLUMN unmatched_data_action TEXT NOT NULL DEFAULT 'remove',
    ADD CONSTRAINT media_policy_profile_unmatched_video_action_known CHECK (
        unmatched_video_action IN ('remove', 'preserve', 'fail')
    ),
    ADD CONSTRAINT media_policy_profile_unmatched_audio_action_known CHECK (
        unmatched_audio_action IN ('remove', 'preserve', 'fail')
    ),
    ADD CONSTRAINT media_policy_profile_unmatched_subtitle_action_known CHECK (
        unmatched_subtitle_action IN ('remove', 'preserve', 'fail')
    ),
    ADD CONSTRAINT media_policy_profile_unmatched_attachment_action_known CHECK (
        unmatched_attachment_action IN ('remove', 'preserve', 'fail')
    ),
    ADD CONSTRAINT media_policy_profile_unmatched_data_action_known CHECK (
        unmatched_data_action IN ('remove', 'preserve', 'fail')
    );

ALTER TABLE media_job
    ADD COLUMN intent_unmatched_video_action TEXT NOT NULL DEFAULT 'fail',
    ADD COLUMN intent_unmatched_audio_action TEXT NOT NULL DEFAULT 'preserve',
    ADD COLUMN intent_unmatched_subtitle_action TEXT NOT NULL DEFAULT 'preserve',
    ADD COLUMN intent_unmatched_attachment_action TEXT NOT NULL DEFAULT 'preserve',
    ADD COLUMN intent_unmatched_data_action TEXT NOT NULL DEFAULT 'remove',
    ADD CONSTRAINT media_job_intent_unmatched_video_action_known CHECK (
        intent_unmatched_video_action IN ('remove', 'preserve', 'fail')
    ),
    ADD CONSTRAINT media_job_intent_unmatched_audio_action_known CHECK (
        intent_unmatched_audio_action IN ('remove', 'preserve', 'fail')
    ),
    ADD CONSTRAINT media_job_intent_unmatched_subtitle_action_known CHECK (
        intent_unmatched_subtitle_action IN ('remove', 'preserve', 'fail')
    ),
    ADD CONSTRAINT media_job_intent_unmatched_attachment_action_known CHECK (
        intent_unmatched_attachment_action IN ('remove', 'preserve', 'fail')
    ),
    ADD CONSTRAINT media_job_intent_unmatched_data_action_known CHECK (
        intent_unmatched_data_action IN ('remove', 'preserve', 'fail')
    );

UPDATE media_policy_profile
   SET unmatched_video_action = CASE
           WHEN unmatched_stream_policy = 'remove' THEN 'fail'
           ELSE replace(unmatched_stream_policy, 'reject', 'fail')
       END,
       unmatched_audio_action = CASE
           WHEN unmatched_stream_policy = 'remove' THEN 'preserve'
           ELSE replace(unmatched_stream_policy, 'reject', 'fail')
       END,
       unmatched_subtitle_action = CASE
           WHEN unmatched_stream_policy = 'remove' THEN 'preserve'
           ELSE replace(unmatched_stream_policy, 'reject', 'fail')
       END,
       unmatched_attachment_action = CASE
           WHEN unmatched_stream_policy = 'remove' THEN 'preserve'
           ELSE replace(unmatched_stream_policy, 'reject', 'fail')
       END,
       unmatched_data_action = CASE
           WHEN unmatched_stream_policy = 'remove' THEN 'remove'
           ELSE replace(unmatched_stream_policy, 'reject', 'fail')
       END
 WHERE unmatched_stream_policy IS NOT NULL;

UPDATE media_job
   SET intent_unmatched_video_action = replace(intent_unmatched_stream_policy, 'reject', 'fail'),
       intent_unmatched_audio_action = replace(intent_unmatched_stream_policy, 'reject', 'fail'),
       intent_unmatched_subtitle_action = replace(intent_unmatched_stream_policy, 'reject', 'fail'),
       intent_unmatched_attachment_action = replace(intent_unmatched_stream_policy, 'reject', 'fail'),
       intent_unmatched_data_action = replace(intent_unmatched_stream_policy, 'reject', 'fail')
 WHERE intent_unmatched_stream_policy IS NOT NULL;

CREATE OR REPLACE FUNCTION media_policy_profile_list_v2()
RETURNS TABLE (
    policy_key TEXT,
    version INT,
    display_name TEXT,
    video_intent TEXT,
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
    verification_playback_probe BOOLEAN
)
LANGUAGE sql
STABLE
AS $$
    SELECT profile.policy_key,
           profile.version,
           profile.display_name,
           profile.video_intent,
           profile.unmatched_video_action,
           profile.unmatched_audio_action,
           profile.unmatched_subtitle_action,
           profile.unmatched_attachment_action,
           profile.unmatched_data_action,
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

CREATE OR REPLACE FUNCTION media_policy_profile_upsert_v2(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    display_name_input TEXT,
    video_intent_input TEXT,
    unmatched_video_action_input TEXT,
    unmatched_audio_action_input TEXT,
    unmatched_subtitle_action_input TEXT,
    unmatched_attachment_action_input TEXT,
    unmatched_data_action_input TEXT,
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
    unmatched_video_action_value TEXT;
    unmatched_audio_action_value TEXT;
    unmatched_subtitle_action_value TEXT;
    unmatched_attachment_action_value TEXT;
    unmatched_data_action_value TEXT;
    legacy_unmatched_policy_value TEXT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    version_value := COALESCE(version_input, 1);
    unmatched_video_action_value := lower(COALESCE(NULLIF(btrim(unmatched_video_action_input), ''), 'fail'));
    unmatched_audio_action_value := lower(COALESCE(NULLIF(btrim(unmatched_audio_action_input), ''), 'preserve'));
    unmatched_subtitle_action_value := lower(COALESCE(NULLIF(btrim(unmatched_subtitle_action_input), ''), 'preserve'));
    unmatched_attachment_action_value := lower(COALESCE(NULLIF(btrim(unmatched_attachment_action_input), ''), 'preserve'));
    unmatched_data_action_value := lower(COALESCE(NULLIF(btrim(unmatched_data_action_input), ''), 'remove'));

    IF unmatched_video_action_value NOT IN ('remove', 'preserve', 'fail')
       OR unmatched_audio_action_value NOT IN ('remove', 'preserve', 'fail')
       OR unmatched_subtitle_action_value NOT IN ('remove', 'preserve', 'fail')
       OR unmatched_attachment_action_value NOT IN ('remove', 'preserve', 'fail')
       OR unmatched_data_action_value NOT IN ('remove', 'preserve', 'fail') THEN
        RAISE EXCEPTION 'invalid media unmatched stream action'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_unmatched_action_invalid';
    END IF;

    IF unmatched_video_action_value = unmatched_audio_action_value
       AND unmatched_video_action_value = unmatched_subtitle_action_value
       AND unmatched_video_action_value = unmatched_attachment_action_value
       AND unmatched_video_action_value = unmatched_data_action_value THEN
        legacy_unmatched_policy_value := CASE WHEN unmatched_video_action_value = 'fail' THEN 'reject' ELSE unmatched_video_action_value END;
    ELSE
        legacy_unmatched_policy_value := 'reject';
    END IF;

    RETURN QUERY
    INSERT INTO media_policy_profile (
        policy_key,
        version,
        display_name,
        video_intent,
        unmatched_stream_policy,
        unmatched_video_action,
        unmatched_audio_action,
        unmatched_subtitle_action,
        unmatched_attachment_action,
        unmatched_data_action,
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
        legacy_unmatched_policy_value,
        unmatched_video_action_value,
        unmatched_audio_action_value,
        unmatched_subtitle_action_value,
        unmatched_attachment_action_value,
        unmatched_data_action_value,
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
        unmatched_stream_policy = EXCLUDED.unmatched_stream_policy,
        unmatched_video_action = EXCLUDED.unmatched_video_action,
        unmatched_audio_action = EXCLUDED.unmatched_audio_action,
        unmatched_subtitle_action = EXCLUDED.unmatched_subtitle_action,
        unmatched_attachment_action = EXCLUDED.unmatched_attachment_action,
        unmatched_data_action = EXCLUDED.unmatched_data_action,
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
        media_policy_profile.unmatched_video_action,
        media_policy_profile.unmatched_audio_action,
        media_policy_profile.unmatched_subtitle_action,
        media_policy_profile.unmatched_attachment_action,
        media_policy_profile.unmatched_data_action,
        media_policy_profile.verification_strictness,
        media_policy_profile.verification_duration_tolerance_millis,
        media_policy_profile.verification_mux_validation,
        media_policy_profile.verification_decode_all_streams,
        media_policy_profile.verification_keyframe_seek,
        media_policy_profile.verification_playback_probe;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_unmatched_stream_actions_fill_v1()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    policy_row media_policy_profile%ROWTYPE;
BEGIN
    IF NEW.intent_policy_profile_id IS NOT NULL THEN
        SELECT *
          INTO policy_row
          FROM media_policy_profile
         WHERE media_policy_profile_id = NEW.intent_policy_profile_id;
    END IF;

    IF policy_row.media_policy_profile_id IS NOT NULL THEN
        NEW.intent_unmatched_video_action := policy_row.unmatched_video_action;
        NEW.intent_unmatched_audio_action := policy_row.unmatched_audio_action;
        NEW.intent_unmatched_subtitle_action := policy_row.unmatched_subtitle_action;
        NEW.intent_unmatched_attachment_action := policy_row.unmatched_attachment_action;
        NEW.intent_unmatched_data_action := policy_row.unmatched_data_action;
        RETURN NEW;
    END IF;

    NEW.intent_unmatched_video_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_video_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'fail')));
    NEW.intent_unmatched_audio_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_audio_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'preserve')));
    NEW.intent_unmatched_subtitle_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_subtitle_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'preserve')));
    NEW.intent_unmatched_attachment_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_attachment_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'preserve')));
    NEW.intent_unmatched_data_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_data_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'remove')));
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS media_job_unmatched_stream_actions_fill_v1 ON media_job;

CREATE TRIGGER media_job_unmatched_stream_actions_fill_v1
BEFORE INSERT ON media_job
FOR EACH ROW
EXECUTE FUNCTION media_job_unmatched_stream_actions_fill_v1();

CREATE FUNCTION media_job_worker_claim_next_v6()
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
