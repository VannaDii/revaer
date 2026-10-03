"""Actual Git metadata, reviewed matrices, summaries and fail-closed job results."""

import json
import shutil
import subprocess
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.release import parse_outputs
from revaer_tooling.settings import load_settings
from revaer_tooling.tasks.automation import (
    VerifySupplyChainResults,
    WorkflowChartVersions,
    WorkflowMatrix,
    WorkflowMetadata,
    WorkflowReport,
)


@pytest.fixture
def repository(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    source = Path(__file__).resolve().parents[2]
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    shutil.copytree(source / ".github/matrices", tmp_path / ".github/matrices")
    for arguments in (
        ("init", "--quiet", "--initial-branch=main"),
        ("add", "."),
        (
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "--no-gpg-sign",
            "-m",
            "Initial fixture",
        ),
    ):
        subprocess.run(
            ["git", *arguments], cwd=tmp_path, check=True, capture_output=True, timeout=30
        )
    monkeypatch.chdir(tmp_path)
    return replace(
        make_context(Options()), settings=load_settings({"GITHUB_OUTPUT": str(tmp_path / "output")})
    )


def test_workflow_outputs_identify_the_actual_checkout_and_reviewed_matrix(
    repository: Context,
) -> None:
    metadata = json.loads(WorkflowMetadata.run(repository).message)
    expected = subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True, timeout=30
    ).stdout.strip()
    assert metadata == {"sha": expected, "short_sha": expected[:7]}
    matrix = json.loads(WorkflowMatrix.run(repository).message)
    assert {entry["platform"] for entry in json.loads(matrix["matrix"])["include"]} == {
        "linux/amd64",
        "linux/arm64",
    }
    assert parse_outputs((repository.root / "output").read_text()) == matrix
    (repository.root / ".github/matrices/build-images.json").write_text('{"include":[]}')
    before = (repository.root / "output").read_bytes()
    with pytest.raises(ToolingError, match="architecture contract"):
        WorkflowMatrix.run(repository)
    assert (repository.root / "output").read_bytes() == before


@pytest.mark.parametrize(
    ("kind", "name", "expected"),
    (
        ("branch", "main", True),
        ("branch", "feature", False),
        ("tag", "v1.2.3", True),
        ("tag", "v1.2.3-dev.1", False),
        ("tag", "v01.2.3", False),
    ),
)
def test_release_matrix_keeps_main_and_stable_tag_boundaries(
    repository: Context, kind: str, name: str, expected: bool
) -> None:
    context = replace(
        repository,
        options=Options(release_matrix=True),
        settings=load_settings({"GITHUB_REF_TYPE": kind, "GITHUB_REF_NAME": name}),
    )
    record = json.loads(WorkflowMatrix.run(context).message)
    assert bool(json.loads(record["matrix"])["include"]) is expected
    context = replace(context, settings=load_settings({}))
    with pytest.raises(ToolingError, match="ref type and name"):
        WorkflowMatrix.run(context)


def test_summary_requires_a_nonempty_owned_report_and_preserves_previous_content(
    repository: Context,
) -> None:
    summary = repository.root / "summary"
    summary.write_text("Earlier step\n")
    report = repository.root / "report.md"
    report.write_text("# Result\n\nPassed with retained evidence.\n")
    context = replace(
        repository,
        options=Options(report=Path("report.md")),
        settings=load_settings({"GITHUB_STEP_SUMMARY": str(summary)}),
    )
    assert WorkflowReport.run(context).message == report.read_text()
    assert summary.read_text() == "Earlier step\n" + report.read_text()
    before = summary.read_bytes()
    report.write_text("")
    with pytest.raises(ToolingError, match="nonempty"):
        WorkflowReport.run(context)
    report.unlink()
    report.symlink_to(summary)
    with pytest.raises(ToolingError, match="current checkout"):
        WorkflowReport.run(context)
    with pytest.raises(ToolingError, match="Select a workflow report"):
        WorkflowReport.run(replace(context, options=Options()))
    assert summary.read_bytes() == before


@pytest.mark.parametrize("name", ("AUDIT_RESULT", "DENY_RESULT", "UDEPS_RESULT"))
@pytest.mark.parametrize("value", ("failure", "cancelled", "skipped", "", "success\n"))
def test_supply_chain_rejects_every_result_other_than_success(
    repository: Context, name: str, value: str
) -> None:
    environment = {"AUDIT_RESULT": "success", "DENY_RESULT": "success", "UDEPS_RESULT": "success"}
    assert (
        "checks passed"
        in VerifySupplyChainResults.run(
            replace(repository, settings=load_settings(environment))
        ).message
    )
    environment[name] = value
    with pytest.raises(ToolingError, match=name):
        VerifySupplyChainResults.run(replace(repository, settings=load_settings(environment)))
    context = replace(
        repository,
        settings=replace(
            repository.settings,
            workflow=replace(repository.settings.workflow, supply_chain_results=()),
        ),
    )
    with pytest.raises(ToolingError, match="must include"):
        VerifySupplyChainResults.run(context)


def test_chart_defaults_and_explicit_overrides_keep_source_identity(repository: Context) -> None:
    source = repository.tools.git.revision()[:7]
    context = replace(
        repository,
        options=Options(chart_version="", app_version="", pull_request=61),
        settings=load_settings({"GITHUB_RUN_NUMBER": "8"}),
    )
    assert json.loads(WorkflowChartVersions.run(context).message) == {
        "pr_number": "61",
        "chart_version": "0.0.0-dev.pr61.8",
        "app_version": f"pr-61-{source}",
    }
    context = replace(context, options=Options(chart_version="1.2.3", app_version="custom-tag"))
    assert json.loads(WorkflowChartVersions.run(context).message) == {
        "pr_number": "",
        "chart_version": "1.2.3",
        "app_version": "custom-tag",
    }
    context = replace(context, options=Options(chart_version="1.2.3", app_version=""))
    assert (
        json.loads(WorkflowChartVersions.run(context).message)["app_version"] == f"verify-{source}"
    )


@pytest.mark.parametrize("number", ("", "0", "-1", "01", "1\n"))
def test_default_chart_version_requires_a_real_workflow_run(
    repository: Context, number: str
) -> None:
    context = replace(
        repository,
        options=Options(chart_version="", app_version="", pull_request=61),
        settings=load_settings({"GITHUB_RUN_NUMBER": number}),
    )
    with pytest.raises(ToolingError, match="positive GITHUB_RUN_NUMBER"):
        WorkflowChartVersions.run(context)


def test_absent_or_invalid_pr_requires_an_explicit_version(repository: Context) -> None:
    for number in (None, 0, -1):
        context = replace(
            repository, options=Options(chart_version="", app_version="", pull_request=number)
        )
        with pytest.raises(ToolingError, match=r"No open pull request|number must be positive"):
            WorkflowChartVersions.run(context)
