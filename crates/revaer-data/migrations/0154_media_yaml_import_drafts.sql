CREATE TABLE media_profile_import_draft (
    media_profile_import_draft_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_profile_import_draft_public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    profile_key TEXT NOT NULL,
    source_root TEXT NOT NULL,
    output_root TEXT NOT NULL,
    source_root_resolved BOOLEAN NOT NULL,
    output_root_resolved BOOLEAN NOT NULL,
    retention_days INT NOT NULL,
    compatibility_target_key TEXT,
    desired_target_key TEXT,
    desired_target_version INT,
    policy_key TEXT NOT NULL,
    created_by_user_id BIGINT NOT NULL REFERENCES app_user(user_id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_profile_import_draft_key_nonempty CHECK (btrim(profile_key) <> ''),
    CONSTRAINT media_profile_import_draft_roots_nonempty CHECK (
        btrim(source_root) <> '' AND btrim(output_root) <> ''
    ),
    CONSTRAINT media_profile_import_draft_unresolved CHECK (
        NOT source_root_resolved OR NOT output_root_resolved
    ),
    CONSTRAINT media_profile_import_draft_retention_bounds CHECK (
        retention_days BETWEEN 1 AND 3650
    ),
    CONSTRAINT media_profile_import_draft_policy_nonempty CHECK (btrim(policy_key) <> ''),
    CONSTRAINT media_profile_import_draft_desired_target_complete CHECK (
        (desired_target_key IS NULL AND desired_target_version IS NULL)
        OR
        (desired_target_key IS NOT NULL
            AND btrim(desired_target_key) <> ''
            AND desired_target_version IS NOT NULL
            AND desired_target_version > 0)
    )
);

CREATE UNIQUE INDEX uq_media_profile_import_draft_key
    ON media_profile_import_draft ((lower(profile_key)));

CREATE OR REPLACE FUNCTION media_profile_import_draft_upsert_v1(
    actor_public_id_input UUID,
    profile_key_input TEXT,
    source_root_input TEXT,
    output_root_input TEXT,
    source_root_resolved_input BOOLEAN,
    output_root_resolved_input BOOLEAN,
    retention_days_input INT,
    compatibility_target_key_input TEXT,
    desired_target_key_input TEXT,
    desired_target_version_input INT,
    policy_key_input TEXT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    draft_public_id UUID;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    IF COALESCE(source_root_resolved_input, FALSE)
       AND COALESCE(output_root_resolved_input, FALSE) THEN
        RAISE EXCEPTION 'draft has no unresolved path'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_yaml_draft_paths_resolved';
    END IF;

    INSERT INTO media_profile_import_draft (
        profile_key,
        source_root,
        output_root,
        source_root_resolved,
        output_root_resolved,
        retention_days,
        compatibility_target_key,
        desired_target_key,
        desired_target_version,
        policy_key,
        created_by_user_id
    ) VALUES (
        btrim(profile_key_input),
        btrim(source_root_input),
        btrim(output_root_input),
        COALESCE(source_root_resolved_input, FALSE),
        COALESCE(output_root_resolved_input, FALSE),
        retention_days_input,
        NULLIF(btrim(compatibility_target_key_input), ''),
        NULLIF(btrim(desired_target_key_input), ''),
        desired_target_version_input,
        btrim(policy_key_input),
        actor_id
    )
    ON CONFLICT ((lower(profile_key)))
    DO UPDATE SET
        source_root = EXCLUDED.source_root,
        output_root = EXCLUDED.output_root,
        source_root_resolved = EXCLUDED.source_root_resolved,
        output_root_resolved = EXCLUDED.output_root_resolved,
        retention_days = EXCLUDED.retention_days,
        compatibility_target_key = EXCLUDED.compatibility_target_key,
        desired_target_key = EXCLUDED.desired_target_key,
        desired_target_version = EXCLUDED.desired_target_version,
        policy_key = EXCLUDED.policy_key,
        created_by_user_id = EXCLUDED.created_by_user_id,
        updated_at = now()
    RETURNING media_profile_import_draft_public_id INTO draft_public_id;

    RETURN draft_public_id;
END;
$$;

CREATE OR REPLACE FUNCTION media_profile_import_draft_delete_v1(profile_key_input TEXT)
RETURNS BOOLEAN
LANGUAGE sql
AS $$
    WITH removed AS (
        DELETE FROM media_profile_import_draft
         WHERE lower(profile_key) = lower(btrim(profile_key_input))
        RETURNING 1
    )
    SELECT EXISTS (SELECT 1 FROM removed);
$$;

CREATE OR REPLACE FUNCTION media_profile_import_draft_list_v1()
RETURNS TABLE (
    media_profile_import_draft_public_id UUID,
    profile_key TEXT,
    source_root TEXT,
    output_root TEXT,
    source_root_resolved BOOLEAN,
    output_root_resolved BOOLEAN,
    retention_days INT,
    compatibility_target_key TEXT,
    desired_target_key TEXT,
    desired_target_version INT,
    policy_key TEXT,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT draft.media_profile_import_draft_public_id,
           draft.profile_key,
           draft.source_root,
           draft.output_root,
           draft.source_root_resolved,
           draft.output_root_resolved,
           draft.retention_days,
           draft.compatibility_target_key,
           draft.desired_target_key,
           draft.desired_target_version,
           draft.policy_key,
           draft.updated_at
      FROM media_profile_import_draft draft
     ORDER BY lower(draft.profile_key);
$$;
