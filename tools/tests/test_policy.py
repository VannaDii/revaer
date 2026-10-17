"""Policy fixtures protect forbidden constructs and the documented boundaries."""

import pytest
from revaer_tooling.policy.isolation import isolation_findings
from revaer_tooling.tasks.policy import drift_findings, rust_findings


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
