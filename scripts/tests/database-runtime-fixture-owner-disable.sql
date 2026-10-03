DO $fixture$
BEGIN
    IF current_database() !~ '^revaer_test_[0-9]+_[0-9]+$' THEN
        RAISE EXCEPTION 'fixture requires an owned disposable test database';
    END IF;
    EXECUTE format('ALTER ROLE %I NOLOGIN', current_database() || '_owner');
END;
$fixture$;
