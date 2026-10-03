"""Run build tasks against a small real Rust workspace and inspect artifacts.

The fixture programs produce the files that the orchestration must carry between
steps. Cargo, Ruff and mdBook run normally; no compiler success is fabricated.
"""

import hashlib
import json
import os
import shutil
import subprocess
import tomllib
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks.build import (
    ApiExport,
    Build,
    Check,
    Docs,
    DocsLinks,
    Fmt,
    FmtFix,
    ReleaseArtifacts,
    Sbom,
)


@pytest.fixture
def workspace(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    source = Path(__file__).resolve().parents[2]
    channel = tomllib.loads((source / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    monkeypatch.setenv("RUSTUP_TOOLCHAIN", channel)
    monkeypatch.delenv("CARGO_TARGET_DIR", raising=False)
    monkeypatch.chdir(tmp_path)
    subprocess.run(
        ["git", "init", "--initial-branch=main", str(tmp_path)],
        check=True,
        capture_output=True,
        timeout=10,
    )
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    shutil.copy(source / "tools/versions.toml", tmp_path / "tools/versions.toml")
    shutil.copy(source / "rust-toolchain.toml", tmp_path / "rust-toolchain.toml")
    cargo_bin = Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo"))) / "bin"
    monkeypatch.setenv("PATH", str(cargo_bin) + os.pathsep + os.environ.get("PATH", ""))
    (tmp_path / "tools/example.py").write_text("answer= 42\n")
    (tmp_path / "tests").mkdir()
    (tmp_path / "tests/__init__.py").touch()
    programs = {
        "revaer-app": 'fn main() { println!("release fixture"); }\n',
        "asset_sync": (
            "fn main() -> std::io::Result<()> { "
            'std::fs::create_dir_all("crates/revaer-ui/static/nexus")?; '
            'std::fs::write("crates/revaer-ui/static/nexus/fixture.txt", "asset") }\n'
        ),
        "revaer-api": (
            "fn main() -> std::io::Result<()> { "
            'std::fs::create_dir_all("docs/api")?; '
            'std::fs::write("docs/api/openapi.json", '
            'r#"{"openapi":"3.1.0","paths":{"/health":{}}}"#) }\n'
        ),
        "revaer-doc-indexer": (
            "fn main() -> std::io::Result<()> { "
            'std::fs::create_dir_all("artifacts")?; '
            'std::fs::write("artifacts/index.json", "[]") }\n'
        ),
    }
    for package, code in programs.items():
        crate = tmp_path / "crates" / package
        (crate / "src").mkdir(parents=True)
        (crate / "src/main.rs").write_text(code)
        manifest = f'[package]\nname = "{package}"\nversion = "0.1.0"\nedition = "2024"\n'
        if package == "revaer-api":
            manifest += '[[bin]]\nname = "generate_openapi"\npath = "src/main.rs"\n'
        (crate / "Cargo.toml").write_text(manifest)
    members = json.dumps([f"crates/{package}" for package in programs])
    (tmp_path / "Cargo.toml").write_text(f'[workspace]\nresolver = "3"\nmembers = {members}\n')
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"], check=True, capture_output=True, timeout=30
    )
    (tmp_path / ".gitignore").write_text(
        "target/\ndist/\nartifacts/\n.ruff_cache/\ndocs/api/\ncrates/revaer-ui/\n"
    )
    for arguments in (
        ("config", "user.name", "Tooling tests"),
        ("config", "user.email", "tests@example.invalid"),
        ("config", "commit.gpgsign", "false"),
        ("add", "."),
        ("commit", "-m", "Initial fixture"),
    ):
        subprocess.run(["git", *arguments], check=True, capture_output=True, timeout=10)
    return make_context(Options())


def test_builds_sync_assets_and_package_verified_release_bytes(workspace: Context) -> None:
    Build.run(workspace)
    assert (workspace.root / "crates/revaer-ui/static/nexus/fixture.txt").read_text() == "asset"
    Check.run(workspace)
    ReleaseArtifacts.run(workspace)
    binary = workspace.root / "dist/revaer-app"
    result = subprocess.run([str(binary)], text=True, capture_output=True, check=True, timeout=10)
    assert result.stdout == "release fixture\n"
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    assert (workspace.root / "dist/revaer-app.sha256").read_text() == f"{digest}  revaer-app\n"
    assert json.loads((workspace.root / "dist/openapi.json").read_text()) == {
        "openapi": "3.1.0",
        "paths": {"/health": {}},
    }
    Sbom.run(workspace)
    metadata = json.loads((workspace.root / "artifacts/sbom.json").read_text())
    assert {package["name"] for package in metadata["packages"]} == {
        "asset_sync",
        "revaer-api",
        "revaer-app",
        "revaer-doc-indexer",
    }


def test_format_check_fails_then_fix_formats_python_and_rust(workspace: Context) -> None:
    with pytest.raises(ToolingError):
        Fmt.run(workspace)
    FmtFix.run(workspace)
    Fmt.run(workspace)
    assert (workspace.root / "tools/example.py").read_text() == "answer = 42\n"


def test_failed_compilation_does_not_create_release_artifacts(workspace: Context) -> None:
    (workspace.root / "crates/revaer-app/src/main.rs").write_text("invalid Rust\n")
    subprocess.run(
        ["git", "commit", "-am", "Broken fixture"],
        cwd=workspace.root,
        check=True,
        capture_output=True,
        timeout=10,
    )
    with pytest.raises(ToolingError):
        ReleaseArtifacts.run(workspace)
    assert not (workspace.root / "dist").exists()


def test_udeps_uses_pinned_compiler_retains_evidence_and_detects_an_unused_dependency(
    workspace: Context,
) -> None:
    from revaer_tooling.tasks.build import Udeps

    source = Path(__file__).resolve().parents[2]
    shutil.copy2(source / "tools/versions.toml", workspace.root / "tools/versions.toml")
    Udeps.run(workspace)
    evidence = (workspace.root / "target/udeps-toolchain-evidence.txt").read_text()
    assert "toolchain=nightly-2026-06-13" in evidence
    assert "cargo-udeps-version=0.1.57" in evidence
    assert "commit-hash:" in evidence
    unused = workspace.root / "crates/unused"
    (unused / "src").mkdir(parents=True)
    (unused / "Cargo.toml").write_text(
        '[package]\nname = "fixture-unused"\nversion = "0.1.0"\nedition = "2024"\n'
    )
    (unused / "src/lib.rs").write_text("pub const VALUE: u32 = 42;\n")
    manifest = workspace.root / "Cargo.toml"
    manifest.write_text(manifest.read_text().replace("members = [", 'members = ["crates/unused", '))
    app = workspace.root / "crates/revaer-app/Cargo.toml"
    app.write_text(app.read_text() + '\n[dependencies]\nfixture-unused = { path = "../unused" }\n')
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"],
        cwd=workspace.root,
        capture_output=True,
        check=True,
        timeout=30,
    )
    with pytest.raises(ToolingError):
        Udeps.run(workspace)
    assert (workspace.root / "target/udeps-toolchain-evidence.txt").read_text() == evidence


@pytest.mark.parametrize("field", ["udeps_version", "udeps_toolchain"])
def test_udeps_rejects_pin_overrides_before_running_the_compiler(
    workspace: Context,
    field: str,
) -> None:
    from dataclasses import replace

    from revaer_tooling.tasks.build import Udeps

    source = Path(__file__).resolve().parents[2]
    shutil.copy2(source / "tools/versions.toml", workspace.root / "tools/versions.toml")
    settings = (
        replace(workspace.settings, udeps_version="wrong-version")
        if field == "udeps_version"
        else replace(workspace.settings, udeps_toolchain="wrong-version")
    )
    context = replace(workspace, settings=settings)
    with pytest.raises(ToolingError, match="must equal the repository pin"):
        Udeps.run(context)
    assert not (workspace.root / "target").exists()


def test_docs_build_then_index_are_both_exercised(workspace: Context) -> None:
    source = workspace.root / "docs/src"
    source.mkdir(parents=True)
    (workspace.root / "docs/book.toml").write_text('[book]\ntitle = "Fixture book"\nsrc = "src"\n')
    (source / "SUMMARY.md").write_text("# Summary\n\n- [Setup](setup.md)\n")
    (source / "setup.md").write_text("# Setup\n\nFixture documentation.\n")
    Docs.run(workspace)
    book = tomllib.loads((workspace.root / "docs/book.toml").read_text())
    assert "mermaid" in book["preprocessor"]
    assert (workspace.root / "docs/mermaid.min.js").is_file()
    assert (workspace.root / "docs/mermaid-init.js").is_file()
    assert "Fixture documentation" in (workspace.root / "docs/book/setup.html").read_text()
    assert json.loads((workspace.root / "artifacts/index.json").read_text()) == []


def test_policy_and_instruction_checks_follow_real_git_changes(workspace: Context) -> None:
    from dataclasses import replace

    from revaer_tooling.tasks.policy import InstructionDrift, SourcePolicy

    root = workspace.root
    assert SourcePolicy.run(workspace).message == "Policy checks passed"
    workflow = root / ".github/workflows/test.yml"
    workflow.parent.mkdir(parents=True)
    workflow.write_text("jobs: {test: {steps: [{uses: actions/checkout@v4}]}}\n")
    with pytest.raises(ToolingError, match="unpinned action"):
        SourcePolicy.run(workspace)
    workflow.write_text(
        "jobs: {test: {runs-on: ubuntu-latest, timeout-minutes: 30, "
        "steps: [{uses: ./.github/actions/setup-revaer, timeout-minutes: 20}, "
        "{run: rv tooling-check}]}}\n"
    )
    assert SourcePolicy.run(workspace).message == "Policy checks passed"
    with pytest.raises(ToolingError, match="Instruction drift"):
        InstructionDrift.run(workspace)
    instruction = root / ".github/instructions/devops.instructions.md"
    instruction.parent.mkdir()
    instruction.write_text("Fixture workflow policy.\n")
    context = replace(workspace, options=Options(base="0" * 40))
    assert InstructionDrift.run(context).message == "Instruction drift checks passed"
    source = root / "crates/revaer-app/src/main.rs"
    source.write_text("fn main() { unimplemented!() }\n")
    with pytest.raises(ToolingError, match="authored stub"):
        SourcePolicy.run(workspace)
    source.unlink()  # Tracked deletions have no source content to scan.
    SourcePolicy.run(workspace)


def test_asset_check_rejects_modified_staged_and_new_generated_files(workspace: Context) -> None:
    from revaer_tooling.tasks.build import CheckAssets, SyncAssets

    root = workspace.root
    ignore = root / ".gitignore"
    ignore.write_text(ignore.read_text().replace("crates/revaer-ui/\n", ""))
    SyncAssets.run(workspace)
    with pytest.raises(ToolingError, match="Generated UI assets"):
        workspace.tools.git.check_assets()
    for args in (
        ("add", "--force", "crates/revaer-ui/static/nexus/fixture.txt"),
        ("commit", "-m", "Track generated fixture"),
    ):
        subprocess.run(["git", *args], cwd=root, check=True, capture_output=True, timeout=10)
    CheckAssets.run(workspace)
    generator = root / "crates/asset_sync/src/main.rs"
    generator.write_text(generator.read_text().replace('"asset"', '"changed"'))
    with pytest.raises(ToolingError, match="Generated UI assets"):
        CheckAssets.run(workspace)
    subprocess.run(
        ["git", "add", "crates/revaer-ui/static/nexus/fixture.txt"],
        cwd=root,
        check=True,
        capture_output=True,
        timeout=10,
    )
    with pytest.raises(ToolingError, match="Generated UI assets"):
        workspace.tools.git.check_assets()
    subprocess.run(
        ["git", "commit", "-m", "Update generated fixture"],
        cwd=root,
        check=True,
        capture_output=True,
        timeout=10,
    )
    (root / "crates/revaer-ui/static/nexus/new-file").write_text("new asset\n")
    with pytest.raises(ToolingError, match="Generated UI assets"):
        workspace.tools.git.check_assets()


def test_lint_checks_real_compilation_and_rejects_production_panics(workspace: Context) -> None:
    from revaer_tooling.tasks.policy import LanguageLint

    root = workspace.root
    (root / "pyproject.toml").write_text('[tool.mypy]\nstrict = true\nfiles = ["tools"]\n')
    (root / "README.md").write_text("# Lint fixture\n")
    (root / "crates/revaer-app/src/lib.rs").write_text(
        "//! Library target in the fixture workspace.\n"
    )
    for manifest in (root / "crates").glob("*/Cargo.toml"):
        text = manifest.read_text()
        manifest.write_text(
            text.replace(
                'edition = "2024"\n',
                'edition = "2024"\nlicense = "MIT"\ndescription = "Lint fixture"\n'
                'repository = "https://example.invalid/revaer"\nreadme = "../../README.md"\n'
                'keywords = ["fixture"]\ncategories = ["development-tools"]\n',
            )
        )
    FmtFix.run(workspace)
    LanguageLint.run(workspace)
    source = root / "crates/revaer-app/src/main.rs"
    source.write_text('fn main() { panic!("not permitted in production"); }\n')
    with pytest.raises(ToolingError):
        LanguageLint.run(workspace)


def test_license_report_is_real_json_and_ignores_fail_closed(workspace: Context) -> None:
    from revaer_tooling.tasks.build import Audit, Deny, Licenses

    Licenses.run(workspace)
    report = json.loads((workspace.root / "artifacts/licenses.json").read_text())
    assert report
    (workspace.root / "deny.toml").write_text("[advisories]\nignore = []\n")
    (workspace.root / ".secignore").write_text("# documented format\nRUSTSEC-0000-0000\n")
    with pytest.raises(ToolingError, match="Advisory ignores"):
        Audit.run(workspace)
    (workspace.root / ".secignore").write_text("")
    (workspace.root / "deny.toml").write_text('[advisories]\nignore = ["RUSTSEC-0000-0000"]\n')
    with pytest.raises(ToolingError, match="Advisory ignores"):
        Deny.run(workspace)


def test_release_uses_cargos_configured_target_directory(
    workspace: Context,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    alternate = workspace.root / "target/alternate directory"
    monkeypatch.setenv("CARGO_TARGET_DIR", str(alternate))
    context = make_context(workspace.options)
    stale = workspace.root / "target/release/revaer-app"
    stale.parent.mkdir(parents=True)
    stale.write_bytes(b"stale binary must not be published")
    ReleaseArtifacts.run(context)
    assert (context.root / "dist/revaer-app").read_bytes() == (
        alternate / "release/revaer-app"
    ).read_bytes()
    assert (context.root / "dist/revaer-app").read_bytes() != stale.read_bytes()


def test_docs_links_install_pinned_checker_and_reject_a_broken_local_link(
    workspace: Context,
) -> None:
    docs = workspace.root / "docs"
    docs.mkdir()
    (docs / "README.md").write_text("# Documentation\n\n[Missing page](missing.md)\n")
    with pytest.raises(ToolingError):
        DocsLinks.run(workspace)
    (docs / "missing.md").write_text("# Resolved page\n")
    DocsLinks.run(workspace)


@pytest.mark.parametrize("payload", ("{}", "[]", "{", '{"openapi":"3.1.0","paths":{}}'))
def test_api_export_rejects_invalid_embedded_input(workspace: Context, payload: str) -> None:
    destination = workspace.root / "docs/api/openapi.json"
    destination.parent.mkdir(parents=True)
    destination.write_text(payload)
    with pytest.raises(ToolingError, match="OpenAPI export"):
        ApiExport.run(workspace)
    assert destination.read_text() == payload
    assert not (workspace.root / "target").exists()


def test_api_export_rejects_successful_empty_generator(workspace: Context) -> None:
    source = workspace.root / "crates/revaer-api/src/main.rs"
    source.write_text(
        source.read_text().replace('{"openapi":"3.1.0","paths":{"/health":{}}}', "{}")
    )
    with pytest.raises(ToolingError, match="nonempty API paths"):
        ApiExport.run(workspace)
