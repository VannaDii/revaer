-- Synthetic reader evidence in an owned disposable database, not root proof.
DO $$
DECLARE
    parent_id bigint;
    version_id bigint;
BEGIN
    SELECT media_profile_id INTO STRICT parent_id FROM public.media_profile
    WHERE profile_key = 'profile-schema-second';
    INSERT INTO public.media_profile_version (
        media_profile_id, version, lifecycle_state, display_name, description, enabled,
        dry_run_only, media_desired_target_profile_id, media_policy_profile_id, created_by_user_id
    ) SELECT parent_id, 1, 'active', 'Disabled active profile', 'Exact reader fixture', false,
        false, t.media_desired_target_profile_id, p.media_policy_profile_id, t.created_by_user_id
    FROM public.media_desired_target_profile t CROSS JOIN public.media_policy_profile p
    WHERE t.target_key = 'profile-schema-test' AND t.version = 1
        AND p.policy_key = 'safe_dry_run' AND p.version = 1
    RETURNING media_profile_version_id INTO STRICT version_id;
    INSERT INTO public.media_profile_version_root_binding (
        media_profile_version_id, media_root_kind_id, logical_key,
        media_root_catalog_slot_attestation_id, resolution_state
    ) SELECT version_id, k.media_root_kind_id, s.logical_key,
        a.media_root_catalog_slot_attestation_id, 'resolved'
    FROM public.media_root_catalog_slot s
    JOIN public.media_root_catalog_slot_attestation a USING (media_root_catalog_slot_id)
    JOIN public.media_root_catalog_slot_kind k USING (media_root_catalog_slot_attestation_id)
    JOIN public.media_root_catalog_state state ON state.active_media_root_catalog_generation_id = a.media_root_catalog_generation_id
    WHERE state.media_root_catalog_state_id = 1
        AND ((s.logical_key = 'reader-1' AND k.media_root_kind_id = 2)
            OR (s.logical_key = 'reader-2' AND k.media_root_kind_id = 3));
    UPDATE public.media_profile SET latest_media_profile_version_id = version_id,
        active_media_profile_version_id = version_id WHERE media_profile_id = parent_id;
END;
$$;
