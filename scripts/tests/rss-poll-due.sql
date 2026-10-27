-- Only the retained administrator of a disposable runtime fixture executes this.
UPDATE indexer_rss_subscription sub
SET next_poll_at = clock_timestamp() - interval '1 second'
FROM indexer_instance inst
WHERE inst.indexer_instance_id = sub.indexer_instance_id
  AND inst.indexer_instance_public_id = current_setting('revaer_test.rss_source')::uuid
  AND sub.is_enabled;
UPDATE indexer_instance SET connect_timeout_ms = 500, read_timeout_ms = 500
WHERE indexer_instance_public_id = current_setting('revaer_test.rss_source')::uuid;
