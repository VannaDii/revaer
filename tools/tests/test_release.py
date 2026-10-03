"""Real release-engine previews on disposable Git histories, without publication."""

import os
import shutil
import subprocess
from pathlib import Path

import pytest
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.release import SemanticRelease, parse_outputs
from revaer_tooling.process import ProcessRunner


def git(root: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        capture_output=True,
        text=True,
        timeout=30,
        check=True,
    )
    return result.stdout.strip()


@pytest.fixture
def release_repo(tmp_path: Path) -> SemanticRelease:
    git(tmp_path, "init", "--initial-branch=main")
    git(tmp_path, "config", "user.name", "Revaer tooling tests")
    git(tmp_path, "config", "user.email", "tooling-tests@example.invalid")
    git(tmp_path, "config", "commit.gpgsign", "false")
    git(tmp_path, "config", "tag.gpgsign", "false")
    git(tmp_path, "remote", "add", "origin", "https://github.com/example/revaer.git")
    (tmp_path / "release").mkdir()
    source = Path(__file__).resolve().parents[2] / "release/semantic-release.toml"
    shutil.copy2(source, tmp_path / "release/semantic-release.toml")
    git(tmp_path, "add", ".")
    git(tmp_path, "commit", "-m", "Initial release fixture")
    git(tmp_path, "tag", "v1.2.3")
    return SemanticRelease(
        "semantic-release", ProcessRunner(lambda message: None), tmp_path, os.environ
    )


@pytest.mark.parametrize(
    ("message", "version"),
    [
        ("fix: correct an existing behavior", "1.2.4-dev.1"),
        ("docs: clarify setup", "1.2.4-dev.1"),
        ("feat: support a new option", "1.3.0-dev.1"),
        (
            "feat!: change the input contract\n\nBREAKING CHANGE: remove the old input",
            "2.0.0-dev.1",
        ),
    ],
)
def test_preview_preserves_release_rules_and_never_changes_refs(
    release_repo: SemanticRelease, message: str, version: str
) -> None:
    root = release_repo.root
    git(root, "commit", "--allow-empty", "-m", message)
    before = git(root, "show-ref")
    plan = release_repo.preview()
    assert plan.required
    assert plan.version == version
    assert plan.tag == f"v{version}"
    assert git(root, "show-ref") == before
    assert not git(root, "status", "--porcelain")


def test_preview_increments_dev_and_supports_existing_stable_branch(
    release_repo: SemanticRelease,
) -> None:
    root = release_repo.root
    git(root, "commit", "--allow-empty", "-m", "feat: new behavior")
    git(root, "tag", "v1.3.0-dev.1")
    git(root, "commit", "--allow-empty", "-m", "fix: follow-up")
    assert release_repo.preview().version == "1.3.0-dev.2"
    git(root, "checkout", "-b", "gh-pages")
    stable = release_repo.preview()
    assert stable.required
    assert stable.version == "1.3.0"


def test_preview_does_not_promote_feature_branch_to_main(release_repo: SemanticRelease) -> None:
    root = release_repo.root
    git(root, "checkout", "-b", "work/change")
    git(root, "commit", "--allow-empty", "-m", "feat: an ineligible change")
    before = git(root, "show-ref")
    with pytest.raises(ToolingError):
        release_repo.preview()
    assert git(root, "show-ref") == before


def test_preview_reports_no_release_for_unchanged_history(release_repo: SemanticRelease) -> None:
    plan = release_repo.preview()
    assert not plan.required
    assert plan.version == "1.2.3"


def test_release_outputs_preserve_multiline_notes_and_reject_incomplete_data() -> None:
    assert parse_outputs("released=true\nnotes<<END\nfirst\nsecond\nEND\n") == {
        "released": "true",
        "notes": "first\nsecond",
    }
    for content in ("notes<<END\nunterminated\n", "not a value\n"):
        with pytest.raises(ToolingError):
            parse_outputs(content)
