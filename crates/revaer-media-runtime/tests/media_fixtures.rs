use revaer_media_core::model::{
    DesiredGraph, DesiredStreamBinding, MediaGraph, MediaStream, StreamKind,
};
use revaer_media_core::plan::{OperationKind, PlannedOperation};
use revaer_media_runtime::execute::{ProcessCommandRunner, execute_step_sequence};
use revaer_media_runtime::inspect::{
    FfprobeInspectAdapter, InspectAdapter, SystemInspectProbeExecutor,
};
use revaer_media_runtime::jobs::{build_job_execution_steps, plan_job_from_source_graph};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Deserialize)]
struct FixtureManifest {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    fixtures: Vec<FixtureEntry>,
}

#[derive(Debug, Deserialize)]
struct FixtureEntry {
    id: String,
    path: String,
    source: String,
    license: String,
    attribution: String,
    container: String,
    #[serde(rename = "expectedVideoCodecs")]
    expected_video_codecs: Vec<String>,
    #[serde(rename = "expectedAudioCodecs")]
    expected_audio_codecs: Vec<String>,
    #[serde(rename = "expectedSubtitleCodecs")]
    expected_subtitle_codecs: Vec<String>,
    #[serde(rename = "expectedVideoStreamCount")]
    expected_video_stream_count: usize,
    #[serde(rename = "expectedAudioStreamCount")]
    expected_audio_stream_count: usize,
    #[serde(rename = "expectedSubtitleStreamCount")]
    expected_subtitle_stream_count: usize,
    purpose: String,
    generated: bool,
    #[serde(rename = "shouldDownload")]
    should_download: bool,
    #[serde(rename = "shouldGenerate")]
    should_generate: bool,
    notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReportRow {
    scope: String,
    fixture_id: String,
    outcome: String,
    details: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PipelineReportRow {
    case_name: String,
    fixture_id: String,
    input_path: String,
    output_path: String,
    operations: Vec<String>,
    outcome: String,
    details: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaterializedGraph {
    operations: Vec<String>,
    verified_output_path: PathBuf,
}

impl MaterializedGraph {
    fn is_noop(&self) -> bool {
        matches!(self.operations.as_slice(), [operation] if operation == "no_op")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MediaConversionReport {
    expected_fixtures: usize,
    fixture_validations: Vec<ReportRow>,
    metadata_checks: Vec<ReportRow>,
    pipeline_actions: Vec<PipelineReportRow>,
    suite_failures: Vec<String>,
}

impl MediaConversionReport {
    const fn new(expected_fixtures: usize) -> Self {
        Self {
            expected_fixtures,
            fixture_validations: Vec::new(),
            metadata_checks: Vec::new(),
            pipeline_actions: Vec::new(),
            suite_failures: Vec::new(),
        }
    }

    fn record_fixture_validation(&mut self, fixture_id: &str, outcome: &str, details: &str) {
        self.fixture_validations.push(ReportRow {
            scope: "fixture validation".to_string(),
            fixture_id: fixture_id.to_string(),
            outcome: outcome.to_string(),
            details: details.to_string(),
        });
    }

    fn record_metadata_check(&mut self, fixture_id: &str, outcome: &str, details: &str) {
        self.metadata_checks.push(ReportRow {
            scope: "metadata validation".to_string(),
            fixture_id: fixture_id.to_string(),
            outcome: outcome.to_string(),
            details: details.to_string(),
        });
    }

    fn record_pipeline_action(&mut self, row: PipelineReportRow) {
        self.pipeline_actions.push(row);
    }

    fn record_suite_failure(&mut self, detail: &str) {
        self.suite_failures.push(detail.to_string());
    }

    fn render_markdown(&self) -> String {
        let failed_fixtures = self
            .fixture_validations
            .iter()
            .filter(|row| row.outcome != "passed")
            .count();
        let failed_metadata = self
            .metadata_checks
            .iter()
            .filter(|row| row.outcome != "passed")
            .count();
        let failed_pipeline = self
            .pipeline_actions
            .iter()
            .filter(|row| row.outcome != "passed")
            .count();
        let video_transcodes = self.operation_count("video_transcode");
        let audio_transcodes = self.operation_count("audio_transcode");
        let suite_outcome = if self.suite_failures.is_empty()
            && failed_fixtures == 0
            && failed_metadata == 0
            && failed_pipeline == 0
        {
            "passed"
        } else {
            "failed"
        };

        let mut markdown = String::new();
        markdown.push_str("# Media Conversion Fixture Report\n\n");
        markdown.push_str("## Summary\n");
        markdown.push_str(&format!("- Outcome: {suite_outcome}\n"));
        markdown.push_str(&format!(
            "- Fixtures expected: {}\n",
            self.expected_fixtures
        ));
        markdown.push_str(&format!(
            "- Fixtures verified: {}\n",
            self.fixture_validations
                .iter()
                .filter(|row| row.outcome == "passed")
                .count()
        ));
        markdown.push_str(&format!(
            "- Metadata checks: {} passed, {} failed\n",
            self.metadata_checks
                .iter()
                .filter(|row| row.outcome == "passed")
                .count(),
            failed_metadata
        ));
        markdown.push_str(&format!(
            "- Pipeline actions: {}\n",
            self.pipeline_actions.len()
        ));
        markdown.push_str(&format!("- Video transcodes: {video_transcodes}\n"));
        markdown.push_str(&format!("- Audio transcodes: {audio_transcodes}\n"));
        markdown.push_str(&format!("- Pipeline failures: {failed_pipeline}\n"));
        markdown.push_str(&format!(
            "- Suite failures: {}\n\n",
            self.suite_failures.len()
        ));

        if !self.suite_failures.is_empty() {
            markdown.push_str("## Suite Failures\n");
            for failure in &self.suite_failures {
                markdown.push_str(&format!("- {}\n", markdown_inline(failure)));
            }
            markdown.push('\n');
        }

        markdown.push_str("## Fixture Validation\n");
        markdown.push_str("| Scope | Fixture | Outcome | Details |\n");
        markdown.push_str("| --- | --- | --- | --- |\n");
        for row in &self.fixture_validations {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                markdown_cell(&row.scope),
                markdown_cell(&row.fixture_id),
                markdown_cell(&row.outcome),
                markdown_cell(&row.details)
            ));
        }
        markdown.push('\n');

        markdown.push_str("## Metadata Checks\n");
        markdown.push_str("| Scope | Fixture | Outcome | Details |\n");
        markdown.push_str("| --- | --- | --- | --- |\n");
        for row in &self.metadata_checks {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                markdown_cell(&row.scope),
                markdown_cell(&row.fixture_id),
                markdown_cell(&row.outcome),
                markdown_cell(&row.details)
            ));
        }
        markdown.push('\n');

        markdown.push_str("## Pipeline Actions\n");
        markdown.push_str("| Case | Fixture | Input | Output | Operations | Outcome | Details |\n");
        markdown.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
        for row in &self.pipeline_actions {
            let operations = if row.operations.is_empty() {
                "none".to_string()
            } else {
                row.operations.join(",")
            };
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                markdown_cell(&row.case_name),
                markdown_cell(&row.fixture_id),
                markdown_cell(&row.input_path),
                markdown_cell(&row.output_path),
                markdown_cell(&operations),
                markdown_cell(&row.outcome),
                markdown_cell(&row.details)
            ));
        }
        markdown
    }

    fn operation_count(&self, operation: &str) -> usize {
        self.pipeline_actions
            .iter()
            .filter(|row| row.operations.iter().any(|item| item == operation))
            .count()
    }
}

#[test]
fn manifest_contract_tracks_fixture_suite() -> TestResult {
    let root = repo_root()?;
    let manifest = load_manifest(&root)?;

    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.fixtures.len(), 30);
    assert_required_fixture_ids(&manifest)?;
    assert_fixture_paths_are_unique(&manifest)?;
    assert_committed_fixture_files_exist(&root)?;
    assert_binary_fixture_dirs_are_gitignored(&root)?;

    for fixture in &manifest.fixtures {
        assert_nonempty_field(fixture, "source", &fixture.source)?;
        assert_nonempty_field(fixture, "license", &fixture.license)?;
        assert_nonempty_field(fixture, "attribution", &fixture.attribution)?;
        assert_nonempty_field(fixture, "container", &fixture.container)?;
        assert_nonempty_field(fixture, "purpose", &fixture.purpose)?;
        assert_nonempty_field(fixture, "notes", &fixture.notes)?;
        if fixture.generated {
            assert!(
                fixture.should_generate && !fixture.should_download,
                "{} generated fixture must generate and not download",
                fixture.id
            );
        } else {
            assert!(
                fixture.should_download && !fixture.should_generate,
                "{} upstream fixture must download and not generate",
                fixture.id
            );
        }
    }

    Ok(())
}

#[test]
fn media_conversion_report_starts_with_summary_and_lists_actions() -> TestResult {
    let mut report = MediaConversionReport::new(2);
    report.record_fixture_validation("bbb-h264-mp4", "passed", "MP4/H.264 graph matched");
    report.record_fixture_validation("multi-audio-mkv", "passed", "MKV multi-audio graph matched");
    report.record_pipeline_action(PipelineReportRow {
        case_name: "common MP4 input".to_string(),
        fixture_id: "bbb-h264-mp4".to_string(),
        input_path: "test-fixtures/source/bbb-h264.mp4".to_string(),
        output_path: "target/media-fixture-integration/bbb-h264.mp4".to_string(),
        operations: vec!["remux".to_string()],
        outcome: "passed".to_string(),
        details: "output probeable with expected streams".to_string(),
    });
    report.record_pipeline_action(PipelineReportRow {
        case_name: "multi-audio ordered selection".to_string(),
        fixture_id: "multi-audio-mkv".to_string(),
        input_path: "test-fixtures/derived/multi-audio.mkv".to_string(),
        output_path: "target/media-fixture-integration/multi-audio-ordered.mkv".to_string(),
        operations: vec!["remux".to_string(), "stream_reorder".to_string()],
        outcome: "passed".to_string(),
        details: "preserved requested audio order spa,eng".to_string(),
    });
    report.record_pipeline_action(PipelineReportRow {
        case_name: "WebM VP8/Vorbis to MP4 H.264/AAC".to_string(),
        fixture_id: "chromium-bear-320x240-webm".to_string(),
        input_path: "test-fixtures/chromium/bear-320x240.webm".to_string(),
        output_path: "target/media-fixture-integration/webm-vp8-vorbis-to-h264-aac.mp4".to_string(),
        operations: vec!["video_transcode".to_string(), "audio_transcode".to_string()],
        outcome: "passed".to_string(),
        details: "output codecs video=[h264] audio=[aac]".to_string(),
    });

    let markdown = report.render_markdown();

    assert!(markdown.starts_with("# Media Conversion Fixture Report\n\n## Summary\n"));
    assert!(markdown.contains("- Fixtures verified: 2"));
    assert!(markdown.contains("- Pipeline actions: 3"));
    assert!(markdown.contains("- Video transcodes: 1"));
    assert!(markdown.contains("- Audio transcodes: 1"));
    assert!(markdown.contains("| common MP4 input | bbb-h264-mp4 |"));
    assert!(markdown.contains("| multi-audio ordered selection | multi-audio-mkv |"));
    assert!(markdown.contains("| WebM VP8/Vorbis to MP4 H.264/AAC |"));
    assert!(markdown.contains("## Pipeline Actions\n"));
    Ok(())
}

#[test]
fn noop_materialization_verifies_source_path_without_requesting_output_artifact() {
    let desired = DesiredGraph {
        output_path: "target/media-fixture-integration/bbb-h264.mp4".to_string(),
        container_chapters: Vec::new(),
        container_format: None,
        container_metadata_policy: None,
        container_chapter_policy: None,
        stream_bindings: Vec::new(),
        streams: Vec::new(),
    };
    let source_path = Path::new("test-fixtures/source/bbb-h264.mp4");

    let verified_path = verified_output_path_for_plan(
        source_path,
        &desired,
        &[PlannedOperation {
            kind: OperationKind::NoOp,
            stream_id: None,
            output_stream_id: None,
        }],
    );

    assert_eq!(verified_path, source_path);
}

#[test]
fn mutating_materialization_verifies_requested_output_artifact() {
    let desired = DesiredGraph {
        output_path: "target/media-fixture-integration/bbb-h264.mp4".to_string(),
        container_chapters: Vec::new(),
        container_format: None,
        container_metadata_policy: None,
        container_chapter_policy: None,
        stream_bindings: Vec::new(),
        streams: Vec::new(),
    };

    let verified_path = verified_output_path_for_plan(
        Path::new("test-fixtures/source/bbb-h264.mp4"),
        &desired,
        &[PlannedOperation {
            kind: OperationKind::Remux,
            stream_id: None,
            output_stream_id: None,
        }],
    );

    assert_eq!(verified_path, PathBuf::from(&desired.output_path));
}

#[test]
#[ignore = "requires downloaded and generated media fixtures"]
fn verify_prepared_fixture_suite() -> TestResult {
    require_tool("ffmpeg")?;
    require_tool("ffprobe")?;
    require_tool("curl")?;
    require_tool("git")?;
    require_tool("base64")?;

    let root = repo_root()?;
    let manifest = load_manifest(&root)?;
    let mut report = MediaConversionReport::new(manifest.fixtures.len());
    let result = verify_prepared_fixture_suite_with_report(&root, &manifest, &mut report);
    if let Err(error) = &result {
        report.record_suite_failure(&error.to_string());
    }
    write_media_conversion_report(&root, &report)?;
    result?;
    println!(
        "verify_prepared_fixture_suite: verified {} fixtures; report written to {}",
        manifest.fixtures.len(),
        media_conversion_report_path(&root)?.display()
    );
    Ok(())
}

fn verify_prepared_fixture_suite_with_report(
    root: &Path,
    manifest: &FixtureManifest,
    report: &mut MediaConversionReport,
) -> TestResult {
    let mut probes = BTreeMap::new();

    for fixture in &manifest.fixtures {
        let fixture_path = root.join(&fixture.path);
        if !fixture_path.is_file() {
            return fail(format!(
                "{} missing fixture file: {}",
                fixture.id, fixture.path
            ));
        }
        let probe = ffprobe_json(&fixture_path)?;
        validate_fixture_probe(fixture, &probe)?;
        write_probe_snapshot_if_requested(root, fixture, &probe)?;
        report.record_fixture_validation(
            &fixture.id,
            "passed",
            &fixture_report_details(fixture, &probe)?,
        );
        probes.insert(fixture.id.as_str(), probe);
    }

    validate_derived_metadata(root, &probes, report)?;
    run_pipeline_cases(root, manifest, report)?;
    Ok(())
}

fn repo_root() -> TestResult<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Some(root) = manifest_dir.parent().and_then(Path::parent) else {
        return fail("unable to resolve repository root");
    };
    Ok(root.to_path_buf())
}

fn load_manifest(root: &Path) -> TestResult<FixtureManifest> {
    let path = root.join("test-fixtures/manifest.json");
    let content = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

fn assert_required_fixture_ids(manifest: &FixtureManifest) -> TestResult {
    let actual = manifest
        .fixtures
        .iter()
        .map(|fixture| fixture.id.as_str())
        .collect::<BTreeSet<_>>();
    let required = [
        "bbb-h264-mp4",
        "bbb-h265-mp4",
        "bbb-av1-mp4",
        "bbb-vp8-webm",
        "bbb-vp9-webm",
        "bbb-h264-mkv",
        "mkv-basic-divx-mp3",
        "mkv-h264-aac-weird-timecode",
        "mkv-h264-mp3-header-stripping",
        "mkv-theora-vorbis-live-style",
        "mkv-multi-audio-multi-subtitles",
        "mkv-audio-gap",
        "chromium-bear-320x240-webm",
        "chromium-bear-vp9-opus-webm",
        "chromium-bear-vp8-webvtt-webm",
        "chromium-bear-1280x720-av-frag-mp4",
        "chromium-bear-320x180-hi10p-mp4",
        "chromium-bear-vp9-profile2-webm",
        "chromium-bear-v-frag-hevc-mp4",
        "chromium-bear-1280x720-aac-he-ts",
        "chromium-bbb-2video-2audio-mp4",
        "chromium-multitrack-3video-2audio-webm",
        "multi-audio-mkv",
        "subtitles-mkv",
        "video-only-mp4",
        "audio-only-m4a",
        "silent-audio-mp4",
        "h264-aac-ts",
        "h264-aac-mov",
        "mpeg4-mp3-avi",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();

    if actual != required {
        return fail(format!(
            "manifest fixture ids mismatch: expected {required:?}, got {actual:?}"
        ));
    }
    Ok(())
}

fn assert_fixture_paths_are_unique(manifest: &FixtureManifest) -> TestResult {
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for fixture in &manifest.fixtures {
        if !ids.insert(fixture.id.as_str()) {
            return fail(format!("duplicate fixture id: {}", fixture.id));
        }
        if !paths.insert(fixture.path.as_str()) {
            return fail(format!("duplicate fixture path: {}", fixture.path));
        }
    }
    Ok(())
}

fn assert_committed_fixture_files_exist(root: &Path) -> TestResult {
    for path in [
        "test-fixtures/README.md",
        "test-fixtures/ATTRIBUTION.md",
        "test-fixtures/manifest.json",
        "scripts/test-fixtures/download-test-fixtures.sh",
        "scripts/test-fixtures/generate-derived-fixtures.sh",
        "scripts/test-fixtures/verify-fixtures.sh",
        "scripts/test-fixtures/clean-test-fixtures.sh",
    ] {
        let absolute = root.join(path);
        if !absolute.is_file() {
            return fail(format!("missing committed fixture file: {path}"));
        }
    }
    Ok(())
}

fn assert_binary_fixture_dirs_are_gitignored(root: &Path) -> TestResult {
    let gitignore = fs::read_to_string(root.join(".gitignore"))?;
    for path in [
        "/test-fixtures/source/",
        "/test-fixtures/matroska/",
        "/test-fixtures/chromium/",
        "/test-fixtures/derived/",
    ] {
        assert!(
            gitignore.lines().any(|line| line.trim() == path),
            "missing .gitignore entry: {path}"
        );
    }
    Ok(())
}

fn assert_nonempty_field(fixture: &FixtureEntry, field: &str, value: &str) -> TestResult {
    if value.trim().is_empty() {
        return fail(format!("{} has empty manifest field {field}", fixture.id));
    }
    Ok(())
}

fn require_tool(tool: &str) -> TestResult {
    let args = match tool {
        "ffmpeg" | "ffprobe" => vec!["-version"],
        "base64" => Vec::new(),
        "curl" | "git" => vec!["--version"],
        _ => vec!["--version"],
    };
    let status = Command::new(tool)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(value) if value.success() => {
            if tool == "ffmpeg" {
                require_ffmpeg_channel_layout_support(tool)?;
            }
            Ok(())
        }
        Ok(value) => fail(format!(
            "required tool preflight failed: {tool} status={value}"
        )),
        Err(error) => fail(format!("required tool missing: {tool}: {error}")),
    }
}

fn require_ffmpeg_channel_layout_support(tool: &str) -> TestResult {
    let output = Command::new(tool)
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-nostdin",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=channel_layout=stereo:sample_rate=48000",
            "-t",
            "0.01",
            "-map",
            "0:a:0",
            "-c:a",
            "pcm_s16le",
            "-channel_layout:a:0",
            "stereo",
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    fail(format!(
        "required FFmpeg channel-layout capability is unavailable: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn ffprobe_json(path: &Path) -> TestResult<Value> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(path)
        .output()?;
    if !output.status.success() {
        return fail(format!(
            "ffprobe failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn validate_fixture_probe(fixture: &FixtureEntry, probe: &Value) -> TestResult {
    assert_stream_count(fixture, probe, "video", fixture.expected_video_stream_count)?;
    assert_stream_count(fixture, probe, "audio", fixture.expected_audio_stream_count)?;
    assert_stream_count(
        fixture,
        probe,
        "subtitle",
        fixture.expected_subtitle_stream_count,
    )?;
    assert_codecs(fixture, probe, "video", &fixture.expected_video_codecs)?;
    assert_codecs(fixture, probe, "audio", &fixture.expected_audio_codecs)?;
    assert_codecs(
        fixture,
        probe,
        "subtitle",
        &fixture.expected_subtitle_codecs,
    )?;
    Ok(())
}

fn assert_stream_count(
    fixture: &FixtureEntry,
    probe: &Value,
    kind: &str,
    expected: usize,
) -> TestResult {
    let actual = streams_of_kind(probe, kind)?.len();
    if actual != expected {
        return fail(format!(
            "{} {kind} stream count mismatch: expected {expected}, got {actual}",
            fixture.id
        ));
    }
    Ok(())
}

fn assert_codecs(
    fixture: &FixtureEntry,
    probe: &Value,
    kind: &str,
    expected: &[String],
) -> TestResult {
    let actual = streams_of_kind(probe, kind)?
        .iter()
        .filter_map(|stream| stream.get("codec_name").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let expected_set = expected.iter().cloned().collect::<BTreeSet<_>>();
    if actual != expected_set {
        return fail(format!(
            "{} {kind} codec mismatch: expected {expected_set:?}, got {actual:?}",
            fixture.id
        ));
    }
    Ok(())
}

fn streams_of_kind<'a>(probe: &'a Value, kind: &str) -> TestResult<Vec<&'a Value>> {
    Ok(streams(probe)?
        .iter()
        .filter(|stream| stream.get("codec_type").and_then(Value::as_str) == Some(kind))
        .collect())
}

fn streams(probe: &Value) -> TestResult<&Vec<Value>> {
    let Some(items) = probe.get("streams").and_then(Value::as_array) else {
        return fail("ffprobe output missing streams array");
    };
    Ok(items)
}

fn write_probe_snapshot_if_requested(
    root: &Path,
    fixture: &FixtureEntry,
    probe: &Value,
) -> TestResult {
    if std::env::var("REVAER_MEDIA_FIXTURE_WRITE_PROBES").as_deref() != Ok("1") {
        return Ok(());
    }
    let snapshot = normalize_probe_snapshot(probe)?;
    let path = root
        .join("test-fixtures/probe")
        .join(format!("{}.json", fixture.id));
    fs::create_dir_all(
        path.parent()
            .ok_or_else(|| std::io::Error::other("probe snapshot parent missing"))?,
    )?;
    fs::write(path, serde_json::to_string_pretty(&snapshot)? + "\n")?;
    Ok(())
}

fn normalize_probe_snapshot(probe: &Value) -> TestResult<Value> {
    let normalized_streams = streams(probe)?
        .iter()
        .map(|stream| {
            json!({
                "index": stream.get("index").cloned().unwrap_or(Value::Null),
                "codec_name": stream.get("codec_name").cloned().unwrap_or(Value::Null),
                "codec_type": stream.get("codec_type").cloned().unwrap_or(Value::Null),
                "disposition": {
                    "default": stream.pointer("/disposition/default").cloned().unwrap_or(Value::Null),
                    "forced": stream.pointer("/disposition/forced").cloned().unwrap_or(Value::Null)
                },
                "tags": {
                    "language": stream.pointer("/tags/language").cloned().unwrap_or(Value::Null),
                    "title": stream.pointer("/tags/title").cloned().unwrap_or(Value::Null)
                }
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "streams": normalized_streams }))
}

fn fixture_report_details(fixture: &FixtureEntry, probe: &Value) -> TestResult<String> {
    let video_codecs = codec_summary(probe, "video")?;
    let audio_codecs = codec_summary(probe, "audio")?;
    let subtitle_codecs = codec_summary(probe, "subtitle")?;
    Ok(format!(
        "container={} streams video={} audio={} subtitle={} codecs video=[{}] audio=[{}] subtitle=[{}]",
        fixture.container,
        streams_of_kind(probe, "video")?.len(),
        streams_of_kind(probe, "audio")?.len(),
        streams_of_kind(probe, "subtitle")?.len(),
        video_codecs,
        audio_codecs,
        subtitle_codecs
    ))
}

fn codec_summary(probe: &Value, kind: &str) -> TestResult<String> {
    let codecs = streams_of_kind(probe, kind)?
        .iter()
        .filter_map(|stream| stream.get("codec_name").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if codecs.is_empty() {
        return Ok("none".to_string());
    }
    Ok(codecs.into_iter().collect::<Vec<_>>().join(","))
}

fn validate_derived_metadata(
    root: &Path,
    probes: &BTreeMap<&str, Value>,
    report: &mut MediaConversionReport,
) -> TestResult {
    let multi_audio = probe_by_id(probes, "multi-audio-mkv")?;
    let audio_streams = streams_of_kind(multi_audio, "audio")?;
    assert_stream_tags(
        "multi-audio-mkv",
        &audio_streams,
        &[
            ("eng", "English AAC Tone"),
            ("jpn", "Japanese Opus Tone"),
            ("spa", "Spanish AC3 Tone"),
            ("und", "Undeclared Silent AAC"),
        ],
    )?;
    report.record_metadata_check(
        "multi-audio-mkv",
        "passed",
        "audio language/title metadata matched eng,jpn,spa,und stream order",
    );

    let subtitles = probe_by_id(probes, "subtitles-mkv")?;
    let subtitle_streams = streams_of_kind(subtitles, "subtitle")?;
    assert_stream_tags(
        "subtitles-mkv",
        &subtitle_streams,
        &[
            ("eng", "English Full Subtitles"),
            ("eng", "English Forced Subtitles"),
        ],
    )?;
    let forced = subtitle_streams
        .get(1)
        .and_then(|stream| stream.pointer("/disposition/forced"))
        .and_then(Value::as_i64);
    if forced != Some(1) {
        return fail(format!(
            "subtitles-mkv forced subtitle disposition mismatch: expected 1, got {forced:?}"
        ));
    }
    report.record_metadata_check(
        "subtitles-mkv",
        "passed",
        "subtitle language/title metadata matched and second subtitle is forced",
    );

    let silent = probe_by_id(probes, "silent-audio-mp4")?;
    if streams_of_kind(silent, "audio")?.is_empty() {
        return fail("silent-audio-mp4 expected an audio stream");
    }
    validate_silence_with_ffmpeg(&root.join("test-fixtures/derived/silent-audio.mp4"))?;
    report.record_metadata_check(
        "silent-audio-mp4",
        "passed",
        "audio stream exists and ffmpeg silencedetect found silence",
    );
    Ok(())
}

fn probe_by_id<'a>(probes: &'a BTreeMap<&str, Value>, id: &str) -> TestResult<&'a Value> {
    probes
        .get(id)
        .ok_or_else(|| std::io::Error::other(format!("missing probe for {id}")).into())
}

fn assert_stream_tags(id: &str, streams: &[&Value], expected: &[(&str, &str)]) -> TestResult {
    if streams.len() != expected.len() {
        return fail(format!(
            "{id} metadata stream count mismatch: expected {}, got {}",
            expected.len(),
            streams.len()
        ));
    }
    for (stream, (language, title)) in streams.iter().zip(expected) {
        let actual_language = stream.pointer("/tags/language").and_then(Value::as_str);
        let actual_title = stream.pointer("/tags/title").and_then(Value::as_str);
        let language_matches =
            actual_language == Some(*language) || (*language == "und" && actual_language.is_none());
        if !language_matches || actual_title != Some(*title) {
            return fail(format!(
                "{id} metadata mismatch: expected language={language} title={title}, got language={actual_language:?} title={actual_title:?}"
            ));
        }
    }
    Ok(())
}

fn validate_silence_with_ffmpeg(path: &Path) -> TestResult {
    let path_display = path.display();
    let output = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-nostats")
        .arg("-i")
        .arg(path)
        .arg("-af")
        .arg("silencedetect=noise=-50dB:d=0.5")
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()?;
    if !output.status.success() {
        return fail(format!(
            "silencedetect failed for {path_display}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.contains("silence_start") {
        return fail(format!(
            "silent-audio-mp4 silence not detected in {path_display}"
        ));
    }
    Ok(())
}

fn run_pipeline_cases(
    root: &Path,
    manifest: &FixtureManifest,
    report: &mut MediaConversionReport,
) -> TestResult {
    let output_root = root.join("target/media-fixture-integration");
    if output_root.exists() {
        fs::remove_dir_all(&output_root)?;
    }
    fs::create_dir_all(&output_root)?;

    for (id, case_name) in [
        ("bbb-h264-mp4", "common MP4 input"),
        ("bbb-h265-mp4", "HEVC MP4 input"),
        ("bbb-av1-mp4", "AV1 MP4 input"),
        ("bbb-vp8-webm", "WebM VP8 input"),
        ("bbb-vp9-webm", "WebM VP9 input"),
        ("bbb-h264-mkv", "MKV H.264 input"),
        ("chromium-bear-1280x720-av-frag-mp4", "fragmented MP4 input"),
        (
            "chromium-bbb-2video-2audio-mp4",
            "multi-track MP4 deterministic selection",
        ),
        (
            "chromium-multitrack-3video-2audio-webm",
            "multi-track WebM deterministic selection",
        ),
        ("h264-aac-ts", "MPEG-TS H.264/AAC input"),
        ("h264-aac-mov", "MOV H.264/AAC input"),
        ("mpeg4-mp3-avi", "AVI MPEG-4/MP3 input"),
        ("video-only-mp4", "video-only explicit behavior"),
        ("audio-only-m4a", "audio-only explicit behavior"),
    ] {
        let fixture = fixture_by_id(manifest, id)?;
        materialize_same_graph(root, fixture, &output_root, case_name, report)?;
    }

    assert_multi_audio_selection(root, manifest, &output_root, report)?;
    assert_subtitle_selection(root, manifest, &output_root, report)?;
    assert_transcoding_cases(root, manifest, &output_root, report)?;
    assert_pipeline_report_has_operation(report, "video_transcode")?;
    assert_pipeline_report_has_operation(report, "audio_transcode")?;
    Ok(())
}

fn fixture_by_id<'a>(manifest: &'a FixtureManifest, id: &str) -> TestResult<&'a FixtureEntry> {
    manifest
        .fixtures
        .iter()
        .find(|fixture| fixture.id == id)
        .ok_or_else(|| std::io::Error::other(format!("missing fixture {id}")).into())
}

fn materialize_same_graph(
    root: &Path,
    fixture: &FixtureEntry,
    output_root: &Path,
    case_name: &str,
    report: &mut MediaConversionReport,
) -> TestResult {
    let source_path = root.join(&fixture.path);
    let graph = inspect_graph(&source_path)?;
    let output_path = output_root.join(
        Path::new(&fixture.path)
            .file_name()
            .ok_or_else(|| std::io::Error::other("fixture output filename missing"))?,
    );
    let desired = desired_graph(path_text(&output_path)?, graph.streams.clone())?;
    let materialized = materialize_desired_graph(&source_path, &graph, &desired)?;
    let output_graph = inspect_graph(&materialized.verified_output_path)?;
    assert_eq!(
        output_graph.streams.len(),
        desired.streams.len(),
        "{} output stream count mismatch",
        fixture.id
    );
    let details = if materialized.is_noop() {
        "source already satisfied desired graph; no output materialized".to_string()
    } else {
        format!(
            "output probeable with {} stream(s)",
            output_graph.streams.len()
        )
    };
    report.record_pipeline_action(PipelineReportRow {
        case_name: case_name.to_string(),
        fixture_id: fixture.id.clone(),
        input_path: fixture.path.clone(),
        output_path: report_path(root, &materialized.verified_output_path),
        operations: materialized.operations,
        outcome: "passed".to_string(),
        details,
    });
    Ok(())
}

fn assert_transcoding_cases(
    root: &Path,
    manifest: &FixtureManifest,
    output_root: &Path,
    report: &mut MediaConversionReport,
) -> TestResult {
    let cases = [
        video_transcode_case(
            "HEVC MP4 video transcode to H.264",
            "bbb-h265-mp4",
            "hevc-to-h264.mp4",
            "h264",
            H264_CODEC,
        ),
        video_transcode_case(
            "AV1 MP4 video transcode to H.264",
            "bbb-av1-mp4",
            "av1-to-h264.mp4",
            "h264",
            H264_CODEC,
        ),
        video_transcode_case(
            "VP8 WebM video transcode to H.264",
            "bbb-vp8-webm",
            "vp8-to-h264.mp4",
            "h264",
            H264_CODEC,
        ),
        video_transcode_case(
            "VP9 WebM video transcode to H.264",
            "bbb-vp9-webm",
            "vp9-to-h264.mp4",
            "h264",
            H264_CODEC,
        ),
        video_transcode_case(
            "H.264 MP4 video transcode to MPEG-4 Part 2",
            "bbb-h264-mp4",
            "h264-to-mpeg4.avi",
            "mpeg4",
            MPEG4_CODEC,
        ),
        audio_transcode_case(
            "MOV AAC audio transcode to Opus",
            "h264-aac-mov",
            "aac-to-opus.mkv",
            OPUS_CODEC,
            H264_CODEC,
        ),
        audio_transcode_case(
            "Audio-only AAC transcode to MP3",
            "audio-only-m4a",
            "audio-only-aac-to-mp3.mp3",
            MP3_CODEC,
            NO_CODECS,
        ),
        audio_transcode_case(
            "VP9/Opus WebM audio transcode to AAC",
            "chromium-bear-vp9-opus-webm",
            "vp9-opus-to-vp9-aac.mkv",
            AAC_CODEC,
            VP9_CODEC,
        ),
        audio_video_transcode_case(
            "WebM VP8/Vorbis to MP4 H.264/AAC",
            "chromium-bear-320x240-webm",
            "webm-vp8-vorbis-to-h264-aac.mp4",
        ),
        audio_video_transcode_case(
            "AVI MPEG-4/MP3 to MP4 H.264/AAC",
            "mpeg4-mp3-avi",
            "avi-mpeg4-mp3-to-h264-aac.mp4",
        ),
        audio_video_transcode_case(
            "Matroska Theora/Vorbis to MP4 H.264/AAC",
            "mkv-theora-vorbis-live-style",
            "theora-vorbis-to-h264-aac.mp4",
        ),
    ];

    for item in cases {
        assert_transcode_case(root, manifest, output_root, report, item)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct TranscodeCase<'a> {
    case_name: &'a str,
    fixture_id: &'a str,
    output_name: &'a str,
    video_codec: Option<&'a str>,
    audio_codecs: &'a [&'a str],
    expected_video_codecs: &'a [&'a str],
    expected_audio_codecs: &'a [&'a str],
    expected_operations: &'a [&'a str],
}

const NO_CODECS: &[&str] = &[];
const AAC_CODEC: &[&str] = &["aac"];
const H264_CODEC: &[&str] = &["h264"];
const MPEG4_CODEC: &[&str] = &["mpeg4"];
const MP3_CODEC: &[&str] = &["mp3"];
const OPUS_CODEC: &[&str] = &["opus"];
const VP9_CODEC: &[&str] = &["vp9"];
const AUDIO_TRANSCODE_OPERATION: &[&str] = &["audio_transcode"];
const VIDEO_TRANSCODE_OPERATION: &[&str] = &["video_transcode"];
const AUDIO_VIDEO_TRANSCODE_OPERATIONS: &[&str] = &["video_transcode", "audio_transcode"];

const fn video_transcode_case(
    case_name: &'static str,
    fixture_id: &'static str,
    output_name: &'static str,
    video_codec: &'static str,
    expected_video_codecs: &'static [&'static str],
) -> TranscodeCase<'static> {
    TranscodeCase {
        case_name,
        fixture_id,
        output_name,
        video_codec: Some(video_codec),
        audio_codecs: NO_CODECS,
        expected_video_codecs,
        expected_audio_codecs: NO_CODECS,
        expected_operations: VIDEO_TRANSCODE_OPERATION,
    }
}

const fn audio_transcode_case(
    case_name: &'static str,
    fixture_id: &'static str,
    output_name: &'static str,
    audio_codecs: &'static [&'static str],
    expected_video_codecs: &'static [&'static str],
) -> TranscodeCase<'static> {
    TranscodeCase {
        case_name,
        fixture_id,
        output_name,
        video_codec: None,
        audio_codecs,
        expected_video_codecs,
        expected_audio_codecs: audio_codecs,
        expected_operations: AUDIO_TRANSCODE_OPERATION,
    }
}

const fn audio_video_transcode_case(
    case_name: &'static str,
    fixture_id: &'static str,
    output_name: &'static str,
) -> TranscodeCase<'static> {
    TranscodeCase {
        case_name,
        fixture_id,
        output_name,
        video_codec: Some("h264"),
        audio_codecs: AAC_CODEC,
        expected_video_codecs: H264_CODEC,
        expected_audio_codecs: AAC_CODEC,
        expected_operations: AUDIO_VIDEO_TRANSCODE_OPERATIONS,
    }
}

fn assert_transcode_case(
    root: &Path,
    manifest: &FixtureManifest,
    output_root: &Path,
    report: &mut MediaConversionReport,
    item: TranscodeCase<'_>,
) -> TestResult {
    let fixture = fixture_by_id(manifest, item.fixture_id)?;
    let source_path = root.join(&fixture.path);
    let source = inspect_graph(&source_path)?;
    let output_path = output_root.join(item.output_name);
    let desired_streams = desired_transcode_streams(
        &source,
        item.fixture_id,
        item.video_codec,
        item.audio_codecs,
    )?;
    let desired = desired_graph(path_text(&output_path)?, desired_streams)?;

    let materialized = materialize_desired_graph(&source_path, &source, &desired)?;
    assert_operations(
        item.case_name,
        &materialized.operations,
        item.expected_operations,
    )?;

    let output = inspect_graph(&materialized.verified_output_path)?;
    assert_stream_codecs(
        item.case_name,
        &output,
        StreamKind::Video,
        item.expected_video_codecs,
    )?;
    assert_stream_codecs(
        item.case_name,
        &output,
        StreamKind::Audio,
        item.expected_audio_codecs,
    )?;

    report.record_pipeline_action(PipelineReportRow {
        case_name: item.case_name.to_string(),
        fixture_id: fixture.id.clone(),
        input_path: fixture.path.clone(),
        output_path: report_path(root, &materialized.verified_output_path),
        operations: materialized.operations,
        outcome: "passed".to_string(),
        details: format!(
            "output codecs video=[{}] audio=[{}]",
            item.expected_video_codecs.join(","),
            item.expected_audio_codecs.join(",")
        ),
    });
    Ok(())
}

fn desired_transcode_streams(
    source: &MediaGraph,
    fixture_id: &str,
    video_codec: Option<&str>,
    audio_codecs: &[&str],
) -> TestResult<Vec<MediaStream>> {
    let mut audio_index = 0_usize;
    let mut desired = Vec::new();
    for stream in &source.streams {
        if stream.kind != StreamKind::Video && stream.kind != StreamKind::Audio {
            continue;
        }
        let mut target = stream.clone();
        match stream.kind {
            StreamKind::Video => {
                if let Some(codec) = video_codec {
                    target.codec = codec.to_string();
                }
            }
            StreamKind::Audio => {
                if let Some(codec) = audio_codecs.get(audio_index) {
                    target.codec = (*codec).to_string();
                }
                audio_index += 1;
            }
            StreamKind::Subtitle
            | StreamKind::Attachment
            | StreamKind::Chapter
            | StreamKind::Data => {}
        }
        desired.push(target);
    }

    if audio_index < audio_codecs.len() {
        return fail(format!(
            "{fixture_id} requested {} audio codec override(s), but source had {audio_index} audio stream(s)",
            audio_codecs.len()
        ));
    }
    Ok(desired)
}

fn assert_operations(label: &str, actual: &[String], expected: &[&str]) -> TestResult {
    let expected_values = expected
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();
    if actual != expected_values {
        return fail(format!(
            "{label} operation mismatch: expected {expected_values:?}, got {actual:?}"
        ));
    }
    Ok(())
}

fn assert_stream_codecs(
    label: &str,
    graph: &MediaGraph,
    kind: StreamKind,
    expected: &[&str],
) -> TestResult {
    let actual = streams_by_kind(graph, kind)
        .iter()
        .map(|stream| stream.codec.clone())
        .collect::<Vec<_>>();
    let expected_values = expected
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();
    if actual != expected_values {
        return fail(format!(
            "{label} {kind:?} codec mismatch: expected {expected_values:?}, got {actual:?}"
        ));
    }
    Ok(())
}

fn assert_multi_audio_selection(
    root: &Path,
    manifest: &FixtureManifest,
    output_root: &Path,
    report: &mut MediaConversionReport,
) -> TestResult {
    let fixture = fixture_by_id(manifest, "multi-audio-mkv")?;
    let source_path = root.join(&fixture.path);
    let graph = inspect_graph(&source_path)?;

    let english_only = graph
        .streams
        .iter()
        .filter(|stream| {
            stream.kind == StreamKind::Video
                || (stream.kind == StreamKind::Audio && stream.language.as_deref() == Some("eng"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let desired = desired_graph(
        path_text(&output_root.join("multi-audio-english-only.mkv"))?,
        english_only,
    )?;
    let materialized = materialize_desired_graph(&source_path, &graph, &desired)?;
    let output = inspect_graph(&materialized.verified_output_path)?;
    assert_audio_languages("multi-audio English only", &output, &["eng"])?;
    report.record_pipeline_action(PipelineReportRow {
        case_name: "multi-audio English-only selection".to_string(),
        fixture_id: fixture.id.clone(),
        input_path: fixture.path.clone(),
        output_path: report_path(root, &materialized.verified_output_path),
        operations: materialized.operations,
        outcome: "passed".to_string(),
        details: "selected only English audio and dropped non-English plus undeclared silent audio"
            .to_string(),
    });

    let mut ordered = graph
        .streams
        .iter()
        .filter(|stream| stream.kind == StreamKind::Video)
        .cloned()
        .collect::<Vec<_>>();
    for language in ["spa", "eng"] {
        let Some(stream) = graph.streams.iter().find(|stream| {
            stream.kind == StreamKind::Audio && stream.language.as_deref() == Some(language)
        }) else {
            return fail(format!("multi-audio missing {language} stream"));
        };
        ordered.push(stream.clone());
    }
    let desired = desired_graph(
        path_text(&output_root.join("multi-audio-ordered.mkv"))?,
        ordered,
    )?;
    let materialized = materialize_desired_graph(&source_path, &graph, &desired)?;
    let output = inspect_graph(&materialized.verified_output_path)?;
    assert_audio_languages("multi-audio ordered", &output, &["spa", "eng"])?;
    report.record_pipeline_action(PipelineReportRow {
        case_name: "multi-audio ordered selection".to_string(),
        fixture_id: fixture.id.clone(),
        input_path: fixture.path.clone(),
        output_path: report_path(root, &materialized.verified_output_path),
        operations: materialized.operations,
        outcome: "passed".to_string(),
        details: "preserved requested audio order spa,eng".to_string(),
    });
    Ok(())
}

fn assert_subtitle_selection(
    root: &Path,
    manifest: &FixtureManifest,
    output_root: &Path,
    report: &mut MediaConversionReport,
) -> TestResult {
    let fixture = fixture_by_id(manifest, "subtitles-mkv")?;
    let source_path = root.join(&fixture.path);
    let graph = inspect_graph(&source_path)?;

    let desired_all = desired_graph(
        path_text(&output_root.join("subtitles-all.mkv"))?,
        graph.streams.clone(),
    )?;
    let materialized = materialize_desired_graph(&source_path, &graph, &desired_all)?;
    let all_output = inspect_graph(&materialized.verified_output_path)?;
    assert_eq!(streams_by_kind(&all_output, StreamKind::Subtitle).len(), 2);
    report.record_pipeline_action(PipelineReportRow {
        case_name: "keep all subtitles".to_string(),
        fixture_id: fixture.id.clone(),
        input_path: fixture.path.clone(),
        output_path: report_path(root, &materialized.verified_output_path),
        operations: materialized.operations,
        outcome: "passed".to_string(),
        details: "retained both subtitle streams".to_string(),
    });

    let forced_only = graph
        .streams
        .iter()
        .filter(|stream| {
            stream.kind == StreamKind::Video
                || (stream.kind == StreamKind::Subtitle
                    && stream.dispositions.iter().any(|item| item == "forced"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let desired_forced = desired_graph(
        path_text(&output_root.join("subtitles-forced.mkv"))?,
        forced_only,
    )?;
    let materialized = materialize_desired_graph(&source_path, &graph, &desired_forced)?;
    let forced_output = inspect_graph(&materialized.verified_output_path)?;
    let forced_subtitles = streams_by_kind(&forced_output, StreamKind::Subtitle);
    assert_eq!(forced_subtitles.len(), 1);
    assert_eq!(forced_subtitles[0].language.as_deref(), Some("eng"));
    assert!(
        forced_subtitles[0]
            .dispositions
            .iter()
            .any(|item| item == "forced")
    );
    report.record_pipeline_action(PipelineReportRow {
        case_name: "keep forced subtitles only".to_string(),
        fixture_id: fixture.id.clone(),
        input_path: fixture.path.clone(),
        output_path: report_path(root, &materialized.verified_output_path),
        operations: materialized.operations,
        outcome: "passed".to_string(),
        details: "retained one forced English subtitle stream".to_string(),
    });

    let no_subtitles = graph
        .streams
        .iter()
        .filter(|stream| stream.kind != StreamKind::Subtitle)
        .cloned()
        .collect::<Vec<_>>();
    let desired_none = desired_graph(
        path_text(&output_root.join("subtitles-none.mkv"))?,
        no_subtitles,
    )?;
    let materialized = materialize_desired_graph(&source_path, &graph, &desired_none)?;
    let none_output = inspect_graph(&materialized.verified_output_path)?;
    assert!(streams_by_kind(&none_output, StreamKind::Subtitle).is_empty());
    report.record_pipeline_action(PipelineReportRow {
        case_name: "drop all subtitles".to_string(),
        fixture_id: fixture.id.clone(),
        input_path: fixture.path.clone(),
        output_path: report_path(root, &materialized.verified_output_path),
        operations: materialized.operations,
        outcome: "passed".to_string(),
        details: "removed all subtitle streams".to_string(),
    });
    Ok(())
}

fn inspect_graph(path: &Path) -> TestResult<MediaGraph> {
    let inspector = FfprobeInspectAdapter::new(Arc::new(SystemInspectProbeExecutor), "ffprobe");
    Ok(inspector.inspect(path)?.graph)
}

fn desired_graph(output_path: String, streams: Vec<MediaStream>) -> TestResult<DesiredGraph> {
    let mut desired_streams = Vec::with_capacity(streams.len());
    let mut stream_bindings = Vec::with_capacity(streams.len());
    for (output_index, mut stream) in streams.into_iter().enumerate() {
        let output_stream_id = u32::try_from(output_index)?;
        stream_bindings.push(DesiredStreamBinding {
            output_stream_id,
            source_stream_id: Some(stream.stream_id),
        });
        stream.stream_id = output_stream_id;
        desired_streams.push(stream);
    }
    Ok(DesiredGraph {
        output_path,
        container_chapters: Vec::new(),
        container_format: None,
        container_metadata_policy: None,
        container_chapter_policy: None,
        stream_bindings,
        streams: desired_streams,
    })
}

fn materialize_desired_graph(
    source_path: &Path,
    source: &MediaGraph,
    desired: &DesiredGraph,
) -> TestResult<MaterializedGraph> {
    let planned = plan_job_from_source_graph(desired, fs::metadata(source_path)?.len(), source)?;
    let verified_output_path =
        verified_output_path_for_plan(source_path, desired, &planned.operations);
    let operation_names = planned
        .operations
        .iter()
        .map(|operation| operation_kind_name(operation.kind).to_string())
        .collect::<Vec<_>>();
    let steps =
        build_job_execution_steps(&path_text(source_path)?, &desired.output_path, &planned)?;
    execute_step_sequence(&steps, &ProcessCommandRunner)?;
    Ok(MaterializedGraph {
        operations: operation_names,
        verified_output_path,
    })
}

fn verified_output_path_for_plan(
    source_path: &Path,
    desired: &DesiredGraph,
    operations: &[PlannedOperation],
) -> PathBuf {
    if planned_operations_are_noop(operations) {
        return source_path.to_path_buf();
    }
    PathBuf::from(&desired.output_path)
}

fn planned_operations_are_noop(operations: &[PlannedOperation]) -> bool {
    matches!(operations, [operation] if operation.kind == OperationKind::NoOp)
}

fn assert_audio_languages(label: &str, graph: &MediaGraph, expected: &[&str]) -> TestResult {
    let actual = streams_by_kind(graph, StreamKind::Audio)
        .iter()
        .map(|stream| stream.language.as_deref().unwrap_or("und").to_string())
        .collect::<Vec<_>>();
    let expected_values = expected
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();
    if actual != expected_values {
        return fail(format!(
            "{label} audio language mismatch: expected {expected_values:?}, got {actual:?}"
        ));
    }
    Ok(())
}

fn assert_pipeline_report_has_operation(
    report: &MediaConversionReport,
    operation: &str,
) -> TestResult {
    if report
        .pipeline_actions
        .iter()
        .any(|row| row.operations.iter().any(|item| item == operation))
    {
        return Ok(());
    }
    fail(format!(
        "media fixture pipeline report did not record a {operation} action"
    ))
}

fn streams_by_kind(graph: &MediaGraph, kind: StreamKind) -> Vec<&MediaStream> {
    graph
        .streams
        .iter()
        .filter(|stream| stream.kind == kind)
        .collect()
}

fn write_media_conversion_report(root: &Path, report: &MediaConversionReport) -> TestResult {
    let path = media_conversion_report_path(root)?;
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, report.render_markdown())?;
    Ok(())
}

fn media_conversion_report_path(root: &Path) -> TestResult<PathBuf> {
    let configured = std::env::var("REVAER_MEDIA_CONVERSION_REPORT").ok();
    let path = configured
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map_or_else(
            || root.join("target/media-conversion-report.md"),
            PathBuf::from,
        );
    if path.is_absolute() {
        return Ok(path);
    }
    Ok(root.join(path))
}

fn report_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_| path.display().to_string(),
        |relative| relative.display().to_string(),
    )
}

const fn operation_kind_name(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::NoOp => "no_op",
        OperationKind::Remux => "remux",
        OperationKind::MetadataRewrite => "metadata_rewrite",
        OperationKind::DispositionRewrite => "disposition_rewrite",
        OperationKind::LabelRewrite => "label_rewrite",
        OperationKind::StreamReorder => "stream_reorder",
        OperationKind::EmbedSubtitle => "embed_subtitle",
        OperationKind::ExtractSubtitle => "extract_subtitle",
        OperationKind::CopySidecarSubtitle => "copy_sidecar_subtitle",
        OperationKind::RemoveSidecarSubtitle => "remove_sidecar_subtitle",
        OperationKind::SubtitleTranscode => "subtitle_transcode",
        OperationKind::AudioTranscode => "audio_transcode",
        OperationKind::VideoTranscode => "video_transcode",
    }
}

fn markdown_cell(value: &str) -> String {
    markdown_inline(value).replace('|', "\\|")
}

fn markdown_inline(value: &str) -> String {
    value.replace('\r', "").replace('\n', "<br>")
}

fn path_text(path: &Path) -> TestResult<String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| std::io::Error::other("path is not valid UTF-8").into())
}

fn fail<T>(message: impl Into<String>) -> TestResult<T> {
    Err(std::io::Error::other(message.into()).into())
}
