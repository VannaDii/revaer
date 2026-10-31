-- Only the retained administrator of an owned disposable runtime fixture executes this.
UPDATE indexer_rss_subscription sub
SET claim_expires_at = clock_timestamp() - interval '1 second',
    next_poll_at = clock_timestamp() - interval '1 second'
FROM indexer_instance inst
WHERE inst.indexer_instance_id = sub.indexer_instance_id
  AND inst.indexer_instance_public_id = current_setting('revaer_test.rss_source')::uuid;
