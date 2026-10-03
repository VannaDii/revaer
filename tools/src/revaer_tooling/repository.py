"""Resolve the repository containing the running, uv-selected implementation."""

import tomllib
from pathlib import Path

from .errors import ToolingError


def checkout(start: Path, *, source_archive: bool = False) -> Path:
    """Stop at the nearest Git boundary, including a worktree's .git file."""
    for candidate in (start, *start.parents):
        if (candidate / ".git").exists():
            if not (candidate / "tools/src/revaer_tooling/cli.py").is_file():
                raise ToolingError(f"This checkout does not contain rv: {candidate}")
            return candidate
    if source_archive:
        # Docker intentionally excludes Git's object database. Only container
        # tasks opt into this fallback; the installed worktree launcher and
        # Git-dependent operations retain the nearest-Git-boundary rule.
        for candidate in (start, *start.parents):
            project = candidate / "pyproject.toml"
            cargo = candidate / "Cargo.toml"
            if project.is_file() and cargo.is_file():
                try:
                    package = tomllib.loads(project.read_text())
                    workspace = tomllib.loads(cargo.read_text())
                    revaer = (
                        package["project"]["name"] == "revaer-tooling"
                        and workspace["workspace"]["package"]["edition"] == "2024"
                    )
                except (tomllib.TOMLDecodeError, KeyError, TypeError) as error:
                    raise ToolingError("Invalid container project metadata") from error
                if (
                    revaer
                    and (candidate / "tools/src/revaer_tooling/cli.py").is_file()
                    and (candidate / "uv.lock").is_file()
                ):
                    return candidate
                raise ToolingError(
                    "Container source must identify Revaer's locked Rust 2024 project"
                )
    raise ToolingError("Run rv from a Revaer checkout")
