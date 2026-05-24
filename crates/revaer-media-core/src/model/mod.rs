//! Core media graph types.

use serde::{Deserialize, Serialize};

/// Stream kind in a media container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamKind {
    /// Video stream.
    Video,
    /// Audio stream.
    Audio,
    /// Subtitle stream.
    Subtitle,
    /// Attachment stream.
    Attachment,
    /// Chapter or timeline metadata stream.
    Chapter,
}

/// Normalized media stream descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaStream {
    /// Stable stream identity within the container.
    pub stream_id: u32,
    /// Stream type.
    pub kind: StreamKind,
    /// Canonical codec identifier.
    pub codec: String,
    /// Canonical ISO-639-3 language code when known.
    pub language: Option<String>,
    /// Human-readable title when present.
    pub title: Option<String>,
    /// Normalized disposition flags.
    pub dispositions: Vec<String>,
}

/// Input media graph discovered from source media.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaGraph {
    /// Source path used for planning context.
    pub source_path: String,
    /// Ordered stream list as observed in container order.
    pub streams: Vec<MediaStream>,
}

/// Desired output graph after policy application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesiredGraph {
    /// Output path for the replacement artifact.
    pub output_path: String,
    /// Required streams in deterministic output order.
    pub streams: Vec<MediaStream>,
}
