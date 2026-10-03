"""Real documentation payload copies, source metadata and publication guardrails."""

import json
import subprocess
from dataclasses import replace
from datetime import UTC, datetime
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.settings import load_settings
from revaer_tooling.tasks.docs import DocsGuard, DocsPrepare


@pytest.fixture
def documentation(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    for name, content in {
        "tools/src/revaer_tooling/cli.py": "",
        "docs/README.md": "# Fixture documentation\n",
        "docs/book/index.html": "<h1>Book</h1>",
        "docs/book/removed.html": "previous chapter",
        "docs/book/llm/stale.json": "old manifest",
        "docs/llm/manifest.json": "{}",
        "docs/llm/schema.json": "{}",
        "docs/llm/summaries.json": "[]",
        ".gitignore": "deploy/\ndocs/book/\n.rv-docs-*\n",
    }.items():
        path = tmp_path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    for args in (
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
            "Docs fixture",
        ),
    ):
        subprocess.run(["git", *args], cwd=tmp_path, check=True, capture_output=True, timeout=20)
    monkeypatch.chdir(tmp_path)
    return replace(
        make_context(Options()),
        invoked_at=datetime(2026, 9, 15, 20, 30, tzinfo=UTC),
        settings=load_settings(
            {"GITHUB_SHA": "unrelated merge", "GITHUB_RUN_ID": "12345", "GITHUB_EVENT_NAME": "push"}
        ),
    )


def test_preparation_publishes_complete_tree_and_actual_source(documentation: Context) -> None:
    root = documentation.root
    values = json.loads(DocsPrepare.run(documentation).message)
    assert values["sha"] == documentation.tools.git.revision()
    assert values["generated_at"] == "2026-09-15 20:30:00 UTC"
    deployed = root / "deploy"
    assert (deployed / "index.html").read_text() == "<h1>Book</h1>"
    assert {path.name for path in (deployed / "llm").iterdir()} == {
        "manifest.json",
        "schema.json",
        "summaries.json",
    }
    assert not (deployed / "CNAME").exists()
    assert (deployed / ".deployment-info").read_text() == (
        "Generated on: 2026-09-15 20:30:00 UTC\n"
        f"Source commit: {values['sha']}\nWorkflow run: 12345\nTrigger: push\n"
    )
    (root / "docs/book/removed.html").unlink()
    (root / "docs/CNAME").write_text("docs.example.invalid\n")
    DocsPrepare.run(replace(documentation, settings=load_settings({})))
    assert not (deployed / "removed.html").exists()
    assert (deployed / "CNAME").read_text() == "docs.example.invalid\n"
    assert "Trigger: local" in (deployed / ".deployment-info").read_text()


class FailedCopy(FileSystem):
    def copy_tree(self, source: Path, destination: Path) -> None:
        if source.name == "llm":
            raise OSError("injected copy failure")
        super().copy_tree(source, destination)


def test_copy_failure_preserves_previous_payload(documentation: Context) -> None:
    DocsPrepare.run(documentation)
    (documentation.root / "docs/book/index.html").write_text("Changed")
    with pytest.raises(OSError, match="copy failure"):
        DocsPrepare.run(replace(documentation, fs=FailedCopy()))
    assert (documentation.root / "deploy/index.html").read_text() == "<h1>Book</h1>"
    assert not tuple(path for path in documentation.root.glob(".rv-docs-*") if path.is_dir())


@pytest.mark.parametrize(
    "name",
    (
        "docs/book/index.html",
        "docs/llm/manifest.json",
        "docs/llm/schema.json",
        "docs/llm/summaries.json",
    ),
)
def test_missing_or_empty_producers_prevent_preparation(documentation: Context, name: str) -> None:
    (documentation.root / name).write_text("")
    with pytest.raises(ToolingError, match="before preparing"):
        DocsPrepare.run(documentation)
    (documentation.root / name).unlink()
    with pytest.raises(ToolingError, match="before preparing"):
        DocsPrepare.run(documentation)
    assert not (documentation.root / "deploy").exists()


@pytest.mark.parametrize(
    "name", ("docs/book/link", "docs/llm/link", "docs/CNAME", "docs/linked.md", "deploy")
)
def test_links_cannot_read_or_replace_other_files(documentation: Context, name: str) -> None:
    target = documentation.root / "outside"
    target.write_text("Unrelated data")
    (documentation.root / name).symlink_to(target)
    with pytest.raises(ToolingError, match=r"symlink|regular|untracked|checkout"):
        DocsPrepare.run(documentation)
    assert target.read_text() == "Unrelated data"


def test_deployment_requires_an_owned_untracked_directory(documentation: Context) -> None:
    deployed = documentation.root / "deploy"
    deployed.mkdir()
    (deployed / "unrelated").write_text("preserve")
    with pytest.raises(ToolingError, match="ownership record"):
        DocsPrepare.run(documentation)
    (deployed / ".deployment-info").write_text("previous deployment")
    subprocess.run(
        ["git", "add", "--force", "deploy"],
        cwd=documentation.root,
        check=True,
        capture_output=True,
        timeout=20,
    )
    with pytest.raises(ToolingError, match="untracked"):
        DocsPrepare.run(documentation)
    assert (deployed / "unrelated").read_text() == "preserve"


@pytest.mark.parametrize(
    "pattern",
    ("AKIA" + "A" * 16, "SECRET_" + "KEY", "token" + "=", "password" + "=", "x-api" + "-key"),
)
def test_secret_guard_preserves_patterns_without_echoing_values(
    documentation: Context, pattern: str
) -> None:
    (documentation.root / "docs/README.md").write_text(f"# Readme\n{pattern} hidden\n")
    with pytest.raises(ToolingError, match=r"docs/README\.md:2: secret-like pattern") as caught:
        DocsGuard.run(documentation)
    assert pattern not in str(caught.value)


@pytest.mark.parametrize("suffix", ("py", "ipynb", "html"))
def test_document_source_type_guard_applies_to_committed_paths(
    documentation: Context, suffix: str
) -> None:
    path = documentation.root / f"docs/unwanted.{suffix}"
    path.touch()
    subprocess.run(["git", "add", str(path)], check=True, capture_output=True, timeout=20)
    with pytest.raises(ToolingError, match="forbidden committed"):
        DocsGuard.run(documentation)
