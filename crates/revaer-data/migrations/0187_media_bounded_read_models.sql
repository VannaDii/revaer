CREATE OR REPLACE FUNCTION media_job_recent_page_v1(
    limit_input INT,
    cursor_queued_at_input TIMESTAMPTZ DEFAULT NULL,
    cursor_public_id_input UUID DEFAULT NULL,
    media_profile_public_id_input UUID DEFAULT NULL
)
RETURNS TABLE (
    media_job_public_id UUID, media_profile_public_id UUID, source_path TEXT,
    output_path TEXT, status_text TEXT, dry_run BOOLEAN, queued_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ, completed_at TIMESTAMPTZ, last_error TEXT,
    operation_count BIGINT, violation_count BIGINT, plan_reason_count BIGINT,
    verification_check_count BIGINT, artifact_count BIGINT, compact_audit_count BIGINT
)
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF limit_input < 1 OR limit_input > 100 THEN
        RAISE EXCEPTION 'recent media job page limit is outside 1..100'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_recent_limit_invalid';
    END IF;
    IF (cursor_queued_at_input IS NULL) <> (cursor_public_id_input IS NULL) THEN
        RAISE EXCEPTION 'recent media job cursor is incomplete'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_recent_cursor_invalid';
    END IF;
    RETURN QUERY
    WITH page AS MATERIALIZED (
        SELECT job.media_job_id, job.media_job_public_id, profile.media_profile_public_id,
               job.source_path, job.output_path, job.status::TEXT AS status_text, job.dry_run,
               job.queued_at, job.started_at, job.completed_at, job.last_error
          FROM media_job job
          JOIN media_profile profile ON profile.media_profile_id = job.media_profile_id
         WHERE (media_profile_public_id_input IS NULL
                OR profile.media_profile_public_id = media_profile_public_id_input)
           AND (cursor_queued_at_input IS NULL
                OR (job.queued_at, job.media_job_public_id)
                   < (cursor_queued_at_input, cursor_public_id_input))
         ORDER BY job.queued_at DESC, job.media_job_public_id DESC
         LIMIT limit_input + 1
    ), diagnostics AS (
        SELECT child.media_job_id, 'operation'::TEXT AS kind FROM media_job_operation child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'violation' FROM media_job_violation child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'plan_reason' FROM media_job_plan_reason child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'verification_check' FROM media_job_verification_check child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'artifact' FROM media_job_artifact child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'compact_audit' FROM media_job_compact_audit child JOIN page USING (media_job_id)
    ), counts AS (
        SELECT diagnostics.media_job_id,
               count(*) FILTER (WHERE kind = 'operation') AS operations,
               count(*) FILTER (WHERE kind = 'violation') AS violations,
               count(*) FILTER (WHERE kind = 'plan_reason') AS plan_reasons,
               count(*) FILTER (WHERE kind = 'verification_check') AS verification_checks,
               count(*) FILTER (WHERE kind = 'artifact') AS artifacts,
               count(*) FILTER (WHERE kind = 'compact_audit') AS compact_audits
          FROM diagnostics GROUP BY diagnostics.media_job_id
    )
    SELECT page.media_job_public_id, page.media_profile_public_id, page.source_path,
           page.output_path, page.status_text, page.dry_run, page.queued_at, page.started_at,
           page.completed_at, page.last_error, coalesce(counts.operations, 0),
           coalesce(counts.violations, 0), coalesce(counts.plan_reasons, 0),
           coalesce(counts.verification_checks, 0), coalesce(counts.artifacts, 0),
           coalesce(counts.compact_audits, 0)
      FROM page LEFT JOIN counts USING (media_job_id)
     ORDER BY page.queued_at DESC, page.media_job_public_id DESC;
END;
$$;

CREATE FUNCTION media_job_worker_claim_next_v4()
RETURNS TABLE (
    media_job_public_id UUID, media_profile_public_id UUID, source_path TEXT, output_path TEXT,
    dry_run BOOLEAN, source_root TEXT, output_root TEXT,
    source_identity TEXT, source_size_bytes BIGINT, source_modified_ns BIGINT,
    source_changed_ns BIGINT, source_sha256 TEXT, compatibility_target_key TEXT,
    policy_key TEXT, target_video_codec TEXT, target_audio_codec TEXT,
    target_audio_channels INT, target_audio_channel_layout TEXT, target_subtitle_policy TEXT,
    policy_video_intent TEXT, desired_target_key TEXT, desired_target_version INT,
    desired_container_format TEXT, unmatched_stream_policy TEXT, verification_strictness TEXT,
    verification_duration_tolerance_millis BIGINT, verification_mux_validation BOOLEAN,
    verification_decode_all_streams BOOLEAN, verification_keyframe_seek BOOLEAN,
    verification_playback_probe BOOLEAN, attempt_number INT, claim_generation BIGINT,
    cancel_generation BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id, job.current_attempt_id
          FROM media_job job
          JOIN media_job_attempt attempt
            ON attempt.media_job_attempt_id = job.current_attempt_id
         WHERE job.status = media_job_status_queued_v1()
           AND attempt.status = media_job_status_queued_v1()
           AND job.intent_source_identity IS NOT NULL
           AND job.intent_source_size_bytes IS NOT NULL
           AND job.intent_source_modified_ns IS NOT NULL
           AND job.intent_source_changed_ns IS NOT NULL
           AND job.intent_source_sha256 IS NOT NULL
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE OF job, attempt SKIP LOCKED
         LIMIT 1
    ),
    updated_attempt AS (
        UPDATE media_job_attempt attempt
           SET status = media_job_status_running_v1(),
               claimed_at = now(),
               heartbeat_at = now(),
               cancel_generation_at_claim = job.cancel_generation
          FROM claimed
          JOIN media_job job ON job.media_job_id = claimed.media_job_id
         WHERE attempt.media_job_attempt_id = claimed.current_attempt_id
        RETURNING attempt.*
    ),
    updated_job AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = now(),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM updated_attempt attempt
         WHERE job.media_job_id = attempt.media_job_id
        RETURNING job.*
    )
    SELECT job.media_job_public_id, profile.media_profile_public_id,
           job.source_path, job.output_path, job.dry_run,
           job.intent_source_root, job.intent_output_root,
           job.intent_source_identity, job.intent_source_size_bytes,
           job.intent_source_modified_ns, job.intent_source_changed_ns,
           job.intent_source_sha256,
           job.intent_compatibility_target_key, job.intent_policy_key,
           job.intent_target_video_codec, job.intent_target_audio_codec,
           job.intent_target_audio_channels, job.intent_target_audio_channel_layout,
           job.intent_target_subtitle_policy, job.intent_policy_video_intent,
           job.intent_desired_target_key, job.intent_desired_target_version,
           job.intent_desired_container_format, job.intent_unmatched_stream_policy,
           job.intent_verification_strictness,
           job.intent_verification_duration_tolerance_millis,
           job.intent_verification_mux_validation,
           job.intent_verification_decode_all_streams,
           job.intent_verification_keyframe_seek,
           job.intent_verification_playback_probe,
           attempt.attempt_number, attempt.claim_generation,
           job.cancel_generation
      FROM updated_job job
      JOIN updated_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_profile profile ON profile.media_profile_id = job.media_profile_id;
END;
$$;

CREATE OR REPLACE FUNCTION media_desired_target_graph_page_v1(limit_input INT)
RETURNS TABLE (
    media_desired_target_profile_public_id UUID, target_key TEXT, version INT,
    display_name TEXT, container_format TEXT, stream_key TEXT, stream_kind TEXT,
    semantic_role TEXT, language_code TEXT, optional BOOLEAN, sort_order INT, codec TEXT,
    channel_count INT, channel_layout TEXT, audio_bitrate_bps INT, audio_sample_rate_hz INT,
    audio_loudness_profile TEXT, audio_dynamic_range TEXT, video_profile TEXT, video_level TEXT,
    video_bitrate_bps INT, color_primaries TEXT, color_transfer TEXT, color_space TEXT,
    hdr_format TEXT, title TEXT, default_disposition BOOLEAN, forced_disposition BOOLEAN,
    subtitle_placement TEXT, image_subtitle_action TEXT
)
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF limit_input < 1 OR limit_input > 128 THEN
        RAISE EXCEPTION 'desired target page limit is outside 1..128'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_page_limit_invalid';
    END IF;
    RETURN QUERY
    WITH target_page AS MATERIALIZED (
        SELECT * FROM media_desired_target_list_v1() LIMIT limit_input
    )
    SELECT target.*, stream.*
      FROM target_page target
      JOIN LATERAL (
          SELECT * FROM media_desired_target_stream_list_v5(
              target.media_desired_target_profile_public_id
          ) LIMIT 1025
      ) stream ON TRUE
     ORDER BY lower(target.target_key), target.version DESC, stream.sort_order, stream.stream_key;
END;
$$;
