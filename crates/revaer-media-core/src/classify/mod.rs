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
    /// Signs and songs subtitles.
    SignsSongs,
    /// Karaoke subtitles.
    Karaoke,
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

    if title_contains(stream, "descriptive")
        || title_contains(stream, "audio description")
        || has_flag(stream, "visual_impaired")
    {
        return SemanticRole::DescriptiveAudio;
    }

    if title_contains(stream, "sdh") || title_contains(stream, "hearing") {
        return SemanticRole::Sdh;
    }

    if title_contains(stream, "signs") && title_contains(stream, "songs") {
        return SemanticRole::SignsSongs;
    }

    if title_contains(stream, "karaoke") {
        return SemanticRole::Karaoke;
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
        .replace(['-', '_', '&'], " ")
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

    #[test]
    fn classify_descriptive_audio_from_title() {
        let stream = MediaStream {
            stream_id: 3,
            kind: StreamKind::Audio,
            codec: "aac".to_string(),
            language: Some("eng".to_string()),
            title: Some("Audio Description".to_string()),
            dispositions: Vec::new(),
        };
        assert_eq!(infer_role(&stream), SemanticRole::DescriptiveAudio);
    }

    #[test]
    fn classify_sdh_from_hearing_impaired_title() {
        let stream = MediaStream {
            stream_id: 4,
            kind: StreamKind::Subtitle,
            codec: "srt".to_string(),
            language: Some("eng".to_string()),
            title: Some("English Hearing-Impaired".to_string()),
            dispositions: Vec::new(),
        };
        assert_eq!(infer_role(&stream), SemanticRole::Sdh);
    }

    #[test]
    fn classify_signs_and_songs_subtitles() {
        let stream = MediaStream {
            stream_id: 5,
            kind: StreamKind::Subtitle,
            codec: "ass".to_string(),
            language: Some("eng".to_string()),
            title: Some("Signs & Songs".to_string()),
            dispositions: Vec::new(),
        };
        assert_eq!(infer_role(&stream), SemanticRole::SignsSongs);
    }

    #[test]
    fn classify_karaoke_subtitles() {
        let stream = MediaStream {
            stream_id: 6,
            kind: StreamKind::Subtitle,
            codec: "ass".to_string(),
            language: Some("jpn".to_string()),
            title: Some("Karaoke".to_string()),
            dispositions: Vec::new(),
        };
        assert_eq!(infer_role(&stream), SemanticRole::Karaoke);
    }

    #[test]
    fn classify_unknown_when_no_semantic_markers_exist() {
        let stream = MediaStream {
            stream_id: 7,
            kind: StreamKind::Subtitle,
            codec: "srt".to_string(),
            language: Some("eng".to_string()),
            title: Some("English".to_string()),
            dispositions: Vec::new(),
        };
        assert_eq!(infer_role(&stream), SemanticRole::Unknown);
    }
}
