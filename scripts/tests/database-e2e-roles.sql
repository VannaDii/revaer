\getenv runtime_password REVAER_TEST_RUNTIME_PASSWORD
BEGIN;
SELECT set_config('revaer_test.fixture_password', :'runtime_password', true) AS configured \gset
\i :roles_file
COMMIT;
