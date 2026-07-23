CREATE TABLE media_job_terminal_outbox (
    media_job_terminal_outbox_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    media_job_attempt_id BIGINT NOT NULL,
    event_kind TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ,
    CONSTRAINT media_job_terminal_outbox_job_unique UNIQUE (media_job_id),
    CONSTRAINT media_job_terminal_outbox_attempt_fk
        FOREIGN KEY (media_job_id, media_job_attempt_id)
        REFERENCES media_job_attempt(media_job_id, media_job_attempt_id) ON DELETE CASCADE,
    CONSTRAINT media_job_terminal_outbox_event_valid CHECK (event_kind = 'completed')
);

CREATE INDEX ix_media_job_terminal_outbox_unpublished
    ON media_job_terminal_outbox (created_at ASC, media_job_terminal_outbox_id ASC)
    WHERE published_at IS NULL;

CREATE OR REPLACE FUNCTION media_job_worker_commit_replacement_terminal_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    observed_cancel_generation_input BIGINT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    job_id BIGINT;
    attempt_id BIGINT;
    job_status media_job_status;
    attempt_status media_job_status;
    cancelled BOOLEAN;
BEGIN
    SELECT job.media_job_id,
           attempt.media_job_attempt_id,
           job.status,
           attempt.status,
           job.cancel_generation > observed_cancel_generation_input
      INTO job_id, attempt_id, job_status, attempt_status, cancelled
      FROM media_job job
      JOIN media_job_attempt attempt
        ON attempt.media_job_attempt_id = job.current_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.claim_generation = claim_generation_input
       AND job.status = attempt.status
     FOR UPDATE OF job, attempt;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;

    IF job_status = media_job_status_completed_v1() THEN
        IF NOT EXISTS (
            SELECT 1
              FROM media_job_terminal_outbox outbox
             WHERE outbox.media_job_id = job_id
               AND outbox.event_kind = 'completed'
        ) THEN
            RAISE EXCEPTION 'completed replacement is missing terminal evidence'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_replacement_terminal_outbox_missing';
        END IF;
        RETURN FALSE;
    END IF;

    IF job_status NOT IN (media_job_status_running_v1(), media_job_status_verifying_v1()) THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;

    IF cancelled THEN
        INSERT INTO media_job_phase (
            media_job_id, media_job_attempt_id, phase_index, phase_name, phase_status, details_text
        ) VALUES (
            job_id, attempt_id, 98, 'runtime_cancellation',
            media_job_status_cancelled_v1(), 'media_job_cancelled_by_operator'
        )
        ON CONFLICT (media_job_attempt_id, phase_index)
        DO UPDATE SET
            phase_name = EXCLUDED.phase_name,
            phase_status = EXCLUDED.phase_status,
            details_text = EXCLUDED.details_text;
        INSERT INTO media_job_verification_check (
            media_job_id, media_job_attempt_id, check_index, check_kind, check_status,
            expected_value, actual_value, details_text
        ) VALUES (
            job_id, attempt_id, 98, 'cancellation', 'skipped',
            'continue', 'operator_cancelled', 'worker observed cancellation at replacement commit'
        )
        ON CONFLICT (media_job_attempt_id, check_index)
        DO UPDATE SET
            check_kind = EXCLUDED.check_kind,
            check_status = EXCLUDED.check_status,
            expected_value = EXCLUDED.expected_value,
            actual_value = EXCLUDED.actual_value,
            details_text = EXCLUDED.details_text;
        UPDATE media_job_attempt
           SET status = media_job_status_cancelled_v1(),
               completed_at = now(),
               last_error = NULL
         WHERE media_job_attempt_id = attempt_id;
        UPDATE media_job
           SET status = media_job_status_cancelled_v1(),
               cancel_acknowledged_generation = cancel_generation,
               completed_at = now(),
               last_error = NULL
         WHERE media_job_id = job_id;
        RETURN TRUE;
    END IF;

    INSERT INTO media_job_phase (
        media_job_id, media_job_attempt_id, phase_index, phase_name, phase_status, details_text
    ) VALUES (
        job_id, attempt_id, 1, 'execute', media_job_status_completed_v1(), NULL
    )
    ON CONFLICT (media_job_attempt_id, phase_index)
    DO UPDATE SET
        phase_name = EXCLUDED.phase_name,
        phase_status = EXCLUDED.phase_status,
        details_text = EXCLUDED.details_text;

    INSERT INTO media_job_phase (
        media_job_id, media_job_attempt_id, phase_index, phase_name, phase_status, details_text
    ) VALUES (
        job_id, attempt_id, 2, 'verify_replace', media_job_status_completed_v1(), NULL
    )
    ON CONFLICT (media_job_attempt_id, phase_index)
    DO UPDATE SET
        phase_name = EXCLUDED.phase_name,
        phase_status = EXCLUDED.phase_status,
        details_text = EXCLUDED.details_text;

    INSERT INTO media_job_verification_check (
        media_job_id, media_job_attempt_id, check_index, check_kind, check_status,
        expected_value, actual_value, details_text
    ) VALUES (
        job_id, attempt_id, 0, 'output_replacement', 'passed',
        'verified_atomic_replace', 'completed', NULL
    )
    ON CONFLICT (media_job_attempt_id, check_index)
    DO UPDATE SET
        check_kind = EXCLUDED.check_kind,
        check_status = EXCLUDED.check_status,
        expected_value = EXCLUDED.expected_value,
        actual_value = EXCLUDED.actual_value,
        details_text = EXCLUDED.details_text;

    UPDATE media_job_attempt
       SET status = media_job_status_completed_v1(),
           completed_at = COALESCE(completed_at, now()),
           last_error = NULL
     WHERE media_job_attempt_id = attempt_id;

    UPDATE media_job
       SET status = media_job_status_completed_v1(),
           completed_at = COALESCE(completed_at, now()),
           last_error = NULL
     WHERE media_job_id = job_id;

    INSERT INTO media_job_terminal_outbox (media_job_id, media_job_attempt_id, event_kind)
    VALUES (job_id, attempt_id, 'completed')
    ON CONFLICT (media_job_id) DO NOTHING;
    RETURN FALSE;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_terminal_outbox_list_unpublished_v1()
RETURNS TABLE (
    media_job_public_id UUID,
    claim_generation BIGINT,
    event_kind TEXT
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT job.media_job_public_id, attempt.claim_generation, outbox.event_kind
      FROM media_job_terminal_outbox outbox
      JOIN media_job job ON job.media_job_id = outbox.media_job_id
      JOIN media_job_attempt attempt
        ON attempt.media_job_attempt_id = outbox.media_job_attempt_id
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
