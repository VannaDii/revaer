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
    /// Opaque timed or container data stream.
    Data,
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
    /// Audio channel count when the stream is audio and the probe can determine it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<u32>,
    /// Audio channel layout when the stream is audio and the probe can determine it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_layout: Option<String>,
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
    /// Canonical container format names reported by the demuxer.
    #[serde(default)]
    pub container_formats: Vec<String>,
    /// Ordered stream list as observed in container order.
    pub streams: Vec<MediaStream>,
}

/// Desired normalized container metadata entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContainerMetadataEntry {
    /// Lowercase metadata key.
    pub key: String,
    /// Trimmed metadata value.
    pub value: String,
}

/// Desired normalized container chapter timeline entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerChapterEntry {
    /// Inclusive chapter start in milliseconds.
    pub start_millis: i64,
    /// Exclusive chapter end in milliseconds.
    pub end_millis: i64,
    /// Exact chapter metadata rows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata: Vec<ContainerMetadataEntry>,
}

/// Desired output graph after policy application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesiredGraph {
    /// Output path for the replacement artifact.
    pub output_path: String,
    /// Required output container muxer when a target selects one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_format: Option<String>,
    /// Desired container metadata policy when a target selects one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_metadata_policy: Option<String>,
    /// Desired exact container metadata rows when the metadata policy is `replace`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub container_metadata: Vec<ContainerMetadataEntry>,
    /// Desired container chapter policy when a target selects one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_chapter_policy: Option<String>,
    /// Desired exact chapter timeline rows when the chapter policy is `replace`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub container_chapters: Vec<ContainerChapterEntry>,
    /// Required streams in deterministic output order.
    pub streams: Vec<MediaStream>,
}
