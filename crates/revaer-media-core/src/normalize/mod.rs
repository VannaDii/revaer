//! Deterministic normalization helpers.

use crate::model::{MediaGraph, MediaStream};

/// Normalize a source graph and return a canonical clone.
#[must_use]
pub fn normalize_graph(graph: &MediaGraph) -> MediaGraph {
    MediaGraph {
        source_path: graph.source_path.trim().to_string(),
        streams: graph.streams.iter().map(normalize_stream).collect(),
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
        _ => raw,
    }
}

fn normalize_language(value: &str) -> String {
    let raw = value.trim().to_ascii_lowercase();
    match raw.as_str() {
        "eng" | "en" => "eng".to_string(),
        "fra" | "fr" => "fra".to_string(),
        "deu" | "de" => "deu".to_string(),
        _ => raw,
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
    use super::normalize_graph;
    use crate::model::{MediaGraph, MediaStream, StreamKind};

    #[test]
    fn normalize_aliases_and_whitespace() {
        let graph = MediaGraph {
            source_path: " /data/source.mkv ".to_string(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: " H.264 ".to_string(),
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
}
