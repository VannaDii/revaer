


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



CREATE FUNCTION public.job_run_retention_purge_v2() RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    BEGIN
        PERFORM job_run_retention_purge_v1();
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1('retention_purge');

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_run_rss_subscription_backfill() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_run_rss_subscription_backfill_v2();
END;
$$;



CREATE FUNCTION public.job_run_rss_subscription_backfill_v1() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    maintenance_completed_at TIMESTAMPTZ;
BEGIN
    SELECT rss_subscription_backfill_completed_at
    INTO maintenance_completed_at
    FROM deployment_maintenance_state
    ORDER BY deployment_maintenance_state_id
    LIMIT 1;

    IF maintenance_completed_at IS NOT NULL THEN
        UPDATE job_schedule
        SET enabled = FALSE
        WHERE job_key = 'rss_subscription_backfill';
        RETURN;
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
    SELECT inst.indexer_instance_id,
           (inst.is_enabled AND inst.enable_rss),
           900,
           NULL,
           CASE
               WHEN inst.is_enabled AND inst.enable_rss THEN
                   now() + make_interval(secs => floor(random() * 60)::INT)
               ELSE NULL
           END,
           NULL,
           NULL
    FROM indexer_instance inst
    LEFT JOIN indexer_rss_subscription sub
        ON sub.indexer_instance_id = inst.indexer_instance_id
    WHERE sub.indexer_instance_id IS NULL;

    UPDATE deployment_maintenance_state
    SET rss_subscription_backfill_completed_at = now(),
        last_updated_at = now();

    IF NOT FOUND THEN
        INSERT INTO deployment_maintenance_state (
            rss_subscription_backfill_completed_at,
            last_updated_at
        )
        VALUES (
            now(),
            now()
        );
    END IF;

    UPDATE job_schedule
    SET enabled = FALSE
    WHERE job_key = 'rss_subscription_backfill';
END;
$$;



CREATE FUNCTION public.job_run_rss_subscription_backfill_v2() RETURNS TABLE(ok boolean, error_code text, error_detail text)
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    ok_value BOOLEAN := TRUE;
    error_code_value TEXT;
    error_detail_value TEXT;
BEGIN
    BEGIN
        PERFORM job_run_rss_subscription_backfill_v1();
    EXCEPTION WHEN OTHERS THEN
        ok_value := FALSE;
        GET STACKED DIAGNOSTICS
            error_code_value = RETURNED_SQLSTATE,
            error_detail_value = PG_EXCEPTION_DETAIL;
    END;

    PERFORM job_schedule_mark_completed_v1('rss_subscription_backfill');

    RETURN QUERY SELECT ok_value, error_code_value, error_detail_value;
END;
$$;



CREATE FUNCTION public.job_schedule_mark_completed(job_key_input public.job_key) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM job_schedule_mark_completed_v1(job_key_input => job_key_input);
END;
$$;



CREATE FUNCTION public.job_schedule_mark_completed_v1(job_key_input public.job_key) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to update job schedule';
    errcode CONSTANT text := 'P0001';
    schedule_id BIGINT;
    cadence_seconds_value INTEGER;
    jitter_seconds_value INTEGER;
    now_ts TIMESTAMPTZ;
    jitter_value INTEGER;
BEGIN
    IF job_key_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'job_key_missing';
    END IF;

    SELECT job_schedule_id, cadence_seconds, jitter_seconds
    INTO schedule_id, cadence_seconds_value, jitter_seconds_value
    FROM job_schedule
    WHERE job_key = job_key_input;

    IF schedule_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'job_not_found';
    END IF;

    now_ts := now();
    jitter_value := random_jitter_seconds(jitter_seconds_value);

    UPDATE job_schedule
    SET last_run_at = now_ts,
        next_run_at = now_ts + make_interval(secs => cadence_seconds_value + jitter_value),
        locked_until = NULL,
        lock_owner = NULL
    WHERE job_schedule_id = schedule_id;
END;
$$;



CREATE FUNCTION public.log_source_metadata_conflict(canonical_torrent_source_id_input bigint, indexer_instance_id_input bigint, conflict_type_input public.conflict_type, existing_value_input text, incoming_value_input text, observed_at_input timestamp with time zone) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM log_source_metadata_conflict_v1(canonical_torrent_source_id_input => canonical_torrent_source_id_input, indexer_instance_id_input => indexer_instance_id_input, conflict_type_input => conflict_type_input, existing_value_input => existing_value_input, incoming_value_input => incoming_value_input, observed_at_input => observed_at_input);
END;
$$;



CREATE FUNCTION public.log_source_metadata_conflict_v1(canonical_torrent_source_id_input bigint, indexer_instance_id_input bigint, conflict_type_input public.conflict_type, existing_value_input text, incoming_value_input text, observed_at_input timestamp with time zone) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    conflict_id BIGINT;
    existing_value TEXT;
    incoming_value TEXT;
    observed_at_value TIMESTAMPTZ;
BEGIN
    existing_value := COALESCE(existing_value_input, '');
    incoming_value := COALESCE(incoming_value_input, '');

    IF char_length(existing_value) > 256 THEN
        existing_value := substring(existing_value FROM 1 FOR 256);
    END IF;
    IF char_length(incoming_value) > 256 THEN
        incoming_value := substring(incoming_value FROM 1 FOR 256);
    END IF;

    observed_at_value := COALESCE(observed_at_input, now());

    INSERT INTO source_metadata_conflict (
        canonical_torrent_source_id,
        conflict_type,
        existing_value,
        incoming_value,
        observed_at
    )
    VALUES (
        canonical_torrent_source_id_input,
        conflict_type_input,
        existing_value,
        incoming_value,
        observed_at_value
    )
    RETURNING source_metadata_conflict_id INTO conflict_id;

    INSERT INTO source_metadata_conflict_audit_log (
        conflict_id,
        action,
        actor_user_id,
        occurred_at,
        note
    )
    VALUES (
        conflict_id,
        'created',
        0,
        now(),
        NULL
    );

    INSERT INTO indexer_health_event (
        indexer_instance_id,
        occurred_at,
        event_type,
        detail
    )
    VALUES (
        indexer_instance_id_input,
        observed_at_value,
        'identity_conflict',
        conflict_type_input::TEXT
    );
END;
$$;



CREATE FUNCTION public.media_actor_id_for_public_id_v1(actor_public_id_input uuid) RETURNS bigint
    LANGUAGE plpgsql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
BEGIN
    SELECT user_id
      INTO actor_id
      FROM app_user
     WHERE user_public_id = actor_public_id_input;

    IF actor_id IS NULL THEN
        RAISE EXCEPTION 'actor not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'app_user_not_found';
    END IF;

    RETURN actor_id;
END;
$$;



CREATE FUNCTION public.media_app_error_code_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'P0001'
$$;



CREATE FUNCTION public.media_audio_channel_layout_count_v1(channel_layout_input text) RETURNS integer
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT CASE lower(btrim(channel_layout_input))
        WHEN 'mono' THEN 1
        WHEN '1c' THEN 1
        WHEN 'stereo' THEN 2
        WHEN '2c' THEN 2
        WHEN '2.1' THEN 3
        WHEN '3.0' THEN 3
        WHEN '3.0(back)' THEN 3
        WHEN '4.0' THEN 4
        WHEN 'quad' THEN 4
        WHEN 'quad(side)' THEN 4
        WHEN '3.1' THEN 4
        WHEN '5.0' THEN 5
        WHEN '5.0(side)' THEN 5
        WHEN '4.1' THEN 5
        WHEN '5.1' THEN 6
        WHEN '5.1(side)' THEN 6
        WHEN '6.1' THEN 7
        WHEN '6.1(back)' THEN 7
        WHEN '7.1' THEN 8
        WHEN '7.1(wide)' THEN 8
        WHEN '7.1(wide-side)' THEN 8
        ELSE NULL
    END
$$;



CREATE FUNCTION public.media_capability_run_status_completed_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'completed'
$$;



CREATE FUNCTION public.media_capability_run_status_failed_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'failed'
$$;



CREATE FUNCTION public.media_capability_run_status_running_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'running'
$$;



CREATE FUNCTION public.media_capability_snapshot_encoder_list_v1(snapshot_run_public_id_input uuid) RETURNS TABLE(encoder_name text, observed_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT mcse.encoder_name,
           mcse.observed_at
      FROM media_capability_snapshot_encoder mcse
     WHERE mcse.snapshot_run_public_id = snapshot_run_public_id_input
     ORDER BY lower(mcse.encoder_name) ASC, mcse.media_capability_snapshot_encoder_id ASC;
$$;



CREATE FUNCTION public.media_capability_snapshot_encoder_record_v1(actor_public_id_input uuid, snapshot_run_public_id_input uuid, encoder_name_input text) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    encoder_id_out BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    IF snapshot_run_public_id_input IS NULL THEN
        RAISE EXCEPTION 'snapshot run id required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_capability_snapshot_run_required';
    END IF;

    INSERT INTO media_capability_snapshot_encoder (
        snapshot_run_public_id,
        encoder_name,
        observed_by_user_id
    )
    VALUES (
        snapshot_run_public_id_input,
        btrim(encoder_name_input),
        actor_id
    )
    ON CONFLICT (snapshot_run_public_id, lower(encoder_name))
    DO UPDATE SET
        observed_at = EXCLUDED.observed_at,
        observed_by_user_id = EXCLUDED.observed_by_user_id
    RETURNING media_capability_snapshot_encoder_id
    INTO encoder_id_out;

    RETURN encoder_id_out;
END;
$$;



CREATE FUNCTION public.media_capability_snapshot_feature_list_v1(snapshot_run_public_id_input uuid) RETURNS TABLE(feature_family text, feature_name text, supported boolean, detail_text text, observed_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT mcsf.feature_family,
           mcsf.feature_name,
           mcsf.supported,
           mcsf.detail_text,
           mcsf.observed_at
      FROM media_capability_snapshot_feature mcsf
     WHERE mcsf.snapshot_run_public_id = snapshot_run_public_id_input
     ORDER BY lower(mcsf.feature_family) ASC,
              lower(mcsf.feature_name) ASC,
              mcsf.media_capability_snapshot_feature_id ASC;
$$;



CREATE FUNCTION public.media_capability_snapshot_feature_record_v1(actor_public_id_input uuid, snapshot_run_public_id_input uuid, feature_family_input text, feature_name_input text, supported_input boolean, detail_text_input text) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    feature_id_out BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    IF snapshot_run_public_id_input IS NULL THEN
        RAISE EXCEPTION 'snapshot run id required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_capability_snapshot_run_required';
    END IF;

    INSERT INTO media_capability_snapshot_feature (
        snapshot_run_public_id,
        feature_family,
        feature_name,
        supported,
        detail_text,
        observed_by_user_id
    )
    VALUES (
        snapshot_run_public_id_input,
        lower(btrim(feature_family_input)),
        lower(btrim(feature_name_input)),
        COALESCE(supported_input, TRUE),
        NULLIF(btrim(COALESCE(detail_text_input, '')), ''),
        actor_id
    )
    ON CONFLICT (
        snapshot_run_public_id,
        lower(feature_family),
        lower(feature_name)
    )
    DO UPDATE SET
        supported = EXCLUDED.supported,
        detail_text = EXCLUDED.detail_text,
        observed_at = EXCLUDED.observed_at,
        observed_by_user_id = EXCLUDED.observed_by_user_id
    RETURNING media_capability_snapshot_feature_id
    INTO feature_id_out;

    RETURN feature_id_out;
END;
$$;



CREATE FUNCTION public.media_capability_snapshot_latest_v1() RETURNS TABLE(media_capability_snapshot_id bigint, ffmpeg_version text, ffprobe_version text, codec_name text, encode_supported boolean, decode_supported boolean, observed_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT
        mcs.media_capability_snapshot_id,
        mcs.ffmpeg_version,
        mcs.ffprobe_version,
        mcs.codec_name,
        mcs.encode_supported,
        mcs.decode_supported,
        mcs.observed_at
    FROM media_capability_snapshot mcs
    ORDER BY mcs.observed_at DESC, mcs.media_capability_snapshot_id DESC
    LIMIT 1;
$$;



CREATE FUNCTION public.media_capability_snapshot_latest_v2() RETURNS TABLE(media_capability_snapshot_id bigint, snapshot_run_public_id uuid, ffmpeg_version text, ffprobe_version text, codec_name text, encode_supported boolean, decode_supported boolean, observed_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    WITH latest_run AS (
        SELECT run.snapshot_run_public_id
          FROM media_capability_snapshot_run run
         WHERE run.status = media_capability_run_status_completed_v1()
           AND run.completed_at IS NOT NULL
         ORDER BY run.completed_at DESC, run.started_at DESC
         LIMIT 1
    )
    SELECT mcs.media_capability_snapshot_id,
           mcs.snapshot_run_public_id,
           mcs.ffmpeg_version,
           mcs.ffprobe_version,
           mcs.codec_name,
           mcs.encode_supported,
           mcs.decode_supported,
           mcs.observed_at
      FROM media_capability_snapshot mcs
      JOIN latest_run lr ON lr.snapshot_run_public_id = mcs.snapshot_run_public_id
     ORDER BY lower(mcs.codec_name) ASC, mcs.media_capability_snapshot_id ASC;
$$;



CREATE FUNCTION public.media_capability_snapshot_record_v1(actor_public_id_input uuid, ffmpeg_version_input text, ffprobe_version_input text, codec_name_input text, encode_supported_input boolean, decode_supported_input boolean) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    snapshot_id_out BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    INSERT INTO media_capability_snapshot (
        ffmpeg_version,
        ffprobe_version,
        codec_name,
        encode_supported,
        decode_supported,
        observed_by_user_id
    )
    VALUES (
        btrim(ffmpeg_version_input),
        btrim(ffprobe_version_input),
        btrim(codec_name_input),
        COALESCE(encode_supported_input, FALSE),
        COALESCE(decode_supported_input, TRUE),
        actor_id
    )
    RETURNING media_capability_snapshot_id
    INTO snapshot_id_out;

    RETURN snapshot_id_out;
END;
$$;



CREATE FUNCTION public.media_capability_snapshot_record_v2(actor_public_id_input uuid, snapshot_run_public_id_input uuid, ffmpeg_version_input text, ffprobe_version_input text, codec_name_input text, encode_supported_input boolean, decode_supported_input boolean) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    snapshot_id_out BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    INSERT INTO media_capability_snapshot (
        snapshot_run_public_id,
        ffmpeg_version,
        ffprobe_version,
        codec_name,
        encode_supported,
        decode_supported,
        observed_by_user_id
    )
    VALUES (
        COALESCE(snapshot_run_public_id_input, gen_random_uuid()),
        btrim(ffmpeg_version_input),
        btrim(ffprobe_version_input),
        btrim(codec_name_input),
        COALESCE(encode_supported_input, FALSE),
        COALESCE(decode_supported_input, TRUE),
        actor_id
    )
    RETURNING media_capability_snapshot_id
    INTO snapshot_id_out;

    RETURN snapshot_id_out;
END;
$$;



CREATE FUNCTION public.media_capability_snapshot_run_complete_v1(snapshot_run_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    UPDATE media_capability_snapshot_run
       SET status = media_capability_run_status_completed_v1(),
           completed_at = now(),
           error_code = NULL
     WHERE snapshot_run_public_id = snapshot_run_public_id_input
       AND status = media_capability_run_status_running_v1();

    IF NOT FOUND THEN
        RAISE EXCEPTION 'capability snapshot run not running'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_capability_snapshot_run_not_running';
    END IF;
END;
$$;



CREATE FUNCTION public.media_capability_snapshot_run_start_v1(actor_public_id_input uuid, snapshot_run_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    INSERT INTO media_capability_snapshot_run (
        snapshot_run_public_id,
        status,
        observed_by_user_id
    )
    VALUES (
        snapshot_run_public_id_input,
        media_capability_run_status_running_v1(),
        actor_id
    );
END;
$$;



CREATE FUNCTION public.media_compatibility_target_list_v1() RETURNS TABLE(compatibility_target_key text, version integer, display_name text, video_codec text, audio_codec text, audio_channels integer, audio_channel_layout text, subtitle_policy text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT target.compatibility_target_key,
           target.version,
           target.display_name,
           target.video_codec,
           target.audio_codec,
           target.audio_channels,
           target.audio_channel_layout,
           target.subtitle_policy
      FROM media_compatibility_target AS target
     WHERE target.enabled
     ORDER BY lower(target.compatibility_target_key), target.version DESC;
$$;



CREATE FUNCTION public.media_compatibility_target_upsert_v1(actor_public_id_input uuid, compatibility_target_key_input text, version_input integer, display_name_input text, video_codec_input text, audio_codec_input text, audio_channels_input integer, audio_channel_layout_input text, subtitle_policy_input text) RETURNS TABLE(compatibility_target_key text, version integer, display_name text, video_codec text, audio_codec text, audio_channels integer, audio_channel_layout text, subtitle_policy text)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
#variable_conflict use_column
DECLARE
    actor_id BIGINT;
    version_value INT;
    audio_channel_layout_value TEXT;
    audio_layout_channel_count INT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    version_value := COALESCE(version_input, 1);
    audio_channel_layout_value := NULLIF(lower(btrim(audio_channel_layout_input)), '');
    audio_layout_channel_count := media_audio_channel_layout_count_v1(audio_channel_layout_value);

    IF audio_channel_layout_value IS NOT NULL AND audio_layout_channel_count IS NULL THEN
        RAISE EXCEPTION 'audio channel layout is not supported'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_audio_shape_invalid';
    END IF;
    IF audio_channels_input IS NOT NULL
        AND audio_layout_channel_count IS NOT NULL
        AND audio_channels_input <> audio_layout_channel_count THEN
        RAISE EXCEPTION 'audio channel count does not match layout'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_audio_shape_invalid';
    END IF;

    RETURN QUERY
    INSERT INTO media_compatibility_target (
        compatibility_target_key,
        version,
        display_name,
        video_codec,
        audio_codec,
        audio_channels,
        audio_channel_layout,
        subtitle_policy,
        enabled,
        updated_at
    )
    VALUES (
        btrim(compatibility_target_key_input),
        version_value,
        btrim(display_name_input),
        lower(btrim(video_codec_input)),
        lower(btrim(audio_codec_input)),
        audio_channels_input,
        audio_channel_layout_value,
        lower(btrim(subtitle_policy_input)),
        TRUE,
        now()
    )
    ON CONFLICT (lower(compatibility_target_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_codec = EXCLUDED.video_codec,
        audio_codec = EXCLUDED.audio_codec,
        audio_channels = EXCLUDED.audio_channels,
        audio_channel_layout = EXCLUDED.audio_channel_layout,
        subtitle_policy = EXCLUDED.subtitle_policy,
        enabled = TRUE,
        updated_at = now()
    RETURNING
        media_compatibility_target.compatibility_target_key,
        media_compatibility_target.version,
        media_compatibility_target.display_name,
        media_compatibility_target.video_codec,
        media_compatibility_target.audio_codec,
        media_compatibility_target.audio_channels,
        media_compatibility_target.audio_channel_layout,
        media_compatibility_target.subtitle_policy;
END;
$$;



CREATE FUNCTION public.media_desired_target_chapter_append_v1(media_desired_target_profile_public_id_input uuid, start_millis_input bigint, end_millis_input bigint) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    target_id BIGINT;
    chapter_policy_value TEXT;
    next_sort_order INT;
BEGIN
    SELECT target.media_desired_target_profile_id,
           container.container_chapter_policy
      INTO target_id,
           chapter_policy_value
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     FOR UPDATE;

    IF target_id IS NULL THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    IF chapter_policy_value <> 'replace' THEN
        RAISE EXCEPTION 'desired target chapter rows require replace policy'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_policy_mismatch';
    END IF;

    IF EXISTS (
        SELECT 1 FROM media_profile WHERE desired_target_profile_id = target_id
        UNION ALL
        SELECT 1 FROM media_job WHERE intent_desired_target_profile_id = target_id
    ) THEN
        RAISE EXCEPTION 'desired target version is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_immutable';
    END IF;

    IF COALESCE(start_millis_input, -1) < 0
       OR COALESCE(end_millis_input, 0) <= COALESCE(start_millis_input, -1) THEN
        RAISE EXCEPTION 'desired target chapter is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_invalid';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM media_desired_target_container_chapter chapter
         WHERE chapter.media_desired_target_profile_id = target_id
           AND chapter.start_millis < end_millis_input
           AND chapter.end_millis > start_millis_input
    ) THEN
        RAISE EXCEPTION 'desired target chapter overlaps'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_overlap';
    END IF;

    IF (
        SELECT COUNT(*)
          FROM media_desired_target_container_chapter chapter
         WHERE chapter.media_desired_target_profile_id = target_id
    ) >= 1024 THEN
        RAISE EXCEPTION 'desired target chapter count exceeds limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_count_exceeded';
    END IF;

    SELECT COALESCE(MAX(chapter.sort_order), 0) + 1
      INTO next_sort_order
      FROM media_desired_target_container_chapter chapter
     WHERE chapter.media_desired_target_profile_id = target_id;

    INSERT INTO media_desired_target_container_chapter (
        media_desired_target_profile_id,
        start_millis,
        end_millis,
        sort_order
    )
    VALUES (
        target_id,
        start_millis_input,
        end_millis_input,
        next_sort_order
    );
END;
$$;



CREATE FUNCTION public.media_desired_target_chapter_list_v1(media_desired_target_profile_public_id_input uuid) RETURNS TABLE(start_millis bigint, end_millis bigint, metadata_key text, metadata_value text)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT chapter.start_millis,
           chapter.end_millis,
           metadata.metadata_key,
           metadata.metadata_value
      FROM media_desired_target_profile target
      JOIN media_desired_target_container_chapter chapter
        ON chapter.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_container_chapter_metadata metadata
        ON metadata.media_desired_target_container_chapter_id = chapter.media_desired_target_container_chapter_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY chapter.start_millis, metadata.metadata_key NULLS FIRST;
$$;



CREATE FUNCTION public.media_desired_target_chapter_metadata_append_v1(media_desired_target_profile_public_id_input uuid, start_millis_input bigint, metadata_key_input text, metadata_value_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    target_id BIGINT;
    chapter_id BIGINT;
    chapter_policy_value TEXT;
    metadata_key_value TEXT;
    metadata_value_value TEXT;
BEGIN
    SELECT target.media_desired_target_profile_id,
           container.container_chapter_policy
      INTO target_id,
           chapter_policy_value
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     FOR UPDATE;

    IF target_id IS NULL THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    IF chapter_policy_value <> 'replace' THEN
        RAISE EXCEPTION 'desired target chapter rows require replace policy'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_policy_mismatch';
    END IF;

    IF EXISTS (
        SELECT 1 FROM media_profile WHERE desired_target_profile_id = target_id
        UNION ALL
        SELECT 1 FROM media_job WHERE intent_desired_target_profile_id = target_id
    ) THEN
        RAISE EXCEPTION 'desired target version is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_immutable';
    END IF;

    SELECT chapter.media_desired_target_container_chapter_id
      INTO chapter_id
      FROM media_desired_target_container_chapter chapter
     WHERE chapter.media_desired_target_profile_id = target_id
       AND chapter.start_millis = start_millis_input;

    IF chapter_id IS NULL THEN
        RAISE EXCEPTION 'desired target chapter not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_not_found';
    END IF;

    metadata_key_value := lower(NULLIF(btrim(metadata_key_input), ''));
    metadata_value_value := NULLIF(btrim(metadata_value_input), '');

    IF metadata_key_value IS NULL OR metadata_value_value IS NULL THEN
        RAISE EXCEPTION 'desired target chapter metadata is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_invalid';
    END IF;

    IF octet_length(metadata_key_value) > 128
       OR octet_length(metadata_value_value) > 4096 THEN
        RAISE EXCEPTION 'desired target chapter metadata exceeds field limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_field_exceeded';
    END IF;

    IF (
        SELECT COUNT(*)
          FROM media_desired_target_container_chapter_metadata metadata
         WHERE metadata.media_desired_target_container_chapter_id = chapter_id
    ) >= 64 THEN
        RAISE EXCEPTION 'desired target chapter metadata count exceeds limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_count_exceeded';
    END IF;

    IF COALESCE((
        SELECT SUM(octet_length(metadata.metadata_key) + octet_length(metadata.metadata_value))
          FROM media_desired_target_container_chapter_metadata metadata
          JOIN media_desired_target_container_chapter chapter
            ON chapter.media_desired_target_container_chapter_id = metadata.media_desired_target_container_chapter_id
         WHERE chapter.media_desired_target_profile_id = target_id
    ), 0) + octet_length(metadata_key_value) + octet_length(metadata_value_value) > 65536 THEN
        RAISE EXCEPTION 'desired target chapter metadata aggregate exceeds limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_bytes_exceeded';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM media_desired_target_container_chapter_metadata metadata
         WHERE metadata.media_desired_target_container_chapter_id = chapter_id
           AND metadata.metadata_key = metadata_key_value
    ) THEN
        RAISE EXCEPTION 'desired target chapter metadata key is duplicated'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_metadata_duplicate';
    END IF;

    INSERT INTO media_desired_target_container_chapter_metadata (
        media_desired_target_container_chapter_id,
        metadata_key,
        metadata_value
    )
    VALUES (
        chapter_id,
        metadata_key_value,
        metadata_value_value
    );
END;
$$;



CREATE FUNCTION public.media_desired_target_create_v1(actor_public_id_input uuid, target_key_input text, version_input integer, display_name_input text, container_format_input text) RETURNS uuid
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT media_desired_target_create_v2(
        actor_public_id_input,
        target_key_input,
        version_input,
        display_name_input,
        container_format_input,
        'preserve'
    );
$$;



CREATE FUNCTION public.media_desired_target_create_v2(actor_public_id_input uuid, target_key_input text, version_input integer, display_name_input text, container_format_input text, container_metadata_policy_input text) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    target_id BIGINT;
    target_public_id UUID;
    metadata_policy_value TEXT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    metadata_policy_value := lower(COALESCE(NULLIF(btrim(container_metadata_policy_input), ''), 'preserve'));

    IF NULLIF(btrim(target_key_input), '') IS NULL
       OR COALESCE(version_input, 0) <= 0
       OR NULLIF(btrim(display_name_input), '') IS NULL
       OR NULLIF(btrim(container_format_input), '') IS NULL
       OR metadata_policy_value NOT IN ('preserve', 'strip') THEN
        RAISE EXCEPTION 'invalid desired target'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_invalid';
    END IF;

    INSERT INTO media_desired_target_profile (
        target_key,
        version,
        display_name,
        created_by_user_id
    )
    VALUES (
        btrim(target_key_input),
        version_input,
        btrim(display_name_input),
        actor_id
    )
    RETURNING media_desired_target_profile_id, media_desired_target_profile_public_id
    INTO target_id, target_public_id;

    INSERT INTO media_desired_target_container (
        media_desired_target_profile_id,
        container_format,
        container_metadata_policy
    )
    VALUES (target_id, lower(btrim(container_format_input)), metadata_policy_value);

    RETURN target_public_id;
END;
$$;



CREATE FUNCTION public.media_desired_target_create_v3(actor_public_id_input uuid, target_key_input text, version_input integer, display_name_input text, container_format_input text, container_metadata_policy_input text, container_chapter_policy_input text) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    target_id BIGINT;
    target_public_id UUID;
    metadata_policy_value TEXT;
    chapter_policy_value TEXT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    metadata_policy_value := lower(COALESCE(NULLIF(btrim(container_metadata_policy_input), ''), 'preserve'));
    chapter_policy_value := lower(COALESCE(NULLIF(btrim(container_chapter_policy_input), ''), 'preserve'));

    IF NULLIF(btrim(target_key_input), '') IS NULL
       OR COALESCE(version_input, 0) <= 0
       OR NULLIF(btrim(display_name_input), '') IS NULL
       OR NULLIF(btrim(container_format_input), '') IS NULL
       OR metadata_policy_value NOT IN ('preserve', 'strip', 'replace')
       OR chapter_policy_value NOT IN ('preserve', 'strip', 'replace') THEN
        RAISE EXCEPTION 'invalid desired target'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_invalid';
    END IF;

    INSERT INTO media_desired_target_profile (
        target_key,
        version,
        display_name,
        created_by_user_id
    )
    VALUES (
        btrim(target_key_input),
        version_input,
        btrim(display_name_input),
        actor_id
    )
    RETURNING media_desired_target_profile_id, media_desired_target_profile_public_id
    INTO target_id, target_public_id;

    INSERT INTO media_desired_target_container (
        media_desired_target_profile_id,
        container_format,
        container_metadata_policy,
        container_chapter_policy
    )
    VALUES (
        target_id,
        lower(btrim(container_format_input)),
        metadata_policy_value,
        chapter_policy_value
    );

    RETURN target_public_id;
END;
$$;



CREATE FUNCTION public.media_desired_target_create_v4(actor_public_id_input uuid, target_key_input text, version_input integer, display_name_input text, container_format_input text, container_metadata_policy_input text, container_chapter_policy_input text, container_attachment_policy_input text) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    target_public_id UUID;
    target_id BIGINT;
    attachment_policy_value TEXT;
BEGIN
    attachment_policy_value := lower(COALESCE(NULLIF(btrim(container_attachment_policy_input), ''), 'preserve'));
    IF attachment_policy_value NOT IN ('preserve', 'strip') THEN
        RAISE EXCEPTION 'invalid desired target'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_invalid';
    END IF;

    target_public_id := media_desired_target_create_v3(
        actor_public_id_input,
        target_key_input,
        version_input,
        display_name_input,
        container_format_input,
        container_metadata_policy_input,
        container_chapter_policy_input
    );

    SELECT media_desired_target_profile_id
      INTO target_id
      FROM media_desired_target_profile
     WHERE media_desired_target_profile_public_id = target_public_id;

    UPDATE media_desired_target_container
       SET container_attachment_policy = attachment_policy_value
     WHERE media_desired_target_profile_id = target_id;

    RETURN target_public_id;
END;
$$;



CREATE FUNCTION public.media_desired_target_graph_page_v1(limit_input integer) RETURNS TABLE(media_desired_target_profile_public_id uuid, target_key text, version integer, display_name text, container_format text, stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, audio_loudness_profile text, audio_dynamic_range text, video_profile text, video_level text, video_bitrate_bps integer, color_primaries text, color_transfer text, color_space text, hdr_format text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE plpgsql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF limit_input < 1 OR limit_input > 128 THEN
        RAISE EXCEPTION 'desired target page limit is outside 1..128'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_page_limit_invalid';
    END IF;
    RETURN QUERY
    WITH target_page AS MATERIALIZED (
        SELECT * FROM media_desired_target_list_v1() LIMIT limit_input
    )
    SELECT target.*, stream.*
      FROM target_page target
      JOIN LATERAL (
          SELECT * FROM media_desired_target_stream_list_v5(
              target.media_desired_target_profile_public_id
          ) LIMIT 1025
      ) stream ON TRUE
     ORDER BY lower(target.target_key), target.version DESC, stream.sort_order, stream.stream_key;
END;
$$;



CREATE FUNCTION public.media_desired_target_graph_page_v2(limit_input integer) RETURNS TABLE(media_desired_target_profile_public_id uuid, target_key text, version integer, display_name text, container_format text, container_metadata_policy text, container_chapter_policy text, container_attachment_policy text, stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, audio_loudness_profile text, audio_dynamic_range text, video_profile text, video_level text, video_bitrate_bps integer, color_primaries text, color_transfer text, color_space text, hdr_format text, hdr10_mastering_red_x text, hdr10_mastering_red_y text, hdr10_mastering_green_x text, hdr10_mastering_green_y text, hdr10_mastering_blue_x text, hdr10_mastering_blue_y text, hdr10_mastering_white_point_x text, hdr10_mastering_white_point_y text, hdr10_mastering_min_luminance text, hdr10_mastering_max_luminance text, hdr10_max_content_light_level text, hdr10_max_frame_average_light_level text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE plpgsql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF limit_input < 1 OR limit_input > 128 THEN
        RAISE EXCEPTION 'desired target page limit is outside 1..128'
            USING ERRCODE = media_app_error_code_v1(),
                  DETAIL = 'media_desired_target_page_limit_invalid';
    END IF;

    RETURN QUERY
    WITH target_page AS MATERIALIZED (
        SELECT * FROM media_desired_target_list_v4() LIMIT limit_input
    )
    SELECT target.*, stream.*
      FROM target_page target
      JOIN LATERAL (
          SELECT * FROM media_desired_target_stream_list_v7(
              target.media_desired_target_profile_public_id
          ) LIMIT 1025
      ) stream ON TRUE
     ORDER BY lower(target.target_key), target.version DESC, stream.sort_order, stream.stream_key;
END;
$$;



CREATE FUNCTION public.media_desired_target_graph_page_v3(limit_input integer) RETURNS TABLE(media_desired_target_profile_public_id uuid, target_key text, version integer, display_name text, container_format text, container_metadata_policy text, container_chapter_policy text, container_attachment_policy text, stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, audio_loudness_profile text, audio_dynamic_range text, video_profile text, video_level text, video_bitrate_bps integer, video_width_px integer, video_height_px integer, video_pixel_format text, video_bit_depth integer, video_average_frame_rate text, color_range text, color_primaries text, color_transfer text, color_space text, hdr_format text, hdr10_mastering_red_x text, hdr10_mastering_red_y text, hdr10_mastering_green_x text, hdr10_mastering_green_y text, hdr10_mastering_blue_x text, hdr10_mastering_blue_y text, hdr10_mastering_white_point_x text, hdr10_mastering_white_point_y text, hdr10_mastering_min_luminance text, hdr10_mastering_max_luminance text, hdr10_max_content_light_level text, hdr10_max_frame_average_light_level text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE plpgsql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF limit_input < 1 OR limit_input > 128 THEN
        RAISE EXCEPTION 'desired target page limit is outside 1..128'
            USING ERRCODE = media_app_error_code_v1(),
                  DETAIL = 'media_desired_target_page_limit_invalid';
    END IF;

    RETURN QUERY
    WITH target_page AS MATERIALIZED (
        SELECT * FROM media_desired_target_list_v4() LIMIT limit_input
    )
    SELECT target.*, stream.*
      FROM target_page target
      JOIN LATERAL (
          SELECT * FROM media_desired_target_stream_list_v8(
              target.media_desired_target_profile_public_id
          ) LIMIT 1025
      ) stream ON TRUE
     ORDER BY lower(target.target_key), target.version DESC, stream.sort_order, stream.stream_key;
END;
$$;



CREATE FUNCTION public.media_desired_target_list_v1() RETURNS TABLE(media_desired_target_profile_public_id uuid, target_key text, version integer, display_name text, container_format text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT target.media_desired_target_profile_public_id,
           target.target_key,
           target.version,
           target.display_name,
           container.container_format
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.enabled
     ORDER BY lower(target.target_key), target.version DESC;
$$;



CREATE FUNCTION public.media_desired_target_list_v2() RETURNS TABLE(media_desired_target_profile_public_id uuid, target_key text, version integer, display_name text, container_format text, container_metadata_policy text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT target.media_desired_target_profile_public_id,
           target.target_key,
           target.version,
           target.display_name,
           container.container_format,
           container.container_metadata_policy
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.enabled
     ORDER BY lower(target.target_key), target.version DESC;
$$;



CREATE FUNCTION public.media_desired_target_list_v3() RETURNS TABLE(media_desired_target_profile_public_id uuid, target_key text, version integer, display_name text, container_format text, container_metadata_policy text, container_chapter_policy text)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT target.media_desired_target_profile_public_id,
           target.target_key,
           target.version,
           target.display_name,
           container.container_format,
           container.container_metadata_policy,
           container.container_chapter_policy
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.enabled
     ORDER BY lower(target.target_key), target.version DESC;
$$;



CREATE FUNCTION public.media_desired_target_list_v4() RETURNS TABLE(media_desired_target_profile_public_id uuid, target_key text, version integer, display_name text, container_format text, container_metadata_policy text, container_chapter_policy text, container_attachment_policy text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT target.media_desired_target_profile_public_id,
           target.target_key,
           target.version,
           target.display_name,
           container.container_format,
           container.container_metadata_policy,
           container.container_chapter_policy,
           container.container_attachment_policy
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.enabled
     ORDER BY lower(target.target_key), target.version DESC;
$$;



CREATE FUNCTION public.media_desired_target_metadata_append_v1(media_desired_target_profile_public_id_input uuid, metadata_key_input text, metadata_value_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    target_id BIGINT;
    metadata_policy_value TEXT;
    metadata_key_value TEXT;
    metadata_value_value TEXT;
BEGIN
    SELECT target.media_desired_target_profile_id,
           container.container_metadata_policy
      INTO target_id,
           metadata_policy_value
      FROM media_desired_target_profile target
      JOIN media_desired_target_container container
        ON container.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     FOR UPDATE;

    IF target_id IS NULL THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    IF metadata_policy_value <> 'replace' THEN
        RAISE EXCEPTION 'desired target metadata rows require replace policy'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_policy_mismatch';
    END IF;

    IF EXISTS (
        SELECT 1 FROM media_profile WHERE desired_target_profile_id = target_id
        UNION ALL
        SELECT 1 FROM media_job WHERE intent_desired_target_profile_id = target_id
    ) THEN
        RAISE EXCEPTION 'desired target version is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_immutable';
    END IF;

    metadata_key_value := lower(NULLIF(btrim(metadata_key_input), ''));
    metadata_value_value := NULLIF(btrim(metadata_value_input), '');

    IF metadata_key_value IS NULL OR metadata_value_value IS NULL THEN
        RAISE EXCEPTION 'desired target metadata is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_invalid';
    END IF;

    IF octet_length(metadata_key_value) > 128
       OR octet_length(metadata_value_value) > 4096 THEN
        RAISE EXCEPTION 'desired target metadata row exceeds byte limits'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_size_exceeded';
    END IF;

    IF (SELECT count(*)
          FROM media_desired_target_container_metadata metadata
         WHERE metadata.media_desired_target_profile_id = target_id) >= 64 THEN
        RAISE EXCEPTION 'desired target metadata row limit exceeded'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_count_exceeded';
    END IF;

    IF COALESCE((
        SELECT sum(octet_length(metadata.metadata_key) + octet_length(metadata.metadata_value))
          FROM media_desired_target_container_metadata metadata
         WHERE metadata.media_desired_target_profile_id = target_id
    ), 0) + octet_length(metadata_key_value) + octet_length(metadata_value_value) > 65536 THEN
        RAISE EXCEPTION 'desired target metadata aggregate byte limit exceeded'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_bytes_exceeded';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM media_desired_target_container_metadata metadata
         WHERE metadata.media_desired_target_profile_id = target_id
           AND metadata.metadata_key = metadata_key_value
    ) THEN
        RAISE EXCEPTION 'desired target metadata key is duplicated'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_duplicate';
    END IF;

    INSERT INTO media_desired_target_container_metadata (
        media_desired_target_profile_id,
        metadata_key,
        metadata_value
    )
    VALUES (
        target_id,
        metadata_key_value,
        metadata_value_value
    );
END;
$$;



CREATE FUNCTION public.media_desired_target_metadata_list_v1(media_desired_target_profile_public_id_input uuid) RETURNS TABLE(metadata_key text, metadata_value text)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT metadata.metadata_key,
           metadata.metadata_value
      FROM media_desired_target_profile target
      JOIN media_desired_target_container_metadata metadata
        ON metadata.media_desired_target_profile_id = target.media_desired_target_profile_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY metadata.metadata_key;
$$;



CREATE FUNCTION public.media_desired_target_stream_append_v1(media_desired_target_profile_public_id_input uuid, stream_key_input text, stream_kind_input text, semantic_role_input text, language_code_input text, optional_input boolean, sort_order_input integer, codec_input text, channel_count_input integer, channel_layout_input text, title_input text, default_disposition_input boolean, forced_disposition_input boolean) RETURNS void
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT media_desired_target_stream_append_v2(
        media_desired_target_profile_public_id_input,
        stream_key_input,
        stream_kind_input,
        semantic_role_input,
        language_code_input,
        optional_input,
        sort_order_input,
        codec_input,
        channel_count_input,
        channel_layout_input,
        title_input,
        default_disposition_input,
        forced_disposition_input,
        CASE WHEN lower(btrim(stream_kind_input)) = 'subtitle' THEN 'embedded' END,
        CASE WHEN lower(btrim(stream_kind_input)) = 'subtitle' THEN 'fail' END
    );
$$;



CREATE FUNCTION public.media_desired_target_stream_append_v2(media_desired_target_profile_public_id_input uuid, stream_key_input text, stream_kind_input text, semantic_role_input text, language_code_input text, optional_input boolean, sort_order_input integer, codec_input text, channel_count_input integer, channel_layout_input text, title_input text, default_disposition_input boolean, forced_disposition_input boolean, subtitle_placement_input text, image_subtitle_action_input text) RETURNS void
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT media_desired_target_stream_append_v3(
        media_desired_target_profile_public_id_input,
        stream_key_input,
        stream_kind_input,
        semantic_role_input,
        language_code_input,
        optional_input,
        sort_order_input,
        codec_input,
        channel_count_input,
        channel_layout_input,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        title_input,
        default_disposition_input,
        forced_disposition_input,
        subtitle_placement_input,
        image_subtitle_action_input
    );
$$;



CREATE FUNCTION public.media_desired_target_stream_append_v3(media_desired_target_profile_public_id_input uuid, stream_key_input text, stream_kind_input text, semantic_role_input text, language_code_input text, optional_input boolean, sort_order_input integer, codec_input text, channel_count_input integer, channel_layout_input text, video_profile_input text, video_level_input text, video_bitrate_bps_input integer, color_primaries_input text, color_transfer_input text, color_space_input text, hdr_format_input text, title_input text, default_disposition_input boolean, forced_disposition_input boolean, subtitle_placement_input text, image_subtitle_action_input text) RETURNS void
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT media_desired_target_stream_append_v4(
        media_desired_target_profile_public_id_input,
        stream_key_input,
        stream_kind_input,
        semantic_role_input,
        language_code_input,
        optional_input,
        sort_order_input,
        codec_input,
        channel_count_input,
        channel_layout_input,
        NULL,
        NULL,
        video_profile_input,
        video_level_input,
        video_bitrate_bps_input,
        color_primaries_input,
        color_transfer_input,
        color_space_input,
        hdr_format_input,
        title_input,
        default_disposition_input,
        forced_disposition_input,
        subtitle_placement_input,
        image_subtitle_action_input
    );
$$;



CREATE FUNCTION public.media_desired_target_stream_append_v4(media_desired_target_profile_public_id_input uuid, stream_key_input text, stream_kind_input text, semantic_role_input text, language_code_input text, optional_input boolean, sort_order_input integer, codec_input text, channel_count_input integer, channel_layout_input text, audio_bitrate_bps_input integer, audio_sample_rate_hz_input integer, video_profile_input text, video_level_input text, video_bitrate_bps_input integer, color_primaries_input text, color_transfer_input text, color_space_input text, hdr_format_input text, title_input text, default_disposition_input boolean, forced_disposition_input boolean, subtitle_placement_input text, image_subtitle_action_input text) RETURNS void
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT media_desired_target_stream_append_v5(
        media_desired_target_profile_public_id_input,
        stream_key_input,
        stream_kind_input,
        semantic_role_input,
        language_code_input,
        optional_input,
        sort_order_input,
        codec_input,
        channel_count_input,
        channel_layout_input,
        audio_bitrate_bps_input,
        audio_sample_rate_hz_input,
        NULL,
        NULL,
        video_profile_input,
        video_level_input,
        video_bitrate_bps_input,
        color_primaries_input,
        color_transfer_input,
        color_space_input,
        hdr_format_input,
        title_input,
        default_disposition_input,
        forced_disposition_input,
        subtitle_placement_input,
        image_subtitle_action_input
    );
$$;



CREATE FUNCTION public.media_desired_target_stream_append_v5(media_desired_target_profile_public_id_input uuid, stream_key_input text, stream_kind_input text, semantic_role_input text, language_code_input text, optional_input boolean, sort_order_input integer, codec_input text, channel_count_input integer, channel_layout_input text, audio_bitrate_bps_input integer, audio_sample_rate_hz_input integer, audio_loudness_profile_input text, audio_dynamic_range_input text, video_profile_input text, video_level_input text, video_bitrate_bps_input integer, color_primaries_input text, color_transfer_input text, color_space_input text, hdr_format_input text, title_input text, default_disposition_input boolean, forced_disposition_input boolean, subtitle_placement_input text, image_subtitle_action_input text) RETURNS void
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT media_desired_target_stream_append_v7(
        media_desired_target_profile_public_id_input,
        stream_key_input,
        stream_kind_input,
        semantic_role_input,
        language_code_input,
        optional_input,
        sort_order_input,
        codec_input,
        channel_count_input,
        channel_layout_input,
        audio_bitrate_bps_input,
        audio_sample_rate_hz_input,
        audio_loudness_profile_input,
        audio_dynamic_range_input,
        video_profile_input,
        video_level_input,
        video_bitrate_bps_input,
        color_primaries_input,
        color_transfer_input,
        color_space_input,
        hdr_format_input,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        NULL::TEXT,
        title_input,
        default_disposition_input,
        forced_disposition_input,
        subtitle_placement_input,
        image_subtitle_action_input
    );
$$;



CREATE FUNCTION public.media_desired_target_stream_append_v7(media_desired_target_profile_public_id_input uuid, stream_key_input text, stream_kind_input text, semantic_role_input text, language_code_input text, optional_input boolean, sort_order_input integer, codec_input text, channel_count_input integer, channel_layout_input text, audio_bitrate_bps_input integer, audio_sample_rate_hz_input integer, audio_loudness_profile_input text, audio_dynamic_range_input text, video_profile_input text, video_level_input text, video_bitrate_bps_input integer, color_primaries_input text, color_transfer_input text, color_space_input text, hdr_format_input text, hdr10_mastering_red_x_input text, hdr10_mastering_red_y_input text, hdr10_mastering_green_x_input text, hdr10_mastering_green_y_input text, hdr10_mastering_blue_x_input text, hdr10_mastering_blue_y_input text, hdr10_mastering_white_point_x_input text, hdr10_mastering_white_point_y_input text, hdr10_mastering_min_luminance_input text, hdr10_mastering_max_luminance_input text, hdr10_max_content_light_level_input text, hdr10_max_frame_average_light_level_input text, title_input text, default_disposition_input boolean, forced_disposition_input boolean, subtitle_placement_input text, image_subtitle_action_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    target_id BIGINT;
    target_stream_id BIGINT;
    stream_kind_value TEXT;
    codec_value TEXT;
    semantic_role_value TEXT;
    subtitle_placement_value TEXT;
    image_subtitle_action_value TEXT;
    channel_layout_value TEXT;
    audio_layout_channel_count INT;
    audio_loudness_profile_value TEXT;
    audio_dynamic_range_value TEXT;
    video_profile_value TEXT;
    video_level_value TEXT;
    color_primaries_value TEXT;
    color_transfer_value TEXT;
    color_space_value TEXT;
    hdr_format_value TEXT;
    hdr10_mastering_red_x_value TEXT;
    hdr10_mastering_red_y_value TEXT;
    hdr10_mastering_green_x_value TEXT;
    hdr10_mastering_green_y_value TEXT;
    hdr10_mastering_blue_x_value TEXT;
    hdr10_mastering_blue_y_value TEXT;
    hdr10_mastering_white_point_x_value TEXT;
    hdr10_mastering_white_point_y_value TEXT;
    hdr10_mastering_min_luminance_value TEXT;
    hdr10_mastering_max_luminance_value TEXT;
    hdr10_max_content_light_level_value TEXT;
    hdr10_max_frame_average_light_level_value TEXT;
    hdr10_field_count INT;
    title_value TEXT;
    default_disposition_value BOOLEAN;
    forced_disposition_value BOOLEAN;
BEGIN
    SELECT media_desired_target_profile_id
      INTO target_id
      FROM media_desired_target_profile
     WHERE media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND enabled
     FOR UPDATE;

    IF target_id IS NULL THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    IF EXISTS (
        SELECT 1 FROM media_profile WHERE desired_target_profile_id = target_id
        UNION ALL
        SELECT 1 FROM media_job WHERE intent_desired_target_profile_id = target_id
    ) THEN
        RAISE EXCEPTION 'desired target version is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_immutable';
    END IF;

    stream_kind_value := lower(btrim(stream_kind_input));
    codec_value := lower(btrim(codec_input));
    semantic_role_value := NULLIF(lower(btrim(semantic_role_input)), '');
    subtitle_placement_value := NULLIF(lower(btrim(subtitle_placement_input)), '');
    image_subtitle_action_value := NULLIF(lower(btrim(image_subtitle_action_input)), '');
    channel_layout_value := NULLIF(lower(btrim(channel_layout_input)), '');
    audio_layout_channel_count := media_audio_channel_layout_count_v1(channel_layout_value);
    audio_loudness_profile_value := NULLIF(lower(btrim(audio_loudness_profile_input)), '');
    audio_dynamic_range_value := NULLIF(lower(btrim(audio_dynamic_range_input)), '');
    video_profile_value := NULLIF(lower(btrim(video_profile_input)), '');
    video_level_value := NULLIF(lower(btrim(video_level_input)), '');
    color_primaries_value := NULLIF(lower(btrim(color_primaries_input)), '');
    color_transfer_value := NULLIF(lower(btrim(color_transfer_input)), '');
    color_space_value := NULLIF(lower(btrim(color_space_input)), '');
    hdr_format_value := NULLIF(lower(btrim(hdr_format_input)), '');
    hdr10_mastering_red_x_value := NULLIF(btrim(hdr10_mastering_red_x_input), '');
    hdr10_mastering_red_y_value := NULLIF(btrim(hdr10_mastering_red_y_input), '');
    hdr10_mastering_green_x_value := NULLIF(btrim(hdr10_mastering_green_x_input), '');
    hdr10_mastering_green_y_value := NULLIF(btrim(hdr10_mastering_green_y_input), '');
    hdr10_mastering_blue_x_value := NULLIF(btrim(hdr10_mastering_blue_x_input), '');
    hdr10_mastering_blue_y_value := NULLIF(btrim(hdr10_mastering_blue_y_input), '');
    hdr10_mastering_white_point_x_value := NULLIF(
        btrim(hdr10_mastering_white_point_x_input),
        ''
    );
    hdr10_mastering_white_point_y_value := NULLIF(
        btrim(hdr10_mastering_white_point_y_input),
        ''
    );
    hdr10_mastering_min_luminance_value := NULLIF(
        btrim(hdr10_mastering_min_luminance_input),
        ''
    );
    hdr10_mastering_max_luminance_value := NULLIF(
        btrim(hdr10_mastering_max_luminance_input),
        ''
    );
    hdr10_max_content_light_level_value := NULLIF(
        btrim(hdr10_max_content_light_level_input),
        ''
    );
    hdr10_max_frame_average_light_level_value := NULLIF(
        btrim(hdr10_max_frame_average_light_level_input),
        ''
    );
    hdr10_field_count := media_hdr10_color_volume_field_count_v1(
        hdr10_mastering_red_x_value,
        hdr10_mastering_red_y_value,
        hdr10_mastering_green_x_value,
        hdr10_mastering_green_y_value,
        hdr10_mastering_blue_x_value,
        hdr10_mastering_blue_y_value,
        hdr10_mastering_white_point_x_value,
        hdr10_mastering_white_point_y_value,
        hdr10_mastering_min_luminance_value,
        hdr10_mastering_max_luminance_value,
        hdr10_max_content_light_level_value,
        hdr10_max_frame_average_light_level_value
    );
    title_value := NULLIF(btrim(title_input), '');
    default_disposition_value := COALESCE(default_disposition_input, FALSE);
    forced_disposition_value := COALESCE(forced_disposition_input, FALSE);

    IF stream_kind_value NOT IN ('video', 'audio', 'subtitle', 'attachment', 'data') THEN
        RAISE EXCEPTION 'desired target stream kind is unsupported'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_stream_kind_unsupported';
    END IF;

    IF stream_kind_value IN ('attachment', 'data')
        AND (
            title_value IS NOT NULL
            OR default_disposition_value
            OR forced_disposition_value
            OR video_profile_value IS NOT NULL
            OR video_level_value IS NOT NULL
            OR video_bitrate_bps_input IS NOT NULL
            OR color_primaries_value IS NOT NULL
            OR color_transfer_value IS NOT NULL
            OR color_space_value IS NOT NULL
            OR hdr_format_value IS NOT NULL
            OR hdr10_field_count > 0
        ) THEN
        RAISE EXCEPTION 'retained stream row can only select exact passthrough'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_retained_stream_shape_invalid';
    END IF;

    IF stream_kind_value = 'subtitle' THEN
        IF subtitle_placement_value IS NULL
            OR subtitle_placement_value NOT IN ('embedded', 'sidecar', 'both', 'none')
            OR image_subtitle_action_value IS NULL
            OR image_subtitle_action_value NOT IN ('preserve', 'remove', 'fail') THEN
            RAISE EXCEPTION 'subtitle target shape is incomplete'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_subtitle_shape_invalid';
        END IF;
        IF semantic_role_value = 'descriptive_audio' THEN
            RAISE EXCEPTION 'subtitle target semantic role is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_subtitle_shape_invalid';
        END IF;
    ELSIF subtitle_placement_value IS NOT NULL OR image_subtitle_action_value IS NOT NULL THEN
        RAISE EXCEPTION 'subtitle shape assigned to non-subtitle target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_subtitle_shape_invalid';
    END IF;

    IF stream_kind_value = 'video' THEN
        IF video_bitrate_bps_input IS NOT NULL AND video_bitrate_bps_input <= 0 THEN
            RAISE EXCEPTION 'video bitrate must be positive'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF hdr_format_value IS NOT NULL AND hdr_format_value NOT IN ('hdr10') THEN
            RAISE EXCEPTION 'HDR format is not supported'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF hdr10_field_count > 0
            AND (
                hdr_format_value IS DISTINCT FROM 'hdr10'
                OR NOT media_hdr10_color_volume_valid_v1(
                    hdr10_mastering_red_x_value,
                    hdr10_mastering_red_y_value,
                    hdr10_mastering_green_x_value,
                    hdr10_mastering_green_y_value,
                    hdr10_mastering_blue_x_value,
                    hdr10_mastering_blue_y_value,
                    hdr10_mastering_white_point_x_value,
                    hdr10_mastering_white_point_y_value,
                    hdr10_mastering_min_luminance_value,
                    hdr10_mastering_max_luminance_value,
                    hdr10_max_content_light_level_value,
                    hdr10_max_frame_average_light_level_value
                )
            ) THEN
            RAISE EXCEPTION 'HDR10 color volume is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF NOT media_video_level_known_v1(codec_value, video_level_value) THEN
            RAISE EXCEPTION 'video level is not supported for codec'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF NOT media_video_color_value_known_v1('color_primaries', color_primaries_value)
            OR NOT media_video_color_value_known_v1('color_transfer', color_transfer_value)
            OR NOT media_video_color_value_known_v1('color_space', color_space_value) THEN
            RAISE EXCEPTION 'video color value is not supported'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
    ELSIF video_profile_value IS NOT NULL
        OR video_level_value IS NOT NULL
        OR video_bitrate_bps_input IS NOT NULL
        OR color_primaries_value IS NOT NULL
        OR color_transfer_value IS NOT NULL
        OR color_space_value IS NOT NULL
        OR hdr_format_value IS NOT NULL
        OR hdr10_field_count > 0 THEN
        RAISE EXCEPTION 'video shape assigned to non-video target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
    END IF;

    IF stream_kind_value = 'audio' THEN
        IF channel_count_input IS NOT NULL AND channel_count_input <= 0 THEN
            RAISE EXCEPTION 'audio channel count must be positive'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF channel_layout_value IS NOT NULL AND audio_layout_channel_count IS NULL THEN
            RAISE EXCEPTION 'audio channel layout is not supported'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF channel_count_input IS NOT NULL
            AND audio_layout_channel_count IS NOT NULL
            AND channel_count_input <> audio_layout_channel_count THEN
            RAISE EXCEPTION 'audio channel count does not match layout'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF audio_bitrate_bps_input IS NOT NULL AND audio_bitrate_bps_input <= 0 THEN
            RAISE EXCEPTION 'audio bitrate must be positive'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF audio_sample_rate_hz_input IS NOT NULL AND audio_sample_rate_hz_input <= 0 THEN
            RAISE EXCEPTION 'audio sample rate must be positive'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF audio_loudness_profile_value IS NOT NULL
            AND audio_loudness_profile_value NOT IN ('dialog-normalized') THEN
            RAISE EXCEPTION 'audio loudness profile is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
        IF audio_dynamic_range_value IS NOT NULL
            AND audio_dynamic_range_value NOT IN ('preserve', 'speech') THEN
            RAISE EXCEPTION 'audio dynamic range is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
        END IF;
    ELSIF channel_count_input IS NOT NULL
        OR channel_layout_value IS NOT NULL
        OR audio_bitrate_bps_input IS NOT NULL
        OR audio_sample_rate_hz_input IS NOT NULL
        OR audio_loudness_profile_value IS NOT NULL
        OR audio_dynamic_range_value IS NOT NULL THEN
        RAISE EXCEPTION 'audio shape assigned to non-audio target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_audio_shape_invalid';
    END IF;

    INSERT INTO media_desired_target_stream (
        media_desired_target_profile_id,
        stream_key,
        stream_kind,
        semantic_role,
        language_code,
        optional,
        sort_order,
        codec,
        title,
        default_disposition,
        forced_disposition,
        subtitle_placement,
        image_subtitle_action,
        video_profile,
        video_level,
        video_bitrate_bps,
        color_primaries,
        color_transfer,
        color_space,
        hdr_format,
        hdr10_mastering_red_x,
        hdr10_mastering_red_y,
        hdr10_mastering_green_x,
        hdr10_mastering_green_y,
        hdr10_mastering_blue_x,
        hdr10_mastering_blue_y,
        hdr10_mastering_white_point_x,
        hdr10_mastering_white_point_y,
        hdr10_mastering_min_luminance,
        hdr10_mastering_max_luminance,
        hdr10_max_content_light_level,
        hdr10_max_frame_average_light_level
    )
    VALUES (
        target_id,
        btrim(stream_key_input),
        stream_kind_value,
        semantic_role_value,
        NULLIF(lower(btrim(language_code_input)), ''),
        COALESCE(optional_input, FALSE),
        sort_order_input,
        codec_value,
        title_value,
        default_disposition_value,
        forced_disposition_value,
        subtitle_placement_value,
        image_subtitle_action_value,
        video_profile_value,
        video_level_value,
        video_bitrate_bps_input,
        color_primaries_value,
        color_transfer_value,
        color_space_value,
        hdr_format_value,
        hdr10_mastering_red_x_value,
        hdr10_mastering_red_y_value,
        hdr10_mastering_green_x_value,
        hdr10_mastering_green_y_value,
        hdr10_mastering_blue_x_value,
        hdr10_mastering_blue_y_value,
        hdr10_mastering_white_point_x_value,
        hdr10_mastering_white_point_y_value,
        hdr10_mastering_min_luminance_value,
        hdr10_mastering_max_luminance_value,
        hdr10_max_content_light_level_value,
        hdr10_max_frame_average_light_level_value
    )
    RETURNING media_desired_target_stream_id INTO target_stream_id;

    IF stream_kind_value = 'audio' THEN
        INSERT INTO media_desired_target_audio_stream (
            media_desired_target_stream_id,
            channel_count,
            channel_layout,
            audio_bitrate_bps,
            audio_sample_rate_hz,
            audio_loudness_profile,
            audio_dynamic_range
        )
        VALUES (
            target_stream_id,
            channel_count_input,
            channel_layout_value,
            audio_bitrate_bps_input,
            audio_sample_rate_hz_input,
            audio_loudness_profile_value,
            audio_dynamic_range_value
        );
    END IF;
END;
$$;



CREATE FUNCTION public.media_desired_target_stream_append_v7(media_desired_target_profile_public_id_input uuid, stream_key_input text, stream_kind_input text, semantic_role_input text, language_code_input text, optional_input boolean, sort_order_input integer, codec_input text, channel_count_input integer, channel_layout_input text, audio_bitrate_bps_input integer, audio_sample_rate_hz_input integer, audio_loudness_profile_input text, audio_dynamic_range_input text, video_profile_input text, video_level_input text, video_bitrate_bps_input integer, video_width_px_input integer, video_height_px_input integer, video_pixel_format_input text, video_average_frame_rate_input text, color_primaries_input text, color_transfer_input text, color_space_input text, hdr_format_input text, hdr10_mastering_red_x_input text, hdr10_mastering_red_y_input text, hdr10_mastering_green_x_input text, hdr10_mastering_green_y_input text, hdr10_mastering_blue_x_input text, hdr10_mastering_blue_y_input text, hdr10_mastering_white_point_x_input text, hdr10_mastering_white_point_y_input text, hdr10_mastering_min_luminance_input text, hdr10_mastering_max_luminance_input text, hdr10_max_content_light_level_input text, hdr10_max_frame_average_light_level_input text, title_input text, default_disposition_input boolean, forced_disposition_input boolean, subtitle_placement_input text, image_subtitle_action_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    stream_kind_value TEXT;
    video_pixel_format_value TEXT;
    video_average_frame_rate_value TEXT;
BEGIN
    stream_kind_value := lower(btrim(stream_kind_input));
    video_pixel_format_value := NULLIF(btrim(video_pixel_format_input), '');
    video_average_frame_rate_value := CASE
        WHEN NULLIF(btrim(video_average_frame_rate_input), '') IS NULL THEN NULL
        ELSE media_video_frame_rate_reduce_v1(video_average_frame_rate_input)
    END;

    IF stream_kind_value = 'video' THEN
        IF video_bitrate_bps_input IS NOT NULL
            AND (video_bitrate_bps_input <= 0 OR video_bitrate_bps_input > 1000000000) THEN
            RAISE EXCEPTION 'video bitrate is outside the supported range'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF video_width_px_input IS NOT NULL
            AND (video_width_px_input <= 0 OR video_width_px_input > 16384) THEN
            RAISE EXCEPTION 'video width is outside the supported range'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF video_height_px_input IS NOT NULL
            AND (video_height_px_input <= 0 OR video_height_px_input > 16384) THEN
            RAISE EXCEPTION 'video height is outside the supported range'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF (video_width_px_input IS NULL) IS DISTINCT FROM (video_height_px_input IS NULL) THEN
            RAISE EXCEPTION 'video resolution requires width and height'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF video_width_px_input IS NOT NULL
            AND video_width_px_input::BIGINT * video_height_px_input::BIGINT > 134217728 THEN
            RAISE EXCEPTION 'video frame area exceeds the supported range'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF NOT media_video_pixel_format_valid_v1(video_pixel_format_value)
            OR (
                video_average_frame_rate_input IS NOT NULL
                AND video_average_frame_rate_value IS NULL
            ) THEN
            RAISE EXCEPTION 'video technical shape is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
    ELSIF video_width_px_input IS NOT NULL
        OR video_height_px_input IS NOT NULL
        OR video_pixel_format_value IS NOT NULL
        OR NULLIF(btrim(video_average_frame_rate_input), '') IS NOT NULL THEN
        RAISE EXCEPTION 'video shape assigned to non-video target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
    END IF;

    PERFORM media_desired_target_stream_append_v7(
        media_desired_target_profile_public_id_input => media_desired_target_profile_public_id_input,
        stream_key_input => stream_key_input,
        stream_kind_input => stream_kind_input,
        semantic_role_input => semantic_role_input,
        language_code_input => language_code_input,
        optional_input => optional_input,
        sort_order_input => sort_order_input,
        codec_input => codec_input,
        channel_count_input => channel_count_input,
        channel_layout_input => channel_layout_input,
        audio_bitrate_bps_input => audio_bitrate_bps_input,
        audio_sample_rate_hz_input => audio_sample_rate_hz_input,
        audio_loudness_profile_input => audio_loudness_profile_input,
        audio_dynamic_range_input => audio_dynamic_range_input,
        video_profile_input => video_profile_input,
        video_level_input => video_level_input,
        video_bitrate_bps_input => video_bitrate_bps_input,
        color_primaries_input => color_primaries_input,
        color_transfer_input => color_transfer_input,
        color_space_input => color_space_input,
        hdr_format_input => hdr_format_input,
        hdr10_mastering_red_x_input => hdr10_mastering_red_x_input,
        hdr10_mastering_red_y_input => hdr10_mastering_red_y_input,
        hdr10_mastering_green_x_input => hdr10_mastering_green_x_input,
        hdr10_mastering_green_y_input => hdr10_mastering_green_y_input,
        hdr10_mastering_blue_x_input => hdr10_mastering_blue_x_input,
        hdr10_mastering_blue_y_input => hdr10_mastering_blue_y_input,
        hdr10_mastering_white_point_x_input => hdr10_mastering_white_point_x_input,
        hdr10_mastering_white_point_y_input => hdr10_mastering_white_point_y_input,
        hdr10_mastering_min_luminance_input => hdr10_mastering_min_luminance_input,
        hdr10_mastering_max_luminance_input => hdr10_mastering_max_luminance_input,
        hdr10_max_content_light_level_input => hdr10_max_content_light_level_input,
        hdr10_max_frame_average_light_level_input => hdr10_max_frame_average_light_level_input,
        title_input => title_input,
        default_disposition_input => default_disposition_input,
        forced_disposition_input => forced_disposition_input,
        subtitle_placement_input => subtitle_placement_input,
        image_subtitle_action_input => image_subtitle_action_input
    );

    UPDATE media_desired_target_stream stream
       SET video_width_px = video_width_px_input,
           video_height_px = video_height_px_input,
           video_pixel_format = video_pixel_format_value,
           video_average_frame_rate = video_average_frame_rate_value
      FROM media_desired_target_profile target
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND stream.media_desired_target_profile_id = target.media_desired_target_profile_id
       AND lower(stream.stream_key) = lower(btrim(stream_key_input));
END;
$$;



CREATE FUNCTION public.media_desired_target_stream_append_v8(media_desired_target_profile_public_id_input uuid, stream_key_input text, stream_kind_input text, semantic_role_input text, language_code_input text, optional_input boolean, sort_order_input integer, codec_input text, channel_count_input integer, channel_layout_input text, audio_bitrate_bps_input integer, audio_sample_rate_hz_input integer, audio_loudness_profile_input text, audio_dynamic_range_input text, video_profile_input text, video_level_input text, video_bitrate_bps_input integer, video_width_px_input integer, video_height_px_input integer, video_pixel_format_input text, video_bit_depth_input integer, video_average_frame_rate_input text, color_range_input text, color_primaries_input text, color_transfer_input text, color_space_input text, hdr_format_input text, hdr10_mastering_red_x_input text, hdr10_mastering_red_y_input text, hdr10_mastering_green_x_input text, hdr10_mastering_green_y_input text, hdr10_mastering_blue_x_input text, hdr10_mastering_blue_y_input text, hdr10_mastering_white_point_x_input text, hdr10_mastering_white_point_y_input text, hdr10_mastering_min_luminance_input text, hdr10_mastering_max_luminance_input text, hdr10_max_content_light_level_input text, hdr10_max_frame_average_light_level_input text, title_input text, default_disposition_input boolean, forced_disposition_input boolean, subtitle_placement_input text, image_subtitle_action_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    stream_kind_value TEXT;
    video_pixel_format_value TEXT;
    color_range_value TEXT;
BEGIN
    stream_kind_value := lower(btrim(stream_kind_input));
    video_pixel_format_value := NULLIF(btrim(video_pixel_format_input), '');
    color_range_value := NULLIF(lower(btrim(color_range_input)), '');

    IF stream_kind_value = 'video' THEN
        IF NOT media_video_bit_depth_supported_v1(video_bit_depth_input) THEN
            RAISE EXCEPTION 'video bit depth is not supported'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF video_bit_depth_input IS NOT NULL
            AND media_video_pixel_format_bit_depth_v1(video_pixel_format_value)
                IS DISTINCT FROM video_bit_depth_input THEN
            RAISE EXCEPTION 'video bit depth must match pixel format'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
        IF NOT media_video_color_range_known_v1(color_range_value) THEN
            RAISE EXCEPTION 'video color range is not supported'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
        END IF;
    ELSIF video_bit_depth_input IS NOT NULL OR color_range_value IS NOT NULL THEN
        RAISE EXCEPTION 'video shape assigned to non-video target stream'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_video_shape_invalid';
    END IF;

    PERFORM media_desired_target_stream_append_v7(
        media_desired_target_profile_public_id_input => media_desired_target_profile_public_id_input,
        stream_key_input => stream_key_input,
        stream_kind_input => stream_kind_input,
        semantic_role_input => semantic_role_input,
        language_code_input => language_code_input,
        optional_input => optional_input,
        sort_order_input => sort_order_input,
        codec_input => codec_input,
        channel_count_input => channel_count_input,
        channel_layout_input => channel_layout_input,
        audio_bitrate_bps_input => audio_bitrate_bps_input,
        audio_sample_rate_hz_input => audio_sample_rate_hz_input,
        audio_loudness_profile_input => audio_loudness_profile_input,
        audio_dynamic_range_input => audio_dynamic_range_input,
        video_profile_input => video_profile_input,
        video_level_input => video_level_input,
        video_bitrate_bps_input => video_bitrate_bps_input,
        video_width_px_input => video_width_px_input,
        video_height_px_input => video_height_px_input,
        video_pixel_format_input => video_pixel_format_input,
        video_average_frame_rate_input => video_average_frame_rate_input,
        color_primaries_input => color_primaries_input,
        color_transfer_input => color_transfer_input,
        color_space_input => color_space_input,
        hdr_format_input => hdr_format_input,
        hdr10_mastering_red_x_input => hdr10_mastering_red_x_input,
        hdr10_mastering_red_y_input => hdr10_mastering_red_y_input,
        hdr10_mastering_green_x_input => hdr10_mastering_green_x_input,
        hdr10_mastering_green_y_input => hdr10_mastering_green_y_input,
        hdr10_mastering_blue_x_input => hdr10_mastering_blue_x_input,
        hdr10_mastering_blue_y_input => hdr10_mastering_blue_y_input,
        hdr10_mastering_white_point_x_input => hdr10_mastering_white_point_x_input,
        hdr10_mastering_white_point_y_input => hdr10_mastering_white_point_y_input,
        hdr10_mastering_min_luminance_input => hdr10_mastering_min_luminance_input,
        hdr10_mastering_max_luminance_input => hdr10_mastering_max_luminance_input,
        hdr10_max_content_light_level_input => hdr10_max_content_light_level_input,
        hdr10_max_frame_average_light_level_input => hdr10_max_frame_average_light_level_input,
        title_input => title_input,
        default_disposition_input => default_disposition_input,
        forced_disposition_input => forced_disposition_input,
        subtitle_placement_input => subtitle_placement_input,
        image_subtitle_action_input => image_subtitle_action_input
    );

    UPDATE media_desired_target_stream stream
       SET video_bit_depth = video_bit_depth_input,
           color_range = color_range_value
      FROM media_desired_target_profile target
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND stream.media_desired_target_profile_id = target.media_desired_target_profile_id
       AND lower(stream.stream_key) = lower(btrim(stream_key_input));
END;
$$;



CREATE FUNCTION public.media_desired_target_stream_count_bounded_v1(media_desired_target_profile_id_input bigint) RETURNS integer
    LANGUAGE sql STABLE PARALLEL SAFE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT count(*)::INT
      FROM (
          SELECT 1
            FROM media_desired_target_stream
           WHERE media_desired_target_profile_id = media_desired_target_profile_id_input
           LIMIT media_desired_target_stream_limit_v1() + 1
      ) bounded_streams
$$;



CREATE FUNCTION public.media_desired_target_stream_insert_guard_v1() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    target_activated_at TIMESTAMPTZ;
    target_enabled BOOLEAN;
    stream_count INT;
BEGIN
    SELECT activated_at, enabled
      INTO target_activated_at, target_enabled
      FROM media_desired_target_profile
     WHERE media_desired_target_profile_id = NEW.media_desired_target_profile_id
     FOR UPDATE;

    IF target_enabled IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;
    IF target_activated_at IS NOT NULL THEN
        RAISE EXCEPTION 'desired target version is immutable after activation'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_immutable';
    END IF;

    stream_count := media_desired_target_stream_count_bounded_v1(
        NEW.media_desired_target_profile_id
    );
    IF stream_count >= media_desired_target_stream_limit_v1() THEN
        RAISE EXCEPTION 'desired target exceeds stream limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_stream_limit_exceeded';
    END IF;

    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_desired_target_stream_limit_v1() RETURNS integer
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 1024
$$;



CREATE FUNCTION public.media_desired_target_stream_list_v1(media_desired_target_profile_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, title text, default_disposition boolean, forced_disposition boolean)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           audio.channel_count,
           audio.channel_layout,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition
      FROM media_desired_target_profile target
      JOIN media_desired_target_stream stream
        ON stream.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_audio_stream audio
        ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_desired_target_stream_list_v2(media_desired_target_profile_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           audio.channel_count,
           audio.channel_layout,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_desired_target_profile target
      JOIN media_desired_target_stream stream
        ON stream.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_audio_stream audio
        ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_desired_target_stream_list_v3(media_desired_target_profile_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, video_profile text, video_level text, video_bitrate_bps integer, color_primaries text, color_transfer text, color_space text, hdr_format text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           audio.channel_count,
           audio.channel_layout,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_desired_target_profile target
      JOIN media_desired_target_stream stream
        ON stream.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_audio_stream audio
        ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_desired_target_stream_list_v4(media_desired_target_profile_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, video_profile text, video_level text, video_bitrate_bps integer, color_primaries text, color_transfer text, color_space text, hdr_format text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           audio.channel_count,
           audio.channel_layout,
           audio.audio_bitrate_bps,
           audio.audio_sample_rate_hz,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_desired_target_profile target
      JOIN media_desired_target_stream stream
        ON stream.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_audio_stream audio
        ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_desired_target_stream_list_v5(media_desired_target_profile_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, audio_loudness_profile text, audio_dynamic_range text, video_profile text, video_level text, video_bitrate_bps integer, color_primaries text, color_transfer text, color_space text, hdr_format text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           audio.channel_count,
           audio.channel_layout,
           audio.audio_bitrate_bps,
           audio.audio_sample_rate_hz,
           audio.audio_loudness_profile,
           audio.audio_dynamic_range,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_desired_target_profile target
      JOIN media_desired_target_stream stream
        ON stream.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_audio_stream audio
        ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_desired_target_stream_list_v7(media_desired_target_profile_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, audio_loudness_profile text, audio_dynamic_range text, video_profile text, video_level text, video_bitrate_bps integer, video_width_px integer, video_height_px integer, video_pixel_format text, video_average_frame_rate text, color_primaries text, color_transfer text, color_space text, hdr_format text, hdr10_mastering_red_x text, hdr10_mastering_red_y text, hdr10_mastering_green_x text, hdr10_mastering_green_y text, hdr10_mastering_blue_x text, hdr10_mastering_blue_y text, hdr10_mastering_white_point_x text, hdr10_mastering_white_point_y text, hdr10_mastering_min_luminance text, hdr10_mastering_max_luminance text, hdr10_max_content_light_level text, hdr10_max_frame_average_light_level text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           audio.channel_count,
           audio.channel_layout,
           audio.audio_bitrate_bps,
           audio.audio_sample_rate_hz,
           audio.audio_loudness_profile,
           audio.audio_dynamic_range,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.video_width_px,
           stream.video_height_px,
           stream.video_pixel_format,
           stream.video_average_frame_rate,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.hdr10_mastering_red_x,
           stream.hdr10_mastering_red_y,
           stream.hdr10_mastering_green_x,
           stream.hdr10_mastering_green_y,
           stream.hdr10_mastering_blue_x,
           stream.hdr10_mastering_blue_y,
           stream.hdr10_mastering_white_point_x,
           stream.hdr10_mastering_white_point_y,
           stream.hdr10_mastering_min_luminance,
           stream.hdr10_mastering_max_luminance,
           stream.hdr10_max_content_light_level,
           stream.hdr10_max_frame_average_light_level,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_desired_target_profile target
      JOIN media_desired_target_stream stream
        ON stream.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_audio_stream audio
        ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_desired_target_stream_list_v8(media_desired_target_profile_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, audio_loudness_profile text, audio_dynamic_range text, video_profile text, video_level text, video_bitrate_bps integer, video_width_px integer, video_height_px integer, video_pixel_format text, video_bit_depth integer, video_average_frame_rate text, color_range text, color_primaries text, color_transfer text, color_space text, hdr_format text, hdr10_mastering_red_x text, hdr10_mastering_red_y text, hdr10_mastering_green_x text, hdr10_mastering_green_y text, hdr10_mastering_blue_x text, hdr10_mastering_blue_y text, hdr10_mastering_white_point_x text, hdr10_mastering_white_point_y text, hdr10_mastering_min_luminance text, hdr10_mastering_max_luminance text, hdr10_max_content_light_level text, hdr10_max_frame_average_light_level text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           audio.channel_count,
           audio.channel_layout,
           audio.audio_bitrate_bps,
           audio.audio_sample_rate_hz,
           audio.audio_loudness_profile,
           audio.audio_dynamic_range,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.video_width_px,
           stream.video_height_px,
           stream.video_pixel_format,
           stream.video_bit_depth,
           stream.video_average_frame_rate,
           stream.color_range,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.hdr10_mastering_red_x,
           stream.hdr10_mastering_red_y,
           stream.hdr10_mastering_green_x,
           stream.hdr10_mastering_green_y,
           stream.hdr10_mastering_blue_x,
           stream.hdr10_mastering_blue_y,
           stream.hdr10_mastering_white_point_x,
           stream.hdr10_mastering_white_point_y,
           stream.hdr10_mastering_min_luminance,
           stream.hdr10_mastering_max_luminance,
           stream.hdr10_max_content_light_level,
           stream.hdr10_max_frame_average_light_level,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_desired_target_profile target
      JOIN media_desired_target_stream stream
        ON stream.media_desired_target_profile_id = target.media_desired_target_profile_id
      LEFT JOIN media_desired_target_audio_stream audio
        ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
     WHERE target.media_desired_target_profile_public_id = media_desired_target_profile_public_id_input
       AND target.enabled
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_desired_target_validate_and_activate_v1(media_desired_target_profile_id_input bigint) RETURNS integer
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    target_enabled BOOLEAN;
    stream_count INT;
BEGIN
    SELECT enabled
      INTO target_enabled
      FROM media_desired_target_profile
     WHERE media_desired_target_profile_id = media_desired_target_profile_id_input
     FOR UPDATE;

    IF target_enabled IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION 'desired target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
    END IF;

    stream_count := media_desired_target_stream_count_bounded_v1(
        media_desired_target_profile_id_input
    );
    IF stream_count = 0 THEN
        RAISE EXCEPTION 'desired target has no streams'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_streams_required';
    END IF;
    IF stream_count > media_desired_target_stream_limit_v1() THEN
        RAISE EXCEPTION 'desired target exceeds stream limit'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_stream_limit_exceeded';
    END IF;

    UPDATE media_desired_target_profile
       SET activated_at = COALESCE(activated_at, now())
     WHERE media_desired_target_profile_id = media_desired_target_profile_id_input;

    RETURN stream_count;
END;
$$;



CREATE FUNCTION public.media_discovery_job_enqueue_v1(actor_public_id_input uuid, media_profile_public_id_input uuid, source_path_input text, output_path_input text, source_size_bytes_input bigint, source_modified_ns_input bigint, source_sha256_input text) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    profile_row media_profile%ROWTYPE;
    source_changed BOOLEAN;
    media_job_public_id_out UUID;
BEGIN
    SELECT * INTO profile_row
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_row.media_profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;
    IF NULLIF(btrim(source_path_input), '') IS NULL
       OR COALESCE(source_size_bytes_input, -1) < 0
       OR COALESCE(source_modified_ns_input, -1) < 0
       OR COALESCE(lower(btrim(source_sha256_input)), '') !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION 'invalid discovery fingerprint'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_fingerprint_invalid';
    END IF;

    WITH changed AS (
        INSERT INTO media_discovery_source_fingerprint (
            media_profile_id,
            source_path,
            source_size_bytes,
            source_modified_ns,
            source_sha256
        )
        VALUES (
            profile_row.media_profile_id,
            btrim(source_path_input),
            source_size_bytes_input,
            source_modified_ns_input,
            lower(btrim(source_sha256_input))
        )
        ON CONFLICT (media_profile_id, source_path) DO UPDATE
        SET source_size_bytes = EXCLUDED.source_size_bytes,
            source_modified_ns = EXCLUDED.source_modified_ns,
            source_sha256 = EXCLUDED.source_sha256,
            last_seen_at = now()
        WHERE media_discovery_source_fingerprint.source_size_bytes
                  IS DISTINCT FROM EXCLUDED.source_size_bytes
           OR media_discovery_source_fingerprint.source_modified_ns
                  IS DISTINCT FROM EXCLUDED.source_modified_ns
           OR media_discovery_source_fingerprint.source_sha256
                  IS DISTINCT FROM EXCLUDED.source_sha256
        RETURNING 1
    )
    SELECT EXISTS (SELECT 1 FROM changed) INTO source_changed;

    IF NOT source_changed THEN
        RETURN NULL;
    END IF;

    media_job_public_id_out := media_job_create_v1(
        actor_public_id_input,
        media_profile_public_id_input,
        source_path_input,
        output_path_input,
        profile_row.dry_run_only
    );

    UPDATE media_discovery_source_fingerprint
       SET last_media_job_public_id = media_job_public_id_out
     WHERE media_profile_id = profile_row.media_profile_id
       AND source_path = btrim(source_path_input);

    RETURN media_job_public_id_out;
END;
$_$;



CREATE FUNCTION public.media_discovery_job_enqueue_v2(actor_public_id_input uuid, media_profile_public_id_input uuid, source_path_input text, output_path_input text, source_size_bytes_input bigint, source_modified_ns_input bigint, source_sha256_input text) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    profile_row media_profile%ROWTYPE;
    source_changed BOOLEAN;
    media_job_public_id_out UUID;
BEGIN
    SELECT * INTO profile_row FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input AND deleted_at IS NULL
     FOR UPDATE;
    IF profile_row.media_profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;
    IF NULLIF(btrim(source_path_input), '') IS NULL OR COALESCE(source_size_bytes_input, -1) < 0
       OR COALESCE(source_modified_ns_input, -1) < 0
       OR COALESCE(lower(btrim(source_sha256_input)), '') !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION 'invalid discovery fingerprint'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_fingerprint_invalid';
    END IF;
    WITH changed AS (
        INSERT INTO media_discovery_source_fingerprint (
            media_profile_id, source_path, source_size_bytes, source_modified_ns, source_sha256
        ) VALUES (profile_row.media_profile_id, btrim(source_path_input), source_size_bytes_input,
                  source_modified_ns_input, lower(btrim(source_sha256_input)))
        ON CONFLICT (media_profile_id, source_path) DO UPDATE
        SET source_size_bytes = EXCLUDED.source_size_bytes,
            source_modified_ns = EXCLUDED.source_modified_ns,
            source_sha256 = EXCLUDED.source_sha256, last_seen_at = now()
        WHERE media_discovery_source_fingerprint.source_size_bytes IS DISTINCT FROM EXCLUDED.source_size_bytes
           OR media_discovery_source_fingerprint.source_modified_ns IS DISTINCT FROM EXCLUDED.source_modified_ns
           OR media_discovery_source_fingerprint.source_sha256 IS DISTINCT FROM EXCLUDED.source_sha256
        RETURNING 1
    ) SELECT EXISTS (SELECT 1 FROM changed) INTO source_changed;
    IF NOT source_changed THEN RETURN NULL; END IF;
    media_job_public_id_out := media_job_create_v1(actor_public_id_input,
        media_profile_public_id_input, source_path_input, output_path_input, profile_row.dry_run_only);
    UPDATE media_job SET discovery_source_size_bytes = source_size_bytes_input,
        discovery_source_modified_ns = source_modified_ns_input,
        discovery_source_sha256 = lower(btrim(source_sha256_input))
     WHERE media_job_public_id = media_job_public_id_out;
    UPDATE media_discovery_source_fingerprint SET last_media_job_public_id = media_job_public_id_out
     WHERE media_profile_id = profile_row.media_profile_id AND source_path = btrim(source_path_input);
    RETURN media_job_public_id_out;
END;
$_$;



CREATE FUNCTION public.media_discovery_job_enqueue_v2(actor_public_id_input uuid, media_profile_public_id_input uuid, source_path_input text, output_path_input text, source_identity_input text, source_size_bytes_input bigint, source_modified_ns_input bigint, source_changed_ns_input bigint, source_sha256_input text) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    profile_row media_profile%ROWTYPE;
    source_changed BOOLEAN;
    media_job_public_id_out UUID;
BEGIN
    SELECT * INTO profile_row
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_row.media_profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;
    IF NULLIF(btrim(source_path_input), '') IS NULL
       OR COALESCE(lower(btrim(source_identity_input)), '') !~ '^[0-9a-f]{16}:[0-9a-f]{16}$'
       OR COALESCE(source_size_bytes_input, -1) < 0
       OR COALESCE(source_modified_ns_input, -1) < 0
       OR COALESCE(source_changed_ns_input, -1) < 0
       OR COALESCE(lower(btrim(source_sha256_input)), '') !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION 'invalid discovery fingerprint'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_fingerprint_invalid';
    END IF;

    WITH changed AS (
        INSERT INTO media_discovery_source_fingerprint (
            media_profile_id, source_path, source_identity, source_size_bytes,
            source_modified_ns, source_changed_ns, source_sha256
        )
        VALUES (
            profile_row.media_profile_id, btrim(source_path_input),
            lower(btrim(source_identity_input)), source_size_bytes_input,
            source_modified_ns_input, source_changed_ns_input,
            lower(btrim(source_sha256_input))
        )
        ON CONFLICT (media_profile_id, source_path) DO UPDATE
        SET source_identity = EXCLUDED.source_identity,
            source_size_bytes = EXCLUDED.source_size_bytes,
            source_modified_ns = EXCLUDED.source_modified_ns,
            source_changed_ns = EXCLUDED.source_changed_ns,
            source_sha256 = EXCLUDED.source_sha256,
            last_seen_at = now()
        WHERE media_discovery_source_fingerprint.source_identity
                  IS DISTINCT FROM EXCLUDED.source_identity
           OR media_discovery_source_fingerprint.source_size_bytes
                  IS DISTINCT FROM EXCLUDED.source_size_bytes
           OR media_discovery_source_fingerprint.source_modified_ns
                  IS DISTINCT FROM EXCLUDED.source_modified_ns
           OR media_discovery_source_fingerprint.source_changed_ns
                  IS DISTINCT FROM EXCLUDED.source_changed_ns
           OR media_discovery_source_fingerprint.source_sha256
                  IS DISTINCT FROM EXCLUDED.source_sha256
        RETURNING 1
    )
    SELECT EXISTS (SELECT 1 FROM changed) INTO source_changed;

    IF NOT source_changed THEN
        RETURN NULL;
    END IF;

    media_job_public_id_out := media_job_create_v1(
        actor_public_id_input, media_profile_public_id_input, source_path_input,
        output_path_input, profile_row.dry_run_only
    );

    UPDATE media_discovery_source_fingerprint
       SET last_media_job_public_id = media_job_public_id_out
     WHERE media_profile_id = profile_row.media_profile_id
       AND source_path = btrim(source_path_input);

    RETURN media_job_public_id_out;
END;
$_$;



CREATE FUNCTION public.media_discovery_root_assert_current_v1(media_profile_root_public_id_input uuid, canonical_path_input text, filesystem_device_input bigint, filesystem_inode_input bigint) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    root_id_out BIGINT;
BEGIN
    SELECT root.media_profile_root_id INTO root_id_out
      FROM media_profile_root root
     WHERE root.media_profile_root_public_id = media_profile_root_public_id_input
       AND root.enabled
       AND root.identity_verified_at IS NOT NULL
       AND root.canonical_path = regexp_replace(btrim(canonical_path_input), '/+$', '')
       AND (root.filesystem_device, root.filesystem_inode)
           = (filesystem_device_input, filesystem_inode_input);
    IF root_id_out IS NULL THEN
        RAISE EXCEPTION 'filesystem identity changed'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_root_identity_changed';
    END IF;
    RETURN root_id_out;
END;
$_$;



CREATE FUNCTION public.media_discovery_schedule_claim_v1(media_discovery_schedule_public_id_input uuid, canonical_path_input text, filesystem_device_input bigint, filesystem_inode_input bigint, claimed_at_input timestamp with time zone) RETURNS TABLE(media_profile_public_id uuid, media_profile_root_public_id uuid, next_run_at timestamp with time zone)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    schedule_id BIGINT;
    root_public_id_value UUID;
    profile_public_id_value UUID;
    interval_value_value INT;
    interval_unit_value TEXT;
    next_run_at_value TIMESTAMPTZ;
BEGIN
    IF claimed_at_input IS NULL THEN
        RAISE EXCEPTION 'schedule claim timestamp is required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_schedule_claim_time_required';
    END IF;

    SELECT schedule.media_discovery_schedule_id,
           root.media_profile_root_public_id,
           profile.media_profile_public_id,
           schedule.interval_value,
           schedule.interval_unit
      INTO schedule_id, root_public_id_value, profile_public_id_value,
           interval_value_value, interval_unit_value
      FROM media_discovery_schedule schedule
      JOIN media_profile_root root
        ON root.media_profile_root_id = schedule.media_profile_root_id
      JOIN media_profile profile
        ON profile.media_profile_id = schedule.media_profile_id
     WHERE schedule.media_discovery_schedule_public_id = media_discovery_schedule_public_id_input
       AND schedule.enabled
       AND schedule.next_run_at <= claimed_at_input
       AND profile.deleted_at IS NULL
     FOR UPDATE OF schedule, root;
    IF schedule_id IS NULL THEN
        RAISE EXCEPTION 'enabled due schedule not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_schedule_not_due';
    END IF;

    PERFORM media_discovery_root_assert_current_v1(
        root_public_id_value, canonical_path_input,
        filesystem_device_input, filesystem_inode_input
    );
    next_run_at_value := claimed_at_input + CASE interval_unit_value
        WHEN 'minutes' THEN make_interval(mins => interval_value_value)
        WHEN 'hours' THEN make_interval(hours => interval_value_value)
        WHEN 'days' THEN make_interval(days => interval_value_value)
    END;
    UPDATE media_discovery_schedule schedule
       SET next_run_at = next_run_at_value
     WHERE schedule.media_discovery_schedule_id = schedule_id;

    RETURN QUERY SELECT profile_public_id_value, root_public_id_value, next_run_at_value;
END;
$$;



CREATE FUNCTION public.media_discovery_schedule_create_v1(media_profile_public_id_input uuid, media_profile_root_public_id_input uuid, interval_value_input integer, interval_unit_input text, sort_order_input integer, enabled_input boolean, next_run_at_input timestamp with time zone) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    profile_id BIGINT;
    root_id BIGINT;
    schedule_public_id_out UUID;
BEGIN
    SELECT profile.media_profile_id, root.media_profile_root_id
      INTO profile_id, root_id
      FROM media_profile profile
      JOIN media_profile_root root ON root.media_profile_id = profile.media_profile_id
     WHERE profile.media_profile_public_id = media_profile_public_id_input
       AND root.media_profile_root_public_id = media_profile_root_public_id_input
       AND root.root_kind = 'source'
       AND root.enabled
       AND root.identity_verified_at IS NOT NULL
       AND profile.deleted_at IS NULL
     FOR UPDATE OF profile, root;
    IF root_id IS NULL THEN
        RAISE EXCEPTION 'verified source root not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_root_not_verified';
    END IF;

    INSERT INTO media_discovery_schedule (
        media_profile_id,
        media_profile_root_id,
        interval_value,
        interval_unit,
        sort_order,
        enabled,
        next_run_at
    ) VALUES (
        profile_id,
        root_id,
        interval_value_input,
        lower(btrim(interval_unit_input)),
        sort_order_input,
        COALESCE(enabled_input, FALSE),
        next_run_at_input
    ) RETURNING media_discovery_schedule_public_id INTO schedule_public_id_out;

    UPDATE media_profile
       SET configuration_version = configuration_version + 1, updated_at = now()
     WHERE media_profile_id = profile_id;
    RETURN schedule_public_id_out;
END;
$$;



CREATE FUNCTION public.media_discovery_watcher_create_v1(media_profile_public_id_input uuid, media_profile_root_public_id_input uuid, debounce_millis_input integer, sort_order_input integer, enabled_input boolean) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    profile_id BIGINT;
    root_id BIGINT;
    watcher_public_id_out UUID;
BEGIN
    SELECT profile.media_profile_id, root.media_profile_root_id
      INTO profile_id, root_id
      FROM media_profile profile
      JOIN media_profile_root root ON root.media_profile_id = profile.media_profile_id
     WHERE profile.media_profile_public_id = media_profile_public_id_input
       AND root.media_profile_root_public_id = media_profile_root_public_id_input
       AND root.root_kind = 'source'
       AND root.enabled
       AND root.identity_verified_at IS NOT NULL
       AND profile.deleted_at IS NULL
     FOR UPDATE OF profile, root;
    IF root_id IS NULL THEN
        RAISE EXCEPTION 'verified source root not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_root_not_verified';
    END IF;

    INSERT INTO media_discovery_watcher (
        media_profile_id, media_profile_root_id, debounce_millis, sort_order, enabled
    ) VALUES (
        profile_id, root_id, debounce_millis_input, sort_order_input, COALESCE(enabled_input, FALSE)
    ) RETURNING media_discovery_watcher_public_id INTO watcher_public_id_out;

    UPDATE media_profile
       SET configuration_version = configuration_version + 1, updated_at = now()
     WHERE media_profile_id = profile_id;
    RETURN watcher_public_id_out;
END;
$$;



CREATE FUNCTION public.media_discovery_watcher_start_v1(media_discovery_watcher_public_id_input uuid, canonical_path_input text, filesystem_device_input bigint, filesystem_inode_input bigint) RETURNS TABLE(media_profile_public_id uuid, media_profile_root_public_id uuid)
    LANGUAGE plpgsql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    root_public_id_value UUID;
    profile_public_id_value UUID;
BEGIN
    SELECT root.media_profile_root_public_id, profile.media_profile_public_id
      INTO root_public_id_value, profile_public_id_value
      FROM media_discovery_watcher watcher
      JOIN media_profile_root root
        ON root.media_profile_root_id = watcher.media_profile_root_id
      JOIN media_profile profile
        ON profile.media_profile_id = watcher.media_profile_id
     WHERE watcher.media_discovery_watcher_public_id = media_discovery_watcher_public_id_input
       AND watcher.enabled
       AND profile.deleted_at IS NULL;
    IF root_public_id_value IS NULL THEN
        RAISE EXCEPTION 'enabled watcher not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_discovery_watcher_not_enabled';
    END IF;

    PERFORM media_discovery_root_assert_current_v1(
        root_public_id_value, canonical_path_input,
        filesystem_device_input, filesystem_inode_input
    );
    RETURN QUERY SELECT profile_public_id_value, root_public_id_value;
END;
$$;



CREATE FUNCTION public.media_display_valid_v1(value_input text) RETURNS boolean
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT value_input IS NOT NULL
       AND value_input = btrim(value_input)
       AND octet_length(value_input) BETWEEN 1 AND 256
       AND char_length(value_input) BETWEEN 1 AND 128
       AND value_input !~ '[[:cntrl:]]'
$$;



CREATE FUNCTION public.media_domain_seed_defaults() RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$DECLARE
    errcode CONSTANT text := 'P0001';

BEGIN
    INSERT INTO media_domain (media_domain_key, display_name)
    VALUES
        ('movies', 'Movies'),
        ('tv', 'TV'),
        ('audiobooks', 'Audiobooks'),
        ('ebooks', 'Ebooks'),
        ('software', 'Software'),
        ('adult_movies', 'Adult Movies'),
        ('adult_scenes', 'Adult Scenes')
    ON CONFLICT (media_domain_key) DO NOTHING;

    IF EXISTS (
        SELECT 1
        FROM media_domain
        WHERE media_domain_key::TEXT <> lower(media_domain_key::TEXT)
    ) THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = 'Failed to seed media domains',
            DETAIL = 'media_domain_key_not_lowercase';
    END IF;
END;
$$;



CREATE FUNCTION public.media_domain_to_torznab_category_delete(actor_user_public_id uuid, media_domain_key_input character varying, torznab_cat_id_input integer) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM media_domain_to_torznab_category_delete_v1(
        actor_user_public_id,
        media_domain_key_input,
        torznab_cat_id_input
    );
END;
$$;



CREATE FUNCTION public.media_domain_to_torznab_category_delete_v1(actor_user_public_id uuid, media_domain_key_input character varying, torznab_cat_id_input integer) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to delete media domain mapping';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    media_domain_id_value BIGINT;
    torznab_category_id_value BIGINT;
    normalized_media_domain VARCHAR(128);
    mapping_id BIGINT;
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

    IF media_domain_key_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'media_domain_missing';
    END IF;

    normalized_media_domain := lower(trim(media_domain_key_input));

    IF normalized_media_domain = '' OR normalized_media_domain <> media_domain_key_input THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'media_domain_key_invalid';
    END IF;

    IF torznab_cat_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'torznab_category_missing';
    END IF;

    SELECT media_domain_id
    INTO media_domain_id_value
    FROM media_domain
    WHERE media_domain_key::TEXT = normalized_media_domain;

    IF media_domain_id_value IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'media_domain_not_found';
    END IF;

    SELECT torznab_category_id
    INTO torznab_category_id_value
    FROM torznab_category
    WHERE torznab_cat_id = torznab_cat_id_input;

    IF torznab_category_id_value IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'torznab_category_not_found';
    END IF;

    SELECT media_domain_to_torznab_category_id
    INTO mapping_id
    FROM media_domain_to_torznab_category
    WHERE media_domain_id = media_domain_id_value
      AND torznab_category_id = torznab_category_id_value;

    IF mapping_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'mapping_not_found';
    END IF;

    DELETE FROM media_domain_to_torznab_category
    WHERE media_domain_to_torznab_category_id = mapping_id;

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'media_domain_to_torznab_category',
        mapping_id,
        NULL,
        'delete',
        actor_user_id,
        'media_domain_mapping_delete'
    );
END;
$$;



CREATE FUNCTION public.media_domain_to_torznab_category_upsert(actor_user_public_id uuid, media_domain_key_input character varying, torznab_cat_id_input integer, is_primary_input boolean) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM media_domain_to_torznab_category_upsert_v1(actor_user_public_id => actor_user_public_id, media_domain_key_input => media_domain_key_input, torznab_cat_id_input => torznab_cat_id_input, is_primary_input => is_primary_input);
END;
$$;



CREATE FUNCTION public.media_domain_to_torznab_category_upsert_v1(actor_user_public_id uuid, media_domain_key_input character varying, torznab_cat_id_input integer, is_primary_input boolean) RETURNS void
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    base_message CONSTANT text := 'Failed to upsert media domain mapping';
    errcode CONSTANT text := 'P0001';
    actor_user_id BIGINT;
    actor_role deployment_role;
    media_domain_id_value BIGINT;
    torznab_category_id_value BIGINT;
    normalized_media_domain VARCHAR(128);
    mapping_id BIGINT;
    resolved_primary BOOLEAN;
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

    IF media_domain_key_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'media_domain_missing';
    END IF;

    normalized_media_domain := lower(trim(media_domain_key_input));

    IF normalized_media_domain = '' OR normalized_media_domain <> media_domain_key_input THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'media_domain_key_invalid';
    END IF;

    SELECT media_domain_id
    INTO media_domain_id_value
    FROM media_domain
    WHERE media_domain_key::TEXT = normalized_media_domain;

    IF media_domain_id_value IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'unknown_key';
    END IF;

    IF torznab_cat_id_input IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'torznab_category_missing';
    END IF;

    SELECT torznab_category_id
    INTO torznab_category_id_value
    FROM torznab_category
    WHERE torznab_cat_id = torznab_cat_id_input;

    IF torznab_category_id_value IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = errcode,
            MESSAGE = base_message,
            DETAIL = 'torznab_category_not_found';
    END IF;

    resolved_primary := COALESCE(is_primary_input, FALSE);

    IF resolved_primary THEN
        UPDATE media_domain_to_torznab_category
        SET is_primary = FALSE
        WHERE media_domain_id = media_domain_id_value;
    END IF;

    INSERT INTO media_domain_to_torznab_category (
        media_domain_id,
        torznab_category_id,
        is_primary
    )
    VALUES (
        media_domain_id_value,
        torznab_category_id_value,
        resolved_primary
    )
    ON CONFLICT (media_domain_id, torznab_category_id)
    DO UPDATE SET is_primary = EXCLUDED.is_primary
    RETURNING media_domain_to_torznab_category_id INTO mapping_id;

    INSERT INTO config_audit_log (
        entity_type,
        entity_pk_bigint,
        entity_public_id,
        action,
        changed_by_user_id,
        change_summary
    )
    VALUES (
        'media_domain_to_torznab_category',
        mapping_id,
        NULL,
        'update',
        actor_user_id,
        'media_domain_mapping_upsert'
    );
END;
$$;



CREATE FUNCTION public.media_hdr10_color_volume_field_count_v1(hdr10_mastering_red_x_input text, hdr10_mastering_red_y_input text, hdr10_mastering_green_x_input text, hdr10_mastering_green_y_input text, hdr10_mastering_blue_x_input text, hdr10_mastering_blue_y_input text, hdr10_mastering_white_point_x_input text, hdr10_mastering_white_point_y_input text, hdr10_mastering_min_luminance_input text, hdr10_mastering_max_luminance_input text, hdr10_max_content_light_level_input text, hdr10_max_frame_average_light_level_input text) RETURNS integer
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT
        CASE WHEN NULLIF(btrim(hdr10_mastering_red_x_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_red_y_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_green_x_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_green_y_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_blue_x_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_blue_y_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_white_point_x_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_white_point_y_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_min_luminance_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_mastering_max_luminance_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_max_content_light_level_input), '') IS NULL THEN 0 ELSE 1 END
        + CASE WHEN NULLIF(btrim(hdr10_max_frame_average_light_level_input), '') IS NULL THEN 0 ELSE 1 END;
$$;



CREATE FUNCTION public.media_hdr10_color_volume_valid_v1(hdr10_mastering_red_x_input text, hdr10_mastering_red_y_input text, hdr10_mastering_green_x_input text, hdr10_mastering_green_y_input text, hdr10_mastering_blue_x_input text, hdr10_mastering_blue_y_input text, hdr10_mastering_white_point_x_input text, hdr10_mastering_white_point_y_input text, hdr10_mastering_min_luminance_input text, hdr10_mastering_max_luminance_input text, hdr10_max_content_light_level_input text, hdr10_max_frame_average_light_level_input text) RETURNS boolean
    LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    red_x DOUBLE PRECISION;
    red_y DOUBLE PRECISION;
    green_x DOUBLE PRECISION;
    green_y DOUBLE PRECISION;
    blue_x DOUBLE PRECISION;
    blue_y DOUBLE PRECISION;
    white_x DOUBLE PRECISION;
    white_y DOUBLE PRECISION;
    min_luminance DOUBLE PRECISION;
    max_luminance DOUBLE PRECISION;
    max_content_light_level DOUBLE PRECISION;
    max_frame_average_light_level DOUBLE PRECISION;
BEGIN
    IF media_hdr10_color_volume_field_count_v1(
        hdr10_mastering_red_x_input,
        hdr10_mastering_red_y_input,
        hdr10_mastering_green_x_input,
        hdr10_mastering_green_y_input,
        hdr10_mastering_blue_x_input,
        hdr10_mastering_blue_y_input,
        hdr10_mastering_white_point_x_input,
        hdr10_mastering_white_point_y_input,
        hdr10_mastering_min_luminance_input,
        hdr10_mastering_max_luminance_input,
        hdr10_max_content_light_level_input,
        hdr10_max_frame_average_light_level_input
    ) <> 12 THEN
        RETURN FALSE;
    END IF;

    red_x := media_hdr10_value_v1(hdr10_mastering_red_x_input);
    red_y := media_hdr10_value_v1(hdr10_mastering_red_y_input);
    green_x := media_hdr10_value_v1(hdr10_mastering_green_x_input);
    green_y := media_hdr10_value_v1(hdr10_mastering_green_y_input);
    blue_x := media_hdr10_value_v1(hdr10_mastering_blue_x_input);
    blue_y := media_hdr10_value_v1(hdr10_mastering_blue_y_input);
    white_x := media_hdr10_value_v1(hdr10_mastering_white_point_x_input);
    white_y := media_hdr10_value_v1(hdr10_mastering_white_point_y_input);
    min_luminance := media_hdr10_value_v1(hdr10_mastering_min_luminance_input);
    max_luminance := media_hdr10_value_v1(hdr10_mastering_max_luminance_input);
    max_content_light_level := media_hdr10_value_v1(hdr10_max_content_light_level_input);
    max_frame_average_light_level := media_hdr10_value_v1(
        hdr10_max_frame_average_light_level_input
    );

    IF red_x IS NULL OR red_y IS NULL
        OR green_x IS NULL OR green_y IS NULL
        OR blue_x IS NULL OR blue_y IS NULL
        OR white_x IS NULL OR white_y IS NULL
        OR min_luminance IS NULL OR max_luminance IS NULL
        OR max_content_light_level IS NULL
        OR max_frame_average_light_level IS NULL THEN
        RETURN FALSE;
    END IF;

    IF red_x <= 0.0 OR red_y <= 0.0 OR red_x + red_y > 1.0
        OR green_x <= 0.0 OR green_y <= 0.0 OR green_x + green_y > 1.0
        OR blue_x <= 0.0 OR blue_y <= 0.0 OR blue_x + blue_y > 1.0
        OR white_x <= 0.0 OR white_y <= 0.0 OR white_x + white_y > 1.0 THEN
        RETURN FALSE;
    END IF;

    IF media_hdr10_triangle_area_v1(red_x, red_y, green_x, green_y, blue_x, blue_y)
        <= 0.000000001 THEN
        RETURN FALSE;
    END IF;

    IF NOT media_hdr10_point_inside_triangle_v1(
        white_x,
        white_y,
        red_x,
        red_y,
        green_x,
        green_y,
        blue_x,
        blue_y
    ) THEN
        RETURN FALSE;
    END IF;

    RETURN min_luminance >= 0.0
        AND max_luminance > min_luminance
        AND max_content_light_level > 0.0
        AND max_frame_average_light_level > 0.0
        AND max_frame_average_light_level <= max_content_light_level;
END;
$$;



CREATE FUNCTION public.media_hdr10_point_inside_triangle_v1(point_x double precision, point_y double precision, first_x double precision, first_y double precision, second_x double precision, second_y double precision, third_x double precision, third_y double precision) RETURNS boolean
    LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    triangle_area DOUBLE PRECISION;
    first_area DOUBLE PRECISION;
    second_area DOUBLE PRECISION;
    third_area DOUBLE PRECISION;
BEGIN
    triangle_area := media_hdr10_triangle_area_v1(
        first_x,
        first_y,
        second_x,
        second_y,
        third_x,
        third_y
    );
    first_area := media_hdr10_triangle_area_v1(
        point_x,
        point_y,
        second_x,
        second_y,
        third_x,
        third_y
    );
    second_area := media_hdr10_triangle_area_v1(
        first_x,
        first_y,
        point_x,
        point_y,
        third_x,
        third_y
    );
    third_area := media_hdr10_triangle_area_v1(
        first_x,
        first_y,
        second_x,
        second_y,
        point_x,
        point_y
    );

    RETURN triangle_area > 0.000000001
        AND abs(triangle_area - (first_area + second_area + third_area)) <= 0.000001;
END;
$$;



CREATE FUNCTION public.media_hdr10_triangle_area_v1(first_x double precision, first_y double precision, second_x double precision, second_y double precision, third_x double precision, third_y double precision) RETURNS double precision
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT abs(
        ((second_x - first_x) * (third_y - first_y))
        - ((third_x - first_x) * (second_y - first_y))
    ) / 2.0;
$$;



CREATE FUNCTION public.media_hdr10_value_v1(value_input text) RETURNS double precision
    LANGUAGE plpgsql IMMUTABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    raw_value TEXT;
    parts TEXT[];
    numerator DOUBLE PRECISION;
    denominator DOUBLE PRECISION;
BEGIN
    raw_value := NULLIF(btrim(value_input), '');
    IF raw_value IS NULL THEN
        RETURN NULL;
    END IF;

    IF position('/' IN raw_value) > 0 THEN
        parts := string_to_array(raw_value, '/');
        IF array_length(parts, 1) IS DISTINCT FROM 2 THEN
            RETURN NULL;
        END IF;
        IF btrim(parts[1]) !~ '^[+-]?([0-9]+([.][0-9]+)?|[.][0-9]+)$'
            OR btrim(parts[2]) !~ '^[+-]?([0-9]+([.][0-9]+)?|[.][0-9]+)$' THEN
            RETURN NULL;
        END IF;
        numerator := btrim(parts[1])::DOUBLE PRECISION;
        denominator := btrim(parts[2])::DOUBLE PRECISION;
        IF denominator <= 0 THEN
            RETURN NULL;
        END IF;
        RETURN numerator / denominator;
    END IF;

    IF raw_value !~ '^[+-]?([0-9]+([.][0-9]+)?|[.][0-9]+)$' THEN
        RETURN NULL;
    END IF;
    RETURN raw_value::DOUBLE PRECISION;
END;
$_$;



CREATE FUNCTION public.media_job_artifact_append_v1(media_job_public_id_input uuid, claim_generation_input bigint, artifact_index_input integer, artifact_kind_input text, artifact_path_input text, size_bytes_input bigint DEFAULT NULL::bigint, content_type_input text DEFAULT NULL::text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    current_job_id BIGINT;
    current_attempt_id BIGINT;
BEGIN
    SELECT media_job_id, media_job_attempt_id
      INTO current_job_id, current_attempt_id
      FROM media_job_current_attempt_v1(media_job_public_id_input, claim_generation_input);
    IF current_attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    INSERT INTO media_job_artifact (
        media_job_id, media_job_attempt_id, artifact_index, artifact_kind,
        artifact_path, size_bytes, content_type
    ) VALUES (
        current_job_id, current_attempt_id, artifact_index_input,
        btrim(artifact_kind_input), btrim(artifact_path_input), size_bytes_input,
        NULLIF(btrim(content_type_input), '')
    )
    ON CONFLICT (media_job_attempt_id, artifact_index)
    DO UPDATE SET artifact_kind = EXCLUDED.artifact_kind,
                  artifact_path = EXCLUDED.artifact_path,
                  size_bytes = EXCLUDED.size_bytes,
                  content_type = EXCLUDED.content_type;
END;
$$;



CREATE FUNCTION public.media_job_artifact_list_v1(media_job_public_id_input uuid) RETURNS TABLE(attempt_number integer, is_current boolean, artifact_index integer, artifact_kind text, artifact_path text, size_bytes bigint, content_type text, created_at timestamp with time zone)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT attempt.attempt_number, evidence.media_job_attempt_id = job.current_attempt_id,
           evidence.artifact_index, evidence.artifact_kind, evidence.artifact_path,
           evidence.size_bytes, evidence.content_type, evidence.created_at
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_job_artifact evidence ON evidence.media_job_attempt_id = attempt.media_job_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY attempt.attempt_number DESC, evidence.artifact_index;
$$;



CREATE FUNCTION public.media_job_artifact_path_is_managed_v1(artifact_path_input text) RETURNS boolean
    LANGUAGE sql IMMUTABLE STRICT
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT
        artifact_path_input = btrim(artifact_path_input)
        AND artifact_path_input LIKE 'jobs/%'
        AND right(artifact_path_input, 1) <> '/'
        AND strpos(artifact_path_input, '//') = 0
        AND strpos(artifact_path_input, chr(92)) = 0
        AND NOT EXISTS (
            SELECT 1
            FROM unnest(string_to_array(artifact_path_input, '/')) AS segment(value)
            WHERE segment.value IN ('', '.', '..')
        );
$$;



CREATE FUNCTION public.media_job_attempt_guard_v1() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF OLD.status IN (
        media_job_status_completed_v1(),
        media_job_status_failed_v1(),
        media_job_status_cancelled_v1()
    ) THEN
        RAISE EXCEPTION 'terminal job attempts are immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_attempt_immutable';
    END IF;
    IF NEW.media_job_id IS DISTINCT FROM OLD.media_job_id
       OR NEW.attempt_number IS DISTINCT FROM OLD.attempt_number
       OR NEW.claim_generation IS DISTINCT FROM OLD.claim_generation
       OR NEW.queued_at IS DISTINCT FROM OLD.queued_at
       OR (OLD.claimed_at IS NOT NULL AND NEW.claimed_at IS DISTINCT FROM OLD.claimed_at) THEN
        RAISE EXCEPTION 'job attempt identity is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_attempt_identity_immutable';
    END IF;
    IF NOT (
        NEW.status = OLD.status
        OR (OLD.status = media_job_status_queued_v1() AND NEW.status = media_job_status_running_v1())
        OR (OLD.status = media_job_status_running_v1() AND NEW.status IN (
            media_job_status_verifying_v1(), media_job_status_completed_v1(),
            media_job_status_failed_v1(), media_job_status_cancelled_v1()
        ))
        OR (OLD.status = media_job_status_verifying_v1() AND NEW.status IN (
            media_job_status_completed_v1(), media_job_status_failed_v1(),
            media_job_status_cancelled_v1()
        ))
    ) THEN
        RAISE EXCEPTION 'job attempt status cannot regress'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_attempt_transition_invalid';
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_attempt_list_v1(media_job_public_id_input uuid) RETURNS TABLE(attempt_number integer, claim_generation bigint, status public.media_job_status, is_current boolean, queued_at timestamp with time zone, claimed_at timestamp with time zone, heartbeat_at timestamp with time zone, completed_at timestamp with time zone, last_error text)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT attempt.attempt_number, attempt.claim_generation, attempt.status,
           attempt.media_job_attempt_id = job.current_attempt_id,
           attempt.queued_at, attempt.claimed_at, attempt.heartbeat_at,
           attempt.completed_at, attempt.last_error
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY attempt.attempt_number DESC;
$$;



CREATE FUNCTION public.media_job_cancel_v1(media_job_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    UPDATE media_job
       SET status = media_job_status_cancelled_v1(),
           completed_at = now()
     WHERE media_job_public_id = media_job_public_id_input
       AND status = media_job_status_queued_v1();

    IF NOT FOUND THEN
        IF EXISTS (
            SELECT 1
              FROM media_job
             WHERE media_job_public_id = media_job_public_id_input
        ) THEN
            RAISE EXCEPTION 'job cancel blocked by status'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_cancel_invalid_status';
        END IF;
        RAISE EXCEPTION 'job not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_not_found';
    END IF;
END;
$$;



CREATE FUNCTION public.media_job_cancel_v2(media_job_public_id_input uuid) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    requested_generation BIGINT;
BEGIN
    UPDATE media_job job
       SET cancel_generation = job.cancel_generation + 1,
           cancel_acknowledged_generation = CASE
               WHEN job.status = media_job_status_queued_v1() THEN job.cancel_generation + 1
               ELSE job.cancel_acknowledged_generation
           END,
           status = CASE
               WHEN job.status = media_job_status_queued_v1() THEN media_job_status_cancelled_v1()
               ELSE job.status
           END,
           completed_at = CASE
               WHEN job.status = media_job_status_queued_v1() THEN now()
               ELSE job.completed_at
           END
     WHERE job.media_job_public_id = media_job_public_id_input
       AND (
           job.status IN (
               media_job_status_queued_v1(),
               media_job_status_running_v1(),
               media_job_status_verifying_v1()
           )
           OR (
               job.status = media_job_status_completed_v1()
               AND EXISTS (
                   SELECT 1 FROM media_job_terminal_outbox terminal_event
                    WHERE terminal_event.media_job_id = job.media_job_id
                      AND terminal_event.published_at IS NULL
               )
           )
       )
    RETURNING job.cancel_generation INTO requested_generation;
    IF requested_generation IS NULL THEN
        IF EXISTS (
            SELECT 1 FROM media_job
             WHERE media_job_public_id = media_job_public_id_input
        ) THEN
            RAISE EXCEPTION 'job cancel blocked by status'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_cancel_invalid_status';
        END IF;
        RAISE EXCEPTION 'job not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_not_found';
    END IF;
    RETURN requested_generation;
END;
$$;



CREATE FUNCTION public.media_job_capture_configuration_v1() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    INSERT INTO media_job_configuration_snapshot (
        media_job_id,
        profile_configuration_version,
        media_policy_profile_id,
        policy_version,
        media_desired_target_profile_id,
        desired_target_version
    )
    SELECT NEW.media_job_id,
           profile.configuration_version,
           NEW.intent_policy_profile_id,
           NEW.intent_policy_version,
           NEW.intent_desired_target_profile_id,
           NEW.intent_desired_target_version
      FROM media_profile profile
     WHERE profile.media_profile_id = NEW.media_profile_id;

    INSERT INTO media_job_root_snapshot (
        media_job_id, root_kind, requested_path, canonical_path,
        filesystem_device, filesystem_inode, media_type, sort_order, enabled
    )
    SELECT NEW.media_job_id, root.root_kind, root.requested_path, root.canonical_path,
           root.filesystem_device, root.filesystem_inode, root.media_type,
           root.sort_order, root.enabled
      FROM media_profile_root root
     WHERE root.media_profile_id = NEW.media_profile_id;

    INSERT INTO media_job_file_rule_snapshot (
        media_job_id, rule_kind, matcher_kind, matcher_value, sort_order, enabled
    )
    SELECT NEW.media_job_id, rule.rule_kind, rule.matcher_kind, rule.matcher_value,
           rule.sort_order, rule.enabled
      FROM media_profile_file_rule rule
     WHERE rule.media_profile_id = NEW.media_profile_id;

    INSERT INTO media_job_filter_snapshot (
        media_job_id, min_size_bytes, max_size_bytes, min_duration_millis,
        max_duration_millis, include_samples, include_trailers,
        exclude_trash, exclude_quarantine
    )
    SELECT NEW.media_job_id, filter.min_size_bytes, filter.max_size_bytes,
           filter.min_duration_millis, filter.max_duration_millis,
           filter.include_samples, filter.include_trailers,
           filter.exclude_trash, filter.exclude_quarantine
      FROM media_profile_filter filter
     WHERE filter.media_profile_id = NEW.media_profile_id;

    INSERT INTO media_job_subtitle_discovery_rule_snapshot (
        media_job_id, discovery_pattern, precedence, enabled
    )
    SELECT NEW.media_job_id, rule.discovery_pattern, rule.precedence, rule.enabled
      FROM media_subtitle_discovery_rule rule
     WHERE rule.media_profile_id = NEW.media_profile_id;

    INSERT INTO media_job_policy_retention_rule_snapshot (
        media_job_id, stream_kind, semantic_role, language_code, codec_or_format,
        action, placement, sort_order, enabled
    )
    SELECT NEW.media_job_id, rule.stream_kind, rule.semantic_role, rule.language_code,
           rule.codec_or_format, rule.action, rule.placement, rule.sort_order, rule.enabled
      FROM media_policy_retention_rule rule
     WHERE rule.media_policy_profile_id = NEW.intent_policy_profile_id;

    INSERT INTO media_job_policy_compatibility_target_snapshot (
        media_job_id, compatibility_target_key, compatibility_target_version,
        sort_order, enabled
    )
    SELECT NEW.media_job_id, target.compatibility_target_key, target.version,
           selected.sort_order, selected.enabled
      FROM media_policy_compatibility_target selected
      JOIN media_compatibility_target target
        ON target.media_compatibility_target_id = selected.media_compatibility_target_id
     WHERE selected.media_policy_profile_id = NEW.intent_policy_profile_id;

    INSERT INTO media_job_policy_operation_cost_snapshot (
        media_job_id, operation_kind, cost_weight, sort_order, enabled
    )
    SELECT NEW.media_job_id, cost.operation_kind, cost.cost_weight,
           cost.sort_order, cost.enabled
      FROM media_policy_operation_cost cost
     WHERE cost.media_policy_profile_id = NEW.intent_policy_profile_id;

    INSERT INTO media_job_stream_classification_rule_snapshot (
        media_job_id, stream_kind, semantic_role, match_kind, match_pattern,
        confidence, sort_order, enabled
    )
    SELECT NEW.media_job_id, rule.stream_kind, rule.semantic_role,
           rule.match_kind, rule.match_pattern, rule.confidence,
           rule.sort_order, rule.enabled
      FROM media_stream_classification_rule rule
     WHERE rule.media_policy_profile_id = NEW.intent_policy_profile_id;

    INSERT INTO media_job_policy_maintenance_window_snapshot (
        media_job_id, day_of_week, start_time, end_time, sort_order, enabled
    )
    SELECT NEW.media_job_id, maintenance.day_of_week, maintenance.start_time,
           maintenance.end_time, maintenance.sort_order, maintenance.enabled
      FROM media_policy_maintenance_window maintenance
     WHERE maintenance.media_policy_profile_id = NEW.intent_policy_profile_id;

    INSERT INTO media_job_policy_behavior_snapshot (
        media_job_id,
        unmatched_video_action, unmatched_audio_action, unmatched_subtitle_action,
        unmatched_attachment_action, unmatched_data_action,
        unsupported_format_action, require_all_compatibility_targets,
        max_concurrency, max_retries, max_runtime_seconds,
        max_io_megabytes_per_second, min_free_space_bytes, pause_on_battery,
        minimum_battery_percent, thermal_pressure_limit,
        pause_when_thermal_exceeded,
        dry_run, replacement_mode, quarantine_enabled,
        preserve_permissions, preserve_ownership,
        workspace_retention_hours, diagnostics_enabled, stale_cleanup_hours,
        max_workspace_bytes, backup_enabled, backup_retention_days,
        backup_min_free_space_bytes, verification_strictness,
        verification_duration_tolerance_millis, verification_mux_validation,
        verification_decode_all_streams, verification_keyframe_seek,
        verification_playback_probe
    )
    SELECT NEW.media_job_id,
           unmatched.video_action, unmatched.audio_action, unmatched.subtitle_action,
           unmatched.attachment_action, unmatched.data_action,
           compatibility.unsupported_format_action, compatibility.require_all_targets,
           runtime.max_concurrency, runtime.max_retries, runtime.max_runtime_seconds,
           runtime.max_io_megabytes_per_second, runtime.min_free_space_bytes,
           runtime.pause_on_battery, runtime.minimum_battery_percent,
           runtime.thermal_pressure_limit, runtime.pause_when_thermal_exceeded,
           output.dry_run, output.replacement_mode,
           output.quarantine_enabled, output.preserve_permissions,
           output.preserve_ownership, workspace.retention_hours,
           workspace.diagnostics_enabled, workspace.stale_cleanup_hours,
           workspace.max_workspace_bytes, backup.enabled, backup.retention_days,
           backup.min_free_space_bytes, verification.strictness,
           verification.duration_tolerance_millis, verification.mux_validation,
           verification.decode_all_streams, verification.keyframe_seek,
           verification.playback_probe
      FROM media_policy_profile policy
      LEFT JOIN media_policy_unmatched_stream_behavior unmatched
        ON unmatched.media_policy_profile_id = policy.media_policy_profile_id
      LEFT JOIN media_policy_compatibility_rule compatibility
        ON compatibility.media_policy_profile_id = policy.media_policy_profile_id
      LEFT JOIN media_policy_runtime_limit runtime
        ON runtime.media_policy_profile_id = policy.media_policy_profile_id
      LEFT JOIN media_policy_output output
        ON output.media_policy_profile_id = policy.media_policy_profile_id
      LEFT JOIN media_policy_workspace workspace
        ON workspace.media_policy_profile_id = policy.media_policy_profile_id
      LEFT JOIN media_policy_backup backup
        ON backup.media_policy_profile_id = policy.media_policy_profile_id
      LEFT JOIN media_policy_verification verification
        ON verification.media_policy_profile_id = policy.media_policy_profile_id
     WHERE policy.media_policy_profile_id = NEW.intent_policy_profile_id;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_cleanup_completed_v1(as_of_input timestamp with time zone) RETURNS integer
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    IF as_of_input IS NULL THEN
        RAISE EXCEPTION 'media cleanup timestamp is required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_cleanup_as_of_required';
    END IF;

    WITH expired_jobs AS (
        SELECT mj.media_job_id
        FROM media_job mj
        JOIN media_profile mp ON mp.media_profile_id = mj.media_profile_id
        WHERE mj.status = media_job_status_completed_v1()
          AND mj.completed_at IS NOT NULL
          AND mj.completed_at <= as_of_input - make_interval(days => mp.retention_days)
    ),
    deleted AS (
        DELETE FROM media_job mj
        USING expired_jobs ej
        WHERE mj.media_job_id = ej.media_job_id
        RETURNING mj.media_job_id
    )
    SELECT COUNT(*)::INTEGER INTO deleted_count
    FROM deleted;

    RETURN COALESCE(deleted_count, 0);
END;
$$;



CREATE FUNCTION public.media_job_cleanup_failed_terminal_diagnostics_v1(as_of_input timestamp with time zone, retention_days_input integer DEFAULT 30) RETURNS integer
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    IF as_of_input IS NULL THEN
        RAISE EXCEPTION 'media diagnostic cleanup timestamp is required'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_diagnostic_cleanup_as_of_required';
    END IF;

    IF retention_days_input IS NULL OR retention_days_input < 1 OR retention_days_input > 3650 THEN
        RAISE EXCEPTION 'media diagnostic cleanup retention is out of bounds'
            USING ERRCODE = 'P0001', DETAIL = 'media_job_diagnostic_cleanup_retention_invalid';
    END IF;

    WITH expired_jobs AS (
        SELECT media_job_id
        FROM media_job
        WHERE status IN ('failed'::media_job_status, 'cancelled'::media_job_status)
          AND completed_at IS NOT NULL
          AND completed_at <= as_of_input - make_interval(days => retention_days_input)
    ),
    deleted_violations AS (
        DELETE FROM media_job_violation mjv
        USING expired_jobs ej
        WHERE mjv.media_job_id = ej.media_job_id
        RETURNING 1
    ),
    deleted_plan_reasons AS (
        DELETE FROM media_job_plan_reason mjpr
        USING expired_jobs ej
        WHERE mjpr.media_job_id = ej.media_job_id
        RETURNING 1
    ),
    deleted_verification_checks AS (
        DELETE FROM media_job_verification_check mjvc
        USING expired_jobs ej
        WHERE mjvc.media_job_id = ej.media_job_id
        RETURNING 1
    ),
    deleted_artifacts AS (
        DELETE FROM media_job_artifact mja
        USING expired_jobs ej
        WHERE mja.media_job_id = ej.media_job_id
        RETURNING 1
    ),
    deleted_audits AS (
        DELETE FROM media_job_compact_audit mjca
        USING expired_jobs ej
        WHERE mjca.media_job_id = ej.media_job_id
        RETURNING 1
    )
    SELECT (
        (SELECT COUNT(*) FROM deleted_violations)
        + (SELECT COUNT(*) FROM deleted_plan_reasons)
        + (SELECT COUNT(*) FROM deleted_verification_checks)
        + (SELECT COUNT(*) FROM deleted_artifacts)
        + (SELECT COUNT(*) FROM deleted_audits)
    )::INTEGER
    INTO deleted_count;

    RETURN COALESCE(deleted_count, 0);
END;
$$;



CREATE FUNCTION public.media_job_compact_audit_append_v1(media_job_public_id_input uuid, claim_generation_input bigint, audit_index_input integer, fact_kind_input text, fact_text_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    current_job_id BIGINT;
    current_attempt_id BIGINT;
BEGIN
    SELECT media_job_id, media_job_attempt_id
      INTO current_job_id, current_attempt_id
      FROM media_job_current_attempt_v1(media_job_public_id_input, claim_generation_input);
    IF current_attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    INSERT INTO media_job_compact_audit (
        media_job_id, media_job_public_id, media_job_attempt_id,
        audit_index, fact_kind, fact_text
    ) VALUES (
        current_job_id, media_job_public_id_input, current_attempt_id,
        audit_index_input, btrim(fact_kind_input), btrim(fact_text_input)
    )
    ON CONFLICT (media_job_attempt_id, audit_index)
    DO UPDATE SET fact_kind = EXCLUDED.fact_kind,
                  fact_text = EXCLUDED.fact_text;
END;
$$;



CREATE FUNCTION public.media_job_compact_audit_list_v1(media_job_public_id_input uuid) RETURNS TABLE(attempt_number integer, is_current boolean, audit_index integer, fact_kind text, fact_text text, created_at timestamp with time zone)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT attempt.attempt_number, evidence.media_job_attempt_id = job.current_attempt_id,
           evidence.audit_index, evidence.fact_kind, evidence.fact_text, evidence.created_at
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_job_compact_audit evidence
        ON evidence.media_job_attempt_id = attempt.media_job_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
    UNION ALL
    SELECT archive.attempt_number, FALSE, archive.audit_index,
           archive.fact_kind, archive.fact_text, archive.created_at
      FROM media_job_compact_audit_archive archive
     WHERE archive.media_job_public_id = media_job_public_id_input
     ORDER BY attempt_number DESC, audit_index;
$$;



CREATE FUNCTION public.media_job_configuration_immutable_v1() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.media_profile_id IS DISTINCT FROM OLD.media_profile_id
       OR NEW.source_path IS DISTINCT FROM OLD.source_path
       OR NEW.output_path IS DISTINCT FROM OLD.output_path
       OR NEW.dry_run IS DISTINCT FROM OLD.dry_run
       OR NEW.intent_source_root IS DISTINCT FROM OLD.intent_source_root
       OR NEW.intent_output_root IS DISTINCT FROM OLD.intent_output_root
       OR NEW.intent_compatibility_target_key IS DISTINCT FROM OLD.intent_compatibility_target_key
       OR NEW.intent_policy_key IS DISTINCT FROM OLD.intent_policy_key
       OR NEW.intent_compatibility_target_id IS DISTINCT FROM OLD.intent_compatibility_target_id
       OR NEW.intent_compatibility_target_version IS DISTINCT FROM OLD.intent_compatibility_target_version
       OR NEW.intent_policy_profile_id IS DISTINCT FROM OLD.intent_policy_profile_id
       OR NEW.intent_policy_version IS DISTINCT FROM OLD.intent_policy_version
       OR NEW.intent_desired_target_profile_id IS DISTINCT FROM OLD.intent_desired_target_profile_id
       OR NEW.intent_desired_target_version IS DISTINCT FROM OLD.intent_desired_target_version THEN
        RAISE EXCEPTION 'job configuration snapshot is immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_configuration_immutable';
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_create_v1(actor_public_id_input uuid, media_profile_public_id_input uuid, source_path_input text, output_path_input text, dry_run_input boolean) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    profile_row media_profile%ROWTYPE;
    compatibility_row media_compatibility_target%ROWTYPE;
    desired_target_row media_desired_target_profile%ROWTYPE;
    desired_container_format TEXT;
    desired_container_metadata_policy TEXT;
    desired_container_chapter_policy TEXT;
    desired_container_attachment_policy TEXT;
    policy_row media_policy_profile%ROWTYPE;
    output_path_value TEXT;
    media_job_id_out BIGINT;
    media_job_public_id_out UUID;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    SELECT * INTO profile_row
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_row.media_profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    output_path_value := NULLIF(btrim(output_path_input), '');
    PERFORM media_job_validate_path_within_root_v1(
        source_path_input,
        profile_row.source_root,
        'media_job_source_path_outside_profile_root'
    );
    IF output_path_value IS NOT NULL THEN
        PERFORM media_job_validate_path_within_root_v1(
            output_path_value,
            profile_row.output_root,
            'media_job_output_path_outside_profile_root'
        );
    END IF;

    IF NOT EXISTS (
        SELECT 1
          FROM media_discovery_source_fingerprint fingerprint
         WHERE fingerprint.media_profile_id = profile_row.media_profile_id
           AND fingerprint.source_path = btrim(source_path_input)
    ) THEN
        RAISE EXCEPTION 'media job source fingerprint required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_source_fingerprint_required';
    END IF;

    IF profile_row.compatibility_target_key IS NOT NULL THEN
        SELECT * INTO compatibility_row
          FROM media_compatibility_target
         WHERE lower(compatibility_target_key) = lower(replace(profile_row.compatibility_target_key, '_', '-'))
           AND enabled
         ORDER BY version DESC, media_compatibility_target_id DESC
         LIMIT 1;
        IF compatibility_row.media_compatibility_target_id IS NULL THEN
            RAISE EXCEPTION 'compatibility target not found'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_not_found';
        END IF;
    END IF;

    IF profile_row.desired_target_profile_id IS NOT NULL THEN
        SELECT target.*
          INTO desired_target_row
         FROM media_desired_target_profile target
         WHERE target.media_desired_target_profile_id = profile_row.desired_target_profile_id
           AND target.enabled
         FOR SHARE;
        IF desired_target_row.media_desired_target_profile_id IS NULL THEN
            RAISE EXCEPTION 'desired target not found'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
        END IF;
        IF NOT EXISTS (
            SELECT 1
              FROM media_desired_target_stream stream
             WHERE stream.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
        ) THEN
            RAISE EXCEPTION 'desired target has no streams'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_streams_required';
        END IF;
        SELECT container.container_format,
               container.container_metadata_policy,
               container.container_chapter_policy,
               container.container_attachment_policy
          INTO desired_container_format,
               desired_container_metadata_policy,
               desired_container_chapter_policy,
               desired_container_attachment_policy
          FROM media_desired_target_container container
         WHERE container.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id;

        IF desired_container_metadata_policy = 'replace'
           AND NOT EXISTS (
               SELECT 1
                 FROM media_desired_target_container_metadata metadata
                WHERE metadata.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           ) THEN
            RAISE EXCEPTION 'replace metadata target has no metadata rows'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_required';
        END IF;

        IF desired_container_metadata_policy <> 'replace'
           AND EXISTS (
               SELECT 1
                 FROM media_desired_target_container_metadata metadata
                WHERE metadata.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           ) THEN
            RAISE EXCEPTION 'metadata rows require replace policy'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_policy_mismatch';
        END IF;

        IF desired_container_chapter_policy = 'replace'
           AND NOT EXISTS (
               SELECT 1
                 FROM media_desired_target_container_chapter chapter
                WHERE chapter.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           ) THEN
            RAISE EXCEPTION 'replace chapter target has no chapter rows'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapters_required';
        END IF;

        IF desired_container_chapter_policy <> 'replace'
           AND EXISTS (
               SELECT 1
                 FROM media_desired_target_container_chapter chapter
                WHERE chapter.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           ) THEN
            RAISE EXCEPTION 'chapter rows require replace policy'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_policy_mismatch';
        END IF;
    END IF;

    SELECT * INTO policy_row
      FROM media_policy_profile
     WHERE lower(policy_key) = lower(COALESCE(profile_row.policy_key, 'safe_dry_run'))
       AND enabled
     ORDER BY version DESC, media_policy_profile_id DESC
     LIMIT 1;
    IF policy_row.media_policy_profile_id IS NULL THEN
        RAISE EXCEPTION 'policy profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_profile_not_found';
    END IF;

    INSERT INTO media_job (
        media_profile_id, source_path, output_path, dry_run,
        intent_source_root, intent_output_root,
        intent_compatibility_target_key, intent_policy_key,
        intent_compatibility_target_id, intent_compatibility_target_version,
        intent_target_video_codec, intent_target_audio_codec,
        intent_target_audio_channels, intent_target_audio_channel_layout,
        intent_target_subtitle_policy,
        intent_policy_profile_id, intent_policy_version, intent_policy_video_intent,
        intent_desired_target_profile_id, intent_desired_target_key,
        intent_desired_target_version, intent_desired_container_format,
        intent_desired_container_metadata_policy, intent_desired_container_chapter_policy,
        intent_desired_container_attachment_policy, intent_unmatched_stream_policy,
        intent_verification_strictness,
        intent_verification_duration_tolerance_millis,
        intent_verification_mux_validation,
        intent_verification_decode_all_streams,
        intent_verification_keyframe_seek,
        intent_verification_playback_probe,
        created_by_user_id
    )
    VALUES (
        profile_row.media_profile_id, btrim(source_path_input), output_path_value,
        COALESCE(dry_run_input, TRUE), profile_row.source_root, profile_row.output_root,
        profile_row.compatibility_target_key, profile_row.policy_key,
        compatibility_row.media_compatibility_target_id, compatibility_row.version,
        compatibility_row.video_codec, compatibility_row.audio_codec,
        compatibility_row.audio_channels, compatibility_row.audio_channel_layout,
        compatibility_row.subtitle_policy,
        policy_row.media_policy_profile_id, policy_row.version, policy_row.video_intent,
        desired_target_row.media_desired_target_profile_id, desired_target_row.target_key,
        desired_target_row.version, desired_container_format,
        desired_container_metadata_policy, desired_container_chapter_policy,
        desired_container_attachment_policy, policy_row.unmatched_stream_policy,
        policy_row.verification_strictness,
        policy_row.verification_duration_tolerance_millis,
        policy_row.verification_mux_validation,
        policy_row.verification_decode_all_streams,
        policy_row.verification_keyframe_seek,
        policy_row.verification_playback_probe,
        actor_id
    )
    RETURNING media_job_id, media_job_public_id
    INTO media_job_id_out, media_job_public_id_out;

    IF desired_target_row.media_desired_target_profile_id IS NOT NULL THEN
        INSERT INTO media_job_desired_target_stream (
            media_job_id, stream_key, stream_kind, semantic_role, language_code,
            optional, sort_order, codec, channel_count, channel_layout,
            audio_bitrate_bps, audio_sample_rate_hz, audio_loudness_profile,
            audio_dynamic_range,
            video_profile, video_level, video_bitrate_bps, color_primaries,
            color_transfer, color_space, hdr_format, hdr10_mastering_red_x,
            hdr10_mastering_red_y, hdr10_mastering_green_x,
            hdr10_mastering_green_y, hdr10_mastering_blue_x,
            hdr10_mastering_blue_y, hdr10_mastering_white_point_x,
            hdr10_mastering_white_point_y, hdr10_mastering_min_luminance,
            hdr10_mastering_max_luminance, hdr10_max_content_light_level,
            hdr10_max_frame_average_light_level, title, default_disposition,
            forced_disposition, subtitle_placement, image_subtitle_action
        )
        SELECT media_job_id_out, stream.stream_key, stream.stream_kind,
               stream.semantic_role, stream.language_code, stream.optional,
               stream.sort_order, stream.codec, audio.channel_count,
               audio.channel_layout, audio.audio_bitrate_bps,
               audio.audio_sample_rate_hz, audio.audio_loudness_profile,
               audio.audio_dynamic_range, stream.video_profile, stream.video_level,
               stream.video_bitrate_bps, stream.color_primaries,
               stream.color_transfer, stream.color_space, stream.hdr_format,
               stream.hdr10_mastering_red_x, stream.hdr10_mastering_red_y,
               stream.hdr10_mastering_green_x, stream.hdr10_mastering_green_y,
               stream.hdr10_mastering_blue_x, stream.hdr10_mastering_blue_y,
               stream.hdr10_mastering_white_point_x,
               stream.hdr10_mastering_white_point_y,
               stream.hdr10_mastering_min_luminance,
               stream.hdr10_mastering_max_luminance,
               stream.hdr10_max_content_light_level,
               stream.hdr10_max_frame_average_light_level,
               stream.title, stream.default_disposition,
               stream.forced_disposition, stream.subtitle_placement,
               stream.image_subtitle_action
          FROM media_desired_target_stream stream
          LEFT JOIN media_desired_target_audio_stream audio
            ON audio.media_desired_target_stream_id = stream.media_desired_target_stream_id
         WHERE stream.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
         ORDER BY stream.sort_order
         LIMIT 129;

        INSERT INTO media_job_desired_target_metadata (
            media_job_id,
            metadata_key,
            metadata_value
        )
        SELECT media_job_id_out,
               metadata.metadata_key,
               metadata.metadata_value
         FROM media_desired_target_container_metadata metadata
         WHERE metadata.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
         ORDER BY metadata.metadata_key
         LIMIT 65;

        WITH copied_chapters AS (
            INSERT INTO media_job_desired_target_chapter (
                media_job_id,
                start_millis,
                end_millis,
                sort_order
            )
            SELECT media_job_id_out,
                   chapter.start_millis,
                   chapter.end_millis,
                   chapter.sort_order
             FROM media_desired_target_container_chapter chapter
             WHERE chapter.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
             ORDER BY chapter.start_millis
             LIMIT 1025
            RETURNING media_job_desired_target_chapter_id, start_millis
        )
        INSERT INTO media_job_desired_target_chapter_metadata (
            media_job_desired_target_chapter_id,
            metadata_key,
            metadata_value
        )
        SELECT copied.media_job_desired_target_chapter_id,
               metadata.metadata_key,
               metadata.metadata_value
          FROM copied_chapters copied
          JOIN media_desired_target_container_chapter chapter
            ON chapter.media_desired_target_profile_id = desired_target_row.media_desired_target_profile_id
           AND chapter.start_millis = copied.start_millis
          JOIN media_desired_target_container_chapter_metadata metadata
            ON metadata.media_desired_target_container_chapter_id = chapter.media_desired_target_container_chapter_id
         ORDER BY copied.start_millis, metadata.metadata_key
         LIMIT 65537;
    END IF;

    RETURN media_job_public_id_out;
END;
$$;



CREATE FUNCTION public.media_job_current_attempt_required_v1() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM media_job job
         WHERE job.media_job_id = NEW.media_job_id
           AND job.current_attempt_id IS NULL
    ) THEN
        RAISE EXCEPTION 'current job attempt is required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_current_attempt_required';
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_current_attempt_v1(media_job_public_id_input uuid, claim_generation_input bigint) RETURNS TABLE(media_job_id bigint, media_job_attempt_id bigint, attempt_number integer)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT job.media_job_id, attempt.media_job_attempt_id, attempt.attempt_number
      FROM media_job job
      JOIN media_job_attempt attempt
        ON attempt.media_job_attempt_id = job.current_attempt_id
       AND attempt.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.claim_generation = claim_generation_input
       AND attempt.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND job.status = attempt.status;
$$;



CREATE FUNCTION public.media_job_desired_target_audio_constraints_snapshot_v1() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.stream_kind = 'audio' THEN
        SELECT audio.audio_bitrate_bps,
               audio.audio_sample_rate_hz,
               audio.audio_loudness_profile,
               audio.audio_dynamic_range
          INTO NEW.audio_bitrate_bps,
               NEW.audio_sample_rate_hz,
               NEW.audio_loudness_profile,
               NEW.audio_dynamic_range
          FROM media_job job
          JOIN media_desired_target_stream target_stream
            ON target_stream.media_desired_target_profile_id = job.intent_desired_target_profile_id
           AND lower(target_stream.stream_key) = lower(NEW.stream_key)
          JOIN media_desired_target_audio_stream audio
            ON audio.media_desired_target_stream_id = target_stream.media_desired_target_stream_id
         WHERE job.media_job_id = NEW.media_job_id;
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_desired_target_chapter_list_v1(media_job_public_id_input uuid) RETURNS TABLE(start_millis bigint, end_millis bigint, metadata_key text, metadata_value text)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT chapter.start_millis,
           chapter.end_millis,
           metadata.metadata_key,
           metadata.metadata_value
      FROM media_job job
      JOIN media_job_desired_target_chapter chapter
        ON chapter.media_job_id = job.media_job_id
      LEFT JOIN media_job_desired_target_chapter_metadata metadata
        ON metadata.media_job_desired_target_chapter_id = chapter.media_job_desired_target_chapter_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY chapter.start_millis, metadata.metadata_key NULLS FIRST
     LIMIT 65537;
$$;



CREATE FUNCTION public.media_job_desired_target_metadata_list_v1(media_job_public_id_input uuid) RETURNS TABLE(metadata_key text, metadata_value text)
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT metadata.metadata_key,
           metadata.metadata_value
      FROM media_job job
      JOIN media_job_desired_target_metadata metadata
        ON metadata.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY metadata.metadata_key
     LIMIT 65;
$$;



CREATE FUNCTION public.media_job_desired_target_snapshot_guard_v1() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.intent_desired_target_profile_id IS NOT NULL THEN
        PERFORM media_desired_target_validate_and_activate_v1(
            NEW.intent_desired_target_profile_id
        );
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_desired_target_stream_list_v1(media_job_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, title text, default_disposition boolean, forced_disposition boolean)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key, stream.stream_kind, stream.semantic_role,
           stream.language_code, stream.optional, stream.sort_order, stream.codec,
           stream.channel_count, stream.channel_layout, stream.title,
           stream.default_disposition, stream.forced_disposition
      FROM media_job job
      JOIN media_job_desired_target_stream stream ON stream.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_job_desired_target_stream_list_v2(media_job_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           stream.channel_count,
           stream.channel_layout,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_job job
      JOIN media_job_desired_target_stream stream ON stream.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_job_desired_target_stream_list_v3(media_job_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, video_profile text, video_level text, video_bitrate_bps integer, color_primaries text, color_transfer text, color_space text, hdr_format text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           stream.channel_count,
           stream.channel_layout,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_job job
      JOIN media_job_desired_target_stream stream ON stream.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_job_desired_target_stream_list_v4(media_job_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, video_profile text, video_level text, video_bitrate_bps integer, color_primaries text, color_transfer text, color_space text, hdr_format text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           stream.channel_count,
           stream.channel_layout,
           stream.audio_bitrate_bps,
           stream.audio_sample_rate_hz,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_job job
      JOIN media_job_desired_target_stream stream ON stream.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_job_desired_target_stream_list_v5(media_job_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, audio_loudness_profile text, audio_dynamic_range text, video_profile text, video_level text, video_bitrate_bps integer, color_primaries text, color_transfer text, color_space text, hdr_format text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           stream.channel_count,
           stream.channel_layout,
           stream.audio_bitrate_bps,
           stream.audio_sample_rate_hz,
           stream.audio_loudness_profile,
           stream.audio_dynamic_range,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_job job
      JOIN media_job_desired_target_stream stream ON stream.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_job_desired_target_stream_list_v7(media_job_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, audio_loudness_profile text, audio_dynamic_range text, video_profile text, video_level text, video_bitrate_bps integer, video_width_px integer, video_height_px integer, video_pixel_format text, video_average_frame_rate text, color_primaries text, color_transfer text, color_space text, hdr_format text, hdr10_mastering_red_x text, hdr10_mastering_red_y text, hdr10_mastering_green_x text, hdr10_mastering_green_y text, hdr10_mastering_blue_x text, hdr10_mastering_blue_y text, hdr10_mastering_white_point_x text, hdr10_mastering_white_point_y text, hdr10_mastering_min_luminance text, hdr10_mastering_max_luminance text, hdr10_max_content_light_level text, hdr10_max_frame_average_light_level text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           stream.channel_count,
           stream.channel_layout,
           stream.audio_bitrate_bps,
           stream.audio_sample_rate_hz,
           stream.audio_loudness_profile,
           stream.audio_dynamic_range,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.video_width_px,
           stream.video_height_px,
           stream.video_pixel_format,
           stream.video_average_frame_rate,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.hdr10_mastering_red_x,
           stream.hdr10_mastering_red_y,
           stream.hdr10_mastering_green_x,
           stream.hdr10_mastering_green_y,
           stream.hdr10_mastering_blue_x,
           stream.hdr10_mastering_blue_y,
           stream.hdr10_mastering_white_point_x,
           stream.hdr10_mastering_white_point_y,
           stream.hdr10_mastering_min_luminance,
           stream.hdr10_mastering_max_luminance,
           stream.hdr10_max_content_light_level,
           stream.hdr10_max_frame_average_light_level,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_job job
      JOIN media_job_desired_target_stream stream ON stream.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_job_desired_target_stream_list_v8(media_job_public_id_input uuid) RETURNS TABLE(stream_key text, stream_kind text, semantic_role text, language_code text, optional boolean, sort_order integer, codec text, channel_count integer, channel_layout text, audio_bitrate_bps integer, audio_sample_rate_hz integer, audio_loudness_profile text, audio_dynamic_range text, video_profile text, video_level text, video_bitrate_bps integer, video_width_px integer, video_height_px integer, video_pixel_format text, video_bit_depth integer, video_average_frame_rate text, color_range text, color_primaries text, color_transfer text, color_space text, hdr_format text, hdr10_mastering_red_x text, hdr10_mastering_red_y text, hdr10_mastering_green_x text, hdr10_mastering_green_y text, hdr10_mastering_blue_x text, hdr10_mastering_blue_y text, hdr10_mastering_white_point_x text, hdr10_mastering_white_point_y text, hdr10_mastering_min_luminance text, hdr10_mastering_max_luminance text, hdr10_max_content_light_level text, hdr10_max_frame_average_light_level text, title text, default_disposition boolean, forced_disposition boolean, subtitle_placement text, image_subtitle_action text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT stream.stream_key,
           stream.stream_kind,
           stream.semantic_role,
           stream.language_code,
           stream.optional,
           stream.sort_order,
           stream.codec,
           stream.channel_count,
           stream.channel_layout,
           stream.audio_bitrate_bps,
           stream.audio_sample_rate_hz,
           stream.audio_loudness_profile,
           stream.audio_dynamic_range,
           stream.video_profile,
           stream.video_level,
           stream.video_bitrate_bps,
           stream.video_width_px,
           stream.video_height_px,
           stream.video_pixel_format,
           stream.video_bit_depth,
           stream.video_average_frame_rate,
           stream.color_range,
           stream.color_primaries,
           stream.color_transfer,
           stream.color_space,
           stream.hdr_format,
           stream.hdr10_mastering_red_x,
           stream.hdr10_mastering_red_y,
           stream.hdr10_mastering_green_x,
           stream.hdr10_mastering_green_y,
           stream.hdr10_mastering_blue_x,
           stream.hdr10_mastering_blue_y,
           stream.hdr10_mastering_white_point_x,
           stream.hdr10_mastering_white_point_y,
           stream.hdr10_mastering_min_luminance,
           stream.hdr10_mastering_max_luminance,
           stream.hdr10_max_content_light_level,
           stream.hdr10_max_frame_average_light_level,
           stream.title,
           stream.default_disposition,
           stream.forced_disposition,
           stream.subtitle_placement,
           stream.image_subtitle_action
      FROM media_job job
      JOIN media_job_desired_target_stream stream ON stream.media_job_id = job.media_job_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY stream.sort_order;
$$;



CREATE FUNCTION public.media_job_desired_target_video_technical_snapshot_v1() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.stream_kind = 'video' THEN
        SELECT target_stream.video_width_px,
               target_stream.video_height_px,
               target_stream.video_pixel_format,
               target_stream.video_bit_depth,
               target_stream.video_average_frame_rate,
               target_stream.color_range
          INTO NEW.video_width_px,
               NEW.video_height_px,
               NEW.video_pixel_format,
               NEW.video_bit_depth,
               NEW.video_average_frame_rate,
               NEW.color_range
          FROM media_job job
          JOIN media_desired_target_stream target_stream
            ON target_stream.media_desired_target_profile_id = job.intent_desired_target_profile_id
           AND lower(target_stream.stream_key) = lower(NEW.stream_key)
         WHERE job.media_job_id = NEW.media_job_id;
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_get_v1(media_job_public_id_input uuid) RETURNS TABLE(media_job_public_id uuid, source_path text, output_path text, status public.media_job_status, dry_run boolean, queued_at timestamp with time zone, started_at timestamp with time zone, completed_at timestamp with time zone, last_error text)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT
        mj.media_job_public_id,
        mj.source_path,
        mj.output_path,
        mj.status,
        mj.dry_run,
        mj.queued_at,
        mj.started_at,
        mj.completed_at,
        mj.last_error
    FROM media_job mj
    WHERE mj.media_job_public_id = media_job_public_id_input;
$$;



CREATE FUNCTION public.media_job_initial_attempt_v1() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    attempt_id_out BIGINT;
BEGIN
    INSERT INTO media_job_attempt (media_job_id, attempt_number, queued_at)
    VALUES (NEW.media_job_id, 1, NEW.queued_at)
    RETURNING media_job_attempt_id INTO attempt_id_out;

    UPDATE media_job
       SET current_attempt_id = attempt_id_out
     WHERE media_job_id = NEW.media_job_id;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_list_v1(media_profile_public_id_input uuid DEFAULT NULL::uuid, status_input public.media_job_status DEFAULT NULL::public.media_job_status, page_size_input integer DEFAULT 50, cursor_queued_at_input timestamp with time zone DEFAULT NULL::timestamp with time zone, cursor_media_job_public_id_input uuid DEFAULT NULL::uuid) RETURNS TABLE(media_job_public_id uuid, source_path text, output_path text, status public.media_job_status, dry_run boolean, queued_at timestamp with time zone, started_at timestamp with time zone, completed_at timestamp with time zone, last_error text)
    LANGUAGE plpgsql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    page_size_value INT;
    cursor_job_id BIGINT;
BEGIN
    page_size_value := COALESCE(page_size_input, 50);
    IF page_size_value < 1 OR page_size_value > 100 THEN
        RAISE EXCEPTION 'media job page size is outside bounds'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_page_size_invalid';
    END IF;
    IF (cursor_queued_at_input IS NULL) <> (cursor_media_job_public_id_input IS NULL) THEN
        RAISE EXCEPTION 'media job cursor is incomplete'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_cursor_incomplete';
    END IF;
    IF cursor_media_job_public_id_input IS NOT NULL THEN
        SELECT job.media_job_id INTO cursor_job_id
          FROM media_job job
         WHERE job.media_job_public_id = cursor_media_job_public_id_input
           AND job.queued_at = cursor_queued_at_input;
        IF cursor_job_id IS NULL THEN
            RAISE EXCEPTION 'media job cursor is invalid'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_cursor_invalid';
        END IF;
    END IF;

    RETURN QUERY
    SELECT job.media_job_public_id, job.source_path, job.output_path,
           job.status, job.dry_run, job.queued_at, job.started_at,
           job.completed_at, job.last_error
      FROM media_job job
      JOIN media_profile profile ON profile.media_profile_id = job.media_profile_id
     WHERE profile.deleted_at IS NULL
       AND (media_profile_public_id_input IS NULL
            OR profile.media_profile_public_id = media_profile_public_id_input)
       AND (status_input IS NULL OR job.status = status_input)
       AND (cursor_job_id IS NULL
            OR (job.queued_at, job.media_job_id) < (cursor_queued_at_input, cursor_job_id))
     ORDER BY job.queued_at DESC, job.media_job_id DESC
     LIMIT page_size_value;
END;
$$;



CREATE FUNCTION public.media_job_mark_completed_v1(media_job_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    affected_count INTEGER;
    current_status media_job_status;
BEGIN
    UPDATE media_job
       SET status = media_job_status_completed_v1(),
           completed_at = now(),
           last_error = NULL
     WHERE media_job_public_id = media_job_public_id_input
       AND status IN (
           media_job_status_queued_v1(),
           media_job_status_running_v1(),
           media_job_status_verifying_v1()
       );

    GET DIAGNOSTICS affected_count = ROW_COUNT;
    IF affected_count > 0 THEN
        RETURN;
    END IF;

    SELECT status INTO current_status
    FROM media_job
    WHERE media_job_public_id = media_job_public_id_input;

    IF current_status IS NULL THEN
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_not_found';
    END IF;

    IF current_status = media_job_status_completed_v1() THEN
        RETURN;
    END IF;

    RAISE EXCEPTION 'media job cannot be completed from current status'
        USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_complete_invalid_status';
END;
$$;



CREATE FUNCTION public.media_job_normalized_absolute_path_v1(path_input text) RETURNS text
    LANGUAGE plpgsql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    trimmed_value TEXT;
    component_value TEXT;
    normalized_value TEXT := '';
BEGIN
    trimmed_value := btrim(path_input);

    IF trimmed_value IS NULL
       OR trimmed_value = ''
       OR left(trimmed_value, 1) <> '/' THEN
        RETURN NULL;
    END IF;

    FOREACH component_value IN ARRAY regexp_split_to_array(trimmed_value, '/+')
    LOOP
        IF component_value = ''
           OR component_value = '.' THEN
            CONTINUE;
        END IF;

        IF component_value = '..' THEN
            RETURN NULL;
        END IF;

        normalized_value := normalized_value || '/' || component_value;
    END LOOP;

    IF normalized_value = '' THEN
        RETURN '/';
    END IF;

    RETURN normalized_value;
END;
$$;



CREATE FUNCTION public.media_job_operation_append_v1(media_job_public_id_input uuid, claim_generation_input bigint, operation_index_input integer, operation_kind_input text, stream_id_input integer, command_bin_input text, arg_1_input text, arg_2_input text, arg_3_input text, arg_4_input text, arg_5_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    current_job_id BIGINT;
    current_attempt_id BIGINT;
BEGIN
    SELECT media_job_id, media_job_attempt_id
      INTO current_job_id, current_attempt_id
      FROM media_job_current_attempt_v1(media_job_public_id_input, claim_generation_input);
    IF current_attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    INSERT INTO media_job_operation (
        media_job_id, media_job_attempt_id, operation_index, operation_kind,
        stream_id, command_bin, arg_1, arg_2, arg_3, arg_4, arg_5
    ) VALUES (
        current_job_id, current_attempt_id, operation_index_input,
        btrim(operation_kind_input), stream_id_input, btrim(command_bin_input),
        NULLIF(btrim(arg_1_input), ''), NULLIF(btrim(arg_2_input), ''),
        NULLIF(btrim(arg_3_input), ''), NULLIF(btrim(arg_4_input), ''),
        NULLIF(btrim(arg_5_input), '')
    )
    ON CONFLICT (media_job_attempt_id, operation_index)
    DO UPDATE SET operation_kind = EXCLUDED.operation_kind,
                  stream_id = EXCLUDED.stream_id,
                  command_bin = EXCLUDED.command_bin,
                  arg_1 = EXCLUDED.arg_1,
                  arg_2 = EXCLUDED.arg_2,
                  arg_3 = EXCLUDED.arg_3,
                  arg_4 = EXCLUDED.arg_4,
                  arg_5 = EXCLUDED.arg_5;
END;
$$;



CREATE FUNCTION public.media_job_operation_list_v1(media_job_public_id_input uuid) RETURNS TABLE(attempt_number integer, is_current boolean, operation_index integer, operation_kind text, stream_id integer, command_bin text, arg_1 text, arg_2 text, arg_3 text, arg_4 text, arg_5 text, created_at timestamp with time zone)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT attempt.attempt_number, evidence.media_job_attempt_id = job.current_attempt_id,
           evidence.operation_index, evidence.operation_kind, evidence.stream_id,
           evidence.command_bin, evidence.arg_1, evidence.arg_2, evidence.arg_3,
           evidence.arg_4, evidence.arg_5, evidence.created_at
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_job_operation evidence ON evidence.media_job_attempt_id = attempt.media_job_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY attempt.attempt_number DESC, evidence.operation_index;
$$;



CREATE FUNCTION public.media_job_phase_append_v1(media_job_public_id_input uuid, claim_generation_input bigint, phase_index_input integer, phase_name_input text, phase_status_input text, details_text_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    current_job_id BIGINT;
    current_attempt_id BIGINT;
BEGIN
    SELECT media_job_id, media_job_attempt_id
      INTO current_job_id, current_attempt_id
      FROM media_job_current_attempt_v1(media_job_public_id_input, claim_generation_input);
    IF current_attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    INSERT INTO media_job_phase (
        media_job_id, media_job_attempt_id, phase_index, phase_name, phase_status, details_text
    ) VALUES (
        current_job_id, current_attempt_id, phase_index_input, btrim(phase_name_input),
        phase_status_input::media_job_status, NULLIF(btrim(details_text_input), '')
    )
    ON CONFLICT (media_job_attempt_id, phase_index)
    DO UPDATE SET phase_name = EXCLUDED.phase_name,
                  phase_status = EXCLUDED.phase_status,
                  details_text = EXCLUDED.details_text;
END;
$$;



CREATE FUNCTION public.media_job_phase_list_v1(media_job_public_id_input uuid) RETURNS TABLE(attempt_number integer, is_current boolean, phase_index integer, phase_name text, phase_status public.media_job_status, details_text text, created_at timestamp with time zone)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT attempt.attempt_number, evidence.media_job_attempt_id = job.current_attempt_id,
           evidence.phase_index, evidence.phase_name, evidence.phase_status,
           evidence.details_text, evidence.created_at
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_job_phase evidence ON evidence.media_job_attempt_id = attempt.media_job_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY attempt.attempt_number DESC, evidence.phase_index;
$$;



CREATE FUNCTION public.media_job_plan_reason_append_v1(media_job_public_id_input uuid, claim_generation_input bigint, reason_index_input integer, candidate_index_input integer, selected_input boolean, reason_code_input text, reason_text_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    current_job_id BIGINT;
    current_attempt_id BIGINT;
BEGIN
    SELECT media_job_id, media_job_attempt_id
      INTO current_job_id, current_attempt_id
      FROM media_job_current_attempt_v1(media_job_public_id_input, claim_generation_input);
    IF current_attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    INSERT INTO media_job_plan_reason (
        media_job_id, media_job_attempt_id, reason_index, candidate_index,
        selected, reason_code, reason_text
    ) VALUES (
        current_job_id, current_attempt_id, reason_index_input, candidate_index_input,
        COALESCE(selected_input, FALSE), btrim(reason_code_input), btrim(reason_text_input)
    )
    ON CONFLICT (media_job_attempt_id, reason_index)
    DO UPDATE SET candidate_index = EXCLUDED.candidate_index,
                  selected = EXCLUDED.selected,
                  reason_code = EXCLUDED.reason_code,
                  reason_text = EXCLUDED.reason_text;
END;
$$;



CREATE FUNCTION public.media_job_plan_reason_list_v1(media_job_public_id_input uuid) RETURNS TABLE(attempt_number integer, is_current boolean, reason_index integer, candidate_index integer, selected boolean, reason_code text, reason_text text, created_at timestamp with time zone)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT attempt.attempt_number, evidence.media_job_attempt_id = job.current_attempt_id,
           evidence.reason_index, evidence.candidate_index, evidence.selected,
           evidence.reason_code, evidence.reason_text, evidence.created_at
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_job_plan_reason evidence ON evidence.media_job_attempt_id = attempt.media_job_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY attempt.attempt_number DESC, evidence.reason_index;
$$;



CREATE FUNCTION public.media_job_recent_page_v1(limit_input integer, cursor_queued_at_input timestamp with time zone DEFAULT NULL::timestamp with time zone, cursor_public_id_input uuid DEFAULT NULL::uuid, media_profile_public_id_input uuid DEFAULT NULL::uuid) RETURNS TABLE(media_job_public_id uuid, media_profile_public_id uuid, source_path text, output_path text, status_text text, dry_run boolean, queued_at timestamp with time zone, started_at timestamp with time zone, completed_at timestamp with time zone, last_error text, operation_count bigint, violation_count bigint, plan_reason_count bigint, verification_check_count bigint, artifact_count bigint, compact_audit_count bigint)
    LANGUAGE plpgsql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF limit_input < 1 OR limit_input > 100 THEN
        RAISE EXCEPTION 'recent media job page limit is outside 1..100'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_recent_limit_invalid';
    END IF;
    IF (cursor_queued_at_input IS NULL) <> (cursor_public_id_input IS NULL) THEN
        RAISE EXCEPTION 'recent media job cursor is incomplete'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_recent_cursor_invalid';
    END IF;
    RETURN QUERY
    WITH page AS MATERIALIZED (
        SELECT job.media_job_id, job.media_job_public_id, profile.media_profile_public_id,
               job.source_path, job.output_path, job.status::TEXT AS status_text, job.dry_run,
               job.queued_at, job.started_at, job.completed_at, job.last_error
          FROM media_job job
          JOIN media_profile profile ON profile.media_profile_id = job.media_profile_id
         WHERE (media_profile_public_id_input IS NULL
                OR profile.media_profile_public_id = media_profile_public_id_input)
           AND (cursor_queued_at_input IS NULL
                OR (job.queued_at, job.media_job_public_id)
                   < (cursor_queued_at_input, cursor_public_id_input))
         ORDER BY job.queued_at DESC, job.media_job_public_id DESC
         LIMIT limit_input + 1
    ), diagnostics AS (
        SELECT child.media_job_id, 'operation'::TEXT AS kind FROM media_job_operation child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'violation' FROM media_job_violation child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'plan_reason' FROM media_job_plan_reason child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'verification_check' FROM media_job_verification_check child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'artifact' FROM media_job_artifact child JOIN page USING (media_job_id)
        UNION ALL SELECT child.media_job_id, 'compact_audit' FROM media_job_compact_audit child JOIN page USING (media_job_id)
    ), counts AS (
        SELECT diagnostics.media_job_id,
               count(*) FILTER (WHERE kind = 'operation') AS operations,
               count(*) FILTER (WHERE kind = 'violation') AS violations,
               count(*) FILTER (WHERE kind = 'plan_reason') AS plan_reasons,
               count(*) FILTER (WHERE kind = 'verification_check') AS verification_checks,
               count(*) FILTER (WHERE kind = 'artifact') AS artifacts,
               count(*) FILTER (WHERE kind = 'compact_audit') AS compact_audits
          FROM diagnostics GROUP BY diagnostics.media_job_id
    )
    SELECT page.media_job_public_id, page.media_profile_public_id, page.source_path,
           page.output_path, page.status_text, page.dry_run, page.queued_at, page.started_at,
           page.completed_at, page.last_error, coalesce(counts.operations, 0),
           coalesce(counts.violations, 0), coalesce(counts.plan_reasons, 0),
           coalesce(counts.verification_checks, 0), coalesce(counts.artifacts, 0),
           coalesce(counts.compact_audits, 0)
      FROM page LEFT JOIN counts USING (media_job_id)
     ORDER BY page.queued_at DESC, page.media_job_public_id DESC;
END;
$$;



CREATE FUNCTION public.media_job_retention_batch_limit_v1() RETURNS integer
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 100
$$;



CREATE FUNCTION public.media_job_retention_policy_get_v1() RETURNS TABLE(completed_retention_days integer, failed_diagnostic_retention_days integer)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT policy.completed_limit,
           policy.failed_diagnostic_limit
    FROM media_job_retention_policy policy
    WHERE lower(policy.policy_key) = media_retention_policy_default_v1()
      AND policy.enabled
    ORDER BY policy.updated_at DESC, policy.media_job_retention_policy_id DESC
    LIMIT 1;
$$;



CREATE FUNCTION public.media_job_retention_policy_get_v2() RETURNS TABLE(completed_enabled boolean, completed_mode text, completed_limit integer, failed_diagnostic_enabled boolean, failed_diagnostic_mode text, failed_diagnostic_limit integer)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT
        policy.completed_enabled,
        policy.completed_mode,
        policy.completed_limit,
        policy.failed_diagnostic_enabled,
        policy.failed_diagnostic_mode,
        policy.failed_diagnostic_limit
    FROM media_job_retention_policy policy
    WHERE lower(policy.policy_key) = media_retention_policy_default_v1()
      AND policy.enabled
    ORDER BY policy.updated_at DESC, policy.media_job_retention_policy_id DESC
    LIMIT 1;
$$;



CREATE FUNCTION public.media_job_retention_policy_update_v1(actor_public_id_input uuid, completed_retention_days_input integer, failed_diagnostic_retention_days_input integer) RETURNS TABLE(completed_retention_days integer, failed_diagnostic_retention_days integer)
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT policy.completed_limit,
           policy.failed_diagnostic_limit
    FROM media_job_retention_policy_update_v2(
        actor_public_id_input,
        TRUE,
        media_retention_mode_age_v1(),
        completed_retention_days_input,
        TRUE,
        media_retention_mode_age_v1(),
        failed_diagnostic_retention_days_input
    ) policy;
$$;



CREATE FUNCTION public.media_job_retention_policy_update_v2(actor_public_id_input uuid, completed_enabled_input boolean, completed_mode_input text, completed_limit_input integer, failed_diagnostic_enabled_input boolean, failed_diagnostic_mode_input text, failed_diagnostic_limit_input integer) RETURNS TABLE(completed_enabled boolean, completed_mode text, completed_limit integer, failed_diagnostic_enabled boolean, failed_diagnostic_mode text, failed_diagnostic_limit integer)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    normalized_completed_mode TEXT;
    normalized_failed_mode TEXT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    normalized_completed_mode := lower(btrim(completed_mode_input));
    normalized_failed_mode := lower(btrim(failed_diagnostic_mode_input));

    IF normalized_completed_mode NOT IN (
        media_retention_mode_age_v1(), media_retention_mode_count_v1()
    ) THEN
        RAISE EXCEPTION 'completed retention mode is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_completed_retention_mode_invalid';
    END IF;

    IF normalized_failed_mode NOT IN (
        media_retention_mode_age_v1(), media_retention_mode_count_v1()
    ) THEN
        RAISE EXCEPTION 'failed diagnostic retention mode is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_failed_retention_mode_invalid';
    END IF;

    IF completed_limit_input IS NULL OR completed_limit_input NOT BETWEEN 1 AND 3650 THEN
        RAISE EXCEPTION 'completed retention limit is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_completed_retention_limit_invalid';
    END IF;

    IF failed_diagnostic_limit_input IS NULL OR failed_diagnostic_limit_input NOT BETWEEN 1 AND 3650 THEN
        RAISE EXCEPTION 'failed diagnostic retention limit is invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_failed_retention_limit_invalid';
    END IF;

    RETURN QUERY
    INSERT INTO media_job_retention_policy (
        policy_key,
        completed_retention_days,
        failed_diagnostic_retention_days,
        completed_enabled,
        completed_mode,
        completed_limit,
        failed_diagnostic_enabled,
        failed_diagnostic_mode,
        failed_diagnostic_limit,
        enabled,
        updated_at
    )
    VALUES (
        media_retention_policy_default_v1(),
        completed_limit_input,
        failed_diagnostic_limit_input,
        completed_enabled_input,
        normalized_completed_mode,
        completed_limit_input,
        failed_diagnostic_enabled_input,
        normalized_failed_mode,
        failed_diagnostic_limit_input,
        TRUE,
        now()
    )
    ON CONFLICT (lower(policy_key)) DO UPDATE SET
        completed_retention_days = EXCLUDED.completed_retention_days,
        failed_diagnostic_retention_days = EXCLUDED.failed_diagnostic_retention_days,
        completed_enabled = EXCLUDED.completed_enabled,
        completed_mode = EXCLUDED.completed_mode,
        completed_limit = EXCLUDED.completed_limit,
        failed_diagnostic_enabled = EXCLUDED.failed_diagnostic_enabled,
        failed_diagnostic_mode = EXCLUDED.failed_diagnostic_mode,
        failed_diagnostic_limit = EXCLUDED.failed_diagnostic_limit,
        enabled = TRUE,
        updated_at = now()
    RETURNING
        media_job_retention_policy.completed_enabled,
        media_job_retention_policy.completed_mode,
        media_job_retention_policy.completed_limit,
        media_job_retention_policy.failed_diagnostic_enabled,
        media_job_retention_policy.failed_diagnostic_mode,
        media_job_retention_policy.failed_diagnostic_limit;
END;
$$;



CREATE FUNCTION public.media_job_retention_run_v1(as_of_input timestamp with time zone) RETURNS TABLE(completed_jobs_deleted integer, failed_jobs_pruned integer, failed_detail_rows_deleted integer)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy media_job_retention_policy%ROWTYPE;
    completed_ids BIGINT[] := ARRAY[]::BIGINT[];
    failed_ids BIGINT[] := ARRAY[]::BIGINT[];
    completed_boundary_at TIMESTAMPTZ;
    completed_boundary_id BIGINT;
    failed_boundary_at TIMESTAMPTZ;
    failed_boundary_id BIGINT;
    completed_count INT := 0;
    failed_count INT := 0;
    detail_count INT := 0;
    affected_count INT := 0;
BEGIN
    IF as_of_input IS NULL THEN
        RAISE EXCEPTION 'media retention timestamp is required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_retention_as_of_required';
    END IF;

    SELECT * INTO policy
      FROM media_job_retention_policy retention
     WHERE lower(retention.policy_key) = media_retention_policy_default_v1()
       AND retention.enabled
     ORDER BY retention.updated_at DESC, retention.media_job_retention_policy_id DESC
     LIMIT 1;
    IF policy.media_job_retention_policy_id IS NULL THEN
        RAISE EXCEPTION 'media retention policy is missing'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_retention_policy_missing';
    END IF;

    IF policy.completed_enabled AND policy.completed_mode = media_retention_mode_count_v1() THEN
        SELECT job.completed_at, job.media_job_id
          INTO completed_boundary_at, completed_boundary_id
          FROM media_job job
         WHERE job.status = media_job_status_completed_v1()
           AND job.completed_at IS NOT NULL
         ORDER BY job.completed_at DESC, job.media_job_id DESC
         OFFSET GREATEST(policy.completed_limit - 1, 0)
         LIMIT 1;
    END IF;

    IF policy.completed_enabled THEN
        SELECT COALESCE(array_agg(candidate.media_job_id), ARRAY[]::BIGINT[])
          INTO completed_ids
          FROM (
              SELECT job.media_job_id
                FROM media_job job
               WHERE job.status = media_job_status_completed_v1()
                 AND job.completed_at IS NOT NULL
                 AND (
                     (policy.completed_mode = media_retention_mode_age_v1()
                         AND job.completed_at <= as_of_input - make_interval(days => policy.completed_limit))
                     OR (policy.completed_mode = media_retention_mode_count_v1()
                         AND completed_boundary_id IS NOT NULL
                         AND (job.completed_at, job.media_job_id)
                             < (completed_boundary_at, completed_boundary_id))
                 )
               ORDER BY job.completed_at, job.media_job_id
               FOR UPDATE SKIP LOCKED
               LIMIT media_job_retention_batch_limit_v1()
          ) candidate;

        INSERT INTO media_job_compact_audit_archive (
            media_job_public_id, attempt_number, audit_index,
            fact_kind, fact_text, created_at
        )
        SELECT audit.media_job_public_id, attempt.attempt_number, audit.audit_index,
               audit.fact_kind, audit.fact_text, audit.created_at
          FROM media_job_compact_audit audit
          JOIN media_job_attempt attempt
            ON attempt.media_job_attempt_id = audit.media_job_attempt_id
         WHERE audit.media_job_id = ANY(completed_ids)
        ON CONFLICT (media_job_public_id, attempt_number, audit_index)
        DO NOTHING;

        DELETE FROM media_job WHERE media_job_id = ANY(completed_ids);
        GET DIAGNOSTICS completed_count = ROW_COUNT;
    END IF;

    IF policy.failed_diagnostic_enabled
       AND policy.failed_diagnostic_mode = media_retention_mode_count_v1() THEN
        SELECT job.completed_at, job.media_job_id
          INTO failed_boundary_at, failed_boundary_id
          FROM media_job job
         WHERE job.status IN (media_job_status_failed_v1(), media_job_status_cancelled_v1())
           AND job.completed_at IS NOT NULL
         ORDER BY job.completed_at DESC, job.media_job_id DESC
         OFFSET GREATEST(policy.failed_diagnostic_limit - 1, 0)
         LIMIT 1;
    END IF;

    IF policy.failed_diagnostic_enabled THEN
        SELECT COALESCE(array_agg(candidate.media_job_id), ARRAY[]::BIGINT[])
          INTO failed_ids
          FROM (
              SELECT job.media_job_id
                FROM media_job job
               WHERE job.status IN (media_job_status_failed_v1(), media_job_status_cancelled_v1())
                 AND job.completed_at IS NOT NULL
                 AND job.diagnostics_pruned_at IS NULL
                 AND (
                     (policy.failed_diagnostic_mode = media_retention_mode_age_v1()
                         AND job.completed_at <= as_of_input - make_interval(days => policy.failed_diagnostic_limit))
                     OR (policy.failed_diagnostic_mode = media_retention_mode_count_v1()
                         AND failed_boundary_id IS NOT NULL
                         AND (job.completed_at, job.media_job_id)
                             < (failed_boundary_at, failed_boundary_id))
                 )
               ORDER BY job.completed_at, job.media_job_id
               FOR UPDATE SKIP LOCKED
               LIMIT media_job_retention_batch_limit_v1()
          ) candidate;

        DELETE FROM media_job_phase WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;
        DELETE FROM media_job_operation WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;
        DELETE FROM media_job_violation WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;
        DELETE FROM media_job_plan_reason WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;
        DELETE FROM media_job_verification_check WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;
        DELETE FROM media_job_artifact WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;
        DELETE FROM media_job_desired_target_stream WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS affected_count = ROW_COUNT;
        detail_count := detail_count + affected_count;

        UPDATE media_job
           SET last_error = NULL,
               diagnostics_pruned_at = COALESCE(diagnostics_pruned_at, as_of_input)
         WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS failed_count = ROW_COUNT;
    END IF;

    RETURN QUERY SELECT completed_count, failed_count, detail_count;
END;
$$;



CREATE FUNCTION public.media_job_retry_v1(media_job_public_id_input uuid) RETURNS integer
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    job_id BIGINT;
    attempt_number_out INT;
    attempt_id_out BIGINT;
BEGIN
    SELECT job.media_job_id INTO job_id
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
       AND job.status IN (media_job_status_failed_v1(), media_job_status_cancelled_v1())
       AND attempt.status = job.status
     FOR UPDATE OF job, attempt;
    IF job_id IS NULL THEN
        RAISE EXCEPTION 'job retry blocked by status'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_retry_invalid_status';
    END IF;

    SELECT COALESCE(max(attempt_number), 0) + 1 INTO attempt_number_out
      FROM media_job_attempt WHERE media_job_id = job_id;
    INSERT INTO media_job_attempt (media_job_id, attempt_number)
    VALUES (job_id, attempt_number_out)
    RETURNING media_job_attempt_id INTO attempt_id_out;

    UPDATE media_job
       SET current_attempt_id = attempt_id_out,
           status = media_job_status_queued_v1(),
           queued_at = now(),
           started_at = NULL,
           heartbeat_at = NULL,
           completed_at = NULL,
           last_error = NULL,
           diagnostics_pruned_at = NULL,
           cancel_acknowledged_generation = cancel_generation
     WHERE media_job_id = job_id;
    RETURN attempt_number_out;
END;
$$;



CREATE FUNCTION public.media_job_snapshot_source_fingerprint_v1() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    fingerprint_row media_discovery_source_fingerprint%ROWTYPE;
BEGIN
    SELECT fingerprint.*
      INTO fingerprint_row
      FROM media_discovery_source_fingerprint fingerprint
     WHERE fingerprint.media_profile_id = NEW.media_profile_id
       AND fingerprint.source_path = NEW.source_path;

    IF fingerprint_row.media_discovery_source_fingerprint_id IS NULL
       OR fingerprint_row.source_identity IS NULL
       OR fingerprint_row.source_changed_ns IS NULL THEN
        RAISE EXCEPTION 'media job source fingerprint required'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_source_fingerprint_required';
    END IF;

    NEW.intent_source_identity := fingerprint_row.source_identity;
    NEW.intent_source_size_bytes := fingerprint_row.source_size_bytes;
    NEW.intent_source_modified_ns := fingerprint_row.source_modified_ns;
    NEW.intent_source_changed_ns := fingerprint_row.source_changed_ns;
    NEW.intent_source_sha256 := fingerprint_row.source_sha256;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_snapshot_update_rejected_v1() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF TG_OP = 'DELETE' AND NOT EXISTS (
        SELECT 1 FROM media_job job WHERE job.media_job_id = OLD.media_job_id
    ) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'job snapshots are immutable'
        USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_snapshot_immutable';
END;
$$;



CREATE FUNCTION public.media_job_status_cancelled_v1() RETURNS public.media_job_status
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'cancelled'::media_job_status
$$;



CREATE FUNCTION public.media_job_status_completed_v1() RETURNS public.media_job_status
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'completed'::media_job_status
$$;



CREATE FUNCTION public.media_job_status_failed_v1() RETURNS public.media_job_status
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'failed'::media_job_status
$$;



CREATE FUNCTION public.media_job_status_queued_v1() RETURNS public.media_job_status
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'queued'::media_job_status
$$;



CREATE FUNCTION public.media_job_status_running_v1() RETURNS public.media_job_status
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'running'::media_job_status
$$;



CREATE FUNCTION public.media_job_status_verifying_v1() RETURNS public.media_job_status
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'verifying'::media_job_status
$$;



CREATE FUNCTION public.media_job_terminal_outbox_list_unpublished_v1() RETURNS TABLE(media_job_public_id uuid, event_kind text)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT job.media_job_public_id, outbox.event_kind
      FROM media_job_terminal_outbox outbox
      JOIN media_job job ON job.media_job_id = outbox.media_job_id
     WHERE outbox.published_at IS NULL
     ORDER BY outbox.created_at ASC, outbox.media_job_terminal_outbox_id ASC
     LIMIT 1024
$$;



CREATE FUNCTION public.media_job_terminal_outbox_mark_published_v1(media_job_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    UPDATE media_job_terminal_outbox outbox
       SET published_at = COALESCE(outbox.published_at, now())
      FROM media_job job
     WHERE job.media_job_id = outbox.media_job_id
       AND job.media_job_public_id = media_job_public_id_input;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'media job terminal outbox row not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_terminal_outbox_not_found';
    END IF;
END;
$$;



CREATE FUNCTION public.media_job_unmatched_stream_actions_fill_v1() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_row media_policy_profile%ROWTYPE;
BEGIN
    IF NEW.intent_policy_profile_id IS NOT NULL THEN
        SELECT *
          INTO policy_row
          FROM media_policy_profile
         WHERE media_policy_profile_id = NEW.intent_policy_profile_id;
    END IF;

    IF policy_row.media_policy_profile_id IS NOT NULL THEN
        NEW.intent_unmatched_video_action := policy_row.unmatched_video_action;
        NEW.intent_unmatched_audio_action := policy_row.unmatched_audio_action;
        NEW.intent_unmatched_subtitle_action := policy_row.unmatched_subtitle_action;
        NEW.intent_unmatched_attachment_action := policy_row.unmatched_attachment_action;
        NEW.intent_unmatched_data_action := policy_row.unmatched_data_action;
        RETURN NEW;
    END IF;

    NEW.intent_unmatched_video_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_video_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'fail')));
    NEW.intent_unmatched_audio_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_audio_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'preserve')));
    NEW.intent_unmatched_subtitle_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_subtitle_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'preserve')));
    NEW.intent_unmatched_attachment_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_attachment_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'preserve')));
    NEW.intent_unmatched_data_action := lower(COALESCE(NULLIF(btrim(NEW.intent_unmatched_data_action), ''), COALESCE(NULLIF(replace(NEW.intent_unmatched_stream_policy, 'reject', 'fail'), ''), 'remove')));
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_job_validate_path_within_root_v1(path_input text, root_input text, detail_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    normalized_path_value TEXT;
    normalized_root_value TEXT;
BEGIN
    normalized_path_value := media_job_normalized_absolute_path_v1(path_input);
    normalized_root_value := media_job_normalized_absolute_path_v1(root_input);

    IF normalized_path_value IS NULL
       OR normalized_root_value IS NULL
       OR NOT (
           normalized_path_value = normalized_root_value
           OR normalized_root_value = '/'
           OR left(normalized_path_value, length(normalized_root_value) + 1) = normalized_root_value || '/'
       ) THEN
        RAISE EXCEPTION 'media job path outside profile root'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = detail_input;
    END IF;
END;
$$;



CREATE FUNCTION public.media_job_verification_check_append_v1(media_job_public_id_input uuid, claim_generation_input bigint, check_index_input integer, check_kind_input text, check_status_input text, expected_value_input text DEFAULT NULL::text, actual_value_input text DEFAULT NULL::text, details_text_input text DEFAULT NULL::text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    current_job_id BIGINT;
    current_attempt_id BIGINT;
BEGIN
    SELECT media_job_id, media_job_attempt_id
      INTO current_job_id, current_attempt_id
      FROM media_job_current_attempt_v1(media_job_public_id_input, claim_generation_input);
    IF current_attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    INSERT INTO media_job_verification_check (
        media_job_id, media_job_attempt_id, check_index, check_kind, check_status,
        expected_value, actual_value, details_text
    ) VALUES (
        current_job_id, current_attempt_id, check_index_input, btrim(check_kind_input),
        lower(btrim(check_status_input)), NULLIF(btrim(expected_value_input), ''),
        NULLIF(btrim(actual_value_input), ''), NULLIF(btrim(details_text_input), '')
    )
    ON CONFLICT (media_job_attempt_id, check_index)
    DO UPDATE SET check_kind = EXCLUDED.check_kind,
                  check_status = EXCLUDED.check_status,
                  expected_value = EXCLUDED.expected_value,
                  actual_value = EXCLUDED.actual_value,
                  details_text = EXCLUDED.details_text;
END;
$$;



CREATE FUNCTION public.media_job_verification_check_list_v1(media_job_public_id_input uuid) RETURNS TABLE(attempt_number integer, is_current boolean, check_index integer, check_kind text, check_status text, expected_value text, actual_value text, details_text text, created_at timestamp with time zone)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT attempt.attempt_number, evidence.media_job_attempt_id = job.current_attempt_id,
           evidence.check_index, evidence.check_kind, evidence.check_status,
           evidence.expected_value, evidence.actual_value, evidence.details_text,
           evidence.created_at
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_job_verification_check evidence ON evidence.media_job_attempt_id = attempt.media_job_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY attempt.attempt_number DESC, evidence.check_index;
$$;



CREATE FUNCTION public.media_job_violation_append_v1(media_job_public_id_input uuid, claim_generation_input bigint, violation_index_input integer, violation_kind_input text, severity_input text, stream_id_input integer DEFAULT NULL::integer) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    current_job_id BIGINT;
    current_attempt_id BIGINT;
BEGIN
    SELECT media_job_id, media_job_attempt_id
      INTO current_job_id, current_attempt_id
      FROM media_job_current_attempt_v1(media_job_public_id_input, claim_generation_input);
    IF current_attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    INSERT INTO media_job_violation (
        media_job_id, media_job_attempt_id, violation_index, violation_kind, severity, stream_id
    ) VALUES (
        current_job_id, current_attempt_id, violation_index_input,
        btrim(violation_kind_input), lower(btrim(severity_input)), stream_id_input
    )
    ON CONFLICT (media_job_attempt_id, violation_index)
    DO UPDATE SET violation_kind = EXCLUDED.violation_kind,
                  severity = EXCLUDED.severity,
                  stream_id = EXCLUDED.stream_id;
END;
$$;



CREATE FUNCTION public.media_job_violation_list_v1(media_job_public_id_input uuid) RETURNS TABLE(attempt_number integer, is_current boolean, violation_index integer, violation_kind text, severity text, stream_id integer, created_at timestamp with time zone)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT attempt.attempt_number, evidence.media_job_attempt_id = job.current_attempt_id,
           evidence.violation_index, evidence.violation_kind, evidence.severity,
           evidence.stream_id, evidence.created_at
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_job_violation evidence ON evidence.media_job_attempt_id = attempt.media_job_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY attempt.attempt_number DESC, evidence.violation_index;
$$;



CREATE FUNCTION public.media_job_worker_acknowledge_cancel_v1(media_job_public_id_input uuid, claim_generation_input bigint, observed_cancel_generation_input bigint) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    acknowledged_generation BIGINT;
BEGIN
    UPDATE media_job_attempt attempt
       SET status = media_job_status_cancelled_v1(), completed_at = now()
      FROM media_job job
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.media_job_attempt_id = job.current_attempt_id
       AND attempt.claim_generation = claim_generation_input
       AND attempt.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND job.cancel_generation > observed_cancel_generation_input
    RETURNING job.cancel_generation INTO acknowledged_generation;
    IF acknowledged_generation IS NULL THEN
        RAISE EXCEPTION 'stale claim or no pending cancellation'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_cancel_not_requested';
    END IF;
    UPDATE media_job
       SET status = media_job_status_cancelled_v1(),
           cancel_acknowledged_generation = acknowledged_generation,
           completed_at = now(), last_error = NULL
     WHERE media_job_public_id = media_job_public_id_input;
    RETURN acknowledged_generation;
END;
$$;



CREATE FUNCTION public.media_job_worker_claim_next_v2() RETURNS TABLE(media_job_public_id uuid, media_profile_public_id uuid, source_path text, output_path text, dry_run boolean, source_root text, output_root text, compatibility_target_key text, policy_key text, target_video_codec text, target_audio_codec text, target_audio_channels integer, target_audio_channel_layout text, target_subtitle_policy text, policy_video_intent text, desired_target_key text, desired_target_version integer, desired_container_format text, unmatched_stream_policy text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean, cancel_generation bigint, attempt_number integer, claim_generation bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id, job.current_attempt_id
          FROM media_job job
          JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
         WHERE job.status = media_job_status_queued_v1()
           AND attempt.status = media_job_status_queued_v1()
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE OF job, attempt SKIP LOCKED
         LIMIT 1
    ),
    updated_attempt AS (
        UPDATE media_job_attempt attempt
           SET status = media_job_status_running_v1(),
               claimed_at = now(),
               heartbeat_at = now(),
               cancel_generation_at_claim = job.cancel_generation
          FROM claimed
          JOIN media_job job ON job.media_job_id = claimed.media_job_id
         WHERE attempt.media_job_attempt_id = claimed.current_attempt_id
        RETURNING attempt.*
    ),
    updated_job AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = now(),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM updated_attempt attempt
         WHERE job.media_job_id = attempt.media_job_id
        RETURNING job.*
    )
    SELECT job.media_job_public_id, profile.media_profile_public_id,
           job.source_path, job.output_path, job.dry_run,
           job.intent_source_root, job.intent_output_root,
           job.intent_compatibility_target_key, job.intent_policy_key,
           job.intent_target_video_codec, job.intent_target_audio_codec,
           job.intent_target_audio_channels, job.intent_target_audio_channel_layout,
           job.intent_target_subtitle_policy, job.intent_policy_video_intent,
           job.intent_desired_target_key, job.intent_desired_target_version,
           job.intent_desired_container_format, job.intent_unmatched_stream_policy,
           job.intent_verification_strictness,
           job.intent_verification_duration_tolerance_millis,
           job.intent_verification_mux_validation,
           job.intent_verification_decode_all_streams,
           job.intent_verification_keyframe_seek,
           job.intent_verification_playback_probe,
           job.cancel_generation,
           attempt.attempt_number,
           attempt.claim_generation
      FROM updated_job job
      JOIN updated_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_profile profile ON profile.media_profile_id = job.media_profile_id;
END;
$$;



CREATE FUNCTION public.media_job_worker_claim_next_v3() RETURNS TABLE(media_job_public_id uuid, media_profile_public_id uuid, source_path text, output_path text, dry_run boolean, source_root text, output_root text, compatibility_target_key text, policy_key text, target_video_codec text, target_audio_codec text, target_audio_channels integer, target_audio_channel_layout text, target_subtitle_policy text, policy_video_intent text, desired_target_key text, desired_target_version integer, desired_container_format text, unmatched_stream_policy text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean, discovery_source_size_bytes bigint, discovery_source_modified_ns bigint, discovery_source_sha256 text, cancel_generation bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id FROM media_job job
         WHERE job.status = media_job_status_queued_v1()
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE SKIP LOCKED LIMIT 1
    ), updated AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = COALESCE(job.started_at, now()), heartbeat_at = now(),
               completed_at = NULL, last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM claimed WHERE job.media_job_id = claimed.media_job_id
        RETURNING job.*
    )
    SELECT updated.media_job_public_id, profile.media_profile_public_id,
           updated.source_path, updated.output_path, updated.dry_run,
           updated.intent_source_root, updated.intent_output_root,
           updated.intent_compatibility_target_key, updated.intent_policy_key,
           updated.intent_target_video_codec, updated.intent_target_audio_codec,
           updated.intent_target_audio_channels, updated.intent_target_audio_channel_layout,
           updated.intent_target_subtitle_policy, updated.intent_policy_video_intent,
           updated.intent_desired_target_key, updated.intent_desired_target_version,
           updated.intent_desired_container_format, updated.intent_unmatched_stream_policy,
           updated.intent_verification_strictness,
           updated.intent_verification_duration_tolerance_millis,
           updated.intent_verification_mux_validation,
           updated.intent_verification_decode_all_streams,
           updated.intent_verification_keyframe_seek,
           updated.intent_verification_playback_probe,
           updated.discovery_source_size_bytes, updated.discovery_source_modified_ns,
           updated.discovery_source_sha256, updated.cancel_generation
      FROM updated
      JOIN media_profile profile ON profile.media_profile_id = updated.media_profile_id;
END;
$$;



CREATE FUNCTION public.media_job_worker_claim_next_v4() RETURNS TABLE(media_job_public_id uuid, media_profile_public_id uuid, source_path text, output_path text, dry_run boolean, source_root text, output_root text, compatibility_target_key text, policy_key text, target_video_codec text, target_audio_codec text, target_audio_channels integer, target_audio_channel_layout text, target_subtitle_policy text, policy_video_intent text, desired_target_key text, desired_target_version integer, desired_container_format text, desired_container_metadata_policy text, desired_container_chapter_policy text, unmatched_stream_policy text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean, cancel_generation bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id
          FROM media_job job
         WHERE job.status = media_job_status_queued_v1()
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ),
    updated AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = COALESCE(job.started_at, now()),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM claimed
         WHERE job.media_job_id = claimed.media_job_id
        RETURNING job.*
    )
    SELECT updated.media_job_public_id, profile.media_profile_public_id,
           updated.source_path, updated.output_path, updated.dry_run,
           updated.intent_source_root, updated.intent_output_root,
           updated.intent_compatibility_target_key, updated.intent_policy_key,
           updated.intent_target_video_codec, updated.intent_target_audio_codec,
           updated.intent_target_audio_channels, updated.intent_target_audio_channel_layout,
           updated.intent_target_subtitle_policy, updated.intent_policy_video_intent,
           updated.intent_desired_target_key, updated.intent_desired_target_version,
           updated.intent_desired_container_format,
           updated.intent_desired_container_metadata_policy,
           updated.intent_desired_container_chapter_policy,
           updated.intent_unmatched_stream_policy,
           updated.intent_verification_strictness,
           updated.intent_verification_duration_tolerance_millis,
           updated.intent_verification_mux_validation,
           updated.intent_verification_decode_all_streams,
           updated.intent_verification_keyframe_seek,
           updated.intent_verification_playback_probe,
           updated.cancel_generation
      FROM updated
      JOIN media_profile profile ON profile.media_profile_id = updated.media_profile_id;
END;
$$;



CREATE FUNCTION public.media_job_worker_claim_next_v5() RETURNS TABLE(media_job_public_id uuid, media_profile_public_id uuid, source_path text, output_path text, dry_run boolean, source_root text, output_root text, compatibility_target_key text, policy_key text, target_video_codec text, target_audio_codec text, target_audio_channels integer, target_audio_channel_layout text, target_subtitle_policy text, policy_video_intent text, desired_target_key text, desired_target_version integer, desired_container_format text, desired_container_metadata_policy text, desired_container_chapter_policy text, desired_container_attachment_policy text, unmatched_stream_policy text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean, cancel_generation bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id
          FROM media_job job
         WHERE job.status = media_job_status_queued_v1()
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ),
    updated AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = COALESCE(job.started_at, now()),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM claimed
         WHERE job.media_job_id = claimed.media_job_id
        RETURNING job.*
    )
    SELECT updated.media_job_public_id, profile.media_profile_public_id,
           updated.source_path, updated.output_path, updated.dry_run,
           updated.intent_source_root, updated.intent_output_root,
           updated.intent_compatibility_target_key, updated.intent_policy_key,
           updated.intent_target_video_codec, updated.intent_target_audio_codec,
           updated.intent_target_audio_channels, updated.intent_target_audio_channel_layout,
           updated.intent_target_subtitle_policy, updated.intent_policy_video_intent,
           updated.intent_desired_target_key, updated.intent_desired_target_version,
           updated.intent_desired_container_format,
           updated.intent_desired_container_metadata_policy,
           updated.intent_desired_container_chapter_policy,
           updated.intent_desired_container_attachment_policy,
           updated.intent_unmatched_stream_policy,
           updated.intent_verification_strictness,
           updated.intent_verification_duration_tolerance_millis,
           updated.intent_verification_mux_validation,
           updated.intent_verification_decode_all_streams,
           updated.intent_verification_keyframe_seek,
           updated.intent_verification_playback_probe,
           updated.cancel_generation
      FROM updated
      JOIN media_profile profile ON profile.media_profile_id = updated.media_profile_id;
END;
$$;



CREATE FUNCTION public.media_job_worker_claim_next_v6() RETURNS TABLE(media_job_public_id uuid, media_profile_public_id uuid, source_path text, output_path text, dry_run boolean, source_root text, output_root text, compatibility_target_key text, policy_key text, target_video_codec text, target_audio_codec text, target_audio_channels integer, target_audio_channel_layout text, target_subtitle_policy text, policy_video_intent text, desired_target_key text, desired_target_version integer, desired_container_format text, desired_container_metadata_policy text, desired_container_chapter_policy text, desired_container_attachment_policy text, unmatched_stream_policy text, unmatched_video_action text, unmatched_audio_action text, unmatched_subtitle_action text, unmatched_attachment_action text, unmatched_data_action text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean, cancel_generation bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id
          FROM media_job job
         WHERE job.status = media_job_status_queued_v1()
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ),
    updated AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = COALESCE(job.started_at, now()),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM claimed
         WHERE job.media_job_id = claimed.media_job_id
        RETURNING job.*
    )
    SELECT updated.media_job_public_id, profile.media_profile_public_id,
           updated.source_path, updated.output_path, updated.dry_run,
           updated.intent_source_root, updated.intent_output_root,
           updated.intent_compatibility_target_key, updated.intent_policy_key,
           updated.intent_target_video_codec, updated.intent_target_audio_codec,
           updated.intent_target_audio_channels, updated.intent_target_audio_channel_layout,
           updated.intent_target_subtitle_policy, updated.intent_policy_video_intent,
           updated.intent_desired_target_key, updated.intent_desired_target_version,
           updated.intent_desired_container_format,
           updated.intent_desired_container_metadata_policy,
           updated.intent_desired_container_chapter_policy,
           updated.intent_desired_container_attachment_policy,
           updated.intent_unmatched_stream_policy,
           updated.intent_unmatched_video_action,
           updated.intent_unmatched_audio_action,
           updated.intent_unmatched_subtitle_action,
           updated.intent_unmatched_attachment_action,
           updated.intent_unmatched_data_action,
           updated.intent_verification_strictness,
           updated.intent_verification_duration_tolerance_millis,
           updated.intent_verification_mux_validation,
           updated.intent_verification_decode_all_streams,
           updated.intent_verification_keyframe_seek,
           updated.intent_verification_playback_probe,
           updated.cancel_generation
      FROM updated
      JOIN media_profile profile ON profile.media_profile_id = updated.media_profile_id;
END;
$$;



CREATE FUNCTION public.media_job_worker_claim_next_v7() RETURNS TABLE(media_job_public_id uuid, media_profile_public_id uuid, source_path text, output_path text, dry_run boolean, source_root text, output_root text, source_identity text, source_size_bytes bigint, source_modified_ns bigint, source_changed_ns bigint, source_sha256 text, compatibility_target_key text, compatibility_target_version integer, policy_key text, target_video_codec text, target_audio_codec text, target_audio_channels integer, target_audio_channel_layout text, target_subtitle_policy text, policy_video_intent text, desired_target_key text, desired_target_version integer, desired_container_format text, desired_container_metadata_policy text, desired_container_chapter_policy text, desired_container_attachment_policy text, unmatched_stream_policy text, unmatched_video_action text, unmatched_audio_action text, unmatched_subtitle_action text, unmatched_attachment_action text, unmatched_data_action text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean, cancel_generation bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id
          FROM media_job job
         WHERE job.status = media_job_status_queued_v1()
           AND job.intent_source_identity IS NOT NULL
           AND job.intent_source_size_bytes IS NOT NULL
           AND job.intent_source_modified_ns IS NOT NULL
           AND job.intent_source_changed_ns IS NOT NULL
           AND job.intent_source_sha256 IS NOT NULL
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE SKIP LOCKED
         LIMIT 1
    ),
    updated AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = COALESCE(job.started_at, now()),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM claimed
         WHERE job.media_job_id = claimed.media_job_id
        RETURNING job.*
    )
    SELECT updated.media_job_public_id, profile.media_profile_public_id,
           updated.source_path, updated.output_path, updated.dry_run,
           updated.intent_source_root, updated.intent_output_root,
           updated.intent_source_identity, updated.intent_source_size_bytes,
           updated.intent_source_modified_ns, updated.intent_source_changed_ns,
           updated.intent_source_sha256,
           updated.intent_compatibility_target_key,
           updated.intent_compatibility_target_version,
           updated.intent_policy_key,
           updated.intent_target_video_codec, updated.intent_target_audio_codec,
           updated.intent_target_audio_channels, updated.intent_target_audio_channel_layout,
           updated.intent_target_subtitle_policy, updated.intent_policy_video_intent,
           updated.intent_desired_target_key, updated.intent_desired_target_version,
           updated.intent_desired_container_format,
           updated.intent_desired_container_metadata_policy,
           updated.intent_desired_container_chapter_policy,
           updated.intent_desired_container_attachment_policy,
           updated.intent_unmatched_stream_policy,
           updated.intent_unmatched_video_action,
           updated.intent_unmatched_audio_action,
           updated.intent_unmatched_subtitle_action,
           updated.intent_unmatched_attachment_action,
           updated.intent_unmatched_data_action,
           updated.intent_verification_strictness,
           updated.intent_verification_duration_tolerance_millis,
           updated.intent_verification_mux_validation,
           updated.intent_verification_decode_all_streams,
           updated.intent_verification_keyframe_seek,
           updated.intent_verification_playback_probe,
           updated.cancel_generation
      FROM updated
      JOIN media_profile profile ON profile.media_profile_id = updated.media_profile_id;
END;
$$;



CREATE FUNCTION public.media_job_worker_claim_next_v8() RETURNS TABLE(media_job_public_id uuid, media_profile_public_id uuid, source_path text, output_path text, dry_run boolean, source_root text, output_root text, source_identity text, source_size_bytes bigint, source_modified_ns bigint, source_changed_ns bigint, source_sha256 text, compatibility_target_key text, compatibility_target_version integer, policy_key text, target_video_codec text, target_audio_codec text, target_audio_channels integer, target_audio_channel_layout text, target_subtitle_policy text, policy_video_intent text, desired_target_key text, desired_target_version integer, desired_container_format text, desired_container_metadata_policy text, desired_container_chapter_policy text, desired_container_attachment_policy text, unmatched_stream_policy text, unmatched_video_action text, unmatched_audio_action text, unmatched_subtitle_action text, unmatched_attachment_action text, unmatched_data_action text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean, attempt_number integer, claim_generation bigint, cancel_generation bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    WITH claimed AS (
        SELECT job.media_job_id, job.current_attempt_id
          FROM media_job job
          JOIN media_job_attempt attempt
            ON attempt.media_job_attempt_id = job.current_attempt_id
           AND attempt.media_job_id = job.media_job_id
         WHERE job.status = media_job_status_queued_v1()
           AND attempt.status = media_job_status_queued_v1()
           AND job.intent_source_identity IS NOT NULL
           AND job.intent_source_size_bytes IS NOT NULL
           AND job.intent_source_modified_ns IS NOT NULL
           AND job.intent_source_changed_ns IS NOT NULL
           AND job.intent_source_sha256 IS NOT NULL
         ORDER BY job.queued_at, job.media_job_id
         FOR UPDATE OF job, attempt SKIP LOCKED
         LIMIT 1
    ),
    updated_attempt AS (
        UPDATE media_job_attempt attempt
           SET status = media_job_status_running_v1(),
               claimed_at = now(),
               heartbeat_at = now(),
               cancel_generation_at_claim = job.cancel_generation
          FROM claimed
          JOIN media_job job ON job.media_job_id = claimed.media_job_id
         WHERE attempt.media_job_attempt_id = claimed.current_attempt_id
        RETURNING attempt.*
    ),
    updated_job AS (
        UPDATE media_job job
           SET status = media_job_status_running_v1(),
               started_at = COALESCE(job.started_at, now()),
               heartbeat_at = now(),
               completed_at = NULL,
               last_error = NULL,
               cancel_acknowledged_generation = job.cancel_generation
          FROM updated_attempt attempt
         WHERE job.media_job_id = attempt.media_job_id
        RETURNING job.*
    )
    SELECT job.media_job_public_id, profile.media_profile_public_id,
           job.source_path, job.output_path, job.dry_run,
           job.intent_source_root, job.intent_output_root,
           job.intent_source_identity, job.intent_source_size_bytes,
           job.intent_source_modified_ns, job.intent_source_changed_ns,
           job.intent_source_sha256,
           job.intent_compatibility_target_key,
           job.intent_compatibility_target_version,
           job.intent_policy_key,
           job.intent_target_video_codec, job.intent_target_audio_codec,
           job.intent_target_audio_channels, job.intent_target_audio_channel_layout,
           job.intent_target_subtitle_policy, job.intent_policy_video_intent,
           job.intent_desired_target_key, job.intent_desired_target_version,
           job.intent_desired_container_format,
           job.intent_desired_container_metadata_policy,
           job.intent_desired_container_chapter_policy,
           job.intent_desired_container_attachment_policy,
           job.intent_unmatched_stream_policy,
           job.intent_unmatched_video_action,
           job.intent_unmatched_audio_action,
           job.intent_unmatched_subtitle_action,
           job.intent_unmatched_attachment_action,
           job.intent_unmatched_data_action,
           job.intent_verification_strictness,
           job.intent_verification_duration_tolerance_millis,
           job.intent_verification_mux_validation,
           job.intent_verification_decode_all_streams,
           job.intent_verification_keyframe_seek,
           job.intent_verification_playback_probe,
           attempt.attempt_number, attempt.claim_generation,
           job.cancel_generation
      FROM updated_job job
      JOIN updated_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_profile profile ON profile.media_profile_id = job.media_profile_id;
END;
$$;



CREATE FUNCTION public.media_job_worker_commit_replacement_terminal_v1(media_job_public_id_input uuid) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    job_id BIGINT;
    job_status media_job_status;
BEGIN
    SELECT media_job_id, status
      INTO job_id, job_status
      FROM media_job
     WHERE media_job_public_id = media_job_public_id_input
     FOR UPDATE;

    IF job_id IS NULL THEN
        RAISE EXCEPTION 'media job not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_not_found';
    END IF;

    IF job_status NOT IN (
        media_job_status_running_v1(),
        media_job_status_verifying_v1(),
        media_job_status_completed_v1()
    ) THEN
        RAISE EXCEPTION 'media job replacement completion invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_replacement_terminal_invalid_status';
    END IF;

    INSERT INTO media_job_phase (
        media_job_id, phase_index, phase_name, phase_status, details_text
    ) VALUES (
        job_id, 1, 'execute', media_job_status_completed_v1(), NULL
    )
    ON CONFLICT (media_job_id, phase_index)
    DO UPDATE SET
        phase_name = EXCLUDED.phase_name,
        phase_status = EXCLUDED.phase_status,
        details_text = EXCLUDED.details_text;

    INSERT INTO media_job_phase (
        media_job_id, phase_index, phase_name, phase_status, details_text
    ) VALUES (
        job_id, 2, 'verify_replace', media_job_status_completed_v1(), NULL
    )
    ON CONFLICT (media_job_id, phase_index)
    DO UPDATE SET
        phase_name = EXCLUDED.phase_name,
        phase_status = EXCLUDED.phase_status,
        details_text = EXCLUDED.details_text;

    INSERT INTO media_job_verification_check (
        media_job_id, check_index, check_kind, check_status,
        expected_value, actual_value, details_text
    ) VALUES (
        job_id, 0, 'output_replacement', 'passed',
        'verified_atomic_replace', 'completed', NULL
    )
    ON CONFLICT (media_job_id, check_index)
    DO UPDATE SET
        check_kind = EXCLUDED.check_kind,
        check_status = EXCLUDED.check_status,
        expected_value = EXCLUDED.expected_value,
        actual_value = EXCLUDED.actual_value,
        details_text = EXCLUDED.details_text;

    UPDATE media_job
       SET status = media_job_status_completed_v1(),
           completed_at = COALESCE(completed_at, now()),
           last_error = NULL
     WHERE media_job_id = job_id;

    INSERT INTO media_job_terminal_outbox (media_job_id, event_kind)
    VALUES (job_id, 'completed')
    ON CONFLICT (media_job_id) DO NOTHING;
END;
$$;



CREATE FUNCTION public.media_job_worker_commit_replacement_terminal_v2(media_job_public_id_input uuid, claim_generation_input bigint) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    job_id BIGINT;
    attempt_id BIGINT;
    attempt_status media_job_status;
BEGIN
    SELECT job.media_job_id, attempt.media_job_attempt_id, attempt.status
      INTO job_id, attempt_id, attempt_status
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.claim_generation = claim_generation_input
       AND attempt.status IN (
           media_job_status_running_v1(),
           media_job_status_verifying_v1(),
           media_job_status_completed_v1()
       )
       AND job.status = attempt.status
     FOR UPDATE OF job, attempt;
    IF attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    IF attempt_status = media_job_status_completed_v1() THEN
        IF NOT EXISTS (
            SELECT 1 FROM media_job_terminal_outbox terminal_event
             WHERE terminal_event.media_job_id = job_id
               AND terminal_event.event_kind = 'completed'
        ) THEN
            RAISE EXCEPTION 'completed replacement is missing terminal event'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_terminal_outbox_missing';
        END IF;
        RETURN;
    END IF;

    INSERT INTO media_job_phase (
        media_job_id, media_job_attempt_id, phase_index, phase_name, phase_status, details_text
    ) VALUES
        (job_id, attempt_id, 1, 'execute', media_job_status_completed_v1(), NULL),
        (job_id, attempt_id, 2, 'verify_replace', media_job_status_completed_v1(), NULL)
    ON CONFLICT (media_job_attempt_id, phase_index)
    DO UPDATE SET phase_name = EXCLUDED.phase_name,
                  phase_status = EXCLUDED.phase_status,
                  details_text = EXCLUDED.details_text;

    INSERT INTO media_job_verification_check (
        media_job_id, media_job_attempt_id, check_index, check_kind, check_status,
        expected_value, actual_value, details_text
    ) VALUES (
        job_id, attempt_id, 0, 'output_replacement', 'passed',
        'verified_atomic_replace', 'completed', NULL
    )
    ON CONFLICT (media_job_attempt_id, check_index)
    DO UPDATE SET check_kind = EXCLUDED.check_kind,
                  check_status = EXCLUDED.check_status,
                  expected_value = EXCLUDED.expected_value,
                  actual_value = EXCLUDED.actual_value,
                  details_text = EXCLUDED.details_text;

    UPDATE media_job_attempt
       SET status = media_job_status_completed_v1(),
           completed_at = now(),
           last_error = NULL
     WHERE media_job_attempt_id = attempt_id;
    UPDATE media_job
       SET status = media_job_status_completed_v1(),
           completed_at = COALESCE(completed_at, now()),
           last_error = NULL
     WHERE media_job_id = job_id;
    INSERT INTO media_job_terminal_outbox (media_job_id, event_kind)
    VALUES (job_id, 'completed')
    ON CONFLICT (media_job_id) DO NOTHING;
END;
$$;



CREATE FUNCTION public.media_job_worker_complete_finalized_v1(media_job_public_id_input uuid) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    late_cancel_acknowledged BOOLEAN;
BEGIN
    WITH finalizable_job AS (
        SELECT
            media_job_id,
            cancel_generation > cancel_acknowledged_generation AS late_cancel_pending
        FROM media_job
        WHERE media_job_public_id = media_job_public_id_input
          AND status IN (
              media_job_status_verifying_v1(),
              media_job_status_completed_v1()
          )
        FOR UPDATE
    ),
    completed AS (
        UPDATE media_job job
           SET status = media_job_status_completed_v1(),
               cancel_acknowledged_generation = job.cancel_generation,
               completed_at = COALESCE(job.completed_at, now()),
               last_error = NULL
          FROM finalizable_job
         WHERE job.media_job_id = finalizable_job.media_job_id
        RETURNING finalizable_job.late_cancel_pending
    )
    SELECT completed.late_cancel_pending INTO late_cancel_acknowledged
    FROM completed;

    IF late_cancel_acknowledged IS NULL THEN
        RAISE EXCEPTION 'media job finalized worker completion invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_status_invalid';
    END IF;

    RETURN late_cancel_acknowledged;
END;
$$;



CREATE FUNCTION public.media_job_worker_complete_finalized_v2(media_job_public_id_input uuid, claim_generation_input bigint) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    late_cancel_acknowledged BOOLEAN;
    current_job_id BIGINT;
    current_attempt_id BIGINT;
BEGIN
    WITH finalizable_job AS (
        SELECT job.media_job_id,
               attempt.media_job_attempt_id,
               job.cancel_generation > job.cancel_acknowledged_generation AS late_cancel_pending
          FROM media_job job
          JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
         WHERE job.media_job_public_id = media_job_public_id_input
           AND attempt.claim_generation = claim_generation_input
           AND attempt.status = media_job_status_completed_v1()
           AND job.status = media_job_status_completed_v1()
         FOR UPDATE OF job, attempt
    ),
    completed AS (
        UPDATE media_job job
           SET cancel_acknowledged_generation = job.cancel_generation,
               completed_at = COALESCE(job.completed_at, now()),
               last_error = NULL
          FROM finalizable_job
         WHERE job.media_job_id = finalizable_job.media_job_id
        RETURNING job.media_job_id,
                  finalizable_job.media_job_attempt_id,
                  finalizable_job.late_cancel_pending
    )
    SELECT completed.media_job_id,
           completed.media_job_attempt_id,
           completed.late_cancel_pending
      INTO current_job_id, current_attempt_id, late_cancel_acknowledged
      FROM completed;
    IF late_cancel_acknowledged IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    IF late_cancel_acknowledged THEN
        INSERT INTO media_job_verification_check (
            media_job_id, media_job_attempt_id, check_index, check_kind, check_status,
            expected_value, actual_value, details_text
        ) VALUES (
            current_job_id, current_attempt_id, 25,
            'late_cancel_after_finalized_replace', 'passed',
            'finalized_replacement_remains_completed', 'late_cancel_acknowledged',
            'operator cancellation arrived after finalized replacement boundary'
        )
        ON CONFLICT (media_job_attempt_id, check_index)
        DO UPDATE SET check_kind = EXCLUDED.check_kind,
                      check_status = EXCLUDED.check_status,
                      expected_value = EXCLUDED.expected_value,
                      actual_value = EXCLUDED.actual_value,
                      details_text = EXCLUDED.details_text;
    END IF;
    RETURN late_cancel_acknowledged;
END;
$$;



CREATE FUNCTION public.media_job_worker_complete_recovered_finalized_v1(media_job_public_id_input uuid) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    late_cancel_acknowledged BOOLEAN;
    current_job_id BIGINT;
    current_attempt_id BIGINT;
BEGIN
    WITH finalizable_job AS (
        SELECT job.media_job_id,
               attempt.media_job_attempt_id,
               job.cancel_generation > job.cancel_acknowledged_generation AS late_cancel_pending
          FROM media_job job
          JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
          JOIN media_job_terminal_outbox terminal_event ON terminal_event.media_job_id = job.media_job_id
         WHERE job.media_job_public_id = media_job_public_id_input
           AND attempt.status = media_job_status_completed_v1()
           AND job.status = media_job_status_completed_v1()
           AND terminal_event.event_kind = 'completed'
         FOR UPDATE OF job, attempt
    ),
    completed AS (
        UPDATE media_job job
           SET cancel_acknowledged_generation = job.cancel_generation,
               completed_at = COALESCE(job.completed_at, now()),
               last_error = NULL
          FROM finalizable_job
         WHERE job.media_job_id = finalizable_job.media_job_id
        RETURNING job.media_job_id,
                  finalizable_job.media_job_attempt_id,
                  finalizable_job.late_cancel_pending
    )
    SELECT completed.media_job_id,
           completed.media_job_attempt_id,
           completed.late_cancel_pending
      INTO current_job_id, current_attempt_id, late_cancel_acknowledged
      FROM completed;
    IF late_cancel_acknowledged IS NULL THEN
        RAISE EXCEPTION 'recovered finalized replacement is not durably terminal'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_recovery_terminal_missing';
    END IF;
    IF late_cancel_acknowledged THEN
        INSERT INTO media_job_verification_check (
            media_job_id, media_job_attempt_id, check_index, check_kind, check_status,
            expected_value, actual_value, details_text
        ) VALUES (
            current_job_id, current_attempt_id, 25,
            'late_cancel_after_finalized_replace', 'passed',
            'finalized_replacement_remains_completed', 'late_cancel_acknowledged',
            'operator cancellation arrived after finalized replacement boundary'
        )
        ON CONFLICT (media_job_attempt_id, check_index)
        DO UPDATE SET check_kind = EXCLUDED.check_kind,
                      check_status = EXCLUDED.check_status,
                      expected_value = EXCLUDED.expected_value,
                      actual_value = EXCLUDED.actual_value,
                      details_text = EXCLUDED.details_text;
    END IF;
    RETURN late_cancel_acknowledged;
END;
$$;



CREATE FUNCTION public.media_job_worker_complete_v1(media_job_public_id_input uuid, claim_generation_input bigint, observed_cancel_generation_input bigint) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    cancelled BOOLEAN;
    terminal_status media_job_status;
BEGIN
    SELECT job.cancel_generation > observed_cancel_generation_input,
           CASE WHEN job.cancel_generation > observed_cancel_generation_input
               THEN media_job_status_cancelled_v1() ELSE media_job_status_completed_v1() END
      INTO cancelled, terminal_status
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.claim_generation = claim_generation_input
       AND attempt.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND job.status = attempt.status
     FOR UPDATE OF job, attempt;
    IF cancelled IS NULL THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    UPDATE media_job_attempt
       SET status = terminal_status, completed_at = now(), last_error = NULL
     WHERE claim_generation = claim_generation_input;
    UPDATE media_job
       SET status = terminal_status,
           cancel_acknowledged_generation = CASE WHEN cancelled THEN cancel_generation ELSE cancel_acknowledged_generation END,
           completed_at = now(), last_error = NULL
     WHERE media_job_public_id = media_job_public_id_input;
    RETURN cancelled;
END;
$$;



CREATE FUNCTION public.media_job_worker_fail_recovered_replacement_v1(media_job_public_id_input uuid, last_error_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    attempt_id BIGINT;
    job_id BIGINT;
BEGIN
    SELECT job.media_job_id, attempt.media_job_attempt_id
      INTO job_id, attempt_id
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND job.status = attempt.status
     FOR UPDATE OF job, attempt;
    IF attempt_id IS NULL THEN
        RAISE EXCEPTION 'recovered replacement job is not active'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_recovery_active_claim_missing';
    END IF;

    UPDATE media_job_attempt
       SET status = media_job_status_failed_v1(),
           completed_at = now(),
           last_error = NULLIF(btrim(last_error_input), '')
     WHERE media_job_attempt_id = attempt_id;
    UPDATE media_job
       SET status = media_job_status_failed_v1(),
           completed_at = now(),
           last_error = NULLIF(btrim(last_error_input), '')
     WHERE media_job_id = job_id;
END;
$$;



CREATE FUNCTION public.media_job_worker_heartbeat_v1(media_job_public_id_input uuid, claim_generation_input bigint) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    UPDATE media_job_attempt attempt
       SET heartbeat_at = now()
      FROM media_job job
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.media_job_attempt_id = job.current_attempt_id
       AND attempt.claim_generation = claim_generation_input
       AND attempt.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND job.status = attempt.status;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    UPDATE media_job SET heartbeat_at = now()
     WHERE media_job_public_id = media_job_public_id_input;
END;
$$;



CREATE FUNCTION public.media_job_worker_mark_status_v1(media_job_public_id_input uuid, claim_generation_input bigint, status_input public.media_job_status, last_error_input text DEFAULT NULL::text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    current_attempt_id BIGINT;
BEGIN
    SELECT attempt.media_job_attempt_id INTO current_attempt_id
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.claim_generation = claim_generation_input
       AND attempt.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND job.status = attempt.status
       AND status_input IN (
           media_job_status_running_v1(), media_job_status_verifying_v1(),
           media_job_status_completed_v1(), media_job_status_failed_v1()
       )
       AND (status_input = media_job_status_failed_v1()
            OR status_input IN (media_job_status_running_v1(), media_job_status_verifying_v1())
            OR job.cancel_generation = job.cancel_acknowledged_generation)
     FOR UPDATE OF job, attempt;
    IF current_attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker claim or invalid transition'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    UPDATE media_job_attempt
       SET status = status_input,
           heartbeat_at = CASE WHEN status_input IN (media_job_status_running_v1(), media_job_status_verifying_v1()) THEN now() ELSE heartbeat_at END,
           completed_at = CASE WHEN status_input IN (media_job_status_completed_v1(), media_job_status_failed_v1()) THEN now() END,
           last_error = NULLIF(btrim(COALESCE(last_error_input, '')), '')
     WHERE media_job_attempt_id = current_attempt_id;
    UPDATE media_job
       SET status = status_input,
           heartbeat_at = CASE WHEN status_input IN (media_job_status_running_v1(), media_job_status_verifying_v1()) THEN now() ELSE heartbeat_at END,
           completed_at = CASE WHEN status_input IN (media_job_status_completed_v1(), media_job_status_failed_v1()) THEN now() END,
           last_error = NULLIF(btrim(COALESCE(last_error_input, '')), '')
     WHERE media_job_public_id = media_job_public_id_input;
END;
$$;



CREATE FUNCTION public.media_job_worker_poll_control_v1(media_job_public_id_input uuid, claim_generation_input bigint, observed_cancel_generation_input bigint) RETURNS TABLE(cancel_requested boolean, cancel_generation bigint)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    RETURN QUERY
    UPDATE media_job_attempt attempt
       SET heartbeat_at = now()
      FROM media_job job
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.media_job_attempt_id = job.current_attempt_id
       AND attempt.claim_generation = claim_generation_input
       AND attempt.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND job.status = attempt.status
    RETURNING job.cancel_generation > observed_cancel_generation_input, job.cancel_generation;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'stale worker claim'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;
    UPDATE media_job SET heartbeat_at = now()
     WHERE media_job_public_id = media_job_public_id_input;
END;
$$;



CREATE FUNCTION public.media_job_worker_recover_stale_v1(stale_after_seconds_input integer) RETURNS TABLE(media_job_public_id uuid, status public.media_job_status, last_error text)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF stale_after_seconds_input IS NULL OR stale_after_seconds_input < 0 THEN
        RAISE EXCEPTION 'stale worker recovery interval invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_stale_after_invalid';
    END IF;

    RETURN QUERY
    WITH stale AS (
        SELECT job.media_job_id,
               attempt.media_job_attempt_id,
               job.cancel_generation > job.cancel_acknowledged_generation AS cancel_pending
          FROM media_job job
          JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
         WHERE job.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
           AND attempt.status = job.status
           AND COALESCE(attempt.heartbeat_at, attempt.claimed_at, attempt.queued_at)
               <= now() - make_interval(secs => stale_after_seconds_input)
         FOR UPDATE OF job, attempt
    ),
    recovered_attempts AS (
        UPDATE media_job_attempt attempt
           SET status = CASE
                   WHEN stale.cancel_pending THEN media_job_status_cancelled_v1()
                   ELSE media_job_status_failed_v1()
               END,
               completed_at = now(),
               last_error = CASE
                   WHEN stale.cancel_pending THEN NULL
                   ELSE 'media_job_worker_heartbeat_stale'
               END
          FROM stale
         WHERE attempt.media_job_attempt_id = stale.media_job_attempt_id
        RETURNING attempt.media_job_id, stale.cancel_pending
    ),
    recovered_jobs AS (
        UPDATE media_job job
           SET status = CASE
                   WHEN recovered_attempts.cancel_pending THEN media_job_status_cancelled_v1()
                   ELSE media_job_status_failed_v1()
               END,
               cancel_acknowledged_generation = CASE
                   WHEN recovered_attempts.cancel_pending THEN job.cancel_generation
                   ELSE job.cancel_acknowledged_generation
               END,
               completed_at = now(),
               last_error = CASE
                   WHEN recovered_attempts.cancel_pending THEN NULL
                   ELSE 'media_job_worker_heartbeat_stale'
               END
          FROM recovered_attempts
         WHERE job.media_job_id = recovered_attempts.media_job_id
        RETURNING job.media_job_public_id, job.status, job.last_error
    )
    SELECT recovered_jobs.media_job_public_id,
           recovered_jobs.status,
           recovered_jobs.last_error
      FROM recovered_jobs;
END;
$$;



CREATE FUNCTION public.media_job_worker_recover_stale_v1(media_job_public_id_input uuid, claim_generation_input bigint, stale_before_input timestamp with time zone, recovered_at_input timestamp with time zone) RETURNS integer
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    job_id BIGINT;
    stale_attempt_id BIGINT;
    next_attempt_number INT;
    next_attempt_id BIGINT;
BEGIN
    IF stale_before_input IS NULL OR recovered_at_input IS NULL
       OR stale_before_input > recovered_at_input THEN
        RAISE EXCEPTION 'stale recovery timestamps are invalid'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_stale_recovery_time_invalid';
    END IF;

    SELECT job.media_job_id, attempt.media_job_attempt_id, attempt.attempt_number + 1
      INTO job_id, stale_attempt_id, next_attempt_number
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_attempt_id = job.current_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
       AND attempt.claim_generation = claim_generation_input
       AND attempt.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
       AND job.status = attempt.status
       AND attempt.heartbeat_at < stale_before_input
     FOR UPDATE OF job, attempt;
    IF stale_attempt_id IS NULL THEN
        RAISE EXCEPTION 'stale worker recovery rejected'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_job_worker_claim_stale';
    END IF;

    UPDATE media_job_attempt
       SET status = media_job_status_failed_v1(),
           completed_at = recovered_at_input,
           last_error = 'worker heartbeat expired'
     WHERE media_job_attempt_id = stale_attempt_id;
    INSERT INTO media_job_attempt (media_job_id, attempt_number, queued_at)
    VALUES (job_id, next_attempt_number, recovered_at_input)
    RETURNING media_job_attempt_id INTO next_attempt_id;
    UPDATE media_job
       SET current_attempt_id = next_attempt_id,
           status = media_job_status_queued_v1(),
           queued_at = recovered_at_input,
           started_at = NULL,
           heartbeat_at = NULL,
           completed_at = NULL,
           last_error = NULL,
           diagnostics_pruned_at = NULL,
           cancel_acknowledged_generation = cancel_generation
     WHERE media_job_id = job_id;
    RETURN next_attempt_number;
END;
$$;



CREATE FUNCTION public.media_key_valid_v1(value_input text) RETURNS boolean
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    SET search_path TO 'public', 'pg_temp'
    AS $_$
    SELECT value_input IS NOT NULL
       AND value_input = btrim(value_input)
       AND octet_length(value_input) BETWEEN 1 AND 128
       AND char_length(value_input) BETWEEN 1 AND 128
       AND value_input ~ '^[a-z0-9]([a-z0-9_-]*[a-z0-9])?$'
$_$;



CREATE FUNCTION public.media_manual_job_create_v1(actor_public_id_input uuid, media_profile_public_id_input uuid, source_path_input text, output_path_input text, source_size_bytes_input bigint, source_modified_ns_input bigint, source_sha256_input text, dry_run_input boolean) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    profile_row media_profile%ROWTYPE;
    media_job_public_id_out UUID;
BEGIN
    SELECT * INTO profile_row
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_row.media_profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    IF NULLIF(btrim(source_path_input), '') IS NULL
       OR COALESCE(source_size_bytes_input, -1) < 0
       OR COALESCE(source_modified_ns_input, -1) < 0
       OR COALESCE(lower(btrim(source_sha256_input)), '') !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION 'invalid manual job fingerprint'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_manual_job_fingerprint_invalid';
    END IF;

    INSERT INTO media_discovery_source_fingerprint (
        media_profile_id,
        source_path,
        source_size_bytes,
        source_modified_ns,
        source_sha256
    )
    VALUES (
        profile_row.media_profile_id,
        btrim(source_path_input),
        source_size_bytes_input,
        source_modified_ns_input,
        lower(btrim(source_sha256_input))
    )
    ON CONFLICT (media_profile_id, source_path) DO UPDATE
    SET source_size_bytes = EXCLUDED.source_size_bytes,
        source_modified_ns = EXCLUDED.source_modified_ns,
        source_sha256 = EXCLUDED.source_sha256,
        last_seen_at = now();

    media_job_public_id_out := media_job_create_v1(
        actor_public_id_input,
        media_profile_public_id_input,
        source_path_input,
        output_path_input,
        COALESCE(dry_run_input, TRUE)
    );

    UPDATE media_discovery_source_fingerprint
       SET last_media_job_public_id = media_job_public_id_out
     WHERE media_profile_id = profile_row.media_profile_id
       AND source_path = btrim(source_path_input);

    RETURN media_job_public_id_out;
END;
$_$;



CREATE FUNCTION public.media_manual_job_create_v2(actor_public_id_input uuid, media_profile_public_id_input uuid, source_path_input text, output_path_input text, source_identity_input text, source_size_bytes_input bigint, source_modified_ns_input bigint, source_changed_ns_input bigint, source_sha256_input text, dry_run_input boolean) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    profile_row media_profile%ROWTYPE;
    media_job_public_id_out UUID;
BEGIN
    SELECT * INTO profile_row
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL;

    IF profile_row.media_profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    IF NULLIF(btrim(source_path_input), '') IS NULL
       OR COALESCE(lower(btrim(source_identity_input)), '') !~ '^[0-9a-f]{16}:[0-9a-f]{16}$'
       OR COALESCE(source_size_bytes_input, -1) < 0
       OR COALESCE(source_modified_ns_input, -1) < 0
       OR COALESCE(source_changed_ns_input, -1) < 0
       OR COALESCE(lower(btrim(source_sha256_input)), '') !~ '^[0-9a-f]{64}$' THEN
        RAISE EXCEPTION 'invalid manual job fingerprint'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_manual_job_fingerprint_invalid';
    END IF;

    INSERT INTO media_discovery_source_fingerprint (
        media_profile_id,
        source_path,
        source_identity,
        source_size_bytes,
        source_modified_ns,
        source_changed_ns,
        source_sha256
    )
    VALUES (
        profile_row.media_profile_id,
        btrim(source_path_input),
        lower(btrim(source_identity_input)),
        source_size_bytes_input,
        source_modified_ns_input,
        source_changed_ns_input,
        lower(btrim(source_sha256_input))
    )
    ON CONFLICT (media_profile_id, source_path) DO UPDATE
    SET source_identity = EXCLUDED.source_identity,
        source_size_bytes = EXCLUDED.source_size_bytes,
        source_modified_ns = EXCLUDED.source_modified_ns,
        source_changed_ns = EXCLUDED.source_changed_ns,
        source_sha256 = EXCLUDED.source_sha256,
        last_seen_at = now();

    media_job_public_id_out := media_job_create_v1(
        actor_public_id_input,
        media_profile_public_id_input,
        source_path_input,
        output_path_input,
        COALESCE(dry_run_input, TRUE)
    );

    UPDATE media_discovery_source_fingerprint
       SET last_media_job_public_id = media_job_public_id_out
     WHERE media_profile_id = profile_row.media_profile_id
       AND source_path = btrim(source_path_input);

    RETURN media_job_public_id_out;
END;
$_$;



CREATE FUNCTION public.media_policy_anime_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'anime'
$$;



CREATE FUNCTION public.media_policy_archival_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'archival'
$$;



CREATE FUNCTION public.media_policy_behavior_set_v1(actor_public_id_input uuid, policy_key_input text, version_input integer, unmatched_video_action_input text, unmatched_audio_action_input text, unmatched_subtitle_action_input text, unmatched_attachment_action_input text, unmatched_data_action_input text, unsupported_format_action_input text, require_all_targets_input boolean, dry_run_input boolean, replacement_mode_input text, quarantine_enabled_input boolean, preserve_permissions_input boolean, preserve_ownership_input boolean) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
BEGIN
    policy_id := media_policy_profile_id_v1(actor_public_id_input, policy_key_input, version_input);
    INSERT INTO media_policy_unmatched_stream_behavior (
        media_policy_profile_id, video_action, audio_action, subtitle_action,
        attachment_action, data_action
    ) VALUES (
        policy_id, lower(btrim(unmatched_video_action_input)),
        lower(btrim(unmatched_audio_action_input)), lower(btrim(unmatched_subtitle_action_input)),
        lower(btrim(unmatched_attachment_action_input)), lower(btrim(unmatched_data_action_input))
    ) ON CONFLICT (media_policy_profile_id) DO UPDATE SET
        video_action = EXCLUDED.video_action, audio_action = EXCLUDED.audio_action,
        subtitle_action = EXCLUDED.subtitle_action,
        attachment_action = EXCLUDED.attachment_action, data_action = EXCLUDED.data_action;

    INSERT INTO media_policy_compatibility_rule (
        media_policy_profile_id, unsupported_format_action, require_all_targets
    ) VALUES (
        policy_id, lower(btrim(unsupported_format_action_input)),
        COALESCE(require_all_targets_input, TRUE)
    ) ON CONFLICT (media_policy_profile_id) DO UPDATE SET
        unsupported_format_action = EXCLUDED.unsupported_format_action,
        require_all_targets = EXCLUDED.require_all_targets;

    INSERT INTO media_policy_output (
        media_policy_profile_id, dry_run, replacement_mode, quarantine_enabled,
        preserve_permissions, preserve_ownership
    ) VALUES (
        policy_id, COALESCE(dry_run_input, TRUE), lower(btrim(replacement_mode_input)),
        COALESCE(quarantine_enabled_input, TRUE),
        COALESCE(preserve_permissions_input, TRUE), COALESCE(preserve_ownership_input, TRUE)
    ) ON CONFLICT (media_policy_profile_id) DO UPDATE SET
        dry_run = EXCLUDED.dry_run, replacement_mode = EXCLUDED.replacement_mode,
        quarantine_enabled = EXCLUDED.quarantine_enabled,
        preserve_permissions = EXCLUDED.preserve_permissions,
        preserve_ownership = EXCLUDED.preserve_ownership;
END;
$$;



CREATE FUNCTION public.media_policy_compatibility_target_append_v1(actor_public_id_input uuid, policy_key_input text, version_input integer, compatibility_target_key_input text, compatibility_target_version_input integer, sort_order_input integer, enabled_input boolean) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
    target_id BIGINT;
BEGIN
    policy_id := media_policy_profile_id_v1(actor_public_id_input, policy_key_input, version_input);
    SELECT target.media_compatibility_target_id INTO target_id
      FROM media_compatibility_target target
     WHERE lower(target.compatibility_target_key) = lower(btrim(compatibility_target_key_input))
       AND target.version = compatibility_target_version_input
       AND target.enabled;
    IF target_id IS NULL THEN
        RAISE EXCEPTION 'compatibility target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_not_found';
    END IF;
    INSERT INTO media_policy_compatibility_target (
        media_policy_profile_id, media_compatibility_target_id, sort_order, enabled
    ) VALUES (policy_id, target_id, sort_order_input, COALESCE(enabled_input, TRUE));
END;
$$;



CREATE FUNCTION public.media_policy_component_version_guard_v1() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
BEGIN
    IF TG_OP = 'DELETE' THEN
        policy_id := OLD.media_policy_profile_id;
    ELSE
        policy_id := NEW.media_policy_profile_id;
    END IF;
    IF EXISTS (
        SELECT 1
          FROM media_job_configuration_snapshot snapshot
         WHERE snapshot.media_policy_profile_id = policy_id
    ) THEN
        RAISE EXCEPTION 'selected policy versions are immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_version_immutable';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_policy_general_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'general'
$$;



CREATE FUNCTION public.media_policy_maintenance_window_append_v1(actor_public_id_input uuid, policy_key_input text, version_input integer, day_of_week_input smallint, start_time_input time without time zone, end_time_input time without time zone, sort_order_input integer, enabled_input boolean) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
BEGIN
    policy_id := media_policy_profile_id_v1(actor_public_id_input, policy_key_input, version_input);
    INSERT INTO media_policy_maintenance_window (
        media_policy_profile_id, day_of_week, start_time, end_time,
        sort_order, enabled
    ) VALUES (
        policy_id, day_of_week_input, start_time_input, end_time_input,
        sort_order_input, COALESCE(enabled_input, TRUE)
    );
END;
$$;



CREATE FUNCTION public.media_policy_operation_cost_append_v1(actor_public_id_input uuid, policy_key_input text, version_input integer, operation_kind_input text, cost_weight_input integer, sort_order_input integer, enabled_input boolean) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
BEGIN
    policy_id := media_policy_profile_id_v1(actor_public_id_input, policy_key_input, version_input);
    INSERT INTO media_policy_operation_cost (
        media_policy_profile_id, operation_kind, cost_weight, sort_order, enabled
    ) VALUES (
        policy_id, lower(btrim(operation_kind_input)), cost_weight_input,
        sort_order_input, COALESCE(enabled_input, TRUE)
    );
END;
$$;



CREATE FUNCTION public.media_policy_profile_id_v1(actor_public_id_input uuid, policy_key_input text, version_input integer) RETURNS bigint
    LANGUAGE plpgsql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    policy_id_out BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    SELECT policy.media_policy_profile_id INTO policy_id_out
      FROM media_policy_profile policy
     WHERE lower(policy.policy_key) = lower(btrim(policy_key_input))
       AND policy.version = version_input
       AND policy.enabled;
    IF policy_id_out IS NULL THEN
        RAISE EXCEPTION 'policy profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_profile_not_found';
    END IF;
    RETURN policy_id_out;
END;
$$;



CREATE FUNCTION public.media_policy_profile_immutable_v1() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.policy_key IS DISTINCT FROM OLD.policy_key
       OR NEW.version IS DISTINCT FROM OLD.version
       OR NEW.display_name IS DISTINCT FROM OLD.display_name
       OR NEW.video_intent IS DISTINCT FROM OLD.video_intent
       OR NEW.verification_strictness IS DISTINCT FROM OLD.verification_strictness
       OR NEW.verification_duration_tolerance_millis IS DISTINCT FROM OLD.verification_duration_tolerance_millis
       OR NEW.verification_mux_validation IS DISTINCT FROM OLD.verification_mux_validation
       OR NEW.verification_decode_all_streams IS DISTINCT FROM OLD.verification_decode_all_streams
       OR NEW.verification_keyframe_seek IS DISTINCT FROM OLD.verification_keyframe_seek
       OR NEW.verification_playback_probe IS DISTINCT FROM OLD.verification_playback_probe THEN
        RAISE EXCEPTION 'policy versions are immutable'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_version_immutable';
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_policy_profile_list_v1() RETURNS TABLE(policy_key text, version integer, display_name text, video_intent text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT profile.policy_key,
           profile.version,
           profile.display_name,
           profile.video_intent,
           profile.verification_strictness,
           profile.verification_duration_tolerance_millis,
           profile.verification_mux_validation,
           profile.verification_decode_all_streams,
           profile.verification_keyframe_seek,
           profile.verification_playback_probe
      FROM media_policy_profile AS profile
     WHERE profile.enabled
     ORDER BY lower(profile.policy_key), profile.version DESC;
$$;



CREATE FUNCTION public.media_policy_profile_list_v2() RETURNS TABLE(policy_key text, version integer, display_name text, video_intent text, unmatched_video_action text, unmatched_audio_action text, unmatched_subtitle_action text, unmatched_attachment_action text, unmatched_data_action text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT profile.policy_key,
           profile.version,
           profile.display_name,
           profile.video_intent,
           profile.unmatched_video_action,
           profile.unmatched_audio_action,
           profile.unmatched_subtitle_action,
           profile.unmatched_attachment_action,
           profile.unmatched_data_action,
           profile.verification_strictness,
           profile.verification_duration_tolerance_millis,
           profile.verification_mux_validation,
           profile.verification_decode_all_streams,
           profile.verification_keyframe_seek,
           profile.verification_playback_probe
      FROM media_policy_profile AS profile
     WHERE profile.enabled
     ORDER BY lower(profile.policy_key), profile.version DESC;
$$;



CREATE FUNCTION public.media_policy_profile_upsert_v1(actor_public_id_input uuid, policy_key_input text, version_input integer, display_name_input text, video_intent_input text, verification_strictness_input text, verification_duration_tolerance_millis_input bigint, verification_mux_validation_input boolean, verification_decode_all_streams_input boolean, verification_keyframe_seek_input boolean, verification_playback_probe_input boolean) RETURNS TABLE(policy_key text, version integer, display_name text, video_intent text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
#variable_conflict use_column
DECLARE
    actor_id BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    BEGIN
        RETURN QUERY
        INSERT INTO media_policy_profile (
            policy_key, version, display_name, video_intent,
            verification_strictness, verification_duration_tolerance_millis,
            verification_mux_validation, verification_decode_all_streams,
            verification_keyframe_seek, verification_playback_probe, enabled
        ) VALUES (
            btrim(policy_key_input), version_input, btrim(display_name_input),
            lower(btrim(video_intent_input)), lower(btrim(verification_strictness_input)),
            verification_duration_tolerance_millis_input,
            verification_mux_validation_input, verification_decode_all_streams_input,
            verification_keyframe_seek_input, verification_playback_probe_input, TRUE
        )
        RETURNING media_policy_profile.policy_key, media_policy_profile.version,
                  media_policy_profile.display_name, media_policy_profile.video_intent,
                  media_policy_profile.verification_strictness,
                  media_policy_profile.verification_duration_tolerance_millis,
                  media_policy_profile.verification_mux_validation,
                  media_policy_profile.verification_decode_all_streams,
                  media_policy_profile.verification_keyframe_seek,
                  media_policy_profile.verification_playback_probe;
    EXCEPTION WHEN unique_violation THEN
        RAISE EXCEPTION 'policy version already exists'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_version_conflict';
    END;
END;
$$;



CREATE FUNCTION public.media_policy_profile_upsert_v2(actor_public_id_input uuid, policy_key_input text, version_input integer, display_name_input text, video_intent_input text, unmatched_video_action_input text, unmatched_audio_action_input text, unmatched_subtitle_action_input text, unmatched_attachment_action_input text, unmatched_data_action_input text, verification_strictness_input text, verification_duration_tolerance_millis_input bigint, verification_mux_validation_input boolean, verification_decode_all_streams_input boolean, verification_keyframe_seek_input boolean, verification_playback_probe_input boolean) RETURNS TABLE(policy_key text, version integer, display_name text, video_intent text, unmatched_video_action text, unmatched_audio_action text, unmatched_subtitle_action text, unmatched_attachment_action text, unmatched_data_action text, verification_strictness text, verification_duration_tolerance_millis bigint, verification_mux_validation boolean, verification_decode_all_streams boolean, verification_keyframe_seek boolean, verification_playback_probe boolean)
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
#variable_conflict use_column
DECLARE
    actor_id BIGINT;
    version_value INT;
    unmatched_video_action_value TEXT;
    unmatched_audio_action_value TEXT;
    unmatched_subtitle_action_value TEXT;
    unmatched_attachment_action_value TEXT;
    unmatched_data_action_value TEXT;
    legacy_unmatched_policy_value TEXT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    version_value := COALESCE(version_input, 1);
    unmatched_video_action_value := lower(COALESCE(NULLIF(btrim(unmatched_video_action_input), ''), 'fail'));
    unmatched_audio_action_value := lower(COALESCE(NULLIF(btrim(unmatched_audio_action_input), ''), 'preserve'));
    unmatched_subtitle_action_value := lower(COALESCE(NULLIF(btrim(unmatched_subtitle_action_input), ''), 'preserve'));
    unmatched_attachment_action_value := lower(COALESCE(NULLIF(btrim(unmatched_attachment_action_input), ''), 'preserve'));
    unmatched_data_action_value := lower(COALESCE(NULLIF(btrim(unmatched_data_action_input), ''), 'remove'));

    IF unmatched_video_action_value NOT IN ('remove', 'preserve', 'fail')
       OR unmatched_audio_action_value NOT IN ('remove', 'preserve', 'fail')
       OR unmatched_subtitle_action_value NOT IN ('remove', 'preserve', 'fail')
       OR unmatched_attachment_action_value NOT IN ('remove', 'preserve', 'fail')
       OR unmatched_data_action_value NOT IN ('remove', 'preserve', 'fail') THEN
        RAISE EXCEPTION 'invalid media unmatched stream action'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_unmatched_action_invalid';
    END IF;

    IF unmatched_video_action_value = unmatched_audio_action_value
       AND unmatched_video_action_value = unmatched_subtitle_action_value
       AND unmatched_video_action_value = unmatched_attachment_action_value
       AND unmatched_video_action_value = unmatched_data_action_value THEN
        legacy_unmatched_policy_value := CASE WHEN unmatched_video_action_value = 'fail' THEN 'reject' ELSE unmatched_video_action_value END;
    ELSE
        legacy_unmatched_policy_value := 'reject';
    END IF;

    RETURN QUERY
    INSERT INTO media_policy_profile (
        policy_key,
        version,
        display_name,
        video_intent,
        unmatched_stream_policy,
        unmatched_video_action,
        unmatched_audio_action,
        unmatched_subtitle_action,
        unmatched_attachment_action,
        unmatched_data_action,
        verification_strictness,
        verification_duration_tolerance_millis,
        verification_mux_validation,
        verification_decode_all_streams,
        verification_keyframe_seek,
        verification_playback_probe,
        enabled,
        updated_at
    )
    VALUES (
        btrim(policy_key_input),
        version_value,
        btrim(display_name_input),
        lower(btrim(video_intent_input)),
        legacy_unmatched_policy_value,
        unmatched_video_action_value,
        unmatched_audio_action_value,
        unmatched_subtitle_action_value,
        unmatched_attachment_action_value,
        unmatched_data_action_value,
        lower(btrim(verification_strictness_input)),
        verification_duration_tolerance_millis_input,
        verification_mux_validation_input,
        verification_decode_all_streams_input,
        verification_keyframe_seek_input,
        verification_playback_probe_input,
        TRUE,
        now()
    )
    ON CONFLICT (lower(policy_key), version) DO UPDATE SET
        display_name = EXCLUDED.display_name,
        video_intent = EXCLUDED.video_intent,
        unmatched_stream_policy = EXCLUDED.unmatched_stream_policy,
        unmatched_video_action = EXCLUDED.unmatched_video_action,
        unmatched_audio_action = EXCLUDED.unmatched_audio_action,
        unmatched_subtitle_action = EXCLUDED.unmatched_subtitle_action,
        unmatched_attachment_action = EXCLUDED.unmatched_attachment_action,
        unmatched_data_action = EXCLUDED.unmatched_data_action,
        verification_strictness = EXCLUDED.verification_strictness,
        verification_duration_tolerance_millis = EXCLUDED.verification_duration_tolerance_millis,
        verification_mux_validation = EXCLUDED.verification_mux_validation,
        verification_decode_all_streams = EXCLUDED.verification_decode_all_streams,
        verification_keyframe_seek = EXCLUDED.verification_keyframe_seek,
        verification_playback_probe = EXCLUDED.verification_playback_probe,
        enabled = TRUE,
        updated_at = now()
    RETURNING
        media_policy_profile.policy_key,
        media_policy_profile.version,
        media_policy_profile.display_name,
        media_policy_profile.video_intent,
        media_policy_profile.unmatched_video_action,
        media_policy_profile.unmatched_audio_action,
        media_policy_profile.unmatched_subtitle_action,
        media_policy_profile.unmatched_attachment_action,
        media_policy_profile.unmatched_data_action,
        media_policy_profile.verification_strictness,
        media_policy_profile.verification_duration_tolerance_millis,
        media_policy_profile.verification_mux_validation,
        media_policy_profile.verification_decode_all_streams,
        media_policy_profile.verification_keyframe_seek,
        media_policy_profile.verification_playback_probe;
END;
$$;



CREATE FUNCTION public.media_policy_retention_rule_append_v1(actor_public_id_input uuid, policy_key_input text, version_input integer, stream_kind_input text, semantic_role_input text, language_code_input text, codec_or_format_input text, action_input text, placement_input text, sort_order_input integer, enabled_input boolean) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
    rule_id_out BIGINT;
BEGIN
    policy_id := media_policy_profile_id_v1(actor_public_id_input, policy_key_input, version_input);
    INSERT INTO media_policy_retention_rule (
        media_policy_profile_id, stream_kind, semantic_role, language_code,
        codec_or_format, action, placement, sort_order, enabled
    ) VALUES (
        policy_id, lower(btrim(stream_kind_input)), NULLIF(btrim(semantic_role_input), ''),
        NULLIF(lower(btrim(language_code_input)), ''), NULLIF(lower(btrim(codec_or_format_input)), ''),
        lower(btrim(action_input)), NULLIF(lower(btrim(placement_input)), ''),
        sort_order_input, COALESCE(enabled_input, TRUE)
    ) RETURNING media_policy_retention_rule_id INTO rule_id_out;
    RETURN rule_id_out;
END;
$$;



CREATE FUNCTION public.media_policy_runtime_limit_set_v1(actor_public_id_input uuid, policy_key_input text, version_input integer, max_concurrency_input integer, max_retries_input integer, max_runtime_seconds_input integer, max_io_megabytes_per_second_input integer, min_free_space_bytes_input bigint, pause_on_battery_input boolean) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
BEGIN
    policy_id := media_policy_profile_id_v1(actor_public_id_input, policy_key_input, version_input);
    INSERT INTO media_policy_runtime_limit (
        media_policy_profile_id, max_concurrency, max_retries, max_runtime_seconds,
        max_io_megabytes_per_second, min_free_space_bytes, pause_on_battery
    ) VALUES (
        policy_id, max_concurrency_input, max_retries_input, max_runtime_seconds_input,
        max_io_megabytes_per_second_input, min_free_space_bytes_input,
        COALESCE(pause_on_battery_input, TRUE)
    )
    ON CONFLICT (media_policy_profile_id) DO UPDATE SET
        max_concurrency = EXCLUDED.max_concurrency,
        max_retries = EXCLUDED.max_retries,
        max_runtime_seconds = EXCLUDED.max_runtime_seconds,
        max_io_megabytes_per_second = EXCLUDED.max_io_megabytes_per_second,
        min_free_space_bytes = EXCLUDED.min_free_space_bytes,
        pause_on_battery = EXCLUDED.pause_on_battery;
END;
$$;



CREATE FUNCTION public.media_policy_runtime_limit_set_v2(actor_public_id_input uuid, policy_key_input text, version_input integer, max_concurrency_input integer, max_retries_input integer, max_runtime_seconds_input integer, max_io_megabytes_per_second_input integer, min_free_space_bytes_input bigint, pause_on_battery_input boolean, minimum_battery_percent_input integer, thermal_pressure_limit_input text, pause_when_thermal_exceeded_input boolean) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
BEGIN
    policy_id := media_policy_profile_id_v1(actor_public_id_input, policy_key_input, version_input);
    INSERT INTO media_policy_runtime_limit (
        media_policy_profile_id, max_concurrency, max_retries, max_runtime_seconds,
        max_io_megabytes_per_second, min_free_space_bytes, pause_on_battery,
        minimum_battery_percent, thermal_pressure_limit,
        pause_when_thermal_exceeded
    ) VALUES (
        policy_id, max_concurrency_input, max_retries_input, max_runtime_seconds_input,
        max_io_megabytes_per_second_input, min_free_space_bytes_input,
        COALESCE(pause_on_battery_input, TRUE), minimum_battery_percent_input,
        lower(btrim(thermal_pressure_limit_input)),
        COALESCE(pause_when_thermal_exceeded_input, TRUE)
    ) ON CONFLICT (media_policy_profile_id) DO UPDATE SET
        max_concurrency = EXCLUDED.max_concurrency,
        max_retries = EXCLUDED.max_retries,
        max_runtime_seconds = EXCLUDED.max_runtime_seconds,
        max_io_megabytes_per_second = EXCLUDED.max_io_megabytes_per_second,
        min_free_space_bytes = EXCLUDED.min_free_space_bytes,
        pause_on_battery = EXCLUDED.pause_on_battery,
        minimum_battery_percent = EXCLUDED.minimum_battery_percent,
        thermal_pressure_limit = EXCLUDED.thermal_pressure_limit,
        pause_when_thermal_exceeded = EXCLUDED.pause_when_thermal_exceeded;
END;
$$;



CREATE FUNCTION public.media_policy_safe_dry_run_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'safe_dry_run'
$$;



CREATE FUNCTION public.media_policy_seed_bounded_defaults_trigger_v1() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM media_policy_seed_bounded_defaults_v1(NEW.media_policy_profile_id);
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_policy_seed_bounded_defaults_v1(policy_id_input bigint) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    INSERT INTO media_policy_unmatched_stream_behavior (
        media_policy_profile_id, video_action, audio_action, subtitle_action,
        attachment_action, data_action
    ) VALUES (policy_id_input, 'fail', 'fail', 'fail', 'fail', 'fail')
    ON CONFLICT (media_policy_profile_id) DO NOTHING;

    INSERT INTO media_policy_compatibility_rule (
        media_policy_profile_id, unsupported_format_action, require_all_targets
    ) VALUES (policy_id_input, 'fail', TRUE)
    ON CONFLICT (media_policy_profile_id) DO NOTHING;

    INSERT INTO media_policy_runtime_limit (
        media_policy_profile_id, max_concurrency, max_retries, max_runtime_seconds,
        max_io_megabytes_per_second, min_free_space_bytes, pause_on_battery,
        minimum_battery_percent, thermal_pressure_limit,
        pause_when_thermal_exceeded
    ) VALUES (
        policy_id_input, 1, 0, 21600, 1024, 10737418240, TRUE,
        20, 'serious', TRUE
    )
    ON CONFLICT (media_policy_profile_id) DO NOTHING;

    INSERT INTO media_policy_output (
        media_policy_profile_id, dry_run, replacement_mode, quarantine_enabled,
        preserve_permissions, preserve_ownership
    ) VALUES (policy_id_input, TRUE, 'disabled', TRUE, TRUE, TRUE)
    ON CONFLICT (media_policy_profile_id) DO NOTHING;

    INSERT INTO media_policy_workspace (
        media_policy_profile_id, retention_hours, diagnostics_enabled,
        stale_cleanup_hours, max_workspace_bytes
    ) VALUES (policy_id_input, 24, TRUE, 48, 107374182400)
    ON CONFLICT (media_policy_profile_id) DO NOTHING;

    INSERT INTO media_policy_backup (
        media_policy_profile_id, enabled, retention_days, min_free_space_bytes
    ) VALUES (policy_id_input, FALSE, NULL, NULL)
    ON CONFLICT (media_policy_profile_id) DO NOTHING;

    INSERT INTO media_policy_verification (
        media_policy_profile_id, strictness, duration_tolerance_millis,
        mux_validation, decode_all_streams, keyframe_seek, playback_probe
    )
    SELECT policy.media_policy_profile_id, policy.verification_strictness,
           policy.verification_duration_tolerance_millis,
           policy.verification_mux_validation,
           policy.verification_decode_all_streams,
           policy.verification_keyframe_seek,
           policy.verification_playback_probe
      FROM media_policy_profile policy
     WHERE policy.media_policy_profile_id = policy_id_input
    ON CONFLICT (media_policy_profile_id) DO NOTHING;
END;
$$;



CREATE FUNCTION public.media_policy_workspace_set_v1(actor_public_id_input uuid, policy_key_input text, version_input integer, retention_hours_input integer, diagnostics_enabled_input boolean, stale_cleanup_hours_input integer, max_workspace_bytes_input bigint, backup_enabled_input boolean, backup_retention_days_input integer, backup_min_free_space_bytes_input bigint) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
BEGIN
    policy_id := media_policy_profile_id_v1(actor_public_id_input, policy_key_input, version_input);
    INSERT INTO media_policy_workspace (
        media_policy_profile_id, retention_hours, diagnostics_enabled,
        stale_cleanup_hours, max_workspace_bytes
    ) VALUES (
        policy_id, retention_hours_input, COALESCE(diagnostics_enabled_input, TRUE),
        stale_cleanup_hours_input, max_workspace_bytes_input
    ) ON CONFLICT (media_policy_profile_id) DO UPDATE SET
        retention_hours = EXCLUDED.retention_hours,
        diagnostics_enabled = EXCLUDED.diagnostics_enabled,
        stale_cleanup_hours = EXCLUDED.stale_cleanup_hours,
        max_workspace_bytes = EXCLUDED.max_workspace_bytes;

    INSERT INTO media_policy_backup (
        media_policy_profile_id, enabled, retention_days, min_free_space_bytes
    ) VALUES (
        policy_id, COALESCE(backup_enabled_input, FALSE),
        backup_retention_days_input, backup_min_free_space_bytes_input
    ) ON CONFLICT (media_policy_profile_id) DO UPDATE SET
        enabled = EXCLUDED.enabled,
        retention_days = EXCLUDED.retention_days,
        min_free_space_bytes = EXCLUDED.min_free_space_bytes;
END;
$$;



CREATE FUNCTION public.media_profile_create_v3(actor_public_id_input uuid, profile_key_input text, source_requested_path_input text, source_canonical_path_input text, source_filesystem_device_input bigint, source_filesystem_inode_input bigint, output_requested_path_input text, output_canonical_path_input text, output_filesystem_device_input bigint, output_filesystem_inode_input bigint, retention_days_input integer, compatibility_target_key_input text, policy_key_input text, watcher_enabled_input boolean, schedule_enabled_input boolean, schedule_interval_minutes_input integer) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    actor_id BIGINT;
    profile_id BIGINT;
    profile_public_id_out UUID;
    source_root_id BIGINT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    PERFORM media_profile_validate_root_identity_v1(
        NULL, source_canonical_path_input, source_filesystem_device_input, source_filesystem_inode_input
    );
    PERFORM media_profile_validate_root_identity_v1(
        NULL, output_canonical_path_input, output_filesystem_device_input, output_filesystem_inode_input
    );
    IF (source_filesystem_device_input, source_filesystem_inode_input)
        = (output_filesystem_device_input, output_filesystem_inode_input)
       OR regexp_replace(btrim(source_canonical_path_input), '/+$', '')
            = regexp_replace(btrim(output_canonical_path_input), '/+$', '')
       OR regexp_replace(btrim(source_canonical_path_input), '/+$', '')
            LIKE regexp_replace(btrim(output_canonical_path_input), '/+$', '') || '/%'
       OR regexp_replace(btrim(output_canonical_path_input), '/+$', '')
            LIKE regexp_replace(btrim(source_canonical_path_input), '/+$', '') || '/%' THEN
        RAISE EXCEPTION 'profile roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_roots_overlap';
    END IF;
    PERFORM media_profile_validate_catalog_refs_v1(
        NULLIF(btrim(compatibility_target_key_input), ''),
        COALESCE(NULLIF(btrim(policy_key_input), ''), media_policy_safe_dry_run_v1())
    );

    BEGIN
        INSERT INTO media_profile (
            profile_key,
            source_root,
            output_root,
            dry_run_only,
            retention_days,
            compatibility_target_key,
            policy_key,
            watcher_enabled,
            schedule_enabled,
            schedule_interval_minutes,
            created_by_user_id
        ) VALUES (
            btrim(profile_key_input),
            regexp_replace(btrim(source_canonical_path_input), '/+$', ''),
            regexp_replace(btrim(output_canonical_path_input), '/+$', ''),
            TRUE,
            COALESCE(retention_days_input, 30),
            NULLIF(btrim(compatibility_target_key_input), ''),
            COALESCE(NULLIF(btrim(policy_key_input), ''), media_policy_safe_dry_run_v1()),
            COALESCE(watcher_enabled_input, FALSE),
            COALESCE(schedule_enabled_input, FALSE),
            schedule_interval_minutes_input,
            actor_id
        ) RETURNING media_profile_id, media_profile_public_id
          INTO profile_id, profile_public_id_out;
    EXCEPTION WHEN unique_violation THEN
        RAISE EXCEPTION 'profile key already exists'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_key_conflict';
    END;

    INSERT INTO media_profile_root (
        media_profile_id, root_kind, requested_path, canonical_path,
        filesystem_device, filesystem_inode, media_type, sort_order, enabled,
        identity_verified_at
    ) VALUES
        (
            profile_id, 'source', btrim(source_requested_path_input),
            regexp_replace(btrim(source_canonical_path_input), '/+$', ''),
            source_filesystem_device_input, source_filesystem_inode_input,
            'mixed', 0, TRUE, now()
        )
    RETURNING media_profile_root_id INTO source_root_id;

    INSERT INTO media_profile_root (
        media_profile_id, root_kind, requested_path, canonical_path,
        filesystem_device, filesystem_inode, media_type, sort_order, enabled,
        identity_verified_at
    ) VALUES
        (
            profile_id, 'output', btrim(output_requested_path_input),
            regexp_replace(btrim(output_canonical_path_input), '/+$', ''),
            output_filesystem_device_input, output_filesystem_inode_input,
            'mixed', 0, TRUE, now()
        );

    IF COALESCE(watcher_enabled_input, FALSE) THEN
        INSERT INTO media_discovery_watcher (
            media_profile_id, media_profile_root_id, debounce_millis,
            sort_order, enabled
        ) VALUES (profile_id, source_root_id, 1000, 0, TRUE);
    END IF;

    IF COALESCE(schedule_enabled_input, FALSE) THEN
        INSERT INTO media_discovery_schedule (
            media_profile_id, media_profile_root_id, interval_value,
            interval_unit, sort_order, enabled, next_run_at
        ) VALUES (
            profile_id, source_root_id, schedule_interval_minutes_input,
            'minutes', 0, TRUE, now()
        );
    END IF;

    RETURN profile_public_id_out;
END;
$_$;



CREATE FUNCTION public.media_profile_desired_target_activation_guard_v1() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.desired_target_profile_id IS NOT NULL
        AND NEW.desired_target_profile_id IS DISTINCT FROM OLD.desired_target_profile_id THEN
        PERFORM media_desired_target_validate_and_activate_v1(
            NEW.desired_target_profile_id
        );
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_profile_desired_target_set_v1(actor_public_id_input uuid, media_profile_public_id_input uuid, desired_target_key_input text, desired_target_version_input integer) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    target_id BIGINT;
    profile_public_id UUID;
    metadata_policy_value TEXT;
    chapter_policy_value TEXT;
BEGIN
    PERFORM media_actor_id_for_public_id_v1(actor_public_id_input);

    IF NULLIF(btrim(desired_target_key_input), '') IS NULL THEN
        UPDATE media_profile
           SET desired_target_profile_id = NULL,
               updated_at = now()
         WHERE media_profile_public_id = media_profile_public_id_input
           AND deleted_at IS NULL
        RETURNING media_profile_public_id INTO profile_public_id;
    ELSE
        SELECT media_desired_target_profile_id
          INTO target_id
          FROM media_desired_target_profile
         WHERE lower(target_key) = lower(btrim(desired_target_key_input))
           AND version = desired_target_version_input
           AND enabled;

        IF target_id IS NULL THEN
            RAISE EXCEPTION 'desired target not found'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_not_found';
        END IF;

        IF NOT EXISTS (
            SELECT 1
              FROM media_desired_target_stream
             WHERE media_desired_target_profile_id = target_id
        ) THEN
            RAISE EXCEPTION 'desired target has no streams'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_streams_required';
        END IF;

        SELECT container.container_metadata_policy,
               container.container_chapter_policy
          INTO metadata_policy_value,
               chapter_policy_value
          FROM media_desired_target_container container
         WHERE container.media_desired_target_profile_id = target_id;

        IF metadata_policy_value = 'replace'
           AND NOT EXISTS (
               SELECT 1
                 FROM media_desired_target_container_metadata metadata
                WHERE metadata.media_desired_target_profile_id = target_id
           ) THEN
            RAISE EXCEPTION 'replace metadata target has no metadata rows'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_required';
        END IF;

        IF metadata_policy_value <> 'replace'
           AND EXISTS (
               SELECT 1
                 FROM media_desired_target_container_metadata metadata
                WHERE metadata.media_desired_target_profile_id = target_id
           ) THEN
            RAISE EXCEPTION 'metadata rows require replace policy'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_metadata_policy_mismatch';
        END IF;

        IF chapter_policy_value = 'replace'
           AND NOT EXISTS (
               SELECT 1
                 FROM media_desired_target_container_chapter chapter
                WHERE chapter.media_desired_target_profile_id = target_id
           ) THEN
            RAISE EXCEPTION 'replace chapter target has no chapter rows'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapters_required';
        END IF;

        IF chapter_policy_value <> 'replace'
           AND EXISTS (
               SELECT 1
                 FROM media_desired_target_container_chapter chapter
                WHERE chapter.media_desired_target_profile_id = target_id
           ) THEN
            RAISE EXCEPTION 'chapter rows require replace policy'
                USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_desired_target_chapter_policy_mismatch';
        END IF;

        UPDATE media_profile
           SET desired_target_profile_id = target_id,
               updated_at = now()
         WHERE media_profile_public_id = media_profile_public_id_input
           AND deleted_at IS NULL
        RETURNING media_profile_public_id INTO profile_public_id;
    END IF;

    IF profile_public_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    RETURN profile_public_id;
END;
$$;



CREATE FUNCTION public.media_profile_file_rule_append_v1(media_profile_public_id_input uuid, rule_kind_input text, matcher_kind_input text, matcher_value_input text, sort_order_input integer, enabled_input boolean) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    profile_id BIGINT;
    rule_id_out BIGINT;
BEGIN
    SELECT media_profile_id INTO profile_id
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL
     FOR UPDATE;
    IF profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    INSERT INTO media_profile_file_rule (
        media_profile_id, rule_kind, matcher_kind, matcher_value, sort_order, enabled
    ) VALUES (
        profile_id,
        lower(btrim(rule_kind_input)),
        lower(btrim(matcher_kind_input)),
        btrim(matcher_value_input),
        sort_order_input,
        COALESCE(enabled_input, TRUE)
    ) RETURNING media_profile_file_rule_id INTO rule_id_out;

    UPDATE media_profile
       SET configuration_version = configuration_version + 1, updated_at = now()
     WHERE media_profile_id = profile_id;
    RETURN rule_id_out;
END;
$$;



CREATE FUNCTION public.media_profile_file_rule_list_v1(media_profile_public_id_input uuid) RETURNS TABLE(rule_kind text, matcher_kind text, matcher_value text, sort_order integer, enabled boolean)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT rule.rule_kind, rule.matcher_kind, rule.matcher_value,
           rule.sort_order, rule.enabled
      FROM media_profile profile
      JOIN media_profile_file_rule rule ON rule.media_profile_id = profile.media_profile_id
     WHERE profile.media_profile_public_id = media_profile_public_id_input
       AND profile.deleted_at IS NULL
     ORDER BY rule.sort_order;
$$;



CREATE FUNCTION public.media_profile_filter_get_v1(media_profile_public_id_input uuid) RETURNS TABLE(min_size_bytes bigint, max_size_bytes bigint, min_duration_millis bigint, max_duration_millis bigint, include_samples boolean, include_trailers boolean, exclude_trash boolean, exclude_quarantine boolean)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT filter.min_size_bytes, filter.max_size_bytes,
           filter.min_duration_millis, filter.max_duration_millis,
           filter.include_samples, filter.include_trailers,
           filter.exclude_trash, filter.exclude_quarantine
      FROM media_profile profile
      JOIN media_profile_filter filter ON filter.media_profile_id = profile.media_profile_id
     WHERE profile.media_profile_public_id = media_profile_public_id_input
       AND profile.deleted_at IS NULL;
$$;



CREATE FUNCTION public.media_profile_filter_set_v1(media_profile_public_id_input uuid, min_size_bytes_input bigint, max_size_bytes_input bigint, min_duration_millis_input bigint, max_duration_millis_input bigint, include_samples_input boolean, include_trailers_input boolean, exclude_trash_input boolean, exclude_quarantine_input boolean) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    profile_id BIGINT;
BEGIN
    SELECT media_profile_id INTO profile_id
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL
     FOR UPDATE;
    IF profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    INSERT INTO media_profile_filter (
        media_profile_id,
        min_size_bytes,
        max_size_bytes,
        min_duration_millis,
        max_duration_millis,
        include_samples,
        include_trailers,
        exclude_trash,
        exclude_quarantine,
        updated_at
    ) VALUES (
        profile_id,
        min_size_bytes_input,
        max_size_bytes_input,
        min_duration_millis_input,
        max_duration_millis_input,
        COALESCE(include_samples_input, FALSE),
        COALESCE(include_trailers_input, FALSE),
        COALESCE(exclude_trash_input, TRUE),
        COALESCE(exclude_quarantine_input, TRUE),
        now()
    )
    ON CONFLICT (media_profile_id) DO UPDATE SET
        min_size_bytes = EXCLUDED.min_size_bytes,
        max_size_bytes = EXCLUDED.max_size_bytes,
        min_duration_millis = EXCLUDED.min_duration_millis,
        max_duration_millis = EXCLUDED.max_duration_millis,
        include_samples = EXCLUDED.include_samples,
        include_trailers = EXCLUDED.include_trailers,
        exclude_trash = EXCLUDED.exclude_trash,
        exclude_quarantine = EXCLUDED.exclude_quarantine,
        updated_at = now();

    UPDATE media_profile
       SET configuration_version = configuration_version + 1, updated_at = now()
     WHERE media_profile_id = profile_id;
END;
$$;



CREATE FUNCTION public.media_profile_get_v1(media_profile_public_id_input uuid) RETURNS TABLE(media_profile_public_id uuid, profile_key text, source_root text, output_root text, dry_run_only boolean, retention_days integer, updated_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT
        mp.media_profile_public_id,
        mp.profile_key,
        mp.source_root,
        mp.output_root,
        mp.dry_run_only,
        mp.retention_days,
        mp.updated_at
    FROM media_profile mp
    WHERE mp.media_profile_public_id = media_profile_public_id_input
      AND mp.deleted_at IS NULL;
$$;



CREATE FUNCTION public.media_profile_get_v2(media_profile_public_id_input uuid) RETURNS TABLE(media_profile_public_id uuid, profile_key text, source_root text, output_root text, dry_run_only boolean, retention_days integer, compatibility_target_key text, policy_key text, watcher_enabled boolean, schedule_enabled boolean, schedule_interval_minutes integer, updated_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT
        mp.media_profile_public_id,
        mp.profile_key,
        mp.source_root,
        mp.output_root,
        mp.dry_run_only,
        mp.retention_days,
        mp.compatibility_target_key,
        mp.policy_key,
        mp.watcher_enabled,
        mp.schedule_enabled,
        mp.schedule_interval_minutes,
        mp.updated_at
    FROM media_profile mp
    WHERE mp.media_profile_public_id = media_profile_public_id_input
      AND mp.deleted_at IS NULL;
$$;



CREATE FUNCTION public.media_profile_get_v3(media_profile_public_id_input uuid) RETURNS TABLE(media_profile_public_id uuid, profile_key text, source_root text, output_root text, dry_run_only boolean, retention_days integer, compatibility_target_key text, policy_key text, watcher_enabled boolean, schedule_enabled boolean, schedule_interval_minutes integer, desired_target_key text, desired_target_version integer, updated_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT profile.media_profile_public_id, profile.profile_key,
           profile.source_root, profile.output_root, profile.dry_run_only,
           profile.retention_days, profile.compatibility_target_key,
           profile.policy_key, profile.watcher_enabled, profile.schedule_enabled,
           profile.schedule_interval_minutes, target.target_key, target.version,
           profile.updated_at
      FROM media_profile profile
      LEFT JOIN media_desired_target_profile target
        ON target.media_desired_target_profile_id = profile.desired_target_profile_id
     WHERE profile.media_profile_public_id = media_profile_public_id_input
       AND profile.deleted_at IS NULL;
$$;



CREATE FUNCTION public.media_profile_import_draft_delete_v1(profile_key_input text) RETURNS boolean
    LANGUAGE sql
    SET search_path TO 'public', 'pg_temp'
    AS $$
    WITH removed AS (
        DELETE FROM media_profile_import_draft
         WHERE lower(profile_key) = lower(btrim(profile_key_input))
        RETURNING 1
    )
    SELECT EXISTS (SELECT 1 FROM removed);
$$;



CREATE FUNCTION public.media_profile_import_draft_list_v1() RETURNS TABLE(media_profile_import_draft_public_id uuid, profile_key text, source_root text, output_root text, source_root_resolved boolean, output_root_resolved boolean, retention_days integer, compatibility_target_key text, desired_target_key text, desired_target_version integer, policy_key text, updated_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT draft.media_profile_import_draft_public_id,
           draft.profile_key,
           draft.source_root,
           draft.output_root,
           draft.source_root_resolved,
           draft.output_root_resolved,
           draft.retention_days,
           draft.compatibility_target_key,
           draft.desired_target_key,
           draft.desired_target_version,
           draft.policy_key,
           draft.updated_at
      FROM media_profile_import_draft draft
     ORDER BY lower(draft.profile_key);
$$;



CREATE FUNCTION public.media_profile_import_draft_upsert_v1(actor_public_id_input uuid, profile_key_input text, source_root_input text, output_root_input text, source_root_resolved_input boolean, output_root_resolved_input boolean, retention_days_input integer, compatibility_target_key_input text, desired_target_key_input text, desired_target_version_input integer, policy_key_input text) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    draft_public_id UUID;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);

    IF COALESCE(source_root_resolved_input, FALSE)
       AND COALESCE(output_root_resolved_input, FALSE) THEN
        RAISE EXCEPTION 'draft has no unresolved path'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_yaml_draft_paths_resolved';
    END IF;

    INSERT INTO media_profile_import_draft (
        profile_key,
        source_root,
        output_root,
        source_root_resolved,
        output_root_resolved,
        retention_days,
        compatibility_target_key,
        desired_target_key,
        desired_target_version,
        policy_key,
        created_by_user_id
    ) VALUES (
        btrim(profile_key_input),
        btrim(source_root_input),
        btrim(output_root_input),
        COALESCE(source_root_resolved_input, FALSE),
        COALESCE(output_root_resolved_input, FALSE),
        retention_days_input,
        NULLIF(btrim(compatibility_target_key_input), ''),
        NULLIF(btrim(desired_target_key_input), ''),
        desired_target_version_input,
        btrim(policy_key_input),
        actor_id
    )
    ON CONFLICT ((lower(profile_key)))
    DO UPDATE SET
        source_root = EXCLUDED.source_root,
        output_root = EXCLUDED.output_root,
        source_root_resolved = EXCLUDED.source_root_resolved,
        output_root_resolved = EXCLUDED.output_root_resolved,
        retention_days = EXCLUDED.retention_days,
        compatibility_target_key = EXCLUDED.compatibility_target_key,
        desired_target_key = EXCLUDED.desired_target_key,
        desired_target_version = EXCLUDED.desired_target_version,
        policy_key = EXCLUDED.policy_key,
        created_by_user_id = EXCLUDED.created_by_user_id,
        updated_at = now()
    RETURNING media_profile_import_draft_public_id INTO draft_public_id;

    RETURN draft_public_id;
END;
$$;



CREATE FUNCTION public.media_profile_list_v1() RETURNS TABLE(media_profile_public_id uuid, profile_key text, source_root text, output_root text, dry_run_only boolean, retention_days integer, updated_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT
        mp.media_profile_public_id,
        mp.profile_key,
        mp.source_root,
        mp.output_root,
        mp.dry_run_only,
        mp.retention_days,
        mp.updated_at
    FROM media_profile mp
    WHERE mp.deleted_at IS NULL
    ORDER BY lower(mp.profile_key) ASC;
$$;



CREATE FUNCTION public.media_profile_list_v2() RETURNS TABLE(media_profile_public_id uuid, profile_key text, source_root text, output_root text, dry_run_only boolean, retention_days integer, compatibility_target_key text, policy_key text, watcher_enabled boolean, schedule_enabled boolean, schedule_interval_minutes integer, updated_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT
        mp.media_profile_public_id,
        mp.profile_key,
        mp.source_root,
        mp.output_root,
        mp.dry_run_only,
        mp.retention_days,
        mp.compatibility_target_key,
        mp.policy_key,
        mp.watcher_enabled,
        mp.schedule_enabled,
        mp.schedule_interval_minutes,
        mp.updated_at
    FROM media_profile mp
    WHERE mp.deleted_at IS NULL
    ORDER BY lower(mp.profile_key) ASC;
$$;



CREATE FUNCTION public.media_profile_list_v3() RETURNS TABLE(media_profile_public_id uuid, profile_key text, source_root text, output_root text, dry_run_only boolean, retention_days integer, compatibility_target_key text, policy_key text, watcher_enabled boolean, schedule_enabled boolean, schedule_interval_minutes integer, desired_target_key text, desired_target_version integer, updated_at timestamp with time zone)
    LANGUAGE sql STABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT profile.media_profile_public_id, profile.profile_key,
           profile.source_root, profile.output_root, profile.dry_run_only,
           profile.retention_days, profile.compatibility_target_key,
           profile.policy_key, profile.watcher_enabled, profile.schedule_enabled,
           profile.schedule_interval_minutes, target.target_key, target.version,
           profile.updated_at
      FROM media_profile profile
      LEFT JOIN media_desired_target_profile target
        ON target.media_desired_target_profile_id = profile.desired_target_profile_id
     WHERE profile.deleted_at IS NULL
     ORDER BY lower(profile.profile_key);
$$;



CREATE FUNCTION public.media_profile_normalized_root_v1(root_input text) RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $_$
    SELECT COALESCE(NULLIF(lower(regexp_replace(btrim(root_input), '/+$', '')), ''), '/')
$_$;



CREATE FUNCTION public.media_profile_root_add_v1(media_profile_public_id_input uuid, root_kind_input text, requested_path_input text, canonical_path_input text, filesystem_device_input bigint, filesystem_inode_input bigint, media_type_input text, sort_order_input integer, enabled_input boolean) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    profile_id BIGINT;
    root_public_id_out UUID;
    canonical_path_value TEXT;
BEGIN
    SELECT media_profile_id INTO profile_id
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL
     FOR UPDATE;

    IF profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    canonical_path_value := regexp_replace(btrim(canonical_path_input), '/+$', '');
    IF canonical_path_value = '' THEN
        canonical_path_value := '/';
    END IF;

    IF COALESCE(enabled_input, FALSE) THEN
        PERFORM media_profile_validate_root_identity_v1(
            NULL,
            canonical_path_value,
            filesystem_device_input,
            filesystem_inode_input
        );
    END IF;

    INSERT INTO media_profile_root (
        media_profile_id,
        root_kind,
        requested_path,
        canonical_path,
        filesystem_device,
        filesystem_inode,
        media_type,
        sort_order,
        enabled,
        identity_verified_at
    )
    VALUES (
        profile_id,
        lower(btrim(root_kind_input)),
        btrim(requested_path_input),
        canonical_path_value,
        filesystem_device_input,
        filesystem_inode_input,
        lower(btrim(media_type_input)),
        sort_order_input,
        COALESCE(enabled_input, FALSE),
        CASE WHEN filesystem_device_input IS NOT NULL AND filesystem_inode_input IS NOT NULL THEN now() END
    )
    RETURNING media_profile_root_public_id INTO root_public_id_out;

    UPDATE media_profile
       SET configuration_version = configuration_version + 1,
           updated_at = now()
     WHERE media_profile_id = profile_id;

    RETURN root_public_id_out;
END;
$_$;



CREATE FUNCTION public.media_profile_root_list_v1(media_profile_public_id_input uuid) RETURNS TABLE(media_profile_root_public_id uuid, root_kind text, requested_path text, canonical_path text, filesystem_device bigint, filesystem_inode bigint, media_type text, sort_order integer, enabled boolean, identity_verified_at timestamp with time zone)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT root.media_profile_root_public_id, root.root_kind,
           root.requested_path, root.canonical_path, root.filesystem_device,
           root.filesystem_inode, root.media_type, root.sort_order,
           root.enabled, root.identity_verified_at
      FROM media_profile profile
      JOIN media_profile_root root ON root.media_profile_id = profile.media_profile_id
     WHERE profile.media_profile_public_id = media_profile_public_id_input
       AND profile.deleted_at IS NULL
     ORDER BY root.root_kind, root.sort_order;
$$;



CREATE FUNCTION public.media_profile_root_revalidate_v1(media_profile_root_public_id_input uuid, canonical_path_input text, filesystem_device_input bigint, filesystem_inode_input bigint) RETURNS timestamp with time zone
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    verified_at_out TIMESTAMPTZ;
BEGIN
    UPDATE media_profile_root root
       SET identity_verified_at = now()
     WHERE root.media_profile_root_public_id = media_profile_root_public_id_input
       AND root.enabled
       AND root.canonical_path = regexp_replace(btrim(canonical_path_input), '/+$', '')
       AND (root.filesystem_device, root.filesystem_inode)
           = (filesystem_device_input, filesystem_inode_input)
    RETURNING root.identity_verified_at INTO verified_at_out;

    IF verified_at_out IS NULL THEN
        RAISE EXCEPTION 'filesystem identity changed'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_root_identity_changed';
    END IF;

    RETURN verified_at_out;
END;
$_$;



CREATE FUNCTION public.media_profile_roots_overlap_v1(left_root_input text, right_root_input text) RETURNS boolean
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT media_profile_normalized_root_v1(left_root_input)
               = media_profile_normalized_root_v1(right_root_input)
        OR media_profile_normalized_root_v1(left_root_input)
               LIKE media_profile_normalized_root_v1(right_root_input) || '/%'
        OR media_profile_normalized_root_v1(right_root_input)
               LIKE media_profile_normalized_root_v1(left_root_input) || '/%'
$$;



CREATE FUNCTION public.media_profile_update_v1(actor_public_id_input uuid, media_profile_public_id_input uuid, source_root_input text, output_root_input text, dry_run_only_input boolean, retention_days_input integer, compatibility_target_key_input text, policy_key_input text, watcher_enabled_input boolean, schedule_enabled_input boolean, schedule_interval_minutes_input integer) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    profile_id BIGINT;
    current_source_root TEXT;
    current_output_root TEXT;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    SELECT media_profile_id, source_root, output_root
      INTO profile_id, current_source_root, current_output_root
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL
     FOR UPDATE;
    IF profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;
    IF (NULLIF(btrim(source_root_input), '') IS NOT NULL
            AND btrim(source_root_input) <> current_source_root)
       OR (NULLIF(btrim(output_root_input), '') IS NOT NULL
            AND btrim(output_root_input) <> current_output_root)
       OR COALESCE(watcher_enabled_input, FALSE)
       OR COALESCE(schedule_enabled_input, FALSE)
       OR schedule_interval_minutes_input IS NOT NULL THEN
        RAISE EXCEPTION 'root and automation changes require verified normalized procedures'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_filesystem_identity_required';
    END IF;
    PERFORM media_profile_validate_catalog_refs_v1(
        CASE WHEN compatibility_target_key_input IS NULL THEN NULL
             ELSE NULLIF(btrim(compatibility_target_key_input), '') END,
        COALESCE(NULLIF(btrim(policy_key_input), ''), media_policy_safe_dry_run_v1())
    );
    UPDATE media_profile
       SET dry_run_only = COALESCE(dry_run_only_input, dry_run_only),
           retention_days = COALESCE(retention_days_input, retention_days),
           compatibility_target_key = CASE
               WHEN compatibility_target_key_input IS NULL THEN compatibility_target_key
               ELSE NULLIF(btrim(compatibility_target_key_input), '')
           END,
           policy_key = COALESCE(NULLIF(btrim(policy_key_input), ''), policy_key),
           configuration_version = configuration_version + 1,
           updated_at = now()
     WHERE media_profile_id = profile_id;
    RETURN media_profile_public_id_input;
END;
$$;



CREATE FUNCTION public.media_profile_update_verified_v1(actor_public_id_input uuid, media_profile_public_id_input uuid, source_requested_path_input text, source_canonical_path_input text, source_filesystem_device_input bigint, source_filesystem_inode_input bigint, output_requested_path_input text, output_canonical_path_input text, output_filesystem_device_input bigint, output_filesystem_inode_input bigint, dry_run_only_input boolean, retention_days_input integer, compatibility_target_key_input text, policy_key_input text, watcher_enabled_input boolean, schedule_enabled_input boolean, schedule_interval_minutes_input integer) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    profile_id BIGINT;
    source_root_id BIGINT;
    output_root_id BIGINT;
    effective_watcher_enabled BOOLEAN;
    effective_schedule_enabled BOOLEAN;
    effective_schedule_interval INTEGER;
    normalized_source_root TEXT;
    normalized_output_root TEXT;
BEGIN
    PERFORM media_actor_id_for_public_id_v1(actor_public_id_input);
    SELECT media_profile_id,
           COALESCE(watcher_enabled_input, watcher_enabled),
           COALESCE(schedule_enabled_input, schedule_enabled),
           COALESCE(schedule_interval_minutes_input, schedule_interval_minutes)
      INTO profile_id, effective_watcher_enabled,
           effective_schedule_enabled, effective_schedule_interval
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL
     FOR UPDATE;
    IF profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;
    IF effective_schedule_enabled AND effective_schedule_interval IS NULL THEN
        RAISE EXCEPTION 'enabled schedule requires an interval'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_schedule_interval_required';
    END IF;

    SELECT media_profile_root_id
      INTO source_root_id
      FROM media_profile_root
     WHERE media_profile_id = profile_id
       AND root_kind = 'source'
       AND sort_order = 0
     FOR UPDATE;
    SELECT media_profile_root_id
      INTO output_root_id
      FROM media_profile_root
     WHERE media_profile_id = profile_id
       AND root_kind = 'output'
       AND sort_order = 0
     FOR UPDATE;
    IF source_root_id IS NULL OR output_root_id IS NULL THEN
        RAISE EXCEPTION 'profile roots are incomplete'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_root_identity_missing';
    END IF;

    normalized_source_root := regexp_replace(btrim(source_canonical_path_input), '/+$', '');
    normalized_output_root := regexp_replace(btrim(output_canonical_path_input), '/+$', '');
    PERFORM media_profile_validate_root_identity_v1(
        source_root_id, normalized_source_root,
        source_filesystem_device_input, source_filesystem_inode_input
    );
    PERFORM media_profile_validate_root_identity_v1(
        output_root_id, normalized_output_root,
        output_filesystem_device_input, output_filesystem_inode_input
    );
    IF (source_filesystem_device_input, source_filesystem_inode_input)
        = (output_filesystem_device_input, output_filesystem_inode_input)
       OR media_profile_roots_overlap_v1(normalized_source_root, normalized_output_root) THEN
        RAISE EXCEPTION 'profile roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_roots_overlap';
    END IF;
    PERFORM media_profile_validate_catalog_refs_v1(
        CASE WHEN compatibility_target_key_input IS NULL THEN NULL
             ELSE NULLIF(btrim(compatibility_target_key_input), '') END,
        COALESCE(NULLIF(btrim(policy_key_input), ''), media_policy_safe_dry_run_v1())
    );

    UPDATE media_profile_root
       SET requested_path = btrim(source_requested_path_input),
           canonical_path = normalized_source_root,
           filesystem_device = source_filesystem_device_input,
           filesystem_inode = source_filesystem_inode_input,
           enabled = TRUE,
           identity_verified_at = now()
     WHERE media_profile_root_id = source_root_id;
    UPDATE media_profile_root
       SET requested_path = btrim(output_requested_path_input),
           canonical_path = normalized_output_root,
           filesystem_device = output_filesystem_device_input,
           filesystem_inode = output_filesystem_inode_input,
           enabled = TRUE,
           identity_verified_at = now()
     WHERE media_profile_root_id = output_root_id;

    UPDATE media_profile
       SET source_root = normalized_source_root,
           output_root = normalized_output_root,
           dry_run_only = COALESCE(dry_run_only_input, dry_run_only),
           retention_days = COALESCE(retention_days_input, retention_days),
           compatibility_target_key = CASE
               WHEN compatibility_target_key_input IS NULL THEN compatibility_target_key
               ELSE NULLIF(btrim(compatibility_target_key_input), '')
           END,
           policy_key = COALESCE(NULLIF(btrim(policy_key_input), ''), policy_key),
           watcher_enabled = effective_watcher_enabled,
           schedule_enabled = effective_schedule_enabled,
           schedule_interval_minutes = effective_schedule_interval,
           configuration_version = configuration_version + 1,
           updated_at = now()
     WHERE media_profile_id = profile_id;

    IF effective_watcher_enabled THEN
        INSERT INTO media_discovery_watcher (
            media_profile_id, media_profile_root_id, debounce_millis, sort_order, enabled
        ) VALUES (profile_id, source_root_id, 1000, 0, TRUE)
        ON CONFLICT (media_profile_root_id) DO UPDATE SET enabled = TRUE;
    ELSE
        UPDATE media_discovery_watcher
           SET enabled = FALSE
         WHERE media_profile_id = profile_id;
    END IF;

    IF effective_schedule_enabled THEN
        INSERT INTO media_discovery_schedule (
            media_profile_id, media_profile_root_id, interval_value,
            interval_unit, sort_order, enabled, next_run_at
        ) VALUES (
            profile_id, source_root_id, effective_schedule_interval,
            'minutes', 0, TRUE, now()
        ) ON CONFLICT (media_profile_root_id) DO UPDATE SET
            interval_value = EXCLUDED.interval_value,
            interval_unit = EXCLUDED.interval_unit,
            enabled = TRUE,
            next_run_at = COALESCE(media_discovery_schedule.next_run_at, now());
    ELSE
        UPDATE media_discovery_schedule
           SET enabled = FALSE,
               next_run_at = NULL
         WHERE media_profile_id = profile_id;
    END IF;
    RETURN media_profile_public_id_input;
END;
$$;



CREATE FUNCTION public.media_profile_upsert_v1(actor_public_id_input uuid, profile_key_input text, source_root_input text, output_root_input text, dry_run_only_input boolean, retention_days_input integer) RETURNS uuid
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT media_profile_upsert_v2(
        actor_public_id_input,
        profile_key_input,
        source_root_input,
        output_root_input,
        TRUE,
        retention_days_input,
        NULL,
        media_policy_safe_dry_run_v1(),
        FALSE,
        FALSE,
        NULL
    )
$$;



CREATE FUNCTION public.media_profile_upsert_v2(actor_public_id_input uuid, profile_key_input text, source_root_input text, output_root_input text, dry_run_only_input boolean, retention_days_input integer, compatibility_target_key_input text, policy_key_input text, watcher_enabled_input boolean, schedule_enabled_input boolean, schedule_interval_minutes_input integer) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    actor_id BIGINT;
    profile_id BIGINT;
    profile_public_id_out UUID;
BEGIN
    actor_id := media_actor_id_for_public_id_v1(actor_public_id_input);
    IF COALESCE(watcher_enabled_input, FALSE) OR COALESCE(schedule_enabled_input, FALSE) THEN
        RAISE EXCEPTION 'verified filesystem identity is required for automation'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_filesystem_identity_required';
    END IF;
    PERFORM media_profile_validate_catalog_refs_v1(
        NULLIF(btrim(compatibility_target_key_input), ''),
        COALESCE(NULLIF(btrim(policy_key_input), ''), media_policy_safe_dry_run_v1())
    );

    BEGIN
        INSERT INTO media_profile (
            profile_key, source_root, output_root, dry_run_only, retention_days,
            compatibility_target_key, policy_key, watcher_enabled, schedule_enabled,
            schedule_interval_minutes, created_by_user_id
        ) VALUES (
            btrim(profile_key_input), btrim(source_root_input), btrim(output_root_input), TRUE,
            COALESCE(retention_days_input, 30), NULLIF(btrim(compatibility_target_key_input), ''),
            COALESCE(NULLIF(btrim(policy_key_input), ''), media_policy_safe_dry_run_v1()),
            FALSE, FALSE, NULL, actor_id
        ) RETURNING media_profile_id, media_profile_public_id
          INTO profile_id, profile_public_id_out;
    EXCEPTION WHEN unique_violation THEN
        RAISE EXCEPTION 'profile key already exists'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_key_conflict';
    END;

    INSERT INTO media_profile_root (
        media_profile_id, root_kind, requested_path, canonical_path,
        media_type, sort_order, enabled
    ) VALUES
        (profile_id, 'source', btrim(source_root_input), btrim(source_root_input), 'mixed', 0, FALSE),
        (profile_id, 'output', btrim(output_root_input), btrim(output_root_input), 'mixed', 0, FALSE);
    RETURN profile_public_id_out;
END;
$$;



CREATE FUNCTION public.media_profile_validate_all_root_overlap_trigger_v1() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    effective_profile_id BIGINT;
BEGIN
    IF NEW.deleted_at IS NULL THEN
        effective_profile_id := NEW.media_profile_id;

        IF TG_OP = 'INSERT' THEN
            SELECT existing.media_profile_id
              INTO effective_profile_id
              FROM media_profile AS existing
             WHERE lower(existing.profile_key) = lower(btrim(NEW.profile_key))
               AND existing.deleted_at IS NULL;

            effective_profile_id := COALESCE(effective_profile_id, NEW.media_profile_id);
        END IF;

        PERFORM media_profile_validate_all_root_overlap_v1(
            effective_profile_id,
            NEW.source_root,
            NEW.output_root
        );
    END IF;
    RETURN NEW;
END;
$$;



CREATE FUNCTION public.media_profile_validate_all_root_overlap_v1(media_profile_id_input bigint, source_root_input text, output_root_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('media_profile_all_root_overlap_v1', 0));

    IF media_profile_roots_overlap_v1(source_root_input, output_root_input) THEN
        RAISE EXCEPTION 'profile roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_roots_overlap';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM media_profile AS existing
         WHERE existing.deleted_at IS NULL
           AND (
               media_profile_id_input IS NULL
               OR existing.media_profile_id <> media_profile_id_input
           )
           AND (
               media_profile_roots_overlap_v1(source_root_input, existing.source_root)
               OR media_profile_roots_overlap_v1(source_root_input, existing.output_root)
               OR media_profile_roots_overlap_v1(output_root_input, existing.source_root)
               OR media_profile_roots_overlap_v1(output_root_input, existing.output_root)
           )
    ) THEN
        RAISE EXCEPTION 'profile roots overlap another active profile'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_discovery_root_overlap';
    END IF;
END;
$$;



CREATE FUNCTION public.media_profile_validate_catalog_refs_v1(compatibility_target_key_input text, policy_key_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    compatibility_target_key_value TEXT;
    policy_key_value TEXT;
BEGIN
    compatibility_target_key_value := NULLIF(btrim(compatibility_target_key_input), '');
    policy_key_value := COALESCE(NULLIF(btrim(policy_key_input), ''), 'safe_dry_run');

    IF compatibility_target_key_value IS NOT NULL
       AND NOT EXISTS (
           SELECT 1
             FROM media_compatibility_target
            WHERE lower(compatibility_target_key) = lower(replace(compatibility_target_key_value, '_', '-'))
              AND enabled
       ) THEN
        RAISE EXCEPTION 'compatibility target not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_compatibility_target_not_found';
    END IF;

    IF NOT EXISTS (
           SELECT 1
             FROM media_policy_profile
            WHERE lower(policy_key) = lower(policy_key_value)
              AND enabled
       ) THEN
        RAISE EXCEPTION 'policy profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_policy_profile_not_found';
    END IF;
END;
$$;



CREATE FUNCTION public.media_profile_validate_discovery_root_overlap_v1(media_profile_id_input bigint, source_root_input text) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    source_root_compare_value TEXT;
BEGIN
    source_root_compare_value := media_profile_normalized_root_v1(source_root_input);

    IF EXISTS (
        SELECT 1
          FROM media_profile AS existing
         WHERE existing.deleted_at IS NULL
           AND (
               media_profile_id_input IS NULL
               OR existing.media_profile_id <> media_profile_id_input
           )
           AND (
               source_root_compare_value = media_profile_normalized_root_v1(existing.source_root)
               OR source_root_compare_value LIKE media_profile_normalized_root_v1(existing.source_root) || '/%'
               OR media_profile_normalized_root_v1(existing.source_root) LIKE source_root_compare_value || '/%'
           )
    ) THEN
        RAISE EXCEPTION 'profile discovery roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_discovery_root_overlap';
    END IF;
END;
$$;



CREATE FUNCTION public.media_profile_validate_root_identity_v1(media_profile_root_id_input bigint, canonical_path_input text, filesystem_device_input bigint, filesystem_inode_input bigint) RETURNS void
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    canonical_path_value TEXT;
BEGIN
    canonical_path_value := regexp_replace(btrim(canonical_path_input), '/+$', '');
    IF canonical_path_value = '' THEN
        canonical_path_value := '/';
    END IF;

    IF canonical_path_value NOT LIKE '/%'
       OR canonical_path_value = '/'
       OR filesystem_device_input IS NULL
       OR filesystem_inode_input IS NULL
       OR filesystem_device_input < 0
       OR filesystem_inode_input < 0 THEN
        RAISE EXCEPTION 'filesystem identity is incomplete'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_root_identity_invalid';
    END IF;

    PERFORM pg_advisory_xact_lock(hashtextextended('media_profile_root_identity_v1', 0));

    IF EXISTS (
        SELECT 1
          FROM media_profile_root root
         WHERE root.enabled
           AND (media_profile_root_id_input IS NULL OR root.media_profile_root_id <> media_profile_root_id_input)
           AND (
               (root.filesystem_device, root.filesystem_inode)
                   = (filesystem_device_input, filesystem_inode_input)
               OR root.canonical_path = canonical_path_value
               OR root.canonical_path LIKE canonical_path_value || '/%'
               OR canonical_path_value LIKE root.canonical_path || '/%'
           )
    ) THEN
        RAISE EXCEPTION 'filesystem roots overlap'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_root_identity_overlap';
    END IF;
END;
$_$;



CREATE FUNCTION public.media_retention_mode_age_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'age'
$$;



CREATE FUNCTION public.media_retention_mode_count_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'count'
$$;



CREATE FUNCTION public.media_retention_policy_default_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'default'
$$;



CREATE FUNCTION public.media_stream_classification_rule_append_v1(actor_public_id_input uuid, policy_key_input text, version_input integer, stream_kind_input text, semantic_role_input text, match_kind_input text, match_pattern_input text, confidence_input smallint, sort_order_input integer, enabled_input boolean) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    policy_id BIGINT;
    rule_id_out BIGINT;
BEGIN
    policy_id := media_policy_profile_id_v1(actor_public_id_input, policy_key_input, version_input);
    INSERT INTO media_stream_classification_rule (
        media_policy_profile_id, stream_kind, semantic_role, match_kind,
        match_pattern, confidence, sort_order, enabled
    ) VALUES (
        policy_id, lower(btrim(stream_kind_input)), btrim(semantic_role_input),
        lower(btrim(match_kind_input)), btrim(match_pattern_input), confidence_input,
        sort_order_input, COALESCE(enabled_input, TRUE)
    ) RETURNING media_stream_classification_rule_id INTO rule_id_out;
    RETURN rule_id_out;
END;
$$;



CREATE FUNCTION public.media_subtitle_discovery_rule_append_v1(media_profile_public_id_input uuid, discovery_pattern_input text, precedence_input integer, enabled_input boolean) RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'public', 'pg_temp'
    AS $$
DECLARE
    profile_id BIGINT;
    rule_id_out BIGINT;
BEGIN
    SELECT media_profile_id INTO profile_id
      FROM media_profile
     WHERE media_profile_public_id = media_profile_public_id_input
       AND deleted_at IS NULL
     FOR UPDATE;
    IF profile_id IS NULL THEN
        RAISE EXCEPTION 'profile not found'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_not_found';
    END IF;

    INSERT INTO media_subtitle_discovery_rule (
        media_profile_id, discovery_pattern, precedence, enabled
    ) VALUES (
        profile_id, btrim(discovery_pattern_input), precedence_input,
        COALESCE(enabled_input, TRUE)
    ) RETURNING media_subtitle_discovery_rule_id INTO rule_id_out;

    UPDATE media_profile
       SET configuration_version = configuration_version + 1, updated_at = now()
     WHERE media_profile_id = profile_id;
    RETURN rule_id_out;
END;
$$;



CREATE FUNCTION public.media_subtitle_policy_all_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'all'
$$;



CREATE FUNCTION public.media_subtitle_policy_none_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'none'
$$;



CREATE FUNCTION public.media_subtitle_policy_selected_v1() RETURNS text
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT 'selected'
$$;



CREATE FUNCTION public.media_video_bit_depth_supported_v1(value_input integer) RETURNS boolean
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT value_input IS NULL OR value_input IN (8, 10, 12, 16)
$$;



CREATE FUNCTION public.media_video_color_range_known_v1(value_input text) RETURNS boolean
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT NULLIF(lower(btrim(value_input)), '') IS NULL
        OR lower(btrim(value_input)) IN ('tv', 'pc')
$$;



CREATE FUNCTION public.media_video_color_value_known_v1(field_input text, value_input text) RETURNS boolean
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT CASE lower(btrim(field_input))
        WHEN 'color_primaries' THEN
            NULLIF(lower(btrim(value_input)), '') IS NULL
            OR lower(btrim(value_input)) IN (
                'bt709',
                'bt470m',
                'bt470bg',
                'smpte170m',
                'smpte240m',
                'film',
                'bt2020',
                'smpte428',
                'smpte431',
                'smpte432',
                'ebu3213'
            )
        WHEN 'color_transfer' THEN
            NULLIF(lower(btrim(value_input)), '') IS NULL
            OR lower(btrim(value_input)) IN (
                'bt709',
                'bt470m',
                'bt470bg',
                'smpte170m',
                'smpte240m',
                'linear',
                'log',
                'log_sqrt',
                'iec61966-2-4',
                'bt1361e',
                'iec61966-2-1',
                'bt2020-10',
                'bt2020-12',
                'smpte2084',
                'smpte428',
                'arib-std-b67'
            )
        WHEN 'color_space' THEN
            NULLIF(lower(btrim(value_input)), '') IS NULL
            OR lower(btrim(value_input)) IN (
                'gbr',
                'bt709',
                'fcc',
                'bt470bg',
                'smpte170m',
                'smpte240m',
                'ycgco',
                'bt2020nc',
                'bt2020c',
                'smpte2085',
                'chroma-derived-nc',
                'chroma-derived-c',
                'ictcp'
            )
        ELSE FALSE
    END
$$;



CREATE FUNCTION public.media_video_frame_rate_reduce_v1(value_input text) RETURNS text
    LANGUAGE plpgsql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $_$
DECLARE
    value_text TEXT;
    numerator_text TEXT;
    denominator_text TEXT;
    numerator_value BIGINT;
    denominator_value BIGINT;
    left_value BIGINT;
    right_value BIGINT;
    remainder_value BIGINT;
BEGIN
    IF value_input IS NULL THEN
        RETURN NULL;
    END IF;
    value_text := btrim(value_input);
    IF value_text !~ '^[1-9][0-9]*(/[1-9][0-9]*)?$' THEN
        RETURN NULL;
    END IF;
    numerator_text := split_part(value_text, '/', 1);
    denominator_text := CASE
        WHEN strpos(value_text, '/') = 0 THEN '1'
        ELSE split_part(value_text, '/', 2)
    END;
    IF length(numerator_text) > 7 OR length(denominator_text) > 7 THEN
        RETURN NULL;
    END IF;
    numerator_value := numerator_text::BIGINT;
    denominator_value := denominator_text::BIGINT;
    IF numerator_value > 1000000
        OR denominator_value > 1000000
        OR numerator_value > 240 * denominator_value THEN
        RETURN NULL;
    END IF;
    left_value := numerator_value;
    right_value := denominator_value;
    WHILE right_value <> 0 LOOP
        remainder_value := left_value % right_value;
        left_value := right_value;
        right_value := remainder_value;
    END LOOP;
    RETURN (numerator_value / left_value)::TEXT || '/' || (denominator_value / left_value)::TEXT;
END;
$_$;



CREATE FUNCTION public.media_video_frame_rate_valid_v1(value_input text) RETURNS boolean
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'public', 'pg_temp'
    AS $$
    SELECT value_input IS NULL OR media_video_frame_rate_reduce_v1(value_input) IS NOT NULL
$$;



