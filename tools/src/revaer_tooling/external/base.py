"""Shared executable discovery; concrete tools own their command arguments."""

import re
import shutil
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from ..process import Completed, Invocation, Runner


@dataclass(frozen=True)
class ToolVersion:
    executable: Path
    version: str


class ExternalTool:
    version_args: tuple[str, ...] = ("--version",)

    def __init__(
        self,
        name: str,
        runner: Runner,
        root: Path,
        environment: Mapping[str, str],
        *,
        installation_hint: str = "run rv setup",
    ) -> None:
        self.name = name
        self.runner = runner
        self.root = root
        self.environment = environment
        self.installation_hint = installation_hint

    def locate(self) -> Path:
        executable = shutil.which(self.name, path=self.environment.get("PATH", ""))
        if executable is None:
            raise ToolingError(f"{self.name} is missing; {self.installation_hint}")
        return Path(executable)

    def verify(self, required: str | None = None) -> ToolVersion:
        executable = self.locate()
        result = self._invoke(self.version_args, capture=True)
        output = result.stdout or result.stderr
        match = re.search(r"(?<![\w.])v?(\d+\.\d+(?:\.\d+)?(?:[-+][\w.]+)?)", output)
        if match is None:
            raise ToolingError(f"Cannot determine {self.name} version from {output.strip()!r}")
        version = match.group(1)
        if required is not None and version != required:
            raise ToolingError(f"{self.name} {version} found; {required} required; run rv setup")
        return ToolVersion(executable, version)

    def _invoke(
        self,
        args: tuple[str, ...],
        *,
        cwd: Path | None = None,
        env: Mapping[str, str] | None = None,
        capture: bool = False,
        input_text: str | None = None,
        timeout: float | None = None,
        accepted_codes: tuple[int, ...] = (0,),
        capture_binary: bool = False,
    ) -> Completed:
        return self.runner.run(
            Invocation(
                (str(self.locate()), *args),
                cwd or self.root,
                {**self.environment, **(env or {})},
                capture,
                input_text,
                timeout,
                accepted_codes,
                capture_binary=capture_binary,
            )
        )
