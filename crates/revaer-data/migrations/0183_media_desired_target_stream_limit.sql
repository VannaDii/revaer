ALTER TABLE media_desired_target_profile
    ADD COLUMN activated_at TIMESTAMPTZ;

CREATE FUNCTION media_desired_target_stream_limit_v1()
RETURNS INT
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT 1024
$$;

CREATE FUNCTION media_desired_target_stream_count_bounded_v1(
    media_desired_target_profile_id_input BIGINT
)
RETURNS INT
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT count(*)::INT
      FROM (
          SELECT 1
            FROM media_desired_target_stream
           WHERE media_desired_target_profile_id = media_desired_target_profile_id_input
           LIMIT media_desired_target_stream_limit_v1() + 1
      ) bounded_streams
$$;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM media_desired_target_profile target
         WHERE media_desired_target_stream_count_bounded_v1(
                   target.media_desired_target_profile_id
               ) > media_desired_target_stream_limit_v1()
    ) THEN
        RAISE EXCEPTION 'existing desired target exceeds stream limit';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM media_desired_target_profile target
         WHERE (
                 EXISTS (
                     SELECT 1
                       FROM media_profile profile
                      WHERE profile.desired_target_profile_id = target.media_desired_target_profile_id
                 )
                 OR EXISTS (
                     SELECT 1
                       FROM media_job job
                      WHERE job.intent_desired_target_profile_id = target.media_desired_target_profile_id
                 )
               )
           AND media_desired_target_stream_count_bounded_v1(
                   target.media_desired_target_profile_id
               ) = 0
    ) THEN
        RAISE EXCEPTION 'existing activated desired target has no streams';
    END IF;
END;
$$;

UPDATE media_desired_target_profile target
   SET activated_at = now()
 WHERE EXISTS (
           SELECT 1
             FROM media_profile profile
            WHERE profile.desired_target_profile_id = target.media_desired_target_profile_id
       )
    OR EXISTS (
           SELECT 1
             FROM media_job job
            WHERE job.intent_desired_target_profile_id = target.media_desired_target_profile_id
       );

CREATE FUNCTION media_desired_target_validate_and_activate_v1(
    media_desired_target_profile_id_input BIGINT
)
RETURNS INT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    target_enabled BOOLEAN;
    stream_count INT;
BEGIN
    SELECT enabled
      INTO target_enabled
      FROM media_desired_target_profile
     WHERE media_desired_target_profile_id = media_desired_target_profile_id_input
     FOR UPDATE;

    IF target_enabled IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    stream_count := media_desired_target_stream_count_bounded_v1(
        media_desired_target_profile_id_input
    );
    IF stream_count = 0 THEN
        RAISE EXCEPTION 'desired target has no streams'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_streams_required';
    END IF;
    IF stream_count > media_desired_target_stream_limit_v1() THEN
        RAISE EXCEPTION 'desired target exceeds stream limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_stream_limit_exceeded';
    END IF;

    UPDATE media_desired_target_profile
       SET activated_at = COALESCE(activated_at, now())
     WHERE media_desired_target_profile_id = media_desired_target_profile_id_input;

    RETURN stream_count;
END;
$$;

CREATE FUNCTION media_desired_target_stream_insert_guard_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    target_activated_at TIMESTAMPTZ;
    target_enabled BOOLEAN;
    stream_count INT;
BEGIN
    SELECT activated_at, enabled
      INTO target_activated_at, target_enabled
      FROM media_desired_target_profile
     WHERE media_desired_target_profile_id = NEW.media_desired_target_profile_id
     FOR UPDATE;

    IF target_enabled IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;
    IF target_activated_at IS NOT NULL THEN
        RAISE EXCEPTION 'desired target version is immutable after activation'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_immutable';
    END IF;

    stream_count := media_desired_target_stream_count_bounded_v1(
        NEW.media_desired_target_profile_id
    );
    IF stream_count >= media_desired_target_stream_limit_v1() THEN
        RAISE EXCEPTION 'desired target exceeds stream limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_stream_limit_exceeded';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER media_desired_target_stream_insert_guard
BEFORE INSERT ON media_desired_target_stream
FOR EACH ROW
EXECUTE FUNCTION media_desired_target_stream_insert_guard_v1();

CREATE FUNCTION media_profile_desired_target_activation_guard_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    IF NEW.desired_target_profile_id IS NOT NULL
        AND NEW.desired_target_profile_id IS DISTINCT FROM OLD.desired_target_profile_id THEN
        PERFORM media_desired_target_validate_and_activate_v1(
            NEW.desired_target_profile_id
        );
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER media_profile_desired_target_activation_guard
BEFORE UPDATE OF desired_target_profile_id ON media_profile
FOR EACH ROW
EXECUTE FUNCTION media_profile_desired_target_activation_guard_v1();

CREATE FUNCTION media_job_desired_target_snapshot_guard_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    IF NEW.intent_desired_target_profile_id IS NOT NULL THEN
        PERFORM media_desired_target_validate_and_activate_v1(
            NEW.intent_desired_target_profile_id
        );
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER media_job_desired_target_snapshot_guard
BEFORE INSERT ON media_job
FOR EACH ROW
EXECUTE FUNCTION media_job_desired_target_snapshot_guard_v1();
