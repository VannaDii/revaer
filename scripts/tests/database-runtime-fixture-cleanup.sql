DO $fixture$
DECLARE
    owner_name text := current_database() || '_owner';
    runtime_name text := current_database() || '_runtime';
BEGIN
    IF current_database() !~ '^revaer_test_[0-9]+_[0-9]+$' THEN
        RAISE EXCEPTION 'fixture requires an owned disposable test database';
    END IF;
    EXECUTE format('REASSIGN OWNED BY %I TO %I', owner_name, current_user);
    EXECUTE format('DROP OWNED BY %I', owner_name);
    EXECUTE format('DROP OWNED BY %I', runtime_name);
    EXECUTE format('DROP ROLE %I', runtime_name);
    EXECUTE format('DROP ROLE %I', owner_name);
END;
$fixture$;
