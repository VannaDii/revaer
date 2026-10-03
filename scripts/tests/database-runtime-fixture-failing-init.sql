-- Intentionally fail after DDL to exercise initializer rollback and fixture cleanup.
CREATE TABLE public.must_roll_back (id integer);
SELECT 1 / 0;
