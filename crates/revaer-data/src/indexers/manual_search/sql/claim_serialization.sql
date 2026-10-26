DO $$ BEGIN
    RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Controlled claim rollback';
END $$;
