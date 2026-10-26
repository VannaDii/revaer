-- Exhaust only the owned fixture's selected shared bucket across the minute boundary.
INSERT INTO rate_limit_state (scope_type, scope_id, window_start, tokens_used)
SELECT current_setting('revaer_test.rss_denied_scope')::rate_limit_scope,
       CASE current_setting('revaer_test.rss_denied_scope')
           WHEN 'routing_policy' THEN 0 ELSE inst.indexer_instance_id END,
       date_trunc('minute', now() AT TIME ZONE 'UTC') + offset_value * interval '1 minute',
       1000000
FROM indexer_instance inst CROSS JOIN generate_series(0, 1) offset_value
WHERE inst.indexer_instance_public_id = current_setting('revaer_test.rss_source')::uuid;
