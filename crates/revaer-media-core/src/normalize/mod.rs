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
    channel_layout
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
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
    use super::{normalize_container_format, normalize_graph, normalize_subtitle_codec};
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
