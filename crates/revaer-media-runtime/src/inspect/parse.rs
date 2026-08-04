use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use revaer_media_core::model::{
    ContainerChapterEntry, ContainerMetadataEntry, MediaGraph, MediaStream, StreamKind,
};
use revaer_media_core::normalize::{normalize_graph, normalize_subtitle_codec};

use super::ffprobe::{
    FfprobeChapter, FfprobeDisposition, FfprobeFormat, FfprobeOutput, FfprobeSideData,
    FfprobeStream, FfprobeTags,
};
use super::model::{
    ChapterInspection, ContainerInspection, InspectError, MediaInspection, MetadataEntry,
    ProbeGraph, ProbeStream, StreamInspection,
};
use crate::sidecar::SidecarFormat;

pub(super) fn parse_source(
    source_path: &Path,
    output: &[u8],
) -> Result<MediaInspection, InspectError> {
    let source = source_path.to_str().ok_or_else(|| {
        InspectError::OutputMalformed("source path is not valid UTF-8".to_string())
    })?;
    let parsed: FfprobeOutput = serde_json::from_slice(output)
        .map_err(|error| InspectError::OutputMalformed(error.to_string()))?;
    let container = normalize_container(parsed.format)?;
    if parsed.streams.is_empty() {
        return Err(InspectError::OutputMalformed(
            "source contains no media streams".to_string(),
        ));
    }
    let mut technical_streams = parsed
        .streams
        .iter()
        .map(normalize_stream_inspection)
        .collect::<Result<Vec<_>, _>>()?;
    let graph_input = ProbeGraph {
        source_path: source.to_string(),
        streams: parsed.streams.into_iter().map(probe_stream_from).collect(),
    };
    let mut graph = normalize_probe_graph(graph_input)?;
    graph.container_formats.clone_from(&container.formats);
    technical_streams.sort_by_key(|stream| stream.stream_id);
    let mut chapters = parsed
        .chapters
        .into_iter()
        .map(normalize_chapter)
        .collect::<Result<Vec<_>, _>>()?;
    chapters.sort_by_key(|chapter| (chapter.start_millis, chapter.chapter_id));
    reject_duplicate_chapters(&chapters)?;
    graph.container_chapters = chapters
        .iter()
        .map(|chapter| ContainerChapterEntry {
            start_millis: chapter.start_millis,
            end_millis: chapter.end_millis,
            metadata: chapter
                .metadata
                .iter()
                .map(|entry| ContainerMetadataEntry {
                    key: entry.key.clone(),
                    value: entry.value.clone(),
                })
                .collect(),
        })
        .collect();
    graph = normalize_graph(&graph);
    Ok(MediaInspection {
        graph,
        container,
        streams: technical_streams,
        chapters,
        sidecars: Vec::new(),
    })
}

pub(super) fn validate_sidecar_output(
    path: &Path,
    format: SidecarFormat,
    output: &[u8],
) -> Result<(), InspectError> {
    let parsed: FfprobeOutput = serde_json::from_slice(output)
        .map_err(|error| InspectError::OutputMalformed(error.to_string()))?;
    let [stream] = parsed.streams.as_slice() else {
        return Err(InspectError::OutputMalformed(format!(
            "sidecar must contain exactly one stream: {}",
            path.display()
        )));
    };
    if !stream.codec_type.eq_ignore_ascii_case("subtitle") {
        return Err(InspectError::OutputMalformed(format!(
            "sidecar stream is not a subtitle: {}",
            path.display()
        )));
    }
    let actual = normalize_subtitle_codec(&stream.codec_name);
    let expected = expected_sidecar_codec(format);
    if actual != expected {
        return Err(InspectError::OutputMalformed(format!(
            "sidecar codec mismatch for {}: expected={expected} actual={actual}",
            path.display()
        )));
    }
    Ok(())
}

fn probe_stream_from(stream: FfprobeStream) -> ProbeStream {
    let (language, title) = stream
        .tags
        .map_or((None, None), |tags| (tags.language, tags.title));
    ProbeStream {
        stream_id: stream.index,
        kind: stream.codec_type,
        codec: stream.codec_name,
        channels: stream.channels,
        channel_layout: stream.channel_layout,
        language,
        title,
        dispositions: dispositions_from_raw(stream.disposition),
    }
}

/// Convert probe-like output into a normalized domain graph.
///
/// # Errors
///
/// Returns an error for unsupported stream kinds, missing codecs, or duplicate stream IDs.
pub fn normalize_probe_graph(input: ProbeGraph) -> Result<MediaGraph, InspectError> {
    let mut streams = input
        .streams
        .into_iter()
        .map(normalize_probe_stream)
        .collect::<Result<Vec<_>, _>>()?;
    streams.sort_by_key(|stream| stream.stream_id);
    reject_duplicate_streams(&streams)?;
    Ok(normalize_graph(&MediaGraph {
        source_path: input.source_path,
        container_chapters: Vec::new(),
        container_formats: Vec::new(),
        streams,
    }))
}

fn normalize_probe_stream(stream: ProbeStream) -> Result<MediaStream, InspectError> {
    let kind = parse_stream_kind(&stream.kind)?;
    let codec = stream.codec.trim().to_ascii_lowercase();
    if codec.is_empty() {
        return Err(InspectError::OutputMalformed(
            "stream codec is missing".to_string(),
        ));
    }
    Ok(MediaStream {
        stream_id: stream.stream_id,
        kind,
        codec,
        channels: (kind == StreamKind::Audio)
            .then_some(stream.channels)
            .flatten()
            .filter(|value| *value > 0),
        channel_layout: (kind == StreamKind::Audio)
            .then_some(stream.channel_layout)
            .flatten()
            .as_deref()
            .and_then(|value| normalize_optional_text(Some(value)))
            .map(|value| value.to_ascii_lowercase()),
        language: stream
            .language
            .as_deref()
            .and_then(|value| normalize_optional_text(Some(value)))
            .map(|value| value.to_ascii_lowercase()),
        title: stream
            .title
            .as_deref()
            .and_then(|value| normalize_optional_text(Some(value))),
        dispositions: stream.dispositions,
    })
}

fn reject_duplicate_streams(streams: &[MediaStream]) -> Result<(), InspectError> {
    let mut identifiers = BTreeSet::new();
    for stream in streams {
        if !identifiers.insert(stream.stream_id) {
            return Err(InspectError::OutputMalformed(format!(
                "duplicate stream identifier: {}",
                stream.stream_id
            )));
        }
    }
    Ok(())
}

fn reject_duplicate_chapters(chapters: &[ChapterInspection]) -> Result<(), InspectError> {
    let mut identifiers = BTreeSet::new();
    for chapter in chapters {
        if !identifiers.insert(chapter.chapter_id) {
            return Err(InspectError::OutputMalformed(format!(
                "duplicate chapter identifier: {}",
                chapter.chapter_id
            )));
        }
    }
    Ok(())
}

fn parse_stream_kind(value: &str) -> Result<StreamKind, InspectError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "video" => Ok(StreamKind::Video),
        "audio" => Ok(StreamKind::Audio),
        "subtitle" => Ok(StreamKind::Subtitle),
        "attachment" => Ok(StreamKind::Attachment),
        "chapter" => Ok(StreamKind::Chapter),
        "data" => Ok(StreamKind::Data),
        other => Err(InspectError::InvalidStreamKind(other.to_string())),
    }
}

fn normalize_stream_inspection(stream: &FfprobeStream) -> Result<StreamInspection, InspectError> {
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
        metadata: stream
            .tags
            .as_ref()
            .map_or_else(Vec::new, metadata_from_stream_tags),
        side_data_types: normalize_side_data(&stream.side_data_list),
    })
}

fn normalize_container(format: Option<FfprobeFormat>) -> Result<ContainerInspection, InspectError> {
    let format = format
        .ok_or_else(|| InspectError::OutputMalformed("container format is missing".to_string()))?;
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
        .filter_map(|(key, value)| {
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            (!key.is_empty() && !value.is_empty()).then_some(MetadataEntry { key, value })
        })
        .collect()
}

fn metadata_from_stream_tags(tags: &FfprobeTags) -> Vec<MetadataEntry> {
    let mut values = tags.extra.clone();
    if let Some(language) = &tags.language {
        values.insert("language".to_string(), language.clone());
    }
    if let Some(title) = &tags.title {
        values.insert("title".to_string(), title.clone());
    }
    normalize_metadata(values)
}

fn normalize_side_data(values: &[FfprobeSideData]) -> Vec<String> {
    let mut normalized = values
        .iter()
        .filter_map(|value| normalize_lowercase_text(Some(&value.side_data_type)))
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
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
    match parse_optional_millis(value, "duration")? {
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
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(InspectError::OutputMalformed(format!(
            "{label} is not a decimal number"
        )));
    }
    let whole_seconds = whole
        .parse::<i64>()
        .map_err(|_| InspectError::OutputMalformed(format!("{label} is not a decimal number")))?;
    let mut digits = fraction.bytes().take(3).map(|byte| i64::from(byte - b'0'));
    let first = digits.next().unwrap_or(0);
    let second = digits.next().unwrap_or(0);
    let third = digits.next().unwrap_or(0);
    let fraction_millis = first * 100 + second * 10 + third;
    let millis = whole_seconds
        .checked_mul(1000)
        .and_then(|whole| whole.checked_add(fraction_millis))
        .ok_or_else(|| InspectError::OutputMalformed(format!("{label} is out of range")))?;
    if negative {
        millis
            .checked_neg()
            .ok_or_else(|| InspectError::OutputMalformed(format!("{label} is out of range")))
    } else {
        Ok(millis)
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
    [
        (raw.default, "default"),
        (raw.forced, "forced"),
        (raw.hearing_impaired, "hearing_impaired"),
        (raw.visual_impaired, "visual_impaired"),
    ]
    .into_iter()
    .filter(|(enabled, _)| *enabled == Some(1))
    .map(|(_, name)| name.to_string())
    .collect()
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
