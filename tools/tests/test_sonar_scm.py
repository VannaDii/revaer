"""Real shallow clones prove event-base preparation without altering active work."""

import subprocess
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.git import ScmArgs
from revaer_tooling.tasks.sonar import SonarPrepareScm


def git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *args], check=True, capture_output=True, text=True, timeout=15
    ).stdout.strip()


@pytest.fixture
def scm_context(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Context, str]:
    origin = tmp_path / "origin.git"
    git(tmp_path, "init", "--bare", str(origin))
    seed = tmp_path / "seed"
    git(tmp_path, "init", "--initial-branch=main", str(seed))
    git(seed, "config", "user.name", "Sonar fixture")
    git(seed, "config", "user.email", "sonar@example.invalid")
    git(seed, "config", "commit.gpgsign", "false")
    package = seed / "tools/src/revaer_tooling"
    package.mkdir(parents=True)
    (package / "cli.py").touch()
    git(seed, "add", ".")
    git(seed, "commit", "-m", "Initial fixture")
    base = git(seed, "rev-parse", "HEAD")
    git(seed, "remote", "add", "origin", str(origin))
    git(seed, "push", "origin", "main")
    git(seed, "switch", "-c", "feature")
    git(seed, "commit", "--allow-empty", "-m", "Feature fixture")
    head = git(seed, "rev-parse", "HEAD")
    git(seed, "push", "origin", "feature")
    git(seed, "switch", "main")
    git(seed, "commit", "--allow-empty", "-m", "Advance base fixture")
    divergent = git(seed, "rev-parse", "HEAD")
    git(seed, "push", "origin", "main")
    work = tmp_path / "work"
    git(tmp_path, "clone", "--branch=feature", "--depth=1", origin.as_uri(), str(work))
    monkeypatch.chdir(work)
    context = make_context(Options())
    settings = replace(context.settings.sonar, base_sha=base, base_ref="main", head_sha=head)
    return replace(context, settings=replace(context.settings, sonar=settings)), divergent


def test_unshallow_preserves_dirty_work_and_binds_exact_event_base(
    scm_context: tuple[Context, str],
) -> None:
    context, _ = scm_context
    root = context.root
    (root / "uncommitted").write_text("preserve working changes\n")
    assert git(root, "rev-parse", "--is-shallow-repository") == "true"
    SonarPrepareScm.run(context)
    assert git(root, "rev-parse", "--is-shallow-repository") == "false"
    assert git(root, "rev-parse", "HEAD") == context.settings.sonar.head_sha
    assert git(root, "branch", "--show-current") == "feature"
    assert (root / "uncommitted").read_text() == "preserve working changes\n"
    evidence = (root / "artifacts/sonar/scm-evidence.txt").read_text()
    assert "base_sha=" + context.settings.sonar.base_sha in evidence
    assert "shallow=false\nreachable_commit_count=3\n" in evidence
    # An empty marker left by another Git operation must not mislead the scanner.
    marker = root / ".git/shallow"
    marker.touch()
    SonarPrepareScm.run(context)
    assert not marker.exists()


@pytest.mark.parametrize("case", ("head", "base", "missing-origin"))
def test_wrong_event_identity_fails_and_invalidates_old_success(
    scm_context: tuple[Context, str], case: str
) -> None:
    context, divergent = scm_context
    SonarPrepareScm.run(context)
    settings = context.settings.sonar
    if case == "head":
        settings = replace(settings, head_sha=settings.base_sha)
    elif case == "base":
        settings = replace(settings, base_sha=divergent)
    else:
        git(context.root, "remote", "remove", "origin")
    context = replace(context, settings=replace(context.settings, sonar=settings))
    with pytest.raises(ToolingError):
        SonarPrepareScm.run(context)
    assert not (context.root / "artifacts/sonar/scm-evidence.txt").exists()


@pytest.mark.parametrize(
    "base,branch,head",
    (
        ("a" * 39, "main", "b" * 40),
        ("a" * 40, "main", "B" * 40),
        ("a" * 40, "../../escape", "b" * 40),
        ("a" * 40, "/main", "b" * 40),
        ("a" * 40, "main/", "b" * 40),
        ("a" * 40, "main\nsecond", "b" * 40),
    ),
)
def test_invalid_scm_arguments_fail_before_fetch(base: str, branch: str, head: str) -> None:
    with pytest.raises(ToolingError):
        ScmArgs(base, branch, head).validate()
