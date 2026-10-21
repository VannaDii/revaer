"""Exercise provider operations, exact-commit merges and asynchronous refreshes."""

import json
import sys
from dataclasses import replace
from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest
from revaer_tooling.automation.dependabot import REFRESH_MARKER, bot_owned, refresh_action
from revaer_tooling.cli import make_context, parser
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.github import AutoMergeArgs, GitHub
from revaer_tooling.external.release import ReleaseRepository
from revaer_tooling.json_data import Json, JsonObject
from revaer_tooling.process import Completed, Invocation, RunningProcess
from revaer_tooling.tasks.dependabot import DependabotAutoMerge

REPOSITORY = ReleaseRepository("github.com", "VannaDii/Revaer")
HEAD, BASE, OTHER = "a" * 40, "b" * 40, "c" * 40
NOW = datetime(2026, 10, 21, 19, tzinfo=UTC)
LOGIN = "revaer-maintenance[bot]"


def request(number: int = 276) -> JsonObject:
    return {
        "number": number,
        "state": "open",
        "draft": False,
        "auto_merge": None,
        "commits": 1,
        "changed_files": 1,
        "user": {"login": "dependabot[bot]", "type": "Bot"},
        "head": {
            "sha": HEAD,
            "ref": "dependabot/cargo/example",
            "repo": {"full_name": REPOSITORY.name},
        },
        "base": {"ref": "main", "repo": {"full_name": REPOSITORY.name}},
    }


def commit() -> JsonObject:
    return {
        "sha": HEAD,
        "author": {"login": "dependabot[bot]", "type": "Bot"},
        "commit": {"verification": {"verified": True}, "committer": {"date": NOW.isoformat()}},
    }


def review(login: str = LOGIN, state: str = "APPROVED", head: str = HEAD) -> JsonObject:
    return {"user": {"login": login, "type": "Bot"}, "state": state, "commit_id": head}


def comment(command: str, *, hours: int = 0, head: str = HEAD) -> JsonObject:
    return {
        "body": f"@dependabot {command}\n\n{REFRESH_MARKER}{head}:{BASE} -->",
        "author_association": "OWNER",
        "created_at": (NOW - timedelta(hours=hours)).isoformat(),
    }


class ProviderRunner:
    def __init__(self) -> None:
        self.item = request()
        self.items: list[JsonObject] | None = None
        self.commits = [commit()]
        self.files: list[JsonObject] = [{"filename": "Cargo.lock", "status": "modified"}]
        self.reviews: list[JsonObject] = []
        self.comments: list[JsonObject] = []
        self.behind = 0
        self.base = BASE
        self.run_status = "completed"
        self.conclusion = "success"
        self.run_head = HEAD
        self.checks: list[JsonObject] = [
            {"name": "Run Checks", "bucket": "pass", "state": "SUCCESS"}
        ]
        self.calls: list[Invocation] = []
        self.move_on_review = ""
        self.fail_merge = False
        self.fail_path = ""
        self.list_response: str | None = None

    def start(self, invocation: Invocation) -> RunningProcess:
        raise AssertionError("Dependabot must not spawn background processes")

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        args = invocation.argv[1:]
        if args[0] == "pr":
            return self.pr_command(args)
        path = next(value for value in args if value.startswith("repos/"))
        if self.fail_path and path.endswith(self.fail_path):
            raise CommandError("Provider unavailable", 1)
        return self.post(path) if "POST" in args else self.read(path)

    def pr_command(self, args: tuple[str, ...]) -> Completed:
        if args[1] == "checks":
            return Completed(0, json.dumps(self.checks))
        if self.fail_merge:
            raise CommandError("Head changed or merge request rejected", 1)
        self.item["auto_merge"] = None if "--disable-auto" in args else {}
        return Completed(0, "")

    def post(self, path: str) -> Completed:
        if path.endswith("/reviews"):
            self.reviews.append(review())
            if self.move_on_review == "head":
                self.item["head"] = {
                    "sha": OTHER,
                    "ref": "dependabot/cargo/example",
                    "repo": {"full_name": REPOSITORY.name},
                }
            elif self.move_on_review == "base":
                self.base = OTHER
            elif self.move_on_review == "ci":
                self.conclusion = "failure"
        return Completed(0, "{}")

    def read(self, path: str) -> Completed:
        if path.endswith("/pulls"):
            items = self.items if self.items is not None else [self.item]
            result = (
                self.list_response
                if self.list_response is not None
                else "\n".join(json.dumps(item) for item in items)
            )
            return Completed(0, result)
        if path.endswith("/git/ref/heads/main"):
            return Completed(0, json.dumps({"object": {"sha": self.base}}))
        if "/compare/" in path:
            return Completed(
                0, json.dumps({"base_commit": {"sha": self.base}, "behind_by": self.behind})
            )
        if path.endswith("/runs"):
            return Completed(
                0,
                json.dumps(
                    {
                        "workflow_runs": [
                            {
                                "head_sha": self.run_head,
                                "head_branch": "dependabot/cargo/example",
                                "event": "pull_request",
                                "path": ".github/workflows/pr.yml",
                                "status": self.run_status,
                                "conclusion": self.conclusion,
                            }
                        ]
                    }
                ),
            )
        for kind, records in (
            ("commits", self.commits),
            ("comments", self.comments),
            ("reviews", self.reviews),
            ("files", self.files),
        ):
            if path.endswith("/" + kind):
                return Completed(0, "\n".join(json.dumps(item) for item in records))
        item = dict(self.item)
        item["number"] = int(path.rsplit("/", 1)[1])
        return Completed(0, json.dumps(item))


def context(runner: ProviderRunner, *, apply: bool = True) -> Context:
    result = make_context(Options(dependabot_apply=apply))
    result.tools.github.name = sys.executable
    result.tools.github.runner = runner
    result.tools.github.environment = {
        "GH_TOKEN": "app-credential",
        "REVAER_DEPENDABOT_REFRESH_TOKEN": "user-credential",
    }
    workflow = replace(
        result.settings.workflow,
        ref="refs/heads/main",
        ref_protected=True,
        event_name="workflow_run",
        dependabot_enabled=True,
        dependabot_app_login=LOGIN,
    )
    return replace(result, settings=replace(result.settings, workflow=workflow), invoked_at=NOW)


def mutations(runner: ProviderRunner) -> list[Invocation]:
    return [call for call in runner.calls if "POST" in call.argv or ("merge" in call.argv)]


def test_current_successful_update_is_approved_before_exact_head_protected_merge() -> None:
    runner = ProviderRunner()
    result = DependabotAutoMerge.run(context(runner))
    assert "Reconciled 1" in result.message
    calls = mutations(runner)
    assert len(calls) == 2
    assert json.loads(calls[0].input_text or "{}")["commit_id"] == HEAD
    assert json.loads(calls[0].input_text or "{}")["event"] == "APPROVE"
    assert calls[1].argv[1:] == (
        "pr",
        "merge",
        "276",
        "--repo",
        REPOSITORY.name,
        "--auto",
        "--squash",
        "--match-head-commit",
        HEAD,
    )
    assert "--paginate" in runner.calls[0].argv
    assert all(
        "--paginate" in call.argv for call in runner.calls if call.argv[-1] == "per_page=100"
    )


def test_existing_current_app_approval_and_merge_request_are_idempotent() -> None:
    runner = ProviderRunner()
    runner.item["auto_merge"] = {}
    runner.reviews = [review()]
    DependabotAutoMerge.run(context(runner))
    assert not mutations(runner)


@pytest.mark.parametrize(
    ("status", "conclusion"),
    [("in_progress", None), ("completed", "failure"), ("completed", "cancelled")],
)
def test_pending_or_failed_latest_run_cannot_reuse_an_earlier_success(
    status: str, conclusion: str | None
) -> None:
    runner = ProviderRunner()
    runner.run_status, runner.conclusion = status, conclusion or ""
    DependabotAutoMerge.run(context(runner))
    assert not mutations(runner)


def test_validation_for_another_head_is_rejected() -> None:
    runner = ProviderRunner()
    runner.run_head = OTHER
    with pytest.raises(ToolingError, match="different head"):
        DependabotAutoMerge.run(context(runner))
    assert not mutations(runner)


@pytest.mark.parametrize("bucket", ["fail", "pending", "cancel"])
def test_required_checks_must_also_allow_merge(bucket: str) -> None:
    runner = ProviderRunner()
    runner.checks[0]["bucket"] = bucket
    DependabotAutoMerge.run(context(runner))
    assert not mutations(runner)


@pytest.mark.parametrize("move", ["head", "base", "ci"])
def test_state_changes_after_review_prevent_merge(move: str) -> None:
    runner = ProviderRunner()
    runner.move_on_review = move
    DependabotAutoMerge.run(context(runner))
    assert len(mutations(runner)) == 1


def test_nonconflicting_behind_branch_requests_rebase_with_user_token() -> None:
    runner = ProviderRunner()
    runner.behind = 1
    runner.item["auto_merge"] = {}
    DependabotAutoMerge.run(context(runner))
    calls = mutations(runner)
    assert "--disable-auto" in calls[0].argv
    assert json.loads(calls[1].input_text or "{}")["body"].startswith("@dependabot rebase")
    assert calls[1].env["GH_TOKEN"] == "user-credential"
    assert all(
        call.env.get("GH_TOKEN") == "app-credential"
        for call in runner.calls
        if call is not calls[1]
    )


@pytest.mark.parametrize(
    ("command", "hours", "expected"),
    [("rebase", 0, None), ("rebase", 2, "recreate"), ("recreate", 0, None)],
)
def test_outstanding_refresh_is_deduplicated_and_recovers_once(
    command: str, hours: int, expected: str | None
) -> None:
    runner = ProviderRunner()
    runner.behind = 2
    runner.comments = [comment(command, hours=hours)]
    DependabotAutoMerge.run(context(runner))
    calls = mutations(runner)
    assert len(calls) == (1 if expected else 0)
    if expected:
        assert json.loads(calls[0].input_text or "{}")["body"].startswith("@dependabot " + expected)


def test_stalled_recreation_fails_without_retrying_or_skipping_other_prs() -> None:
    runner = ProviderRunner()
    runner.behind = 1
    runner.comments = [comment("recreate", hours=2)]
    runner.items = [request(), request(277)]
    with pytest.raises(ToolingError, match="maintainer action"):
        DependabotAutoMerge.run(context(runner))
    assert not mutations(runner)
    assert any("repos/VannaDii/Revaer/pulls/277" in call.argv for call in runner.calls)


@pytest.mark.parametrize("change", ["human", "unverified", "missing", "requested"])
def test_human_work_and_uncertain_ownership_prevent_recreation_and_approval(change: str) -> None:
    runner = ProviderRunner()
    runner.behind = 1
    runner.comments = [comment("rebase", hours=2)]
    if change == "human":
        runner.commits[0]["author"] = {"login": "maintainer", "type": "User"}
    elif change == "unverified":
        runner.commits[0]["commit"] = {"verification": {"verified": False}}
    elif change == "missing":
        runner.commits = []
    else:
        runner.reviews = [review("maintainer", "CHANGES_REQUESTED")]
    with pytest.raises(ToolingError, match="maintainer action"):
        DependabotAutoMerge.run(context(runner))
    assert not mutations(runner)


@pytest.mark.parametrize("behind", [0, 1])
def test_dry_run_never_changes_a_pr(behind: int) -> None:
    runner = ProviderRunner()
    runner.behind = behind
    DependabotAutoMerge.run(context(runner, apply=False))
    assert not mutations(runner)


@pytest.mark.parametrize(
    "field", ["dependabot_enabled", "ref_protected", "ref", "event_name", "dependabot_app_login"]
)
def test_mutations_require_enabled_trusted_main_and_app_identity(field: str) -> None:
    runner = ProviderRunner()
    active = context(runner)
    workflow = active.settings.workflow
    if field == "dependabot_enabled":
        workflow = replace(workflow, dependabot_enabled=False)
    elif field == "ref_protected":
        workflow = replace(workflow, ref_protected=False)
    elif field == "ref":
        workflow = replace(workflow, ref="refs/heads/feature")
    elif field == "event_name":
        workflow = replace(workflow, event_name="pull_request")
    else:
        workflow = replace(workflow, dependabot_app_login="")
    active = replace(active, settings=replace(active.settings, workflow=workflow))
    with pytest.raises(ToolingError, match="protected main"):
        DependabotAutoMerge.run(active)
    assert not runner.calls


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("user", {"login": "human", "type": "User"}),
        ("user", {"login": "dependabot[bot]", "type": "User"}),
        ("draft", True),
        ("state", "closed"),
        ("head", {"sha": HEAD, "repo": {"full_name": "fork/revaer"}}),
        ("head", {"sha": HEAD, "repo": None}),
        ("base", {"ref": "other", "repo": {"full_name": REPOSITORY.name}}),
    ],
)
def test_ineligible_requests_are_untouched(field: str, value: Json) -> None:
    runner = ProviderRunner()
    runner.item[field] = value
    DependabotAutoMerge.run(context(runner))
    assert len(runner.calls) == 1


@pytest.mark.parametrize("response", ["not json", "{}", "[[{}]]", '{"number":1,"number":2}'])
def test_malformed_provider_output_is_not_no_updates(response: str) -> None:
    runner = ProviderRunner()
    runner.list_response = response
    with pytest.raises(ToolingError):
        DependabotAutoMerge.run(context(runner))


@pytest.mark.parametrize("path", ["/commits", "/reviews", "/runs"])
def test_provider_errors_propagate_without_mutations(path: str) -> None:
    runner = ProviderRunner()
    runner.fail_path = path
    with pytest.raises(ToolingError, match="unavailable"):
        DependabotAutoMerge.run(context(runner))
    assert not mutations(runner)


def test_missing_refresh_credential_fails_without_posting() -> None:
    runner = ProviderRunner()
    runner.behind = 1
    active = context(runner)
    active.tools.github.environment = {"GH_TOKEN": "app-credential"}
    with pytest.raises(ToolingError, match="REFRESH_TOKEN"):
        DependabotAutoMerge.run(active)
    assert not mutations(runner)


def test_rejected_merge_has_no_bypass_or_retry(tmp_path: Path) -> None:
    runner = ProviderRunner()
    runner.fail_merge = True
    github = GitHub(sys.executable, runner, tmp_path, {})
    with pytest.raises(CommandError, match="rejected"):
        github.enable_auto_merge(AutoMergeArgs(REPOSITORY, 276, HEAD))
    assert len(runner.calls) == 1


def test_refresh_observes_manual_commands_and_ignores_other_heads() -> None:
    manual = comment("rebase")
    manual["body"] = "@dependabot rebase"
    assert refresh_action((manual,), HEAD, NOW, NOW) == "wait"
    assert refresh_action((comment("recreate", hours=3, head=OTHER),), HEAD, NOW, NOW) == "rebase"
    outsider = comment("recreate", hours=3)
    outsider["author_association"] = "NONE"
    assert refresh_action((outsider,), HEAD, NOW, NOW) == "rebase"


def test_partial_commit_inventory_and_cli_scope_fail_closed() -> None:
    assert not bot_owned((commit(),), 2, HEAD)
    parsed = parser().parse_args(["dependabot-auto-merge", "--apply", "--pr-number", "276"])
    assert parsed.dependabot_apply and parsed.pull_request == 276
    with pytest.raises(SystemExit):
        parser().parse_args(["ci-qualify", "--apply"])


def test_paginated_selection_keeps_each_provider_pr_identity() -> None:
    runner = ProviderRunner()
    runner.items = [request(), request(277)]
    active = context(runner, apply=False)
    selected = active.tools.github.dependabot_pull_requests(REPOSITORY)
    assert [item.number for item in selected] == [276, 277]
    assert "--paginate" in runner.calls[0].argv
    assert "per_page=100" in runner.calls[0].argv


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("number", True),
        ("number", 0),
        ("number", "276"),
        ("head", {"sha": "bad", "repo": {"full_name": REPOSITORY.name}}),
    ],
)
def test_malformed_pr_identity_fails_before_any_mutation(field: str, value: Json) -> None:
    runner = ProviderRunner()
    runner.item[field] = value
    with pytest.raises(ToolingError):
        DependabotAutoMerge.run(context(runner))
    assert not mutations(runner)


def test_previous_head_approval_is_replaced_and_manual_target_is_respected() -> None:
    runner = ProviderRunner()
    runner.items = [request(), request(277)]
    runner.reviews = [review(head=OTHER)]
    active = context(runner)
    active = replace(active, options=replace(active.options, pull_request=277))
    DependabotAutoMerge.run(active)
    calls = mutations(runner)
    assert len(calls) == 2
    assert any("/277/reviews" in argument for argument in calls[0].argv)
    assert json.loads(calls[0].input_text or "{}")["commit_id"] == HEAD
    assert calls[1].argv[3] == "277"


@pytest.mark.parametrize("filename", ["vendor/yew/Cargo.toml", "tools/src/revaer_tooling/cli.py"])
def test_unexpected_file_changes_pause_approval_and_destructive_refresh(filename: str) -> None:
    runner = ProviderRunner()
    runner.files[0]["filename"] = filename
    runner.behind = 1
    runner.comments = [comment("rebase", hours=2)]
    with pytest.raises(ToolingError, match="maintainer action"):
        DependabotAutoMerge.run(context(runner))
    assert not mutations(runner)
