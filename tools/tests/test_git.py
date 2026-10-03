"""Real Git checks cover dirty worktrees and committed branch differences."""

import os
import subprocess
from pathlib import Path

import pytest
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.git import Git
from revaer_tooling.process import ProcessRunner


def git(root: Path, *arguments: str) -> None:
    subprocess.run(
        ["git", "-C", str(root), *arguments], check=True, capture_output=True, timeout=10
    )


@pytest.fixture
def repository(tmp_path: Path) -> Git:
    git(tmp_path, "init", "--initial-branch=main")
    git(tmp_path, "config", "user.name", "Tooling tests")
    git(tmp_path, "config", "user.email", "tests@example.invalid")
    git(tmp_path, "config", "commit.gpgsign", "false")
    git(tmp_path, "commit", "--allow-empty", "-m", "Initial fixture")
    return Git("git", ProcessRunner(lambda message: None), tmp_path, os.environ)


def test_clean_checkout_uses_branch_diff_instead_of_skipping_checks(repository: Git) -> None:
    root = repository.root
    git(root, "update-ref", "refs/remotes/origin/main", "HEAD")
    git(root, "checkout", "-b", "work/change")
    name = "file with a newline\nin its name"
    (root / name).touch()
    git(root, "add", ".")
    git(root, "commit", "-m", "Change on branch")
    assert repository.changed_files() == (name,)
    repository.require_clean()


def test_dirty_checkout_includes_staged_unstaged_and_untracked(repository: Git) -> None:
    root = repository.root
    (root / "tracked").touch()
    git(root, "add", ".")
    git(root, "commit", "-m", "Track fixture")
    (root / "tracked").write_text("changed")
    (root / "staged").touch()
    git(root, "add", "staged")
    (root / "untracked").touch()
    assert repository.changed_files() == ("staged", "tracked", "untracked")
    assert repository.files() == ("staged", "tracked")
    assert repository.files(include_untracked=True) == ("staged", "tracked", "untracked")
    with pytest.raises(ToolingError, match="clean checkout"):
        repository.require_clean()


def test_initial_repo_and_parent_fallback_are_distinct(repository: Git) -> None:
    assert repository.changed_files() == ()
    (repository.root / "changed").touch()
    git(repository.root, "add", ".")
    git(repository.root, "commit", "-m", "One change")
    assert repository.changed_files() == ("changed",)
    assert repository.changed_files("HEAD", "HEAD") == ()
    with pytest.raises(ToolingError):
        repository.changed_files("--output=unwanted")
    assert not (repository.root / "unwanted").exists()
