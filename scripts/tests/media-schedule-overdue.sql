-- Owned fixture only: expose eleven overdue two-minute intervals.
UPDATE public.media_discovery_schedule_state
SET anchor_due_at = clock_timestamp() - interval '20 minutes',
    next_due_at = clock_timestamp() - interval '20 minutes'
WHERE media_discovery_association_version_id =
    current_setting('revaer_test.association_version')::bigint;
