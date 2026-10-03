"""Python tools always run from the selected checkout's locked environment."""

from collections.abc import Mapping
from dataclasses import dataclass, replace
from pathlib import Path

from ..e2e.settings import E2eSettings
from ..errors import ToolingError
from ..json_data import array_value, decode_unique, object_value, string_value
from ..process import Completed, Invocation
from .base import ExternalTool


@dataclass(frozen=True)
class PytestArgs:
    paths: tuple[str, ...]
    coverage: bool = False


@dataclass(frozen=True)
class E2ePytestArgs:
    phase: str
    settings: E2eSettings
    results: Path
    report: Path
    coverage: Path
    include_media: bool = False


@dataclass(frozen=True)
class PythonCoverageArgs:
    inputs: tuple[Path, ...]
    sources: tuple[str, ...]
    output: Path


class Python(ExternalTool):
    def merge_coverage(self, args: PythonCoverageArgs) -> None:
        """Use Coverage.py's supported merge/report commands without rewriting hits.

        Keep original data files as evidence. Listing all authored sources also
        makes never-imported files visible with zero execution in the report.
        """
        data = str(args.output / ".coverage")
        self._invoke(
            (
                "-m",
                "coverage",
                "combine",
                "--keep",
                "--data-file",
                data,
                *(str(path) for path in args.inputs),
            ),
            # Coverage.py otherwise warns and skips corrupt input databases.
            # Every supplied phase is required evidence, so that must fail.
            env={"PYTHONWARNINGS": "error"},
        )
        for format in ("xml", "lcov"):
            self._invoke(
                (
                    "-m",
                    "coverage",
                    format,
                    "--data-file",
                    data,
                    "--rcfile",
                    str(self.root / "tools/coverage-report.toml"),
                    "-o",
                    str(args.output / ("python." + format)),
                    *args.sources,
                )
            )

    def e2e(self, args: E2ePytestArgs, env: Mapping[str, str]) -> Completed:
        settings = args.settings
        ui = args.phase.startswith("ui-")
        command = [
            "-m",
            "pytest",
            "tests/specs/ui" if ui else "tests/specs/api",
            *((f"tests/specs/media/{'ui' if ui else 'api'}",) if args.include_media else ()),
            "--reruns",
            str(settings.retries),
            "--timeout",
            str(settings.test_timeout_ms / 1000),
            "--timeout-method=signal",
            "--html",
            str(args.report / "index.html"),
            "--self-contained-html",
            "--junitxml",
            str(args.results / "junit.xml"),
            "--output",
            str(args.results),
            "--cov=tests",
            "--cov=tools/src/revaer_tooling/e2e",
            "--cov-report=xml:" + str(args.coverage / "python.xml"),
            "--cov-report=lcov:" + str(args.coverage / "python.lcov"),
            # A phase is partial evidence (anonymous/authenticated/browser).
            # The independent tooling runtime gate still enforces its 90% floor.
            "--cov-fail-under=0",
        ]
        if ui:
            command.extend(("--browser", args.phase.removeprefix("ui-")))
            if not settings.headless:
                command.append("--headed")
            if settings.browser_channel:
                command.extend(("--browser-channel", settings.browser_channel))
            if settings.workers > 1:
                command.extend(("--numprocesses", str(settings.workers)))
        return self._invoke(
            tuple(command),
            env={
                **env,
                "REVAER_E2E_PHASE": args.phase,
                "COVERAGE_FILE": str(args.coverage / ".coverage"),
            },
        )

    def pytest(self, args: PytestArgs, env: Mapping[str, str] | None = None) -> Completed:
        command = ["-m", "pytest", *args.paths]
        if args.coverage:
            command.extend(
                (
                    "--cov",
                    "--cov-report=term-missing",
                    "--cov-report=xml:coverage/python.xml",
                    "--cov-report=lcov:coverage/python.lcov",
                )
            )
        return self._invoke(tuple(command), env=env)

    def lint(self, fix: bool = False) -> Completed:
        return self._invoke(("-m", "ruff", "check", "tools", "tests", *(("--fix",) if fix else ())))

    def runtime_coverage(self) -> Completed:
        """Retain the runtime's floor independently of covered test scaffolding.

        The complete XML and LCOV reports include authored tests. This additional
        terminal-only gate prevents that larger denominator diluting the existing
        90% requirement on the implementation and installed launcher.
        """
        return self._invoke(
            (
                "-m",
                "coverage",
                "report",
                "--include=tools/src/revaer_tooling/*,tools/launcher/src/revaer_launcher/*",
            )
        )

    def format(self, fix: bool = False) -> Completed:
        return self._invoke(
            ("-m", "ruff", "format", "tools", "tests", *(() if fix else ("--check",)))
        )

    def types(self) -> Completed:
        return self._invoke(("-m", "mypy"))

    def audit(self, lock: Path) -> Completed:
        """Audit uv's standard lock export, including versioned source archives.

        Requirements exports lose source-package versions. PEP 751 keeps both
        the version used by advisory databases and uv's archive hash/provenance.
        A successful process must still prove that no dependency was skipped.
        """
        result = self._invoke(
            ("-m", "pip_audit", "--locked", "--strict", "--format", "json", str(lock.parent)),
            capture=True,
        )
        report = object_value(decode_unique(result.stdout))
        dependencies = array_value(report.get("dependencies"))
        if not dependencies:
            raise ToolingError("Python audit returned no dependencies")
        for item in dependencies:
            dependency = object_value(item)
            if (
                "skip_reason" in dependency
                or not string_value(dependency.get("name"))
                or not string_value(dependency.get("version"))
                or array_value(dependency.get("vulns"))
            ):
                raise ToolingError(
                    "Python audit contains skipped, invalid or vulnerable dependencies"
                )
        return result

    def install_browsers(self, browsers: tuple[str, ...], dependencies: bool) -> Completed:
        return self._invoke(
            ("-m", "playwright", "install", *(("--with-deps",) if dependencies else ()), *browsers)
        )


class Uv(ExternalTool):
    def limited_command(self, command: Invocation, python: Path, soft_limit: int) -> Completed:
        """Let uv apply a child-only Unix resource limit for a native command.

        No project environment is resolved for this native process. Explicit
        directory/project/interpreter arguments override foreign uv selectors
        while preserving their original environment values for bootstrap tests.
        """
        if (
            not command.argv
            or not Path(command.argv[0]).is_absolute()
            or not python.is_absolute()
            or soft_limit < 1
        ):
            raise ToolingError(
                "Limited native execution requires absolute tools and a positive limit"
            )
        return self.runner.run(
            replace(
                command,
                argv=(
                    str(self.locate()),
                    "run",
                    "--no-project",
                    "--no-config",
                    "--directory",
                    str(command.cwd),
                    "--project",
                    str(self.root),
                    "--python",
                    str(python),
                    "--",
                    *command.argv,
                ),
                env={**command.env, "UV_RUN_RLIMIT_NOFILE": str(soft_limit)},
            )
        )

    def e2e_environment(self, environment_file: Path, arguments: tuple[str, ...]) -> Completed:
        """Let uv load dotenv syntax and precedence before rv reads any settings."""
        return self._invoke(
            (
                "run",
                "--locked",
                "--env-file",
                str(environment_file),
                "--",
                "python",
                "-m",
                "revaer_tooling.cli",
                *arguments,
                "--e2e-environment-ready",
            )
        )

    def install_launcher(self, source: Path, python: str) -> Completed:
        """Install a snapshot, avoiding a global dependency on one live worktree.

        Reinstall refreshes changed local source; omitting --force lets uv reject
        collisions with an unrelated executable already named rv.
        """
        return self._invoke(("tool", "install", "--python", python, "--reinstall", str(source)))

    def export(self, destination: Path) -> Completed:
        return self._invoke(
            (
                "export",
                "--locked",
                "--all-groups",
                "--no-emit-project",
                "--format",
                "pylock.toml",
                "--output-file",
                str(destination),
            ),
            capture=True,
        )

    def check_lock(self) -> Completed:
        return self._invoke(("lock", "--check"))

    def lock(self) -> Completed:
        """Refresh the project lock through uv's normal resolver, without upgrades."""
        return self._invoke(("lock",))
