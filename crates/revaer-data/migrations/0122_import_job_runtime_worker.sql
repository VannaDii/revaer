-- Runtime worker procedures for import job execution.

CREATE OR REPLACE FUNCTION import_job_worker_claim_next_v1()
RETURNS TABLE(
    import_job_public_id UUID,
    source import_source,
    is_dry_run BOOLEAN,
    config_detail VARCHAR
)
LANGUAGE plpgsql
AS $$
BEGIN
    RETURN QUERY
    SELECT j.import_job_public_id,
           j.source,
           j.is_dry_run,
           j.error_detail
    FROM import_job j
    WHERE j.status = 'running'
      AND j.finished_at IS NULL
    ORDER BY COALESCE(j.started_at, j.created_at) ASC,
             j.import_job_id ASC
    LIMIT 1;
END;
$$;

CREATE OR REPLACE FUNCTION import_job_worker_claim_next()
RETURNS TABLE(
    import_job_public_id UUID,
    source import_source,
    is_dry_run BOOLEAN,
    config_detail VARCHAR
)
LANGUAGE plpgsql
AS $$
BEGIN
    RETURN QUERY
    SELECT * FROM import_job_worker_claim_next_v1();
END;
$$;

CREATE OR REPLACE FUNCTION import_job_worker_record_result_v1(
    import_job_public_id_input UUID,
    prowlarr_identifier_input VARCHAR,
    status_input import_indexer_result_status,
    detail_input VARCHAR,
    resolved_is_enabled_input BOOLEAN,
    resolved_priority_input INTEGER,
    missing_secret_fields_input INTEGER
)
RETURNS VOID
LANGUAGE plpgsql
AS $$
DECLARE
    base_message CONSTANT text := 'Failed to record import job result';
    errcode CONSTANT text := 'P0001';
    job_id BIGINT;
BEGIN
    IF import_job_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_missing';
    END IF;

    IF prowlarr_identifier_input IS NULL OR btrim(prowlarr_identifier_input) = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'prowlarr_identifier_missing';
    END IF;

    SELECT import_job_id
    INTO job_id
    FROM import_job
    WHERE import_job_public_id = import_job_public_id_input
      AND status = 'running'
      AND finished_at IS NULL;

    IF job_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_running';
    END IF;

    INSERT INTO import_indexer_result (
        import_job_id,
        prowlarr_identifier,
        status,
        detail,
        resolved_is_enabled,
        resolved_priority,
        missing_secret_fields
    )
    VALUES (
        job_id,
        btrim(prowlarr_identifier_input),
        status_input,
        detail_input,
        resolved_is_enabled_input,
        resolved_priority_input,
        COALESCE(missing_secret_fields_input, 0)
    )
    ON CONFLICT (import_job_id, prowlarr_identifier)
    DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION import_job_worker_record_result(
    import_job_public_id_input UUID,
    prowlarr_identifier_input VARCHAR,
    status_input import_indexer_result_status,
    detail_input VARCHAR,
    resolved_is_enabled_input BOOLEAN,
    resolved_priority_input INTEGER,
    missing_secret_fields_input INTEGER
)
RETURNS VOID
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM import_job_worker_record_result_v1(
        import_job_public_id_input,
        prowlarr_identifier_input,
        status_input,
        detail_input,
        resolved_is_enabled_input,
        resolved_priority_input,
        missing_secret_fields_input
    );
END;
$$;

CREATE OR REPLACE FUNCTION import_job_worker_mark_terminal_v1(
    import_job_public_id_input UUID,
    status_input import_job_status,
    error_detail_input VARCHAR
)
RETURNS VOID
LANGUAGE plpgsql
AS $$
DECLARE
    base_message CONSTANT text := 'Failed to mark import job terminal';
    errcode CONSTANT text := 'P0001';
    job_id BIGINT;
BEGIN
    IF import_job_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_missing';
    END IF;

    IF status_input NOT IN ('completed', 'failed', 'canceled') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'terminal_status_invalid';
    END IF;

    SELECT import_job_id
    INTO job_id
    FROM import_job
    WHERE import_job_public_id = import_job_public_id_input
      AND status = 'running'
      AND finished_at IS NULL;

    IF job_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_running';
    END IF;

    UPDATE import_job
    SET status = status_input,
        finished_at = now(),
        error_detail = error_detail_input
    WHERE import_job_id = job_id;
END;
$$;

CREATE OR REPLACE FUNCTION import_job_worker_mark_terminal(
    import_job_public_id_input UUID,
    status_input import_job_status,
    error_detail_input VARCHAR
)
RETURNS VOID
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM import_job_worker_mark_terminal_v1(
        import_job_public_id_input,
        status_input,
        error_detail_input
    );
END;
$$;
