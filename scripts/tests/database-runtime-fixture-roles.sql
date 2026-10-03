DO $fixture$
DECLARE
    owner_name text := current_database() || '_owner';
    runtime_name text := current_database() || '_runtime';
    password_value text := current_setting('revaer_test.fixture_password');
BEGIN
    IF current_database() !~ '^revaer_test_[0-9]+_[0-9]+$' THEN
        RAISE EXCEPTION 'fixture requires an owned disposable test database';
    END IF;
    EXECUTE format('CREATE ROLE %I LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD %L', owner_name, password_value);
    EXECUTE format('CREATE ROLE %I LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD %L', runtime_name, password_value);
    EXECUTE format('ALTER DATABASE %I OWNER TO %I', current_database(), owner_name);
END;
$fixture$;
