//! Process-boundary regressions for read-only baseline verification.

use std::process::Command;

#[test]
fn missing_endpoint_fails_without_selecting_an_implicit_database() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_verify_database_baseline"))
        .env_remove("DATABASE_URL")
        .output()?;
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "Database baseline verification requires DATABASE_URL.\n"
    );
    Ok(())
}

#[test]
fn invalid_endpoint_fails_without_exposing_submitted_credentials() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_verify_database_baseline"))
        .env(
            "DATABASE_URL",
            "postgres://private-credential@private-host:not-a-port/database",
        )
        .output()?;
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "Database baseline connection configuration is invalid.\n"
    );
    Ok(())
}
