"""Native source builds through CMake's configure, build, and install commands."""

from dataclasses import dataclass
from pathlib import Path

from ..process import Invocation
from .base import ExternalTool


@dataclass(frozen=True)
class CmakeInstallArgs:
    source: Path
    build: Path
    prefix: Path
    logs: Path
    jobs: int = 2


class Cmake(ExternalTool):
    def install(self, args: CmakeInstallArgs) -> None:
        executable = self.verify().executable
        stages = {
            "configure": (
                "-S",
                str(args.source),
                "-B",
                str(args.build),
                "-DCMAKE_BUILD_TYPE=Release",
                "-DCMAKE_INSTALL_PREFIX=" + str(args.prefix),
            ),
            "build": ("--build", str(args.build), "--parallel", str(args.jobs)),
            "install": ("--install", str(args.build)),
        }
        for name, arguments in stages.items():
            self.runner.run(
                Invocation(
                    (str(executable), *arguments),
                    self.root,
                    self.environment,
                    timeout=600,
                    log_path=args.logs / (name + ".log"),
                )
            )
