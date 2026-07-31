//! Deterministic normalization helpers.

use crate::model::{MediaGraph, MediaStream};

/// Normalize a source graph and return a canonical clone.
#[must_use]
pub fn normalize_graph(graph: &MediaGraph) -> MediaGraph {
    let mut container_formats = graph
        .container_formats
        .iter()
        .map(|format| normalize_container_format(format))
        .filter(|format| !format.is_empty())
        .collect::<Vec<_>>();
    container_formats.sort();
    container_formats.dedup();
    MediaGraph {
        source_path: graph.source_path.trim().to_string(),
        container_formats,
        streams: graph.streams.iter().map(normalize_stream).collect(),
    }
}

/// Normalize a container alias to the canonical `FFmpeg` muxer name.
#[must_use]
pub fn normalize_container_format(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "mkv" => "matroska".to_string(),
        "m4a" | "m4v" | "mov" => "mp4".to_string(),
        "ts" | "m2ts" => "mpegts".to_string(),
        normalized => normalized.to_string(),
    }
}

/// Normalize a desired-target container metadata policy.
#[must_use]
pub fn normalize_container_metadata_policy(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "preserve" => Some("preserve"),
        "strip" => Some("strip"),
        "replace" => Some("replace"),
        _ => None,
    }
}

/// Normalize a desired-target container chapter policy.
#[must_use]
pub fn normalize_container_chapter_policy(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "preserve" => Some("preserve"),
        "strip" => Some("strip"),
        "replace" => Some("replace"),
        _ => None,
    }
}

/// Normalize subtitle codec aliases used by probes, targets, and sidecar formats.
#[must_use]
pub fn normalize_subtitle_codec(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "srt" | "subrip" => "subrip".to_string(),
        "vtt" | "webvtt" => "webvtt".to_string(),
        "pgs" | "hdmv_pgs_subtitle" => "hdmv_pgs_subtitle".to_string(),
        "vobsub" | "dvd_subtitle" => "dvd_subtitle".to_string(),
        normalized => normalized.to_string(),
    }
}

/// Return the canonical channel-layout label for supported audio layouts.
#[must_use]
pub fn normalize_audio_channel_layout(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mono" | "1c" => Some("mono"),
        "stereo" | "2c" => Some("stereo"),
        "2.1" => Some("2.1"),
        "3.0" => Some("3.0"),
        "3.0(back)" => Some("3.0(back)"),
        "4.0" => Some("4.0"),
        "quad" => Some("quad"),
        "quad(side)" => Some("quad(side)"),
        "3.1" => Some("3.1"),
        "5.0" => Some("5.0"),
        "5.0(side)" => Some("5.0(side)"),
        "4.1" => Some("4.1"),
        "5.1" => Some("5.1"),
        "5.1(side)" => Some("5.1(side)"),
        "6.1" => Some("6.1"),
        "6.1(back)" => Some("6.1(back)"),
        "7.1" => Some("7.1"),
        "7.1(wide)" => Some("7.1(wide)"),
        "7.1(wide-side)" => Some("7.1(wide-side)"),
        _ => None,
    }
}

/// Return the required channel count for a canonical supported audio layout.
#[must_use]
pub fn audio_channel_count_for_layout(value: &str) -> Option<u32> {
    match normalize_audio_channel_layout(value) {
        Some("mono") => Some(1),
        Some("stereo") => Some(2),
        Some("2.1" | "3.0" | "3.0(back)") => Some(3),
        Some("4.0" | "quad" | "quad(side)" | "3.1") => Some(4),
        Some("5.0" | "5.0(side)" | "4.1") => Some(5),
        Some("5.1" | "5.1(side)") => Some(6),
        Some("6.1" | "6.1(back)") => Some(7),
        Some("7.1" | "7.1(wide)" | "7.1(wide-side)") => Some(8),
        Some(_) | None => None,
    }
}

fn normalize_stream(stream: &MediaStream) -> MediaStream {
    let mut dispositions: Vec<String> = stream
        .dispositions
        .iter()
        .map(|value| normalize_disposition(value))
        .filter(|value| !value.is_empty())
        .collect();
    dispositions.sort();
    dispositions.dedup();

    MediaStream {
        stream_id: stream.stream_id,
        kind: stream.kind,
        codec: normalize_codec(&stream.codec),
        channels: normalize_channels(stream.kind, stream.channels),
        channel_layout: normalize_channel_layout(stream.kind, stream.channel_layout.as_deref()),
        language: stream
            .language
            .as_ref()
            .map(|value| normalize_language(value)),
        title: stream.title.as_ref().map(|value| normalize_title(value)),
        dispositions,
    }
}

fn normalize_codec(value: &str) -> String {
    let raw = value.trim().to_ascii_lowercase();
    match raw.as_str() {
        "x264" | "h.264" => "h264".to_string(),
        "x265" | "h.265" | "hevc" => "hevc".to_string(),
        "dca" => "dts".to_string(),
        "subrip" => "srt".to_string(),
        _ => raw,
    }
}

fn normalize_channels(kind: crate::model::StreamKind, channels: Option<u32>) -> Option<u32> {
    if kind == crate::model::StreamKind::Audio {
        channels.filter(|value| *value > 0)
    } else {
        None
    }
}

fn normalize_channel_layout(
    kind: crate::model::StreamKind,
    channel_layout: Option<&str>,
) -> Option<String> {
    if kind != crate::model::StreamKind::Audio {
        return None;
    }
    channel_layout.and_then(|value| {
        normalize_audio_channel_layout(value)
            .map(str::to_string)
            .or_else(|| {
                let normalized = value.trim().to_ascii_lowercase();
                (!normalized.is_empty()).then_some(normalized)
            })
    })
}

fn normalize_language(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    let raw = normalized
        .split(['-', '_'])
        .next()
        .map_or(normalized.as_str(), |language| language);
    match raw {
        "eng" | "en" => "eng".to_string(),
        "fra" | "fr" => "fra".to_string(),
        "deu" | "de" => "deu".to_string(),
        _ => raw.to_string(),
    }
}

fn normalize_title(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_disposition(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::{
        audio_channel_count_for_layout, normalize_audio_channel_layout, normalize_container_format,
        normalize_graph, normalize_subtitle_codec,
    };
    use crate::model::{MediaGraph, MediaStream, StreamKind};

    #[test]
    fn normalize_aliases_and_whitespace() {
        let graph = MediaGraph {
            source_path: " /data/source.mkv ".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: " H.264 ".to_string(),
                channels: None,
                channel_layout: None,
                language: Some("EN".to_string()),
                title: Some(" Main   Video ".to_string()),
                dispositions: vec!["default".to_string(), "Default".to_string()],
            }],
        };

        let normalized = normalize_graph(&graph);
        assert_eq!(normalized.source_path, "/data/source.mkv");
        assert_eq!(normalized.streams[0].codec, "h264");
        assert_eq!(normalized.streams[0].language.as_deref(), Some("eng"));
        assert_eq!(normalized.streams[0].title.as_deref(), Some("Main Video"));
        assert_eq!(normalized.streams[0].dispositions, vec!["default"]);
    }

    #[test]
    fn normalize_container_aliases_to_ffmpeg_muxers() {
        assert_eq!(normalize_container_format(" MKV "), "matroska");
        assert_eq!(normalize_container_format("webm"), "webm");
        assert_eq!(normalize_container_format("m4a"), "mp4");
        assert_eq!(normalize_container_format("m2ts"), "mpegts");
    }

    #[test]
    fn normalize_subtitle_codec_aliases() {
        assert_eq!(normalize_subtitle_codec("SRT"), "subrip");
        assert_eq!(normalize_subtitle_codec("vtt"), "webvtt");
        assert_eq!(normalize_subtitle_codec("pgs"), "hdmv_pgs_subtitle");
        assert_eq!(normalize_subtitle_codec("vobsub"), "dvd_subtitle");
    }

    #[test]
    fn normalize_audio_channel_layouts_to_supported_contract() {
        assert_eq!(normalize_audio_channel_layout(" 2C "), Some("stereo"));
        assert_eq!(
            normalize_audio_channel_layout("5.1(SIDE)"),
            Some("5.1(side)")
        );
        assert_eq!(
            normalize_audio_channel_layout("7.1(wide-side)"),
            Some("7.1(wide-side)")
        );
        assert_eq!(normalize_audio_channel_layout("ambisonic"), None);
        assert_eq!(audio_channel_count_for_layout("mono"), Some(1));
        assert_eq!(audio_channel_count_for_layout("5.1(side)"), Some(6));
        assert_eq!(audio_channel_count_for_layout("7.1"), Some(8));
    }

    #[test]
    fn normalize_codec_aliases_from_tool_output() {
        let graph = MediaGraph {
            source_path: "/data/source.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Audio,
                    codec: "dca".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Subtitle,
                    codec: "subrip".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };

        let normalized = normalize_graph(&graph);
        assert_eq!(normalized.streams[0].codec, "dts");
        assert_eq!(normalized.streams[1].codec, "srt");
    }

    #[test]
    fn normalize_language_region_aliases() {
        let graph = MediaGraph {
            source_path: "/data/source.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: None,
                channel_layout: None,
                language: Some("eng-US".to_string()),
                title: None,
                dispositions: Vec::new(),
            }],
        };

        let normalized = normalize_graph(&graph);
        assert_eq!(normalized.streams[0].language.as_deref(), Some("eng"));
    }
}
