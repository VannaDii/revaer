-- Disposable ADR 569 D3 fixture only. No production routine is replaced.
INSERT INTO public.indexer_definition (
    indexer_definition_id, upstream_source, upstream_slug, display_name,
    protocol, engine, schema_version, definition_hash, is_deprecated
) OVERRIDING SYSTEM VALUE VALUES (
    569001, 'cardigann', 'ingestion-proof', 'Ingestion proof',
    'torrent', 'cardigann', 1, repeat('a', 64), false
);
INSERT INTO public.indexer_instance (
    indexer_instance_id, indexer_instance_public_id, indexer_definition_id,
    display_name, is_enabled, migration_state, trust_tier_key,
    created_by_user_id, updated_by_user_id
) OVERRIDING SYSTEM VALUE VALUES (
    569001, '56900000-0000-4000-8000-000000000001', 569001,
    'Ingestion proof', true, 'ready', 'public', 0, 0
);
INSERT INTO public.policy_snapshot (
    policy_snapshot_id, snapshot_hash
) OVERRIDING SYSTEM VALUE VALUES (569001, repeat('b', 64));
INSERT INTO public.search_request (
    search_request_id, search_request_public_id, policy_snapshot_id,
    query_text, query_type, page_size, status
) OVERRIDING SYSTEM VALUE VALUES (
    569001, '56900000-0000-4000-8000-000000000002', 569001,
    'Ingestion proof', 'free_text', 10, 'running'
);
INSERT INTO public.search_request_indexer_run (
    search_request_id, indexer_instance_id, status
) VALUES (569001, 569001, 'queued');
