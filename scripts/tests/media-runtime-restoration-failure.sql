-- Fail only the approved physical refresh in this owned runtime database.
CREATE FUNCTION public.fixture_restoration_failure() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.intent_source_identity IS DISTINCT FROM OLD.intent_source_identity
        OR NEW.intent_source_modified_ns IS DISTINCT FROM OLD.intent_source_modified_ns
        OR NEW.intent_source_changed_ns IS DISTINCT FROM OLD.intent_source_changed_ns THEN
        RAISE EXCEPTION 'injected restoration persistence failure'
            USING DETAIL = 'fixture_restoration_persistence_failed';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER fixture_restoration_failure BEFORE UPDATE ON public.media_job
    FOR EACH ROW EXECUTE FUNCTION public.fixture_restoration_failure();
