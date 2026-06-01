CREATE TABLE IF NOT EXISTS media_job_artifact (
    media_job_artifact_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    artifact_index INT NOT NULL,
    artifact_kind TEXT NOT NULL,
    artifact_path TEXT NOT NULL,
    size_bytes BIGINT,
    content_type TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_artifact_index_nonnegative CHECK (artifact_index >= 0),
    CONSTRAINT media_job_artifact_kind_nonempty CHECK (btrim(artifact_kind) <> ''),
    CONSTRAINT media_job_artifact_path_nonempty CHECK (btrim(artifact_path) <> ''),
    CONSTRAINT media_job_artifact_size_nonnegative CHECK (
        size_bytes IS NULL OR size_bytes >= 0
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_job_artifact_job_index
    ON media_job_artifact (media_job_id, artifact_index);

CREATE TABLE IF NOT EXISTS media_job_compact_audit (
    media_job_compact_audit_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    audit_index INT NOT NULL,
    fact_kind TEXT NOT NULL,
    fact_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_compact_audit_index_nonnegative CHECK (audit_index >= 0),
    CONSTRAINT media_job_compact_audit_kind_nonempty CHECK (btrim(fact_kind) <> ''),
    CONSTRAINT media_job_compact_audit_text_nonempty CHECK (btrim(fact_text) <> '')
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_media_job_compact_audit_job_index
    ON media_job_compact_audit (media_job_id, audit_index);

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
BEGIN
    SELECT media_job_id INTO job_id
    FROM media_job
    WHERE media_job_public_id = media_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
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
        btrim(artifact_path_input),
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

CREATE OR REPLACE FUNCTION media_job_artifact_list_v1(
    media_job_public_id_input UUID
)
RETURNS TABLE (
    artifact_index INT,
    artifact_kind TEXT,
    artifact_path TEXT,
    size_bytes BIGINT,
    content_type TEXT,
    created_at TIMESTAMPTZ
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT
        mja.artifact_index,
        mja.artifact_kind,
        mja.artifact_path,
        mja.size_bytes,
        mja.content_type,
        mja.created_at
    FROM media_job_artifact mja
    JOIN media_job mj ON mj.media_job_id = mja.media_job_id
    WHERE mj.media_job_public_id = media_job_public_id_input
    ORDER BY mja.artifact_index;
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
BEGIN
    SELECT media_job_id INTO job_id
    FROM media_job
    WHERE media_job_public_id = media_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_not_found';
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
        btrim(fact_kind_input),
        btrim(fact_text_input)
    )
    ON CONFLICT (media_job_id, audit_index)
    DO UPDATE SET
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
        mjca.audit_index,
        mjca.fact_kind,
        mjca.fact_text,
        mjca.created_at
    FROM media_job_compact_audit mjca
    JOIN media_job mj ON mj.media_job_id = mjca.media_job_id
    WHERE mj.media_job_public_id = media_job_public_id_input
    ORDER BY mjca.audit_index;
$$;
