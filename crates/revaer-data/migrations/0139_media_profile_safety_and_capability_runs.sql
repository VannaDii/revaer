ALTER TABLE media_capability_snapshot
    ADD COLUMN snapshot_run_public_id UUID;

UPDATE media_capability_snapshot
   SET snapshot_run_public_id = gen_random_uuid()
 WHERE snapshot_run_public_id IS NULL;

ALTER TABLE media_capability_snapshot
    ALTER COLUMN snapshot_run_public_id SET NOT NULL;

CREATE INDEX ix_media_capability_snapshot_run_observed
    ON media_capability_snapshot (snapshot_run_public_id, observed_at DESC, media_capability_snapshot_id DESC);

CREATE OR REPLACE FUNCTION media_capability_snapshot_record_v2(
    actor_public_id_input UUID,
    snapshot_run_public_id_input UUID,
    ffmpeg_version_input TEXT,
    ffprobe_version_input TEXT,
    codec_name_input TEXT,
    encode_supported_input BOOLEAN,
    decode_supported_input BOOLEAN
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    snapshot_id_out BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    INSERT INTO media_capability_snapshot (
        snapshot_run_public_id,
        ffmpeg_version,
        ffprobe_version,
        codec_name,
        encode_supported,
        decode_supported,
        observed_by_user_id
    )
    VALUES (
        COALESCE(snapshot_run_public_id_input, gen_random_uuid()),
        btrim(ffmpeg_version_input),
        btrim(ffprobe_version_input),
        btrim(codec_name_input),
        COALESCE(encode_supported_input, FALSE),
        COALESCE(decode_supported_input, TRUE),
        actor_id
    )
    RETURNING media_capability_snapshot_id
    INTO snapshot_id_out;

    RETURN snapshot_id_out;
END;
$$;

CREATE OR REPLACE FUNCTION media_capability_snapshot_latest_v2()
RETURNS TABLE (
    media_capability_snapshot_id BIGINT,
    snapshot_run_public_id UUID,
    ffmpeg_version TEXT,
    ffprobe_version TEXT,
    codec_name TEXT,
    encode_supported BOOLEAN,
    decode_supported BOOLEAN,
    observed_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    WITH latest_run AS (
        SELECT mcs.snapshot_run_public_id
        FROM media_capability_snapshot mcs
        GROUP BY mcs.snapshot_run_public_id
        ORDER BY max(mcs.observed_at) DESC, max(mcs.media_capability_snapshot_id) DESC
        LIMIT 1
    )
    SELECT
        mcs.media_capability_snapshot_id,
        mcs.snapshot_run_public_id,
        mcs.ffmpeg_version,
        mcs.ffprobe_version,
        mcs.codec_name,
        mcs.encode_supported,
        mcs.decode_supported,
        mcs.observed_at
    FROM media_capability_snapshot mcs
    JOIN latest_run lr ON lr.snapshot_run_public_id = mcs.snapshot_run_public_id
    ORDER BY lower(mcs.codec_name) ASC, mcs.media_capability_snapshot_id ASC;
$$;
