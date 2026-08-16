ALTER TABLE media_profile
    ADD COLUMN configuration_version BIGINT NOT NULL DEFAULT 1,
    ADD CONSTRAINT media_profile_configuration_version_positive CHECK (
        configuration_version > 0
    );

CREATE TABLE media_profile_root (
    media_profile_root_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_profile_root_public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    media_profile_id BIGINT NOT NULL REFERENCES media_profile(media_profile_id) ON DELETE CASCADE,
    root_kind TEXT NOT NULL,
    requested_path TEXT NOT NULL,
    canonical_path TEXT NOT NULL,
    filesystem_device BIGINT,
    filesystem_inode BIGINT,
    media_type TEXT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    identity_verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_profile_root_kind_known CHECK (
        root_kind IN ('source', 'output', 'workspace', 'backup', 'quarantine')
    ),
    CONSTRAINT media_profile_root_paths_absolute CHECK (
        requested_path LIKE '/%' AND canonical_path LIKE '/%'
    ),
    CONSTRAINT media_profile_root_paths_bounded CHECK (
        char_length(requested_path) BETWEEN 1 AND 4096
        AND char_length(canonical_path) BETWEEN 1 AND 4096
    ),
    CONSTRAINT media_profile_root_media_type_nonempty CHECK (btrim(media_type) <> ''),
    CONSTRAINT media_profile_root_sort_nonnegative CHECK (sort_order >= 0),
    CONSTRAINT media_profile_root_identity_complete CHECK (
        (filesystem_device IS NULL AND filesystem_inode IS NULL AND identity_verified_at IS NULL)
        OR (
            filesystem_device IS NOT NULL
            AND filesystem_inode IS NOT NULL
            AND filesystem_device >= 0
            AND filesystem_inode >= 0
            AND identity_verified_at IS NOT NULL
        )
    ),
    CONSTRAINT media_profile_root_enabled_verified CHECK (
        NOT enabled OR identity_verified_at IS NOT NULL
    )
);

CREATE UNIQUE INDEX uq_media_profile_root_order
    ON media_profile_root (media_profile_id, root_kind, sort_order);

CREATE UNIQUE INDEX uq_media_profile_root_identity_enabled
    ON media_profile_root (filesystem_device, filesystem_inode)
    WHERE enabled;

CREATE INDEX ix_media_profile_root_canonical_enabled
    ON media_profile_root (canonical_path, media_profile_root_id)
    WHERE enabled;

INSERT INTO media_profile_root (
    media_profile_id,
    root_kind,
    requested_path,
    canonical_path,
    media_type,
    sort_order,
    enabled
)
SELECT profile.media_profile_id,
       roots.root_kind,
       roots.root_path,
       roots.root_path,
       'mixed',
       0,
       FALSE
  FROM media_profile profile
 CROSS JOIN LATERAL (
     VALUES
         ('source', profile.source_root),
         ('output', profile.output_root)
 ) roots(root_kind, root_path);

CREATE TABLE media_profile_file_rule (
    media_profile_file_rule_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_profile_id BIGINT NOT NULL REFERENCES media_profile(media_profile_id) ON DELETE CASCADE,
    rule_kind TEXT NOT NULL,
    matcher_kind TEXT NOT NULL,
    matcher_value TEXT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_profile_file_rule_kind_known CHECK (rule_kind IN ('include', 'exclude')),
    CONSTRAINT media_profile_file_rule_matcher_known CHECK (matcher_kind IN ('glob', 'extension')),
    CONSTRAINT media_profile_file_rule_value_bounded CHECK (
        char_length(btrim(matcher_value)) BETWEEN 1 AND 512
    ),
    CONSTRAINT media_profile_file_rule_sort_nonnegative CHECK (sort_order >= 0),
    UNIQUE (media_profile_id, sort_order)
);

CREATE TABLE media_profile_filter (
    media_profile_id BIGINT PRIMARY KEY REFERENCES media_profile(media_profile_id) ON DELETE CASCADE,
    min_size_bytes BIGINT,
    max_size_bytes BIGINT,
    min_duration_millis BIGINT,
    max_duration_millis BIGINT,
    include_samples BOOLEAN NOT NULL DEFAULT FALSE,
    include_trailers BOOLEAN NOT NULL DEFAULT FALSE,
    exclude_trash BOOLEAN NOT NULL DEFAULT TRUE,
    exclude_quarantine BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_profile_filter_size_bounds CHECK (
        (min_size_bytes IS NULL OR min_size_bytes >= 0)
        AND (max_size_bytes IS NULL OR max_size_bytes >= 0)
        AND (min_size_bytes IS NULL OR max_size_bytes IS NULL OR min_size_bytes <= max_size_bytes)
    ),
    CONSTRAINT media_profile_filter_duration_bounds CHECK (
        (min_duration_millis IS NULL OR min_duration_millis >= 0)
        AND (max_duration_millis IS NULL OR max_duration_millis >= 0)
        AND (
            min_duration_millis IS NULL
            OR max_duration_millis IS NULL
            OR min_duration_millis <= max_duration_millis
        )
    ),
    CONSTRAINT media_profile_filter_is_bounded CHECK (
        min_size_bytes IS NOT NULL
        OR max_size_bytes IS NOT NULL
        OR min_duration_millis IS NOT NULL
        OR max_duration_millis IS NOT NULL
    )
);

CREATE TABLE media_subtitle_discovery_rule (
    media_subtitle_discovery_rule_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_profile_id BIGINT NOT NULL REFERENCES media_profile(media_profile_id) ON DELETE CASCADE,
    discovery_pattern TEXT NOT NULL,
    precedence INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_subtitle_discovery_rule_pattern_bounded CHECK (
        char_length(btrim(discovery_pattern)) BETWEEN 1 AND 512
    ),
    CONSTRAINT media_subtitle_discovery_rule_precedence_nonnegative CHECK (precedence >= 0),
    UNIQUE (media_profile_id, precedence)
);

CREATE TABLE media_discovery_schedule (
    media_discovery_schedule_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_discovery_schedule_public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    media_profile_id BIGINT NOT NULL REFERENCES media_profile(media_profile_id) ON DELETE CASCADE,
    media_profile_root_id BIGINT NOT NULL REFERENCES media_profile_root(media_profile_root_id) ON DELETE CASCADE,
    interval_value INT NOT NULL,
    interval_unit TEXT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    next_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_discovery_schedule_interval_bounds CHECK (interval_value BETWEEN 1 AND 525600),
    CONSTRAINT media_discovery_schedule_unit_known CHECK (interval_unit IN ('minutes', 'hours', 'days')),
    CONSTRAINT media_discovery_schedule_sort_nonnegative CHECK (sort_order >= 0),
    CONSTRAINT media_discovery_schedule_enabled_due CHECK (NOT enabled OR next_run_at IS NOT NULL),
    UNIQUE (media_profile_id, sort_order),
    UNIQUE (media_profile_root_id)
);

CREATE TABLE media_discovery_watcher (
    media_discovery_watcher_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_discovery_watcher_public_id UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    media_profile_id BIGINT NOT NULL REFERENCES media_profile(media_profile_id) ON DELETE CASCADE,
    media_profile_root_id BIGINT NOT NULL REFERENCES media_profile_root(media_profile_root_id) ON DELETE CASCADE,
    debounce_millis INT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_discovery_watcher_debounce_bounds CHECK (debounce_millis BETWEEN 100 AND 600000),
    CONSTRAINT media_discovery_watcher_sort_nonnegative CHECK (sort_order >= 0),
    UNIQUE (media_profile_id, sort_order),
    UNIQUE (media_profile_root_id)
);

CREATE TABLE media_policy_retention_rule (
    media_policy_retention_rule_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_policy_profile_id BIGINT NOT NULL REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    stream_kind TEXT NOT NULL,
    semantic_role TEXT,
    language_code TEXT,
    codec_or_format TEXT,
    action TEXT NOT NULL,
    placement TEXT,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT media_policy_retention_rule_stream_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle', 'attachment', 'data')
    ),
    CONSTRAINT media_policy_retention_rule_action_known CHECK (
        action IN ('retain', 'drop', 'convert', 'extract')
    ),
    CONSTRAINT media_policy_retention_rule_sort_nonnegative CHECK (sort_order >= 0),
    UNIQUE (media_policy_profile_id, sort_order)
);

CREATE TABLE media_policy_unmatched_stream_behavior (
    media_policy_profile_id BIGINT PRIMARY KEY REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    video_action TEXT NOT NULL,
    audio_action TEXT NOT NULL,
    subtitle_action TEXT NOT NULL,
    attachment_action TEXT NOT NULL,
    data_action TEXT NOT NULL,
    CONSTRAINT media_policy_unmatched_stream_actions_known CHECK (
        video_action IN ('retain', 'drop', 'fail')
        AND audio_action IN ('retain', 'drop', 'fail')
        AND subtitle_action IN ('retain', 'drop', 'fail')
        AND attachment_action IN ('retain', 'drop', 'fail')
        AND data_action IN ('retain', 'drop', 'fail')
    )
);

CREATE TABLE media_policy_compatibility_rule (
    media_policy_profile_id BIGINT PRIMARY KEY REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    unsupported_format_action TEXT NOT NULL,
    require_all_targets BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT media_policy_compatibility_rule_action_known CHECK (
        unsupported_format_action IN ('transcode', 'drop', 'fail')
    )
);

CREATE TABLE media_policy_compatibility_target (
    media_policy_profile_id BIGINT NOT NULL REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    media_compatibility_target_id BIGINT NOT NULL REFERENCES media_compatibility_target(media_compatibility_target_id),
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT media_policy_compatibility_target_sort_nonnegative CHECK (sort_order >= 0),
    PRIMARY KEY (media_policy_profile_id, media_compatibility_target_id),
    UNIQUE (media_policy_profile_id, sort_order)
);

CREATE TABLE media_policy_operation_cost (
    media_policy_profile_id BIGINT NOT NULL REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    operation_kind TEXT NOT NULL,
    cost_weight INT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT media_policy_operation_cost_kind_nonempty CHECK (btrim(operation_kind) <> ''),
    CONSTRAINT media_policy_operation_cost_weight_bounds CHECK (cost_weight BETWEEN 0 AND 1000000),
    CONSTRAINT media_policy_operation_cost_sort_nonnegative CHECK (sort_order >= 0),
    PRIMARY KEY (media_policy_profile_id, operation_kind),
    UNIQUE (media_policy_profile_id, sort_order)
);

CREATE TABLE media_stream_classification_rule (
    media_stream_classification_rule_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_policy_profile_id BIGINT NOT NULL REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    stream_kind TEXT NOT NULL,
    semantic_role TEXT NOT NULL,
    match_kind TEXT NOT NULL,
    match_pattern TEXT NOT NULL,
    confidence SMALLINT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_stream_classification_rule_stream_known CHECK (
        stream_kind IN ('video', 'audio', 'subtitle', 'attachment', 'data')
    ),
    CONSTRAINT media_stream_classification_rule_role_bounded CHECK (
        char_length(btrim(semantic_role)) BETWEEN 1 AND 64
    ),
    CONSTRAINT media_stream_classification_rule_match_known CHECK (
        match_kind IN ('title_contains', 'title_regex', 'disposition', 'language', 'codec', 'filename_glob')
    ),
    CONSTRAINT media_stream_classification_rule_pattern_bounded CHECK (
        char_length(btrim(match_pattern)) BETWEEN 1 AND 512
    ),
    CONSTRAINT media_stream_classification_rule_confidence_bounds CHECK (confidence BETWEEN 0 AND 100),
    CONSTRAINT media_stream_classification_rule_sort_nonnegative CHECK (sort_order >= 0),
    UNIQUE (media_policy_profile_id, sort_order)
);

CREATE TABLE media_policy_runtime_limit (
    media_policy_profile_id BIGINT PRIMARY KEY REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    max_concurrency INT NOT NULL,
    max_retries INT NOT NULL,
    max_runtime_seconds INT NOT NULL,
    max_io_megabytes_per_second INT NOT NULL,
    min_free_space_bytes BIGINT NOT NULL,
    pause_on_battery BOOLEAN NOT NULL DEFAULT TRUE,
    minimum_battery_percent INT,
    thermal_pressure_limit TEXT NOT NULL DEFAULT 'serious',
    pause_when_thermal_exceeded BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT media_policy_runtime_limit_concurrency_bounds CHECK (max_concurrency BETWEEN 1 AND 256),
    CONSTRAINT media_policy_runtime_limit_retry_bounds CHECK (max_retries BETWEEN 0 AND 100),
    CONSTRAINT media_policy_runtime_limit_runtime_bounds CHECK (max_runtime_seconds BETWEEN 1 AND 604800),
    CONSTRAINT media_policy_runtime_limit_io_bounds CHECK (max_io_megabytes_per_second BETWEEN 1 AND 1048576),
    CONSTRAINT media_policy_runtime_limit_space_bounds CHECK (min_free_space_bytes BETWEEN 0 AND 1152921504606846976),
    CONSTRAINT media_policy_runtime_limit_battery_bounds CHECK (
        minimum_battery_percent IS NULL OR minimum_battery_percent BETWEEN 1 AND 100
    ),
    CONSTRAINT media_policy_runtime_limit_battery_complete CHECK (
        pause_on_battery OR minimum_battery_percent IS NULL
    ),
    CONSTRAINT media_policy_runtime_limit_thermal_known CHECK (
        thermal_pressure_limit IN ('nominal', 'fair', 'serious', 'critical')
    )
);

CREATE TABLE media_policy_maintenance_window (
    media_policy_profile_id BIGINT NOT NULL REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    day_of_week SMALLINT NOT NULL,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT media_policy_maintenance_window_day_bounds CHECK (day_of_week BETWEEN 0 AND 6),
    CONSTRAINT media_policy_maintenance_window_not_empty CHECK (start_time <> end_time),
    CONSTRAINT media_policy_maintenance_window_sort_nonnegative CHECK (sort_order >= 0),
    PRIMARY KEY (media_policy_profile_id, day_of_week, start_time),
    UNIQUE (media_policy_profile_id, sort_order)
);

CREATE TABLE media_policy_output (
    media_policy_profile_id BIGINT PRIMARY KEY REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    dry_run BOOLEAN NOT NULL DEFAULT TRUE,
    replacement_mode TEXT NOT NULL,
    quarantine_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    preserve_permissions BOOLEAN NOT NULL DEFAULT TRUE,
    preserve_ownership BOOLEAN NOT NULL DEFAULT TRUE,
    CONSTRAINT media_policy_output_replacement_known CHECK (
        replacement_mode IN ('disabled', 'atomic_replace', 'side_by_side')
    )
);

CREATE TABLE media_policy_workspace (
    media_policy_profile_id BIGINT PRIMARY KEY REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    retention_hours INT NOT NULL,
    diagnostics_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    stale_cleanup_hours INT NOT NULL,
    max_workspace_bytes BIGINT NOT NULL,
    CONSTRAINT media_policy_workspace_retention_bounds CHECK (retention_hours BETWEEN 1 AND 87600),
    CONSTRAINT media_policy_workspace_cleanup_bounds CHECK (stale_cleanup_hours BETWEEN 1 AND 87600),
    CONSTRAINT media_policy_workspace_size_bounds CHECK (max_workspace_bytes BETWEEN 1 AND 1152921504606846976)
);

CREATE TABLE media_policy_backup (
    media_policy_profile_id BIGINT PRIMARY KEY REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    retention_days INT,
    min_free_space_bytes BIGINT,
    CONSTRAINT media_policy_backup_complete CHECK (
        NOT enabled OR (retention_days IS NOT NULL AND min_free_space_bytes IS NOT NULL)
    ),
    CONSTRAINT media_policy_backup_retention_bounds CHECK (
        retention_days IS NULL OR retention_days BETWEEN 1 AND 3650
    ),
    CONSTRAINT media_policy_backup_space_bounds CHECK (
        min_free_space_bytes IS NULL OR min_free_space_bytes BETWEEN 0 AND 1152921504606846976
    )
);

CREATE TABLE media_policy_verification (
    media_policy_profile_id BIGINT PRIMARY KEY REFERENCES media_policy_profile(media_policy_profile_id) ON DELETE CASCADE,
    strictness TEXT NOT NULL,
    duration_tolerance_millis BIGINT NOT NULL,
    mux_validation BOOLEAN NOT NULL,
    decode_all_streams BOOLEAN NOT NULL,
    keyframe_seek BOOLEAN NOT NULL,
    playback_probe BOOLEAN NOT NULL,
    CONSTRAINT media_policy_verification_strictness_known CHECK (strictness IN ('strict', 'balanced', 'fast')),
    CONSTRAINT media_policy_verification_duration_bounds CHECK (duration_tolerance_millis BETWEEN 0 AND 60000),
    CONSTRAINT media_policy_verification_strict_complete CHECK (
        strictness <> 'strict'
        OR (mux_validation AND decode_all_streams AND keyframe_seek AND playback_probe)
    )
);

INSERT INTO media_policy_verification (
    media_policy_profile_id,
    strictness,
    duration_tolerance_millis,
    mux_validation,
    decode_all_streams,
    keyframe_seek,
    playback_probe
)
SELECT media_policy_profile_id,
       verification_strictness,
       verification_duration_tolerance_millis,
       verification_mux_validation,
       verification_decode_all_streams,
       verification_keyframe_seek,
       verification_playback_probe
  FROM media_policy_profile;

CREATE OR REPLACE FUNCTION media_policy_seed_bounded_defaults_v1(policy_id_input BIGINT)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

SELECT media_policy_seed_bounded_defaults_v1(policy.media_policy_profile_id)
  FROM media_policy_profile policy;

CREATE OR REPLACE FUNCTION media_policy_seed_bounded_defaults_trigger_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM media_policy_seed_bounded_defaults_v1(NEW.media_policy_profile_id);
    RETURN NEW;
END;
$$;

CREATE TRIGGER media_policy_seed_bounded_defaults_trigger
AFTER INSERT ON media_policy_profile
FOR EACH ROW
EXECUTE FUNCTION media_policy_seed_bounded_defaults_trigger_v1();

CREATE TABLE media_job_configuration_snapshot (
    media_job_id BIGINT PRIMARY KEY REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    profile_configuration_version BIGINT NOT NULL,
    media_policy_profile_id BIGINT REFERENCES media_policy_profile(media_policy_profile_id),
    policy_version INT,
    media_desired_target_profile_id BIGINT REFERENCES media_desired_target_profile(media_desired_target_profile_id),
    desired_target_version INT,
    captured_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT media_job_configuration_snapshot_profile_version_positive CHECK (
        profile_configuration_version > 0
    )
);

CREATE TABLE media_job_root_snapshot (
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    root_kind TEXT NOT NULL,
    requested_path TEXT NOT NULL,
    canonical_path TEXT NOT NULL,
    filesystem_device BIGINT,
    filesystem_inode BIGINT,
    media_type TEXT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (media_job_id, root_kind, sort_order)
);

CREATE TABLE media_job_file_rule_snapshot (
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    rule_kind TEXT NOT NULL,
    matcher_kind TEXT NOT NULL,
    matcher_value TEXT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (media_job_id, sort_order)
);

CREATE TABLE media_job_filter_snapshot (
    media_job_id BIGINT PRIMARY KEY REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    min_size_bytes BIGINT,
    max_size_bytes BIGINT,
    min_duration_millis BIGINT,
    max_duration_millis BIGINT,
    include_samples BOOLEAN NOT NULL,
    include_trailers BOOLEAN NOT NULL,
    exclude_trash BOOLEAN NOT NULL,
    exclude_quarantine BOOLEAN NOT NULL
);

CREATE TABLE media_job_subtitle_discovery_rule_snapshot (
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    discovery_pattern TEXT NOT NULL,
    precedence INT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (media_job_id, precedence)
);

CREATE TABLE media_job_policy_retention_rule_snapshot (
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    stream_kind TEXT NOT NULL,
    semantic_role TEXT,
    language_code TEXT,
    codec_or_format TEXT,
    action TEXT NOT NULL,
    placement TEXT,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (media_job_id, sort_order)
);

CREATE TABLE media_job_policy_compatibility_target_snapshot (
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    compatibility_target_key TEXT NOT NULL,
    compatibility_target_version INT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (media_job_id, sort_order)
);

CREATE TABLE media_job_policy_operation_cost_snapshot (
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    operation_kind TEXT NOT NULL,
    cost_weight INT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (media_job_id, sort_order)
);

CREATE TABLE media_job_stream_classification_rule_snapshot (
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    stream_kind TEXT NOT NULL,
    semantic_role TEXT NOT NULL,
    match_kind TEXT NOT NULL,
    match_pattern TEXT NOT NULL,
    confidence SMALLINT NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (media_job_id, sort_order)
);

CREATE TABLE media_job_policy_maintenance_window_snapshot (
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    day_of_week SMALLINT NOT NULL,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    sort_order INT NOT NULL,
    enabled BOOLEAN NOT NULL,
    PRIMARY KEY (media_job_id, sort_order)
);

CREATE TABLE media_job_policy_behavior_snapshot (
    media_job_id BIGINT PRIMARY KEY REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    unmatched_video_action TEXT,
    unmatched_audio_action TEXT,
    unmatched_subtitle_action TEXT,
    unmatched_attachment_action TEXT,
    unmatched_data_action TEXT,
    unsupported_format_action TEXT,
    require_all_compatibility_targets BOOLEAN,
    max_concurrency INT,
    max_retries INT,
    max_runtime_seconds INT,
    max_io_megabytes_per_second INT,
    min_free_space_bytes BIGINT,
    pause_on_battery BOOLEAN,
    minimum_battery_percent INT,
    thermal_pressure_limit TEXT,
    pause_when_thermal_exceeded BOOLEAN,
    dry_run BOOLEAN,
    replacement_mode TEXT,
    quarantine_enabled BOOLEAN,
    preserve_permissions BOOLEAN,
    preserve_ownership BOOLEAN,
    workspace_retention_hours INT,
    diagnostics_enabled BOOLEAN,
    stale_cleanup_hours INT,
    max_workspace_bytes BIGINT,
    backup_enabled BOOLEAN,
    backup_retention_days INT,
    backup_min_free_space_bytes BIGINT,
    verification_strictness TEXT,
    verification_duration_tolerance_millis BIGINT,
    verification_mux_validation BOOLEAN,
    verification_decode_all_streams BOOLEAN,
    verification_keyframe_seek BOOLEAN,
    verification_playback_probe BOOLEAN
);

CREATE OR REPLACE FUNCTION media_profile_validate_root_identity_v1(
    media_profile_root_id_input BIGINT,
    canonical_path_input TEXT,
    filesystem_device_input BIGINT,
    filesystem_inode_input BIGINT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
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
$$;

CREATE OR REPLACE FUNCTION media_profile_root_add_v1(
    media_profile_public_id_input UUID,
    root_kind_input TEXT,
    requested_path_input TEXT,
    canonical_path_input TEXT,
    filesystem_device_input BIGINT,
    filesystem_inode_input BIGINT,
    media_type_input TEXT,
    sort_order_input INT,
    enabled_input BOOLEAN
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
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
$$;
CREATE OR REPLACE FUNCTION media_profile_root_revalidate_v1(
    media_profile_root_public_id_input UUID,
    canonical_path_input TEXT,
    filesystem_device_input BIGINT,
    filesystem_inode_input BIGINT
)
RETURNS TIMESTAMPTZ
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
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
$$;

CREATE OR REPLACE FUNCTION media_profile_file_rule_append_v1(
    media_profile_public_id_input UUID,
    rule_kind_input TEXT,
    matcher_kind_input TEXT,
    matcher_value_input TEXT,
    sort_order_input INT,
    enabled_input BOOLEAN
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_profile_filter_set_v1(
    media_profile_public_id_input UUID,
    min_size_bytes_input BIGINT,
    max_size_bytes_input BIGINT,
    min_duration_millis_input BIGINT,
    max_duration_millis_input BIGINT,
    include_samples_input BOOLEAN,
    include_trailers_input BOOLEAN,
    exclude_trash_input BOOLEAN,
    exclude_quarantine_input BOOLEAN
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_subtitle_discovery_rule_append_v1(
    media_profile_public_id_input UUID,
    discovery_pattern_input TEXT,
    precedence_input INT,
    enabled_input BOOLEAN
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_discovery_schedule_create_v1(
    media_profile_public_id_input UUID,
    media_profile_root_public_id_input UUID,
    interval_value_input INT,
    interval_unit_input TEXT,
    sort_order_input INT,
    enabled_input BOOLEAN,
    next_run_at_input TIMESTAMPTZ
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_discovery_watcher_create_v1(
    media_profile_public_id_input UUID,
    media_profile_root_public_id_input UUID,
    debounce_millis_input INT,
    sort_order_input INT,
    enabled_input BOOLEAN
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_discovery_root_assert_current_v1(
    media_profile_root_public_id_input UUID,
    canonical_path_input TEXT,
    filesystem_device_input BIGINT,
    filesystem_inode_input BIGINT
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
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
$$;

CREATE OR REPLACE FUNCTION media_discovery_schedule_claim_v1(
    media_discovery_schedule_public_id_input UUID,
    canonical_path_input TEXT,
    filesystem_device_input BIGINT,
    filesystem_inode_input BIGINT,
    claimed_at_input TIMESTAMPTZ
)
RETURNS TABLE (
    media_profile_public_id UUID,
    media_profile_root_public_id UUID,
    next_run_at TIMESTAMPTZ
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_discovery_watcher_start_v1(
    media_discovery_watcher_public_id_input UUID,
    canonical_path_input TEXT,
    filesystem_device_input BIGINT,
    filesystem_inode_input BIGINT
)
RETURNS TABLE (
    media_profile_public_id UUID,
    media_profile_root_public_id UUID
)
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_profile_create_v3(
    actor_public_id_input UUID,
    profile_key_input TEXT,
    source_requested_path_input TEXT,
    source_canonical_path_input TEXT,
    source_filesystem_device_input BIGINT,
    source_filesystem_inode_input BIGINT,
    output_requested_path_input TEXT,
    output_canonical_path_input TEXT,
    output_filesystem_device_input BIGINT,
    output_filesystem_inode_input BIGINT,
    retention_days_input INT,
    compatibility_target_key_input TEXT,
    policy_key_input TEXT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    actor_id BIGINT;
    profile_id BIGINT;
    profile_public_id_out UUID;
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
            FALSE,
            FALSE,
            NULL,
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
        ),
        (
            profile_id, 'output', btrim(output_requested_path_input),
            regexp_replace(btrim(output_canonical_path_input), '/+$', ''),
            output_filesystem_device_input, output_filesystem_inode_input,
            'mixed', 0, TRUE, now()
        );

    RETURN profile_public_id_out;
END;
$$;

CREATE OR REPLACE FUNCTION media_profile_upsert_v2(
    actor_public_id_input UUID,
    profile_key_input TEXT,
    source_root_input TEXT,
    output_root_input TEXT,
    dry_run_only_input BOOLEAN,
    retention_days_input INT,
    compatibility_target_key_input TEXT,
    policy_key_input TEXT,
    watcher_enabled_input BOOLEAN,
    schedule_enabled_input BOOLEAN,
    schedule_interval_minutes_input INT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_profile_upsert_v1(
    actor_public_id_input UUID,
    profile_key_input TEXT,
    source_root_input TEXT,
    output_root_input TEXT,
    dry_run_only_input BOOLEAN,
    retention_days_input INT
)
RETURNS UUID
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
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

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM media_profile
         WHERE deleted_at IS NULL AND (watcher_enabled OR schedule_enabled)
    ) THEN
        RAISE EXCEPTION 'legacy media automation requires filesystem identity migration'
            USING ERRCODE = media_app_error_code_v1(), DETAIL = 'media_profile_legacy_automation_unverified';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION media_profile_update_v1(
    actor_public_id_input UUID,
    media_profile_public_id_input UUID,
    source_root_input TEXT,
    output_root_input TEXT,
    dry_run_only_input BOOLEAN,
    retention_days_input INT,
    compatibility_target_key_input TEXT,
    policy_key_input TEXT,
    watcher_enabled_input BOOLEAN,
    schedule_enabled_input BOOLEAN,
    schedule_interval_minutes_input INT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_policy_profile_immutable_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
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

CREATE TRIGGER media_policy_profile_immutable_trigger
BEFORE UPDATE ON media_policy_profile
FOR EACH ROW
EXECUTE FUNCTION media_policy_profile_immutable_v1();

CREATE OR REPLACE FUNCTION media_job_capture_configuration_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
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

CREATE TRIGGER media_job_capture_configuration_trigger
AFTER INSERT ON media_job
FOR EACH ROW
EXECUTE FUNCTION media_job_capture_configuration_v1();

INSERT INTO media_job_configuration_snapshot (
    media_job_id,
    profile_configuration_version,
    media_policy_profile_id,
    policy_version,
    media_desired_target_profile_id,
    desired_target_version
)
SELECT job.media_job_id,
       profile.configuration_version,
       job.intent_policy_profile_id,
       job.intent_policy_version,
       job.intent_desired_target_profile_id,
       job.intent_desired_target_version
  FROM media_job job
  JOIN media_profile profile ON profile.media_profile_id = job.media_profile_id;

INSERT INTO media_job_root_snapshot (
    media_job_id, root_kind, requested_path, canonical_path,
    filesystem_device, filesystem_inode, media_type, sort_order, enabled
)
SELECT job.media_job_id, root.root_kind, root.requested_path, root.canonical_path,
       root.filesystem_device, root.filesystem_inode, root.media_type,
       root.sort_order, root.enabled
  FROM media_job job
  JOIN media_profile_root root ON root.media_profile_id = job.media_profile_id;

INSERT INTO media_job_file_rule_snapshot (
    media_job_id, rule_kind, matcher_kind, matcher_value, sort_order, enabled
)
SELECT job.media_job_id, rule.rule_kind, rule.matcher_kind, rule.matcher_value,
       rule.sort_order, rule.enabled
  FROM media_job job
  JOIN media_profile_file_rule rule ON rule.media_profile_id = job.media_profile_id;

INSERT INTO media_job_filter_snapshot (
    media_job_id, min_size_bytes, max_size_bytes, min_duration_millis,
    max_duration_millis, include_samples, include_trailers,
    exclude_trash, exclude_quarantine
)
SELECT job.media_job_id, filter.min_size_bytes, filter.max_size_bytes,
       filter.min_duration_millis, filter.max_duration_millis,
       filter.include_samples, filter.include_trailers,
       filter.exclude_trash, filter.exclude_quarantine
  FROM media_job job
  JOIN media_profile_filter filter ON filter.media_profile_id = job.media_profile_id;

INSERT INTO media_job_subtitle_discovery_rule_snapshot (
    media_job_id, discovery_pattern, precedence, enabled
)
SELECT job.media_job_id, rule.discovery_pattern, rule.precedence, rule.enabled
  FROM media_job job
  JOIN media_subtitle_discovery_rule rule ON rule.media_profile_id = job.media_profile_id;

INSERT INTO media_job_policy_retention_rule_snapshot (
    media_job_id, stream_kind, semantic_role, language_code, codec_or_format,
    action, placement, sort_order, enabled
)
SELECT job.media_job_id, rule.stream_kind, rule.semantic_role,
       rule.language_code, rule.codec_or_format, rule.action, rule.placement,
       rule.sort_order, rule.enabled
  FROM media_job job
  JOIN media_policy_retention_rule rule
    ON rule.media_policy_profile_id = job.intent_policy_profile_id;

INSERT INTO media_job_policy_compatibility_target_snapshot (
    media_job_id, compatibility_target_key, compatibility_target_version,
    sort_order, enabled
)
SELECT job.media_job_id, target.compatibility_target_key, target.version,
       selected.sort_order, selected.enabled
  FROM media_job job
  JOIN media_policy_compatibility_target selected
    ON selected.media_policy_profile_id = job.intent_policy_profile_id
  JOIN media_compatibility_target target
    ON target.media_compatibility_target_id = selected.media_compatibility_target_id;

INSERT INTO media_job_policy_operation_cost_snapshot (
    media_job_id, operation_kind, cost_weight, sort_order, enabled
)
SELECT job.media_job_id, cost.operation_kind, cost.cost_weight,
       cost.sort_order, cost.enabled
  FROM media_job job
  JOIN media_policy_operation_cost cost
    ON cost.media_policy_profile_id = job.intent_policy_profile_id;

INSERT INTO media_job_stream_classification_rule_snapshot (
    media_job_id, stream_kind, semantic_role, match_kind, match_pattern,
    confidence, sort_order, enabled
)
SELECT job.media_job_id, rule.stream_kind, rule.semantic_role,
       rule.match_kind, rule.match_pattern, rule.confidence,
       rule.sort_order, rule.enabled
  FROM media_job job
  JOIN media_stream_classification_rule rule
    ON rule.media_policy_profile_id = job.intent_policy_profile_id;

INSERT INTO media_job_policy_maintenance_window_snapshot (
    media_job_id, day_of_week, start_time, end_time, sort_order, enabled
)
SELECT job.media_job_id, maintenance.day_of_week, maintenance.start_time,
       maintenance.end_time, maintenance.sort_order, maintenance.enabled
  FROM media_job job
  JOIN media_policy_maintenance_window maintenance
    ON maintenance.media_policy_profile_id = job.intent_policy_profile_id;

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
SELECT job.media_job_id,
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
  FROM media_job job
  JOIN media_policy_profile policy
    ON policy.media_policy_profile_id = job.intent_policy_profile_id
  JOIN media_policy_unmatched_stream_behavior unmatched
    ON unmatched.media_policy_profile_id = policy.media_policy_profile_id
  JOIN media_policy_compatibility_rule compatibility
    ON compatibility.media_policy_profile_id = policy.media_policy_profile_id
  JOIN media_policy_runtime_limit runtime
    ON runtime.media_policy_profile_id = policy.media_policy_profile_id
  JOIN media_policy_output output
    ON output.media_policy_profile_id = policy.media_policy_profile_id
  JOIN media_policy_workspace workspace
    ON workspace.media_policy_profile_id = policy.media_policy_profile_id
  JOIN media_policy_backup backup
    ON backup.media_policy_profile_id = policy.media_policy_profile_id
  JOIN media_policy_verification verification
    ON verification.media_policy_profile_id = policy.media_policy_profile_id;

CREATE OR REPLACE FUNCTION media_job_snapshot_update_rejected_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
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

CREATE TRIGGER media_job_configuration_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_configuration_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_root_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_root_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_file_rule_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_file_rule_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_filter_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_filter_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_subtitle_discovery_rule_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_subtitle_discovery_rule_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_policy_retention_rule_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_policy_retention_rule_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_policy_compatibility_target_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_policy_compatibility_target_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_policy_operation_cost_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_policy_operation_cost_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_stream_classification_rule_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_stream_classification_rule_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_policy_maintenance_window_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_policy_maintenance_window_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();
CREATE TRIGGER media_job_policy_behavior_snapshot_immutable_trigger
BEFORE UPDATE OR DELETE ON media_job_policy_behavior_snapshot
FOR EACH ROW EXECUTE FUNCTION media_job_snapshot_update_rejected_v1();

CREATE OR REPLACE FUNCTION media_policy_component_version_guard_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
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

CREATE TRIGGER media_policy_retention_rule_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_retention_rule
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_unmatched_stream_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_unmatched_stream_behavior
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_compatibility_rule_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_compatibility_rule
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_compatibility_target_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_compatibility_target
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_operation_cost_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_operation_cost
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_stream_classification_rule_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_stream_classification_rule
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_runtime_limit_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_runtime_limit
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_maintenance_window_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_maintenance_window
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_output_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_output
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_workspace_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_workspace
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_backup_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_backup
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();
CREATE TRIGGER media_policy_verification_version_guard_trigger
BEFORE INSERT OR UPDATE OR DELETE ON media_policy_verification
FOR EACH ROW EXECUTE FUNCTION media_policy_component_version_guard_v1();

CREATE OR REPLACE FUNCTION media_job_configuration_immutable_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
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

CREATE TRIGGER media_job_configuration_immutable_trigger
BEFORE UPDATE ON media_job
FOR EACH ROW
EXECUTE FUNCTION media_job_configuration_immutable_v1();

CREATE TABLE media_job_attempt (
    media_job_attempt_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    media_job_id BIGINT NOT NULL REFERENCES media_job(media_job_id) ON DELETE CASCADE,
    attempt_number INT NOT NULL,
    claim_generation BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    status media_job_status NOT NULL DEFAULT media_job_status_queued_v1(),
    queued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    claimed_at TIMESTAMPTZ,
    heartbeat_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    last_error TEXT,
    cancel_generation_at_claim BIGINT,
    CONSTRAINT media_job_attempt_number_positive CHECK (attempt_number > 0),
    CONSTRAINT media_job_attempt_lifecycle CHECK (
        (status = media_job_status_queued_v1()
            AND claimed_at IS NULL AND heartbeat_at IS NULL AND completed_at IS NULL)
        OR (status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
            AND claimed_at IS NOT NULL AND heartbeat_at IS NOT NULL AND completed_at IS NULL)
        OR (status IN (
                media_job_status_completed_v1(),
                media_job_status_failed_v1(),
                media_job_status_cancelled_v1()
            )
            AND claimed_at IS NOT NULL AND completed_at IS NOT NULL)
    ),
    UNIQUE (media_job_id, attempt_number),
    UNIQUE (media_job_id, media_job_attempt_id)
);

CREATE INDEX ix_media_job_attempt_current
    ON media_job_attempt (media_job_id, attempt_number DESC, media_job_attempt_id DESC);

INSERT INTO media_job_attempt (
    media_job_id,
    attempt_number,
    status,
    queued_at,
    claimed_at,
    heartbeat_at,
    completed_at,
    last_error,
    cancel_generation_at_claim
)
SELECT job.media_job_id,
       1,
       job.status,
       job.queued_at,
       CASE WHEN job.status = media_job_status_queued_v1() THEN NULL ELSE COALESCE(job.started_at, job.queued_at) END,
       CASE
           WHEN job.status IN (media_job_status_running_v1(), media_job_status_verifying_v1())
               THEN COALESCE(job.heartbeat_at, job.started_at, job.queued_at)
       END,
       CASE
           WHEN job.status IN (
               media_job_status_completed_v1(), media_job_status_failed_v1(), media_job_status_cancelled_v1()
           ) THEN COALESCE(job.completed_at, job.started_at, job.queued_at)
       END,
       job.last_error,
       CASE WHEN job.status = media_job_status_queued_v1() THEN NULL ELSE job.cancel_generation END
  FROM media_job job;

ALTER TABLE media_job
    ADD COLUMN current_attempt_id BIGINT,
    ADD CONSTRAINT media_job_current_attempt_fk
        FOREIGN KEY (media_job_id, current_attempt_id)
        REFERENCES media_job_attempt(media_job_id, media_job_attempt_id);

UPDATE media_job job
   SET current_attempt_id = attempt.media_job_attempt_id
  FROM media_job_attempt attempt
 WHERE attempt.media_job_id = job.media_job_id
   AND attempt.attempt_number = 1;

CREATE OR REPLACE FUNCTION media_job_initial_attempt_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
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

CREATE TRIGGER media_job_initial_attempt_trigger
AFTER INSERT ON media_job
FOR EACH ROW
EXECUTE FUNCTION media_job_initial_attempt_v1();

CREATE OR REPLACE FUNCTION media_job_current_attempt_required_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
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

CREATE CONSTRAINT TRIGGER media_job_current_attempt_required_trigger
AFTER INSERT OR UPDATE OF current_attempt_id ON media_job
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW
EXECUTE FUNCTION media_job_current_attempt_required_v1();

ALTER TABLE media_job_phase ADD COLUMN media_job_attempt_id BIGINT;
ALTER TABLE media_job_operation ADD COLUMN media_job_attempt_id BIGINT;
ALTER TABLE media_job_violation ADD COLUMN media_job_attempt_id BIGINT;
ALTER TABLE media_job_plan_reason ADD COLUMN media_job_attempt_id BIGINT;
ALTER TABLE media_job_verification_check ADD COLUMN media_job_attempt_id BIGINT;
ALTER TABLE media_job_artifact ADD COLUMN media_job_attempt_id BIGINT;
ALTER TABLE media_job_compact_audit ADD COLUMN media_job_attempt_id BIGINT;

UPDATE media_job_phase evidence
   SET media_job_attempt_id = job.current_attempt_id
  FROM media_job job WHERE job.media_job_id = evidence.media_job_id;
UPDATE media_job_operation evidence
   SET media_job_attempt_id = job.current_attempt_id
  FROM media_job job WHERE job.media_job_id = evidence.media_job_id;
UPDATE media_job_violation evidence
   SET media_job_attempt_id = job.current_attempt_id
  FROM media_job job WHERE job.media_job_id = evidence.media_job_id;
UPDATE media_job_plan_reason evidence
   SET media_job_attempt_id = job.current_attempt_id
  FROM media_job job WHERE job.media_job_id = evidence.media_job_id;
UPDATE media_job_verification_check evidence
   SET media_job_attempt_id = job.current_attempt_id
  FROM media_job job WHERE job.media_job_id = evidence.media_job_id;
UPDATE media_job_artifact evidence
   SET media_job_attempt_id = job.current_attempt_id
  FROM media_job job WHERE job.media_job_id = evidence.media_job_id;
UPDATE media_job_compact_audit evidence
   SET media_job_attempt_id = job.current_attempt_id
  FROM media_job job WHERE job.media_job_id = evidence.media_job_id;

ALTER TABLE media_job_phase
    ALTER COLUMN media_job_attempt_id SET NOT NULL,
    ADD CONSTRAINT media_job_phase_attempt_fk FOREIGN KEY (media_job_id, media_job_attempt_id)
        REFERENCES media_job_attempt(media_job_id, media_job_attempt_id) ON DELETE CASCADE;
ALTER TABLE media_job_operation
    ALTER COLUMN media_job_attempt_id SET NOT NULL,
    ADD CONSTRAINT media_job_operation_attempt_fk FOREIGN KEY (media_job_id, media_job_attempt_id)
        REFERENCES media_job_attempt(media_job_id, media_job_attempt_id) ON DELETE CASCADE;
ALTER TABLE media_job_violation
    ALTER COLUMN media_job_attempt_id SET NOT NULL,
    ADD CONSTRAINT media_job_violation_attempt_fk FOREIGN KEY (media_job_id, media_job_attempt_id)
        REFERENCES media_job_attempt(media_job_id, media_job_attempt_id) ON DELETE CASCADE;
ALTER TABLE media_job_plan_reason
    ALTER COLUMN media_job_attempt_id SET NOT NULL,
    ADD CONSTRAINT media_job_plan_reason_attempt_fk FOREIGN KEY (media_job_id, media_job_attempt_id)
        REFERENCES media_job_attempt(media_job_id, media_job_attempt_id) ON DELETE CASCADE;
ALTER TABLE media_job_verification_check
    ALTER COLUMN media_job_attempt_id SET NOT NULL,
    ADD CONSTRAINT media_job_verification_check_attempt_fk FOREIGN KEY (media_job_id, media_job_attempt_id)
        REFERENCES media_job_attempt(media_job_id, media_job_attempt_id) ON DELETE CASCADE;
ALTER TABLE media_job_artifact
    ALTER COLUMN media_job_attempt_id SET NOT NULL,
    ADD CONSTRAINT media_job_artifact_attempt_fk FOREIGN KEY (media_job_id, media_job_attempt_id)
        REFERENCES media_job_attempt(media_job_id, media_job_attempt_id) ON DELETE CASCADE;
ALTER TABLE media_job_compact_audit
    ALTER COLUMN media_job_attempt_id SET NOT NULL,
    ADD CONSTRAINT media_job_compact_audit_attempt_fk FOREIGN KEY (media_job_id, media_job_attempt_id)
        REFERENCES media_job_attempt(media_job_id, media_job_attempt_id) ON DELETE CASCADE;

DROP INDEX uq_media_job_phase_job_index;
DROP INDEX uq_media_job_operation_job_index;
DROP INDEX uq_media_job_violation_job_index;
DROP INDEX uq_media_job_plan_reason_job_index;
DROP INDEX uq_media_job_verification_check_job_index;
DROP INDEX uq_media_job_artifact_job_index;
DROP INDEX uq_media_job_compact_audit_public_job_index;

CREATE UNIQUE INDEX uq_media_job_phase_attempt_index
    ON media_job_phase (media_job_attempt_id, phase_index);
CREATE UNIQUE INDEX uq_media_job_operation_attempt_index
    ON media_job_operation (media_job_attempt_id, operation_index);
CREATE UNIQUE INDEX uq_media_job_violation_attempt_index
    ON media_job_violation (media_job_attempt_id, violation_index);
CREATE UNIQUE INDEX uq_media_job_plan_reason_attempt_index
    ON media_job_plan_reason (media_job_attempt_id, reason_index);
CREATE UNIQUE INDEX uq_media_job_verification_check_attempt_index
    ON media_job_verification_check (media_job_attempt_id, check_index);
CREATE UNIQUE INDEX uq_media_job_artifact_attempt_index
    ON media_job_artifact (media_job_attempt_id, artifact_index);
CREATE UNIQUE INDEX uq_media_job_compact_audit_attempt_index
    ON media_job_compact_audit (media_job_attempt_id, audit_index);

CREATE OR REPLACE FUNCTION media_job_attempt_guard_v1()
RETURNS TRIGGER
LANGUAGE plpgsql
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

CREATE TRIGGER media_job_attempt_guard_trigger
BEFORE UPDATE ON media_job_attempt
FOR EACH ROW
EXECUTE FUNCTION media_job_attempt_guard_v1();

CREATE OR REPLACE FUNCTION media_job_current_attempt_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT
)
RETURNS TABLE (media_job_id BIGINT, media_job_attempt_id BIGINT, attempt_number INT)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_job_retry_v1(UUID);
CREATE FUNCTION media_job_retry_v1(media_job_public_id_input UUID)
RETURNS INT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_job_worker_recover_stale_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    stale_before_input TIMESTAMPTZ,
    recovered_at_input TIMESTAMPTZ
)
RETURNS INT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_job_worker_claim_next_v1();
DROP FUNCTION media_job_worker_claim_next_v2();
CREATE FUNCTION media_job_worker_claim_next_v2()
RETURNS TABLE (
    media_job_public_id UUID,
    media_profile_public_id UUID,
    source_path TEXT,
    output_path TEXT,
    dry_run BOOLEAN,
    source_root TEXT,
    output_root TEXT,
    compatibility_target_key TEXT,
    policy_key TEXT,
    target_video_codec TEXT,
    target_audio_codec TEXT,
    target_audio_channels INT,
    target_audio_channel_layout TEXT,
    target_subtitle_policy TEXT,
    policy_video_intent TEXT,
    desired_target_key TEXT,
    desired_target_version INT,
    desired_container_format TEXT,
    unmatched_stream_policy TEXT,
    verification_strictness TEXT,
    verification_duration_tolerance_millis BIGINT,
    verification_mux_validation BOOLEAN,
    verification_decode_all_streams BOOLEAN,
    verification_keyframe_seek BOOLEAN,
    verification_playback_probe BOOLEAN,
    cancel_generation BIGINT,
    attempt_number INT,
    claim_generation BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_job_worker_heartbeat_v1(UUID);
CREATE FUNCTION media_job_worker_heartbeat_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_job_worker_poll_control_v1(UUID, BIGINT);
CREATE FUNCTION media_job_worker_poll_control_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    observed_cancel_generation_input BIGINT
)
RETURNS TABLE (cancel_requested BOOLEAN, cancel_generation BIGINT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_job_worker_acknowledge_cancel_v1(UUID, BIGINT);
CREATE FUNCTION media_job_worker_acknowledge_cancel_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    observed_cancel_generation_input BIGINT
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_job_worker_complete_v1(UUID, BIGINT);
CREATE FUNCTION media_job_worker_complete_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    observed_cancel_generation_input BIGINT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_job_worker_mark_status_v1(UUID, media_job_status, TEXT);
CREATE FUNCTION media_job_worker_mark_status_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    status_input media_job_status,
    last_error_input TEXT DEFAULT NULL
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_job_phase_append_v1(UUID, INT, TEXT, TEXT, TEXT);
CREATE FUNCTION media_job_phase_append_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    phase_index_input INT,
    phase_name_input TEXT,
    phase_status_input TEXT,
    details_text_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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
    );
END;
$$;

DROP FUNCTION media_job_operation_append_v1(UUID, INT, TEXT, INT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT);
CREATE FUNCTION media_job_operation_append_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    operation_index_input INT,
    operation_kind_input TEXT,
    stream_id_input INT,
    command_bin_input TEXT,
    arg_1_input TEXT,
    arg_2_input TEXT,
    arg_3_input TEXT,
    arg_4_input TEXT,
    arg_5_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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
    );
END;
$$;

DROP FUNCTION media_job_violation_append_v1(UUID, INT, TEXT, TEXT, INT);
CREATE FUNCTION media_job_violation_append_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    violation_index_input INT,
    violation_kind_input TEXT,
    severity_input TEXT,
    stream_id_input INT DEFAULT NULL
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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
    );
END;
$$;

DROP FUNCTION media_job_plan_reason_append_v1(UUID, INT, INT, BOOLEAN, TEXT, TEXT);
CREATE FUNCTION media_job_plan_reason_append_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    reason_index_input INT,
    candidate_index_input INT,
    selected_input BOOLEAN,
    reason_code_input TEXT,
    reason_text_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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
    );
END;
$$;

DROP FUNCTION media_job_verification_check_append_v1(UUID, INT, TEXT, TEXT, TEXT, TEXT, TEXT);
CREATE FUNCTION media_job_verification_check_append_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    check_index_input INT,
    check_kind_input TEXT,
    check_status_input TEXT,
    expected_value_input TEXT DEFAULT NULL,
    actual_value_input TEXT DEFAULT NULL,
    details_text_input TEXT DEFAULT NULL
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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
    );
END;
$$;

DROP FUNCTION media_job_artifact_append_v1(UUID, INT, TEXT, TEXT, BIGINT, TEXT);
CREATE FUNCTION media_job_artifact_append_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    artifact_index_input INT,
    artifact_kind_input TEXT,
    artifact_path_input TEXT,
    size_bytes_input BIGINT DEFAULT NULL,
    content_type_input TEXT DEFAULT NULL
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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
    );
END;
$$;

DROP FUNCTION media_job_compact_audit_append_v1(UUID, INT, TEXT, TEXT);
CREATE FUNCTION media_job_compact_audit_append_v1(
    media_job_public_id_input UUID,
    claim_generation_input BIGINT,
    audit_index_input INT,
    fact_kind_input TEXT,
    fact_text_input TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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
    );
END;
$$;

CREATE OR REPLACE FUNCTION media_job_attempt_list_v1(media_job_public_id_input UUID)
RETURNS TABLE (
    attempt_number INT,
    claim_generation BIGINT,
    status media_job_status,
    is_current BOOLEAN,
    queued_at TIMESTAMPTZ,
    claimed_at TIMESTAMPTZ,
    heartbeat_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    last_error TEXT
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_job_operation_list_v1(UUID);
CREATE FUNCTION media_job_operation_list_v1(media_job_public_id_input UUID)
RETURNS TABLE (
    attempt_number INT, is_current BOOLEAN, operation_index INT,
    operation_kind TEXT, stream_id INT, command_bin TEXT, arg_1 TEXT,
    arg_2 TEXT, arg_3 TEXT, arg_4 TEXT, arg_5 TEXT, created_at TIMESTAMPTZ
)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public, pg_temp
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

DROP FUNCTION media_job_violation_list_v1(UUID);
CREATE FUNCTION media_job_violation_list_v1(media_job_public_id_input UUID)
RETURNS TABLE (
    attempt_number INT, is_current BOOLEAN, violation_index INT,
    violation_kind TEXT, severity TEXT, stream_id INT, created_at TIMESTAMPTZ
)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public, pg_temp
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

DROP FUNCTION media_job_plan_reason_list_v1(UUID);
CREATE FUNCTION media_job_plan_reason_list_v1(media_job_public_id_input UUID)
RETURNS TABLE (
    attempt_number INT, is_current BOOLEAN, reason_index INT, candidate_index INT,
    selected BOOLEAN, reason_code TEXT, reason_text TEXT, created_at TIMESTAMPTZ
)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public, pg_temp
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

DROP FUNCTION media_job_verification_check_list_v1(UUID);
CREATE FUNCTION media_job_verification_check_list_v1(media_job_public_id_input UUID)
RETURNS TABLE (
    attempt_number INT, is_current BOOLEAN, check_index INT, check_kind TEXT,
    check_status TEXT, expected_value TEXT, actual_value TEXT, details_text TEXT,
    created_at TIMESTAMPTZ
)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public, pg_temp
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

DROP FUNCTION media_job_artifact_list_v1(UUID);
CREATE FUNCTION media_job_artifact_list_v1(media_job_public_id_input UUID)
RETURNS TABLE (
    attempt_number INT, is_current BOOLEAN, artifact_index INT, artifact_kind TEXT,
    artifact_path TEXT, size_bytes BIGINT, content_type TEXT, created_at TIMESTAMPTZ
)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public, pg_temp
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

DROP FUNCTION media_job_compact_audit_list_v1(UUID);
CREATE FUNCTION media_job_compact_audit_list_v1(media_job_public_id_input UUID)
RETURNS TABLE (
    attempt_number INT, is_current BOOLEAN, audit_index INT,
    fact_kind TEXT, fact_text TEXT, created_at TIMESTAMPTZ
)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = public, pg_temp
AS $$
    SELECT attempt.attempt_number, evidence.media_job_attempt_id = job.current_attempt_id,
           evidence.audit_index, evidence.fact_kind, evidence.fact_text, evidence.created_at
      FROM media_job job
      JOIN media_job_attempt attempt ON attempt.media_job_id = job.media_job_id
      JOIN media_job_compact_audit evidence ON evidence.media_job_attempt_id = attempt.media_job_attempt_id
     WHERE job.media_job_public_id = media_job_public_id_input
     ORDER BY attempt.attempt_number DESC, evidence.audit_index;
$$;



-- Evidence procedures follow the attempt and worker lifecycle definitions.

CREATE INDEX ix_media_job_history_all
    ON media_job (queued_at DESC, media_job_id DESC);
CREATE INDEX ix_media_job_history_status
    ON media_job (status, queued_at DESC, media_job_id DESC);
CREATE INDEX ix_media_job_history_profile
    ON media_job (media_profile_id, queued_at DESC, media_job_id DESC);
CREATE INDEX ix_media_job_history_profile_status
    ON media_job (media_profile_id, status, queued_at DESC, media_job_id DESC);

DROP FUNCTION media_job_list_v1(UUID, media_job_status);
CREATE FUNCTION media_job_list_v1(
    media_profile_public_id_input UUID DEFAULT NULL,
    status_input media_job_status DEFAULT NULL,
    page_size_input INT DEFAULT 50,
    cursor_queued_at_input TIMESTAMPTZ DEFAULT NULL,
    cursor_media_job_public_id_input UUID DEFAULT NULL
)
RETURNS TABLE (
    media_job_public_id UUID,
    source_path TEXT,
    output_path TEXT,
    status media_job_status,
    dry_run BOOLEAN,
    queued_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    last_error TEXT
)
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
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

ALTER TABLE media_job
    ADD COLUMN diagnostics_pruned_at TIMESTAMPTZ,
    ADD CONSTRAINT media_job_diagnostics_pruned_terminal CHECK (
        diagnostics_pruned_at IS NULL
        OR status IN (media_job_status_failed_v1(), media_job_status_cancelled_v1())
    );

CREATE INDEX ix_media_job_diagnostic_prune
    ON media_job (completed_at, media_job_id)
    WHERE diagnostics_pruned_at IS NULL AND status IN ('failed', 'cancelled');

CREATE OR REPLACE FUNCTION media_job_retention_batch_limit_v1()
RETURNS INT
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT 100
$$;

DROP FUNCTION media_job_retention_run_v1(TIMESTAMPTZ);
CREATE FUNCTION media_job_retention_run_v1(as_of_input TIMESTAMPTZ)
RETURNS TABLE (
    completed_jobs_deleted INT,
    failed_jobs_pruned INT,
    failed_detail_rows_deleted INT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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
           SET last_error = NULL, diagnostics_pruned_at = as_of_input
         WHERE media_job_id = ANY(failed_ids);
        GET DIAGNOSTICS failed_count = ROW_COUNT;
    END IF;

    RETURN QUERY SELECT completed_count, failed_count, detail_count;
END;
$$;

CREATE OR REPLACE FUNCTION media_policy_profile_id_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT
)
RETURNS BIGINT
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
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

DROP FUNCTION media_policy_profile_upsert_v1(
    UUID, TEXT, INT, TEXT, TEXT, TEXT, BIGINT, BOOLEAN, BOOLEAN, BOOLEAN, BOOLEAN
);
CREATE FUNCTION media_policy_profile_upsert_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    display_name_input TEXT,
    video_intent_input TEXT,
    verification_strictness_input TEXT,
    verification_duration_tolerance_millis_input BIGINT,
    verification_mux_validation_input BOOLEAN,
    verification_decode_all_streams_input BOOLEAN,
    verification_keyframe_seek_input BOOLEAN,
    verification_playback_probe_input BOOLEAN
)
RETURNS TABLE (
    policy_key TEXT,
    version INT,
    display_name TEXT,
    video_intent TEXT,
    verification_strictness TEXT,
    verification_duration_tolerance_millis BIGINT,
    verification_mux_validation BOOLEAN,
    verification_decode_all_streams BOOLEAN,
    verification_keyframe_seek BOOLEAN,
    verification_playback_probe BOOLEAN
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_policy_retention_rule_append_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    stream_kind_input TEXT,
    semantic_role_input TEXT,
    language_code_input TEXT,
    codec_or_format_input TEXT,
    action_input TEXT,
    placement_input TEXT,
    sort_order_input INT,
    enabled_input BOOLEAN
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_policy_compatibility_target_append_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    compatibility_target_key_input TEXT,
    compatibility_target_version_input INT,
    sort_order_input INT,
    enabled_input BOOLEAN
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_policy_operation_cost_append_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    operation_kind_input TEXT,
    cost_weight_input INT,
    sort_order_input INT,
    enabled_input BOOLEAN
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_stream_classification_rule_append_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    stream_kind_input TEXT,
    semantic_role_input TEXT,
    match_kind_input TEXT,
    match_pattern_input TEXT,
    confidence_input SMALLINT,
    sort_order_input INT,
    enabled_input BOOLEAN
)
RETURNS BIGINT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_policy_maintenance_window_append_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    day_of_week_input SMALLINT,
    start_time_input TIME,
    end_time_input TIME,
    sort_order_input INT,
    enabled_input BOOLEAN
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_policy_runtime_limit_set_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    max_concurrency_input INT,
    max_retries_input INT,
    max_runtime_seconds_input INT,
    max_io_megabytes_per_second_input INT,
    min_free_space_bytes_input BIGINT,
    pause_on_battery_input BOOLEAN
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_policy_runtime_limit_set_v2(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    max_concurrency_input INT,
    max_retries_input INT,
    max_runtime_seconds_input INT,
    max_io_megabytes_per_second_input INT,
    min_free_space_bytes_input BIGINT,
    pause_on_battery_input BOOLEAN,
    minimum_battery_percent_input INT,
    thermal_pressure_limit_input TEXT,
    pause_when_thermal_exceeded_input BOOLEAN
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_policy_behavior_set_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    unmatched_video_action_input TEXT,
    unmatched_audio_action_input TEXT,
    unmatched_subtitle_action_input TEXT,
    unmatched_attachment_action_input TEXT,
    unmatched_data_action_input TEXT,
    unsupported_format_action_input TEXT,
    require_all_targets_input BOOLEAN,
    dry_run_input BOOLEAN,
    replacement_mode_input TEXT,
    quarantine_enabled_input BOOLEAN,
    preserve_permissions_input BOOLEAN,
    preserve_ownership_input BOOLEAN
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_policy_workspace_set_v1(
    actor_public_id_input UUID,
    policy_key_input TEXT,
    version_input INT,
    retention_hours_input INT,
    diagnostics_enabled_input BOOLEAN,
    stale_cleanup_hours_input INT,
    max_workspace_bytes_input BIGINT,
    backup_enabled_input BOOLEAN,
    backup_retention_days_input INT,
    backup_min_free_space_bytes_input BIGINT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_job_phase_list_v1(media_job_public_id_input UUID)
RETURNS TABLE (
    attempt_number INT,
    is_current BOOLEAN,
    phase_index INT,
    phase_name TEXT,
    phase_status media_job_status,
    details_text TEXT,
    created_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_profile_root_list_v1(media_profile_public_id_input UUID)
RETURNS TABLE (
    media_profile_root_public_id UUID,
    root_kind TEXT,
    requested_path TEXT,
    canonical_path TEXT,
    filesystem_device BIGINT,
    filesystem_inode BIGINT,
    media_type TEXT,
    sort_order INT,
    enabled BOOLEAN,
    identity_verified_at TIMESTAMPTZ
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
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

CREATE OR REPLACE FUNCTION media_profile_file_rule_list_v1(media_profile_public_id_input UUID)
RETURNS TABLE (
    rule_kind TEXT,
    matcher_kind TEXT,
    matcher_value TEXT,
    sort_order INT,
    enabled BOOLEAN
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    SELECT rule.rule_kind, rule.matcher_kind, rule.matcher_value,
           rule.sort_order, rule.enabled
      FROM media_profile profile
      JOIN media_profile_file_rule rule ON rule.media_profile_id = profile.media_profile_id
     WHERE profile.media_profile_public_id = media_profile_public_id_input
       AND profile.deleted_at IS NULL
     ORDER BY rule.sort_order;
$$;

CREATE OR REPLACE FUNCTION media_profile_filter_get_v1(media_profile_public_id_input UUID)
RETURNS TABLE (
    min_size_bytes BIGINT,
    max_size_bytes BIGINT,
    min_duration_millis BIGINT,
    max_duration_millis BIGINT,
    include_samples BOOLEAN,
    include_trailers BOOLEAN,
    exclude_trash BOOLEAN,
    exclude_quarantine BOOLEAN
)
LANGUAGE sql
STABLE
SECURITY DEFINER
SET search_path = public, pg_temp
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
