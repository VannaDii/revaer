"""GitHub's documented environment-file protocol, without shell interpolation."""

import hashlib
import re
from collections.abc import Mapping
from pathlib import Path

from ..errors import ToolingError
from ..filesystem import FileSystem


def write_values(fs: FileSystem, destination: Path, values: Mapping[str, str]) -> None:
    """Validate the entire record before appending any multiline-safe values."""
    records: list[str] = []
    for name, value in values.items():
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name) or "\0" in value:
            raise ToolingError("Workflow command files require valid names and NUL-free values")
        delimiter = "rv_" + hashlib.sha256(value.encode()).hexdigest()
        if delimiter in value.splitlines():
            raise ToolingError("Workflow value collides with its multiline delimiter")
        records.append(f"{name}<<{delimiter}\n{value}\n{delimiter}\n")
    fs.append(destination, "".join(records))
