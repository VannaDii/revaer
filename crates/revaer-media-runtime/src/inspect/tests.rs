use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use revaer_media_core::model::StreamKind;

use super::parse::{parse_source, validate_sidecar_output};
use super::*;
use crate::sidecar::{SidecarDiscoverer, SidecarDiscoveryError, SidecarFormat, SidecarSubtitle};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

const SOURCE_JSON: &str = r#"{
  "streams": [
    {"index": 2, "codec_type": "audio", "codec_name": "aac", "channels": 2,
     "channel_layout": "stereo", "sample_rate": "48000", "tags": {"language": "EN"}},
    {"index": 0, "codec_type": "video", "codec_name": "h264", "width": 1920,
     "height": 1080, "pix_fmt": "YUV420P", "avg_frame_rate": "24000/1001"}
  ],
  "chapters": [
    {"id": 4, "start_time": "10.000000", "end_time": "20.000000"},
    {"id": 3, "start_time": "0.000000", "end_time": "10.000000"}
  ],
  "format": {"format_name": "matroska,webm", "duration": "20.000000", "size": "4"}
}"#;
const SIDECAR_JSON: &str =
    r#"{"streams":[{"index":0,"codec_type":"subtitle","codec_name":"subrip"}]}"#;
const VOBSUB_JSON: &str =
    r#"{"streams":[{"index":0,"codec_type":"subtitle","codec_name":"dvd_subtitle"}]}"#;

#[derive(Clone)]
struct StaticDiscoverer {
    sidecars: Vec<SidecarSubtitle>,
}

impl SidecarDiscoverer for StaticDiscoverer {
    fn discover(&self, _source_path: &Path) -> Result<Vec<SidecarSubtitle>, SidecarDiscoveryError> {
        Ok(self.sidecars.clone())
    }
}

#[derive(Default)]
struct QueueExecutor {
    outputs: Mutex<VecDeque<InspectProbeOutput>>,
    requests: Mutex<Vec<InspectProbeRequest>>,
}

impl QueueExecutor {
    fn from_stdout(outputs: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self {
            outputs: Mutex::new(
                outputs
                    .into_iter()
                    .map(|stdout| InspectProbeOutput {
                        stdout,
                        stderr: Vec::new(),
                    })
                    .collect(),
            ),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn from_outputs(outputs: impl IntoIterator<Item = InspectProbeOutput>) -> Self {
        Self {
            outputs: Mutex::new(outputs.into_iter().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn request_count(&self) -> Result<usize, InspectError> {
        self.requests
            .lock()
            .map(|requests| requests.len())
            .map_err(|error| InspectError::ProbeFailed(error.to_string()))
    }

    fn requests(&self) -> Result<Vec<InspectProbeRequest>, InspectError> {
        self.requests
            .lock()
            .map(|requests| requests.clone())
            .map_err(|error| InspectError::ProbeFailed(error.to_string()))
    }
}

impl InspectProbeExecutor for QueueExecutor {
    fn run(
        &self,
        request: &InspectProbeRequest,
        _cancellation: &dyn InspectCancellation,
    ) -> Result<InspectProbeOutput, InspectError> {
        self.requests
            .lock()
            .map_err(|error| InspectError::ProbeFailed(error.to_string()))?
            .push(request.clone());
        self.outputs
            .lock()
            .map_err(|error| InspectError::ProbeFailed(error.to_string()))?
            .pop_front()
            .ok_or_else(|| InspectError::ProbeFailed("unexpected probe".to_string()))
    }
}

struct MutatingExecutor {
    path: PathBuf,
}

impl InspectProbeExecutor for MutatingExecutor {
    fn run(
        &self,
        _request: &InspectProbeRequest,
        _cancellation: &dyn InspectCancellation,
    ) -> Result<InspectProbeOutput, InspectError> {
        fs::write(&self.path, b"changed while probing")
            .map_err(|error| InspectError::ProbeFailed(error.to_string()))?;
        Ok(InspectProbeOutput {
            stdout: SOURCE_JSON.as_bytes().to_vec(),
            stderr: Vec::new(),
        })
    }
}

#[test]
fn inspection_limits_default_to_reviewed_values() {
    assert_eq!(InspectionLimits::default(), InspectionLimits::reviewed());
}

#[test]
fn normalizes_complete_report_and_deterministic_result_order()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("deterministic")?;
    let source = write_source(&directory)?;
    let french = write_sidecar(&directory, "movie.fr.srt", b"french", SidecarFormat::Srt)?;
    let english = write_sidecar(&directory, "movie.en.srt", b"english", SidecarFormat::Srt)?;
    let executor = Arc::new(QueueExecutor::from_stdout([
        SOURCE_JSON.as_bytes().to_vec(),
        SIDECAR_JSON.as_bytes().to_vec(),
        SIDECAR_JSON.as_bytes().to_vec(),
    ]));
    let adapter = adapter(
        Arc::clone(&executor),
        vec![french.clone(), english.clone()],
        InspectionLimits::reviewed(),
    );

    let inspection = adapter.inspect(&source)?;

    assert_eq!(inspection.graph.streams[0].stream_id, 0);
    assert_eq!(inspection.graph.streams[1].stream_id, 2);
    assert_eq!(inspection.streams[0].stream_id, 0);
    assert_eq!(inspection.chapters[0].chapter_id, 3);
    assert_eq!(inspection.container.formats, ["matroska", "webm"]);
    assert_eq!(inspection.sidecars[0].path, english.path);
    assert_eq!(inspection.sidecars[1].path, french.path);
    let requests = executor.requests()?;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].args.last(), Some(&source.into_os_string()));

    remove_temp_directory(&directory)
}

#[test]
fn normalizes_full_technical_metadata_and_dispositions() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("technical")?;
    let source = write_source(&directory)?;
    let output = br#"{
      "streams":[{
        "index":0,"codec_type":"video","codec_name":"h264","profile":"High",
        "duration":"1.2509","bit_rate":"9000","sample_rate":"99999","width":1920,
        "height":1080,"pix_fmt":"YUV420P10LE","sample_aspect_ratio":"1:1",
        "display_aspect_ratio":"16:9","avg_frame_rate":"0/0","color_range":"TV",
        "color_space":"BT2020NC","color_transfer":"SMPTE2084","color_primaries":"BT2020",
        "chroma_location":"LEFT","field_order":"PROGRESSIVE",
        "disposition":{"default":1,"forced":1,"hearing_impaired":1,"visual_impaired":1},
        "tags":{"title":" Main Video ","provider":" studio "},
        "side_data_list":[
          {"side_data_type":"HDR10+","red_x":"34000/50000"},
          {"side_data_type":"hdr10+","max_content":1000}
        ]
      }],
      "chapters":[{"id":1,"start_time":"-0.500","end_time":"0.250",
                   "tags":{"title":" Intro "}}],
      "format":{"format_name":"mov","duration":"1.2509","start_time":"-0.500",
                "size":"5","bit_rate":"9000","tags":{"ENCODER":" tool "}}
    }"#;

    let inspection = parse_source(&source, output)?;

    assert_eq!(inspection.graph.container_formats, ["mp4"]);
    assert_eq!(inspection.graph.streams[0].dispositions.len(), 4);
    assert_eq!(inspection.container.start_time_millis, Some(-500));
    assert_eq!(inspection.streams[0].duration_millis, Some(1250));
    assert_eq!(inspection.streams[0].sample_rate, None);
    assert_eq!(inspection.streams[0].average_frame_rate, None);
    assert_eq!(inspection.streams[0].side_data_types, ["hdr10+"]);
    assert_eq!(inspection.streams[0].side_data.len(), 1);
    assert_eq!(
        inspection.streams[0].side_data[0].metadata,
        [
            MetadataEntry {
                key: "max_content".to_string(),
                value: "1000".to_string(),
            },
            MetadataEntry {
                key: "red_x".to_string(),
                value: "34000/50000".to_string(),
            },
        ]
    );
    assert_eq!(inspection.streams[0].metadata.len(), 2);

    remove_temp_directory(&directory)
}

#[test]
fn accepts_unindexed_frame_records_without_attributing_side_data()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("unindexed-frame")?;
    let source = write_source(&directory)?;
    let inspection = parse_source(
        &source,
        br#"{
          "frames":[{
            "media_type":"subtitle",
            "side_data_list":[{"side_data_type":"unattributed"}]
          }],
          "streams":[{
            "index":0,"codec_type":"subtitle","codec_name":"webvtt"
          }],
          "format":{"format_name":"matroska,webm"}
        }"#,
    )?;

    assert_eq!(inspection.graph.streams[0].kind, StreamKind::Subtitle);
    assert_eq!(inspection.streams[0].side_data_types, Vec::<String>::new());

    remove_temp_directory(&directory)
}

#[test]
fn merges_only_indexed_frame_side_data_into_its_stream() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("indexed-frame")?;
    let source = write_source(&directory)?;
    let inspection = parse_source(
        &source,
        br#"{
          "frames":[
            {"side_data_list":[{"side_data_type":"unattributed"}]},
            {"stream_index":1,"side_data_list":[{"side_data_type":"HDR10+"}]}
          ],
          "streams":[
            {"index":0,"codec_type":"video","codec_name":"h264"},
            {"index":1,"codec_type":"video","codec_name":"hevc"}
          ],
          "format":{"format_name":"matroska"}
        }"#,
    )?;

    assert_eq!(inspection.streams[0].side_data_types, Vec::<String>::new());
    assert_eq!(inspection.streams[1].side_data_types, ["hdr10+"]);

    remove_temp_directory(&directory)
}

#[test]
fn source_probe_requests_a_bounded_frame_sample() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("frame-probe-args")?;
    let source = write_source(&directory)?;
    let executor = Arc::new(QueueExecutor::from_stdout([SOURCE_JSON
        .as_bytes()
        .to_vec()]));
    let adapter = adapter(
        Arc::clone(&executor),
        Vec::new(),
        InspectionLimits::reviewed(),
    );

    adapter.inspect(&source)?;

    let requests = executor.requests()?;
    assert_eq!(requests.len(), 1);
    assert!(requests[0].args.windows(2).any(|arguments| {
        arguments == [OsString::from("-read_intervals"), OsString::from("%+#1")]
    }));
    assert!(requests[0].args.contains(&OsString::from("-show_frames")));

    remove_temp_directory(&directory)
}

#[test]
fn attachment_without_codec_name_uses_stable_marker() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("attachment-no-codec")?;
    let source = write_source(&directory)?;
    let inspection = parse_source(
        &source,
        br#"{"streams":[{"index":0,"codec_type":"attachment","tags":{"filename":"note.txt"}}],"format":{"format_name":"matroska"}}"#,
    )?;

    assert_eq!(inspection.graph.streams[0].kind, StreamKind::Attachment);
    assert_eq!(inspection.graph.streams[0].codec, "attachment");

    remove_temp_directory(&directory)
}

#[test]
fn attachment_codec_name_is_normalized_to_stable_marker() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = temp_directory("attachment-codec")?;
    let source = write_source(&directory)?;
    let inspection = parse_source(
        &source,
        br#"{"streams":[{"index":0,"codec_type":"attachment","codec_name":"text"}],"format":{"format_name":"matroska"}}"#,
    )?;

    assert_eq!(inspection.graph.streams[0].codec, "attachment");

    remove_temp_directory(&directory)
}

#[test]
fn normalize_probe_graph_covers_supported_kinds_and_rejects_invalid_fields() {
    let streams = [
        "video",
        "audio",
        "subtitle",
        "attachment",
        "chapter",
        "data",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, kind)| ProbeStream {
        stream_id: u32::try_from(index).unwrap_or(u32::MAX),
        kind: kind.to_string(),
        codec: " codec ".to_string(),
        channels: Some(0),
        channel_layout: Some(" Stereo ".to_string()),
        language: Some(" EN ".to_string()),
        title: Some(" Title ".to_string()),
        dispositions: vec![" DEFAULT ".to_string()],
    })
    .collect::<Vec<_>>();
    let graph = normalize_probe_graph(ProbeGraph {
        source_path: " source ".to_string(),
        streams,
    });
    assert!(graph.is_ok());

    for (kind, codec, expected) in [
        ("unknown", "h264", "invalid stream kind"),
        ("video", " ", "stream codec is missing"),
    ] {
        let result = normalize_probe_graph(ProbeGraph {
            source_path: "source".to_string(),
            streams: vec![ProbeStream {
                stream_id: 0,
                kind: kind.to_string(),
                codec: codec.to_string(),
                channels: None,
                channel_layout: None,
                language: None,
                title: None,
                dispositions: Vec::new(),
            }],
        });
        assert!(result.is_err_and(|error| error.to_string().contains(expected)));
    }
}

#[test]
fn parser_rejects_invalid_container_timeline_and_numeric_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("parse-errors")?;
    let source = write_source(&directory)?;
    let invalid_documents = [
        r#"{"streams":[],"format":null}"#,
        r#"{"streams":[],"format":{"format_name":" "}}"#,
        r#"{"streams":[],"format":{"format_name":"mkv"}}"#,
        r#"{"streams":[],"chapters":[{"id":1,"start_time":"1","end_time":"1"}],"format":{"format_name":"mkv"}}"#,
        r#"{"streams":[],"chapters":[{"id":1,"start_time":"0","end_time":"1"},{"id":1,"start_time":"1","end_time":"2"}],"format":{"format_name":"mkv"}}"#,
        r#"{"streams":[],"format":{"format_name":"mkv","duration":"-1"}}"#,
        r#"{"streams":[],"format":{"format_name":"mkv","duration":"1.bad"}}"#,
        r#"{"streams":[],"format":{"format_name":"mkv","duration":"bad"}}"#,
        r#"{"streams":[],"format":{"format_name":"mkv","duration":"9223372036854775807"}}"#,
        r#"{"streams":[],"format":{"format_name":"mkv","size":"-1"}}"#,
        r#"{"streams":[{"index":0,"codec_type":"audio","codec_name":"aac","sample_rate":"4294967296"}],"format":{"format_name":"mkv"}}"#,
    ];
    for document in invalid_documents {
        assert!(matches!(
            parse_source(&source, document.as_bytes()),
            Err(InspectError::OutputMalformed(_))
        ));
    }

    remove_temp_directory(&directory)
}

#[test]
fn sidecar_probe_requires_one_matching_subtitle_stream() {
    let path = Path::new("subtitle.srt");
    for output in [
        br#"{"streams":[]}"#.as_slice(),
        br#"{"streams":[{"index":0,"codec_type":"audio","codec_name":"aac"}]}"#.as_slice(),
        br#"{"streams":[{"index":0,"codec_type":"subtitle","codec_name":"ass"}]}"#.as_slice(),
    ] {
        assert!(matches!(
            validate_sidecar_output(path, SidecarFormat::Srt, output),
            Err(InspectError::OutputMalformed(_))
        ));
    }
    for (format, codec) in [
        (SidecarFormat::Srt, "subrip"),
        (SidecarFormat::Ass, "ass"),
        (SidecarFormat::Vtt, "webvtt"),
        (SidecarFormat::Sup, "hdmv_pgs_subtitle"),
        (SidecarFormat::Sub, "microdvd"),
        (SidecarFormat::VobSub, "dvd_subtitle"),
    ] {
        let output = format!(
            "{{\"streams\":[{{\"index\":0,\"codec_type\":\"subtitle\",\"codec_name\":\"{codec}\"}}]}}"
        );
        assert!(validate_sidecar_output(path, format, output.as_bytes()).is_ok());
    }
}

#[test]
fn logical_sidecar_budget_accepts_maximum() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("sidecar-max")?;
    let source = write_source(&directory)?;
    let mut sidecars = Vec::new();
    for index in 0..64 {
        sidecars.push(write_sidecar(
            &directory,
            &format!("movie.{index:02}.srt"),
            b"x",
            SidecarFormat::Srt,
        )?);
    }
    let outputs = std::iter::once(SOURCE_JSON.as_bytes().to_vec())
        .chain((0..64).map(|_| SIDECAR_JSON.as_bytes().to_vec()));
    let executor = Arc::new(QueueExecutor::from_stdout(outputs));
    let adapter = adapter(
        Arc::clone(&executor),
        sidecars,
        InspectionLimits::reviewed(),
    );

    assert_eq!(adapter.inspect(&source)?.sidecars.len(), 64);
    assert_eq!(executor.request_count()?, 65);

    remove_temp_directory(&directory)
}

#[test]
fn logical_sidecar_budget_rejects_maximum_plus_one() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("sidecar-max-plus-one")?;
    let source = write_source(&directory)?;
    let mut sidecars = Vec::new();
    for index in 0..65 {
        sidecars.push(write_sidecar(
            &directory,
            &format!("movie.{index:02}.srt"),
            b"x",
            SidecarFormat::Srt,
        )?);
    }
    let executor = Arc::new(QueueExecutor::default());
    let adapter = adapter(
        Arc::clone(&executor),
        sidecars,
        InspectionLimits::reviewed(),
    );

    assert!(matches!(
        adapter.inspect(&source),
        Err(InspectError::SidecarLimitExceeded(64))
    ));
    assert_eq!(executor.request_count()?, 0);

    remove_temp_directory(&directory)
}

#[test]
fn aggregate_output_budget_accepts_exact_maximum() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("output-max")?;
    let source = write_source(&directory)?;
    let output = padded_source_json(1024)?;
    let limits = test_limits(1024);
    let adapter = adapter(
        Arc::new(QueueExecutor::from_stdout([output])),
        Vec::new(),
        limits,
    );

    assert!(adapter.inspect(&source).is_ok());

    remove_temp_directory(&directory)
}

#[test]
fn aggregate_output_budget_rejects_maximum_plus_one() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("output-max-plus-one")?;
    let source = write_source(&directory)?;
    let output = padded_source_json(1025)?;
    let adapter = adapter(
        Arc::new(QueueExecutor::from_stdout([output])),
        Vec::new(),
        test_limits(1024),
    );

    assert!(matches!(
        adapter.inspect(&source),
        Err(InspectError::TotalOutputLimitExceeded(1024))
    ));

    remove_temp_directory(&directory)
}

#[test]
fn default_adapter_uses_filesystem_sidecar_discovery() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("default-adapter")?;
    let source = write_source(&directory)?;
    fs::write(directory.join("movie.en.srt"), b"subtitle")?;
    let executor = Arc::new(QueueExecutor::from_stdout([
        SOURCE_JSON.as_bytes().to_vec(),
        SIDECAR_JSON.as_bytes().to_vec(),
    ]));
    let collaborator: Arc<dyn InspectProbeExecutor> = executor.clone();
    let adapter = FfprobeInspectAdapter::new(collaborator, "ffprobe");

    assert_eq!(adapter.inspect(&source)?.sidecars.len(), 1);
    assert_eq!(executor.request_count()?, 2);

    remove_temp_directory(&directory)
}

#[test]
fn adapter_defends_process_and_invocation_limits_from_collaborator_violations()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("collaborator-limits")?;
    let source = write_source(&directory)?;
    let limits = InspectionLimits {
        max_invocations: 1,
        max_sidecars: 1,
        deadline: Duration::from_secs(1),
        max_stdout_bytes: 4,
        max_stderr_bytes: 2,
        max_total_output_bytes: 10,
    };
    let stdout_adapter = adapter(
        Arc::new(QueueExecutor::from_outputs([InspectProbeOutput {
            stdout: vec![b'x'; 5],
            stderr: Vec::new(),
        }])),
        Vec::new(),
        limits,
    );
    assert!(matches!(
        stdout_adapter.inspect(&source),
        Err(InspectError::ProcessOutputLimitExceeded {
            stream: "stdout",
            maximum_bytes: 4
        })
    ));

    let stderr_adapter = adapter(
        Arc::new(QueueExecutor::from_outputs([InspectProbeOutput {
            stdout: b"{}".to_vec(),
            stderr: vec![b'x'; 3],
        }])),
        Vec::new(),
        limits,
    );
    assert!(matches!(
        stderr_adapter.inspect(&source),
        Err(InspectError::ProcessOutputLimitExceeded {
            stream: "stderr",
            maximum_bytes: 2
        })
    ));

    let sidecar = write_sidecar(&directory, "movie.en.srt", b"subtitle", SidecarFormat::Srt)?;
    let invocation_adapter = adapter(Arc::new(QueueExecutor::default()), vec![sidecar], limits);
    assert!(matches!(
        invocation_adapter.inspect(&source),
        Err(InspectError::InvocationLimitExceeded(1))
    ));

    remove_temp_directory(&directory)
}

#[test]
fn adapter_rejects_invalid_sidecar_physical_shapes() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("inventory-shapes")?;
    let source = write_source(&directory)?;
    let path = directory.join("movie.en.srt");
    let companion = directory.join("movie.en.sub");
    fs::write(&path, b"one")?;
    fs::write(&companion, b"two")?;
    let cases = [
        SidecarSubtitle {
            path: path.clone(),
            companion_path: None,
            language: None,
            role: None,
            format: SidecarFormat::VobSub,
            size_bytes: 3,
        },
        SidecarSubtitle {
            path: path.clone(),
            companion_path: Some(companion),
            language: None,
            role: None,
            format: SidecarFormat::Srt,
            size_bytes: 6,
        },
    ];
    for sidecar in cases {
        let invalid = adapter(
            Arc::new(QueueExecutor::default()),
            vec![sidecar],
            InspectionLimits::reviewed(),
        );
        assert!(matches!(
            invalid.inspect(&source),
            Err(InspectError::InvalidSidecarInventory(_))
        ));
    }

    let duplicate = SidecarSubtitle {
        path,
        companion_path: None,
        language: None,
        role: None,
        format: SidecarFormat::Srt,
        size_bytes: 3,
    };
    let invalid = adapter(
        Arc::new(QueueExecutor::default()),
        vec![duplicate.clone(), duplicate],
        InspectionLimits::reviewed(),
    );
    assert!(matches!(
        invalid.inspect(&source),
        Err(InspectError::InvalidSidecarInventory(_))
    ));

    remove_temp_directory(&directory)
}

#[test]
fn adapter_rejects_unsafe_missing_and_already_expired_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("unsafe-input")?;
    let unsafe_adapter = adapter(
        Arc::new(QueueExecutor::default()),
        Vec::new(),
        InspectionLimits::reviewed(),
    );
    assert!(matches!(
        unsafe_adapter.inspect(&directory),
        Err(InspectError::UnsafeInput(path)) if path == directory
    ));
    let missing = directory.join("missing.mkv");
    assert!(matches!(
        unsafe_adapter.inspect(&missing),
        Err(InspectError::InputMetadata { path, .. }) if path == missing
    ));

    let source = write_source(&directory)?;
    let expired = adapter(
        Arc::new(QueueExecutor::default()),
        Vec::new(),
        InspectionLimits {
            deadline: Duration::ZERO,
            ..InspectionLimits::reviewed()
        },
    );
    assert!(matches!(
        expired.inspect(&source),
        Err(InspectError::DeadlineExceeded(Duration::ZERO))
    ));

    remove_temp_directory(&directory)
}

#[test]
fn vobsub_pair_counts_as_one_logical_probe_and_both_files() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = temp_directory("vobsub")?;
    let source = write_source(&directory)?;
    let index = directory.join("movie.en.idx");
    let companion = directory.join("movie.en.sub");
    fs::write(&index, b"idx")?;
    fs::write(&companion, b"data")?;
    let sidecar = SidecarSubtitle {
        path: index.clone(),
        companion_path: Some(companion.clone()),
        language: Some("eng".to_string()),
        role: None,
        format: SidecarFormat::VobSub,
        size_bytes: 7,
    };
    let executor = Arc::new(QueueExecutor::from_stdout([
        SOURCE_JSON.as_bytes().to_vec(),
        VOBSUB_JSON.as_bytes().to_vec(),
    ]));
    let adapter = adapter(
        Arc::clone(&executor),
        vec![sidecar],
        InspectionLimits::reviewed(),
    );

    assert_eq!(adapter.inspect(&source)?.sidecars.len(), 1);
    let requests = executor.requests()?;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].args.last(), Some(&index.into_os_string()));
    assert!(
        requests
            .iter()
            .all(|request| { request.args.last() != Some(&companion.clone().into_os_string()) })
    );

    remove_temp_directory(&directory)
}

#[test]
fn vobsub_pair_rejects_incorrect_physical_byte_accounting() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = temp_directory("vobsub-bytes")?;
    let source = write_source(&directory)?;
    let index = directory.join("movie.en.idx");
    let companion = directory.join("movie.en.sub");
    fs::write(&index, b"idx")?;
    fs::write(&companion, b"data")?;
    let sidecar = SidecarSubtitle {
        path: index,
        companion_path: Some(companion),
        language: Some("eng".to_string()),
        role: None,
        format: SidecarFormat::VobSub,
        size_bytes: 6,
    };
    let executor = Arc::new(QueueExecutor::default());
    let adapter = adapter(
        Arc::clone(&executor),
        vec![sidecar],
        InspectionLimits::reviewed(),
    );

    assert!(matches!(
        adapter.inspect(&source),
        Err(InspectError::InvalidSidecarInventory(_))
    ));
    assert_eq!(executor.request_count()?, 0);

    remove_temp_directory(&directory)
}

#[test]
fn rejects_input_changed_during_probe() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("changed-input")?;
    let source = write_source(&directory)?;
    let adapter = adapter(
        Arc::new(MutatingExecutor {
            path: source.clone(),
        }),
        Vec::new(),
        InspectionLimits::reviewed(),
    );

    assert!(matches!(
        adapter.inspect(&source),
        Err(InspectError::InputChanged(path)) if path == source
    ));

    remove_temp_directory(&directory)
}

#[test]
fn rejects_malformed_and_ambiguous_probe_output() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("malformed")?;
    let source = write_source(&directory)?;
    let malformed = adapter(
        Arc::new(QueueExecutor::from_stdout([b"not json".to_vec()])),
        Vec::new(),
        InspectionLimits::reviewed(),
    );
    assert!(matches!(
        malformed.inspect(&source),
        Err(InspectError::OutputMalformed(_))
    ));

    let duplicate = br#"{"streams":[
      {"index":0,"codec_type":"video","codec_name":"h264"},
      {"index":0,"codec_type":"audio","codec_name":"aac"}],
      "format":{"format_name":"matroska"}}"#;
    let ambiguous = adapter(
        Arc::new(QueueExecutor::from_stdout([duplicate.to_vec()])),
        Vec::new(),
        InspectionLimits::reviewed(),
    );
    assert!(matches!(
        ambiguous.inspect(&source),
        Err(InspectError::OutputMalformed(message)) if message.contains("duplicate stream")
    ));

    remove_temp_directory(&directory)
}

#[test]
fn cancellation_before_discovery_fails_without_process_invocation()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("pre-cancel")?;
    let source = write_source(&directory)?;
    let executor = Arc::new(QueueExecutor::default());
    let adapter = adapter(
        Arc::clone(&executor),
        Vec::new(),
        InspectionLimits::reviewed(),
    );
    let cancellation = InspectCancellationToken::default();
    cancellation.cancel();

    assert!(matches!(
        adapter.inspect_with_cancellation(&source, &cancellation),
        Err(InspectError::Cancelled)
    ));
    assert_eq!(executor.request_count()?, 0);

    remove_temp_directory(&directory)
}

#[cfg(unix)]
#[test]
fn system_executor_terminates_hung_process_at_deadline() {
    let request = shell_request("sleep 5", Duration::from_millis(40), 16, 16);
    let started = Instant::now();

    assert!(matches!(
        SystemInspectProbeExecutor.run(&request, &NeverCancelled),
        Err(InspectError::DeadlineExceeded(_))
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[cfg(unix)]
#[test]
fn system_executor_terminates_process_on_cancellation() -> Result<(), Box<dyn std::error::Error>> {
    let cancellation = Arc::new(InspectCancellationToken::default());
    let request = shell_request("sleep 5", Duration::from_secs(5), 16, 16);
    let canceller = Arc::clone(&cancellation);
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        canceller.cancel();
    });
    let started = Instant::now();

    let result = SystemInspectProbeExecutor.run(&request, cancellation.as_ref());
    thread.join().map_err(|_| "canceller thread failed")?;
    assert!(matches!(result, Err(InspectError::Cancelled)));
    assert!(started.elapsed() < Duration::from_secs(2));
    Ok(())
}

#[cfg(unix)]
#[test]
fn system_executor_accepts_stdout_maximum_and_rejects_maximum_plus_one()
-> Result<(), Box<dyn std::error::Error>> {
    let exact = shell_request("printf 1234", Duration::from_secs(1), 4, 1);
    let output = SystemInspectProbeExecutor.run(&exact, &NeverCancelled)?;
    assert_eq!(output.stdout, b"1234");

    let exceeded = shell_request("printf 12345; sleep 5", Duration::from_secs(1), 4, 1);
    let exceeded_result = SystemInspectProbeExecutor.run(&exceeded, &NeverCancelled);
    assert!(
        matches!(
            exceeded_result,
            Err(InspectError::ProcessOutputLimitExceeded {
                stream: "stdout",
                maximum_bytes: 4
            })
        ),
        "unexpected result: {exceeded_result:?}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn system_executor_maps_spawn_nonzero_and_stderr_limit_failures() {
    let missing = InspectProbeRequest {
        program: OsString::from("/definitely/missing/revaer-ffprobe"),
        args: Vec::new(),
        timeout: Duration::from_secs(1),
        max_stdout_bytes: 1,
        max_stderr_bytes: 1,
    };
    assert!(matches!(
        SystemInspectProbeExecutor.run(&missing, &NeverCancelled),
        Err(InspectError::ProbeFailed(_))
    ));

    let nonzero = shell_request("printf failure >&2; exit 7", Duration::from_secs(1), 1, 16);
    let nonzero_result = SystemInspectProbeExecutor.run(&nonzero, &NeverCancelled);
    assert!(
        nonzero_result
            .as_ref()
            .is_err_and(|error| error.to_string().contains('7')),
        "unexpected result: {nonzero_result:?}"
    );

    let stderr = shell_request("printf 12345 >&2; sleep 5", Duration::from_secs(1), 1, 4);
    assert!(matches!(
        SystemInspectProbeExecutor.run(&stderr, &NeverCancelled),
        Err(InspectError::ProcessOutputLimitExceeded {
            stream: "stderr",
            maximum_bytes: 4
        })
    ));
}

fn adapter<E>(
    executor: Arc<E>,
    sidecars: Vec<SidecarSubtitle>,
    limits: InspectionLimits,
) -> FfprobeInspectAdapter
where
    E: InspectProbeExecutor + 'static,
{
    FfprobeInspectAdapter::with_collaborators(
        executor,
        "ffprobe",
        Arc::new(StaticDiscoverer { sidecars }),
        limits,
    )
}

fn test_limits(max_total_output_bytes: usize) -> InspectionLimits {
    InspectionLimits {
        max_invocations: 65,
        max_sidecars: 64,
        deadline: Duration::from_secs(5),
        max_stdout_bytes: 2048,
        max_stderr_bytes: 128,
        max_total_output_bytes,
    }
}

fn padded_source_json(length: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut output = SOURCE_JSON.as_bytes().to_vec();
    if output.len() > length {
        return Err(format!("source fixture exceeds requested length {length}").into());
    }
    output.resize(length, b' ');
    Ok(output)
}

fn write_source(directory: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = directory.join("movie.mkv");
    fs::write(&path, b"media")?;
    Ok(path)
}

fn write_sidecar(
    directory: &Path,
    name: &str,
    contents: &[u8],
    format: SidecarFormat,
) -> Result<SidecarSubtitle, Box<dyn std::error::Error>> {
    let path = directory.join(name);
    fs::write(&path, contents)?;
    Ok(SidecarSubtitle {
        path,
        companion_path: None,
        language: None,
        role: None,
        format,
        size_bytes: u64::try_from(contents.len())?,
    })
}

#[cfg(unix)]
fn shell_request(
    script: &str,
    timeout: Duration,
    max_stdout_bytes: usize,
    max_stderr_bytes: usize,
) -> InspectProbeRequest {
    InspectProbeRequest {
        program: OsString::from("/bin/sh"),
        args: vec![OsString::from("-c"), OsString::from(script)],
        timeout,
        max_stdout_bytes,
        max_stderr_bytes,
    }
}

fn temp_directory(test_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "revaer-inspection-{test_name}-{}-{counter}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn remove_temp_directory(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::remove_dir_all(path)?;
    Ok(())
}
