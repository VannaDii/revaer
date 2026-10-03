"""Run only inside the disposable uv/Debian setup test container."""

import json
import subprocess
from pathlib import Path

from revaer_tooling.cli import main


def verify() -> None:
    root = Path.cwd()
    lock = (root / "uv.lock").read_bytes()
    arguments = ["setup", "--profile", "python", "--no-launcher", "--apt-packages", "libdw-dev"]
    assert main(arguments) == 0
    package = subprocess.run(
        ["dpkg-query", "--show", "--showformat=${Status}\n${Version}", "libdw-dev"],
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    ).stdout
    assert package.startswith("install ok installed\n")
    assert main(arguments) == 0
    assert main([*arguments[:-2], "--apt-packages=--allow-unauthenticated"]) == 1
    missing = main([*arguments[:-1], "revaer-nonexistent-setup-fixture-a86143c7"])
    assert missing == 100
    assert main(arguments) == 0
    assert (root / "uv.lock").read_bytes() == lock
    (root / "setup-proof.json").write_text(
        json.dumps({"package": package, "missing_package_exit": missing, "lock_unchanged": True})
        + "\n"
    )


if __name__ == "__main__":
    verify()
