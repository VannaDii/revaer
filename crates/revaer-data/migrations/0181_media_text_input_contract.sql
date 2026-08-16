CREATE OR REPLACE FUNCTION media_key_valid_v1(value_input TEXT)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT value_input IS NOT NULL
       AND value_input = btrim(value_input)
       AND octet_length(value_input) BETWEEN 1 AND 128
       AND char_length(value_input) BETWEEN 1 AND 128
       AND value_input ~ '^[a-z0-9]([a-z0-9_-]*[a-z0-9])?$'
$$;

CREATE OR REPLACE FUNCTION media_display_valid_v1(value_input TEXT)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT value_input IS NOT NULL
       AND value_input = btrim(value_input)
       AND octet_length(value_input) BETWEEN 1 AND 256
       AND char_length(value_input) BETWEEN 1 AND 128
       AND value_input !~ '[[:cntrl:]]'
$$;

ALTER TABLE media_profile
    ADD CONSTRAINT media_profile_key_contract CHECK (media_key_valid_v1(profile_key)),
    ADD CONSTRAINT media_profile_compatibility_key_contract CHECK (
        compatibility_target_key IS NULL OR media_key_valid_v1(compatibility_target_key)
    ),
    ADD CONSTRAINT media_profile_policy_key_contract CHECK (media_key_valid_v1(policy_key));

ALTER TABLE media_target
    ADD CONSTRAINT media_target_key_contract CHECK (media_key_valid_v1(target_key));

ALTER TABLE media_compatibility_target
    ADD CONSTRAINT media_compatibility_target_key_contract CHECK (
        media_key_valid_v1(compatibility_target_key)
    ),
    ADD CONSTRAINT media_compatibility_target_display_contract CHECK (
        media_display_valid_v1(display_name)
    );

ALTER TABLE media_policy_profile
    ADD CONSTRAINT media_policy_profile_key_contract CHECK (media_key_valid_v1(policy_key)),
    ADD CONSTRAINT media_policy_profile_display_contract CHECK (
        media_display_valid_v1(display_name)
    );

ALTER TABLE media_job_retention_policy
    ADD CONSTRAINT media_job_retention_policy_key_contract CHECK (
        media_key_valid_v1(policy_key)
    );

ALTER TABLE media_desired_target_profile
    ADD CONSTRAINT media_desired_target_profile_key_contract CHECK (
        media_key_valid_v1(target_key)
    ),
    ADD CONSTRAINT media_desired_target_profile_display_contract CHECK (
        media_display_valid_v1(display_name)
    );

ALTER TABLE media_desired_target_stream
    ADD CONSTRAINT media_desired_target_stream_key_contract CHECK (
        media_key_valid_v1(stream_key)
    ),
    ADD CONSTRAINT media_desired_target_stream_title_contract CHECK (
        title IS NULL OR media_display_valid_v1(title)
    );

ALTER TABLE media_job_desired_target_stream
    ADD CONSTRAINT media_job_desired_target_stream_key_contract CHECK (
        media_key_valid_v1(stream_key)
    ),
    ADD CONSTRAINT media_job_desired_target_stream_title_contract CHECK (
        title IS NULL OR media_display_valid_v1(title)
    );

ALTER TABLE media_job
    ADD CONSTRAINT media_job_compatibility_key_contract CHECK (
        intent_compatibility_target_key IS NULL
        OR media_key_valid_v1(intent_compatibility_target_key)
    ),
    ADD CONSTRAINT media_job_policy_key_contract CHECK (
        intent_policy_key IS NULL OR media_key_valid_v1(intent_policy_key)
    ),
    ADD CONSTRAINT media_job_desired_target_key_contract CHECK (
        intent_desired_target_key IS NULL OR media_key_valid_v1(intent_desired_target_key)
    );
