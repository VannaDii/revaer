ALTER TABLE media_desired_target_container
    DROP CONSTRAINT media_desired_target_container_metadata_policy_known,
    ADD CONSTRAINT media_desired_target_container_metadata_policy_known CHECK (
        container_metadata_policy IN ('preserve', 'strip')
    );

ALTER TABLE media_job
    DROP CONSTRAINT media_job_intent_desired_target_complete,
    ADD CONSTRAINT media_job_intent_desired_target_complete CHECK (
        (intent_desired_target_profile_id IS NULL
            AND intent_desired_target_key IS NULL
            AND intent_desired_target_version IS NULL
            AND intent_desired_container_format IS NULL
            AND intent_desired_container_metadata_policy IS NULL)
        OR
        (intent_desired_target_profile_id IS NOT NULL
            AND btrim(intent_desired_target_key) <> ''
            AND intent_desired_target_version > 0
            AND btrim(intent_desired_container_format) <> ''
            AND intent_desired_container_metadata_policy IN ('preserve', 'strip'))
    );

CREATE OR REPLACE FUNCTION media_desired_target_create_v2(
    actor_public_id_input UUID,
    target_key_input TEXT,
    version_input INT,
    display_name_input TEXT,
    container_format_input TEXT,
    container_metadata_policy_input TEXT
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
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    metadata_policy_value := lower(COALESCE(NULLIF(btrim(container_metadata_policy_input), ''), 'preserve'));

    IF NULLIF(btrim(target_key_input), '') IS NULL
       OR COALESCE(version_input, 0) <= 0
       OR NULLIF(btrim(display_name_input), '') IS NULL
       OR NULLIF(btrim(container_format_input), '') IS NULL
       OR metadata_policy_value NOT IN ('preserve', 'strip') THEN
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
        container_metadata_policy
    )
    VALUES (target_id, lower(btrim(container_format_input)), metadata_policy_value);

    RETURN target_public_id;
END;
$$;
