CREATE OR REPLACE FUNCTION media_job_operation_append_v1(
    media_job_public_id_input UUID,
    operation_index_input INT,
    operation_kind_input TEXT,
    stream_id_input INT,
    command_bin_input TEXT,
    arg_1_input TEXT,
    arg_2_input TEXT,
    arg_3_input TEXT,
    arg_4_input TEXT,
    arg_5_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    job_id BIGINT;
BEGIN
    SELECT media_job_id INTO job_id
    FROM media_job
    WHERE media_job_public_id = media_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
    END IF;

    INSERT INTO media_job_operation (
        media_job_id,
        operation_index,
        operation_kind,
        stream_id,
        command_bin,
        arg_1,
        arg_2,
        arg_3,
        arg_4,
        arg_5
    )
    VALUES (
        job_id,
        operation_index_input,
        btrim(operation_kind_input),
        stream_id_input,
        btrim(command_bin_input),
        NULLIF(btrim(arg_1_input), ''),
        NULLIF(btrim(arg_2_input), ''),
        NULLIF(btrim(arg_3_input), ''),
        NULLIF(btrim(arg_4_input), ''),
        NULLIF(btrim(arg_5_input), '')
    )
    ON CONFLICT (media_job_id, operation_index)
    DO UPDATE SET
        operation_kind = EXCLUDED.operation_kind,
        stream_id = EXCLUDED.stream_id,
        command_bin = EXCLUDED.command_bin,
        arg_1 = EXCLUDED.arg_1,
        arg_2 = EXCLUDED.arg_2,
        arg_3 = EXCLUDED.arg_3,
        arg_4 = EXCLUDED.arg_4,
        arg_5 = EXCLUDED.arg_5;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_operation_list_v1(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    operation_index INT,
    operation_kind TEXT,
    stream_id INT,
    command_bin TEXT,
    arg_1 TEXT,
    arg_2 TEXT,
    arg_3 TEXT,
    arg_4 TEXT,
    arg_5 TEXT,
    created_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        mjo.operation_index,
        mjo.operation_kind,
        mjo.stream_id,
        mjo.command_bin,
        mjo.arg_1,
        mjo.arg_2,
        mjo.arg_3,
        mjo.arg_4,
        mjo.arg_5,
        mjo.created_at
    FROM media_job_operation mjo
    JOIN media_job mj ON mj.media_job_id = mjo.media_job_id
    WHERE mj.media_job_public_id = media_job_public_id_input
    ORDER BY mjo.operation_index ASC;
$$;
