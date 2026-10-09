"""Rust test variants preserve their feature sets and database overrides.

These commands use the caller's available database. Provisioning belongs to the
database/CI lifecycle task, so targeted test runs do not replace a running server.
"""

import re

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.rust import AppRegression, CargoArgs, CargoOperation
from .base import Task


def test_environment(context: Context) -> dict[str, str]:
    database = context.settings.database
    if database.test_url is None:
        raise ToolingError(
            "Set REVAER_TEST_DATABASE_URL or DATABASE_URL to a disposable test database"
        )
    return {
        "REVAER_TEST_DATABASE_URL": database.test_url,
        "DATABASE_URL": database.url,
    }


class Test(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.execute(CargoArgs(CargoOperation.TEST), test_environment(context))
        return TaskResult()


class TestNative(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.cargo.execute(
            CargoArgs(CargoOperation.TEST, ("revaer-torrent-libt",), test_threads=1),
            {**test_environment(context), "REVAER_NATIVE_IT": "1"},
        )
        return TaskResult()


class TestFeaturesMinimal(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        for package in ("revaer-api", "revaer-app"):
            context.tools.cargo.execute(
                CargoArgs(
                    CargoOperation.TEST,
                    (package,),
                    all_features=False,
                    no_default_features=True,
                ),
                test_environment(context),
            )
        return TaskResult()


class TestRuntimeShutdown(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        for minimal in (False, True):
            result = context.tools.cargo.shutdown_tests(minimal)
            context.emit(result.stdout)
        return TaskResult("Runtime shutdown checks passed in both feature modes")


class LintRuntimeShutdown(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        for production in (False, True):
            for minimal in (False, True):
                context.tools.cargo.clippy(production, packages=("revaer-app",), minimal=minimal)
        return TaskResult("Runtime shutdown Clippy checks passed in both feature modes")


class UiE2eAppTest(Task):
    """Exercise launch guards and production compliance without starting E2E."""

    @staticmethod
    def run(context: Context) -> TaskResult:
        environment = test_environment(context)
        for group in AppRegression:
            result = context.tools.cargo.app_regressions(group, environment)
            context.emit(result.stdout)
        return TaskResult("Application launch, compliance and bootstrap regressions passed")


class TestMediaRecovery(Task):
    """Exercise interruption/replay and the private workspace reuse boundary."""

    @staticmethod
    def run(context: Context) -> TaskResult:
        selections: tuple[CargoArgs, ...] = (
            CargoArgs(
                CargoOperation.TEST,
                ("revaer-app",),
                test_filter="production_media_job_runtime_executes_and_persists_verified_replacement",
                include_ignored=True,
                test_threads=1,
            ),
            CargoArgs(
                CargoOperation.TEST,
                ("revaer-app",),
                all_features=False,
                no_default_features=True,
                test_filter="media_discovery_fingerprint::tests",
                test_threads=1,
            ),
            CargoArgs(
                CargoOperation.TEST,
                ("revaer-app",),
                all_features=False,
                no_default_features=True,
                test_filter="media::",
                test_threads=1,
            ),
            CargoArgs(CargoOperation.TEST, ("revaer-media-runtime",)),
            CargoArgs(CargoOperation.TEST, ("revaer-data",), test_filter="media::"),
            CargoArgs(
                CargoOperation.TEST,
                ("revaer-app",),
                all_features=False,
                no_default_features=True,
                test_filter="media_job_runtime::tests::media_job_runtime_",
                test_threads=1,
            ),
        )
        if context.host.system == "linux":
            selections += (
                CargoArgs(
                    CargoOperation.TEST,
                    ("revaer-app",),
                    all_features=False,
                    no_default_features=True,
                    test_filter="bootstrap::root_catalog::native::tests",
                    test_threads=1,
                ),
            )
        for selection in selections:
            result = context.tools.cargo.execute(selection, test_environment(context), capture=True)
            context.emit(result.stdout)
            if not re.search(
                r"(?m)^test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;", result.stdout
            ):
                raise ToolingError("Media recovery regression selection ran no passing tests")
        return TaskResult()


class TestMediaServiceRecovery(Task):
    """Run the explicit persistent-mount, active-service Linux qualification."""

    @staticmethod
    def run(context: Context) -> TaskResult:
        if context.host.system != "linux":
            raise ToolingError("Native media service recovery requires Linux")
        result = context.tools.cargo.execute(
            CargoArgs(
                CargoOperation.TEST,
                ("revaer-app",),
                test_filter="bootstrap::service_recovery_tests::native_service_shutdown_resumes_active_ffmpeg",
                include_ignored=True,
                test_threads=1,
            ),
            test_environment(context),
            capture=True,
        )
        context.emit(result.stdout)
        if not re.search(r"(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;", result.stdout):
            raise ToolingError(
                "Native service recovery qualification did not execute one passing test"
            )
        return TaskResult("Native media service recovery passed")
