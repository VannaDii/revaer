DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM search_request)
       OR EXISTS (SELECT 1 FROM canonical_torrent)
       OR EXISTS (SELECT 1 FROM acquisition_attempt)
    THEN RAISE EXCEPTION 'RSS must not create search, canonical or acquisition state'; END IF;
    IF (SELECT count(*) FROM outbound_request_log log
        JOIN indexer_instance inst USING (indexer_instance_id)
        WHERE inst.indexer_instance_public_id = current_setting('revaer_test.rss_source')::uuid
          AND log.request_type = 'rss' AND log.search_request_id IS NULL
          AND log.outcome = 'success' AND log.parse_ok AND log.result_count = 2) <> 1
    THEN RAISE EXCEPTION 'Expected exactly one RSS completion recording both parsed rows'; END IF;
    IF (SELECT count(*) FROM indexer_rss_item_seen) <> 1
    THEN RAISE EXCEPTION 'Repeated rows must insert one identifier'; END IF;
END $$;
