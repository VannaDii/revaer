-- Expected scheduler outcomes are data, never failed transactions.
CREATE TYPE job_claim_outcome AS ENUM ('claimed', 'not_due', 'disabled', 'locked');
DROP FUNCTION job_claim_next(job_key);
DROP FUNCTION job_claim_next_v1(job_key);

CREATE FUNCTION job_claim_next(
    job_key_input job_key
)
RETURNS TABLE (claimed BOOLEAN, outcome job_claim_outcome)
LANGUAGE plpgsql
AS $$
DECLARE
    base_message CONSTANT text := 'Failed to claim job';
    errcode CONSTANT text := 'P0001';
    schedule_id BIGINT;
    schedule_enabled BOOLEAN;
    schedule_next_run TIMESTAMPTZ;
    schedule_locked_until TIMESTAMPTZ;
    now_ts TIMESTAMPTZ;
    lock_acquired BOOLEAN;
BEGIN
    IF job_key_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'job_key_missing';
    END IF;

    lock_acquired := pg_try_advisory_xact_lock(hashtext(job_key_input::text)::BIGINT);
    IF lock_acquired IS NOT TRUE THEN
        RETURN QUERY SELECT false, 'locked'::job_claim_outcome;
        RETURN;
    END IF;

    SELECT job_schedule_id, enabled, next_run_at, locked_until
    INTO schedule_id, schedule_enabled, schedule_next_run, schedule_locked_until
    FROM job_schedule
    WHERE job_key = job_key_input
    FOR UPDATE SKIP LOCKED;

    IF schedule_id IS NULL AND EXISTS (SELECT 1 FROM job_schedule WHERE job_key = job_key_input) THEN
        RETURN QUERY SELECT false, 'locked'::job_claim_outcome;
        RETURN;
    END IF;

    IF schedule_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'job_not_found';
    END IF;

    IF schedule_enabled IS NOT TRUE THEN
        RETURN QUERY SELECT false, 'disabled'::job_claim_outcome;
        RETURN;
    END IF;

    now_ts := now();

    IF schedule_next_run > now_ts THEN
        RETURN QUERY SELECT false, 'not_due'::job_claim_outcome;
        RETURN;
    END IF;

    IF schedule_locked_until IS NOT NULL AND schedule_locked_until > now_ts THEN
        RETURN QUERY SELECT false, 'locked'::job_claim_outcome;
        RETURN;
    END IF;

    UPDATE job_schedule
    SET locked_until = now_ts + make_interval(secs => job_claim_lease_seconds(job_key_input)),
        lock_owner = current_user
    WHERE job_schedule_id = schedule_id;
    RETURN QUERY SELECT true, 'claimed'::job_claim_outcome;
END;
$$;

-- Retain stable entry points as the implementations themselves, not forwarding layers.
CREATE TEMP TABLE revaer_boundary_aliases (
    entry_identity text NOT NULL,
    implementation_identity text NOT NULL,
    old_name text NOT NULL,
    entry_name text NOT NULL
) ON COMMIT DROP;
INSERT INTO revaer_boundary_aliases VALUES
    ('app_user_create(varchar, varchar)', 'app_user_create_v1(varchar, varchar)', 'app_user_create_v1', 'app_user_create'),
    ('app_user_update(uuid, varchar)', 'app_user_update_v1(uuid, varchar)', 'app_user_update_v1', 'app_user_update'),
    ('app_user_verify_email(uuid)', 'app_user_verify_email_v1(uuid)', 'app_user_verify_email_v1', 'app_user_verify_email'),
    ('canonical_disambiguation_rule_create(uuid, disambiguation_rule_type, disambiguation_identity_type, varchar, uuid, disambiguation_identity_type, varchar, uuid, varchar)', 'canonical_disambiguation_rule_create_v1(uuid, disambiguation_rule_type, disambiguation_identity_type, varchar, uuid, disambiguation_identity_type, varchar, uuid, varchar)', 'canonical_disambiguation_rule_create_v1', 'canonical_disambiguation_rule_create'),
    ('canonical_merge_by_infohash(char(64), char(40))', 'canonical_merge_by_infohash_v1(char(64), char(40))', 'canonical_merge_by_infohash_v1', 'canonical_merge_by_infohash'),
    ('canonical_prune_low_confidence()', 'canonical_prune_low_confidence_v1()', 'canonical_prune_low_confidence_v1', 'canonical_prune_low_confidence'),
    ('canonical_recompute_best_source(uuid, scoring_context)', 'canonical_recompute_best_source_v1(uuid, scoring_context)', 'canonical_recompute_best_source_v1', 'canonical_recompute_best_source'),
    ('compute_title_size_hash(text, bigint)', 'compute_title_size_hash_v1(text, bigint)', 'compute_title_size_hash_v1', 'compute_title_size_hash'),
    ('deployment_init(uuid)', 'deployment_init_v1(uuid)', 'deployment_init_v1', 'deployment_init'),
    ('derive_magnet_hash(text, text, text)', 'derive_magnet_hash_v1(text, text, text)', 'derive_magnet_hash_v1', 'derive_magnet_hash'),
    ('import_job_create(uuid, import_source, boolean, uuid, uuid)', 'import_job_create_v1(uuid, import_source, boolean, uuid, uuid)', 'import_job_create_v1', 'import_job_create'),
    ('import_job_get_status(uuid)', 'import_job_get_status_v1(uuid)', 'import_job_get_status_v1', 'import_job_get_status'),
    ('import_job_list_results(uuid)', 'import_job_list_results_v1(uuid)', 'import_job_list_results_v1', 'import_job_list_results'),
    ('import_job_run_prowlarr_api(uuid, varchar, uuid)', 'import_job_run_prowlarr_api_v1(uuid, varchar, uuid)', 'import_job_run_prowlarr_api_v1', 'import_job_run_prowlarr_api'),
    ('import_job_run_prowlarr_backup(uuid, varchar)', 'import_job_run_prowlarr_backup_v1(uuid, varchar)', 'import_job_run_prowlarr_backup_v1', 'import_job_run_prowlarr_backup'),
    ('import_job_worker_claim_next()', 'import_job_worker_claim_next_v1()', 'import_job_worker_claim_next_v1', 'import_job_worker_claim_next'),
    ('import_job_worker_mark_terminal(uuid, import_job_status, varchar)', 'import_job_worker_mark_terminal_v1(uuid, import_job_status, varchar)', 'import_job_worker_mark_terminal_v1', 'import_job_worker_mark_terminal'),
    ('import_job_worker_record_result(uuid, varchar, import_indexer_result_status, varchar, boolean, integer, integer)', 'import_job_worker_record_result_v1(uuid, varchar, import_indexer_result_status, varchar, boolean, integer, integer)', 'import_job_worker_record_result_v1', 'import_job_worker_record_result'),
    ('indexer_backup_export_indexer_instance_list(uuid)', 'indexer_backup_export_indexer_instance_list_v1(uuid)', 'indexer_backup_export_indexer_instance_list_v1', 'indexer_backup_export_indexer_instance_list'),
    ('indexer_backup_export_rate_limit_policy_list(uuid)', 'indexer_backup_export_rate_limit_policy_list_v1(uuid)', 'indexer_backup_export_rate_limit_policy_list_v1', 'indexer_backup_export_rate_limit_policy_list'),
    ('indexer_backup_export_routing_policy_list(uuid)', 'indexer_backup_export_routing_policy_list_v1(uuid)', 'indexer_backup_export_routing_policy_list_v1', 'indexer_backup_export_routing_policy_list'),
    ('indexer_backup_export_tag_list(uuid)', 'indexer_backup_export_tag_list_v1(uuid)', 'indexer_backup_export_tag_list_v1', 'indexer_backup_export_tag_list'),
    ('indexer_cf_state_get(uuid, uuid)', 'indexer_cf_state_get_v1(uuid, uuid)', 'indexer_cf_state_get_v1', 'indexer_cf_state_get'),
    ('indexer_cf_state_reset(uuid, uuid, varchar)', 'indexer_cf_state_reset_v1(uuid, uuid, varchar)', 'indexer_cf_state_reset_v1', 'indexer_cf_state_reset'),
    ('indexer_connectivity_profile_get(uuid, uuid)', 'indexer_connectivity_profile_get_v1(uuid, uuid)', 'indexer_connectivity_profile_get_v1', 'indexer_connectivity_profile_get'),
    ('indexer_definition_import_cardigann_begin(uuid, varchar, varchar, text, boolean)', 'indexer_definition_import_cardigann_begin_v1(uuid, varchar, varchar, text, boolean)', 'indexer_definition_import_cardigann_begin_v1', 'indexer_definition_import_cardigann_begin'),
    ('indexer_definition_import_cardigann_complete(uuid, varchar)', 'indexer_definition_import_cardigann_complete_v1(uuid, varchar)', 'indexer_definition_import_cardigann_complete_v1', 'indexer_definition_import_cardigann_complete'),
    ('indexer_definition_import_cardigann_field(uuid, varchar, varchar, varchar, field_type, boolean, boolean, integer, varchar, integer, varchar, boolean, varchar[], varchar[])', 'indexer_definition_import_cardigann_field_v1(uuid, varchar, varchar, varchar, field_type, boolean, boolean, integer, varchar, integer, varchar, boolean, varchar[], varchar[])', 'indexer_definition_import_cardigann_field_v1', 'indexer_definition_import_cardigann_field'),
    ('indexer_definition_list(uuid)', 'indexer_definition_list_v1(uuid)', 'indexer_definition_list_v1', 'indexer_definition_list'),
    ('indexer_health_event_list(uuid, uuid, integer)', 'indexer_health_event_list_v1(uuid, uuid, integer)', 'indexer_health_event_list_v1', 'indexer_health_event_list'),
    ('indexer_health_notification_hook_delete(uuid, uuid)', 'indexer_health_notification_hook_delete_v1(uuid, uuid)', 'indexer_health_notification_hook_delete_v1', 'indexer_health_notification_hook_delete'),
    ('indexer_health_notification_hook_get(uuid, uuid)', 'indexer_health_notification_hook_get_v1(uuid, uuid)', 'indexer_health_notification_hook_get_v1', 'indexer_health_notification_hook_get'),
    ('indexer_health_notification_hook_list(uuid)', 'indexer_health_notification_hook_list_v1(uuid)', 'indexer_health_notification_hook_list_v1', 'indexer_health_notification_hook_list'),
    ('indexer_instance_create(uuid, varchar, varchar, integer, trust_tier_key, uuid)', 'indexer_instance_create_v1(uuid, varchar, varchar, integer, trust_tier_key, uuid)', 'indexer_instance_create_v1', 'indexer_instance_create'),
    ('indexer_instance_field_bind_secret(uuid, uuid, varchar, uuid)', 'indexer_instance_field_bind_secret_v1(uuid, uuid, varchar, uuid)', 'indexer_instance_field_bind_secret_v1', 'indexer_instance_field_bind_secret'),
    ('indexer_instance_field_set_value(uuid, uuid, varchar, varchar, integer, numeric, boolean)', 'indexer_instance_field_set_value_v1(uuid, uuid, varchar, varchar, integer, numeric, boolean)', 'indexer_instance_field_set_value_v1', 'indexer_instance_field_set_value'),
    ('indexer_instance_set_media_domains(uuid, uuid, text[])', 'indexer_instance_set_media_domains_v1(uuid, uuid, text[])', 'indexer_instance_set_media_domains_v1', 'indexer_instance_set_media_domains'),
    ('indexer_instance_set_rate_limit_policy(uuid, uuid, uuid)', 'indexer_instance_set_rate_limit_policy_v1(uuid, uuid, uuid)', 'indexer_instance_set_rate_limit_policy_v1', 'indexer_instance_set_rate_limit_policy'),
    ('indexer_instance_set_tags(uuid, uuid, uuid[], text[])', 'indexer_instance_set_tags_v1(uuid, uuid, uuid[], text[])', 'indexer_instance_set_tags_v1', 'indexer_instance_set_tags'),
    ('indexer_instance_test_finalize(uuid, uuid, boolean, error_class, varchar(64), varchar(256), integer)', 'indexer_instance_test_finalize_v1(uuid, uuid, boolean, error_class, varchar(64), varchar(256), integer)', 'indexer_instance_test_finalize_v1', 'indexer_instance_test_finalize'),
    ('indexer_instance_test_prepare(uuid, uuid)', 'indexer_instance_test_prepare_v1(uuid, uuid)', 'indexer_instance_test_prepare_v1', 'indexer_instance_test_prepare'),
    ('indexer_instance_update(uuid, uuid, varchar, integer, trust_tier_key, uuid, boolean, boolean, boolean, boolean)', 'indexer_instance_update_v1(uuid, uuid, varchar, integer, trust_tier_key, uuid, boolean, boolean, boolean, boolean)', 'indexer_instance_update_v1', 'indexer_instance_update'),
    ('indexer_policy_set_rule_list(uuid)', 'indexer_policy_set_rule_list_v1(uuid)', 'indexer_policy_set_rule_list_v1', 'indexer_policy_set_rule_list'),
    ('indexer_rss_item_seen_list(uuid, uuid, integer)', 'indexer_rss_item_seen_list_v1(uuid, uuid, integer)', 'indexer_rss_item_seen_list_v1', 'indexer_rss_item_seen_list'),
    ('indexer_rss_item_seen_mark(uuid, uuid, varchar, char(40), char(64), char(64))', 'indexer_rss_item_seen_mark_v1(uuid, uuid, varchar, char(40), char(64), char(64))', 'indexer_rss_item_seen_mark_v1', 'indexer_rss_item_seen_mark'),
    ('indexer_rss_subscription_disable(uuid, uuid)', 'indexer_rss_subscription_disable_v1(uuid, uuid)', 'indexer_rss_subscription_disable_v1', 'indexer_rss_subscription_disable'),
    ('indexer_rss_subscription_get(uuid, uuid)', 'indexer_rss_subscription_get_v1(uuid, uuid)', 'indexer_rss_subscription_get_v1', 'indexer_rss_subscription_get'),
    ('indexer_rss_subscription_set(uuid, uuid, boolean, integer)', 'indexer_rss_subscription_set_v1(uuid, uuid, boolean, integer)', 'indexer_rss_subscription_set_v1', 'indexer_rss_subscription_set'),
    ('indexer_search_profile_list(uuid)', 'indexer_search_profile_list_v1(uuid)', 'indexer_search_profile_list_v1', 'indexer_search_profile_list'),
    ('indexer_source_reputation_list(uuid, uuid, reputation_window, integer)', 'indexer_source_reputation_list_v1(uuid, uuid, reputation_window, integer)', 'indexer_source_reputation_list_v1', 'indexer_source_reputation_list'),
    ('indexer_torznab_instance_list(uuid)', 'indexer_torznab_instance_list_v1(uuid)', 'indexer_torznab_instance_list_v1', 'indexer_torznab_instance_list'),
    ('job_run_base_score_refresh_recent()', 'job_run_base_score_refresh_recent_v2()', 'job_run_base_score_refresh_recent_v2', 'job_run_base_score_refresh_recent'),
    ('job_run_canonical_backfill_best_source()', 'job_run_canonical_backfill_best_source_v2()', 'job_run_canonical_backfill_best_source_v2', 'job_run_canonical_backfill_best_source'),
    ('job_run_canonical_prune_low_confidence()', 'job_run_canonical_prune_low_confidence_v2()', 'job_run_canonical_prune_low_confidence_v2', 'job_run_canonical_prune_low_confidence'),
    ('job_run_connectivity_profile_refresh()', 'job_run_connectivity_profile_refresh_v2()', 'job_run_connectivity_profile_refresh_v2', 'job_run_connectivity_profile_refresh'),
    ('job_run_policy_snapshot_gc()', 'job_run_policy_snapshot_gc_v2()', 'job_run_policy_snapshot_gc_v2', 'job_run_policy_snapshot_gc'),
    ('job_run_policy_snapshot_refcount_repair()', 'job_run_policy_snapshot_refcount_repair_v2()', 'job_run_policy_snapshot_refcount_repair_v2', 'job_run_policy_snapshot_refcount_repair'),
    ('job_run_rate_limit_state_purge()', 'job_run_rate_limit_state_purge_v2()', 'job_run_rate_limit_state_purge_v2', 'job_run_rate_limit_state_purge'),
    ('job_run_reputation_rollup(reputation_window)', 'job_run_reputation_rollup_v2(reputation_window)', 'job_run_reputation_rollup_v2', 'job_run_reputation_rollup'),
    ('job_run_retention_purge()', 'job_run_retention_purge_v2()', 'job_run_retention_purge_v2', 'job_run_retention_purge'),
    ('job_run_rss_subscription_backfill()', 'job_run_rss_subscription_backfill_v2()', 'job_run_rss_subscription_backfill_v2', 'job_run_rss_subscription_backfill'),
    ('job_schedule_mark_completed(job_key)', 'job_schedule_mark_completed_v1(job_key)', 'job_schedule_mark_completed_v1', 'job_schedule_mark_completed'),
    ('log_source_metadata_conflict(bigint, bigint, conflict_type, text, text, timestamptz)', 'log_source_metadata_conflict_v1(bigint, bigint, conflict_type, text, text, timestamptz)', 'log_source_metadata_conflict_v1', 'log_source_metadata_conflict'),
    ('media_domain_to_torznab_category_delete(uuid, varchar, integer)', 'media_domain_to_torznab_category_delete_v1(uuid, varchar, integer)', 'media_domain_to_torznab_category_delete_v1', 'media_domain_to_torznab_category_delete'),
    ('media_domain_to_torznab_category_upsert(uuid, varchar, integer, boolean)', 'media_domain_to_torznab_category_upsert_v1(uuid, varchar, integer, boolean)', 'media_domain_to_torznab_category_upsert_v1', 'media_domain_to_torznab_category_upsert'),
    ('normalize_magnet_uri(text)', 'normalize_magnet_uri_v1(text)', 'normalize_magnet_uri_v1', 'normalize_magnet_uri'),
    ('normalize_title(text)', 'normalize_title_v1(text)', 'normalize_title_v1', 'normalize_title'),
    ('outbound_request_log_write(uuid, uuid, uuid, outbound_request_type, uuid, smallint, timestamptz, timestamptz, outbound_request_outcome, outbound_via_mitigation, rate_limit_scope, error_class, integer, integer, boolean, integer, boolean, integer, varchar)', 'outbound_request_log_write_v1(uuid, uuid, uuid, outbound_request_type, uuid, smallint, timestamptz, timestamptz, outbound_request_outcome, outbound_via_mitigation, rate_limit_scope, error_class, integer, integer, boolean, integer, boolean, integer, varchar)', 'outbound_request_log_write_v1', 'outbound_request_log_write'),
    ('policy_int_match(integer, policy_match_operator, integer, bigint)', 'policy_int_match_v1(integer, policy_match_operator, integer, bigint)', 'policy_int_match_v1', 'policy_int_match'),
    ('policy_release_group_match(bigint, text, policy_match_operator, text, bigint, boolean)', 'policy_release_group_match_v1(bigint, text, policy_match_operator, text, bigint, boolean)', 'policy_release_group_match_v1', 'policy_release_group_match'),
    ('policy_rule_create(uuid, uuid, policy_rule_type, policy_match_field, policy_match_operator, integer, varchar, integer, uuid, policy_rule_value_item[], policy_action, policy_severity, boolean, varchar, timestamptz)', 'policy_rule_create_v1(uuid, uuid, policy_rule_type, policy_match_field, policy_match_operator, integer, varchar, integer, uuid, policy_rule_value_item[], policy_action, policy_severity, boolean, varchar, timestamptz)', 'policy_rule_create_v1', 'policy_rule_create'),
    ('policy_rule_disable(uuid, uuid)', 'policy_rule_disable_v1(uuid, uuid)', 'policy_rule_disable_v1', 'policy_rule_disable'),
    ('policy_rule_enable(uuid, uuid)', 'policy_rule_enable_v1(uuid, uuid)', 'policy_rule_enable_v1', 'policy_rule_enable'),
    ('policy_rule_reorder(uuid, uuid, uuid[])', 'policy_rule_reorder_v1(uuid, uuid, uuid[])', 'policy_rule_reorder_v1', 'policy_rule_reorder'),
    ('policy_set_create(uuid, varchar, policy_scope, boolean)', 'policy_set_create_v1(uuid, varchar, policy_scope, boolean)', 'policy_set_create_v1', 'policy_set_create'),
    ('policy_set_disable(uuid, uuid)', 'policy_set_disable_v1(uuid, uuid)', 'policy_set_disable_v1', 'policy_set_disable'),
    ('policy_set_enable(uuid, uuid)', 'policy_set_enable_v1(uuid, uuid)', 'policy_set_enable_v1', 'policy_set_enable'),
    ('policy_set_reorder(uuid, uuid[])', 'policy_set_reorder_v1(uuid, uuid[])', 'policy_set_reorder_v1', 'policy_set_reorder'),
    ('policy_set_update(uuid, uuid, varchar)', 'policy_set_update_v1(uuid, uuid, varchar)', 'policy_set_update_v1', 'policy_set_update'),
    ('policy_text_match(text, policy_match_operator, text, bigint, boolean)', 'policy_text_match_v1(text, policy_match_operator, text, bigint, boolean)', 'policy_text_match_v1', 'policy_text_match'),
    ('policy_uuid_match(uuid, policy_match_operator, uuid, bigint)', 'policy_uuid_match_v1(uuid, policy_match_operator, uuid, bigint)', 'policy_uuid_match_v1', 'policy_uuid_match'),
    ('rate_limit_policy_create(uuid, varchar, integer, integer, integer)', 'rate_limit_policy_create_v1(uuid, varchar, integer, integer, integer)', 'rate_limit_policy_create_v1', 'rate_limit_policy_create'),
    ('rate_limit_policy_soft_delete(uuid, uuid)', 'rate_limit_policy_soft_delete_v1(uuid, uuid)', 'rate_limit_policy_soft_delete_v1', 'rate_limit_policy_soft_delete'),
    ('rate_limit_policy_update(uuid, uuid, varchar, integer, integer, integer)', 'rate_limit_policy_update_v1(uuid, uuid, varchar, integer, integer, integer)', 'rate_limit_policy_update_v1', 'rate_limit_policy_update'),
    ('rate_limit_try_consume(rate_limit_scope, bigint, integer, integer)', 'rate_limit_try_consume_v1(rate_limit_scope, bigint, integer, integer)', 'rate_limit_try_consume_v1', 'rate_limit_try_consume'),
    ('routing_policy_bind_secret(uuid, uuid, routing_param_key, uuid)', 'routing_policy_bind_secret_v1(uuid, uuid, routing_param_key, uuid)', 'routing_policy_bind_secret_v1', 'routing_policy_bind_secret'),
    ('routing_policy_create(uuid, varchar, routing_policy_mode)', 'routing_policy_create_v1(uuid, varchar, routing_policy_mode)', 'routing_policy_create_v1', 'routing_policy_create'),
    ('routing_policy_get(uuid, uuid)', 'routing_policy_get_v1(uuid, uuid)', 'routing_policy_get_v1', 'routing_policy_get'),
    ('routing_policy_set_param(uuid, uuid, routing_param_key, varchar, integer, boolean)', 'routing_policy_set_param_v1(uuid, uuid, routing_param_key, varchar, integer, boolean)', 'routing_policy_set_param_v1', 'routing_policy_set_param'),
    ('routing_policy_set_rate_limit_policy(uuid, uuid, uuid)', 'routing_policy_set_rate_limit_policy_v1(uuid, uuid, uuid)', 'routing_policy_set_rate_limit_policy_v1', 'routing_policy_set_rate_limit_policy'),
    ('rss_poll_apply(bigint, uuid, smallint, timestamptz, timestamptz, outbound_request_outcome, error_class, integer, integer, boolean, integer, outbound_via_mitigation, rate_limit_scope, boolean, boolean, varchar[], char(40)[], char(64)[], char(64)[])', 'rss_poll_apply_v1(bigint, uuid, smallint, timestamptz, timestamptz, outbound_request_outcome, error_class, integer, integer, boolean, integer, outbound_via_mitigation, rate_limit_scope, boolean, boolean, varchar[], char(40)[], char(64)[], char(64)[])', 'rss_poll_apply_v1', 'rss_poll_apply'),
    ('rss_poll_claim(integer)', 'rss_poll_claim_v1(integer)', 'rss_poll_claim_v1', 'rss_poll_claim'),
    ('search_indexer_run_enqueue(uuid, uuid)', 'search_indexer_run_enqueue_v1(uuid, uuid)', 'search_indexer_run_enqueue_v1', 'search_indexer_run_enqueue'),
    ('search_indexer_run_mark_canceled(uuid, uuid)', 'search_indexer_run_mark_canceled_v1(uuid, uuid)', 'search_indexer_run_mark_canceled_v1', 'search_indexer_run_mark_canceled'),
    ('search_indexer_run_mark_failed(uuid, uuid, error_class, varchar, integer, smallint, rate_limit_scope)', 'search_indexer_run_mark_failed_v1(uuid, uuid, error_class, varchar, integer, smallint, rate_limit_scope)', 'search_indexer_run_mark_failed_v1', 'search_indexer_run_mark_failed'),
    ('search_indexer_run_mark_finished(uuid, uuid, integer, integer, integer)', 'search_indexer_run_mark_finished_v1(uuid, uuid, integer, integer, integer)', 'search_indexer_run_mark_finished_v1', 'search_indexer_run_mark_finished'),
    ('search_indexer_run_mark_started(uuid, uuid)', 'search_indexer_run_mark_started_v1(uuid, uuid)', 'search_indexer_run_mark_started_v1', 'search_indexer_run_mark_started'),
    ('search_page_fetch(uuid, uuid, integer)', 'search_page_fetch_v1(uuid, uuid, integer)', 'search_page_fetch_v1', 'search_page_fetch'),
    ('search_page_list(uuid, uuid)', 'search_page_list_v1(uuid, uuid)', 'search_page_list_v1', 'search_page_list'),
    ('search_profile_add_policy_set(uuid, uuid, uuid)', 'search_profile_add_policy_set_v1(uuid, uuid, uuid)', 'search_profile_add_policy_set_v1', 'search_profile_add_policy_set'),
    ('search_profile_create(uuid, varchar, boolean, integer, varchar, uuid)', 'search_profile_create_v1(uuid, varchar, boolean, integer, varchar, uuid)', 'search_profile_create_v1', 'search_profile_create'),
    ('search_profile_indexer_allow(uuid, uuid, uuid[])', 'search_profile_indexer_allow_v1(uuid, uuid, uuid[])', 'search_profile_indexer_allow_v1', 'search_profile_indexer_allow'),
    ('search_profile_indexer_block(uuid, uuid, uuid[])', 'search_profile_indexer_block_v1(uuid, uuid, uuid[])', 'search_profile_indexer_block_v1', 'search_profile_indexer_block'),
    ('search_profile_remove_policy_set(uuid, uuid, uuid)', 'search_profile_remove_policy_set_v1(uuid, uuid, uuid)', 'search_profile_remove_policy_set_v1', 'search_profile_remove_policy_set'),
    ('search_profile_set_default(uuid, uuid, integer)', 'search_profile_set_default_v1(uuid, uuid, integer)', 'search_profile_set_default_v1', 'search_profile_set_default'),
    ('search_profile_set_default_domain(uuid, uuid, varchar)', 'search_profile_set_default_domain_v1(uuid, uuid, varchar)', 'search_profile_set_default_domain_v1', 'search_profile_set_default_domain'),
    ('search_profile_set_domain_allowlist(uuid, uuid, text[])', 'search_profile_set_domain_allowlist_v1(uuid, uuid, text[])', 'search_profile_set_domain_allowlist_v1', 'search_profile_set_domain_allowlist'),
    ('search_profile_tag_allow(uuid, uuid, uuid[], text[])', 'search_profile_tag_allow_v1(uuid, uuid, uuid[], text[])', 'search_profile_tag_allow_v1', 'search_profile_tag_allow'),
    ('search_profile_tag_block(uuid, uuid, uuid[], text[])', 'search_profile_tag_block_v1(uuid, uuid, uuid[], text[])', 'search_profile_tag_block_v1', 'search_profile_tag_block'),
    ('search_profile_tag_prefer(uuid, uuid, uuid[], text[])', 'search_profile_tag_prefer_v1(uuid, uuid, uuid[], text[])', 'search_profile_tag_prefer_v1', 'search_profile_tag_prefer'),
    ('search_profile_update(uuid, uuid, varchar, integer)', 'search_profile_update_v1(uuid, uuid, varchar, integer)', 'search_profile_update_v1', 'search_profile_update'),
    ('search_request_cancel(uuid, uuid)', 'search_request_cancel_v1(uuid, uuid)', 'search_request_cancel_v1', 'search_request_cancel'),
    ('search_request_create(uuid, varchar, query_type, torznab_mode, varchar, integer, uuid, uuid, integer, integer, identifier_type[], text[], integer[])', 'search_request_create_v1(uuid, varchar, query_type, torznab_mode, varchar, integer, uuid, uuid, integer, integer, identifier_type[], text[], integer[])', 'search_request_create_v1', 'search_request_create'),
    ('search_request_explainability(uuid, uuid)', 'search_request_explainability_v1(uuid, uuid)', 'search_request_explainability_v1', 'search_request_explainability'),
    ('secret_create(uuid, secret_type, varchar)', 'secret_create_v1(uuid, secret_type, varchar)', 'secret_create_v1', 'secret_create'),
    ('secret_metadata_list(uuid)', 'secret_metadata_list_v1(uuid)', 'secret_metadata_list_v1', 'secret_metadata_list'),
    ('secret_read(uuid, uuid)', 'secret_read_v1(uuid, uuid)', 'secret_read_v1', 'secret_read'),
    ('secret_revoke(uuid, uuid)', 'secret_revoke_v1(uuid, uuid)', 'secret_revoke_v1', 'secret_revoke'),
    ('secret_rotate(uuid, uuid, varchar)', 'secret_rotate_v1(uuid, uuid, varchar)', 'secret_rotate_v1', 'secret_rotate'),
    ('secret_session_configure(uuid, varchar, varchar)', 'secret_session_configure_v1(uuid, varchar, varchar)', 'secret_session_configure_v1', 'secret_session_configure'),
    ('source_metadata_conflict_list(uuid, boolean, integer)', 'source_metadata_conflict_list_v1(uuid, boolean, integer)', 'source_metadata_conflict_list_v1', 'source_metadata_conflict_list'),
    ('source_metadata_conflict_reopen(uuid, bigint, varchar)', 'source_metadata_conflict_reopen_v1(uuid, bigint, varchar)', 'source_metadata_conflict_reopen_v1', 'source_metadata_conflict_reopen'),
    ('source_metadata_conflict_resolve(uuid, bigint, conflict_resolution, varchar)', 'source_metadata_conflict_resolve_v1(uuid, bigint, conflict_resolution, varchar)', 'source_metadata_conflict_resolve_v1', 'source_metadata_conflict_resolve'),
    ('tag_create(uuid, varchar, varchar)', 'tag_create_v1(uuid, varchar, varchar)', 'tag_create_v1', 'tag_create'),
    ('tag_list(uuid)', 'tag_list_v1(uuid)', 'tag_list_v1', 'tag_list'),
    ('tag_soft_delete(uuid, uuid, varchar)', 'tag_soft_delete_v1(uuid, uuid, varchar)', 'tag_soft_delete_v1', 'tag_soft_delete'),
    ('tag_update(uuid, uuid, varchar, varchar)', 'tag_update_v1(uuid, uuid, varchar, varchar)', 'tag_update_v1', 'tag_update'),
    ('torznab_download_prepare(uuid, uuid)', 'torznab_download_prepare_v1(uuid, uuid)', 'torznab_download_prepare_v1', 'torznab_download_prepare'),
    ('torznab_instance_authenticate(uuid, varchar)', 'torznab_instance_authenticate_v1(uuid, varchar)', 'torznab_instance_authenticate_v1', 'torznab_instance_authenticate'),
    ('torznab_instance_create(uuid, uuid, varchar)', 'torznab_instance_create_v1(uuid, uuid, varchar)', 'torznab_instance_create_v1', 'torznab_instance_create'),
    ('torznab_instance_enable_disable(uuid, uuid, boolean)', 'torznab_instance_enable_disable_v1(uuid, uuid, boolean)', 'torznab_instance_enable_disable_v1', 'torznab_instance_enable_disable'),
    ('torznab_instance_rotate_key(uuid, uuid)', 'torznab_instance_rotate_key_v1(uuid, uuid)', 'torznab_instance_rotate_key_v1', 'torznab_instance_rotate_key'),
    ('torznab_instance_soft_delete(uuid, uuid)', 'torznab_instance_soft_delete_v1(uuid, uuid)', 'torznab_instance_soft_delete_v1', 'torznab_instance_soft_delete'),
    ('tracker_category_mapping_delete(uuid, uuid, varchar, uuid, integer, integer)', 'tracker_category_mapping_delete_v1(uuid, uuid, varchar, uuid, integer, integer)', 'tracker_category_mapping_delete_v1', 'tracker_category_mapping_delete'),
    ('tracker_category_mapping_delete(uuid, varchar, integer, integer)', 'tracker_category_mapping_delete_v1(uuid, varchar, integer, integer)', 'tracker_category_mapping_delete_v1', 'tracker_category_mapping_delete'),
    ('tracker_category_mapping_resolve_feed(uuid, uuid, integer, integer)', 'tracker_category_mapping_resolve_feed_v1(uuid, uuid, integer, integer)', 'tracker_category_mapping_resolve_feed_v1', 'tracker_category_mapping_resolve_feed'),
    ('tracker_category_mapping_upsert(uuid, uuid, varchar, uuid, integer, integer, integer, varchar)', 'tracker_category_mapping_upsert_v1(uuid, uuid, varchar, uuid, integer, integer, integer, varchar)', 'tracker_category_mapping_upsert_v1', 'tracker_category_mapping_upsert'),
    ('tracker_category_mapping_upsert(uuid, varchar, integer, integer, integer, varchar)', 'tracker_category_mapping_upsert_v1(uuid, varchar, integer, integer, integer, varchar)', 'tracker_category_mapping_upsert_v1', 'tracker_category_mapping_upsert');

DO $boundary$
DECLARE
    alias_record record;
    grant_record record;
    entry_privileges aclitem[];
    implementation_privileges aclitem[];
BEGIN
    FOR alias_record IN SELECT * FROM revaer_boundary_aliases ORDER BY entry_identity LOOP
        IF to_regprocedure(alias_record.entry_identity) IS NULL OR
           to_regprocedure(alias_record.implementation_identity) IS NULL THEN
            RAISE EXCEPTION 'missing database boundary routine: %', alias_record.entry_identity;
        END IF;
        SELECT COALESCE(proacl, acldefault('f', proowner)) INTO entry_privileges
        FROM pg_proc WHERE oid = to_regprocedure(alias_record.entry_identity);
        SELECT COALESCE(proacl, acldefault('f', proowner)) INTO implementation_privileges
        FROM pg_proc WHERE oid = to_regprocedure(alias_record.implementation_identity);
        EXECUTE format('DROP FUNCTION %s', alias_record.entry_identity);
        EXECUTE format('ALTER FUNCTION %s RENAME TO %I', alias_record.implementation_identity, alias_record.entry_name);
        -- Preserve callers' permissions on the public entry point, including revoked PUBLIC access.
        FOR grant_record IN SELECT DISTINCT grantee FROM aclexplode(implementation_privileges) LOOP
            EXECUTE format('REVOKE ALL ON FUNCTION %s FROM %s', alias_record.entry_identity,
                CASE WHEN grant_record.grantee = 0 THEN 'PUBLIC' ELSE quote_ident(pg_get_userbyid(grant_record.grantee)) END);
        END LOOP;
        FOR grant_record IN SELECT * FROM aclexplode(entry_privileges) LOOP
            EXECUTE format('GRANT %s ON FUNCTION %s TO %s%s', grant_record.privilege_type, alias_record.entry_identity,
                CASE WHEN grant_record.grantee = 0 THEN 'PUBLIC' ELSE quote_ident(pg_get_userbyid(grant_record.grantee)) END,
                CASE WHEN grant_record.is_grantable THEN ' WITH GRANT OPTION' ELSE '' END);
        END LOOP;
    END LOOP;
END;
$boundary$;

-- Capture definitions before replacing return types. The list is restricted to
-- routines owned by this migration corpus; extension functions are never adopted.
CREATE TEMP TABLE revaer_boundary_definitions ON COMMIT DROP AS
SELECT p.oid::regprocedure::text AS identity,
       pg_get_functiondef(p.oid) AS definition,
       p.prorettype = 'void'::regtype AS void_result,
       p.proowner AS owner_id,
       p.proacl AS privileges
FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
WHERE p.prokind = 'f'
  AND (CASE WHEN n.nspname = 'public' THEN p.proname ELSE n.nspname || '.' || p.proname END) = ANY(ARRAY[
        'app_user_create',
        'app_user_create_v1',
        'app_user_update',
        'app_user_update_v1',
        'app_user_verify_email',
        'app_user_verify_email_v1',
        'canonical_disambiguation_rule_create',
        'canonical_disambiguation_rule_create_v1',
        'canonical_merge_by_infohash',
        'canonical_merge_by_infohash_v1',
        'canonical_prune_low_confidence',
        'canonical_prune_low_confidence_v1',
        'canonical_recompute_best_source',
        'canonical_recompute_best_source_v1',
        'compute_title_size_hash',
        'compute_title_size_hash_v1',
        'deployment_init',
        'deployment_init_v1',
        'derive_magnet_hash',
        'derive_magnet_hash_v1',
        'import_job_create',
        'import_job_create_v1',
        'import_job_get_status',
        'import_job_get_status_v1',
        'import_job_list_results',
        'import_job_list_results_v1',
        'import_job_run_prowlarr_api',
        'import_job_run_prowlarr_api_v1',
        'import_job_run_prowlarr_backup',
        'import_job_run_prowlarr_backup_v1',
        'import_job_worker_claim_next',
        'import_job_worker_claim_next_v1',
        'import_job_worker_mark_terminal',
        'import_job_worker_mark_terminal_v1',
        'import_job_worker_record_result',
        'import_job_worker_record_result_v1',
        'indexer_backup_assert_actor_v1',
        'indexer_backup_export_indexer_instance_list',
        'indexer_backup_export_indexer_instance_list_v1',
        'indexer_backup_export_rate_limit_policy_list',
        'indexer_backup_export_rate_limit_policy_list_v1',
        'indexer_backup_export_routing_policy_list',
        'indexer_backup_export_routing_policy_list_v1',
        'indexer_backup_export_tag_list',
        'indexer_backup_export_tag_list_v1',
        'indexer_cf_state_get',
        'indexer_cf_state_get_v1',
        'indexer_cf_state_reset',
        'indexer_cf_state_reset_v1',
        'indexer_connectivity_profile_get',
        'indexer_connectivity_profile_get_v1',
        'indexer_definition_import_cardigann_begin',
        'indexer_definition_import_cardigann_begin_v1',
        'indexer_definition_import_cardigann_complete',
        'indexer_definition_import_cardigann_complete_v1',
        'indexer_definition_import_cardigann_field',
        'indexer_definition_import_cardigann_field_v1',
        'indexer_definition_list',
        'indexer_definition_list_v1',
        'indexer_health_event_list',
        'indexer_health_event_list_v1',
        'indexer_health_notification_hook_create',
        'indexer_health_notification_hook_create_v1',
        'indexer_health_notification_hook_delete',
        'indexer_health_notification_hook_delete_v1',
        'indexer_health_notification_hook_get',
        'indexer_health_notification_hook_get_v1',
        'indexer_health_notification_hook_list',
        'indexer_health_notification_hook_list_v1',
        'indexer_health_notification_hook_update',
        'indexer_health_notification_hook_update_v1',
        'indexer_instance_create',
        'indexer_instance_create_v1',
        'indexer_instance_field_bind_secret',
        'indexer_instance_field_bind_secret_v1',
        'indexer_instance_field_set_value',
        'indexer_instance_field_set_value_v1',
        'indexer_instance_set_media_domains',
        'indexer_instance_set_media_domains_v1',
        'indexer_instance_set_rate_limit_policy',
        'indexer_instance_set_rate_limit_policy_v1',
        'indexer_instance_set_tags',
        'indexer_instance_set_tags_v1',
        'indexer_instance_test_finalize',
        'indexer_instance_test_finalize_v1',
        'indexer_instance_test_prepare',
        'indexer_instance_test_prepare_v1',
        'indexer_instance_update',
        'indexer_instance_update_v1',
        'indexer_policy_set_rule_list',
        'indexer_policy_set_rule_list_v1',
        'indexer_rss_item_seen_list',
        'indexer_rss_item_seen_list_v1',
        'indexer_rss_item_seen_mark',
        'indexer_rss_item_seen_mark_v1',
        'indexer_rss_subscription_disable',
        'indexer_rss_subscription_disable_v1',
        'indexer_rss_subscription_get',
        'indexer_rss_subscription_get_v1',
        'indexer_rss_subscription_set',
        'indexer_rss_subscription_set_v1',
        'indexer_search_profile_list',
        'indexer_search_profile_list_v1',
        'indexer_source_reputation_list',
        'indexer_source_reputation_list_v1',
        'indexer_torznab_instance_list',
        'indexer_torznab_instance_list_v1',
        'job_claim_lease_seconds',
        'job_claim_lease_seconds_v1',
        'job_claim_next',
        'job_claim_next_v1',
        'job_run_base_score_refresh_recent',
        'job_run_base_score_refresh_recent_v1',
        'job_run_base_score_refresh_recent_v2',
        'job_run_canonical_backfill_best_source',
        'job_run_canonical_backfill_best_source_v1',
        'job_run_canonical_backfill_best_source_v2',
        'job_run_canonical_prune_low_confidence',
        'job_run_canonical_prune_low_confidence_v2',
        'job_run_connectivity_profile_refresh',
        'job_run_connectivity_profile_refresh_v1',
        'job_run_connectivity_profile_refresh_v2',
        'job_run_policy_snapshot_gc',
        'job_run_policy_snapshot_gc_v1',
        'job_run_policy_snapshot_gc_v2',
        'job_run_policy_snapshot_refcount_repair',
        'job_run_policy_snapshot_refcount_repair_v1',
        'job_run_policy_snapshot_refcount_repair_v2',
        'job_run_rate_limit_state_purge',
        'job_run_rate_limit_state_purge_v1',
        'job_run_rate_limit_state_purge_v2',
        'job_run_reputation_rollup',
        'job_run_reputation_rollup_v1',
        'job_run_reputation_rollup_v2',
        'job_run_retention_purge',
        'job_run_retention_purge_v1',
        'job_run_retention_purge_v2',
        'job_run_rss_subscription_backfill',
        'job_run_rss_subscription_backfill_v1',
        'job_run_rss_subscription_backfill_v2',
        'job_schedule_mark_completed',
        'job_schedule_mark_completed_v1',
        'log_source_metadata_conflict',
        'log_source_metadata_conflict_v1',
        'media_domain_seed_defaults',
        'media_domain_to_torznab_category_delete',
        'media_domain_to_torznab_category_delete_v1',
        'media_domain_to_torznab_category_upsert',
        'media_domain_to_torznab_category_upsert_v1',
        'normalize_magnet_uri',
        'normalize_magnet_uri_v1',
        'normalize_title',
        'normalize_title_v1',
        'outbound_request_log_write',
        'outbound_request_log_write_v1',
        'policy_action_to_decision_type',
        'policy_int_match',
        'policy_int_match_v1',
        'policy_release_group_match',
        'policy_release_group_match_v1',
        'policy_rule_create',
        'policy_rule_create_v1',
        'policy_rule_disable',
        'policy_rule_disable_v1',
        'policy_rule_enable',
        'policy_rule_enable_v1',
        'policy_rule_reorder',
        'policy_rule_reorder_v1',
        'policy_set_create',
        'policy_set_create_v1',
        'policy_set_disable',
        'policy_set_disable_v1',
        'policy_set_enable',
        'policy_set_enable_v1',
        'policy_set_reorder',
        'policy_set_reorder_v1',
        'policy_set_update',
        'policy_set_update_v1',
        'policy_text_match',
        'policy_text_match_v1',
        'policy_uuid_match',
        'policy_uuid_match_v1',
        'random_jitter_seconds',
        'rate_limit_policy_create',
        'rate_limit_policy_create_v1',
        'rate_limit_policy_soft_delete',
        'rate_limit_policy_soft_delete_v1',
        'rate_limit_policy_update',
        'rate_limit_policy_update_v1',
        'rate_limit_try_consume',
        'rate_limit_try_consume_v1',
        'revaer_bump_revision',
        'revaer_config.bump_app_profile_version',
        'revaer_config.bump_revision',
        'revaer_config.cleanup_expired_setup_tokens',
        'revaer_config.consume_setup_token',
        'revaer_config.delete_api_key',
        'revaer_config.delete_secret',
        'revaer_config.factory_reset',
        'revaer_config.fetch_active_setup_token',
        'revaer_config.fetch_api_key_auth',
        'revaer_config.fetch_api_key_hash',
        'revaer_config.fetch_api_keys',
        'revaer_config.fetch_app_profile_row',
        'revaer_config.fetch_engine_profile_row',
        'revaer_config.fetch_fs_policy_row',
        'revaer_config.fetch_revision',
        'revaer_config.fetch_secret_by_name',
        'revaer_config.insert_api_key',
        'revaer_config.insert_setup_token',
        'revaer_config.invalidate_active_setup_tokens',
        'revaer_config.list_app_label_policies',
        'revaer_config.replace_app_label_policies',
        'revaer_config.set_engine_alt_speed',
        'revaer_config.set_engine_ip_filter',
        'revaer_config.set_engine_list_values',
        'revaer_config.set_fs_list',
        'revaer_config.set_peer_classes',
        'revaer_config.set_tracker_config',
        'revaer_config.update_api_key_enabled',
        'revaer_config.update_api_key_expires_at',
        'revaer_config.update_api_key_hash',
        'revaer_config.update_api_key_label',
        'revaer_config.update_api_key_rate_limit',
        'revaer_config.update_app_auth_mode',
        'revaer_config.update_app_bind_addr',
        'revaer_config.update_app_http_port',
        'revaer_config.update_app_immutable_keys',
        'revaer_config.update_app_instance_name',
        'revaer_config.update_app_local_networks',
        'revaer_config.update_app_mode',
        'revaer_config.update_app_telemetry',
        'revaer_config.update_engine_profile',
        'revaer_config.update_fs_array_field',
        'revaer_config.update_fs_boolean_field',
        'revaer_config.update_fs_optional_string_field',
        'revaer_config.update_fs_string_field',
        'revaer_config.upsert_secret',
        'revaer_runtime.delete_torrent',
        'revaer_runtime.fs_job_state',
        'revaer_runtime.list_torrent_files',
        'revaer_runtime.list_torrents',
        'revaer_runtime.mark_fs_job_completed',
        'revaer_runtime.mark_fs_job_failed',
        'revaer_runtime.mark_fs_job_started',
        'revaer_runtime.upsert_torrent',
        'revaer_touch_updated_at',
        'routing_policy_bind_secret',
        'routing_policy_bind_secret_v1',
        'routing_policy_create',
        'routing_policy_create_v1',
        'routing_policy_get',
        'routing_policy_get_v1',
        'routing_policy_set_param',
        'routing_policy_set_param_v1',
        'routing_policy_set_rate_limit_policy',
        'routing_policy_set_rate_limit_policy_v1',
        'rss_poll_apply',
        'rss_poll_apply_v1',
        'rss_poll_claim',
        'rss_poll_claim_v1',
        'search_indexer_run_enqueue',
        'search_indexer_run_enqueue_v1',
        'search_indexer_run_mark_canceled',
        'search_indexer_run_mark_canceled_v1',
        'search_indexer_run_mark_failed',
        'search_indexer_run_mark_failed_v1',
        'search_indexer_run_mark_finished',
        'search_indexer_run_mark_finished_v1',
        'search_indexer_run_mark_started',
        'search_indexer_run_mark_started_v1',
        'search_page_fetch',
        'search_page_fetch_v1',
        'search_page_list',
        'search_page_list_v1',
        'search_profile_add_policy_set',
        'search_profile_add_policy_set_v1',
        'search_profile_create',
        'search_profile_create_v1',
        'search_profile_indexer_allow',
        'search_profile_indexer_allow_v1',
        'search_profile_indexer_block',
        'search_profile_indexer_block_v1',
        'search_profile_remove_policy_set',
        'search_profile_remove_policy_set_v1',
        'search_profile_set_default',
        'search_profile_set_default_domain',
        'search_profile_set_default_domain_v1',
        'search_profile_set_default_v1',
        'search_profile_set_domain_allowlist',
        'search_profile_set_domain_allowlist_v1',
        'search_profile_tag_allow',
        'search_profile_tag_allow_v1',
        'search_profile_tag_block',
        'search_profile_tag_block_v1',
        'search_profile_tag_prefer',
        'search_profile_tag_prefer_v1',
        'search_profile_update',
        'search_profile_update_v1',
        'search_request_cancel',
        'search_request_cancel_v1',
        'search_request_create',
        'search_request_create_v1',
        'search_request_explainability',
        'search_request_explainability_v1',
        'search_request_finalize_on_runs_terminal_v1',
        'search_result_ingest',
        'search_result_ingest_v1',
        'secret_create',
        'secret_create_v1',
        'secret_metadata_list',
        'secret_metadata_list_v1',
        'secret_read',
        'secret_read_v1',
        'secret_revoke',
        'secret_revoke_v1',
        'secret_rotate',
        'secret_rotate_v1',
        'secret_session_configure',
        'secret_session_configure_v1',
        'source_metadata_conflict_list',
        'source_metadata_conflict_list_v1',
        'source_metadata_conflict_reopen',
        'source_metadata_conflict_reopen_v1',
        'source_metadata_conflict_resolve',
        'source_metadata_conflict_resolve_v1',
        'tag_create',
        'tag_create_v1',
        'tag_list',
        'tag_list_v1',
        'tag_soft_delete',
        'tag_soft_delete_v1',
        'tag_update',
        'tag_update_v1',
        'torznab_category_list',
        'torznab_category_list_v1',
        'torznab_download_prepare',
        'torznab_download_prepare_v1',
        'torznab_instance_authenticate',
        'torznab_instance_authenticate_v1',
        'torznab_instance_create',
        'torznab_instance_create_v1',
        'torznab_instance_enable_disable',
        'torznab_instance_enable_disable_v1',
        'torznab_instance_rotate_key',
        'torznab_instance_rotate_key_v1',
        'torznab_instance_soft_delete',
        'torznab_instance_soft_delete_v1',
        'tracker_category_mapping_delete',
        'tracker_category_mapping_delete_v1',
        'tracker_category_mapping_resolve_feed',
        'tracker_category_mapping_resolve_feed_v1',
        'tracker_category_mapping_upsert',
        'tracker_category_mapping_upsert_v1',
        'trust_tier_seed_defaults'
  ])
  AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e');

DO $boundary$
DECLARE
    routine record;
    alias_record record;
    definition text;
    grant_record record;
BEGIN
    FOR routine IN SELECT * FROM revaer_boundary_definitions ORDER BY identity LOOP
        definition := routine.definition;
        FOR alias_record IN SELECT DISTINCT old_name, entry_name FROM revaer_boundary_aliases ORDER BY old_name LOOP
            definition := regexp_replace(definition, '\m' || alias_record.old_name || '\M', alias_record.entry_name, 'g');
        END LOOP;
        IF routine.void_result THEN
            definition := replace(definition, 'RETURNS void', 'RETURNS boolean');
            definition := regexp_replace(definition, '\mRETURN\s*;', 'RETURN true;', 'gi');
            definition := regexp_replace(definition, '\mEND;?\s*\$function\$', E'RETURN true;\nEND;\n$function$', 'i');
            IF definition = routine.definition OR definition NOT LIKE '%RETURNS boolean%' THEN
                RAISE EXCEPTION 'could not establish routine outcome: %', routine.identity;
            END IF;
            EXECUTE format('DROP FUNCTION %s', routine.identity);
        END IF;
        EXECUTE definition;
        IF routine.void_result THEN
            EXECUTE format('ALTER FUNCTION %s OWNER TO %I', routine.identity, pg_get_userbyid(routine.owner_id));
            IF routine.privileges IS NOT NULL THEN
                EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC', routine.identity);
                FOR grant_record IN SELECT * FROM aclexplode(routine.privileges) LOOP
                    EXECUTE format('GRANT %s ON FUNCTION %s TO %s%s', grant_record.privilege_type, routine.identity,
                        CASE WHEN grant_record.grantee = 0 THEN 'PUBLIC' ELSE quote_ident(pg_get_userbyid(grant_record.grantee)) END,
                        CASE WHEN grant_record.is_grantable THEN ' WITH GRANT OPTION' ELSE '' END);
                END LOOP;
            END IF;
        END IF;
    END LOOP;
END;
$boundary$;
