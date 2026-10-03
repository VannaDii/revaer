"""Native Cargo selection proves the exact qualifications run and missing tests fail."""

import subprocess
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import COMMANDS, make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.rust import Cargo
from revaer_tooling.settings import load_settings

LIBRARY = """
#[cfg(test)]
fn record(variable: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert!(cfg!(feature = "exercise"));
    let path = std::path::PathBuf::from(std::env::var(variable)?);
    let content = std::fs::read_to_string(&path)?;
    assert_eq!(content, "disposable fixture");
    std::fs::write(path.with_extension(name), content)?;
    println!("qualification:{name}");
    Ok(())
}

#[cfg(test)]
mod indexers {
    mod search_results {
        mod pool_proof_tests {
            #[test]
            fn application_pool_qualification() -> Result<(), Box<dyn std::error::Error>> {
                crate::record("REVAER_INGESTION_POOL_PROOF", "pool")
            }
            #[test]
            fn application_pool_qualification_decoy() {
                assert_eq!("selected exact test", "selected decoy");
            }
        }
        mod cancellation_proof_tests {
            #[test]
            fn application_cancellation_qualification() -> Result<(), Box<dyn std::error::Error>> {
                crate::record("REVAER_INGESTION_CANCELLATION_PROOF", "cancellation")
            }
        }
    }
}

#[cfg(test)]
mod baseline {
    #[test]
    fn application_baseline() -> Result<(), Box<dyn std::error::Error>> {
        assert!(cfg!(feature = "exercise"));
        assert_eq!(std::env::var("REVAER_TEST_DATABASE_URL")?, "postgres://tests.invalid/admin");
        let path = std::path::PathBuf::from(std::env::var("RV_PROBE_FIXTURE_OUTPUT")?);
        std::fs::write(path, "baseline")?;
        Ok(())
    }
}
"""


@pytest.fixture
def qualification(tmp_path: Path) -> Context:
    context = make_context(Options())
    (tmp_path / "src").mkdir()
    (tmp_path / "Cargo.toml").write_text(
        '[package]\nname = "revaer-data"\nversion = "0.1.0"\nedition = "2024"\n'
        '[workspace]\nresolver = "3"\n[features]\nexercise = []\n'
    )
    (tmp_path / "src/lib.rs").write_text(LIBRARY)
    (tmp_path / "input.json").write_text("disposable fixture")
    (tmp_path / "rust-toolchain.toml").write_bytes(
        (context.root / "rust-toolchain.toml").read_bytes()
    )
    original = context.tools.cargo
    environment = {
        key: value for key, value in original.environment.items() if key != "CARGO_TARGET_DIR"
    }
    environment["RV_PROBE_FIXTURE_OUTPUT"] = str(tmp_path / "baseline-result")
    cargo = Cargo(original.name, original.runner, tmp_path, environment)
    subprocess.run(
        [str(cargo.locate()), "generate-lockfile"],
        cwd=tmp_path,
        env=environment,
        capture_output=True,
        check=True,
        timeout=30,
    )
    database = replace(
        context.settings.database,
        pool_proof=Path("input.json"),
        cancellation_proof=tmp_path / "input.json",
        test_url="postgres://tests.invalid/admin",
    )
    return replace(
        context,
        root=tmp_path,
        tools=replace(context.tools, cargo=cargo),
        settings=replace(context.settings, database=database),
    )


@pytest.mark.parametrize(
    "command,suffix",
    (
        ("db-init-pool-probe", "pool"),
        ("db-init-cancellation-probe", "cancellation"),
    ),
)
def test_qualification_runs_only_the_exact_library_test(
    qualification: Context,
    command: str,
    suffix: str,
) -> None:
    # --lib must keep unrelated integration targets out of this qualification.
    # The near-name decoy in the library must also remain filtered out.
    tests = qualification.root / "tests"
    tests.mkdir()
    (tests / "unrelated.rs").write_text('compile_error!("unrelated integration target");')
    messages: list[str] = []
    context = replace(qualification, emit=messages.append)
    assert "qualification passed" in COMMANDS[command](context).message
    assert (context.root / f"input.{suffix}").read_text() == "disposable fixture"
    assert any(f"qualification:{suffix}" in message for message in messages)
    (context.root / "input.json").write_text("invalid")
    with pytest.raises(ToolingError):
        COMMANDS[command](context)


def test_missing_proof_input_fails_before_cargo(qualification: Context) -> None:
    context = replace(
        qualification,
        settings=replace(
            qualification.settings,
            database=replace(
                qualification.settings.database,
                pool_proof=None,
                cancellation_proof=None,
            ),
        ),
    )
    for command in ("db-init-pool-probe", "db-init-cancellation-probe"):
        with pytest.raises(ToolingError, match="explicit disposable proof input"):
            COMMANDS[command](context)
    assert not (context.root / "target").exists()


def test_cargo_zero_selected_tests_cannot_report_a_qualification(qualification: Context) -> None:
    (qualification.root / "src/lib.rs").write_text("")
    with pytest.raises(ToolingError, match="exactly one passing test"):
        COMMANDS["db-init-pool-probe"](qualification)


def test_baseline_filter_uses_all_workspace_features(qualification: Context) -> None:
    COMMANDS["test-database-baseline-read"](qualification)
    assert (qualification.root / "baseline-result").read_text() == "baseline"
    assert not (qualification.root / "input.pool").exists()
    assert not (qualification.root / "input.cancellation").exists()


def test_explicit_proof_settings_are_loaded_at_the_boundary() -> None:
    values = load_settings(
        {
            "REVAER_INGESTION_POOL_PROOF": "path with spaces/pool.json",
            "REVAER_INGESTION_CANCELLATION_PROOF": "/tmp/cancellation.json",
        }
    ).database
    assert values.pool_proof == Path("path with spaces/pool.json")
    assert values.cancellation_proof == Path("/tmp/cancellation.json")
    defaults = load_settings({}).database
    assert defaults.pool_proof is None and defaults.cancellation_proof is None
