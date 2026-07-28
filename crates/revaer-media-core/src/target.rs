//! Desired-target compilation into a concrete output graph.

use crate::classify::{SemanticRole, infer_role};
use crate::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};
use crate::normalize::{normalize_container_format, normalize_subtitle_codec};
use std::collections::BTreeSet;
use std::path::Path;
use thiserror::Error;

/// Desired output stream independent of source-container stream indexes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetStream {
    /// Stable identity within the target version.
    pub stream_key: String,
    /// Required media kind.
    pub kind: StreamKind,
    /// Optional semantic-role selector.
    pub role: Option<SemanticRole>,
    /// Optional normalized language selector.
    pub language: Option<String>,
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
    pub dispositions: Vec<String>,
    /// Desired subtitle placement. Non-subtitle rows leave this unset.
    pub subtitle_placement: Option<SubtitlePlacement>,
    /// Image-subtitle action. Non-subtitle rows leave this unset.
    pub image_subtitle_action: Option<ImageSubtitleAction>,
}

/// Desired placement for a selected subtitle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredTarget {
    /// Stable target key.
    pub target_key: String,
    /// Immutable target version.
    pub version: u32,
    /// Desired output container format.
    pub container: String,
    /// Desired streams in final mux order.
    pub streams: Vec<TargetStream>,
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

/// Desired-target validation or matching failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TargetCompileError {
    /// Target identity is blank or its version is zero.
    #[error("desired target identity is invalid")]
    InvalidTargetIdentity,
    /// Target container is blank.
    #[error("desired target container is empty")]
    EmptyContainer,
    /// A target stream key is blank.
    #[error("desired target stream key is empty")]
    EmptyStreamKey,
    /// A target stream key occurs more than once.
    #[error("duplicate desired target stream key: {0}")]
    DuplicateStreamKey(String),
    /// A target stream codec is blank.
    #[error("desired target stream codec is empty: {0}")]
    EmptyCodec(String),
    /// Audio-only properties were assigned to a non-audio stream.
    #[error("audio shape assigned to non-audio target stream: {0}")]
    AudioShapeOnNonAudioStream(String),
    /// Video-only properties were assigned to a non-video stream.
    #[error("video shape assigned to non-video target stream: {0}")]
    VideoShapeOnNonVideoStream(String),
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

/// Compile an immutable target and an independent unmatched-stream policy into a desired graph.
///
/// Target rows are evaluated in declared order. Each row consumes the first still-unmatched
/// source stream satisfying its kind, semantic role, and language selectors. Source stream ids
/// are therefore runtime correlation values only; they are never part of persisted target identity.
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
    validate_target(target)?;

    let mut consumed = BTreeSet::new();
    let mut desired_streams = Vec::with_capacity(target.streams.len());
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
        let source_stream = source
            .streams
            .iter()
            .find(|stream| !consumed.contains(&stream.stream_id) && matches(stream, target_stream));
        match source_stream {
            Some(stream) => {
                consumed.insert(stream.stream_id);
                if target_stream.subtitle_placement != Some(SubtitlePlacement::None) {
                    desired_streams.push(apply_target_stream(stream, target_stream));
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
        .filter(|stream| !consumed.contains(&stream.stream_id))
    {
        match unmatched_policy {
            UnmatchedStreamPolicy::Remove => {}
            UnmatchedStreamPolicy::Preserve => desired_streams.push(stream.clone()),
            UnmatchedStreamPolicy::Reject => {
                return Err(TargetCompileError::UnmatchedSourceStream(stream.stream_id));
            }
        }
    }

    Ok(DesiredGraph {
        output_path: output_path.to_string(),
        container_format: Some(normalize_container_format(&target.container)),
        streams: desired_streams,
    })
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
    validate_target(target)?;
    let mut consumed_streams = BTreeSet::new();
    let mut consumed_sidecars = BTreeSet::new();
    let mut state = TargetCompilationState::new(target.streams.len());
    let mut next_stream_id = source
        .streams
        .iter()
        .map(|stream| stream.stream_id)
        .max()
        .map_or(Ok(0), |value| {
            value
                .checked_add(1)
                .ok_or(TargetCompileError::SidecarStreamIdentityOverflow)
        })?;

    for target_stream in &target.streams {
        if target_stream.kind != StreamKind::Subtitle {
            compile_primary_stream(
                source,
                target_stream,
                &mut consumed_streams,
                &mut state.desired_streams,
            )?;
            continue;
        }

        let embedded = source.streams.iter().find(|stream| {
            !consumed_streams.contains(&stream.stream_id) && matches(stream, target_stream)
        });
        let sidecar = sidecars.iter().enumerate().find(|(index, sidecar)| {
            !consumed_sidecars.contains(index) && sidecar_matches(sidecar, target_stream)
        });
        match (embedded, sidecar) {
            (Some(stream), _) => {
                consumed_streams.insert(stream.stream_id);
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
                    next_stream_id,
                    &mut state,
                )?;
                next_stream_id = next_stream_id
                    .checked_add(1)
                    .ok_or(TargetCompileError::SidecarStreamIdentityOverflow)?;
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
        unmatched_policy,
        &consumed_streams,
        &mut state.desired_streams,
    )?;
    append_unmatched_sidecars(
        sidecars,
        unmatched_policy,
        &consumed_sidecars,
        &mut state.removals,
    )?;
    Ok(CompiledDesiredTarget {
        graph: DesiredGraph {
            output_path: output_path.to_string(),
            container_format: Some(normalize_container_format(&target.container)),
            streams: state.desired_streams,
        },
        sidecar_embeddings: state.embeddings,
        sidecar_outputs: state.outputs,
        sidecar_removals: state.removals.into_iter().collect(),
    })
}

struct TargetCompilationState {
    desired_streams: Vec<MediaStream>,
    embeddings: Vec<SidecarEmbedding>,
    outputs: Vec<DesiredSidecarOutput>,
    removals: BTreeSet<String>,
    output_paths: BTreeSet<String>,
}

impl TargetCompilationState {
    fn new(stream_capacity: usize) -> Self {
        Self {
            desired_streams: Vec::with_capacity(stream_capacity),
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
    consumed: &mut BTreeSet<u32>,
    desired: &mut Vec<MediaStream>,
) -> Result<(), TargetCompileError> {
    let selected = source
        .streams
        .iter()
        .find(|stream| !consumed.contains(&stream.stream_id) && matches(stream, target));
    match selected {
        Some(stream) => {
            consumed.insert(stream.stream_id);
            desired.push(apply_target_stream(stream, target));
            Ok(())
        }
        None if target.optional => Ok(()),
        None => Err(TargetCompileError::RequiredStreamMissing(
            target.stream_key.clone(),
        )),
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
        state
            .desired_streams
            .push(apply_target_stream(source, target));
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
    stream_id: u32,
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
        let output_stream = apply_sidecar_target(source, target, stream_id);
        state.desired_streams.push(output_stream.clone());
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
    let language = target.language.as_deref().unwrap_or("und");
    let role = target.role.and_then(role_path_token);
    let suffix = role.map_or_else(
        || format!("{language}.{extension}"),
        |role| format!("{language}.{role}.{extension}"),
    );
    let path = media_path.with_file_name(format!("{stem}.{suffix}"));
    let path = path.to_str().map(str::to_string).ok_or_else(|| {
        TargetCompileError::InvalidSidecarOutputPath(media_output_path.to_string())
    })?;
    Ok(path)
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
    policy: UnmatchedStreamPolicy,
    consumed: &BTreeSet<u32>,
    desired: &mut Vec<MediaStream>,
) -> Result<(), TargetCompileError> {
    for stream in source
        .streams
        .iter()
        .filter(|stream| !consumed.contains(&stream.stream_id))
    {
        match policy {
            UnmatchedStreamPolicy::Remove => {}
            UnmatchedStreamPolicy::Preserve => desired.push(stream.clone()),
            UnmatchedStreamPolicy::Reject => {
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
        && target.language.as_deref().is_none_or(|language| {
            source
                .language
                .as_deref()
                .is_some_and(|source_language| language_matches(source_language, language))
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
        language: target.language.clone().or_else(|| source.language.clone()),
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

fn validate_target_stream(stream: &TargetStream, key: &str) -> Result<(), TargetCompileError> {
    if stream.codec.trim().is_empty() {
        return Err(TargetCompileError::EmptyCodec(key.to_string()));
    }
    if matches!(
        stream.kind,
        StreamKind::Attachment | StreamKind::Chapter | StreamKind::Data
    ) {
        return Err(TargetCompileError::UnsupportedDesiredStreamKind(
            key.to_string(),
        ));
    }
    if stream.kind != StreamKind::Audio && has_audio_shape(stream) {
        return Err(TargetCompileError::AudioShapeOnNonAudioStream(
            key.to_string(),
        ));
    }
    if stream.kind == StreamKind::Audio && has_invalid_audio_policy(stream) {
        return Err(TargetCompileError::AudioShapeOnNonAudioStream(
            key.to_string(),
        ));
    }
    if stream.kind != StreamKind::Video && has_video_shape(stream) {
        return Err(TargetCompileError::VideoShapeOnNonVideoStream(
            key.to_string(),
        ));
    }
    if stream.kind == StreamKind::Video
        && let Some(hdr_format) = stream.hdr_format.as_deref()
        && !hdr_format.trim().eq_ignore_ascii_case("hdr10")
    {
        return Err(TargetCompileError::UnsupportedHdrFormat {
            stream_key: key.to_string(),
            hdr_format: hdr_format.to_string(),
        });
    }
    if stream.kind == StreamKind::Video
        && let Some(video_level) = stream.video_level.as_deref()
        && !is_known_video_level(&stream.codec, video_level)
    {
        return Err(TargetCompileError::UnsupportedVideoLevel {
            stream_key: key.to_string(),
            codec: stream.codec.clone(),
            video_level: video_level.to_string(),
        });
    }
    if stream.kind != StreamKind::Subtitle
        && (stream.subtitle_placement.is_some() || stream.image_subtitle_action.is_some())
    {
        return Err(TargetCompileError::SubtitleShapeOnNonSubtitleStream(
            key.to_string(),
        ));
    }
    if stream.kind == StreamKind::Subtitle && stream.role == Some(SemanticRole::DescriptiveAudio) {
        return Err(TargetCompileError::InvalidSubtitleRole(key.to_string()));
    }
    Ok(())
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

fn is_known_video_level(codec: &str, level: &str) -> bool {
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
        && target.language.as_deref().is_none_or(|language| {
            source
                .language
                .as_deref()
                .is_some_and(|source_language| language_matches(source_language, language))
        })
}

fn language_matches(source: &str, target: &str) -> bool {
    source.trim().eq_ignore_ascii_case(target.trim())
}

fn apply_target_stream(source: &MediaStream, target: &TargetStream) -> MediaStream {
    MediaStream {
        stream_id: source.stream_id,
        kind: source.kind,
        codec: target.codec.trim().to_ascii_lowercase(),
        channels: target.channels,
        channel_layout: target.channel_layout.clone(),
        language: target
            .language
            .as_deref()
            .map(str::trim)
            .filter(|language| !language.is_empty())
            .map(str::to_ascii_lowercase)
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
        DesiredTarget, ImageSubtitleAction, SidecarOutputSource, SidecarSubtitleInput,
        SubtitlePlacement, TargetCompileError, TargetStream, UnmatchedStreamPolicy,
        compile_desired_target, compile_desired_target_with_sidecars,
        compile_desired_target_with_sidecars_at,
    };
    use crate::classify::SemanticRole;
    use crate::model::{MediaGraph, MediaStream, StreamKind};

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
            language: language.map(str::to_string),
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
            desired
                .streams
                .iter()
                .map(|stream| (stream.stream_id, stream.codec.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (3, "png"),
                (7, "opus"),
                (5, "av1"),
                (9, "aac"),
                (13, "webvtt")
            ]
        );
        assert_eq!(desired.streams[1].channels, Some(2));
        assert_eq!(desired.streams[1].channel_layout.as_deref(), Some("stereo"));
        assert_eq!(desired.streams[4].dispositions, vec!["forced"]);
        Ok(())
    }

    #[test]
    fn optional_rows_skip_and_preserve_policy_appends_unmatched_streams()
    -> Result<(), TargetCompileError> {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![stream(2, StreamKind::Video, "h264", None, None, &[])],
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
            streams: vec![optional_audio],
        };

        let desired = compile_desired_target(
            &source,
            "/output/movie.mkv",
            &target,
            UnmatchedStreamPolicy::Preserve,
        )?;

        assert_eq!(desired.streams, source.streams);
        Ok(())
    }

    #[test]
    fn missing_required_row_and_rejected_unmatched_stream_are_errors() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![stream(2, StreamKind::Video, "h264", None, None, &[])],
        };
        let target = DesiredTarget {
            target_key: "strict".to_string(),
            version: 1,
            container: "matroska".to_string(),
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
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let first = target_stream("main", StreamKind::Video, None, None, "hevc");
        let duplicate = target_stream("MAIN", StreamKind::Video, None, None, "av1");
        let duplicate_target = DesiredTarget {
            target_key: "invalid".to_string(),
            version: 1,
            container: "matroska".to_string(),
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
            ..invalid_shape_target.clone()
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

        let unsupported_target = DesiredTarget {
            streams: vec![target_stream(
                "chapter-main",
                StreamKind::Chapter,
                None,
                None,
                "bin_data",
            )],
            ..invalid_shape_target
        };
        assert_eq!(
            compile_desired_target(
                &source,
                "/output/movie.mkv",
                &unsupported_target,
                UnmatchedStreamPolicy::Remove,
            ),
            Err(TargetCompileError::UnsupportedDesiredStreamKind(
                "chapter-main".to_string()
            ))
        );
    }

    #[test]
    fn target_validation_rejects_unknown_video_level() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let mut unsupported_level = target_stream("video", StreamKind::Video, None, None, "hevc");
        unsupported_level.video_level = Some("7.9".to_string());
        let unsupported_level_target = DesiredTarget {
            target_key: "invalid-level".to_string(),
            version: 1,
            container: "matroska".to_string(),
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
            container_formats: Vec::new(),
            streams: Vec::new(),
        };
        let unsupported_target = DesiredTarget {
            target_key: "unsupported-data".to_string(),
            version: 1,
            container: "matroska".to_string(),
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
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream(2, StreamKind::Video, "h264", None, None, &[])],
        };
        let target = DesiredTarget {
            target_key: "embed-sidecar".to_string(),
            version: 1,
            container: "matroska".to_string(),
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
        assert_eq!(compiled.graph.streams[1].stream_id, 3);
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
            container_formats: vec!["matroska".to_string()],
            streams: vec![stream(2, StreamKind::Video, "h264", None, None, &[])],
        };
        let target = DesiredTarget {
            target_key: "video-only".to_string(),
            version: 1,
            container: "matroska".to_string(),
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
}
