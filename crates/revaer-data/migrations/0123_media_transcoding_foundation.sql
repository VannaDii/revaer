CREATE TYPE media_job_status AS ENUM (
    'queued',
    'running',
    'verifying',
    'completed',
    'failed',
    'cancelled'
);

CREATE TABLE media_profile (
    media_profile_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_profile_public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    profile_key TEXT NOT NULL,
    source_root TEXT NOT NULL,
    output_root TEXT NOT NULL,
    dry_run_only BOOLEAN NOT NULL DEFAULT TRUE,
    retention_days INT NOT NULL DEFAULT 30,
    created_by_user_id BIGINT NOT NULL REFERENCES app_user(user_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    CONSTRAINT media_profile_key_nonempty CHECK (btrim(profile_key) <> ''),
    CONSTRAINT media_profile_roots_nonempty CHECK (
        btrim(source_root) <> '' AND btrim(output_root) <> ''
    ),
    CONSTRAINT media_profile_retention_bounds CHECK (retention_days BETWEEN 1 AND 3650)
);

CREATE UNIQUE INDEX uq_media_profile_profile_key_active
    ON media_profile ((lower(profile_key)))
    WHERE deleted_at IS NULL;

CREATE TABLE media_target (
    media_target_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_profile_id BIGINT NOT NULL REFERENCES media_profile(media_profile_id) ON DELETE CASCADE,
    target_key TEXT NOT NULL,
    video_codec TEXT,
    audio_codec TEXT,
    subtitle_codec TEXT,
    priority SMALLINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_target_key_nonempty CHECK (btrim(target_key) <> '')
);

CREATE UNIQUE INDEX uq_media_target_profile_target_key
    ON media_target (media_profile_id, lower(target_key));

CREATE TABLE media_job (
    media_job_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    media_profile_id BIGINT NOT NULL REFERENCES media_profile(media_profile_id),
    source_path TEXT NOT NULL,
    output_path TEXT,
    status media_job_status NOT NULL DEFAULT 'queued',
    queued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    last_error TEXT,
    dry_run BOOLEAN NOT NULL DEFAULT TRUE,
    created_by_user_id BIGINT NOT NULL REFERENCES app_user(user_id),
    CONSTRAINT media_job_paths_nonempty CHECK (btrim(source_path) <> '')
);

CREATE INDEX ix_media_job_profile_status ON media_job (media_profile_id, status, queued_at DESC);

CREATE TABLE media_job_phase (
    media_job_phase_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    phase_index INT NOT NULL,
    phase_name TEXT NOT NULL,
    phase_status media_job_status NOT NULL,
    details_text TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_phase_name_nonempty CHECK (btrim(phase_name) <> ''),
    CONSTRAINT media_job_phase_index_nonnegative CHECK (phase_index >= 0)
);

CREATE UNIQUE INDEX uq_media_job_phase_job_index ON media_job_phase (media_job_id, phase_index);

CREATE TABLE media_job_operation (
    media_job_operation_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    operation_index INT NOT NULL,
    operation_kind TEXT NOT NULL,
    stream_id INT,
    command_bin TEXT NOT NULL,
    arg_1 TEXT,
    arg_2 TEXT,
    arg_3 TEXT,
    arg_4 TEXT,
    arg_5 TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_operation_kind_nonempty CHECK (btrim(operation_kind) <> ''),
    CONSTRAINT media_job_operation_bin_nonempty CHECK (btrim(command_bin) <> ''),
    CONSTRAINT media_job_operation_index_nonnegative CHECK (operation_index >= 0)
);

CREATE UNIQUE INDEX uq_media_job_operation_job_index ON media_job_operation (media_job_id, operation_index);

CREATE TABLE media_capability_snapshot (
    media_capability_snapshot_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    ffmpeg_version TEXT NOT NULL,
    ffprobe_version TEXT NOT NULL,
    codec_name TEXT NOT NULL,
    encode_supported BOOLEAN NOT NULL DEFAULT FALSE,
    decode_supported BOOLEAN NOT NULL DEFAULT TRUE,
    observed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    observed_by_user_id BIGINT NOT NULL REFERENCES app_user(user_id),
    CONSTRAINT media_capability_versions_nonempty CHECK (
        btrim(ffmpeg_version) <> '' AND btrim(ffprobe_version) <> ''
    ),
    CONSTRAINT media_capability_codec_nonempty CHECK (btrim(codec_name) <> '')
);

CREATE INDEX ix_media_capability_snapshot_observed_at
    ON media_capability_snapshot (observed_at DESC);

CREATE OR REPLACE FUNCTION media_profile_upsert_v1(
    actor_public_id_input UUID,
    profile_key_input TEXT,
    source_root_input TEXT,
    output_root_input TEXT,
    dry_run_only_input BOOLEAN,
    retention_days_input INT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    profile_public_id_out UUID;
BEGIN
    SELECT user_id
      INTO actor_id
      FROM app_user
     WHERE user_public_id = actor_public_id_input;

    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'actor not found'
            USING ERRCODE = 'P0001', DETAIL = 'app_user_not_found';
    END IF;

    IF lower(btrim(source_root_input)) = lower(btrim(output_root_input))
       OR lower(btrim(source_root_input)) LIKE lower(btrim(output_root_input)) || '/%'
       OR lower(btrim(output_root_input)) LIKE lower(btrim(source_root_input)) || '/%' THEN
        RAISE EXCEPTION 'profile roots overlap'
            USING ERRCODE = 'P0001', DETAIL = 'media_profile_roots_overlap';
    END IF;

    INSERT INTO media_profile (
        profile_key,
        source_root,
        output_root,
        dry_run_only,
        retention_days,
        created_by_user_id
    )
    VALUES (
        btrim(profile_key_input),
        btrim(source_root_input),
        btrim(output_root_input),
        COALESCE(dry_run_only_input, TRUE),
        COALESCE(retention_days_input, 30),
        actor_id
    )
    ON CONFLICT ((lower(profile_key)))
    WHERE deleted_at IS NULL
    DO UPDATE SET
        source_root = EXCLUDED.source_root,
        output_root = EXCLUDED.output_root,
        dry_run_only = EXCLUDED.dry_run_only,
        retention_days = EXCLUDED.retention_days,
        updated_at = now()
    RETURNING media_profile_public_id
    INTO profile_public_id_out;

    RETURN profile_public_id_out;
END;
$$;

CREATE OR REPLACE FUNCTION media_profile_list_v1()
RETURNS TABLE (
    media_profile_public_id UUID,
    profile_key TEXT,
    source_root TEXT,
    output_root TEXT,
    dry_run_only BOOLEAN,
    retention_days INT,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        mp.media_profile_public_id,
        mp.profile_key,
        mp.source_root,
        mp.output_root,
        mp.dry_run_only,
        mp.retention_days,
        mp.updated_at
    FROM media_profile mp
    WHERE mp.deleted_at IS NULL
    ORDER BY lower(mp.profile_key);
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
    profile_id BIGINT;
    media_job_public_id_out UUID;
BEGIN
    SELECT user_id
      INTO actor_id
      FROM app_user
     WHERE user_public_id = actor_public_id_input;

    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'actor not found'
            USING ERRCODE = 'P0001', DETAIL = 'app_user_not_found';
    END IF;

    SELECT media_profile_id
      INTO profile_id
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_profile_not_found';
    END IF;

    INSERT INTO media_job (
        media_profile_id,
        source_path,
        output_path,
        dry_run,
        created_by_user_id
    )
    VALUES (
        profile_id,
        btrim(source_path_input),
        NULLIF(btrim(output_path_input), ''),
        COALESCE(dry_run_input, TRUE),
        actor_id
    )
    RETURNING media_job_public_id
    INTO media_job_public_id_out;

    RETURN media_job_public_id_out;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_phase_append_v1(
    media_job_public_id_input UUID,
    phase_index_input INT,
    phase_name_input TEXT,
    phase_status_input TEXT,
    details_text_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    job_id BIGINT;
BEGIN
    SELECT media_job_id
      INTO job_id
      FROM media_job
     WHERE media_job_public_id = media_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
    END IF;

    INSERT INTO media_job_phase (
        media_job_id,
        phase_index,
        phase_name,
        phase_status,
        details_text
    )
    VALUES (
        job_id,
        phase_index_input,
        btrim(phase_name_input),
        phase_status_input::media_job_status,
        NULLIF(btrim(details_text_input), '')
    )
    ON CONFLICT (media_job_id, phase_index)
    DO UPDATE SET
        phase_name = EXCLUDED.phase_name,
        phase_status = EXCLUDED.phase_status,
        details_text = EXCLUDED.details_text;
END;
$$;

CREATE OR REPLACE FUNCTION media_capability_snapshot_record_v1(
    actor_public_id_input UUID,
    ffmpeg_version_input TEXT,
    ffprobe_version_input TEXT,
    codec_name_input TEXT,
    encode_supported_input BOOLEAN,
    decode_supported_input BOOLEAN
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    snapshot_id_out BIGINT;
BEGIN
    SELECT user_id
      INTO actor_id
      FROM app_user
     WHERE user_public_id = actor_public_id_input;

    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'actor not found'
            USING ERRCODE = 'P0001', DETAIL = 'app_user_not_found';
    END IF;

    INSERT INTO media_capability_snapshot (
        ffmpeg_version,
        ffprobe_version,
        codec_name,
        encode_supported,
        decode_supported,
        observed_by_user_id
    )
    VALUES (
        btrim(ffmpeg_version_input),
        btrim(ffprobe_version_input),
        btrim(codec_name_input),
        COALESCE(encode_supported_input, FALSE),
        COALESCE(decode_supported_input, TRUE),
        actor_id
    )
    RETURNING media_capability_snapshot_id
    INTO snapshot_id_out;

    RETURN snapshot_id_out;
END;
$$;

CREATE OR REPLACE FUNCTION media_capability_snapshot_latest_v1()
RETURNS TABLE (
    media_capability_snapshot_id BIGINT,
    ffmpeg_version TEXT,
    ffprobe_version TEXT,
    codec_name TEXT,
    encode_supported BOOLEAN,
    decode_supported BOOLEAN,
    observed_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        mcs.media_capability_snapshot_id,
        mcs.ffmpeg_version,
        mcs.ffprobe_version,
        mcs.codec_name,
        mcs.encode_supported,
        mcs.decode_supported,
        mcs.observed_at
    FROM media_capability_snapshot mcs
    ORDER BY mcs.observed_at DESC, mcs.media_capability_snapshot_id DESC
    LIMIT 1;
$$;

CREATE OR REPLACE FUNCTION media_job_list_v1(
    media_profile_public_id_input UUID,
    status_input media_job_status DEFAULT NULL
)
RETURNS TABLE (
    media_job_public_id UUID,
    source_path TEXT,
    output_path TEXT,
    status media_job_status,
    dry_run BOOLEAN,
    queued_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    last_error TEXT
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        mj.media_job_public_id,
        mj.source_path,
        mj.output_path,
        mj.status,
        mj.dry_run,
        mj.queued_at,
        mj.started_at,
        mj.completed_at,
        mj.last_error
    FROM media_job mj
    JOIN media_profile mp ON mp.media_profile_id = mj.media_profile_id
    WHERE mp.media_profile_public_id = media_profile_public_id_input
      AND mp.deleted_at IS NULL
      AND (status_input IS NULL OR mj.status = status_input)
    ORDER BY mj.queued_at DESC;
$$;
