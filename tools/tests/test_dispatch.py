"""Dispatch and filesystem isolation across real checkout-shaped directories."""

import subprocess
import sys
from pathlib import Path

import pytest
from revaer_tooling.cli import COMMANDS, parser
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.repository import checkout
from revaer_tooling.tasks.build import Check


def create_checkout(root: Path) -> None:
    (root / "tools/src/revaer_tooling").mkdir(parents=True)
    (root / "tools/src/revaer_tooling/cli.py").touch()
    (root / ".git").write_text("gitdir: elsewhere")
    (root / "Cargo.toml").touch()


def test_dispatch_uses_static_method_references() -> None:
    assert COMMANDS["check"] is Check.run
    args = parser().parse_args(["setup", "--profile", "python", "--no-launcher"])
    assert args.profile == "python"
    assert args.no_launcher


def test_cli_help_does_not_require_native_tools_at_import_time(tmp_path: Path) -> None:
    result = subprocess.run(
        [sys.executable, "-m", "revaer_tooling.cli", "--help"],
        cwd=tmp_path,
        env={"PATH": str(tmp_path)},
        capture_output=True,
        text=True,
        timeout=10,
        check=True,
    )
    assert "Revaer development and automation" in result.stdout
    assert not result.stderr


def test_worktrees_are_selected_from_current_directory(tmp_path: Path) -> None:
    first = tmp_path / "first tree"
    second = tmp_path / "second tree"
    for root in (first, second):
        create_checkout(root)
        assert checkout(root / "tools/src") == root


def test_old_checkout_does_not_fall_through_to_other_implementation(tmp_path: Path) -> None:
    create_checkout(tmp_path)
    nested = tmp_path / "old"
    nested.mkdir()
    (nested / ".git").touch()
    (nested / "Cargo.toml").touch()
    with pytest.raises(ToolingError, match="does not contain rv"):
        checkout(nested)


def test_cleanup_rejects_symlink_and_owner_root(tmp_path: Path) -> None:
    fs = FileSystem()
    child = tmp_path / "child"
    child.symlink_to(tmp_path, target_is_directory=True)
    for target in (child, tmp_path):
        with pytest.raises(ToolingError, match="Refusing cleanup"):
            fs.remove_owned(target, tmp_path)


def test_owned_cleanup_and_private_write(tmp_path: Path) -> None:
    fs = FileSystem()
    path = tmp_path / "artifacts/secret"
    fs.write(path, "value", 0o600)
    assert path.stat().st_mode & 0o777 == 0o600
    fs.remove_owned(path.parent, tmp_path)
    assert not path.exists()


def test_write_replaces_destination_without_following_symlink(tmp_path: Path) -> None:
    original = tmp_path / "unrelated"
    original.write_text("keep me")
    destination = tmp_path / "output"
    destination.symlink_to(original)
    FileSystem().write(destination, "new content", 0o600)
    assert original.read_text() == "keep me"
    assert not destination.is_symlink()
    assert destination.read_text() == "new content"
    assert destination.stat().st_mode & 0o777 == 0o600
