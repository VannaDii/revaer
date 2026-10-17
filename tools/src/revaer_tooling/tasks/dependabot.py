"""Request native auto-merge while GitHub enforces the complete protected branch gates."""

from ..context import Context, TaskResult
from .base import Task
from .release import release_repository


class DependabotAutoMerge(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        repository = release_repository(context)
        requests = context.tools.github.dependabot_pull_requests(repository)
        for request in requests:
            context.tools.github.enable_auto_merge(request)
            context.emit(
                f"Requested protected squash auto-merge for Dependabot PR #{request.number}"
            )
        return TaskResult(f"Requested auto-merge for {len(requests)} Dependabot pull requests")
