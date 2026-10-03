"""Typed Debian package installation through the system package manager.

Package tokens are data, never shell fragments. Apt owns repository signatures,
dependency resolution and installed-package state; rv does not bypass any of them.
"""

import re
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from ..process import Invocation, Runner
from .base import ExternalTool


@dataclass(frozen=True)
class AptInstallArgs:
    packages: tuple[str, ...]

    def validate(self) -> None:
        token = r"[a-z0-9][a-z0-9.+-]*(?::[a-z0-9][a-z0-9-]*)?(?:=[A-Za-z0-9.+:~_-]+)?"
        if not self.packages or any(re.fullmatch(token, value) is None for value in self.packages):
            raise ToolingError("Apt requires package names with optional architecture/version pins")
        if len(self.packages) != len(set(self.packages)):
            raise ToolingError("Apt package selection contains duplicates")


class Apt(ExternalTool):
    def __init__(
        self,
        name: str,
        runner: Runner,
        root: Path,
        environment: Mapping[str, str],
        privilege: ExternalTool | None,
    ) -> None:
        super().__init__(name, runner, root, environment)
        self.privilege = privilege

    def install(self, args: AptInstallArgs) -> None:
        args.validate()
        executable = self.verify().executable
        prefix: tuple[str, ...] = ()
        if self.privilege is not None:
            prefix = (
                str(self.privilege.verify().executable),
                "--non-interactive",
                "--preserve-env=DEBIAN_FRONTEND",
                "--",
            )
        for operation in (("update",), ("install", "--yes", "--", *args.packages)):
            self.runner.run(
                Invocation(
                    (*prefix, str(executable), *operation),
                    self.root,
                    {**self.environment, "DEBIAN_FRONTEND": "noninteractive"},
                    timeout=900,
                )
            )


@dataclass(frozen=True)
class ApkInstallArgs:
    packages: tuple[str, ...]

    def validate(self) -> None:
        if not self.packages or any(
            not re.fullmatch(r"[a-z0-9][a-z0-9+_.-]*=[0-9][A-Za-z0-9+_.~-]*", package)
            for package in self.packages
        ):
            raise ToolingError("APK packages require exact name=version pins")
        names = [package.partition("=")[0] for package in self.packages]
        if len(names) != len(set(names)):
            raise ToolingError("APK package selection contains duplicate names")


class Apk(ExternalTool):
    def install(self, args: ApkInstallArgs) -> None:
        args.validate()
        # APK owns signature verification, repository resolution and installed
        # package state. No trust bypass or unversioned fallback is permitted.
        self._invoke(("add", "--no-cache", "--", *args.packages), timeout=900)
