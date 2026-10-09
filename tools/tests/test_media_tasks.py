"""Fresh reports, literal Cargo selectors, and cleanup limited to this checkout."""

import hashlib
import json
import shutil
import subprocess
import tomllib
from dataclasses import replace
from pathlib import Path

import pytest
from fixtures.media import ProbeRunner, media_context
from revaer_tooling.cli import COMMANDS
from revaer_tooling.context import Context
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.media import Ffprobe
from revaer_tooling.external.release import parse_outputs
from revaer_tooling.external.rust import Cargo
from revaer_tooling.tasks.media import (
    CleanFixtures,
    CleanTestMedia,
    DownloadFixtures,
    FixtureCacheKey,
    UpdateFixtureProbes,
    VerifyFixtures,
    verify_conversion_report,
)
from revaer_tooling.tasks.media import (
    TestMediaConversion as MediaConversion,
)

__all__ = ["media_context"]
REPORT = """# Synthetic conversion boundary report
- Outcome: passed
- Pipeline actions: 1
- Video transcodes: 1
- Audio transcodes: 1
- Pipeline failures: 0
- Suite failures: 0
"""


def probe_context(context: Context) -> Context:
    probe = Ffprobe("ffprobe", ProbeRunner(), context.root, context.tools.ffprobe.environment)
    return replace(context, tools=replace(context.tools, ffprobe=probe))


def test_static_dispatch_and_fresh_fixture_report(media_context: Context) -> None:
    context = probe_context(media_context)
    assert COMMANDS["verify-test-fixtures"] is VerifyFixtures.run
    assert COMMANDS["download-test-fixtures"] is DownloadFixtures.run
    VerifyFixtures.run(context)
    report = context.root / context.settings.fixtures.report
    assert "- Locked source fixtures: 2\n" in report.read_text()
    (context.root / "test-fixtures/source/bbb-h264-mp4.mp4").write_bytes(b"corrupted")
    with pytest.raises(ToolingError):
        VerifyFixtures.run(context)
    assert not report.exists()


def test_invalid_catalog_invalidates_previous_success(media_context: Context) -> None:
    context = probe_context(media_context)
    VerifyFixtures.run(context)
    (context.root / context.settings.fixtures.manifest).write_text("invalid JSON")
    with pytest.raises(ToolingError, match="valid JSON"):
        VerifyFixtures.run(context)
    assert not (context.root / context.settings.fixtures.report).exists()


@pytest.mark.parametrize(
    "field",
    (
        "Outcome",
        "Pipeline actions",
        "Video transcodes",
        "Audio transcodes",
        "Pipeline failures",
        "Suite failures",
    ),
)
@pytest.mark.parametrize("change", ("missing", "wrong", "duplicate"))
def test_conversion_report_requires_one_exact_value(
    tmp_path: Path, field: str, change: str
) -> None:
    path = tmp_path / "report"
    line = next(value for value in REPORT.splitlines() if value.startswith("- " + field + ":"))
    wrong = "failed" if field == "Outcome" else "1" if field.endswith("failures") else "0"
    replacement = (
        ""
        if change == "missing"
        else line + "\n" + line
        if change == "duplicate"
        else f"- {field}: {wrong}"
    )
    path.write_text(REPORT.replace(line, replacement))
    with pytest.raises(ToolingError, match=field):
        verify_conversion_report(path)


def test_cleanup_refuses_tracked_or_linked_files_before_any_removal(media_context: Context) -> None:
    context = media_context
    path = context.root / "test-fixtures/source/bbb-h264-mp4.mp4"
    subprocess.run(["git", "add", "--", str(path)], check=True, capture_output=True, timeout=10)
    with pytest.raises(ToolingError, match="tracked source"):
        CleanFixtures.run(context)
    assert path.is_file()
    subprocess.run(
        ["git", "rm", "--cached", "--", str(path)], check=True, capture_output=True, timeout=10
    )
    foreign = context.root.parent / "foreign"
    foreign.mkdir()
    link = context.root / "test-fixtures/chromium/linked"
    link.symlink_to(foreign)
    with pytest.raises(ToolingError, match="symlinks"):
        CleanFixtures.run(context)
    assert path.is_file()
    assert foreign.is_dir()
    link.unlink()
    CleanFixtures.run(context)
    assert not path.exists()
    assert (context.root / "test-fixtures/manifest.json").exists()
    assert (context.root / "test-fixtures/probe").is_dir()


def test_download_refuses_to_replace_a_tracked_fixture(media_context: Context) -> None:
    context = media_context
    path = context.root / "test-fixtures/source/bbb-h264-mp4.mp4"
    subprocess.run(["git", "add", "--", str(path)], check=True, capture_output=True, timeout=10)
    with pytest.raises(ToolingError, match="tracked source"):
        DownloadFixtures.run(context)


def test_media_cleanup_keeps_other_checkouts_and_old_global_temporary_data(
    media_context: Context,
) -> None:
    context = media_context
    owned = context.root / "target/media-conversion/tmp/revaer-media-conversion.current"
    owned.mkdir(parents=True)
    (owned / "payload").write_text("temporary")
    foreign = context.root.parent / "revaer-media-conversion.other"
    foreign.mkdir()
    CleanTestMedia.run(context)
    assert foreign.is_dir()
    assert not owned.exists()


def test_update_command_changes_only_declared_snapshots(media_context: Context) -> None:
    context = probe_context(media_context)
    path = context.root / "test-fixtures/probe/bbb-h264-mp4.json"
    path.write_text("stale reviewed snapshot")
    untouched = path.parent / "other.json"
    untouched.write_text("unrelated")
    UpdateFixtureProbes.run(context)
    assert json.loads(path.read_text())["streams"][0]["codec_name"] == "h264"
    assert untouched.read_text() == "unrelated"


def test_cache_key_binds_native_headers_and_fixture_implementation(media_context: Context) -> None:
    context = media_context
    original = Path(__file__).parents[2]
    paths = (
        "tools/src/revaer_tooling/tasks/media.py",
        "tools/src/revaer_tooling/external/media.py",
        "tools/src/revaer_tooling/external/curl.py",
        "tools/src/revaer_tooling/process.py",
    )
    for name in paths:
        destination = context.root / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(original / name, destination)
    shutil.copytree(
        original / "tools/src/revaer_tooling/media",
        context.root / "tools/src/revaer_tooling/media",
        ignore=shutil.ignore_patterns("__pycache__"),
    )
    output = context.root / "workflow-output"
    context = replace(
        context,
        settings=replace(
            context.settings, workflow=replace(context.settings.workflow, output=output)
        ),
    )
    first = json.loads(FixtureCacheKey.run(context).message)
    headers = (
        context.tools.ffmpeg.report().splitlines()[0]
        + b"\n"
        + context.tools.ffprobe.report().splitlines()[0]
        + b"\n"
    )
    assert first["tool_version"] == hashlib.sha256(headers).hexdigest()
    assert parse_outputs(output.read_text()) == first
    implementation = context.root / paths[0]
    implementation.write_text(implementation.read_text() + "\n# Fixture mutation\n")
    second = json.loads(FixtureCacheKey.run(context).message)
    assert second["tool_version"] == first["tool_version"]
    assert second["fixtures"] != first["fixtures"]


@pytest.fixture
def cargo_context(media_context: Context) -> Context:
    context = probe_context(media_context)
    root = context.root
    channel = tomllib.loads((Path(__file__).parents[2] / "rust-toolchain.toml").read_text())[
        "toolchain"
    ]["channel"]
    (root / "Cargo.toml").write_text(
        '[package]\nname="revaer-media-runtime"\nversion="0.1.0"\nedition="2024"\n'
        "[features]\nfixture-feature=[]\n[workspace]\n"
    )
    (root / "src").mkdir()
    (root / "src/lib.rs").write_text("//! Synthetic Cargo selection fixture.\n")
    (root / "tests").mkdir()
    (root / "tests/media_fixtures.rs").write_text(r"""
#[test]
#[ignore]
fn selected_ignored_integration_runs() -> Result<(), Box<dyn std::error::Error>> {
    assert!(cfg!(feature = "fixture-feature"));
    let report = std::path::PathBuf::from(std::env::var("REVAER_MEDIA_CONVERSION_REPORT")?);
    let temporary = std::path::PathBuf::from(std::env::var("TMPDIR")?);
    assert!(temporary.starts_with(std::env::current_dir()?.join("target/media-conversion")));
    std::fs::write(temporary.join("owned-media"), "observed")?;
    let mode = std::env::var("RV_FIXTURE_MODE")?;
    if mode == "failure" { return Err("injected test failure".into()); }
    if mode == "missing" { return Ok(()); }
    let text = std::env::var("RV_FIXTURE_REPORT")?;
    std::fs::write(report, text)?;
    Ok(())
}
""")
    environment = {
        **context.tools.cargo.environment,
        "RUSTUP_TOOLCHAIN": channel,
        "CARGO_TARGET_DIR": str(root / "target/cargo"),
        "RV_FIXTURE_REPORT": REPORT,
        "RV_FIXTURE_MODE": "pass",
    }
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"],
        cwd=root,
        env=environment,
        check=True,
        capture_output=True,
        timeout=30,
    )
    cargo = Cargo("cargo", context.tools.cargo.runner, root, environment)
    return replace(context, tools=replace(context.tools, cargo=cargo))


def test_native_cargo_runs_ignored_integration_with_all_features(cargo_context: Context) -> None:
    context = cargo_context
    result = MediaConversion.run(context)
    assert "positive audio/video actions" in result.message
    assert (context.root / context.settings.fixtures.report).read_text() == REPORT
    assert (context.root / "target/media-conversion/tmp/owned-media").read_text() == "observed"
    assert (context.root / (str(context.settings.fixtures.report) + ".preparation")).is_file()
    # Repeat through the same native crate: stale success must not survive a
    # failing test process or a successful process that failed to write a report.
    for mode in ("failure", "missing"):
        cargo = Cargo(
            "cargo",
            context.tools.cargo.runner,
            context.root,
            {**context.tools.cargo.environment, "RV_FIXTURE_MODE": mode},
        )
        attempt = replace(context, tools=replace(context.tools, cargo=cargo))
        with pytest.raises((CommandError, ToolingError)):
            MediaConversion.run(attempt)
        assert not (context.root / context.settings.fixtures.report).exists()


@pytest.mark.parametrize("worker", ("pass", "failure", "absent"))
def test_conversion_requires_executed_production_worker(
    cargo_context: Context, worker: str
) -> None:
    context = cargo_context
    manifest = context.root / "Cargo.toml"
    manifest.write_text(manifest.read_text().replace("[workspace]", '[workspace]\nmembers=["app"]'))
    app = context.root / "app"
    (app / "src").mkdir(parents=True)
    (app / "Cargo.toml").write_text(
        '[package]\nname="revaer-app"\nversion="0.1.0"\nedition="2024"\n'
    )
    name = (
        "unselected_worker"
        if worker == "absent"
        else "production_media_job_runtime_executes_and_persists_verified_replacement"
    )
    (app / "src/lib.rs").write_text(
        "mod media_job_runtime { mod tests {\n#[test]\n#[ignore]\n"
        f"fn {name}() -> Result<(), Box<dyn std::error::Error>> {{\n"
        'let database = std::env::var("REVAER_TEST_DATABASE_URL")?;\n'
        'assert_eq!(database, "postgresql://localhost/fixture");\n'
        'std::fs::write("worker-executed", "observed")?;\n'
        + ('Err("injected worker failure".into())' if worker == "failure" else "Ok(())")
        + "\n} } }\n"
    )
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"],
        cwd=context.root,
        env=context.tools.cargo.environment,
        check=True,
        capture_output=True,
        timeout=30,
    )
    context = replace(
        context,
        settings=replace(
            context.settings,
            database=replace(context.settings.database, test_url="postgresql://localhost/fixture"),
        ),
    )
    if worker == "pass":
        MediaConversion.run(context)
        assert (app / "worker-executed").read_text() == "observed"
        assert (context.root / context.settings.fixtures.report).read_text() == REPORT
    else:
        with pytest.raises((CommandError, ToolingError)):
            MediaConversion.run(context)
        assert not (context.root / context.settings.fixtures.report).exists()
        assert not (context.root / "target/media-conversion/tmp/owned-media").exists()


def test_foundation_reports_only_fixture_verification(cargo_context: Context) -> None:
    context = cargo_context
    manifest = context.root / "Cargo.toml"
    manifest.write_text(
        manifest.read_text().replace('name="revaer-media-runtime"', 'name="foundation"')
    )
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"],
        cwd=context.root,
        env=context.tools.cargo.environment,
        check=True,
        capture_output=True,
        timeout=30,
    )
    assert "Source integrity" in MediaConversion.run(context).message
    assert not (context.root / "target/media-conversion/tmp/owned-media").exists()
