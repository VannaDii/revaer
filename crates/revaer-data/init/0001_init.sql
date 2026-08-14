


SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', 'public', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;


CREATE SCHEMA revaer_config;



CREATE SCHEMA revaer_runtime;



CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;



CREATE EXTENSION IF NOT EXISTS unaccent WITH SCHEMA public;



CREATE TYPE public.acquisition_failure_class AS ENUM (
    'dead',
    'dmca',
    'passworded',
    'corrupted',
    'stalled',
    'not_enough_space',
    'auth_error',
    'network_error',
    'client_error',
    'user_canceled',
    'unknown'
);



CREATE TYPE public.acquisition_origin AS ENUM (
    'torznab',
    'ui',
    'api',
    'automation'
);



CREATE TYPE public.acquisition_status AS ENUM (
    'started',
    'succeeded',
    'failed',
    'canceled'
);



CREATE TYPE public.attr_value_type AS ENUM (
    'text',
    'int',
    'bigint',
    'numeric',
    'bool',
    'uuid'
);



CREATE TYPE public.audit_action AS ENUM (
    'create',
    'update',
    'enable',
    'disable',
    'soft_delete',
    'restore',
    'delete'
);



CREATE TYPE public.audit_entity_type AS ENUM (
    'indexer_instance',
    'indexer_instance_field_value',
    'routing_policy',
    'routing_policy_parameter',
    'policy_set',
    'policy_rule',
    'search_profile',
    'search_profile_rule',
    'tag',
    'canonical_disambiguation_rule',
    'torznab_instance',
    'rate_limit_policy',
    'tracker_category_mapping',
    'media_domain_to_torznab_category'
);



CREATE TYPE public.cf_state AS ENUM (
    'clear',
    'challenged',
    'solved',
    'banned',
    'cooldown'
);



CREATE TYPE public.conflict_resolution AS ENUM (
    'accepted_incoming',
    'kept_existing',
    'merged',
    'ignored'
);



CREATE TYPE public.conflict_type AS ENUM (
    'tracker_name',
    'tracker_category',
    'external_id',
    'hash',
    'source_guid'
);



CREATE TYPE public.connectivity_status AS ENUM (
    'healthy',
    'degraded',
    'failing',
    'quarantined'
);



CREATE TYPE public.context_key_type AS ENUM (
    'policy_snapshot',
    'search_profile',
    'search_request'
);



CREATE TYPE public.cursor_type AS ENUM (
    'offset_limit',
    'page_number',
    'since_time',
    'opaque_token'
);



CREATE TYPE public.decision_type AS ENUM (
    'drop_canonical',
    'drop_source',
    'downrank',
    'flag'
);



CREATE TYPE public.depends_on_operator AS ENUM (
    'eq',
    'neq',
    'in_set'
);



CREATE TYPE public.deployment_role AS ENUM (
    'owner',
    'admin',
    'user'
);



CREATE TYPE public.disambiguation_identity_type AS ENUM (
    'infohash_v1',
    'infohash_v2',
    'magnet_hash',
    'canonical_public_id'
);



CREATE TYPE public.disambiguation_rule_type AS ENUM (
    'prevent_merge'
);



CREATE TYPE public.durable_source_attr_key AS ENUM (
    'tracker_name',
    'tracker_category',
    'tracker_subcategory',
    'size_bytes_reported',
    'files_count',
    'imdb_id',
    'tmdb_id',
    'tvdb_id',
    'season',
    'episode',
    'year'
);



CREATE TYPE public.engine AS ENUM (
    'torznab',
    'cardigann'
);



CREATE TYPE public.error_class AS ENUM (
    'dns',
    'tls',
    'timeout',
    'connection_refused',
    'http_403',
    'http_429',
    'http_5xx',
    'parse_error',
    'auth_error',
    'cf_challenge',
    'rate_limited',
    'unknown'
);



CREATE TYPE public.failure_class AS ENUM (
    'coordinator_error',
    'db_error',
    'auth_error',
    'invalid_request',
    'timeout',
    'canceled_by_system'
);



CREATE TYPE public.field_type AS ENUM (
    'string',
    'password',
    'api_key',
    'cookie',
    'token',
    'header_value',
    'number_int',
    'number_decimal',
    'bool',
    'select_single'
);



CREATE TYPE public.health_event_type AS ENUM (
    'identity_conflict'
);



CREATE TYPE public.identifier_type AS ENUM (
    'imdb',
    'tmdb',
    'tvdb'
);



CREATE TYPE public.identity_strategy AS ENUM (
    'infohash_v1',
    'infohash_v2',
    'magnet_hash',
    'title_size_fallback'
);



CREATE TYPE public.import_indexer_result_status AS ENUM (
    'imported_ready',
    'imported_needs_secret',
    'imported_test_failed',
    'unmapped_definition',
    'skipped_duplicate'
);



CREATE TYPE public.import_job_status AS ENUM (
    'pending',
    'running',
    'completed',
    'failed',
    'canceled'
);



CREATE TYPE public.import_payload_format AS ENUM (
    'prowlarr_indexer_json_v1'
);



CREATE TYPE public.import_source AS ENUM (
    'prowlarr_api',
    'prowlarr_backup'
);



CREATE TYPE public.import_source_system AS ENUM (
    'prowlarr'
);



CREATE TYPE public.indexer_health_notification_channel AS ENUM (
    'email',
    'webhook'
);



CREATE TYPE public.indexer_health_notification_threshold AS ENUM (
    'degraded',
    'failing',
    'quarantined'
);



CREATE TYPE public.indexer_instance_migration_state AS ENUM (
    'ready',
    'needs_secret',
    'test_failed',
    'unmapped_definition',
    'duplicate_suspected'
);



CREATE TYPE public.job_key AS ENUM (
    'retention_purge',
    'reputation_rollup_1h',
    'reputation_rollup_24h',
    'reputation_rollup_7d',
    'connectivity_profile_refresh',
    'canonical_backfill_best_source',
    'base_score_refresh_recent',
    'canonical_prune_low_confidence',
    'policy_snapshot_gc',
    'policy_snapshot_refcount_repair',
    'rate_limit_state_purge',
    'rss_poll',
    'rss_subscription_backfill'
);



CREATE TYPE public.media_domain_key AS ENUM (
    'movies',
    'tv',
    'audiobooks',
    'ebooks',
    'software',
    'adult_movies',
    'adult_scenes'
);



CREATE TYPE public.media_job_status AS ENUM (
    'queued',
    'running',
    'verifying',
    'completed',
    'failed',
    'cancelled'
);



CREATE TYPE public.observation_attr_key AS ENUM (
    'tracker_name',
    'tracker_category',
    'tracker_subcategory',
    'size_bytes_reported',
    'files_count',
    'imdb_id',
    'tmdb_id',
    'tvdb_id',
    'season',
    'episode',
    'year',
    'release_group',
    'freeleech',
    'internal_flag',
    'scene_flag',
    'minimum_ratio',
    'minimum_seed_time_hours',
    'language_primary',
    'subtitles_primary'
);



CREATE TYPE public.outbound_request_outcome AS ENUM (
    'success',
    'failure'
);



CREATE TYPE public.outbound_request_type AS ENUM (
    'caps',
    'search',
    'tvsearch',
    'moviesearch',
    'rss',
    'probe'
);



CREATE TYPE public.outbound_via_mitigation AS ENUM (
    'none',
    'proxy',
    'flaresolverr'
);



CREATE TYPE public.policy_action AS ENUM (
    'drop_canonical',
    'drop_source',
    'downrank',
    'require',
    'prefer',
    'flag'
);



CREATE TYPE public.policy_match_field AS ENUM (
    'infohash_v1',
    'infohash_v2',
    'magnet_hash',
    'title',
    'release_group',
    'uploader',
    'tracker',
    'indexer_instance_public_id',
    'media_domain_key',
    'trust_tier_key',
    'trust_tier_rank'
);



CREATE TYPE public.policy_match_operator AS ENUM (
    'eq',
    'contains',
    'regex',
    'starts_with',
    'ends_with',
    'in_set'
);



CREATE TYPE public.policy_rule_type AS ENUM (
    'block_infohash_v1',
    'block_infohash_v2',
    'block_magnet',
    'block_title_regex',
    'block_release_group',
    'block_uploader',
    'block_tracker',
    'block_indexer_instance',
    'allow_release_group',
    'allow_title_regex',
    'allow_indexer_instance',
    'downrank_title_regex',
    'require_trust_tier_min',
    'require_media_domain',
    'prefer_indexer_instance',
    'prefer_trust_tier'
);



CREATE TYPE public.policy_rule_value_item AS (
	value_text character varying,
	value_int integer,
	value_bigint bigint,
	value_uuid uuid
);



CREATE TYPE public.policy_scope AS ENUM (
    'global',
    'user',
    'profile',
    'request'
);



CREATE TYPE public.policy_severity AS ENUM (
    'hard',
    'soft'
);



CREATE TYPE public.protocol AS ENUM (
    'torrent',
    'usenet'
);



CREATE TYPE public.query_type AS ENUM (
    'free_text',
    'imdb',
    'tmdb',
    'tvdb',
    'season_episode'
);



CREATE TYPE public.rate_limit_scope AS ENUM (
    'indexer_instance',
    'routing_policy'
);



CREATE TYPE public.reputation_window AS ENUM (
    '1h',
    '24h',
    '7d'
);



CREATE TYPE public.routing_param_key AS ENUM (
    'verify_tls',
    'proxy_host',
    'proxy_port',
    'proxy_username',
    'proxy_use_tls',
    'http_proxy_auth',
    'socks_host',
    'socks_port',
    'socks_username',
    'socks_proxy_auth',
    'fs_url',
    'fs_timeout_ms',
    'fs_session_ttl_seconds',
    'fs_user_agent'
);



CREATE TYPE public.routing_policy_mode AS ENUM (
    'direct',
    'http_proxy',
    'socks_proxy',
    'flaresolverr',
    'vpn_route',
    'tor'
);



CREATE TYPE public.run_status AS ENUM (
    'queued',
    'running',
    'finished',
    'failed',
    'canceled'
);



CREATE TYPE public.scoring_context AS ENUM (
    'global_current'
);



CREATE TYPE public.search_status AS ENUM (
    'running',
    'canceled',
    'finished',
    'failed'
);



CREATE TYPE public.secret_audit_action AS ENUM (
    'create',
    'rotate',
    'revoke',
    'bind',
    'unbind'
);



CREATE TYPE public.secret_binding_name AS ENUM (
    'api_key',
    'password',
    'cookie',
    'token',
    'header_value',
    'proxy_password',
    'socks_password'
);



CREATE TYPE public.secret_bound_table AS ENUM (
    'indexer_instance_field_value',
    'routing_policy_parameter'
);



CREATE TYPE public.secret_type AS ENUM (
    'api_key',
    'password',
    'cookie',
    'token',
    'header_value'
);



CREATE TYPE public.signal_key AS ENUM (
    'release_group',
    'resolution',
    'source_type',
    'codec',
    'audio_codec',
    'container',
    'language',
    'subtitles',
    'edition',
    'year',
    'season',
    'episode'
);



CREATE TYPE public.source_metadata_conflict_action AS ENUM (
    'created',
    'resolved',
    'reopened',
    'ignored'
);



CREATE TYPE public.torrent_client_name AS ENUM (
    'revaer_internal',
    'transmission',
    'qbittorrent',
    'deluge',
    'rtorrent',
    'aria2',
    'unknown'
);



CREATE TYPE public.torznab_mode AS ENUM (
    'generic',
    'tv',
    'movie'
);



CREATE TYPE public.trust_tier_key AS ENUM (
    'public',
    'semi_private',
    'private',
    'invite_only'
);



CREATE TYPE public.upstream_source AS ENUM (
    'prowlarr_indexers',
    'cardigann'
);



CREATE TYPE public.user_action AS ENUM (
    'viewed',
    'selected',
    'deselected',
    'downloaded',
    'blocked',
    'reported_fake',
    'preferred_source',
    'separated_canonical',
    'feedback_positive',
    'feedback_negative'
);



CREATE TYPE public.user_action_kv_key AS ENUM (
    'ui_surface',
    'device',
    'chosen_indexer_instance_public_id',
    'chosen_source_public_id',
    'note_short'
);



CREATE TYPE public.user_reason_code AS ENUM (
    'none',
    'wrong_title',
    'wrong_language',
    'wrong_quality',
    'suspicious',
    'known_bad_group',
    'dmca_risk',
    'dead_torrent',
    'duplicate',
    'personal_preference',
    'other'
);



CREATE TYPE public.validation_type AS ENUM (
    'min_length',
    'max_length',
    'min_value',
    'max_value',
    'regex',
    'allowed_value',
    'required_if_field_equals'
);



CREATE TYPE public.value_set_type AS ENUM (
    'text',
    'int',
    'bigint',
    'uuid'
);



CREATE TYPE revaer_runtime.fs_status AS ENUM (
    'pending',
    'moving',
    'moved',
    'failed',
    'skipped'
);



CREATE TYPE revaer_runtime.torrent_state AS ENUM (
    'queued',
    'fetching_metadata',
    'downloading',
    'seeding',
    'completed',
    'failed',
    'stopped'
);



CREATE FUNCTION public.policy_action_to_decision_type(action_input public.policy_action) RETURNS public.decision_type
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT CASE action_input
        WHEN 'drop_canonical'::policy_action THEN 'drop_canonical'::decision_type
        WHEN 'drop_source'::policy_action THEN 'drop_source'::decision_type
        WHEN 'downrank'::policy_action THEN 'downrank'::decision_type
        WHEN 'flag'::policy_action THEN 'flag'::decision_type
        WHEN 'require'::policy_action THEN 'flag'::decision_type
        WHEN 'prefer'::policy_action THEN 'flag'::decision_type
    END;
$$;



CREATE CAST (public.policy_action AS public.decision_type) WITH FUNCTION public.policy_action_to_decision_type(public.policy_action) AS ASSIGNMENT;



CREATE FUNCTION public.app_user_create(email_input character varying, display_name_input character varying) RETURNS uuid
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT app_user_create_v1(email_input => email_input, display_name_input => display_name_input);
$$;



CREATE FUNCTION public.app_user_create_v1(email_input character varying, display_name_input character varying) RETURNS uuid
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to create app user';
    errcode CONSTANT text := 'P0001';
    normalized_email VARCHAR(320);
    new_public_id UUID;
    trimmed_email VARCHAR(320);
    trimmed_display_name VARCHAR(256);
BEGIN
    IF email_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'email_missing';
    END IF;

    trimmed_email := trim(email_input);
    normalized_email := lower(trimmed_email);

    IF normalized_email = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'email_empty';
    END IF;

    IF display_name_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'display_name_missing';
    END IF;

    trimmed_display_name := trim(display_name_input);

    IF trimmed_display_name = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'display_name_empty';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM app_user
        WHERE email_normalized = normalized_email
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'email_already_registered';
    END IF;

    new_public_id := gen_random_uuid();

    INSERT INTO app_user (
        user_public_id,
        email,
        email_normalized,
        is_email_verified,
        display_name,
        role
    )
    VALUES (
        new_public_id,
        trimmed_email,
        normalized_email,
        FALSE,
        trimmed_display_name,
        'user'
    )
    RETURNING user_public_id INTO new_public_id;

    RETURN new_public_id;
END;
$$;



CREATE FUNCTION public.app_user_update(user_public_id_input uuid, display_name_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM app_user_update_v1(user_public_id_input => user_public_id_input, display_name_input => display_name_input);
END;
$$;



CREATE FUNCTION public.app_user_update_v1(user_public_id_input uuid, display_name_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to update app user';
    errcode CONSTANT text := 'P0001';
    trimmed_display_name VARCHAR(256);
BEGIN
    IF user_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'user_missing';
    END IF;

    IF display_name_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'display_name_missing';
    END IF;

    trimmed_display_name := trim(display_name_input);

    IF trimmed_display_name = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'display_name_empty';
    END IF;

    UPDATE app_user
    SET display_name = trimmed_display_name
    WHERE user_public_id = user_public_id_input;

    IF NOT FOUND THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'user_not_found';
    END IF;
END;
$$;



CREATE FUNCTION public.app_user_verify_email(user_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM app_user_verify_email_v1(user_public_id_input => user_public_id_input);
END;
$$;



CREATE FUNCTION public.app_user_verify_email_v1(user_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$DECLARE
    base_message CONSTANT text := 'Failed to verify app user email';
    errcode CONSTANT text := 'P0001';

BEGIN
    IF user_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'user_missing';
    END IF;

    UPDATE app_user
    SET is_email_verified = TRUE
    WHERE user_public_id = user_public_id_input;

    IF NOT FOUND THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'user_not_found';
    END IF;
END;
$$;



CREATE FUNCTION public.canonical_disambiguation_rule_create(actor_user_public_id uuid, rule_type_input public.disambiguation_rule_type, identity_left_type_input public.disambiguation_identity_type, identity_left_value_text_input character varying, identity_left_value_uuid_input uuid, identity_right_type_input public.disambiguation_identity_type, identity_right_value_text_input character varying, identity_right_value_uuid_input uuid, reason_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM canonical_disambiguation_rule_create_v1(actor_user_public_id => actor_user_public_id, rule_type_input => rule_type_input, identity_left_type_input => identity_left_type_input, identity_left_value_text_input => identity_left_value_text_input, identity_left_value_uuid_input => identity_left_value_uuid_input, identity_right_type_input => identity_right_type_input, identity_right_value_text_input => identity_right_value_text_input, identity_right_value_uuid_input => identity_right_value_uuid_input, reason_input => reason_input);
END;
$$;



CREATE FUNCTION public.canonical_disambiguation_rule_create_v1(actor_user_public_id uuid, rule_type_input public.disambiguation_rule_type, identity_left_type_input public.disambiguation_identity_type, identity_left_value_text_input character varying, identity_left_value_uuid_input uuid, identity_right_type_input public.disambiguation_identity_type, identity_right_value_text_input character varying, identity_right_value_uuid_input uuid, reason_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    base_message CONSTANT text := 'Failed to create disambiguation rule';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    left_text_norm VARCHAR(64);
    right_text_norm VARCHAR(64);
    left_uuid_value UUID;
    right_uuid_value UUID;
    left_type_value disambiguation_identity_type;
    right_type_value disambiguation_identity_type;
    rule_id BIGINT;
    should_swap BOOLEAN;
    temp_text VARCHAR(64);
    temp_uuid UUID;
    temp_type disambiguation_identity_type;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF rule_type_input IS NULL
        OR identity_left_type_input IS NULL
        OR identity_right_type_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'rule_definition_missing';
    END IF;

    IF reason_input IS NOT NULL AND char_length(reason_input) > 256 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'reason_too_long';
    END IF;

    left_type_value := identity_left_type_input;
    right_type_value := identity_right_type_input;

    left_text_norm := NULL;
    right_text_norm := NULL;
    left_uuid_value := NULL;
    right_uuid_value := NULL;

    IF left_type_value = 'canonical_public_id' THEN
        IF identity_left_value_text_input IS NOT NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'left_value_invalid';
        END IF;
        IF identity_left_value_uuid_input IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'left_value_missing';
        END IF;
        left_uuid_value := identity_left_value_uuid_input;
    ELSE
        IF identity_left_value_uuid_input IS NOT NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'left_value_invalid';
        END IF;
        IF identity_left_value_text_input IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'left_value_missing';
        END IF;
        left_text_norm := lower(trim(identity_left_value_text_input));
        IF left_text_norm = '' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'left_value_invalid';
        END IF;
        IF char_length(left_text_norm) > 64 THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'left_value_invalid';
        END IF;
    END IF;

    IF right_type_value = 'canonical_public_id' THEN
        IF identity_right_value_text_input IS NOT NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'right_value_invalid';
        END IF;
        IF identity_right_value_uuid_input IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'right_value_missing';
        END IF;
        right_uuid_value := identity_right_value_uuid_input;
    ELSE
        IF identity_right_value_uuid_input IS NOT NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'right_value_invalid';
        END IF;
        IF identity_right_value_text_input IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'right_value_missing';
        END IF;
        right_text_norm := lower(trim(identity_right_value_text_input));
        IF right_text_norm = '' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'right_value_invalid';
        END IF;
        IF char_length(right_text_norm) > 64 THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'right_value_invalid';
        END IF;
    END IF;

    IF left_type_value IN ('infohash_v1', 'infohash_v2', 'magnet_hash') THEN
        IF left_text_norm IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'left_value_invalid';
        END IF;
        IF left_type_value = 'infohash_v1' AND left_text_norm !~ '^[0-9a-f]{40}$' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'left_value_invalid';
        END IF;
        IF left_type_value IN ('infohash_v2', 'magnet_hash')
            AND left_text_norm !~ '^[0-9a-f]{64}$' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'left_value_invalid';
        END IF;
    END IF;

    IF right_type_value IN ('infohash_v1', 'infohash_v2', 'magnet_hash') THEN
        IF right_text_norm IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'right_value_invalid';
        END IF;
        IF right_type_value = 'infohash_v1' AND right_text_norm !~ '^[0-9a-f]{40}$' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'right_value_invalid';
        END IF;
        IF right_type_value IN ('infohash_v2', 'magnet_hash')
            AND right_text_norm !~ '^[0-9a-f]{64}$' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'right_value_invalid';
        END IF;
    END IF;

    IF ROW(left_type_value, left_text_norm, left_uuid_value)
        = ROW(right_type_value, right_text_norm, right_uuid_value) THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'rule_identity_equal';
    END IF;

    should_swap := ROW(left_type_value, left_text_norm, left_uuid_value)
        > ROW(right_type_value, right_text_norm, right_uuid_value);

    IF should_swap THEN
        temp_type := left_type_value;
        left_type_value := right_type_value;
        right_type_value := temp_type;

        temp_text := left_text_norm;
        left_text_norm := right_text_norm;
        right_text_norm := temp_text;

        temp_uuid := left_uuid_value;
        left_uuid_value := right_uuid_value;
        right_uuid_value := temp_uuid;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM canonical_disambiguation_rule
        WHERE identity_left_type = left_type_value
          AND identity_left_value_text IS NOT DISTINCT FROM left_text_norm
          AND identity_left_value_uuid IS NOT DISTINCT FROM left_uuid_value
          AND identity_right_type = right_type_value
          AND identity_right_value_text IS NOT DISTINCT FROM right_text_norm
          AND identity_right_value_uuid IS NOT DISTINCT FROM right_uuid_value
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'rule_exists';
    END IF;

    INSERT INTO canonical_disambiguation_rule (
        created_by_user_id,
        rule_type,
        identity_left_type,
        identity_left_value_text,
        identity_left_value_uuid,
        identity_right_type,
        identity_right_value_text,
        identity_right_value_uuid,
        reason
    )
    VALUES (
        actor_user_id,
        rule_type_input,
        left_type_value,
        left_text_norm,
        left_uuid_value,
        right_type_value,
        right_text_norm,
        right_uuid_value,
        reason_input
    )
    RETURNING canonical_disambiguation_rule_id INTO rule_id;

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'canonical_disambiguation_rule',
        rule_id,
        NULL,
        'create',
        actor_user_id,
        'canonical_disambiguation_rule_create'
    );
END;
$_$;



CREATE FUNCTION public.canonical_merge_by_infohash(infohash_v2_input character, infohash_v1_input character) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM canonical_merge_by_infohash_v1(infohash_v2_input => infohash_v2_input, infohash_v1_input => infohash_v1_input);
END;
$$;



CREATE FUNCTION public.canonical_merge_by_infohash_v1(infohash_v2_input character, infohash_v1_input character) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    base_message CONSTANT text := 'Failed to merge canonical';
    errcode CONSTANT text := 'P0001';
    infohash_v2_value TEXT;
    infohash_v1_value TEXT;
    hash_type disambiguation_identity_type;
    hash_value TEXT;
    candidate_count INTEGER;
    winner_id BIGINT;
    winner_public_id UUID;
    winner_strategy identity_strategy;
    winner_infohash_v2 TEXT;
    winner_infohash_v1 TEXT;
    winner_magnet_hash TEXT;
    loser_id BIGINT;
    loser_public_id UUID;
    v2_distinct INTEGER;
    v1_distinct INTEGER;
    magnet_distinct INTEGER;
    v2_value TEXT;
    v1_value TEXT;
    magnet_value TEXT;
    best_imdb_id TEXT;
    best_tmdb_id INTEGER;
    best_tvdb_id INTEGER;
    sample_count INTEGER;
    sample_median NUMERIC(20,4);
    sample_min BIGINT;
    sample_max BIGINT;
    sample_first BIGINT;
    best_title_display TEXT;
BEGIN
    infohash_v2_value := NULLIF(lower(infohash_v2_input), '');
    infohash_v1_value := NULLIF(lower(infohash_v1_input), '');

    IF infohash_v2_value IS NULL AND infohash_v1_value IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'hash_missing';
    END IF;

    IF infohash_v2_value IS NOT NULL THEN
        IF infohash_v2_value !~ '^[0-9a-f]{64}$' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'hash_invalid';
        END IF;
        hash_type := 'infohash_v2';
        hash_value := infohash_v2_value;
    ELSE
        IF infohash_v1_value !~ '^[0-9a-f]{40}$' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'hash_invalid';
        END IF;
        hash_type := 'infohash_v1';
        hash_value := infohash_v1_value;
    END IF;

    CREATE TEMP TABLE tmp_merge_candidates ON COMMIT DROP AS
    SELECT canonical_torrent_id,
           canonical_torrent_public_id,
           created_at,
           identity_strategy,
           infohash_v2,
           infohash_v1,
           magnet_hash,
           title_normalized,
           size_bytes
    FROM canonical_torrent
    WHERE (infohash_v2_value IS NOT NULL AND infohash_v2 = infohash_v2_value)
       OR (infohash_v2_value IS NULL AND infohash_v1 = infohash_v1_value);

    SELECT COUNT(*)
    INTO candidate_count
    FROM tmp_merge_candidates;

    IF candidate_count < 2 THEN
        RETURN;
    END IF;

    SELECT canonical_torrent_id,
           canonical_torrent_public_id,
           identity_strategy,
           infohash_v2,
           infohash_v1,
           magnet_hash
    INTO winner_id,
         winner_public_id,
         winner_strategy,
         winner_infohash_v2,
         winner_infohash_v1,
         winner_magnet_hash
    FROM tmp_merge_candidates
    ORDER BY created_at ASC, canonical_torrent_id ASC
    LIMIT 1;

    FOR loser_id, loser_public_id IN
        SELECT canonical_torrent_id, canonical_torrent_public_id
        FROM tmp_merge_candidates
        WHERE canonical_torrent_id <> winner_id
    LOOP
        IF EXISTS (
            SELECT 1
            FROM canonical_disambiguation_rule
            WHERE rule_type = 'prevent_merge'
              AND (
                  (
                      identity_left_type = 'canonical_public_id'
                      AND identity_right_type = 'canonical_public_id'
                      AND identity_left_value_uuid = LEAST(winner_public_id, loser_public_id)
                      AND identity_right_value_uuid = GREATEST(winner_public_id, loser_public_id)
                  )
                  OR (
                      identity_left_type = 'canonical_public_id'
                      AND identity_left_value_uuid = winner_public_id
                      AND identity_right_type = hash_type
                      AND identity_right_value_text = hash_value
                  )
                  OR (
                      identity_right_type = 'canonical_public_id'
                      AND identity_right_value_uuid = winner_public_id
                      AND identity_left_type = hash_type
                      AND identity_left_value_text = hash_value
                  )
                  OR (
                      identity_left_type = 'canonical_public_id'
                      AND identity_left_value_uuid = loser_public_id
                      AND identity_right_type = hash_type
                      AND identity_right_value_text = hash_value
                  )
                  OR (
                      identity_right_type = 'canonical_public_id'
                      AND identity_right_value_uuid = loser_public_id
                      AND identity_left_type = hash_type
                      AND identity_left_value_text = hash_value
                  )
              )
        ) THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'prevent_merge';
        END IF;
    END LOOP;

    SELECT COUNT(DISTINCT infohash_v2), MIN(infohash_v2)
    INTO v2_distinct, v2_value
    FROM tmp_merge_candidates
    WHERE infohash_v2 IS NOT NULL;

    SELECT COUNT(DISTINCT infohash_v1), MIN(infohash_v1)
    INTO v1_distinct, v1_value
    FROM tmp_merge_candidates
    WHERE infohash_v1 IS NOT NULL;

    SELECT COUNT(DISTINCT magnet_hash), MIN(magnet_hash)
    INTO magnet_distinct, magnet_value
    FROM tmp_merge_candidates
    WHERE magnet_hash IS NOT NULL;

    IF winner_infohash_v2 IS NULL AND v2_distinct = 1 THEN
        UPDATE canonical_torrent
        SET infohash_v2 = v2_value,
            updated_at = now()
        WHERE canonical_torrent_id = winner_id;
        winner_infohash_v2 := v2_value;
    END IF;

    IF winner_infohash_v1 IS NULL AND v1_distinct = 1 THEN
        UPDATE canonical_torrent
        SET infohash_v1 = v1_value,
            updated_at = now()
        WHERE canonical_torrent_id = winner_id;
        winner_infohash_v1 := v1_value;
    END IF;

    IF winner_magnet_hash IS NULL AND magnet_distinct = 1 THEN
        UPDATE canonical_torrent
        SET magnet_hash = magnet_value,
            updated_at = now()
        WHERE canonical_torrent_id = winner_id;
        winner_magnet_hash := magnet_value;
    END IF;

    FOR loser_id, loser_public_id IN
        SELECT canonical_torrent_id, canonical_torrent_public_id
        FROM tmp_merge_candidates
        WHERE canonical_torrent_id <> winner_id
    LOOP
        INSERT INTO canonical_external_id (
            canonical_torrent_id,
            id_type,
            id_value_text,
            trust_tier_rank,
            first_seen_at,
            last_seen_at,
            source_canonical_torrent_source_id
        )
        SELECT winner_id,
               id_type,
               id_value_text,
               trust_tier_rank,
               first_seen_at,
               last_seen_at,
               source_canonical_torrent_source_id
        FROM canonical_external_id
        WHERE canonical_torrent_id = loser_id
          AND id_value_text IS NOT NULL
        ON CONFLICT (canonical_torrent_id, id_type, id_value_text)
        DO UPDATE SET
            first_seen_at = LEAST(canonical_external_id.first_seen_at, EXCLUDED.first_seen_at),
            last_seen_at = GREATEST(canonical_external_id.last_seen_at, EXCLUDED.last_seen_at),
            trust_tier_rank = GREATEST(canonical_external_id.trust_tier_rank, EXCLUDED.trust_tier_rank),
            source_canonical_torrent_source_id = COALESCE(
                canonical_external_id.source_canonical_torrent_source_id,
                EXCLUDED.source_canonical_torrent_source_id
            );

        INSERT INTO canonical_external_id (
            canonical_torrent_id,
            id_type,
            id_value_int,
            trust_tier_rank,
            first_seen_at,
            last_seen_at,
            source_canonical_torrent_source_id
        )
        SELECT winner_id,
               id_type,
               id_value_int,
               trust_tier_rank,
               first_seen_at,
               last_seen_at,
               source_canonical_torrent_source_id
        FROM canonical_external_id
        WHERE canonical_torrent_id = loser_id
          AND id_value_int IS NOT NULL
        ON CONFLICT (canonical_torrent_id, id_type, id_value_int)
        DO UPDATE SET
            first_seen_at = LEAST(canonical_external_id.first_seen_at, EXCLUDED.first_seen_at),
            last_seen_at = GREATEST(canonical_external_id.last_seen_at, EXCLUDED.last_seen_at),
            trust_tier_rank = GREATEST(canonical_external_id.trust_tier_rank, EXCLUDED.trust_tier_rank),
            source_canonical_torrent_source_id = COALESCE(
                canonical_external_id.source_canonical_torrent_source_id,
                EXCLUDED.source_canonical_torrent_source_id
            );

        DELETE FROM canonical_external_id
        WHERE canonical_torrent_id = loser_id;

        INSERT INTO canonical_torrent_signal (
            canonical_torrent_id,
            signal_key,
            value_text,
            value_int,
            confidence,
            parser_version
        )
        SELECT winner_id,
               signal_key,
               value_text,
               value_int,
               confidence,
               parser_version
        FROM canonical_torrent_signal
        WHERE canonical_torrent_id = loser_id
        ON CONFLICT (canonical_torrent_id, signal_key, value_text, value_int)
        DO UPDATE SET
            confidence = LEAST(
                1.0,
                GREATEST(canonical_torrent_signal.confidence, EXCLUDED.confidence) + 0.05
            ),
            parser_version = LEAST(canonical_torrent_signal.parser_version, EXCLUDED.parser_version);

        DELETE FROM canonical_torrent_signal
        WHERE canonical_torrent_id = loser_id;

        DELETE FROM canonical_size_sample loser_sample
        USING canonical_size_sample winner_sample
        WHERE loser_sample.canonical_torrent_id = loser_id
          AND winner_sample.canonical_torrent_id = winner_id
          AND loser_sample.observed_at = winner_sample.observed_at
          AND loser_sample.size_bytes = winner_sample.size_bytes;

        UPDATE canonical_size_sample
        SET canonical_torrent_id = winner_id
        WHERE canonical_torrent_id = loser_id;

        DELETE FROM canonical_size_rollup
        WHERE canonical_torrent_id = loser_id;

        INSERT INTO canonical_torrent_source_base_score (
            canonical_torrent_id,
            canonical_torrent_source_id,
            score_total_base,
            score_seed,
            score_leech,
            score_age,
            score_trust,
            score_health,
            score_reputation,
            computed_at
        )
        SELECT winner_id,
               canonical_torrent_source_id,
               score_total_base,
               score_seed,
               score_leech,
               score_age,
               score_trust,
               score_health,
               score_reputation,
               computed_at
        FROM canonical_torrent_source_base_score
        WHERE canonical_torrent_id = loser_id
        ON CONFLICT (canonical_torrent_id, canonical_torrent_source_id)
        DO UPDATE SET
            score_total_base = CASE
                WHEN EXCLUDED.score_total_base >= canonical_torrent_source_base_score.score_total_base
                    THEN EXCLUDED.score_total_base
                ELSE canonical_torrent_source_base_score.score_total_base
            END,
            score_seed = CASE
                WHEN EXCLUDED.score_total_base >= canonical_torrent_source_base_score.score_total_base
                    THEN EXCLUDED.score_seed
                ELSE canonical_torrent_source_base_score.score_seed
            END,
            score_leech = CASE
                WHEN EXCLUDED.score_total_base >= canonical_torrent_source_base_score.score_total_base
                    THEN EXCLUDED.score_leech
                ELSE canonical_torrent_source_base_score.score_leech
            END,
            score_age = CASE
                WHEN EXCLUDED.score_total_base >= canonical_torrent_source_base_score.score_total_base
                    THEN EXCLUDED.score_age
                ELSE canonical_torrent_source_base_score.score_age
            END,
            score_trust = CASE
                WHEN EXCLUDED.score_total_base >= canonical_torrent_source_base_score.score_total_base
                    THEN EXCLUDED.score_trust
                ELSE canonical_torrent_source_base_score.score_trust
            END,
            score_health = CASE
                WHEN EXCLUDED.score_total_base >= canonical_torrent_source_base_score.score_total_base
                    THEN EXCLUDED.score_health
                ELSE canonical_torrent_source_base_score.score_health
            END,
            score_reputation = CASE
                WHEN EXCLUDED.score_total_base >= canonical_torrent_source_base_score.score_total_base
                    THEN EXCLUDED.score_reputation
                ELSE canonical_torrent_source_base_score.score_reputation
            END,
            computed_at = GREATEST(
                canonical_torrent_source_base_score.computed_at,
                EXCLUDED.computed_at
            );

        DELETE FROM canonical_torrent_source_base_score
        WHERE canonical_torrent_id = loser_id;

        INSERT INTO canonical_torrent_source_context_score (
            context_key_type,
            context_key_id,
            canonical_torrent_id,
            canonical_torrent_source_id,
            score_total_context,
            score_policy_adjust,
            score_tag_adjust,
            is_dropped,
            computed_at
        )
        SELECT context_key_type,
               context_key_id,
               winner_id,
               canonical_torrent_source_id,
               score_total_context,
               score_policy_adjust,
               score_tag_adjust,
               is_dropped,
               computed_at
        FROM canonical_torrent_source_context_score
        WHERE canonical_torrent_id = loser_id
        ON CONFLICT (context_key_type, context_key_id, canonical_torrent_id, canonical_torrent_source_id)
        DO UPDATE SET
            score_total_context = CASE
                WHEN EXCLUDED.score_total_context >= canonical_torrent_source_context_score.score_total_context
                    THEN EXCLUDED.score_total_context
                ELSE canonical_torrent_source_context_score.score_total_context
            END,
            score_policy_adjust = CASE
                WHEN EXCLUDED.score_total_context >= canonical_torrent_source_context_score.score_total_context
                    THEN EXCLUDED.score_policy_adjust
                ELSE canonical_torrent_source_context_score.score_policy_adjust
            END,
            score_tag_adjust = CASE
                WHEN EXCLUDED.score_total_context >= canonical_torrent_source_context_score.score_total_context
                    THEN EXCLUDED.score_tag_adjust
                ELSE canonical_torrent_source_context_score.score_tag_adjust
            END,
            is_dropped = CASE
                WHEN EXCLUDED.score_total_context >= canonical_torrent_source_context_score.score_total_context
                    THEN EXCLUDED.is_dropped
                ELSE canonical_torrent_source_context_score.is_dropped
            END,
            computed_at = GREATEST(
                canonical_torrent_source_context_score.computed_at,
                EXCLUDED.computed_at
            );

        DELETE FROM canonical_torrent_source_context_score
        WHERE canonical_torrent_id = loser_id;

        DELETE FROM canonical_torrent_best_source_global
        WHERE canonical_torrent_id = loser_id;

        DELETE FROM canonical_torrent_best_source_context loser_context
        USING canonical_torrent_best_source_context winner_context
        WHERE loser_context.canonical_torrent_id = loser_id
          AND winner_context.canonical_torrent_id = winner_id
          AND loser_context.context_key_type = winner_context.context_key_type
          AND loser_context.context_key_id = winner_context.context_key_id;

        UPDATE canonical_torrent_best_source_context
        SET canonical_torrent_id = winner_id
        WHERE canonical_torrent_id = loser_id;

        DELETE FROM search_request_canonical loser_link
        USING search_request_canonical winner_link
        WHERE loser_link.canonical_torrent_id = loser_id
          AND winner_link.canonical_torrent_id = winner_id
          AND loser_link.search_request_id = winner_link.search_request_id;

        UPDATE search_request_canonical
        SET canonical_torrent_id = winner_id
        WHERE canonical_torrent_id = loser_id;

        UPDATE search_request_source_observation
        SET canonical_torrent_id = winner_id
        WHERE canonical_torrent_id = loser_id;

        UPDATE search_filter_decision
        SET canonical_torrent_id = winner_id
        WHERE canonical_torrent_id = loser_id;

        UPDATE user_result_action
        SET canonical_torrent_id = winner_id
        WHERE canonical_torrent_id = loser_id;

        UPDATE acquisition_attempt
        SET canonical_torrent_id = winner_id
        WHERE canonical_torrent_id = loser_id;

        DELETE FROM canonical_torrent
        WHERE canonical_torrent_id = loser_id;
    END LOOP;

    SELECT COUNT(*), percentile_cont(0.5) WITHIN GROUP (ORDER BY size_bytes),
           MIN(size_bytes), MAX(size_bytes)
    INTO sample_count, sample_median, sample_min, sample_max
    FROM canonical_size_sample
    WHERE canonical_torrent_id = winner_id;

    IF sample_count IS NOT NULL AND sample_count > 0 THEN
        SELECT size_bytes
        INTO sample_first
        FROM canonical_size_sample
        WHERE canonical_torrent_id = winner_id
        ORDER BY observed_at ASC, canonical_size_sample_id ASC
        LIMIT 1;

        INSERT INTO canonical_size_rollup (
            canonical_torrent_id,
            sample_count,
            size_median,
            size_min,
            size_max,
            updated_at
        )
        VALUES (
            winner_id,
            sample_count,
            sample_median::BIGINT,
            sample_min,
            sample_max,
            now()
        )
        ON CONFLICT (canonical_torrent_id)
        DO UPDATE SET
            sample_count = EXCLUDED.sample_count,
            size_median = EXCLUDED.size_median,
            size_min = EXCLUDED.size_min,
            size_max = EXCLUDED.size_max,
            updated_at = EXCLUDED.updated_at;

        IF winner_strategy <> 'title_size_fallback' THEN
            UPDATE canonical_torrent
            SET size_bytes = CASE
                    WHEN sample_count >= 3 THEN sample_median::BIGINT
                    ELSE COALESCE(sample_first, sample_min)
                END,
                updated_at = now()
            WHERE canonical_torrent_id = winner_id;
        END IF;
    ELSE
        DELETE FROM canonical_size_rollup
        WHERE canonical_torrent_id = winner_id;
    END IF;

    SELECT id_value_text
    INTO best_imdb_id
    FROM canonical_external_id
    WHERE canonical_torrent_id = winner_id
      AND id_type = 'imdb'
    ORDER BY trust_tier_rank DESC,
             last_seen_at DESC,
             canonical_external_id_id ASC
    LIMIT 1;

    IF best_imdb_id IS NOT NULL THEN
        UPDATE canonical_torrent
        SET imdb_id = best_imdb_id,
            updated_at = now()
        WHERE canonical_torrent_id = winner_id;
    END IF;

    SELECT id_value_int
    INTO best_tmdb_id
    FROM canonical_external_id
    WHERE canonical_torrent_id = winner_id
      AND id_type = 'tmdb'
    ORDER BY trust_tier_rank DESC,
             last_seen_at DESC,
             canonical_external_id_id ASC
    LIMIT 1;

    IF best_tmdb_id IS NOT NULL THEN
        UPDATE canonical_torrent
        SET tmdb_id = best_tmdb_id,
            updated_at = now()
        WHERE canonical_torrent_id = winner_id;
    END IF;

    SELECT id_value_int
    INTO best_tvdb_id
    FROM canonical_external_id
    WHERE canonical_torrent_id = winner_id
      AND id_type = 'tvdb'
    ORDER BY trust_tier_rank DESC,
             last_seen_at DESC,
             canonical_external_id_id ASC
    LIMIT 1;

    IF best_tvdb_id IS NOT NULL THEN
        UPDATE canonical_torrent
        SET tvdb_id = best_tvdb_id,
            updated_at = now()
        WHERE canonical_torrent_id = winner_id;
    END IF;

    SELECT obs.title_raw
    INTO best_title_display
    FROM search_request_source_observation obs
    JOIN indexer_instance inst
        ON inst.indexer_instance_id = obs.indexer_instance_id
    LEFT JOIN trust_tier tt
        ON tt.trust_tier_key = inst.trust_tier_key
    LEFT JOIN canonical_torrent_source_base_score bs
        ON bs.canonical_torrent_id = winner_id
       AND bs.canonical_torrent_source_id = obs.canonical_torrent_source_id
    WHERE obs.canonical_torrent_id = winner_id
    ORDER BY COALESCE(tt.rank, 0) DESC,
             COALESCE(bs.score_total_base, 0) DESC,
             obs.observed_at DESC,
             obs.observation_id ASC
    LIMIT 1;

    IF best_title_display IS NOT NULL THEN
        UPDATE canonical_torrent
        SET title_display = best_title_display,
            updated_at = now()
        WHERE canonical_torrent_id = winner_id;
    END IF;

    PERFORM canonical_recompute_best_source_v1(
        (SELECT canonical_torrent_public_id FROM canonical_torrent WHERE canonical_torrent_id = winner_id),
        'global_current'
    );
END;
$_$;



CREATE FUNCTION public.canonical_prune_low_confidence() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM canonical_prune_low_confidence_v1();
END;
$$;



CREATE FUNCTION public.canonical_prune_low_confidence_v1() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    cutoff_ts TIMESTAMPTZ;
BEGIN
    cutoff_ts := now() - make_interval(days => 30);

    WITH candidates AS (
        SELECT canonical_torrent_id,
               infohash_v1,
               infohash_v2,
               magnet_hash
        FROM canonical_torrent
        WHERE identity_strategy = 'title_size_fallback'
          AND identity_confidence <= 0.60
          AND created_at < cutoff_ts
    ),
    filtered AS (
        SELECT c.canonical_torrent_id,
               c.infohash_v1,
               c.infohash_v2,
               c.magnet_hash
        FROM candidates c
        WHERE NOT EXISTS (
            SELECT 1
            FROM acquisition_attempt a
            WHERE a.canonical_torrent_id = c.canonical_torrent_id
               OR (c.infohash_v1 IS NOT NULL AND a.infohash_v1 = c.infohash_v1)
               OR (c.infohash_v2 IS NOT NULL AND a.infohash_v2 = c.infohash_v2)
               OR (c.magnet_hash IS NOT NULL AND a.magnet_hash = c.magnet_hash)
        )
          AND NOT EXISTS (
            SELECT 1
            FROM user_result_action ura
            WHERE ura.canonical_torrent_id = c.canonical_torrent_id
              AND ura.action IN ('selected', 'downloaded')
        )
    ),
    canonical_source_link AS (
        SELECT canonical_torrent_id,
               canonical_torrent_source_id
        FROM canonical_torrent_source_base_score
        UNION
        SELECT canonical_torrent_id,
               canonical_torrent_source_id
        FROM canonical_torrent_source_context_score
        UNION
        SELECT canonical_torrent_id,
               canonical_torrent_source_id
        FROM canonical_torrent_best_source_global
        UNION
        SELECT canonical_torrent_id,
               canonical_torrent_source_id
        FROM canonical_torrent_best_source_context
    ),
    candidate_sources AS (
        SELECT link.canonical_torrent_id,
               link.canonical_torrent_source_id
        FROM canonical_source_link link
        JOIN filtered candidate
            ON candidate.canonical_torrent_id = link.canonical_torrent_id
    ),
    sources_with_non_candidate_links AS (
        SELECT DISTINCT source_link.canonical_torrent_id
        FROM candidate_sources source_link
        JOIN canonical_source_link link
            ON link.canonical_torrent_source_id = source_link.canonical_torrent_source_id
           AND link.canonical_torrent_id <> source_link.canonical_torrent_id
        LEFT JOIN filtered candidate
            ON candidate.canonical_torrent_id = link.canonical_torrent_id
        WHERE candidate.canonical_torrent_id IS NULL
    ),
    eligible AS (
        SELECT candidate.canonical_torrent_id
        FROM filtered candidate
        WHERE NOT EXISTS (
            SELECT 1
            FROM sources_with_non_candidate_links blocked
            WHERE blocked.canonical_torrent_id = candidate.canonical_torrent_id
        )
    )
    DELETE FROM canonical_torrent
    WHERE canonical_torrent_id IN (
        SELECT canonical_torrent_id
        FROM eligible
    );
END;
$$;



CREATE FUNCTION public.canonical_recompute_best_source(canonical_torrent_public_id_input uuid, scoring_context_input public.scoring_context DEFAULT 'global_current'::public.scoring_context) RETURNS uuid
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT canonical_recompute_best_source_v1(canonical_torrent_public_id_input => canonical_torrent_public_id_input, scoring_context_input => scoring_context_input);
$$;



CREATE FUNCTION public.canonical_recompute_best_source_v1(canonical_torrent_public_id_input uuid, scoring_context_input public.scoring_context DEFAULT 'global_current'::public.scoring_context) RETURNS uuid
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to recompute best source';
    errcode CONSTANT text := 'P0001';
    canonical_id BIGINT;
    winner_source_id BIGINT;
    winner_source_public_id UUID;
    context_value scoring_context;
BEGIN
    IF canonical_torrent_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'canonical_missing';
    END IF;

    context_value := COALESCE(scoring_context_input, 'global_current');
    IF context_value <> 'global_current' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'scoring_context_invalid';
    END IF;

    SELECT canonical_torrent_id
    INTO canonical_id
    FROM canonical_torrent
    WHERE canonical_torrent_public_id = canonical_torrent_public_id_input;

    IF canonical_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'canonical_not_found';
    END IF;

    SELECT bs.canonical_torrent_source_id,
           src.canonical_torrent_source_public_id
    INTO winner_source_id,
         winner_source_public_id
    FROM canonical_torrent_source_base_score bs
    JOIN canonical_torrent_source src
        ON src.canonical_torrent_source_id = bs.canonical_torrent_source_id
    WHERE bs.canonical_torrent_id = canonical_id
    ORDER BY bs.score_total_base DESC,
             src.last_seen_at DESC,
             src.canonical_torrent_source_public_id ASC
    LIMIT 1;

    IF winner_source_id IS NULL THEN
        SELECT src.canonical_torrent_source_id,
               src.canonical_torrent_source_public_id
        INTO winner_source_id,
             winner_source_public_id
        FROM canonical_torrent_source src
        WHERE EXISTS (
            SELECT 1
            FROM search_request_source_observation obs
            WHERE obs.canonical_torrent_id = canonical_id
              AND obs.canonical_torrent_source_id = src.canonical_torrent_source_id
        )
        ORDER BY COALESCE(src.last_seen_seeders, 0) DESC,
                 src.last_seen_at DESC,
                 src.canonical_torrent_source_public_id ASC
        LIMIT 1;
    END IF;

    IF winner_source_id IS NOT NULL THEN
        INSERT INTO canonical_torrent_best_source_global (
            canonical_torrent_id,
            canonical_torrent_source_id,
            computed_at
        )
        VALUES (
            canonical_id,
            winner_source_id,
            now()
        )
        ON CONFLICT (canonical_torrent_id)
        DO UPDATE SET
            canonical_torrent_source_id = EXCLUDED.canonical_torrent_source_id,
            computed_at = EXCLUDED.computed_at;
    END IF;

    RETURN winner_source_public_id;
END;
$$;



CREATE FUNCTION public.compute_title_size_hash(title_normalized_input text, size_bytes_input bigint) RETURNS text
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT compute_title_size_hash_v1(title_normalized_input => title_normalized_input, size_bytes_input => size_bytes_input);
$$;



CREATE FUNCTION public.compute_title_size_hash_v1(title_normalized_input text, size_bytes_input bigint) RETURNS text
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    hash_hex TEXT;
BEGIN
    IF title_normalized_input IS NULL OR size_bytes_input IS NULL THEN
        RETURN NULL;
    END IF;

    hash_hex := encode(
        digest(title_normalized_input || '|' || size_bytes_input::TEXT, 'sha256'),
        'hex'
    );
    RETURN lower(hash_hex);
END;
$$;



CREATE FUNCTION public.deployment_init(actor_user_public_id uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM deployment_init_v1(actor_user_public_id => actor_user_public_id);
END;
$$;



CREATE FUNCTION public.deployment_init_v1(actor_user_public_id uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to initialize deployment';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    actor_verified BOOLEAN;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role, is_email_verified
    INTO actor_user_id, actor_role, actor_verified
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_verified IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unverified';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    INSERT INTO app_user (
        user_id,
        user_public_id,
        email,
        email_normalized,
        is_email_verified,
        display_name,
        role,
        created_at
    ) OVERRIDING SYSTEM VALUE
    SELECT
        0,
        '00000000-0000-0000-0000-000000000000',
        'system@revaer.local',
        'system@revaer.local',
        TRUE,
        'System',
        'owner',
        now()
    WHERE NOT EXISTS (
        SELECT 1
        FROM app_user
        WHERE user_id = 0
           OR user_public_id = '00000000-0000-0000-0000-000000000000'
    );

    PERFORM trust_tier_seed_defaults();
    PERFORM media_domain_seed_defaults();

    IF NOT EXISTS (SELECT 1 FROM deployment_config) THEN
        INSERT INTO deployment_config DEFAULT VALUES;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM deployment_maintenance_state) THEN
        INSERT INTO deployment_maintenance_state DEFAULT VALUES;
    END IF;

    INSERT INTO rate_limit_policy (
        rate_limit_policy_public_id,
        display_name,
        requests_per_minute,
        burst,
        concurrent_requests,
        is_system
    )
    VALUES
        (gen_random_uuid(), 'default_indexer', 60, 30, 2, TRUE),
        (gen_random_uuid(), 'default_routing', 120, 60, 4, TRUE)
    ON CONFLICT (display_name) DO NOTHING;

    WITH seed_jobs (job_key, cadence_seconds, enabled) AS (
        VALUES
            ('retention_purge'::job_key, 3600, TRUE),
            ('reputation_rollup_1h'::job_key, 300, TRUE),
            ('reputation_rollup_24h'::job_key, 3600, TRUE),
            ('reputation_rollup_7d'::job_key, 21600, TRUE),
            ('connectivity_profile_refresh'::job_key, 300, TRUE),
            ('canonical_backfill_best_source'::job_key, 86400, TRUE),
            ('base_score_refresh_recent'::job_key, 3600, TRUE),
            ('canonical_prune_low_confidence'::job_key, 86400, TRUE),
            ('policy_snapshot_gc'::job_key, 86400, TRUE),
            ('policy_snapshot_refcount_repair'::job_key, 86400, TRUE),
            ('rate_limit_state_purge'::job_key, 3600, TRUE),
            ('rss_poll'::job_key, 60, TRUE),
            ('rss_subscription_backfill'::job_key, 300, TRUE)
    )
    INSERT INTO job_schedule (
        job_key,
        cadence_seconds,
        jitter_seconds,
        enabled,
        next_run_at
    )
    SELECT
        seed_jobs.job_key,
        seed_jobs.cadence_seconds,
        0,
        seed_jobs.enabled,
        now() + make_interval(
            secs => random_jitter_seconds(seed_jobs.cadence_seconds - 1)
        )
    FROM seed_jobs
    ON CONFLICT (job_key) DO NOTHING;
END;
$$;



CREATE FUNCTION public.derive_magnet_hash(infohash_v2_input text, infohash_v1_input text, magnet_uri_input text) RETURNS text
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT derive_magnet_hash_v1(infohash_v2_input => infohash_v2_input, infohash_v1_input => infohash_v1_input, magnet_uri_input => magnet_uri_input);
$$;



CREATE FUNCTION public.derive_magnet_hash_v1(infohash_v2_input text, infohash_v1_input text, magnet_uri_input text) RETURNS text
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    normalized_uri TEXT;
    hash_hex TEXT;
BEGIN
    IF infohash_v2_input IS NOT NULL THEN
        hash_hex := encode(digest(decode(infohash_v2_input, 'hex'), 'sha256'), 'hex');
        RETURN lower(hash_hex);
    END IF;

    IF infohash_v1_input IS NOT NULL THEN
        hash_hex := encode(digest(decode(infohash_v1_input, 'hex'), 'sha256'), 'hex');
        RETURN lower(hash_hex);
    END IF;

    IF magnet_uri_input IS NULL THEN
        RETURN NULL;
    END IF;

    normalized_uri := normalize_magnet_uri_v1(magnet_uri_input);
    IF normalized_uri IS NULL THEN
        RETURN NULL;
    END IF;

    hash_hex := encode(digest(normalized_uri, 'sha256'), 'hex');
    RETURN lower(hash_hex);
END;
$$;



CREATE FUNCTION public.import_job_create(actor_user_public_id uuid, source_input public.import_source, is_dry_run_input boolean, target_search_profile_public_id_input uuid, target_torznab_instance_public_id_input uuid) RETURNS uuid
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT import_job_create_v1(actor_user_public_id => actor_user_public_id, source_input => source_input, is_dry_run_input => is_dry_run_input, target_search_profile_public_id_input => target_search_profile_public_id_input, target_torznab_instance_public_id_input => target_torznab_instance_public_id_input);
$$;



CREATE FUNCTION public.import_job_create_v1(actor_user_public_id uuid, source_input public.import_source, is_dry_run_input boolean, target_search_profile_public_id_input uuid, target_torznab_instance_public_id_input uuid) RETURNS uuid
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to create import job';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    target_search_profile_id_value BIGINT;
    target_torznab_instance_id_value BIGINT;
    job_public_id UUID;
    dry_run_value BOOLEAN;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF source_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'source_missing';
    END IF;

    IF target_search_profile_public_id_input IS NOT NULL THEN
        SELECT search_profile_id
        INTO target_search_profile_id_value
        FROM search_profile
        WHERE search_profile_public_id = target_search_profile_public_id_input
          AND deleted_at IS NULL;

        IF target_search_profile_id_value IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'search_profile_not_found';
        END IF;
    END IF;

    IF target_torznab_instance_public_id_input IS NOT NULL THEN
        SELECT torznab_instance_id
        INTO target_torznab_instance_id_value
        FROM torznab_instance
        WHERE torznab_instance_public_id = target_torznab_instance_public_id_input
          AND deleted_at IS NULL;

        IF target_torznab_instance_id_value IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'torznab_instance_not_found';
        END IF;
    END IF;

    dry_run_value := COALESCE(is_dry_run_input, FALSE);
    job_public_id := gen_random_uuid();

    INSERT INTO import_job (
        import_job_public_id,
        target_search_profile_id,
        target_torznab_instance_id,
        created_by_user_id,
        source,
        is_dry_run,
        status
    )
    VALUES (
        job_public_id,
        target_search_profile_id_value,
        target_torznab_instance_id_value,
        actor_user_id,
        source_input,
        dry_run_value,
        'pending'
    );

    RETURN job_public_id;
END;
$$;



CREATE FUNCTION public.import_job_get_status(import_job_public_id_input uuid) RETURNS TABLE(status public.import_job_status, result_total integer, result_imported_ready integer, result_imported_needs_secret integer, result_imported_test_failed integer, result_unmapped_definition integer, result_skipped_duplicate integer)
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT * FROM import_job_get_status_v1(import_job_public_id_input => import_job_public_id_input);
$$;



CREATE FUNCTION public.import_job_get_status_v1(import_job_public_id_input uuid) RETURNS TABLE(status public.import_job_status, result_total integer, result_imported_ready integer, result_imported_needs_secret integer, result_imported_test_failed integer, result_unmapped_definition integer, result_skipped_duplicate integer)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to fetch import status';
    errcode CONSTANT text := 'P0001';
    job_id BIGINT;
BEGIN
    IF import_job_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_missing';
    END IF;

    SELECT import_job_id, import_job.status
    INTO job_id, status
    FROM import_job
    WHERE import_job_public_id = import_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_found';
    END IF;

    SELECT COUNT(*),
           COUNT(*) FILTER (WHERE import_indexer_result.status = 'imported_ready'),
           COUNT(*) FILTER (WHERE import_indexer_result.status = 'imported_needs_secret'),
           COUNT(*) FILTER (WHERE import_indexer_result.status = 'imported_test_failed'),
           COUNT(*) FILTER (WHERE import_indexer_result.status = 'unmapped_definition'),
           COUNT(*) FILTER (WHERE import_indexer_result.status = 'skipped_duplicate')
    INTO result_total,
         result_imported_ready,
         result_imported_needs_secret,
         result_imported_test_failed,
         result_unmapped_definition,
         result_skipped_duplicate
    FROM import_indexer_result
    WHERE import_job_id = job_id;

    RETURN NEXT;
END;
$$;



CREATE FUNCTION public.import_job_list_results(import_job_public_id_input uuid) RETURNS TABLE(prowlarr_identifier character varying, upstream_slug character varying, indexer_instance_public_id uuid, status public.import_indexer_result_status, detail character varying, resolved_is_enabled boolean, resolved_priority integer, missing_secret_fields integer, media_domain_keys character varying[], tag_keys character varying[], created_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT * FROM import_job_list_results_v1(import_job_public_id_input);
END;
$$;



CREATE FUNCTION public.import_job_list_results_v1(import_job_public_id_input uuid) RETURNS TABLE(prowlarr_identifier character varying, upstream_slug character varying, indexer_instance_public_id uuid, status public.import_indexer_result_status, detail character varying, resolved_is_enabled boolean, resolved_priority integer, missing_secret_fields integer, media_domain_keys character varying[], tag_keys character varying[], created_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to fetch import results';
    errcode CONSTANT text := 'P0001';
    job_id BIGINT;
BEGIN
    IF import_job_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_missing';
    END IF;

    SELECT import_job_id
    INTO job_id
    FROM import_job
    WHERE import_job_public_id = import_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_found';
    END IF;

    RETURN QUERY
    SELECT r.prowlarr_identifier,
           r.upstream_slug,
           i.indexer_instance_public_id,
           r.status,
           r.detail,
           r.resolved_is_enabled,
           r.resolved_priority,
           r.missing_secret_fields,
           ARRAY(
               SELECT md.media_domain_key::VARCHAR
               FROM import_indexer_result_media_domain ird
               JOIN media_domain md
                 ON md.media_domain_id = ird.media_domain_id
               WHERE ird.import_indexer_result_id = r.import_indexer_result_id
               ORDER BY md.media_domain_key
           ),
           ARRAY(
               SELECT t.tag_key
               FROM import_indexer_result_tag irt
               JOIN tag t
                 ON t.tag_id = irt.tag_id
               WHERE irt.import_indexer_result_id = r.import_indexer_result_id
               ORDER BY t.tag_key
           ),
           r.created_at
    FROM import_indexer_result r
    LEFT JOIN indexer_instance i
        ON i.indexer_instance_id = r.indexer_instance_id
    WHERE r.import_job_id = job_id
    ORDER BY r.created_at ASC, r.import_indexer_result_id ASC;
END;
$$;



CREATE FUNCTION public.import_job_run_prowlarr_api(import_job_public_id_input uuid, prowlarr_url_input character varying, prowlarr_api_key_secret_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM import_job_run_prowlarr_api_v1(import_job_public_id_input => import_job_public_id_input, prowlarr_url_input => prowlarr_url_input, prowlarr_api_key_secret_public_id_input => prowlarr_api_key_secret_public_id_input);
END;
$$;



CREATE FUNCTION public.import_job_run_prowlarr_api_v1(import_job_public_id_input uuid, prowlarr_url_input character varying, prowlarr_api_key_secret_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to start import job';
    errcode CONSTANT text := 'P0001';
    job_id BIGINT;
    job_source import_source;
    job_status import_job_status;
    secret_id_value BIGINT;
    trimmed_url VARCHAR(2048);
    config_detail VARCHAR(1024);
BEGIN
    IF import_job_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_missing';
    END IF;

    SELECT import_job_id, source, status
    INTO job_id, job_source, job_status
    FROM import_job
    WHERE import_job_public_id = import_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_found';
    END IF;

    IF job_source <> 'prowlarr_api' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_source_mismatch';
    END IF;

    IF job_status NOT IN ('pending', 'failed') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_startable';
    END IF;

    IF prowlarr_url_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'prowlarr_url_missing';
    END IF;

    trimmed_url := btrim(prowlarr_url_input);
    IF trimmed_url = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'prowlarr_url_missing';
    END IF;

    IF char_length(trimmed_url) > 2048 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'prowlarr_url_too_long';
    END IF;

    IF prowlarr_api_key_secret_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'secret_missing';
    END IF;

    SELECT secret_id
    INTO secret_id_value
    FROM secret
    WHERE secret_public_id = prowlarr_api_key_secret_public_id_input
      AND is_revoked = FALSE;

    IF secret_id_value IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'secret_not_found';
    END IF;

    config_detail := 'prowlarr_url=' || trimmed_url || ';secret_public_id='
        || prowlarr_api_key_secret_public_id_input::TEXT;

    IF char_length(config_detail) > 1024 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'config_too_long';
    END IF;

    UPDATE import_job
    SET status = 'running',
        started_at = now(),
        error_detail = config_detail
    WHERE import_job_id = job_id;
END;
$$;



CREATE FUNCTION public.import_job_run_prowlarr_backup(import_job_public_id_input uuid, backup_blob_ref_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM import_job_run_prowlarr_backup_v1(import_job_public_id_input => import_job_public_id_input, backup_blob_ref_input => backup_blob_ref_input);
END;
$$;



CREATE FUNCTION public.import_job_run_prowlarr_backup_v1(import_job_public_id_input uuid, backup_blob_ref_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to start import job';
    errcode CONSTANT text := 'P0001';
    job_id BIGINT;
    job_source import_source;
    job_status import_job_status;
    trimmed_ref VARCHAR(1024);
    config_detail VARCHAR(1024);
BEGIN
    IF import_job_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_missing';
    END IF;

    SELECT import_job_id, source, status
    INTO job_id, job_source, job_status
    FROM import_job
    WHERE import_job_public_id = import_job_public_id_input;

    IF job_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_found';
    END IF;

    IF job_source <> 'prowlarr_backup' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_source_mismatch';
    END IF;

    IF job_status NOT IN ('pending', 'failed') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_startable';
    END IF;

    IF backup_blob_ref_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'backup_blob_missing';
    END IF;

    trimmed_ref := btrim(backup_blob_ref_input);
    IF trimmed_ref = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'backup_blob_missing';
    END IF;

    IF char_length(trimmed_ref) > 1024 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'backup_blob_too_long';
    END IF;

    config_detail := 'backup_blob_ref=' || trimmed_ref;

    IF char_length(config_detail) > 1024 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'config_too_long';
    END IF;

    UPDATE import_job
    SET status = 'running',
        started_at = now(),
        error_detail = config_detail
    WHERE import_job_id = job_id;
END;
$$;



CREATE FUNCTION public.import_job_worker_claim_next() RETURNS TABLE(import_job_public_id uuid, source public.import_source, is_dry_run boolean, config_detail character varying)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT * FROM import_job_worker_claim_next_v1();
END;
$$;



CREATE FUNCTION public.import_job_worker_claim_next_v1() RETURNS TABLE(import_job_public_id uuid, source public.import_source, is_dry_run boolean, config_detail character varying)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT j.import_job_public_id,
           j.source,
           j.is_dry_run,
           j.error_detail
    FROM import_job j
    WHERE j.status = 'running'
      AND j.finished_at IS NULL
    ORDER BY COALESCE(j.started_at, j.created_at) ASC,
             j.import_job_id ASC
    LIMIT 1;
END;
$$;



CREATE FUNCTION public.import_job_worker_mark_terminal(import_job_public_id_input uuid, status_input public.import_job_status, error_detail_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM import_job_worker_mark_terminal_v1(
        import_job_public_id_input,
        status_input,
        error_detail_input
    );
END;
$$;



CREATE FUNCTION public.import_job_worker_mark_terminal_v1(import_job_public_id_input uuid, status_input public.import_job_status, error_detail_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to mark import job terminal';
    errcode CONSTANT text := 'P0001';
    job_id BIGINT;
BEGIN
    IF import_job_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_missing';
    END IF;

    IF status_input NOT IN ('completed', 'failed', 'canceled') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'terminal_status_invalid';
    END IF;

    SELECT import_job_id
    INTO job_id
    FROM import_job
    WHERE import_job_public_id = import_job_public_id_input
      AND status = 'running'
      AND finished_at IS NULL;

    IF job_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_running';
    END IF;

    UPDATE import_job
    SET status = status_input,
        finished_at = now(),
        error_detail = error_detail_input
    WHERE import_job_id = job_id;
END;
$$;



CREATE FUNCTION public.import_job_worker_record_result(import_job_public_id_input uuid, prowlarr_identifier_input character varying, status_input public.import_indexer_result_status, detail_input character varying, resolved_is_enabled_input boolean, resolved_priority_input integer, missing_secret_fields_input integer) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM import_job_worker_record_result_v1(
        import_job_public_id_input,
        prowlarr_identifier_input,
        status_input,
        detail_input,
        resolved_is_enabled_input,
        resolved_priority_input,
        missing_secret_fields_input
    );
END;
$$;



CREATE FUNCTION public.import_job_worker_record_result_v1(import_job_public_id_input uuid, prowlarr_identifier_input character varying, status_input public.import_indexer_result_status, detail_input character varying, resolved_is_enabled_input boolean, resolved_priority_input integer, missing_secret_fields_input integer) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to record import job result';
    errcode CONSTANT text := 'P0001';
    job_id BIGINT;
BEGIN
    IF import_job_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_missing';
    END IF;

    IF prowlarr_identifier_input IS NULL OR btrim(prowlarr_identifier_input) = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'prowlarr_identifier_missing';
    END IF;

    SELECT import_job_id
    INTO job_id
    FROM import_job
    WHERE import_job_public_id = import_job_public_id_input
      AND status = 'running'
      AND finished_at IS NULL;

    IF job_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'import_job_not_running';
    END IF;

    INSERT INTO import_indexer_result (
        import_job_id,
        prowlarr_identifier,
        status,
        detail,
        resolved_is_enabled,
        resolved_priority,
        missing_secret_fields
    )
    VALUES (
        job_id,
        btrim(prowlarr_identifier_input),
        status_input,
        detail_input,
        resolved_is_enabled_input,
        resolved_priority_input,
        COALESCE(missing_secret_fields_input, 0)
    )
    ON CONFLICT (import_job_id, prowlarr_identifier)
    DO NOTHING;
END;
$$;



CREATE FUNCTION public.indexer_backup_assert_actor_v1(actor_user_public_id uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to export indexer backup';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    actor_verified BOOLEAN;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role, is_email_verified
    INTO actor_user_id, actor_role, actor_verified
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_verified IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unverified';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;
END;
$$;



CREATE FUNCTION public.indexer_backup_export_indexer_instance_list(actor_user_public_id uuid) RETURNS TABLE(indexer_instance_public_id uuid, upstream_slug character varying, display_name character varying, is_enabled boolean, enable_rss boolean, enable_automatic_search boolean, enable_interactive_search boolean, priority integer, trust_tier_key public.trust_tier_key, routing_policy_public_id uuid, routing_policy_display_name character varying, connect_timeout_ms integer, read_timeout_ms integer, max_parallel_requests integer, rate_limit_policy_public_id uuid, rate_limit_display_name character varying, rss_subscription_enabled boolean, rss_interval_seconds integer, media_domain_key public.media_domain_key, tag_key character varying, field_name character varying, field_type public.field_type, value_plain character varying, value_int integer, value_decimal character varying, value_bool boolean, secret_public_id uuid, secret_type public.secret_type)
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT *
    FROM indexer_backup_export_indexer_instance_list_v1(actor_user_public_id);
$$;



CREATE FUNCTION public.indexer_backup_export_indexer_instance_list_v1(actor_user_public_id uuid) RETURNS TABLE(indexer_instance_public_id uuid, upstream_slug character varying, display_name character varying, is_enabled boolean, enable_rss boolean, enable_automatic_search boolean, enable_interactive_search boolean, priority integer, trust_tier_key public.trust_tier_key, routing_policy_public_id uuid, routing_policy_display_name character varying, connect_timeout_ms integer, read_timeout_ms integer, max_parallel_requests integer, rate_limit_policy_public_id uuid, rate_limit_display_name character varying, rss_subscription_enabled boolean, rss_interval_seconds integer, media_domain_key public.media_domain_key, tag_key character varying, field_name character varying, field_type public.field_type, value_plain character varying, value_int integer, value_decimal character varying, value_bool boolean, secret_public_id uuid, secret_type public.secret_type)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_backup_assert_actor_v1(actor_user_public_id);

    RETURN QUERY
    SELECT
        instance.indexer_instance_public_id,
        definition.upstream_slug::VARCHAR,
        instance.display_name::VARCHAR,
        instance.is_enabled,
        instance.enable_rss,
        instance.enable_automatic_search,
        instance.enable_interactive_search,
        instance.priority,
        instance.trust_tier_key,
        routing.routing_policy_public_id,
        routing.display_name::VARCHAR,
        instance.connect_timeout_ms,
        instance.read_timeout_ms,
        instance.max_parallel_requests,
        rate_limit.rate_limit_policy_public_id,
        rate_limit.display_name::VARCHAR,
        rss.is_enabled,
        rss.interval_seconds,
        media_domain.media_domain_key,
        tag.tag_key::VARCHAR,
        field_value.field_name::VARCHAR,
        field_value.field_type,
        field_value.value_plain::VARCHAR,
        field_value.value_int,
        field_value.value_decimal::VARCHAR,
        field_value.value_bool,
        secret.secret_public_id,
        secret.secret_type
    FROM indexer_instance instance
    JOIN indexer_definition definition
        ON definition.indexer_definition_id = instance.indexer_definition_id
    LEFT JOIN routing_policy routing
        ON routing.routing_policy_id = instance.routing_policy_id
       AND routing.deleted_at IS NULL
    LEFT JOIN indexer_instance_rate_limit instance_rate_limit
        ON instance_rate_limit.indexer_instance_id = instance.indexer_instance_id
    LEFT JOIN rate_limit_policy rate_limit
        ON rate_limit.rate_limit_policy_id = instance_rate_limit.rate_limit_policy_id
       AND rate_limit.deleted_at IS NULL
    LEFT JOIN indexer_rss_subscription rss
        ON rss.indexer_instance_id = instance.indexer_instance_id
    LEFT JOIN indexer_instance_media_domain instance_media_domain
        ON instance_media_domain.indexer_instance_id = instance.indexer_instance_id
    LEFT JOIN media_domain
        ON media_domain.media_domain_id = instance_media_domain.media_domain_id
    LEFT JOIN indexer_instance_tag instance_tag
        ON instance_tag.indexer_instance_id = instance.indexer_instance_id
    LEFT JOIN tag
        ON tag.tag_id = instance_tag.tag_id
       AND tag.deleted_at IS NULL
    LEFT JOIN indexer_instance_field_value field_value
        ON field_value.indexer_instance_id = instance.indexer_instance_id
    LEFT JOIN secret_binding binding
        ON binding.bound_table = 'indexer_instance_field_value'
       AND binding.bound_id = field_value.indexer_instance_field_value_id
    LEFT JOIN secret
        ON secret.secret_id = binding.secret_id
       AND secret.is_revoked = FALSE
    WHERE instance.deleted_at IS NULL
    ORDER BY instance.display_name, field_value.field_name NULLS FIRST;
END;
$$;



CREATE FUNCTION public.indexer_backup_export_rate_limit_policy_list(actor_user_public_id uuid) RETURNS TABLE(rate_limit_policy_public_id uuid, display_name character varying, requests_per_minute integer, burst integer, concurrent_requests integer, is_system boolean)
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT *
    FROM indexer_backup_export_rate_limit_policy_list_v1(actor_user_public_id);
$$;



CREATE FUNCTION public.indexer_backup_export_rate_limit_policy_list_v1(actor_user_public_id uuid) RETURNS TABLE(rate_limit_policy_public_id uuid, display_name character varying, requests_per_minute integer, burst integer, concurrent_requests integer, is_system boolean)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_backup_assert_actor_v1(actor_user_public_id);

    RETURN QUERY
    SELECT
        policy.rate_limit_policy_public_id,
        policy.display_name::VARCHAR,
        policy.requests_per_minute,
        policy.burst,
        policy.concurrent_requests,
        policy.is_system
    FROM rate_limit_policy policy
    WHERE policy.deleted_at IS NULL
    ORDER BY policy.display_name;
END;
$$;



CREATE FUNCTION public.indexer_backup_export_routing_policy_list(actor_user_public_id uuid) RETURNS TABLE(routing_policy_public_id uuid, display_name character varying, mode public.routing_policy_mode, rate_limit_policy_public_id uuid, rate_limit_display_name character varying, param_key public.routing_param_key, value_plain character varying, value_int integer, value_bool boolean, secret_public_id uuid, secret_type public.secret_type)
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT *
    FROM indexer_backup_export_routing_policy_list_v1(actor_user_public_id);
$$;



CREATE FUNCTION public.indexer_backup_export_routing_policy_list_v1(actor_user_public_id uuid) RETURNS TABLE(routing_policy_public_id uuid, display_name character varying, mode public.routing_policy_mode, rate_limit_policy_public_id uuid, rate_limit_display_name character varying, param_key public.routing_param_key, value_plain character varying, value_int integer, value_bool boolean, secret_public_id uuid, secret_type public.secret_type)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_backup_assert_actor_v1(actor_user_public_id);

    RETURN QUERY
    SELECT
        policy.routing_policy_public_id,
        policy.display_name::VARCHAR,
        policy.mode,
        rate_limit.rate_limit_policy_public_id,
        rate_limit.display_name::VARCHAR,
        param.param_key,
        param.value_plain::VARCHAR,
        param.value_int,
        param.value_bool,
        secret.secret_public_id,
        secret.secret_type
    FROM routing_policy policy
    LEFT JOIN routing_policy_rate_limit policy_rate_limit
        ON policy_rate_limit.routing_policy_id = policy.routing_policy_id
    LEFT JOIN rate_limit_policy rate_limit
        ON rate_limit.rate_limit_policy_id = policy_rate_limit.rate_limit_policy_id
       AND rate_limit.deleted_at IS NULL
    LEFT JOIN routing_policy_parameter param
        ON param.routing_policy_id = policy.routing_policy_id
    LEFT JOIN secret_binding binding
        ON binding.bound_table = 'routing_policy_parameter'
       AND binding.bound_id = param.routing_policy_parameter_id
    LEFT JOIN secret
        ON secret.secret_id = binding.secret_id
       AND secret.is_revoked = FALSE
    WHERE policy.deleted_at IS NULL
    ORDER BY policy.display_name, param.param_key NULLS FIRST;
END;
$$;



CREATE FUNCTION public.indexer_backup_export_tag_list(actor_user_public_id uuid) RETURNS TABLE(tag_public_id uuid, tag_key character varying, display_name character varying)
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT *
    FROM indexer_backup_export_tag_list_v1(actor_user_public_id);
$$;



CREATE FUNCTION public.indexer_backup_export_tag_list_v1(actor_user_public_id uuid) RETURNS TABLE(tag_public_id uuid, tag_key character varying, display_name character varying)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_backup_assert_actor_v1(actor_user_public_id);

    RETURN QUERY
    SELECT
        tag.tag_public_id,
        tag.tag_key::VARCHAR,
        tag.display_name::VARCHAR
    FROM tag
    WHERE tag.deleted_at IS NULL
    ORDER BY tag.tag_key;
END;
$$;



CREATE FUNCTION public.indexer_cf_state_get(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS TABLE(state public.cf_state, last_changed_at timestamp with time zone, cf_session_expires_at timestamp with time zone, cooldown_until timestamp with time zone, backoff_seconds integer, consecutive_failures integer, last_error_class public.error_class)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT *
    FROM indexer_cf_state_get_v1(actor_user_public_id, indexer_instance_public_id_input);
$$;



CREATE FUNCTION public.indexer_cf_state_get_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS TABLE(state public.cf_state, last_changed_at timestamp with time zone, cf_session_expires_at timestamp with time zone, cooldown_until timestamp with time zone, backoff_seconds integer, consecutive_failures integer, last_error_class public.error_class)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    errcode CONSTANT text := 'P0001';
    v_indexer_instance_id BIGINT;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION 'actor_missing' USING ERRCODE = errcode, DETAIL = 'actor_missing';
    END IF;

    SELECT indexer_instance_id
    INTO v_indexer_instance_id
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input
      AND deleted_at IS NULL;

    IF v_indexer_instance_id IS NULL THEN
        RAISE EXCEPTION 'indexer_not_found' USING ERRCODE = errcode, DETAIL = 'indexer_not_found';
    END IF;

    RETURN QUERY
    SELECT
        cf.state,
        cf.last_changed_at,
        cf.cf_session_expires_at,
        cf.cooldown_until,
        cf.backoff_seconds,
        cf.consecutive_failures,
        cf.last_error_class
    FROM indexer_cf_state AS cf
    WHERE cf.indexer_instance_id = v_indexer_instance_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'indexer_not_found' USING ERRCODE = errcode, DETAIL = 'indexer_not_found';
    END IF;
END;
$$;



CREATE FUNCTION public.indexer_cf_state_reset(actor_user_public_id uuid, indexer_instance_public_id_input uuid, reason_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_cf_state_reset_v1(
        actor_user_public_id,
        indexer_instance_public_id_input,
        reason_input
    );
END;
$$;



CREATE FUNCTION public.indexer_cf_state_reset_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, reason_input character varying) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to reset Cloudflare state';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    indexer_id BIGINT;
    indexer_deleted_at TIMESTAMPTZ;
    connectivity_status_value connectivity_status;
    connectivity_error_class error_class;
    trimmed_reason VARCHAR(1024);
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    IF reason_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'reason_missing';
    END IF;

    trimmed_reason := trim(reason_input);

    IF trimmed_reason = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'reason_empty';
    END IF;

    IF char_length(trimmed_reason) > 1024 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'reason_too_long';
    END IF;

    SELECT indexer_instance_id, deleted_at
    INTO indexer_id, indexer_deleted_at
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF indexer_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF indexer_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    INSERT INTO indexer_cf_state (
        indexer_instance_id,
        state,
        last_changed_at,
        cf_session_id,
        cf_session_expires_at,
        cooldown_until,
        backoff_seconds,
        consecutive_failures,
        last_error_class
    )
    VALUES (
        indexer_id,
        'clear',
        now(),
        NULL,
        NULL,
        NULL,
        NULL,
        0,
        NULL
    )
    ON CONFLICT (indexer_instance_id)
    DO UPDATE SET
        state = 'clear',
        last_changed_at = now(),
        cf_session_id = NULL,
        cf_session_expires_at = NULL,
        cooldown_until = NULL,
        backoff_seconds = NULL,
        consecutive_failures = 0,
        last_error_class = NULL;

    SELECT status, error_class
    INTO connectivity_status_value, connectivity_error_class
    FROM indexer_connectivity_profile
    WHERE indexer_instance_id = indexer_id;

    IF connectivity_status_value = 'quarantined'
        AND connectivity_error_class IN ('cf_challenge', 'http_429') THEN
        UPDATE indexer_connectivity_profile
        SET status = 'degraded',
            error_class = 'unknown',
            last_checked_at = now()
        WHERE indexer_instance_id = indexer_id;
    END IF;

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary,
        change_detail
    )
    VALUES (
        'indexer_instance',
        indexer_id,
        indexer_instance_public_id_input,
        'update',
        actor_user_id,
        'cf_state_reset',
        trimmed_reason
    );
END;
$$;



CREATE FUNCTION public.indexer_connectivity_profile_get(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS TABLE(profile_exists boolean, status public.connectivity_status, error_class public.error_class, latency_p50_ms integer, latency_p95_ms integer, success_rate_1h numeric, success_rate_24h numeric, last_checked_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_connectivity_profile_get_v1(
        actor_user_public_id,
        indexer_instance_public_id_input
    );
END;
$$;



CREATE FUNCTION public.indexer_connectivity_profile_get_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS TABLE(profile_exists boolean, status public.connectivity_status, error_class public.error_class, latency_p50_ms integer, latency_p95_ms integer, success_rate_1h numeric, success_rate_24h numeric, last_checked_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to fetch connectivity profile';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT inst.indexer_instance_id, inst.deleted_at
    INTO instance_id, instance_deleted_at
    FROM indexer_instance inst
    WHERE inst.indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    RETURN QUERY
    SELECT
        profile.indexer_instance_id IS NOT NULL,
        profile.status,
        profile.error_class,
        profile.latency_p50_ms,
        profile.latency_p95_ms,
        profile.success_rate_1h,
        profile.success_rate_24h,
        profile.last_checked_at
    FROM indexer_instance inst
    LEFT JOIN indexer_connectivity_profile profile
        ON profile.indexer_instance_id = inst.indexer_instance_id
    WHERE inst.indexer_instance_id = instance_id;
END;
$$;



CREATE FUNCTION public.indexer_definition_import_cardigann_begin(actor_user_public_id_input uuid, upstream_slug_input character varying, display_name_input character varying, canonical_definition_text_input text, is_deprecated_input boolean DEFAULT false) RETURNS TABLE(upstream_source character varying, upstream_slug character varying, display_name character varying, protocol character varying, engine character varying, schema_version integer, definition_hash character, is_deprecated boolean, created_at timestamp with time zone, updated_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_definition_import_cardigann_begin_v1(
        actor_user_public_id_input,
        upstream_slug_input,
        display_name_input,
        canonical_definition_text_input,
        is_deprecated_input
    );
END;
$$;



CREATE FUNCTION public.indexer_definition_import_cardigann_begin_v1(actor_user_public_id_input uuid, upstream_slug_input character varying, display_name_input character varying, canonical_definition_text_input text, is_deprecated_input boolean DEFAULT false) RETURNS TABLE(upstream_source character varying, upstream_slug character varying, display_name character varying, protocol character varying, engine character varying, schema_version integer, definition_hash character, is_deprecated boolean, created_at timestamp with time zone, updated_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to import Cardigann definition';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    definition_id BIGINT;
    normalized_slug VARCHAR(128);
    normalized_display_name VARCHAR(256);
BEGIN
    IF actor_user_public_id_input IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_missing';
    END IF;

    SELECT user_id
    INTO actor_user_id
    FROM app_user
    WHERE user_public_id = actor_user_public_id_input;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_not_found';
    END IF;

    normalized_slug := lower(btrim(coalesce(upstream_slug_input, '')));
    IF normalized_slug = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_upstream_slug_missing';
    END IF;

    normalized_display_name := btrim(coalesce(display_name_input, ''));
    IF normalized_display_name = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_display_name_missing';
    END IF;

    IF canonical_definition_text_input IS NULL
        OR btrim(canonical_definition_text_input) = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_canonical_text_missing';
    END IF;

    INSERT INTO indexer_definition (
        upstream_source,
        upstream_slug,
        display_name,
        protocol,
        engine,
        schema_version,
        definition_hash,
        is_deprecated
    )
    VALUES (
        'cardigann',
        normalized_slug,
        normalized_display_name,
        'torrent',
        'cardigann',
        1,
        lower(encode(digest(canonical_definition_text_input, 'sha256'), 'hex')),
        coalesce(is_deprecated_input, false)
    )
    ON CONFLICT ON CONSTRAINT indexer_definition_upstream_uq
    DO UPDATE
    SET display_name = EXCLUDED.display_name,
        protocol = EXCLUDED.protocol,
        engine = EXCLUDED.engine,
        schema_version = EXCLUDED.schema_version,
        definition_hash = EXCLUDED.definition_hash,
        is_deprecated = EXCLUDED.is_deprecated,
        updated_at = now()
    RETURNING indexer_definition.indexer_definition_id
    INTO definition_id;

    DELETE FROM indexer_definition_field_option
    WHERE indexer_definition_field_id IN (
        SELECT indexer_definition_field_id
        FROM indexer_definition_field
        WHERE indexer_definition_id = definition_id
    );

    DELETE FROM indexer_definition_field_validation
    WHERE indexer_definition_field_id IN (
        SELECT indexer_definition_field_id
        FROM indexer_definition_field
        WHERE indexer_definition_id = definition_id
    );

    DELETE FROM indexer_definition_field
    WHERE indexer_definition_id = definition_id;

    RETURN QUERY
    SELECT
        definition.upstream_source::VARCHAR,
        definition.upstream_slug,
        definition.display_name,
        definition.protocol::VARCHAR,
        definition.engine::VARCHAR,
        definition.schema_version,
        definition.definition_hash,
        definition.is_deprecated,
        definition.created_at,
        definition.updated_at
    FROM indexer_definition definition
    WHERE definition.indexer_definition_id = definition_id;
END;
$$;



CREATE FUNCTION public.indexer_definition_import_cardigann_complete(actor_user_public_id_input uuid, upstream_slug_input character varying) RETURNS TABLE(upstream_source character varying, upstream_slug character varying, display_name character varying, protocol character varying, engine character varying, schema_version integer, definition_hash character, is_deprecated boolean, created_at timestamp with time zone, updated_at timestamp with time zone, field_count integer, option_count integer)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_definition_import_cardigann_complete_v1(
        actor_user_public_id_input,
        upstream_slug_input
    );
END;
$$;



CREATE FUNCTION public.indexer_definition_import_cardigann_complete_v1(actor_user_public_id_input uuid, upstream_slug_input character varying) RETURNS TABLE(upstream_source character varying, upstream_slug character varying, display_name character varying, protocol character varying, engine character varying, schema_version integer, definition_hash character, is_deprecated boolean, created_at timestamp with time zone, updated_at timestamp with time zone, field_count integer, option_count integer)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to finalize Cardigann definition import';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    definition_id BIGINT;
    normalized_slug VARCHAR(128);
BEGIN
    IF actor_user_public_id_input IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_missing';
    END IF;

    SELECT user_id
    INTO actor_user_id
    FROM app_user
    WHERE user_public_id = actor_user_public_id_input;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_not_found';
    END IF;

    normalized_slug := lower(btrim(coalesce(upstream_slug_input, '')));
    IF normalized_slug = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_upstream_slug_missing';
    END IF;

    SELECT indexer_definition_id
    INTO definition_id
    FROM indexer_definition definition
    WHERE definition.upstream_source = 'cardigann'
      AND definition.upstream_slug = normalized_slug;

    IF definition_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'cardigann_definition_not_found';
    END IF;

    RETURN QUERY
    SELECT
        definition.upstream_source::VARCHAR,
        definition.upstream_slug,
        definition.display_name,
        definition.protocol::VARCHAR,
        definition.engine::VARCHAR,
        definition.schema_version,
        definition.definition_hash,
        definition.is_deprecated,
        definition.created_at,
        definition.updated_at,
        (
            SELECT count(*)::INTEGER
            FROM indexer_definition_field
            WHERE indexer_definition_id = definition.indexer_definition_id
        ),
        (
            SELECT count(*)::INTEGER
            FROM indexer_definition_field_option option_row
            JOIN indexer_definition_field field_row
              ON field_row.indexer_definition_field_id = option_row.indexer_definition_field_id
            WHERE field_row.indexer_definition_id = definition.indexer_definition_id
        )
    FROM indexer_definition definition
    WHERE definition.indexer_definition_id = definition_id;
END;
$$;



CREATE FUNCTION public.indexer_definition_import_cardigann_field(actor_user_public_id_input uuid, upstream_slug_input character varying, field_name_input character varying, label_input character varying, field_type_input public.field_type, is_required_input boolean, is_advanced_input boolean, display_order_input integer, default_value_plain_input character varying DEFAULT NULL::character varying, default_value_int_input integer DEFAULT NULL::integer, default_value_decimal_input character varying DEFAULT NULL::character varying, default_value_bool_input boolean DEFAULT NULL::boolean, option_values_input character varying[] DEFAULT NULL::character varying[], option_labels_input character varying[] DEFAULT NULL::character varying[]) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_definition_import_cardigann_field_v1(
        actor_user_public_id_input,
        upstream_slug_input,
        field_name_input,
        label_input,
        field_type_input,
        is_required_input,
        is_advanced_input,
        display_order_input,
        default_value_plain_input,
        default_value_int_input,
        default_value_decimal_input,
        default_value_bool_input,
        option_values_input,
        option_labels_input
    );
END;
$$;



CREATE FUNCTION public.indexer_definition_import_cardigann_field_v1(actor_user_public_id_input uuid, upstream_slug_input character varying, field_name_input character varying, label_input character varying, field_type_input public.field_type, is_required_input boolean, is_advanced_input boolean, display_order_input integer, default_value_plain_input character varying DEFAULT NULL::character varying, default_value_int_input integer DEFAULT NULL::integer, default_value_decimal_input character varying DEFAULT NULL::character varying, default_value_bool_input boolean DEFAULT NULL::boolean, option_values_input character varying[] DEFAULT NULL::character varying[], option_labels_input character varying[] DEFAULT NULL::character varying[]) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to import Cardigann definition field';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    definition_id BIGINT;
    field_id BIGINT;
    normalized_slug VARCHAR(128);
    normalized_field_name VARCHAR(128);
    normalized_label VARCHAR(256);
BEGIN
    IF actor_user_public_id_input IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_missing';
    END IF;

    SELECT user_id
    INTO actor_user_id
    FROM app_user
    WHERE user_public_id = actor_user_public_id_input;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_not_found';
    END IF;

    normalized_slug := lower(btrim(coalesce(upstream_slug_input, '')));
    IF normalized_slug = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_upstream_slug_missing';
    END IF;

    SELECT indexer_definition_id
    INTO definition_id
    FROM indexer_definition definition
    WHERE definition.upstream_source = 'cardigann'
      AND definition.upstream_slug = normalized_slug;

    IF definition_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'cardigann_definition_not_found';
    END IF;

    normalized_field_name := lower(btrim(coalesce(field_name_input, '')));
    IF normalized_field_name = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_field_name_missing';
    END IF;

    normalized_label := btrim(coalesce(label_input, ''));
    IF normalized_label = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_field_label_missing';
    END IF;

    IF coalesce(array_length(option_values_input, 1), 0)
        <> coalesce(array_length(option_labels_input, 1), 0) THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_option_length_mismatch';
    END IF;

    INSERT INTO indexer_definition_field (
        indexer_definition_id,
        name,
        label,
        field_type,
        is_required,
        is_advanced,
        display_order,
        default_value_plain,
        default_value_int,
        default_value_decimal,
        default_value_bool
    )
    VALUES (
        definition_id,
        normalized_field_name,
        normalized_label,
        field_type_input,
        coalesce(is_required_input, false),
        coalesce(is_advanced_input, false),
        coalesce(display_order_input, 1000),
        default_value_plain_input,
        default_value_int_input,
        CASE
            WHEN default_value_decimal_input IS NULL
                OR btrim(default_value_decimal_input) = '' THEN NULL
            ELSE btrim(default_value_decimal_input)::NUMERIC(12, 4)
        END,
        default_value_bool_input
    )
    RETURNING indexer_definition_field_id
    INTO field_id;

    IF coalesce(array_length(option_values_input, 1), 0) > 0 THEN
        INSERT INTO indexer_definition_field_option (
            indexer_definition_field_id,
            option_value,
            option_label,
            sort_order
        )
        SELECT
            field_id,
            option_values_input[item.ordinality],
            option_labels_input[item.ordinality],
            item.ordinality
        FROM unnest(option_values_input) WITH ORDINALITY AS item(option_value, ordinality);
    END IF;
END;
$$;



CREATE FUNCTION public.indexer_definition_list(actor_user_public_id uuid) RETURNS TABLE(upstream_source character varying, upstream_slug character varying, display_name character varying, protocol character varying, engine character varying, schema_version integer, definition_hash character, is_deprecated boolean, created_at timestamp with time zone, updated_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT * FROM indexer_definition_list_v1(actor_user_public_id);
END;
$$;



CREATE FUNCTION public.indexer_definition_list_v1(actor_user_public_id uuid) RETURNS TABLE(upstream_source character varying, upstream_slug character varying, display_name character varying, protocol character varying, engine character varying, schema_version integer, definition_hash character, is_deprecated boolean, created_at timestamp with time zone, updated_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to list indexer definitions';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id
    INTO actor_user_id
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    RETURN QUERY
    SELECT indexer_definition.upstream_source::varchar,
           indexer_definition.upstream_slug,
           indexer_definition.display_name,
           indexer_definition.protocol::varchar,
           indexer_definition.engine::varchar,
           indexer_definition.schema_version,
           indexer_definition.definition_hash,
           indexer_definition.is_deprecated,
           indexer_definition.created_at,
           indexer_definition.updated_at
    FROM indexer_definition
    ORDER BY indexer_definition.display_name ASC,
             indexer_definition.indexer_definition_id ASC;
END;
$$;



CREATE FUNCTION public.indexer_health_event_list(actor_user_public_id uuid, indexer_instance_public_id_input uuid, limit_input integer) RETURNS TABLE(occurred_at timestamp with time zone, event_type public.health_event_type, latency_ms integer, http_status integer, error_class public.error_class, detail character varying)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_health_event_list_v1(
        actor_user_public_id,
        indexer_instance_public_id_input,
        limit_input
    );
END;
$$;



CREATE FUNCTION public.indexer_health_event_list_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, limit_input integer) RETURNS TABLE(occurred_at timestamp with time zone, event_type public.health_event_type, latency_ms integer, http_status integer, error_class public.error_class, detail character varying)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to list health events';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    event_limit INTEGER;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT inst.indexer_instance_id, inst.deleted_at
    INTO instance_id, instance_deleted_at
    FROM indexer_instance inst
    WHERE inst.indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    event_limit := COALESCE(limit_input, 20);
    IF event_limit < 1 OR event_limit > 100 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'limit_out_of_range';
    END IF;

    RETURN QUERY
    SELECT
        health.occurred_at,
        health.event_type,
        health.latency_ms,
        health.http_status,
        health.error_class,
        health.detail
    FROM indexer_health_event health
    WHERE health.indexer_instance_id = instance_id
    ORDER BY health.occurred_at DESC, health.indexer_health_event_id DESC
    LIMIT event_limit;
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_create(actor_user_public_id uuid, channel_input public.indexer_health_notification_channel, display_name_input character varying, status_threshold_input public.indexer_health_notification_threshold, webhook_url_input character varying, email_input character varying) RETURNS uuid
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN indexer_health_notification_hook_create_v1(
        actor_user_public_id,
        channel_input,
        display_name_input,
        status_threshold_input,
        webhook_url_input,
        email_input
    );
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_create_v1(actor_user_public_id uuid, channel_input public.indexer_health_notification_channel, display_name_input character varying, status_threshold_input public.indexer_health_notification_threshold, webhook_url_input character varying, email_input character varying) RETURNS uuid
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to create health notification hook';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    normalized_display_name VARCHAR(120);
    normalized_email VARCHAR(320);
    normalized_webhook_url VARCHAR(2048);
    hook_public_id UUID;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_unauthorized';
    END IF;

    normalized_display_name := NULLIF(trim(display_name_input), '');
    IF normalized_display_name IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'display_name_missing';
    END IF;

    IF status_threshold_input IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'status_threshold_missing';
    END IF;

    CASE channel_input
        WHEN 'webhook' THEN
            normalized_webhook_url := NULLIF(trim(webhook_url_input), '');
            IF normalized_webhook_url IS NULL THEN
                RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'webhook_url_missing';
            END IF;
            IF normalized_webhook_url !~ '^https?://.+' THEN
                RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'webhook_url_invalid';
            END IF;
        WHEN 'email' THEN
            normalized_email := NULLIF(trim(email_input), '');
            IF normalized_email IS NULL THEN
                RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'email_missing';
            END IF;
            normalized_email := lower(normalized_email);
            IF position('@' IN normalized_email) < 2 THEN
                RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'email_invalid';
            END IF;
        ELSE
            RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'channel_invalid';
    END CASE;

    INSERT INTO indexer_health_notification_hook (
        channel,
        display_name,
        status_threshold,
        webhook_url,
        email,
        email_normalized,
        created_by_user_id,
        updated_by_user_id
    )
    VALUES (
        channel_input,
        normalized_display_name,
        status_threshold_input,
        normalized_webhook_url,
        normalized_email,
        normalized_email,
        actor_user_id,
        actor_user_id
    )
    RETURNING indexer_health_notification_hook_public_id
    INTO hook_public_id;

    RETURN hook_public_id;
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_delete(actor_user_public_id uuid, indexer_health_notification_hook_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_health_notification_hook_delete_v1(
        actor_user_public_id,
        indexer_health_notification_hook_public_id_input
    );
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_delete_v1(actor_user_public_id uuid, indexer_health_notification_hook_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to delete health notification hook';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_health_notification_hook_public_id_input IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'hook_missing';
    END IF;

    DELETE FROM indexer_health_notification_hook
    WHERE indexer_health_notification_hook_public_id = indexer_health_notification_hook_public_id_input;

    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'hook_not_found';
    END IF;
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_get(actor_user_public_id uuid, indexer_health_notification_hook_public_id_input uuid) RETURNS TABLE(indexer_health_notification_hook_public_id uuid, channel public.indexer_health_notification_channel, display_name character varying, status_threshold public.indexer_health_notification_threshold, webhook_url character varying, email character varying, is_enabled boolean, updated_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_health_notification_hook_get_v1(
        actor_user_public_id,
        indexer_health_notification_hook_public_id_input
    );
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_get_v1(actor_user_public_id uuid, indexer_health_notification_hook_public_id_input uuid) RETURNS TABLE(indexer_health_notification_hook_public_id uuid, channel public.indexer_health_notification_channel, display_name character varying, status_threshold public.indexer_health_notification_threshold, webhook_url character varying, email character varying, is_enabled boolean, updated_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to get health notification hook';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_health_notification_hook_public_id_input IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'hook_missing';
    END IF;

    RETURN QUERY
    SELECT
        hook.indexer_health_notification_hook_public_id,
        hook.channel,
        hook.display_name,
        hook.status_threshold,
        hook.webhook_url,
        hook.email,
        hook.is_enabled,
        hook.updated_at
    FROM indexer_health_notification_hook hook
    WHERE hook.indexer_health_notification_hook_public_id =
        indexer_health_notification_hook_public_id_input;

    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'hook_not_found';
    END IF;
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_list(actor_user_public_id uuid) RETURNS TABLE(indexer_health_notification_hook_public_id uuid, channel public.indexer_health_notification_channel, display_name character varying, status_threshold public.indexer_health_notification_threshold, webhook_url character varying, email character varying, is_enabled boolean, updated_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_health_notification_hook_list_v1(actor_user_public_id);
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_list_v1(actor_user_public_id uuid) RETURNS TABLE(indexer_health_notification_hook_public_id uuid, channel public.indexer_health_notification_channel, display_name character varying, status_threshold public.indexer_health_notification_threshold, webhook_url character varying, email character varying, is_enabled boolean, updated_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to list health notification hooks';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_unauthorized';
    END IF;

    RETURN QUERY
    SELECT
        hook.indexer_health_notification_hook_public_id,
        hook.channel,
        hook.display_name,
        hook.status_threshold,
        hook.webhook_url,
        hook.email,
        hook.is_enabled,
        hook.updated_at
    FROM indexer_health_notification_hook hook
    ORDER BY hook.display_name ASC, hook.indexer_health_notification_hook_id ASC;
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_update(actor_user_public_id uuid, indexer_health_notification_hook_public_id_input uuid, display_name_input character varying, status_threshold_input public.indexer_health_notification_threshold, webhook_url_input character varying, email_input character varying, is_enabled_input boolean) RETURNS uuid
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN indexer_health_notification_hook_update_v1(
        actor_user_public_id,
        indexer_health_notification_hook_public_id_input,
        display_name_input,
        status_threshold_input,
        webhook_url_input,
        email_input,
        is_enabled_input
    );
END;
$$;



CREATE FUNCTION public.indexer_health_notification_hook_update_v1(actor_user_public_id uuid, indexer_health_notification_hook_public_id_input uuid, display_name_input character varying, status_threshold_input public.indexer_health_notification_threshold, webhook_url_input character varying, email_input character varying, is_enabled_input boolean) RETURNS uuid
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to update health notification hook';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    hook_id BIGINT;
    current_channel indexer_health_notification_channel;
    next_display_name VARCHAR(120);
    next_status_threshold indexer_health_notification_threshold;
    next_webhook_url VARCHAR(2048);
    next_email VARCHAR(320);
    next_email_normalized VARCHAR(320);
    next_is_enabled BOOLEAN;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_health_notification_hook_public_id_input IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'hook_missing';
    END IF;

    SELECT
        indexer_health_notification_hook_id,
        channel,
        display_name,
        status_threshold,
        webhook_url,
        email,
        email_normalized,
        is_enabled
    INTO
        hook_id,
        current_channel,
        next_display_name,
        next_status_threshold,
        next_webhook_url,
        next_email,
        next_email_normalized,
        next_is_enabled
    FROM indexer_health_notification_hook
    WHERE indexer_health_notification_hook_public_id = indexer_health_notification_hook_public_id_input;

    IF hook_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'hook_not_found';
    END IF;

    IF display_name_input IS NOT NULL THEN
        next_display_name := NULLIF(trim(display_name_input), '');
        IF next_display_name IS NULL THEN
            RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'display_name_missing';
        END IF;
    END IF;

    next_status_threshold := COALESCE(status_threshold_input, next_status_threshold);
    next_is_enabled := COALESCE(is_enabled_input, next_is_enabled);

    IF current_channel = 'webhook' THEN
        IF webhook_url_input IS NOT NULL THEN
            next_webhook_url := NULLIF(trim(webhook_url_input), '');
            IF next_webhook_url IS NULL THEN
                RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'webhook_url_missing';
            END IF;
            IF next_webhook_url !~ '^https?://.+' THEN
                RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'webhook_url_invalid';
            END IF;
        END IF;
        IF email_input IS NOT NULL THEN
            RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'channel_payload_mismatch';
        END IF;
        next_email := NULL;
        next_email_normalized := NULL;
    ELSE
        IF email_input IS NOT NULL THEN
            next_email := NULLIF(trim(email_input), '');
            IF next_email IS NULL THEN
                RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'email_missing';
            END IF;
            next_email_normalized := lower(next_email);
            IF position('@' IN next_email_normalized) < 2 THEN
                RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'email_invalid';
            END IF;
        END IF;
        IF webhook_url_input IS NOT NULL THEN
            RAISE EXCEPTION USING ERRCODE = errcode, MESSAGE = base_message, DETAIL = 'channel_payload_mismatch';
        END IF;
        next_webhook_url := NULL;
    END IF;

    UPDATE indexer_health_notification_hook
    SET
        display_name = next_display_name,
        status_threshold = next_status_threshold,
        webhook_url = next_webhook_url,
        email = next_email,
        email_normalized = next_email_normalized,
        is_enabled = next_is_enabled,
        updated_at = now(),
        updated_by_user_id = actor_user_id
    WHERE indexer_health_notification_hook_id = hook_id;

    RETURN indexer_health_notification_hook_public_id_input;
END;
$$;



CREATE FUNCTION public.indexer_instance_create(actor_user_public_id uuid, indexer_definition_upstream_slug_input character varying, display_name_input character varying, priority_input integer, trust_tier_key_input public.trust_tier_key, routing_policy_public_id_input uuid) RETURNS uuid
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT indexer_instance_create_v1(
        actor_user_public_id => actor_user_public_id,
        indexer_definition_upstream_slug_input => indexer_definition_upstream_slug_input,
        display_name_input => display_name_input,
        priority_input => priority_input,
        trust_tier_key_input => trust_tier_key_input,
        routing_policy_public_id_input => routing_policy_public_id_input
    );
$$;



CREATE FUNCTION public.indexer_instance_create_v1(actor_user_public_id uuid, indexer_definition_upstream_slug_input character varying, display_name_input character varying, priority_input integer, trust_tier_key_input public.trust_tier_key, routing_policy_public_id_input uuid) RETURNS uuid
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to create indexer instance';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    definition_id BIGINT;
    definition_protocol protocol;
    definition_deprecated BOOLEAN;
    routing_policy_id_value BIGINT;
    routing_policy_deleted_at TIMESTAMPTZ;
    new_instance_id BIGINT;
    new_instance_public_id UUID;
    trimmed_display_name VARCHAR(256);
    trimmed_definition_upstream_slug VARCHAR(128);
    resolved_priority INTEGER;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_definition_upstream_slug_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_missing';
    END IF;

    trimmed_definition_upstream_slug := trim(indexer_definition_upstream_slug_input);

    IF trimmed_definition_upstream_slug = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_missing';
    END IF;

    SELECT indexer_definition_id, protocol, is_deprecated
    INTO definition_id, definition_protocol, definition_deprecated
    FROM indexer_definition
    WHERE upstream_slug = trimmed_definition_upstream_slug;

    IF definition_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_not_found';
    END IF;

    IF definition_deprecated THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_deprecated';
    END IF;

    IF definition_protocol <> 'torrent' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'unsupported_protocol';
    END IF;

    IF display_name_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'display_name_missing';
    END IF;

    trimmed_display_name := trim(display_name_input);

    IF trimmed_display_name = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'display_name_empty';
    END IF;

    IF char_length(trimmed_display_name) > 256 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'display_name_too_long';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM indexer_instance
        WHERE display_name = trimmed_display_name
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'display_name_already_exists';
    END IF;

    resolved_priority := COALESCE(priority_input, 50);

    IF resolved_priority < 0 OR resolved_priority > 100 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'priority_out_of_range';
    END IF;

    IF routing_policy_public_id_input IS NOT NULL THEN
        SELECT routing_policy_id, deleted_at
        INTO routing_policy_id_value, routing_policy_deleted_at
        FROM routing_policy
        WHERE routing_policy_public_id = routing_policy_public_id_input;

        IF routing_policy_id_value IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'routing_policy_not_found';
        END IF;

        IF routing_policy_deleted_at IS NOT NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'routing_policy_deleted';
        END IF;
    ELSE
        routing_policy_id_value := NULL;
    END IF;

    IF trust_tier_key_input IS NOT NULL THEN
        IF NOT EXISTS (
            SELECT 1
            FROM trust_tier
            WHERE trust_tier_key = trust_tier_key_input
        ) THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'trust_tier_not_found';
        END IF;
    END IF;

    new_instance_public_id := gen_random_uuid();

    INSERT INTO indexer_instance (
        indexer_instance_public_id,
        indexer_definition_id,
        display_name,
        is_enabled,
        priority,
        trust_tier_key,
        routing_policy_id,
        created_by_user_id,
        updated_by_user_id
    )
    VALUES (
        new_instance_public_id,
        definition_id,
        trimmed_display_name,
        TRUE,
        resolved_priority,
        trust_tier_key_input,
        routing_policy_id_value,
        actor_user_id,
        actor_user_id
    )
    RETURNING indexer_instance_id INTO new_instance_id;

    INSERT INTO indexer_cf_state (
        indexer_instance_id,
        state,
        last_changed_at,
        cf_session_id,
        cf_session_expires_at,
        cooldown_until,
        backoff_seconds,
        consecutive_failures,
        last_error_class
    )
    VALUES (
        new_instance_id,
        'clear',
        now(),
        NULL,
        NULL,
        NULL,
        NULL,
        0,
        NULL
    );

    INSERT INTO indexer_rss_subscription (
        indexer_instance_id,
        is_enabled,
        interval_seconds,
        last_polled_at,
        next_poll_at,
        backoff_seconds,
        last_error_class
    )
    VALUES (
        new_instance_id,
        TRUE,
        900,
        NULL,
        now() + make_interval(secs => random_jitter_seconds(60)),
        NULL,
        NULL
    );

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'indexer_instance',
        new_instance_id,
        new_instance_public_id,
        'create',
        actor_user_id,
        'indexer_instance_create'
    );

    RETURN new_instance_public_id;
END;
$$;



CREATE FUNCTION public.indexer_instance_field_bind_secret(actor_user_public_id uuid, indexer_instance_public_id_input uuid, field_name_input character varying, secret_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_instance_field_bind_secret_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input, field_name_input => field_name_input, secret_public_id_input => secret_public_id_input);
END;
$$;



CREATE FUNCTION public.indexer_instance_field_bind_secret_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, field_name_input character varying, secret_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to bind indexer field secret';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    definition_id BIGINT;
    field_id BIGINT;
    field_type_value field_type;
    trimmed_field_name VARCHAR(128);
    secret_id_value BIGINT;
    binding_name_value secret_binding_name;
    field_value_id BIGINT;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    IF secret_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'secret_missing';
    END IF;

    SELECT indexer_instance_id, deleted_at, indexer_definition_id
    INTO instance_id, instance_deleted_at, definition_id
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    IF field_name_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_name_missing';
    END IF;

    trimmed_field_name := lower(trim(field_name_input));

    IF trimmed_field_name = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_name_empty';
    END IF;

    IF char_length(trimmed_field_name) > 128 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_name_too_long';
    END IF;

    SELECT indexer_definition_field_id, field_type
    INTO field_id, field_type_value
    FROM indexer_definition_field
    WHERE indexer_definition_id = definition_id
      AND name = trimmed_field_name;

    IF field_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_not_found';
    END IF;

    IF field_type_value NOT IN ('password', 'api_key', 'cookie', 'token', 'header_value') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_not_secret';
    END IF;

    SELECT secret_id
    INTO secret_id_value
    FROM secret
    WHERE secret_public_id = secret_public_id_input
      AND is_revoked = FALSE;

    IF secret_id_value IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'secret_not_found';
    END IF;

    binding_name_value := CASE field_type_value
        WHEN 'password' THEN 'password'
        WHEN 'api_key' THEN 'api_key'
        WHEN 'cookie' THEN 'cookie'
        WHEN 'token' THEN 'token'
        WHEN 'header_value' THEN 'header_value'
        ELSE 'password'
    END;

    INSERT INTO indexer_instance_field_value (
        indexer_instance_id,
        field_name,
        field_type,
        value_plain,
        value_int,
        value_decimal,
        value_bool,
        updated_by_user_id
    )
    VALUES (
        instance_id,
        trimmed_field_name,
        field_type_value,
        NULL,
        NULL,
        NULL,
        NULL,
        actor_user_id
    )
    ON CONFLICT (indexer_instance_id, field_name)
    DO UPDATE SET
        field_type = EXCLUDED.field_type,
        value_plain = NULL,
        value_int = NULL,
        value_decimal = NULL,
        value_bool = NULL,
        updated_by_user_id = EXCLUDED.updated_by_user_id,
        updated_at = now();

    SELECT indexer_instance_field_value_id
    INTO field_value_id
    FROM indexer_instance_field_value
    WHERE indexer_instance_id = instance_id
      AND field_name = trimmed_field_name;

    DELETE FROM secret_binding
    WHERE bound_table = 'indexer_instance_field_value'
      AND bound_id = field_value_id
      AND binding_name = binding_name_value;

    INSERT INTO secret_binding (
        secret_id,
        bound_table,
        bound_id,
        binding_name
    )
    VALUES (
        secret_id_value,
        'indexer_instance_field_value',
        field_value_id,
        binding_name_value
    );

    INSERT INTO secret_audit_log (
        secret_id,
        action,
        actor_user_id,
        detail
    )
    VALUES (
        secret_id_value,
        'bind',
        actor_user_id,
        'indexer_field_bind'
    );

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'indexer_instance_field_value',
        field_value_id,
        NULL,
        'update',
        actor_user_id,
        'indexer_field_bind_secret'
    );
END;
$$;



CREATE FUNCTION public.indexer_instance_field_set_value(actor_user_public_id uuid, indexer_instance_public_id_input uuid, field_name_input character varying, value_plain_input character varying, value_int_input integer, value_decimal_input numeric, value_bool_input boolean) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_instance_field_set_value_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input, field_name_input => field_name_input, value_plain_input => value_plain_input, value_int_input => value_int_input, value_decimal_input => value_decimal_input, value_bool_input => value_bool_input);
END;
$$;



CREATE FUNCTION public.indexer_instance_field_set_value_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, field_name_input character varying, value_plain_input character varying, value_int_input integer, value_decimal_input numeric, value_bool_input boolean) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to set indexer field value';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    definition_id BIGINT;
    field_id BIGINT;
    field_type_value field_type;
    existing_field_type field_type;
    trimmed_field_name VARCHAR(128);
    trimmed_value_plain VARCHAR(2048);
    value_count INTEGER;
    validation_record RECORD;
    dep_value_plain VARCHAR(2048);
    dep_value_int INTEGER;
    dep_value_decimal NUMERIC;
    dep_value_bool BOOLEAN;
    condition_met BOOLEAN;
    value_set_type_value value_set_type;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT indexer_instance_id, deleted_at, indexer_definition_id
    INTO instance_id, instance_deleted_at, definition_id
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    IF field_name_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_name_missing';
    END IF;

    trimmed_field_name := lower(trim(field_name_input));

    IF trimmed_field_name = '' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_name_empty';
    END IF;

    IF char_length(trimmed_field_name) > 128 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_name_too_long';
    END IF;

    SELECT indexer_definition_field_id, field_type
    INTO field_id, field_type_value
    FROM indexer_definition_field
    WHERE indexer_definition_id = definition_id
      AND name = trimmed_field_name;

    IF field_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_not_found';
    END IF;

    IF field_type_value IN ('password', 'api_key', 'cookie', 'token', 'header_value') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_requires_secret';
    END IF;

    trimmed_value_plain := NULL;
    IF value_plain_input IS NOT NULL THEN
        trimmed_value_plain := trim(value_plain_input);
        IF trimmed_value_plain = '' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'value_empty';
        END IF;
    END IF;

    value_count := (trimmed_value_plain IS NOT NULL)::INT
        + (value_int_input IS NOT NULL)::INT
        + (value_decimal_input IS NOT NULL)::INT
        + (value_bool_input IS NOT NULL)::INT;

    IF value_count <> 1 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'value_count_invalid';
    END IF;

    IF field_type_value IN ('string', 'select_single') THEN
        IF trimmed_value_plain IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'value_type_mismatch';
        END IF;
    ELSIF field_type_value = 'number_int' THEN
        IF value_int_input IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'value_type_mismatch';
        END IF;
    ELSIF field_type_value = 'number_decimal' THEN
        IF value_decimal_input IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'value_type_mismatch';
        END IF;
    ELSIF field_type_value = 'bool' THEN
        IF value_bool_input IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'value_type_mismatch';
        END IF;
    END IF;

    IF trimmed_value_plain IS NOT NULL AND char_length(trimmed_value_plain) > 2048 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'value_too_long';
    END IF;

    FOR validation_record IN
        SELECT
            validation_type,
            int_value,
            numeric_value,
            text_value,
            text_value_norm,
            value_set_id,
            depends_on_field_name,
            depends_on_operator,
            depends_on_value_plain_norm,
            depends_on_value_int,
            depends_on_value_bool,
            depends_on_value_set_id
        FROM indexer_definition_field_validation
        WHERE indexer_definition_field_id = field_id
    LOOP
        IF validation_record.validation_type = 'min_length' THEN
            IF trimmed_value_plain IS NOT NULL
                AND char_length(trimmed_value_plain) < validation_record.int_value THEN
                RAISE EXCEPTION USING
                    ERRCODE = errcode,
                    MESSAGE = base_message,
                    DETAIL = 'value_too_short';
            END IF;
        ELSIF validation_record.validation_type = 'max_length' THEN
            IF trimmed_value_plain IS NOT NULL
                AND char_length(trimmed_value_plain) > validation_record.int_value THEN
                RAISE EXCEPTION USING
                    ERRCODE = errcode,
                    MESSAGE = base_message,
                    DETAIL = 'value_too_long';
            END IF;
        ELSIF validation_record.validation_type = 'min_value' THEN
            IF value_int_input IS NOT NULL
                AND value_int_input < validation_record.numeric_value THEN
                RAISE EXCEPTION USING
                    ERRCODE = errcode,
                    MESSAGE = base_message,
                    DETAIL = 'value_too_small';
            END IF;
            IF value_decimal_input IS NOT NULL
                AND value_decimal_input < validation_record.numeric_value THEN
                RAISE EXCEPTION USING
                    ERRCODE = errcode,
                    MESSAGE = base_message,
                    DETAIL = 'value_too_small';
            END IF;
        ELSIF validation_record.validation_type = 'max_value' THEN
            IF value_int_input IS NOT NULL
                AND value_int_input > validation_record.numeric_value THEN
                RAISE EXCEPTION USING
                    ERRCODE = errcode,
                    MESSAGE = base_message,
                    DETAIL = 'value_too_large';
            END IF;
            IF value_decimal_input IS NOT NULL
                AND value_decimal_input > validation_record.numeric_value THEN
                RAISE EXCEPTION USING
                    ERRCODE = errcode,
                    MESSAGE = base_message,
                    DETAIL = 'value_too_large';
            END IF;
        ELSIF validation_record.validation_type = 'regex' THEN
            IF trimmed_value_plain IS NOT NULL
                AND trimmed_value_plain !~ validation_record.text_value THEN
                RAISE EXCEPTION USING
                    ERRCODE = errcode,
                    MESSAGE = base_message,
                    DETAIL = 'value_regex_mismatch';
            END IF;
        ELSIF validation_record.validation_type = 'allowed_value' THEN
            IF validation_record.value_set_id IS NULL THEN
                IF trimmed_value_plain IS NULL
                    OR lower(trimmed_value_plain) <> validation_record.text_value_norm THEN
                    RAISE EXCEPTION USING
                        ERRCODE = errcode,
                        MESSAGE = base_message,
                        DETAIL = 'value_not_allowed';
                END IF;
            ELSE
                SELECT value_set_type
                INTO value_set_type_value
                FROM indexer_definition_field_value_set
                WHERE value_set_id = validation_record.value_set_id;

                IF value_set_type_value = 'text' THEN
                    IF trimmed_value_plain IS NULL OR NOT EXISTS (
                        SELECT 1
                        FROM indexer_definition_field_value_set_item
                        WHERE value_set_id = validation_record.value_set_id
                          AND value_text = lower(trimmed_value_plain)
                    ) THEN
                        RAISE EXCEPTION USING
                            ERRCODE = errcode,
                            MESSAGE = base_message,
                            DETAIL = 'value_not_allowed';
                    END IF;
                ELSIF value_set_type_value = 'int' THEN
                    IF value_int_input IS NULL OR NOT EXISTS (
                        SELECT 1
                        FROM indexer_definition_field_value_set_item
                        WHERE value_set_id = validation_record.value_set_id
                          AND value_int = value_int_input
                    ) THEN
                        RAISE EXCEPTION USING
                            ERRCODE = errcode,
                            MESSAGE = base_message,
                            DETAIL = 'value_not_allowed';
                    END IF;
                ELSIF value_set_type_value = 'bigint' THEN
                    IF value_int_input IS NULL OR NOT EXISTS (
                        SELECT 1
                        FROM indexer_definition_field_value_set_item
                        WHERE value_set_id = validation_record.value_set_id
                          AND value_bigint = value_int_input::BIGINT
                    ) THEN
                        RAISE EXCEPTION USING
                            ERRCODE = errcode,
                            MESSAGE = base_message,
                            DETAIL = 'value_not_allowed';
                    END IF;
                END IF;
            END IF;
        ELSIF validation_record.validation_type = 'required_if_field_equals' THEN
            condition_met := FALSE;

            SELECT value_plain, value_int, value_decimal, value_bool
            INTO dep_value_plain, dep_value_int, dep_value_decimal, dep_value_bool
            FROM indexer_instance_field_value
            WHERE indexer_instance_id = instance_id
              AND field_name = validation_record.depends_on_field_name;

            IF validation_record.depends_on_operator = 'eq' THEN
                IF validation_record.depends_on_value_plain_norm IS NOT NULL THEN
                    condition_met := dep_value_plain IS NOT NULL
                        AND lower(trim(dep_value_plain)) = validation_record.depends_on_value_plain_norm;
                ELSIF validation_record.depends_on_value_int IS NOT NULL THEN
                    condition_met := dep_value_int IS NOT NULL
                        AND dep_value_int = validation_record.depends_on_value_int;
                ELSIF validation_record.depends_on_value_bool IS NOT NULL THEN
                    condition_met := dep_value_bool IS NOT NULL
                        AND dep_value_bool = validation_record.depends_on_value_bool;
                ELSIF validation_record.depends_on_value_set_id IS NOT NULL THEN
                    SELECT value_set_type
                    INTO value_set_type_value
                    FROM indexer_definition_field_value_set
                    WHERE value_set_id = validation_record.depends_on_value_set_id;

                    IF value_set_type_value = 'text' THEN
                        condition_met := dep_value_plain IS NOT NULL AND EXISTS (
                            SELECT 1
                            FROM indexer_definition_field_value_set_item
                            WHERE value_set_id = validation_record.depends_on_value_set_id
                              AND value_text = lower(trim(dep_value_plain))
                        );
                    ELSIF value_set_type_value = 'int' THEN
                        condition_met := dep_value_int IS NOT NULL AND EXISTS (
                            SELECT 1
                            FROM indexer_definition_field_value_set_item
                            WHERE value_set_id = validation_record.depends_on_value_set_id
                              AND value_int = dep_value_int
                        );
                    ELSIF value_set_type_value = 'bigint' THEN
                        condition_met := dep_value_int IS NOT NULL AND EXISTS (
                            SELECT 1
                            FROM indexer_definition_field_value_set_item
                            WHERE value_set_id = validation_record.depends_on_value_set_id
                              AND value_bigint = dep_value_int::BIGINT
                        );
                    END IF;
                END IF;
            ELSIF validation_record.depends_on_operator = 'neq' THEN
                IF validation_record.depends_on_value_plain_norm IS NOT NULL THEN
                    condition_met := dep_value_plain IS NOT NULL
                        AND lower(trim(dep_value_plain)) <> validation_record.depends_on_value_plain_norm;
                ELSIF validation_record.depends_on_value_int IS NOT NULL THEN
                    condition_met := dep_value_int IS NOT NULL
                        AND dep_value_int <> validation_record.depends_on_value_int;
                ELSIF validation_record.depends_on_value_bool IS NOT NULL THEN
                    condition_met := dep_value_bool IS NOT NULL
                        AND dep_value_bool <> validation_record.depends_on_value_bool;
                ELSIF validation_record.depends_on_value_set_id IS NOT NULL THEN
                    SELECT value_set_type
                    INTO value_set_type_value
                    FROM indexer_definition_field_value_set
                    WHERE value_set_id = validation_record.depends_on_value_set_id;

                    IF value_set_type_value = 'text' THEN
                        condition_met := dep_value_plain IS NOT NULL AND NOT EXISTS (
                            SELECT 1
                            FROM indexer_definition_field_value_set_item
                            WHERE value_set_id = validation_record.depends_on_value_set_id
                              AND value_text = lower(trim(dep_value_plain))
                        );
                    ELSIF value_set_type_value = 'int' THEN
                        condition_met := dep_value_int IS NOT NULL AND NOT EXISTS (
                            SELECT 1
                            FROM indexer_definition_field_value_set_item
                            WHERE value_set_id = validation_record.depends_on_value_set_id
                              AND value_int = dep_value_int
                        );
                    ELSIF value_set_type_value = 'bigint' THEN
                        condition_met := dep_value_int IS NOT NULL AND NOT EXISTS (
                            SELECT 1
                            FROM indexer_definition_field_value_set_item
                            WHERE value_set_id = validation_record.depends_on_value_set_id
                              AND value_bigint = dep_value_int::BIGINT
                        );
                    END IF;
                END IF;
            ELSIF validation_record.depends_on_operator = 'in_set' THEN
                IF validation_record.depends_on_value_set_id IS NOT NULL THEN
                    SELECT value_set_type
                    INTO value_set_type_value
                    FROM indexer_definition_field_value_set
                    WHERE value_set_id = validation_record.depends_on_value_set_id;

                    IF value_set_type_value = 'text' THEN
                        condition_met := dep_value_plain IS NOT NULL AND EXISTS (
                            SELECT 1
                            FROM indexer_definition_field_value_set_item
                            WHERE value_set_id = validation_record.depends_on_value_set_id
                              AND value_text = lower(trim(dep_value_plain))
                        );
                    ELSIF value_set_type_value = 'int' THEN
                        condition_met := dep_value_int IS NOT NULL AND EXISTS (
                            SELECT 1
                            FROM indexer_definition_field_value_set_item
                            WHERE value_set_id = validation_record.depends_on_value_set_id
                              AND value_int = dep_value_int
                        );
                    ELSIF value_set_type_value = 'bigint' THEN
                        condition_met := dep_value_int IS NOT NULL AND EXISTS (
                            SELECT 1
                            FROM indexer_definition_field_value_set_item
                            WHERE value_set_id = validation_record.depends_on_value_set_id
                              AND value_bigint = dep_value_int::BIGINT
                        );
                    END IF;
                END IF;
            END IF;

            IF condition_met AND value_count = 0 THEN
                RAISE EXCEPTION USING
                    ERRCODE = errcode,
                    MESSAGE = base_message,
                    DETAIL = 'value_required';
            END IF;
        END IF;
    END LOOP;

    SELECT field_type
    INTO existing_field_type
    FROM indexer_instance_field_value
    WHERE indexer_instance_id = instance_id
      AND field_name = trimmed_field_name;

    IF existing_field_type IS NOT NULL AND existing_field_type <> field_type_value THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'field_type_mismatch';
    END IF;

    INSERT INTO indexer_instance_field_value (
        indexer_instance_id,
        field_name,
        field_type,
        value_plain,
        value_int,
        value_decimal,
        value_bool,
        updated_by_user_id
    )
    VALUES (
        instance_id,
        trimmed_field_name,
        field_type_value,
        trimmed_value_plain,
        value_int_input,
        value_decimal_input,
        value_bool_input,
        actor_user_id
    )
    ON CONFLICT (indexer_instance_id, field_name)
    DO UPDATE SET
        field_type = EXCLUDED.field_type,
        value_plain = EXCLUDED.value_plain,
        value_int = EXCLUDED.value_int,
        value_decimal = EXCLUDED.value_decimal,
        value_bool = EXCLUDED.value_bool,
        updated_by_user_id = EXCLUDED.updated_by_user_id,
        updated_at = now();

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'indexer_instance_field_value',
        (SELECT indexer_instance_field_value_id
         FROM indexer_instance_field_value
         WHERE indexer_instance_id = instance_id
           AND field_name = trimmed_field_name),
        NULL,
        'update',
        actor_user_id,
        'indexer_field_set'
    );
END;
$$;



CREATE FUNCTION public.indexer_instance_set_media_domains(actor_user_public_id uuid, indexer_instance_public_id_input uuid, media_domain_keys_input text[]) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_instance_set_media_domains_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input, media_domain_keys_input => media_domain_keys_input);
END;
$$;



CREATE FUNCTION public.indexer_instance_set_media_domains_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, media_domain_keys_input text[]) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to set media domains';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    normalized_keys TEXT[];
    input_count INTEGER;
    resolved_count INTEGER;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT indexer_instance_id, deleted_at
    INTO instance_id, instance_deleted_at
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    IF media_domain_keys_input IS NULL THEN
        normalized_keys := ARRAY[]::TEXT[];
    ELSE
        SELECT array_agg(DISTINCT lower(trim(value)))
        INTO normalized_keys
        FROM unnest(media_domain_keys_input) AS value;

        IF EXISTS (
            SELECT 1
            FROM unnest(media_domain_keys_input) AS value
            WHERE trim(value) = '' OR trim(value) <> lower(trim(value))
        ) THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'media_domain_key_invalid';
        END IF;
    END IF;

    IF normalized_keys IS NULL THEN
        normalized_keys := ARRAY[]::TEXT[];
    END IF;

    SELECT count(*)
    INTO input_count
    FROM unnest(normalized_keys) AS value
    WHERE value IS NOT NULL AND value <> '';

    IF input_count = 0 THEN
        DELETE FROM indexer_instance_media_domain
        WHERE indexer_instance_id = instance_id;
    ELSE
        SELECT count(*)
        INTO resolved_count
        FROM media_domain
        WHERE media_domain_key::TEXT = ANY(normalized_keys);

        IF resolved_count <> input_count THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'unknown_key';
        END IF;

        DELETE FROM indexer_instance_media_domain
        WHERE indexer_instance_id = instance_id;

        INSERT INTO indexer_instance_media_domain (
            indexer_instance_id,
            media_domain_id
        )
        SELECT
            instance_id,
            media_domain_id
        FROM media_domain
        WHERE media_domain_key::TEXT = ANY(normalized_keys);
    END IF;

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'indexer_instance',
        instance_id,
        indexer_instance_public_id_input,
        'update',
        actor_user_id,
        'indexer_media_domains_set'
    );
END;
$$;



CREATE FUNCTION public.indexer_instance_set_rate_limit_policy(actor_user_public_id uuid, indexer_instance_public_id_input uuid, rate_limit_policy_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_instance_set_rate_limit_policy_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input, rate_limit_policy_public_id_input => rate_limit_policy_public_id_input);
END;
$$;



CREATE FUNCTION public.indexer_instance_set_rate_limit_policy_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, rate_limit_policy_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to set indexer rate limit policy';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    indexer_id BIGINT;
    indexer_deleted_at TIMESTAMPTZ;
    policy_id BIGINT;
    policy_deleted_at TIMESTAMPTZ;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT indexer_instance_id, deleted_at
    INTO indexer_id, indexer_deleted_at
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF indexer_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF indexer_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    IF rate_limit_policy_public_id_input IS NULL THEN
        DELETE FROM indexer_instance_rate_limit
        WHERE indexer_instance_id = indexer_id;
    ELSE
        SELECT rate_limit_policy_id, deleted_at
        INTO policy_id, policy_deleted_at
        FROM rate_limit_policy
        WHERE rate_limit_policy_public_id = rate_limit_policy_public_id_input;

        IF policy_id IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'policy_not_found';
        END IF;

        IF policy_deleted_at IS NOT NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'policy_deleted';
        END IF;

        INSERT INTO indexer_instance_rate_limit (
            indexer_instance_id,
            rate_limit_policy_id
        )
        VALUES (
            indexer_id,
            policy_id
        )
        ON CONFLICT (indexer_instance_id)
        DO UPDATE SET rate_limit_policy_id = EXCLUDED.rate_limit_policy_id;
    END IF;

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'indexer_instance',
        indexer_id,
        indexer_instance_public_id_input,
        'update',
        actor_user_id,
        'indexer_rate_limit_set'
    );
END;
$$;



CREATE FUNCTION public.indexer_instance_set_tags(actor_user_public_id uuid, indexer_instance_public_id_input uuid, tag_public_ids_input uuid[], tag_keys_input text[]) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_instance_set_tags_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input, tag_public_ids_input => tag_public_ids_input, tag_keys_input => tag_keys_input);
END;
$$;



CREATE FUNCTION public.indexer_instance_set_tags_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, tag_public_ids_input uuid[], tag_keys_input text[]) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to set tags';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    resolved_ids_from_public UUID[];
    resolved_ids_from_keys UUID[];
    normalized_keys TEXT[];
    public_count INTEGER;
    public_resolved INTEGER;
    key_count INTEGER;
    key_resolved INTEGER;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT indexer_instance_id, deleted_at
    INTO instance_id, instance_deleted_at
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    IF tag_public_ids_input IS NULL AND tag_keys_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'tag_reference_missing';
    END IF;

    IF tag_public_ids_input IS NOT NULL THEN
        SELECT array_agg(DISTINCT tag_public_id), count(DISTINCT tag_public_id)
        INTO resolved_ids_from_public, public_resolved
        FROM tag
        WHERE tag_public_id = ANY(tag_public_ids_input)
          AND deleted_at IS NULL;

        SELECT count(DISTINCT value)
        INTO public_count
        FROM unnest(tag_public_ids_input) AS value;

        IF public_resolved IS NULL OR public_resolved <> public_count THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'tag_not_found';
        END IF;
    END IF;

    IF tag_keys_input IS NOT NULL THEN
        SELECT array_agg(DISTINCT lower(trim(value)))
        INTO normalized_keys
        FROM unnest(tag_keys_input) AS value;

        IF EXISTS (
            SELECT 1
            FROM unnest(tag_keys_input) AS value
            WHERE trim(value) = ''
               OR trim(value) <> lower(trim(value))
               OR char_length(trim(value)) > 128
        ) THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'tag_key_invalid';
        END IF;

        SELECT count(*)
        INTO key_count
        FROM unnest(normalized_keys) AS value
        WHERE value IS NOT NULL AND value <> '';

        IF key_count = 0 THEN
            normalized_keys := ARRAY[]::TEXT[];
        END IF;

        SELECT array_agg(DISTINCT tag_public_id), count(DISTINCT tag_public_id)
        INTO resolved_ids_from_keys, key_resolved
        FROM tag
        WHERE tag_key = ANY(normalized_keys)
          AND deleted_at IS NULL;

        IF key_resolved IS NULL OR key_resolved <> key_count THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'unknown_key';
        END IF;
    END IF;

    IF tag_public_ids_input IS NOT NULL AND tag_keys_input IS NOT NULL THEN
        IF EXISTS (
            SELECT value
            FROM unnest(resolved_ids_from_public) AS value
            EXCEPT
            SELECT value
            FROM unnest(resolved_ids_from_keys) AS value
        ) OR EXISTS (
            SELECT value
            FROM unnest(resolved_ids_from_keys) AS value
            EXCEPT
            SELECT value
            FROM unnest(resolved_ids_from_public) AS value
        ) THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'invalid_tag_reference';
        END IF;
    END IF;

    DELETE FROM indexer_instance_tag
    WHERE indexer_instance_id = instance_id;

    INSERT INTO indexer_instance_tag (
        indexer_instance_id,
        tag_id
    )
    SELECT
        instance_id,
        tag_id
    FROM tag
    WHERE tag_public_id = ANY(
        COALESCE(resolved_ids_from_public, resolved_ids_from_keys, ARRAY[]::UUID[])
    );

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'indexer_instance',
        instance_id,
        indexer_instance_public_id_input,
        'update',
        actor_user_id,
        'indexer_tags_set'
    );
END;
$$;



CREATE FUNCTION public.indexer_instance_test_finalize(actor_user_public_id uuid, indexer_instance_public_id_input uuid, ok_input boolean, error_class_input public.error_class, error_code_input character varying, detail_input character varying, result_count_input integer) RETURNS TABLE(ok boolean, error_class public.error_class, error_code character varying, detail character varying, result_count integer)
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT * FROM indexer_instance_test_finalize_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input, ok_input => ok_input, error_class_input => error_class_input, error_code_input => error_code_input, detail_input => detail_input, result_count_input => result_count_input);
$$;



CREATE FUNCTION public.indexer_instance_test_finalize_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, ok_input boolean, error_class_input public.error_class, error_code_input character varying, detail_input character varying, result_count_input integer) RETURNS TABLE(ok boolean, error_class public.error_class, error_code character varying, detail character varying, result_count integer)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to finalize indexer test';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    migration_state_value indexer_instance_migration_state;
BEGIN
    IF actor_user_public_id IS NOT NULL THEN
        SELECT user_id, role
        INTO actor_user_id, actor_role
        FROM app_user
        WHERE user_public_id = actor_user_public_id;

        IF actor_user_id IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'actor_not_found';
        END IF;

        IF actor_role NOT IN ('owner', 'admin') THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'actor_unauthorized';
        END IF;
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    IF ok_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'ok_missing';
    END IF;

    IF error_code_input IS NOT NULL AND char_length(error_code_input) > 64 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'error_code_too_long';
    END IF;

    IF detail_input IS NOT NULL AND char_length(detail_input) > 256 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'detail_too_long';
    END IF;

    SELECT indexer_instance_id, deleted_at, migration_state
    INTO instance_id, instance_deleted_at, migration_state_value
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    IF ok_input THEN
        IF migration_state_value IS NOT NULL THEN
            IF migration_state_value = 'duplicate_suspected' THEN
                UPDATE indexer_instance
                SET migration_state = 'ready',
                    migration_detail = NULL
                WHERE indexer_instance_id = instance_id;
            ELSE
                UPDATE indexer_instance
                SET migration_state = 'ready',
                    migration_detail = NULL,
                    is_enabled = TRUE
                WHERE indexer_instance_id = instance_id;
            END IF;
        END IF;
    ELSE
        IF error_code_input = 'missing_secret' THEN
            IF migration_state_value IS NOT NULL THEN
                UPDATE indexer_instance
                SET migration_state = 'needs_secret',
                    is_enabled = FALSE
                WHERE indexer_instance_id = instance_id;
            END IF;
        ELSE
            IF migration_state_value IN (
                'ready',
                'needs_secret',
                'test_failed',
                'duplicate_suspected'
            ) THEN
                UPDATE indexer_instance
                SET migration_state = 'test_failed',
                    migration_detail = detail_input,
                    is_enabled = FALSE
                WHERE indexer_instance_id = instance_id;
            END IF;
        END IF;
    END IF;

    ok := ok_input;
    error_class := error_class_input;
    error_code := error_code_input;
    detail := detail_input;
    result_count := result_count_input;

    RETURN NEXT;
    RETURN;
END;
$$;



CREATE FUNCTION public.indexer_instance_test_prepare(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS TABLE(can_execute boolean, error_class public.error_class, error_code character varying, detail character varying, engine public.engine, routing_policy_public_id uuid, connect_timeout_ms integer, read_timeout_ms integer, field_names character varying[], field_types public.field_type[], value_plain character varying[], value_int integer[], value_decimal numeric[], value_bool boolean[], secret_public_ids uuid[])
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT * FROM indexer_instance_test_prepare_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input);
$$;



CREATE FUNCTION public.indexer_instance_test_prepare_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS TABLE(can_execute boolean, error_class public.error_class, error_code character varying, detail character varying, engine public.engine, routing_policy_public_id uuid, connect_timeout_ms integer, read_timeout_ms integer, field_names character varying[], field_types public.field_type[], value_plain character varying[], value_int integer[], value_decimal numeric[], value_bool boolean[], secret_public_ids uuid[])
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to prepare indexer test';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    migration_state_value indexer_instance_migration_state;
    routing_policy_id_value BIGINT;
    routing_policy_deleted_at TIMESTAMPTZ;
    definition_id BIGINT;
    missing_fields TEXT;
    missing_detail VARCHAR(256);
    missing_count INTEGER;
BEGIN
    IF actor_user_public_id IS NOT NULL THEN
        SELECT user_id, role
        INTO actor_user_id, actor_role
        FROM app_user
        WHERE user_public_id = actor_user_public_id;

        IF actor_user_id IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'actor_not_found';
        END IF;

        IF actor_role NOT IN ('owner', 'admin') THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'actor_unauthorized';
        END IF;
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT
        indexer_instance.indexer_instance_id,
        indexer_instance.deleted_at,
        indexer_instance.migration_state,
        indexer_instance.routing_policy_id,
        indexer_instance.connect_timeout_ms,
        indexer_instance.read_timeout_ms,
        indexer_instance.indexer_definition_id
    INTO
        instance_id,
        instance_deleted_at,
        migration_state_value,
        routing_policy_id_value,
        connect_timeout_ms,
        read_timeout_ms,
        definition_id
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    SELECT indexer_definition.engine
    INTO engine
    FROM indexer_definition
    WHERE indexer_definition_id = definition_id;

    IF engine IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'definition_not_found';
    END IF;

    routing_policy_public_id := NULL;
    IF routing_policy_id_value IS NOT NULL THEN
        SELECT routing_policy.routing_policy_public_id, routing_policy.deleted_at
        INTO routing_policy_public_id, routing_policy_deleted_at
        FROM routing_policy
        WHERE routing_policy_id = routing_policy_id_value;

        IF routing_policy_public_id IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'routing_policy_not_found';
        END IF;

        IF routing_policy_deleted_at IS NOT NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'routing_policy_deleted';
        END IF;
    END IF;

    SELECT
        string_agg(required.name, ', ' ORDER BY required.name),
        count(*)
    INTO missing_fields, missing_count
    FROM (
        SELECT name
        FROM indexer_definition_field
        WHERE indexer_definition_id = definition_id
          AND is_required = TRUE
          AND field_type IN ('password', 'api_key', 'cookie', 'token', 'header_value')
    ) AS required
    LEFT JOIN (
        SELECT fv.field_name
        FROM indexer_instance_field_value fv
        JOIN secret_binding sb
            ON sb.bound_table = 'indexer_instance_field_value'
            AND sb.bound_id = fv.indexer_instance_field_value_id
        JOIN secret s
            ON s.secret_id = sb.secret_id
            AND s.is_revoked = FALSE
        WHERE fv.indexer_instance_id = instance_id
    ) AS bound
        ON bound.field_name = required.name
    WHERE bound.field_name IS NULL;

    IF missing_count > 0 THEN
        missing_detail := left(missing_fields, 256);

        can_execute := FALSE;
        error_class := 'auth_error';
        error_code := 'missing_secret';
        detail := missing_detail;
        field_names := NULL;
        field_types := NULL;
        value_plain := NULL;
        value_int := NULL;
        value_decimal := NULL;
        value_bool := NULL;
        secret_public_ids := NULL;

        IF migration_state_value IS NOT NULL THEN
            UPDATE indexer_instance
            SET migration_state = 'needs_secret',
                is_enabled = FALSE
            WHERE indexer_instance_id = instance_id;
        END IF;

        RETURN NEXT;
        RETURN;
    END IF;

    SELECT
        array_agg(fv.field_name ORDER BY fv.field_name),
        array_agg(fv.field_type ORDER BY fv.field_name),
        array_agg(fv.value_plain ORDER BY fv.field_name),
        array_agg(fv.value_int ORDER BY fv.field_name),
        array_agg(fv.value_decimal ORDER BY fv.field_name),
        array_agg(fv.value_bool ORDER BY fv.field_name),
        array_agg(s.secret_public_id ORDER BY fv.field_name)
    INTO
        field_names,
        field_types,
        value_plain,
        value_int,
        value_decimal,
        value_bool,
        secret_public_ids
    FROM indexer_instance_field_value fv
    LEFT JOIN secret_binding sb
        ON sb.bound_table = 'indexer_instance_field_value'
        AND sb.bound_id = fv.indexer_instance_field_value_id
    LEFT JOIN secret s
        ON s.secret_id = sb.secret_id
        AND s.is_revoked = FALSE
    WHERE fv.indexer_instance_id = instance_id;

    can_execute := TRUE;
    error_class := NULL;
    error_code := NULL;
    detail := NULL;

    RETURN NEXT;
    RETURN;
END;
$$;



CREATE FUNCTION public.indexer_instance_update(actor_user_public_id uuid, indexer_instance_public_id_input uuid, display_name_input character varying, priority_input integer, trust_tier_key_input public.trust_tier_key, routing_policy_public_id_input uuid, is_enabled_input boolean, enable_rss_input boolean, enable_automatic_search_input boolean, enable_interactive_search_input boolean) RETURNS uuid
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT indexer_instance_update_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input, display_name_input => display_name_input, priority_input => priority_input, trust_tier_key_input => trust_tier_key_input, routing_policy_public_id_input => routing_policy_public_id_input, is_enabled_input => is_enabled_input, enable_rss_input => enable_rss_input, enable_automatic_search_input => enable_automatic_search_input, enable_interactive_search_input => enable_interactive_search_input);
$$;



CREATE FUNCTION public.indexer_instance_update_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, display_name_input character varying, priority_input integer, trust_tier_key_input public.trust_tier_key, routing_policy_public_id_input uuid, is_enabled_input boolean, enable_rss_input boolean, enable_automatic_search_input boolean, enable_interactive_search_input boolean) RETURNS uuid
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to update indexer instance';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    current_display_name VARCHAR(256);
    current_is_enabled BOOLEAN;
    current_enable_rss BOOLEAN;
    current_priority INTEGER;
    current_trust_tier trust_tier_key;
    current_routing_policy_id BIGINT;
    new_display_name VARCHAR(256);
    new_priority INTEGER;
    new_trust_tier trust_tier_key;
    new_routing_policy_id BIGINT;
    new_is_enabled BOOLEAN;
    new_enable_rss BOOLEAN;
    new_enable_automatic_search BOOLEAN;
    new_enable_interactive_search BOOLEAN;
    routing_policy_deleted_at TIMESTAMPTZ;
    audit_action audit_action;
    rss_row_exists BOOLEAN;
    rss_is_enabled BOOLEAN;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT
        indexer_instance_id,
        deleted_at,
        display_name,
        is_enabled,
        enable_rss,
        enable_automatic_search,
        enable_interactive_search,
        priority,
        trust_tier_key,
        routing_policy_id
    INTO
        instance_id,
        instance_deleted_at,
        current_display_name,
        current_is_enabled,
        current_enable_rss,
        new_enable_automatic_search,
        new_enable_interactive_search,
        current_priority,
        current_trust_tier,
        current_routing_policy_id
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    IF display_name_input IS NOT NULL THEN
        new_display_name := trim(display_name_input);

        IF new_display_name = '' THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'display_name_empty';
        END IF;

        IF char_length(new_display_name) > 256 THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'display_name_too_long';
        END IF;

        IF EXISTS (
            SELECT 1
            FROM indexer_instance
            WHERE display_name = new_display_name
              AND indexer_instance_id <> instance_id
        ) THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'display_name_already_exists';
        END IF;
    ELSE
        new_display_name := current_display_name;
    END IF;

    IF priority_input IS NOT NULL THEN
        IF priority_input < 0 OR priority_input > 100 THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'priority_out_of_range';
        END IF;
        new_priority := priority_input;
    ELSE
        new_priority := current_priority;
    END IF;

    IF trust_tier_key_input IS NOT NULL THEN
        IF NOT EXISTS (
            SELECT 1
            FROM trust_tier
            WHERE trust_tier_key = trust_tier_key_input
        ) THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'unknown_key';
        END IF;
        new_trust_tier := trust_tier_key_input;
    ELSE
        new_trust_tier := current_trust_tier;
    END IF;

    IF routing_policy_public_id_input IS NOT NULL THEN
        SELECT routing_policy_id, deleted_at
        INTO new_routing_policy_id, routing_policy_deleted_at
        FROM routing_policy
        WHERE routing_policy_public_id = routing_policy_public_id_input;

        IF new_routing_policy_id IS NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'routing_policy_not_found';
        END IF;

        IF routing_policy_deleted_at IS NOT NULL THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'routing_policy_deleted';
        END IF;
    ELSE
        new_routing_policy_id := current_routing_policy_id;
    END IF;

    new_is_enabled := COALESCE(is_enabled_input, current_is_enabled);
    new_enable_rss := COALESCE(enable_rss_input, current_enable_rss);
    new_enable_automatic_search := COALESCE(
        enable_automatic_search_input,
        new_enable_automatic_search
    );
    new_enable_interactive_search := COALESCE(
        enable_interactive_search_input,
        new_enable_interactive_search
    );

    UPDATE indexer_instance
    SET display_name = new_display_name,
        priority = new_priority,
        trust_tier_key = new_trust_tier,
        routing_policy_id = new_routing_policy_id,
        is_enabled = new_is_enabled,
        enable_rss = new_enable_rss,
        enable_automatic_search = new_enable_automatic_search,
        enable_interactive_search = new_enable_interactive_search,
        updated_by_user_id = actor_user_id,
        updated_at = now()
    WHERE indexer_instance_id = instance_id;

    SELECT EXISTS (
        SELECT 1
        FROM indexer_rss_subscription
        WHERE indexer_instance_id = instance_id
    ) INTO rss_row_exists;

    IF new_is_enabled IS FALSE OR new_enable_rss IS FALSE THEN
        IF rss_row_exists THEN
            UPDATE indexer_rss_subscription
            SET is_enabled = FALSE,
                next_poll_at = NULL
            WHERE indexer_instance_id = instance_id;
        ELSE
            INSERT INTO indexer_rss_subscription (
                indexer_instance_id,
                is_enabled,
                interval_seconds,
                last_polled_at,
                next_poll_at,
                backoff_seconds,
                last_error_class
            )
            VALUES (
                instance_id,
                FALSE,
                900,
                NULL,
                NULL,
                NULL,
                NULL
            );
        END IF;
    ELSIF new_is_enabled IS TRUE AND new_enable_rss IS TRUE THEN
        IF rss_row_exists IS FALSE THEN
            INSERT INTO indexer_rss_subscription (
                indexer_instance_id,
                is_enabled,
                interval_seconds,
                last_polled_at,
                next_poll_at,
                backoff_seconds,
                last_error_class
            )
            VALUES (
                instance_id,
                TRUE,
                900,
                NULL,
                now() + make_interval(secs => random_jitter_seconds(60)),
                NULL,
                NULL
            );
        END IF;
    END IF;

    IF current_is_enabled IS DISTINCT FROM new_is_enabled THEN
        IF new_is_enabled THEN
            audit_action := 'enable';
        ELSE
            audit_action := 'disable';
        END IF;
    ELSE
        audit_action := 'update';
    END IF;

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'indexer_instance',
        instance_id,
        indexer_instance_public_id_input,
        audit_action,
        actor_user_id,
        'indexer_instance_update'
    );

    RETURN indexer_instance_public_id_input;
END;
$$;



CREATE FUNCTION public.indexer_policy_set_rule_list(actor_user_public_id uuid) RETURNS TABLE(policy_set_public_id uuid, policy_set_display_name character varying, scope public.policy_scope, is_enabled boolean, user_public_id uuid, policy_rule_public_id uuid, rule_type public.policy_rule_type, match_field public.policy_match_field, match_operator public.policy_match_operator, sort_order integer, match_value_text character varying, match_value_int integer, match_value_uuid uuid, action public.policy_action, severity public.policy_severity, is_case_insensitive boolean, rationale character varying, expires_at timestamp with time zone, is_rule_disabled boolean)
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT *
    FROM indexer_policy_set_rule_list_v1(actor_user_public_id);
$$;



CREATE FUNCTION public.indexer_policy_set_rule_list_v1(actor_user_public_id uuid) RETURNS TABLE(policy_set_public_id uuid, policy_set_display_name character varying, scope public.policy_scope, is_enabled boolean, user_public_id uuid, policy_rule_public_id uuid, rule_type public.policy_rule_type, match_field public.policy_match_field, match_operator public.policy_match_operator, sort_order integer, match_value_text character varying, match_value_int integer, match_value_uuid uuid, action public.policy_action, severity public.policy_severity, is_case_insensitive boolean, rationale character varying, expires_at timestamp with time zone, is_rule_disabled boolean)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_backup_assert_actor_v1(actor_user_public_id);

    RETURN QUERY
    SELECT
        policy.policy_set_public_id,
        policy.display_name,
        policy.scope,
        policy.is_enabled,
        owner.user_public_id,
        rule.policy_rule_public_id,
        rule.rule_type,
        rule.match_field,
        rule.match_operator,
        rule.sort_order,
        rule.match_value_text,
        rule.match_value_int,
        rule.match_value_uuid,
        rule.action,
        rule.severity,
        rule.is_case_insensitive,
        rule.rationale,
        rule.expires_at,
        rule.is_disabled
    FROM policy_set policy
    LEFT JOIN app_user owner
        ON owner.user_id = policy.user_id
    LEFT JOIN policy_rule rule
        ON rule.policy_set_id = policy.policy_set_id
    WHERE policy.deleted_at IS NULL
    ORDER BY
        policy.display_name,
        policy.policy_set_public_id,
        rule.sort_order NULLS FIRST,
        rule.policy_rule_public_id NULLS FIRST;
END;
$$;



CREATE FUNCTION public.indexer_rss_item_seen_list(actor_user_public_id uuid, indexer_instance_public_id_input uuid, limit_input integer) RETURNS TABLE(item_guid character varying, infohash_v1 character, infohash_v2 character, magnet_hash character, first_seen_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_rss_item_seen_list_v1(
        actor_user_public_id,
        indexer_instance_public_id_input,
        limit_input
    );
END;
$$;



CREATE FUNCTION public.indexer_rss_item_seen_list_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, limit_input integer) RETURNS TABLE(item_guid character varying, infohash_v1 character, infohash_v2 character, magnet_hash character, first_seen_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to list RSS items';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    item_limit INTEGER;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT inst.indexer_instance_id, inst.deleted_at
    INTO instance_id, instance_deleted_at
    FROM indexer_instance inst
    WHERE inst.indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    item_limit := COALESCE(limit_input, 25);
    IF item_limit < 1 OR item_limit > 200 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'limit_out_of_range';
    END IF;

    RETURN QUERY
    SELECT
        seen.item_guid,
        seen.infohash_v1,
        seen.infohash_v2,
        seen.magnet_hash,
        seen.first_seen_at
    FROM indexer_rss_item_seen seen
    WHERE seen.indexer_instance_id = instance_id
    ORDER BY seen.first_seen_at DESC, seen.rss_item_seen_id DESC
    LIMIT item_limit;
END;
$$;



CREATE FUNCTION public.indexer_rss_item_seen_mark(actor_user_public_id uuid, indexer_instance_public_id_input uuid, item_guid_input character varying, infohash_v1_input character, infohash_v2_input character, magnet_hash_input character) RETURNS TABLE(item_guid character varying, infohash_v1 character, infohash_v2 character, magnet_hash character, first_seen_at timestamp with time zone, inserted boolean)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_rss_item_seen_mark_v1(
        actor_user_public_id,
        indexer_instance_public_id_input,
        item_guid_input,
        infohash_v1_input,
        infohash_v2_input,
        magnet_hash_input
    );
END;
$$;



CREATE FUNCTION public.indexer_rss_item_seen_mark_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, item_guid_input character varying, infohash_v1_input character, infohash_v2_input character, magnet_hash_input character) RETURNS TABLE(item_guid character varying, infohash_v1 character, infohash_v2 character, magnet_hash character, first_seen_at timestamp with time zone, inserted boolean)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    base_message CONSTANT text := 'Failed to mark RSS item seen';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    item_guid_value VARCHAR(256);
    infohash_v1_value CHAR(40);
    infohash_v2_value CHAR(64);
    magnet_hash_value CHAR(64);
    first_seen_value TIMESTAMPTZ;
    inserted_value BOOLEAN := FALSE;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT inst.indexer_instance_id, inst.deleted_at
    INTO instance_id, instance_deleted_at
    FROM indexer_instance inst
    WHERE inst.indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    item_guid_value := NULLIF(lower(trim(item_guid_input)), '');
    infohash_v1_value := NULLIF(lower(trim(infohash_v1_input)), '')::CHAR(40);
    infohash_v2_value := NULLIF(lower(trim(infohash_v2_input)), '')::CHAR(64);
    magnet_hash_value := NULLIF(lower(trim(magnet_hash_input)), '')::CHAR(64);

    IF item_guid_value IS NOT NULL AND char_length(item_guid_value) > 256 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'item_guid_too_long';
    END IF;

    IF infohash_v1_value IS NOT NULL AND infohash_v1_value !~ '^[0-9a-f]{40}$' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'infohash_v1_invalid';
    END IF;

    IF infohash_v2_value IS NOT NULL AND infohash_v2_value !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'infohash_v2_invalid';
    END IF;

    IF magnet_hash_value IS NOT NULL AND magnet_hash_value !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'magnet_hash_invalid';
    END IF;

    IF item_guid_value IS NULL
        AND infohash_v1_value IS NULL
        AND infohash_v2_value IS NULL
        AND magnet_hash_value IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'rss_item_identifier_missing';
    END IF;

    INSERT INTO indexer_rss_item_seen (
        indexer_instance_id,
        item_guid,
        infohash_v1,
        infohash_v2,
        magnet_hash,
        first_seen_at
    )
    VALUES (
        instance_id,
        item_guid_value,
        infohash_v1_value,
        infohash_v2_value,
        magnet_hash_value,
        now()
    )
    ON CONFLICT DO NOTHING
    RETURNING indexer_rss_item_seen.first_seen_at
    INTO first_seen_value;

    inserted_value := first_seen_value IS NOT NULL;

    IF inserted_value IS FALSE THEN
        SELECT seen.first_seen_at
        INTO first_seen_value
        FROM indexer_rss_item_seen seen
        WHERE seen.indexer_instance_id = instance_id
          AND (
              (item_guid_value IS NOT NULL AND seen.item_guid = item_guid_value)
              OR (infohash_v1_value IS NOT NULL AND seen.infohash_v1 = infohash_v1_value)
              OR (infohash_v2_value IS NOT NULL AND seen.infohash_v2 = infohash_v2_value)
              OR (magnet_hash_value IS NOT NULL AND seen.magnet_hash = magnet_hash_value)
          )
        ORDER BY seen.first_seen_at DESC, seen.rss_item_seen_id DESC
        LIMIT 1;
    END IF;

    RETURN QUERY
    SELECT
        item_guid_value,
        infohash_v1_value,
        infohash_v2_value,
        magnet_hash_value,
        first_seen_value,
        inserted_value;
END;
$_$;



CREATE FUNCTION public.indexer_rss_subscription_disable(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_rss_subscription_disable_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input);
END;
$$;



CREATE FUNCTION public.indexer_rss_subscription_disable_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to disable RSS subscription';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT indexer_instance_id, deleted_at
    INTO instance_id, instance_deleted_at
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    INSERT INTO indexer_rss_subscription (
        indexer_instance_id,
        is_enabled,
        interval_seconds,
        last_polled_at,
        next_poll_at,
        backoff_seconds,
        last_error_class
    )
    VALUES (
        instance_id,
        FALSE,
        900,
        NULL,
        NULL,
        NULL,
        NULL
    )
    ON CONFLICT (indexer_instance_id)
    DO UPDATE SET
        is_enabled = FALSE,
        next_poll_at = NULL;

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'indexer_instance',
        instance_id,
        indexer_instance_public_id_input,
        'update',
        actor_user_id,
        'rss_subscription_disable'
    );
END;
$$;



CREATE FUNCTION public.indexer_rss_subscription_get(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS TABLE(indexer_instance_public_id uuid, instance_is_enabled boolean, instance_enable_rss boolean, subscription_exists boolean, subscription_is_enabled boolean, interval_seconds integer, last_polled_at timestamp with time zone, next_poll_at timestamp with time zone, backoff_seconds integer, last_error_class public.error_class)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_rss_subscription_get_v1(
        actor_user_public_id,
        indexer_instance_public_id_input
    );
END;
$$;



CREATE FUNCTION public.indexer_rss_subscription_get_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid) RETURNS TABLE(indexer_instance_public_id uuid, instance_is_enabled boolean, instance_enable_rss boolean, subscription_exists boolean, subscription_is_enabled boolean, interval_seconds integer, last_polled_at timestamp with time zone, next_poll_at timestamp with time zone, backoff_seconds integer, last_error_class public.error_class)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to fetch RSS subscription';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT inst.indexer_instance_id, inst.deleted_at
    INTO instance_id, instance_deleted_at
    FROM indexer_instance inst
    WHERE inst.indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    RETURN QUERY
    SELECT
        inst.indexer_instance_public_id,
        inst.is_enabled,
        inst.enable_rss,
        sub.indexer_rss_subscription_id IS NOT NULL,
        COALESCE(sub.is_enabled, FALSE),
        COALESCE(sub.interval_seconds, 900),
        sub.last_polled_at,
        sub.next_poll_at,
        sub.backoff_seconds,
        sub.last_error_class
    FROM indexer_instance inst
    LEFT JOIN indexer_rss_subscription sub
        ON sub.indexer_instance_id = inst.indexer_instance_id
    WHERE inst.indexer_instance_id = instance_id;
END;
$$;



CREATE FUNCTION public.indexer_rss_subscription_set(actor_user_public_id uuid, indexer_instance_public_id_input uuid, is_enabled_input boolean, interval_seconds_input integer) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_rss_subscription_set_v1(actor_user_public_id => actor_user_public_id, indexer_instance_public_id_input => indexer_instance_public_id_input, is_enabled_input => is_enabled_input, interval_seconds_input => interval_seconds_input);
END;
$$;



CREATE FUNCTION public.indexer_rss_subscription_set_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, is_enabled_input boolean, interval_seconds_input integer) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to update RSS subscription';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    instance_is_enabled BOOLEAN;
    instance_enable_rss BOOLEAN;
    rss_row_exists BOOLEAN;
    rss_is_enabled BOOLEAN;
    rss_interval INTEGER;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    IF is_enabled_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'rss_enabled_missing';
    END IF;

    SELECT indexer_instance_id, deleted_at, is_enabled, enable_rss
    INTO instance_id, instance_deleted_at, instance_is_enabled, instance_enable_rss
    FROM indexer_instance
    WHERE indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    IF is_enabled_input AND (instance_is_enabled IS FALSE OR instance_enable_rss IS FALSE) THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'rss_enable_indexer_disabled';
    END IF;

    IF interval_seconds_input IS NOT NULL THEN
        IF interval_seconds_input < 300 OR interval_seconds_input > 86400 THEN
            RAISE EXCEPTION USING
                ERRCODE = errcode,
                MESSAGE = base_message,
                DETAIL = 'interval_out_of_range';
        END IF;
    END IF;

    SELECT EXISTS (
        SELECT 1
        FROM indexer_rss_subscription
        WHERE indexer_instance_id = instance_id
    ) INTO rss_row_exists;

    IF rss_row_exists THEN
        SELECT is_enabled, interval_seconds
        INTO rss_is_enabled, rss_interval
        FROM indexer_rss_subscription
        WHERE indexer_instance_id = instance_id;

        IF is_enabled_input THEN
            IF rss_is_enabled THEN
                UPDATE indexer_rss_subscription
                SET interval_seconds = COALESCE(interval_seconds_input, interval_seconds)
                WHERE indexer_instance_id = instance_id;
            ELSE
                UPDATE indexer_rss_subscription
                SET is_enabled = TRUE,
                    interval_seconds = COALESCE(interval_seconds_input, interval_seconds),
                    last_error_class = NULL,
                    backoff_seconds = NULL,
                    next_poll_at = now() + make_interval(secs => random_jitter_seconds(60))
                WHERE indexer_instance_id = instance_id;
            END IF;
        ELSE
            UPDATE indexer_rss_subscription
            SET is_enabled = FALSE,
                interval_seconds = COALESCE(interval_seconds_input, interval_seconds),
                next_poll_at = NULL
            WHERE indexer_instance_id = instance_id;
        END IF;
    ELSE
        INSERT INTO indexer_rss_subscription (
            indexer_instance_id,
            is_enabled,
            interval_seconds,
            last_polled_at,
            next_poll_at,
            backoff_seconds,
            last_error_class
        )
        VALUES (
            instance_id,
            is_enabled_input,
            COALESCE(interval_seconds_input, 900),
            NULL,
            CASE
                WHEN is_enabled_input THEN now() + make_interval(secs => random_jitter_seconds(60))
                ELSE NULL
            END,
            NULL,
            NULL
        );
    END IF;

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'indexer_instance',
        instance_id,
        indexer_instance_public_id_input,
        'update',
        actor_user_id,
        'rss_subscription_set'
    );
END;
$$;



CREATE FUNCTION public.indexer_search_profile_list(actor_user_public_id uuid) RETURNS TABLE(search_profile_public_id uuid, display_name character varying, is_default boolean, page_size integer, default_media_domain_key public.media_domain_key, media_domain_keys character varying[], policy_set_public_ids uuid[], policy_set_display_names character varying[], allow_indexer_public_ids uuid[], block_indexer_public_ids uuid[], allow_tag_keys character varying[], block_tag_keys character varying[], prefer_tag_keys character varying[])
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT *
    FROM indexer_search_profile_list_v1(actor_user_public_id);
$$;



CREATE FUNCTION public.indexer_search_profile_list_v1(actor_user_public_id uuid) RETURNS TABLE(search_profile_public_id uuid, display_name character varying, is_default boolean, page_size integer, default_media_domain_key public.media_domain_key, media_domain_keys character varying[], policy_set_public_ids uuid[], policy_set_display_names character varying[], allow_indexer_public_ids uuid[], block_indexer_public_ids uuid[], allow_tag_keys character varying[], block_tag_keys character varying[], prefer_tag_keys character varying[])
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_backup_assert_actor_v1(actor_user_public_id);

    RETURN QUERY
    SELECT
        profile.search_profile_public_id,
        profile.display_name,
        profile.is_default,
        profile.page_size,
        default_domain.media_domain_key,
        COALESCE((
            SELECT array_agg(domain.media_domain_key::VARCHAR ORDER BY domain.media_domain_key)
            FROM search_profile_media_domain mapping
            INNER JOIN media_domain domain
                ON domain.media_domain_id = mapping.media_domain_id
            WHERE mapping.search_profile_id = profile.search_profile_id
        ), ARRAY[]::VARCHAR[]),
        COALESCE((
            SELECT array_agg(policy.policy_set_public_id ORDER BY policy.display_name)
            FROM search_profile_policy_set mapping
            INNER JOIN policy_set policy
                ON policy.policy_set_id = mapping.policy_set_id
               AND policy.deleted_at IS NULL
            WHERE mapping.search_profile_id = profile.search_profile_id
        ), ARRAY[]::UUID[]),
        COALESCE((
            SELECT array_agg(policy.display_name ORDER BY policy.display_name)
            FROM search_profile_policy_set mapping
            INNER JOIN policy_set policy
                ON policy.policy_set_id = mapping.policy_set_id
               AND policy.deleted_at IS NULL
            WHERE mapping.search_profile_id = profile.search_profile_id
        ), ARRAY[]::VARCHAR[]),
        COALESCE((
            SELECT array_agg(instance.indexer_instance_public_id ORDER BY instance.display_name)
            FROM search_profile_indexer_allow mapping
            INNER JOIN indexer_instance instance
                ON instance.indexer_instance_id = mapping.indexer_instance_id
               AND instance.deleted_at IS NULL
            WHERE mapping.search_profile_id = profile.search_profile_id
        ), ARRAY[]::UUID[]),
        COALESCE((
            SELECT array_agg(instance.indexer_instance_public_id ORDER BY instance.display_name)
            FROM search_profile_indexer_block mapping
            INNER JOIN indexer_instance instance
                ON instance.indexer_instance_id = mapping.indexer_instance_id
               AND instance.deleted_at IS NULL
            WHERE mapping.search_profile_id = profile.search_profile_id
        ), ARRAY[]::UUID[]),
        COALESCE((
            SELECT array_agg(tag.tag_key ORDER BY tag.tag_key)
            FROM search_profile_tag_allow mapping
            INNER JOIN tag
                ON tag.tag_id = mapping.tag_id
               AND tag.deleted_at IS NULL
            WHERE mapping.search_profile_id = profile.search_profile_id
        ), ARRAY[]::VARCHAR[]),
        COALESCE((
            SELECT array_agg(tag.tag_key ORDER BY tag.tag_key)
            FROM search_profile_tag_block mapping
            INNER JOIN tag
                ON tag.tag_id = mapping.tag_id
               AND tag.deleted_at IS NULL
            WHERE mapping.search_profile_id = profile.search_profile_id
        ), ARRAY[]::VARCHAR[]),
        COALESCE((
            SELECT array_agg(tag.tag_key ORDER BY tag.tag_key)
            FROM search_profile_tag_prefer mapping
            INNER JOIN tag
                ON tag.tag_id = mapping.tag_id
               AND tag.deleted_at IS NULL
            WHERE mapping.search_profile_id = profile.search_profile_id
        ), ARRAY[]::VARCHAR[])
    FROM search_profile profile
    LEFT JOIN media_domain default_domain
        ON default_domain.media_domain_id = profile.default_media_domain_id
    WHERE profile.deleted_at IS NULL
    ORDER BY profile.display_name, profile.search_profile_public_id;
END;
$$;



CREATE FUNCTION public.indexer_source_reputation_list(actor_user_public_id uuid, indexer_instance_public_id_input uuid, window_key_input public.reputation_window, limit_input integer) RETURNS TABLE(window_key public.reputation_window, window_start timestamp with time zone, request_success_rate numeric, acquisition_success_rate numeric, fake_rate numeric, dmca_rate numeric, request_count integer, request_success_count integer, acquisition_count integer, acquisition_success_count integer, min_samples integer, computed_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    SELECT *
    FROM indexer_source_reputation_list_v1(
        actor_user_public_id,
        indexer_instance_public_id_input,
        window_key_input,
        limit_input
    );
END;
$$;



CREATE FUNCTION public.indexer_source_reputation_list_v1(actor_user_public_id uuid, indexer_instance_public_id_input uuid, window_key_input public.reputation_window, limit_input integer) RETURNS TABLE(window_key public.reputation_window, window_start timestamp with time zone, request_success_rate numeric, acquisition_success_rate numeric, fake_rate numeric, dmca_rate numeric, request_count integer, request_success_count integer, acquisition_count integer, acquisition_success_count integer, min_samples integer, computed_at timestamp with time zone)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to list source reputation';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    instance_id BIGINT;
    instance_deleted_at TIMESTAMPTZ;
    reputation_limit INTEGER;
    resolved_window reputation_window;
BEGIN
    IF actor_user_public_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_missing';
    END IF;

    SELECT user_id, role
    INTO actor_user_id, actor_role
    FROM app_user
    WHERE user_public_id = actor_user_public_id;

    IF actor_user_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_not_found';
    END IF;

    IF actor_role NOT IN ('owner', 'admin') THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'actor_unauthorized';
    END IF;

    IF indexer_instance_public_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_missing';
    END IF;

    SELECT inst.indexer_instance_id, inst.deleted_at
    INTO instance_id, instance_deleted_at
    FROM indexer_instance inst
    WHERE inst.indexer_instance_public_id = indexer_instance_public_id_input;

    IF instance_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_not_found';
    END IF;

    IF instance_deleted_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'indexer_deleted';
    END IF;

    resolved_window := COALESCE(window_key_input, '1h'::reputation_window);
    reputation_limit := COALESCE(limit_input, 10);
    IF reputation_limit < 1 OR reputation_limit > 100 THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'limit_out_of_range';
    END IF;

    RETURN QUERY
    SELECT
        reputation.window_key,
        reputation.window_start,
        reputation.request_success_rate,
        reputation.acquisition_success_rate,
        reputation.fake_rate,
        reputation.dmca_rate,
        reputation.request_count,
        reputation.request_success_count,
        reputation.acquisition_count,
        reputation.acquisition_success_count,
        reputation.min_samples,
        reputation.computed_at
    FROM source_reputation reputation
    WHERE reputation.indexer_instance_id = instance_id
      AND reputation.window_key = resolved_window
    ORDER BY reputation.window_start DESC
    LIMIT reputation_limit;
END;
$$;



CREATE FUNCTION public.indexer_torznab_instance_list(actor_user_public_id uuid) RETURNS TABLE(torznab_instance_public_id uuid, display_name character varying, is_enabled boolean, search_profile_public_id uuid, search_profile_display_name character varying)
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT *
    FROM indexer_torznab_instance_list_v1(actor_user_public_id);
$$;



CREATE FUNCTION public.indexer_torznab_instance_list_v1(actor_user_public_id uuid) RETURNS TABLE(torznab_instance_public_id uuid, display_name character varying, is_enabled boolean, search_profile_public_id uuid, search_profile_display_name character varying)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM indexer_backup_assert_actor_v1(actor_user_public_id);

    RETURN QUERY
    SELECT
        instance.torznab_instance_public_id,
        instance.display_name,
        instance.is_enabled,
        profile.search_profile_public_id,
        profile.display_name
    FROM torznab_instance instance
    INNER JOIN search_profile profile
        ON profile.search_profile_id = instance.search_profile_id
       AND profile.deleted_at IS NULL
    WHERE instance.deleted_at IS NULL
    ORDER BY instance.display_name, instance.torznab_instance_public_id;
END;
$$;



CREATE FUNCTION public.job_claim_lease_seconds(job_key_input public.job_key) RETURNS integer
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN job_claim_lease_seconds_v1(job_key_input => job_key_input);
END;
$$;



CREATE FUNCTION public.job_claim_lease_seconds_v1(job_key_input public.job_key) RETURNS integer
    LANGUAGE sql IMMUTABLE STRICT
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT CASE job_key_input
        WHEN 'connectivity_profile_refresh' THEN 30
        WHEN 'reputation_rollup_1h' THEN 60
        WHEN 'reputation_rollup_24h' THEN 300
        WHEN 'reputation_rollup_7d' THEN 600
        WHEN 'retention_purge' THEN 300
        WHEN 'canonical_backfill_best_source' THEN 900
        WHEN 'base_score_refresh_recent' THEN 900
        WHEN 'canonical_prune_low_confidence' THEN 900
        WHEN 'policy_snapshot_gc' THEN 900
        WHEN 'policy_snapshot_refcount_repair' THEN 900
        WHEN 'rate_limit_state_purge' THEN 300
        WHEN 'rss_poll' THEN 60
        WHEN 'rss_subscription_backfill' THEN 300
        ELSE 300
    END;
$$;



CREATE FUNCTION public.job_claim_next(job_key_input public.job_key) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_claim_next_v1(job_key_input => job_key_input);
END;
$$;



CREATE FUNCTION public.job_claim_next_v1(job_key_input public.job_key) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
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
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'job_locked';
    END IF;

    SELECT job_schedule_id, enabled, next_run_at, locked_until
    INTO schedule_id, schedule_enabled, schedule_next_run, schedule_locked_until
    FROM job_schedule
    WHERE job_key = job_key_input
    FOR UPDATE;

    IF schedule_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'job_not_found';
    END IF;

    IF schedule_enabled IS NOT TRUE THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'job_disabled';
    END IF;

    now_ts := now();

    IF schedule_next_run > now_ts THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'job_not_due';
    END IF;

    IF schedule_locked_until IS NOT NULL AND schedule_locked_until > now_ts THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'job_locked';
    END IF;

    UPDATE job_schedule
    SET locked_until = now_ts + make_interval(secs => job_claim_lease_seconds_v1(job_key_input)),
        lock_owner = current_user
    WHERE job_schedule_id = schedule_id;
END;
$$;



CREATE FUNCTION public.job_run_base_score_refresh_recent() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_base_score_refresh_recent_v2();
END;
$$;



CREATE FUNCTION public.job_run_base_score_refresh_recent_v1() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    cutoff_recent TIMESTAMPTZ;
    canonical_public_id UUID;
BEGIN
    cutoff_recent := now() - make_interval(days => 7);

    WITH candidate_pairs AS (
        SELECT DISTINCT cs.canonical_torrent_id,
                        cs.canonical_torrent_source_id
        FROM canonical_torrent_source_context_score cs
        JOIN canonical_torrent_source s
            ON s.canonical_torrent_source_id = cs.canonical_torrent_source_id
        WHERE s.last_seen_at >= cutoff_recent
        UNION
        SELECT DISTINCT bs.canonical_torrent_id,
                        bs.canonical_torrent_source_id
        FROM canonical_torrent_source_base_score bs
        JOIN canonical_torrent_source s
            ON s.canonical_torrent_source_id = bs.canonical_torrent_source_id
        WHERE s.last_seen_at >= cutoff_recent
    ),
    domain_map AS (
        SELECT imd.indexer_instance_id,
               CASE WHEN COUNT(*) = 1 THEN MAX(md.media_domain_key) ELSE NULL END AS media_domain_key
        FROM indexer_instance_media_domain imd
        JOIN media_domain md
            ON md.media_domain_id = imd.media_domain_id
        GROUP BY imd.indexer_instance_id
    ),
    reputation_latest AS (
        SELECT DISTINCT ON (indexer_instance_id)
               indexer_instance_id,
               request_success_rate,
               acquisition_success_rate,
               request_count,
               acquisition_count
        FROM source_reputation
        WHERE window_key = '24h'
        ORDER BY indexer_instance_id, window_start DESC
    ),
    scored AS (
        SELECT p.canonical_torrent_id,
               s.canonical_torrent_source_id,
               s.last_seen_seeders,
               s.last_seen_leechers,
               s.last_seen_published_at,
               s.last_seen_at,
               COALESCE(t.default_weight, 0) AS score_trust,
               d.media_domain_key,
               cp.status,
               cp.error_class,
               cp.latency_p95_ms,
               COALESCE(r.request_success_rate, 0) AS request_success_rate,
               COALESCE(r.acquisition_success_rate, 0) AS acquisition_success_rate,
               COALESCE(r.request_count, 0) AS request_count,
               COALESCE(r.acquisition_count, 0) AS acquisition_count
        FROM candidate_pairs p
        JOIN canonical_torrent_source s
            ON s.canonical_torrent_source_id = p.canonical_torrent_source_id
        JOIN indexer_instance i
            ON i.indexer_instance_id = s.indexer_instance_id
        LEFT JOIN trust_tier t
            ON t.trust_tier_key = i.trust_tier_key
        LEFT JOIN domain_map d
            ON d.indexer_instance_id = i.indexer_instance_id
        LEFT JOIN indexer_connectivity_profile cp
            ON cp.indexer_instance_id = i.indexer_instance_id
        LEFT JOIN reputation_latest r
            ON r.indexer_instance_id = i.indexer_instance_id
    ),
    weighted AS (
        SELECT s.*,
               CASE s.media_domain_key
                   WHEN 'movies' THEN 1.0
                   WHEN 'tv' THEN 3.0
                   WHEN 'audiobooks' THEN 0.5
                   WHEN 'ebooks' THEN 0.5
                   WHEN 'software' THEN 0.75
                   WHEN 'adult_movies' THEN 1.5
                   WHEN 'adult_scenes' THEN 2.0
                   ELSE 1.0
               END AS media_domain_weight,
               CASE WHEN s.last_seen_published_at IS NULL THEN 0.5 ELSE 1.0 END AS age_multiplier,
               COALESCE(s.last_seen_published_at, s.last_seen_at) AS age_reference
        FROM scored s
    ),
    computed AS (
        SELECT w.canonical_torrent_id,
               w.canonical_torrent_source_id,
               CASE
                   WHEN w.last_seen_seeders IS NULL THEN 0
                   ELSE ln(1 + GREATEST(w.last_seen_seeders, 0)) * 10.0
               END AS score_seed,
               CASE
                   WHEN w.last_seen_leechers IS NULL THEN 0
                   ELSE ln(1 + GREATEST(w.last_seen_leechers, 0)) * 2.0
               END AS score_leech,
               CASE
                   WHEN w.age_reference IS NULL THEN 0
                   WHEN now() - w.age_reference < make_interval(hours => 6)
                   THEN 6 * w.media_domain_weight * w.age_multiplier
                   WHEN now() - w.age_reference < make_interval(hours => 24)
                   THEN 4 * w.media_domain_weight * w.age_multiplier
                   WHEN now() - w.age_reference < make_interval(hours => 72)
                   THEN 2 * w.media_domain_weight * w.age_multiplier
                   WHEN now() - w.age_reference < make_interval(days => 14)
                   THEN 1 * w.media_domain_weight * w.age_multiplier
                   ELSE 0
               END AS score_age,
               w.score_trust,
               CASE
                   WHEN w.status = 'quarantined' THEN
                       -1000
                       + (CASE WHEN w.last_seen_seeders IS NULL THEN -0.5 ELSE 0 END)
                       + (CASE WHEN w.last_seen_leechers IS NULL THEN -0.5 ELSE 0 END)
                   ELSE
                       (CASE
                           WHEN w.latency_p95_ms IS NULL THEN 0
                           WHEN w.latency_p95_ms <= 500 THEN 0
                           WHEN w.latency_p95_ms <= 1500 THEN -2
                           WHEN w.latency_p95_ms <= 4000 THEN -5
                           ELSE -10
                       END)
                       + (CASE
                           WHEN w.error_class = 'http_429' THEN -8
                           WHEN w.error_class = 'timeout' THEN -10
                           WHEN w.error_class = 'cf_challenge' THEN -12
                           WHEN w.error_class = 'auth_error' THEN -15
                           WHEN w.error_class = 'http_403' THEN -10
                           WHEN w.error_class IN ('tls', 'dns', 'connection_refused') THEN -12
                           WHEN w.error_class = 'parse_error' THEN -8
                           WHEN w.error_class = 'http_5xx' THEN -6
                           WHEN w.error_class = 'unknown' THEN -5
                           ELSE 0
                       END)
                       + (CASE WHEN w.last_seen_seeders IS NULL THEN -0.5 ELSE 0 END)
                       + (CASE WHEN w.last_seen_leechers IS NULL THEN -0.5 ELSE 0 END)
               END AS score_health,
               CASE
                   WHEN w.acquisition_count >= 10 THEN
                       GREATEST(LEAST((w.acquisition_success_rate - 0.5) * 10, 5), -5)
                   WHEN w.request_count >= 30 THEN
                       GREATEST(LEAST((w.request_success_rate - 0.5) * 10, 5), -5)
                   ELSE
                       0
               END AS score_reputation
        FROM weighted w
    ),
    totals AS (
        SELECT c.canonical_torrent_id,
               c.canonical_torrent_source_id,
               c.score_seed,
               c.score_leech,
               c.score_age,
               c.score_trust,
               c.score_health,
               c.score_reputation,
               LEAST(10000, GREATEST(-10000,
                   c.score_seed + c.score_leech + c.score_age
                   + c.score_trust + c.score_health + c.score_reputation
               )) AS score_total_base
        FROM computed c
    )
    INSERT INTO canonical_torrent_source_base_score (
        canonical_torrent_id,
        canonical_torrent_source_id,
        score_total_base,
        score_seed,
        score_leech,
        score_age,
        score_trust,
        score_health,
        score_reputation,
        computed_at
    )
    SELECT t.canonical_torrent_id,
           t.canonical_torrent_source_id,
           round(t.score_total_base::NUMERIC, 4),
           t.score_seed,
           t.score_leech,
           t.score_age,
           t.score_trust,
           t.score_health,
           t.score_reputation,
           now()
    FROM totals t
    ON CONFLICT (canonical_torrent_id, canonical_torrent_source_id)
    DO UPDATE SET
        score_total_base = EXCLUDED.score_total_base,
        score_seed = EXCLUDED.score_seed,
        score_leech = EXCLUDED.score_leech,
        score_age = EXCLUDED.score_age,
        score_trust = EXCLUDED.score_trust,
        score_health = EXCLUDED.score_health,
        score_reputation = EXCLUDED.score_reputation,
        computed_at = EXCLUDED.computed_at;

    FOR canonical_public_id IN
        SELECT c.canonical_torrent_public_id
        FROM canonical_torrent c
        WHERE EXISTS (
            SELECT 1
            FROM canonical_torrent_source_context_score cs
            JOIN canonical_torrent_source s
                ON s.canonical_torrent_source_id = cs.canonical_torrent_source_id
            WHERE cs.canonical_torrent_id = c.canonical_torrent_id
              AND s.last_seen_at >= cutoff_recent
            UNION
            SELECT 1
            FROM canonical_torrent_source_base_score bs
            JOIN canonical_torrent_source s
                ON s.canonical_torrent_source_id = bs.canonical_torrent_source_id
            WHERE bs.canonical_torrent_id = c.canonical_torrent_id
              AND s.last_seen_at >= cutoff_recent
        )
    LOOP
        PERFORM canonical_recompute_best_source_v1(canonical_public_id, 'global_current');
    END LOOP;
END;
$$;



CREATE FUNCTION public.job_run_base_score_refresh_recent_v2() RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    BEGIN
        PERFORM job_run_base_score_refresh_recent_v1();
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1('base_score_refresh_recent');

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_run_canonical_backfill_best_source() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_canonical_backfill_best_source_v2();
END;
$$;



CREATE FUNCTION public.job_run_canonical_backfill_best_source_v1() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    cutoff_recent TIMESTAMPTZ;
    canonical_public_id UUID;
BEGIN
    cutoff_recent := now() - make_interval(days => 7);

    FOR canonical_public_id IN
        WITH recent_canonical AS (
            SELECT DISTINCT cs.canonical_torrent_id
            FROM canonical_torrent_source_context_score cs
            JOIN canonical_torrent_source s
                ON s.canonical_torrent_source_id = cs.canonical_torrent_source_id
            WHERE s.last_seen_at >= cutoff_recent
            UNION
            SELECT DISTINCT bs.canonical_torrent_id
            FROM canonical_torrent_source_base_score bs
            JOIN canonical_torrent_source s
                ON s.canonical_torrent_source_id = bs.canonical_torrent_source_id
            WHERE s.last_seen_at >= cutoff_recent
        )
        SELECT c.canonical_torrent_public_id
        FROM canonical_torrent c
        LEFT JOIN canonical_torrent_best_source_global b
            ON b.canonical_torrent_id = c.canonical_torrent_id
        LEFT JOIN recent_canonical r
            ON r.canonical_torrent_id = c.canonical_torrent_id
        WHERE b.canonical_torrent_id IS NULL
           OR r.canonical_torrent_id IS NOT NULL
           OR (c.identity_strategy = 'title_size_fallback' AND c.identity_confidence <= 0.60)
    LOOP
        PERFORM canonical_recompute_best_source_v1(canonical_public_id, 'global_current');
    END LOOP;
END;
$$;



CREATE FUNCTION public.job_run_canonical_backfill_best_source_v2() RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    BEGIN
        PERFORM job_run_canonical_backfill_best_source_v1();
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1('canonical_backfill_best_source');

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_run_canonical_prune_low_confidence() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_canonical_prune_low_confidence_v2();
END;
$$;



CREATE FUNCTION public.job_run_canonical_prune_low_confidence_v2() RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    BEGIN
        PERFORM canonical_prune_low_confidence_v1();
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1('canonical_prune_low_confidence');

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_run_connectivity_profile_refresh() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_connectivity_profile_refresh_v2();
END;
$$;



CREATE FUNCTION public.job_run_connectivity_profile_refresh_v1() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    now_ts TIMESTAMPTZ;
BEGIN
    now_ts := now();

    WITH indexer_scope AS (
        SELECT indexer_instance_id
        FROM indexer_instance
        WHERE deleted_at IS NULL
    ),
    samples_1h AS (
        SELECT indexer_instance_id,
               COUNT(*) AS total_samples,
               COUNT(*) FILTER (WHERE outcome = 'success' AND parse_ok = TRUE) AS success_count,
               COUNT(*) FILTER (WHERE outcome = 'failure') AS failure_count,
               COUNT(*) FILTER (WHERE error_class = 'http_429') AS http_429_count,
               COUNT(latency_ms) FILTER (WHERE latency_ms IS NOT NULL) AS latency_count,
               percentile_cont(0.5) WITHIN GROUP (ORDER BY latency_ms) AS latency_p50,
               percentile_cont(0.95) WITHIN GROUP (ORDER BY latency_ms) AS latency_p95
        FROM outbound_request_log
        WHERE request_type IN ('caps', 'search', 'tvsearch', 'moviesearch', 'rss', 'probe')
          AND finished_at >= now_ts - make_interval(hours => 1)
          AND error_class IS DISTINCT FROM 'rate_limited'
        GROUP BY indexer_instance_id
    ),
    samples_24h AS (
        SELECT indexer_instance_id,
               COUNT(*) AS total_samples,
               COUNT(*) FILTER (WHERE outcome = 'success' AND parse_ok = TRUE) AS success_count
        FROM outbound_request_log
        WHERE request_type IN ('caps', 'search', 'tvsearch', 'moviesearch', 'rss', 'probe')
          AND finished_at >= now_ts - make_interval(hours => 24)
          AND error_class IS DISTINCT FROM 'rate_limited'
        GROUP BY indexer_instance_id
    ),
    failures_1h AS (
        SELECT indexer_instance_id,
               error_class,
               COUNT(*) AS failure_count
        FROM outbound_request_log
        WHERE request_type IN ('caps', 'search', 'tvsearch', 'moviesearch', 'rss', 'probe')
          AND finished_at >= now_ts - make_interval(hours => 1)
          AND outcome = 'failure'
          AND error_class IS DISTINCT FROM 'rate_limited'
        GROUP BY indexer_instance_id, error_class
    ),
    dominant_failures AS (
        SELECT DISTINCT ON (indexer_instance_id)
               indexer_instance_id,
               error_class,
               failure_count
        FROM failures_1h
        ORDER BY indexer_instance_id, failure_count DESC, error_class ASC
    ),
    burst_10m AS (
        SELECT indexer_instance_id,
               COUNT(*) AS total_samples,
               COUNT(*) FILTER (WHERE error_class = 'http_429') AS http_429_count
        FROM outbound_request_log
        WHERE request_type IN ('caps', 'search', 'tvsearch', 'moviesearch', 'rss', 'probe')
          AND finished_at >= now_ts - make_interval(mins => 10)
          AND error_class IS DISTINCT FROM 'rate_limited'
        GROUP BY indexer_instance_id
    ),
    combined AS (
        SELECT s.indexer_instance_id,
               s1.total_samples AS total_samples_1h,
               s1.success_count AS success_count_1h,
               s1.failure_count AS failure_count_1h,
               s1.http_429_count AS http_429_count_1h,
               s1.latency_count,
               s1.latency_p50,
               s1.latency_p95,
               s24.total_samples AS total_samples_24h,
               s24.success_count AS success_count_24h,
               df.error_class AS dominant_error_class,
               df.failure_count AS dominant_failure_count,
               b10.total_samples AS total_samples_10m,
               b10.http_429_count AS http_429_count_10m
        FROM indexer_scope s
        LEFT JOIN samples_1h s1
            ON s1.indexer_instance_id = s.indexer_instance_id
        LEFT JOIN samples_24h s24
            ON s24.indexer_instance_id = s.indexer_instance_id
        LEFT JOIN dominant_failures df
            ON df.indexer_instance_id = s.indexer_instance_id
        LEFT JOIN burst_10m b10
            ON b10.indexer_instance_id = s.indexer_instance_id
    ),
    derived AS (
        SELECT c.indexer_instance_id,
               CASE
                   WHEN c.total_samples_1h IS NULL OR c.total_samples_1h = 0 THEN NULL
                   ELSE c.success_count_1h::NUMERIC / c.total_samples_1h
               END AS success_rate_1h,
               CASE
                   WHEN c.total_samples_24h IS NULL OR c.total_samples_24h = 0 THEN NULL
                   ELSE c.success_count_24h::NUMERIC / c.total_samples_24h
               END AS success_rate_24h,
               CASE
                   WHEN c.latency_count IS NULL OR c.latency_count < 5 THEN 0
                   WHEN c.latency_count < 20 THEN COALESCE(c.latency_p50, 0)
                   ELSE COALESCE(c.latency_p95, 0)
               END AS effective_latency_ms,
               c.latency_p50,
               c.latency_p95,
               CASE
                   WHEN c.dominant_failure_count IS NOT NULL
                        AND c.total_samples_1h IS NOT NULL
                        AND c.dominant_failure_count >= 5
                        AND c.dominant_failure_count >= (c.total_samples_1h * 0.2)
                   THEN c.dominant_error_class
                   ELSE NULL
               END AS dominant_error_class,
               CASE
                   WHEN c.http_429_count_10m IS NOT NULL
                        AND c.http_429_count_10m >= 10 THEN TRUE
                   WHEN c.http_429_count_10m IS NOT NULL
                        AND c.total_samples_10m IS NOT NULL
                        AND c.total_samples_10m >= 20
                        AND (c.http_429_count_10m::NUMERIC / c.total_samples_10m) >= 0.3
                   THEN TRUE
                   ELSE FALSE
               END AS http_429_burst
        FROM combined c
    ),
    scored AS (
        SELECT d.indexer_instance_id,
               d.success_rate_1h,
               d.success_rate_24h,
               d.latency_p50,
               d.latency_p95,
               d.dominant_error_class,
               d.http_429_burst,
               CASE
                   WHEN d.success_rate_1h IS NULL THEN 'degraded'::connectivity_status
                   WHEN d.success_rate_1h >= 0.98 AND d.effective_latency_ms <= 1500 THEN 'healthy'::connectivity_status
                   WHEN d.success_rate_1h < 0.90 THEN 'failing'::connectivity_status
                   WHEN d.dominant_error_class IN ('auth_error', 'cf_challenge', 'tls', 'dns') THEN 'failing'::connectivity_status
                   WHEN d.success_rate_1h >= 0.90 OR d.effective_latency_ms <= 4000 THEN 'degraded'::connectivity_status
                   ELSE 'failing'::connectivity_status
               END AS base_status
        FROM derived d
    ),
    with_prev AS (
        SELECT s.indexer_instance_id,
               s.success_rate_1h,
               s.success_rate_24h,
               s.latency_p50,
               s.latency_p95,
               s.dominant_error_class,
               s.http_429_burst,
               s.base_status,
               p.status AS prev_status,
               p.error_class AS prev_error_class,
               p.last_checked_at AS prev_checked_at
        FROM scored s
        LEFT JOIN indexer_connectivity_profile p
            ON p.indexer_instance_id = s.indexer_instance_id
    ),
    status_resolved AS (
        SELECT w.indexer_instance_id,
               CASE
                   WHEN w.prev_status = 'quarantined'
                        AND w.prev_checked_at >= now_ts - make_interval(mins => 30)
                   THEN 'quarantined'::connectivity_status
                   WHEN w.prev_status = 'quarantined'
                        AND w.base_status = 'healthy'
                   THEN 'degraded'::connectivity_status
                   WHEN w.base_status = 'failing'
                        AND (w.dominant_error_class IN ('cf_challenge', 'auth_error') OR w.http_429_burst)
                        AND w.prev_status IN ('failing', 'quarantined')
                        AND w.prev_checked_at <= now_ts - make_interval(mins => 30)
                   THEN 'quarantined'::connectivity_status
                   ELSE w.base_status
               END AS status,
               w.dominant_error_class,
               w.http_429_burst,
               w.prev_error_class,
               w.success_rate_1h,
               w.success_rate_24h,
               w.latency_p50,
               w.latency_p95
        FROM with_prev w
    ),
    final AS (
        SELECT s.indexer_instance_id,
               s.status,
               CASE
                   WHEN s.status = 'healthy' THEN NULL
                   WHEN s.dominant_error_class IS NOT NULL THEN s.dominant_error_class
                   WHEN s.http_429_burst THEN 'http_429'::error_class
                   WHEN s.prev_error_class IS NOT NULL THEN s.prev_error_class
                   ELSE 'unknown'::error_class
               END AS error_class,
               s.success_rate_1h,
               s.success_rate_24h,
               s.latency_p50,
               s.latency_p95
        FROM status_resolved s
    )
    INSERT INTO indexer_connectivity_profile (
        indexer_instance_id,
        status,
        error_class,
        latency_p50_ms,
        latency_p95_ms,
        success_rate_1h,
        success_rate_24h,
        last_checked_at
    )
    SELECT f.indexer_instance_id,
           f.status,
           CASE WHEN f.status = 'healthy' THEN NULL ELSE f.error_class END,
           CASE WHEN f.latency_p50 IS NULL THEN NULL ELSE f.latency_p50::INTEGER END,
           CASE WHEN f.latency_p95 IS NULL THEN NULL ELSE f.latency_p95::INTEGER END,
           f.success_rate_1h,
           f.success_rate_24h,
           now_ts
    FROM final f
    ON CONFLICT (indexer_instance_id)
    DO UPDATE SET
        status = EXCLUDED.status,
        error_class = EXCLUDED.error_class,
        latency_p50_ms = EXCLUDED.latency_p50_ms,
        latency_p95_ms = EXCLUDED.latency_p95_ms,
        success_rate_1h = EXCLUDED.success_rate_1h,
        success_rate_24h = EXCLUDED.success_rate_24h,
        last_checked_at = EXCLUDED.last_checked_at;
END;
$$;



CREATE FUNCTION public.job_run_connectivity_profile_refresh_v2() RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    BEGIN
        PERFORM job_run_connectivity_profile_refresh_v1();
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1('connectivity_profile_refresh');

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_run_policy_snapshot_gc() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_policy_snapshot_gc_v2();
END;
$$;



CREATE FUNCTION public.job_run_policy_snapshot_gc_v1() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    DELETE FROM policy_snapshot
    WHERE ref_count = 0
      AND created_at < now() - make_interval(days => 30);
END;
$$;



CREATE FUNCTION public.job_run_policy_snapshot_gc_v2() RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    BEGIN
        PERFORM job_run_policy_snapshot_refcount_repair_v1();
        PERFORM job_run_policy_snapshot_gc_v1();
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1('policy_snapshot_gc');

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_run_policy_snapshot_refcount_repair() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_policy_snapshot_refcount_repair_v2();
END;
$$;



CREATE FUNCTION public.job_run_policy_snapshot_refcount_repair_v1() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    WITH counts AS (
        SELECT policy_snapshot_id, COUNT(*) AS request_count
        FROM search_request
        GROUP BY policy_snapshot_id
    )
    UPDATE policy_snapshot
    SET ref_count = counts.request_count
    FROM counts
    WHERE policy_snapshot.policy_snapshot_id = counts.policy_snapshot_id
      AND policy_snapshot.ref_count IS DISTINCT FROM counts.request_count;

    UPDATE policy_snapshot
    SET ref_count = 0
    WHERE NOT EXISTS (
        SELECT 1
        FROM search_request
        WHERE search_request.policy_snapshot_id = policy_snapshot.policy_snapshot_id
    )
      AND ref_count <> 0;
END;
$$;



CREATE FUNCTION public.job_run_policy_snapshot_refcount_repair_v2() RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    BEGIN
        PERFORM job_run_policy_snapshot_refcount_repair_v1();
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1('policy_snapshot_refcount_repair');

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_run_rate_limit_state_purge() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_rate_limit_state_purge_v2();
END;
$$;



CREATE FUNCTION public.job_run_rate_limit_state_purge_v1() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    DELETE FROM rate_limit_state
    WHERE window_start < now() - make_interval(hours => 6);
END;
$$;



CREATE FUNCTION public.job_run_rate_limit_state_purge_v2() RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    BEGIN
        PERFORM job_run_rate_limit_state_purge_v1();
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1('rate_limit_state_purge');

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_run_reputation_rollup(window_key_input public.reputation_window) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_reputation_rollup_v2(window_key_input => window_key_input);
END;
$$;



CREATE FUNCTION public.job_run_reputation_rollup_v1(window_key_input public.reputation_window) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to roll up reputation';
    errcode CONSTANT text := 'P0001';
    window_key_value reputation_window;
    window_start_value TIMESTAMPTZ;
    window_cutoff TIMESTAMPTZ;
    now_ts TIMESTAMPTZ;
BEGIN
    IF window_key_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'window_missing';
    END IF;

    window_key_value := window_key_input;
    now_ts := now();

    IF window_key_value = '1h' THEN
        window_start_value := date_trunc('hour', now_ts);
        window_cutoff := now_ts - make_interval(hours => 1);
    ELSIF window_key_value = '24h' THEN
        window_start_value := date_trunc('hour', now_ts);
        window_cutoff := now_ts - make_interval(hours => 24);
    ELSIF window_key_value = '7d' THEN
        window_start_value := date_trunc('day', now_ts);
        window_cutoff := now_ts - make_interval(days => 7);
    ELSE
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'window_invalid';
    END IF;

    WITH indexer_scope AS (
        SELECT indexer_instance_id
        FROM indexer_instance
        WHERE deleted_at IS NULL
    ),
    request_stats AS (
        SELECT indexer_instance_id,
               COUNT(*) AS request_count,
               COUNT(*) FILTER (WHERE outcome = 'success' AND parse_ok = TRUE) AS request_success_count
        FROM outbound_request_log
        WHERE request_type IN ('caps', 'search', 'tvsearch', 'moviesearch', 'rss', 'probe')
          AND finished_at >= window_cutoff
          AND error_class IS DISTINCT FROM 'rate_limited'
        GROUP BY indexer_instance_id
    ),
    acquisition_stats AS (
        SELECT src.indexer_instance_id,
               COUNT(*) AS acquisition_count,
               COUNT(*) FILTER (WHERE a.status = 'succeeded') AS acquisition_success_count,
               COUNT(*) FILTER (WHERE a.failure_class = 'dmca') AS dmca_count,
               COUNT(*) FILTER (WHERE a.failure_class IN ('corrupted', 'passworded')) AS fake_failure_count
        FROM acquisition_attempt a
        JOIN canonical_torrent_source src
            ON src.canonical_torrent_source_id = a.canonical_torrent_source_id
        JOIN indexer_scope s
            ON s.indexer_instance_id = src.indexer_instance_id
        WHERE a.started_at >= window_cutoff
        GROUP BY src.indexer_instance_id
    ),
    fake_reports AS (
        SELECT src.indexer_instance_id,
               COUNT(*) AS fake_report_count
        FROM user_result_action ura
        JOIN user_result_action_kv kv
            ON kv.user_result_action_id = ura.user_result_action_id
           AND kv.key = 'chosen_source_public_id'
        JOIN canonical_torrent_source src
            ON src.canonical_torrent_source_public_id::TEXT = kv.value
        JOIN indexer_scope s
            ON s.indexer_instance_id = src.indexer_instance_id
        WHERE ura.created_at >= window_cutoff
          AND ura.action = 'reported_fake'
        GROUP BY src.indexer_instance_id
    ),
    combined AS (
        SELECT s.indexer_instance_id,
               COALESCE(r.request_count, 0) AS request_count,
               COALESCE(r.request_success_count, 0) AS request_success_count,
               COALESCE(a.acquisition_count, 0) AS acquisition_count,
               COALESCE(a.acquisition_success_count, 0) AS acquisition_success_count,
               COALESCE(a.dmca_count, 0) AS dmca_count,
               COALESCE(a.fake_failure_count, 0) AS fake_failure_count,
               COALESCE(f.fake_report_count, 0) AS fake_report_count
        FROM indexer_scope s
        LEFT JOIN request_stats r
            ON r.indexer_instance_id = s.indexer_instance_id
        LEFT JOIN acquisition_stats a
            ON a.indexer_instance_id = s.indexer_instance_id
        LEFT JOIN fake_reports f
            ON f.indexer_instance_id = s.indexer_instance_id
    ),
    eligible AS (
        SELECT c.*
        FROM combined c
        WHERE c.request_count >= 30
           OR c.acquisition_count >= 10
    )
    INSERT INTO source_reputation (
        indexer_instance_id,
        window_key,
        window_start,
        request_success_rate,
        acquisition_success_rate,
        fake_rate,
        dmca_rate,
        request_count,
        request_success_count,
        acquisition_count,
        acquisition_success_count,
        min_samples,
        computed_at
    )
    SELECT e.indexer_instance_id,
           window_key_value,
           window_start_value,
           CASE
               WHEN e.request_count > 0 THEN e.request_success_count::NUMERIC / e.request_count
               ELSE 0
           END AS request_success_rate,
           CASE
               WHEN e.acquisition_count > 0 THEN e.acquisition_success_count::NUMERIC / e.acquisition_count
               ELSE 0
           END AS acquisition_success_rate,
           CASE
               WHEN e.acquisition_count > 0 THEN
                   LEAST(e.fake_failure_count + e.fake_report_count, e.acquisition_count)::NUMERIC / e.acquisition_count
               ELSE 0
           END AS fake_rate,
           CASE
               WHEN e.acquisition_count > 0 THEN e.dmca_count::NUMERIC / e.acquisition_count
               ELSE 0
           END AS dmca_rate,
           e.request_count,
           e.request_success_count,
           e.acquisition_count,
           e.acquisition_success_count,
           CASE
               WHEN e.acquisition_count >= 10 THEN 10
               ELSE 30
           END AS min_samples,
           now_ts
    FROM eligible e
    ON CONFLICT (indexer_instance_id, window_key, window_start)
    DO UPDATE SET
        request_success_rate = EXCLUDED.request_success_rate,
        acquisition_success_rate = EXCLUDED.acquisition_success_rate,
        fake_rate = EXCLUDED.fake_rate,
        dmca_rate = EXCLUDED.dmca_rate,
        request_count = EXCLUDED.request_count,
        request_success_count = EXCLUDED.request_success_count,
        acquisition_count = EXCLUDED.acquisition_count,
        acquisition_success_count = EXCLUDED.acquisition_success_count,
        min_samples = EXCLUDED.min_samples,
        computed_at = EXCLUDED.computed_at;
END;
$$;



CREATE FUNCTION public.job_run_reputation_rollup_v2(window_key_input public.reputation_window) RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    job_key_value job_key;
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    job_key_value := CASE window_key_input
        WHEN '1h' THEN 'reputation_rollup_1h'::job_key
        WHEN '24h' THEN 'reputation_rollup_24h'::job_key
        WHEN '7d' THEN 'reputation_rollup_7d'::job_key
    END;

    BEGIN
        PERFORM job_run_reputation_rollup_v1(window_key_input => window_key_input);
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1(job_key_value);

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_run_retention_purge() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_retention_purge_v2();
END;
$$;



CREATE FUNCTION public.job_run_retention_purge_v1() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    errcode CONSTANT text := 'P0001';
    retention_search_days_value INTEGER;
    retention_outbound_days_value INTEGER;
    retention_rss_days_value INTEGER;
    retention_conflict_days_value INTEGER;
    retention_conflict_audit_days_value INTEGER;
    retention_health_days_value INTEGER;
    retention_reputation_days_value INTEGER;
    cutoff_search TIMESTAMPTZ;
    cutoff_outbound TIMESTAMPTZ;
    cutoff_rss TIMESTAMPTZ;
    cutoff_conflict TIMESTAMPTZ;
    cutoff_conflict_audit TIMESTAMPTZ;
    cutoff_health TIMESTAMPTZ;
    cutoff_reputation TIMESTAMPTZ;
BEGIN
    SELECT retention_search_days,
           retention_outbound_request_log_days,
           retention_rss_item_seen_days,
           retention_source_metadata_conflict_days,
           retention_source_metadata_conflict_audit_days,
           retention_health_events_days,
           retention_reputation_days
    INTO retention_search_days_value,
         retention_outbound_days_value,
         retention_rss_days_value,
         retention_conflict_days_value,
         retention_conflict_audit_days_value,
         retention_health_days_value,
         retention_reputation_days_value
    FROM deployment_config
    ORDER BY deployment_config_id
    LIMIT 1;

    IF retention_search_days_value IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = 'Failed to purge retention data',
            DETAIL = 'deployment_config_missing';
    END IF;

    cutoff_search := now() - make_interval(days => retention_search_days_value);
    cutoff_outbound := now() - make_interval(days => retention_outbound_days_value);
    cutoff_rss := now() - make_interval(days => retention_rss_days_value);
    cutoff_conflict := now() - make_interval(days => retention_conflict_days_value);
    cutoff_conflict_audit := now() - make_interval(days => retention_conflict_audit_days_value);
    cutoff_health := now() - make_interval(days => retention_health_days_value);
    cutoff_reputation := now() - make_interval(days => retention_reputation_days_value);

    WITH purged_requests AS (
        DELETE FROM search_request
        WHERE finished_at IS NOT NULL
          AND finished_at < cutoff_search
        RETURNING search_request_id, policy_snapshot_id
    ),
    policy_counts AS (
        SELECT policy_snapshot_id, COUNT(*) AS request_count
        FROM purged_requests
        GROUP BY policy_snapshot_id
    ),
    policy_updates AS (
        UPDATE policy_snapshot
        SET ref_count = GREATEST(ref_count - policy_counts.request_count, 0)
        FROM policy_counts
        WHERE policy_snapshot.policy_snapshot_id = policy_counts.policy_snapshot_id
        RETURNING policy_snapshot.policy_snapshot_id
    ),
    purged_context_scores AS (
        DELETE FROM canonical_torrent_source_context_score
        USING purged_requests
        WHERE canonical_torrent_source_context_score.context_key_type = 'search_request'
          AND canonical_torrent_source_context_score.context_key_id = purged_requests.search_request_id
        RETURNING canonical_torrent_source_context_score_id
    ),
    purged_best_context AS (
        DELETE FROM canonical_torrent_best_source_context
        USING purged_requests
        WHERE canonical_torrent_best_source_context.context_key_type = 'search_request'
          AND canonical_torrent_best_source_context.context_key_id = purged_requests.search_request_id
        RETURNING canonical_torrent_best_source_context_id
    )
    DELETE FROM policy_set
    USING purged_requests
    WHERE policy_set.is_auto_created = TRUE
      AND policy_set.created_for_search_request_id = purged_requests.search_request_id;

    DELETE FROM outbound_request_log
    WHERE COALESCE(finished_at, started_at) < cutoff_outbound;

    DELETE FROM indexer_rss_item_seen
    WHERE first_seen_at < cutoff_rss;

    DELETE FROM source_metadata_conflict
    WHERE observed_at < cutoff_conflict;

    DELETE FROM source_metadata_conflict_audit_log
    WHERE occurred_at < cutoff_conflict_audit;

    DELETE FROM indexer_health_event
    WHERE occurred_at < cutoff_health;

    DELETE FROM source_reputation
    WHERE window_start < cutoff_reputation;
END;
$$;



