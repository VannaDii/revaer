"""Compose the foundation's complete validation and release-build gates.

The command registry remains the public surface. This sequence reuses static
task methods, preserving their diagnostics and stopping at the first failure.
Media-specific gates must be integrated alongside that stack's task ports; see
tools/migration.md for the separate integration acceptance requirements.
"""

from collections.abc import Callable
from dataclasses import replace

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.database import with_database
from .base import Task
from .bootstrap import ScriptCoverage
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
from .e2e import UiE2e, UiE2eCoverage
from .javascript_coverage import JavaScriptCoverageMerge
from .native import SonarCompileDatabase
from .policy import InstructionDrift, Lint
from .python_coverage import PythonCoverageMerge
from .sonar import (
    SonarPackageReport,
    SonarPrepareScm,
    SonarPrepareSources,
    SonarScan,
    SonarVerifyInputs,
    SonarVerifyResult,
)
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


class CiQualify(Task):
    """Run complete local qualification before pushing a pull-request snapshot."""

    @staticmethod
    def run(context: Context) -> TaskResult:
        sonar = context.settings.sonar
        if (
            not sonar.scanner_token
            or not sonar.auth_token
            or not sonar.pull_request
            or not sonar.head_ref
        ):
            raise ToolingError("ci-qualify requires Sonar credentials and SONAR_PULL_REQUEST")
        if not sonar.native_required:
            raise ToolingError("ci-qualify requires REVAER_REQUIRE_NATIVE_COVERAGE=1")
        e2e = context.settings.e2e
        if (
            e2e.shard_total != 1
            or e2e.retries != 0
            or not e2e.browser_coverage
            or set(e2e.phases())
            != {"api-none", "api-api-key", "ui-chromium", "ui-firefox", "ui-webkit"}
        ):
            raise ToolingError(
                "ci-qualify requires all API/browser phases, coverage and no retries"
            )
        if (context.root / "coverage/producers").exists():
            raise ToolingError(
                "ci-qualify requires local inputs, not downloaded producer artifacts"
            )
        context.tools.kcov.verify()
        SonarPrepareScm.run(context)
        for execute in (
            Ci.run,
            UiE2e.run,
            UiE2eCoverage.run,
            ScriptCoverage.run,
            PythonCoverageMerge.run,
            JavaScriptCoverageMerge.run,
            SonarCompileDatabase.run,
            SonarVerifyInputs.run,
            SonarPrepareSources.run,
            SonarScan.run,
            SonarPackageReport.run,
            SonarVerifyResult.run,
        ):
            result = execute(context)
            if result.message:
                context.emit(result.message)
        return TaskResult("Complete local CI, API/browser coverage and Sonar qualification passed")
