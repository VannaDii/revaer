DO $$ BEGIN
    RAISE EXCEPTION USING ERRCODE = '40P01', MESSAGE = 'Controlled claim rollback';
END $$;
