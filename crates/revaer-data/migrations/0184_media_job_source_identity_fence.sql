ALTER TABLE media_discovery_source_fingerprint
    ADD COLUMN source_identity TEXT,
    ADD COLUMN source_changed_ns BIGINT,
    ADD CONSTRAINT media_discovery_source_identity_valid CHECK (
        (source_identity IS NULL AND source_changed_ns IS NULL)
        OR
        (
            source_identity ~ '^[0-9a-f]{16}:[0-9a-f]{16}$'
            AND source_changed_ns >= 0
        )
    );

ALTER TABLE media_job
    ADD COLUMN intent_source_identity TEXT,
    ADD COLUMN intent_source_size_bytes BIGINT,
    ADD COLUMN intent_source_modified_ns BIGINT,
    ADD COLUMN intent_source_changed_ns BIGINT,
    ADD COLUMN intent_source_sha256 TEXT,
    ADD CONSTRAINT media_job_source_fingerprint_snapshot_valid CHECK (
        (
            intent_source_identity IS NULL
            AND intent_source_size_bytes IS NULL
            AND intent_source_modified_ns IS NULL
            AND intent_source_changed_ns IS NULL
            AND intent_source_sha256 IS NULL
        )
        OR
        (
            intent_source_identity ~ '^[0-9a-f]{16}:[0-9a-f]{16}$'
            AND intent_source_size_bytes >= 0
            AND intent_source_modified_ns >= 0
            AND intent_source_changed_ns >= 0
            AND intent_source_sha256 ~ '^[0-9a-f]{64}$'
        )
    );

CREATE OR REPLACE FUNCTION media_job_configuration_immutable_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.media_profile_id IS DISTINCT FROM OLD.media_profile_id
       OR NEW.source_path IS DISTINCT FROM OLD.source_path
       OR NEW.output_path IS DISTINCT FROM OLD.output_path
       OR NEW.dry_run IS DISTINCT FROM OLD.dry_run
       OR NEW.intent_source_root IS DISTINCT FROM OLD.intent_source_root
       OR NEW.intent_output_root IS DISTINCT FROM OLD.intent_output_root
       OR NEW.intent_source_identity IS DISTINCT FROM OLD.intent_source_identity
       OR NEW.intent_source_size_bytes IS DISTINCT FROM OLD.intent_source_size_bytes
       OR NEW.intent_source_modified_ns IS DISTINCT FROM OLD.intent_source_modified_ns
       OR NEW.intent_source_changed_ns IS DISTINCT FROM OLD.intent_source_changed_ns
       OR NEW.intent_source_sha256 IS DISTINCT FROM OLD.intent_source_sha256
       OR NEW.intent_compatibility_target_key IS DISTINCT FROM OLD.intent_compatibility_target_key
       OR NEW.intent_policy_key IS DISTINCT FROM OLD.intent_policy_key
       OR NEW.intent_compatibility_target_id IS DISTINCT FROM OLD.intent_compatibility_target_id
       OR NEW.intent_compatibility_target_version IS DISTINCT FROM OLD.intent_compatibility_target_version
       OR NEW.intent_policy_profile_id IS DISTINCT FROM OLD.intent_policy_profile_id
       OR NEW.intent_policy_version IS DISTINCT FROM OLD.intent_policy_version
       OR NEW.intent_desired_target_profile_id IS DISTINCT FROM OLD.intent_desired_target_profile_id
       OR NEW.intent_desired_target_version IS DISTINCT FROM OLD.intent_desired_target_version THEN
        RAISE EXCEPTION 'job configuration snapshot is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_configuration_immutable';
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION media_job_snapshot_source_fingerprint_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    fingerprint_row media_discovery_source_fingerprint%ROWTYPE;
BEGIN
    SELECT fingerprint.*
      INTO fingerprint_row
      FROM media_discovery_source_fingerprint fingerprint
     WHERE fingerprint.media_profile_id = NEW.media_profile_id
       AND fingerprint.source_path = NEW.source_path
     FOR SHARE;

    IF fingerprint_row.media_discovery_source_fingerprint_id IS NULL
       OR fingerprint_row.source_identity IS NULL
       OR fingerprint_row.source_changed_ns IS NULL THEN
        RAISE EXCEPTION 'media job source fingerprint required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_source_fingerprint_required';
    END IF;

    NEW.intent_source_identity := fingerprint_row.source_identity;
    NEW.intent_source_size_bytes := fingerprint_row.source_size_bytes;
    NEW.intent_source_modified_ns := fingerprint_row.source_modified_ns;
    NEW.intent_source_changed_ns := fingerprint_row.source_changed_ns;
    NEW.intent_source_sha256 := fingerprint_row.source_sha256;
    RETURN NEW;
END;
$$;

CREATE TRIGGER media_job_snapshot_source_fingerprint_before_insert
BEFORE INSERT ON media_job
FOR EACH ROW
EXECUTE FUNCTION media_job_snapshot_source_fingerprint_v1();

CREATE FUNCTION media_discovery_job_enqueue_v3(
    actor_public_id_input UUID,
    media_profile_public_id_input UUID,
    source_path_input TEXT,
    output_path_input TEXT,
    dry_run_input BOOLEAN,
    source_identity_input TEXT,
    source_size_bytes_input BIGINT,
    source_modified_ns_input BIGINT,
    source_changed_ns_input BIGINT,
    source_sha256_input TEXT
)
RETURNS TABLE (
    media_job_public_id UUID,
    dry_run BOOLEAN
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    profile_row media_profile%ROWTYPE;
    source_changed BOOLEAN;
    media_job_public_id_out UUID;
    dry_run_out BOOLEAN;
BEGIN
    SELECT * INTO profile_row
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL
     FOR UPDATE;

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
        RAISE EXCEPTION 'invalid discovery fingerprint'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_fingerprint_invalid';
    END IF;

    WITH changed AS (
        INSERT INTO media_discovery_source_fingerprint (
            media_profile_id, source_path, source_identity, source_size_bytes,
            source_modified_ns, source_changed_ns, source_sha256
        )
        VALUES (
            profile_row.media_profile_id, btrim(source_path_input),
            lower(btrim(source_identity_input)), source_size_bytes_input,
            source_modified_ns_input, source_changed_ns_input,
            lower(btrim(source_sha256_input))
        )
        ON CONFLICT (media_profile_id, source_path) DO UPDATE
        SET source_identity = EXCLUDED.source_identity,
            source_size_bytes = EXCLUDED.source_size_bytes,
            source_modified_ns = EXCLUDED.source_modified_ns,
            source_changed_ns = EXCLUDED.source_changed_ns,
            source_sha256 = EXCLUDED.source_sha256,
            last_seen_at = now()
        WHERE media_discovery_source_fingerprint.source_identity
                  IS DISTINCT FROM EXCLUDED.source_identity
           OR media_discovery_source_fingerprint.source_size_bytes
                  IS DISTINCT FROM EXCLUDED.source_size_bytes
           OR media_discovery_source_fingerprint.source_modified_ns
                  IS DISTINCT FROM EXCLUDED.source_modified_ns
           OR media_discovery_source_fingerprint.source_changed_ns
                  IS DISTINCT FROM EXCLUDED.source_changed_ns
           OR media_discovery_source_fingerprint.source_sha256
                  IS DISTINCT FROM EXCLUDED.source_sha256
        RETURNING 1
    )
    SELECT EXISTS (SELECT 1 FROM changed) INTO source_changed;

    IF NOT source_changed THEN
        RETURN;
    END IF;

    dry_run_out := COALESCE(dry_run_input, TRUE) OR profile_row.dry_run_only;
    media_job_public_id_out := media_job_create_v1(
        actor_public_id_input, media_profile_public_id_input, source_path_input,
        output_path_input, dry_run_out
    );

    UPDATE media_discovery_source_fingerprint
       SET last_media_job_public_id = media_job_public_id_out
     WHERE media_profile_id = profile_row.media_profile_id
       AND source_path = btrim(source_path_input);

    RETURN QUERY SELECT media_job_public_id_out, dry_run_out;
END;
$$;

CREATE FUNCTION media_discovery_job_enqueue_v2(
    actor_public_id_input UUID,
    media_profile_public_id_input UUID,
    source_path_input TEXT,
    output_path_input TEXT,
    source_identity_input TEXT,
    source_size_bytes_input BIGINT,
    source_modified_ns_input BIGINT,
    source_changed_ns_input BIGINT,
    source_sha256_input TEXT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    media_job_public_id_out UUID;
BEGIN
    SELECT enqueued.media_job_public_id
      INTO media_job_public_id_out
      FROM media_discovery_job_enqueue_v3(
          actor_public_id_input,
          media_profile_public_id_input,
          source_path_input,
          output_path_input,
          TRUE,
          source_identity_input,
          source_size_bytes_input,
          source_modified_ns_input,
          source_changed_ns_input,
          source_sha256_input
      ) enqueued;
    RETURN media_job_public_id_out;
END;
$$;
