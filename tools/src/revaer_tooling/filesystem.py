"""Explicit filesystem collaborator for tasks."""

import fcntl
import os
import shutil
import stat
import tempfile
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path

from .errors import ToolingError


class FileSystem:
    @contextmanager
    def lock(self, path: Path) -> Iterator[None]:
        """Serialize local Unix operations without following a lock-file symlink."""
        descriptor = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        with os.fdopen(descriptor, "w") as stream:
            try:
                fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise ToolingError(f"Another operation holds {path}") from error
            yield

    def read(self, path: Path) -> str:
        return path.read_text(encoding="utf-8")

    def read_regular_bytes(self, path: Path) -> bytes:
        """Read exact bytes from a regular, unlinked file without blocking on a FIFO.

        Callers that require checkout confinement also validate parent paths.
        Opening before fstat prevents replacing the final component with a link
        between a separate type check and the read.
        """
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, "rb") as stream:
            metadata = os.fstat(stream.fileno())
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
                raise ToolingError(f"Input must be a regular, unlinked file: {path}")
            return stream.read()

    def is_file(self, path: Path) -> bool:
        return path.is_file()

    def write(self, path: Path, value: str, mode: int = 0o644) -> None:
        self.write_bytes(path, value.encode("utf-8"), mode)

    def write_bytes(self, path: Path, value: bytes, mode: int = 0o644) -> None:
        """Publish a complete file with its intended permissions from creation.

        NamedTemporaryFile starts private. Replacing the destination also avoids
        following a pre-existing symlink or truncating a good artifact on failure.
        """
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(mode="wb", dir=path.parent, delete=False) as stream:
            temporary = Path(stream.name)
            try:
                os.fchmod(stream.fileno(), mode)
                stream.write(value)
                stream.flush()
                temporary.replace(path)
            finally:
                temporary.unlink(missing_ok=True)

    def mkdir(self, path: Path) -> None:
        path.mkdir(parents=True, exist_ok=True)

    def temporary_directory(self, parent: Path, prefix: str) -> Path:
        return Path(tempfile.mkdtemp(dir=parent, prefix=prefix))

    def create_disk_image(self, path: Path, size: int) -> None:
        """Exclusively create a private sparse backing file for a functional fixture."""
        if size <= 0:
            raise ToolingError("Disk fixture size must be positive")
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(descriptor, "wb") as stream:
            stream.truncate(size)

    def directory_permissions(self, path: Path, uid: int, gid: int, mode: int) -> None:
        if path.is_symlink() or not path.is_dir():
            raise ToolingError(f"Directory permissions require a nonlinked directory: {path}")
        os.chown(path, uid, gid)
        path.chmod(mode)

    def append(self, path: Path, value: str) -> None:
        """Append to a regular command file without following a placed symlink.

        GitHub supplies an existing file per step. O_APPEND preserves earlier
        values; a private creation mode also supports local integration fixtures.
        """
        descriptor = os.open(
            path, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK, 0o600
        )
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                raise ToolingError(f"Command file must be a regular file: {path}")
            stream.write(value)

    def copy(self, source: Path, destination: Path) -> None:
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)

    def copy_tree(self, source: Path, destination: Path) -> None:
        """Archive an owned artifact tree without following unrelated links."""
        if source.is_symlink() or any(path.is_symlink() for path in source.rglob("*")):
            raise ToolingError(f"Artifact trees must not contain symlinks: {source}")
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(source, destination)

    def remove_owned(self, path: Path, owner: Path) -> None:
        if path.is_symlink() or not path.resolve().is_relative_to(owner.resolve()):
            raise ToolingError(f"Refusing cleanup outside {owner}: {path}")
        if path.resolve() == owner.resolve():
            raise ToolingError(f"Refusing cleanup of ownership root {owner}")
        if path.is_dir():
            shutil.rmtree(path)
        else:
            path.unlink(missing_ok=True)
