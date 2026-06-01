CREATE OR REPLACE FUNCTION media_job_artifact_path_is_managed_v1(
    artifact_path_input TEXT
)
RETURNS BOOLEAN
LANGUAGE sql
IMMUTABLE
STRICT
AS $$
    SELECT
        artifact_path_input = btrim(artifact_path_input)
        AND artifact_path_input LIKE 'jobs/%'
        AND right(artifact_path_input, 1) <> '/'
        AND strpos(artifact_path_input, '//') = 0
        AND strpos(artifact_path_input, chr(92)) = 0
        AND NOT EXISTS (
            SELECT 1
            FROM unnest(string_to_array(artifact_path_input, '/')) AS segment(value)
            WHERE segment.value IN ('', '.', '..')
        );
$$;

ALTER TABLE media_job_artifact
    ADD CONSTRAINT media_job_artifact_path_managed CHECK (
        media_job_artifact_path_is_managed_v1(artifact_path)
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
    artifact_path_value TEXT;
BEGIN
    SELECT media_job_id INTO job_id
    FROM media_job
    WHERE media_job_public_id = media_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
    END IF;

    artifact_path_value := btrim(artifact_path_input);
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
        btrim(artifact_kind_input),
        artifact_path_value,
        size_bytes_input,
        NULLIF(btrim(content_type_input), '')
    )
    ON CONFLICT (media_job_id, artifact_index)
    DO UPDATE SET
        artifact_kind = EXCLUDED.artifact_kind,
        artifact_path = EXCLUDED.artifact_path,
        size_bytes = EXCLUDED.size_bytes,
        content_type = EXCLUDED.content_type;
END;
$$;
