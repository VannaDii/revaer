DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM media_desired_target_stream
         WHERE stream_kind IN ('attachment', 'chapter')
    ) OR EXISTS (
        SELECT 1
          FROM media_job_desired_target_stream
         WHERE stream_kind IN ('attachment', 'chapter')
    ) THEN
        RAISE EXCEPTION 'unsupported desired target stream kind exists'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_stream_kind_unsupported';
    END IF;
END;
$$;

ALTER TABLE media_desired_target_stream
    DROP CONSTRAINT media_desired_target_stream_kind_known,
    ADD CONSTRAINT media_desired_target_stream_kind_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle')
    );

ALTER TABLE media_job_desired_target_stream
    DROP CONSTRAINT media_job_desired_target_stream_kind_known,
    ADD CONSTRAINT media_job_desired_target_stream_kind_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle')
    );
