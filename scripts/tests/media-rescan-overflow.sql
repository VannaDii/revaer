-- Owned fixture only: verify overflow rolls back the candidate request mutation.
DO $$
DECLARE
    version_id bigint := current_setting('revaer_test.association_version')::bigint;
BEGIN
    BEGIN
        UPDATE public.media_discovery_rescan
        SET requested_sequence = 9223372036854775807
        WHERE media_discovery_association_version_id = version_id;
        PERFORM public.media_discovery_rescan_publish_v1(version_id, 'overflow');
        RAISE EXCEPTION 'rescan sequence overflow was accepted';
    EXCEPTION WHEN numeric_value_out_of_range THEN
        -- This exception rolls back both operations in the nested block.
        NULL;
    END;
END;
$$;
