CREATE OR REPLACE FUNCTION media_job_cleanup_failed_terminal_diagnostics_v1(
    as_of_input TIMESTAMPTZ,
    retention_days_input INT DEFAULT 30
)
RETURNS INTEGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    IF as_of_input IS NULL THEN
        RAISE EXCEPTION 'media diagnostic cleanup timestamp is required'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_diagnostic_cleanup_as_of_required';
    END IF;

    IF retention_days_input IS NULL OR retention_days_input < 1 OR retention_days_input > 3650 THEN
        RAISE EXCEPTION 'media diagnostic cleanup retention is out of bounds'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_diagnostic_cleanup_retention_invalid';
    END IF;

    WITH expired_jobs AS (
        SELECT media_job_id
        FROM media_job
        WHERE status IN ('failed'::media_job_status, 'cancelled'::media_job_status)
          AND completed_at IS NOT NULL
          AND completed_at <= as_of_input - make_interval(days => retention_days_input)
    ),
    deleted_violations AS (
        DELETE FROM media_job_violation mjv
        USING expired_jobs ej
        WHERE mjv.media_job_id = ej.media_job_id
        RETURNING 1
    ),
    deleted_plan_reasons AS (
        DELETE FROM media_job_plan_reason mjpr
        USING expired_jobs ej
        WHERE mjpr.media_job_id = ej.media_job_id
        RETURNING 1
    ),
    deleted_verification_checks AS (
        DELETE FROM media_job_verification_check mjvc
        USING expired_jobs ej
        WHERE mjvc.media_job_id = ej.media_job_id
        RETURNING 1
    ),
    deleted_artifacts AS (
        DELETE FROM media_job_artifact mja
        USING expired_jobs ej
        WHERE mja.media_job_id = ej.media_job_id
        RETURNING 1
    ),
    deleted_audits AS (
        DELETE FROM media_job_compact_audit mjca
        USING expired_jobs ej
        WHERE mjca.media_job_id = ej.media_job_id
        RETURNING 1
    )
    SELECT (
        (SELECT COUNT(*) FROM deleted_violations)
        + (SELECT COUNT(*) FROM deleted_plan_reasons)
        + (SELECT COUNT(*) FROM deleted_verification_checks)
        + (SELECT COUNT(*) FROM deleted_artifacts)
        + (SELECT COUNT(*) FROM deleted_audits)
    )::INTEGER
    INTO deleted_count;

    RETURN COALESCE(deleted_count, 0);
END;
$$;
