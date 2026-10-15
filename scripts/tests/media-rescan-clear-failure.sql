-- Remove the owned publication failure after proving it cannot confirm absence.
ALTER TABLE public.media_discovery_source_fingerprint
    DROP CONSTRAINT revaer_test_absence_reset_failure;
