//! Semantic classification helpers.

use crate::model::{MediaStream, StreamKind};

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

/// Return a deterministically ranked clone of the provided stream list.
#[must_use]
pub fn rank_streams(streams: &[MediaStream], language_priority: &[&str]) -> Vec<MediaStream> {
    let mut ranked = streams.to_vec();
    ranked.sort_by_key(|stream| {
        (
            kind_rank(stream.kind),
            language_rank(stream.language.as_deref(), language_priority),
            role_rank(infer_role(stream)),
            codec_rank(stream.kind, &stream.codec),
            stream.stream_id,
        )
    });
    ranked
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

const fn kind_rank(kind: StreamKind) -> usize {
    match kind {
        StreamKind::Video => 0,
        StreamKind::Audio => 1,
        StreamKind::Subtitle => 2,
        StreamKind::Attachment => 3,
        StreamKind::Chapter => 4,
    }
}

fn language_rank(language: Option<&str>, priority: &[&str]) -> usize {
    let Some(language) = language else {
        return priority.len() + 1;
    };
    let normalized = language.trim().to_ascii_lowercase();
    priority
        .iter()
        .position(|candidate| language_matches(&normalized, candidate))
        .unwrap_or(priority.len())
}

fn language_matches(language: &str, candidate: &str) -> bool {
    let normalized_candidate = candidate.trim().to_ascii_lowercase();
    language == normalized_candidate
        || language
            .strip_prefix(&normalized_candidate)
            .is_some_and(|suffix| suffix.starts_with(['-', '_']))
}

const fn role_rank(role: SemanticRole) -> usize {
    match role {
        SemanticRole::Forced => 0,
        SemanticRole::Primary => 1,
        SemanticRole::SignsSongs => 2,
        SemanticRole::Sdh => 3,
        SemanticRole::DescriptiveAudio => 4,
        SemanticRole::Karaoke => 5,
        SemanticRole::Unknown => 6,
        SemanticRole::Commentary => 7,
    }
}

fn codec_rank(kind: StreamKind, codec: &str) -> usize {
    let normalized = codec.trim().to_ascii_lowercase().replace('-', "_");
    match kind {
        StreamKind::Audio => match normalized.as_str() {
            "truehd" => 0,
            "dts_hd_ma" | "dtshd_ma" => 1,
            "dts" => 2,
            "eac3" => 3,
            "ac3" => 4,
            "aac" => 5,
            "mp3" => 6,
            _ => 100,
        },
        StreamKind::Subtitle => match normalized.as_str() {
            "ass" => 0,
            "srt" | "subrip" => 1,
            "vtt" => 2,
            "pgs" => 3,
            "vobsub" => 4,
            _ => 100,
        },
        StreamKind::Video => match normalized.as_str() {
            "av1" => 0,
            "hevc" | "h265" => 1,
            "h264" => 2,
            _ => 100,
        },
        StreamKind::Attachment | StreamKind::Chapter => 100,
    }
}

#[cfg(test)]
mod tests {
    use super::{SemanticRole, infer_role, rank_streams};
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

    #[test]
    fn rank_streams_orders_preferred_primary_audio_by_codec_quality() {
        let streams = vec![
            MediaStream {
                stream_id: 3,
                kind: StreamKind::Audio,
                codec: "truehd".to_string(),
                language: Some("jpn".to_string()),
                title: None,
                dispositions: vec!["default".to_string()],
            },
            MediaStream {
                stream_id: 2,
                kind: StreamKind::Audio,
                codec: "truehd".to_string(),
                language: Some("eng".to_string()),
                title: None,
                dispositions: vec!["default".to_string()],
            },
            MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                language: Some("eng".to_string()),
                title: None,
                dispositions: vec!["default".to_string()],
            },
        ];

        let ranked = rank_streams(&streams, &["eng", "jpn"]);
        let ids = ranked
            .iter()
            .map(|stream| stream.stream_id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![2, 1, 3]);
    }

    #[test]
    fn rank_streams_orders_subtitle_roles_before_codec_quality() {
        let streams = vec![
            MediaStream {
                stream_id: 4,
                kind: StreamKind::Subtitle,
                codec: "ass".to_string(),
                language: Some("eng".to_string()),
                title: Some("Karaoke".to_string()),
                dispositions: Vec::new(),
            },
            MediaStream {
                stream_id: 2,
                kind: StreamKind::Subtitle,
                codec: "srt".to_string(),
                language: Some("eng".to_string()),
                title: Some("Signs & Songs".to_string()),
                dispositions: Vec::new(),
            },
            MediaStream {
                stream_id: 3,
                kind: StreamKind::Subtitle,
                codec: "ass".to_string(),
                language: Some("eng".to_string()),
                title: Some("English SDH".to_string()),
                dispositions: Vec::new(),
            },
        ];

        let ranked = rank_streams(&streams, &["eng"]);
        let ids = ranked
            .iter()
            .map(|stream| stream.stream_id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![2, 3, 4]);
    }

    #[test]
    fn rank_streams_breaks_complete_ties_by_stream_id() {
        let streams = vec![
            MediaStream {
                stream_id: 9,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                language: Some("eng".to_string()),
                title: None,
                dispositions: Vec::new(),
            },
            MediaStream {
                stream_id: 2,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                language: Some("eng".to_string()),
                title: None,
                dispositions: Vec::new(),
            },
        ];

        let ranked = rank_streams(&streams, &["eng"]);
        let ids = ranked
            .iter()
            .map(|stream| stream.stream_id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![2, 9]);
    }
}
