"""Private atomic recovery receipts for a single development loop per checkout."""

import json
import stat
from dataclasses import asdict, dataclass
from pathlib import Path

from ..errors import ToolingError
from ..filesystem import FileSystem
from ..json_data import Json, decode, object_value, string_value
from .processes import ProcessIdentity


def directory(root: Path, fs: FileSystem) -> Path:
    path = root / "target/rv-dev"
    for parent in (root / "target", path):
        if parent.is_symlink() or (parent.exists() and not parent.is_dir()):
            raise ToolingError(f"Development state requires an unlinked directory: {parent}")
    fs.mkdir(path)
    return path


def _identity(value: Json) -> ProcessIdentity:
    record = object_value(value)
    if (
        set(record) != {"pid", "created", "token"}
        or type(record["pid"]) is not int
        or type(record["created"]) not in (int, float)
        or not isinstance(record["token"], str)
    ):
        raise ToolingError("Malformed development process identity")
    pid, created = record["pid"], record["created"]
    if not isinstance(pid, int) or not isinstance(created, (int, float)):
        raise ToolingError("Development process identity must be numeric")
    token = record["token"]
    if not isinstance(token, str):
        raise ToolingError("Development session token must be a string")
    result = ProcessIdentity(pid, float(created), token)
    result.validate()
    return result


@dataclass(frozen=True)
class Session:
    root: Path
    supervisor: ProcessIdentity
    workers: tuple[ProcessIdentity, ...]

    def write(self, fs: FileSystem, path: Path) -> None:
        fs.write(
            path,
            json.dumps(
                {
                    "schema": 1,
                    "checkout": str(self.root),
                    "supervisor": asdict(self.supervisor),
                    "workers": [asdict(worker) for worker in self.workers],
                },
                indent=2,
            )
            + "\n",
            mode=0o600,
        )

    @staticmethod
    def read(fs: FileSystem, path: Path, root: Path) -> "Session":
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
            raise ToolingError("Development receipt must be a regular, unlinked file")
        if info.st_size > 4096:
            raise ToolingError("Development receipt exceeds its size bound")
        record = object_value(decode(fs.read(path)))
        workers = record.get("workers")
        if (
            set(record) != {"schema", "checkout", "supervisor", "workers"}
            or type(record["schema"]) is not int
            or record["schema"] != 1
            or string_value(record["checkout"]) != str(root)
            or not isinstance(workers, list)
            or len(workers) > 2
        ):
            raise ToolingError("Development receipt does not identify this checkout")
        return Session(root, _identity(record["supervisor"]), tuple(map(_identity, workers)))
