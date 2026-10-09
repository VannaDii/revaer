-- Owned fixture only: a failed rescan publication must not advance cadence.
DO $$
DECLARE
    version_id bigint := current_setting('revaer_test.association_version')::bigint;
    due_before timestamptz;
BEGIN
    UPDATE public.media_discovery_rescan SET requested_sequence = 9223372036854775807
    WHERE media_discovery_association_version_id = version_id;
    UPDATE public.media_discovery_schedule_state SET next_due_at = anchor_due_at
    WHERE media_discovery_association_version_id = version_id;
    SELECT next_due_at INTO STRICT due_before FROM public.media_discovery_schedule_state
    WHERE media_discovery_association_version_id = version_id;
    BEGIN
        PERFORM public.media_discovery_schedule_observe_due_v1(
            current_setting('revaer_test.association')::uuid, 1,
            current_setting('revaer_test.generation')::bigint,
            decode(repeat('33', 32), 'hex'), 'schedule');
        RAISE EXCEPTION 'schedule publication overflow was accepted';
    EXCEPTION WHEN numeric_value_out_of_range THEN
        NULL; -- Subtransaction rolls back cadence and publication together.
    END;
    IF (SELECT next_due_at FROM public.media_discovery_schedule_state
        WHERE media_discovery_association_version_id = version_id) IS DISTINCT FROM due_before THEN
        RAISE EXCEPTION 'failed publication advanced schedule due time';
    END IF;
END;
$$;
