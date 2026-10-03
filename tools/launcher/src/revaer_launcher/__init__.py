"""The installed `rv` selects a checkout; uv supplies its locked implementation.

This package intentionally has no runtime dependencies. A uv tool environment
does not consume a project's uv.lock, so installing the implementation here
would give local development and automation different dependency environments.
"""

import os
import shutil
import sys
from pathlib import Path


def command(start: Path, arguments: list[str], uv: str) -> list[str]:
    """Select the nearest Git checkout, refusing to escape a nested repository."""
    for root in (start, *start.parents):
        if (root / ".git").exists():
            if not (root / "tools/src/revaer_tooling/cli.py").is_file():
                raise RuntimeError(
                    f"This checkout has no rv implementation; run {root / 'setup.sh'}"
                )
            # Invoke the module explicitly: resolving `rv` again could select
            # this installed entry point and recurse if project setup is broken.
            return [
                uv,
                "run",
                "--locked",
                "--directory",
                str(root),
                "--",
                "python",
                "-m",
                "revaer_tooling.cli",
                *arguments,
            ]
    raise RuntimeError("Run rv from a Revaer checkout")


def main() -> int:
    """Replace the launcher with uv so signals and the final exit status survive."""
    try:
        uv = shutil.which("uv")
        if uv is None:
            raise RuntimeError("uv is missing from PATH; run ./setup.sh from your Revaer checkout")
        argv = command(Path.cwd().resolve(), sys.argv[1:], uv)
        environment = dict(os.environ)
        # Discard only interpreter/project overrides that can select another
        # checkout. uv cache, index, credentials, and tool-directory options stay.
        for name in (
            "VIRTUAL_ENV",
            "UV_PROJECT_ENVIRONMENT",
            "UV_PROJECT",
            "UV_WORKING_DIR",
            "PYTHONHOME",
            "PYTHONPATH",
        ):
            environment.pop(name, None)
        os.execve(uv, argv, environment)
    except (OSError, RuntimeError) as error:
        print(f"rv: {error}", file=sys.stderr)
        return 1
