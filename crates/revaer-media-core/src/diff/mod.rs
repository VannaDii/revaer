//! Media graph diffing.

use crate::model::{DesiredGraph, MediaGraph};

/// Diff result for graph comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDiff {
    /// Stream ids present in source but absent in desired output.
    pub removed_streams: Vec<u32>,
    /// Stream ids whose codecs differ.
    pub recoded_streams: Vec<u32>,
}

/// Compare source and desired graphs.
#[must_use]
pub fn diff_graphs(source: &MediaGraph, desired: &DesiredGraph) -> GraphDiff {
    let mut removed_streams = Vec::new();
    let mut recoded_streams = Vec::new();

    for stream in &source.streams {
        match desired
            .streams
            .iter()
            .find(|candidate| candidate.stream_id == stream.stream_id)
        {
            Some(target) if target.codec != stream.codec => recoded_streams.push(stream.stream_id),
            Some(_) => {}
            None => removed_streams.push(stream.stream_id),
        }
    }

    GraphDiff {
        removed_streams,
        recoded_streams,
    }
}

#[cfg(test)]
mod tests {
    use super::diff_graphs;
    use crate::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};

    #[test]
    fn diff_removed_and_recoded_streams() {
        let source = MediaGraph {
            source_path: "/input/movie.mkv".to_string(),
            streams: vec![
                MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                },
                MediaStream {
                    stream_id: 1,
                    kind: StreamKind::Audio,
                    codec: "dts".to_string(),
                    language: Some("eng".to_string()),
                    title: None,
                    dispositions: Vec::new(),
                },
            ],
        };
        let desired = DesiredGraph {
            output_path: "/output/movie.mkv".to_string(),
            streams: vec![MediaStream {
                stream_id: 0,
                kind: StreamKind::Video,
                codec: "hevc".to_string(),
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        };

        let diff = diff_graphs(&source, &desired);
        assert_eq!(diff.removed_streams, vec![1]);
        assert_eq!(diff.recoded_streams, vec![0]);
    }
}
