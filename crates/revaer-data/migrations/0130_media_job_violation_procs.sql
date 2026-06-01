CREATE TABLE IF NOT EXISTS media_job_violation (
    media_job_violation_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    violation_index INT NOT NULL,
    violation_kind TEXT NOT NULL,
    severity TEXT NOT NULL,
    stream_id INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_violation_kind_nonempty CHECK (btrim(violation_kind) <> ''),
    CONSTRAINT media_job_violation_severity_valid CHECK (severity IN ('low', 'medium', 'high')),
    CONSTRAINT media_job_violation_index_nonnegative CHECK (violation_index >= 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_job_violation_job_index
    ON media_job_violation (media_job_id, violation_index);

CREATE OR REPLACE FUNCTION media_job_violation_append_v1(
    media_job_public_id_input UUID,
    violation_index_input INT,
    violation_kind_input TEXT,
    severity_input TEXT,
    stream_id_input INT DEFAULT NULL
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

    INSERT INTO media_job_violation (
        media_job_id,
        violation_index,
        violation_kind,
        severity,
        stream_id
    )
    VALUES (
        job_id,
        violation_index_input,
        btrim(violation_kind_input),
        lower(btrim(severity_input)),
        stream_id_input
    )
    ON CONFLICT (media_job_id, violation_index)
    DO UPDATE SET
        violation_kind = EXCLUDED.violation_kind,
        severity = EXCLUDED.severity,
        stream_id = EXCLUDED.stream_id;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_violation_list_v1(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    violation_index INT,
    violation_kind TEXT,
    severity TEXT,
    stream_id INT,
    created_at TIMESTAMPTZ
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT
        mjv.violation_index,
        mjv.violation_kind,
        mjv.severity,
        mjv.stream_id,
        mjv.created_at
    FROM media_job_violation mjv
    JOIN media_job mj ON mj.media_job_id = mjv.media_job_id
    WHERE mj.media_job_public_id = media_job_public_id_input
    ORDER BY mjv.violation_index;
$$;
