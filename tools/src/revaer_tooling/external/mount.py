"""Native private tmpfs lifecycle for disposable Linux E2E roots."""

from pathlib import Path

from ..errors import ToolingError
from ..process import Invocation
from .base import ExternalTool


class Mount(ExternalTool):
    def temporary(self, path: Path, uid: int, gid: int, privilege: ExternalTool | None) -> None:
        metadata = path.stat()
        if (
            not path.is_absolute()
            or path.is_symlink()
            or not path.is_dir()
            or metadata.st_uid != uid
            or metadata.st_gid != gid
            or metadata.st_mode & 0o777 != 0o700
            or any(path.iterdir())
        ):
            raise ToolingError("E2E mount requires an empty private owned directory")
        arguments = (
            "-t",
            "tmpfs",
            "-o",
            f"size=256m,mode=0700,uid={uid},gid={gid}",
            "revaer-e2e",
            str(path),
        )
        prefix = (str(privilege.locate()), "--non-interactive", "--") if privilege else ()
        self.runner.run(
            Invocation(
                (*prefix, str(self.locate()), *arguments), self.root, self.environment, timeout=30
            )
        )


class Unmount(ExternalTool):
    def temporary(self, path: Path, privilege: ExternalTool | None) -> None:
        prefix = (str(privilege.locate()), "--non-interactive", "--") if privilege else ()
        self.runner.run(
            Invocation(
                (*prefix, str(self.locate()), "--", str(path)),
                self.root,
                self.environment,
                timeout=30,
            )
        )
