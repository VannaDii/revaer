-- Owned fixture only: fail the job insert after the candidate fingerprint write.
ALTER TABLE public.media_job ADD CONSTRAINT revaer_test_fingerprint_insert_failure
    CHECK (intent_source_modified_ns <> 102);
