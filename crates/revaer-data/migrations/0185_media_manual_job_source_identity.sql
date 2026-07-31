CREATE FUNCTION media_manual_job_create_v2(
    actor_public_id_input UUID,
    media_profile_public_id_input UUID,
    source_path_input TEXT,
    output_path_input TEXT,
    source_identity_input TEXT,
    source_size_bytes_input BIGINT,
    source_modified_ns_input BIGINT,
    source_changed_ns_input BIGINT,
    source_sha256_input TEXT,
    dry_run_input BOOLEAN
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    profile_row media_profile%ROWTYPE;
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
       OR COALESCE(lower(btrim(source_identity_input)), '') !~ '^[0-9a-f]{16}:[0-9a-f]{16}$'
       OR COALESCE(source_size_bytes_input, -1) < 0
       OR COALESCE(source_modified_ns_input, -1) < 0
       OR COALESCE(source_changed_ns_input, -1) < 0
       OR COALESCE(lower(btrim(source_sha256_input)), '') !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION 'invalid manual job fingerprint'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_manual_job_fingerprint_invalid';
    END IF;

    INSERT INTO media_discovery_source_fingerprint (
        media_profile_id,
        source_path,
        source_identity,
        source_size_bytes,
        source_modified_ns,
        source_changed_ns,
        source_sha256
    )
    VALUES (
        profile_row.media_profile_id,
        btrim(source_path_input),
        lower(btrim(source_identity_input)),
        source_size_bytes_input,
        source_modified_ns_input,
        source_changed_ns_input,
        lower(btrim(source_sha256_input))
    )
    ON CONFLICT (media_profile_id, source_path) DO UPDATE
    SET source_identity = EXCLUDED.source_identity,
        source_size_bytes = EXCLUDED.source_size_bytes,
        source_modified_ns = EXCLUDED.source_modified_ns,
        source_changed_ns = EXCLUDED.source_changed_ns,
        source_sha256 = EXCLUDED.source_sha256,
        last_seen_at = now();

    media_job_public_id_out := media_job_create_v1(
        actor_public_id_input,
        media_profile_public_id_input,
        source_path_input,
        output_path_input,
        COALESCE(dry_run_input, TRUE)
    );

    UPDATE media_discovery_source_fingerprint
       SET last_media_job_public_id = media_job_public_id_out
     WHERE media_profile_id = profile_row.media_profile_id
       AND source_path = btrim(source_path_input);

    RETURN media_job_public_id_out;
END;
$$;
