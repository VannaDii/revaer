//! Desired-target compilation into a concrete output graph.

use crate::classify::{SemanticRole, infer_role};
use crate::model::{
    ContainerChapterEntry, ContainerMetadataEntry, DesiredGraph, DesiredStreamBinding, MediaGraph,
    MediaStream, StreamKind,
};
use crate::normalize::{
    audio_channel_count_for_layout, normalize_audio_channel_layout,
    normalize_container_attachment_policy, normalize_container_chapter_policy,
    normalize_container_format, normalize_container_metadata, normalize_container_metadata_policy,
    normalize_subtitle_codec,
};
use serde::{Deserialize, Deserializer};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};
use thiserror::Error;

/// Maximum accepted desired-target YAML payload size.
pub const MAX_DESIRED_TARGET_YAML_BYTES: usize = 1_048_576;
/// Maximum desired streams accepted in one target document.
pub const MAX_DESIRED_TARGET_STREAMS: usize = 1_024;
/// Maximum exact container-metadata rows accepted by every ingress and execution boundary.
pub const MAX_CONTAINER_METADATA_ENTRIES: usize = 64;
/// Maximum UTF-8 bytes accepted in one normalized container-metadata key.
pub const MAX_CONTAINER_METADATA_KEY_BYTES: usize = 128;
/// Maximum UTF-8 bytes accepted in one normalized container-metadata value.
pub const MAX_CONTAINER_METADATA_VALUE_BYTES: usize = 4_096;
/// Maximum aggregate UTF-8 key/value bytes accepted for one desired target.
pub const MAX_CONTAINER_METADATA_TOTAL_BYTES: usize = 65_536;
/// Supported aggregate argument bytes reserved for container-metadata `FFmpeg` options.
pub const MAX_CONTAINER_METADATA_ARG_BYTES: usize = 131_072;
/// Maximum chapter timeline entries accepted at every boundary.
pub const MAX_CONTAINER_CHAPTERS: usize = 1_024;
/// Maximum metadata entries accepted for one chapter.
pub const MAX_CONTAINER_CHAPTER_METADATA_ENTRIES: usize = 64;
/// Maximum aggregate chapter metadata UTF-8 bytes across one timeline.
pub const MAX_CONTAINER_CHAPTER_METADATA_TOTAL_BYTES: usize = 65_536;

/// Validated normalized ISO-639-3 language token used by desired targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageToken(String);

impl LanguageToken {
    /// Parse one lowercase three-letter ASCII language token.
    ///
    /// # Errors
    ///
    /// Returns [`TargetCompileError::InvalidLanguage`] for separators, dot segments, controls,
    /// percent-encoded bytes, mixed case, or any non-ISO token shape.
    pub fn parse(value: &str) -> Result<Self, TargetCompileError> {
        let normalized = value.trim();
        if normalized.len() != 3 || !normalized.bytes().all(|byte| byte.is_ascii_lowercase()) {
            return Err(TargetCompileError::InvalidLanguage(value.to_string()));
        }
        Ok(Self(normalized.to_string()))
    }

    /// Return the normalized token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for LanguageToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

/// Desired output stream independent of source-container stream indexes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetStream {
    /// Stable identity within the target version.
    pub stream_key: String,
    /// Required media kind.
    pub kind: StreamKind,
    /// Optional semantic-role selector.
    pub role: Option<SemanticRole>,
    /// Optional normalized language selector.
    pub language: Option<LanguageToken>,
    /// Optional stable binding key shared by outputs that reuse one selected source stream.
    pub source_binding_key: Option<String>,
    /// Whether absence of a matching source stream is acceptable.
    pub optional: bool,
    /// Desired output codec.
    pub codec: String,
    /// Desired audio channel count.
    pub channels: Option<u32>,
    /// Desired audio channel layout.
    pub channel_layout: Option<String>,
    /// Desired average audio bitrate in bits per second.
    pub audio_bitrate_bps: Option<u32>,
    /// Desired audio sample rate in hertz.
    pub audio_sample_rate_hz: Option<u32>,
    /// Desired audio loudness processing profile.
    pub audio_loudness_profile: Option<String>,
    /// Desired audio dynamic-range behavior.
    pub audio_dynamic_range: Option<String>,
    /// Desired video profile, such as `main` or `main10`.
    pub video_profile: Option<String>,
    /// Desired video level, stored as the canonical toolchain string.
    pub video_level: Option<String>,
    /// Desired average video bitrate in bits per second.
    pub video_bitrate_bps: Option<u32>,
    /// Desired video color primaries.
    pub color_primaries: Option<String>,
    /// Desired video transfer characteristic.
    pub color_transfer: Option<String>,
    /// Desired video color space.
    pub color_space: Option<String>,
    /// Desired HDR format label.
    pub hdr_format: Option<String>,
    /// Desired stream title. `None` removes the source title.
    pub title: Option<String>,
    /// Complete desired disposition set.
    #[serde(default)]
    pub dispositions: Vec<String>,
    /// Desired subtitle placement. Non-subtitle rows leave this unset.
    pub subtitle_placement: Option<SubtitlePlacement>,
    /// Image-subtitle action. Non-subtitle rows leave this unset.
    pub image_subtitle_action: Option<ImageSubtitleAction>,
}

/// Desired placement for a selected subtitle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubtitlePlacement {
    /// Keep the selected subtitle in the media container.
    Embedded,
    /// Materialize the selected subtitle as an adjacent sidecar only.
    Sidecar,
    /// Keep an embedded stream and materialize an adjacent sidecar.
    Both,
    /// Remove the selected subtitle from both locations.
    None,
}

/// Policy for image-based subtitle inputs when OCR is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSubtitleAction {
    /// Preserve the image subtitle without format conversion.
    Preserve,
    /// Remove the selected image subtitle.
    Remove,
    /// Reject compilation when the selected source is image-based.
    Fail,
}

/// Existing adjacent subtitle available as a target input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarSubtitleInput {
    /// Primary sidecar path.
    pub path: String,
    /// Companion path for paired formats such as `VobSub`.
    pub companion_path: Option<String>,
    /// Normalized language when known.
    pub language: Option<String>,
    /// Normalized semantic role when known.
    pub role: Option<SemanticRole>,
    /// Canonical subtitle codec.
    pub codec: String,
    /// Whether the source contains image subtitles.
    pub image_based: bool,
}

/// External sidecar selected as an embedded stream input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarEmbedding {
    /// Primary sidecar input path.
    pub path: String,
    /// Companion path when required by the input format.
    pub companion_path: Option<String>,
    /// Synthetic desired output stream.
    pub output_stream: MediaStream,
    /// Canonical input codec.
    pub source_codec: String,
}

/// Source used to materialize one desired sidecar output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarOutputSource {
    /// Copy an existing sidecar, including its optional companion.
    ExistingSidecar {
        /// Primary source path.
        path: String,
        /// Companion source path.
        companion_path: Option<String>,
        /// Canonical source codec.
        codec: String,
    },
    /// Extract an embedded stream from the source media.
    EmbeddedStream {
        /// Source-container stream id.
        stream_id: u32,
    },
}

/// One desired sidecar artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredSidecarOutput {
    /// Primary managed workspace candidate path.
    pub path: String,
    /// Companion managed workspace candidate path for paired formats.
    pub companion_path: Option<String>,
    /// Final source-adjacent destination path.
    pub destination_path: String,
    /// Final source-adjacent companion destination for paired formats.
    pub destination_companion_path: Option<String>,
    /// Materialization source.
    pub source: SidecarOutputSource,
    /// Canonical output codec.
    pub codec: String,
}

/// Complete target compilation, including auxiliary subtitle artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledDesiredTarget {
    /// Desired primary media graph.
    pub graph: DesiredGraph,
    /// Existing sidecars selected as embedded input streams.
    pub sidecar_embeddings: Vec<SidecarEmbedding>,
    /// Sidecar artifacts to copy or extract into the managed workspace.
    pub sidecar_outputs: Vec<DesiredSidecarOutput>,
    /// Existing sidecars to remove only after verified replacement commits.
    pub sidecar_removals: Vec<String>,
}

/// Versioned desired output graph definition.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredTarget {
    /// Stable target key.
    pub target_key: String,
    /// Immutable target version.
    pub version: u32,
    /// Desired output container format.
    pub container: String,
    /// Desired output container metadata policy.
    #[serde(default = "default_container_preservation_policy")]
    pub container_metadata_policy: String,
    /// Desired exact container metadata rows for the `replace` metadata policy.
    #[serde(default)]
    pub container_metadata: Vec<ContainerMetadataEntry>,
    /// Desired output container chapter policy.
    #[serde(default = "default_container_preservation_policy")]
    pub container_chapter_policy: String,
    /// Desired exact chapter timeline rows for the `replace` chapter policy.
    #[serde(default)]
    pub container_chapters: Vec<ContainerChapterEntry>,
    /// Desired output attachment policy.
    pub container_attachment_policy: String,
    /// Desired streams in final mux order.
    pub streams: Vec<TargetStream>,
}

fn default_container_preservation_policy() -> String {
    "preserve".to_string()
}

/// Policy for source streams that no target row selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmatchedStreamPolicy {
    /// Remove unmatched streams from the output.
    Remove,
    /// Append unmatched streams in source order.
    Preserve,
    /// Reject compilation when any source stream is unmatched.
    Reject,
}

/// Policy set for source streams that no target row selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnmatchedStreamPolicies {
    /// Policy for unmatched video streams.
    pub video: UnmatchedStreamPolicy,
    /// Policy for unmatched audio streams.
    pub audio: UnmatchedStreamPolicy,
    /// Policy for unmatched subtitle streams and sidecars.
    pub subtitle: UnmatchedStreamPolicy,
    /// Policy for unmatched attachment streams.
    pub attachment: UnmatchedStreamPolicy,
    /// Policy for unmatched opaque data streams.
    pub data: UnmatchedStreamPolicy,
}

impl UnmatchedStreamPolicies {
    /// Return the first-release default policy set from `MEDIA_TRANSCODING.md`.
    #[must_use]
    pub const fn first_release_defaults() -> Self {
        Self {
            video: UnmatchedStreamPolicy::Reject,
            audio: UnmatchedStreamPolicy::Preserve,
            subtitle: UnmatchedStreamPolicy::Preserve,
            attachment: UnmatchedStreamPolicy::Preserve,
            data: UnmatchedStreamPolicy::Remove,
        }
    }

    /// Apply one legacy policy value to every authored stream family.
    #[must_use]
    pub const fn from_single(policy: UnmatchedStreamPolicy) -> Self {
        Self {
            video: policy,
            audio: policy,
            subtitle: policy,
            attachment: policy,
            data: policy,
        }
    }

    const fn for_kind(self, kind: StreamKind) -> UnmatchedStreamPolicy {
        match kind {
            StreamKind::Video => self.video,
            StreamKind::Audio => self.audio,
            StreamKind::Subtitle => self.subtitle,
            StreamKind::Attachment => self.attachment,
            StreamKind::Data => self.data,
            StreamKind::Chapter => UnmatchedStreamPolicy::Remove,
        }
    }
}

impl Default for UnmatchedStreamPolicies {
    fn default() -> Self {
        Self::first_release_defaults()
    }
}

/// Desired-target validation or matching failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TargetCompileError {
    /// Target identity is blank or its version is zero.
    #[error("desired target identity is invalid")]
    InvalidTargetIdentity,
    /// Target container is blank.
    #[error("desired target container is empty")]
    EmptyContainer,
    /// The target requested an unsupported container metadata policy.
    #[error("unsupported desired target container metadata policy: {0}")]
    UnsupportedContainerMetadataPolicy(String),
    /// A desired container metadata key is blank.
    #[error("desired target container metadata key is empty")]
    EmptyContainerMetadataKey,
    /// A desired container metadata value is blank.
    #[error("desired target container metadata value is empty: {0}")]
    EmptyContainerMetadataValue(String),
    /// A desired container metadata key occurs more than once.
    #[error("duplicate desired target container metadata key: {0}")]
    DuplicateContainerMetadataKey(String),
    /// Desired container metadata rows are incompatible with the selected metadata policy.
    #[error("desired target container metadata rows require replace policy")]
    ContainerMetadataRequiresReplacePolicy,
    /// Replace policy requires at least one desired container metadata row.
    #[error("desired target container metadata replace policy requires metadata rows")]
    ReplaceContainerMetadataRequiresRows,
    /// A desired target carries too many container metadata rows.
    #[error("desired target container metadata row count exceeds the accepted limit")]
    TooManyContainerMetadataEntries,
    /// A desired container metadata key exceeds the UTF-8 byte limit.
    #[error("desired target container metadata key exceeds the accepted byte limit")]
    ContainerMetadataKeyTooLong,
    /// A desired container metadata value exceeds the UTF-8 byte limit.
    #[error("desired target container metadata value exceeds the accepted byte limit: {0}")]
    ContainerMetadataValueTooLong(String),
    /// Aggregate desired container metadata exceeds the UTF-8 byte limit.
    #[error("desired target container metadata exceeds the accepted aggregate byte limit")]
    ContainerMetadataBytesExceeded,
    /// The target requested an unsupported container chapter policy.
    #[error("unsupported desired target container chapter policy: {0}")]
    UnsupportedContainerChapterPolicy(String),
    /// Desired container chapter rows are incompatible with the selected chapter policy.
    #[error("desired target container chapter rows require replace policy")]
    ContainerChaptersRequireReplacePolicy,
    /// Replace chapter policy requires at least one desired chapter row.
    #[error("desired target container chapter replace policy requires chapter rows")]
    ReplaceContainerChaptersRequiresRows,
    /// A desired container chapter has an invalid time range.
    #[error("desired target container chapter time range is invalid: {start_millis}..{end_millis}")]
    InvalidContainerChapterRange {
        /// Inclusive start in milliseconds.
        start_millis: i64,
        /// Exclusive end in milliseconds.
        end_millis: i64,
    },
    /// A desired container chapter overlaps another desired chapter.
    #[error(
        "desired target container chapter overlaps previous chapter: {start_millis}..{end_millis}"
    )]
    OverlappingContainerChapter {
        /// Inclusive start in milliseconds.
        start_millis: i64,
        /// Exclusive end in milliseconds.
        end_millis: i64,
    },
    /// A desired container chapter metadata key is blank.
    #[error("desired target container chapter metadata key is empty")]
    EmptyContainerChapterMetadataKey,
    /// A desired container chapter metadata value is blank.
    #[error("desired target container chapter metadata value is empty: {0}")]
    EmptyContainerChapterMetadataValue(String),
    /// A desired container chapter metadata key occurs more than once in one chapter.
    #[error("duplicate desired target container chapter metadata key: {0}")]
    DuplicateContainerChapterMetadataKey(String),
    /// A desired target carries too many chapter rows.
    #[error("desired target container chapter count exceeds the accepted limit")]
    TooManyContainerChapters,
    /// A desired chapter carries too many metadata rows.
    #[error("desired target container chapter metadata row count exceeds the accepted limit")]
    TooManyContainerChapterMetadataEntries,
    /// A desired chapter metadata key exceeds the UTF-8 byte limit.
    #[error("desired target container chapter metadata key exceeds the accepted byte limit")]
    ContainerChapterMetadataKeyTooLong,
    /// A desired chapter metadata value exceeds the UTF-8 byte limit.
    #[error("desired target container chapter metadata value exceeds the accepted byte limit: {0}")]
    ContainerChapterMetadataValueTooLong(String),
    /// Aggregate desired chapter metadata exceeds the UTF-8 byte limit.
    #[error("desired target container chapter metadata exceeds the accepted aggregate byte limit")]
    ContainerChapterMetadataBytesExceeded,
    /// The target requested an unsupported container attachment policy.
    #[error("unsupported desired target container attachment policy: {0}")]
    UnsupportedContainerAttachmentPolicy(String),
    /// A target stream key is blank.
    #[error("desired target stream key is empty")]
    EmptyStreamKey,
    /// A target stream key occurs more than once.
    #[error("duplicate desired target stream key: {0}")]
    DuplicateStreamKey(String),
    /// A source binding key is present but empty.
    #[error("desired target source binding key is empty: {0}")]
    EmptySourceBindingKey(String),
    /// A language selector is not one normalized ISO-639-3 token.
    #[error("desired target language is invalid: {0}")]
    InvalidLanguage(String),
    /// Desired-target YAML exceeds the accepted byte limit.
    #[error("desired target YAML payload exceeds the accepted byte limit")]
    YamlPayloadTooLarge,
    /// Desired-target YAML cannot be decoded as the versioned target shape.
    #[error("desired target YAML payload is invalid")]
    InvalidYaml,
    /// A desired target exceeds the accepted stream cardinality.
    #[error("desired target stream count exceeds the accepted limit")]
    TooManyTargetStreams,
    /// A desired target contains no stream contract.
    #[error("desired target must contain at least one stream")]
    EmptyTargetStreams,
    /// Reused source binding does not satisfy all grouped target selectors.
    #[error("desired target source binding is incompatible: {0}")]
    IncompatibleSourceBinding(String),
    /// A target stream codec is blank.
    #[error("desired target stream codec is empty: {0}")]
    EmptyCodec(String),
    /// Audio-only properties were assigned to a non-audio stream.
    #[error("audio shape assigned to non-audio target stream: {0}")]
    AudioShapeOnNonAudioStream(String),
    /// An audio property carries an impossible zero value.
    #[error("invalid desired target audio constraint {field} on stream {stream_key}")]
    InvalidAudioConstraint {
        /// Target stream identity.
        stream_key: String,
        /// Invalid field name.
        field: &'static str,
    },
    /// A target requested an unsupported audio channel layout.
    #[error(
        "unsupported desired target audio channel layout on stream {stream_key}: {channel_layout}"
    )]
    UnsupportedAudioChannelLayout {
        /// Target stream identity.
        stream_key: String,
        /// Requested channel layout.
        channel_layout: String,
    },
    /// A target requested a channel count that conflicts with its channel layout.
    #[error(
        "desired target audio channel count does not match layout on stream {stream_key}: {channels} != {layout_channels}"
    )]
    AudioChannelLayoutCountMismatch {
        /// Target stream identity.
        stream_key: String,
        /// Requested channel count.
        channels: u32,
        /// Channel count required by the requested layout.
        layout_channels: u32,
    },
    /// Video-only properties were assigned to a non-video stream.
    #[error("video shape assigned to non-video target stream: {0}")]
    VideoShapeOnNonVideoStream(String),
    /// A video property carries an impossible zero value.
    #[error("invalid desired target video constraint {field} on stream {stream_key}")]
    InvalidVideoConstraint {
        /// Target stream identity.
        stream_key: String,
        /// Invalid field name.
        field: &'static str,
    },
    /// The target requested an HDR format without an implemented verification contract.
    #[error("unsupported desired target HDR format on stream {stream_key}: {hdr_format}")]
    UnsupportedHdrFormat {
        /// Target stream identity.
        stream_key: String,
        /// Requested HDR format.
        hdr_format: String,
    },
    /// The target requested a video level without a supported codec contract.
    #[error("unsupported desired target video level on stream {stream_key}: {codec} {video_level}")]
    UnsupportedVideoLevel {
        /// Target stream identity.
        stream_key: String,
        /// Requested video codec.
        codec: String,
        /// Requested video level.
        video_level: String,
    },
    /// The target requested a video color value outside the verified FFmpeg/ffprobe contract.
    #[error("unsupported desired target video color {field} on stream {stream_key}: {value}")]
    UnsupportedVideoColor {
        /// Target stream identity.
        stream_key: String,
        /// Requested color field.
        field: &'static str,
        /// Requested color value.
        value: String,
    },
    /// Subtitle-only properties were assigned to another stream kind.
    #[error("subtitle shape assigned to non-subtitle target stream: {0}")]
    SubtitleShapeOnNonSubtitleStream(String),
    /// The desired stream kind is retained by inspection but not yet accepted in target contracts.
    #[error("unsupported desired target stream kind: {0}")]
    UnsupportedDesiredStreamKind(String),
    /// A subtitle target uses an audio-only semantic role.
    #[error("subtitle target uses an invalid semantic role: {0}")]
    InvalidSubtitleRole(String),
    /// The graph-only compiler cannot materialize a sidecar output.
    #[error("subtitle placement requires sidecar-aware compilation: {0}")]
    SidecarPlacementRequiresArtifactPlan(String),
    /// A sidecar output path cannot be derived from the managed media output path.
    #[error("sidecar output path cannot be derived: {0}")]
    InvalidSidecarOutputPath(String),
    /// Two target rows resolve to the same sidecar output identity.
    #[error("duplicate desired sidecar output path: {0}")]
    DuplicateSidecarOutputPath(String),
    /// A desired subtitle codec has no supported sidecar representation.
    #[error("desired subtitle codec has no supported sidecar format: {0}")]
    UnsupportedSidecarCodec(String),
    /// A synthetic external-input stream id cannot be allocated.
    #[error("sidecar embedding stream identity overflow")]
    SidecarStreamIdentityOverflow,
    /// A desired output stream identity cannot be allocated.
    #[error("desired output stream identity overflow")]
    DesiredStreamIdentityOverflow,
    /// Image-subtitle policy rejected a selected source.
    #[error("image subtitle policy rejected selected source: {0}")]
    ImageSubtitleRejected(String),
    /// Image-subtitle conversion would require OCR.
    #[error("image subtitle conversion is not allowed: {0}")]
    ImageSubtitleConversionNotAllowed(String),
    /// A required target stream had no deterministic source match.
    #[error("required desired target stream has no source match: {0}")]
    RequiredStreamMissing(String),
    /// Policy rejects source streams not selected by the target.
    #[error("source stream is not selected by desired target: {0}")]
    UnmatchedSourceStream(u32),
    /// Policy rejects an adjacent sidecar not selected by the target.
    #[error("sidecar subtitle is not selected by desired target: {0}")]
    UnmatchedSidecar(String),
}

/// Parse and validate one bounded desired-target YAML document.
///
/// # Errors
///
/// Returns a [`TargetCompileError`] when the payload exceeds its byte or stream bound, cannot be
/// decoded, or violates target semantics.
pub fn parse_desired_target_yaml(yaml_payload: &str) -> Result<DesiredTarget, TargetCompileError> {
    if yaml_payload.len() > MAX_DESIRED_TARGET_YAML_BYTES {
        return Err(TargetCompileError::YamlPayloadTooLarge);
    }
    let target: DesiredTarget =
        serde_yaml::from_str(yaml_payload).map_err(|_| TargetCompileError::InvalidYaml)?;
    if target.streams.is_empty() {
        return Err(TargetCompileError::EmptyTargetStreams);
    }
    if target.streams.len() > MAX_DESIRED_TARGET_STREAMS {
        return Err(TargetCompileError::TooManyTargetStreams);
    }
    validate_target(&target)?;
    Ok(target)
}

/// Compile an immutable target and an independent unmatched-stream policy into a desired graph.
///
/// Target rows are evaluated in declared order. A row without `source_binding_key` consumes the
/// first still-unmatched source satisfying its selectors. Rows sharing an explicit binding key
/// reuse one selected source while retaining independent desired output identities. Source stream
/// ids are therefore runtime correlation values only; they are never persisted target identity.
///
/// # Errors
///
/// Returns a [`TargetCompileError`] when the target is malformed, a required stream cannot be
/// matched, or the unmatched-stream policy rejects a source stream.
pub fn compile_desired_target(
    source: &MediaGraph,
    output_path: &str,
    target: &DesiredTarget,
    unmatched_policy: UnmatchedStreamPolicy,
) -> Result<DesiredGraph, TargetCompileError> {
    compile_desired_target_with_unmatched_policies(
        source,
        output_path,
        target,
        UnmatchedStreamPolicies::from_single(unmatched_policy),
    )
}

/// Compile an immutable target and per-kind unmatched-stream policies into a desired graph.
///
/// # Errors
///
/// Returns a [`TargetCompileError`] when the target is malformed, a required stream cannot be
/// matched, or unmatched policies reject a source stream.
pub fn compile_desired_target_with_unmatched_policies(
    source: &MediaGraph,
    output_path: &str,
    target: &DesiredTarget,
    unmatched_policies: UnmatchedStreamPolicies,
) -> Result<DesiredGraph, TargetCompileError> {
    validate_target(target)?;
    let container_chapter_policy = normalized_container_chapter_policy(target)?;
    let attachment_policy = normalized_container_attachment_policy(target)?;

    let mut source_bindings = SourceBindingState::default();
    let mut desired_streams = Vec::with_capacity(target.streams.len());
    let mut desired_bindings = Vec::with_capacity(target.streams.len());
    for target_stream in &target.streams {
        if target_stream.kind == StreamKind::Subtitle
            && matches!(
                target_stream.subtitle_placement,
                Some(SubtitlePlacement::Sidecar | SubtitlePlacement::Both)
            )
        {
            return Err(TargetCompileError::SidecarPlacementRequiresArtifactPlan(
                target_stream.stream_key.clone(),
            ));
        }
        match source_bindings.select(source, target_stream)? {
            Some(stream) => {
                if target_stream.subtitle_placement != Some(SubtitlePlacement::None) {
                    let output_stream_id = next_output_id(&desired_streams)?;
                    push_bound_stream(
                        &mut desired_streams,
                        &mut desired_bindings,
                        apply_target_stream(stream, target_stream, output_stream_id),
                        Some(stream.stream_id),
                    );
                }
            }
            None if target_stream.optional => {}
            None => {
                return Err(TargetCompileError::RequiredStreamMissing(
                    target_stream.stream_key.clone(),
                ));
            }
        }
    }

    for stream in source
        .streams
        .iter()
        .filter(|stream| !source_bindings.consumed.contains(&stream.stream_id))
    {
        let stream_policy = unmatched_policies.for_kind(stream.kind);
        match (stream.kind, attachment_policy.as_str(), stream_policy) {
            (StreamKind::Attachment, "strip", _) | (_, _, UnmatchedStreamPolicy::Remove) => {}
            (_, _, UnmatchedStreamPolicy::Preserve) => {
                let mut desired_stream = stream.clone();
                desired_stream.stream_id = next_output_id(&desired_streams)?;
                push_bound_stream(
                    &mut desired_streams,
                    &mut desired_bindings,
                    desired_stream,
                    Some(stream.stream_id),
                );
            }
            (_, _, UnmatchedStreamPolicy::Reject) => {
                return Err(TargetCompileError::UnmatchedSourceStream(stream.stream_id));
            }
        }
    }

    Ok(DesiredGraph {
        output_path: output_path.to_string(),
        container_format: Some(normalize_container_format(&target.container)),
        container_metadata_policy: Some(normalized_container_metadata_policy(target)?),
        container_metadata: desired_container_metadata(source, target)?,
        container_chapters: desired_container_chapters(source, target, &container_chapter_policy)?,
        container_chapter_policy: Some(container_chapter_policy),
        stream_bindings: desired_bindings,
        container_attachment_policy: Some(attachment_policy),
        streams: desired_streams,
    })
}

#[derive(Default)]
struct SourceBindingState {
    consumed: BTreeSet<u32>,
    selected: BTreeMap<String, u32>,
}

impl SourceBindingState {
    fn select<'a>(
        &mut self,
        source: &'a MediaGraph,
        target: &TargetStream,
    ) -> Result<Option<&'a MediaStream>, TargetCompileError> {
        let binding_key = normalized_source_binding_key(target);
        if let Some(stream_id) = self.selected.get(&binding_key) {
            let selected = source
                .streams
                .iter()
                .find(|stream| stream.stream_id == *stream_id)
                .filter(|stream| matches(stream, target))
                .ok_or_else(|| {
                    TargetCompileError::IncompatibleSourceBinding(binding_key.clone())
                })?;
            return Ok(Some(selected));
        }

        let selected = source
            .streams
            .iter()
            .find(|stream| !self.consumed.contains(&stream.stream_id) && matches(stream, target));
        if let Some(stream) = selected {
            self.consumed.insert(stream.stream_id);
            self.selected.insert(binding_key, stream.stream_id);
        }
        Ok(selected)
    }
}

fn normalized_source_binding_key(target: &TargetStream) -> String {
    target
        .source_binding_key
        .as_deref()
        .unwrap_or(&target.stream_key)
        .trim()
        .to_ascii_lowercase()
}

fn next_output_id(streams: &[MediaStream]) -> Result<u32, TargetCompileError> {
    u32::try_from(streams.len()).map_err(|_| TargetCompileError::DesiredStreamIdentityOverflow)
}

fn push_bound_stream(
    streams: &mut Vec<MediaStream>,
    bindings: &mut Vec<DesiredStreamBinding>,
    stream: MediaStream,
    source_stream_id: Option<u32>,
) {
    bindings.push(DesiredStreamBinding {
        output_stream_id: stream.stream_id,
        source_stream_id,
    });
    streams.push(stream);
}

/// Compile the primary graph and every required subtitle artifact from embedded and sidecar inputs.
///
/// Target rows remain ordered and consume embedded streams before sidecars when both match. Image
/// subtitles are never converted through OCR: `preserve` requires an identical codec, `remove`
/// consumes the selected source without output, and `fail` rejects compilation.
///
/// # Errors
///
/// Returns [`TargetCompileError`] when the target is malformed, a required source is missing,
/// sidecar output identity is invalid or duplicated, stream identity overflows, or image policy
/// cannot be honored without OCR.
pub fn compile_desired_target_with_sidecars(
    source: &MediaGraph,
    output_path: &str,
    target: &DesiredTarget,
    unmatched_policy: UnmatchedStreamPolicy,
    sidecars: &[SidecarSubtitleInput],
) -> Result<CompiledDesiredTarget, TargetCompileError> {
    compile_desired_target_with_sidecars_at(
        source,
        output_path,
        output_path,
        target,
        unmatched_policy,
        sidecars,
    )
}

/// Compile a desired target with distinct workspace and final sidecar locations.
///
/// # Errors
///
/// Returns a [`TargetCompileError`] when the target or an artifact path is invalid.
pub fn compile_desired_target_with_sidecars_at(
    source: &MediaGraph,
    output_path: &str,
    destination_media_path: &str,
    target: &DesiredTarget,
    unmatched_policy: UnmatchedStreamPolicy,
    sidecars: &[SidecarSubtitleInput],
) -> Result<CompiledDesiredTarget, TargetCompileError> {
    compile_desired_target_with_sidecars_at_and_unmatched_policies(
        source,
        output_path,
        destination_media_path,
        target,
        UnmatchedStreamPolicies::from_single(unmatched_policy),
        sidecars,
    )
}

/// Compile a desired target with distinct workspace and final sidecar locations and per-kind
/// unmatched-stream policies.
///
/// # Errors
///
/// Returns a [`TargetCompileError`] when the target or an artifact path is invalid.
pub fn compile_desired_target_with_sidecars_at_and_unmatched_policies(
    source: &MediaGraph,
    output_path: &str,
    destination_media_path: &str,
    target: &DesiredTarget,
    unmatched_policies: UnmatchedStreamPolicies,
    sidecars: &[SidecarSubtitleInput],
) -> Result<CompiledDesiredTarget, TargetCompileError> {
    validate_target(target)?;
    let mut source_bindings = SourceBindingState::default();
    let container_chapter_policy = normalized_container_chapter_policy(target)?;
    let mut consumed_sidecars = BTreeSet::new();
    let mut state = TargetCompilationState::new(target.streams.len());

    for target_stream in &target.streams {
        if target_stream.kind != StreamKind::Subtitle {
            compile_primary_stream(source, target_stream, &mut source_bindings, &mut state)?;
            continue;
        }

        let embedded = source_bindings.select(source, target_stream)?;
        let sidecar = sidecars.iter().enumerate().find(|(index, sidecar)| {
            !consumed_sidecars.contains(index) && sidecar_matches(sidecar, target_stream)
        });
        match (embedded, sidecar) {
            (Some(stream), _) => {
                compile_embedded_subtitle(
                    output_path,
                    destination_media_path,
                    stream,
                    target_stream,
                    &mut state,
                )?;
            }
            (None, Some((index, sidecar))) => {
                consumed_sidecars.insert(index);
                compile_sidecar_subtitle(
                    output_path,
                    destination_media_path,
                    sidecar,
                    target_stream,
                    &mut state,
                )?;
            }
            (None, None) if target_stream.optional => {}
            (None, None) => {
                return Err(TargetCompileError::RequiredStreamMissing(
                    target_stream.stream_key.clone(),
                ));
            }
        }
    }

    append_unmatched_streams(
        source,
        unmatched_policies,
        &normalized_container_attachment_policy(target)?,
        &source_bindings.consumed,
        &mut state,
    )?;
    append_unmatched_sidecars(
        sidecars,
        unmatched_policies.subtitle,
        &consumed_sidecars,
        &mut state.removals,
    )?;
    Ok(CompiledDesiredTarget {
        graph: DesiredGraph {
            output_path: output_path.to_string(),
            container_format: Some(normalize_container_format(&target.container)),
            container_metadata_policy: Some(normalized_container_metadata_policy(target)?),
            container_metadata: desired_container_metadata(source, target)?,
            container_chapters: desired_container_chapters(
                source,
                target,
                &container_chapter_policy,
            )?,
            container_chapter_policy: Some(container_chapter_policy),
            stream_bindings: state.desired_bindings,
            container_attachment_policy: Some(normalized_container_attachment_policy(target)?),
            streams: state.desired_streams,
        },
        sidecar_embeddings: state.embeddings,
        sidecar_outputs: state.outputs,
        sidecar_removals: state.removals.into_iter().collect(),
    })
}

struct TargetCompilationState {
    desired_streams: Vec<MediaStream>,
    desired_bindings: Vec<DesiredStreamBinding>,
    embeddings: Vec<SidecarEmbedding>,
    outputs: Vec<DesiredSidecarOutput>,
    removals: BTreeSet<String>,
    output_paths: BTreeSet<String>,
}

impl TargetCompilationState {
    fn new(stream_capacity: usize) -> Self {
        Self {
            desired_streams: Vec::with_capacity(stream_capacity),
            desired_bindings: Vec::with_capacity(stream_capacity),
            embeddings: Vec::new(),
            outputs: Vec::new(),
            removals: BTreeSet::new(),
            output_paths: BTreeSet::new(),
        }
    }
}

fn compile_primary_stream(
    source: &MediaGraph,
    target: &TargetStream,
    source_bindings: &mut SourceBindingState,
    state: &mut TargetCompilationState,
) -> Result<(), TargetCompileError> {
    match source_bindings.select(source, target)? {
        Some(stream) => {
            state.push_stream(
                apply_target_stream(stream, target, state.next_output_id()?),
                Some(stream.stream_id),
            );
            Ok(())
        }
        None if target.optional => Ok(()),
        None => Err(TargetCompileError::RequiredStreamMissing(
            target.stream_key.clone(),
        )),
    }
}

impl TargetCompilationState {
    fn next_output_id(&self) -> Result<u32, TargetCompileError> {
        next_output_id(&self.desired_streams)
    }

    fn push_stream(&mut self, stream: MediaStream, source_stream_id: Option<u32>) {
        push_bound_stream(
            &mut self.desired_streams,
            &mut self.desired_bindings,
            stream,
            source_stream_id,
        );
    }
}

fn compile_embedded_subtitle(
    media_output_path: &str,
    destination_media_path: &str,
    source: &MediaStream,
    target: &TargetStream,
    state: &mut TargetCompilationState,
) -> Result<(), TargetCompileError> {
    if remove_image_subtitle(
        is_image_subtitle_codec(&source.codec),
        &source.codec,
        target,
    )? {
        return Ok(());
    }
    let placement = target
        .subtitle_placement
        .unwrap_or(SubtitlePlacement::Embedded);
    if matches!(
        placement,
        SubtitlePlacement::Embedded | SubtitlePlacement::Both
    ) {
        state.push_stream(
            apply_target_stream(source, target, state.next_output_id()?),
            Some(source.stream_id),
        );
    }
    if matches!(
        placement,
        SubtitlePlacement::Sidecar | SubtitlePlacement::Both
    ) {
        state.outputs.push(sidecar_output(
            media_output_path,
            destination_media_path,
            target,
            SidecarOutputSource::EmbeddedStream {
                stream_id: source.stream_id,
            },
            &mut state.output_paths,
        )?);
    }
    Ok(())
}

fn compile_sidecar_subtitle(
    media_output_path: &str,
    destination_media_path: &str,
    source: &SidecarSubtitleInput,
    target: &TargetStream,
    state: &mut TargetCompilationState,
) -> Result<(), TargetCompileError> {
    if remove_image_subtitle(source.image_based, &source.codec, target)? {
        state.removals.insert(source.path.clone());
        if let Some(companion) = &source.companion_path {
            state.removals.insert(companion.clone());
        }
        return Ok(());
    }
    let placement = target
        .subtitle_placement
        .unwrap_or(SubtitlePlacement::Embedded);
    if matches!(
        placement,
        SubtitlePlacement::Embedded | SubtitlePlacement::Both
    ) {
        let output_stream = apply_sidecar_target(source, target, state.next_output_id()?);
        state.push_stream(output_stream.clone(), None);
        state.embeddings.push(SidecarEmbedding {
            path: source.path.clone(),
            companion_path: source.companion_path.clone(),
            output_stream,
            source_codec: source.codec.clone(),
        });
    }
    if matches!(
        placement,
        SubtitlePlacement::Sidecar | SubtitlePlacement::Both
    ) {
        state.outputs.push(sidecar_output(
            media_output_path,
            destination_media_path,
            target,
            SidecarOutputSource::ExistingSidecar {
                path: source.path.clone(),
                companion_path: source.companion_path.clone(),
                codec: source.codec.clone(),
            },
            &mut state.output_paths,
        )?);
    } else {
        state.removals.insert(source.path.clone());
        if let Some(companion) = &source.companion_path {
            state.removals.insert(companion.clone());
        }
    }
    Ok(())
}

fn remove_image_subtitle(
    image_based: bool,
    source_codec: &str,
    target: &TargetStream,
) -> Result<bool, TargetCompileError> {
    if !image_based {
        return Ok(false);
    }
    match target
        .image_subtitle_action
        .unwrap_or(ImageSubtitleAction::Fail)
    {
        ImageSubtitleAction::Remove => Ok(true),
        ImageSubtitleAction::Fail => Err(TargetCompileError::ImageSubtitleRejected(
            target.stream_key.clone(),
        )),
        ImageSubtitleAction::Preserve
            if normalize_subtitle_codec(source_codec)
                == normalize_subtitle_codec(&target.codec) =>
        {
            Ok(false)
        }
        ImageSubtitleAction::Preserve => Err(
            TargetCompileError::ImageSubtitleConversionNotAllowed(target.stream_key.clone()),
        ),
    }
}

fn sidecar_output(
    media_output_path: &str,
    destination_media_path: &str,
    target: &TargetStream,
    source: SidecarOutputSource,
    output_paths: &mut BTreeSet<String>,
) -> Result<DesiredSidecarOutput, TargetCompileError> {
    let extension = sidecar_extension(&target.codec)?;
    let path = derive_sidecar_path(media_output_path, target, extension)?;
    let destination_path = derive_sidecar_path(destination_media_path, target, extension)?;
    if !output_paths.insert(destination_path.clone()) {
        return Err(TargetCompileError::DuplicateSidecarOutputPath(
            destination_path,
        ));
    }
    let companion_path = derive_companion_path(&path, extension);
    let destination_companion_path = derive_companion_path(&destination_path, extension);
    Ok(DesiredSidecarOutput {
        path,
        companion_path,
        destination_path,
        destination_companion_path,
        source,
        codec: target.codec.trim().to_ascii_lowercase(),
    })
}

fn derive_sidecar_path(
    media_output_path: &str,
    target: &TargetStream,
    extension: &str,
) -> Result<String, TargetCompileError> {
    let media_path = Path::new(media_output_path);
    let stem = media_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            TargetCompileError::InvalidSidecarOutputPath(media_output_path.to_string())
        })?;
    let language = target
        .language
        .as_ref()
        .map_or("und", LanguageToken::as_str);
    let role = target.role.and_then(role_path_token);
    let suffix = role.map_or_else(
        || format!("{language}.{extension}"),
        |role| format!("{language}.{role}.{extension}"),
    );
    let file_name = format!("{stem}.{suffix}");
    if !is_single_path_component(&file_name) {
        return Err(TargetCompileError::InvalidSidecarOutputPath(file_name));
    }
    let path = media_path.with_file_name(&file_name);
    if path.parent() != media_path.parent() {
        return Err(TargetCompileError::InvalidSidecarOutputPath(
            path.to_string_lossy().into_owned(),
        ));
    }
    let path = path.to_str().map(str::to_string).ok_or_else(|| {
        TargetCompileError::InvalidSidecarOutputPath(media_output_path.to_string())
    })?;
    Ok(path)
}

fn is_single_path_component(value: &str) -> bool {
    if value.contains(['/', '\\']) {
        return false;
    }
    let mut components = Path::new(value).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn derive_companion_path(path: &str, extension: &str) -> Option<String> {
    (extension == "idx").then(|| {
        Path::new(path)
            .with_extension("sub")
            .to_string_lossy()
            .into_owned()
    })
}

fn append_unmatched_streams(
    source: &MediaGraph,
    policies: UnmatchedStreamPolicies,
    attachment_policy: &str,
    consumed: &BTreeSet<u32>,
    state: &mut TargetCompilationState,
) -> Result<(), TargetCompileError> {
    for stream in source
        .streams
        .iter()
        .filter(|stream| !consumed.contains(&stream.stream_id))
    {
        let stream_policy = policies.for_kind(stream.kind);
        match (stream.kind, attachment_policy, stream_policy) {
            (StreamKind::Attachment, "strip", _) | (_, _, UnmatchedStreamPolicy::Remove) => {}
            (_, _, UnmatchedStreamPolicy::Preserve) => {
                let mut desired_stream = stream.clone();
                desired_stream.stream_id = state.next_output_id()?;
                state.push_stream(desired_stream, Some(stream.stream_id));
            }
            (_, _, UnmatchedStreamPolicy::Reject) => {
                return Err(TargetCompileError::UnmatchedSourceStream(stream.stream_id));
            }
        }
    }
    Ok(())
}

fn append_unmatched_sidecars(
    sidecars: &[SidecarSubtitleInput],
    policy: UnmatchedStreamPolicy,
    consumed: &BTreeSet<usize>,
    removals: &mut BTreeSet<String>,
) -> Result<(), TargetCompileError> {
    for (_, sidecar) in sidecars
        .iter()
        .enumerate()
        .filter(|(index, _)| !consumed.contains(index))
    {
        match policy {
            UnmatchedStreamPolicy::Remove => {
                removals.insert(sidecar.path.clone());
                if let Some(companion) = &sidecar.companion_path {
                    removals.insert(companion.clone());
                }
            }
            UnmatchedStreamPolicy::Preserve => {}
            UnmatchedStreamPolicy::Reject => {
                return Err(TargetCompileError::UnmatchedSidecar(sidecar.path.clone()));
            }
        }
    }
    Ok(())
}

fn sidecar_matches(source: &SidecarSubtitleInput, target: &TargetStream) -> bool {
    target.role.is_none_or(|role| source.role == Some(role))
        && target.language.as_ref().is_none_or(|language| {
            source
                .language
                .as_deref()
                .is_some_and(|source_language| language_matches(source_language, language.as_str()))
        })
}

fn apply_sidecar_target(
    source: &SidecarSubtitleInput,
    target: &TargetStream,
    stream_id: u32,
) -> MediaStream {
    MediaStream {
        stream_id,
        kind: StreamKind::Subtitle,
        codec: target.codec.trim().to_ascii_lowercase(),
        channels: None,
        channel_layout: None,
        language: target
            .language
            .as_ref()
            .map(|language| language.as_str().to_string())
            .or_else(|| source.language.clone()),
        title: target.title.clone(),
        dispositions: normalized_dispositions(&target.dispositions),
    }
}

fn is_image_subtitle_codec(codec: &str) -> bool {
    matches!(
        codec.trim().to_ascii_lowercase().as_str(),
        "dvd_subtitle" | "hdmv_pgs_subtitle" | "pgs" | "vobsub"
    )
}

fn sidecar_extension(codec: &str) -> Result<&'static str, TargetCompileError> {
    match codec.trim().to_ascii_lowercase().as_str() {
        "subrip" | "srt" => Ok("srt"),
        "ass" | "ssa" => Ok("ass"),
        "webvtt" | "vtt" => Ok("vtt"),
        "hdmv_pgs_subtitle" | "pgs" => Ok("sup"),
        "dvd_subtitle" | "vobsub" => Ok("idx"),
        "microdvd" => Ok("sub"),
        _ => Err(TargetCompileError::UnsupportedSidecarCodec(
            codec.trim().to_ascii_lowercase(),
        )),
    }
}

const fn role_path_token(role: SemanticRole) -> Option<&'static str> {
    match role {
        SemanticRole::Forced => Some("forced"),
        SemanticRole::Commentary => Some("commentary"),
        SemanticRole::Sdh => Some("sdh"),
        SemanticRole::SignsSongs => Some("signs_songs"),
        SemanticRole::Karaoke => Some("karaoke"),
        SemanticRole::Primary | SemanticRole::DescriptiveAudio | SemanticRole::Unknown => None,
    }
}

fn validate_target(target: &DesiredTarget) -> Result<(), TargetCompileError> {
    if target.target_key.trim().is_empty() || target.version == 0 {
        return Err(TargetCompileError::InvalidTargetIdentity);
    }
    if target.container.trim().is_empty() {
        return Err(TargetCompileError::EmptyContainer);
    }
    if target.streams.len() > MAX_DESIRED_TARGET_STREAMS {
        return Err(TargetCompileError::TooManyTargetStreams);
    }
    let metadata_policy = normalized_container_metadata_policy(target)?;
    validate_container_metadata_policy(&metadata_policy, &target.container_metadata)?;
    let _metadata = normalized_container_metadata(target)?;
    let chapter_policy = normalized_container_chapter_policy(target)?;
    validate_container_chapter_policy(&chapter_policy, &target.container_chapters)?;
    let _chapters = normalized_container_chapters(target)?;
    let _attachment_policy = normalized_container_attachment_policy(target)?;

    let mut keys = BTreeSet::new();
    for stream in &target.streams {
        let key = stream.stream_key.trim();
        if key.is_empty() {
            return Err(TargetCompileError::EmptyStreamKey);
        }
        if !keys.insert(key.to_ascii_lowercase()) {
            return Err(TargetCompileError::DuplicateStreamKey(key.to_string()));
        }
        validate_target_stream(stream, key)?;
    }
    Ok(())
}

fn validate_container_metadata_policy(
    policy: &str,
    entries: &[ContainerMetadataEntry],
) -> Result<(), TargetCompileError> {
    match policy {
        "replace" if entries.is_empty() => {
            Err(TargetCompileError::ReplaceContainerMetadataRequiresRows)
        }
        "replace" => Ok(()),
        _ if entries.is_empty() => Ok(()),
        _ => Err(TargetCompileError::ContainerMetadataRequiresReplacePolicy),
    }
}

fn normalized_container_metadata_policy(
    target: &DesiredTarget,
) -> Result<String, TargetCompileError> {
    normalize_container_metadata_policy(&target.container_metadata_policy)
        .map(str::to_string)
        .ok_or_else(|| {
            TargetCompileError::UnsupportedContainerMetadataPolicy(
                target.container_metadata_policy.clone(),
            )
        })
}

fn normalized_container_metadata(
    target: &DesiredTarget,
) -> Result<Vec<ContainerMetadataEntry>, TargetCompileError> {
    if target.container_metadata.len() > MAX_CONTAINER_METADATA_ENTRIES {
        return Err(TargetCompileError::TooManyContainerMetadataEntries);
    }
    let mut seen = BTreeSet::new();
    let mut entries = Vec::with_capacity(target.container_metadata.len());
    let mut total_bytes = 0usize;
    for entry in &target.container_metadata {
        let key = entry.key.trim().to_ascii_lowercase();
        if key.is_empty() {
            return Err(TargetCompileError::EmptyContainerMetadataKey);
        }
        if key.len() > MAX_CONTAINER_METADATA_KEY_BYTES {
            return Err(TargetCompileError::ContainerMetadataKeyTooLong);
        }
        if !seen.insert(key.clone()) {
            return Err(TargetCompileError::DuplicateContainerMetadataKey(key));
        }
        let value = entry.value.trim().to_string();
        if value.is_empty() {
            return Err(TargetCompileError::EmptyContainerMetadataValue(key));
        }
        if value.len() > MAX_CONTAINER_METADATA_VALUE_BYTES {
            return Err(TargetCompileError::ContainerMetadataValueTooLong(key));
        }
        total_bytes = total_bytes
            .checked_add(key.len())
            .and_then(|bytes| bytes.checked_add(value.len()))
            .filter(|bytes| *bytes <= MAX_CONTAINER_METADATA_TOTAL_BYTES)
            .ok_or(TargetCompileError::ContainerMetadataBytesExceeded)?;
        entries.push(ContainerMetadataEntry { key, value });
    }
    entries.sort();
    Ok(entries)
}

fn desired_container_metadata(
    source: &MediaGraph,
    target: &DesiredTarget,
) -> Result<Vec<ContainerMetadataEntry>, TargetCompileError> {
    match normalized_container_metadata_policy(target)?.as_str() {
        "preserve" => Ok(normalize_container_metadata(&source.container_metadata)),
        "strip" => Ok(Vec::new()),
        "replace" => normalized_container_metadata(target),
        policy => Err(TargetCompileError::UnsupportedContainerMetadataPolicy(
            policy.to_string(),
        )),
    }
}

fn normalized_container_chapter_policy(
    target: &DesiredTarget,
) -> Result<String, TargetCompileError> {
    normalize_container_chapter_policy(&target.container_chapter_policy)
        .map(str::to_string)
        .ok_or_else(|| {
            TargetCompileError::UnsupportedContainerChapterPolicy(
                target.container_chapter_policy.clone(),
            )
        })
}

fn desired_container_chapters(
    source: &MediaGraph,
    target: &DesiredTarget,
    container_chapter_policy: &str,
) -> Result<Vec<ContainerChapterEntry>, TargetCompileError> {
    match container_chapter_policy {
        "preserve" => Ok(source.container_chapters.clone()),
        "strip" => Ok(Vec::new()),
        "replace" => normalized_container_chapters(target),
        policy => Err(TargetCompileError::UnsupportedContainerChapterPolicy(
            policy.to_string(),
        )),
    }
}

fn normalized_container_attachment_policy(
    target: &DesiredTarget,
) -> Result<String, TargetCompileError> {
    normalize_container_attachment_policy(&target.container_attachment_policy)
        .map(str::to_string)
        .ok_or_else(|| {
            TargetCompileError::UnsupportedContainerAttachmentPolicy(
                target.container_attachment_policy.clone(),
            )
        })
}

fn validate_container_chapter_policy(
    policy: &str,
    chapters: &[ContainerChapterEntry],
) -> Result<(), TargetCompileError> {
    match policy {
        "replace" if chapters.is_empty() => {
            Err(TargetCompileError::ReplaceContainerChaptersRequiresRows)
        }
        "replace" => Ok(()),
        _ if chapters.is_empty() => Ok(()),
        _ => Err(TargetCompileError::ContainerChaptersRequireReplacePolicy),
    }
}

fn normalized_container_chapters(
    target: &DesiredTarget,
) -> Result<Vec<ContainerChapterEntry>, TargetCompileError> {
    if target.container_chapters.len() > MAX_CONTAINER_CHAPTERS {
        return Err(TargetCompileError::TooManyContainerChapters);
    }
    let mut chapters = Vec::with_capacity(target.container_chapters.len());
    let mut total_metadata_bytes = 0_usize;
    for chapter in &target.container_chapters {
        if chapter.start_millis < 0 || chapter.end_millis <= chapter.start_millis {
            return Err(TargetCompileError::InvalidContainerChapterRange {
                start_millis: chapter.start_millis,
                end_millis: chapter.end_millis,
            });
        }
        chapters.push(ContainerChapterEntry {
            start_millis: chapter.start_millis,
            end_millis: chapter.end_millis,
            metadata: normalized_container_chapter_metadata(
                &chapter.metadata,
                &mut total_metadata_bytes,
            )?,
        });
    }
    chapters.sort_by(|left, right| {
        left.start_millis
            .cmp(&right.start_millis)
            .then(left.end_millis.cmp(&right.end_millis))
    });
    let mut previous_end = None;
    for chapter in &chapters {
        if previous_end.is_some_and(|end| chapter.start_millis < end) {
            return Err(TargetCompileError::OverlappingContainerChapter {
                start_millis: chapter.start_millis,
                end_millis: chapter.end_millis,
            });
        }
        previous_end = Some(chapter.end_millis);
    }
    Ok(chapters)
}

fn normalized_container_chapter_metadata(
    metadata: &[ContainerMetadataEntry],
    total_bytes: &mut usize,
) -> Result<Vec<ContainerMetadataEntry>, TargetCompileError> {
    if metadata.len() > MAX_CONTAINER_CHAPTER_METADATA_ENTRIES {
        return Err(TargetCompileError::TooManyContainerChapterMetadataEntries);
    }
    let mut seen = BTreeSet::new();
    let mut entries = Vec::with_capacity(metadata.len());
    for entry in metadata {
        let key = entry.key.trim().to_ascii_lowercase();
        if key.is_empty() {
            return Err(TargetCompileError::EmptyContainerChapterMetadataKey);
        }
        if key.len() > MAX_CONTAINER_METADATA_KEY_BYTES {
            return Err(TargetCompileError::ContainerChapterMetadataKeyTooLong);
        }
        if !seen.insert(key.clone()) {
            return Err(TargetCompileError::DuplicateContainerChapterMetadataKey(
                key,
            ));
        }
        let value = entry.value.trim().to_string();
        if value.is_empty() {
            return Err(TargetCompileError::EmptyContainerChapterMetadataValue(key));
        }
        if value.len() > MAX_CONTAINER_METADATA_VALUE_BYTES {
            return Err(TargetCompileError::ContainerChapterMetadataValueTooLong(
                key,
            ));
        }
        *total_bytes = total_bytes
            .checked_add(key.len())
            .and_then(|bytes| bytes.checked_add(value.len()))
            .filter(|bytes| *bytes <= MAX_CONTAINER_CHAPTER_METADATA_TOTAL_BYTES)
            .ok_or(TargetCompileError::ContainerChapterMetadataBytesExceeded)?;
        entries.push(ContainerMetadataEntry { key, value });
    }
    entries.sort();
    Ok(entries)
}

fn validate_target_stream(stream: &TargetStream, key: &str) -> Result<(), TargetCompileError> {
    if let Some(language) = &stream.language {
        LanguageToken::parse(language.as_str())?;
    }
    if stream
        .source_binding_key
        .as_deref()
        .is_some_and(|binding| binding.trim().is_empty())
    {
        return Err(TargetCompileError::EmptySourceBindingKey(key.to_string()));
    }
    if stream.codec.trim().is_empty() {
        return Err(TargetCompileError::EmptyCodec(key.to_string()));
    }

    match stream.kind {
        StreamKind::Audio => validate_audio_target_stream(stream, key),
        StreamKind::Video => validate_video_target_stream(stream, key),
        StreamKind::Subtitle => validate_subtitle_target_stream(stream, key),
        StreamKind::Attachment | StreamKind::Chapter | StreamKind::Data => Err(
            TargetCompileError::UnsupportedDesiredStreamKind(key.to_string()),
        ),
    }
}

fn validate_audio_target_stream(
    stream: &TargetStream,
    key: &str,
) -> Result<(), TargetCompileError> {
    if has_invalid_audio_policy(stream) {
        return Err(TargetCompileError::AudioShapeOnNonAudioStream(
            key.to_string(),
        ));
    }
    validate_audio_constraints(stream, key)?;
    if has_video_shape(stream) {
        return Err(TargetCompileError::VideoShapeOnNonVideoStream(
            key.to_string(),
        ));
    }
    if has_subtitle_shape(stream) {
        return Err(TargetCompileError::SubtitleShapeOnNonSubtitleStream(
            key.to_string(),
        ));
    }
    Ok(())
}

fn validate_video_target_stream(
    stream: &TargetStream,
    key: &str,
) -> Result<(), TargetCompileError> {
    if has_audio_shape(stream) {
        return Err(TargetCompileError::AudioShapeOnNonAudioStream(
            key.to_string(),
        ));
    }
    if has_subtitle_shape(stream) {
        return Err(TargetCompileError::SubtitleShapeOnNonSubtitleStream(
            key.to_string(),
        ));
    }
    validate_video_constraints(stream, key)?;
    validate_video_hdr_format(stream, key)?;
    validate_video_level(stream, key)?;
    validate_video_color_value(key, "color_primaries", stream.color_primaries.as_deref())?;
    validate_video_color_value(key, "color_transfer", stream.color_transfer.as_deref())?;
    validate_video_color_value(key, "color_space", stream.color_space.as_deref())
}

fn validate_subtitle_target_stream(
    stream: &TargetStream,
    key: &str,
) -> Result<(), TargetCompileError> {
    if has_audio_shape(stream) {
        return Err(TargetCompileError::AudioShapeOnNonAudioStream(
            key.to_string(),
        ));
    }
    if has_video_shape(stream) {
        return Err(TargetCompileError::VideoShapeOnNonVideoStream(
            key.to_string(),
        ));
    }
    if stream.role == Some(SemanticRole::DescriptiveAudio) {
        return Err(TargetCompileError::InvalidSubtitleRole(key.to_string()));
    }
    Ok(())
}

fn validate_audio_constraints(stream: &TargetStream, key: &str) -> Result<(), TargetCompileError> {
    if stream.channels == Some(0) {
        return Err(TargetCompileError::InvalidAudioConstraint {
            stream_key: key.to_string(),
            field: "channels",
        });
    }
    if stream.audio_bitrate_bps == Some(0) {
        return Err(TargetCompileError::InvalidAudioConstraint {
            stream_key: key.to_string(),
            field: "audio_bitrate_bps",
        });
    }
    if stream.audio_sample_rate_hz == Some(0) {
        return Err(TargetCompileError::InvalidAudioConstraint {
            stream_key: key.to_string(),
            field: "audio_sample_rate_hz",
        });
    }
    if let Some(layout) = stream.channel_layout.as_deref() {
        let canonical_layout = normalize_audio_channel_layout(layout).ok_or_else(|| {
            TargetCompileError::UnsupportedAudioChannelLayout {
                stream_key: key.to_string(),
                channel_layout: layout.to_string(),
            }
        })?;
        let Some(layout_channels) = audio_channel_count_for_layout(canonical_layout) else {
            return Err(TargetCompileError::UnsupportedAudioChannelLayout {
                stream_key: key.to_string(),
                channel_layout: layout.to_string(),
            });
        };
        if let Some(channels) = stream.channels
            && channels != layout_channels
        {
            return Err(TargetCompileError::AudioChannelLayoutCountMismatch {
                stream_key: key.to_string(),
                channels,
                layout_channels,
            });
        }
    }
    Ok(())
}

fn validate_video_constraints(stream: &TargetStream, key: &str) -> Result<(), TargetCompileError> {
    if stream.video_bitrate_bps == Some(0) {
        return Err(TargetCompileError::InvalidVideoConstraint {
            stream_key: key.to_string(),
            field: "video_bitrate_bps",
        });
    }
    Ok(())
}

fn validate_video_hdr_format(stream: &TargetStream, key: &str) -> Result<(), TargetCompileError> {
    let Some(hdr_format) = stream.hdr_format.as_deref() else {
        return Ok(());
    };
    if hdr_format.trim().eq_ignore_ascii_case("hdr10") {
        return Ok(());
    }
    Err(TargetCompileError::UnsupportedHdrFormat {
        stream_key: key.to_string(),
        hdr_format: hdr_format.to_string(),
    })
}

fn validate_video_level(stream: &TargetStream, key: &str) -> Result<(), TargetCompileError> {
    let Some(video_level) = stream.video_level.as_deref() else {
        return Ok(());
    };
    if is_known_video_level(&stream.codec, video_level) {
        return Ok(());
    }
    Err(TargetCompileError::UnsupportedVideoLevel {
        stream_key: key.to_string(),
        codec: stream.codec.clone(),
        video_level: video_level.to_string(),
    })
}

fn validate_video_color_value(
    stream_key: &str,
    field: &'static str,
    value: Option<&str>,
) -> Result<(), TargetCompileError> {
    let Some(value) = value else {
        return Ok(());
    };
    let normalized = value.trim().to_ascii_lowercase();
    let known = match field {
        "color_primaries" => is_known_color_primaries(&normalized),
        "color_transfer" => is_known_color_transfer(&normalized),
        "color_space" => is_known_color_space(&normalized),
        _ => false,
    };
    if known {
        return Ok(());
    }
    Err(TargetCompileError::UnsupportedVideoColor {
        stream_key: stream_key.to_string(),
        field,
        value: value.to_string(),
    })
}

const fn has_audio_shape(stream: &TargetStream) -> bool {
    stream.channels.is_some()
        || stream.channel_layout.is_some()
        || stream.audio_bitrate_bps.is_some()
        || stream.audio_sample_rate_hz.is_some()
        || stream.audio_loudness_profile.is_some()
        || stream.audio_dynamic_range.is_some()
}

const fn has_video_shape(stream: &TargetStream) -> bool {
    stream.video_profile.is_some()
        || stream.video_level.is_some()
        || stream.video_bitrate_bps.is_some()
        || stream.color_primaries.is_some()
        || stream.color_transfer.is_some()
        || stream.color_space.is_some()
        || stream.hdr_format.is_some()
}

const fn has_subtitle_shape(stream: &TargetStream) -> bool {
    stream.subtitle_placement.is_some() || stream.image_subtitle_action.is_some()
}

fn has_invalid_audio_policy(stream: &TargetStream) -> bool {
    stream
        .audio_loudness_profile
        .as_deref()
        .is_some_and(is_unknown_loudness_profile)
        || stream
            .audio_dynamic_range
            .as_deref()
            .is_some_and(is_unknown_dynamic_range)
}

fn is_unknown_loudness_profile(value: &str) -> bool {
    !value.trim().eq_ignore_ascii_case("dialog-normalized")
}

fn is_unknown_dynamic_range(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "preserve" | "speech"
    )
}

/// Return whether a codec/level pair is supported by the target compiler.
#[must_use]
pub fn is_known_video_level(codec: &str, level: &str) -> bool {
    let Some(level) = normalized_video_level(level) else {
        return false;
    };
    match normalized_video_codec(codec).as_str() {
        "h264" => matches!(
            level,
            NormalizedVideoLevel::H264Level1b
                | NormalizedVideoLevel::Number(
                    10 | 11
                        | 12
                        | 13
                        | 20
                        | 21
                        | 22
                        | 30
                        | 31
                        | 32
                        | 40
                        | 41
                        | 42
                        | 50
                        | 51
                        | 52
                        | 60
                        | 61
                        | 62
                )
        ),
        "hevc" => matches!(
            level,
            NormalizedVideoLevel::Number(
                10 | 20 | 21 | 30 | 31 | 40 | 41 | 50 | 51 | 52 | 60 | 61 | 62
            )
        ),
        "av1" => matches!(
            level,
            NormalizedVideoLevel::Number(
                20 | 21
                    | 22
                    | 23
                    | 30
                    | 31
                    | 32
                    | 33
                    | 40
                    | 41
                    | 42
                    | 43
                    | 50
                    | 51
                    | 52
                    | 53
                    | 60
                    | 61
                    | 62
                    | 63
                    | 70
                    | 71
                    | 72
                    | 73
            )
        ),
        _ => false,
    }
}

fn normalized_video_codec(codec: &str) -> String {
    match codec.trim().to_ascii_lowercase().as_str() {
        "avc" | "avc1" | "libx264" | "x264" => "h264".to_string(),
        "h265" | "libx265" | "x265" => "hevc".to_string(),
        "av01" | "libaom-av1" | "librav1e" | "libsvtav1" | "libsvt-av1" => "av1".to_string(),
        normalized => normalized.to_string(),
    }
}

/// Return whether a video color-primaries label is supported by the target compiler.
#[must_use]
pub fn is_known_color_primaries(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    matches!(
        value.as_str(),
        "bt709"
            | "bt470m"
            | "bt470bg"
            | "smpte170m"
            | "smpte240m"
            | "film"
            | "bt2020"
            | "smpte428"
            | "smpte431"
            | "smpte432"
            | "ebu3213"
    )
}

/// Return whether a video transfer-characteristic label is supported by the target compiler.
#[must_use]
pub fn is_known_color_transfer(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    matches!(
        value.as_str(),
        "bt709"
            | "bt470m"
            | "bt470bg"
            | "smpte170m"
            | "smpte240m"
            | "linear"
            | "log"
            | "log_sqrt"
            | "iec61966-2-4"
            | "bt1361e"
            | "iec61966-2-1"
            | "bt2020-10"
            | "bt2020-12"
            | "smpte2084"
            | "smpte428"
            | "arib-std-b67"
    )
}

/// Return whether a video color-space label is supported by the target compiler.
#[must_use]
pub fn is_known_color_space(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    matches!(
        value.as_str(),
        "gbr"
            | "bt709"
            | "fcc"
            | "bt470bg"
            | "smpte170m"
            | "smpte240m"
            | "ycgco"
            | "bt2020nc"
            | "bt2020c"
            | "smpte2085"
            | "chroma-derived-nc"
            | "chroma-derived-c"
            | "ictcp"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NormalizedVideoLevel {
    Number(u16),
    H264Level1b,
}

fn normalized_video_level(level: &str) -> Option<NormalizedVideoLevel> {
    let candidate = level.trim().to_ascii_lowercase();
    if candidate == "1b" {
        return Some(NormalizedVideoLevel::H264Level1b);
    }
    if let Some((major, minor)) = candidate.split_once('.') {
        if major.is_empty() || minor.len() != 1 || minor.contains('.') {
            return None;
        }
        let major = major.parse::<u16>().ok()?;
        let minor = minor.parse::<u16>().ok()?;
        return major
            .checked_mul(10)?
            .checked_add(minor)
            .map(NormalizedVideoLevel::Number);
    }
    let parsed = candidate.parse::<u16>().ok()?;
    if candidate.len() == 1 {
        parsed.checked_mul(10).map(NormalizedVideoLevel::Number)
    } else {
        Some(NormalizedVideoLevel::Number(parsed))
    }
}

fn matches(source: &MediaStream, target: &TargetStream) -> bool {
    source.kind == target.kind
        && target
            .role
            .is_none_or(|required_role| infer_role(source) == required_role)
        && target.language.as_ref().is_none_or(|language| {
            source
                .language
                .as_deref()
                .is_some_and(|source_language| language_matches(source_language, language.as_str()))
        })
}

fn language_matches(source: &str, target: &str) -> bool {
    source.trim().eq_ignore_ascii_case(target.trim())
}

fn apply_target_stream(
    source: &MediaStream,
    target: &TargetStream,
    output_stream_id: u32,
) -> MediaStream {
    MediaStream {
        stream_id: output_stream_id,
        kind: source.kind,
        codec: target.codec.trim().to_ascii_lowercase(),
        channels: target.channels,
        channel_layout: target
            .channel_layout
            .as_deref()
            .and_then(normalize_audio_channel_layout)
            .map(str::to_string),
        language: target
            .language
            .as_ref()
            .map(|language| language.as_str().to_string())
            .or_else(|| source.language.clone()),
        title: target.title.clone(),
        dispositions: normalized_dispositions(&target.dispositions),
    }
}

fn normalized_dispositions(dispositions: &[String]) -> Vec<String> {
    dispositions
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        DesiredTarget, ImageSubtitleAction, LanguageToken, MAX_CONTAINER_CHAPTER_METADATA_ENTRIES,
        MAX_CONTAINER_CHAPTERS, MAX_CONTAINER_METADATA_ENTRIES, MAX_CONTAINER_METADATA_KEY_BYTES,
        MAX_CONTAINER_METADATA_VALUE_BYTES, SidecarOutputSource, SidecarSubtitleInput,
        SubtitlePlacement, TargetCompileError, TargetStream, UnmatchedStreamPolicies,
        UnmatchedStreamPolicy, compile_desired_target, compile_desired_target_with_sidecars,
        compile_desired_target_with_sidecars_at, compile_desired_target_with_unmatched_policies,
        is_single_path_component, parse_desired_target_yaml,
    };
    use crate::classify::SemanticRole;
    use crate::model::{
        ContainerChapterEntry, ContainerMetadataEntry, MediaGraph, MediaStream, StreamKind,
    };
    use std::path::Path;

    fn stream(
        stream_id: u32,
        kind: StreamKind,
        codec: &str,
        language: Option<&str>,
        title: Option<&str>,
        dispositions: &[&str],
    ) -> MediaStream {
        MediaStream {
            stream_id,
            kind,
            codec: codec.to_string(),
            channels: (kind == StreamKind::Audio).then_some(6),
            channel_layout: (kind == StreamKind::Audio).then(|| "5.1".to_string()),
            language: language.map(str::to_string),
            title: title.map(str::to_string),
            dispositions: dispositions
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        }
    }

    fn target_stream(
        key: &str,
        kind: StreamKind,
        role: Option<SemanticRole>,
        language: Option<&str>,
        codec: &str,
    ) -> TargetStream {
        TargetStream {
            stream_key: key.to_string(),
            kind,
            role,
            language: language.map(|value| LanguageToken(value.to_string())),
            source_binding_key: None,
            optional: false,
            codec: codec.to_string(),
            channels: None,
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
            title: None,
            dispositions: Vec::new(),
            subtitle_placement: (kind == StreamKind::Subtitle)
                .then_some(SubtitlePlacement::Embedded),
            image_subtitle_action: (kind == StreamKind::Subtitle)
                .then_some(ImageSubtitleAction::Fail),
        }
    }

    fn multistream_source() -> MediaGraph {
        MediaGraph {
            source_path: "/input/episode.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![
                stream(
                    7,
                    StreamKind::Audio,
                    "aac",
                    Some("eng"),
                    Some("Main"),
                    &["default"],
                ),
                stream(
                    3,
                    StreamKind::Video,
                    "h264",
                    None,
                    Some("Main"),
                    &["default"],
                ),
                stream(
                    11,
                    StreamKind::Subtitle,
                    "ass",
                    Some("eng"),
                    Some("Full"),
                    &[],
                ),
                stream(5, StreamKind::Video, "mjpeg", None, Some("Cover"), &[]),
                stream(
                    9,
                    StreamKind::Audio,
                    "aac",
                    Some("eng"),
                    Some("Director Commentary"),
                    &[],
                ),
                stream(
                    13,
                    StreamKind::Subtitle,
                    "subrip",
                    Some("eng"),
                    None,
                    &["forced"],
                ),
            ],
        }
    }

    fn ordered_multistream_target() -> DesiredTarget {
        let mut main_audio = target_stream(
            "audio-main",
            StreamKind::Audio,
            Some(SemanticRole::Primary),
            Some("eng"),
            "opus",
        );
        main_audio.channels = Some(2);
        main_audio.channel_layout = Some("stereo".to_string());
        main_audio.title = Some("English".to_string());
        main_audio.dispositions = vec!["default".to_string()];
        let mut forced = target_stream(
            "subtitle-forced",
            StreamKind::Subtitle,
            Some(SemanticRole::Forced),
            Some("eng"),
            "webvtt",
        );
        forced.dispositions = vec!["forced".to_string()];
        DesiredTarget {
            target_key: "web-playback".to_string(),
            version: 4,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![
                target_stream("video-cover", StreamKind::Video, None, None, "png"),
                main_audio,
                target_stream("video-main", StreamKind::Video, None, None, "av1"),
                target_stream(
                    "audio-commentary",
                    StreamKind::Audio,
                    Some(SemanticRole::Commentary),
                    Some("eng"),
                    "aac",
                ),
                forced,
            ],
        }
    }

    #[test]
    fn compiles_ordered_multistream_target_without_durable_source_indexes()
    -> Result<(), TargetCompileError> {
        let source = multistream_source();
        let mut target = ordered_multistream_target();
        target.container = " MKV ".to_string();

        let desired = compile_desired_target(
            &source,
            "/output/episode.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
        )?;

        assert_eq!(desired.container_format.as_deref(), Some("matroska"));
        assert_eq!(
            desired.container_metadata_policy.as_deref(),
            Some("preserve")
        );

        assert_eq!(
            desired
                .streams
                .iter()
                .map(|stream| (stream.stream_id, stream.codec.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (0, "png"),
                (1, "opus"),
                (2, "av1"),
                (3, "aac"),
                (4, "webvtt")
            ]
        );
        assert_eq!(
            desired
                .stream_bindings
                .iter()
                .map(|binding| (binding.output_stream_id, binding.source_stream_id))
                .collect::<Vec<_>>(),
            vec![
                (0, Some(3)),
                (1, Some(7)),
                (2, Some(5)),
                (3, Some(9)),
                (4, Some(13))
            ]
        );
        assert_eq!(desired.streams[1].channels, Some(2));
        assert_eq!(desired.streams[1].channel_layout.as_deref(), Some("stereo"));
        assert_eq!(desired.streams[4].dispositions, vec!["forced"]);
        Ok(())
    }

    #[test]
    fn compiles_strip_container_metadata_policy() -> Result<(), TargetCompileError> {
        let source = multistream_source();
        let mut target = ordered_multistream_target();
        target.container_metadata_policy = " Strip ".to_string();

        let desired = compile_desired_target(
            &source,
            "/output/episode.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
        )?;

        assert_eq!(desired.container_metadata_policy.as_deref(), Some("strip"));
        assert!(desired.container_metadata.is_empty());
        Ok(())
    }

    #[test]
    fn compiles_preserved_container_metadata_values() -> Result<(), TargetCompileError> {
        let mut source = multistream_source();
        source.container_metadata = vec![ContainerMetadataEntry {
            key: " TITLE ".to_string(),
            value: " Episode One ".to_string(),
        }];

        let desired = compile_desired_target(
            &source,
            "/output/episode.mkv",
            &ordered_multistream_target(),
            UnmatchedStreamPolicy::Remove,
        )?;

        assert_eq!(
            desired.container_metadata,
            vec![ContainerMetadataEntry {
                key: "title".to_string(),
                value: "Episode One".to_string(),
            }]
        );
        Ok(())
    }

    #[test]
    fn compiles_replace_container_metadata_policy() -> Result<(), TargetCompileError> {
        let source = multistream_source();
        let mut target = ordered_multistream_target();
        target.container_metadata_policy = " Replace ".to_string();
        target.container_metadata = vec![
            ContainerMetadataEntry {
                key: " TITLE ".to_string(),
                value: " Episode One ".to_string(),
            },
            ContainerMetadataEntry {
                key: "album".to_string(),
                value: "Season One".to_string(),
            },
        ];

        let desired = compile_desired_target(
            &source,
            "/output/episode.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
        )?;

        assert_eq!(
            desired.container_metadata_policy.as_deref(),
            Some("replace")
        );
        assert_eq!(
            desired.container_metadata,
            vec![
                ContainerMetadataEntry {
                    key: "album".to_string(),
                    value: "Season One".to_string(),
                },
                ContainerMetadataEntry {
                    key: "title".to_string(),
                    value: "Episode One".to_string(),
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn compiles_strip_container_chapter_policy() -> Result<(), TargetCompileError> {
        let mut source = multistream_source();
        source.container_chapters = vec![ContainerChapterEntry {
            start_millis: 0,
            end_millis: 1_000,
            metadata: Vec::new(),
        }];
        let mut target = ordered_multistream_target();
        target.container_chapter_policy = " Strip ".to_string();

        let desired = compile_desired_target(
            &source,
            "/output/episode.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
        )?;

        assert_eq!(desired.container_chapter_policy.as_deref(), Some("strip"));
        assert!(desired.container_chapters.is_empty());
        Ok(())
    }

    #[test]
    fn compiles_preserved_container_chapter_value() -> Result<(), TargetCompileError> {
        let mut source = multistream_source();
        source.container_chapters = vec![ContainerChapterEntry {
            start_millis: 0,
            end_millis: 1_000,
            metadata: Vec::new(),
        }];
        let target = ordered_multistream_target();

        let desired = compile_desired_target(
            &source,
            "/output/episode.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
        )?;

        assert_eq!(
            desired.container_chapter_policy.as_deref(),
            Some("preserve")
        );
        assert_eq!(desired.container_chapters, source.container_chapters);
        Ok(())
    }

    #[test]
    fn compiles_strip_container_attachment_policy() -> Result<(), TargetCompileError> {
        let source = MediaGraph {
            source_path: "/input/episode.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![
                stream(
                    0,
                    StreamKind::Video,
                    "h264",
                    None,
                    Some("Main"),
                    &["default"],
                ),
                stream(1, StreamKind::Attachment, "ttf", None, Some("Font"), &[]),
            ],
        };
        let mut target = DesiredTarget {
            target_key: "web-playback".to_string(),
            version: 4,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: " Strip ".to_string(),
            streams: vec![target_stream(
                "video-main",
                StreamKind::Video,
                None,
                None,
                "h264",
            )],
        };
        target.streams[0].dispositions = vec!["default".to_string()];

        let desired = compile_desired_target(
            &source,
            "/output/episode.mkv",
            &target,
            UnmatchedStreamPolicy::Reject,
        )?;

        assert_eq!(
            desired.container_attachment_policy.as_deref(),
            Some("strip")
        );
        assert_eq!(
            desired
                .streams
                .iter()
                .map(|stream| (stream.stream_id, stream.kind))
                .collect::<Vec<_>>(),
            vec![(0, StreamKind::Video)]
        );
        Ok(())
    }

    #[test]
    fn compiles_replace_container_chapter_policy() -> Result<(), TargetCompileError> {
        let source = multistream_source();
        let mut target = ordered_multistream_target();
        target.container_chapter_policy = " Replace ".to_string();
        target.container_chapters = vec![
            ContainerChapterEntry {
                start_millis: 60_000,
                end_millis: 120_000,
                metadata: vec![ContainerMetadataEntry {
                    key: "title".to_string(),
                    value: "Act Two".to_string(),
                }],
            },
            ContainerChapterEntry {
                start_millis: 0,
                end_millis: 60_000,
                metadata: vec![ContainerMetadataEntry {
                    key: " TITLE ".to_string(),
                    value: " Act One ".to_string(),
                }],
            },
        ];

        let desired = compile_desired_target(
            &source,
            "/output/episode.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
        )?;

        assert_eq!(desired.container_chapter_policy.as_deref(), Some("replace"));
        assert_eq!(
            desired.container_chapters,
            vec![
                ContainerChapterEntry {
                    start_millis: 0,
                    end_millis: 60_000,
                    metadata: vec![ContainerMetadataEntry {
                        key: "title".to_string(),
                        value: "Act One".to_string(),
                    }],
                },
                ContainerChapterEntry {
                    start_millis: 60_000,
                    end_millis: 120_000,
                    metadata: vec![ContainerMetadataEntry {
                        key: "title".to_string(),
                        value: "Act Two".to_string(),
                    }],
                },
            ]
        );
        Ok(())
    }

    #[test]
    fn target_validation_rejects_unknown_container_metadata_policy() {
        let source = multistream_source();
        let mut target = ordered_multistream_target();
        target.container_metadata_policy = "rewrite".to_string();

        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedContainerMetadataPolicy(
                "rewrite".to_string()
            ))
        );
    }

    #[test]
    fn target_validation_rejects_inconsistent_container_metadata_rows() {
        let source = multistream_source();
        let mut preserve_with_rows = ordered_multistream_target();
        preserve_with_rows.container_metadata = vec![ContainerMetadataEntry {
            key: "title".to_string(),
            value: "Episode One".to_string(),
        }];
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &preserve_with_rows,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::ContainerMetadataRequiresReplacePolicy)
        );

        let mut replace_without_rows = ordered_multistream_target();
        replace_without_rows.container_metadata_policy = "replace".to_string();
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &replace_without_rows,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::ReplaceContainerMetadataRequiresRows)
        );

        let mut duplicate_rows = ordered_multistream_target();
        duplicate_rows.container_metadata_policy = "replace".to_string();
        duplicate_rows.container_metadata = vec![
            ContainerMetadataEntry {
                key: "title".to_string(),
                value: "Episode One".to_string(),
            },
            ContainerMetadataEntry {
                key: " TITLE ".to_string(),
                value: "Duplicate".to_string(),
            },
        ];
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &duplicate_rows,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::DuplicateContainerMetadataKey(
                "title".to_string()
            ))
        );
    }

    #[test]
    fn target_validation_bounds_container_metadata_resources() {
        let source = multistream_source();
        let mut target = ordered_multistream_target();
        target.container_metadata_policy = "replace".to_string();
        target.container_metadata = (0..=MAX_CONTAINER_METADATA_ENTRIES)
            .map(|index| ContainerMetadataEntry {
                key: format!("key-{index}"),
                value: "value".to_string(),
            })
            .collect();
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::TooManyContainerMetadataEntries)
        );

        target.container_metadata = vec![ContainerMetadataEntry {
            key: "é".repeat(MAX_CONTAINER_METADATA_KEY_BYTES / 2 + 1),
            value: "value".to_string(),
        }];
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::ContainerMetadataKeyTooLong)
        );

        target.container_metadata = (0..17)
            .map(|index| ContainerMetadataEntry {
                key: format!("key-{index}"),
                value: "x".repeat(MAX_CONTAINER_METADATA_VALUE_BYTES),
            })
            .collect();
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::ContainerMetadataBytesExceeded)
        );
    }

    #[test]
    fn target_validation_rejects_unknown_container_chapter_policy() {
        let source = multistream_source();
        let mut target = ordered_multistream_target();
        target.container_chapter_policy = "rewrite".to_string();

        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedContainerChapterPolicy(
                "rewrite".to_string()
            ))
        );
    }

    #[test]
    fn target_validation_rejects_unknown_container_attachment_policy() {
        let source = multistream_source();
        let mut target = ordered_multistream_target();
        target.container_attachment_policy = "rewrite".to_string();

        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedContainerAttachmentPolicy(
                "rewrite".to_string()
            ))
        );
    }

    #[test]
    fn target_validation_rejects_inconsistent_container_chapter_rows() {
        let source = multistream_source();
        let mut preserve_with_rows = ordered_multistream_target();
        preserve_with_rows.container_chapters = vec![ContainerChapterEntry {
            start_millis: 0,
            end_millis: 60_000,
            metadata: Vec::new(),
        }];
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &preserve_with_rows,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::ContainerChaptersRequireReplacePolicy)
        );

        let mut replace_without_rows = ordered_multistream_target();
        replace_without_rows.container_chapter_policy = "replace".to_string();
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &replace_without_rows,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::ReplaceContainerChaptersRequiresRows)
        );

        let mut invalid_range = ordered_multistream_target();
        invalid_range.container_chapter_policy = "replace".to_string();
        invalid_range.container_chapters = vec![ContainerChapterEntry {
            start_millis: 60_000,
            end_millis: 60_000,
            metadata: Vec::new(),
        }];
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &invalid_range,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::InvalidContainerChapterRange {
                start_millis: 60_000,
                end_millis: 60_000,
            })
        );

        let mut overlapping = ordered_multistream_target();
        overlapping.container_chapter_policy = "replace".to_string();
        overlapping.container_chapters = vec![
            ContainerChapterEntry {
                start_millis: 0,
                end_millis: 60_000,
                metadata: Vec::new(),
            },
            ContainerChapterEntry {
                start_millis: 59_999,
                end_millis: 90_000,
                metadata: Vec::new(),
            },
        ];
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &overlapping,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::OverlappingContainerChapter {
                start_millis: 59_999,
                end_millis: 90_000,
            })
        );
    }

    #[test]
    fn target_validation_rejects_invalid_container_chapter_metadata() {
        let source = multistream_source();
        let mut duplicate_metadata = ordered_multistream_target();
        duplicate_metadata.container_chapter_policy = "replace".to_string();
        duplicate_metadata.container_chapters = vec![ContainerChapterEntry {
            start_millis: 0,
            end_millis: 60_000,
            metadata: vec![
                ContainerMetadataEntry {
                    key: "title".to_string(),
                    value: "Act One".to_string(),
                },
                ContainerMetadataEntry {
                    key: " TITLE ".to_string(),
                    value: "Duplicate".to_string(),
                },
            ],
        }];
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &duplicate_metadata,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::DuplicateContainerChapterMetadataKey(
                "title".to_string()
            ))
        );
    }

    #[test]
    fn target_validation_bounds_container_chapter_values() {
        let source = multistream_source();
        let mut too_many_chapters = ordered_multistream_target();
        too_many_chapters.container_chapter_policy = "replace".to_string();
        too_many_chapters.container_chapters = (0..=MAX_CONTAINER_CHAPTERS)
            .map(|index| ContainerChapterEntry {
                start_millis: i64::try_from(index).unwrap_or_default() * 2,
                end_millis: i64::try_from(index).unwrap_or_default() * 2 + 1,
                metadata: Vec::new(),
            })
            .collect();
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &too_many_chapters,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::TooManyContainerChapters)
        );

        let mut too_many_metadata = ordered_multistream_target();
        too_many_metadata.container_chapter_policy = "replace".to_string();
        too_many_metadata.container_chapters = vec![ContainerChapterEntry {
            start_millis: 0,
            end_millis: 1,
            metadata: (0..=MAX_CONTAINER_CHAPTER_METADATA_ENTRIES)
                .map(|index| ContainerMetadataEntry {
                    key: format!("key-{index}"),
                    value: "value".to_string(),
                })
                .collect(),
        }];
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &too_many_metadata,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::TooManyContainerChapterMetadataEntries)
        );

        let mut aggregate_bytes = ordered_multistream_target();
        aggregate_bytes.container_chapter_policy = "replace".to_string();
        aggregate_bytes.container_chapters = vec![ContainerChapterEntry {
            start_millis: 0,
            end_millis: 1,
            metadata: (0..17)
                .map(|index| ContainerMetadataEntry {
                    key: format!("key-{index}"),
                    value: "x".repeat(MAX_CONTAINER_METADATA_VALUE_BYTES),
                })
                .collect(),
        }];
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/episode.mkv",
                &aggregate_bytes,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::ContainerChapterMetadataBytesExceeded)
        );
    }

    #[test]
    fn optional_rows_skip_and_preserve_policy_appends_unmatched_streams()
    -> Result<(), TargetCompileError> {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![
                stream(2, StreamKind::Video, "h264", None, None, &[]),
                stream(6, StreamKind::Attachment, "ttf", None, Some("Font"), &[]),
                stream(
                    7,
                    StreamKind::Data,
                    "bin_data",
                    Some("eng"),
                    Some("Timecode"),
                    &["default"],
                ),
            ],
        };
        let mut optional_audio = target_stream(
            "audio-optional",
            StreamKind::Audio,
            None,
            Some("eng"),
            "aac",
        );
        optional_audio.optional = true;
        let target = DesiredTarget {
            target_key: "preserve".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![optional_audio],
        };

        let desired = compile_desired_target(
            &source,
            "/output/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Preserve,
        )?;

        assert_eq!(desired.streams.len(), 3);
        assert_eq!(desired.streams[0].stream_id, 0);
        assert_eq!(desired.streams[0].codec, source.streams[0].codec);
        assert_eq!(desired.stream_bindings[0].source_stream_id, Some(2));
        assert_eq!(desired.streams[1].kind, StreamKind::Attachment);
        assert_eq!(desired.stream_bindings[1].source_stream_id, Some(6));
        assert_eq!(desired.streams[2].kind, StreamKind::Data);
        assert_eq!(desired.stream_bindings[2].source_stream_id, Some(7));
        Ok(())
    }

    #[test]
    fn per_kind_unmatched_policies_apply_independently() -> Result<(), TargetCompileError> {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![
                stream(1, StreamKind::Video, "h264", None, None, &[]),
                stream(2, StreamKind::Audio, "aac", Some("eng"), None, &[]),
                stream(3, StreamKind::Subtitle, "subrip", Some("eng"), None, &[]),
                stream(4, StreamKind::Attachment, "ttf", None, Some("Font"), &[]),
                stream(5, StreamKind::Data, "bin_data", None, None, &[]),
            ],
        };
        let target = DesiredTarget {
            target_key: "per-kind".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: Vec::new(),
        };

        assert_eq!(
            compile_desired_target_with_unmatched_policies(
                &source,
                "/output/movie.mkv",
                &target,
                UnmatchedStreamPolicies::first_release_defaults(),
            ),
            Err(TargetCompileError::UnmatchedSourceStream(1))
        );

        let desired = compile_desired_target_with_unmatched_policies(
            &source,
            "/output/movie.mkv",
            &target,
            UnmatchedStreamPolicies {
                video: UnmatchedStreamPolicy::Remove,
                ..UnmatchedStreamPolicies::first_release_defaults()
            },
        )?;

        assert_eq!(
            desired
                .streams
                .iter()
                .map(|stream| stream.stream_id)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            desired
                .stream_bindings
                .iter()
                .map(|binding| binding.source_stream_id)
                .collect::<Vec<_>>(),
            vec![Some(2), Some(3), Some(4)]
        );
        Ok(())
    }

    #[test]
    fn missing_required_row_and_rejected_unmatched_stream_are_errors() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![stream(2, StreamKind::Video, "h264", None, None, &[])],
        };
        let target = DesiredTarget {
            target_key: "strict".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![target_stream(
                "audio-main",
                StreamKind::Audio,
                None,
                Some("eng"),
                "aac",
            )],
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::RequiredStreamMissing(
                "audio-main".to_string()
            ))
        );

        let empty_target = DesiredTarget {
            streams: Vec::new(),
            ..target
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &empty_target,
                UnmatchedStreamPolicy::Reject,
            ),
            Err(TargetCompileError::UnmatchedSourceStream(2))
        );
    }

    #[test]
    fn target_validation_rejects_duplicate_keys_and_cross_kind_shapes() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let first = target_stream("main", StreamKind::Video, None, None, "hevc");
        let duplicate = target_stream("MAIN", StreamKind::Video, None, None, "av1");
        let duplicate_target = DesiredTarget {
            target_key: "invalid".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![first, duplicate],
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &duplicate_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::DuplicateStreamKey("MAIN".to_string()))
        );

        let mut invalid_video = target_stream("video", StreamKind::Video, None, None, "hevc");
        invalid_video.channels = Some(2);
        let invalid_shape_target = DesiredTarget {
            streams: vec![invalid_video],
            ..duplicate_target
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &invalid_shape_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::AudioShapeOnNonAudioStream(
                "video".to_string()
            ))
        );

        let mut invalid_audio = target_stream("audio", StreamKind::Audio, None, None, "aac");
        invalid_audio.video_profile = Some("main10".to_string());
        let invalid_shape_target = DesiredTarget {
            streams: vec![invalid_audio],
            ..invalid_shape_target
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &invalid_shape_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::VideoShapeOnNonVideoStream(
                "audio".to_string()
            ))
        );

        let mut unsupported_hdr = target_stream("video", StreamKind::Video, None, None, "hevc");
        unsupported_hdr.hdr_format = Some("dolby_vision".to_string());
        let unsupported_hdr_target = DesiredTarget {
            streams: vec![unsupported_hdr],
            ..invalid_shape_target
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &unsupported_hdr_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedHdrFormat {
                stream_key: "video".to_string(),
                hdr_format: "dolby_vision".to_string(),
            })
        );
    }

    #[test]
    fn target_validation_rejects_unsupported_stream_kinds() {
        let source = multistream_source();
        for (key, kind) in [
            ("attachment-font", StreamKind::Attachment),
            ("chapter-main", StreamKind::Chapter),
            ("data-main", StreamKind::Data),
        ] {
            let mut target = ordered_multistream_target();
            target.streams = vec![target_stream(key, kind, None, None, "bin_data")];
            assert_eq!(
                compile_desired_target(
                    &source,
                    "/output/movie.mkv",
                    &target,
                    UnmatchedStreamPolicy::Remove,
                ),
                Err(TargetCompileError::UnsupportedDesiredStreamKind(
                    key.to_string()
                ))
            );
        }
    }

    #[test]
    fn target_validation_prioritizes_invalid_audio_policy() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let mut invalid_audio_policy = target_stream("audio", StreamKind::Audio, None, None, "aac");
        invalid_audio_policy.audio_dynamic_range = Some("flatten".to_string());
        invalid_audio_policy.video_profile = Some("main10".to_string());
        let target = DesiredTarget {
            target_key: "invalid".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![invalid_audio_policy],
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::AudioShapeOnNonAudioStream(
                "audio".to_string()
            ))
        );
    }

    #[test]
    fn target_validation_rejects_zero_audio_constraints() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let target = DesiredTarget {
            target_key: "invalid".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![target_stream("audio", StreamKind::Audio, None, None, "aac")],
        };

        let mut invalid_channels = target.clone();
        invalid_channels.streams[0].channels = Some(0);
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &invalid_channels,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::InvalidAudioConstraint {
                stream_key: "audio".to_string(),
                field: "channels",
            })
        );

        let mut invalid_bitrate = target.clone();
        invalid_bitrate.streams[0].audio_bitrate_bps = Some(0);
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &invalid_bitrate,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::InvalidAudioConstraint {
                stream_key: "audio".to_string(),
                field: "audio_bitrate_bps",
            })
        );

        let mut invalid_sample_rate = target;
        invalid_sample_rate.streams[0].audio_sample_rate_hz = Some(0);
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &invalid_sample_rate,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::InvalidAudioConstraint {
                stream_key: "audio".to_string(),
                field: "audio_sample_rate_hz",
            })
        );
    }

    #[test]
    fn target_validation_rejects_unsupported_audio_channel_layouts() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let target = DesiredTarget {
            target_key: "invalid".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![target_stream("audio", StreamKind::Audio, None, None, "aac")],
        };

        let mut invalid_layout = target.clone();
        invalid_layout.streams[0].channel_layout = Some("ambisonic".to_string());
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &invalid_layout,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedAudioChannelLayout {
                stream_key: "audio".to_string(),
                channel_layout: "ambisonic".to_string(),
            })
        );

        let mut count_mismatch = target;
        count_mismatch.streams[0].channels = Some(6);
        count_mismatch.streams[0].channel_layout = Some("stereo".to_string());
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &count_mismatch,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::AudioChannelLayoutCountMismatch {
                stream_key: "audio".to_string(),
                channels: 6,
                layout_channels: 2,
            })
        );
    }

    #[test]
    fn target_compilation_canonicalizes_supported_audio_channel_layouts() {
        let source = multistream_source();
        let mut target = DesiredTarget {
            target_key: "audio-layout".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![target_stream(
                "audio-main",
                StreamKind::Audio,
                Some(SemanticRole::Primary),
                Some("eng"),
                "opus",
            )],
        };
        target.streams[0].channels = Some(2);
        target.streams[0].channel_layout = Some(" 2C ".to_string());

        let desired = compile_desired_target(
            &source,
            "/output/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
        )
        .expect("supported channel layout should compile");

        assert_eq!(desired.streams[0].channel_layout.as_deref(), Some("stereo"));
    }

    #[test]
    fn target_validation_rejects_zero_video_bitrate() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let mut invalid_video = target_stream("video", StreamKind::Video, None, None, "hevc");
        invalid_video.video_bitrate_bps = Some(0);
        let target = DesiredTarget {
            target_key: "invalid".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![invalid_video],
        };

        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::InvalidVideoConstraint {
                stream_key: "video".to_string(),
                field: "video_bitrate_bps",
            })
        );
    }

    #[test]
    fn target_validation_rejects_unknown_video_color_values() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let mut unsupported_color = target_stream("video", StreamKind::Video, None, None, "hevc");
        unsupported_color.color_transfer = Some("make-it-pop".to_string());
        let unsupported_color_target = DesiredTarget {
            target_key: "invalid-color".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![unsupported_color],
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &unsupported_color_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedVideoColor {
                stream_key: "video".to_string(),
                field: "color_transfer",
                value: "make-it-pop".to_string(),
            })
        );

        let mut supported_sdr_color = target_stream("video", StreamKind::Video, None, None, "h264");
        supported_sdr_color.color_primaries = Some("bt709".to_string());
        supported_sdr_color.color_transfer = Some("bt709".to_string());
        supported_sdr_color.color_space = Some("bt709".to_string());
        let supported_sdr_color_target = DesiredTarget {
            target_key: "sdr-color".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![supported_sdr_color],
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &supported_sdr_color_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::RequiredStreamMissing(
                "video".to_string()
            ))
        );
    }

    #[test]
    fn target_validation_rejects_unknown_video_level() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let mut unsupported_level = target_stream("video", StreamKind::Video, None, None, "hevc");
        unsupported_level.video_level = Some("7.9".to_string());
        let unsupported_level_target = DesiredTarget {
            target_key: "invalid-level".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![unsupported_level],
        };

        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &unsupported_level_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedVideoLevel {
                stream_key: "video".to_string(),
                codec: "hevc".to_string(),
                video_level: "7.9".to_string(),
            })
        );

        let mut supported_av1_level = target_stream("video", StreamKind::Video, None, None, "av1");
        supported_av1_level.video_level = Some("7.3".to_string());
        let supported_av1_level_target = DesiredTarget {
            target_key: "av1-level".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![supported_av1_level],
        };

        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &supported_av1_level_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::RequiredStreamMissing(
                "video".to_string()
            ))
        );

        let mut unsupported_av1_level =
            target_stream("video", StreamKind::Video, None, None, "libaom-av1");
        unsupported_av1_level.video_level = Some("7.9".to_string());
        let unsupported_av1_level_target = DesiredTarget {
            target_key: "invalid-av1-level".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![unsupported_av1_level],
        };

        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &unsupported_av1_level_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedVideoLevel {
                stream_key: "video".to_string(),
                codec: "libaom-av1".to_string(),
                video_level: "7.9".to_string(),
            })
        );
    }

    #[test]
    fn target_validation_rejects_explicit_data_stream_rows() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let unsupported_target = DesiredTarget {
            target_key: "unsupported-data".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![target_stream(
                "opaque-data",
                StreamKind::Data,
                None,
                None,
                "bin_data",
            )],
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &unsupported_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedDesiredStreamKind(
                "opaque-data".to_string()
            ))
        );
    }

    #[test]
    fn embeds_existing_sidecar_and_schedules_source_removal() -> Result<(), TargetCompileError> {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream(2, StreamKind::Video, "h264", None, None, &[])],
        };
        let target = DesiredTarget {
            target_key: "embed-sidecar".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![
                target_stream("video", StreamKind::Video, None, None, "h264"),
                target_stream(
                    "subtitle",
                    StreamKind::Subtitle,
                    Some(SemanticRole::Forced),
                    Some("eng"),
                    "webvtt",
                ),
            ],
        };
        let sidecars = vec![SidecarSubtitleInput {
            path: "/input/movie.eng.forced.srt".to_string(),
            companion_path: None,
            language: Some("eng".to_string()),
            role: Some(SemanticRole::Forced),
            codec: "srt".to_string(),
            image_based: false,
        }];

        let compiled = compile_desired_target_with_sidecars(
            &source,
            "/workspace/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
            &sidecars,
        )?;

        assert_eq!(compiled.graph.streams.len(), 2);
        assert_eq!(compiled.graph.streams[1].stream_id, 1);
        assert_eq!(compiled.graph.stream_bindings[1].source_stream_id, None);
        assert_eq!(compiled.sidecar_embeddings.len(), 1);
        assert!(compiled.sidecar_outputs.is_empty());
        assert_eq!(compiled.sidecar_removals, vec![sidecars[0].path.clone()]);
        Ok(())
    }

    #[test]
    fn unmatched_sidecars_follow_remove_preserve_and_reject_policy()
    -> Result<(), TargetCompileError> {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream(2, StreamKind::Video, "h264", None, None, &[])],
        };
        let target = DesiredTarget {
            target_key: "video-only".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![target_stream(
                "video",
                StreamKind::Video,
                None,
                None,
                "h264",
            )],
        };
        let sidecars = vec![SidecarSubtitleInput {
            path: "/input/movie.eng.idx".to_string(),
            companion_path: Some("/input/movie.eng.sub".to_string()),
            language: Some("eng".to_string()),
            role: None,
            codec: "dvd_subtitle".to_string(),
            image_based: true,
        }];

        let removed = compile_desired_target_with_sidecars(
            &source,
            "/workspace/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
            &sidecars,
        )?;
        assert_eq!(
            removed.sidecar_removals,
            vec![
                "/input/movie.eng.idx".to_string(),
                "/input/movie.eng.sub".to_string()
            ]
        );

        let preserved = compile_desired_target_with_sidecars(
            &source,
            "/workspace/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Preserve,
            &sidecars,
        )?;
        assert!(preserved.sidecar_removals.is_empty());

        assert_eq!(
            compile_desired_target_with_sidecars(
                &source,
                "/workspace/movie.mkv",
                &target,
                UnmatchedStreamPolicy::Reject,
                &sidecars,
            ),
            Err(TargetCompileError::UnmatchedSidecar(
                "/input/movie.eng.idx".to_string()
            ))
        );
        Ok(())
    }

    #[test]
    fn both_placement_keeps_embedded_stream_and_extracts_managed_sidecar()
    -> Result<(), TargetCompileError> {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream(
                4,
                StreamKind::Subtitle,
                "subrip",
                Some("eng"),
                Some("Forced"),
                &["forced"],
            )],
        };
        let mut subtitle = target_stream(
            "forced",
            StreamKind::Subtitle,
            Some(SemanticRole::Forced),
            Some("eng"),
            "srt",
        );
        subtitle.subtitle_placement = Some(SubtitlePlacement::Both);
        let target = DesiredTarget {
            target_key: "both".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![subtitle],
        };

        let compiled = compile_desired_target_with_sidecars(
            &source,
            "/workspace/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
            &[],
        )?;

        assert_eq!(compiled.graph.streams.len(), 1);
        assert_eq!(compiled.sidecar_outputs.len(), 1);
        assert_eq!(
            compiled.sidecar_outputs[0].path,
            "/workspace/movie.eng.forced.srt"
        );
        assert_eq!(
            compiled.sidecar_outputs[0].source,
            SidecarOutputSource::EmbeddedStream { stream_id: 4 }
        );
        Ok(())
    }

    #[test]
    fn sidecar_compilation_separates_workspace_candidate_and_final_destination()
    -> Result<(), TargetCompileError> {
        let source = MediaGraph {
            source_path: "/library/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream(
                4,
                StreamKind::Subtitle,
                "subrip",
                Some("eng"),
                Some("Forced"),
                &["forced"],
            )],
        };
        let mut subtitle = target_stream(
            "forced",
            StreamKind::Subtitle,
            Some(SemanticRole::Forced),
            Some("eng"),
            "srt",
        );
        subtitle.subtitle_placement = Some(SubtitlePlacement::Sidecar);
        let target = DesiredTarget {
            target_key: "sidecar".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![subtitle],
        };

        let compiled = compile_desired_target_with_sidecars_at(
            &source,
            "/workspace/movie.mkv",
            "/library/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
            &[],
        )?;

        assert_eq!(
            compiled.sidecar_outputs[0].path,
            "/workspace/movie.eng.forced.srt"
        );
        assert_eq!(
            compiled.sidecar_outputs[0].destination_path,
            "/library/movie.eng.forced.srt"
        );
        Ok(())
    }

    #[test]
    fn sidecar_compilation_rejects_unknown_output_codec() {
        let source = MediaGraph {
            source_path: "/library/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![stream(
                1,
                StreamKind::Subtitle,
                "subrip",
                Some("eng"),
                None,
                &[],
            )],
        };
        let mut subtitle = target_stream(
            "subtitle",
            StreamKind::Subtitle,
            None,
            Some("eng"),
            "unknown-subtitle",
        );
        subtitle.subtitle_placement = Some(SubtitlePlacement::Sidecar);
        let target = DesiredTarget {
            target_key: "invalid-sidecar".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![subtitle],
        };

        assert_eq!(
            compile_desired_target_with_sidecars(
                &source,
                "/workspace/movie.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
                &[],
            ),
            Err(TargetCompileError::UnsupportedSidecarCodec(
                "unknown-subtitle".to_string()
            ))
        );
    }

    #[test]
    fn image_subtitles_never_require_ocr() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![stream(
                8,
                StreamKind::Subtitle,
                "hdmv_pgs_subtitle",
                Some("eng"),
                None,
                &[],
            )],
        };
        let mut subtitle = target_stream("image", StreamKind::Subtitle, None, Some("eng"), "srt");
        subtitle.image_subtitle_action = Some(ImageSubtitleAction::Preserve);
        let target = DesiredTarget {
            target_key: "image".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![subtitle],
        };

        assert_eq!(
            compile_desired_target_with_sidecars(
                &source,
                "/workspace/movie.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
                &[],
            ),
            Err(TargetCompileError::ImageSubtitleConversionNotAllowed(
                "image".to_string()
            ))
        );
    }

    #[test]
    fn graph_only_compiler_rejects_sidecar_artifact_placement() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: Vec::new(),
            streams: vec![stream(
                1,
                StreamKind::Subtitle,
                "srt",
                Some("eng"),
                None,
                &[],
            )],
        };
        let mut subtitle =
            target_stream("subtitle", StreamKind::Subtitle, None, Some("eng"), "srt");
        subtitle.subtitle_placement = Some(SubtitlePlacement::Sidecar);
        let target = DesiredTarget {
            target_key: "sidecar".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![subtitle],
        };

        assert_eq!(
            compile_desired_target(
                &source,
                "/workspace/movie.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::SidecarPlacementRequiresArtifactPlan(
                "subtitle".to_string()
            ))
        );
    }

    #[test]
    fn explicit_source_binding_fans_one_audio_source_out_to_two_outputs()
    -> Result<(), TargetCompileError> {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream(
                17,
                StreamKind::Audio,
                "dts",
                Some("eng"),
                Some("Main"),
                &["default"],
            )],
        };
        let mut stereo = target_stream(
            "audio-stereo",
            StreamKind::Audio,
            Some(SemanticRole::Primary),
            Some("eng"),
            "aac",
        );
        stereo.source_binding_key = Some("main-dts".to_string());
        stereo.channels = Some(2);
        stereo.channel_layout = Some("stereo".to_string());
        let mut surround = target_stream(
            "audio-surround",
            StreamKind::Audio,
            Some(SemanticRole::Primary),
            Some("eng"),
            "eac3",
        );
        surround.source_binding_key = Some("main-dts".to_string());
        surround.channels = Some(6);
        surround.channel_layout = Some("5.1".to_string());
        let target = DesiredTarget {
            target_key: "audio-fanout".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![stereo, surround],
        };

        let desired = compile_desired_target(
            &source,
            "/output/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Reject,
        )?;

        assert_eq!(desired.streams.len(), 2);
        assert_eq!(desired.streams[0].stream_id, 0);
        assert_eq!(desired.streams[0].codec, "aac");
        assert_eq!(desired.streams[1].stream_id, 1);
        assert_eq!(desired.streams[1].codec, "eac3");
        assert_eq!(
            desired
                .stream_bindings
                .iter()
                .map(|binding| binding.source_stream_id)
                .collect::<Vec<_>>(),
            vec![Some(17), Some(17)]
        );
        Ok(())
    }

    #[test]
    fn desired_target_yaml_rejects_language_path_payloads() -> Result<(), TargetCompileError> {
        let invalid = [
            "../escape",
            "/absolute",
            r"..\escape",
            "eng/escape",
            "eng%2fescape",
            "eng%2Fescape",
            "eng%5cescape",
            "eng%5Cescape",
            r"C:\escape",
            r"\\server\share",
            "en\u{0000}",
            "ENG",
            ".",
        ];
        for value in invalid {
            assert!(LanguageToken::parse(value).is_err(), "accepted {value:?}");
            let yaml = format!(
                "target_key: language-boundary\nversion: 1\ncontainer: matroska\ncontainer_attachment_policy: preserve\nstreams:\n  - stream_key: subtitles\n    kind: subtitle\n    language: {value:?}\n    optional: false\n    codec: subrip\n"
            );
            assert!(
                parse_desired_target_yaml(&yaml).is_err(),
                "desired-target YAML accepted {value:?}"
            );
        }
        let valid = parse_desired_target_yaml(
            "target_key: language-boundary\nversion: 1\ncontainer: matroska\ncontainer_attachment_policy: preserve\nstreams:\n  - stream_key: subtitles\n    kind: subtitle\n    language: eng\n    optional: false\n    codec: subrip\n",
        )?;
        assert_eq!(
            valid.streams[0]
                .language
                .as_ref()
                .map(LanguageToken::as_str),
            Some("eng")
        );
        Ok(())
    }

    #[test]
    fn compiler_rejects_forged_language_before_sidecar_derivation() {
        let source = MediaGraph {
            source_path: "/library/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream(
                4,
                StreamKind::Subtitle,
                "subrip",
                Some("eng"),
                Some("Forced"),
                &["forced"],
            )],
        };
        let mut subtitle = target_stream(
            "forced",
            StreamKind::Subtitle,
            Some(SemanticRole::Forced),
            Some("eng"),
            "srt",
        );
        subtitle.language = Some(LanguageToken("../escape".to_string()));
        subtitle.subtitle_placement = Some(SubtitlePlacement::Sidecar);
        let target = DesiredTarget {
            target_key: "forged-language".to_string(),
            version: 1,
            container: "matroska".to_string(),
            container_metadata_policy: "preserve".to_string(),
            container_metadata: Vec::new(),
            container_chapter_policy: "preserve".to_string(),
            container_chapters: Vec::new(),
            container_attachment_policy: "preserve".to_string(),
            streams: vec![subtitle],
        };

        assert_eq!(
            compile_desired_target_with_sidecars_at(
                &source,
                "/workspace/movie.mkv",
                "/library/movie.mkv",
                &target,
                UnmatchedStreamPolicy::Remove,
                &[],
            ),
            Err(TargetCompileError::InvalidLanguage("../escape".to_string()))
        );
    }

    #[test]
    fn desired_target_yaml_compiles_contained_workspace_and_destination_sidecars()
    -> Result<(), TargetCompileError> {
        let target = parse_desired_target_yaml(
            "target_key: imported-sidecar\nversion: 1\ncontainer: matroska\ncontainer_attachment_policy: preserve\nstreams:\n  - stream_key: forced\n    kind: subtitle\n    role: forced\n    language: eng\n    optional: false\n    codec: srt\n    subtitle_placement: sidecar\n",
        )?;
        let source = MediaGraph {
            source_path: "/library/movie.mkv".to_string(),
            container_metadata: Vec::new(),
            container_chapters: Vec::new(),
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream(
                4,
                StreamKind::Subtitle,
                "subrip",
                Some("eng"),
                Some("Forced"),
                &["forced"],
            )],
        };

        let compiled = compile_desired_target_with_sidecars_at(
            &source,
            "/workspace/movie.mkv",
            "/library/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Remove,
            &[],
        )?;
        let output = &compiled.sidecar_outputs[0];
        assert_eq!(
            Path::new(&output.path).parent(),
            Some(Path::new("/workspace"))
        );
        assert_eq!(
            Path::new(&output.destination_path).parent(),
            Some(Path::new("/library"))
        );
        assert!(
            Path::new(&output.path)
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_single_path_component)
        );
        assert!(
            Path::new(&output.destination_path)
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_single_path_component)
        );
        Ok(())
    }

    #[test]
    fn sidecar_file_name_is_exactly_one_relative_component() {
        assert!(is_single_path_component("movie.eng.srt"));
        for value in ["../movie.eng.srt", "/movie.eng.srt", r"..\movie.eng.srt"] {
            assert!(!is_single_path_component(value), "accepted {value:?}");
        }
    }
}
