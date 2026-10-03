"""The official uv core install keeps the real CLI usable without test engines."""

import json
import os
import shutil
import subprocess
import tomllib
from pathlib import Path


def test_core_install_uses_the_same_locked_cli_and_audit_exports_all_groups(tmp_path: Path) -> None:
    source = Path(__file__).parents[2]
    for name in ("pyproject.toml", "uv.lock", ".python-version"):
        shutil.copy(source / name, tmp_path / name)
    shutil.copytree(source / "tools/src", tmp_path / "tools/src")
    shutil.copy(source / "tools/versions.toml", tmp_path / "tools/versions.toml")
    for arguments in (
        ("init", "--quiet"),
        ("add", "."),
        (
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "Core install fixture",
        ),
    ):
        subprocess.run(
            ["git", *arguments], cwd=tmp_path, check=True, capture_output=True, timeout=20
        )
    lock = (tmp_path / "uv.lock").read_bytes()
    # The environment and cache belong only to this fixture. uv creates and
    # populates them through its supported project interface.
    selectors = {
        "UV_PROJECT",
        "UV_PROJECT_ENVIRONMENT",
        "UV_WORKING_DIRECTORY",
        "UV_PYTHON",
        "PYTHONHOME",
        "PYTHONPATH",
        "VIRTUAL_ENV",
        "CONDA_PREFIX",
    }
    environment = {key: value for key, value in os.environ.items() if key not in selectors}
    environment.update(
        UV_PROJECT_ENVIRONMENT=str(tmp_path / ".venv"), UV_CACHE_DIR=str(tmp_path / "cache")
    )

    def run(*arguments: str) -> str:
        return subprocess.run(
            ["uv", *arguments],
            cwd=tmp_path,
            env=environment,
            check=True,
            capture_output=True,
            text=True,
            timeout=180,
        ).stdout

    run("sync", "--locked", "--no-default-groups")
    packages = {
        item["name"]
        for item in json.loads(
            run("pip", "list", "--python", str(tmp_path / ".venv/bin/python"), "--format", "json")
        )
    }
    assert "revaer-tooling" in packages
    assert not packages & {
        "playwright",
        "pytest",
        "python-semantic-release",
        "tree-sitter",
        "ruff",
        "psutil",
        "watchfiles",
        "pglast",
    }
    help_text = run("run", "--locked", "--no-default-groups", "--", "rv", "--help")
    assert "workflow-metadata" in help_text and "ui-e2e" in help_text
    metadata = json.loads(
        run("run", "--locked", "--no-default-groups", "--", "rv", "workflow-metadata")
    )
    assert len(metadata["sha"]) == 40 and metadata["short_sha"] == metadata["sha"][:7]
    exported = run(
        "export", "--locked", "--all-groups", "--no-emit-project", "--format", "pylock.toml"
    )
    exported_packages = {item["name"]: item for item in tomllib.loads(exported)["packages"]}
    for name in ("playwright", "pytest", "python-semantic-release", "tree-sitter", "ruff"):
        assert exported_packages[name]["version"]
    assert exported_packages["python-semantic-release"]["archive"]["hashes"]["sha256"]
    assert (tmp_path / "uv.lock").read_bytes() == lock


def test_alpine_core_uses_uv_managed_python_and_the_installed_cli(tmp_path: Path) -> None:
    source = Path(__file__).parents[2]
    root = tmp_path / "checkout"
    root.mkdir()
    for name in ("pyproject.toml", "uv.lock", ".python-version"):
        shutil.copy(source / name, root / name)
    shutil.copytree(
        source / "tools/src", root / "tools/src", ignore=shutil.ignore_patterns("__pycache__")
    )
    shutil.copy(source / "tools/tests/fixtures/alpine_core.py", root / "verify-core.py")
    lock = (root / "uv.lock").read_bytes()
    # Official uv 0.12.13 Alpine 3.23, pinned by its multi-platform index digest.
    image = (
        "ghcr.io/astral-sh/uv@sha256:"
        "e73003739c99f562680444e8e9a0c605dad7a87ff2fbcc0a715efe875713623e"
    )
    identifier = subprocess.run(
        [
            "docker",
            "create",
            "--workdir",
            "/work",
            "--mount",
            f"type=bind,src={root},dst=/work",
            "--env",
            "UV_LINK_MODE=copy",
            image,
            "uv",
            "run",
            "--locked",
            "--no-default-groups",
            "--",
            "python",
            "verify-core.py",
        ],
        check=True,
        capture_output=True,
        text=True,
        timeout=300,
    ).stdout.strip()
    try:
        log_path = tmp_path / "alpine-core.log"
        with log_path.open("w") as log:
            result = subprocess.run(
                ["docker", "start", "--attach", identifier],
                stdout=log,
                stderr=subprocess.STDOUT,
                check=False,
                timeout=300,
            )
        assert result.returncode == 0, log_path.read_text()
        assert "warning:" not in log_path.read_text().lower()
        proof = json.loads((root / "core-proof.json").read_text())
        assert proof["python"] == [3, 13, 12]
        assert "musl" in proof["interpreter"]
        assert "revaer-tooling" in proof["packages"]
        assert not set(proof["packages"]) & {
            "playwright",
            "pytest",
            "python-semantic-release",
            "tree-sitter",
        }
        assert proof["lock_unchanged"] is True
        assert (root / "uv.lock").read_bytes() == lock
    finally:
        subprocess.run(
            ["docker", "rm", "--force", identifier], check=True, capture_output=True, timeout=30
        )
