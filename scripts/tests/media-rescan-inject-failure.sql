-- Owned fixture: reject reset/publication for the missing source while allowing
-- present-source observations and a clean-completion increment.
ALTER TABLE public.media_discovery_source_fingerprint
    ADD CONSTRAINT revaer_test_absence_reset_failure
    CHECK (source_path <> 'nested/second.mkv' OR absence_observations <> 0) NOT VALID;
