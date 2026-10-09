"""Prove Cargo feature selection and test failure propagation with real tests."""

import json
import subprocess
import tomllib
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.rust import Cargo
from revaer_tooling.tasks.testing import (
    LintRuntimeShutdown,
    TestMediaRecovery,
    TestMediaServiceRecovery,
    TestRuntimeShutdown,
)
from revaer_tooling.tasks.testing import Test as RustTest
from revaer_tooling.tasks.testing import TestFeaturesMinimal as MinimalFeatures
from revaer_tooling.tasks.testing import TestNative as NativeTest


@pytest.fixture
def test_workspace(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    source = Path(__file__).resolve().parents[2]
    channel = tomllib.loads((source / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    monkeypatch.setenv("RUSTUP_TOOLCHAIN", channel)
    monkeypatch.delenv("CARGO_TARGET_DIR", raising=False)
    monkeypatch.delenv("REVAER_NATIVE_IT", raising=False)
    monkeypatch.setenv("REVAER_TEST_DATABASE_URL", "postgres://tests.invalid/admin")
    monkeypatch.setenv("DATABASE_URL", "postgres://tests.invalid/application")
    monkeypatch.setenv("RV_FIXTURE_OUTPUT", str(tmp_path / "observations"))
    monkeypatch.chdir(tmp_path)
    (tmp_path / "observations").mkdir()
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    packages = ("revaer-api", "revaer-app", "revaer-torrent-libt")
    (tmp_path / "Cargo.toml").write_text(
        f'[workspace]\nresolver = "3"\nmembers = {json.dumps(packages)}\n'
    )
    for package in packages:
        crate = tmp_path / package
        (crate / "src").mkdir(parents=True)
        (crate / "Cargo.toml").write_text(
            f'[package]\nname = "{package}"\nversion = "0.1.0"\nedition = "2024"\n'
            '[features]\ndefault = ["extended"]\nextended = []\n'
        )
        (crate / "src/lib.rs").write_text(
            r"""#[cfg(test)]
mod tests {
    #[test]
    fn selected_inputs_reach_real_tests() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(std::env::var("REVAER_TEST_DATABASE_URL")?, "postgres://tests.invalid/admin");
        assert_eq!(std::env::var("DATABASE_URL")?, "postgres://tests.invalid/application");
        assert!(std::env::var_os("RV_FIXTURE_FAIL").is_none(), "injected test failure");
        let native = std::env::var("REVAER_NATIVE_IT").unwrap_or_default();
        let path = std::path::PathBuf::from(std::env::var("RV_FIXTURE_OUTPUT")?)
            .join(env!("CARGO_PKG_NAME"));
        std::fs::write(path, format!("extended={};native={native}", cfg!(feature = "extended")))?;
        Ok(())
    }
}
"""
        )
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"], check=True, capture_output=True, timeout=30
    )
    return make_context(Options())


def observations(context: Context) -> dict[str, str]:
    return {path.name: path.read_text() for path in (context.root / "observations").iterdir()}


def test_native_service_recipe_requires_linux_and_one_real_selection(
    test_workspace: Context,
) -> None:
    unsupported = replace(test_workspace, host=replace(test_workspace.host, system="darwin"))
    with pytest.raises(ToolingError, match="requires Linux"):
        TestMediaServiceRecovery.run(unsupported)
    selected = replace(test_workspace, host=replace(test_workspace.host, system="linux"))
    with pytest.raises(ToolingError, match="did not execute one passing test"):
        TestMediaServiceRecovery.run(selected)
    source = test_workspace.root / "revaer-app/src/lib.rs"
    source.write_text(r"""
#[cfg(test)] mod bootstrap { mod service_recovery_tests {
    #[test] #[ignore = "explicit selection"]
    fn native_service_shutdown_resumes_active_ffmpeg() -> Result<(), Box<dyn std::error::Error>> {
        assert!(cfg!(feature = "extended"));
        assert_eq!(std::env::var("REVAER_TEST_DATABASE_URL")?, "postgres://tests.invalid/admin");
        Ok(())
    }
} }
""")
    TestMediaServiceRecovery.run(selected)
    source.write_text(source.read_text().replace("Ok(())", 'Err("fixture failure".into())'))
    with pytest.raises(ToolingError):
        TestMediaServiceRecovery.run(selected)


def test_variants_execute_the_required_packages_and_feature_sets(test_workspace: Context) -> None:
    RustTest.run(test_workspace)
    assert observations(test_workspace) == dict.fromkeys(
        ("revaer-api", "revaer-app", "revaer-torrent-libt"), "extended=true;native="
    )
    for path in (test_workspace.root / "observations").iterdir():
        path.unlink()
    NativeTest.run(test_workspace)
    assert observations(test_workspace) == {"revaer-torrent-libt": "extended=true;native=1"}
    (test_workspace.root / "observations/revaer-torrent-libt").unlink()
    MinimalFeatures.run(test_workspace)
    assert observations(test_workspace) == dict.fromkeys(
        ("revaer-api", "revaer-app"), "extended=false;native="
    )


def test_a_failed_rust_test_is_a_failed_task(test_workspace: Context) -> None:
    original = test_workspace.tools.cargo
    cargo = Cargo(
        original.name,
        original.runner,
        original.root,
        {**original.environment, "RV_FIXTURE_FAIL": "1"},
    )
    context = replace(test_workspace, tools=replace(test_workspace.tools, cargo=cargo))
    with pytest.raises(ToolingError) as failure:
        RustTest.run(context)
    assert failure.value.exit_code != 0
    assert not observations(context)


def test_missing_disposable_database_fails_before_launching_cargo(test_workspace: Context) -> None:
    context = replace(
        test_workspace,
        settings=replace(
            test_workspace.settings,
            database=replace(test_workspace.settings.database, test_url=None),
        ),
    )
    with pytest.raises(ToolingError, match="disposable test database"):
        RustTest.run(context)
    assert not (context.root / "target").exists()


def test_shutdown_executes_both_modes_and_rejects_missing_tests(test_workspace: Context) -> None:
    crate = test_workspace.root / "revaer-app"
    source = crate / "src/lib.rs"
    source.write_text(r"""
#[cfg(test)]
mod bootstrap {
    mod shutdown_tests {
        #[test]
        fn shutdown_observed() -> Result<(), Box<dyn std::error::Error>> {
            let mode = if cfg!(feature = "extended") { "all" } else { "minimal" };
            let root = std::path::PathBuf::from(std::env::var("RV_FIXTURE_OUTPUT")?);
            std::fs::write(root.join(mode), "shutdown executed")?;
            Ok(())
        }
    }
}
""")
    TestRuntimeShutdown.run(test_workspace)
    assert observations(test_workspace) == {
        "all": "shutdown executed",
        "minimal": "shutdown executed",
    }
    source.write_text("")
    with pytest.raises(ToolingError, match="did not execute"):
        TestRuntimeShutdown.run(test_workspace)


def test_shutdown_failure_propagates(test_workspace: Context) -> None:
    (test_workspace.root / "revaer-app/src/lib.rs").write_text(
        "#[cfg(test)] mod bootstrap { mod shutdown_tests { #[test] "
        'fn fails() -> Result<(), String> { Err("injected failure".into()) } } }'
    )
    with pytest.raises(ToolingError):
        TestRuntimeShutdown.run(test_workspace)


def test_shutdown_clippy_checks_minimal_configuration(test_workspace: Context) -> None:
    crate = test_workspace.root / "revaer-app"
    # Clippy inspects metadata for every workspace member, even with -p.
    for manifest in test_workspace.root.glob("*/Cargo.toml"):
        manifest.write_text(
            manifest.read_text().replace(
                "[features]",
                'description="Native shutdown fixture"\nlicense="MIT"\n'
                'repository="https://example.invalid/revaer"\nreadme="README.md"\n'
                'keywords=["testing"]\ncategories=["development-tools"]\n[features]',
            )
        )
        (manifest.parent / "README.md").write_text("Shutdown fixture")
    (crate / "src/lib.rs").write_text("")
    LintRuntimeShutdown.run(test_workspace)
    # This warning exists only without default features: the second mode must
    # actually compile, and -D warnings must propagate its failure to the task.
    (crate / "src/lib.rs").write_text('#[cfg(not(feature="extended"))] fn unused_minimal_only() {}')
    with pytest.raises(ToolingError):
        LintRuntimeShutdown.run(test_workspace)


@pytest.mark.parametrize("system", ["linux", "darwin"])
def test_media_recovery_requires_actual_tests_and_propagates_failure(
    test_workspace: Context,
    system: str,
) -> None:
    test_workspace = replace(test_workspace, host=replace(test_workspace.host, system=system))
    source = test_workspace.root / "revaer-app/src/lib.rs"
    source.write_text(r"""
#[cfg(test)] mod media_discovery_fingerprint { mod tests {
    #[test] fn cancellable_source_read() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!cfg!(feature = "extended"));
        std::fs::write(std::path::PathBuf::from(std::env::var("RV_FIXTURE_OUTPUT")?)
            .join("fingerprint"), "executed")?;
        Ok(())
    }
} }
#[cfg(test)] mod media {
    #[test] fn native_association_yield() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!cfg!(feature = "extended"));
        std::fs::write(std::path::PathBuf::from(std::env::var("RV_FIXTURE_OUTPUT")?)
            .join("admission"), "executed")?;
        Ok(())
    }
}
#[cfg(test)] mod media_job_runtime { mod tests {
    #[test] #[ignore = "explicit production-runtime selection"]
    fn production_media_job_runtime_executes_and_persists_verified_replacement()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(cfg!(feature = "extended"));
        std::fs::write(std::path::PathBuf::from(std::env::var("RV_FIXTURE_OUTPUT")?)
            .join("production"), "executed")?;
        Ok(())
    }
    #[test] fn media_job_runtime_interrupted() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!cfg!(feature = "extended"));
        std::fs::write(std::path::PathBuf::from(std::env::var("RV_FIXTURE_OUTPUT")?)
            .join("recovery"), "executed")?;
        Ok(())
    }
} }
#[cfg(test)] mod bootstrap { mod root_catalog { mod native { mod tests {
    #[test] fn retained_inventory() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!cfg!(feature = "extended"));
        std::fs::write(std::path::PathBuf::from(std::env::var("RV_FIXTURE_OUTPUT")?)
            .join("native-roots"), "executed")?;
        Ok(())
    }
} } } }
""")
    runtime = test_workspace.root / "revaer-media-runtime"
    (runtime / "src").mkdir(parents=True)
    (runtime / "Cargo.toml").write_text(
        '[package]\nname="revaer-media-runtime"\nversion="0.1.0"\nedition="2024"\n'
    )
    (runtime / "src/lib.rs").write_text(r"""
#[cfg(test)] mod workspace { mod tests {
    #[test] fn reopen_stopped_workspace() -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(std::path::PathBuf::from(std::env::var("RV_FIXTURE_OUTPUT")?)
            .join("workspace"), "executed")?;
        Ok(())
    }
} }
""")
    manifest = test_workspace.root / "Cargo.toml"
    data = test_workspace.root / "revaer-data"
    (data / "src").mkdir(parents=True)
    (data / "Cargo.toml").write_text(
        '[package]\nname="revaer-data"\nversion="0.1.0"\nedition="2024"\n'
    )
    (data / "src/lib.rs").write_text(r"""
#[cfg(test)] mod media {
    #[test] fn stopped_attempt() -> Result<(), Box<dyn std::error::Error>> {
        std::fs::write(std::path::PathBuf::from(std::env::var("RV_FIXTURE_OUTPUT")?)
            .join("data"), "executed")?;
        Ok(())
    }
}
""")
    manifest.write_text(
        manifest.read_text().replace(
            '"revaer-app"', '"revaer-app", "revaer-media-runtime", "revaer-data"'
        )
    )
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"], check=True, capture_output=True, timeout=30
    )
    TestMediaRecovery.run(test_workspace)
    expected = {
        "production": "executed",
        "fingerprint": "executed",
        "admission": "executed",
        "recovery": "executed",
        "workspace": "executed",
        "data": "executed",
    }
    if test_workspace.host.system == "linux":
        expected["native-roots"] = "executed"
    assert observations(test_workspace) == expected
    source.write_text("")
    with pytest.raises(ToolingError, match="ran no passing tests"):
        TestMediaRecovery.run(test_workspace)
    source.write_text(
        "#[cfg(test)] mod media_job_runtime { mod tests { #[test] "
        'fn media_job_runtime_failure() -> Result<(), String> { Err("failure".into()) } } }'
    )
    with pytest.raises(ToolingError):
        TestMediaRecovery.run(test_workspace)
