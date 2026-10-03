"""Mutate reviewed container pins, runtime declarations and source boundaries."""

import shutil
import tempfile
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.packages import ApkInstallArgs
from revaer_tooling.images.inputs import (
    NATIVE_HASH_FIELDS,
    NATIVE_PROOF_FIELDS,
    BuildInputs,
    verify_project_pins,
)
from revaer_tooling.images.policy import EVIDENCE, LABELS, media_compliance_findings
from revaer_tooling.repository import checkout
from revaer_tooling.tasks.compliance import MediaComplianceGuardrails
from revaer_tooling.tasks.containers import container_inputs
from revaer_tooling.tasks.policy import Policy
from test_image_compliance import context as compliance_context

__all__ = ["compliance_context"]


@pytest.fixture
def context(compliance_context: Context) -> Context:
    source = Path(__file__).parents[2]
    for name in (
        "Dockerfile",
        "pyproject.toml",
        "rust-toolchain.toml",
        ".uv-version",
        ".python-version",
    ):
        shutil.copy(source / name, compliance_context.root / name)
    return compliance_context


def test_reviewed_manifest_and_complete_runtime_declaration_pass(context: Context) -> None:
    inputs = BuildInputs.load(context.fs, context.root)
    verify_project_pins(context.fs, context.root, inputs)
    assert inputs.uv_version == "0.12.13" and inputs.python_version == "3.13.12"
    assert not media_compliance_findings(context)
    assert "passed" in MediaComplianceGuardrails.run(context).message


def test_canonical_policy_blocks_inconsistent_runtime_pins(context: Context) -> None:
    source = Path(__file__).parents[2]
    for name in (".secignore", "deny.toml"):
        shutil.copy(source / name, context.root / name)
    manifest = context.root / ".github/build-inputs.env"
    manifest.write_text(manifest.read_text().replace("ffmpeg=8.0.1-r1", "ffmpeg=8.0.1-r2"))
    # Exercise the public policy composition with the real preceding checks.
    # A separate compliance command must not be the only way to catch drift.
    with pytest.raises(ToolingError, match="Runtime inventory must match the exact ffmpeg"):
        Policy.run(context)


@pytest.mark.parametrize(
    "line",
    (
        "UNKNOWN=1",
        "UV_VERSION=0.12.13",
        "UV_VERSION=",
        "UV_VERSION",
        "UV_VERSION=1 extra",
        'UV_VERSION="$UNEXPECTED"',
        'UV_VERSION="`false`"',
        'UV_VERSION="unterminated',
    ),
)
def test_manifest_rejects_unknown_duplicate_or_executable_assignments(
    context: Context, line: str
) -> None:
    path = context.root / ".github/build-inputs.env"
    path.write_text(path.read_text() + line + "\n")
    with pytest.raises((ToolingError, ValueError)):
        BuildInputs.load(context.fs, context.root)


@pytest.mark.parametrize(
    "old,new",
    (
        ("UV_VERSION=0.12.13\n", ""),
        ("UV_VERSION=0.12.13", "UV_VERSION=latest"),
        ("UV_VERSION=0.12.13", "UV_VERSION=0.12.12"),
        ("@sha256:b485", "@sha256:invalid"),
        ("ffmpeg=8.0.1-r1", "ffmpeg"),
    ),
)
def test_missing_unpinned_or_conflicting_versions_fail(
    context: Context, old: str, new: str
) -> None:
    path = context.root / ".github/build-inputs.env"
    path.write_text(path.read_text().replace(old, new))
    with pytest.raises(ToolingError):
        inputs = BuildInputs.load(context.fs, context.root)
        verify_project_pins(context.fs, context.root, inputs)


@pytest.mark.parametrize("label", LABELS)
def test_all_media_labels_remain_required(context: Context, label: str) -> None:
    path = context.root / "Dockerfile"
    path.write_text(
        "\n".join(
            line
            for line in path.read_text().splitlines()
            if not line.startswith(f"LABEL revaer.media.{label}=")
        )
    )
    with pytest.raises(ToolingError, match="media label"):
        MediaComplianceGuardrails.run(context)


@pytest.mark.parametrize("name", EVIDENCE)
def test_missing_compliance_artifacts_fail(context: Context, name: str) -> None:
    (context.root / "release/media-compliance" / name).unlink()
    with pytest.raises((ToolingError, OSError)):
        MediaComplianceGuardrails.run(context)


@pytest.mark.parametrize("change", ("version", "missing", "duplicate"))
def test_runtime_package_pins_must_agree_with_the_inventory(context: Context, change: str) -> None:
    path = context.root / ".github/build-inputs.env"
    replacement = {
        "version": "ffmpeg=8.0.1-r2",
        "missing": "",
        "duplicate": "ffmpeg=8.0.1-r1 ffmpeg=8.0.1-r2",
    }[change]
    path.write_text(path.read_text().replace("ffmpeg=8.0.1-r1", replacement))
    with pytest.raises(ToolingError):
        MediaComplianceGuardrails.run(context)


def test_changed_frontend_and_authored_nonfree_configuration_fail(context: Context) -> None:
    path = context.root / "Dockerfile"
    path.write_text(
        path.read_text().replace("# syntax=", "# wrong=") + "RUN configure --enable-nonfree\n"
    )
    failures = media_compliance_findings(context)
    assert any("exact reviewed pin" in failure for failure in failures)
    assert any("nonfree" in failure for failure in failures)


@pytest.mark.parametrize(
    "packages", ((), ("curl",), ("--help",), ("curl=1", "curl=2"), ("curl=1;command",))
)
def test_apk_requires_distinct_exact_package_pins(packages: tuple[str, ...]) -> None:
    with pytest.raises(ToolingError):
        ApkInstallArgs(packages).validate()


def test_container_preparation_refuses_the_developer_host(context: Context) -> None:
    with pytest.raises(ToolingError, match="Alpine build stage"):
        container_inputs(replace(context, host=replace(context.host, uid=1000)))
    if context.host.system != "linux":
        with pytest.raises(ToolingError, match="Alpine build stage"):
            container_inputs(replace(context, host=replace(context.host, uid=0)))


def test_source_archives_require_explicit_opt_in_and_complete_revaer_metadata() -> None:
    # Outside any Git checkout: an archive nested in another worktree must not
    # bypass the existing nearest-Git-boundary rule.
    with tempfile.TemporaryDirectory(prefix="rv-archive-", dir="/tmp") as temporary:
        root = Path(temporary)
        (root / "tools/src/revaer_tooling").mkdir(parents=True)
        (root / "tools/src/revaer_tooling/cli.py").touch()
        (root / "pyproject.toml").write_text('[project]\nname="revaer-tooling"\n')
        (root / "Cargo.toml").write_text('[workspace.package]\nedition="2024"\n')
        (root / "uv.lock").touch()
        with pytest.raises(ToolingError, match="Revaer checkout"):
            checkout(root)
        assert checkout(root / "tools", source_archive=True) == root
        (root / "uv.lock").unlink()
        with pytest.raises(ToolingError, match="locked Rust 2024"):
            checkout(root, source_archive=True)
        (root / "uv.lock").touch()
        for text in ('[workspace.package]\nedition="2021"\n', "workspace=1", "[invalid"):
            (root / "Cargo.toml").write_text(text)
            with pytest.raises(ToolingError):
                checkout(root, source_archive=True)
        nested = root / "foreign"
        (nested / ".git").mkdir(parents=True)
        with pytest.raises(ToolingError, match="does not contain rv"):
            checkout(nested, source_archive=True)
    assert Options().source_archive is False


def native_proof_values() -> dict[str, str]:
    """Synthetic public pins exercise shape checks without fetching debugger tools."""
    return {
        **dict.fromkeys(NATIVE_HASH_FIELDS, "a" * 64),
        "POSTGRES_NATIVE_DEBUGGER_IMAGE": "sha256:" + "b" * 64,
        "POSTGRES_NATIVE_ALPINE_VERSION": "3.24.1",
        "POSTGRES_NATIVE_APK_PACKAGES": "gdb=16.3-r4 musl-dbg=1.2.6-r2",
        "POSTGRES_NATIVE_MUSL_SOURCE_URL": "https://musl.libc.org/releases/musl-1.2.6.tar.gz",
    }


def append_native_proof(context: Context, values: dict[str, str]) -> None:
    path = context.root / ".github/build-inputs.env"
    path.write_text(
        path.read_text()
        + "\n"
        + "\n".join(f'{name}="{value}"' for name, value in values.items())
        + "\n"
    )


def test_shared_manifest_accepts_complete_native_debugger_pins(context: Context) -> None:
    before = BuildInputs.load(context.fs, context.root)
    append_native_proof(context, native_proof_values())
    assert BuildInputs.load(context.fs, context.root) == before


@pytest.mark.parametrize("missing", sorted(NATIVE_PROOF_FIELDS))
def test_shared_manifest_rejects_partial_native_debugger_pins(
    context: Context, missing: str
) -> None:
    values = native_proof_values()
    del values[missing]
    append_native_proof(context, values)
    with pytest.raises(ToolingError, match="Missing native PostgreSQL inputs"):
        BuildInputs.load(context.fs, context.root)


@pytest.mark.parametrize("field", sorted(NATIVE_PROOF_FIELDS))
def test_shared_manifest_rejects_unpinned_native_debugger_values(
    context: Context, field: str
) -> None:
    values = native_proof_values()
    values[field] = "unpinned"
    append_native_proof(context, values)
    with pytest.raises(ToolingError):
        BuildInputs.load(context.fs, context.root)


@pytest.mark.parametrize(
    "url",
    (
        "http://example.invalid/source.tar.gz",
        "https://example.invalid/source?token=x",
        "https://example.invalid/source#fragment",
        "https://user@example.invalid/source",
        "https://[broken/source",
        "https://example.invalid:bad/source",
        "https://example.invalid:0/source",
        "https://example.invalid:65536/source",
        "https://example.invalid",
    ),
)
def test_shared_manifest_rejects_unsafe_native_source_urls(context: Context, url: str) -> None:
    values = native_proof_values()
    values["POSTGRES_NATIVE_MUSL_SOURCE_URL"] = url
    append_native_proof(context, values)
    with pytest.raises(ToolingError):
        BuildInputs.load(context.fs, context.root)
