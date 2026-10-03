\if :verify_owner
SELECT EXISTS (
    SELECT 1 FROM pg_catalog.pg_database
    WHERE datname = :'database_name'
      AND pg_catalog.pg_get_userbyid(datdba) = :'database_name' || '_owner'
) AS owned \gset
\if :owned
\else
DO $ownership$
BEGIN
    RAISE EXCEPTION USING ERRCODE = 'P0001', MESSAGE = 'test_database_ownership_unverified';
END;
$ownership$;
\endif
\endif
SELECT format('DROP DATABASE %I WITH (FORCE)', :'database_name') \gexec
\if :drop_roles
SELECT format('DROP ROLE %I, %I', :'database_name' || '_runtime', :'database_name' || '_owner') \gexec
\endif
