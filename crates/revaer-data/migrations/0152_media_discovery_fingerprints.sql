CREATE TABLE media_discovery_source_fingerprint (
    media_discovery_source_fingerprint_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_profile_id BIGINT NOT NULL REFERENCES media_profile(media_profile_id) ON DELETE CASCADE,
    source_path TEXT NOT NULL,
    source_size_bytes BIGINT NOT NULL,
    source_modified_ns BIGINT NOT NULL,
    source_sha256 TEXT NOT NULL,
    last_media_job_public_id UUID,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_discovery_source_path_nonempty CHECK (btrim(source_path) <> ''),
    CONSTRAINT media_discovery_source_size_nonnegative CHECK (source_size_bytes >= 0),
    CONSTRAINT media_discovery_source_modified_nonnegative CHECK (source_modified_ns >= 0),
    CONSTRAINT media_discovery_source_sha256_valid CHECK (source_sha256 ~ '^[0-9a-f]{64}$')
);

CREATE UNIQUE INDEX uq_media_discovery_source_fingerprint_profile_path
    ON media_discovery_source_fingerprint (media_profile_id, source_path);

CREATE OR REPLACE FUNCTION media_discovery_job_enqueue_v1(
    actor_public_id_input UUID,
    media_profile_public_id_input UUID,
    source_path_input TEXT,
    output_path_input TEXT,
    source_size_bytes_input BIGINT,
    source_modified_ns_input BIGINT,
    source_sha256_input TEXT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    profile_row media_profile%ROWTYPE;
    source_changed BOOLEAN;
    media_job_public_id_out UUID;
BEGIN
    SELECT * INTO profile_row
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_row.media_profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;
    IF NULLIF(btrim(source_path_input), '') IS NULL
       OR COALESCE(source_size_bytes_input, -1) < 0
       OR COALESCE(source_modified_ns_input, -1) < 0
       OR COALESCE(lower(btrim(source_sha256_input)), '') !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION 'invalid discovery fingerprint'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_fingerprint_invalid';
    END IF;

    WITH changed AS (
        INSERT INTO media_discovery_source_fingerprint (
            media_profile_id,
            source_path,
            source_size_bytes,
            source_modified_ns,
            source_sha256
        )
        VALUES (
            profile_row.media_profile_id,
            btrim(source_path_input),
            source_size_bytes_input,
            source_modified_ns_input,
            lower(btrim(source_sha256_input))
        )
        ON CONFLICT (media_profile_id, source_path) DO UPDATE
        SET source_size_bytes = EXCLUDED.source_size_bytes,
            source_modified_ns = EXCLUDED.source_modified_ns,
            source_sha256 = EXCLUDED.source_sha256,
            last_seen_at = now()
        WHERE media_discovery_source_fingerprint.source_size_bytes
                  IS DISTINCT FROM EXCLUDED.source_size_bytes
           OR media_discovery_source_fingerprint.source_modified_ns
                  IS DISTINCT FROM EXCLUDED.source_modified_ns
           OR media_discovery_source_fingerprint.source_sha256
                  IS DISTINCT FROM EXCLUDED.source_sha256
        RETURNING 1
    )
    SELECT EXISTS (SELECT 1 FROM changed) INTO source_changed;

    IF NOT source_changed THEN
        RETURN NULL;
    END IF;

    media_job_public_id_out := media_job_create_v1(
        actor_public_id_input,
        media_profile_public_id_input,
        source_path_input,
        output_path_input,
        profile_row.dry_run_only
    );

    UPDATE media_discovery_source_fingerprint
       SET last_media_job_public_id = media_job_public_id_out
     WHERE media_profile_id = profile_row.media_profile_id
       AND source_path = btrim(source_path_input);

    RETURN media_job_public_id_out;
END;
$$;
