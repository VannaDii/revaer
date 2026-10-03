"""Typed Bash and kcov operations for the single root bootstrap script."""

import re
import resource
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from ..process import Completed, Invocation, Runner
from .base import ExternalTool, ToolVersion
from .python import Uv


@dataclass(frozen=True)
class BootstrapArgs:
    script: Path
    arguments: tuple[str, ...]
    cwd: Path
    environment: Mapping[str, str]
    log: Path
    accepted_codes: tuple[int, ...] = (0,)


class Bash(ExternalTool):
    def verify(self, required: str | None = None) -> ToolVersion:
        version = super().verify(required)
        if tuple(int(part) for part in version.version.split(".")[:2]) < (4, 2):
            raise ToolingError("Bootstrap coverage requires Bash 4.2 or newer with BASH_XTRACEFD")
        return version

    def bootstrap(self, args: BootstrapArgs) -> Completed:
        return self.runner.run(
            Invocation(
                (str(self.locate()), str(args.script), *args.arguments),
                args.cwd,
                args.environment,
                capture=True,
                timeout=180,
                accepted_codes=args.accepted_codes,
                log_path=args.log,
            )
        )


class Kcov(ExternalTool):
    def __init__(
        self,
        name: str,
        runner: Runner,
        root: Path,
        environment: Mapping[str, str],
        python: Path,
        uv: Uv,
    ) -> None:
        super().__init__(name, runner, root, environment)
        self.python = python
        self.uv = uv

    def verify(self, required: str | None = None) -> ToolVersion:
        executable = self.locate()
        actual = self._invoke(("--version",), capture=True).stdout.strip()
        if actual != "kcov " + (required or "43"):
            raise ToolingError("Expected kcov " + (required or "43") + "; found " + actual)
        return ToolVersion(executable, actual.removeprefix("kcov "))

    def collect(self, args: BootstrapArgs, bash: Path, output: Path) -> Completed:
        executable = self.verify().executable
        if "," in str(args.script):
            raise ToolingError("Kcov's include-path option cannot represent a comma in a path")
        soft, _ = resource.getrlimit(resource.RLIMIT_NOFILE)
        limit = 4096 if soft == resource.RLIM_INFINITY else min(soft, 4096)
        if limit < 16:
            raise ToolingError("Kcov requires an open-file limit of at least 16")
        # Kcov derives its trace descriptor from the soft limit. uv applies the
        # selected limit only to its child and preserves the inherited hard limit.
        result = self.uv.limited_command(
            Invocation(
                (
                    str(executable),
                    "--bash-parser=" + str(bash),
                    # DEBUG is kcov's documented method. It removes BASH_ENV
                    # before child tools run, so no generated helper needs a patch.
                    "--bash-method=DEBUG",
                    "--include-path=" + str(args.script),
                    "--bash-dont-parse-binary-dir",
                    str(output),
                    str(args.script),
                    *args.arguments,
                ),
                args.cwd,
                args.environment,
                capture=True,
                timeout=180,
                accepted_codes=args.accepted_codes,
                log_path=args.log,
            ),
            self.python,
            limit,
        )
        if re.search(
            r"^\s*kcov: (?:error|warning)(?::|$)|"
            r"^Failed to (?:exchange stderr|"
            r"get the maximum number of open file descriptors|execute script)",
            result.stdout + "\n" + result.stderr,
            re.IGNORECASE | re.MULTILINE,
        ):
            raise ToolingError("Kcov emitted an error or warning; complete log retained")
        return result
