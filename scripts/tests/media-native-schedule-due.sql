-- Owned service fixture only: simulate missed configured due intervals.
DO $$
DECLARE changed_rows integer;
BEGIN
    UPDATE public.media_discovery_schedule_state s
    SET anchor_due_at = clock_timestamp() - interval '20 minutes',
        next_due_at = clock_timestamp() - interval '20 minutes'
    FROM public.media_discovery_association a
    WHERE a.media_discovery_association_public_id = current_setting('revaer_test.association')::uuid
        AND s.media_discovery_association_version_id = a.latest_media_discovery_association_version_id;
    GET DIAGNOSTICS changed_rows = ROW_COUNT;
    IF changed_rows <> 1 THEN
        RAISE EXCEPTION 'owned service cadence is missing';
    END IF;
END;
$$;
