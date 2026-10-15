"""Native private ext4 lifecycle for restart-persistent Linux E2E roots."""

from pathlib import Path

from ..errors import ToolingError
from ..process import Invocation
from .base import ExternalTool


class Mount(ExternalTool):
    def temporary(
        self, path: Path, image: Path, uid: int, gid: int, privilege: ExternalTool | None
    ) -> None:
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
        validate_image(image, path.parent, uid)
        arguments = (
            "-t",
            "ext4",
            "-o",
            "loop,nodev,nosuid,noexec",
            str(image),
            str(path),
        )
        prefix = (str(privilege.locate()), "--non-interactive", "--") if privilege else ()
        self.runner.run(
            Invocation(
                (*prefix, str(self.locate()), *arguments), self.root, self.environment, timeout=30
            )
        )


def validate_image(image: Path, parent: Path, uid: int) -> None:
    metadata = image.lstat()
    directory = parent.lstat()
    if (
        not image.is_absolute()
        or image.is_symlink()
        or not image.is_file()
        or image.parent != parent
        or metadata.st_uid != uid
        or metadata.st_mode & 0o777 != 0o600
        or metadata.st_nlink != 1
        or parent.is_symlink()
        or directory.st_uid != uid
        or directory.st_mode & 0o777 != 0o700
    ):
        raise ToolingError("E2E disk fixture requires a private owned backing file")


class MkfsExt4(ExternalTool):
    def image(self, image: Path, uid: int, gid: int) -> None:
        validate_image(image, image.parent, uid)
        self._invoke(("-q", "-F", "-E", f"root_owner={uid}:{gid}", str(image)), timeout=30)


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
