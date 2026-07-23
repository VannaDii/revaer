CREATE OR REPLACE FUNCTION media_profile_roots_overlap_v1(
    left_root_input TEXT,
    right_root_input TEXT
)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT media_profile_normalized_root_v1(left_root_input)
               = media_profile_normalized_root_v1(right_root_input)
        OR media_profile_normalized_root_v1(left_root_input)
               LIKE media_profile_normalized_root_v1(right_root_input) || '/%'
        OR media_profile_normalized_root_v1(right_root_input)
               LIKE media_profile_normalized_root_v1(left_root_input) || '/%'
$$;

CREATE OR REPLACE FUNCTION media_profile_validate_all_root_overlap_v1(
    media_profile_id_input BIGINT,
    source_root_input TEXT,
    output_root_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('media_profile_all_root_overlap_v1', 0));

    IF media_profile_roots_overlap_v1(source_root_input, output_root_input) THEN
        RAISE EXCEPTION 'profile roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_roots_overlap';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM media_profile AS existing
         WHERE existing.deleted_at IS NULL
           AND (
               media_profile_id_input IS NULL
               OR existing.media_profile_id <> media_profile_id_input
           )
           AND (
               media_profile_roots_overlap_v1(source_root_input, existing.source_root)
               OR media_profile_roots_overlap_v1(source_root_input, existing.output_root)
               OR media_profile_roots_overlap_v1(output_root_input, existing.source_root)
               OR media_profile_roots_overlap_v1(output_root_input, existing.output_root)
           )
    ) THEN
        RAISE EXCEPTION 'profile roots overlap another active profile'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_discovery_root_overlap';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION media_profile_validate_all_root_overlap_trigger_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    effective_profile_id BIGINT;
BEGIN
    IF NEW.deleted_at IS NULL THEN
        effective_profile_id := NEW.media_profile_id;

        IF TG_OP = 'INSERT' THEN
            SELECT existing.media_profile_id
              INTO effective_profile_id
              FROM media_profile AS existing
             WHERE lower(existing.profile_key) = lower(btrim(NEW.profile_key))
               AND existing.deleted_at IS NULL;

            effective_profile_id := COALESCE(effective_profile_id, NEW.media_profile_id);
        END IF;

        PERFORM media_profile_validate_all_root_overlap_v1(
            effective_profile_id,
            NEW.source_root,
            NEW.output_root
        );
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS media_profile_all_root_overlap_trigger ON media_profile;

CREATE TRIGGER media_profile_all_root_overlap_trigger
BEFORE INSERT OR UPDATE OF source_root, output_root, deleted_at
ON media_profile
FOR EACH ROW
EXECUTE FUNCTION media_profile_validate_all_root_overlap_trigger_v1();

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM media_profile AS left_profile
          JOIN media_profile AS right_profile
            ON left_profile.media_profile_id < right_profile.media_profile_id
         WHERE left_profile.deleted_at IS NULL
           AND right_profile.deleted_at IS NULL
           AND (
               media_profile_roots_overlap_v1(left_profile.source_root, right_profile.source_root)
               OR media_profile_roots_overlap_v1(left_profile.source_root, right_profile.output_root)
               OR media_profile_roots_overlap_v1(left_profile.output_root, right_profile.source_root)
               OR media_profile_roots_overlap_v1(left_profile.output_root, right_profile.output_root)
           )
    ) THEN
        RAISE EXCEPTION 'existing active media profile roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_discovery_root_overlap';
    END IF;
END;
$$;
