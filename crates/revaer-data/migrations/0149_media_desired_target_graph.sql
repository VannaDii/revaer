CREATE TABLE media_desired_target_profile (
    media_desired_target_profile_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_desired_target_profile_public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    target_key TEXT NOT NULL,
    version INT NOT NULL,
    display_name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_by_user_id BIGINT NOT NULL REFERENCES app_user(user_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_desired_target_profile_key_nonempty CHECK (btrim(target_key) <> ''),
    CONSTRAINT media_desired_target_profile_display_nonempty CHECK (btrim(display_name) <> ''),
    CONSTRAINT media_desired_target_profile_version_positive CHECK (version > 0)
);

CREATE UNIQUE INDEX uq_media_desired_target_profile_key_version
    ON media_desired_target_profile (lower(target_key), version);

CREATE TABLE media_desired_target_container (
    media_desired_target_profile_id BIGINT PRIMARY KEY
        REFERENCES media_desired_target_profile(media_desired_target_profile_id) ON DELETE CASCADE,
    container_format TEXT NOT NULL,
    CONSTRAINT media_desired_target_container_format_nonempty CHECK (btrim(container_format) <> '')
);

CREATE TABLE media_desired_target_stream (
    media_desired_target_stream_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_desired_target_profile_id BIGINT NOT NULL
        REFERENCES media_desired_target_profile(media_desired_target_profile_id) ON DELETE CASCADE,
    stream_key TEXT NOT NULL,
    stream_kind TEXT NOT NULL,
    semantic_role TEXT,
    language_code TEXT,
    optional BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INT NOT NULL,
    codec TEXT NOT NULL,
    title TEXT,
    default_disposition BOOLEAN NOT NULL DEFAULT FALSE,
    forced_disposition BOOLEAN NOT NULL DEFAULT FALSE,
    CONSTRAINT media_desired_target_stream_key_nonempty CHECK (btrim(stream_key) <> ''),
    CONSTRAINT media_desired_target_stream_kind_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle', 'attachment', 'chapter')
    ),
    CONSTRAINT media_desired_target_stream_role_known CHECK (
        semantic_role IS NULL OR semantic_role IN (
            'primary', 'forced', 'commentary', 'descriptive_audio', 'sdh',
            'signs_songs', 'karaoke', 'unknown'
        )
    ),
    CONSTRAINT media_desired_target_stream_language_nonempty CHECK (
        language_code IS NULL OR btrim(language_code) <> ''
    ),
    CONSTRAINT media_desired_target_stream_sort_nonnegative CHECK (sort_order >= 0),
    CONSTRAINT media_desired_target_stream_codec_nonempty CHECK (btrim(codec) <> ''),
    CONSTRAINT media_desired_target_stream_forced_subtitle_only CHECK (
        NOT forced_disposition OR stream_kind = 'subtitle'
    )
);

CREATE UNIQUE INDEX uq_media_desired_target_stream_key
    ON media_desired_target_stream (media_desired_target_profile_id, lower(stream_key));

CREATE UNIQUE INDEX uq_media_desired_target_stream_order
    ON media_desired_target_stream (media_desired_target_profile_id, sort_order);

CREATE TABLE media_desired_target_audio_stream (
    media_desired_target_stream_id BIGINT PRIMARY KEY
        REFERENCES media_desired_target_stream(media_desired_target_stream_id) ON DELETE CASCADE,
    channel_count INT,
    channel_layout TEXT,
    CONSTRAINT media_desired_target_audio_channels_positive CHECK (
        channel_count IS NULL OR channel_count > 0
    ),
    CONSTRAINT media_desired_target_audio_layout_nonempty CHECK (
        channel_layout IS NULL OR btrim(channel_layout) <> ''
    )
);

ALTER TABLE media_profile
    ADD COLUMN desired_target_profile_id BIGINT
        REFERENCES media_desired_target_profile(media_desired_target_profile_id);

ALTER TABLE media_policy_profile
    ADD COLUMN unmatched_stream_policy TEXT NOT NULL DEFAULT 'remove',
    ADD CONSTRAINT media_policy_profile_unmatched_stream_policy_known CHECK (
        unmatched_stream_policy IN ('remove', 'preserve', 'reject')
    );

ALTER TABLE media_job
    ADD COLUMN intent_desired_target_profile_id BIGINT
        REFERENCES media_desired_target_profile(media_desired_target_profile_id),
    ADD COLUMN intent_desired_target_key TEXT,
    ADD COLUMN intent_desired_target_version INT,
    ADD COLUMN intent_desired_container_format TEXT,
    ADD COLUMN intent_unmatched_stream_policy TEXT,
    ADD CONSTRAINT media_job_intent_desired_target_complete CHECK (
        (intent_desired_target_profile_id IS NULL
            AND intent_desired_target_key IS NULL
            AND intent_desired_target_version IS NULL
            AND intent_desired_container_format IS NULL)
        OR
        (intent_desired_target_profile_id IS NOT NULL
            AND btrim(intent_desired_target_key) <> ''
            AND intent_desired_target_version > 0
            AND btrim(intent_desired_container_format) <> '')
    ),
    ADD CONSTRAINT media_job_intent_unmatched_stream_policy_known CHECK (
        intent_unmatched_stream_policy IS NULL
        OR intent_unmatched_stream_policy IN ('remove', 'preserve', 'reject')
    );

CREATE TABLE media_job_desired_target_stream (
    media_job_desired_target_stream_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    stream_key TEXT NOT NULL,
    stream_kind TEXT NOT NULL,
    semantic_role TEXT,
    language_code TEXT,
    optional BOOLEAN NOT NULL,
    sort_order INT NOT NULL,
    codec TEXT NOT NULL,
    channel_count INT,
    channel_layout TEXT,
    title TEXT,
    default_disposition BOOLEAN NOT NULL,
    forced_disposition BOOLEAN NOT NULL,
    CONSTRAINT media_job_desired_target_stream_key_nonempty CHECK (btrim(stream_key) <> ''),
    CONSTRAINT media_job_desired_target_stream_kind_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle', 'attachment', 'chapter')
    ),
    CONSTRAINT media_job_desired_target_stream_role_known CHECK (
        semantic_role IS NULL OR semantic_role IN (
            'primary', 'forced', 'commentary', 'descriptive_audio', 'sdh',
            'signs_songs', 'karaoke', 'unknown'
        )
    ),
    CONSTRAINT media_job_desired_target_stream_sort_nonnegative CHECK (sort_order >= 0),
    CONSTRAINT media_job_desired_target_stream_codec_nonempty CHECK (btrim(codec) <> ''),
    CONSTRAINT media_job_desired_target_stream_audio_shape CHECK (
        stream_kind = 'audio' OR (channel_count IS NULL AND channel_layout IS NULL)
    )
);

CREATE UNIQUE INDEX uq_media_job_desired_target_stream_key
    ON media_job_desired_target_stream (media_job_id, lower(stream_key));

CREATE UNIQUE INDEX uq_media_job_desired_target_stream_order
    ON media_job_desired_target_stream (media_job_id, sort_order);

CREATE OR REPLACE FUNCTION media_desired_target_create_v1(
    actor_public_id_input UUID,
    target_key_input TEXT,
    version_input INT,
    display_name_input TEXT,
    container_format_input TEXT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    target_id BIGINT;
    target_public_id UUID;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    IF NULLIF(btrim(target_key_input), '') IS NULL
       OR COALESCE(version_input, 0) <= 0
       OR NULLIF(btrim(display_name_input), '') IS NULL
       OR NULLIF(btrim(container_format_input), '') IS NULL THEN
        RAISE EXCEPTION 'invalid desired target'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_invalid';
    END IF;

    INSERT INTO media_desired_target_profile (
        target_key,
        version,
        display_name,
        created_by_user_id
    )
    VALUES (
        btrim(target_key_input),
        version_input,
        btrim(display_name_input),
        actor_id
    )
    RETURNING media_desired_target_profile_id, media_desired_target_profile_public_id
    INTO target_id, target_public_id;

    INSERT INTO media_desired_target_container (
        media_desired_target_profile_id,
        container_format
    )
    VALUES (target_id, lower(btrim(container_format_input)));

    RETURN target_public_id;
END;
$$;

CREATE OR REPLACE FUNCTION media_desired_target_stream_append_v1(
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
    title_input TEXT,
    default_disposition_input BOOLEAN,
    forced_disposition_input BOOLEAN
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
        forced_disposition
    )
    VALUES (
        target_id,
        btrim(stream_key_input),
        stream_kind_value,
        NULLIF(lower(btrim(semantic_role_input)), ''),
        NULLIF(lower(btrim(language_code_input)), ''),
        COALESCE(optional_input, FALSE),
        sort_order_input,
        lower(btrim(codec_input)),
        NULLIF(btrim(title_input), ''),
        COALESCE(default_disposition_input, FALSE),
        COALESCE(forced_disposition_input, FALSE)
    )
    RETURNING media_desired_target_stream_id INTO target_stream_id;

    IF stream_kind_value = 'audio' THEN
        INSERT INTO media_desired_target_audio_stream (
            media_desired_target_stream_id,
            channel_count,
            channel_layout
        )
        VALUES (
            target_stream_id,
            channel_count_input,
            NULLIF(lower(btrim(channel_layout_input)), '')
        );
    ELSIF channel_count_input IS NOT NULL OR NULLIF(btrim(channel_layout_input), '') IS NOT NULL THEN
        RAISE EXCEPTION 'audio shape assigned to non-audio target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION media_desired_target_list_v1()
RETURNS TABLE (
    media_desired_target_profile_public_id UUID,
    target_key TEXT,
    version INT,
    display_name TEXT,
    container_format TEXT
)
LANGUAGE sql
STABLE
AS $$
    SELECT target.media_desired_target_profile_public_id,
           target.target_key,
           target.version,
           target.display_name,
           container.container_format
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.enabled
     ORDER BY lower(target.target_key), target.version DESC;
$$;

CREATE OR REPLACE FUNCTION media_desired_target_stream_list_v1(
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
    title TEXT,
    default_disposition BOOLEAN,
    forced_disposition BOOLEAN
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
           stream.title,
           stream.default_disposition,
           stream.forced_disposition
      FROM media_desired_target_profile target
      JOIN media_desired_target_stream stream
        ON stream.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_audio_stream audio
        ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY stream.sort_order;
$$;

CREATE OR REPLACE FUNCTION media_profile_desired_target_set_v1(
    actor_public_id_input UUID,
    media_profile_public_id_input UUID,
    desired_target_key_input TEXT,
    desired_target_version_input INT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    target_id BIGINT;
    profile_public_id UUID;
BEGIN
    PERFORM media_actor_id_for_public_id_v1(actor_public_id_input);

    IF NULLIF(btrim(desired_target_key_input), '') IS NULL THEN
        UPDATE media_profile
           SET desired_target_profile_id = NULL,
               updated_at = now()
         WHERE media_profile_public_id = media_profile_public_id_input
           AND deleted_at IS NULL
        RETURNING media_profile_public_id INTO profile_public_id;
    ELSE
        SELECT media_desired_target_profile_id
          INTO target_id
          FROM media_desired_target_profile
         WHERE lower(target_key) = lower(btrim(desired_target_key_input))
           AND version = desired_target_version_input
           AND enabled;

        IF target_id IS NULL THEN
            RAISE EXCEPTION 'desired target not found'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
        END IF;

        UPDATE media_profile
           SET desired_target_profile_id = target_id,
               updated_at = now()
         WHERE media_profile_public_id = media_profile_public_id_input
           AND deleted_at IS NULL
        RETURNING media_profile_public_id INTO profile_public_id;
    END IF;

    IF profile_public_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    RETURN profile_public_id;
END;
$$;

CREATE OR REPLACE FUNCTION media_profile_list_v3()
RETURNS TABLE (
    media_profile_public_id UUID,
    profile_key TEXT,
    source_root TEXT,
    output_root TEXT,
    dry_run_only BOOLEAN,
    retention_days INT,
    compatibility_target_key TEXT,
    policy_key TEXT,
    watcher_enabled BOOLEAN,
    schedule_enabled BOOLEAN,
    schedule_interval_minutes INT,
    desired_target_key TEXT,
    desired_target_version INT,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT profile.media_profile_public_id, profile.profile_key,
           profile.source_root, profile.output_root, profile.dry_run_only,
           profile.retention_days, profile.compatibility_target_key,
           profile.policy_key, profile.watcher_enabled, profile.schedule_enabled,
           profile.schedule_interval_minutes, target.target_key, target.version,
           profile.updated_at
      FROM media_profile profile
      LEFT JOIN media_desired_target_profile target
        ON target.media_desired_target_profile_id = profile.desired_target_profile_id
     WHERE profile.deleted_at IS NULL
     ORDER BY lower(profile.profile_key);
$$;

CREATE OR REPLACE FUNCTION media_profile_get_v3(media_profile_public_id_input UUID)
RETURNS TABLE (
    media_profile_public_id UUID,
    profile_key TEXT,
    source_root TEXT,
    output_root TEXT,
    dry_run_only BOOLEAN,
    retention_days INT,
    compatibility_target_key TEXT,
    policy_key TEXT,
    watcher_enabled BOOLEAN,
    schedule_enabled BOOLEAN,
    schedule_interval_minutes INT,
    desired_target_key TEXT,
    desired_target_version INT,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT profile.media_profile_public_id, profile.profile_key,
           profile.source_root, profile.output_root, profile.dry_run_only,
           profile.retention_days, profile.compatibility_target_key,
           profile.policy_key, profile.watcher_enabled, profile.schedule_enabled,
           profile.schedule_interval_minutes, target.target_key, target.version,
           profile.updated_at
      FROM media_profile profile
      LEFT JOIN media_desired_target_profile target
        ON target.media_desired_target_profile_id = profile.desired_target_profile_id
     WHERE profile.media_profile_public_id = media_profile_public_id_input
       AND profile.deleted_at IS NULL;
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
        intent_unmatched_stream_policy, created_by_user_id
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
        policy_row.unmatched_stream_policy, actor_id
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

CREATE OR REPLACE FUNCTION media_job_desired_target_stream_list_v1(
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
    title TEXT,
    default_disposition BOOLEAN,
    forced_disposition BOOLEAN
)
LANGUAGE sql
STABLE
AS $$
    SELECT stream.stream_key, stream.stream_kind, stream.semantic_role,
           stream.language_code, stream.optional, stream.sort_order, stream.codec,
           stream.channel_count, stream.channel_layout, stream.title,
           stream.default_disposition, stream.forced_disposition
      FROM media_job job
      JOIN media_job_desired_target_stream stream ON stream.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY stream.sort_order;
$$;

DROP FUNCTION media_job_worker_claim_next_v1();

CREATE FUNCTION media_job_worker_claim_next_v1()
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
    unmatched_stream_policy TEXT
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
               heartbeat_at = now(), completed_at = NULL, last_error = NULL
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
           updated.intent_desired_container_format, updated.intent_unmatched_stream_policy
      FROM updated
      JOIN media_profile profile ON profile.media_profile_id = updated.media_profile_id;
END;
$$;
