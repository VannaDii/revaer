-- Restore the owned fixture after proving transactional fingerprint rollback.
ALTER TABLE public.media_job DROP CONSTRAINT revaer_test_fingerprint_insert_failure;
