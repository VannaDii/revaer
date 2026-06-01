CREATE TABLE IF NOT EXISTS media_job_verification_check (
    media_job_verification_check_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    check_index INT NOT NULL,
    check_kind TEXT NOT NULL,
    check_status TEXT NOT NULL,
    expected_value TEXT,
    actual_value TEXT,
    details_text TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_verification_check_index_nonnegative CHECK (check_index >= 0),
    CONSTRAINT media_job_verification_check_kind_nonempty CHECK (btrim(check_kind) <> ''),
    CONSTRAINT media_job_verification_check_status_valid CHECK (
        check_status IN ('passed', 'failed', 'skipped')
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_job_verification_check_job_index
    ON media_job_verification_check (media_job_id, check_index);

CREATE OR REPLACE FUNCTION media_job_verification_check_append_v1(
    media_job_public_id_input UUID,
    check_index_input INT,
    check_kind_input TEXT,
    check_status_input TEXT,
    expected_value_input TEXT DEFAULT NULL,
    actual_value_input TEXT DEFAULT NULL,
    details_text_input TEXT DEFAULT NULL
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
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
    END IF;

    INSERT INTO media_job_verification_check (
        media_job_id,
        check_index,
        check_kind,
        check_status,
        expected_value,
        actual_value,
        details_text
    )
    VALUES (
        job_id,
        check_index_input,
        btrim(check_kind_input),
        lower(btrim(check_status_input)),
        NULLIF(btrim(expected_value_input), ''),
        NULLIF(btrim(actual_value_input), ''),
        NULLIF(btrim(details_text_input), '')
    )
    ON CONFLICT (media_job_id, check_index)
    DO UPDATE SET
        check_kind = EXCLUDED.check_kind,
        check_status = EXCLUDED.check_status,
        expected_value = EXCLUDED.expected_value,
        actual_value = EXCLUDED.actual_value,
        details_text = EXCLUDED.details_text;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_verification_check_list_v1(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    check_index INT,
    check_kind TEXT,
    check_status TEXT,
    expected_value TEXT,
    actual_value TEXT,
    details_text TEXT,
    created_at TIMESTAMPTZ
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT
        mjvc.check_index,
        mjvc.check_kind,
        mjvc.check_status,
        mjvc.expected_value,
        mjvc.actual_value,
        mjvc.details_text,
        mjvc.created_at
    FROM media_job_verification_check mjvc
    JOIN media_job mj ON mj.media_job_id = mjvc.media_job_id
    WHERE mj.media_job_public_id = media_job_public_id_input
    ORDER BY mjvc.check_index;
$$;
