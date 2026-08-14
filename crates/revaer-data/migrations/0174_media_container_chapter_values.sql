ALTER TABLE media_desired_target_container
    DROP CONSTRAINT media_desired_target_container_chapter_policy_known,
    ADD CONSTRAINT media_desired_target_container_chapter_policy_known CHECK (
        container_chapter_policy IN ('preserve', 'strip', 'replace')
    );

CREATE TABLE media_desired_target_container_chapter (
    media_desired_target_container_chapter_id BIGSERIAL PRIMARY KEY,
    media_desired_target_profile_id BIGINT NOT NULL
        REFERENCES media_desired_target_profile(media_desired_target_profile_id)
        ON DELETE CASCADE,
    start_millis BIGINT NOT NULL,
    end_millis BIGINT NOT NULL,
    sort_order INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_desired_target_container_chapter_range_valid CHECK (
        start_millis >= 0 AND end_millis > start_millis
    ),
    CONSTRAINT media_desired_target_container_chapter_start_unique UNIQUE (
        media_desired_target_profile_id, start_millis
    ),
    CONSTRAINT media_desired_target_container_chapter_order_unique UNIQUE (
        media_desired_target_profile_id, sort_order
    )
);

CREATE TABLE media_desired_target_container_chapter_metadata (
    media_desired_target_container_chapter_metadata_id BIGSERIAL PRIMARY KEY,
    media_desired_target_container_chapter_id BIGINT NOT NULL
        REFERENCES media_desired_target_container_chapter(media_desired_target_container_chapter_id)
        ON DELETE CASCADE,
    metadata_key TEXT NOT NULL,
    metadata_value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_desired_target_container_chapter_metadata_key_valid CHECK (
        btrim(metadata_key) <> ''
        AND metadata_key = lower(btrim(metadata_key))
        AND octet_length(metadata_key) <= 128
    ),
    CONSTRAINT media_desired_target_container_chapter_metadata_value_valid CHECK (
        btrim(metadata_value) <> '' AND octet_length(metadata_value) <= 4096
    ),
    CONSTRAINT media_desired_target_container_chapter_metadata_key_unique UNIQUE (
        media_desired_target_container_chapter_id, metadata_key
    )
);

CREATE TABLE media_job_desired_target_chapter (
    media_job_desired_target_chapter_id BIGSERIAL PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    start_millis BIGINT NOT NULL,
    end_millis BIGINT NOT NULL,
    sort_order INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_desired_target_chapter_range_valid CHECK (
        start_millis >= 0 AND end_millis > start_millis
    ),
    CONSTRAINT media_job_desired_target_chapter_start_unique UNIQUE (
        media_job_id, start_millis
    ),
    CONSTRAINT media_job_desired_target_chapter_order_unique UNIQUE (
        media_job_id, sort_order
    )
);

CREATE TABLE media_job_desired_target_chapter_metadata (
    media_job_desired_target_chapter_metadata_id BIGSERIAL PRIMARY KEY,
    media_job_desired_target_chapter_id BIGINT NOT NULL
        REFERENCES media_job_desired_target_chapter(media_job_desired_target_chapter_id)
        ON DELETE CASCADE,
    metadata_key TEXT NOT NULL,
    metadata_value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_desired_target_chapter_metadata_key_valid CHECK (
        btrim(metadata_key) <> ''
        AND metadata_key = lower(btrim(metadata_key))
        AND octet_length(metadata_key) <= 128
    ),
    CONSTRAINT media_job_desired_target_chapter_metadata_value_valid CHECK (
        btrim(metadata_value) <> '' AND octet_length(metadata_value) <= 4096
    ),
    CONSTRAINT media_job_desired_target_chapter_metadata_key_unique UNIQUE (
        media_job_desired_target_chapter_id, metadata_key
    )
);

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
            AND intent_desired_container_metadata_policy IN ('preserve', 'strip', 'replace')
            AND intent_desired_container_chapter_policy IN ('preserve', 'strip', 'replace'))
    );

CREATE OR REPLACE FUNCTION media_desired_target_create_v3(
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
    actor_id BIGINT;
    target_id BIGINT;
    target_public_id UUID;
    metadata_policy_value TEXT;
    chapter_policy_value TEXT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    metadata_policy_value := lower(COALESCE(NULLIF(btrim(container_metadata_policy_input), ''), 'preserve'));
    chapter_policy_value := lower(COALESCE(NULLIF(btrim(container_chapter_policy_input), ''), 'preserve'));

    IF NULLIF(btrim(target_key_input), '') IS NULL
       OR COALESCE(version_input, 0) <= 0
       OR NULLIF(btrim(display_name_input), '') IS NULL
       OR NULLIF(btrim(container_format_input), '') IS NULL
       OR metadata_policy_value NOT IN ('preserve', 'strip', 'replace')
       OR chapter_policy_value NOT IN ('preserve', 'strip', 'replace') THEN
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
        container_format,
        container_metadata_policy,
        container_chapter_policy
    )
    VALUES (
        target_id,
        lower(btrim(container_format_input)),
        metadata_policy_value,
        chapter_policy_value
    );

    RETURN target_public_id;
END;
$$;

CREATE FUNCTION media_desired_target_chapter_append_v1(
    media_desired_target_profile_public_id_input UUID,
    start_millis_input BIGINT,
    end_millis_input BIGINT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    target_id BIGINT;
    chapter_policy_value TEXT;
    next_sort_order INT;
BEGIN
    SELECT target.media_desired_target_profile_id,
           container.container_chapter_policy
      INTO target_id,
           chapter_policy_value
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     FOR UPDATE;

    IF target_id IS NULL THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    IF chapter_policy_value <> 'replace' THEN
        RAISE EXCEPTION 'desired target chapter rows require replace policy'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_policy_mismatch';
    END IF;

    IF EXISTS (
        SELECT 1 FROM media_profile WHERE desired_target_profile_id = target_id
        UNION ALL
        SELECT 1 FROM media_job WHERE intent_desired_target_profile_id = target_id
    ) THEN
        RAISE EXCEPTION 'desired target version is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_immutable';
    END IF;

    IF COALESCE(start_millis_input, -1) < 0
       OR COALESCE(end_millis_input, 0) <= COALESCE(start_millis_input, -1) THEN
        RAISE EXCEPTION 'desired target chapter is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_invalid';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM media_desired_target_container_chapter chapter
         WHERE chapter.media_desired_target_profile_id = target_id
           AND chapter.start_millis < end_millis_input
           AND chapter.end_millis > start_millis_input
    ) THEN
        RAISE EXCEPTION 'desired target chapter overlaps'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_overlap';
    END IF;

    IF (
        SELECT COUNT(*)
          FROM media_desired_target_container_chapter chapter
         WHERE chapter.media_desired_target_profile_id = target_id
    ) >= 1024 THEN
        RAISE EXCEPTION 'desired target chapter count exceeds limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_count_exceeded';
    END IF;

    SELECT COALESCE(MAX(chapter.sort_order), 0) + 1
      INTO next_sort_order
      FROM media_desired_target_container_chapter chapter
     WHERE chapter.media_desired_target_profile_id = target_id;

    INSERT INTO media_desired_target_container_chapter (
        media_desired_target_profile_id,
        start_millis,
        end_millis,
        sort_order
    )
    VALUES (
        target_id,
        start_millis_input,
        end_millis_input,
        next_sort_order
    );
END;
$$;

CREATE FUNCTION media_desired_target_chapter_metadata_append_v1(
    media_desired_target_profile_public_id_input UUID,
    start_millis_input BIGINT,
    metadata_key_input TEXT,
    metadata_value_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    target_id BIGINT;
    chapter_id BIGINT;
    chapter_policy_value TEXT;
    metadata_key_value TEXT;
    metadata_value_value TEXT;
BEGIN
    SELECT target.media_desired_target_profile_id,
           container.container_chapter_policy
      INTO target_id,
           chapter_policy_value
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     FOR UPDATE;

    IF target_id IS NULL THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    IF chapter_policy_value <> 'replace' THEN
        RAISE EXCEPTION 'desired target chapter rows require replace policy'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_policy_mismatch';
    END IF;

    IF EXISTS (
        SELECT 1 FROM media_profile WHERE desired_target_profile_id = target_id
        UNION ALL
        SELECT 1 FROM media_job WHERE intent_desired_target_profile_id = target_id
    ) THEN
        RAISE EXCEPTION 'desired target version is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_immutable';
    END IF;

    SELECT chapter.media_desired_target_container_chapter_id
      INTO chapter_id
      FROM media_desired_target_container_chapter chapter
     WHERE chapter.media_desired_target_profile_id = target_id
       AND chapter.start_millis = start_millis_input;

    IF chapter_id IS NULL THEN
        RAISE EXCEPTION 'desired target chapter not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_not_found';
    END IF;

    metadata_key_value := lower(NULLIF(btrim(metadata_key_input), ''));
    metadata_value_value := NULLIF(btrim(metadata_value_input), '');

    IF metadata_key_value IS NULL OR metadata_value_value IS NULL THEN
        RAISE EXCEPTION 'desired target chapter metadata is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_invalid';
    END IF;

    IF octet_length(metadata_key_value) > 128
       OR octet_length(metadata_value_value) > 4096 THEN
        RAISE EXCEPTION 'desired target chapter metadata exceeds field limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_field_exceeded';
    END IF;

    IF (
        SELECT COUNT(*)
          FROM media_desired_target_container_chapter_metadata metadata
         WHERE metadata.media_desired_target_container_chapter_id = chapter_id
    ) >= 64 THEN
        RAISE EXCEPTION 'desired target chapter metadata count exceeds limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_count_exceeded';
    END IF;

    IF COALESCE((
        SELECT SUM(octet_length(metadata.metadata_key) + octet_length(metadata.metadata_value))
          FROM media_desired_target_container_chapter_metadata metadata
          JOIN media_desired_target_container_chapter chapter
            ON chapter.media_desired_target_container_chapter_id = metadata.media_desired_target_container_chapter_id
         WHERE chapter.media_desired_target_profile_id = target_id
    ), 0) + octet_length(metadata_key_value) + octet_length(metadata_value_value) > 65536 THEN
        RAISE EXCEPTION 'desired target chapter metadata aggregate exceeds limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_bytes_exceeded';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM media_desired_target_container_chapter_metadata metadata
         WHERE metadata.media_desired_target_container_chapter_id = chapter_id
           AND metadata.metadata_key = metadata_key_value
    ) THEN
        RAISE EXCEPTION 'desired target chapter metadata key is duplicated'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_duplicate';
    END IF;

    INSERT INTO media_desired_target_container_chapter_metadata (
        media_desired_target_container_chapter_id,
        metadata_key,
        metadata_value
    )
    VALUES (
        chapter_id,
        metadata_key_value,
        metadata_value_value
    );
END;
$$;

CREATE FUNCTION media_desired_target_chapter_list_v1(
    media_desired_target_profile_public_id_input UUID
)
RETURNS TABLE (
    start_millis BIGINT,
    end_millis BIGINT,
    metadata_key TEXT,
    metadata_value TEXT
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT chapter.start_millis,
           chapter.end_millis,
           metadata.metadata_key,
           metadata.metadata_value
      FROM media_desired_target_profile target
      JOIN media_desired_target_container_chapter chapter
        ON chapter.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_container_chapter_metadata metadata
        ON metadata.media_desired_target_container_chapter_id = chapter.media_desired_target_container_chapter_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY chapter.start_millis, metadata.metadata_key NULLS FIRST;
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
    metadata_policy_value TEXT;
    chapter_policy_value TEXT;
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

        IF NOT EXISTS (
            SELECT 1
              FROM media_desired_target_stream
             WHERE media_desired_target_profile_id = target_id
        ) THEN
            RAISE EXCEPTION 'desired target has no streams'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_streams_required';
        END IF;

        SELECT container.container_metadata_policy,
               container.container_chapter_policy
          INTO metadata_policy_value,
               chapter_policy_value
          FROM media_desired_target_container container
         WHERE container.media_desired_target_profile_id = target_id;

        IF metadata_policy_value = 'replace'
           AND NOT EXISTS (
               SELECT 1
                 FROM media_desired_target_container_metadata metadata
                WHERE metadata.media_desired_target_profile_id = target_id
           ) THEN
            RAISE EXCEPTION 'replace metadata target has no metadata rows'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_required';
        END IF;

        IF metadata_policy_value <> 'replace'
           AND EXISTS (
               SELECT 1
                 FROM media_desired_target_container_metadata metadata
                WHERE metadata.media_desired_target_profile_id = target_id
           ) THEN
            RAISE EXCEPTION 'metadata rows require replace policy'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_policy_mismatch';
        END IF;

        IF chapter_policy_value = 'replace'
           AND NOT EXISTS (
               SELECT 1
                 FROM media_desired_target_container_chapter chapter
                WHERE chapter.media_desired_target_profile_id = target_id
           ) THEN
            RAISE EXCEPTION 'replace chapter target has no chapter rows'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapters_required';
        END IF;

        IF chapter_policy_value <> 'replace'
           AND EXISTS (
               SELECT 1
                 FROM media_desired_target_container_chapter chapter
                WHERE chapter.media_desired_target_profile_id = target_id
           ) THEN
            RAISE EXCEPTION 'chapter rows require replace policy'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_policy_mismatch';
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

CREATE FUNCTION media_job_desired_target_chapter_list_v1(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    start_millis BIGINT,
    end_millis BIGINT,
    metadata_key TEXT,
    metadata_value TEXT
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT chapter.start_millis,
           chapter.end_millis,
           metadata.metadata_key,
           metadata.metadata_value
      FROM media_job job
      JOIN media_job_desired_target_chapter chapter
        ON chapter.media_job_id = job.media_job_id
      LEFT JOIN media_job_desired_target_chapter_metadata metadata
        ON metadata.media_job_desired_target_chapter_id = chapter.media_job_desired_target_chapter_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY chapter.start_millis, metadata.metadata_key NULLS FIRST
     LIMIT 65537;
$$;
