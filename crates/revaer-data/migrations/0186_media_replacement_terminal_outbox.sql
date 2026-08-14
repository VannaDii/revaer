CREATE TABLE media_job_terminal_outbox (
    media_job_terminal_outbox_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    event_kind TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ,
    CONSTRAINT media_job_terminal_outbox_job_unique UNIQUE (media_job_id),
    CONSTRAINT media_job_terminal_outbox_event_valid CHECK (event_kind = 'completed')
);

CREATE INDEX ix_media_job_terminal_outbox_unpublished
    ON media_job_terminal_outbox (created_at ASC, media_job_terminal_outbox_id ASC)
    WHERE published_at IS NULL;

CREATE OR REPLACE FUNCTION media_job_worker_commit_replacement_terminal_v1(
    media_job_public_id_input UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    job_id BIGINT;
    job_status media_job_status;
BEGIN
    SELECT media_job_id, status
      INTO job_id, job_status
      FROM media_job
     WHERE media_job_public_id = media_job_public_id_input
     FOR UPDATE;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_not_found';
    END IF;

    IF job_status NOT IN (
        media_job_status_running_v1(),
        media_job_status_verifying_v1(),
        media_job_status_completed_v1()
    ) THEN
        RAISE EXCEPTION 'media job replacement completion invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_replacement_terminal_invalid_status';
    END IF;

    INSERT INTO media_job_phase (
        media_job_id, phase_index, phase_name, phase_status, details_text
    ) VALUES (
        job_id, 1, 'execute', media_job_status_completed_v1(), NULL
    )
    ON CONFLICT (media_job_id, phase_index)
    DO UPDATE SET
        phase_name = EXCLUDED.phase_name,
        phase_status = EXCLUDED.phase_status,
        details_text = EXCLUDED.details_text;

    INSERT INTO media_job_phase (
        media_job_id, phase_index, phase_name, phase_status, details_text
    ) VALUES (
        job_id, 2, 'verify_replace', media_job_status_completed_v1(), NULL
    )
    ON CONFLICT (media_job_id, phase_index)
    DO UPDATE SET
        phase_name = EXCLUDED.phase_name,
        phase_status = EXCLUDED.phase_status,
        details_text = EXCLUDED.details_text;

    INSERT INTO media_job_verification_check (
        media_job_id, check_index, check_kind, check_status,
        expected_value, actual_value, details_text
    ) VALUES (
        job_id, 0, 'output_replacement', 'passed',
        'verified_atomic_replace', 'completed', NULL
    )
    ON CONFLICT (media_job_id, check_index)
    DO UPDATE SET
        check_kind = EXCLUDED.check_kind,
        check_status = EXCLUDED.check_status,
        expected_value = EXCLUDED.expected_value,
        actual_value = EXCLUDED.actual_value,
        details_text = EXCLUDED.details_text;

    UPDATE media_job
       SET status = media_job_status_completed_v1(),
           completed_at = COALESCE(completed_at, now()),
           last_error = NULL
     WHERE media_job_id = job_id;

    INSERT INTO media_job_terminal_outbox (media_job_id, event_kind)
    VALUES (job_id, 'completed')
    ON CONFLICT (media_job_id) DO NOTHING;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_terminal_outbox_list_unpublished_v1()
RETURNS TABLE (
    media_job_public_id UUID,
    event_kind TEXT
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT job.media_job_public_id, outbox.event_kind
      FROM media_job_terminal_outbox outbox
      JOIN media_job job ON job.media_job_id = outbox.media_job_id
     WHERE outbox.published_at IS NULL
     ORDER BY outbox.created_at ASC, outbox.media_job_terminal_outbox_id ASC
     LIMIT 1024
$$;

CREATE OR REPLACE FUNCTION media_job_terminal_outbox_mark_published_v1(
    media_job_public_id_input UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    UPDATE media_job_terminal_outbox outbox
       SET published_at = COALESCE(outbox.published_at, now())
      FROM media_job job
     WHERE job.media_job_id = outbox.media_job_id
       AND job.media_job_public_id = media_job_public_id_input;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'media job terminal outbox row not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_terminal_outbox_not_found';
    END IF;
END;
$$;
