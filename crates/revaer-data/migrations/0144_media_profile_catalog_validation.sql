CREATE OR REPLACE FUNCTION media_profile_validate_catalog_refs_v1(
    compatibility_target_key_input TEXT,
    policy_key_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    compatibility_target_key_value TEXT;
    policy_key_value TEXT;
BEGIN
    compatibility_target_key_value := NULLIF(btrim(compatibility_target_key_input), '');
    policy_key_value := COALESCE(NULLIF(btrim(policy_key_input), ''), 'safe_dry_run');

    IF compatibility_target_key_value IS NOT NULL
       AND NOT EXISTS (
           SELECT 1
             FROM media_compatibility_target
            WHERE lower(compatibility_target_key) = lower(replace(compatibility_target_key_value, '_', '-'))
              AND enabled
       ) THEN
        RAISE EXCEPTION 'compatibility target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_not_found';
    END IF;

    IF NOT EXISTS (
           SELECT 1
             FROM media_policy_profile
            WHERE lower(policy_key) = lower(policy_key_value)
              AND enabled
       ) THEN
        RAISE EXCEPTION 'policy profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_profile_not_found';
    END IF;
END;
$$;
