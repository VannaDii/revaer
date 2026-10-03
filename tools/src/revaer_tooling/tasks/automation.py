"""Small workflow operations with the same local CLI and typed context."""

import json
import re

from ..automation.files import write_values
from ..automation.settings import SUPPLY_CHAIN_RESULTS
from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.github import PullRequestArgs
from ..images.model import IMAGE_MATRIX
from ..json_data import decode, object_value
from .base import Task
from .charts import validate_app_version, validate_version
from .release import STABLE_TAG, release_repository


def outputs(context: Context, values: dict[str, str]) -> TaskResult:
    destination = context.settings.workflow.output
    if destination is not None:
        write_values(context.fs, destination, values)
    return TaskResult(json.dumps(values, indent=2))


def source_values(context: Context) -> dict[str, str]:
    source = context.tools.git.revision()
    return {"sha": source, "short_sha": source[:7]}


class WorkflowMetadata(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        # Use the actual checkout. GITHUB_SHA can identify a merge commit even
        # when the job intentionally checked out the reviewed PR head instead.
        return outputs(context, source_values(context))


class WorkflowChartVersions(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        chart, app = context.options.chart_version, context.options.app_version
        if chart:
            validate_version(chart)
        if app:
            validate_app_version(app)
        run = context.settings.workflow
        number = context.options.pull_request
        if number is not None and number < 1:
            raise ToolingError("Pull request number must be positive")
        if number is None and run.ref_type == "branch" and run.ref_name:
            number = context.tools.github.open_pull_request(
                PullRequestArgs(release_repository(context), run.ref_name)
            )
        if not chart:
            if number is None:
                raise ToolingError("No open pull request found; provide --chart-version explicitly")
            if not re.fullmatch(r"[1-9][0-9]*", run.run_number):
                raise ToolingError("Default chart versions require a positive GITHUB_RUN_NUMBER")
            chart = f"0.0.0-dev.pr{number}.{run.run_number}"
        source = source_values(context)
        if not app:
            app = (
                f"pr-{number}-{source['short_sha']}" if number else f"verify-{source['short_sha']}"
            )
        return outputs(
            context,
            {
                "pr_number": str(number) if number else "",
                "chart_version": validate_version(chart),
                "app_version": validate_app_version(app),
            },
        )


class WorkflowMatrix(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        matrix = object_value(
            decode(context.fs.read(context.root / ".github/matrices/build-images.json"))
        )
        if matrix != IMAGE_MATRIX:
            raise ToolingError("Image matrix does not match the audited architecture contract")
        if context.options.release_matrix:
            run = context.settings.workflow
            if run.ref_type not in ("branch", "tag") or not run.ref_name:
                raise ToolingError(
                    "Release matrix selection requires the workflow ref type and name"
                )
            eligible = (run.ref_type == "branch" and run.ref_name == "main") or (
                run.ref_type == "tag" and re.fullmatch(STABLE_TAG, run.ref_name) is not None
            )
            if not eligible:
                matrix = {"include": []}
        return outputs(
            context, {**source_values(context), "matrix": json.dumps(matrix, separators=(",", ":"))}
        )


class WorkflowReport(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        report = context.options.report
        if report is None:
            raise ToolingError("Select a workflow report to publish")
        report = context.root / report
        if report.is_symlink() or not report.resolve().is_relative_to(context.root.resolve()):
            raise ToolingError("Workflow reports must belong to the current checkout")
        content = context.fs.read(report)
        if not content.strip():
            raise ToolingError("Workflow report must be nonempty")
        destination = context.settings.workflow.summary
        if destination is not None:
            context.fs.append(destination, content.rstrip() + "\n")
        return TaskResult(content)


class VerifySupplyChainResults(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        if (
            tuple(name for name, _ in context.settings.workflow.supply_chain_results)
            != SUPPLY_CHAIN_RESULTS
        ):
            raise ToolingError(
                "Supply-chain results must include audit, deny and unused dependencies"
            )
        failures = []
        for name, result in context.settings.workflow.supply_chain_results:
            if result != "success":
                state = (
                    result
                    if result in ("failure", "cancelled", "skipped")
                    else "missing or invalid result"
                )
                failures.append(f"{name}: {state}")
        if failures:
            raise ToolingError("Supply-chain prerequisites failed:\n" + "\n".join(failures))
        return TaskResult("Audit, deny and unused-dependency checks passed")
