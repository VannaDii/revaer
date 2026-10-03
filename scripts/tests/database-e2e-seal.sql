BEGIN;
\i :init_file
SELECT * FROM revaer_system.seal_database_baseline_v1(
    1::smallint, decode(:'init_digest', 'hex'), :'runtime_role');
COMMIT;
