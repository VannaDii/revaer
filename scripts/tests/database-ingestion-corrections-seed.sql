-- Disposable D4 policy-replacement fixture; no production routine is replaced.
INSERT INTO public.policy_set (
    policy_set_id, policy_set_public_id, display_name, scope, is_enabled,
    created_by_user_id, updated_by_user_id
) OVERRIDING SYSTEM VALUE VALUES (
    588004, '58800000-0000-4000-8000-000000000004', 'Ingestion correction proof', 'global', true, 0, 0
);
INSERT INTO public.policy_rule (
    policy_rule_id, policy_set_id, policy_rule_public_id, rule_type, match_field,
    match_operator, match_value_text, action, severity, created_by_user_id, updated_by_user_id
) OVERRIDING SYSTEM VALUE VALUES (
    588005, 588004, '58800000-0000-4000-8000-000000000005', 'block_title_regex',
    'title', 'regex', 'Ingestion', 'flag', 'soft', 0, 0
);
INSERT INTO public.policy_snapshot (policy_snapshot_id, snapshot_hash)
OVERRIDING SYSTEM VALUE VALUES (588002, repeat('c', 64));
INSERT INTO public.policy_snapshot_rule (policy_snapshot_id, policy_rule_public_id, rule_order)
VALUES (588002, '58800000-0000-4000-8000-000000000005', 1);
INSERT INTO public.search_request (
    search_request_id, search_request_public_id, policy_snapshot_id, query_text,
    query_type, page_size, status
) OVERRIDING SYSTEM VALUE VALUES (
    588003, '58800000-0000-4000-8000-000000000003', 588002, 'Second request', 'free_text', 10, 'running'
);
INSERT INTO public.search_request_indexer_run (search_request_id, indexer_instance_id, status)
VALUES (588003, 569001, 'queued');
