-- Owned fixture proof: live job snapshots remain immutable before retention.
DO $guard$
DECLARE
    job_id_value BIGINT;
    detail_value TEXT;
    rejected BOOLEAN := FALSE;
BEGIN
    SELECT media_job_id INTO STRICT job_id_value
    FROM public.media_job
    WHERE media_job_public_id = current_setting('revaer_test.job_id')::UUID;
    IF (SELECT count(*) FROM public.media_job_root_snapshot
        WHERE media_job_id = job_id_value) <> 5 THEN
        RAISE EXCEPTION 'native job root snapshot is incomplete';
    END IF;
    BEGIN
        DELETE FROM public.media_job_root_snapshot WHERE media_job_id = job_id_value;
    EXCEPTION WHEN SQLSTATE 'P0001' THEN
        GET STACKED DIAGNOSTICS detail_value = PG_EXCEPTION_DETAIL;
        IF detail_value IS DISTINCT FROM 'media_job_snapshot_immutable' THEN
            RAISE;
        END IF;
        rejected := TRUE;
    END;
    IF NOT rejected THEN
        RAISE EXCEPTION 'live job root snapshot deletion was accepted';
    END IF;
    IF (SELECT count(*) FROM public.media_job_root_snapshot
        WHERE media_job_id = job_id_value) <> 5 THEN
        RAISE EXCEPTION 'rejected deletion altered native root snapshots';
    END IF;
END;
$guard$;
