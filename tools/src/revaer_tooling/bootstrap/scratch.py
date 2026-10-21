"""Keep command and child-process scratch beneath the selected repository."""

import os
import tempfile
from collections.abc import Iterator, Mapping
from contextlib import contextmanager
from pathlib import Path

from ..errors import ToolingError
from ..filesystem import FileSystem


def uses_test_scratch(name: str) -> bool:
    return name.startswith(("test", "ui-e2e", "tooling-")) or name in (
        "ci",
        "ci-qualify",
        "validate",
        "cov",
        "script-coverage",
        "helm-lint",
        "runbook",
    )


def scratch_parent(root: Path, environment: Mapping[str, str]) -> Path:
    parent = Path(environment.get("REVAER_TEST_ROOT") or root / ".test-tmp")
    if parent.is_symlink():
        raise ToolingError("Repository test scratch must not be a symlink")
    parent.mkdir(mode=0o700, exist_ok=True)
    return parent


@contextmanager
def repository_scratch(root: Path, fs: FileSystem) -> Iterator[None]:
    parent = scratch_parent(root, {})
    temporary = fs.temporary_directory(parent, "run-")
    previous = {name: os.environ.get(name) for name in ("TMPDIR", "REVAER_TEST_ROOT")}
    cached = tempfile.tempdir
    try:
        # Child services such as compiler caches can outlive this command.
        # Their temporary root must survive cleanup of this process's private child.
        os.environ.update({"TMPDIR": str(parent), "REVAER_TEST_ROOT": str(parent)})
        tempfile.tempdir = str(temporary)
        yield
    finally:
        tempfile.tempdir = cached
        for name, value in previous.items():
            if value is None:
                os.environ.pop(name, None)
            else:
                os.environ[name] = value
        fs.remove_owned(temporary, parent)
