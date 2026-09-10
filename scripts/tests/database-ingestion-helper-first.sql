-- Disposable D3 proof: compile pure helpers before ingestion at caller defaults.
-- This does not replace routines or claim mutating-helper/trigger closure.
SELECT 'helpers:' || json_build_object(
    'normalize_title_v1', json_build_array(
        public.normalize_title_v1(NULL),
        public.normalize_title_v1('Proof.1080p.H264.mkv')
    ),
    'normalize_magnet_uri_v1', json_build_array(
        public.normalize_magnet_uri_v1(NULL),
        public.normalize_magnet_uri_v1('  '),
        public.normalize_magnet_uri_v1('https://example.invalid/proof'),
        public.normalize_magnet_uri_v1('magnet:'),
        public.normalize_magnet_uri_v1('magnet:?xt=opaque&DN=Proof')
    ),
    'derive_magnet_hash_v1', json_build_array(
        public.derive_magnet_hash_v1(NULL, NULL, NULL),
        public.derive_magnet_hash_v1(repeat('b', 64), repeat('a', 40), NULL),
        public.derive_magnet_hash_v1(NULL, repeat('a', 40), NULL),
        public.derive_magnet_hash_v1(NULL, NULL, 'magnet:?xt=opaque&DN=Proof')
    ),
    'compute_title_size_hash_v1', json_build_array(
        public.compute_title_size_hash_v1(NULL, 1024),
        public.compute_title_size_hash_v1('proof', NULL),
        public.compute_title_size_hash_v1('proof', 1024)
    ),
    'policy_text_match_v1', json_build_array(
        public.policy_text_match_v1(NULL, 'eq', 'proof', NULL, false),
        public.policy_text_match_v1('Proof', 'eq', 'proof', NULL, false),
        public.policy_text_match_v1('aProofb', 'contains', 'proof', NULL, false),
        public.policy_text_match_v1('Proofb', 'starts_with', 'proof', NULL, false),
        public.policy_text_match_v1('aProof', 'ends_with', 'proof', NULL, false),
        public.policy_text_match_v1('Proof', 'regex', '^Proof$', NULL, false),
        public.policy_text_match_v1('Proof', 'regex', '^proof$', NULL, false),
        public.policy_text_match_v1('Proof', 'regex', '^proof$', NULL, true),
        public.policy_text_match_v1('proof', 'regex', NULL, NULL, false),
        public.policy_text_match_v1('proof', 'in_set', NULL, 0, false),
        public.policy_text_match_v1('proof', 'eq', NULL, NULL, false)
    ),
    'policy_uuid_match_v1', json_build_array(
        public.policy_uuid_match_v1(NULL, 'eq', NULL, NULL),
        public.policy_uuid_match_v1('56900000-0000-4000-8000-000000000001', 'eq', '56900000-0000-4000-8000-000000000001', NULL),
        public.policy_uuid_match_v1('56900000-0000-4000-8000-000000000001', 'in_set', NULL, 0),
        public.policy_uuid_match_v1('56900000-0000-4000-8000-000000000001', 'regex', NULL, NULL)
    ),
    'policy_int_match_v1', json_build_array(
        public.policy_int_match_v1(NULL, 'eq', 5, NULL),
        public.policy_int_match_v1(5, 'eq', 5, NULL),
        public.policy_int_match_v1(5, 'in_set', NULL, 0),
        public.policy_int_match_v1(5, 'regex', NULL, NULL)
    ),
    'policy_release_group_match_v1', json_build_array(
        public.policy_release_group_match_v1(0, 'proof', 'eq', 'proof', NULL, false),
        public.policy_release_group_match_v1(0, 'other', 'eq', 'proof', NULL, false),
        public.policy_release_group_match_v1(0, NULL, 'eq', 'proof', NULL, false)
    ),
    'policy_action_to_decision_type', json_build_array(
        public.policy_action_to_decision_type('drop_canonical'),
        public.policy_action_to_decision_type('drop_source'),
        public.policy_action_to_decision_type('downrank'),
        public.policy_action_to_decision_type('flag'),
        public.policy_action_to_decision_type('require'),
        public.policy_action_to_decision_type('prefer')
    )
)::text;
