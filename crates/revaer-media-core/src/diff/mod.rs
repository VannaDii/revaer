//! Media graph diffing.

use crate::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};
use crate::normalize::{normalize_audio_channel_layout, normalize_container_format};

/// Stream requiring codec-level recode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecodedStream {
    /// Stream id in source container.
    pub stream_id: u32,
    /// Source stream kind used for operation planning.
    pub kind: StreamKind,
}

/// Container-level desired policy comparison state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContainerPolicyDiff {
    /// Source output already satisfies the desired policy.
    #[default]
    Satisfied,
    /// Source output requires a container-level policy rewrite.
    Mismatched,
}

impl ContainerPolicyDiff {
    /// Return whether the source and desired policy differ.
    #[must_use]
    pub const fn is_mismatched(self) -> bool {
        matches!(self, Self::Mismatched)
    }
}

/// Diff result for graph comparison.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphDiff {
    /// Source container does not satisfy the selected desired muxer.
    pub container_mismatch: bool,
    /// Source container metadata does not satisfy the selected desired policy.
    pub container_metadata_mismatch: bool,
    /// Source chapter timeline comparison against the selected desired policy.
    pub container_chapter_diff: ContainerPolicyDiff,
    /// Stream ids present in source but absent in desired output.
    pub removed_streams: Vec<u32>,
    /// Desired stream ids absent from the source graph.
    pub missing_desired_streams: Vec<u32>,
    /// Stream ids whose desired language or title metadata differs.
    pub stream_metadata_mismatched_streams: Vec<u32>,
    /// Stream ids whose desired disposition flags differ.
    pub disposition_mismatched_streams: Vec<u32>,
    /// Stream ids whose codecs differ.
    pub recoded_streams: Vec<RecodedStream>,
    /// Audio stream ids whose channel count or layout differs.
    pub audio_channel_mismatched_streams: Vec<u32>,
    /// Source retained streams appear in a different order than the desired output.
    pub stream_order_changed: bool,
}

/// Compare source and desired graphs.
#[must_use]
pub fn diff_graphs(source: &MediaGraph, desired: &DesiredGraph) -> GraphDiff {
    let container_mismatch = desired
        .container_format
        .as_deref()
        .is_some_and(|desired_format| {
            !source
                .container_formats
                .iter()
                .any(|source_format| container_formats_match(source_format, desired_format))
        });
    let container_metadata_mismatch =
        desired
            .container_metadata_policy
            .as_deref()
            .is_some_and(|policy| {
                matches!(
                    policy.trim().to_ascii_lowercase().as_str(),
                    "strip" | "replace"
                )
            });
    let container_chapter_diff = if desired
        .container_chapter_policy
        .as_deref()
        .is_some_and(|policy| policy.trim().eq_ignore_ascii_case("strip"))
    {
        ContainerPolicyDiff::Mismatched
    } else {
        ContainerPolicyDiff::Satisfied
    };
    let stream_diff = diff_source_streams(source, desired);
    let missing_desired_streams = missing_desired_streams(source, desired);
    let stream_order_changed = stream_order_changed(source, desired, &missing_desired_streams);

    GraphDiff {
        container_mismatch,
        container_metadata_mismatch,
        container_chapter_diff,
        removed_streams: stream_diff.removed_ids,
        missing_desired_streams,
        stream_metadata_mismatched_streams: stream_diff.metadata_mismatched_ids,
        disposition_mismatched_streams: stream_diff.disposition_mismatched_ids,
        recoded_streams: stream_diff.recoded,
        audio_channel_mismatched_streams: stream_diff.audio_channel_mismatched_ids,
        stream_order_changed,
    }
}

#[derive(Default)]
struct SourceStreamDiff {
    removed_ids: Vec<u32>,
    metadata_mismatched_ids: Vec<u32>,
    disposition_mismatched_ids: Vec<u32>,
    recoded: Vec<RecodedStream>,
    audio_channel_mismatched_ids: Vec<u32>,
}

fn diff_source_streams(source: &MediaGraph, desired: &DesiredGraph) -> SourceStreamDiff {
    let mut diff = SourceStreamDiff::default();

    for stream in &source.streams {
        match desired_stream(desired, stream.stream_id) {
            Some(target) => diff_existing_stream(&mut diff, stream, target),
            None => diff.removed_ids.push(stream.stream_id),
        }
    }

    diff
}

fn desired_stream(desired: &DesiredGraph, stream_id: u32) -> Option<&MediaStream> {
    desired
        .streams
        .iter()
        .find(|candidate| candidate.stream_id == stream_id)
}

fn diff_existing_stream(diff: &mut SourceStreamDiff, source: &MediaStream, desired: &MediaStream) {
    if desired.codec != source.codec {
        diff.recoded.push(RecodedStream {
            stream_id: source.stream_id,
            kind: source.kind,
        });
    }
    if stream_metadata_differs(source, desired) {
        diff.metadata_mismatched_ids.push(source.stream_id);
    }
    if dispositions_differ(source, desired) {
        diff.disposition_mismatched_ids.push(source.stream_id);
    }
    if audio_shape_differs(source, desired) {
        diff.audio_channel_mismatched_ids.push(source.stream_id);
    }
}

fn missing_desired_streams(source: &MediaGraph, desired: &DesiredGraph) -> Vec<u32> {
    desired
        .streams
        .iter()
        .filter(|stream| !source_stream_contains(source, stream.stream_id))
        .map(|stream| stream.stream_id)
        .collect()
}

fn source_stream_contains(source: &MediaGraph, stream_id: u32) -> bool {
    source
        .streams
        .iter()
        .any(|stream| stream.stream_id == stream_id)
}

fn stream_order_changed(
    source: &MediaGraph,
    desired: &DesiredGraph,
    missing_desired_streams: &[u32],
) -> bool {
    let desired_stream_ids = desired_stream_ids(desired);
    let retained_source_stream_ids = retained_source_stream_ids(source, &desired_stream_ids);

    retained_source_stream_ids.len() == desired_stream_ids.len()
        && missing_desired_streams.is_empty()
        && retained_source_stream_ids != desired_stream_ids
}

fn desired_stream_ids(desired: &DesiredGraph) -> Vec<u32> {
    desired
        .streams
        .iter()
        .map(|stream| stream.stream_id)
        .collect()
}

fn retained_source_stream_ids(source: &MediaGraph, desired_stream_ids: &[u32]) -> Vec<u32> {
    source
        .streams
        .iter()
        .filter(|stream| desired_stream_ids.contains(&stream.stream_id))
        .map(|stream| stream.stream_id)
        .collect()
}

fn container_formats_match(source: &str, desired: &str) -> bool {
    normalize_container_format(source) == normalize_container_format(desired)
}

fn audio_shape_differs(source: &MediaStream, desired: &MediaStream) -> bool {
    if source.kind != StreamKind::Audio || desired.kind != StreamKind::Audio {
        return false;
    }

    let channels_differ = desired
        .channels
        .is_some_and(|desired_channels| source.channels != Some(desired_channels));
    let layout_differ =
        normalize_channel_layout(desired.channel_layout.as_deref()).is_some_and(|desired_layout| {
            normalize_channel_layout(source.channel_layout.as_deref()) != Some(desired_layout)
        });

    channels_differ || layout_differ
}

fn stream_metadata_differs(source: &MediaStream, desired: &MediaStream) -> bool {
    normalized_language(source.language.as_deref())
        != normalized_language(desired.language.as_deref())
        || normalized_optional_text(source.title.as_deref())
            != normalized_optional_text(desired.title.as_deref())
}

fn dispositions_differ(source: &MediaStream, desired: &MediaStream) -> bool {
    normalized_dispositions(&source.dispositions) != normalized_dispositions(&desired.dispositions)
}

fn normalized_language(value: Option<&str>) -> Option<String> {
    normalized_optional_text(value).map(|item| item.to_ascii_lowercase())
}

fn normalize_channel_layout(value: Option<&str>) -> Option<String> {
    value.and_then(|item| normalize_audio_channel_layout(item).map(str::to_string))
}

fn normalized_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

fn normalized_dispositions(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{GraphDiff, RecodedStream, diff_graphs};
    use crate::compliance::{Severity, Status, ViolationKind, report_for_status, score_diff};
    use crate::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};

    #[test]
    fn diff_treats_container_aliases_as_the_same_muxer() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: vec!["matroska".to_string(), "webm".to_string()],
            streams: Vec::new(),
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: Some("mkv".to_string()),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };

        assert!(!diff_graphs(&source, &desired).container_mismatch);
    }

    #[test]
    fn diff_detects_container_mismatch() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: vec!["matroska".to_string()],
            streams: Vec::new(),
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: Some("mp4".to_string()),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };

        assert!(diff_graphs(&source, &desired).container_mismatch);
    }

    #[test]
    fn diff_scores_strip_container_metadata_policy() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: vec!["matroska".to_string()],
            streams: Vec::new(),
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            container_metadata_policy: Some("strip".to_string()),
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };

        let diff = diff_graphs(&source, &desired);
        let report = score_diff(&diff);

        assert!(diff.container_metadata_mismatch);
        assert_eq!(report.status, Status::NonCompliant);
        assert!(report.violations.iter().any(|violation| {
            violation.kind == ViolationKind::ContainerMetadataMismatch
                && violation.severity == Severity::Medium
                && violation.stream_id.is_none()
        }));
    }

    #[test]
    fn diff_scores_strip_container_chapter_policy() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: vec!["matroska".to_string()],
            streams: Vec::new(),
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: Some("matroska".to_string()),
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: Some("strip".to_string()),
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: Vec::new(),
        };

        let diff = diff_graphs(&source, &desired);
        let report = score_diff(&diff);

        assert!(diff.container_chapter_diff.is_mismatched());
        assert_eq!(report.status, Status::NonCompliant);
        assert!(report.violations.iter().any(|violation| {
            violation.kind == ViolationKind::ContainerChapterMismatch
                && violation.severity == Severity::Medium
                && violation.stream_id.is_none()
        }));
    }

    #[test]
    fn diff_removed_and_recoded_streams() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                    codec: "dts".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };

        let diff = diff_graphs(&source, &desired);
        assert_eq!(diff.removed_streams, vec![1]);
        assert_eq!(
            diff.recoded_streams,
            vec![RecodedStream {
                stream_id: 0,
                kind: StreamKind::Video,
            }]
        );
        assert!(diff.audio_channel_mismatched_streams.is_empty());
        assert!(!diff.stream_order_changed);
    }

    #[test]
    fn diff_detects_desired_stream_order_change() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };

        let diff = diff_graphs(&source, &desired);

        assert!(diff.removed_streams.is_empty());
        assert!(diff.recoded_streams.is_empty());
        assert!(diff.audio_channel_mismatched_streams.is_empty());
        assert!(diff.stream_order_changed);
    }

    #[test]
    fn diff_detects_audio_channel_shape_changes() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(6),
                channel_layout: Some("5.1(side)".to_string()),
                language: Some("eng".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["default".to_string()],
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: Some("eng".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["default".to_string()],
            }],
        };

        let diff = diff_graphs(&source, &desired);

        assert!(diff.recoded_streams.is_empty());
        assert_eq!(diff.audio_channel_mismatched_streams, vec![1]);
    }

    #[test]
    fn diff_detects_stream_language_and_title_metadata_changes() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 2,
                kind: StreamKind::Subtitle,
                codec: "subrip".to_string(),
                channels: None,
                channel_layout: None,
                language: Some("jpn".to_string()),
                title: Some("Signs".to_string()),
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 2,
                kind: StreamKind::Subtitle,
                codec: "subrip".to_string(),
                channels: None,
                channel_layout: None,
                language: Some("eng".to_string()),
                title: Some("English Signs".to_string()),
                dispositions: Vec::new(),
            }],
        };

        let diff = diff_graphs(&source, &desired);

        assert_eq!(diff.stream_metadata_mismatched_streams, vec![2]);
        assert!(diff.disposition_mismatched_streams.is_empty());
        assert!(diff.recoded_streams.is_empty());
    }

    #[test]
    fn diff_detects_stream_disposition_changes_without_order_sensitivity() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: Some("eng".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["default".to_string()],
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: Some("ENG".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["forced".to_string(), "default".to_string()],
            }],
        };

        let diff = diff_graphs(&source, &desired);

        assert!(diff.stream_metadata_mismatched_streams.is_empty());
        assert_eq!(diff.disposition_mismatched_streams, vec![1]);
        assert!(diff.recoded_streams.is_empty());
    }

    #[test]
    fn diff_detects_missing_desired_streams() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "h264".to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 9,
                    kind: StreamKind::Audio,
                    codec: "aac".to_string(),
                    channels: Some(2),
                    channel_layout: Some("stereo".to_string()),
                    language: Some("eng".to_string()),
                    title: Some("Commentary".to_string()),
                    dispositions: Vec::new(),
                },
            ],
        };

        let diff = diff_graphs(&source, &desired);

        assert!(diff.removed_streams.is_empty());
        assert_eq!(diff.missing_desired_streams, vec![9]);
        assert!(diff.recoded_streams.is_empty());
        assert!(diff.audio_channel_mismatched_streams.is_empty());
        assert!(!diff.stream_order_changed);
    }

    #[test]
    fn diff_reports_channel_shape_when_audio_codec_also_changes() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            container_formats: Vec::new(),
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "dts".to_string(),
                channels: Some(6),
                channel_layout: Some("5.1(side)".to_string()),
                language: Some("eng".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["default".to_string()],
            }],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            container_format: None,
            container_metadata_policy: None,
            container_metadata: Vec::new(),
            container_chapter_policy: None,
            container_chapters: Vec::new(),
            container_attachment_policy: None,
            streams: vec![MediaStream {
                stream_id: 1,
                kind: StreamKind::Audio,
                codec: "aac".to_string(),
                channels: Some(2),
                channel_layout: Some("stereo".to_string()),
                language: Some("eng".to_string()),
                title: Some("Main".to_string()),
                dispositions: vec!["default".to_string()],
            }],
        };

        let diff = diff_graphs(&source, &desired);

        assert_eq!(
            diff.recoded_streams,
            vec![RecodedStream {
                stream_id: 1,
                kind: StreamKind::Audio,
            }]
        );
        assert_eq!(diff.audio_channel_mismatched_streams, vec![1]);
    }

    #[test]
    fn score_diff_reports_removed_and_recoded_stream_violations() {
        let diff = GraphDiff {
            removed_streams: vec![3],
            recoded_streams: vec![
                RecodedStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                },
                RecodedStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                },
            ],
            ..GraphDiff::default()
        };

        let report = score_diff(&diff);

        assert_eq!(report.status, Status::NonCompliant);
        assert_eq!(report.score, 40);
        assert_eq!(
            report
                .violations
                .iter()
                .map(|item| (item.kind, item.severity, item.stream_id))
                .collect::<Vec<_>>(),
            vec![
                (ViolationKind::RemovedStream, Severity::Medium, Some(3)),
                (ViolationKind::AudioCodecMismatch, Severity::High, Some(1)),
                (ViolationKind::VideoCodecMismatch, Severity::High, Some(0)),
            ]
        );
    }

    #[test]
    fn score_diff_reports_stream_order_violation() {
        let report = score_diff(&GraphDiff {
            stream_order_changed: true,
            ..GraphDiff::default()
        });

        assert_eq!(report.status, Status::NonCompliant);
        assert_eq!(report.score, 90);
        assert_eq!(
            report
                .violations
                .iter()
                .map(|item| (item.kind, item.severity, item.stream_id))
                .collect::<Vec<_>>(),
            vec![(ViolationKind::StreamOrderMismatch, Severity::Medium, None)]
        );
    }

    #[test]
    fn score_diff_reports_container_violation() {
        let report = score_diff(&GraphDiff {
            container_mismatch: true,
            ..GraphDiff::default()
        });

        assert_eq!(report.status, Status::NonCompliant);
        assert_eq!(report.score, 90);
        assert_eq!(
            report
                .violations
                .iter()
                .map(|item| (item.kind, item.severity, item.stream_id))
                .collect::<Vec<_>>(),
            vec![(ViolationKind::ContainerMismatch, Severity::Medium, None)]
        );
    }

    #[test]
    fn score_diff_reports_audio_channel_shape_violation() {
        let report = score_diff(&GraphDiff {
            audio_channel_mismatched_streams: vec![1],
            ..GraphDiff::default()
        });

        assert_eq!(report.status, Status::NonCompliant);
        assert_eq!(report.score, 80);
        assert_eq!(
            report
                .violations
                .iter()
                .map(|item| (item.kind, item.severity, item.stream_id))
                .collect::<Vec<_>>(),
            vec![(
                ViolationKind::AudioChannelShapeMismatch,
                Severity::High,
                Some(1)
            )]
        );
    }

    #[test]
    fn score_diff_reports_stream_metadata_and_disposition_violations() {
        let report = score_diff(&GraphDiff {
            stream_metadata_mismatched_streams: vec![2],
            disposition_mismatched_streams: vec![3],
            ..GraphDiff::default()
        });

        assert_eq!(report.status, Status::NonCompliant);
        assert_eq!(report.score, 80);
        assert_eq!(
            report
                .violations
                .iter()
                .map(|item| (item.kind, item.severity, item.stream_id))
                .collect::<Vec<_>>(),
            vec![
                (
                    ViolationKind::StreamMetadataMismatch,
                    Severity::Medium,
                    Some(2)
                ),
                (
                    ViolationKind::StreamDispositionMismatch,
                    Severity::Medium,
                    Some(3)
                ),
            ]
        );
    }

    #[test]
    fn score_diff_reports_missing_required_stream_violation() {
        let report = score_diff(&GraphDiff {
            missing_desired_streams: vec![9],
            ..GraphDiff::default()
        });

        assert_eq!(report.status, Status::NonCompliant);
        assert_eq!(report.score, 70);
        assert_eq!(
            report
                .violations
                .iter()
                .map(|item| (item.kind, item.severity, item.stream_id))
                .collect::<Vec<_>>(),
            vec![(
                ViolationKind::MissingRequiredStream,
                Severity::High,
                Some(9)
            )]
        );
    }

    #[test]
    fn score_diff_returns_full_score_for_empty_diff() {
        let report = score_diff(&GraphDiff::default());

        assert_eq!(report.status, Status::Compliant);
        assert_eq!(report.score, 100);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn report_for_status_preserves_terminal_compliance_status() {
        let statuses = [
            Status::Unsupported,
            Status::DryRunPlanned,
            Status::FailedValidation,
            Status::FailedExecution,
            Status::FailedVerification,
        ];

        for status in statuses {
            let report = report_for_status(status);
            assert_eq!(report.status, status);
            assert!(report.violations.is_empty());
        }
    }
}
