use revaer_media_core::classify::SemanticRole;
use revaer_media_core::model::{MediaGraph, MediaStream, StreamKind};
use revaer_media_core::pipeline::{PlanningConstraints, compile_and_plan};
use revaer_media_core::plan::{OperationKind, PlanSelection};
use revaer_media_core::target::{
    DesiredTarget, LanguageToken, TargetStream, UnmatchedStreamPolicy,
};
use revaer_media_core::verify::verify_plan_against_graphs;

fn audio_target(
    stream_key: &str,
    codec: &str,
    channels: u32,
    layout: &str,
    language: &LanguageToken,
) -> TargetStream {
    TargetStream {
        stream_key: stream_key.to_string(),
        kind: StreamKind::Audio,
        role: Some(SemanticRole::Primary),
        language: Some(language.clone()),
        source_binding_key: Some("main-dts".to_string()),
        optional: false,
        codec: codec.to_string(),
        channels: Some(channels),
        channel_layout: Some(layout.to_string()),
        audio_bitrate_bps: None,
        audio_sample_rate_hz: None,
        audio_loudness_profile: None,
        audio_dynamic_range: None,
        video_profile: None,
        video_level: None,
        video_bitrate_bps: None,
        color_primaries: None,
        color_transfer: None,
        color_space: None,
        hdr_format: None,
        title: Some(stream_key.to_string()),
        dispositions: Vec::new(),
        subtitle_placement: None,
        image_subtitle_action: None,
    }
}

fn source_fixture() -> MediaGraph {
    MediaGraph {
        source_path: "/fixtures/fanout-source.mkv".to_string(),
        container_metadata: Vec::new(),
        container_chapters: Vec::new(),
        container_formats: vec!["matroska".to_string()],
        streams: vec![MediaStream {
            stream_id: 41,
            kind: StreamKind::Audio,
            codec: "dts".to_string(),
            channels: Some(6),
            channel_layout: Some("5.1".to_string()),
            language: Some("eng".to_string()),
            title: Some("Main".to_string()),
            dispositions: vec!["default".to_string()],
        }],
    }
}

#[test]
fn one_source_compiles_and_plans_two_independent_outputs() -> Result<(), Box<dyn std::error::Error>>
{
    let language = LanguageToken::parse("eng")?;
    let source = source_fixture();
    let target = DesiredTarget {
        target_key: "fanout-fixture".to_string(),
        version: 1,
        container: "matroska".to_string(),
        container_metadata_policy: "preserve".to_string(),
        container_metadata: Vec::new(),
        container_chapter_policy: "preserve".to_string(),
        container_chapters: Vec::new(),
        container_attachment_policy: "preserve".to_string(),
        streams: vec![
            audio_target("stereo", "aac", 2, "stereo", &language),
            audio_target("surround", "eac3", 6, "5.1", &language),
        ],
    };
    let outcome = compile_and_plan(
        &source,
        "/fixtures/fanout-output.mkv",
        &target,
        UnmatchedStreamPolicy::Reject,
        &PlanningConstraints::all_supported(),
    )?;

    assert_fanout_plan(&outcome.selection);
    verify_plan_against_graphs(
        &source,
        &outcome.desired_graph,
        &outcome.selection.selected.operations,
    )
    .map_err(std::io::Error::other)?;
    Ok(())
}

fn assert_fanout_plan(selection: &PlanSelection) {
    assert_eq!(selection.selected.operations.len(), 2);
    assert!(selection.selected.operations.iter().all(|operation| {
        operation.kind == OperationKind::AudioTranscode && operation.stream_id == Some(41)
    }));
    assert_eq!(
        selection
            .selected
            .operations
            .iter()
            .filter_map(|operation| operation.output_stream_id)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}
