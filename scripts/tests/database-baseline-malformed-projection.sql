-- Negative transport fixture only, applied to a private disposable test database.
-- Integer role columns must fail decoding, even when a text cast would match.
CREATE SCHEMA revaer_system;
CREATE FUNCTION revaer_system.read_database_baseline_v1()
RETURNS TABLE (
    contract_version smallint,
    init_sha256 bytea,
    postgres_version_num integer,
    schema_owner_role integer,
    runtime_role integer,
    sealed_at timestamptz
)
LANGUAGE sql
AS $malformed_projection$
    SELECT 1::smallint, decode(repeat('5a', 32), 'hex'), 180006, 1, 2,
        transaction_timestamp();
$malformed_projection$;
