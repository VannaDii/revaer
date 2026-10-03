"""Compose the foundation's complete validation and release-build gates.

The command registry remains the public surface. This sequence reuses static
task methods, preserving their diagnostics and stopping at the first failure.
Media-specific gates must be integrated alongside that stack's task ports; see
tools/migration.md for the separate integration acceptance requirements.
"""

from collections.abc import Callable
from dataclasses import replace

from ..context import Context, TaskResult
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
from .policy import InstructionDrift, Lint
from .testing import Test, TestFeaturesMinimal

VALIDATION_STEPS: dict[str, Callable[[Context], TaskResult]] = {
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
        DatabaseStart.run(context)
        # Preserve the normalized local endpoint and prevent a concurrent managed
        # database reset throughout validation. Explicit test overrides remain
        # explicit; the managed default supplies its own administrative endpoint.
        with database_connection(context) as url:
            database = replace(
                context.settings.database,
                url=url,
                test_url=context.settings.database.test_url or with_database(url, "postgres"),
            )
            active = replace(context, settings=replace(context.settings, database=database))
            for name, execute in VALIDATION_STEPS.items():
                context.emit("Validation: " + name)
                result = execute(active)
                if result.message:
                    context.emit(result.message)
        return TaskResult("Validation gates passed")


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
