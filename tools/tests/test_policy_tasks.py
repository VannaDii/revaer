"""Exercise policy task boundaries with real Git inventories and reviewed inputs."""

import json
import subprocess
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks.build import Audit, Deny
from revaer_tooling.tasks.policy import AdvisoryPolicy, SourcePolicy, rust_findings
from revaer_tooling.tasks.workflows import WorkflowPolicy


def test_generated_cxx_documentation_exception_cannot_cover_other_code() -> None:
    path = "crates/revaer-torrent-libt/src/ffi/bridge.rs"
    exception = "#[allow(clippy::missing_errors_doc)]\n"
    bridge = '#[cxx::bridge(namespace = "revaer")]\npub mod ffi {}\n'
    assert not rust_findings(path, exception + bridge)
    assert rust_findings("crates/other/src/lib.rs", exception + bridge)
    assert rust_findings(path, exception + "pub fn authored() {}\n")
    assert rust_findings(path, (exception + bridge) * 2)
    assert rust_findings(path, exception.replace("missing_errors_doc", "all") + bridge)
    assert rust_findings(path, exception + bridge + exception + "pub fn authored() {}\n")


def test_workflow_task_reads_real_contract_inputs(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    fixture = Path(__file__).parent / "fixtures/workflow-contracts.json"
    record = json.loads(fixture.read_text())
    subprocess.run(["git", "init", str(tmp_path)], check=True, capture_output=True, timeout=10)
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    for name, document in record["documents"].items():
        path = tmp_path / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(document))  # JSON is a strict subset of YAML.
    (tmp_path / ".github/matrices").mkdir()
    (tmp_path / ".github/matrices/build-images.json").write_text(json.dumps(record["matrix"]))
    (tmp_path / "config").mkdir()
    contexts = tmp_path / "config/required-pr-checks.txt"
    contexts.write_text("\n".join(record["required_contexts"]) + "\n")
    monkeypatch.chdir(tmp_path)
    context = make_context(Options())
    WorkflowPolicy.run(context)
    # Adding the media configuration tightens this same foundation task; it
    # does not require a second implementation on the branch rebased onto it.
    phase = tmp_path / "config/database-rebaseline.env"
    phase.touch()
    with pytest.raises(ToolingError, match="recognized transition phase"):
        WorkflowPolicy.run(context)
    phase.write_text("TRANSITION_PHASE=finalization\n")
    WorkflowPolicy.run(context)
    contexts.write_text("only-one-context\n")
    with pytest.raises(ToolingError, match=r"required.*check|context"):
        WorkflowPolicy.run(context)


@pytest.mark.parametrize(
    ("secignore", "deny"),
    [
        ("RUSTSEC-2099-0001\n", "[advisories]\nignore = []\n"),
        ("", '[advisories]\nignore = ["RUSTSEC-2099-0001"]\n'),
        ("", "[advisories]\n"),
    ],
)
@pytest.mark.parametrize("task", (AdvisoryPolicy, Audit, Deny))
def test_direct_audit_entrypoints_share_exact_exception_policy(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    task: type[AdvisoryPolicy | Audit | Deny],
    secignore: str,
    deny: str,
) -> None:
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    (tmp_path / ".secignore").write_text(secignore)
    # A missing key used to be treated as an empty ignore list. Reject it at
    # the task boundary before invoking an external audit with weaker policy.
    (tmp_path / "deny.toml").write_text(deny)
    monkeypatch.chdir(tmp_path)
    with pytest.raises(ToolingError, match="Advisory ignores are forbidden"):
        task.run(make_context(Options()))


def test_suppression_scan_includes_new_files_and_preserves_operational_boundaries(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    subprocess.run(["git", "init", str(tmp_path)], check=True, capture_output=True, timeout=10)
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    postgres = tmp_path / "crates/revaer-test-support/src/postgres.rs"
    postgres.parent.mkdir(parents=True)
    postgres.write_text('sqlx::query("SELECT pg_terminate_backend($1)")\n')
    monkeypatch.chdir(tmp_path)
    context = make_context(Options())
    SourcePolicy.run(context)
    source = tmp_path / "tools/example.py"
    source.write_text("value = 1  # NO" + "SONAR\n")
    with pytest.raises(ToolingError, match=r"tools/example\.py:1: Sonar suppression"):
        SourcePolicy.run(context)
