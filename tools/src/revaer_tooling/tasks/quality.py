"""Compose the foundation's complete validation and release-build gates.

The command registry remains the public surface. This sequence reuses static
task methods, preserving their diagnostics and stopping at the first failure.
"""

from collections.abc import Callable
from dataclasses import replace

from ..context import Context, TaskResult
from ..e2e.database import single_init_database, uses_single_init
from ..errors import ToolingError
from ..external.database import with_database
from .base import Task
from .build import (
    Audit,
    BuildRelease,
    CheckAssets,
    Deny,
    Fmt,
    ToolingCoverage,
    Udeps,
    UiBuild,
)
from .charts import HelmLint
from .coverage import Coverage
from .database import DatabaseStart, database_connection
from .database_probes import DatabaseBaselineVerify
from .e2e import UiE2e, UiE2eCoverage
from .policy import InstructionDrift, Lint
from .testing import Test, TestFeaturesMinimal

VALIDATION_STEPS: dict[str, Callable[[Context], TaskResult]] = {
    "db-baseline-verify": DatabaseBaselineVerify.run,
    "fmt": Fmt.run,
    "lint": Lint.run,
    "helm-lint": HelmLint.run,
    "instruction-drift": InstructionDrift.run,
    "check-assets": CheckAssets.run,
    "udeps": Udeps.run,
    "audit": Audit.run,
    "deny": Deny.run,
    "ui-build": UiBuild.run,
    "test": Test.run,
    "test-features-min": TestFeaturesMinimal.run,
    "cov": Coverage.run,
    "tooling-cov": ToolingCoverage.run,
}


class Validate(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        if uses_single_init(context):
            with single_init_database(context) as url:
                Validate.run_steps(context, url)
            return TaskResult("Validation gates passed")
        DatabaseStart.run(context)
        # Preserve the normalized local endpoint and prevent a concurrent managed
        # database reset throughout validation. Explicit test overrides remain
        # explicit; the managed default supplies its own administrative endpoint.
        with database_connection(context) as url:
            Validate.run_steps(context, url)
        return TaskResult("Validation gates passed")

    @staticmethod
    def run_steps(context: Context, url: str) -> None:
        database = replace(
            context.settings.database,
            url=url,
            test_url=context.settings.database.test_url
            or context.settings.e2e.admin_url
            or with_database(url, "postgres"),
        )
        active = replace(context, settings=replace(context.settings, database=database))
        for name, execute in VALIDATION_STEPS.items():
            context.emit("Validation: " + name)
            result = execute(active)
            if result.message:
                context.emit(result.message)


class Ci(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        Validate.run(context)
        BuildRelease.run(context)
        return TaskResult("Validation and release build passed")


class Lock(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.uv.lock()
        return TaskResult("Project dependencies locked with uv")


class CargoLock(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.lock_workspace()
        return TaskResult("Cargo workspace lock reconciled without registry upgrades")


class CiQualify(Task):
    """Run local CI and API/browser gates without a PR or hosted Sonar scan."""

    @staticmethod
    def run(context: Context) -> TaskResult:
        e2e = context.settings.e2e
        if (
            e2e.shard_total != 1
            or e2e.retries != 0
            or not e2e.browser_coverage
            or set(e2e.phases(media=uses_single_init(context)))
            != {
                "api-none",
                "api-api-key",
                "ui-chromium",
                "ui-firefox",
                "ui-webkit",
                *(
                    ("api-none-missing-catalog", "api-api-key-missing-catalog")
                    if uses_single_init(context)
                    else ()
                ),
            }
        ):
            raise ToolingError(
                "ci-qualify requires all API/browser phases, coverage and no retries"
            )
        if (context.root / "coverage/producers").exists():
            raise ToolingError(
                "ci-qualify requires local inputs, not downloaded producer artifacts"
            )
        for execute in (Ci.run, UiE2e.run, UiE2eCoverage.run):
            result = execute(context)
            if result.message:
                context.emit(result.message)
        return TaskResult(
            "Local CI and API/browser qualification passed; local Sonar review is separate"
        )
