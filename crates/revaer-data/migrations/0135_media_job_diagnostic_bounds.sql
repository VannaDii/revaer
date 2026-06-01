ALTER TABLE media_job_artifact
    ADD CONSTRAINT media_job_artifact_kind_bounded CHECK (
        char_length(btrim(artifact_kind)) <= 64
    ),
    ADD CONSTRAINT media_job_artifact_path_bounded CHECK (
        char_length(btrim(artifact_path)) <= 1024
    ),
    ADD CONSTRAINT media_job_artifact_content_type_bounded CHECK (
        content_type IS NULL OR char_length(btrim(content_type)) <= 128
    );

ALTER TABLE media_job_compact_audit
    ADD CONSTRAINT media_job_compact_audit_kind_bounded CHECK (
        char_length(btrim(fact_kind)) <= 64
    ),
    ADD CONSTRAINT media_job_compact_audit_text_bounded CHECK (
        char_length(btrim(fact_text)) <= 1024
    );

CREATE OR REPLACE FUNCTION media_job_artifact_append_v1(
    media_job_public_id_input UUID,
    artifact_index_input INT,
    artifact_kind_input TEXT,
    artifact_path_input TEXT,
    size_bytes_input BIGINT DEFAULT NULL,
    content_type_input TEXT DEFAULT NULL
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    job_id BIGINT;
    artifact_kind_value TEXT;
    artifact_path_value TEXT;
    content_type_value TEXT;
BEGIN
    SELECT media_job_id INTO job_id
    FROM media_job
    WHERE media_job_public_id = media_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
    END IF;

    artifact_kind_value := btrim(artifact_kind_input);
    artifact_path_value := btrim(artifact_path_input);
    content_type_value := NULLIF(btrim(content_type_input), '');

    IF char_length(artifact_kind_value) > 64 THEN
        RAISE EXCEPTION 'media job artifact kind is too long'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_artifact_kind_too_long';
    END IF;

    IF char_length(artifact_path_value) > 1024 THEN
        RAISE EXCEPTION 'media job artifact path is too long'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_artifact_path_too_long';
    END IF;

    IF content_type_value IS NOT NULL AND char_length(content_type_value) > 128 THEN
        RAISE EXCEPTION 'media job artifact content type is too long'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_artifact_content_type_too_long';
    END IF;

    IF NOT media_job_artifact_path_is_managed_v1(artifact_path_value) THEN
        RAISE EXCEPTION 'media job artifact path is not managed'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_artifact_path_invalid';
    END IF;

    INSERT INTO media_job_artifact (
        media_job_id,
        artifact_index,
        artifact_kind,
        artifact_path,
        size_bytes,
        content_type
    )
    VALUES (
        job_id,
        artifact_index_input,
        artifact_kind_value,
        artifact_path_value,
        size_bytes_input,
        content_type_value
    )
    ON CONFLICT (media_job_id, artifact_index)
    DO UPDATE SET
        artifact_kind = EXCLUDED.artifact_kind,
        artifact_path = EXCLUDED.artifact_path,
        size_bytes = EXCLUDED.size_bytes,
        content_type = EXCLUDED.content_type;
END;
$$;

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
    fact_kind_value TEXT;
    fact_text_value TEXT;
BEGIN
    SELECT media_job_id INTO job_id
    FROM media_job
    WHERE media_job_public_id = media_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
    END IF;

    fact_kind_value := btrim(fact_kind_input);
    fact_text_value := btrim(fact_text_input);

    IF char_length(fact_kind_value) > 64 THEN
        RAISE EXCEPTION 'media job compact audit kind is too long'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_compact_audit_kind_too_long';
    END IF;

    IF char_length(fact_text_value) > 1024 THEN
        RAISE EXCEPTION 'media job compact audit text is too long'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_compact_audit_text_too_long';
    END IF;

    INSERT INTO media_job_compact_audit (
        media_job_id,
        audit_index,
        fact_kind,
        fact_text
    )
    VALUES (
        job_id,
        audit_index_input,
        fact_kind_value,
        fact_text_value
    )
    ON CONFLICT (media_job_id, audit_index)
    DO UPDATE SET
        fact_kind = EXCLUDED.fact_kind,
        fact_text = EXCLUDED.fact_text;
END;
$$;
