-- A minimal real initializer for the reusable fixture lifecycle contract.
-- Production baseline behavior is tested by the data crate on the media stack.
CREATE SCHEMA revaer_system;
CREATE TABLE revaer_system.fixture_seal (digest bytea NOT NULL);
CREATE FUNCTION revaer_system.seal_database_baseline_v1(smallint, bytea, text)
RETURNS void LANGUAGE plpgsql AS $seal$
BEGIN
    INSERT INTO revaer_system.fixture_seal VALUES ($2);
    EXECUTE format('GRANT USAGE ON SCHEMA revaer_system TO %I', $3);
    EXECUTE format('GRANT SELECT ON revaer_system.fixture_seal TO %I', $3);
END;
$seal$;
