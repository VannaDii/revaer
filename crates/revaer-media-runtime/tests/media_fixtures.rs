use std::path::Path;
use std::sync::Arc;

use revaer_media_core::pipeline::{PlanningConstraints, compile_and_plan};
use revaer_media_core::target::{DesiredTarget, UnmatchedStreamPolicy, parse_desired_target_yaml};
use revaer_media_runtime::capabilities::{CapabilitySnapshot, CodecCapability};
use revaer_media_runtime::execute::{
    CommandRunner, ProcessCommandRunner, VideoTranscodePolicy, build_desired_graph_ffmpeg_argv,
};
use revaer_media_runtime::inspect::{
    FfprobeInspectAdapter, InspectAdapter, SystemInspectProbeExecutor,
};

#[test]
fn one_audio_source_executes_to_two_verified_outputs_and_cleans_up()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let fixture_path = fixture.path().to_path_buf();
    let source = fixture.path().join("source.wav");
    let output = fixture.path().join("fanout.mka");
    let runner = ProcessCommandRunner;
    runner.run(
        "ffmpeg",
        &[
            "-hide_banner".to_string(),
            "-loglevel".to_string(),
            "error".to_string(),
            "-f".to_string(),
            "lavfi".to_string(),
            "-i".to_string(),
            "sine=frequency=440:duration=0.5".to_string(),
            "-c:a".to_string(),
            "pcm_s16le".to_string(),
            path_text(&source)?,
        ],
    )?;

    let inspector = FfprobeInspectAdapter::new(Arc::new(SystemInspectProbeExecutor), "ffprobe");
    let source_inspection = inspector.inspect(&source)?;
    let target = fanout_target()?;
    let output_text = path_text(&output)?;
    let outcome = compile_and_plan(
        &source_inspection.graph,
        &output_text,
        &target,
        UnmatchedStreamPolicy::Reject,
        &PlanningConstraints::all_supported(),
    )?;
    assert_eq!(outcome.desired_graph.streams.len(), 2);
    assert!(
        outcome
            .desired_graph
            .stream_bindings
            .iter()
            .all(|binding| binding.source_stream_id == Some(0))
    );

    let argv = build_desired_graph_ffmpeg_argv(
        &path_text(&source)?,
        &output_text,
        &source_inspection.graph,
        &outcome.desired_graph,
        &outcome.selection.selected.operations,
        Some(&fanout_capabilities()),
        VideoTranscodePolicy::default(),
    )?;
    runner.run("ffmpeg", &argv)?;

    let output_inspection = inspector.inspect(&output)?;
    let audio_streams = output_inspection
        .graph
        .streams
        .iter()
        .filter(|stream| stream.kind == revaer_media_core::model::StreamKind::Audio)
        .collect::<Vec<_>>();
    assert_eq!(audio_streams.len(), 2);
    assert_eq!(audio_streams[0].codec, "aac");
    assert_eq!(audio_streams[0].channels, Some(2));
    assert_eq!(audio_streams[1].codec, "eac3");
    assert_eq!(audio_streams[1].channels, Some(6));

    drop(fixture);
    assert!(!fixture_path.exists());
    Ok(())
}

fn fanout_target() -> Result<DesiredTarget, Box<dyn std::error::Error>> {
    let target = parse_desired_target_yaml(
        r#"
target_key: audio-fanout
version: 1
container: matroska
streams:
  - stream_key: stereo-aac
    kind: audio
    role: null
    language: null
    source_binding_key: source-audio
    optional: false
    codec: aac
    channels: 2
    channel_layout: stereo
    audio_bitrate_bps: 96000
    audio_sample_rate_hz: 48000
    audio_loudness_profile: null
    audio_dynamic_range: null
    video_profile: null
    video_level: null
    video_bitrate_bps: null
    color_primaries: null
    color_transfer: null
    color_space: null
    hdr_format: null
    title: Stereo
    dispositions: [default]
    subtitle_placement: null
    image_subtitle_action: null
  - stream_key: surround-eac3
    kind: audio
    role: null
    language: null
    source_binding_key: source-audio
    optional: false
    codec: eac3
    channels: 6
    channel_layout: 5.1
    audio_bitrate_bps: 384000
    audio_sample_rate_hz: 48000
    audio_loudness_profile: null
    audio_dynamic_range: null
    video_profile: null
    video_level: null
    video_bitrate_bps: null
    color_primaries: null
    color_transfer: null
    color_space: null
    hdr_format: null
    title: Surround
    dispositions: []
    subtitle_placement: null
    image_subtitle_action: null
"#,
    )?;
    Ok(target)
}

fn fanout_capabilities() -> CapabilitySnapshot {
    let codecs = vec!["aac".to_string(), "eac3".to_string()];
    CapabilitySnapshot {
        ffmpeg_version: "fixture".to_string(),
        ffprobe_version: "fixture".to_string(),
        codec_support: codecs
            .iter()
            .map(|name| CodecCapability {
                name: name.clone(),
                encode_supported: true,
                decode_supported: true,
            })
            .collect(),
        codecs: codecs.clone(),
        encoders: codecs,
        decoders: vec!["pcm_s16le".to_string()],
        muxers: vec!["matroska".to_string()],
        demuxers: vec!["wav".to_string()],
        ..CapabilitySnapshot::default()
    }
}

fn path_text(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| "fixture path is not UTF-8".into())
}
