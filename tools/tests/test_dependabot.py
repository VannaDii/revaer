"""Provider identities and exact-head native auto-merge retain protected CI gates."""

import copy
import json
import sys
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.github import AutoMergeArgs, GitHub
from revaer_tooling.external.release import ReleaseRepository
from revaer_tooling.process import Completed, Invocation, RunningProcess
from revaer_tooling.tasks.dependabot import DependabotAutoMerge

REPOSITORY = ReleaseRepository("github.com", "VannaDii/revaer")
HEAD = "a" * 40


def request(number: int = 276) -> dict[str, object]:
    return {
        "number": number,
        "state": "open",
        "draft": False,
        "auto_merge": None,
        "user": {"login": "dependabot[bot]", "type": "Bot"},
        "head": {"sha": HEAD, "repo": {"full_name": REPOSITORY.name}},
        "base": {"ref": "main", "repo": {"full_name": REPOSITORY.name}},
    }


class ProviderRunner:
    def __init__(self, response: str, *, fail_merge: bool = False) -> None:
        self.response, self.fail_merge = response, fail_merge
        self.calls: list[Invocation] = []

    def start(self, invocation: Invocation) -> RunningProcess:
        raise AssertionError("Auto-merge must not spawn background processes")

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        if "merge" in invocation.argv:
            if self.fail_merge:
                raise CommandError("Head changed or merge request rejected", 1)
            return Completed(0, "")
        return Completed(0, self.response)


def test_auto_merge_paginates_and_requests_protected_squash_for_each_head() -> None:
    runner = ProviderRunner("\n".join(json.dumps(item) for item in (request(), request(277))))
    context = make_context(Options())
    context.tools.github.runner = runner
    assert "2 Dependabot" in DependabotAutoMerge.run(context).message
    assert "--paginate" in runner.calls[0].argv
    assert ".[] | @json" in runner.calls[0].argv
    for number, call in zip((276, 277), runner.calls[1:], strict=True):
        assert call.argv[1:] == (
            "pr",
            "merge",
            str(number),
            "--repo",
            REPOSITORY.name,
            "--auto",
            "--squash",
            "--match-head-commit",
            HEAD,
        )


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("user", {"login": "human", "type": "User"}),
        ("user", {"login": "dependabot[bot]", "type": "User"}),
        ("draft", True),
        ("state", "closed"),
        ("auto_merge", {}),
        ("head", {"sha": HEAD, "repo": {"full_name": "fork/revaer"}}),
        ("head", {"sha": HEAD, "repo": None}),
        ("base", {"ref": "other", "repo": {"full_name": REPOSITORY.name}}),
        ("base", {"ref": "main", "repo": {"full_name": "other/revaer"}}),
    ),
)
def test_auto_merge_leaves_ineligible_or_already_enabled_prs_untouched(
    tmp_path: Path, field: str, value: object
) -> None:
    item = request()
    item[field] = value
    runner = ProviderRunner(json.dumps(item))
    assert GitHub(sys.executable, runner, tmp_path, {}).dependabot_pull_requests(REPOSITORY) == ()
    assert len(runner.calls) == 1


@pytest.mark.parametrize(
    ("field", "value"),
    (
        ("number", True),
        ("number", 0),
        ("number", "276"),
        ("head", {"sha": "bad", "repo": {"full_name": REPOSITORY.name}}),
    ),
)
def test_malformed_provider_identity_fails_closed(
    tmp_path: Path, field: str, value: object
) -> None:
    item = copy.deepcopy(request())
    item[field] = value
    runner = ProviderRunner(json.dumps(item))
    github = GitHub(sys.executable, runner, tmp_path, {})
    with pytest.raises(ToolingError):
        github.dependabot_pull_requests(REPOSITORY)
    assert len(runner.calls) == 1


def test_rejected_merge_is_propagated_without_retry_or_bypass(tmp_path: Path) -> None:
    runner = ProviderRunner("", fail_merge=True)
    github = GitHub(sys.executable, runner, tmp_path, {})
    args = AutoMergeArgs(REPOSITORY, 276, HEAD)
    with pytest.raises(CommandError, match="rejected"):
        github.enable_auto_merge(args)
    assert len(runner.calls) == 1


@pytest.mark.parametrize("response", ("not json", "{}", "[[{}]]"))
def test_provider_errors_are_not_treated_as_no_updates(tmp_path: Path, response: str) -> None:
    runner = ProviderRunner(response)
    github = GitHub(sys.executable, runner, tmp_path, {})
    with pytest.raises(ToolingError):
        github.dependabot_pull_requests(REPOSITORY)
