-- Owned media fixture only: hold a source lock or inspect the waiting writer.
\if :hold_source
BEGIN;
SELECT pg_advisory_xact_lock(hashtextextended(
    'media_discovery_association_overlap_v1', a.media_root_catalog_slot_attestation_id))
FROM public.media_root_catalog_state c
JOIN public.media_root_catalog_slot_attestation a
    ON a.media_root_catalog_generation_id = c.active_media_root_catalog_generation_id
JOIN public.media_root_catalog_slot s USING (media_root_catalog_slot_id)
WHERE c.media_root_catalog_state_id = 1 AND s.logical_key = 'source';
SELECT 'LOCK_READY';
\else
WITH waiting AS (
    SELECT pid FROM pg_stat_activity
    WHERE datname = current_database() AND wait_event = 'advisory'
        AND query LIKE 'SELECT media_profile_version_replace_v1(%'
)
SELECT (SELECT count(*) FROM waiting), count(*)
FROM pg_locks l JOIN waiting w USING (pid)
WHERE l.relation IN ('public.media_desired_target_profile'::regclass,
    'public.media_policy_profile'::regclass, 'public.media_profile'::regclass);
\endif
