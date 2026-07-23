CREATE OR REPLACE FUNCTION media_retention_mode_age_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'age'
$$;

CREATE OR REPLACE FUNCTION media_retention_mode_count_v1()
RETURNS TEXT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 'count'
$$;

ALTER TABLE media_job_retention_policy
    ADD COLUMN completed_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN completed_mode TEXT NOT NULL DEFAULT media_retention_mode_age_v1(),
    ADD COLUMN completed_limit INT NOT NULL DEFAULT 30,
    ADD COLUMN failed_diagnostic_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN failed_diagnostic_mode TEXT NOT NULL DEFAULT media_retention_mode_age_v1(),
    ADD COLUMN failed_diagnostic_limit INT NOT NULL DEFAULT 30,
    ADD CONSTRAINT media_job_retention_policy_completed_mode_known CHECK (
        completed_mode IN (media_retention_mode_age_v1(), media_retention_mode_count_v1())
    ),
    ADD CONSTRAINT media_job_retention_policy_completed_limit_bounds CHECK (
        completed_limit BETWEEN 1 AND 3650
    ),
    ADD CONSTRAINT media_job_retention_policy_failed_mode_known CHECK (
        failed_diagnostic_mode IN (media_retention_mode_age_v1(), media_retention_mode_count_v1())
    ),
    ADD CONSTRAINT media_job_retention_policy_failed_limit_bounds CHECK (
        failed_diagnostic_limit BETWEEN 1 AND 3650
    );

UPDATE media_job_retention_policy
SET completed_limit = completed_retention_days,
    failed_diagnostic_limit = failed_diagnostic_retention_days;

ALTER TABLE media_job_compact_audit
    ADD COLUMN media_job_public_id UUID;

UPDATE media_job_compact_audit audit
SET media_job_public_id = job.media_job_public_id
FROM media_job job
WHERE job.media_job_id = audit.media_job_id;

ALTER TABLE media_job_compact_audit
    ALTER COLUMN media_job_public_id SET NOT NULL,
    ALTER COLUMN media_job_id DROP NOT NULL,
    DROP CONSTRAINT media_job_compact_audit_media_job_id_fkey,
    ADD CONSTRAINT media_job_compact_audit_media_job_id_fkey
        FOREIGN KEY (media_job_id) REFERENCES media_job(media_job_id) ON DELETE SET NULL;

DROP INDEX uq_media_job_compact_audit_job_index;

CREATE UNIQUE INDEX uq_media_job_compact_audit_public_job_index
    ON media_job_compact_audit (media_job_public_id, audit_index);

CREATE OR REPLACE FUNCTION media_job_compact_audit_append_v1(
    media_job_public_id_input UUID,
    audit_index_input INT,
    fact_kind_input TEXT,
    fact_text_input TEXT
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
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_not_found';
    END IF;

    INSERT INTO media_job_compact_audit (
        media_job_id,
        media_job_public_id,
        audit_index,
        fact_kind,
        fact_text
    )
    VALUES (
        job_id,
        media_job_public_id_input,
        audit_index_input,
        btrim(fact_kind_input),
        btrim(fact_text_input)
    )
    ON CONFLICT (media_job_public_id, audit_index)
    DO UPDATE SET
        media_job_id = EXCLUDED.media_job_id,
        fact_kind = EXCLUDED.fact_kind,
        fact_text = EXCLUDED.fact_text;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_compact_audit_list_v1(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    audit_index INT,
    fact_kind TEXT,
    fact_text TEXT,
    created_at TIMESTAMPTZ
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT
        audit.audit_index,
        audit.fact_kind,
        audit.fact_text,
        audit.created_at
    FROM media_job_compact_audit audit
    WHERE audit.media_job_public_id = media_job_public_id_input
    ORDER BY audit.audit_index ASC;
$$;

CREATE OR REPLACE FUNCTION revaer_config.seed_media_configuration_defaults()
RETURNS VOID AS
$$
BEGIN
    INSERT INTO media_compatibility_target (
        compatibility_target_key,
        version,
        display_name,
        video_codec,
        audio_codec,
        subtitle_policy
    )
    VALUES
        ('hevc-aac', 1, 'HEVC/AAC', 'hevc', 'aac', media_subtitle_policy_selected_v1()),
        ('plex-apple-tv', 1, 'Plex Apple TV HEVC/AAC', 'hevc', 'aac', media_subtitle_policy_selected_v1()),
        ('plex-general-hevc-aac', 1, 'Plex General HEVC/AAC', 'hevc', 'aac', media_subtitle_policy_selected_v1())
    ON CONFLICT (lower(compatibility_target_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_codec = EXCLUDED.video_codec,
        audio_codec = EXCLUDED.audio_codec,
        subtitle_policy = EXCLUDED.subtitle_policy,
        enabled = TRUE,
        updated_at = now();

    INSERT INTO media_policy_profile (
        policy_key,
        version,
        display_name,
        video_intent
    )
    VALUES
        (media_policy_safe_dry_run_v1(), 1, 'Safe dry run', media_policy_general_v1()),
        (media_policy_general_v1(), 1, 'General media', media_policy_general_v1()),
        (media_policy_anime_v1(), 1, 'Anime', media_policy_anime_v1()),
        (media_policy_archival_v1(), 1, 'Archival', media_policy_archival_v1())
    ON CONFLICT (lower(policy_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_intent = EXCLUDED.video_intent,
        enabled = TRUE,
        updated_at = now();

    INSERT INTO media_job_retention_policy (
        policy_key,
        completed_retention_days,
        failed_diagnostic_retention_days,
        completed_enabled,
        completed_mode,
        completed_limit,
        failed_diagnostic_enabled,
        failed_diagnostic_mode,
        failed_diagnostic_limit
    )
    VALUES (
        media_retention_policy_default_v1(),
        30,
        30,
        FALSE,
        media_retention_mode_age_v1(),
        30,
        TRUE,
        media_retention_mode_age_v1(),
        30
    )
    ON CONFLICT (lower(policy_key)) DO UPDATE SET
        completed_retention_days = EXCLUDED.completed_retention_days,
        failed_diagnostic_retention_days = EXCLUDED.failed_diagnostic_retention_days,
        completed_enabled = EXCLUDED.completed_enabled,
        completed_mode = EXCLUDED.completed_mode,
        completed_limit = EXCLUDED.completed_limit,
        failed_diagnostic_enabled = EXCLUDED.failed_diagnostic_enabled,
        failed_diagnostic_mode = EXCLUDED.failed_diagnostic_mode,
        failed_diagnostic_limit = EXCLUDED.failed_diagnostic_limit,
        enabled = TRUE,
        updated_at = now();
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION media_job_retention_policy_get_v2()
RETURNS TABLE (
    completed_enabled BOOLEAN,
    completed_mode TEXT,
    completed_limit INT,
    failed_diagnostic_enabled BOOLEAN,
    failed_diagnostic_mode TEXT,
    failed_diagnostic_limit INT
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        policy.completed_enabled,
        policy.completed_mode,
        policy.completed_limit,
        policy.failed_diagnostic_enabled,
        policy.failed_diagnostic_mode,
        policy.failed_diagnostic_limit
    FROM media_job_retention_policy policy
    WHERE lower(policy.policy_key) = media_retention_policy_default_v1()
      AND policy.enabled
    ORDER BY policy.updated_at DESC, policy.media_job_retention_policy_id DESC
    LIMIT 1;
$$;

CREATE OR REPLACE FUNCTION media_job_retention_policy_update_v2(
    actor_public_id_input UUID,
    completed_enabled_input BOOLEAN,
    completed_mode_input TEXT,
    completed_limit_input INT,
    failed_diagnostic_enabled_input BOOLEAN,
    failed_diagnostic_mode_input TEXT,
    failed_diagnostic_limit_input INT
)
RETURNS TABLE (
    completed_enabled BOOLEAN,
    completed_mode TEXT,
    completed_limit INT,
    failed_diagnostic_enabled BOOLEAN,
    failed_diagnostic_mode TEXT,
    failed_diagnostic_limit INT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    normalized_completed_mode TEXT;
    normalized_failed_mode TEXT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    normalized_completed_mode := lower(btrim(completed_mode_input));
    normalized_failed_mode := lower(btrim(failed_diagnostic_mode_input));

    IF normalized_completed_mode NOT IN (
        media_retention_mode_age_v1(), media_retention_mode_count_v1()
    ) THEN
        RAISE EXCEPTION 'completed retention mode is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_completed_retention_mode_invalid';
    END IF;

    IF normalized_failed_mode NOT IN (
        media_retention_mode_age_v1(), media_retention_mode_count_v1()
    ) THEN
        RAISE EXCEPTION 'failed diagnostic retention mode is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_failed_retention_mode_invalid';
    END IF;

    IF completed_limit_input IS NULL OR completed_limit_input NOT BETWEEN 1 AND 3650 THEN
        RAISE EXCEPTION 'completed retention limit is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_completed_retention_limit_invalid';
    END IF;

    IF failed_diagnostic_limit_input IS NULL OR failed_diagnostic_limit_input NOT BETWEEN 1 AND 3650 THEN
        RAISE EXCEPTION 'failed diagnostic retention limit is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_failed_retention_limit_invalid';
    END IF;

    RETURN QUERY
    INSERT INTO media_job_retention_policy (
        policy_key,
        completed_retention_days,
        failed_diagnostic_retention_days,
        completed_enabled,
        completed_mode,
        completed_limit,
        failed_diagnostic_enabled,
        failed_diagnostic_mode,
        failed_diagnostic_limit,
        enabled,
        updated_at
    )
    VALUES (
        media_retention_policy_default_v1(),
        completed_limit_input,
        failed_diagnostic_limit_input,
        completed_enabled_input,
        normalized_completed_mode,
        completed_limit_input,
        failed_diagnostic_enabled_input,
        normalized_failed_mode,
        failed_diagnostic_limit_input,
        TRUE,
        now()
    )
    ON CONFLICT (lower(policy_key)) DO UPDATE SET
        completed_retention_days = EXCLUDED.completed_retention_days,
        failed_diagnostic_retention_days = EXCLUDED.failed_diagnostic_retention_days,
        completed_enabled = EXCLUDED.completed_enabled,
        completed_mode = EXCLUDED.completed_mode,
        completed_limit = EXCLUDED.completed_limit,
        failed_diagnostic_enabled = EXCLUDED.failed_diagnostic_enabled,
        failed_diagnostic_mode = EXCLUDED.failed_diagnostic_mode,
        failed_diagnostic_limit = EXCLUDED.failed_diagnostic_limit,
        enabled = TRUE,
        updated_at = now()
    RETURNING
        media_job_retention_policy.completed_enabled,
        media_job_retention_policy.completed_mode,
        media_job_retention_policy.completed_limit,
        media_job_retention_policy.failed_diagnostic_enabled,
        media_job_retention_policy.failed_diagnostic_mode,
        media_job_retention_policy.failed_diagnostic_limit;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_retention_run_v1(
    as_of_input TIMESTAMPTZ
)
RETURNS TABLE (
    completed_jobs_deleted INT,
    failed_jobs_pruned INT,
    failed_detail_rows_deleted INT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    policy media_job_retention_policy%ROWTYPE;
    completed_ids BIGINT[] := ARRAY[]::BIGINT[];
    failed_ids BIGINT[] := ARRAY[]::BIGINT[];
    completed_count INT := 0;
    failed_count INT := 0;
    detail_count INT := 0;
    affected_count INT := 0;
BEGIN
    IF as_of_input IS NULL THEN
        RAISE EXCEPTION 'media retention timestamp is required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_retention_as_of_required';
    END IF;

    SELECT * INTO policy
    FROM media_job_retention_policy retention
    WHERE lower(retention.policy_key) = media_retention_policy_default_v1()
      AND retention.enabled
    ORDER BY retention.updated_at DESC, retention.media_job_retention_policy_id DESC
    LIMIT 1;

    IF policy.media_job_retention_policy_id IS NULL THEN
        RAISE EXCEPTION 'media retention policy is missing'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_retention_policy_missing';
    END IF;

    IF policy.completed_enabled THEN
        WITH ranked AS (
            SELECT
                job.media_job_id,
                job.completed_at,
                row_number() OVER (
                    ORDER BY job.completed_at DESC, job.media_job_id DESC
                ) AS retained_rank
            FROM media_job job
            WHERE job.status = media_job_status_completed_v1()
              AND job.completed_at IS NOT NULL
        )
        SELECT COALESCE(array_agg(ranked.media_job_id), ARRAY[]::BIGINT[])
        INTO completed_ids
        FROM ranked
        WHERE (
            policy.completed_mode = media_retention_mode_age_v1()
            AND ranked.completed_at <= as_of_input - make_interval(days => policy.completed_limit)
        ) OR (
            policy.completed_mode = media_retention_mode_count_v1()
            AND ranked.retained_rank > policy.completed_limit
        );

        DELETE FROM media_job
        WHERE media_job_id = ANY(completed_ids);
        GET DIAGNOSTICS completed_count = ROW_COUNT;
    END IF;

    IF policy.failed_diagnostic_enabled THEN
        WITH ranked AS (
            SELECT
                job.media_job_id,
                job.completed_at,
                row_number() OVER (
                    ORDER BY job.completed_at DESC, job.media_job_id DESC
                ) AS retained_rank
            FROM media_job job
            WHERE job.status IN (media_job_status_failed_v1(), media_job_status_cancelled_v1())
              AND job.completed_at IS NOT NULL
        )
        SELECT COALESCE(array_agg(ranked.media_job_id), ARRAY[]::BIGINT[])
        INTO failed_ids
        FROM ranked
        WHERE (
            policy.failed_diagnostic_mode = media_retention_mode_age_v1()
            AND ranked.completed_at <= as_of_input - make_interval(days => policy.failed_diagnostic_limit)
        ) OR (
            policy.failed_diagnostic_mode = media_retention_mode_count_v1()
            AND ranked.retained_rank > policy.failed_diagnostic_limit
        );

        DELETE FROM media_job_phase WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;

        DELETE FROM media_job_operation WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;

        DELETE FROM media_job_violation WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;

        DELETE FROM media_job_plan_reason WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;

        DELETE FROM media_job_verification_check WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;

        DELETE FROM media_job_artifact WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;

        DELETE FROM media_job_desired_target_stream WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;

        UPDATE media_job
        SET last_error = NULL
        WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS failed_count = ROW_COUNT;
    END IF;

    RETURN QUERY SELECT completed_count, failed_count, detail_count;
END;
$$;

CREATE OR REPLACE FUNCTION media_job_retention_policy_get_v1()
RETURNS TABLE (
    completed_retention_days INT,
    failed_diagnostic_retention_days INT
)
LANGUAGE sql
STABLE
AS $$
    SELECT policy.completed_limit,
           policy.failed_diagnostic_limit
    FROM media_job_retention_policy policy
    WHERE lower(policy.policy_key) = media_retention_policy_default_v1()
      AND policy.enabled
    ORDER BY policy.updated_at DESC, policy.media_job_retention_policy_id DESC
    LIMIT 1;
$$;

CREATE OR REPLACE FUNCTION media_job_retention_policy_update_v1(
    actor_public_id_input UUID,
    completed_retention_days_input INT,
    failed_diagnostic_retention_days_input INT
)
RETURNS TABLE (
    completed_retention_days INT,
    failed_diagnostic_retention_days INT
)
LANGUAGE sql
AS $$
    SELECT policy.completed_limit,
           policy.failed_diagnostic_limit
    FROM media_job_retention_policy_update_v2(
        actor_public_id_input,
        TRUE,
        media_retention_mode_age_v1(),
        completed_retention_days_input,
        TRUE,
        media_retention_mode_age_v1(),
        failed_diagnostic_retention_days_input
    ) policy;
$$;
