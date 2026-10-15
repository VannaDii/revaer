-- Synthetic save dependencies in an owned disposable database; no filesystem proof.
DO $$
DECLARE
    actor_id bigint;
    target_id bigint;
    policy_id bigint;
BEGIN
    SELECT user_id INTO STRICT actor_id FROM public.app_user
    WHERE user_public_id = '00000000-0000-0000-0000-000000000000';
    INSERT INTO public.media_desired_target_profile (target_key, version, display_name, created_by_user_id)
    VALUES ('profile-save-target', 1, 'Save fixture', actor_id)
    RETURNING media_desired_target_profile_id INTO target_id;
    INSERT INTO public.media_desired_target_stream (
        media_desired_target_profile_id, stream_key, stream_kind, semantic_role, sort_order, codec
    ) VALUES (target_id, 'video', 'video', 'primary', 0, 'h264');
    INSERT INTO public.media_policy_profile (policy_key, version, display_name)
    VALUES ('profile-save-policy', 1, 'Save fixture')
    RETURNING media_policy_profile_id INTO policy_id;
    UPDATE public.media_policy_output SET quarantine_enabled = false WHERE media_policy_profile_id = policy_id;
END;
$$;
