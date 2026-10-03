"""Reject invalid Helm/OCI version inputs before any external operation."""

import sys
from pathlib import Path

import pytest
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.github import GitHub, PullRequestArgs
from revaer_tooling.external.release import ReleaseRepository
from revaer_tooling.tasks.charts import validate_app_version, validate_version
from test_external import RecordingRunner


@pytest.mark.parametrize(
    "version", ("0.0.0", "1.2.3-dev.pr61.7", "1.2.3+01.build", "1.2.3-0.a-b+build.9")
)
def test_semver_versions_are_accepted(version: str) -> None:
    assert validate_version(version) == version


@pytest.mark.parametrize(
    "version",
    (
        "",
        "01.2.3",
        "1.2.3-01",
        "1.2.3-..",
        "1.2.3+",
        "1.2.3-a..b",
        "\u0661.2.3",
        "v1.2.3",
        "1.2.3\n",
        "--1.2.3",
    ),
)
def test_malformed_chart_versions_fail(version: str) -> None:
    with pytest.raises(ToolingError, match="Invalid chart version"):
        validate_version(version)


@pytest.mark.parametrize("version", ("", "a/b", "a b", "a\n", "é", "1;command"))
def test_app_version_keeps_the_ascii_workflow_contract(version: str) -> None:
    with pytest.raises(ToolingError, match="Invalid chart application version"):
        validate_app_version(version)


@pytest.mark.parametrize(
    "response",
    (
        "invalid",
        "{}",
        "[null]",
        '[{"number":1,"head":null}]',
        '[{"number":true,"state":"open","head":{"ref":"branch","repo":{"full_name":"owner/repo"}}}]',
        '[{"number":1,"state":"closed","head":{"ref":"branch","repo":{"full_name":"owner/repo"}}}]',
        '[{"number":1,"state":"open","head":{"ref":"wrong","repo":{"full_name":"owner/repo"}}}]',
        '[{"number":1,"state":"open","head":{"ref":"branch","repo":{"full_name":"other/repo"}}}]',
    ),
)
def test_invalid_pull_request_responses_are_errors(tmp_path: Path, response: str) -> None:
    tool = GitHub(sys.executable, RecordingRunner(response), tmp_path, {})
    with pytest.raises(ToolingError):
        tool.open_pull_request(
            PullRequestArgs(ReleaseRepository("github.com", "owner/repo"), "branch")
        )


@pytest.mark.parametrize(
    ("repository", "branch"),
    (("invalid", "branch"), ("owner/repo", ""), ("owner/repo", "branch\n")),
)
def test_invalid_query_inputs_never_start_a_child(
    tmp_path: Path, repository: str, branch: str
) -> None:
    runner = RecordingRunner()
    tool = GitHub(sys.executable, runner, tmp_path, {})
    with pytest.raises(ToolingError):
        tool.open_pull_request(PullRequestArgs(ReleaseRepository("github.com", repository), branch))
    assert not runner.calls
