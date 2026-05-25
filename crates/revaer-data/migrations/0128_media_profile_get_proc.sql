CREATE OR REPLACE FUNCTION media_profile_get_v1(
    media_profile_public_id_input UUID
)
RETURNS TABLE (
    media_profile_public_id UUID,
    profile_key TEXT,
    source_root TEXT,
    output_root TEXT,
    dry_run_only BOOLEAN,
    retention_days INT,
    updated_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
AS $$
    SELECT
        mp.media_profile_public_id,
        mp.profile_key,
        mp.source_root,
        mp.output_root,
        mp.dry_run_only,
        mp.retention_days,
        mp.updated_at
    FROM media_profile mp
    WHERE mp.media_profile_public_id = media_profile_public_id_input
      AND mp.deleted_at IS NULL;
$$;
