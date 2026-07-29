//! Inspection adapter interfaces.

use revaer_media_core::model::{MediaGraph, MediaStream, StreamKind};
use revaer_media_core::normalize::{normalize_graph, normalize_subtitle_codec};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use thiserror::Error;

use crate::sidecar::{
    FilesystemSidecarDiscoverer, SidecarDiscoverer, SidecarDiscoveryError, SidecarFormat,
    SidecarSubtitle,
};

/// Error emitted by inspect adapters.
#[derive(Debug, Error)]
pub enum InspectError {
    /// Adapter failed with details.
    #[error("inspect adapter failure: {0}")]
    Adapter(String),
    /// Invalid stream kind from adapter input.
    #[error("invalid stream kind: {0}")]
    InvalidStreamKind(String),
    /// Probe command failed with details.
    #[error("inspect probe command failed: {0}")]
    ProbeFailed(String),
    /// Probe output failed to parse.
    #[error("inspect probe output malformed: {0}")]
    OutputMalformed(String),
    /// Adjacent sidecar discovery failed.
    #[error(transparent)]
    Sidecar(#[from] SidecarDiscoveryError),
}

/// Inspect media and return a parsed graph.
pub trait InspectAdapter {
    /// Inspect the path and return a deterministic media graph.
    ///
    /// # Errors
    ///
    /// Returns [`InspectError`] when the adapter cannot inspect or parse the source media.
    fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError>;

    /// Inspect the path and return the complete normalized technical report.
    ///
    /// # Errors
    ///
    /// Returns [`InspectError`] when the adapter cannot inspect or parse the source media.
    fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
        self.inspect(source_path).map(MediaInspection::from_graph)
    }
}

/// One normalized metadata key/value pair.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MetadataEntry {
    /// Lowercase metadata key.
    pub key: String,
    /// Trimmed metadata value.
    pub value: String,
}

/// One normalized stream side-data record.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SideDataInspection {
    /// Lowercase side-data type.
    pub side_data_type: String,
    /// Normalized side-data payload fields.
    pub metadata: Vec<MetadataEntry>,
}

/// Normalized container-level inspection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerInspection {
    /// Canonical format names reported by the demuxer.
    pub formats: Vec<String>,
    /// Container duration in milliseconds when known.
    pub duration_millis: Option<u64>,
    /// Container start time in milliseconds when known.
    pub start_time_millis: Option<i64>,
    /// Container size in bytes when reported by the probe.
    pub size_bytes: Option<u64>,
    /// Aggregate container bitrate in bits per second when known.
    pub bit_rate: Option<u64>,
    /// Normalized container metadata.
    pub metadata: Vec<MetadataEntry>,
}

/// Normalized chapter timeline entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterInspection {
    /// Probe chapter identifier.
    pub chapter_id: u32,
    /// Inclusive chapter start in milliseconds.
    pub start_millis: i64,
    /// Exclusive chapter end in milliseconds.
    pub end_millis: i64,
    /// Normalized chapter metadata.
    pub metadata: Vec<MetadataEntry>,
}

/// Technical properties retained for one normalized stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamInspection {
    /// Stream identifier matching [`revaer_media_core::model::MediaStream::stream_id`].
    pub stream_id: u32,
    /// Codec profile when reported.
    pub profile: Option<String>,
    /// Stream duration in milliseconds when known.
    pub duration_millis: Option<u64>,
    /// Stream bitrate in bits per second when known.
    pub bit_rate: Option<u64>,
    /// Audio sample rate in hertz when known.
    pub sample_rate: Option<u32>,
    /// Video width in pixels when applicable.
    pub width: Option<u32>,
    /// Video height in pixels when applicable.
    pub height: Option<u32>,
    /// Pixel format when applicable.
    pub pixel_format: Option<String>,
    /// Sample aspect ratio when applicable.
    pub sample_aspect_ratio: Option<String>,
    /// Display aspect ratio when applicable.
    pub display_aspect_ratio: Option<String>,
    /// Average frame rate as an unreduced probe fraction when applicable.
    pub average_frame_rate: Option<String>,
    /// Color range when reported.
    pub color_range: Option<String>,
    /// Color space when reported.
    pub color_space: Option<String>,
    /// Color transfer characteristic when reported.
    pub color_transfer: Option<String>,
    /// Color primaries when reported.
    pub color_primaries: Option<String>,
    /// Chroma location when reported.
    pub chroma_location: Option<String>,
    /// Field order when reported.
    pub field_order: Option<String>,
    /// Normalized stream metadata, including provider and edition tags.
    pub metadata: Vec<MetadataEntry>,
    /// Normalized side-data type names, including HDR and Dolby Vision descriptors.
    pub side_data_types: Vec<String>,
    /// Normalized side-data records, including HDR value payloads when reported.
    pub side_data: Vec<SideDataInspection>,
}

/// Complete normalized media inspection report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaInspection {
    /// Planning graph retained for existing diff and plan contracts.
    pub graph: MediaGraph,
    /// Container-level state.
    pub container: ContainerInspection,
    /// Technical stream details in stream order.
    pub streams: Vec<StreamInspection>,
    /// Chapters in timeline order.
    pub chapters: Vec<ChapterInspection>,
    /// Existing adjacent subtitle sidecars in deterministic filename order.
    pub sidecars: Vec<SidecarSubtitle>,
}

impl MediaInspection {
    const fn from_graph(graph: MediaGraph) -> Self {
        Self {
            graph,
            container: ContainerInspection {
                formats: Vec::new(),
                duration_millis: None,
                start_time_millis: None,
                size_bytes: None,
                bit_rate: None,
                metadata: Vec::new(),
            },
            streams: Vec::new(),
            chapters: Vec::new(),
            sidecars: Vec::new(),
        }
    }
}

/// Command probe abstraction for inspection adapters.
pub trait InspectProbeExecutor: Send + Sync {
    /// Execute one command and return UTF-8 stdout.
    ///
    /// # Errors
    ///
    /// Returns [`InspectError::ProbeFailed`] when command execution fails or exits non-zero.
    /// Returns [`InspectError::OutputMalformed`] when stdout is not valid UTF-8.
    fn run(&self, bin: &str, args: &[&str]) -> Result<String, InspectError>;
}

/// System command probe executor.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemInspectProbeExecutor;

impl InspectProbeExecutor for SystemInspectProbeExecutor {
    fn run(&self, bin: &str, args: &[&str]) -> Result<String, InspectError> {
        let output = Command::new(bin)
            .args(args)
            .output()
            .map_err(|err| InspectError::ProbeFailed(err.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stderr.is_empty() {
                format!("{bin} exited with status {}", output.status)
            } else {
                format!("{bin} exited with status {}: {stderr}", output.status)
            };
            return Err(InspectError::ProbeFailed(message));
        }
        String::from_utf8(output.stdout)
            .map_err(|err| InspectError::OutputMalformed(err.to_string()))
    }
}

/// ffprobe-backed inspection adapter.
pub struct FfprobeInspectAdapter {
    executor: Arc<dyn InspectProbeExecutor>,
    sidecar_discoverer: Arc<dyn SidecarDiscoverer>,
    ffprobe_bin: String,
}

impl FfprobeInspectAdapter {
    /// Construct adapter with injected probe executor and binary name.
    pub fn new(executor: Arc<dyn InspectProbeExecutor>, ffprobe_bin: impl Into<String>) -> Self {
        Self {
            executor,
            sidecar_discoverer: Arc::new(FilesystemSidecarDiscoverer),
            ffprobe_bin: ffprobe_bin.into(),
        }
    }

    /// Construct an adapter with injected probe and sidecar discovery boundaries.
    pub fn with_sidecar_discoverer(
        executor: Arc<dyn InspectProbeExecutor>,
        ffprobe_bin: impl Into<String>,
        sidecar_discoverer: Arc<dyn SidecarDiscoverer>,
    ) -> Self {
        Self {
            executor,
            sidecar_discoverer,
            ffprobe_bin: ffprobe_bin.into(),
        }
    }
}

impl InspectAdapter for FfprobeInspectAdapter {
    fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
        self.inspect_full(source_path)
            .map(|inspection| inspection.graph)
    }

    fn inspect_full(&self, source_path: &str) -> Result<MediaInspection, InspectError> {
        let args = [
            "-v",
            "error",
            "-read_intervals",
            "%+#1",
            "-show_streams",
            "-show_frames",
            "-show_format",
            "-show_chapters",
            "-of",
            "json",
            source_path,
        ];
        let output = self.executor.run(&self.ffprobe_bin, &args)?;
        let parsed: FfprobeOutput = serde_json::from_str(&output)
            .map_err(|err| InspectError::OutputMalformed(err.to_string()))?;
        let frame_side_data = frame_side_data_by_stream(&parsed.frames);
        let technical_streams = parsed
            .streams
            .iter()
            .map(|stream| normalize_stream_inspection(stream, &frame_side_data))
            .collect::<Result<Vec<_>, _>>()?;
        let probe = ProbeGraph {
            source_path: source_path.to_string(),
            streams: parsed
                .streams
                .into_iter()
                .map(|item| {
                    let (language, title) = match item.tags {
                        Some(tags) => (tags.language, tags.title),
                        None => (None, None),
                    };
                    ProbeStream {
                        stream_id: item.index,
                        kind: item.codec_type,
                        codec: item.codec_name,
                        channels: item.channels,
                        channel_layout: item.channel_layout,
                        language,
                        title,
                        dispositions: dispositions_from_raw(item.disposition),
                    }
                })
                .collect(),
        };
        let mut graph = normalize_probe_graph(probe)?;
        let container = normalize_container(parsed.format)?;
        graph.container_formats.clone_from(&container.formats);
        let chapters = parsed
            .chapters
            .into_iter()
            .map(normalize_chapter)
            .collect::<Result<Vec<_>, _>>()?;
        let sidecars = self.sidecar_discoverer.discover(Path::new(source_path))?;
        validate_sidecars(self.executor.as_ref(), &self.ffprobe_bin, &sidecars)?;
        Ok(MediaInspection {
            graph,
            container,
            streams: technical_streams,
            chapters,
            sidecars,
        })
    }
}

fn validate_sidecars(
    executor: &dyn InspectProbeExecutor,
    ffprobe_bin: &str,
    sidecars: &[SidecarSubtitle],
) -> Result<(), InspectError> {
    for sidecar in sidecars {
        let path = sidecar.path.to_str().ok_or_else(|| {
            InspectError::OutputMalformed("sidecar path is not valid UTF-8".to_string())
        })?;
        let args = ["-v", "error", "-show_streams", "-of", "json", path];
        let output = executor.run(ffprobe_bin, &args)?;
        let parsed: FfprobeOutput = serde_json::from_str(&output)
            .map_err(|error| InspectError::OutputMalformed(error.to_string()))?;
        let [stream] = parsed.streams.as_slice() else {
            return Err(InspectError::OutputMalformed(format!(
                "sidecar must contain exactly one subtitle stream: {path}"
            )));
        };
        if !stream.codec_type.eq_ignore_ascii_case("subtitle") {
            return Err(InspectError::OutputMalformed(format!(
                "sidecar does not contain a subtitle stream: {path}"
            )));
        }
        let actual_codec = normalize_subtitle_codec(&stream.codec_name);
        let expected_codec = expected_sidecar_codec(sidecar.format);
        if actual_codec != expected_codec {
            return Err(InspectError::OutputMalformed(format!(
                "sidecar codec does not match its declared format: path={path} expected={expected_codec} actual={actual_codec}"
            )));
        }
    }
    Ok(())
}

const fn expected_sidecar_codec(format: SidecarFormat) -> &'static str {
    match format {
        SidecarFormat::Srt => "subrip",
        SidecarFormat::Ass => "ass",
        SidecarFormat::Vtt => "webvtt",
        SidecarFormat::Sup => "hdmv_pgs_subtitle",
        SidecarFormat::Sub => "microdvd",
        SidecarFormat::VobSub => "dvd_subtitle",
    }
}

/// Probe-like stream shape accepted by normalizers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeStream {
    /// Stream id in source container.
    pub stream_id: u32,
    /// Stream kind (`video`, `audio`, `subtitle`, `attachment`, `chapter`).
    pub kind: String,
    /// Codec identifier.
    pub codec: String,
    /// Audio channel count when known.
    pub channels: Option<u32>,
    /// Audio channel layout when known.
    pub channel_layout: Option<String>,
    /// Optional language code.
    pub language: Option<String>,
    /// Optional title.
    pub title: Option<String>,
    /// Raw dispositions.
    pub dispositions: Vec<String>,
}

/// Probe-like graph shape accepted by normalizers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeGraph {
    /// Source path from inspection context.
    pub source_path: String,
    /// Raw stream list.
    pub streams: Vec<ProbeStream>,
}

/// Convert probe-like output into normalized domain graph.
///
/// # Errors
///
/// Returns [`InspectError::InvalidStreamKind`] when a stream kind is not recognized.
pub fn normalize_probe_graph(input: ProbeGraph) -> Result<MediaGraph, InspectError> {
    let mut streams = Vec::with_capacity(input.streams.len());
    for stream in input.streams {
        let kind = parse_stream_kind(&stream.kind)?;
        let codec = stream.codec.trim().to_ascii_lowercase();
        if codec.is_empty() {
            return Err(InspectError::OutputMalformed(
                "stream codec is missing".to_string(),
            ));
        }
        streams.push(MediaStream {
            stream_id: stream.stream_id,
            kind,
            codec,
            channels: channel_count_for_stream(kind, stream.channels),
            channel_layout: channel_layout_for_stream(kind, stream.channel_layout.as_deref()),
            language: stream
                .language
                .as_deref()
                .and_then(normalize_optional_language),
            title: stream.title.as_deref().and_then(normalize_optional_title),
            dispositions: stream
                .dispositions
                .into_iter()
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| !value.is_empty())
                .collect(),
        });
    }

    let graph = MediaGraph {
        source_path: input.source_path,
        container_formats: Vec::new(),
        streams,
    };
    Ok(normalize_graph(&graph))
}

fn parse_stream_kind(value: &str) -> Result<StreamKind, InspectError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "video" => Ok(StreamKind::Video),
        "audio" => Ok(StreamKind::Audio),
        "subtitle" => Ok(StreamKind::Subtitle),
        "attachment" => Ok(StreamKind::Attachment),
        "chapter" => Ok(StreamKind::Chapter),
        "data" => Ok(StreamKind::Data),
        _ => Err(InspectError::InvalidStreamKind(normalized)),
    }
}

fn normalize_optional_language(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_optional_title(value: &str) -> Option<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn channel_count_for_stream(kind: StreamKind, channels: Option<u32>) -> Option<u32> {
    if kind == StreamKind::Audio {
        channels.filter(|value| *value > 0)
    } else {
        None
    }
}

fn channel_layout_for_stream(kind: StreamKind, channel_layout: Option<&str>) -> Option<String> {
    if kind != StreamKind::Audio {
        return None;
    }
    channel_layout
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}

fn normalize_stream_inspection(
    stream: &FfprobeStream,
    frame_side_data: &BTreeMap<u32, Vec<&FfprobeSideData>>,
) -> Result<StreamInspection, InspectError> {
    let empty_side_data = Vec::new();
    let frame_side_data = frame_side_data
        .get(&stream.index)
        .map_or(empty_side_data.as_slice(), Vec::as_slice);
    Ok(StreamInspection {
        stream_id: stream.index,
        profile: normalize_optional_text(stream.profile.as_deref()),
        duration_millis: parse_optional_duration_millis(stream.duration.as_deref())?,
        bit_rate: parse_optional_u64(stream.bit_rate.as_deref(), "stream bit rate")?,
        sample_rate: sample_rate_for_stream(&stream.codec_type, stream.sample_rate.as_deref())?,
        width: stream.width.filter(|value| *value > 0),
        height: stream.height.filter(|value| *value > 0),
        pixel_format: normalize_lowercase_text(stream.pix_fmt.as_deref()),
        sample_aspect_ratio: normalize_optional_text(stream.sample_aspect_ratio.as_deref()),
        display_aspect_ratio: normalize_optional_text(stream.display_aspect_ratio.as_deref()),
        average_frame_rate: normalize_optional_fraction(stream.avg_frame_rate.as_deref()),
        color_range: normalize_lowercase_text(stream.color_range.as_deref()),
        color_space: normalize_lowercase_text(stream.color_space.as_deref()),
        color_transfer: normalize_lowercase_text(stream.color_transfer.as_deref()),
        color_primaries: normalize_lowercase_text(stream.color_primaries.as_deref()),
        chroma_location: normalize_lowercase_text(stream.chroma_location.as_deref()),
        field_order: normalize_lowercase_text(stream.field_order.as_deref()),
        metadata: metadata_from_stream(stream),
        side_data_types: normalize_side_data(&stream.side_data_list, frame_side_data),
        side_data: normalize_side_data_records(&stream.side_data_list, frame_side_data),
    })
}

fn normalize_container(format: Option<FfprobeFormat>) -> Result<ContainerInspection, InspectError> {
    let Some(format) = format else {
        return Err(InspectError::OutputMalformed(
            "container format is missing".to_string(),
        ));
    };
    let mut formats = format
        .format_name
        .split(',')
        .filter_map(|value| normalize_lowercase_text(Some(value)))
        .collect::<Vec<_>>();
    formats.sort();
    formats.dedup();
    if formats.is_empty() {
        return Err(InspectError::OutputMalformed(
            "container format name is missing".to_string(),
        ));
    }
    Ok(ContainerInspection {
        formats,
        duration_millis: parse_optional_duration_millis(format.duration.as_deref())?,
        start_time_millis: parse_optional_millis(format.start_time.as_deref(), "start time")?,
        size_bytes: parse_optional_u64(format.size.as_deref(), "container size")?,
        bit_rate: parse_optional_u64(format.bit_rate.as_deref(), "container bit rate")?,
        metadata: normalize_metadata(format.tags),
    })
}

fn normalize_chapter(chapter: FfprobeChapter) -> Result<ChapterInspection, InspectError> {
    let start_millis = parse_required_millis(&chapter.start_time, "chapter start time")?;
    let end_millis = parse_required_millis(&chapter.end_time, "chapter end time")?;
    if end_millis <= start_millis {
        return Err(InspectError::OutputMalformed(
            "chapter end time must be after start time".to_string(),
        ));
    }
    Ok(ChapterInspection {
        chapter_id: chapter.id,
        start_millis,
        end_millis,
        metadata: normalize_metadata(chapter.tags),
    })
}

fn normalize_metadata(values: BTreeMap<String, String>) -> Vec<MetadataEntry> {
    values
        .into_iter()
        .filter_map(|(key, value)| metadata_entry(&key, &value))
        .collect()
}

fn metadata_from_stream(stream: &FfprobeStream) -> Vec<MetadataEntry> {
    let mut values = stream.tags.as_ref().map_or_else(BTreeMap::new, |tags| {
        let mut values = tags.extra.clone();
        if let Some(language) = &tags.language {
            values.insert("language".to_string(), language.clone());
        }
        if let Some(title) = &tags.title {
            values.insert("title".to_string(), title.clone());
        }
        values
    });
    if let Some(level) = stream.level.as_ref().and_then(side_data_scalar_text) {
        values.insert("level".to_string(), level);
    }
    normalize_metadata(values)
}

fn metadata_entry(key: &str, value: &str) -> Option<MetadataEntry> {
    let key = key.trim().to_ascii_lowercase();
    let value = value.trim().to_string();
    (!key.is_empty() && !value.is_empty()).then_some(MetadataEntry { key, value })
}

fn normalize_side_data(
    stream_values: &[FfprobeSideData],
    frame_values: &[&FfprobeSideData],
) -> Vec<String> {
    let mut normalized = stream_values
        .iter()
        .filter_map(|value| normalize_lowercase_text(Some(&value.side_data_type)))
        .collect::<Vec<_>>();
    normalized.extend(
        frame_values
            .iter()
            .filter_map(|value| normalize_lowercase_text(Some(&value.side_data_type))),
    );
    normalized.sort();
    normalized.dedup();
    normalized
}

fn normalize_side_data_records(
    stream_values: &[FfprobeSideData],
    frame_values: &[&FfprobeSideData],
) -> Vec<SideDataInspection> {
    let mut merged: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for value in stream_values {
        merge_side_data_record(&mut merged, value);
    }
    for value in frame_values {
        merge_side_data_record(&mut merged, value);
    }
    let mut normalized = merged
        .into_iter()
        .map(|(side_data_type, values)| SideDataInspection {
            side_data_type,
            metadata: normalize_metadata(values),
        })
        .collect::<Vec<_>>();
    normalized.sort_by(|left, right| {
        left.side_data_type
            .cmp(&right.side_data_type)
            .then_with(|| left.metadata.cmp(&right.metadata))
    });
    normalized.dedup();
    normalized
}

fn merge_side_data_record(
    target: &mut BTreeMap<String, BTreeMap<String, String>>,
    value: &FfprobeSideData,
) {
    let Some(side_data_type) = normalize_lowercase_text(Some(&value.side_data_type)) else {
        return;
    };
    let values = target.entry(side_data_type).or_default();
    for entry in normalize_side_data_metadata(&value.extra) {
        values.insert(entry.key, entry.value);
    }
}

fn frame_side_data_by_stream(frames: &[FfprobeFrame]) -> BTreeMap<u32, Vec<&FfprobeSideData>> {
    let mut values: BTreeMap<u32, Vec<&FfprobeSideData>> = BTreeMap::new();
    for frame in frames {
        values
            .entry(frame.stream_index)
            .or_default()
            .extend(frame.side_data_list.iter());
    }
    values
}

fn normalize_side_data_metadata(values: &BTreeMap<String, Value>) -> Vec<MetadataEntry> {
    values
        .iter()
        .filter_map(|(key, value)| {
            side_data_scalar_text(value).and_then(|text| metadata_entry(key, &text))
        })
        .collect()
}

fn side_data_scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("n/a"))
        .map(str::to_string)
}

fn normalize_lowercase_text(value: Option<&str>) -> Option<String> {
    normalize_optional_text(value).map(|value| value.to_ascii_lowercase())
}

fn normalize_optional_fraction(value: Option<&str>) -> Option<String> {
    normalize_optional_text(value).filter(|value| value != "0/0")
}

fn parse_optional_duration_millis(value: Option<&str>) -> Result<Option<u64>, InspectError> {
    let millis = parse_optional_millis(value, "duration")?;
    match millis {
        Some(value) if value < 0 => Err(InspectError::OutputMalformed(
            "duration must not be negative".to_string(),
        )),
        Some(value) => u64::try_from(value)
            .map(Some)
            .map_err(|error| InspectError::OutputMalformed(error.to_string())),
        None => Ok(None),
    }
}

fn parse_optional_millis(
    value: Option<&str>,
    label: &'static str,
) -> Result<Option<i64>, InspectError> {
    normalize_optional_text(value)
        .map(|value| parse_required_millis(&value, label))
        .transpose()
}

fn parse_required_millis(value: &str, label: &'static str) -> Result<i64, InspectError> {
    let trimmed = value.trim();
    let (negative, unsigned) = trimmed
        .strip_prefix('-')
        .map_or((false, trimmed), |value| (true, value));
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let whole_seconds = whole
        .parse::<i64>()
        .map_err(|_| InspectError::OutputMalformed(format!("{label} is not a decimal number")))?;
    let fraction_digits = fraction
        .chars()
        .take(3)
        .map(|character| character.to_digit(10))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| InspectError::OutputMalformed(format!("{label} is not a decimal number")))?;
    let fraction_value = fraction_digits
        .iter()
        .fold(0_i64, |value, digit| value * 10 + i64::from(*digit));
    let fraction_scale = match fraction_digits.len() {
        0 => 1000,
        1 => 100,
        2 => 10,
        3 => 1,
        _ => {
            return Err(InspectError::OutputMalformed(format!(
                "{label} has invalid precision"
            )));
        }
    };
    let unsigned_millis = whole_seconds
        .checked_mul(1000)
        .and_then(|value| value.checked_add(fraction_value * fraction_scale))
        .ok_or_else(|| InspectError::OutputMalformed(format!("{label} is out of range")))?;
    if negative {
        unsigned_millis
            .checked_neg()
            .ok_or_else(|| InspectError::OutputMalformed(format!("{label} is out of range")))
    } else {
        Ok(unsigned_millis)
    }
}

fn parse_optional_u64(
    value: Option<&str>,
    label: &'static str,
) -> Result<Option<u64>, InspectError> {
    normalize_optional_text(value)
        .map(|value| {
            value.parse::<u64>().map_err(|_| {
                InspectError::OutputMalformed(format!("{label} is not an unsigned integer"))
            })
        })
        .transpose()
}

fn sample_rate_for_stream(kind: &str, value: Option<&str>) -> Result<Option<u32>, InspectError> {
    if !kind.eq_ignore_ascii_case("audio") {
        return Ok(None);
    }
    parse_optional_u64(value, "sample rate").and_then(|parsed| {
        parsed
            .map(|sample_rate| {
                u32::try_from(sample_rate)
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| {
                        InspectError::OutputMalformed("sample rate is out of range".to_string())
                    })
            })
            .transpose()
    })
}

fn dispositions_from_raw(raw: Option<FfprobeDisposition>) -> Vec<String> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    let mut dispositions = Vec::new();
    if raw.default == Some(1) {
        dispositions.push("default".to_string());
    }
    if raw.forced == Some(1) {
        dispositions.push("forced".to_string());
    }
    if raw.hearing_impaired == Some(1) {
        dispositions.push("hearing_impaired".to_string());
    }
    if raw.visual_impaired == Some(1) {
        dispositions.push("visual_impaired".to_string());
    }
    dispositions
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
    #[serde(default)]
    frames: Vec<FfprobeFrame>,
    #[serde(default)]
    chapters: Vec<FfprobeChapter>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    index: u32,
    codec_type: String,
    codec_name: String,
    channels: Option<u32>,
    channel_layout: Option<String>,
    disposition: Option<FfprobeDisposition>,
    tags: Option<FfprobeTags>,
    profile: Option<String>,
    duration: Option<String>,
    bit_rate: Option<String>,
    sample_rate: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: Option<String>,
    sample_aspect_ratio: Option<String>,
    display_aspect_ratio: Option<String>,
    avg_frame_rate: Option<String>,
    color_range: Option<String>,
    color_space: Option<String>,
    color_transfer: Option<String>,
    color_primaries: Option<String>,
    chroma_location: Option<String>,
    field_order: Option<String>,
    level: Option<Value>,
    #[serde(default)]
    side_data_list: Vec<FfprobeSideData>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFrame {
    stream_index: u32,
    #[serde(default)]
    side_data_list: Vec<FfprobeSideData>,
}

#[derive(Debug, Deserialize)]
struct FfprobeDisposition {
    default: Option<u8>,
    forced: Option<u8>,
    hearing_impaired: Option<u8>,
    visual_impaired: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct FfprobeTags {
    language: Option<String>,
    title: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    format_name: String,
    duration: Option<String>,
    start_time: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
    #[serde(default)]
    tags: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeChapter {
    id: u32,
    start_time: String,
    end_time: String,
    #[serde(default)]
    tags: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeSideData {
    side_data_type: String,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::{
        FfprobeInspectAdapter, InspectAdapter, InspectError, InspectProbeExecutor, ProbeGraph,
        ProbeStream, SystemInspectProbeExecutor, normalize_probe_graph,
    };
    use revaer_media_core::model::StreamKind;
    use std::collections::HashMap;
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    use crate::sidecar::{SidecarDiscoverer, SidecarDiscoveryError, SidecarSubtitle};

    const PRIMARY_INSPECT_PROBE_PREFIX: &str = "ffprobe -v error -read_intervals %+#1 -show_streams -show_frames -show_format -show_chapters -of json";

    fn primary_inspect_probe_key(path: &str) -> String {
        format!("{PRIMARY_INSPECT_PROBE_PREFIX} {path}")
    }

    #[derive(Default)]
    struct StubInspectExecutor {
        outputs: HashMap<String, String>,
        calls: Mutex<Vec<String>>,
    }

    impl InspectProbeExecutor for StubInspectExecutor {
        fn run(&self, bin: &str, args: &[&str]) -> Result<String, InspectError> {
            let key = format!("{bin} {}", args.join(" "));
            self.calls
                .lock()
                .map_err(|_| {
                    InspectError::ProbeFailed("inspect executor lock poisoned".to_string())
                })?
                .push(key.clone());
            let legacy_key = key.replace(" -show_format -show_chapters", "");
            let output = self
                .outputs
                .get(&key)
                .or_else(|| self.outputs.get(&legacy_key))
                .cloned()
                .ok_or_else(|| {
                    InspectError::ProbeFailed(format!("missing probe output for {key}"))
                })?;
            let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&output) else {
                return Ok(output);
            };
            if value.get("format").is_none() {
                value["format"] = serde_json::json!({"format_name": "matroska"});
            }
            serde_json::to_string(&value)
                .map_err(|error| InspectError::OutputMalformed(error.to_string()))
        }
    }

    #[derive(Debug, Default)]
    struct EmptySidecarDiscoverer;

    impl SidecarDiscoverer for EmptySidecarDiscoverer {
        fn discover(
            &self,
            _source_path: &Path,
        ) -> Result<Vec<SidecarSubtitle>, SidecarDiscoveryError> {
            Ok(Vec::new())
        }
    }

    fn test_adapter(
        executor: Arc<dyn InspectProbeExecutor>,
        ffprobe_bin: &str,
    ) -> FfprobeInspectAdapter {
        FfprobeInspectAdapter::with_sidecar_discoverer(
            executor,
            ffprobe_bin,
            Arc::new(EmptySidecarDiscoverer),
        )
    }

    #[test]
    fn normalize_probe_graph_maps_stream_fields() {
        let graph_result = normalize_probe_graph(ProbeGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![ProbeStream {
                stream_id: 1,
                kind: "AuDiO".to_string(),
                codec: " AAC ".to_string(),
                channels: None,
                channel_layout: None,
                language: Some(" ENG ".to_string()),
                title: Some(" Main ".to_string()),
                dispositions: vec![" DEFAULT ".to_string(), " ".to_string()],
            }],
        });
        assert!(graph_result.is_ok(), "expected normalization success");
        let Ok(graph) = graph_result else {
            return;
        };

        assert_eq!(graph.streams.len(), 1);
        assert_eq!(graph.streams[0].kind, StreamKind::Audio);
        assert_eq!(graph.streams[0].codec, "aac");
        assert_eq!(graph.streams[0].language.as_deref(), Some("eng"));
        assert_eq!(graph.streams[0].title.as_deref(), Some("Main"));
        assert_eq!(graph.streams[0].dispositions, vec!["default".to_string()]);
    }

    #[test]
    fn normalize_probe_graph_drops_blank_optional_fields() {
        let graph_result = normalize_probe_graph(ProbeGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![ProbeStream {
                stream_id: 1,
                kind: "subtitle".to_string(),
                codec: " subrip ".to_string(),
                channels: None,
                channel_layout: None,
                language: Some("  ".to_string()),
                title: Some("\t".to_string()),
                dispositions: Vec::new(),
            }],
        });
        assert!(graph_result.is_ok(), "expected normalization success");
        let Ok(graph) = graph_result else {
            return;
        };

        assert_eq!(graph.streams.len(), 1);
        assert_eq!(graph.streams[0].language, None);
        assert_eq!(graph.streams[0].title, None);
    }

    #[test]
    fn normalize_probe_graph_rejects_missing_codec() {
        let graph_result = normalize_probe_graph(ProbeGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![ProbeStream {
                stream_id: 2,
                kind: "audio".to_string(),
                codec: "   ".to_string(),
                channels: None,
                channel_layout: None,
                language: Some("eng".to_string()),
                title: Some("Main".to_string()),
                dispositions: Vec::new(),
            }],
        });
        assert!(matches!(
            graph_result,
            Err(InspectError::OutputMalformed(message))
            if message == "stream codec is missing"
        ));
    }

    #[test]
    fn normalize_probe_graph_accepts_data_stream_kind() {
        let graph_result = normalize_probe_graph(ProbeGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![ProbeStream {
                stream_id: 1,
                kind: "data".to_string(),
                codec: "bin".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        });
        assert!(graph_result.is_ok(), "expected data stream to normalize");
        let Ok(graph) = graph_result else {
            return;
        };
        assert_eq!(graph.streams[0].kind, StreamKind::Data);
    }

    #[test]
    fn normalize_probe_graph_accepts_all_supported_stream_kinds() {
        let graph_result = normalize_probe_graph(ProbeGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![
                ProbeStream {
                    stream_id: 0,
                    kind: "ViDeO".to_string(),
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                ProbeStream {
                    stream_id: 1,
                    kind: "AUDIO".to_string(),
                    codec: "aac".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
                ProbeStream {
                    stream_id: 2,
                    kind: "subtitle".to_string(),
                    codec: "subrip".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("spa".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
                ProbeStream {
                    stream_id: 3,
                    kind: "Attachment".to_string(),
                    codec: "ttf".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: Some("font".to_string()),
                    dispositions: Vec::new(),
                },
                ProbeStream {
                    stream_id: 4,
                    kind: "chapter".to_string(),
                    codec: "chapter".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        });
        assert!(
            graph_result.is_ok(),
            "expected supported kinds to normalize"
        );
        let Ok(graph) = graph_result else {
            return;
        };

        assert_eq!(graph.streams.len(), 5);
        assert_eq!(graph.streams[0].kind, StreamKind::Video);
        assert_eq!(graph.streams[1].kind, StreamKind::Audio);
        assert_eq!(graph.streams[2].kind, StreamKind::Subtitle);
        assert_eq!(graph.streams[3].kind, StreamKind::Attachment);
        assert_eq!(graph.streams[4].kind, StreamKind::Chapter);
    }

    #[test]
    fn reject_empty_stream_codec() {
        let graph_result = normalize_probe_graph(ProbeGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![ProbeStream {
                stream_id: 1,
                kind: "video".to_string(),
                codec: "   ".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        });
        assert_eq!(
            graph_result.err().map(|err| err.to_string()),
            Some(InspectError::OutputMalformed("stream codec is missing".to_string()).to_string())
        );
    }

    #[test]
    fn ffprobe_adapter_builds_expected_argv_and_maps_streams() {
        let key = primary_inspect_probe_key("/input/movie.mkv");
        let mut outputs = HashMap::new();
        outputs.insert(
            key.clone(),
            r#"{
                "streams": [
                    {
                        "index": 0,
                        "codec_type": "video",
                        "codec_name": "h264",
                        "disposition": {"default": 1, "forced": 0, "hearing_impaired": 0, "visual_impaired": 0},
                        "tags": {"language": "eng", "title": " Main Video "}
                    },
                    {
                        "index": 1,
                        "codec_type": "subtitle",
                        "codec_name": "subrip",
                        "disposition": {"default": 0, "forced": 1, "hearing_impaired": 1, "visual_impaired": 1},
                        "tags": {"language": "spa"}
                    }
                ]
            }"#
            .to_string(),
        );
        let executor = Arc::new(StubInspectExecutor {
            outputs,
            calls: Mutex::new(Vec::new()),
        });
        let adapter = test_adapter(executor.clone(), "ffprobe");

        let graph_result = adapter.inspect("/input/movie.mkv");
        assert!(graph_result.is_ok(), "expected inspect success");
        let Ok(graph) = graph_result else {
            return;
        };
        assert_eq!(graph.streams.len(), 2);
        assert_eq!(graph.streams[0].kind, StreamKind::Video);
        assert_eq!(graph.streams[0].title.as_deref(), Some("Main Video"));
        assert_eq!(graph.streams[0].dispositions, vec!["default".to_string()]);
        assert_eq!(graph.streams[1].kind, StreamKind::Subtitle);
        assert_eq!(
            graph.streams[1].dispositions,
            vec![
                "forced".to_string(),
                "hearing_impaired".to_string(),
                "visual_impaired".to_string(),
            ]
        );

        executor
            .calls
            .lock()
            .map(|calls| assert_eq!(calls.as_slice(), &[key]))
            .expect("inspect executor calls lock poisoned");
    }

    #[test]
    fn ffprobe_adapter_includes_adjacent_sidecars_in_complete_inspection() {
        let directory = tempdir();
        assert!(directory.is_ok());
        let Ok(directory) = directory else {
            return;
        };
        let source = directory.path().join("episode.mkv");
        let sidecar = directory.path().join("episode.en.forced.srt");
        assert!(std::fs::write(&source, b"source").is_ok());
        assert!(std::fs::write(&sidecar, b"1\n00:00:00,000 --> 00:00:01,000\nText\n").is_ok());
        let Some(source_text) = source.to_str() else {
            return;
        };
        let key = format!("{PRIMARY_INSPECT_PROBE_PREFIX} {source_text}");
        let mut outputs = HashMap::new();
        outputs.insert(
            key,
            r#"{
                "streams": [{
                    "index": 0,
                    "codec_type": "video",
                    "codec_name": "h264",
                    "disposition": {"default": 1}
                }],
                "format": {"format_name": "matroska"}
            }"#
            .to_string(),
        );
        outputs.insert(
            format!(
                "ffprobe -v error -show_streams -of json {}",
                sidecar.to_string_lossy()
            ),
            r#"{
                "streams": [{
                    "index": 0,
                    "codec_type": "subtitle",
                    "codec_name": "subrip"
                }]
            }"#
            .to_string(),
        );
        let adapter = FfprobeInspectAdapter::new(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let result = adapter.inspect_full(source_text);
        assert!(result.is_ok());
        let Ok(inspection) = result else {
            return;
        };
        assert_eq!(inspection.sidecars.len(), 1);
        assert_eq!(inspection.sidecars[0].path, sidecar);
        assert_eq!(inspection.sidecars[0].language.as_deref(), Some("eng"));
        assert_eq!(
            inspection.sidecars[0].role,
            Some(crate::sidecar::SidecarRole::Forced)
        );
    }

    #[test]
    fn ffprobe_adapter_rejects_sidecar_codec_that_disagrees_with_extension() {
        let directory = tempdir();
        assert!(directory.is_ok());
        let Ok(directory) = directory else {
            return;
        };
        let source = directory.path().join("episode.mkv");
        let sidecar = directory.path().join("episode.eng.sub");
        assert!(std::fs::write(&source, b"source").is_ok());
        assert!(std::fs::write(&sidecar, b"image subtitle data").is_ok());
        let Some(source_text) = source.to_str() else {
            return;
        };
        let mut outputs = HashMap::new();
        outputs.insert(
            format!("{PRIMARY_INSPECT_PROBE_PREFIX} {source_text}"),
            r#"{
                "streams": [{
                    "index": 0,
                    "codec_type": "video",
                    "codec_name": "h264"
                }],
                "format": {"format_name": "matroska"}
            }"#
            .to_string(),
        );
        outputs.insert(
            format!(
                "ffprobe -v error -show_streams -of json {}",
                sidecar.to_string_lossy()
            ),
            r#"{
                "streams": [{
                    "index": 0,
                    "codec_type": "subtitle",
                    "codec_name": "dvd_subtitle"
                }]
            }"#
            .to_string(),
        );
        let adapter = FfprobeInspectAdapter::new(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let result = adapter.inspect_full(source_text);

        assert!(
            matches!(result, Err(InspectError::OutputMalformed(message)) if message.contains("sidecar codec does not match"))
        );
    }

    #[test]
    fn ffprobe_adapter_rejects_incomplete_vobsub_inventory() {
        let directory = tempdir();
        assert!(directory.is_ok());
        let Ok(directory) = directory else {
            return;
        };
        let source = directory.path().join("episode.mkv");
        assert!(std::fs::write(&source, b"source").is_ok());
        assert!(std::fs::write(directory.path().join("episode.eng.idx"), b"index").is_ok());
        let Some(source_text) = source.to_str() else {
            return;
        };
        let key = format!("{PRIMARY_INSPECT_PROBE_PREFIX} {source_text}");
        let mut outputs = HashMap::new();
        outputs.insert(
            key,
            r#"{
                "streams": [{
                    "index": 0,
                    "codec_type": "video",
                    "codec_name": "h264",
                    "disposition": {"default": 1}
                }],
                "format": {"format_name": "matroska"}
            }"#
            .to_string(),
        );
        let adapter = FfprobeInspectAdapter::new(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        assert!(matches!(
            adapter.inspect_full(source_text),
            Err(InspectError::Sidecar(
                crate::sidecar::SidecarDiscoveryError::MissingVobSubCompanion(_)
            ))
        ));
    }

    const COMPLETE_HDR_PROBE_OUTPUT: &str = r#"{
                "streams": [
                    {
                        "index": 0,
                        "codec_type": "video",
                        "codec_name": "hevc",
                        "profile": "Main 10",
                        "level": 153,
                        "duration": "60.125",
                        "bit_rate": "4000000",
                        "width": 3840,
                        "height": 2160,
                        "pix_fmt": "yuv420p10le",
                        "sample_aspect_ratio": "1:1",
                        "display_aspect_ratio": "16:9",
                        "avg_frame_rate": "24000/1001",
                        "color_range": "tv",
                        "color_space": "bt2020nc",
                        "color_transfer": "smpte2084",
                        "color_primaries": "bt2020",
                        "chroma_location": "left",
                        "field_order": "progressive",
                        "disposition": {"default": 1},
                        "tags": {"language": "eng", "BPS": "4000000"},
                        "side_data_list": [
                            {
                                "side_data_type": "Mastering display metadata",
                                "red_x": "34000/50000"
                            }
                        ]
                    },
                    {
                        "index": 1,
                        "codec_type": "data",
                        "codec_name": "bin_data"
                    }
                ],
                "frames": [
                    {
                        "stream_index": 0,
                        "side_data_list": [
                            {
                                "side_data_type": "Mastering display metadata",
                                "red_x": "34000/50000",
                                "red_y": "16000/50000",
                                "green_x": "13250/50000",
                                "green_y": "34500/50000",
                                "blue_x": "7500/50000",
                                "blue_y": "3000/50000",
                                "white_point_x": "15635/50000",
                                "white_point_y": "16450/50000",
                                "min_luminance": "50/10000",
                                "max_luminance": "10000000/10000"
                            },
                            {
                                "side_data_type": "Content light level metadata",
                                "max_content": 1000,
                                "max_average": 400
                            }
                        ]
                    }
                ],
                "chapters": [
                    {
                        "id": 7,
                        "start_time": "0.000",
                        "end_time": "60.125",
                        "tags": {"TITLE": "Opening"}
                    }
                ],
                "format": {
                    "format_name": "matroska,webm",
                    "duration": "60.125",
                    "start_time": "-0.007",
                    "size": "30125000",
                    "bit_rate": "4008316",
                    "tags": {"TITLE": "HDR Fixture", "TMDB": "123"}
                }
            }"#;

    #[test]
    fn ffprobe_adapter_normalizes_complete_container_timeline_and_hdr_state() {
        let key =
            "ffprobe -v error -read_intervals %+#1 -show_streams -show_frames -show_format -show_chapters -of json /input/hdr.mkv"
                .to_string();
        let mut outputs = HashMap::new();
        outputs.insert(key, COMPLETE_HDR_PROBE_OUTPUT.to_string());
        let adapter = test_adapter(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let result = adapter.inspect_full("/input/hdr.mkv");
        assert!(result.is_ok(), "expected complete inspection success");
        let Ok(inspection) = result else {
            return;
        };
        assert_eq!(inspection.container.formats, vec!["matroska", "webm"]);
        assert_eq!(inspection.graph.container_formats, vec!["matroska", "webm"]);
        assert_eq!(inspection.container.duration_millis, Some(60_125));
        assert_eq!(inspection.container.start_time_millis, Some(-7));
        assert_eq!(inspection.graph.streams[1].kind, StreamKind::Data);
        assert_eq!(inspection.chapters[0].chapter_id, 7);
        assert_eq!(inspection.chapters[0].end_millis, 60_125);
        assert_eq!(
            inspection.streams[0].color_transfer.as_deref(),
            Some("smpte2084")
        );
        assert!(
            inspection.streams[0]
                .metadata
                .iter()
                .any(|entry| entry.key == "level" && entry.value == "153")
        );
        assert_eq!(
            inspection.streams[0].side_data_types,
            vec![
                "content light level metadata".to_string(),
                "mastering display metadata".to_string()
            ]
        );
        assert!(inspection.streams[0].side_data.iter().any(|side_data| {
            side_data.side_data_type == "content light level metadata"
                && side_data
                    .metadata
                    .iter()
                    .any(|entry| entry.key == "max_content" && entry.value == "1000")
                && side_data
                    .metadata
                    .iter()
                    .any(|entry| entry.key == "max_average" && entry.value == "400")
        }));
        assert!(inspection.streams[0].side_data.iter().any(|side_data| {
            side_data.side_data_type == "mastering display metadata"
                && side_data
                    .metadata
                    .iter()
                    .any(|entry| entry.key == "red_x" && entry.value == "34000/50000")
                && side_data
                    .metadata
                    .iter()
                    .any(|entry| entry.key == "max_luminance" && entry.value == "10000000/10000")
        }));
        assert!(
            inspection
                .container
                .metadata
                .iter()
                .any(|entry| entry.key == "tmdb" && entry.value == "123")
        );
    }

    #[test]
    fn ffprobe_adapter_preserves_audio_channel_shape() {
        let key = primary_inspect_probe_key("/input/movie.mkv");
        let mut outputs = HashMap::new();
        let probe_output = r#"{
                "streams": [
                    {
                        "index": 1,
                        "codec_type": "audio",
                        "codec_name": "aac",
                        "channels": 6,
                        "sample_rate": "48000",
                        "channel_layout": "5.1(side)",
                        "disposition": {"default": 1},
                        "tags": {"language": "eng", "title": "Main"}
                    }
                ]
            }"#;
        outputs.insert(key, probe_output.to_string());
        let adapter = test_adapter(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let graph_result = adapter.inspect("/input/movie.mkv");
        assert!(graph_result.is_ok(), "expected inspect success");
        let Ok(graph) = graph_result else {
            return;
        };
        let graph_value = serde_json::to_value(&graph);
        assert!(
            graph_value.is_ok(),
            "expected inspected graph to serialize for metadata assertions"
        );
        let Ok(graph_value) = graph_value else {
            return;
        };

        assert_eq!(
            graph_value
                .pointer("/streams/0/channels")
                .and_then(serde_json::Value::as_u64),
            Some(6)
        );
        assert_eq!(
            graph_value
                .pointer("/streams/0/channel_layout")
                .and_then(serde_json::Value::as_str),
            Some("5.1(side)")
        );

        let inspection_result = adapter.inspect_full("/input/movie.mkv");
        assert!(inspection_result.is_ok(), "expected full inspect success");
        let Ok(inspection) = inspection_result else {
            return;
        };
        assert_eq!(inspection.streams[0].sample_rate, Some(48_000));
    }

    #[test]
    fn ffprobe_adapter_rejects_malformed_json() {
        let key = primary_inspect_probe_key("/input/movie.mkv");
        let mut outputs = HashMap::new();
        outputs.insert(key, "{not-json".to_string());
        let adapter = test_adapter(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let result = adapter.inspect("/input/movie.mkv");
        assert!(matches!(result, Err(InspectError::OutputMalformed(_))));
    }

    #[test]
    fn ffprobe_adapter_rejects_stream_with_missing_codec() {
        let key = primary_inspect_probe_key("/input/movie.mkv");
        let mut outputs = HashMap::new();
        outputs.insert(
            key,
            r#"{
                "streams": [
                    {
                        "index": 0,
                        "codec_type": "video",
                        "codec_name": "   ",
                        "disposition": {"default": 1},
                        "tags": {"language": "eng"}
                    }
                ]
            }"#
            .to_string(),
        );
        let adapter = test_adapter(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let result = adapter.inspect("/input/movie.mkv");
        assert_eq!(
            result.err().map(|err| err.to_string()),
            Some(InspectError::OutputMalformed("stream codec is missing".to_string()).to_string())
        );
    }

    #[test]
    fn ffprobe_adapter_normalizes_blank_tag_fields_to_none() {
        let key = primary_inspect_probe_key("/input/movie.mkv");
        let mut outputs = HashMap::new();
        outputs.insert(
            key,
            r#"{
                "streams": [
                    {
                        "index": 1,
                        "codec_type": "subtitle",
                        "codec_name": "subrip",
                        "disposition": {"default": 0, "forced": 0},
                        "tags": {"language": "  ", "title": "\t"}
                    }
                ]
            }"#
            .to_string(),
        );
        let adapter = test_adapter(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let graph_result = adapter.inspect("/input/movie.mkv");
        assert!(graph_result.is_ok(), "expected inspect success");
        let Ok(graph) = graph_result else {
            return;
        };

        assert_eq!(graph.streams.len(), 1);
        assert_eq!(graph.streams[0].language, None);
        assert_eq!(graph.streams[0].title, None);
    }

    #[test]
    fn ffprobe_adapter_ignores_zero_dispositions() {
        let key = primary_inspect_probe_key("/input/movie.mkv");
        let mut outputs = HashMap::new();
        outputs.insert(
            key,
            r#"{
                "streams": [
                    {
                        "index": 0,
                        "codec_type": "audio",
                        "codec_name": "aac",
                        "disposition": {
                            "default": 0,
                            "forced": 0,
                            "hearing_impaired": 0,
                            "visual_impaired": 0
                        }
                    }
                ]
            }"#
            .to_string(),
        );
        let adapter = test_adapter(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let graph_result = adapter.inspect("/input/movie.mkv");
        assert!(graph_result.is_ok(), "expected inspect success");
        let Ok(graph) = graph_result else {
            return;
        };
        assert_eq!(graph.streams.len(), 1);
        assert!(graph.streams[0].dispositions.is_empty());
    }

    #[test]
    fn ffprobe_adapter_handles_missing_tags_object() {
        let key = primary_inspect_probe_key("/input/movie.mkv");
        let mut outputs = HashMap::new();
        outputs.insert(
            key,
            r#"{
                "streams": [
                    {
                        "index": 7,
                        "codec_type": "audio",
                        "codec_name": "aac",
                        "disposition": {"default": 1}
                    }
                ]
            }"#
            .to_string(),
        );
        let adapter = test_adapter(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let graph_result = adapter.inspect("/input/movie.mkv");
        assert!(graph_result.is_ok(), "expected inspect success");
        let Ok(graph) = graph_result else {
            return;
        };
        assert_eq!(graph.streams.len(), 1);
        assert_eq!(graph.streams[0].language, None);
        assert_eq!(graph.streams[0].title, None);
        assert_eq!(graph.streams[0].dispositions, vec!["default".to_string()]);
    }

    #[test]
    fn ffprobe_adapter_emits_dispositions_in_stable_order() {
        let key = primary_inspect_probe_key("/input/movie.mkv");
        let mut outputs = HashMap::new();
        outputs.insert(
            key,
            r#"{
                "streams": [
                    {
                        "index": 0,
                        "codec_type": "audio",
                        "codec_name": "aac",
                        "disposition": {
                            "default": 1,
                            "forced": 1,
                            "hearing_impaired": 1,
                            "visual_impaired": 1
                        }
                    }
                ]
            }"#
            .to_string(),
        );
        let adapter = test_adapter(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let graph_result = adapter.inspect("/input/movie.mkv");
        assert!(graph_result.is_ok(), "expected inspect success");
        let Ok(graph) = graph_result else {
            return;
        };
        assert_eq!(graph.streams.len(), 1);
        assert_eq!(
            graph.streams[0].dispositions,
            vec![
                "default".to_string(),
                "forced".to_string(),
                "hearing_impaired".to_string(),
                "visual_impaired".to_string(),
            ]
        );
    }

    #[test]
    fn system_probe_executor_maps_non_zero_exit_to_probe_failed() {
        let executor = SystemInspectProbeExecutor;
        let result = executor.run("sh", &["-c", "printf 'bad stderr' 1>&2; exit 9"]);
        assert!(matches!(
            result,
            Err(InspectError::ProbeFailed(message))
            if message.contains("exited with status") && message.contains("bad stderr")
        ));
    }

    #[test]
    fn system_probe_executor_maps_invalid_utf8_stdout_to_output_malformed() {
        let executor = SystemInspectProbeExecutor;
        let result = executor.run(
            "python3",
            &["-c", "import sys; sys.stdout.buffer.write(b'\\xff')"],
        );
        assert!(matches!(result, Err(InspectError::OutputMalformed(_))));
    }
}
