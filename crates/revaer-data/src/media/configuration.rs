//! Stored-procedure access for media configuration catalogs.

use crate::error::{Result, try_op};
use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

/// Typed boolean used by persisted media verification contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct MediaVerificationToggle(bool);

impl MediaVerificationToggle {
    /// Return the stored enabled state.
    #[must_use]
    pub const fn enabled(self) -> bool {
        self.0
    }
}

impl From<bool> for MediaVerificationToggle {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

const MEDIA_COMPATIBILITY_TARGET_LIST_V1: &str = "SELECT compatibility_target_key, version, display_name, video_codec, audio_codec, audio_channels, audio_channel_layout, subtitle_policy FROM media_compatibility_target_list_v1()";
const MEDIA_COMPATIBILITY_TARGET_UPSERT_V1: &str = "SELECT compatibility_target_key, version, display_name, video_codec, audio_codec, audio_channels, audio_channel_layout, subtitle_policy FROM media_compatibility_target_upsert_v1($1, $2, $3, $4, $5, $6, $7, $8, $9)";
const MEDIA_POLICY_PROFILE_LIST_V2: &str = "SELECT policy_key, version, display_name, video_intent, unmatched_video_action, unmatched_audio_action, unmatched_subtitle_action, unmatched_attachment_action, unmatched_data_action, verification_strictness, verification_duration_tolerance_millis, verification_mux_validation, verification_decode_all_streams, verification_keyframe_seek, verification_playback_probe FROM media_policy_profile_list_v2()";
const MEDIA_POLICY_PROFILE_UPSERT_V2: &str = "SELECT policy_key, version, display_name, video_intent, unmatched_video_action, unmatched_audio_action, unmatched_subtitle_action, unmatched_attachment_action, unmatched_data_action, verification_strictness, verification_duration_tolerance_millis, verification_mux_validation, verification_decode_all_streams, verification_keyframe_seek, verification_playback_probe FROM media_policy_profile_upsert_v2(actor_public_id_input => $1, policy_key_input => $2, version_input => $3, display_name_input => $4, video_intent_input => $5, unmatched_video_action_input => $6, unmatched_audio_action_input => $7, unmatched_subtitle_action_input => $8, unmatched_attachment_action_input => $9, unmatched_data_action_input => $10, verification_strictness_input => $11, verification_duration_tolerance_millis_input => $12, verification_mux_validation_input => $13, verification_decode_all_streams_input => $14, verification_keyframe_seek_input => $15, verification_playback_probe_input => $16)";
const MEDIA_JOB_RETENTION_POLICY_GET_V2: &str = "SELECT completed_enabled, completed_mode, completed_limit, failed_diagnostic_enabled, failed_diagnostic_mode, failed_diagnostic_limit FROM media_job_retention_policy_get_v2()";
const MEDIA_JOB_RETENTION_POLICY_UPDATE_V2: &str = "SELECT completed_enabled, completed_mode, completed_limit, failed_diagnostic_enabled, failed_diagnostic_mode, failed_diagnostic_limit FROM media_job_retention_policy_update_v2(actor_public_id_input => $1, completed_enabled_input => $2, completed_mode_input => $3, completed_limit_input => $4, failed_diagnostic_enabled_input => $5, failed_diagnostic_mode_input => $6, failed_diagnostic_limit_input => $7)";
const MEDIA_DESIRED_TARGET_CREATE_V4: &str = "SELECT media_desired_target_create_v4(actor_public_id_input => $1, target_key_input => $2, version_input => $3, display_name_input => $4, container_format_input => $5, container_metadata_policy_input => $6, container_chapter_policy_input => $7, container_attachment_policy_input => $8)";
const MEDIA_DESIRED_TARGET_CHAPTER_APPEND_V1: &str = "SELECT media_desired_target_chapter_append_v1(media_desired_target_profile_public_id_input => $1, start_millis_input => $2, end_millis_input => $3)";
const MEDIA_DESIRED_TARGET_CHAPTER_METADATA_APPEND_V1: &str = "SELECT media_desired_target_chapter_metadata_append_v1(media_desired_target_profile_public_id_input => $1, start_millis_input => $2, metadata_key_input => $3, metadata_value_input => $4)";
const MEDIA_DESIRED_TARGET_CHAPTER_LIST_V1: &str = "SELECT start_millis, end_millis, metadata_key, metadata_value FROM media_desired_target_chapter_list_v1(media_desired_target_profile_public_id_input => $1)";
const MEDIA_DESIRED_TARGET_METADATA_APPEND_V1: &str = "SELECT media_desired_target_metadata_append_v1(media_desired_target_profile_public_id_input => $1, metadata_key_input => $2, metadata_value_input => $3)";
const MEDIA_DESIRED_TARGET_METADATA_LIST_V1: &str = "SELECT metadata_key, metadata_value FROM media_desired_target_metadata_list_v1(media_desired_target_profile_public_id_input => $1)";
const MEDIA_DESIRED_TARGET_LIST_V4: &str = "SELECT media_desired_target_profile_public_id, target_key, version, display_name, container_format, container_metadata_policy, container_chapter_policy, container_attachment_policy FROM media_desired_target_list_v4()";
const MEDIA_DESIRED_TARGET_GRAPH_PAGE_V2: &str = "SELECT media_desired_target_profile_public_id, target_key, version, display_name, container_format, container_metadata_policy, container_chapter_policy, container_attachment_policy, stream_key, stream_kind, semantic_role, language_code, optional, sort_order, codec, channel_count, channel_layout, audio_bitrate_bps, audio_sample_rate_hz, audio_loudness_profile, audio_dynamic_range, video_profile, video_level, video_bitrate_bps, color_primaries, color_transfer, color_space, hdr_format, hdr10_mastering_red_x, hdr10_mastering_red_y, hdr10_mastering_green_x, hdr10_mastering_green_y, hdr10_mastering_blue_x, hdr10_mastering_blue_y, hdr10_mastering_white_point_x, hdr10_mastering_white_point_y, hdr10_mastering_min_luminance, hdr10_mastering_max_luminance, hdr10_max_content_light_level, hdr10_max_frame_average_light_level, title, default_disposition, forced_disposition, subtitle_placement, image_subtitle_action FROM media_desired_target_graph_page_v2($1)";
const MEDIA_DESIRED_TARGET_STREAM_APPEND_V7: &str = "SELECT media_desired_target_stream_append_v7(media_desired_target_profile_public_id_input => $1, stream_key_input => $2, stream_kind_input => $3, semantic_role_input => $4, language_code_input => $5, optional_input => $6, sort_order_input => $7, codec_input => $8, channel_count_input => $9, channel_layout_input => $10, audio_bitrate_bps_input => $11, audio_sample_rate_hz_input => $12, audio_loudness_profile_input => $13, audio_dynamic_range_input => $14, video_profile_input => $15, video_level_input => $16, video_bitrate_bps_input => $17, color_primaries_input => $18, color_transfer_input => $19, color_space_input => $20, hdr_format_input => $21, hdr10_mastering_red_x_input => $22, hdr10_mastering_red_y_input => $23, hdr10_mastering_green_x_input => $24, hdr10_mastering_green_y_input => $25, hdr10_mastering_blue_x_input => $26, hdr10_mastering_blue_y_input => $27, hdr10_mastering_white_point_x_input => $28, hdr10_mastering_white_point_y_input => $29, hdr10_mastering_min_luminance_input => $30, hdr10_mastering_max_luminance_input => $31, hdr10_max_content_light_level_input => $32, hdr10_max_frame_average_light_level_input => $33, title_input => $34, default_disposition_input => $35, forced_disposition_input => $36, subtitle_placement_input => $37, image_subtitle_action_input => $38)";
const MEDIA_DESIRED_TARGET_STREAM_LIST_V7: &str = "SELECT stream_key, stream_kind, semantic_role, language_code, optional, sort_order, codec, channel_count, channel_layout, audio_bitrate_bps, audio_sample_rate_hz, audio_loudness_profile, audio_dynamic_range, video_profile, video_level, video_bitrate_bps, color_primaries, color_transfer, color_space, hdr_format, hdr10_mastering_red_x, hdr10_mastering_red_y, hdr10_mastering_green_x, hdr10_mastering_green_y, hdr10_mastering_blue_x, hdr10_mastering_blue_y, hdr10_mastering_white_point_x, hdr10_mastering_white_point_y, hdr10_mastering_min_luminance, hdr10_mastering_max_luminance, hdr10_max_content_light_level, hdr10_max_frame_average_light_level, title, default_disposition, forced_disposition, subtitle_placement, image_subtitle_action FROM media_desired_target_stream_list_v7(media_desired_target_profile_public_id_input => $1)";
const MEDIA_PROFILE_DESIRED_TARGET_SET_V1: &str = "SELECT media_profile_desired_target_set_v1(actor_public_id_input => $1, media_profile_public_id_input => $2, desired_target_key_input => $3, desired_target_version_input => $4)";

/// Compatibility target upsert payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpsertMediaCompatibilityTargetInput<'a> {
    /// Actor performing the write.
    pub actor_public_id: Uuid,
    /// Stable target key.
    pub compatibility_target_key: &'a str,
    /// Version to create or replace.
    pub version: i32,
    /// Operator display label.
    pub display_name: &'a str,
    /// Desired video codec.
    pub video_codec: &'a str,
    /// Desired audio codec.
    pub audio_codec: &'a str,
    /// Optional desired audio channel count.
    pub audio_channels: Option<i32>,
    /// Optional desired audio channel layout.
    pub audio_channel_layout: Option<&'a str>,
    /// Subtitle retention policy.
    pub subtitle_policy: &'a str,
}

/// Policy profile upsert payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpsertMediaPolicyProfileInput<'a> {
    /// Actor performing the write.
    pub actor_public_id: Uuid,
    /// Stable policy key.
    pub policy_key: &'a str,
    /// Version to create or replace.
    pub version: i32,
    /// Operator display label.
    pub display_name: &'a str,
    /// Worker video intent.
    pub video_intent: &'a str,
    /// Action for unmatched source video streams.
    pub unmatched_video_action: &'a str,
    /// Action for unmatched source audio streams.
    pub unmatched_audio_action: &'a str,
    /// Action for unmatched source subtitle streams.
    pub unmatched_subtitle_action: &'a str,
    /// Action for unmatched source attachment streams.
    pub unmatched_attachment_action: &'a str,
    /// Action for unmatched source opaque data streams.
    pub unmatched_data_action: &'a str,
    /// Verification strictness (`strict`, `balanced`, or `fast`).
    pub verification_strictness: &'a str,
    /// Maximum source/candidate duration delta in milliseconds.
    pub verification_duration_tolerance_millis: i64,
    /// Whether normalized mux-structure validation is selected.
    pub verification_mux_validation: MediaVerificationToggle,
    /// Whether every candidate stream must decode without errors.
    pub verification_decode_all_streams: MediaVerificationToggle,
    /// Whether midpoint video keyframe seeking must succeed.
    pub verification_keyframe_seek: MediaVerificationToggle,
    /// Whether the noninteractive playback smoke probe is selected.
    pub verification_playback_probe: MediaVerificationToggle,
}

/// Job retention update payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMediaJobRetentionPolicyInput {
    /// Actor performing the write.
    pub actor_public_id: Uuid,
    /// Whether completed-job deletion is enabled.
    pub completed_enabled: bool,
    /// Completed-job retention mode (`age` or `count`).
    pub completed_mode: String,
    /// Completed-job age in days or retained-job count.
    pub completed_limit: i32,
    /// Whether failed-terminal diagnostic pruning is enabled.
    pub failed_diagnostic_enabled: bool,
    /// Failed-terminal diagnostic retention mode (`age` or `count`).
    pub failed_diagnostic_mode: String,
    /// Failed-terminal diagnostic age in days or retained-job count.
    pub failed_diagnostic_limit: i32,
}

/// Immutable desired-target version creation payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateMediaDesiredTargetInput<'a> {
    /// Actor performing the write.
    pub actor_public_id: Uuid,
    /// Stable desired-target key.
    pub target_key: &'a str,
    /// Positive immutable version.
    pub version: i32,
    /// Operator display label.
    pub display_name: &'a str,
    /// Desired output container format.
    pub container_format: &'a str,
    /// Desired container metadata policy.
    pub container_metadata_policy: &'a str,
    /// Desired container chapter policy.
    pub container_chapter_policy: &'a str,
    /// Desired container attachment policy.
    pub container_attachment_policy: &'a str,
}

/// Desired target container metadata append payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendMediaDesiredTargetMetadataInput<'a> {
    /// Desired-target version public id.
    pub media_desired_target_profile_public_id: Uuid,
    /// Lowercase metadata key.
    pub metadata_key: &'a str,
    /// Trimmed metadata value.
    pub metadata_value: &'a str,
}

/// Desired target container chapter append payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendMediaDesiredTargetChapterInput {
    /// Desired-target version public id.
    pub media_desired_target_profile_public_id: Uuid,
    /// Inclusive chapter start in milliseconds.
    pub start_millis: i64,
    /// Exclusive chapter end in milliseconds.
    pub end_millis: i64,
}

/// Desired target container chapter metadata append payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendMediaDesiredTargetChapterMetadataInput<'a> {
    /// Desired-target version public id.
    pub media_desired_target_profile_public_id: Uuid,
    /// Inclusive chapter start in milliseconds.
    pub start_millis: i64,
    /// Lowercase metadata key.
    pub metadata_key: &'a str,
    /// Trimmed metadata value.
    pub metadata_value: &'a str,
}

/// Ordered desired-target stream creation payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppendMediaDesiredTargetStreamInput<'a> {
    /// Desired-target version public id.
    pub media_desired_target_profile_public_id: Uuid,
    /// Stable stream key within the target version.
    pub stream_key: &'a str,
    /// Media stream kind.
    pub stream_kind: &'a str,
    /// Optional semantic role selector.
    pub semantic_role: Option<&'a str>,
    /// Optional language selector.
    pub language_code: Option<&'a str>,
    /// Whether a missing source match is acceptable.
    pub optional: bool,
    /// Final mux ordering position.
    pub sort_order: i32,
    /// Desired codec.
    pub codec: &'a str,
    /// Optional desired audio channel count.
    pub channel_count: Option<i32>,
    /// Optional desired audio channel layout.
    pub channel_layout: Option<&'a str>,
    /// Optional desired average audio bitrate in bits per second.
    pub audio_bitrate_bps: Option<i32>,
    /// Optional desired audio sample rate in hertz.
    pub audio_sample_rate_hz: Option<i32>,
    /// Optional desired audio loudness processing profile.
    pub audio_loudness_profile: Option<&'a str>,
    /// Optional desired audio dynamic-range behavior.
    pub audio_dynamic_range: Option<&'a str>,
    /// Optional desired video profile.
    pub video_profile: Option<&'a str>,
    /// Optional desired video level.
    pub video_level: Option<&'a str>,
    /// Optional desired average video bitrate in bits per second.
    pub video_bitrate_bps: Option<i32>,
    /// Optional desired video color primaries.
    pub color_primaries: Option<&'a str>,
    /// Optional desired video transfer characteristic.
    pub color_transfer: Option<&'a str>,
    /// Optional desired video color space.
    pub color_space: Option<&'a str>,
    /// Optional desired HDR format label.
    pub hdr_format: Option<&'a str>,
    /// Optional HDR10 mastering red x coordinate.
    pub hdr10_mastering_red_x: Option<&'a str>,
    /// Optional HDR10 mastering red y coordinate.
    pub hdr10_mastering_red_y: Option<&'a str>,
    /// Optional HDR10 mastering green x coordinate.
    pub hdr10_mastering_green_x: Option<&'a str>,
    /// Optional HDR10 mastering green y coordinate.
    pub hdr10_mastering_green_y: Option<&'a str>,
    /// Optional HDR10 mastering blue x coordinate.
    pub hdr10_mastering_blue_x: Option<&'a str>,
    /// Optional HDR10 mastering blue y coordinate.
    pub hdr10_mastering_blue_y: Option<&'a str>,
    /// Optional HDR10 mastering white point x coordinate.
    pub hdr10_mastering_white_point_x: Option<&'a str>,
    /// Optional HDR10 mastering white point y coordinate.
    pub hdr10_mastering_white_point_y: Option<&'a str>,
    /// Optional HDR10 mastering minimum luminance.
    pub hdr10_mastering_min_luminance: Option<&'a str>,
    /// Optional HDR10 mastering maximum luminance.
    pub hdr10_mastering_max_luminance: Option<&'a str>,
    /// Optional HDR10 maximum content light level.
    pub hdr10_max_content_light_level: Option<&'a str>,
    /// Optional HDR10 maximum frame-average light level.
    pub hdr10_max_frame_average_light_level: Option<&'a str>,
    /// Optional desired title.
    pub title: Option<&'a str>,
    /// Desired default disposition.
    pub default_disposition: bool,
    /// Desired forced disposition.
    pub forced_disposition: bool,
    /// Desired subtitle placement for subtitle streams.
    pub subtitle_placement: Option<&'a str>,
    /// Image-subtitle behavior for subtitle streams.
    pub image_subtitle_action: Option<&'a str>,
}

/// Desired-target version row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaDesiredTargetRow {
    /// Desired-target version public id.
    pub media_desired_target_profile_public_id: Uuid,
    /// Stable target key.
    pub target_key: String,
    /// Immutable target version.
    pub version: i32,
    /// Operator display label.
    pub display_name: String,
    /// Desired output container format.
    pub container_format: String,
    /// Desired container metadata policy.
    pub container_metadata_policy: String,
    /// Desired container chapter policy.
    pub container_chapter_policy: String,
    /// Desired container attachment policy.
    pub container_attachment_policy: String,
}

/// Desired target container metadata row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaDesiredTargetMetadataRow {
    /// Lowercase metadata key.
    pub metadata_key: String,
    /// Trimmed metadata value.
    pub metadata_value: String,
}

/// Desired target container chapter row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaDesiredTargetChapterRow {
    /// Inclusive chapter start in milliseconds.
    pub start_millis: i64,
    /// Exclusive chapter end in milliseconds.
    pub end_millis: i64,
    /// Optional lowercase metadata key.
    pub metadata_key: Option<String>,
    /// Optional trimmed metadata value.
    pub metadata_value: Option<String>,
}

/// Ordered desired-target stream row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaDesiredTargetStreamRow {
    /// Stable stream key.
    pub stream_key: String,
    /// Media stream kind.
    pub stream_kind: String,
    /// Optional semantic role selector.
    pub semantic_role: Option<String>,
    /// Optional language selector.
    pub language_code: Option<String>,
    /// Whether a missing source match is acceptable.
    pub optional: bool,
    /// Final mux ordering position.
    pub sort_order: i32,
    /// Desired codec.
    pub codec: String,
    /// Optional desired audio channel count.
    pub channel_count: Option<i32>,
    /// Optional desired audio channel layout.
    pub channel_layout: Option<String>,
    /// Optional desired average audio bitrate in bits per second.
    pub audio_bitrate_bps: Option<i32>,
    /// Optional desired audio sample rate in hertz.
    pub audio_sample_rate_hz: Option<i32>,
    /// Optional desired audio loudness processing profile.
    pub audio_loudness_profile: Option<String>,
    /// Optional desired audio dynamic-range behavior.
    pub audio_dynamic_range: Option<String>,
    /// Optional desired video profile.
    pub video_profile: Option<String>,
    /// Optional desired video level.
    pub video_level: Option<String>,
    /// Optional desired average video bitrate in bits per second.
    pub video_bitrate_bps: Option<i32>,
    /// Optional desired video color primaries.
    pub color_primaries: Option<String>,
    /// Optional desired video transfer characteristic.
    pub color_transfer: Option<String>,
    /// Optional desired video color space.
    pub color_space: Option<String>,
    /// Optional desired HDR format label.
    pub hdr_format: Option<String>,
    /// Optional HDR10 mastering red x coordinate.
    pub hdr10_mastering_red_x: Option<String>,
    /// Optional HDR10 mastering red y coordinate.
    pub hdr10_mastering_red_y: Option<String>,
    /// Optional HDR10 mastering green x coordinate.
    pub hdr10_mastering_green_x: Option<String>,
    /// Optional HDR10 mastering green y coordinate.
    pub hdr10_mastering_green_y: Option<String>,
    /// Optional HDR10 mastering blue x coordinate.
    pub hdr10_mastering_blue_x: Option<String>,
    /// Optional HDR10 mastering blue y coordinate.
    pub hdr10_mastering_blue_y: Option<String>,
    /// Optional HDR10 mastering white point x coordinate.
    pub hdr10_mastering_white_point_x: Option<String>,
    /// Optional HDR10 mastering white point y coordinate.
    pub hdr10_mastering_white_point_y: Option<String>,
    /// Optional HDR10 mastering minimum luminance.
    pub hdr10_mastering_min_luminance: Option<String>,
    /// Optional HDR10 mastering maximum luminance.
    pub hdr10_mastering_max_luminance: Option<String>,
    /// Optional HDR10 maximum content light level.
    pub hdr10_max_content_light_level: Option<String>,
    /// Optional HDR10 maximum frame-average light level.
    pub hdr10_max_frame_average_light_level: Option<String>,
    /// Optional desired title.
    pub title: Option<String>,
    /// Desired default disposition.
    pub default_disposition: bool,
    /// Desired forced disposition.
    pub forced_disposition: bool,
    /// Desired subtitle placement for subtitle streams.
    pub subtitle_placement: Option<String>,
    /// Image-subtitle behavior for subtitle streams.
    pub image_subtitle_action: Option<String>,
}

/// One flattened row from a bounded desired-target graph page.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaDesiredTargetGraphRow {
    /// Repeated target summary.
    #[sqlx(flatten)]
    pub target: MediaDesiredTargetRow,
    /// One ordered child stream.
    #[sqlx(flatten)]
    pub stream: MediaDesiredTargetStreamRow,
}

/// Versioned compatibility target row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaCompatibilityTargetRow {
    /// Stable target key.
    pub compatibility_target_key: String,
    /// Target version.
    pub version: i32,
    /// Display label.
    pub display_name: String,
    /// Desired video codec.
    pub video_codec: String,
    /// Desired audio codec.
    pub audio_codec: String,
    /// Optional desired audio channel count.
    pub audio_channels: Option<i32>,
    /// Optional desired audio channel layout.
    pub audio_channel_layout: Option<String>,
    /// Subtitle retention policy.
    pub subtitle_policy: String,
}

/// Versioned policy profile row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaPolicyProfileRow {
    /// Stable policy key.
    pub policy_key: String,
    /// Policy version.
    pub version: i32,
    /// Display label.
    pub display_name: String,
    /// Video transcode intent.
    pub video_intent: String,
    /// Action for unmatched source video streams.
    pub unmatched_video_action: String,
    /// Action for unmatched source audio streams.
    pub unmatched_audio_action: String,
    /// Action for unmatched source subtitle streams.
    pub unmatched_subtitle_action: String,
    /// Action for unmatched source attachment streams.
    pub unmatched_attachment_action: String,
    /// Action for unmatched source opaque data streams.
    pub unmatched_data_action: String,
    /// Verification strictness.
    pub verification_strictness: String,
    /// Maximum source/candidate duration delta in milliseconds.
    pub verification_duration_tolerance_millis: i64,
    /// Whether normalized mux-structure validation is selected.
    pub verification_mux_validation: MediaVerificationToggle,
    /// Whether every candidate stream must decode without errors.
    pub verification_decode_all_streams: MediaVerificationToggle,
    /// Whether midpoint video keyframe seeking must succeed.
    pub verification_keyframe_seek: MediaVerificationToggle,
    /// Whether the noninteractive playback smoke probe is selected.
    pub verification_playback_probe: MediaVerificationToggle,
}

/// Job retention policy row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct MediaJobRetentionPolicyRow {
    /// Whether completed-job deletion is enabled.
    pub completed_enabled: bool,
    /// Completed-job retention mode.
    pub completed_mode: String,
    /// Completed-job age in days or retained-job count.
    pub completed_limit: i32,
    /// Whether failed-terminal diagnostic pruning is enabled.
    pub failed_diagnostic_enabled: bool,
    /// Failed-terminal diagnostic retention mode.
    pub failed_diagnostic_mode: String,
    /// Failed-terminal diagnostic age in days or retained-job count.
    pub failed_diagnostic_limit: i32,
}

/// List enabled compatibility target versions.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_compatibility_targets(
    pool: &PgPool,
) -> Result<Vec<MediaCompatibilityTargetRow>> {
    sqlx::query_as::<_, MediaCompatibilityTargetRow>(MEDIA_COMPATIBILITY_TARGET_LIST_V1)
        .fetch_all(pool)
        .await
        .map_err(try_op("media compatibility target list"))
}

/// Create or replace a compatibility target version.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn upsert_media_compatibility_target(
    pool: &PgPool,
    input: UpsertMediaCompatibilityTargetInput<'_>,
) -> Result<MediaCompatibilityTargetRow> {
    upsert_media_compatibility_target_with_executor(pool, input).await
}

/// Create or replace a compatibility target version in a caller-owned transaction.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn upsert_media_compatibility_target_with_executor<'e, E>(
    executor: E,
    input: UpsertMediaCompatibilityTargetInput<'_>,
) -> Result<MediaCompatibilityTargetRow>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as::<_, MediaCompatibilityTargetRow>(MEDIA_COMPATIBILITY_TARGET_UPSERT_V1)
        .bind(input.actor_public_id)
        .bind(input.compatibility_target_key)
        .bind(input.version)
        .bind(input.display_name)
        .bind(input.video_codec)
        .bind(input.audio_codec)
        .bind(input.audio_channels)
        .bind(input.audio_channel_layout)
        .bind(input.subtitle_policy)
        .fetch_one(executor)
        .await
        .map_err(try_op("media compatibility target upsert"))
}

/// List enabled policy profile versions.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_policy_profiles(pool: &PgPool) -> Result<Vec<MediaPolicyProfileRow>> {
    sqlx::query_as::<_, MediaPolicyProfileRow>(MEDIA_POLICY_PROFILE_LIST_V2)
        .fetch_all(pool)
        .await
        .map_err(try_op("media policy profile list"))
}

/// Create or replace a policy profile version.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn upsert_media_policy_profile(
    pool: &PgPool,
    input: UpsertMediaPolicyProfileInput<'_>,
) -> Result<MediaPolicyProfileRow> {
    upsert_media_policy_profile_with_executor(pool, input).await
}

/// Create or replace a policy profile version in a caller-owned transaction.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn upsert_media_policy_profile_with_executor<'e, E>(
    executor: E,
    input: UpsertMediaPolicyProfileInput<'_>,
) -> Result<MediaPolicyProfileRow>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as::<_, MediaPolicyProfileRow>(MEDIA_POLICY_PROFILE_UPSERT_V2)
        .bind(input.actor_public_id)
        .bind(input.policy_key)
        .bind(input.version)
        .bind(input.display_name)
        .bind(input.video_intent)
        .bind(input.unmatched_video_action)
        .bind(input.unmatched_audio_action)
        .bind(input.unmatched_subtitle_action)
        .bind(input.unmatched_attachment_action)
        .bind(input.unmatched_data_action)
        .bind(input.verification_strictness)
        .bind(input.verification_duration_tolerance_millis)
        .bind(input.verification_mux_validation.enabled())
        .bind(input.verification_decode_all_streams.enabled())
        .bind(input.verification_keyframe_seek.enabled())
        .bind(input.verification_playback_probe.enabled())
        .fetch_one(executor)
        .await
        .map_err(try_op("media policy profile upsert"))
}

/// Read the active job retention policy.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn get_media_job_retention_policy(
    pool: &PgPool,
) -> Result<Option<MediaJobRetentionPolicyRow>> {
    sqlx::query_as::<_, MediaJobRetentionPolicyRow>(MEDIA_JOB_RETENTION_POLICY_GET_V2)
        .fetch_optional(pool)
        .await
        .map_err(try_op("media job retention policy get"))
}

/// Update the active job retention policy.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn update_media_job_retention_policy(
    pool: &PgPool,
    input: UpdateMediaJobRetentionPolicyInput,
) -> Result<MediaJobRetentionPolicyRow> {
    sqlx::query_as::<_, MediaJobRetentionPolicyRow>(MEDIA_JOB_RETENTION_POLICY_UPDATE_V2)
        .bind(input.actor_public_id)
        .bind(input.completed_enabled)
        .bind(input.completed_mode)
        .bind(input.completed_limit)
        .bind(input.failed_diagnostic_enabled)
        .bind(input.failed_diagnostic_mode)
        .bind(input.failed_diagnostic_limit)
        .fetch_one(pool)
        .await
        .map_err(try_op("media job retention policy update"))
}

/// Create one immutable desired-target version.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn create_media_desired_target(
    pool: &PgPool,
    input: CreateMediaDesiredTargetInput<'_>,
) -> Result<Uuid> {
    create_media_desired_target_with_executor(pool, input).await
}

/// Create one immutable desired-target version with a caller-provided executor.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn create_media_desired_target_with_executor<'e, E>(
    executor: E,
    input: CreateMediaDesiredTargetInput<'_>,
) -> Result<Uuid>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, Uuid>(MEDIA_DESIRED_TARGET_CREATE_V4)
        .bind(input.actor_public_id)
        .bind(input.target_key)
        .bind(input.version)
        .bind(input.display_name)
        .bind(input.container_format)
        .bind(input.container_metadata_policy)
        .bind(input.container_chapter_policy)
        .bind(input.container_attachment_policy)
        .fetch_one(executor)
        .await
        .map_err(try_op("media desired target create"))
}

/// Append one desired container metadata row before a target version is assigned.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn append_media_desired_target_metadata(
    pool: &PgPool,
    input: AppendMediaDesiredTargetMetadataInput<'_>,
) -> Result<()> {
    append_media_desired_target_metadata_with_executor(pool, input).await
}

/// Append one desired container metadata row with a caller-provided executor.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn append_media_desired_target_metadata_with_executor<'e, E>(
    executor: E,
    input: AppendMediaDesiredTargetMetadataInput<'_>,
) -> Result<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(MEDIA_DESIRED_TARGET_METADATA_APPEND_V1)
        .bind(input.media_desired_target_profile_public_id)
        .bind(input.metadata_key)
        .bind(input.metadata_value)
        .execute(executor)
        .await
        .map_err(try_op("media desired target metadata append"))?;
    Ok(())
}

/// Append one desired chapter row before a target version is assigned.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn append_media_desired_target_chapter(
    pool: &PgPool,
    input: AppendMediaDesiredTargetChapterInput,
) -> Result<()> {
    append_media_desired_target_chapter_with_executor(pool, input).await
}

/// Append one desired chapter row with a caller-provided executor.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn append_media_desired_target_chapter_with_executor<'e, E>(
    executor: E,
    input: AppendMediaDesiredTargetChapterInput,
) -> Result<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(MEDIA_DESIRED_TARGET_CHAPTER_APPEND_V1)
        .bind(input.media_desired_target_profile_public_id)
        .bind(input.start_millis)
        .bind(input.end_millis)
        .execute(executor)
        .await
        .map_err(try_op("media desired target chapter append"))?;
    Ok(())
}

/// Append one desired chapter metadata row before a target version is assigned.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn append_media_desired_target_chapter_metadata(
    pool: &PgPool,
    input: AppendMediaDesiredTargetChapterMetadataInput<'_>,
) -> Result<()> {
    append_media_desired_target_chapter_metadata_with_executor(pool, input).await
}

/// Append one desired chapter metadata row with a caller-provided executor.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn append_media_desired_target_chapter_metadata_with_executor<'e, E>(
    executor: E,
    input: AppendMediaDesiredTargetChapterMetadataInput<'_>,
) -> Result<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(MEDIA_DESIRED_TARGET_CHAPTER_METADATA_APPEND_V1)
        .bind(input.media_desired_target_profile_public_id)
        .bind(input.start_millis)
        .bind(input.metadata_key)
        .bind(input.metadata_value)
        .execute(executor)
        .await
        .map_err(try_op("media desired target chapter metadata append"))?;
    Ok(())
}

/// Append one ordered stream before a desired-target version is assigned.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn append_media_desired_target_stream(
    pool: &PgPool,
    input: AppendMediaDesiredTargetStreamInput<'_>,
) -> Result<()> {
    append_media_desired_target_stream_with_executor(pool, input).await
}

/// Append one ordered stream with a caller-provided executor.
///
/// # Errors
///
/// Returns an error when validation or stored-procedure execution fails.
pub async fn append_media_desired_target_stream_with_executor<'e, E>(
    executor: E,
    input: AppendMediaDesiredTargetStreamInput<'_>,
) -> Result<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(MEDIA_DESIRED_TARGET_STREAM_APPEND_V7)
        .bind(input.media_desired_target_profile_public_id)
        .bind(input.stream_key)
        .bind(input.stream_kind)
        .bind(input.semantic_role)
        .bind(input.language_code)
        .bind(input.optional)
        .bind(input.sort_order)
        .bind(input.codec)
        .bind(input.channel_count)
        .bind(input.channel_layout)
        .bind(input.audio_bitrate_bps)
        .bind(input.audio_sample_rate_hz)
        .bind(input.audio_loudness_profile)
        .bind(input.audio_dynamic_range)
        .bind(input.video_profile)
        .bind(input.video_level)
        .bind(input.video_bitrate_bps)
        .bind(input.color_primaries)
        .bind(input.color_transfer)
        .bind(input.color_space)
        .bind(input.hdr_format)
        .bind(input.hdr10_mastering_red_x)
        .bind(input.hdr10_mastering_red_y)
        .bind(input.hdr10_mastering_green_x)
        .bind(input.hdr10_mastering_green_y)
        .bind(input.hdr10_mastering_blue_x)
        .bind(input.hdr10_mastering_blue_y)
        .bind(input.hdr10_mastering_white_point_x)
        .bind(input.hdr10_mastering_white_point_y)
        .bind(input.hdr10_mastering_min_luminance)
        .bind(input.hdr10_mastering_max_luminance)
        .bind(input.hdr10_max_content_light_level)
        .bind(input.hdr10_max_frame_average_light_level)
        .bind(input.title)
        .bind(input.default_disposition)
        .bind(input.forced_disposition)
        .bind(input.subtitle_placement)
        .bind(input.image_subtitle_action)
        .execute(executor)
        .await
        .map_err(try_op("media desired target stream append"))?;
    Ok(())
}

/// List enabled desired-target versions.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_desired_targets(pool: &PgPool) -> Result<Vec<MediaDesiredTargetRow>> {
    sqlx::query_as::<_, MediaDesiredTargetRow>(MEDIA_DESIRED_TARGET_LIST_V4)
        .fetch_all(pool)
        .await
        .map_err(try_op("media desired target list"))
}

/// Read one bounded desired-target page and all bounded child streams in one query.
///
/// # Errors
///
/// Returns an error when the limit is invalid or stored-procedure execution fails.
pub async fn list_media_desired_target_graph_page(
    pool: &PgPool,
    limit: i32,
) -> Result<Vec<MediaDesiredTargetGraphRow>> {
    sqlx::query_as::<_, MediaDesiredTargetGraphRow>(MEDIA_DESIRED_TARGET_GRAPH_PAGE_V2)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(try_op("media desired target graph page"))
}

/// List desired container metadata rows for one desired-target version.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_desired_target_metadata(
    pool: &PgPool,
    media_desired_target_profile_public_id: Uuid,
) -> Result<Vec<MediaDesiredTargetMetadataRow>> {
    sqlx::query_as::<_, MediaDesiredTargetMetadataRow>(MEDIA_DESIRED_TARGET_METADATA_LIST_V1)
        .bind(media_desired_target_profile_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media desired target metadata list"))
}

/// List desired container chapter rows for one desired-target version.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_desired_target_chapters(
    pool: &PgPool,
    media_desired_target_profile_public_id: Uuid,
) -> Result<Vec<MediaDesiredTargetChapterRow>> {
    sqlx::query_as::<_, MediaDesiredTargetChapterRow>(MEDIA_DESIRED_TARGET_CHAPTER_LIST_V1)
        .bind(media_desired_target_profile_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media desired target chapter list"))
}

/// List the ordered stream graph for one desired-target version.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn list_media_desired_target_streams(
    pool: &PgPool,
    media_desired_target_profile_public_id: Uuid,
) -> Result<Vec<MediaDesiredTargetStreamRow>> {
    sqlx::query_as::<_, MediaDesiredTargetStreamRow>(MEDIA_DESIRED_TARGET_STREAM_LIST_V7)
        .bind(media_desired_target_profile_public_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("media desired target stream list"))
}

/// Pin a profile to one immutable desired-target version, or clear its target.
///
/// # Errors
///
/// Returns an error when the profile or target does not exist or execution fails.
pub async fn set_media_profile_desired_target(
    pool: &PgPool,
    actor_public_id: Uuid,
    media_profile_public_id: Uuid,
    target_key: Option<&str>,
    version: Option<i32>,
) -> Result<Uuid> {
    set_media_profile_desired_target_with_executor(
        pool,
        actor_public_id,
        media_profile_public_id,
        target_key,
        version,
    )
    .await
}

/// Pin a profile to a desired target in a caller-owned transaction.
///
/// # Errors
///
/// Returns an error when the profile or target does not exist or execution fails.
pub async fn set_media_profile_desired_target_with_executor<'e, E>(
    executor: E,
    actor_public_id: Uuid,
    media_profile_public_id: Uuid,
    target_key: Option<&str>,
    version: Option<i32>,
) -> Result<Uuid>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, Uuid>(MEDIA_PROFILE_DESIRED_TARGET_SET_V1)
        .bind(actor_public_id)
        .bind(media_profile_public_id)
        .bind(target_key.unwrap_or_default())
        .bind(version.unwrap_or_default())
        .fetch_one(executor)
        .await
        .map_err(try_op("media profile desired target set"))
}

#[cfg(test)]
mod tests {
    use super::{
        AppendMediaDesiredTargetChapterInput, AppendMediaDesiredTargetChapterMetadataInput,
        AppendMediaDesiredTargetMetadataInput, AppendMediaDesiredTargetStreamInput,
        CreateMediaDesiredTargetInput, MediaDesiredTargetStreamRow, MediaJobRetentionPolicyRow,
        UpdateMediaJobRetentionPolicyInput, UpsertMediaCompatibilityTargetInput,
        UpsertMediaPolicyProfileInput, append_media_desired_target_chapter,
        append_media_desired_target_chapter_metadata,
        append_media_desired_target_chapter_with_executor, append_media_desired_target_metadata,
        append_media_desired_target_stream, create_media_desired_target,
        get_media_job_retention_policy, list_media_compatibility_targets,
        list_media_desired_target_chapters, list_media_desired_target_graph_page,
        list_media_desired_target_streams, list_media_desired_targets, list_media_policy_profiles,
        set_media_profile_desired_target, update_media_job_retention_policy,
        upsert_media_compatibility_target, upsert_media_policy_profile,
    };
    use crate::config::factory_reset;
    use crate::media::jobs::{
        EnqueueDiscoveredMediaJobInput, enqueue_discovered_media_job,
        list_media_job_desired_target_streams, media_job_worker_claim_next,
    };
    use crate::media::profiles::{UpsertMediaProfileInput, upsert_media_profile};
    use crate::media::schema_tests::{MediaTestDb, setup_media_db};
    use uuid::Uuid;

    fn assert_default_retention(retention: &MediaJobRetentionPolicyRow) {
        assert!(!retention.completed_enabled);
        assert_eq!(retention.completed_mode, "age");
        assert_eq!(retention.completed_limit, 30);
        assert!(retention.failed_diagnostic_enabled);
        assert_eq!(retention.failed_diagnostic_mode, "age");
        assert_eq!(retention.failed_diagnostic_limit, 30);
    }

    #[test]
    fn migration_guards_constant_query_bounded_target_graph() {
        let migration = include_str!("../../migrations/0189_media_desired_target_graph_hdr10.sql");
        assert!(migration.contains("media_desired_target_graph_page_v2(limit_input INT)"));
        assert!(migration.contains("media_desired_target_list_v4()"));
        assert!(migration.contains("media_desired_target_stream_list_v7("));
        assert!(migration.contains("limit_input > 128"));
        assert!(migration.contains("LIMIT 1025"));
    }

    #[tokio::test]
    async fn desired_target_graph_accepts_maximum_page_and_rejects_max_plus_one()
    -> anyhow::Result<()> {
        let Some(db) = setup_media_db("desired_target_graph_page").await? else {
            return Ok(());
        };
        for index in 0..128 {
            let target_id = create_media_desired_target(
                db.pool(),
                CreateMediaDesiredTargetInput {
                    actor_public_id: db.system_user_public_id,
                    target_key: &format!("bounded-target-{index:03}"),
                    version: 1,
                    display_name: &format!("Bounded target {index}"),
                    container_format: "matroska",
                    container_metadata_policy: "preserve",
                    container_chapter_policy: "preserve",
                    container_attachment_policy: "preserve",
                },
            )
            .await?;
            append_media_desired_target_stream(
                db.pool(),
                AppendMediaDesiredTargetStreamInput {
                    media_desired_target_profile_public_id: target_id,
                    stream_key: "video-main",
                    stream_kind: "video",
                    semantic_role: Some("main"),
                    language_code: None,
                    optional: false,
                    sort_order: 0,
                    codec: "hevc",
                    channel_count: None,
                    channel_layout: None,
                    audio_bitrate_bps: None,
                    audio_sample_rate_hz: None,
                    audio_loudness_profile: None,
                    audio_dynamic_range: None,
                    video_profile: None,
                    video_level: None,
                    video_bitrate_bps: None,
                    color_primaries: None,
                    color_transfer: None,
                    color_space: None,
                    hdr_format: None,
                    hdr10_mastering_red_x: None,
                    hdr10_mastering_red_y: None,
                    hdr10_mastering_green_x: None,
                    hdr10_mastering_green_y: None,
                    hdr10_mastering_blue_x: None,
                    hdr10_mastering_blue_y: None,
                    hdr10_mastering_white_point_x: None,
                    hdr10_mastering_white_point_y: None,
                    hdr10_mastering_min_luminance: None,
                    hdr10_mastering_max_luminance: None,
                    hdr10_max_content_light_level: None,
                    hdr10_max_frame_average_light_level: None,
                    title: None,
                    default_disposition: true,
                    forced_disposition: false,
                    subtitle_placement: None,
                    image_subtitle_action: None,
                },
            )
            .await?;
        }
        assert_eq!(
            list_media_desired_target_graph_page(db.pool(), 128)
                .await?
                .len(),
            128
        );
        assert!(
            list_media_desired_target_graph_page(db.pool(), 129)
                .await
                .is_err()
        );
        Ok(())
    }

    #[tokio::test]
    async fn desired_target_graph_decodes_policies_and_hdr10_color_volume() -> anyhow::Result<()> {
        let Some(db) = setup_media_db("desired_target_graph_hdr10").await? else {
            return Ok(());
        };
        let target_id = create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: db.system_user_public_id,
                target_key: "hdr10-master",
                version: 1,
                display_name: "HDR10 master",
                container_format: "matroska",
                container_metadata_policy: "strip",
                container_chapter_policy: "replace",
                container_attachment_policy: "remove",
            },
        )
        .await?;
        append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                media_desired_target_profile_public_id: target_id,
                stream_key: "video-main",
                stream_kind: "video",
                semantic_role: Some("main"),
                language_code: None,
                optional: false,
                sort_order: 0,
                codec: "hevc",
                channel_count: None,
                channel_layout: None,
                audio_bitrate_bps: None,
                audio_sample_rate_hz: None,
                audio_loudness_profile: None,
                audio_dynamic_range: None,
                video_profile: Some("main10"),
                video_level: None,
                video_bitrate_bps: None,
                color_primaries: Some("bt2020"),
                color_transfer: Some("smpte2084"),
                color_space: Some("bt2020nc"),
                hdr_format: Some("hdr10"),
                hdr10_mastering_red_x: Some("34000/50000"),
                hdr10_mastering_red_y: Some("16000/50000"),
                hdr10_mastering_green_x: Some("13250/50000"),
                hdr10_mastering_green_y: Some("34500/50000"),
                hdr10_mastering_blue_x: Some("7500/50000"),
                hdr10_mastering_blue_y: Some("3000/50000"),
                hdr10_mastering_white_point_x: Some("15635/50000"),
                hdr10_mastering_white_point_y: Some("16450/50000"),
                hdr10_mastering_min_luminance: Some("50/10000"),
                hdr10_mastering_max_luminance: Some("10000000/10000"),
                hdr10_max_content_light_level: Some("1000"),
                hdr10_max_frame_average_light_level: Some("400"),
                title: Some("HDR10"),
                default_disposition: true,
                forced_disposition: false,
                subtitle_placement: None,
                image_subtitle_action: None,
            },
        )
        .await?;

        let rows = list_media_desired_target_graph_page(db.pool(), 1).await?;

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.target.container_metadata_policy, "strip");
        assert_eq!(row.target.container_chapter_policy, "replace");
        assert_eq!(row.target.container_attachment_policy, "remove");
        assert_eq!(
            row.stream.hdr10_mastering_red_x.as_deref(),
            Some("34000/50000")
        );
        assert_eq!(
            row.stream.hdr10_mastering_max_luminance.as_deref(),
            Some("10000000/10000")
        );
        assert_eq!(
            row.stream.hdr10_max_content_light_level.as_deref(),
            Some("1000")
        );
        assert_eq!(
            row.stream.hdr10_max_frame_average_light_level.as_deref(),
            Some("400")
        );
        Ok(())
    }

    async fn create_ordered_desired_target(db: &MediaTestDb) -> anyhow::Result<uuid::Uuid> {
        let target_id = create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: db.system_user_public_id,
                target_key: "theater-master",
                version: 2,
                display_name: "Theater master",
                container_format: "matroska",
                container_metadata_policy: "preserve",
                container_chapter_policy: "preserve",
                container_attachment_policy: "preserve",
            },
        )
        .await?;
        for input in [
            video_target_stream(target_id),
            audio_target_stream(target_id),
            subtitle_target_stream(target_id),
        ] {
            append_media_desired_target_stream(db.pool(), input).await?;
        }
        Ok(target_id)
    }

    async fn create_chapter_replacement_target(
        db: &MediaTestDb,
        target_key: &str,
    ) -> anyhow::Result<Uuid> {
        Ok(create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: db.system_user_public_id,
                target_key,
                version: 1,
                display_name: "Chapter replacement boundary",
                container_format: "matroska",
                container_metadata_policy: "preserve",
                container_chapter_policy: "replace",
                container_attachment_policy: "preserve",
            },
        )
        .await?)
    }

    #[tokio::test]
    async fn desired_target_chapter_limit_serializes_concurrent_appends() -> anyhow::Result<()> {
        let Some(db) = setup_media_db("desired_target_chapter_limit").await? else {
            return Ok(());
        };
        let target_id = create_chapter_replacement_target(&db, "chapter-limit").await?;
        let mut transaction = db.pool().begin().await?;
        for chapter_index in 0_i64..1_023 {
            append_media_desired_target_chapter_with_executor(
                &mut *transaction,
                AppendMediaDesiredTargetChapterInput {
                    media_desired_target_profile_public_id: target_id,
                    start_millis: chapter_index * 2,
                    end_millis: chapter_index * 2 + 1,
                },
            )
            .await?;
        }
        transaction.commit().await?;

        let first = append_media_desired_target_chapter(
            db.pool(),
            AppendMediaDesiredTargetChapterInput {
                media_desired_target_profile_public_id: target_id,
                start_millis: 2_046,
                end_millis: 2_047,
            },
        );
        let second = append_media_desired_target_chapter(
            db.pool(),
            AppendMediaDesiredTargetChapterInput {
                media_desired_target_profile_public_id: target_id,
                start_millis: 2_048,
                end_millis: 2_049,
            },
        );
        let (first_result, second_result) = tokio::join!(first, second);
        let results: [_; 2] = (first_result, second_result).into();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        let rejection = results
            .iter()
            .find_map(|result| result.as_ref().err())
            .ok_or_else(|| anyhow::anyhow!("one concurrent append should be rejected"))?;
        assert_eq!(
            rejection.database_detail(),
            Some("media_desired_target_chapter_count_exceeded")
        );
        assert_eq!(
            list_media_desired_target_chapters(db.pool(), target_id)
                .await?
                .len(),
            1_024
        );
        Ok(())
    }

    #[tokio::test]
    async fn desired_target_chapter_metadata_enforces_exact_aggregate_limit() -> anyhow::Result<()>
    {
        let Some(db) = setup_media_db("desired_target_chapter_metadata_limit").await? else {
            return Ok(());
        };
        let target_id = create_chapter_replacement_target(&db, "chapter-metadata-limit").await?;
        append_media_desired_target_chapter(
            db.pool(),
            AppendMediaDesiredTargetChapterInput {
                media_desired_target_profile_public_id: target_id,
                start_millis: 0,
                end_millis: 1,
            },
        )
        .await?;
        let value = "v".repeat(4_093);
        for metadata_index in 0..16 {
            let key = format!("k{metadata_index:02}");
            append_media_desired_target_chapter_metadata(
                db.pool(),
                AppendMediaDesiredTargetChapterMetadataInput {
                    media_desired_target_profile_public_id: target_id,
                    start_millis: 0,
                    metadata_key: &key,
                    metadata_value: &value,
                },
            )
            .await?;
        }

        let rejection = append_media_desired_target_chapter_metadata(
            db.pool(),
            AppendMediaDesiredTargetChapterMetadataInput {
                media_desired_target_profile_public_id: target_id,
                start_millis: 0,
                metadata_key: "overflow",
                metadata_value: "v",
            },
        )
        .await
        .expect_err("metadata beyond the aggregate byte limit should be rejected");
        assert_eq!(
            rejection.database_detail(),
            Some("media_desired_target_chapter_metadata_bytes_exceeded")
        );
        Ok(())
    }

    fn video_target_stream(target_id: uuid::Uuid) -> AppendMediaDesiredTargetStreamInput<'static> {
        AppendMediaDesiredTargetStreamInput {
            media_desired_target_profile_public_id: target_id,
            stream_key: "video-main",
            stream_kind: "video",
            semantic_role: None,
            language_code: None,
            optional: false,
            sort_order: 0,
            codec: "hevc",
            channel_count: None,
            channel_layout: None,
            audio_bitrate_bps: None,
            audio_sample_rate_hz: None,
            audio_loudness_profile: None,
            audio_dynamic_range: None,
            video_profile: Some("main10"),
            video_level: Some("5.1"),
            video_bitrate_bps: Some(8_000_000),
            color_primaries: Some("bt2020"),
            color_transfer: Some("smpte2084"),
            color_space: Some("bt2020nc"),
            hdr_format: Some("hdr10"),
            hdr10_mastering_red_x: Some("34000/50000"),
            hdr10_mastering_red_y: Some("16000/50000"),
            hdr10_mastering_green_x: Some("13250/50000"),
            hdr10_mastering_green_y: Some("34500/50000"),
            hdr10_mastering_blue_x: Some("7500/50000"),
            hdr10_mastering_blue_y: Some("3000/50000"),
            hdr10_mastering_white_point_x: Some("15635/50000"),
            hdr10_mastering_white_point_y: Some("16450/50000"),
            hdr10_mastering_min_luminance: Some("50/10000"),
            hdr10_mastering_max_luminance: Some("10000000/10000"),
            hdr10_max_content_light_level: Some("1000"),
            hdr10_max_frame_average_light_level: Some("400"),
            title: Some("Main picture"),
            default_disposition: true,
            forced_disposition: false,
            subtitle_placement: None,
            image_subtitle_action: None,
        }
    }

    fn audio_target_stream(target_id: uuid::Uuid) -> AppendMediaDesiredTargetStreamInput<'static> {
        AppendMediaDesiredTargetStreamInput {
            media_desired_target_profile_public_id: target_id,
            stream_key: "audio-main",
            stream_kind: "audio",
            semantic_role: Some("primary"),
            language_code: Some("eng"),
            optional: false,
            sort_order: 1,
            codec: "opus",
            channel_count: Some(2),
            channel_layout: Some("stereo"),
            audio_bitrate_bps: Some(160_000),
            audio_sample_rate_hz: Some(48_000),
            audio_loudness_profile: Some("dialog-normalized"),
            audio_dynamic_range: Some("speech"),
            video_profile: None,
            video_level: None,
            video_bitrate_bps: None,
            color_primaries: None,
            color_transfer: None,
            color_space: None,
            hdr_format: None,
            hdr10_mastering_red_x: None,
            hdr10_mastering_red_y: None,
            hdr10_mastering_green_x: None,
            hdr10_mastering_green_y: None,
            hdr10_mastering_blue_x: None,
            hdr10_mastering_blue_y: None,
            hdr10_mastering_white_point_x: None,
            hdr10_mastering_white_point_y: None,
            hdr10_mastering_min_luminance: None,
            hdr10_mastering_max_luminance: None,
            hdr10_max_content_light_level: None,
            hdr10_max_frame_average_light_level: None,
            title: Some("English"),
            default_disposition: true,
            forced_disposition: false,
            subtitle_placement: None,
            image_subtitle_action: None,
        }
    }

    fn subtitle_target_stream(
        target_id: uuid::Uuid,
    ) -> AppendMediaDesiredTargetStreamInput<'static> {
        AppendMediaDesiredTargetStreamInput {
            media_desired_target_profile_public_id: target_id,
            stream_key: "subtitle-forced",
            stream_kind: "subtitle",
            semantic_role: Some("forced"),
            language_code: Some("eng"),
            optional: true,
            sort_order: 2,
            codec: "webvtt",
            channel_count: None,
            channel_layout: None,
            audio_bitrate_bps: None,
            audio_sample_rate_hz: None,
            audio_loudness_profile: None,
            audio_dynamic_range: None,
            video_profile: None,
            video_level: None,
            video_bitrate_bps: None,
            color_primaries: None,
            color_transfer: None,
            color_space: None,
            hdr_format: None,
            hdr10_mastering_red_x: None,
            hdr10_mastering_red_y: None,
            hdr10_mastering_green_x: None,
            hdr10_mastering_green_y: None,
            hdr10_mastering_blue_x: None,
            hdr10_mastering_blue_y: None,
            hdr10_mastering_white_point_x: None,
            hdr10_mastering_white_point_y: None,
            hdr10_mastering_min_luminance: None,
            hdr10_mastering_max_luminance: None,
            hdr10_max_content_light_level: None,
            hdr10_max_frame_average_light_level: None,
            title: None,
            default_disposition: false,
            forced_disposition: true,
            subtitle_placement: Some("both"),
            image_subtitle_action: Some("remove"),
        }
    }

    fn retained_target_stream(
        target_id: uuid::Uuid,
        stream_key: &'static str,
        stream_kind: &'static str,
        sort_order: i32,
        codec: &'static str,
    ) -> AppendMediaDesiredTargetStreamInput<'static> {
        AppendMediaDesiredTargetStreamInput {
            media_desired_target_profile_public_id: target_id,
            stream_key,
            stream_kind,
            semantic_role: None,
            language_code: None,
            optional: false,
            sort_order,
            codec,
            channel_count: None,
            channel_layout: None,
            audio_bitrate_bps: None,
            audio_sample_rate_hz: None,
            audio_loudness_profile: None,
            audio_dynamic_range: None,
            video_profile: None,
            video_level: None,
            video_bitrate_bps: None,
            color_primaries: None,
            color_transfer: None,
            color_space: None,
            hdr_format: None,
            hdr10_mastering_red_x: None,
            hdr10_mastering_red_y: None,
            hdr10_mastering_green_x: None,
            hdr10_mastering_green_y: None,
            hdr10_mastering_blue_x: None,
            hdr10_mastering_blue_y: None,
            hdr10_mastering_white_point_x: None,
            hdr10_mastering_white_point_y: None,
            hdr10_mastering_min_luminance: None,
            hdr10_mastering_max_luminance: None,
            hdr10_max_content_light_level: None,
            hdr10_max_frame_average_light_level: None,
            title: None,
            default_disposition: false,
            forced_disposition: false,
            subtitle_placement: None,
            image_subtitle_action: None,
        }
    }

    async fn assert_invalid_fast_policy_rejected(pool: &sqlx::PgPool, actor_public_id: uuid::Uuid) {
        let result = upsert_media_policy_profile(
            pool,
            UpsertMediaPolicyProfileInput {
                actor_public_id,
                policy_key: "invalid-fast",
                version: 1,
                display_name: "Invalid fast",
                video_intent: "general",
                unmatched_video_action: "fail",
                unmatched_audio_action: "preserve",
                unmatched_subtitle_action: "preserve",
                unmatched_attachment_action: "preserve",
                unmatched_data_action: "remove",
                verification_strictness: "fast",
                verification_duration_tolerance_millis: 5_000,
                verification_mux_validation: true.into(),
                verification_decode_all_streams: true.into(),
                verification_keyframe_seek: false.into(),
                verification_playback_probe: false.into(),
            },
        )
        .await;
        assert!(result.is_err());
    }

    async fn assert_factory_reset_restores_defaults(pool: &sqlx::PgPool) -> anyhow::Result<()> {
        factory_reset(pool).await?;
        let reset_targets = list_media_compatibility_targets(pool).await?;
        assert!(reset_targets.iter().any(|target| {
            target.compatibility_target_key == "hevc-aac"
                && target.version == 1
                && target.video_codec == "hevc"
                && target.audio_codec == "aac"
        }));
        let reset_policies = list_media_policy_profiles(pool).await?;
        assert!(
            reset_policies
                .iter()
                .any(|policy| policy.policy_key == "safe_dry_run" && policy.version == 1)
        );
        let Some(reset_retention) = get_media_job_retention_policy(pool).await? else {
            return Err(anyhow::anyhow!(
                "retention policy should be restored by factory reset"
            ));
        };
        assert_default_retention(&reset_retention);
        Ok(())
    }

    #[tokio::test]
    async fn media_configuration_catalogs_are_stored_procedure_backed() -> anyhow::Result<()> {
        let Some(db) = setup_media_db("media_configuration_catalogs").await? else {
            return Ok(());
        };

        let targets = list_media_compatibility_targets(db.pool()).await?;
        assert!(targets.iter().any(|target| {
            target.compatibility_target_key == "plex-apple-tv"
                && target.version == 1
                && target.video_codec == "hevc"
                && target.audio_codec == "aac"
                && target.subtitle_policy == "selected"
        }));

        let policies = list_media_policy_profiles(db.pool()).await?;
        assert!(policies.iter().any(|policy| {
            policy.policy_key == "anime" && policy.version == 1 && policy.video_intent == "anime"
        }));

        let retention = get_media_job_retention_policy(db.pool()).await?;
        let Some(retention) = retention else {
            return Err(anyhow::anyhow!("retention policy should be seeded"));
        };
        assert_default_retention(&retention);

        let actor = db.system_user_public_id;
        let target = upsert_media_compatibility_target(
            db.pool(),
            UpsertMediaCompatibilityTargetInput {
                actor_public_id: actor,
                compatibility_target_key: "plex-living-room",
                version: 2,
                display_name: "Plex living room",
                video_codec: "av1",
                audio_codec: "opus",
                audio_channels: Some(2),
                audio_channel_layout: Some("stereo"),
                subtitle_policy: "all",
            },
        )
        .await?;
        assert_eq!(target.compatibility_target_key, "plex-living-room");
        assert_eq!(target.version, 2);
        assert_eq!(target.video_codec, "av1");
        assert_eq!(target.audio_channels, Some(2));
        assert_eq!(target.audio_channel_layout.as_deref(), Some("stereo"));
        assert_invalid_compatibility_audio_layout_rejected(&db, actor).await?;

        let policy = upsert_media_policy_profile(
            db.pool(),
            UpsertMediaPolicyProfileInput {
                actor_public_id: actor,
                policy_key: "archive-quality",
                version: 3,
                display_name: "Archive quality",
                video_intent: "archival",
                unmatched_video_action: "fail",
                unmatched_audio_action: "preserve",
                unmatched_subtitle_action: "preserve",
                unmatched_attachment_action: "preserve",
                unmatched_data_action: "remove",
                verification_strictness: "strict",
                verification_duration_tolerance_millis: 50,
                verification_mux_validation: true.into(),
                verification_decode_all_streams: true.into(),
                verification_keyframe_seek: true.into(),
                verification_playback_probe: true.into(),
            },
        )
        .await?;
        assert_eq!(policy.policy_key, "archive-quality");
        assert_eq!(policy.version, 3);
        assert_eq!(policy.video_intent, "archival");
        assert_eq!(policy.verification_strictness, "strict");
        assert_eq!(policy.verification_duration_tolerance_millis, 50);
        assert!(policy.verification_playback_probe.enabled());
        assert_invalid_fast_policy_rejected(db.pool(), actor).await;

        let retention = update_media_job_retention_policy(
            db.pool(),
            UpdateMediaJobRetentionPolicyInput {
                actor_public_id: actor,
                completed_enabled: true,
                completed_mode: "count".to_string(),
                completed_limit: 90,
                failed_diagnostic_enabled: true,
                failed_diagnostic_mode: "age".to_string(),
                failed_diagnostic_limit: 180,
            },
        )
        .await?;
        assert!(retention.completed_enabled);
        assert_eq!(retention.completed_mode, "count");
        assert_eq!(retention.completed_limit, 90);
        assert_eq!(retention.failed_diagnostic_limit, 180);

        assert_factory_reset_restores_defaults(db.pool()).await
    }

    #[tokio::test]
    async fn desired_target_versions_are_ordered_immutable_job_snapshots() -> anyhow::Result<()> {
        let Some(db) = setup_media_db("desired_target_versions_are_ordered").await? else {
            return Ok(());
        };
        let actor = db.system_user_public_id;
        let target_id = create_ordered_desired_target(&db).await?;

        let targets = list_media_desired_targets(db.pool()).await?;
        assert!(targets.iter().any(|target| {
            target.media_desired_target_profile_public_id == target_id
                && target.target_key == "theater-master"
                && target.version == 2
                && target.container_format == "matroska"
                && target.container_metadata_policy == "preserve"
                && target.container_chapter_policy == "preserve"
                && target.container_attachment_policy == "preserve"
        }));
        let streams = list_media_desired_target_streams(db.pool(), target_id).await?;
        assert_eq!(
            streams
                .iter()
                .map(|stream| (stream.stream_key.as_str(), stream.sort_order))
                .collect::<Vec<_>>(),
            vec![("video-main", 0), ("audio-main", 1), ("subtitle-forced", 2)]
        );
        assert_desired_stream_hdr10_color_volume(&streams[0]);
        assert_eq!(streams[1].channel_count, Some(2));
        assert_eq!(streams[1].channel_layout.as_deref(), Some("stereo"));
        assert_eq!(streams[1].audio_bitrate_bps, Some(160_000));
        assert_eq!(streams[1].audio_sample_rate_hz, Some(48_000));
        assert_eq!(streams[2].subtitle_placement.as_deref(), Some("both"));
        assert_eq!(streams[2].image_subtitle_action.as_deref(), Some("remove"));

        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: actor,
                profile_key: "target-snapshot-profile",
                source_root: "/input/target-snapshot",
                output_root: "/output/target-snapshot",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;
        set_media_profile_desired_target(
            db.pool(),
            actor,
            profile_id,
            Some("theater-master"),
            Some(2),
        )
        .await?;
        let job_id = create_target_snapshot_job(&db, actor, profile_id).await?;
        let job_streams = list_media_job_desired_target_streams(db.pool(), job_id).await?;
        assert_job_desired_target_stream_snapshot(&job_streams);
        let claimed = media_job_worker_claim_next(db.pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("target snapshot job should be claimable"))?;
        assert_claimed_container_policies(&claimed);
        assert_immutable_desired_target_stream_rejects_append(&db, target_id).await;
        Ok(())
    }

    async fn assert_immutable_desired_target_stream_rejects_append(
        db: &MediaTestDb,
        target_id: uuid::Uuid,
    ) {
        let immutable_write = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                media_desired_target_profile_public_id: target_id,
                stream_key: "audio-late",
                stream_kind: "audio",
                semantic_role: None,
                language_code: None,
                optional: true,
                sort_order: 3,
                codec: "aac",
                channel_count: None,
                channel_layout: None,
                audio_bitrate_bps: None,
                audio_sample_rate_hz: None,
                audio_loudness_profile: None,
                audio_dynamic_range: None,
                video_profile: None,
                video_level: None,
                video_bitrate_bps: None,
                color_primaries: None,
                color_transfer: None,
                color_space: None,
                hdr_format: None,
                hdr10_mastering_red_x: None,
                hdr10_mastering_red_y: None,
                hdr10_mastering_green_x: None,
                hdr10_mastering_green_y: None,
                hdr10_mastering_blue_x: None,
                hdr10_mastering_blue_y: None,
                hdr10_mastering_white_point_x: None,
                hdr10_mastering_white_point_y: None,
                hdr10_mastering_min_luminance: None,
                hdr10_mastering_max_luminance: None,
                hdr10_max_content_light_level: None,
                hdr10_max_frame_average_light_level: None,
                title: None,
                default_disposition: false,
                forced_disposition: false,
                subtitle_placement: None,
                image_subtitle_action: None,
            },
        )
        .await;
        assert!(immutable_write.is_err());
    }

    fn assert_desired_stream_hdr10_color_volume(stream: &MediaDesiredTargetStreamRow) {
        assert_hdr10_color_volume_fields(&[
            (
                "hdr10_mastering_red_x",
                stream.hdr10_mastering_red_x.as_deref(),
                "34000/50000",
            ),
            (
                "hdr10_mastering_red_y",
                stream.hdr10_mastering_red_y.as_deref(),
                "16000/50000",
            ),
            (
                "hdr10_mastering_green_x",
                stream.hdr10_mastering_green_x.as_deref(),
                "13250/50000",
            ),
            (
                "hdr10_mastering_green_y",
                stream.hdr10_mastering_green_y.as_deref(),
                "34500/50000",
            ),
            (
                "hdr10_mastering_blue_x",
                stream.hdr10_mastering_blue_x.as_deref(),
                "7500/50000",
            ),
            (
                "hdr10_mastering_blue_y",
                stream.hdr10_mastering_blue_y.as_deref(),
                "3000/50000",
            ),
            (
                "hdr10_mastering_white_point_x",
                stream.hdr10_mastering_white_point_x.as_deref(),
                "15635/50000",
            ),
            (
                "hdr10_mastering_white_point_y",
                stream.hdr10_mastering_white_point_y.as_deref(),
                "16450/50000",
            ),
            (
                "hdr10_mastering_min_luminance",
                stream.hdr10_mastering_min_luminance.as_deref(),
                "50/10000",
            ),
            (
                "hdr10_mastering_max_luminance",
                stream.hdr10_mastering_max_luminance.as_deref(),
                "10000000/10000",
            ),
            (
                "hdr10_max_content_light_level",
                stream.hdr10_max_content_light_level.as_deref(),
                "1000",
            ),
            (
                "hdr10_max_frame_average_light_level",
                stream.hdr10_max_frame_average_light_level.as_deref(),
                "400",
            ),
        ]);
    }

    #[tokio::test]
    async fn desired_target_accepts_exact_retained_stream_rows() -> anyhow::Result<()> {
        let Some(db) = setup_media_db("desired_target_retained_stream_rows").await? else {
            return Ok(());
        };
        let target_id = create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: db.system_user_public_id,
                target_key: "retained-streams",
                version: 1,
                display_name: "Retained streams",
                container_format: "matroska",
                container_metadata_policy: "preserve",
                container_chapter_policy: "preserve",
                container_attachment_policy: "preserve",
            },
        )
        .await?;

        append_media_desired_target_stream(
            db.pool(),
            retained_target_stream(target_id, "font-main", "attachment", 0, "ttf"),
        )
        .await?;
        append_media_desired_target_stream(
            db.pool(),
            retained_target_stream(target_id, "timecode-main", "data", 1, "bin_data"),
        )
        .await?;
        let streams = list_media_desired_target_streams(db.pool(), target_id).await?;
        assert_eq!(
            streams
                .iter()
                .map(|stream| {
                    (
                        stream.stream_key.as_str(),
                        stream.stream_kind.as_str(),
                        stream.codec.as_str(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("font-main", "attachment", "ttf"),
                ("timecode-main", "data", "bin_data")
            ]
        );

        let rewritten = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "font-renamed",
                sort_order: 2,
                title: Some("Renamed Font"),
                ..retained_target_stream(target_id, "font-renamed", "attachment", 2, "ttf")
            },
        )
        .await;
        let Err(rewritten) = rewritten else {
            return Err(anyhow::anyhow!("retained stream rewrite was accepted"));
        };
        assert_eq!(
            rewritten.database_detail(),
            Some("media_desired_target_retained_stream_shape_invalid")
        );
        Ok(())
    }

    #[tokio::test]
    async fn desired_target_metadata_enforces_count_and_aggregate_limits() -> anyhow::Result<()> {
        let Some(db) = setup_media_db("desired_target_metadata_limits").await? else {
            return Ok(());
        };
        let count_target = create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: db.system_user_public_id,
                target_key: "metadata-count-limit",
                version: 1,
                display_name: "Metadata count limit",
                container_format: "matroska",
                container_metadata_policy: "replace",
                container_chapter_policy: "preserve",
                container_attachment_policy: "preserve",
            },
        )
        .await?;
        for index in 0..64 {
            append_media_desired_target_metadata(
                db.pool(),
                AppendMediaDesiredTargetMetadataInput {
                    media_desired_target_profile_public_id: count_target,
                    metadata_key: &format!("key-{index}"),
                    metadata_value: "value",
                },
            )
            .await?;
        }
        let count_error = append_media_desired_target_metadata(
            db.pool(),
            AppendMediaDesiredTargetMetadataInput {
                media_desired_target_profile_public_id: count_target,
                metadata_key: "overflow",
                metadata_value: "value",
            },
        )
        .await
        .expect_err("metadata row 65 must be rejected");
        assert_eq!(
            count_error.database_detail(),
            Some("media_desired_target_metadata_count_exceeded")
        );

        let bytes_target = create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: db.system_user_public_id,
                target_key: "metadata-byte-limit",
                version: 1,
                display_name: "Metadata byte limit",
                container_format: "matroska",
                container_metadata_policy: "replace",
                container_chapter_policy: "preserve",
                container_attachment_policy: "preserve",
            },
        )
        .await?;
        let maximum_value = "x".repeat(4_090);
        for index in 0..16 {
            append_media_desired_target_metadata(
                db.pool(),
                AppendMediaDesiredTargetMetadataInput {
                    media_desired_target_profile_public_id: bytes_target,
                    metadata_key: &format!("tag-{index:02}"),
                    metadata_value: &maximum_value,
                },
            )
            .await?;
        }
        let bytes_error = append_media_desired_target_metadata(
            db.pool(),
            AppendMediaDesiredTargetMetadataInput {
                media_desired_target_profile_public_id: bytes_target,
                metadata_key: "overflow",
                metadata_value: "x",
            },
        )
        .await
        .expect_err("aggregate metadata beyond 65536 bytes must be rejected");
        assert_eq!(
            bytes_error.database_detail(),
            Some("media_desired_target_metadata_bytes_exceeded")
        );
        Ok(())
    }

    async fn create_target_snapshot_job(
        db: &MediaTestDb,
        actor: Uuid,
        profile_id: Uuid,
    ) -> anyhow::Result<Uuid> {
        enqueue_discovered_media_job(
            db.pool(),
            &EnqueueDiscoveredMediaJobInput {
                actor_public_id: actor,
                media_profile_public_id: profile_id,
                source_path: "/input/target-snapshot/movie.mkv",
                output_path: Some("/output/target-snapshot/movie.mkv"),
                source_identity: "0000000000000001:0000000000000001",
                source_size_bytes: 100,
                source_modified_ns: 100,
                source_changed_ns: 100,
                source_sha256: &"1".repeat(64),
            },
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("target snapshot job should be queued"))
    }

    #[tokio::test]
    async fn desired_target_pin_rejects_empty_stream_graph() -> anyhow::Result<()> {
        let Some(db) = setup_media_db("desired_target_empty_stream_graph").await? else {
            return Ok(());
        };
        let actor = db.system_user_public_id;
        create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: actor,
                target_key: "empty-stream-target",
                version: 1,
                display_name: "Empty stream target",
                container_format: "matroska",
                container_metadata_policy: "preserve",
                container_chapter_policy: "preserve",
                container_attachment_policy: "preserve",
            },
        )
        .await?;
        let profile_id = upsert_media_profile(
            db.pool(),
            &UpsertMediaProfileInput {
                actor_public_id: actor,
                profile_key: "empty-stream-profile",
                source_root: "/input/empty-stream",
                output_root: "/output/empty-stream",
                dry_run_only: true,
                retention_days: 30,
                compatibility_target_key: None,
                policy_key: "safe_dry_run",
                watcher_enabled: false,
                schedule_enabled: false,
                schedule_interval_minutes: None,
            },
        )
        .await?;

        let err = set_media_profile_desired_target(
            db.pool(),
            actor,
            profile_id,
            Some("empty-stream-target"),
            Some(1),
        )
        .await
        .expect_err("empty desired-target stream graph should not be pinnable");
        assert_eq!(
            err.database_detail(),
            Some("media_desired_target_streams_required")
        );
        Ok(())
    }

    fn assert_job_desired_target_stream_snapshot(
        job_streams: &[crate::media::jobs::MediaJobDesiredTargetStreamRow],
    ) {
        assert_eq!(job_streams.len(), 3);
        assert_eq!(job_streams[0].video_profile.as_deref(), Some("main10"));
        assert_eq!(job_streams[0].video_level.as_deref(), Some("5.1"));
        assert_eq!(job_streams[0].video_bitrate_bps, Some(8_000_000));
        assert_eq!(job_streams[0].color_primaries.as_deref(), Some("bt2020"));
        assert_eq!(job_streams[0].color_transfer.as_deref(), Some("smpte2084"));
        assert_eq!(job_streams[0].color_space.as_deref(), Some("bt2020nc"));
        assert_eq!(job_streams[0].hdr_format.as_deref(), Some("hdr10"));
        assert_job_stream_hdr10_color_volume(&job_streams[0]);
        assert_eq!(job_streams[1].audio_bitrate_bps, Some(160_000));
        assert_eq!(job_streams[1].audio_sample_rate_hz, Some(48_000));
        assert_eq!(
            job_streams[1].audio_loudness_profile.as_deref(),
            Some("dialog-normalized")
        );
        assert_eq!(
            job_streams[1].audio_dynamic_range.as_deref(),
            Some("speech")
        );
        assert_eq!(job_streams[2].stream_key, "subtitle-forced");
        assert!(job_streams[2].optional);
        assert_eq!(job_streams[2].subtitle_placement.as_deref(), Some("both"));
        assert_eq!(
            job_streams[2].image_subtitle_action.as_deref(),
            Some("remove")
        );
    }

    fn assert_job_stream_hdr10_color_volume(
        stream: &crate::media::jobs::MediaJobDesiredTargetStreamRow,
    ) {
        assert_hdr10_color_volume_fields(&[
            (
                "hdr10_mastering_red_x",
                stream.hdr10_mastering_red_x.as_deref(),
                "34000/50000",
            ),
            (
                "hdr10_mastering_red_y",
                stream.hdr10_mastering_red_y.as_deref(),
                "16000/50000",
            ),
            (
                "hdr10_mastering_green_x",
                stream.hdr10_mastering_green_x.as_deref(),
                "13250/50000",
            ),
            (
                "hdr10_mastering_green_y",
                stream.hdr10_mastering_green_y.as_deref(),
                "34500/50000",
            ),
            (
                "hdr10_mastering_blue_x",
                stream.hdr10_mastering_blue_x.as_deref(),
                "7500/50000",
            ),
            (
                "hdr10_mastering_blue_y",
                stream.hdr10_mastering_blue_y.as_deref(),
                "3000/50000",
            ),
            (
                "hdr10_mastering_white_point_x",
                stream.hdr10_mastering_white_point_x.as_deref(),
                "15635/50000",
            ),
            (
                "hdr10_mastering_white_point_y",
                stream.hdr10_mastering_white_point_y.as_deref(),
                "16450/50000",
            ),
            (
                "hdr10_mastering_min_luminance",
                stream.hdr10_mastering_min_luminance.as_deref(),
                "50/10000",
            ),
            (
                "hdr10_mastering_max_luminance",
                stream.hdr10_mastering_max_luminance.as_deref(),
                "10000000/10000",
            ),
            (
                "hdr10_max_content_light_level",
                stream.hdr10_max_content_light_level.as_deref(),
                "1000",
            ),
            (
                "hdr10_max_frame_average_light_level",
                stream.hdr10_max_frame_average_light_level.as_deref(),
                "400",
            ),
        ]);
    }

    fn assert_hdr10_color_volume_fields(fields: &[(&str, Option<&str>, &str)]) {
        for &(field_name, actual, expected) in fields {
            assert_eq!(actual, Some(expected), "{field_name}");
        }
    }

    fn assert_claimed_container_policies(claimed: &crate::media::jobs::ClaimedMediaJobRow) {
        assert_eq!(
            claimed.desired_container_metadata_policy.as_deref(),
            Some("preserve")
        );
        assert_eq!(
            claimed.desired_container_chapter_policy.as_deref(),
            Some("preserve")
        );
        assert_eq!(
            claimed.desired_container_attachment_policy.as_deref(),
            Some("preserve")
        );
    }

    #[tokio::test]
    async fn desired_target_subtitle_shape_rejects_invalid_values_and_cross_kind_fields()
    -> anyhow::Result<()> {
        let Some(db) = setup_media_db("desired_target_subtitle_shape").await? else {
            return Ok(());
        };
        let target_id = create_subtitle_shape_validation_target(&db).await?;
        assert_invalid_subtitle_placement_rejected(&db, target_id).await?;
        assert_cross_kind_subtitle_fields_rejected(&db, target_id).await?;
        assert_unknown_hdr_format_rejected(&db, target_id).await?;
        assert_hdr10_color_volume_requires_hdr10_format(&db, target_id).await?;
        assert_unknown_video_color_rejected(&db, target_id).await?;
        assert_unknown_video_level_rejected(&db, target_id).await?;
        assert_invalid_audio_channel_layout_rejected(&db, target_id).await?;
        assert_audio_channel_layout_count_mismatch_rejected(&db, target_id).await?;
        assert_unsupported_target_stream_kinds_rejected(&db, target_id).await
    }

    async fn assert_invalid_compatibility_audio_layout_rejected(
        db: &MediaTestDb,
        actor: Uuid,
    ) -> anyhow::Result<()> {
        let invalid_audio_layout = upsert_media_compatibility_target(
            db.pool(),
            UpsertMediaCompatibilityTargetInput {
                actor_public_id: actor,
                compatibility_target_key: "invalid-audio-layout",
                version: 1,
                display_name: "Invalid audio layout",
                video_codec: "av1",
                audio_codec: "opus",
                audio_channels: Some(2),
                audio_channel_layout: Some("5.1(side)"),
                subtitle_policy: "all",
            },
        )
        .await;
        let Err(invalid_audio_layout) = invalid_audio_layout else {
            return Err(anyhow::anyhow!(
                "compatibility target audio layout/count mismatch was accepted"
            ));
        };
        assert_eq!(
            invalid_audio_layout.database_detail(),
            Some("media_compatibility_target_audio_shape_invalid")
        );
        Ok(())
    }

    async fn create_subtitle_shape_validation_target(db: &MediaTestDb) -> anyhow::Result<Uuid> {
        Ok(create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: db.system_user_public_id,
                target_key: "subtitle-shape-validation",
                version: 1,
                display_name: "Subtitle shape validation",
                container_format: "matroska",
                container_metadata_policy: "preserve",
                container_chapter_policy: "preserve",
                container_attachment_policy: "preserve",
            },
        )
        .await?)
    }

    async fn assert_invalid_subtitle_placement_rejected(
        db: &MediaTestDb,
        target_id: Uuid,
    ) -> anyhow::Result<()> {
        let invalid_placement = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                media_desired_target_profile_public_id: target_id,
                stream_key: "subtitle-invalid",
                stream_kind: "subtitle",
                semantic_role: None,
                language_code: Some("eng"),
                optional: false,
                sort_order: 0,
                codec: "srt",
                channel_count: None,
                channel_layout: None,
                audio_bitrate_bps: None,
                audio_sample_rate_hz: None,
                audio_loudness_profile: None,
                audio_dynamic_range: None,
                video_profile: None,
                video_level: None,
                video_bitrate_bps: None,
                color_primaries: None,
                color_transfer: None,
                color_space: None,
                hdr_format: None,
                hdr10_mastering_red_x: None,
                hdr10_mastering_red_y: None,
                hdr10_mastering_green_x: None,
                hdr10_mastering_green_y: None,
                hdr10_mastering_blue_x: None,
                hdr10_mastering_blue_y: None,
                hdr10_mastering_white_point_x: None,
                hdr10_mastering_white_point_y: None,
                hdr10_mastering_min_luminance: None,
                hdr10_mastering_max_luminance: None,
                hdr10_max_content_light_level: None,
                hdr10_max_frame_average_light_level: None,
                title: None,
                default_disposition: false,
                forced_disposition: false,
                subtitle_placement: Some("elsewhere"),
                image_subtitle_action: Some("fail"),
            },
        )
        .await;
        let Err(invalid_placement) = invalid_placement else {
            return Err(anyhow::anyhow!("invalid subtitle placement was accepted"));
        };
        assert_eq!(
            invalid_placement.database_detail(),
            Some("media_desired_target_subtitle_shape_invalid")
        );
        Ok(())
    }

    async fn assert_cross_kind_subtitle_fields_rejected(
        db: &MediaTestDb,
        target_id: Uuid,
    ) -> anyhow::Result<()> {
        let cross_kind = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                subtitle_placement: Some("embedded"),
                image_subtitle_action: Some("fail"),
                ..video_target_stream(target_id)
            },
        )
        .await;
        let Err(cross_kind) = cross_kind else {
            return Err(anyhow::anyhow!("subtitle fields on video were accepted"));
        };
        assert_eq!(
            cross_kind.database_detail(),
            Some("media_desired_target_subtitle_shape_invalid")
        );
        Ok(())
    }

    async fn assert_unknown_hdr_format_rejected(
        db: &MediaTestDb,
        target_id: Uuid,
    ) -> anyhow::Result<()> {
        let unknown_hdr = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "video-unknown-hdr",
                sort_order: 0,
                hdr_format: Some("dolby_vision"),
                ..video_target_stream(target_id)
            },
        )
        .await;
        let Err(unknown_hdr) = unknown_hdr else {
            return Err(anyhow::anyhow!("unknown HDR format was accepted"));
        };
        assert_eq!(
            unknown_hdr.database_detail(),
            Some("media_desired_target_video_shape_invalid")
        );
        Ok(())
    }

    async fn assert_hdr10_color_volume_requires_hdr10_format(
        db: &MediaTestDb,
        target_id: Uuid,
    ) -> anyhow::Result<()> {
        let missing_hdr_format = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "video-hdr10-volume-without-format",
                sort_order: 0,
                hdr_format: None,
                ..video_target_stream(target_id)
            },
        )
        .await;
        let Err(missing_hdr_format) = missing_hdr_format else {
            return Err(anyhow::anyhow!(
                "HDR10 color volume without hdr10 format was accepted"
            ));
        };
        assert_eq!(
            missing_hdr_format.database_detail(),
            Some("media_desired_target_video_shape_invalid")
        );
        Ok(())
    }

    async fn assert_unknown_video_color_rejected(
        db: &MediaTestDb,
        target_id: Uuid,
    ) -> anyhow::Result<()> {
        let unknown_color = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "video-unknown-color",
                sort_order: 0,
                color_transfer: Some("make-it-pop"),
                ..video_target_stream(target_id)
            },
        )
        .await;
        let Err(unknown_color) = unknown_color else {
            return Err(anyhow::anyhow!("unknown video color was accepted"));
        };
        assert_eq!(
            unknown_color.database_detail(),
            Some("media_desired_target_video_shape_invalid")
        );
        Ok(())
    }

    async fn assert_unknown_video_level_rejected(
        db: &MediaTestDb,
        target_id: Uuid,
    ) -> anyhow::Result<()> {
        let unknown_level = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "video-unknown-level",
                sort_order: 0,
                video_level: Some("7.9"),
                ..video_target_stream(target_id)
            },
        )
        .await;
        let Err(unknown_level) = unknown_level else {
            return Err(anyhow::anyhow!("unknown video level was accepted"));
        };
        assert_eq!(
            unknown_level.database_detail(),
            Some("media_desired_target_video_shape_invalid")
        );

        let av1_target_id = create_media_desired_target(
            db.pool(),
            CreateMediaDesiredTargetInput {
                actor_public_id: db.system_user_public_id,
                target_key: "av1-level-validation",
                version: 1,
                display_name: "AV1 level validation",
                container_format: "matroska",
                container_metadata_policy: "preserve",
                container_chapter_policy: "preserve",
                container_attachment_policy: "preserve",
            },
        )
        .await?;

        append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "video-av1-level",
                sort_order: 0,
                codec: "av1",
                video_level: Some("7.3"),
                ..video_target_stream(av1_target_id)
            },
        )
        .await?;

        let unknown_av1_level = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "video-unknown-av1-level",
                sort_order: 1,
                codec: "libaom-av1",
                video_level: Some("7.9"),
                ..video_target_stream(target_id)
            },
        )
        .await;
        let Err(unknown_av1_level) = unknown_av1_level else {
            return Err(anyhow::anyhow!("unknown AV1 video level was accepted"));
        };
        assert_eq!(
            unknown_av1_level.database_detail(),
            Some("media_desired_target_video_shape_invalid")
        );
        Ok(())
    }

    async fn assert_invalid_audio_channel_layout_rejected(
        db: &MediaTestDb,
        target_id: Uuid,
    ) -> anyhow::Result<()> {
        let invalid_layout = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "audio-unknown-layout",
                sort_order: 0,
                channel_layout: Some("ambisonic"),
                ..audio_target_stream(target_id)
            },
        )
        .await;
        let Err(invalid_layout) = invalid_layout else {
            return Err(anyhow::anyhow!("unknown audio channel layout was accepted"));
        };
        assert_eq!(
            invalid_layout.database_detail(),
            Some("media_desired_target_audio_shape_invalid")
        );
        Ok(())
    }

    async fn assert_audio_channel_layout_count_mismatch_rejected(
        db: &MediaTestDb,
        target_id: Uuid,
    ) -> anyhow::Result<()> {
        let count_mismatch = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "audio-layout-count-mismatch",
                sort_order: 0,
                channel_count: Some(6),
                channel_layout: Some("stereo"),
                ..audio_target_stream(target_id)
            },
        )
        .await;
        let Err(count_mismatch) = count_mismatch else {
            return Err(anyhow::anyhow!(
                "audio channel layout/count mismatch was accepted"
            ));
        };
        assert_eq!(
            count_mismatch.database_detail(),
            Some("media_desired_target_audio_shape_invalid")
        );
        Ok(())
    }

    async fn assert_unsupported_target_stream_kinds_rejected(
        db: &MediaTestDb,
        target_id: Uuid,
    ) -> anyhow::Result<()> {
        let unsupported_attachment = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "cover-art",
                stream_kind: "attachment",
                codec: "mjpeg",
                sort_order: 0,
                ..video_target_stream(target_id)
            },
        )
        .await;
        if unsupported_attachment.is_ok() {
            return Err(anyhow::anyhow!(
                "unsupported attachment target stream was accepted"
            ));
        }

        let unsupported_chapter = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "chapters",
                stream_kind: "chapter",
                codec: "bin_data",
                sort_order: 0,
                ..video_target_stream(target_id)
            },
        )
        .await;
        if unsupported_chapter.is_ok() {
            return Err(anyhow::anyhow!(
                "unsupported chapter target stream was accepted"
            ));
        }

        let unsupported_data = append_media_desired_target_stream(
            db.pool(),
            AppendMediaDesiredTargetStreamInput {
                stream_key: "opaque-data",
                stream_kind: "data",
                codec: "bin_data",
                sort_order: 0,
                ..video_target_stream(target_id)
            },
        )
        .await;
        if unsupported_data.is_ok() {
            return Err(anyhow::anyhow!(
                "unsupported data target stream was accepted"
            ));
        }
        Ok(())
    }
}
