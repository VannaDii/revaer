"""Reconcile bot branches, current validation and protected native squash merges."""

import re

from ..automation.dependabot import (
    bot_owned,
    dependency_files,
    refresh_action,
    reviews_state,
    timestamp,
)
from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.github import AutoMergeArgs
from ..json_data import JsonObject, object_value, string_value
from .base import Task
from .release import release_repository

STATE_CHANGED = "head or base changed; deferred"


def unchanged(context: Context, args: AutoMergeArgs, base: str) -> JsonObject | None:
    github = context.tools.github
    item = github.dependency_request(args)
    if AutoMergeArgs.from_pull_request(args.repository, item) != args:
        return None
    if github.dependency_base(args.repository) != base:
        return None
    return item


def disarm_queue(context: Context, args: AutoMergeArgs, base: str) -> bool:
    apply = context.options.dependabot_apply
    if apply:
        if unchanged(context, args, base) is None:
            return False
        context.tools.github.disable_dependency_merge(args)
    context.emit(f"PR #{args.number}: {'disabled' if apply else 'would disable'} old auto-merge")
    return True


def request_refresh(
    context: Context, args: AutoMergeArgs, base: str, commits: tuple[JsonObject, ...]
) -> str:
    github = context.tools.github
    head_commit = object_value(commits[-1].get("commit"))
    head_time = timestamp(object_value(head_commit.get("committer")).get("date"))
    action = refresh_action(
        github.dependency_records(args, "comments"), args.head, head_time, context.invoked_at
    )
    if action == "wait":
        return "refresh pending"
    if unchanged(context, args, base) is None:
        return STATE_CHANGED
    if context.options.dependabot_apply:
        github.refresh_dependency(args, base, action)
    return f"{'requested' if context.options.dependabot_apply else 'would request'} {action}"


def request_merge(
    context: Context, args: AutoMergeArgs, base: str, branch: str, approved: bool, queued: bool
) -> str:
    github = context.tools.github
    if not github.dependency_ci_passed(args, branch):
        # A same-head rerun must not retain a previously queued merge.
        if queued and not disarm_queue(context, args, base):
            return STATE_CHANGED
        return "waiting for successful current validation"
    if unchanged(context, args, base) is None:
        return STATE_CHANGED
    if not context.options.dependabot_apply:
        return "would approve current head and request protected auto-merge"
    if not approved:
        github.approve_dependency(args)
    latest = unchanged(context, args, base)
    if latest is None or not github.dependency_ci_passed(args, branch):
        return "head, base or validation changed after review; deferred"
    if latest.get("auto_merge") is None:
        github.enable_auto_merge(args)
    return "current head approved; protected merge requested"


def reconcile(context: Context, initial: AutoMergeArgs, login: str) -> str:
    github = context.tools.github
    item = github.dependency_request(initial)
    args = AutoMergeArgs.from_pull_request(initial.repository, item)
    if args is None:
        return "closed or no longer eligible"
    if args.number != initial.number:
        raise ToolingError("Dependabot returned a different pull request")
    base = github.dependency_base(args.repository)
    commits = github.dependency_records(args, "commits")
    owned = bot_owned(commits, item.get("commits"), args.head) and dependency_files(
        github.dependency_records(args, "files"), item.get("changed_files")
    )
    approved, changes_requested = reviews_state(
        github.dependency_records(args, "reviews"), login, args.head
    )
    behind = github.dependency_behind(args, base)
    queued = item.get("auto_merge") is not None
    if queued and (not owned or changes_requested or behind or not approved):
        if not disarm_queue(context, args, base):
            return STATE_CHANGED
        queued = False
    if not owned or changes_requested:
        raise ToolingError(
            "Human edits, unexpected files, uncertain ownership or requested changes; "
            "maintainer action required"
        )
    if behind:
        return request_refresh(context, args, base, commits)
    branch = string_value(object_value(item.get("head")).get("ref"))
    return request_merge(context, args, base, branch, approved, queued)


class DependabotAutoMerge(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        workflow = context.settings.workflow
        login = workflow.dependabot_app_login
        if context.options.dependabot_apply and (
            not workflow.dependabot_enabled
            or workflow.ref != "refs/heads/main"
            or not workflow.ref_protected
            or workflow.event_name not in ("push", "workflow_run", "schedule", "workflow_dispatch")
            or re.fullmatch(r"[a-z0-9-]+\[bot\]", login) is None
        ):
            raise ToolingError(
                "Dependabot mutations require enabled, protected main automation "
                "and its App identity"
            )
        number = (
            context.options.pull_request
            if context.options.pull_request is not None
            else workflow.dependabot_number
        )
        if number is not None and number < 1:
            raise ToolingError("Dependabot PR number must be positive")
        repository = release_repository(context)
        requests = context.tools.github.dependabot_pull_requests(repository)
        failures: list[str] = []
        processed = 0
        for request in requests:
            if number is not None and request.number != number:
                continue
            processed += 1
            try:
                result = reconcile(context, request, login)
                context.emit(f"PR #{request.number}: {result}")
            except ToolingError as error:
                # One stuck dependency must not prevent refreshing other PRs.
                message = f"PR #{request.number}: {error}"
                failures.append(message)
        if failures:
            raise ToolingError("Dependabot maintenance failed:\n" + "\n".join(failures))
        mode = "Reconciled" if context.options.dependabot_apply else "Dry-run: inspected"
        return TaskResult(f"{mode} {processed} Dependabot pull requests")
