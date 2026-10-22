"""Policy fixtures protect forbidden constructs and the documented boundaries."""

from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.git import Git
from revaer_tooling.policy.isolation import isolation_findings
from revaer_tooling.tasks.policy import InstructionDrift, drift_findings, rust_findings


@pytest.mark.parametrize(
    "source",
    [
        "#[allow(dead_code)]",
        "#![expect(unused)]",
        "todo!()",
        "unimplemented!()",
        'sqlx::query_scalar("SELECT 1")',
        "query_as!(Thing, query)",
        'let sql = "UPDATE things SET value = 4";',
        'let sql = "INSERT INTO things VALUES (4)";',
        'let sql = "CREATE TABLE things (id int)";',
        "std::panic::catch_unwind(operation)",
        "unsafe extern fn boundary() {}",
    ],
)
def test_rust_policy_rejects_forbidden_constructs(source: str) -> None:
    findings = rust_findings("crates/revaer-app/src/lib.rs", source)
    assert len(findings) == 1
    assert findings[0].startswith("crates/revaer-app/src/lib.rs:1:")


def test_documented_boundaries_are_precise() -> None:
    assert rust_findings("crates/revaer-data/src/lib.rs", 'sqlx::query("SELECT procedure()")') == []
    assert rust_findings("crates/revaer-torrent-libt/src/ffi.rs", "unsafe { call() }") == []
    assert (
        rust_findings("crates/revaer-torrent-libt/src/ffi/callback.rs", "catch_unwind(call)") == []
    )
    assert rust_findings("crates/revaer-data/src/tests.rs", '"CREATE TABLE fixture"') == []
    assert rust_findings("crates/revaer-data/src/lib.rs", '"CREATE TABLE application_state"')
    assert rust_findings("crates/revaer-torrent-libt/src/ffi_neighbour.rs", "unsafe { call() }")
    assert rust_findings("crates/revaer-data/src/tests.rs", "#[allow(unused)]")


def test_instruction_drift_covers_nested_automation_and_python_sources() -> None:
    for changed in (
        (".github/actions/nested/setup/action.yml",),
        ("release/scripts/nested/command.py",),
        ("tools/src/revaer_tooling/tasks/new.py",),
        ("sonar-project.properties",),
    ):
        assert drift_findings(changed)
        assert not drift_findings((*changed, ".github/instructions/devops.instructions.md"))
    assert not drift_findings(("docs/guide.md",))


@pytest.mark.parametrize(
    "path", ("release/package-lock.json", "release/package.json", "uv.lock", "pyproject.toml")
)
def test_only_dependabot_dependency_changes_skip_instruction_updates(path: str) -> None:
    assert drift_findings((path,))
    assert not drift_findings((path,), dependabot=True)


@pytest.mark.parametrize(
    "path",
    (
        "release/scripts/publish.py",
        ".github/workflows/pr.yml",
        "tools/src/revaer_tooling/tasks/policy.py",
    ),
)
def test_dependabot_mixed_changes_keep_instruction_requirements(path: str) -> None:
    assert drift_findings(("release/package-lock.json", path), dependabot=True)


@pytest.mark.parametrize(
    ("event", "author", "head_repository", "permitted"),
    (
        ("pull_request", "dependabot[bot]", "VannaDii/revaer", True),
        ("pull_request", "dependabot[bot]", "vannadii/Revaer", True),
        ("pull_request", "dependabot[bot]", "attacker/revaer", False),
        ("pull_request", "dependabot", "VannaDii/revaer", False),
        ("pull_request", "human", "VannaDii/revaer", False),
        ("push", "dependabot[bot]", "VannaDii/revaer", False),
        ("pull_request", "dependabot[bot]", "", False),
    ),
)
def test_instruction_drift_uses_pr_identity_instead_of_rerun_actor(
    monkeypatch: pytest.MonkeyPatch, event: str, author: str, head_repository: str, permitted: bool
) -> None:
    for name, value in {
        "GITHUB_EVENT_NAME": event,
        "GITHUB_ACTOR": "human-rerunning-the-check",
        "REVAER_INSTRUCTION_PR_AUTHOR": author,
        "REVAER_INSTRUCTION_PR_HEAD_REPOSITORY": head_repository,
    }.items():
        monkeypatch.setenv(name, value)
    context = make_context(Options())
    monkeypatch.setattr(
        Git, "changed_files", lambda self, base, head: ("release/package-lock.json",)
    )
    if permitted:
        assert InstructionDrift.run(context).message == "Instruction drift checks passed"
    else:
        with pytest.raises(ToolingError, match="Instruction drift"):
            InstructionDrift.run(context)


def test_privileged_dependabot_workflow_executes_only_trusted_code() -> None:
    from revaer_tooling.policy.formats import yaml_document
    from revaer_tooling.policy.workflows import mapping, steps

    source = Path(__file__).parents[2] / ".github/workflows/dependabot.yml"
    document = yaml_document(source.read_text(), str(source))
    triggers = mapping(document["on"])
    assert set(triggers) == {"workflow_run", "workflow_dispatch", "push", "schedule"}
    assert mapping(triggers["workflow_run"]) == {"workflows": ["PR"], "types": ["completed"]}
    job = mapping(mapping(document["jobs"])["auto-merge"])
    assert job["if"] == "github.ref == 'refs/heads/main' && github.ref_protected"
    assert set(mapping(document["permissions"]).values()) == {"read"}
    assert "permissions" not in job
    entries = steps(job)
    assert mapping(entries[0]["with"]) == {
        "persist-credentials": "false",
    }
    assert entries[1]["uses"] == "./.github/actions/setup-revaer"
    assert (
        entries[2]["uses"]
        == "actions/create-github-app-token@64ab729a6294000f5aa4a4bc0b73d1f32bc45510"
    )
    assert mapping(entries[2]["with"]) == {
        "client-id": "${{ vars.REVAER_APP_CLIENT_ID }}",
        "private-key": "${{ secrets.REVAER_APP_PRIVATE_KEY }}",
        "owner": "${{ github.repository_owner }}",
        "repositories": "${{ github.event.repository.name }}",
        "permission-contents": "write",
        "permission-pull-requests": "write",
        "permission-actions": "read",
        "permission-checks": "read",
        "permission-statuses": "read",
        "permission-workflows": "write",
    }
    assert "DEPENDABOT_AUTOMATION_ENABLED == 'true'" in str(entries[2]["if"])
    assert mapping(entries[3]["env"])["GH_TOKEN"] == "${{ github.token }}"
    assert entries[3]["run"] == "uv run --locked --no-build --no-sync -- rv dependabot-auto-merge"
    assert mapping(entries[4]["env"])["GH_TOKEN"] == "${{ steps.merge-token.outputs.token }}"
    assert (
        entries[4]["run"]
        == "uv run --locked --no-build --no-sync -- rv dependabot-auto-merge --apply"
    )
    assert entries[4]["if"] == entries[2]["if"]
    assert "DEPENDABOT_AUTOMATION_ENABLED != 'true'" in str(entries[3]["if"])
    assert (
        mapping(mapping(mapping(triggers["workflow_dispatch"])["inputs"])["dry_run"])["default"]
        == "true"
    )
    assert all(
        "REVAER_DEPENDABOT_REFRESH_TOKEN" not in mapping(step.get("env")) for step in entries[:4]
    )
    assert len(entries) == 5


@pytest.mark.parametrize(
    "source",
    [
        "losetup --find disk.img",
        "mkfs.ext4 disk.img",
        '"loop,nodev,nosuid"',
        "--device=/dev/sda",
        "/dev/loop0",
        "--privileged",
        "privileged: true",
        "CAP_SYS_ADMIN",
    ],
)
def test_isolation_policy_rejects_device_provisioning(source: str) -> None:
    for path in (
        "tools/src/revaer_tooling/tasks/e2e.py",
        ".github/workflows/pr.yml",
        "crates/revaer-app/src/bootstrap.rs",
    ):
        assert isolation_findings(path, source)


def test_isolation_policy_allows_ordinary_ipc_and_storage_observations() -> None:
    source = '"127.0.0.1:7070 /proc/self/mountinfo /dev/null"; Stdio::piped(); UnixStream::pair();'
    source += 'symlink("loop", path);'
    assert not isolation_findings("crates/revaer-app/src/bootstrap.rs", source)
    assert not isolation_findings("tools/tests/test_policy.py", "losetup disk.img")


@pytest.mark.parametrize(
    ("path", "before", "after", "allowed"),
    [
        (
            ".github/actions/setup/action.yml",
            "runs:\n  steps:\n    - uses: owner/action@" + "a" * 40,
            "runs:\n  steps:\n    - uses: owner/action@" + "b" * 40,
            True,
        ),
        (
            ".github/workflows/pr.yml",
            "permissions: {contents: read}",
            "permissions: {contents: write}",
            False,
        ),
        (
            ".github/workflows/pr.yml",
            "jobs: {test: {steps: [{run: safe}]}}",
            "jobs: {test: {steps: [{run: unsafe}]}}",
            False,
        ),
        (
            "Dockerfile",
            "ARG UV_IMAGE=ghcr.io/astral-sh/uv:0.12.13@sha256:" + "a" * 64,
            "ARG UV_IMAGE=ghcr.io/astral-sh/uv:0.12.19@sha256:" + "b" * 64,
            True,
        ),
        ("Dockerfile", "RUN safe", "RUN unsafe", False),
        (
            "Dockerfile",
            "ARG UV_IMAGE=owner/image@sha256:" + "a" * 64,
            "ARG UV_IMAGE=other/image@sha256:" + "b" * 64,
            False,
        ),
        (
            "rust-toolchain.toml",
            '[toolchain]\nchannel="1.96.0"',
            '[toolchain]\nchannel="1.97.0"',
            True,
        ),
        (
            "rust-toolchain.toml",
            '[toolchain]\nchannel="1.96.0"',
            '[toolchain]\nchannel="nightly"',
            False,
        ),
        (
            "tools/launcher/pyproject.toml",
            '[build-system]\nrequires=["uv_build==0.12.13"]',
            '[build-system]\nrequires=["uv_build==0.12.19"]',
            True,
        ),
        (
            "tools/launcher/pyproject.toml",
            '[project]\nname="launcher"',
            '[project]\nname="other"',
            False,
        ),
    ],
)
def test_dependency_reference_exemption_preserves_behavior(
    path: str, before: str, after: str, allowed: bool
) -> None:
    from revaer_tooling.policy.dependency_updates import reference_only

    assert reference_only(path, before, after) is allowed
    references = frozenset((path,)) if allowed else frozenset()
    if path.startswith((".github/", "tools/")) or path == "Dockerfile":
        assert bool(drift_findings((path,), dependabot=True, references=references)) is not allowed


def test_action_reference_exemption_does_not_normalize_environment_values() -> None:
    from revaer_tooling.policy.dependency_updates import reference_only

    before = "env: {uses: 'owner/action@" + "a" * 40 + "'}"
    after = "env: {uses: 'owner/action@" + "b" * 40 + "'}"
    assert not reference_only(".github/workflows/pr.yml", before, after)
