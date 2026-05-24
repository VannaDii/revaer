//! Semantic classification helpers.

use crate::model::MediaStream;

/// Role inferred from normalized stream metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticRole {
    /// Primary content stream.
    Primary,
    /// Stream for forced subtitles.
    Forced,
    /// Commentary stream.
    Commentary,
    /// Descriptive audio stream.
    DescriptiveAudio,
    /// SDH or hearing-impaired subtitles.
    Sdh,
    /// Unknown role.
    Unknown,
}

/// Infer semantic role from dispositions and title.
#[must_use]
pub fn infer_role(stream: &MediaStream) -> SemanticRole {
    if has_flag(stream, "forced") {
        return SemanticRole::Forced;
    }

    if title_contains(stream, "commentary") {
        return SemanticRole::Commentary;
    }

    if title_contains(stream, "descriptive") || has_flag(stream, "visual_impaired") {
        return SemanticRole::DescriptiveAudio;
    }

    if title_contains(stream, "sdh") || title_contains(stream, "hearing") {
        return SemanticRole::Sdh;
    }

    if has_flag(stream, "default") {
        return SemanticRole::Primary;
    }

    SemanticRole::Unknown
}

fn has_flag(stream: &MediaStream, key: &str) -> bool {
    stream.dispositions.iter().any(|value| value == key)
}

fn title_contains(stream: &MediaStream, token: &str) -> bool {
    stream
        .title
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .contains(token)
}

#[cfg(test)]
mod tests {
    use super::{SemanticRole, infer_role};
    use crate::model::{MediaStream, StreamKind};

    #[test]
    fn classify_commentary() {
        let stream = MediaStream {
            stream_id: 1,
            kind: StreamKind::Audio,
            codec: "aac".to_string(),
            language: Some("eng".to_string()),
            title: Some("Director Commentary".to_string()),
            dispositions: Vec::new(),
        };
        assert_eq!(infer_role(&stream), SemanticRole::Commentary);
    }

    #[test]
    fn classify_forced() {
        let stream = MediaStream {
            stream_id: 2,
            kind: StreamKind::Subtitle,
            codec: "srt".to_string(),
            language: Some("eng".to_string()),
            title: None,
            dispositions: vec!["forced".to_string()],
        };
        assert_eq!(infer_role(&stream), SemanticRole::Forced);
    }
}
