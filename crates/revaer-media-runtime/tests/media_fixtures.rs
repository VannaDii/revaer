use revaer_media_core::model::{DesiredGraph, MediaGraph, MediaStream, StreamKind};
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
#[ignore = "requires downloaded and generated media fixtures"]
fn verify_prepared_fixture_suite() -> TestResult {
    require_tool("ffmpeg")?;
    require_tool("ffprobe")?;
    require_tool("curl")?;
    require_tool("git")?;
    require_tool("base64")?;

    let root = repo_root()?;
    let manifest = load_manifest(&root)?;
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
        write_probe_snapshot_if_requested(&root, fixture, &probe)?;
        probes.insert(fixture.id.as_str(), probe);
    }

    validate_derived_metadata(&root, &probes)?;
    run_pipeline_cases(&root, &manifest)?;
    println!(
        "verify_prepared_fixture_suite: verified {} fixtures",
        manifest.fixtures.len()
    );
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
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(value) if value.success() => Ok(()),
        Ok(value) => fail(format!(
            "required tool preflight failed: {tool} status={value}"
        )),
        Err(error) => fail(format!("required tool missing: {tool}: {error}")),
    }
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

fn validate_derived_metadata(root: &Path, probes: &BTreeMap<&str, Value>) -> TestResult {
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

    let silent = probe_by_id(probes, "silent-audio-mp4")?;
    if streams_of_kind(silent, "audio")?.is_empty() {
        return fail("silent-audio-mp4 expected an audio stream");
    }
    validate_silence_with_ffmpeg(&root.join("test-fixtures/derived/silent-audio.mp4"))?;
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

fn run_pipeline_cases(root: &Path, manifest: &FixtureManifest) -> TestResult {
    let output_root = root.join("target/media-fixture-integration");
    if output_root.exists() {
        fs::remove_dir_all(&output_root)?;
    }
    fs::create_dir_all(&output_root)?;

    for id in [
        "bbb-h264-mp4",
        "bbb-h265-mp4",
        "bbb-av1-mp4",
        "bbb-vp8-webm",
        "bbb-vp9-webm",
        "bbb-h264-mkv",
        "chromium-bear-1280x720-av-frag-mp4",
        "chromium-bbb-2video-2audio-mp4",
        "chromium-multitrack-3video-2audio-webm",
        "h264-aac-ts",
        "h264-aac-mov",
        "mpeg4-mp3-avi",
        "video-only-mp4",
        "audio-only-m4a",
    ] {
        let fixture = fixture_by_id(manifest, id)?;
        materialize_same_graph(root, fixture, &output_root)?;
    }

    assert_multi_audio_selection(root, manifest, &output_root)?;
    assert_subtitle_selection(root, manifest, &output_root)?;
    Ok(())
}

fn fixture_by_id<'a>(manifest: &'a FixtureManifest, id: &str) -> TestResult<&'a FixtureEntry> {
    manifest
        .fixtures
        .iter()
        .find(|fixture| fixture.id == id)
        .ok_or_else(|| std::io::Error::other(format!("missing fixture {id}")).into())
}

fn materialize_same_graph(root: &Path, fixture: &FixtureEntry, output_root: &Path) -> TestResult {
    let source_path = root.join(&fixture.path);
    let graph = inspect_graph(&source_path)?;
    let output_path = output_root.join(
        Path::new(&fixture.path)
            .file_name()
            .ok_or_else(|| std::io::Error::other("fixture output filename missing"))?,
    );
    let desired = DesiredGraph {
        output_path: path_text(&output_path)?,
        streams: graph.streams.clone(),
    };
    materialize_desired_graph(&source_path, &graph, &desired)?;
    let output_graph = inspect_graph(&output_path)?;
    assert_eq!(
        output_graph.streams.len(),
        desired.streams.len(),
        "{} output stream count mismatch",
        fixture.id
    );
    Ok(())
}

fn assert_multi_audio_selection(
    root: &Path,
    manifest: &FixtureManifest,
    output_root: &Path,
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
    let desired = DesiredGraph {
        output_path: path_text(&output_root.join("multi-audio-english-only.mkv"))?,
        streams: english_only,
    };
    materialize_desired_graph(&source_path, &graph, &desired)?;
    let output = inspect_graph(Path::new(&desired.output_path))?;
    assert_audio_languages("multi-audio English only", &output, &["eng"])?;

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
    let desired = DesiredGraph {
        output_path: path_text(&output_root.join("multi-audio-ordered.mkv"))?,
        streams: ordered,
    };
    materialize_desired_graph(&source_path, &graph, &desired)?;
    let output = inspect_graph(Path::new(&desired.output_path))?;
    assert_audio_languages("multi-audio ordered", &output, &["spa", "eng"])?;
    Ok(())
}

fn assert_subtitle_selection(
    root: &Path,
    manifest: &FixtureManifest,
    output_root: &Path,
) -> TestResult {
    let fixture = fixture_by_id(manifest, "subtitles-mkv")?;
    let source_path = root.join(&fixture.path);
    let graph = inspect_graph(&source_path)?;

    let desired_all = DesiredGraph {
        output_path: path_text(&output_root.join("subtitles-all.mkv"))?,
        streams: graph.streams.clone(),
    };
    materialize_desired_graph(&source_path, &graph, &desired_all)?;
    let all_output = inspect_graph(Path::new(&desired_all.output_path))?;
    assert_eq!(streams_by_kind(&all_output, StreamKind::Subtitle).len(), 2);

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
    let desired_forced = DesiredGraph {
        output_path: path_text(&output_root.join("subtitles-forced.mkv"))?,
        streams: forced_only,
    };
    materialize_desired_graph(&source_path, &graph, &desired_forced)?;
    let forced_output = inspect_graph(Path::new(&desired_forced.output_path))?;
    let forced_subtitles = streams_by_kind(&forced_output, StreamKind::Subtitle);
    assert_eq!(forced_subtitles.len(), 1);
    assert_eq!(forced_subtitles[0].language.as_deref(), Some("eng"));
    assert!(
        forced_subtitles[0]
            .dispositions
            .iter()
            .any(|item| item == "forced")
    );

    let no_subtitles = graph
        .streams
        .iter()
        .filter(|stream| stream.kind != StreamKind::Subtitle)
        .cloned()
        .collect::<Vec<_>>();
    let desired_none = DesiredGraph {
        output_path: path_text(&output_root.join("subtitles-none.mkv"))?,
        streams: no_subtitles,
    };
    materialize_desired_graph(&source_path, &graph, &desired_none)?;
    let none_output = inspect_graph(Path::new(&desired_none.output_path))?;
    assert!(streams_by_kind(&none_output, StreamKind::Subtitle).is_empty());
    Ok(())
}

fn inspect_graph(path: &Path) -> TestResult<MediaGraph> {
    let inspector = FfprobeInspectAdapter::new(Arc::new(SystemInspectProbeExecutor), "ffprobe");
    Ok(inspector.inspect(&path_text(path)?)?)
}

fn materialize_desired_graph(
    source_path: &Path,
    source: &MediaGraph,
    desired: &DesiredGraph,
) -> TestResult {
    let planned = plan_job_from_source_graph(desired, fs::metadata(source_path)?.len(), source)?;
    let steps =
        build_job_execution_steps(&path_text(source_path)?, &desired.output_path, &planned)?;
    execute_step_sequence(&steps, &ProcessCommandRunner)?;
    Ok(())
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

fn streams_by_kind(graph: &MediaGraph, kind: StreamKind) -> Vec<&MediaStream> {
    graph
        .streams
        .iter()
        .filter(|stream| stream.kind == kind)
        .collect()
}

fn path_text(path: &Path) -> TestResult<String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| std::io::Error::other("path is not valid UTF-8").into())
}

fn fail<T>(message: impl Into<String>) -> TestResult<T> {
    Err(std::io::Error::other(message.into()).into())
}
