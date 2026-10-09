-- Deliberate corruption of the single job in an owned runtime fixture.
-- No production/runtime procedure may mutate immutable job snapshots.
DO $$
BEGIN
    IF (SELECT count(*) FROM public.media_job) <> 1 THEN
        RAISE EXCEPTION 'missing-cost fixture requires exactly one job';
    END IF;
END;
$$;
ALTER TABLE public.media_job_policy_operation_cost_snapshot
    DISABLE TRIGGER media_job_policy_operation_cost_snapshot_immutable_trigger;
DELETE FROM public.media_job_policy_operation_cost_snapshot;
ALTER TABLE public.media_job_policy_operation_cost_snapshot
    ENABLE TRIGGER media_job_policy_operation_cost_snapshot_immutable_trigger;
