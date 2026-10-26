DO $$ BEGIN
    IF (SELECT count(*) FROM outbound_request_log log
        JOIN indexer_instance inst USING (indexer_instance_id)
        WHERE inst.indexer_instance_public_id = current_setting('revaer_test.rss_source')::uuid
          AND log.request_type = 'rss' AND log.error_class = 'rate_limited'
          AND log.rate_limit_denied_scope::text = current_setting('revaer_test.rss_denied_scope')
          AND log.result_count = 0 AND log.latency_ms = 0 AND NOT log.parse_ok) <> 1
    THEN RAISE EXCEPTION 'Expected one correctly scoped RSS budget denial'; END IF;
END $$;
