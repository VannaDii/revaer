CREATE TABLE IF NOT EXISTS media_job_plan_reason (
    media_job_plan_reason_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    reason_index INT NOT NULL,
    candidate_index INT,
    selected BOOLEAN NOT NULL DEFAULT FALSE,
    reason_code TEXT NOT NULL,
    reason_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_plan_reason_index_nonnegative CHECK (reason_index >= 0),
    CONSTRAINT media_job_plan_reason_candidate_nonnegative CHECK (
        candidate_index IS NULL OR candidate_index >= 0
    ),
    CONSTRAINT media_job_plan_reason_code_nonempty CHECK (btrim(reason_code) <> ''),
    CONSTRAINT media_job_plan_reason_text_nonempty CHECK (btrim(reason_text) <> '')
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_job_plan_reason_job_index
    ON media_job_plan_reason (media_job_id, reason_index);

CREATE OR REPLACE FUNCTION media_job_plan_reason_append_v1(
    media_job_public_id_input UUID,
    reason_index_input INT,
    candidate_index_input INT,
    selected_input BOOLEAN,
    reason_code_input TEXT,
    reason_text_input TEXT
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

    INSERT INTO media_job_plan_reason (
        media_job_id,
        reason_index,
        candidate_index,
        selected,
        reason_code,
        reason_text
    )
    VALUES (
        job_id,
        reason_index_input,
        candidate_index_input,
        COALESCE(selected_input, FALSE),
        btrim(reason_code_input),
        btrim(reason_text_input)
    )
    ON CONFLICT (media_job_id, reason_index)
    DO UPDATE SET
        candidate_index = EXCLUDED.candidate_index,
        selected = EXCLUDED.selected,
        reason_code = EXCLUDED.reason_code,
        reason_text = EXCLUDED.reason_text;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_plan_reason_list_v1(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    reason_index INT,
    candidate_index INT,
    selected BOOLEAN,
    reason_code TEXT,
    reason_text TEXT,
    created_at TIMESTAMPTZ
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT
        mjpr.reason_index,
        mjpr.candidate_index,
        mjpr.selected,
        mjpr.reason_code,
        mjpr.reason_text,
        mjpr.created_at
    FROM media_job_plan_reason mjpr
    JOIN media_job mj ON mj.media_job_id = mjpr.media_job_id
    WHERE mj.media_job_public_id = media_job_public_id_input
    ORDER BY mjpr.reason_index;
$$;
