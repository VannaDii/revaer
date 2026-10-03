"""Inspect the actual uv-managed musl interpreter and installed core entry point."""

import importlib.metadata
import json
import subprocess
import sys
from pathlib import Path


def main() -> None:
    root = Path.cwd()
    lock = (root / "uv.lock").read_bytes()
    output = subprocess.run(
        [str(root / ".venv/bin/rv"), "--help"],
        check=True,
        capture_output=True,
        text=True,
        timeout=20,
    )
    if "workflow-metadata" not in output.stdout or "ui-e2e" not in output.stdout:
        raise ValueError("The installed entry point did not expose the actual CLI")
    (root / "core-proof.json").write_text(
        json.dumps(
            {
                "python": list(sys.version_info[:3]),
                "interpreter": sys.base_prefix,
                "packages": sorted(
                    str(distribution.metadata["Name"]).lower().replace("_", "-")
                    for distribution in importlib.metadata.distributions()
                ),
                "lock_unchanged": (root / "uv.lock").read_bytes() == lock,
                "entry_point": "rv",
            },
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    main()
