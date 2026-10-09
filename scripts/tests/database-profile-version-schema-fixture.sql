-- Owned disposable schema tests only; no service or filesystem evidence.
DO $$
DECLARE
    actor_id bigint;
    target_id bigint;
    policy_id bigint;
    parent_id bigint;
    other_parent_id bigint;
    version_id bigint;
    invalid_kind smallint;
    invalid_version record;
    failed_constraint text;
BEGIN
    SELECT user_id INTO STRICT actor_id FROM public.app_user
    WHERE user_public_id = '00000000-0000-0000-0000-000000000000';
    SELECT media_policy_profile_id INTO STRICT policy_id FROM public.media_policy_profile
    WHERE policy_key = 'safe_dry_run' AND version = 1;
    INSERT INTO public.media_desired_target_profile (target_key, version, display_name, created_by_user_id)
    VALUES ('profile-schema-test', 1, 'Schema fixture', actor_id)
    RETURNING media_desired_target_profile_id INTO target_id;
    INSERT INTO public.media_profile (profile_key, source_root, output_root, created_by_user_id)
    VALUES ('profile-schema-first', '/schema-test/input-first', '/schema-test/output-first', actor_id)
    RETURNING media_profile_id INTO parent_id;
    INSERT INTO public.media_profile (profile_key, source_root, output_root, created_by_user_id)
    VALUES ('profile-schema-second', '/schema-test/input-second', '/schema-test/output-second', actor_id)
    RETURNING media_profile_id INTO other_parent_id;
    INSERT INTO public.media_profile_version (
        media_profile_id, version, lifecycle_state, display_name, description, enabled,
        dry_run_only, media_desired_target_profile_id, media_policy_profile_id, created_by_user_id
    ) VALUES (parent_id, 1, 'draft', ' Exact profile name ', '', false, true, target_id, policy_id, actor_id)
    RETURNING media_profile_version_id INTO version_id;
    UPDATE public.media_profile SET latest_media_profile_version_id = version_id
    WHERE media_profile_id = parent_id;
    BEGIN
        UPDATE public.media_profile SET latest_media_profile_version_id = version_id
        WHERE media_profile_id = other_parent_id;
        RAISE EXCEPTION 'cross-profile latest head accepted';
    EXCEPTION WHEN foreign_key_violation THEN
        GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
        IF failed_constraint <> 'media_profile_latest_version_fkey' THEN RAISE; END IF;
    END;
    BEGIN
        UPDATE public.media_profile SET active_media_profile_version_id = version_id
        WHERE media_profile_id = other_parent_id;
        RAISE EXCEPTION 'cross-profile active head accepted';
    EXCEPTION WHEN foreign_key_violation THEN
        GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
        IF failed_constraint <> 'media_profile_active_version_fkey' THEN RAISE; END IF;
    END;
    BEGIN
        INSERT INTO public.media_profile_version (
            media_profile_id, version, lifecycle_state, display_name, description, enabled,
            dry_run_only, media_desired_target_profile_id, media_policy_profile_id, created_by_user_id
        ) VALUES (parent_id, 1, 'draft', 'Duplicate', '', false, true, target_id, policy_id, actor_id);
        RAISE EXCEPTION 'duplicate profile version accepted';
    EXCEPTION WHEN unique_violation THEN
        GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
        IF failed_constraint <> 'media_profile_version_profile_version_key' THEN RAISE; END IF;
    END;
    FOR invalid_version IN SELECT * FROM (VALUES
        (0, 'draft', 'Name', '', 'media_profile_version_version_positive'),
        (2, 'unknown', 'Name', '', 'media_profile_version_lifecycle_known'),
        (2, 'draft', '', '', 'media_profile_version_display_contract'),
        (2, 'draft', repeat('x', 129), '', 'media_profile_version_display_contract'),
        (2, 'draft', repeat(chr(233), 65), '', 'media_profile_version_display_contract'),
        (2, 'draft', 'Name', repeat('x', 1025), 'media_profile_version_description_bounds')
    ) AS invalid(version, lifecycle, display_name, description, constraint_name) LOOP
        BEGIN
            INSERT INTO public.media_profile_version (
                media_profile_id, version, lifecycle_state, display_name, description, enabled,
                dry_run_only, media_desired_target_profile_id, media_policy_profile_id, created_by_user_id
            ) VALUES (parent_id, invalid_version.version, invalid_version.lifecycle,
                invalid_version.display_name, invalid_version.description, false, true, target_id, policy_id, actor_id);
            RAISE EXCEPTION 'invalid profile version accepted';
        EXCEPTION WHEN check_violation THEN
            GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
            IF failed_constraint <> invalid_version.constraint_name THEN RAISE; END IF;
        END;
    END LOOP;
    INSERT INTO public.media_profile_version (
        media_profile_id, version, lifecycle_state, display_name, description, enabled,
        dry_run_only, media_desired_target_profile_id, media_policy_profile_id, created_by_user_id
    ) VALUES (parent_id, 2, 'draft', repeat(chr(233), 64), repeat(chr(233), 512),
        false, true, target_id, policy_id, actor_id);
    IF NOT EXISTS (SELECT 1 FROM public.media_profile_version
        WHERE media_profile_version_id = version_id AND display_name = ' Exact profile name ') THEN
        RAISE EXCEPTION 'submitted display name was normalized';
    END IF;
    BEGIN
        UPDATE public.media_profile_version SET display_name = 'Changed'
        WHERE media_profile_version_id = version_id;
        RAISE EXCEPTION 'immutable profile changed';
    EXCEPTION WHEN check_violation THEN
        IF SQLERRM <> 'media_profile_version_immutable' THEN RAISE; END IF;
    END;
    BEGIN
        DELETE FROM public.media_profile_version WHERE media_profile_version_id = version_id;
        RAISE EXCEPTION 'immutable profile deleted';
    EXCEPTION WHEN check_violation THEN
        IF SQLERRM <> 'media_profile_version_immutable' THEN RAISE; END IF;
    END;
    INSERT INTO public.media_profile_version_root_binding (
        media_profile_version_id, media_root_kind_id, logical_key, resolution_state
    ) VALUES (version_id, 2, 'output-key', 'unmapped'), (version_id, 3, 'workspace-key', 'unmapped');
    FOR invalid_kind IN SELECT v FROM (VALUES (1::smallint), (6::smallint)) AS kinds(v) LOOP
        BEGIN
            INSERT INTO public.media_profile_version_root_binding (
                media_profile_version_id, media_root_kind_id, logical_key, resolution_state
            ) VALUES (version_id, invalid_kind, 'invalid-role', 'unmapped');
            RAISE EXCEPTION 'invalid profile root kind accepted';
        EXCEPTION WHEN check_violation THEN
            GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
            IF failed_constraint <> 'media_profile_version_root_binding_kind' THEN RAISE; END IF;
        END;
    END LOOP;
    BEGIN
        INSERT INTO public.media_profile_version_root_binding (
            media_profile_version_id, media_root_kind_id, logical_key, resolution_state
        ) VALUES (version_id, 4, 'backup-key', 'resolved');
        RAISE EXCEPTION 'resolved binding without attestation accepted';
    EXCEPTION WHEN check_violation THEN
        GET STACKED DIAGNOSTICS failed_constraint = CONSTRAINT_NAME;
        IF failed_constraint <> 'media_profile_version_root_binding_resolution_coherent' THEN RAISE; END IF;
    END;
    BEGIN
        UPDATE public.media_profile_version_root_binding SET logical_key = 'changed'
        WHERE media_profile_version_id = version_id;
        RAISE EXCEPTION 'immutable bindings changed';
    EXCEPTION WHEN check_violation THEN
        IF SQLERRM <> 'media_profile_version_immutable' THEN RAISE; END IF;
    END;
    BEGIN
        DELETE FROM public.media_profile_version_root_binding WHERE media_profile_version_id = version_id;
        RAISE EXCEPTION 'immutable bindings deleted';
    EXCEPTION WHEN check_violation THEN
        IF SQLERRM <> 'media_profile_version_immutable' THEN RAISE; END IF;
    END;
END;
$$;
