-- Owned synthetic mode-transition fixture for fenced procedure tests.
-- Operator activation is exercised separately through normal configuration writers.
ALTER TABLE public.media_discovery_association_version
    DISABLE TRIGGER media_discovery_association_version_immutable_trigger;
DO $$
DECLARE changed_rows integer;
BEGIN
    UPDATE public.media_discovery_association_version
    SET schedule_enabled = TRUE
    WHERE media_discovery_association_version_id =
        (SELECT latest_media_discovery_association_version_id FROM public.media_discovery_association
         WHERE media_discovery_association_public_id = current_setting('revaer_test.association')::uuid);
    GET DIAGNOSTICS changed_rows = ROW_COUNT;
    IF changed_rows <> 1 THEN
        RAISE EXCEPTION 'synthetic rescan association is missing';
    END IF;
END;
$$;
ALTER TABLE public.media_discovery_association_version
    ENABLE TRIGGER media_discovery_association_version_immutable_trigger;
