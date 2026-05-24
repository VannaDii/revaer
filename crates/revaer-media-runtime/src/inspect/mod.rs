//! Inspection adapter interfaces.

use revaer_media_core::model::{MediaGraph, MediaStream, StreamKind};
use thiserror::Error;

/// Error emitted by inspect adapters.
#[derive(Debug, Error)]
pub enum InspectError {
    /// Adapter failed with details.
    #[error("inspect adapter failure: {0}")]
    Adapter(String),
    /// Invalid stream kind from adapter input.
    #[error("invalid stream kind: {0}")]
    InvalidStreamKind(String),
}

/// Inspect media and return a parsed graph.
pub trait InspectAdapter {
    /// Inspect the path and return a deterministic media graph.
    ///
    /// # Errors
    ///
    /// Returns [`InspectError`] when the adapter cannot inspect or parse the source media.
    fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError>;
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
        streams.push(MediaStream {
            stream_id: stream.stream_id,
            kind,
            codec: stream.codec.trim().to_ascii_lowercase(),
            language: stream
                .language
                .map(|value| value.trim().to_ascii_lowercase()),
            title: stream.title.map(|value| value.trim().to_string()),
            dispositions: stream
                .dispositions
                .into_iter()
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| !value.is_empty())
                .collect(),
        });
    }

    Ok(MediaGraph {
        source_path: input.source_path,
        streams,
    })
}

fn parse_stream_kind(value: &str) -> Result<StreamKind, InspectError> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "video" => Ok(StreamKind::Video),
        "audio" => Ok(StreamKind::Audio),
        "subtitle" => Ok(StreamKind::Subtitle),
        "attachment" => Ok(StreamKind::Attachment),
        "chapter" => Ok(StreamKind::Chapter),
        _ => Err(InspectError::InvalidStreamKind(normalized)),
    }
}

#[cfg(test)]
mod tests {
    use super::{InspectError, ProbeGraph, ProbeStream, normalize_probe_graph};
    use revaer_media_core::model::StreamKind;

    #[test]
    fn normalize_probe_graph_maps_stream_fields() {
        let graph_result = normalize_probe_graph(ProbeGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![ProbeStream {
                stream_id: 1,
                kind: "AuDiO".to_string(),
                codec: " AAC ".to_string(),
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
    fn reject_unknown_stream_kind() {
        let graph_result = normalize_probe_graph(ProbeGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![ProbeStream {
                stream_id: 1,
                kind: "data".to_string(),
                codec: "bin".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        });
        assert_eq!(
            graph_result.err().map(|err| err.to_string()),
            Some(InspectError::InvalidStreamKind("data".to_string()).to_string())
        );
    }
}
