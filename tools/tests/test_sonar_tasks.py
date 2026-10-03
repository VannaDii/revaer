"""Exercise real process and HTTP boundaries without submitting a Sonar analysis."""

import base64
import json
import os
import subprocess
import sys
import tarfile
from dataclasses import replace
from pathlib import Path

import pytest
from fixtures.sonar import ApiState, ScannerFixtureRunner, sonar_server
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.sonar import SonarApi, SonarScanner
from revaer_tooling.tasks.sonar import SonarPackageReport, SonarScan, SonarVerifyResult
from revaer_tooling.tasks.workflows import SonarPolicy


@pytest.fixture
def scanner_context(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> tuple[Context, ScannerFixtureRunner]:
    subprocess.run(["git", "init", str(tmp_path)], check=True, capture_output=True, timeout=10)
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    (tmp_path / "tools/versions.toml").write_text('[sonar]\nversion="8.1.0.6389"\n')
    (tmp_path / ".gitignore").write_text("artifacts/\n.scannerwork/\n")
    sentinel = tmp_path / ".sonar-test-scope/.gitkeep"
    sentinel.parent.mkdir()
    sentinel.touch()
    subprocess.run(
        ["git", "add", ".sonar-test-scope/.gitkeep"],
        cwd=tmp_path,
        check=True,
        capture_output=True,
        timeout=10,
    )
    configuration = Path(__file__).resolve().parents[2] / "sonar-project.properties"
    changes = {
        "sonar.sources": ".gitignore,sonar-project.properties,tools",
        "sonar.lang.patterns.yaml": "",
        "sonar.lang.patterns.kubernetes": "",
    }
    (tmp_path / "sonar-project.properties").write_text(
        "\n".join(
            key + "=" + changes[key] if (key := line.split("=", 1)[0]) in changes else line
            for line in configuration.read_text().splitlines()
            if not line.startswith("sonar.sca.sbomImportPaths=")
        )
        + "\n"
    )
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("SONAR_TOKEN", "private-scanner-fixture-token")
    monkeypatch.setenv("SONAR_API_RETRY_ATTEMPTS", "2")
    monkeypatch.setenv("SONAR_API_RETRY_DELAY_SECONDS", "0")
    monkeypatch.setenv("SONAR_RESULT_RETRY_ATTEMPTS", "2")
    monkeypatch.setenv("SONAR_RESULT_RETRY_DELAY_SECONDS", "0")
    monkeypatch.setenv("RV_TEST_SCANNER_CASE", "success")
    context = make_context(Options())
    runner = ScannerFixtureRunner()
    scanner = SonarScanner(sys.executable, runner, tmp_path, dict(os.environ))
    return replace(context, tools=replace(context.tools, sonar_scanner=scanner)), runner


def test_one_invocation_redaction_and_complete_archive(
    scanner_context: tuple[Context, ScannerFixtureRunner],
) -> None:
    context, runner = scanner_context
    assert SonarScan.run(context).message == "Authoritative Sonar task: owned-task"
    assert [invocation.argv[1:] for invocation in runner.invocations] == [("--version",), ()]
    log = context.root / "artifacts/sonar/scanner.log"
    assert "private-scanner-fixture-token" not in log.read_text()
    assert "[redacted]" in log.read_text()
    SonarPackageReport.run(context)
    archive = context.root / ".scannerwork/scanner-report.tar.xz"
    assert archive.stat().st_mode & 0o777 == 0o600
    with tarfile.open(archive) as report:
        assert report.getnames() == ["scanner-report/metadata.pb"]
        content = report.extractfile(report.getmembers()[0])
        assert content is not None
        with content:
            assert content.read() == b"\x00complete submitted report\xff"


def test_direct_scan_checks_scope_and_invalidates_old_evidence(
    scanner_context: tuple[Context, ScannerFixtureRunner],
) -> None:
    context, runner = scanner_context
    SonarScan.run(context)
    SonarPackageReport.run(context)
    calls = len(runner.invocations)
    (context.root / "new-source.py").write_text("value = 1\n")
    with pytest.raises(ToolingError, match=r"sonar\.sources"):
        SonarScan.run(context)
    assert len(runner.invocations) == calls
    assert not (context.root / ".scannerwork/report-task.txt").exists()
    assert not (context.root / ".scannerwork/scanner-report.tar.xz").exists()


@pytest.mark.parametrize("case", ("untracked", "nonempty", "nested", "symlink"))
def test_sonar_sentinel_is_tracked_empty_and_cannot_hide_authored_files(
    scanner_context: tuple[Context, ScannerFixtureRunner], case: str
) -> None:
    context, _ = scanner_context
    sentinel = context.root / ".sonar-test-scope"
    if case == "untracked":
        subprocess.run(
            ["git", "rm", "--cached", ".sonar-test-scope/.gitkeep"],
            cwd=context.root,
            check=True,
            capture_output=True,
            timeout=10,
        )
    elif case == "nonempty":
        (sentinel / ".gitkeep").write_text("authored content\n")
    elif case == "nested":
        (sentinel / "nested").mkdir()
    else:
        (sentinel / "source.py").symlink_to(context.root / "tools/src/revaer_tooling/cli.py")
    with pytest.raises(ToolingError, match=r"sentinel|sonar-test-scope"):
        SonarPolicy.run(context)


@pytest.mark.parametrize(
    "case",
    (
        "bad-version",
        "exit-failure",
        "empty-log",
        "warning",
        "missing-task",
        "duplicate-task",
        "missing-report",
    ),
)
def test_scanner_failure_never_reuses_previous_success(
    scanner_context: tuple[Context, ScannerFixtureRunner], case: str
) -> None:
    context, runner = scanner_context
    SonarScan.run(context)
    SonarPackageReport.run(context)
    previous_calls = len(runner.invocations)
    scanner = context.tools.sonar_scanner
    active = SonarScanner(
        scanner.name, runner, context.root, {**scanner.environment, "RV_TEST_SCANNER_CASE": case}
    )
    context = replace(context, tools=replace(context.tools, sonar_scanner=active))
    with pytest.raises(ToolingError) as error:
        SonarScan.run(context)
    if case == "exit-failure":
        assert error.value.exit_code == 7
    if case in ("bad-version", "exit-failure", "empty-log", "missing-task"):
        assert not (context.root / ".scannerwork/report-task.txt").exists()
    assert not (context.root / ".scannerwork/scanner-report.tar.xz").exists()
    assert len(runner.invocations) - previous_calls == (1 if case == "bad-version" else 2)


def test_archive_rejects_missing_or_linked_report_and_preserves_other_files(
    scanner_context: tuple[Context, ScannerFixtureRunner],
) -> None:
    context, _ = scanner_context
    SonarScan.run(context)
    report = context.root / ".scannerwork/scanner-report/metadata.pb"
    report.unlink()
    with pytest.raises(ToolingError, match="nonempty evidence"):
        SonarPackageReport.run(context)
    report.symlink_to(context.root / "tools/versions.toml")
    with pytest.raises(ToolingError, match="regular files"):
        SonarPackageReport.run(context)
    assert (context.root / "tools/versions.toml").is_file()
    assert not (context.root / ".scannerwork/scanner-report.tar.xz").exists()


def connect(context: Context, url: str, *, pull_request: str = "") -> Context:
    settings = replace(context.settings.sonar, api_base=url, pull_request=pull_request)
    api = SonarApi(context.tools.http, settings, lambda delay: None, context.emit)
    return replace(
        context,
        settings=replace(context.settings, sonar=settings),
        tools=replace(context.tools, sonar_api=api),
    )


@pytest.mark.parametrize("pull_request", ("", "12"))
def test_result_queries_exact_analysis_and_correct_backlog_scope(
    scanner_context: tuple[Context, ScannerFixtureRunner], pull_request: str
) -> None:
    context, _ = scanner_context
    SonarScan.run(context)
    state = ApiState(statuses=[429], pending_tasks=2)
    with sonar_server(state) as url:
        context = connect(context, url, pull_request=pull_request)
        result = SonarVerifyResult.run(context)
    assert "analysis=owned-analysis" in result.message
    expected_authorization = (
        "Basic " + base64.b64encode((context.settings.sonar.auth_token + ":").encode()).decode()
    )
    for endpoint, query, authorization in state.requests:
        assert authorization == expected_authorization
        if endpoint == "/api/qualitygates/project_status":
            assert query == {"analysisId": ["owned-analysis"]}
        elif endpoint in ("/api/issues/search", "/api/hotspots/search"):
            assert "status" not in query and "statuses" not in query
            if pull_request:
                assert query["pullRequest"] == [pull_request]
                assert query["inNewCodePeriod"] == ["true"]
            else:
                assert "pullRequest" not in query and "inNewCodePeriod" not in query
    evidence = context.root / "artifacts/sonar/api"
    assert {path.stem for path in evidence.glob("*.json")} == set(state.records)
    for name, record in state.records.items():
        path = evidence / (name + ".json")
        assert json.loads(path.read_text()) == record
        assert path.stat().st_mode & 0o777 == 0o600


def test_failing_result_retains_final_evidence_and_never_hides_hotspots(
    scanner_context: tuple[Context, ScannerFixtureRunner],
) -> None:
    context, _ = scanner_context
    SonarScan.run(context)
    evidence = context.root / "artifacts/sonar/api"
    context.fs.write(evidence / "notes.txt", "other evidence")
    context.fs.write(evidence / "issues.json", '{"total":0}')
    state = ApiState()
    state.records["hotspots"] = {"paging": {"total": 1}, "hotspots": [{"status": "REVIEWED"}]}
    with sonar_server(state) as url, pytest.raises(ToolingError, match="current hotspots"):
        SonarVerifyResult.run(connect(context, url))
    assert json.loads((evidence / "hotspots.json").read_text()) == state.records["hotspots"]
    assert (evidence / "notes.txt").read_text() == "other evidence"


def test_api_authentication_and_retry_exhaustion_fail(
    scanner_context: tuple[Context, ScannerFixtureRunner],
) -> None:
    context, _ = scanner_context
    for statuses, message, count in (([401], "HTTP 401", 1), ([503, 503], "after 2 attempts", 2)):
        state = ApiState(statuses=statuses)
        with sonar_server(state) as url:
            active = connect(context, url)
            with pytest.raises(ToolingError, match=message):
                active.tools.sonar_api.get("ce/task", {"id": "owned-task"})
        assert len(state.requests) == count


@pytest.mark.parametrize(
    "name,value",
    (
        ("SONAR_SCANNER_JSON_PARAMS", '{"sonar.exclusions":"**/*"}'),
        ("SONARQUBE_SCANNER_PARAMS", '{"sonar.scm.disabled":"true"}'),
        ("SONAR_SCANNER_PARAMS", "override"),
        ("SONAR_SCANNER_OPTS", "-Dsonar.exclusions=**/*"),
        ("SONAR_SCANNER_JAVA_OPTS", "-Xmx4G -Dsonar.sources=other"),
        ("JAVA_TOOL_OPTIONS", "-Dproject.settings=other.properties"),
        ("JDK_JAVA_OPTIONS", "-Dsonar.scm.disabled=true"),
    ),
)
def test_environment_cannot_replace_reviewed_scanner_criteria(
    scanner_context: tuple[Context, ScannerFixtureRunner], name: str, value: str
) -> None:
    context, runner = scanner_context
    template = context.tools.sonar_scanner
    scanner = SonarScanner(
        template.name, runner, context.root, {**template.environment, name: value}
    )
    with pytest.raises(ToolingError, match="cannot override"):
        SonarScan.run(replace(context, tools=replace(context.tools, sonar_scanner=scanner)))
    assert [call.argv[1:] for call in runner.invocations] == [("--version",)]
