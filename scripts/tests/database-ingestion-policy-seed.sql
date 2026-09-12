-- Disposable D3 policy inputs only; application identities come from real ingestion.
-- Re-seed only audit timestamps on the stock read inputs in this recorded fixture
-- transaction. Their keys, ranks, weights and all consumed values remain unchanged.
UPDATE public.trust_tier SET created_at = transaction_timestamp();
UPDATE public.media_domain SET created_at = transaction_timestamp();
INSERT INTO public.policy_set (
    policy_set_id, policy_set_public_id, display_name, scope, is_enabled,
    created_by_user_id, updated_by_user_id, created_at, updated_at
) OVERRIDING SYSTEM VALUE
SELECT 596000 + n, ('59600000-0000-4000-8000-' || lpad(n::text, 12, '0'))::uuid,
       'D3 policy ' || scope, scope::public.policy_scope, true, 0, 0,
       '2026-09-11T00:00:00Z', '2026-09-11T00:00:00Z'
FROM (VALUES (1, 'request'), (2, 'profile'), (3, 'user'), (4, 'global')) s(n, scope);

-- These six populated sets also provide helper-first known answers. The backing
-- rules are not snapshot members and therefore cannot satisfy ingestion assertions.
INSERT INTO public.policy_rule (
    policy_rule_id, policy_set_id, policy_rule_public_id, rule_type, match_field,
    match_operator, action, severity, created_by_user_id, updated_by_user_id,
    created_at, updated_at
) OVERRIDING SYSTEM VALUE
SELECT 596200 + n, 596004,
       ('59600000-0000-4000-8000-' || lpad((200 + n)::text, 12, '0'))::uuid,
       'block_title_regex', 'title', 'in_set', 'flag', 'soft', 0, 0,
       '2026-09-11T00:00:00Z', '2026-09-11T00:00:00Z'
FROM generate_series(1, 6) n;
INSERT INTO public.policy_rule_value_set (value_set_id, policy_rule_id, value_set_type)
OVERRIDING SYSTEM VALUE
SELECT 596200 + n, 596200 + n, kind::public.value_set_type
FROM (VALUES (1, 'text'), (2, 'uuid'), (3, 'int'), (4, 'text'), (5, 'uuid'), (6, 'int')) s(n, kind);
UPDATE public.policy_rule SET value_set_id = policy_rule_id
WHERE policy_rule_id BETWEEN 596201 AND 596206;
INSERT INTO public.policy_rule_value_set_item (value_set_id, value_text)
SELECT 596201, value FROM (VALUES
    (repeat('a', 40)), (repeat('b', 64)), (repeat('c', 64)), ('policy'),
    ('group'), ('uploader'), ('proof-tracker'), ('movies'), ('public')) s(value);
INSERT INTO public.policy_rule_value_set_item (value_set_id, value_text) VALUES (596204, 'not-a-match');
INSERT INTO public.policy_rule_value_set_item (value_set_id, value_uuid) VALUES
    (596202, '56900000-0000-4000-8000-000000000001'),
    (596205, '59600000-0000-4000-8000-000000000099');
INSERT INTO public.policy_rule_value_set_item (value_set_id, value_int) VALUES (596203, 10), (596206, 99);

INSERT INTO public.search_profile (
    search_profile_id, search_profile_public_id, display_name, is_default,
    created_by_user_id, updated_by_user_id, created_at, updated_at
) OVERRIDING SYSTEM VALUE VALUES (
    596001, '59600000-0000-4000-8000-000000000010', 'D3 policy profile', false,
    0, 0, '2026-09-11T00:00:00Z', '2026-09-11T00:00:00Z'
);
INSERT INTO public.tag (
    tag_id, tag_public_id, tag_key, display_name, created_by_user_id, updated_by_user_id,
    created_at, updated_at
) OVERRIDING SYSTEM VALUE VALUES (
    596001, '59600000-0000-4000-8000-000000000011', 'd3-policy', 'D3 policy',
    0, 0, '2026-09-11T00:00:00Z', '2026-09-11T00:00:00Z'
);
INSERT INTO public.indexer_instance_tag (indexer_instance_id, tag_id) VALUES (569001, 596001);
INSERT INTO public.search_profile_tag_prefer (search_profile_id, tag_id, weight_override) VALUES (596001, 596001, 0);
INSERT INTO public.indexer_instance_media_domain (indexer_instance_id, media_domain_id)
SELECT 569001, media_domain_id FROM public.media_domain WHERE media_domain_key = 'movies';
INSERT INTO public.policy_snapshot (policy_snapshot_id, snapshot_hash, created_at)
OVERRIDING SYSTEM VALUE VALUES (596001, repeat('d', 64), '2026-09-11T00:00:00Z');
INSERT INTO public.search_request (
    search_request_id, search_request_public_id, policy_snapshot_id, search_profile_id,
    query_text, query_type, page_size, status
) OVERRIDING SYSTEM VALUE VALUES (
    596001, '59600000-0000-4000-8000-000000000012', 596001, 596001,
    'D3 populated policy', 'free_text', 10, 'running'
);
INSERT INTO public.search_request_indexer_run (search_request_id, indexer_instance_id, status)
VALUES (596001, 569001, 'queued');
