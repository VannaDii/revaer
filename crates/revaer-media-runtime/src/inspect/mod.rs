//! Inspection adapter interfaces.

use revaer_media_core::model::{MediaGraph, MediaStream, StreamKind};
use serde::Deserialize;
use std::process::Command;
use std::sync::Arc;
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
    /// Probe command failed with details.
    #[error("inspect probe command failed: {0}")]
    ProbeFailed(String),
    /// Probe output failed to parse.
    #[error("inspect probe output malformed: {0}")]
    OutputMalformed(String),
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

/// Command probe abstraction for inspection adapters.
pub trait InspectProbeExecutor: Send + Sync {
    /// Execute one command and return UTF-8 stdout.
    ///
    /// # Errors
    ///
    /// Returns [`InspectError::ProbeFailed`] when command execution fails or exits non-zero.
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
        String::from_utf8(output.stdout).map_err(|err| InspectError::ProbeFailed(err.to_string()))
    }
}

/// ffprobe-backed inspection adapter.
pub struct FfprobeInspectAdapter {
    executor: Arc<dyn InspectProbeExecutor>,
    ffprobe_bin: String,
}

impl FfprobeInspectAdapter {
    /// Construct adapter with injected probe executor and binary name.
    pub fn new(executor: Arc<dyn InspectProbeExecutor>, ffprobe_bin: impl Into<String>) -> Self {
        Self {
            executor,
            ffprobe_bin: ffprobe_bin.into(),
        }
    }
}

impl InspectAdapter for FfprobeInspectAdapter {
    fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError> {
        const ARGS_PREFIX: [&str; 7] = ["-v", "error", "-show_streams", "-of", "json", "-i", ""];
        let mut args = ARGS_PREFIX;
        args[6] = source_path;
        let output = self.executor.run(&self.ffprobe_bin, &args)?;
        let parsed: FfprobeOutput = serde_json::from_str(&output)
            .map_err(|err| InspectError::OutputMalformed(err.to_string()))?;
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
                        language,
                        title,
                        dispositions: dispositions_from_raw(item.disposition),
                    }
                })
                .collect(),
        };
        normalize_probe_graph(probe)
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
    dispositions
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    index: u32,
    codec_type: String,
    codec_name: String,
    disposition: Option<FfprobeDisposition>,
    tags: Option<FfprobeTags>,
}

#[derive(Debug, Deserialize)]
struct FfprobeDisposition {
    default: Option<u8>,
    forced: Option<u8>,
    hearing_impaired: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct FfprobeTags {
    language: Option<String>,
    title: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        FfprobeInspectAdapter, InspectAdapter, InspectError, InspectProbeExecutor, ProbeGraph,
        ProbeStream, normalize_probe_graph,
    };
    use revaer_media_core::model::StreamKind;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct StubInspectExecutor {
        outputs: HashMap<String, String>,
        calls: Mutex<Vec<String>>,
    }

    impl InspectProbeExecutor for StubInspectExecutor {
        fn run(&self, bin: &str, args: &[&str]) -> Result<String, InspectError> {
            let key = format!("{bin} {}", args.join(" "));
            let mut calls = self.calls.lock().map_err(|_| {
                InspectError::ProbeFailed("inspect executor lock poisoned".to_string())
            })?;
            calls.push(key.clone());
            self.outputs
                .get(&key)
                .cloned()
                .ok_or_else(|| InspectError::ProbeFailed(format!("missing probe output for {key}")))
        }
    }

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

    #[test]
    fn ffprobe_adapter_builds_expected_argv_and_maps_streams() {
        let key = "ffprobe -v error -show_streams -of json -i /input/movie.mkv".to_string();
        let mut outputs = HashMap::new();
        outputs.insert(
            key.clone(),
            r#"{
                "streams": [
                    {
                        "index": 0,
                        "codec_type": "video",
                        "codec_name": "h264",
                        "disposition": {"default": 1, "forced": 0, "hearing_impaired": 0},
                        "tags": {"language": "eng", "title": " Main Video "}
                    },
                    {
                        "index": 1,
                        "codec_type": "subtitle",
                        "codec_name": "subrip",
                        "disposition": {"default": 0, "forced": 1, "hearing_impaired": 1},
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
        let adapter = FfprobeInspectAdapter::new(executor.clone(), "ffprobe");

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
            vec!["forced".to_string(), "hearing_impaired".to_string()]
        );

        let calls = executor.calls.lock();
        assert!(calls.is_ok());
        let Ok(calls) = calls else {
            return;
        };
        assert_eq!(calls.as_slice(), &[key]);
    }

    #[test]
    fn ffprobe_adapter_rejects_malformed_json() {
        let key = "ffprobe -v error -show_streams -of json -i /input/movie.mkv".to_string();
        let mut outputs = HashMap::new();
        outputs.insert(key, "{not-json".to_string());
        let adapter = FfprobeInspectAdapter::new(
            Arc::new(StubInspectExecutor {
                outputs,
                calls: Mutex::new(Vec::new()),
            }),
            "ffprobe",
        );

        let result = adapter.inspect("/input/movie.mkv");
        assert!(matches!(result, Err(InspectError::OutputMalformed(_))));
    }
}
